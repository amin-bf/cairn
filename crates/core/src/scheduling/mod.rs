//! See `CONTEXT.md` beside this file for the vocabulary, the binding ADR sections, and the rules
//! that break silently.
//!
//! FSRS-6 arithmetic: given a card's ordered grades and day numbers, what is its memory state, when
//! is it next due, and which box does it show. Pure — no clock, no randomness, no storage.
//!
//! This context owns `leitner-core`'s one dependency, the `fsrs` crate (ADR-0027). Two rules travel
//! with it and are load-bearing here:
//!
//! * **The fuzz is ours, not the crate's** (ADR-0027 §5, ADR-0001 §7). `fsrs` ships its own interval
//!   fuzz over `rand`, but exposes the **un-fuzzed** interval through [`fsrs::FSRS::next_interval`]
//!   (which routes to a pure `stability / factor * (retention^(1/decay) − 1)` with no RNG on the
//!   path — `rand` is confined to the crate's training and simulation modules). This was
//!   [#78](https://github.com/amin-bf/leitner/issues/78)'s open item and the answer is a fact read
//!   out of the pinned version: the un-fuzzed interval is available, so we take it and apply fuzz
//!   seeded from the [`CardRef`] encoding. A fuzz the crate seeds is one two devices do not agree on.
//! * **`rand`, `serde`, `rayon`, `ndarray` arrive transitively and are not ours to reach for**
//!   (ADR-0027 §3). Nothing in this module touches them.

use std::sync::{Arc, Mutex};

use fsrs::{
    CombinedProgressState, ComputeParametersInput, DEFAULT_PARAMETERS, FSRS, FSRSItem, FSRSReview,
    MemoryState as FsrsMemoryState, compute_parameters,
};

use crate::content::CardRef;

/// The target recall probability at the scheduled due date, fixed at 0.9 and not user-exposed
/// (ADR-0001 §6). At exactly this retention the scheduled interval is numerically equal to
/// stability, which is why [`box_of`] and the interval can be read off the same quantity.
pub const DESIRED_RETENTION: f32 = 0.9;

/// The number of weights in an FSRS-6 parameter vector (ADR-0001 §6).
pub const PARAMETER_COUNT: usize = 21;

/// A grade: the user's rating of a single recall attempt (ADR-0001 §2). `Forgot` is the only
/// failure; `Barely`, `Good` and `Easy` all take the success branch of the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Grade {
    Forgot = 1,
    Barely = 2,
    Good = 3,
    Easy = 4,
}

impl Grade {
    /// Parse the raw 1–4 a `reviewed` row carries (ADR-0004 §5). Returns `None` for anything else,
    /// so a row with an out-of-range grade is skipped by replay rather than aborting it.
    pub fn from_raw(raw: u8) -> Option<Grade> {
        match raw {
            1 => Some(Grade::Forgot),
            2 => Some(Grade::Barely),
            3 => Some(Grade::Good),
            4 => Some(Grade::Easy),
            _ => None,
        }
    }

    /// The raw 1–4 value the model consumes and the log stores.
    pub fn raw(self) -> u8 {
        self as u8
    }

    /// Whether this grade is the failure branch. Only `Forgot` is (ADR-0001 §2).
    pub fn is_failure(self) -> bool {
        matches!(self, Grade::Forgot)
    }
}

/// A card's memory state: the `(stability, difficulty)` pair FSRS derives by replaying its review
/// history (ADR-0001 §1). Never authored directly, never the source of truth.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemoryState {
    /// Days for recall probability to fall to 90%. Non-decreasing on a pass, non-increasing on a
    /// lapse (ADR-0001 §3) — which is what makes it safe to build a box on.
    pub stability: f32,
    /// How hard it is to increase a card's stability, clamped to `[1, 10]`.
    pub difficulty: f32,
}

impl From<FsrsMemoryState> for MemoryState {
    fn from(s: FsrsMemoryState) -> Self {
        MemoryState {
            stability: s.stability,
            difficulty: s.difficulty,
        }
    }
}

impl From<MemoryState> for FsrsMemoryState {
    fn from(s: MemoryState) -> Self {
        FsrsMemoryState {
            stability: s.stability,
            difficulty: s.difficulty,
        }
    }
}

/// The 21-weight vector plus the desired retention that turn stability into an interval. Collection
/// state carried in the log (ADR-0001 §6), initialised to the published FSRS-6 defaults.
///
/// The algorithm identity and fitted-over count that ADR-0004 §6 groups with the weights are not
/// carried here: they are informational on the `config-set` row and do not enter the arithmetic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SchedulerParameters {
    weights: [f32; PARAMETER_COUNT],
}

impl SchedulerParameters {
    pub fn new(weights: [f32; PARAMETER_COUNT]) -> Self {
        SchedulerParameters { weights }
    }

    /// Build from a `config-set` row's weight list, rejecting the wrong length so a malformed row
    /// cannot install a partial vector.
    pub fn from_slice(weights: &[f32]) -> Option<Self> {
        let array: [f32; PARAMETER_COUNT] = weights.try_into().ok()?;
        Some(SchedulerParameters::new(array))
    }

    pub fn weights(&self) -> &[f32; PARAMETER_COUNT] {
        &self.weights
    }
}

impl Default for SchedulerParameters {
    fn default() -> Self {
        SchedulerParameters::new(DEFAULT_PARAMETERS)
    }
}

/// One review as scheduling consumes it: a grade and the **day number** it fell on (ADR-0004 §4,
/// frozen at write time). The day gap between consecutive reviews is what FSRS calls `delta_t`;
/// this context computes it from the day numbers, so callers pass the raw pair the log records.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Review {
    pub grade: Grade,
    pub day: i64,
}

/// The scheduler: an FSRS-6 model wrapped so the domain speaks in [`Grade`], [`MemoryState`] and
/// [`CardRef`] rather than in the crate's types. Constructed once per parameter vector; replay
/// rebuilds it only when a `config-set` row changes the weights.
#[derive(Debug, Clone)]
pub struct Scheduler {
    fsrs: FSRS,
    params: SchedulerParameters,
}

impl Scheduler {
    pub fn new(params: SchedulerParameters) -> Self {
        let fsrs = FSRS::new(params.weights()).expect("a full 21-weight vector is always valid");
        Scheduler { fsrs, params }
    }

    pub fn parameters(&self) -> SchedulerParameters {
        self.params
    }

    /// Fold one more review into a card's memory state. `state` is `None` for the card's first
    /// review, where `delta_t` is ignored and the state is initialised from the grade alone; for a
    /// later review `delta_t` is the day gap since the previous one.
    ///
    /// Folding review-by-review this way is byte-for-byte identical to handing FSRS the whole
    /// history at once (the crate threads only `(stability, difficulty)` between reviews), which is
    /// what lets replay switch parameters mid-history when a `config-set` row lands between two of a
    /// card's reviews.
    pub fn advance(&self, state: Option<MemoryState>, grade: Grade, delta_t: u32) -> MemoryState {
        let item = FSRSItem {
            reviews: vec![FSRSReview {
                rating: u32::from(grade.raw()),
                delta_t,
            }],
        };
        let next = self
            .fsrs
            .memory_state(item, state.map(Into::into))
            .expect("in-range grades and non-negative day gaps keep the state finite");
        next.into()
    }

    /// Replay a card's ordered `(grade, day)` history to its current memory state, or `None` if it
    /// has never been reviewed. `delta_t` for each review is the day gap since the previous one,
    /// which the day-ordered log guarantees is non-negative (ADR-0004 §9).
    pub fn memory_state(&self, reviews: &[Review]) -> Option<MemoryState> {
        let mut state = None;
        let mut prev_day: Option<i64> = None;
        for review in reviews {
            let delta_t = match prev_day {
                None => 0,
                Some(prev) => day_gap(prev, review.day),
            };
            state = Some(self.advance(state, review.grade, delta_t));
            prev_day = Some(review.day);
        }
        state
    }

    /// Current recall probability for a memory state after `elapsed_days` (ADR-0001 §1). By the
    /// definition of stability, this equals [`DESIRED_RETENTION`] exactly when `elapsed_days`
    /// equals the stability.
    pub fn retrievability(&self, state: MemoryState, elapsed_days: f32) -> f32 {
        // The decay parameter is the last weight; passing it keeps this consistent with a custom
        // vector installed by a `config-set` row, not only the default.
        let decay = self.params.weights[20];
        fsrs::current_retrievability(state.into(), elapsed_days, decay)
    }

    /// The **un-fuzzed** interval for a memory state, in days (ADR-0027 §5). This is what the crate
    /// exposes and what our fuzz is applied on top of; it is never displayed directly, because
    /// [`Scheduler::next_interval`] is what the schedule uses.
    pub fn next_interval_unfuzzed(&self, state: MemoryState) -> f32 {
        // Rating is ignored when a stability is supplied (only a brand-new card reads it).
        self.fsrs.next_interval(
            Some(state.stability),
            DESIRED_RETENTION,
            u32::from(Grade::Good.raw()),
        )
    }

    /// The scheduled next interval in days, with **our** fuzz applied — seeded from the `CardRef`
    /// encoding and the review count, never from an RNG (ADR-0001 §7, ADR-0027 §3, §5). Two devices
    /// replaying one log therefore compute the same due date.
    ///
    /// Fuzz shifts only a future due date; the quantity the model consumes is the elapsed time
    /// actually recorded, so fuzz can never perturb replayed memory state (ADR-0001 §7). This is
    /// also why the box is read off stability and never off this interval (ADR-0001 §3).
    pub fn next_interval(&self, card: &CardRef, review_count: u32, state: MemoryState) -> u32 {
        let interval = self.next_interval_unfuzzed(state);
        apply_fuzz(interval, fuzz_seed(card, review_count))
    }
}

// --- The optimisation run (ADR-0014) -----------------------------------------------------------
//
// Fitting a fresh parameter vector to this collection's own review history. The compute cost is not
// the constraint (4.3 s for a decade of the heaviest use, ADR-0014 context); the constraint is that
// Android freezes a backgrounded app, so this is driven from a worker thread the frame loop polls,
// with progress and cancellation taken from the scheduler crate's own facilities and **nothing
// persisted until it completes** (client-stack rule 10, ADR-0014 §3). Those facilities are exposed
// through [`OptimisationProgress`] rather than raw, so a run wraps them and callers above this crate
// never touch the underlying scheduler types (ADR-0027 §3).

/// The result of an [`optimise`] run: the fitted vector and the **fitted-over count**, the number of
/// reviews it trained on. The count is frozen onto the `config-set` row at write time and never
/// recomputed (ADR-0014 §6, scheduling `CONTEXT.md`).
#[derive(Debug, Clone, PartialEq)]
pub struct OptimisationOutcome {
    pub parameters: SchedulerParameters,
    pub fitted_over: u64,
}

/// A handle onto an optimisation run's progress and its cancellation flag (ADR-0014 §3, §4). It
/// wraps the scheduler crate's own `current()`/`total()` progress and `want_abort` — determinate
/// progress and cooperative cancellation are already supported and cost one `bool` — so the layers
/// above `leitner-core` read progress and request cancellation without ever seeing a scheduler type.
///
/// The two phases fall out of `total()` (ADR-0014 §4, and the corpus-build open item): it reads zero
/// during the uncancellable corpus build and the pre-training set-up, then becomes positive once the
/// determinate training loop starts. A caller renders an indeterminate lead-in while it is zero.
#[derive(Clone, Default)]
pub struct OptimisationProgress {
    state: Arc<Mutex<CombinedProgressState>>,
}

impl OptimisationProgress {
    pub fn new() -> Self {
        OptimisationProgress {
            state: CombinedProgressState::new_shared(),
        }
    }

    /// Training steps completed so far. Zero throughout the uncancellable first phase.
    pub fn current(&self) -> usize {
        self.state.lock().expect("progress lock poisoned").current()
    }

    /// Total training steps, or zero before the determinate training loop begins (ADR-0014 §4).
    pub fn total(&self) -> usize {
        self.state.lock().expect("progress lock poisoned").total()
    }

    /// Whether the run has reported completion. A short run can finish without `total()` ever leaving
    /// zero (fewer than a batch of items never enters the training loop), so this is the honest
    /// "done" signal, distinct from `current() == total()`.
    pub fn finished(&self) -> bool {
        self.state
            .lock()
            .expect("progress lock poisoned")
            .finished()
    }

    /// Request cancellation (ADR-0014 §4's Cancel). Cooperative: the run stops at the next check and
    /// [`optimise`] then returns `None`, so nothing is written.
    pub fn request_abort(&self) {
        self.state
            .lock()
            .expect("progress lock poisoned")
            .want_abort = true;
    }

    /// Whether cancellation has been requested.
    pub fn is_aborted(&self) -> bool {
        self.state
            .lock()
            .expect("progress lock poisoned")
            .want_abort
    }
}

/// Fit a parameter vector to a collection's review history (ADR-0014 §1). `histories` is one entry
/// per card, each the card's reviews in chronological order — the raw material `replay` extracts
/// from the log. `progress` carries the determinate training progress and the cancellation flag.
///
/// Returns `None` when the run was cancelled (or the optimiser rejected the corpus): there is then
/// no vector, so the caller writes nothing and the recovery action is to run it again (ADR-0014 §3).
/// An empty or history-less collection fits the published defaults, which equals what is current, so
/// the write is skipped upstream (ADR-0014 §5) with no special case here.
///
/// The **fitted-over count is every review the run saw**, summed across cards and frozen into the
/// outcome — not derived by counting rows later, which after a merge reports a fit that never
/// happened (ADR-0014 §6).
pub fn optimise(
    histories: &[Vec<Review>],
    progress: &OptimisationProgress,
) -> Option<OptimisationOutcome> {
    let fitted_over: u64 = histories.iter().map(|h| h.len() as u64).sum();
    let train_set = training_items(histories);
    let weights = compute_parameters(ComputeParametersInput {
        train_set,
        progress: Some(progress.state.clone()),
        ..Default::default()
    })
    .ok()?;
    // A cancelled run may still return a (partial) vector; honour the abort and discard it, so a
    // cancelled run is byte-identical to one never started (ADR-0014 §3).
    if progress.is_aborted() {
        return None;
    }
    let parameters = SchedulerParameters::from_slice(&weights)?;
    Some(OptimisationOutcome {
        parameters,
        fitted_over,
    })
}

/// The corpus build (ADR-0014 §4's uncancellable lead-in): one training item per review carrying its
/// full prefix. Each card's reviews become `FSRSReview`s — the first with `delta_t = 0`, each later
/// one with the day gap since the previous — and an item is emitted at every review **from the
/// second onward whose day gap is positive**. A same-day re-show (gap zero) is not its own training
/// example, matching how the scheduler crate converts a review history, so the fit sees the same
/// corpus a from-scratch conversion would.
fn training_items(histories: &[Vec<Review>]) -> Vec<FSRSItem> {
    let mut items = Vec::new();
    for history in histories {
        let mut reviews: Vec<FSRSReview> = Vec::with_capacity(history.len());
        let mut prev_day: Option<i64> = None;
        for review in history {
            let delta_t = match prev_day {
                None => 0,
                Some(prev) => day_gap(prev, review.day),
            };
            reviews.push(FSRSReview {
                rating: u32::from(review.grade.raw()),
                delta_t,
            });
            prev_day = Some(review.day);
            if reviews.len() >= 2 && delta_t > 0 {
                items.push(FSRSItem {
                    reviews: reviews.clone(),
                });
            }
        }
    }
    items
}

/// The FSRS day gap between two frozen day numbers, clamped to the non-negative `u32` the model
/// consumes (ADR-0004 §9 guarantees `to >= from` in replay order). A gap wider than `u32::MAX` — only
/// reachable from a corrupt row — saturates rather than truncating silently.
pub fn day_gap(from: i64, to: i64) -> u32 {
    u32::try_from((to - from).max(0)).unwrap_or(u32::MAX)
}

/// The box a memory state shows: 1–5, from **stability alone** at thresholds 1 / 7 / 30 / 180 days
/// (ADR-0001 §3). A never-reviewed card (`None`) is box 1. Never derived from the scheduled
/// interval, which fuzz and the minimum gap make non-monotone.
pub fn box_of(state: Option<MemoryState>) -> u8 {
    match state {
        None => 1,
        Some(s) if s.stability < 1.0 => 1,
        Some(s) if s.stability < 7.0 => 2,
        Some(s) if s.stability < 30.0 => 3,
        Some(s) if s.stability < 180.0 => 4,
        Some(_) => 5,
    }
}

// --- Interval fuzz, ours and seeded from card identity (ADR-0001 §7) --------------------------
//
// The fuzz *ranges* below are FSRS's standard shape — the same widening bands the crate applies over
// `rand` — so the magnitude of the jitter matches the algorithm. What is ours is the **seed**: a
// deterministic hash of the 18-byte `CardRef` encoding and the review count, in place of the crate's
// RNG, so every device computes the same date (ADR-0027 §5).

/// `(lower, upper, factor)` day bands; the fuzz half-width grows as the interval crosses each band.
const FUZZ_RANGES: &[(f32, f32, f32)] = &[
    (2.5, 7.0, 0.15),
    (7.0, 20.0, 0.10),
    (20.0, f32::INFINITY, 0.05),
];

/// A deterministic value in `[0, 1)` from the card identity and review count. FNV-1a over the
/// 18-byte encoding followed by the count's big-endian bytes; no RNG, so it is reproducible across
/// devices and runs.
fn fuzz_seed(card: &CardRef, review_count: u32) -> f32 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut mix = |byte: u8| {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    };
    for byte in card.encode() {
        mix(byte);
    }
    for byte in review_count.to_be_bytes() {
        mix(byte);
    }
    // Fold to five decimal digits of resolution in [0, 1); ample for a day-granularity jitter.
    (hash % 100_000) as f32 / 100_000.0
}

/// Apply the fuzz to an un-fuzzed interval. Intervals under 2.5 days are not fuzzed (there is no
/// room for it and clumping is not yet a problem); larger ones are jittered within a band that
/// widens with the interval, picked deterministically by `seed`.
fn apply_fuzz(interval: f32, seed: f32) -> u32 {
    if !interval.is_finite() || interval < 2.5 {
        return interval.round().max(1.0) as u32;
    }
    let mut delta = 1.0f32;
    for &(start, end, factor) in FUZZ_RANGES {
        delta += factor * (interval.min(end) - start).max(0.0);
    }
    let min_ivl = (interval - delta).round().max(2.0);
    let max_ivl = (interval + delta).round().max(min_ivl);
    let span = max_ivl - min_ivl;
    let chosen = min_ivl + (seed * (span + 1.0)).floor();
    chosen.min(max_ivl).max(1.0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::NoteId;

    fn card(ordinal: u16) -> CardRef {
        CardRef::new(NoteId([0xAB; 16]), ordinal)
    }

    #[test]
    fn only_forgot_is_a_failure() {
        // ADR-0001 §2: grades 2, 3, 4 all take the success branch; only 1 fails.
        assert!(Grade::Forgot.is_failure());
        assert!(!Grade::Barely.is_failure());
        assert!(!Grade::Good.is_failure());
        assert!(!Grade::Easy.is_failure());
    }

    #[test]
    fn grade_raw_round_trips_and_rejects_out_of_range() {
        for raw in 1..=4u8 {
            assert_eq!(Grade::from_raw(raw).unwrap().raw(), raw);
        }
        assert_eq!(Grade::from_raw(0), None);
        assert_eq!(Grade::from_raw(5), None);
    }

    #[test]
    fn box_thresholds_are_one_seven_thirty_one_eighty_on_stability() {
        // ADR-0001 §3. The lower edge of each band belongs to the higher box.
        let boxed = |s: f32| {
            box_of(Some(MemoryState {
                stability: s,
                difficulty: 5.0,
            }))
        };
        assert_eq!(box_of(None), 1, "never reviewed is box 1");
        assert_eq!(boxed(0.99), 1);
        assert_eq!(boxed(1.0), 2);
        assert_eq!(boxed(6.99), 2);
        assert_eq!(boxed(7.0), 3);
        assert_eq!(boxed(29.99), 3);
        assert_eq!(boxed(30.0), 4);
        assert_eq!(boxed(179.99), 4);
        assert_eq!(boxed(180.0), 5);
        assert_eq!(boxed(1000.0), 5);
    }

    #[test]
    fn arithmetic_over_a_hand_written_list_of_grades_and_days() {
        // The verification surface #78 exists to establish: a hand-written list of grades and day
        // numbers, no rows, no writers, no merge, no database, no window, no handset.
        let sched = Scheduler::new(SchedulerParameters::default());

        // A card recalled well over three weeks: Good on day 0, then passes at widening gaps.
        let history = [
            Review {
                grade: Grade::Good,
                day: 0,
            },
            Review {
                grade: Grade::Good,
                day: 3,
            },
            Review {
                grade: Grade::Good,
                day: 10,
            },
            Review {
                grade: Grade::Easy,
                day: 24,
            },
        ];

        // The first review's stability is the initial stability for the grade — the third default
        // weight, exactly, since nothing has decayed yet (ADR-0001 §3).
        let after_first = sched.memory_state(&history[..1]).unwrap();
        assert!(
            (after_first.stability - DEFAULT_PARAMETERS[2]).abs() < 1e-4,
            "first Good initialises stability to w2, got {}",
            after_first.stability
        );

        // Stability is non-decreasing on every passing grade, and so is the box (ADR-0001 §3).
        let mut prev_stability = 0.0f32;
        let mut prev_box = 0u8;
        for n in 1..=history.len() {
            let state = sched.memory_state(&history[..n]).unwrap();
            assert!(
                state.stability >= prev_stability - 1e-3,
                "stability fell on a pass at step {n}: {prev_stability} -> {}",
                state.stability
            );
            let b = box_of(Some(state));
            assert!(
                b >= prev_box,
                "box fell on a pass at step {n}: {prev_box} -> {b}"
            );
            prev_stability = state.stability;
            prev_box = b;
        }

        // Difficulty stays inside the model's clamp.
        let final_state = sched.memory_state(&history).unwrap();
        assert!((1.0..=10.0).contains(&final_state.difficulty));
    }

    #[test]
    fn a_lapse_collapses_stability_and_drops_the_box() {
        // ADR-0001 §5: a single Forgot returns a well-learned card toward the start, without any
        // reset rule of ours — the model provides it.
        let sched = Scheduler::new(SchedulerParameters::default());
        let mut history = vec![Review {
            grade: Grade::Good,
            day: 0,
        }];
        // Build the card up over months of successful review.
        for day in [1, 4, 12, 35, 100, 260] {
            history.push(Review {
                grade: Grade::Good,
                day,
            });
        }
        let strong = sched.memory_state(&history).unwrap();
        assert!(
            box_of(Some(strong)) >= 4,
            "expected a durable card, got box {}",
            box_of(Some(strong))
        );

        // Now fail it.
        history.push(Review {
            grade: Grade::Forgot,
            day: 400,
        });
        let lapsed = sched.memory_state(&history).unwrap();
        assert!(
            lapsed.stability < strong.stability,
            "a lapse must not raise stability: {} -> {}",
            strong.stability,
            lapsed.stability
        );
        assert!(
            box_of(Some(lapsed)) < box_of(Some(strong)),
            "the box must drop on a lapse"
        );
    }

    #[test]
    fn retrievability_is_one_at_zero_elapsed_and_the_retention_at_the_stability() {
        // Stability is defined as the days for recall to fall to 0.9, so this is a direct check of
        // the arithmetic against its own definition (ADR-0001 §1, §3).
        let sched = Scheduler::new(SchedulerParameters::default());
        let state = sched
            .memory_state(&[
                Review {
                    grade: Grade::Good,
                    day: 0,
                },
                Review {
                    grade: Grade::Good,
                    day: 5,
                },
            ])
            .unwrap();
        assert!((sched.retrievability(state, 0.0) - 1.0).abs() < 1e-4);
        assert!(
            (sched.retrievability(state, state.stability) - DESIRED_RETENTION).abs() < 1e-3,
            "retrievability at elapsed == stability must be the desired retention"
        );
        // And it falls with elapsed time.
        assert!(sched.retrievability(state, 2.0) > sched.retrievability(state, 20.0));
    }

    #[test]
    fn fuzz_is_deterministic_from_card_and_count_and_stays_near_the_unfuzzed_interval() {
        let sched = Scheduler::new(SchedulerParameters::default());
        let state = MemoryState {
            stability: 40.0,
            difficulty: 5.0,
        };

        // Same card, same count -> same interval, on every call (no RNG).
        let a = sched.next_interval(&card(0), 3, state);
        let b = sched.next_interval(&card(0), 3, state);
        assert_eq!(a, b, "fuzz must be reproducible for a fixed (card, count)");

        // The fuzzed interval sits within the widening band around the un-fuzzed one.
        let base = sched.next_interval_unfuzzed(state);
        assert!(
            (a as f32 - base).abs() <= base * 0.1 + 2.0,
            "fuzzed {a} strayed too far from un-fuzzed {base}"
        );
    }

    #[test]
    fn fuzz_varies_across_cards_and_counts() {
        // Different seeds should not all collapse to one value — a weak check that the seed is used.
        let sched = Scheduler::new(SchedulerParameters::default());
        let state = MemoryState {
            stability: 60.0,
            difficulty: 5.0,
        };
        let mut seen = std::collections::HashSet::new();
        for ordinal in 0..24u16 {
            seen.insert(sched.next_interval(&card(ordinal), 1, state));
        }
        assert!(seen.len() > 1, "fuzz seed had no effect across cards");
    }

    /// A card's history from a compact `(grade, day)` list, for the optimiser tests.
    fn history(pairs: &[(Grade, i64)]) -> Vec<Review> {
        pairs
            .iter()
            .map(|&(grade, day)| Review { grade, day })
            .collect()
    }

    #[test]
    fn optimising_empty_history_returns_the_defaults_and_a_zero_count() {
        // ADR-0014 §5: a collection with no review history fits the defaults, so the outcome equals
        // what is current and the store then writes nothing. The count is zero.
        let progress = OptimisationProgress::new();
        let outcome = optimise(&[], &progress).expect("a fit over nothing still yields a vector");
        assert_eq!(outcome.parameters, SchedulerParameters::default());
        assert_eq!(outcome.fitted_over, 0);
    }

    #[test]
    fn optimising_real_history_yields_a_valid_vector_and_counts_every_review() {
        // The corpus build turns each review (from the second, with a positive day gap) into a
        // training item; the fitted-over count is every review the run saw (ADR-0014 §6).
        let mut histories = Vec::new();
        for _ in 0..12 {
            histories.push(history(&[
                (Grade::Good, 0),
                (Grade::Good, 2),
                (Grade::Barely, 6),
                (Grade::Good, 15),
                (Grade::Easy, 40),
            ]));
        }
        let progress = OptimisationProgress::new();
        let outcome = optimise(&histories, &progress).expect("a real corpus fits");

        // Every review counts toward the fit, across all cards.
        assert_eq!(outcome.fitted_over, 12 * 5);
        // The fitted vector is a full, finite 21-weight vector — a scheduler can be built from it.
        let _ = Scheduler::new(outcome.parameters);
        assert!(outcome.parameters.weights().iter().all(|w| w.is_finite()));
    }

    #[test]
    fn an_aborted_run_yields_nothing_to_write() {
        // ADR-0014 §3: cancellation is cooperative, and a cancelled run holds no partial state — it
        // returns None, so the caller writes nothing and the recovery action is to press again.
        let progress = OptimisationProgress::new();
        progress.request_abort();
        let histories = vec![history(&[
            (Grade::Good, 0),
            (Grade::Good, 3),
            (Grade::Good, 9),
        ])];
        assert!(optimise(&histories, &progress).is_none());
    }

    #[test]
    fn small_intervals_are_not_fuzzed() {
        // Below 2.5 days there is no room to jitter; the interval is returned rounded (ADR-0001 §7).
        assert_eq!(apply_fuzz(1.0, 0.0), 1);
        assert_eq!(apply_fuzz(2.0, 0.999), 2);
        assert_eq!(
            apply_fuzz(0.3, 0.5),
            1,
            "a sub-day interval floors to the one-day minimum"
        );
    }
}
