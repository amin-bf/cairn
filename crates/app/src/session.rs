//! The review session, as logic the rest of the app draws a screen around. **Not a domain object**
//! (ADR-0005 §6): a session exists only here, and its defining property for issue #94 is that its
//! **position is never stored — only derived from the log**. Grading a card writes a `reviewed` row
//! that moves the card's due day into the future, so re-deriving the queue from the log after a
//! force-quit simply no longer offers it. There is nothing to persist and nothing to resume.
//!
//! Everything in this module is pure: it takes the current card set and a [`Replayed`] projection
//! (both from `cairn-core`) plus the **device-local day**, and returns what to offer. That day is
//! the edge value replay refuses to read (replay `CONTEXT.md`): "due today" is measured against the
//! device's local day, never the collection day scale.

use std::collections::{HashMap, HashSet};

use cairn_core::content::{CardRef, NoteId};
use cairn_core::replay::{
    Leech, NewCard, Replayed, introduction_candidates, notes_introduced_today,
};
use cairn_core::scheduling::{Grade, MemoryState, Scheduler, SchedulerParameters, day_gap};

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

/// Derive the queue from the current card set, the notes' authored positions, and the replayed state,
/// against the device-local `today`.
///
/// **Due** is due cards *minus those whose latest review was a pass* (ADR-0011 §9): a card is offered
/// when its scheduled `due_day` has arrived **or** its latest projected grade was a failure — the
/// same-session lapse re-show, which the bare `due_day` would floor a day out and lose. A passed card
/// is scheduled ahead and simply absent, which is how a just-graded card leaves the session with no
/// stored position (issue #94).
///
/// **New** is the introduction candidates (ADR-0011 §7, §8): the never-introduced cards, taken in
/// `(position, ordinal)` order up to `rate`, at most one per note, excluding suspended cards and
/// notes already introduced today. The rate is the only enforced limit in the app (ADR-0011 §1, §2);
/// `positions` carries each note's order key, and `suspended` is the mutable-surface flag (ADR-0010
/// §8) — a suspended card leaves **every** due count and is never introduced. Ordering is
/// deterministic so two runs, or a force-quit and relaunch, agree.
pub fn compose(
    current: &HashSet<CardRef>,
    positions: &HashMap<NoteId, String>,
    replayed: &Replayed,
    today: i64,
    rate: usize,
    suspended: &HashSet<CardRef>,
) -> Queue {
    let mut due: Vec<(i64, Offered)> = Vec::new();
    let mut new_cards: Vec<NewCard> = Vec::new();

    for &card in current {
        // A suspended card leaves **every** due count and is never introduced (ADR-0010 §8): it is
        // still replayed — its box goes on meaning durability — but it is not *offered*, which is the
        // one thing suspension changes. Skipped here before the due/new split so it enters neither.
        if suspended.contains(&card) {
            continue;
        }
        match replayed.cards.get(&card) {
            Some(state) if state.due_day <= today || state.last_grade.is_failure() => due.push((
                state.due_day,
                Offered {
                    card,
                    box_: state.box_,
                    is_new: false,
                    memory: Some(state.memory),
                    last_day: state.last_day,
                },
            )),
            Some(_) => {} // passed and scheduled ahead — not offered, and not stored anywhere
            None => new_cards.push(NewCard {
                card,
                position: positions.get(&card.note).cloned().unwrap_or_default(),
            }),
        }
    }

    due.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.card.encode().cmp(&b.1.card.encode()))
    });

    let introduced_today = notes_introduced_today(replayed, today);
    let new = introduction_candidates(&new_cards, &introduced_today, suspended, rate)
        .into_iter()
        .map(|card| Offered {
            card,
            box_: 1,
            is_new: true,
            memory: None,
            last_day: today,
        })
        .collect();

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

/// The leeches that **crossed the floor during the sitting just finished** — the end-of-session
/// pointer's contents (ADR-0010 §6). `before` is the set of leech cards captured when the sitting
/// began; `now` is [`leeches`](cairn_core::replay::leeches) recomputed at its end. A leech in `now`
/// but not in `before` crossed *this* session, caused by a failure the running app just logged — and
/// this needs **zero stored state**: no dismissal flag, no last-seen marker (ADR-0010 §6), only the
/// in-memory snapshot a sitting already is. The order of `now` (worst first) is preserved.
///
/// A card the user saw and ignored does not reappear here next session — it is not in a *later*
/// sitting's `now \ before` unless it crosses again — but it keeps its place on the dedicated screen,
/// which is the durable recourse.
pub fn crossed_this_session(before: &HashSet<CardRef>, now: &[Leech]) -> Vec<Leech> {
    now.iter()
        .filter(|leech| !before.contains(&leech.card))
        .cloned()
        .collect()
}

/// The state the review destination is in, chosen so the empty, new-deck and backlog cases are
/// **explicit worded states** rather than a card screen showing nothing (issue #94). `total` is the
/// number of cards the current content generates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewState {
    /// No cards exist at all — an empty collection.
    Empty,
    /// Cards exist and **nothing in the collection has ever been reviewed**: a genuinely first look
    /// (ADR-0006 §8's *"fresh deck, first look"*, whose parenthesis is *zero review history*). Carries
    /// how many are waiting.
    NewDeck { new: usize },
    /// Nothing is due, and what is left to offer is cards never yet seen. **Distinct from
    /// [`NewDeck`](ReviewState::NewDeck), which it looks identical to from the queue alone** — the two
    /// differ only in whether the collection has any history, and telling a reviewer of four years that
    /// their deck is fresh is the failure. This state is *routine*, not exceptional: ADR-0011 §2's rate
    /// caps introductions per day, so "the day's reviews are done and the rate still has room" is the
    /// ordinary shape of an afternoon. ADR-0006 §8 predates the rate and named only two worded states.
    NewOnly { new: usize },
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
    /// Classify the queue against the total card count and whether the collection has **any** review
    /// history.
    ///
    /// The history flag cannot be recovered from the queue, which is why it is a parameter: a queue of
    /// new cards with nothing due looks the same whether this is the collection's first minute or its
    /// fourth year, and only one of those is a fresh deck.
    pub fn of(queue: &Queue, total: usize, reviewed_ever: bool) -> ReviewState {
        if total == 0 {
            return ReviewState::Empty;
        }
        if queue.due.is_empty() {
            // Nothing due. Three different situations, and the copy each gets is a different sentence:
            // nothing left to introduce either (the resting state), cards waiting in a collection with
            // no history at all (a first look), or cards waiting in one that has plenty (the day's
            // reviews are done and ADR-0011 §2's rate still has room).
            return match (queue.new.is_empty(), reviewed_ever) {
                (true, _) => ReviewState::CaughtUp,
                (false, false) => ReviewState::NewDeck {
                    new: queue.new.len(),
                },
                (false, true) => ReviewState::NewOnly {
                    new: queue.new.len(),
                },
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
    use cairn_core::content::{BASIC, NoteId};
    use cairn_core::log::DayScale;
    use cairn_core::replay::replay;
    use cairn_core::scheduling::Grade;
    use cairn_store::Collection;
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

    /// A position map assigning each note an ascending order key by its place in `notes` — enough for
    /// the introduction order the queue reads. `basic` notes each generate one card, so a note's
    /// position is its card's introduction rank.
    fn positions(notes: &[NoteId]) -> HashMap<NoteId, String> {
        notes
            .iter()
            .enumerate()
            .map(|(i, &n)| (n, format!("{i:04}")))
            .collect()
    }

    /// The default new-card rate, high enough not to bind in the tests that are not about the cap.
    const RATE: usize = 100;

    #[test]
    fn a_brand_new_deck_is_all_new_and_nothing_due() {
        // Cards exist, none reviewed: every card is a new-card introduction (issue #94's new-deck
        // state), and the queue offers them as new, not due.
        let notes = [note(1), note(2)];
        let cards = current(&notes);
        let queue = compose(
            &cards,
            &positions(&notes),
            &Replayed::default(),
            0,
            RATE,
            &HashSet::new(),
        );
        assert_eq!(queue.due.len(), 0);
        assert_eq!(queue.new.len(), 2);
        assert_eq!(
            ReviewState::of(&queue, cards.len(), false),
            ReviewState::NewDeck { new: 2 }
        );
    }

    #[test]
    fn a_queue_of_new_cards_is_a_fresh_deck_only_when_nothing_was_ever_reviewed() {
        // The two states are **indistinguishable from the queue** — same empty due list, same new
        // cards — so the history flag is the whole difference, and dropping it tells a reviewer with
        // years of history that their deck is fresh (ADR-0006 §8's parenthesis is *zero review
        // history*; ADR-0011 §2's rate makes the other case an everyday one).
        let notes = [note(1), note(2)];
        let cards = current(&notes);
        let queue = compose(
            &cards,
            &positions(&notes),
            &Replayed::default(),
            0,
            RATE,
            &HashSet::new(),
        );
        assert_eq!(
            ReviewState::of(&queue, cards.len(), false),
            ReviewState::NewDeck { new: 2 },
            "no history anywhere: a genuine first look"
        );
        assert_eq!(
            ReviewState::of(&queue, cards.len(), true),
            ReviewState::NewOnly { new: 2 },
            "history behind it: the day's repeats are done, the rate still has room"
        );
    }

    #[test]
    fn an_empty_collection_is_the_empty_state() {
        let queue = compose(
            &HashSet::new(),
            &HashMap::new(),
            &Replayed::default(),
            0,
            RATE,
            &HashSet::new(),
        );
        assert_eq!(ReviewState::of(&queue, 0, false), ReviewState::Empty);
    }

    #[test]
    fn a_sitting_is_capped_by_the_chosen_count_and_by_what_exists() {
        let notes = [note(1), note(2), note(3)];
        let cards = current(&notes);
        let queue = compose(
            &cards,
            &positions(&notes),
            &Replayed::default(),
            0,
            RATE,
            &HashSet::new(),
        );
        assert_eq!(queue.sitting(2).len(), 2, "capped by the chosen count");
        assert_eq!(queue.sitting(10).len(), 3, "capped by what actually exists");
    }

    #[test]
    fn the_new_card_rate_caps_introductions_and_holds_position_order() {
        // ADR-0011 §2, §7, §8: the rate is the only enforced limit, and candidates come in position
        // order. Four fresh notes authored 0..3, a rate of two: only the first two are introduced.
        let notes = [note(1), note(2), note(3), note(4)];
        let cards = current(&notes);
        let queue = compose(
            &cards,
            &positions(&notes),
            &Replayed::default(),
            0,
            2,
            &HashSet::new(),
        );
        assert_eq!(queue.new.len(), 2, "the rate caps introductions");
        assert_eq!(
            queue.new.iter().map(|o| o.card).collect::<Vec<_>>(),
            vec![CardRef::new(note(1), 0), CardRef::new(note(2), 0)],
            "in authored position order"
        );
    }

    #[test]
    fn a_failed_card_stays_in_the_queue_for_a_same_session_re_show() {
        // ADR-0011 §9: the session queue is due cards minus *passes*, so a card failed today is still
        // offered — the same-session lapse re-show. A passed card the same day is gone.
        let data = TempDir::new().unwrap();
        let state = TempDir::new().unwrap();
        let mut coll = Collection::open(data.path(), state.path()).unwrap();
        let failed = note(1);
        let passed = note(2);
        let cards = current(&[failed, passed]);
        const TODAY: i64 = 20_514;
        let today_ms = TODAY * 86_400_000 + 4 * 3_600_000;

        coll.append_review(
            CardRef::new(failed, 0),
            Grade::Forgot,
            today_ms,
            DayScale::default(),
            1000,
        )
        .unwrap();
        coll.append_review(
            CardRef::new(passed, 0),
            Grade::Good,
            today_ms,
            DayScale::default(),
            1000,
        )
        .unwrap();

        let lines = coll.log_lines().unwrap();
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        let replayed = replay(&cards, &refs);
        let queue = compose(
            &cards,
            &HashMap::new(),
            &replayed,
            TODAY,
            RATE,
            &HashSet::new(),
        );

        assert_eq!(
            queue.due.iter().map(|o| o.card).collect::<Vec<_>>(),
            vec![CardRef::new(failed, 0)],
            "the failed card stays due; the passed one leaves"
        );
        // Both were introduced today, so neither is offered as new (ADR-0011 §8).
        assert!(queue.new.is_empty());
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
        let queue = compose(
            &cards,
            &HashMap::new(),
            &replayed,
            TODAY,
            RATE,
            &HashSet::new(),
        );
        assert_eq!(
            queue.available(),
            0,
            "a card graded today is excluded on re-derivation"
        );
        assert_eq!(
            ReviewState::of(&queue, cards.len(), true),
            ReviewState::CaughtUp
        );
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
        let queue = compose(
            &cards,
            &HashMap::new(),
            &replayed,
            TODAY + 3650,
            RATE,
            &HashSet::new(),
        );
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
    fn a_suspended_card_leaves_the_due_queue_entirely() {
        // ADR-0010 §8: a suspended card is excluded from every due count — otherwise the number could
        // never reach zero. A due card, suspended, must vanish from the queue and leave it caught up.
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

        // Far in the future so the card is genuinely due; then suspend it and it is not offered.
        let due_far = TODAY + 3650;
        let offered = compose(
            &cards,
            &HashMap::new(),
            &replayed,
            due_far,
            RATE,
            &HashSet::new(),
        );
        assert_eq!(offered.due.len(), 1, "the card is due when not suspended");

        let suspended = HashSet::from([card]);
        let queue = compose(
            &cards,
            &HashMap::new(),
            &replayed,
            due_far,
            RATE,
            &suspended,
        );
        assert!(
            queue.due.is_empty(),
            "a suspended card leaves the due queue"
        );
        assert_eq!(
            ReviewState::of(&queue, cards.len(), true),
            ReviewState::CaughtUp
        );
    }

    #[test]
    fn a_suspended_new_card_is_not_introduced() {
        // ADR-0010 §8, ADR-0011 §8: a suspended card is not introduced either. A fresh note whose only
        // card is suspended offers nothing.
        let n = note(1);
        let cards = current(&[n]);
        let suspended = HashSet::from([CardRef::new(n, 0)]);
        let queue = compose(
            &cards,
            &positions(&[n]),
            &Replayed::default(),
            0,
            RATE,
            &suspended,
        );
        assert!(
            queue.new.is_empty(),
            "a suspended new card is not introduced"
        );
        assert_eq!(
            ReviewState::of(&queue, cards.len(), true),
            ReviewState::CaughtUp
        );
    }

    #[test]
    fn the_end_of_session_pointer_covers_only_leeches_that_crossed_this_session() {
        // ADR-0010 §6: the pointer is leeches now minus leeches at the sitting's start — the cards
        // whose crossing this session caused, with zero stored state.
        let already = Leech {
            card: CardRef::new(note(1), 0),
            failure_days: 5,
            last_failure_day: 40,
        };
        let fresh = Leech {
            card: CardRef::new(note(2), 0),
            failure_days: 4,
            last_failure_day: 41,
        };
        let before = HashSet::from([already.card]);
        let now = vec![already.clone(), fresh.clone()];
        assert_eq!(
            crossed_this_session(&before, &now),
            vec![fresh.clone()],
            "a leech already crossed before the session does not appear in the pointer"
        );
        // Nothing new crossed: the pointer is empty and the session says nothing.
        assert!(crossed_this_session(&HashSet::from([already.card, fresh.card]), &now).is_empty());
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
                cairn_core::replay::CardState {
                    memory: cairn_core::scheduling::MemoryState {
                        stability: 5.0,
                        difficulty: 5.0,
                    },
                    box_: 2,
                    review_count: 1,
                    first_day: 0,
                    last_grade: Grade::Good,
                    last_day: 0,
                    due_day: 1,
                    failure_days: Vec::new(),
                },
            );
        }
        let queue = compose(
            &cards,
            &HashMap::new(),
            &replayed,
            100,
            RATE,
            &HashSet::new(),
        );
        match ReviewState::of(&queue, cards.len(), true) {
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
