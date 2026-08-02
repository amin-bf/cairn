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
use crate::scheduling::{Grade, MemoryState, Scheduler, SchedulerParameters, box_of};

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
                if let Setting::SchedulerParameters(weights) = &cfg.setting {
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
                            },
                        );
                    }
                    Some(existing) => {
                        let delta_t = crate::scheduling::day_gap(existing.last_day, rev.day);
                        existing.state = scheduler.advance(Some(existing.state), grade, delta_t);
                        existing.last_grade = grade;
                        existing.last_day = rev.day;
                        existing.count += 1;
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

    /// Build a `reviewed` interchange line by hand — `leitner-core` writes no interchange form
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
