//! The parameter-optimisation **experience** (ADR-0014): the one action the user takes to fit the
//! scheduler to their own review history, and everything the application says around it. Like `sync`,
//! this module is deliberately made of the parts that erode silently — the words that carry no
//! threshold and no quality claim, and the run's shape — so they are provable with no window.
//!
//! Three things live here, each a decision rather than a rendering:
//!
//! - **The nudge states a fact and nothing more** (ADR-0014 §2). [`nudge_text`] turns
//!   [`OptimisationNudge`]'s two counts into the one sentence each carries — no floor below which it
//!   stays silent, no badge, no colour, no verb. It appears **only in settings**, never at session
//!   end, and that placement is the settings screen's to honour.
//! - **The completion message states that every due date moved and makes no quality claim**
//!   (ADR-0014 §4). [`COMPLETION_MESSAGE`] is a constant precisely because the tempting addition —
//!   "your scheduling is now more accurate" — is an unfalsifiable claim the application cannot back.
//! - **The run is a worker thread polled by the frame loop, with a two-phase display and
//!   cancellation** (ADR-0014 §3, §4, client-stack rule 10). [`OptimiseJob`] owns the thread and the
//!   progress handle; the freezer is **not** engineered against — nothing is persisted until [`poll`]
//!   hands back a complete outcome, so a frozen or killed run holds no partial state and the recovery
//!   action is to start it again.
//!
//! [`poll`]: OptimiseJob::poll

use std::thread::JoinHandle;

use cairn_core::replay::{OptimisationNudge, training_histories};
use cairn_core::scheduling::{OptimisationOutcome, OptimisationProgress, optimise};

/// What the settings screen says on completion (ADR-0014 §4). Two facts and no third: the parameters
/// changed, and — the half that is not decoration — **every due date moved**, because the current
/// vector is applied over the whole history so every card's `(S, D)` is recomputed (ADR-0001 §6). It
/// makes **no quality claim**: the population-average benefit is someone else's benchmark, and the
/// application has no instrument that tells *this* user whether *their* collection improved.
pub const COMPLETION_MESSAGE: &str = "Parameters updated. Due dates have been recalculated.";

/// The nudge sentence for the settings screen (ADR-0014 §2), from the fact `replay` derived. Carries
/// only the counts — no threshold, no badge, no verb — so the user infers "that sounds like a lot"
/// without the application making a claim it cannot support.
pub fn nudge_text(nudge: &OptimisationNudge) -> String {
    match nudge {
        OptimisationNudge::Standard { reviews_total } => format!(
            "Using the standard parameters. You've reviewed {} {}.",
            grouped(*reviews_total),
            times(*reviews_total),
        ),
        OptimisationNudge::Fitted {
            fitted_over,
            reviews_since,
        } => format!(
            "Fitted over {} {}. You've reviewed {} {} since.",
            grouped(*fitted_over),
            reviews(*fitted_over),
            grouped(*reviews_since),
            times(*reviews_since),
        ),
    }
}

/// `"review"` or `"reviews"` for a count — the nudge reads as a sentence, so a one-review fit is not
/// "1 reviews".
fn reviews(n: u64) -> &'static str {
    if n == 1 { "review" } else { "reviews" }
}

/// `"time"` or `"times"`, the same courtesy for the review tally.
fn times(n: u64) -> &'static str {
    if n == 1 { "time" } else { "times" }
}

/// A count with thousands separators — the nudge's numbers are large enough that "3,120" reads where
/// "3120" does not (ADR-0014 §2's own examples group this way).
fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let len = digits.len();
    let mut out = String::with_capacity(len + len / 3);
    for (i, ch) in digits.char_indices() {
        if i > 0 && (len - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// Which phase the two-phase progress display is in (ADR-0014 §4, and the corpus-build open item).
/// The scheduler crate's progress covers **training only**, so the corpus build and pre-training
/// set-up have no `current()`/`total()` to read and no abort to honour: an indeterminate lead-in,
/// then the determinate bar. The distinction is read straight off the progress handle — `total()` is
/// zero until the training loop begins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// The uncancellable corpus build and set-up (ADR-0014 §4). Indeterminate — render a spinner, not
    /// a bar. A Cancel pressed here is honoured only once training starts, which is the open item's
    /// accepted shape, not a bug.
    Preparing,
    /// The determinate training loop: `current` of `total` steps done.
    Training { current: usize, total: usize },
}

/// A running optimisation (ADR-0014 §3). Spawns the corpus build and the fit on a worker thread and
/// exposes progress, cancellation and the result to the frame loop — **the freezer is not engineered
/// against**, because nothing is persisted until [`poll`](Self::poll) yields a complete outcome, so a
/// backgrounded (frozen or killed) run holds no partial state and the recovery action is to start
/// another.
///
/// **This never touches the store or the network.** The leading sync of ADR-0014 §7 — *sync, then
/// train* — is the caller's to sequence before starting a job, and is a no-op where no transport is
/// enrolled or reachable (an offline device optimising on local history is a fine outcome). The write
/// of the fitted vector is likewise the caller's, once `poll` hands back the outcome, through
/// [`cairn_store::Collection::set_scheduler_parameters`], which skips an unchanged vector.
pub struct OptimiseJob {
    progress: OptimisationProgress,
    handle: Option<JoinHandle<Option<OptimisationOutcome>>>,
}

impl OptimiseJob {
    /// Start a run over a snapshot of the log lines. The lines are moved onto the worker thread, which
    /// builds the training corpus (`training_histories`) and fits the vector (`optimise`) — both off
    /// the frame thread, so a decade-scale fit never blocks a frame or trips Android's ANR watchdog.
    pub fn start(log_lines: Vec<String>) -> Self {
        let progress = OptimisationProgress::new();
        let worker = progress.clone();
        let handle = std::thread::spawn(move || {
            let refs: Vec<&str> = log_lines.iter().map(String::as_str).collect();
            let histories = training_histories(&refs);
            optimise(&histories, &worker)
        });
        OptimiseJob {
            progress,
            handle: Some(handle),
        }
    }

    /// The current display phase (ADR-0014 §4). `Preparing` until the training loop reports a total.
    pub fn phase(&self) -> Phase {
        let total = self.progress.total();
        if total == 0 {
            Phase::Preparing
        } else {
            Phase::Training {
                current: self.progress.current(),
                total,
            }
        }
    }

    /// Request cancellation (ADR-0014 §4). Cooperative and honoured once training is under way; a run
    /// cancelled during the uncancellable lead-in stops at the first training check and then yields
    /// nothing. Either way the run leaves no partial state.
    pub fn cancel(&self) {
        self.progress.request_abort();
    }

    /// Poll for completion, to be called each frame after `ctx.request_repaint()`. Returns `None`
    /// while the run is still going; `Some(outcome)` once it has finished, where the inner value is
    /// `None` if the run was cancelled or the optimiser produced nothing to write. The caller then
    /// writes the vector (if any) and drops the job. A worker-thread panic reads as "nothing to
    /// write", the same recovery as a cancel.
    pub fn poll(&mut self) -> Option<Option<OptimisationOutcome>> {
        if !self.handle.as_ref().is_some_and(JoinHandle::is_finished) {
            return None;
        }
        let handle = self.handle.take()?;
        Some(handle.join().unwrap_or(None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_standard_nudge_states_the_total_and_no_verb() {
        // ADR-0014 §2: "Using the standard parameters. You've reviewed 4,200 times." — a fact, no
        // threshold, no verb, no claim.
        let text = nudge_text(&OptimisationNudge::Standard {
            reviews_total: 4_200,
        });
        assert_eq!(
            text,
            "Using the standard parameters. You've reviewed 4,200 times."
        );
    }

    #[test]
    fn the_fitted_nudge_states_both_counts() {
        // ADR-0014 §2: "Fitted over 3,120 reviews. You've reviewed 1,400 times since."
        let text = nudge_text(&OptimisationNudge::Fitted {
            fitted_over: 3_120,
            reviews_since: 1_400,
        });
        assert_eq!(
            text,
            "Fitted over 3,120 reviews. You've reviewed 1,400 times since."
        );
    }

    #[test]
    fn the_nudge_carries_no_threshold_or_quality_claim() {
        // ADR-0014 §2: no badge, no colour, no verb — and nothing that reads as a judgement.
        for nudge in [
            OptimisationNudge::Standard { reviews_total: 900 },
            OptimisationNudge::Fitted {
                fitted_over: 900,
                reviews_since: 30,
            },
        ] {
            let lower = nudge_text(&nudge).to_lowercase();
            for banned in [
                "should",
                "recommend",
                "accurate",
                "better",
                "improve",
                "optimal",
                "stale",
                "due",
            ] {
                assert!(
                    !lower.contains(banned),
                    "the nudge made a claim ({banned:?}): {lower}"
                );
            }
        }
    }

    #[test]
    fn a_one_review_nudge_reads_singular() {
        assert_eq!(
            nudge_text(&OptimisationNudge::Fitted {
                fitted_over: 1,
                reviews_since: 1,
            }),
            "Fitted over 1 review. You've reviewed 1 time since."
        );
    }

    #[test]
    fn the_completion_message_states_the_due_date_move_and_claims_no_quality() {
        // ADR-0014 §4: the second sentence is load-bearing (every due date moved), and there is no
        // claim about accuracy — the application has no instrument for it.
        let lower = COMPLETION_MESSAGE.to_lowercase();
        assert!(
            lower.contains("due date"),
            "must state that due dates moved: {COMPLETION_MESSAGE}"
        );
        for banned in [
            "accurate", "better", "improved", "quality", "optimal", "faster",
        ] {
            assert!(
                !lower.contains(banned),
                "the completion message made a quality claim ({banned:?})"
            );
        }
    }

    #[test]
    fn a_job_runs_on_a_worker_thread_and_hands_back_a_complete_outcome() {
        // ADR-0014 §3: the run is off the frame thread; nothing is read until it finishes, and what
        // comes back is a whole vector — never a partial one.
        let mut lines: Vec<String> = Vec::new();
        // A handful of cards with multi-day histories, enough to fit a real (non-default) vector.
        for card in 0..8u64 {
            let note = format!("{card:08}-1111-4111-8111-111111111111");
            for (i, (grade, day)) in [(3, 0), (3, 2), (2, 6), (3, 15), (4, 40)]
                .iter()
                .enumerate()
            {
                lines.push(format!(
                    r#"{{"k":"rev","w":"w","s":{},"n":"{}","o":0,"g":{},"t":"day-{:08}","d":{},"ms":1000}}"#,
                    card * 100 + i as u64,
                    note,
                    grade,
                    day,
                    day
                ));
            }
        }

        let mut job = OptimiseJob::start(lines);
        // Before it finishes, the phase is a valid two-phase reading.
        assert!(matches!(
            job.phase(),
            Phase::Preparing | Phase::Training { .. }
        ));

        let outcome = loop {
            if let Some(result) = job.poll() {
                break result;
            }
            std::thread::yield_now();
        };
        let outcome = outcome.expect("a real corpus yields a vector");
        // Every review across all cards is counted, and the vector is whole.
        assert_eq!(outcome.fitted_over, 8 * 5);
        assert!(outcome.parameters.weights().iter().all(|w| w.is_finite()));
    }

    #[test]
    fn a_cancelled_job_yields_nothing_to_write() {
        // ADR-0014 §3: a cancelled run holds no partial state — poll hands back `None`, so the caller
        // writes nothing and the recovery is to start another.
        let mut lines = Vec::new();
        for i in 0..4u64 {
            lines.push(format!(
                r#"{{"k":"rev","w":"w","s":{i},"n":"11111111-1111-4111-8111-111111111111","o":0,"g":3,"t":"day-{i:08}","d":{i},"ms":1000}}"#
            ));
        }
        let mut job = OptimiseJob::start(lines);
        job.cancel();
        let outcome = loop {
            if let Some(result) = job.poll() {
                break result;
            }
            std::thread::yield_now();
        };
        assert!(outcome.is_none(), "a cancelled run writes nothing");
    }
}
