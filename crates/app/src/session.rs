//! The review session, as logic the rest of the app draws a screen around. **Not a domain object**
//! (ADR-0005 §6): a session exists only here, and its defining property for issue #94 is that its
//! **position is never stored — only derived from the log**. Grading a card writes a `reviewed` row
//! that moves the card's due day into the future, so re-deriving the queue from the log after a
//! force-quit simply no longer offers it. There is nothing to persist and nothing to resume.
//!
//! Everything in this module is pure: it takes the current card set and a [`Replayed`] projection
//! (both from `leitner-core`) plus the **device-local day**, and returns what to offer. That day is
//! the edge value replay refuses to read (replay `CONTEXT.md`): "due today" is measured against the
//! device's local day, never the collection day scale.

use std::collections::HashSet;

use leitner_core::content::CardRef;
use leitner_core::replay::Replayed;
use leitner_core::scheduling::{Grade, MemoryState, Scheduler, SchedulerParameters, day_gap};

/// More cards due than a sitting will clear: past this, the count picker **frames** the backlog
/// rather than reporting a bare number (ADR-0001 §3 forbids a due-count presented as a queue; the
/// issue asks for framing, "pick a comfortable size, the rest will keep"). The threshold only
/// decides *whether to reassure*; it is never shown.
pub const COMFORTABLE_SITTING: usize = 20;

/// One card the session may offer, with the two facts the card screen needs — and no more. The box
/// is carried so the badge can be drawn, but the badge appears **only after reveal** and is never
/// sorted, counted or presented as a queue (scheduling `CONTEXT.md`); this struct is not that queue,
/// it is one card at a time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Offered {
    pub card: CardRef,
    /// The durability box, 1–5 (ADR-0001 §3). `1` for a never-reviewed card.
    pub box_: u8,
    /// Whether this card has never been reviewed — a first introduction rather than a due repeat.
    pub is_new: bool,
    /// The card's memory state after its projected history, or `None` if it has never been reviewed.
    /// The interval preview on each grade button is computed from this (ADR-0006's illustrative
    /// preview); it is carried on the offered card so the screen needs no second replay.
    pub memory: Option<MemoryState>,
    /// The frozen day of the card's most recent review, or `today` for a never-reviewed card — the
    /// left edge of the elapsed-time the preview projects forward from.
    pub last_day: i64,
}

/// What the collection looks like to the review screen right now: the cards due today and the cards
/// never yet seen, each already ordered. Derived fresh every time from the log — there is no stored
/// session state to fall out of step with it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Queue {
    /// Cards with a projected state whose scheduled day has arrived, earliest-scheduled first.
    pub due: Vec<Offered>,
    /// Cards the current content generates that carry no reviews yet, in card-identity order.
    pub new: Vec<Offered>,
}

impl Queue {
    /// Total cards a session could draw from: everything due plus everything new.
    pub fn available(&self) -> usize {
        self.due.len() + self.new.len()
    }

    /// The cards a sitting of `count` would show: due first, then new, capped at `count` and at what
    /// exists. This is the **only** bound on the session (ADR-0011 §1): there is no daily review
    /// limit, and the cap is "what the user chose, but never more than is actually due".
    pub fn sitting(&self, count: usize) -> Vec<Offered> {
        self.due
            .iter()
            .chain(self.new.iter())
            .take(count)
            .copied()
            .collect()
    }
}

/// Derive the queue from the current card set and the replayed state, against the device-local
/// `today`.
///
/// A card is **due** if it has a projected state whose `due_day` has arrived; **new** if the content
/// generates it but no review projects onto it; and simply absent if it was reviewed and is not yet
/// due — which is exactly how a just-graded card leaves the session without any stored position
/// (issue #94). Ordering is deterministic so two runs, or a force-quit and relaunch, agree: due by
/// `(due_day, card bytes)`, new by card bytes.
pub fn compose(current: &HashSet<CardRef>, replayed: &Replayed, today: i64) -> Queue {
    let mut due: Vec<(i64, Offered)> = Vec::new();
    let mut new: Vec<Offered> = Vec::new();

    for &card in current {
        match replayed.cards.get(&card) {
            Some(state) if state.due_day <= today => due.push((
                state.due_day,
                Offered {
                    card,
                    box_: state.box_,
                    is_new: false,
                    memory: Some(state.memory),
                    last_day: state.last_day,
                },
            )),
            Some(_) => {} // reviewed but scheduled ahead — not offered, and not stored anywhere
            None => new.push(Offered {
                card,
                box_: 1,
                is_new: true,
                memory: None,
                last_day: today,
            }),
        }
    }

    due.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.card.encode().cmp(&b.1.card.encode()))
    });
    new.sort_by_key(|a| a.card.encode());

    Queue {
        due: due.into_iter().map(|(_, o)| o).collect(),
        new,
    }
}

/// The illustrative next interval, in days, if `offered` were graded `grade` today — the figure the
/// grade button previews (ADR-0006). Un-fuzzed, because a preview is illustrative and the per-card
/// fuzz would only add day-scale jitter the user cannot act on; and computed under the default
/// parameters, which is exact until a `config-set` row installs a custom vector (a later ticket owns
/// threading replay's final parameters through). Always at least one day.
pub fn interval_preview(offered: &Offered, grade: Grade, today: i64) -> u32 {
    let scheduler = Scheduler::new(SchedulerParameters::default());
    let elapsed = day_gap(offered.last_day, today);
    let next = scheduler.advance(offered.memory, grade, elapsed);
    scheduler.next_interval_unfuzzed(next).round().max(1.0) as u32
}

/// The state the review destination is in, chosen so the empty, new-deck and backlog cases are
/// **explicit worded states** rather than a card screen showing nothing (issue #94). `total` is the
/// number of cards the current content generates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewState {
    /// No cards exist at all — an empty collection.
    Empty,
    /// Cards exist but none has ever been reviewed: a brand-new deck. Carries how many are waiting.
    NewDeck { new: usize },
    /// Everything reviewed is scheduled ahead; nothing is due. The reassuring resting state.
    CaughtUp,
    /// Cards are due. `backlog` is set when there are more than a comfortable sitting, so the picker
    /// frames rather than alarms. `new` counts first-time cards also available.
    Due {
        due: usize,
        new: usize,
        backlog: bool,
    },
}

impl ReviewState {
    /// Classify the queue against the total card count.
    pub fn of(queue: &Queue, total: usize) -> ReviewState {
        if total == 0 {
            return ReviewState::Empty;
        }
        if queue.due.is_empty() {
            // Nothing due: a resting state if there is nothing to introduce either, otherwise a
            // deck of fresh cards waiting — whether *all* the cards are new or only some makes no
            // difference to how the picker frames them.
            return if queue.new.is_empty() {
                ReviewState::CaughtUp
            } else {
                ReviewState::NewDeck {
                    new: queue.new.len(),
                }
            };
        }
        ReviewState::Due {
            due: queue.due.len(),
            new: queue.new.len(),
            backlog: queue.due.len() > COMFORTABLE_SITTING,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use leitner_core::content::{BASIC, NoteId};
    use leitner_core::log::DayScale;
    use leitner_core::replay::replay;
    use leitner_core::scheduling::Grade;
    use leitner_store::Collection;
    use tempfile::TempDir;

    fn note(byte: u8) -> NoteId {
        NoteId([byte; 16])
    }

    fn current(notes: &[NoteId]) -> HashSet<CardRef> {
        let mut set = HashSet::new();
        for n in notes {
            for c in BASIC.generated_cards(*n) {
                set.insert(c);
            }
        }
        set
    }

    #[test]
    fn a_brand_new_deck_is_all_new_and_nothing_due() {
        // Cards exist, none reviewed: every card is a new-card introduction (issue #94's new-deck
        // state), and the queue offers them as new, not due.
        let cards = current(&[note(1), note(2)]);
        let queue = compose(&cards, &Replayed::default(), 0);
        assert_eq!(queue.due.len(), 0);
        assert_eq!(queue.new.len(), 2);
        assert_eq!(
            ReviewState::of(&queue, cards.len()),
            ReviewState::NewDeck { new: 2 }
        );
    }

    #[test]
    fn an_empty_collection_is_the_empty_state() {
        let queue = compose(&HashSet::new(), &Replayed::default(), 0);
        assert_eq!(ReviewState::of(&queue, 0), ReviewState::Empty);
    }

    #[test]
    fn a_sitting_is_capped_by_the_chosen_count_and_by_what_exists() {
        let cards = current(&[note(1), note(2), note(3)]);
        let queue = compose(&cards, &Replayed::default(), 0);
        assert_eq!(queue.sitting(2).len(), 2, "capped by the chosen count");
        assert_eq!(queue.sitting(10).len(), 3, "capped by what actually exists");
    }

    #[test]
    fn a_graded_card_leaves_the_session_with_no_stored_position() {
        // The heart of #94: session position is derived from the log. Grade a due card and re-derive
        // — it is gone from the queue, because its due day moved, not because anything was stored.
        let data = TempDir::new().unwrap();
        let state = TempDir::new().unwrap();
        let mut coll = Collection::open(data.path(), state.path()).unwrap();

        let n = note(1);
        let card = CardRef::new(n, 0);
        let cards = current(&[n]);
        const TODAY: i64 = 20_514;
        let day0_ms = TODAY * 86_400_000 + 4 * 3_600_000; // a 4am-scale instant landing on TODAY

        // Introduce and grade the card as reviewed today.
        coll.append_review(card, Grade::Good, day0_ms, DayScale::default(), 1000)
            .unwrap();

        let lines = coll.log_lines().unwrap();
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        let replayed = replay(&cards, &refs);

        // Reviewed today, scheduled ahead: neither due nor new — the session no longer offers it.
        let queue = compose(&cards, &replayed, TODAY);
        assert_eq!(
            queue.available(),
            0,
            "a card graded today is excluded on re-derivation"
        );
        assert_eq!(ReviewState::of(&queue, cards.len()), ReviewState::CaughtUp);
    }

    #[test]
    fn a_due_card_is_offered_again_once_its_scheduled_day_arrives() {
        let data = TempDir::new().unwrap();
        let state = TempDir::new().unwrap();
        let mut coll = Collection::open(data.path(), state.path()).unwrap();
        let n = note(1);
        let card = CardRef::new(n, 0);
        let cards = current(&[n]);
        const TODAY: i64 = 20_514;
        let day0_ms = TODAY * 86_400_000 + 4 * 3_600_000;

        coll.append_review(card, Grade::Good, day0_ms, DayScale::default(), 1000)
            .unwrap();
        let lines = coll.log_lines().unwrap();
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        let replayed = replay(&cards, &refs);

        // Far enough in the future, the scheduled day has passed and it is due once more.
        let queue = compose(&cards, &replayed, TODAY + 3650);
        assert_eq!(queue.due.len(), 1);
        assert!(!queue.due[0].is_new);
        assert!(queue.due[0].box_ >= 1);
    }

    #[test]
    fn a_higher_grade_previews_at_least_as_long_an_interval() {
        // The preview's one invariant worth pinning without a screen: Easy never schedules sooner
        // than Good, which never schedules sooner than Barely (ADR-0001 §2's success ordering).
        let offered = Offered {
            card: CardRef::new(note(1), 0),
            box_: 1,
            is_new: true,
            memory: None,
            last_day: 0,
        };
        let barely = interval_preview(&offered, Grade::Barely, 0);
        let good = interval_preview(&offered, Grade::Good, 0);
        let easy = interval_preview(&offered, Grade::Easy, 0);
        assert!(
            barely <= good,
            "Good must not be shorter than Barely ({barely} vs {good})"
        );
        assert!(
            good <= easy,
            "Easy must not be shorter than Good ({good} vs {easy})"
        );
        assert!(barely >= 1, "every preview is at least a day");
    }

    #[test]
    fn a_large_due_pile_is_flagged_as_backlog() {
        // Build enough due cards to cross the comfortable-sitting threshold; the state frames it.
        let notes: Vec<NoteId> = (0..(COMFORTABLE_SITTING as u8 + 5)).map(note).collect();
        let cards = current(&notes);
        // Force every card due by handing replay a projection with due_day in the past.
        let mut replayed = Replayed::default();
        for &card in &cards {
            replayed.cards.insert(
                card,
                leitner_core::replay::CardState {
                    memory: leitner_core::scheduling::MemoryState {
                        stability: 5.0,
                        difficulty: 5.0,
                    },
                    box_: 2,
                    review_count: 1,
                    last_day: 0,
                    due_day: 1,
                },
            );
        }
        let queue = compose(&cards, &replayed, 100);
        match ReviewState::of(&queue, cards.len()) {
            ReviewState::Due { backlog, due, .. } => {
                assert!(
                    backlog,
                    "a pile past a comfortable sitting must frame as backlog"
                );
                assert_eq!(due, cards.len());
            }
            other => panic!("expected a due/backlog state, got {other:?}"),
        }
    }
}
