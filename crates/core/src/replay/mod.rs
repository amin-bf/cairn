//! See `CONTEXT.md` beside this file for the vocabulary, the binding ADR sections, and the rules
//! that break silently.
//!
//! The join. Given the current content (which cards exist) and the whole review log, replay computes
//! what the collection looks like right now: each card's memory state, its box, and when it is next
//! scheduled. Everything it produces is derived and disposable — the log is the only authority.
//!
//! **Replay takes no clock and no randomness** (ADR-0001 §7, ADR-0004 §4). Day numbers are frozen on
//! the row at write time and read back, never recomputed; fuzz is seeded from the `CardRef`. If
//! replay ever read the wall clock, two devices would stop agreeing and the entire merge design
//! would be void — so `now` appears nowhere in this module's signatures.

use std::collections::{HashMap, HashSet};

use crate::content::{CardRef, NoteId};
use crate::log::{ParsedLine, Row, Setting, parse_line};
use crate::scheduling::{Grade, MemoryState, Review, Scheduler, SchedulerParameters, box_of};

/// The constant identifying our replay arithmetic together with the pinned scheduler version (replay
/// `CONTEXT.md`, ADR-0004 §9). A cache stamped with a different value cannot be trusted and is
/// discarded and rebuilt — **the derivation is versioned; the projection is not**, so there is no
/// migration path. Bump the `replay-N` half whenever a change here would make old cached state
/// disagree with fresh arithmetic; the `fsrs-x.y.z` half must track the `=` pin in the workspace
/// `Cargo.toml`, since a scheduler change re-derives every interval.
pub const DERIVATION_VERSION: &str = "replay-1/fsrs-6.6.1";

/// The replayed state of one currently-generated card that has at least one projected review.
///
/// A card the current content generates but which has no reviews does not appear in [`Replayed`];
/// its absence *is* the never-reviewed state, which [`box_of`] maps to box 1.
#[derive(Debug, Clone, PartialEq)]
pub struct CardState {
    /// The card's memory state after its whole projected history.
    pub memory: MemoryState,
    /// The box it shows — from stability alone (ADR-0001 §3).
    pub box_: u8,
    /// How many reviews were projected onto it.
    pub review_count: u32,
    /// The frozen day number of the card's **earliest** projected review — the day it was
    /// *introduced* (ADR-0011 §5, replay `CONTEXT.md`). A card is "introduced today" when this equals
    /// the device-local day, which is how [`notes_introduced_today`] reads it. Never a lapse re-show:
    /// a re-show is not an earliest row, so it never moves this value.
    pub first_day: i64,
    /// The most recent projected grade. What lets the session queue keep a **lapsed** card and drop a
    /// **passed** one (ADR-0011 §9): a failed card genuinely is still due and returns within the same
    /// session, where the plain scheduled `due_day` would floor it a day out and lose the re-show.
    pub last_grade: Grade,
    /// The frozen day number of its most recent projected review.
    pub last_day: i64,
    /// The next scheduled day: the last review's day plus the fuzzed interval. This is a scheduled
    /// date, not a "due today" verdict — the latter is measured against the device-local day at the
    /// edge, which replay has no access to and wants none (ADR-0004 §4, replay `CONTEXT.md`).
    pub due_day: i64,
    /// The distinct frozen day numbers on which this card was graded `1 Forgot` at least once, in
    /// ascending order — its **failure days** (ADR-0010 §2, replay `CONTEXT.md`). Counted in days,
    /// never in rows: a same-session re-show is a real logged row with a zero day gap, so three
    /// grade-1 rows one evening are **one** failure day, not three. This is the raw material the leech
    /// query filters to the trailing window; replay itself never reads the edge, so the window is
    /// applied by [`leeches`], not here.
    pub failure_days: Vec<i64>,
}

/// The whole projection: the state of every currently-generated card that carries reviews.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Replayed {
    pub cards: HashMap<CardRef, CardState>,
}

/// A card's running accumulator during the fold.
struct Accumulator {
    state: MemoryState,
    first_day: i64,
    last_grade: Grade,
    last_day: i64,
    count: u32,
    /// Distinct failure days, ascending. Rows arrive in day order, so a failure day is appended only
    /// when it differs from the last one already recorded — that dedup by day is the whole point of
    /// counting days rather than rows (ADR-0010 §2).
    failure_days: Vec<i64>,
}

/// Replay the log against the set of currently-generated cards.
///
/// `current_cards` is the card set computed from **current content** (ADR-0002 §7): a row whose
/// `CardRef` is not in it is **retained and simply not projected** — never deleted, never an error.
/// `lines` is the log in its canonical interchange form (ADR-0004 §11); order does not matter,
/// because replay sorts internally.
///
/// The mechanism, per replay's `CONTEXT.md`:
///
/// * The log is read in `(day number, then instant, then writer, then sequence)` order (ADR-0004
///   §9) — a deterministic total order every device reproduces.
/// * Rows before a `history-cutoff-set` (by frozen day number) are ignored entirely (ADR-0004 §1).
/// * `config-set` rows change the scheduler parameters **from that point forward** (#78) — each
///   review is folded under the parameters current when it is reached.
/// * Unknown row kinds and malformed lines are skipped and never abort the replay (ADR-0004 §11).
pub fn replay(current_cards: &HashSet<CardRef>, lines: &[&str]) -> Replayed {
    // Parse, dropping unknown kinds and malformed lines — both are skipped for projection.
    // Merging two logs is **set union with duplicate `(writer, sequence)` pairs dropped** (ADR-0004
    // §2): two rows with the same identity are the same row, so a repeat must not be folded twice.
    let mut rows: Vec<Row> = Vec::new();
    let mut seen: HashSet<crate::log::RowId> = HashSet::new();
    for line in lines {
        if let ParsedLine::Row(row) = parse_line(line)
            && seen.insert(row.id().clone())
        {
            rows.push(row);
        }
    }

    // The cutoff is the highest frozen day number any `history-cutoff-set` names; reviewed rows
    // strictly before it are disowned (ADR-0004 §1). Config-set rows are not reviewed rows and are
    // never erased by a cutoff, so parameter history before it still applies.
    let cutoff_day: Option<i64> = rows
        .iter()
        .filter_map(|row| match row {
            Row::HistoryCutoff(r) => Some(r.day),
            _ => None,
        })
        .max();

    // Order the reviewed and config-set rows into the single total order every device reproduces.
    let mut ordered: Vec<&Row> = rows
        .iter()
        .filter(|row| matches!(row, Row::Reviewed(_) | Row::ConfigSet(_)))
        .collect();
    ordered.sort_by(|a, b| {
        a.day()
            .cmp(&b.day())
            .then_with(|| a.instant().cmp(b.instant()))
            .then_with(|| a.id().writer.cmp(&b.id().writer))
            .then_with(|| a.id().sequence.cmp(&b.id().sequence))
    });

    let mut scheduler = Scheduler::new(SchedulerParameters::default());
    let mut acc: HashMap<CardRef, Accumulator> = HashMap::new();

    for row in ordered {
        match row {
            Row::ConfigSet(cfg) => {
                if let Setting::SchedulerParameters { weights, .. } = &cfg.setting {
                    scheduler = Scheduler::new(SchedulerParameters::new(*weights));
                }
                // Other settings do not enter this ticket's arithmetic.
            }
            Row::Reviewed(rev) => {
                if let Some(cutoff) = cutoff_day
                    && rev.day < cutoff
                {
                    continue;
                }
                // Dormant: a row whose card the current content does not generate is retained in the
                // log and simply not projected (ADR-0002 §7).
                if !current_cards.contains(&rev.card) {
                    continue;
                }
                // An out-of-range grade cannot be projected; skip it rather than abort.
                let Some(grade) = Grade::from_raw(rev.grade) else {
                    continue;
                };

                match acc.get_mut(&rev.card) {
                    None => {
                        // First projected review of this card: state initialised from the grade,
                        // day gap ignored. This row's day is the introduction day (ADR-0011 §5) —
                        // rows arrive in day order, so the first seen is the earliest.
                        let state = scheduler.advance(None, grade, 0);
                        acc.insert(
                            rev.card,
                            Accumulator {
                                state,
                                first_day: rev.day,
                                last_grade: grade,
                                last_day: rev.day,
                                count: 1,
                                failure_days: if grade.is_failure() {
                                    vec![rev.day]
                                } else {
                                    Vec::new()
                                },
                            },
                        );
                    }
                    Some(existing) => {
                        let delta_t = crate::scheduling::day_gap(existing.last_day, rev.day);
                        existing.state = scheduler.advance(Some(existing.state), grade, delta_t);
                        existing.last_grade = grade;
                        existing.last_day = rev.day;
                        existing.count += 1;
                        // A failure day is recorded once per distinct day (ADR-0010 §2). Rows fold in
                        // day order, so a new failure day is simply one that differs from the last
                        // recorded — a same-day re-show never adds a second.
                        if grade.is_failure() && existing.failure_days.last() != Some(&rev.day) {
                            existing.failure_days.push(rev.day);
                        }
                    }
                }
            }
            Row::HistoryCutoff(_) => {}
        }
    }

    // Project each accumulator into a card state, scheduling the next interval with the final
    // parameters and the card-seeded fuzz.
    let mut cards = HashMap::with_capacity(acc.len());
    for (card, a) in acc {
        let interval = scheduler.next_interval(&card, a.count, a.state);
        cards.insert(
            card,
            CardState {
                memory: a.state,
                box_: box_of(Some(a.state)),
                review_count: a.count,
                first_day: a.first_day,
                last_grade: a.last_grade,
                last_day: a.last_day,
                due_day: a.last_day + i64::from(interval),
                failure_days: a.failure_days,
            },
        );
    }

    Replayed { cards }
}

/// A never-introduced card paired with the authored `position` of its note — the raw material for
/// [`introduction_candidates`]. `position` is the order key `content` mints (ADR-0011 §7); an empty
/// string sorts first, which is exactly the defined state of a note that predates the field
/// (ADR-0011 §7's *"a note that predates the field reads it as empty"*).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewCard {
    pub card: CardRef,
    pub position: String,
}

/// The notes that already had a card introduced today (ADR-0011 §8), derived from the projection.
///
/// A card is *introduced* the day of its **earliest** projected review ([`CardState::first_day`]);
/// "today" is the **device-local** day, the edge value replay itself refuses to read (replay
/// `CONTEXT.md`) and the caller passes in. Derived, never stored (ADR-0011 §5): losing the cache
/// loses nothing. A lapse re-show is not an earliest row, so it never enters this set.
pub fn notes_introduced_today(replayed: &Replayed, today: i64) -> HashSet<NoteId> {
    let mut notes = HashSet::new();
    for (card, state) in &replayed.cards {
        if state.first_day == today {
            notes.insert(card.note);
        }
    }
    notes
}

/// Choose the new cards to introduce, in `(position, ordinal)` order up to `rate`, **at most one card
/// per note**, skipping suspended cards and notes that already had a card introduced today (ADR-0011
/// §7, §8).
///
/// This is **queue composition, never a due-date adjustment** (replay `CONTEXT.md`): it decides only
/// what is *offered* and touches nothing the log records or any interval computes, which is why it is
/// permitted where ADR-0001 §7's sibling avoidance was not. `new_cards` are the currently-generated
/// cards that carry **no** projected review — the caller finds them by difference against
/// [`Replayed::cards`]; `introduced_today` is [`notes_introduced_today`]; `suspended` is the mutable
/// surface's per-`CardRef` flag (ADR-0010 §8), empty until #87 wires it.
///
/// Order is `(position, note id, ordinal)`: authored `position` first, ties broken by note id exactly
/// as ADR-0011 §7 fixed, and a note's own cards fall in slot order after that — though at most one of
/// them is ever taken. A suspended card is skipped **without** consuming its note's one-per-day slot,
/// so a note keeps its introduction when only a sibling is suspended.
///
/// The `rate` is the **daily** total, so cards already introduced today spend it: the remaining
/// budget is `rate` minus the count in `introduced_today` (which, by the one-per-note rule, is exactly
/// the number of cards introduced today). A device that has already met its cap earlier today offers
/// nothing more, and one part-way through offers only the rest — never the full rate a second time.
pub fn introduction_candidates(
    new_cards: &[NewCard],
    introduced_today: &HashSet<NoteId>,
    suspended: &HashSet<CardRef>,
    rate: usize,
) -> Vec<CardRef> {
    // Today's introductions already spent part of the daily rate (ADR-0011 §5, §8). One card per note
    // per day means the introduced-note count *is* the introduced-card count, so it is the amount
    // spent; what remains is the whole budget for the rest of today.
    let remaining = rate.saturating_sub(introduced_today.len());

    let mut ordered: Vec<&NewCard> = new_cards.iter().collect();
    ordered.sort_by(|a, b| {
        a.position
            .cmp(&b.position)
            .then_with(|| a.card.note.0.cmp(&b.card.note.0))
            .then_with(|| a.card.ordinal.cmp(&b.card.ordinal))
    });

    // The one-per-note guard is seeded with the notes already introduced today, so the same rule
    // covers both "this note's sibling is in this batch" and "this note had a card introduced earlier
    // today" without a second test.
    let mut used = introduced_today.clone();
    let mut chosen = Vec::new();
    for entry in ordered {
        if chosen.len() >= remaining {
            break;
        }
        if suspended.contains(&entry.card) || used.contains(&entry.card.note) {
            continue;
        }
        used.insert(entry.card.note);
        chosen.push(entry.card);
    }
    chosen
}

/// The leech floor: a card is a leech at **four or more** failure days in the trailing window
/// (ADR-0010 §2). An invention, recorded as one — roughly calibrated at both ends but with no
/// measurement behind it, and free to move (ADR-0010 §2, consequences). Nothing in the mechanism
/// depends on the exact value.
pub const LEECH_FAILURE_DAYS: u32 = 4;

/// The trailing window, in days, the failure days are counted within (ADR-0010 §2). A window in
/// **days**, not reviews, because the harm is a budget harm: a card failed twice a decade apart costs
/// nothing and a day window excludes it for free. Its right edge is the device-local day the caller
/// passes to [`leeches`].
pub const LEECH_WINDOW_DAYS: i64 = 90;

// --- Parameter optimisation: the training corpus and the settings nudge (ADR-0014) -------------

/// Parse, dedup and order the log into the single total order every device reproduces (ADR-0004 §9),
/// keeping only the `reviewed` and `config-set` rows, and return it with the history cutoff day.
/// Shared by [`training_histories`] and [`optimisation_nudge`], and the same ordering [`replay`]
/// uses — a duplicate `(writer, sequence)` is one row, unknown and malformed lines are dropped.
fn ordered_log(lines: &[&str]) -> (Vec<Row>, Option<i64>) {
    let mut rows: Vec<Row> = Vec::new();
    let mut seen: HashSet<crate::log::RowId> = HashSet::new();
    for line in lines {
        if let ParsedLine::Row(row) = parse_line(line)
            && seen.insert(row.id().clone())
        {
            rows.push(row);
        }
    }
    let cutoff = rows
        .iter()
        .filter_map(|row| match row {
            Row::HistoryCutoff(r) => Some(r.day),
            _ => None,
        })
        .max();
    let mut ordered: Vec<Row> = rows
        .into_iter()
        .filter(|row| matches!(row, Row::Reviewed(_) | Row::ConfigSet(_)))
        .collect();
    ordered.sort_by(|a, b| {
        a.day()
            .cmp(&b.day())
            .then_with(|| a.instant().cmp(b.instant()))
            .then_with(|| a.id().writer.cmp(&b.id().writer))
            .then_with(|| a.id().sequence.cmp(&b.id().sequence))
    });
    (ordered, cutoff)
}

/// One card's review history from the log, for the optimiser (ADR-0014 §1). Returns one entry per
/// card that carries at least one post-cutoff review, each the card's `(grade, day)` reviews in the
/// canonical total order — the raw material [`crate::scheduling::optimise`] turns into a training
/// corpus.
///
/// Unlike [`replay`] this takes **no** `current_cards`: an optimisation run fits the collection's own
/// review history, and a card the current content no longer generates still had genuine recalls that
/// inform the fit — so a dormant card's reviews are included, where projection would drop them
/// (ADR-0002 §7). Reviews before a `history-cutoff-set` are disowned exactly as in replay (ADR-0004
/// §1), and an out-of-range grade is skipped rather than aborting. The cards come back in
/// [`CardRef`]-encoding order so the corpus is reproducible.
pub fn training_histories(lines: &[&str]) -> Vec<Vec<Review>> {
    let (ordered, cutoff) = ordered_log(lines);
    let mut by_card: HashMap<CardRef, Vec<Review>> = HashMap::new();
    for row in &ordered {
        let Row::Reviewed(rev) = row else { continue };
        if let Some(cut) = cutoff
            && rev.day < cut
        {
            continue;
        }
        let Some(grade) = Grade::from_raw(rev.grade) else {
            continue;
        };
        by_card.entry(rev.card).or_default().push(Review {
            grade,
            day: rev.day,
        });
    }
    let mut cards: Vec<CardRef> = by_card.keys().copied().collect();
    cards.sort_by_key(|card| card.encode());
    cards
        .into_iter()
        .map(|card| by_card.remove(&card).expect("key just listed"))
        .collect()
}

/// The fact the settings nudge states (ADR-0014 §2). Two shapes, and no third — the nudge carries no
/// threshold, no floor and no verb, so this type carries only the counts it reports and the app turns
/// them into the one sentence each. **Absence of a parameter row is the distinction**, not a
/// default-valued one (ADR-0004 §6): a collection that has never run the optimiser is [`Standard`],
/// even though its effective vector is the published default.
///
/// [`Standard`]: OptimisationNudge::Standard
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptimisationNudge {
    /// No optimisation run has ever written a parameter row: the collection uses the published
    /// defaults. Carries the total reviews, the one honest number there is to state.
    Standard { reviews_total: u64 },
    /// A run has fitted a vector. Carries its **frozen** fitted-over count and how many reviews have
    /// happened since — `reviews_total − fitted_over`, which is exactly the stale-fit signal a device
    /// that trained while behind exposes (ADR-0014 §6): the count read off the row lags the merged
    /// log, so `reviews_since` runs large and the user re-runs, which is the correcting action.
    Fitted {
        fitted_over: u64,
        reviews_since: u64,
    },
}

/// What the settings nudge should say, derived from the log (ADR-0014 §2). Reads the **latest**
/// parameter row's frozen fitted-over count — never derived by counting rows around it, which after a
/// merge reports a fit that never happened (ADR-0014 §6, scheduling `CONTEXT.md`) — and the total
/// reviews the same way [`training_histories`] counts them, so the two numbers share a unit.
pub fn optimisation_nudge(lines: &[&str]) -> OptimisationNudge {
    let reviews_total: u64 = training_histories(lines)
        .iter()
        .map(|history| history.len() as u64)
        .sum();
    let (ordered, _cutoff) = ordered_log(lines);
    let latest_fit = ordered.iter().rev().find_map(|row| match row {
        Row::ConfigSet(cfg) => match &cfg.setting {
            Setting::SchedulerParameters { fitted_over, .. } => Some(*fitted_over),
            Setting::Other(_) => None,
        },
        _ => None,
    });
    match latest_fit {
        None => OptimisationNudge::Standard { reviews_total },
        Some(fitted_over) => OptimisationNudge::Fitted {
            fitted_over,
            reviews_since: reviews_total.saturating_sub(fitted_over),
        },
    }
}

/// One card that has crossed the leech floor, with the cost that ranks it (ADR-0010 §2, §4).
///
/// **Derived, never stored** — read out of replayed history and fed nowhere back into it, so no leech
/// signal reaches memory state (ADR-0010 §1, §3). It is self-clearing: learn the card and its failure
/// days age out of the window, and it leaves the list with nobody doing anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leech {
    pub card: CardRef,
    /// Distinct failure days inside the trailing window — the count that crossed the floor, and the
    /// primary rank key (worst first).
    pub failure_days: u32,
    /// The most recent failure day inside the window, the recency half of the rank.
    pub last_failure_day: i64,
}

/// The cards that have crossed the leech floor, **ranked by recent failure cost, worst first** — a
/// list ordered rather than cut (ADR-0010 §4), so there is no bright line to defend and a healthy
/// collection's worst card is still below the floor and absent.
///
/// A leech is a card with at least [`LEECH_FAILURE_DAYS`] **failure days** in the trailing
/// [`LEECH_WINDOW_DAYS`], measured with `today` — the **device-local** day — as the window's right
/// edge (ADR-0010 §2). The failure days themselves are the frozen collection-scale day numbers
/// [`replay`] recorded ([`CardState::failure_days`]); replay never reads the edge, so the window is
/// applied here where the caller supplies it (replay `CONTEXT.md`).
///
/// This never touches suspension: leech-ness is pure history, so a suspended card is still a leech if
/// its record says so (its permanent home on the leech screen is ADR-0010 §8's, decided by the
/// caller). FSRS difficulty is **not** consulted — the load-bearing rejection (ADR-0010 §3): binding
/// the surface to a scheduler parameter would re-couple what the design keeps swappable.
pub fn leeches(replayed: &Replayed, today: i64) -> Vec<Leech> {
    // The window is the LEECH_WINDOW_DAYS ending at `today` inclusive: a failure day counts when it
    // falls on or after `today - (window - 1)` and on or before `today`. A row dated ahead of today
    // (clock skew) is outside it and does not count.
    let earliest = today - (LEECH_WINDOW_DAYS - 1);
    let mut out: Vec<Leech> = replayed
        .cards
        .iter()
        .filter_map(|(card, state)| {
            let in_window: Vec<i64> = state
                .failure_days
                .iter()
                .copied()
                .filter(|&d| d >= earliest && d <= today)
                .collect();
            if (in_window.len() as u32) < LEECH_FAILURE_DAYS {
                return None;
            }
            Some(Leech {
                card: *card,
                failure_days: in_window.len() as u32,
                last_failure_day: *in_window
                    .last()
                    .expect("at least LEECH_FAILURE_DAYS entries"),
            })
        })
        .collect();

    // Ranked worst first (ADR-0010 §4): most failure days, then most recent failure, then a stable
    // card-identity tie-break so two devices reach the same order without communicating.
    out.sort_by(|a, b| {
        b.failure_days
            .cmp(&a.failure_days)
            .then_with(|| b.last_failure_day.cmp(&a.last_failure_day))
            .then_with(|| a.card.encode().cmp(&b.card.encode()))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{BASIC, KindDefinition, NoteId};

    fn note(byte: u8) -> NoteId {
        NoteId([byte; 16])
    }

    fn current_cards(notes: &[NoteId]) -> HashSet<CardRef> {
        let mut set = HashSet::new();
        for n in notes {
            for card in BASIC.generated_cards(*n) {
                set.insert(card);
            }
        }
        set
    }

    /// Build a `reviewed` interchange line by hand — `cairn-core` writes no interchange form
    /// (ADR-0004 §11), so tests construct the bytes directly.
    fn rev(writer: &str, seq: u64, note: NoteId, ord: u16, grade: u8, day: i64) -> String {
        // The instant tie-break is derived from the day so ordering is well-defined in these tests.
        format!(
            r#"{{"k":"rev","w":"{}","s":{},"n":"{}","o":{},"g":{},"t":"day-{:08}","d":{},"ms":1000}}"#,
            writer,
            seq,
            note.to_canonical(),
            ord,
            grade,
            day,
            day
        )
    }

    /// A `config-set` params line carrying a fitted-over count, built by hand.
    fn cfg_params(writer: &str, seq: u64, day: i64, fitted_over: u64) -> String {
        let mut v = String::from("[");
        for i in 0..crate::scheduling::PARAMETER_COUNT {
            if i > 0 {
                v.push(',');
            }
            v.push_str("0.3");
        }
        v.push(']');
        format!(
            r#"{{"k":"cfg","w":"{writer}","s":{seq},"t":"day-{day:08}","d":{day},"set":"params","v":{v},"fov":{fitted_over}}}"#
        )
    }

    #[test]
    fn training_histories_group_each_cards_reviews_in_order() {
        // ADR-0014 §1: one entry per card, its reviews in the canonical total order — the corpus the
        // optimiser fits. Two cards, interleaved across two writers, must come back grouped.
        let a = note(1);
        let b = note(2);
        let lines = [
            rev("L", 1, a, 0, 3, 0),
            rev("P", 1, b, 0, 2, 1),
            rev("L", 2, a, 0, 4, 5),
            rev("P", 2, b, 0, 3, 6),
            rev("L", 3, a, 0, 3, 12),
        ];
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        let histories = training_histories(&refs);

        assert_eq!(histories.len(), 2, "two cards, two histories");
        let total: usize = histories.iter().map(Vec::len).sum();
        assert_eq!(total, 5, "every review is present exactly once");
        // Each card's history is day-ascending.
        for history in &histories {
            assert!(history.windows(2).all(|w| w[0].day <= w[1].day));
        }
    }

    #[test]
    fn training_histories_include_dormant_cards_but_honour_the_cutoff() {
        // The corpus is the whole review history (ADR-0014 §1): a card the current content no longer
        // generates still contributes, unlike projection — but rows before a history cutoff are
        // disowned exactly as in replay (ADR-0004 §1). No `current_cards` filter exists here.
        let n = note(1);
        let lines = [
            rev("w", 1, n, 0, 3, 0), // disowned by the cutoff below
            r#"{"k":"cut","w":"w","s":2,"t":"day-00000005","d":5}"#.to_string(),
            rev("w", 3, n, 0, 3, 10),
            rev("w", 4, n, 0, 2, 18),
            rev("w", 5, n, 7, 3, 20), // slot 7: `basic` never generates it — still trained on
        ];
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        let histories = training_histories(&refs);

        let total: usize = histories.iter().map(Vec::len).sum();
        assert_eq!(
            total, 3,
            "two post-cutoff reviews on slot 0, one on dormant slot 7"
        );
        assert_eq!(histories.len(), 2, "the dormant card is its own history");
    }

    #[test]
    fn the_nudge_is_standard_until_a_run_writes_a_parameter_row() {
        // ADR-0014 §2: absence of a parameter row — not a default-valued one — is "using the standard
        // parameters", and the count is the total reviews.
        let n = note(1);
        let lines = [
            rev("w", 1, n, 0, 3, 0),
            rev("w", 2, n, 0, 3, 4),
            rev("w", 3, n, 0, 4, 11),
        ];
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        assert_eq!(
            optimisation_nudge(&refs),
            OptimisationNudge::Standard { reviews_total: 3 }
        );
    }

    #[test]
    fn the_nudge_reads_the_frozen_fit_and_the_reviews_since() {
        // ADR-0014 §6: the fitted-over count is read off the latest parameter row, frozen, and
        // "reviews since" is the total now minus it — the stale-fit signal.
        let n = note(1);
        let lines = [
            rev("w", 1, n, 0, 3, 0),
            rev("w", 2, n, 0, 3, 4),
            // A run fitted over 2 reviews landed on day 5.
            cfg_params("w", 3, 5, 2),
            // Three more reviews since.
            rev("w", 4, n, 0, 3, 11),
            rev("w", 5, n, 0, 2, 20),
            rev("w", 6, n, 0, 3, 33),
        ];
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        assert_eq!(
            optimisation_nudge(&refs),
            OptimisationNudge::Fitted {
                fitted_over: 2,
                reviews_since: 3, // 5 reviews total − 2 fitted-over
            }
        );
    }

    #[test]
    fn the_nudge_takes_the_latest_of_several_fits() {
        // The current vector is the last parameter row in the total order; its count is the one shown.
        let n = note(1);
        let lines = [
            rev("w", 1, n, 0, 3, 0),
            cfg_params("w", 2, 1, 1),
            rev("w", 3, n, 0, 3, 4),
            cfg_params("w", 4, 5, 2), // the later fit wins
        ];
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        let OptimisationNudge::Fitted { fitted_over, .. } = optimisation_nudge(&refs) else {
            panic!("a fit was written");
        };
        assert_eq!(fitted_over, 2, "the latest fit's frozen count");
    }

    #[test]
    fn a_single_card_projects_its_memory_state_and_box() {
        let n = note(1);
        let cards = current_cards(&[n]);
        let lines = [
            rev("w", 1, n, 0, 3, 0),
            rev("w", 2, n, 0, 3, 3),
            rev("w", 3, n, 0, 4, 12),
        ];
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        let out = replay(&cards, &refs);

        let card = CardRef::new(n, 0);
        let state = out.cards.get(&card).expect("the card should be projected");
        assert_eq!(state.review_count, 3);
        assert_eq!(state.last_day, 12);
        assert!(state.box_ >= 1);
        assert!(
            state.due_day >= state.last_day,
            "the next schedule is not in the past"
        );
    }

    #[test]
    fn a_row_naming_no_current_card_is_retained_but_not_projected() {
        // ADR-0002 §7: card retirement does not exist. A row for a card the current content does not
        // generate is not projected and does not error.
        let present = note(1);
        let cards = current_cards(&[present]); // only `present`'s slot 0 exists
        let absent = note(2);
        let lines = [
            rev("w", 1, present, 0, 3, 0),
            rev("w", 2, absent, 0, 3, 0), // dormant: no such note in current content
            rev("w", 3, present, 7, 3, 0), // dormant: `basic` never generates slot 7
        ];
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        let out = replay(&cards, &refs);

        assert!(out.cards.contains_key(&CardRef::new(present, 0)));
        assert!(!out.cards.contains_key(&CardRef::new(absent, 0)));
        assert!(!out.cards.contains_key(&CardRef::new(present, 7)));
        assert_eq!(out.cards.len(), 1);
    }

    #[test]
    fn any_interleaving_replays_to_the_same_state() {
        // The highest-value test in the repository (ADR-0004 §2, ADR-0009 §8): merge is set union
        // with duplicates dropped, so any interleaving of two devices' rows must replay identically.
        let a = note(1);
        let b = note(2);
        let cards = current_cards(&[a, b]);

        // Two writers, interleaved days.
        let device_l = [
            rev("L", 1, a, 0, 3, 0),
            rev("L", 2, a, 0, 2, 5),
            rev("L", 3, b, 0, 4, 6),
        ];
        let device_p = [
            rev("P", 1, b, 0, 3, 1),
            rev("P", 2, a, 0, 4, 9),
            rev("P", 3, b, 0, 1, 12),
        ];

        let mut all: Vec<String> = Vec::new();
        all.extend(device_l.iter().cloned());
        all.extend(device_p.iter().cloned());

        let refs: Vec<&str> = all.iter().map(String::as_str).collect();
        let baseline = replay(&cards, &refs);

        // Several deterministic re-orderings, including duplicates dropped by set semantics. We do
        // not use an RNG — that would make the test itself non-reproducible.
        let orderings: [Vec<usize>; 4] = [
            vec![5, 4, 3, 2, 1, 0],
            vec![0, 3, 1, 4, 2, 5],
            vec![3, 0, 4, 1, 5, 2],
            // A duplicate row (same writer+seq) must not change the result.
            vec![0, 1, 2, 3, 4, 5, 0, 3],
        ];
        for order in &orderings {
            let shuffled: Vec<&str> = order.iter().map(|&i| all[i].as_str()).collect();
            assert_eq!(
                replay(&cards, &shuffled),
                baseline,
                "interleaving {order:?} diverged"
            );
        }
    }

    #[test]
    fn every_permutation_of_two_devices_rows_replays_identically() {
        // The claim the whole sync design rests on (ADR-0004 §2, replay `CONTEXT.md`), pinned by
        // brute force rather than a handful of hand-picked orderings: merge is set union with
        // duplicates dropped, so *every* interleaving of two devices' rows — all 720 permutations of
        // six — must replay to one state. No RNG: exhaustion is both stronger and reproducible.
        let a = note(1);
        let b = note(2);
        let cards = current_cards(&[a, b]);
        let all = [
            rev("L", 1, a, 0, 3, 0),
            rev("L", 2, a, 0, 2, 5),
            rev("L", 3, b, 0, 4, 6),
            rev("P", 1, b, 0, 3, 1),
            rev("P", 2, a, 0, 4, 9),
            rev("P", 3, b, 0, 1, 12),
        ];
        let baseline = replay(&cards, &all.iter().map(String::as_str).collect::<Vec<_>>());

        let mut indices = [0, 1, 2, 3, 4, 5];
        let mut seen: HashSet<[usize; 6]> = HashSet::new();
        let len = indices.len();
        permutations(&mut indices, len, &mut |order| {
            seen.insert(order.try_into().expect("six indices"));
            let shuffled: Vec<&str> = order.iter().map(|&i| all[i].as_str()).collect();
            assert_eq!(
                replay(&cards, &shuffled),
                baseline,
                "permutation {order:?} diverged"
            );
            // The same rows with an arbitrary duplicate appended must not change the result either.
            let mut with_dup = shuffled.clone();
            with_dup.push(all[order[0]].as_str());
            assert_eq!(
                replay(&cards, &with_dup),
                baseline,
                "a duplicate row changed the result"
            );
        });
        assert_eq!(
            seen.len(),
            720,
            "all 6! distinct interleavings must have been checked"
        );
    }

    /// Heap's algorithm — every permutation of the first `k` elements of `slice`, handed to `emit`
    /// exactly once. Kept local to the test: exhaustive interleaving is the one property here worth
    /// proving without leaning on a crate.
    fn permutations(slice: &mut [usize], k: usize, emit: &mut impl FnMut(&[usize])) {
        if k <= 1 {
            emit(slice);
            return;
        }
        for i in 0..k {
            permutations(slice, k - 1, emit);
            if k.is_multiple_of(2) {
                slice.swap(i, k - 1);
            } else {
                slice.swap(0, k - 1);
            }
        }
    }

    #[test]
    fn rows_before_a_history_cutoff_are_ignored() {
        // ADR-0004 §1: replay ignores every reviewed row before the cutoff's frozen day.
        let n = note(1);
        let cards = current_cards(&[n]);

        let with_old = [
            rev("w", 1, n, 0, 1, 0), // disowned
            rev("w", 2, n, 0, 1, 5), // disowned
            r#"{"k":"cut","w":"w","s":3,"t":"day-00000010","d":10}"#.to_string(),
            rev("w", 4, n, 0, 3, 20),
            rev("w", 5, n, 0, 3, 30),
        ];
        let refs: Vec<&str> = with_old.iter().map(String::as_str).collect();
        let cut = replay(&cards, &refs);

        // The same log with only the post-cutoff reviews must give the same state.
        let only_new = [rev("w", 4, n, 0, 3, 20), rev("w", 5, n, 0, 3, 30)];
        let refs2: Vec<&str> = only_new.iter().map(String::as_str).collect();
        let fresh = replay(&cards, &refs2);

        let card = CardRef::new(n, 0);
        assert_eq!(
            cut.cards.get(&card).unwrap().memory,
            fresh.cards.get(&card).unwrap().memory,
            "disowned rows must not influence the projected state"
        );
        assert_eq!(cut.cards.get(&card).unwrap().review_count, 2);
    }

    #[test]
    fn a_config_set_changes_parameters_from_that_point_forward() {
        // #78: config-set rows change parameters going forward. A vector with much larger initial
        // stabilities produces a different memory state for reviews that follow it.
        let n = note(1);
        let cards = current_cards(&[n]);

        // A config-set landing on day 4 that raises w8 — the success-step lever every inter-day
        // pass uses (`stability_after_success` reads `w[8].exp()`), so it changes the day-8 review
        // even though that review is not the card's first.
        let mut inflated = *SchedulerParameters::default().weights();
        inflated[8] += 2.0;
        let mut v = String::from("[");
        for (i, w) in inflated.iter().enumerate() {
            if i > 0 {
                v.push(',');
            }
            v.push_str(&format!("{w}"));
        }
        v.push(']');
        let cfg = format!(
            r#"{{"k":"cfg","w":"w","s":2,"t":"day-00000004","d":4,"set":"params","v":{v}}}"#
        );

        let with_cfg = [
            rev("w", 1, n, 0, 3, 0),
            cfg.clone(),
            rev("w", 3, n, 0, 3, 8),
        ];
        let without_cfg = [rev("w", 1, n, 0, 3, 0), rev("w", 3, n, 0, 3, 8)];

        let refs_a: Vec<&str> = with_cfg.iter().map(String::as_str).collect();
        let refs_b: Vec<&str> = without_cfg.iter().map(String::as_str).collect();

        let card = CardRef::new(n, 0);
        let a = replay(&cards, &refs_a).cards.remove(&card).unwrap();
        let b = replay(&cards, &refs_b).cards.remove(&card).unwrap();
        assert_ne!(
            a.memory, b.memory,
            "a config-set must change the arithmetic for reviews after it"
        );
    }

    #[test]
    fn malformed_and_unknown_lines_do_not_abort_or_perturb_replay() {
        // ADR-0004 §11: a malformed line never aborts replay; an unknown kind is skipped.
        let n = note(1);
        let cards = current_cards(&[n]);

        let clean = [rev("w", 1, n, 0, 3, 0), rev("w", 2, n, 0, 3, 4)];
        let dirty = [
            rev("w", 1, n, 0, 3, 0),
            "this is not json".to_string(),
            r#"{"k":"future","w":"w","s":99,"blob":[1,2,3]}"#.to_string(),
            "{".to_string(),
            rev("w", 2, n, 0, 3, 4),
        ];
        let refs_clean: Vec<&str> = clean.iter().map(String::as_str).collect();
        let refs_dirty: Vec<&str> = dirty.iter().map(String::as_str).collect();

        assert_eq!(
            replay(&cards, &refs_clean),
            replay(&cards, &refs_dirty),
            "noise lines must be skipped without changing the result"
        );
    }

    #[test]
    fn an_empty_log_projects_nothing() {
        let cards = current_cards(&[note(1)]);
        assert_eq!(replay(&cards, &[]).cards.len(), 0);
    }

    /// A `NewCard` for slot `ord` of `n` at authored `position`.
    fn new_card(n: NoteId, ord: u16, position: &str) -> NewCard {
        NewCard {
            card: CardRef::new(n, ord),
            position: position.to_owned(),
        }
    }

    #[test]
    fn introductions_are_taken_in_position_order_up_to_the_rate() {
        // ADR-0011 §8: candidates are taken in (position, ordinal) order up to the rate. Three notes
        // authored c, a, b — a rate of two takes the first two in *position* order, not input order.
        let (a, b, c) = (note(1), note(2), note(3));
        let new = [
            new_card(c, 0, "c"),
            new_card(a, 0, "a"),
            new_card(b, 0, "b"),
        ];
        let chosen = introduction_candidates(&new, &HashSet::new(), &HashSet::new(), 2);
        assert_eq!(
            chosen,
            vec![CardRef::new(a, 0), CardRef::new(b, 0)],
            "position order, capped at the rate"
        );
    }

    #[test]
    fn at_most_one_card_per_note_is_introduced() {
        // ADR-0011 §8: a note's two siblings are never both offered the same day — the second
        // measures ninety-second recall, not the separate skill it exists to schedule. One note with
        // two cards, a generous rate: exactly one card comes back.
        let n = note(1);
        let new = [new_card(n, 0, "a"), new_card(n, 1, "a")];
        let chosen = introduction_candidates(&new, &HashSet::new(), &HashSet::new(), 5);
        assert_eq!(
            chosen,
            vec![CardRef::new(n, 0)],
            "the lower slot, and only it"
        );
    }

    #[test]
    fn a_note_already_introduced_today_is_skipped() {
        // ADR-0011 §8: a note that already had a card introduced today is skipped — this is how a
        // reverse sibling waits until tomorrow. `a` was introduced today; only `b` is offered.
        let (a, b) = (note(1), note(2));
        let new = [new_card(a, 0, "a"), new_card(b, 0, "b")];
        let introduced = HashSet::from([a]);
        let chosen = introduction_candidates(&new, &introduced, &HashSet::new(), 5);
        assert_eq!(chosen, vec![CardRef::new(b, 0)]);
    }

    #[test]
    fn a_suspended_card_is_skipped_and_does_not_consume_its_notes_slot() {
        // ADR-0011 §8, ADR-0010 §8: a suspended card is skipped and never counted; a note keeps its
        // one introduction when only a sibling is suspended.
        let n = note(1);
        let new = [new_card(n, 0, "a"), new_card(n, 1, "a")];
        let suspended = HashSet::from([CardRef::new(n, 0)]);
        let chosen = introduction_candidates(&new, &HashSet::new(), &suspended, 5);
        assert_eq!(
            chosen,
            vec![CardRef::new(n, 1)],
            "the un-suspended sibling is still introduced"
        );
    }

    #[test]
    fn cards_already_introduced_today_spend_the_daily_rate() {
        // ADR-0011 §5, §8: the rate is a daily total. With a rate of two and one note already
        // introduced today, only one more card may be introduced — never a second full rate.
        let already = note(1);
        let (b, c) = (note(2), note(3));
        let new = [new_card(b, 0, "b"), new_card(c, 0, "c")];
        let introduced = HashSet::from([already]);
        let chosen = introduction_candidates(&new, &introduced, &HashSet::new(), 2);
        assert_eq!(
            chosen,
            vec![CardRef::new(b, 0)],
            "one of the daily budget of two was already spent"
        );
    }

    #[test]
    fn a_rate_already_met_today_introduces_nothing_more() {
        // ADR-0011 §5: a device that has met its cap earlier today offers no further new cards.
        let new = [new_card(note(5), 0, "e")];
        let introduced = HashSet::from([note(1), note(2)]);
        assert!(
            introduction_candidates(&new, &introduced, &HashSet::new(), 2).is_empty(),
            "the daily rate of two is already spent"
        );
    }

    #[test]
    fn a_rate_of_zero_introduces_nothing() {
        // ADR-0011 §3: zero is legal and is the backlog escape hatch — no card is introduced.
        let new = [new_card(note(1), 0, "a"), new_card(note(2), 0, "b")];
        assert!(introduction_candidates(&new, &HashSet::new(), &HashSet::new(), 0).is_empty());
    }

    #[test]
    fn equal_positions_break_the_tie_by_note_id() {
        // ADR-0011 §7: "need not be unique — ties broken by note id", deterministic on every device.
        // Two notes share a position; the lower note id wins the earlier slot.
        let (low, high) = (note(1), note(9));
        let new = [new_card(high, 0, "m"), new_card(low, 0, "m")];
        let chosen = introduction_candidates(&new, &HashSet::new(), &HashSet::new(), 1);
        assert_eq!(chosen, vec![CardRef::new(low, 0)]);
    }

    #[test]
    fn introduced_today_is_the_earliest_review_day_against_the_local_day() {
        // ADR-0011 §5, §8: a note is "introduced today" when its card's *earliest* projected review
        // falls on the device-local day — not its latest. A card first seen on day 5 and re-shown on
        // day 5, then reviewed again on day 9, is introduced on day 5, never day 9.
        let n = note(1);
        let cards = current_cards(&[n]);
        let lines = [
            rev("w", 1, n, 0, 1, 5),
            rev("w", 2, n, 0, 3, 5),
            rev("w", 3, n, 0, 3, 9),
        ];
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        let replayed = replay(&cards, &refs);

        assert_eq!(replayed.cards[&CardRef::new(n, 0)].first_day, 5);
        assert_eq!(notes_introduced_today(&replayed, 5), HashSet::from([n]));
        assert!(
            notes_introduced_today(&replayed, 9).is_empty(),
            "the introduction day is the earliest review, not the latest"
        );
    }

    #[test]
    fn last_grade_tracks_the_most_recent_projected_review() {
        // ADR-0011 §9: the session queue keeps a lapsed card and drops a passed one, which needs the
        // latest grade. A card passed then failed reads its latest as the failure.
        let n = note(1);
        let cards = current_cards(&[n]);
        let passed_then_failed = [rev("w", 1, n, 0, 3, 0), rev("w", 2, n, 0, 1, 0)];
        let refs: Vec<&str> = passed_then_failed.iter().map(String::as_str).collect();
        let state = replay(&cards, &refs)
            .cards
            .remove(&CardRef::new(n, 0))
            .unwrap();
        assert!(
            state.last_grade.is_failure(),
            "the latest projected grade is the failure"
        );
    }

    #[test]
    fn four_failure_days_in_the_window_make_a_leech_but_three_do_not() {
        // ADR-0010 §2: a leech is four or more failure *days* in the trailing ninety. Three fail on
        // separate days — below the floor; a fourth crosses it.
        let three = note(1);
        let four = note(2);
        let cards = current_cards(&[three, four]);
        let lines = [
            rev("w", 1, three, 0, 1, 10),
            rev("w", 2, three, 0, 1, 20),
            rev("w", 3, three, 0, 1, 30),
            rev("w", 4, four, 0, 1, 10),
            rev("w", 5, four, 0, 1, 20),
            rev("w", 6, four, 0, 1, 30),
            rev("w", 7, four, 0, 1, 40),
        ];
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        let replayed = replay(&cards, &refs);
        let leeched = leeches(&replayed, 40);

        assert_eq!(
            leeched.iter().map(|l| l.card).collect::<Vec<_>>(),
            vec![CardRef::new(four, 0)],
            "only the card with four failure days crosses the floor"
        );
        assert_eq!(leeched[0].failure_days, 4);
    }

    #[test]
    fn same_day_re_shows_count_as_one_failure_day() {
        // ADR-0010 §2: a failure *day* is distinct — three grade-1 rows one evening (a fumbled new
        // card and its same-session re-shows) are one act of forgetting, never three. Four rows on
        // one day must not make a leech.
        let n = note(1);
        let cards = current_cards(&[n]);
        let lines = [
            rev("w", 1, n, 0, 1, 5),
            rev("w", 2, n, 0, 1, 5),
            rev("w", 3, n, 0, 1, 5),
            rev("w", 4, n, 0, 1, 5),
        ];
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        let replayed = replay(&cards, &refs);

        assert_eq!(
            replayed.cards[&CardRef::new(n, 0)].failure_days,
            vec![5],
            "one distinct day, however many rows"
        );
        assert!(
            leeches(&replayed, 5).is_empty(),
            "four rows in one day is one episode, not a leech"
        );
    }

    #[test]
    fn failures_outside_the_trailing_window_do_not_count() {
        // ADR-0010 §2: the window is the trailing ninety with the device-local day as its right edge,
        // and it is self-clearing. Four failure days on days 3..=6: at day 92 the earliest edge is
        // day 3, so all four are in and the card is a leech; one day later day 3 has aged out, so only
        // three remain and the leech self-clears — no stored state cleared it.
        let n = note(1);
        let cards = current_cards(&[n]);
        let lines = [
            rev("w", 1, n, 0, 1, 3),
            rev("w", 2, n, 0, 1, 4),
            rev("w", 3, n, 0, 1, 5),
            rev("w", 4, n, 0, 1, 6),
        ];
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        let replayed = replay(&cards, &refs);

        assert_eq!(
            leeches(&replayed, 6).len(),
            1,
            "all four days are today-or-recent"
        );
        // Day 92: earliest = 92 - 89 = 3, so the day-3 failure sits on the inclusive edge — still a
        // leech at exactly ninety days.
        assert_eq!(
            leeches(&replayed, 92).len(),
            1,
            "day 3 sits on the window's earliest edge"
        );
        // Day 93: earliest = 4, day 3 has fallen out — three failure days left, below the floor.
        assert!(
            leeches(&replayed, 93).is_empty(),
            "the oldest failure has aged out of the trailing ninety and the leech self-clears"
        );
    }

    #[test]
    fn the_list_is_ranked_by_failure_days_then_recency() {
        // ADR-0010 §4: the list is ranked, not filtered — worst first. The card with more failure days
        // outranks one with fewer; ties break to the more recent failure.
        let worst = note(1);
        let middle = note(2);
        let recent = note(3);
        let cards = current_cards(&[worst, middle, recent]);
        let mut lines = Vec::new();
        // worst: five failure days.
        for (i, day) in [10, 20, 30, 40, 50].into_iter().enumerate() {
            lines.push(rev("w", 1 + i as u64, worst, 0, 1, day));
        }
        // middle: four failure days, latest on day 45.
        for (i, day) in [12, 22, 32, 45].into_iter().enumerate() {
            lines.push(rev("w", 10 + i as u64, middle, 0, 1, day));
        }
        // recent: four failure days, latest on day 60 — more recent than middle.
        for (i, day) in [15, 25, 35, 60].into_iter().enumerate() {
            lines.push(rev("w", 20 + i as u64, recent, 0, 1, day));
        }
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        let replayed = replay(&cards, &refs);
        let ranked = leeches(&replayed, 60);

        assert_eq!(
            ranked.iter().map(|l| l.card).collect::<Vec<_>>(),
            vec![
                CardRef::new(worst, 0),  // five failure days
                CardRef::new(recent, 0), // four, most recent failure day 60
                CardRef::new(middle, 0), // four, most recent failure day 45
            ],
            "most failure days first, ties broken by recency"
        );
    }

    #[test]
    fn a_passed_card_is_never_a_leech_and_the_difficulty_is_never_read() {
        // ADR-0010 §1, §3: nothing but failure days drives the list. A card passed every time — however
        // hard the scheduler thinks it — is not a leech.
        let n = note(1);
        let cards = current_cards(&[n]);
        let lines = [
            rev("w", 1, n, 0, 3, 0),
            rev("w", 2, n, 0, 3, 5),
            rev("w", 3, n, 0, 3, 10),
            rev("w", 4, n, 0, 3, 20),
            rev("w", 5, n, 0, 3, 40),
        ];
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        let replayed = replay(&cards, &refs);
        assert!(
            leeches(&replayed, 40).is_empty(),
            "a card the user keeps passing is not a leech, whatever its difficulty"
        );
    }

    #[test]
    fn a_history_cutoff_clears_the_failures_before_it() {
        // ADR-0010 §5: a cutoff makes replay ignore earlier reviews, so failures before it stop
        // counting and the leech clears. Four failure days, a cutoff after three of them, leaves one.
        let n = note(1);
        let cards = current_cards(&[n]);
        let lines = [
            rev("w", 1, n, 0, 1, 10),
            rev("w", 2, n, 0, 1, 20),
            rev("w", 3, n, 0, 1, 30),
            r#"{"k":"cut","w":"w","s":4,"t":"day-00000035","d":35}"#.to_string(),
            rev("w", 5, n, 0, 1, 40),
        ];
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        let replayed = replay(&cards, &refs);
        assert_eq!(
            replayed.cards[&CardRef::new(n, 0)].failure_days,
            vec![40],
            "only the post-cutoff failure survives"
        );
        assert!(leeches(&replayed, 40).is_empty());
    }

    #[test]
    fn siblings_generated_from_the_same_note_track_separately() {
        // A hypothetical two-card kind: slot 0 and slot 1 from one note are different cards with
        // independent histories (ADR-0002 §1). `basic` ships one slot, so this uses an ad-hoc
        // definition to prove replay keys strictly on the whole `CardRef`.
        const PAIR: KindDefinition = KindDefinition {
            id: "pair",
            fields: &[],
            cards: &[
                crate::content::CardTemplate {
                    slot: 0,
                    prompt: &[],
                    answer: &[],
                },
                crate::content::CardTemplate {
                    slot: 1,
                    prompt: &[],
                    answer: &[],
                },
            ],
        };
        let n = note(1);
        let cards: HashSet<CardRef> = PAIR.generated_cards(n).into_iter().collect();
        let lines = [
            rev("w", 1, n, 0, 3, 0),
            rev("w", 2, n, 1, 1, 0),
            rev("w", 3, n, 0, 3, 5),
        ];
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        let out = replay(&cards, &refs);

        assert_eq!(out.cards.get(&CardRef::new(n, 0)).unwrap().review_count, 2);
        assert_eq!(out.cards.get(&CardRef::new(n, 1)).unwrap().review_count, 1);
        // The failed sibling is in a lower box than the twice-passed one.
        assert!(out.cards[&CardRef::new(n, 1)].box_ <= out.cards[&CardRef::new(n, 0)].box_);
    }
}
