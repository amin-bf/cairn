//! The sitting — one run of Review, as an object with operations (ADR-0046 §3).
//!
//! The rules a sitting carries are refused **by the operation**, not by each renderer remembering
//! them: [`Sitting::grade`] is refused unless the card on screen is revealed (ADR-0006 §4), and
//! [`Sitting::reveal`] has one cause (ADR-0006 §3). What stays outside is presentation — the reveal's
//! animation, scroll, which screen is showing — and the queue itself, which is re-derived from the log
//! by [`session::compose`](crate::session::compose) every time it is needed.
//!
//! # Time is a value
//!
//! Nothing in this file reads a clock. The caller passes two kinds of number, and they are not
//! interchangeable:
//!
//! - **`now_mono_ms`** — a monotonic millisecond count, with any origin, that never runs backwards.
//!   Every *duration* is a difference of two of these: the sitting's age for the checkpoint
//!   (ADR-0006 §1) and how long a card was on screen for the row's answer duration (ADR-0004 §5).
//! - **`now_wall_ms`** — wall-clock milliseconds since the epoch, which is what a `reviewed` row
//!   records as *when* (ADR-0004 §5). It is never subtracted from anything.
//!
//! So the ten-minute checkpoint is crossed in a test by passing a later monotonic time, never by
//! waiting (ADR-0009 §8).

use std::collections::HashSet;
use std::fmt;
use std::time::Duration;

use cairn_core::content::CardRef;
use cairn_core::log::DayScale;
use cairn_core::scheduling::Grade;
use cairn_store::{Collection, StoreError};

use crate::fixtures;

/// How long a sitting runs before the 10-minute checkpoint surfaces (ADR-0006 §1). Named rather than
/// inlined so the bench's override reads as *shorter than this* rather than as a second number.
const CHECKPOINT_AFTER: Duration = Duration::from_secs(600);

/// A sitting of review, held **only in memory** (ADR-0006 §6, issue #94): nothing about it is stored,
/// so a force-quit loses nothing — relaunch re-derives the queue from the log and every already-graded
/// card is simply no longer due. The queue is **re-derived every frame** rather than snapshotted, so
/// a card failed mid-sitting returns the same session (ADR-0011 §9) and an edit that makes a card
/// dormant drops it without ceremony.
///
/// The size the user picked counts **gradings, not distinct cards** (ADR-0011 §9): every graded event
/// advances `graded`, re-shows included, so the progress line always moves when the user acts. A
/// sitting ends when `graded` reaches `chosen` or the queue empties, whichever comes first.
#[derive(Debug)]
pub struct Sitting {
    /// The count the user chose — the number of **gradings** this sitting runs to.
    chosen: usize,
    /// Gradings performed so far. This is the progress numerator, and it counts every grade press,
    /// including a lapsed card's same-session re-show (ADR-0011 §9).
    graded: usize,
    /// The card currently on screen, so a reveal survives a frame and resets when the card changes.
    shown: Option<CardRef>,
    /// Private, so the reveal has one cause — [`Sitting::reveal`] (ADR-0006 §3).
    revealed: bool,
    /// When the sitting began — the quiet 10-minute timer runs from here (issue #94).
    started: u64,
    /// When the current card came on screen, so the row can record how long the answer took
    /// (ADR-0004 §5).
    card_shown: u64,
    /// Set once the user answers the 10-minute checkpoint's "keep going", so it does not nag again.
    checkpoint_dismissed: bool,
    /// The set of cards that were already leeches when this sitting began — the `before` half of the
    /// end-of-session pointer (ADR-0010 §6). Held **in memory only**, like everything else about a
    /// sitting: the pointer covers cards that crossed *this* session, which is exactly `leeches now`
    /// minus this snapshot, and it needs no stored dismissal or last-seen state.
    leeches_at_start: HashSet<CardRef>,
}

/// Why [`Sitting::reveal`] did nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevealRefused {
    /// No card is on screen, so there is no answer to show.
    NoCardShown,
}

/// Why [`Sitting::grade`] recorded nothing and counted nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GradeRefused {
    /// No card is on screen.
    NoCardShown,
    /// The card on screen has not been revealed: no self-grading before the answer is seen
    /// (ADR-0006 §4).
    NotRevealed,
}

impl fmt::Display for GradeRefused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GradeRefused::NoCardShown => f.write_str("no card is on screen"),
            GradeRefused::NotRevealed => f.write_str("the answer has not been revealed"),
        }
    }
}

impl std::error::Error for GradeRefused {}

/// What a permitted grading did to the store.
#[derive(Debug)]
pub enum Graded {
    /// The `reviewed` row was appended.
    Recorded,
    /// The append failed. The grading still counts and the card still leaves the screen: a failed
    /// append drops this one review rather than wedging the sitting, and the next derivation of the
    /// queue reflects whatever did commit.
    NotRecorded(StoreError),
}

impl Sitting {
    /// A sitting of `chosen` gradings that began at `now_mono_ms`.
    pub fn new(chosen: usize, leeches_at_start: HashSet<CardRef>, now_mono_ms: u64) -> Self {
        Sitting {
            chosen,
            graded: 0,
            shown: None,
            revealed: false,
            started: now_mono_ms,
            card_shown: now_mono_ms,
            checkpoint_dismissed: false,
            leeches_at_start,
        }
    }

    /// The count the user chose.
    pub fn chosen(&self) -> usize {
        self.chosen
    }

    /// Gradings performed so far.
    pub fn graded(&self) -> usize {
        self.graded
    }

    /// The card on screen, if any.
    pub fn shown(&self) -> Option<CardRef> {
        self.shown
    }

    /// Whether the answer of the card on screen has been revealed.
    pub fn revealed(&self) -> bool {
        self.revealed
    }

    /// The leeches that existed when the sitting began.
    pub fn leeches_at_start(&self) -> &HashSet<CardRef> {
        &self.leeches_at_start
    }

    /// Put `card` on screen at `now_mono_ms`. A card that is not the one already shown resets the
    /// reveal and the answer timer; the same card across calls keeps both, so a reveal survives a
    /// repaint. Returns whether the card changed — the renderer's cue to snap its reveal animation.
    pub fn offer(&mut self, card: CardRef, now_mono_ms: u64) -> bool {
        if self.shown == Some(card) {
            return false;
        }
        self.shown = Some(card);
        self.revealed = false;
        self.card_shown = now_mono_ms;
        true
    }

    /// Take the card off the screen **without grading it**: its content no longer generates (an edit
    /// removed it). Nothing is written and `graded` does not move; the reveal goes with the card, so
    /// the same card coming back later is a fresh showing.
    pub fn drop_shown(&mut self) {
        self.shown = None;
        self.revealed = false;
    }

    /// Show the answer of the card on screen (ADR-0006 §3). The one cause of a reveal: the person
    /// tapping the card. Revealing twice is harmless.
    pub fn reveal(&mut self) -> Result<(), RevealRefused> {
        if self.shown.is_none() {
            return Err(RevealRefused::NoCardShown);
        }
        self.revealed = true;
        Ok(())
    }

    /// Grade the card on screen — **refused unless it is revealed** (ADR-0006 §4), in which case
    /// nothing is written and nothing is counted.
    ///
    /// A permitted grading appends the `reviewed` row, stamped `now_wall_ms` and carrying how long the
    /// card was on screen (`now_mono_ms` minus the moment it came up, ADR-0004 §5). It advances
    /// `graded` whatever the grade — the re-derived queue decides whether the card returns (a lapse)
    /// or leaves (a pass), not this counter — and clears the card so the next one is offered fresh.
    pub fn grade(
        &mut self,
        coll: &mut Collection,
        grade: Grade,
        now_mono_ms: u64,
        now_wall_ms: i64,
    ) -> Result<Graded, GradeRefused> {
        let card = self.shown.ok_or(GradeRefused::NoCardShown)?;
        if !self.revealed {
            return Err(GradeRefused::NotRevealed);
        }
        let duration_ms = now_mono_ms.saturating_sub(self.card_shown);
        let outcome =
            match coll.append_review(card, grade, now_wall_ms, DayScale::default(), duration_ms) {
                Ok(_) => Graded::Recorded,
                Err(e) => Graded::NotRecorded(e),
            };
        self.graded += 1;
        self.revealed = false;
        self.shown = None;
        Ok(outcome)
    }

    /// The checkpoint is due once ten minutes have passed and the user has not already waved it away.
    /// It is a **courtesy**, never an enforcement — reaching the chosen count is what ends a session
    /// (issue #94).
    ///
    /// **The ten minutes are ADR-0006 §1's and are written here**, where the behaviour is; the bench
    /// only ever *shortens* what it is handed ([`fixtures::checkpoint_after`]). That direction is the
    /// point: this is the one decided state no pre-made collection can reach — it hangs off a
    /// monotonic clock rather than off the log — so the alternative to a lever is ten real minutes
    /// that no capture run and nobody holding a handset ever waits for, which is precisely why the
    /// checkpoint contradicted its own ADR for ten months without anything failing.
    pub fn checkpoint_due(&self, now_mono_ms: u64) -> bool {
        self.checkpoint_due_after(now_mono_ms, fixtures::checkpoint_after(CHECKPOINT_AFTER))
    }

    /// [`checkpoint_due`](Self::checkpoint_due) with the interval handed in, so the arithmetic is
    /// testable without the process-global bench lever.
    fn checkpoint_due_after(&self, now_mono_ms: u64, after: Duration) -> bool {
        !self.checkpoint_dismissed
            && Duration::from_millis(now_mono_ms.saturating_sub(self.started)) >= after
    }

    /// The user answered the checkpoint with *Keep going*: it does not come back this sitting.
    pub fn keep_going(&mut self) {
        self.checkpoint_dismissed = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cairn_core::log::{ParsedLine, ReviewedRow, Row, parse_line};
    use tempfile::TempDir;

    const WALL: i64 = 1_700_000_000_000;

    fn one_card() -> (Collection, CardRef, TempDir, TempDir) {
        let data = TempDir::new().unwrap();
        let state = TempDir::new().unwrap();
        let mut coll = Collection::open(data.path(), state.path()).unwrap();
        let note = coll
            .create_note("basic", &[("Front", "chien"), ("Back", "dog")])
            .unwrap();
        (coll, CardRef::new(note, 0), data, state)
    }

    fn review_rows(coll: &Collection) -> Vec<ReviewedRow> {
        coll.log_lines()
            .unwrap()
            .iter()
            .filter_map(|line| match parse_line(line) {
                ParsedLine::Row(Row::Reviewed(row)) => Some(row),
                _ => None,
            })
            .collect()
    }

    /// ADR-0006 §4, refused by the operation: a stray grade on an unrevealed card writes nothing and
    /// counts nothing.
    #[test]
    fn grading_an_unrevealed_card_is_refused_and_writes_nothing() {
        let (mut coll, card, _d, _s) = one_card();
        let mut sitting = Sitting::new(5, HashSet::new(), 0);
        sitting.offer(card, 100);

        let refused = sitting.grade(&mut coll, Grade::Good, 4_000, WALL);

        assert!(matches!(refused, Err(GradeRefused::NotRevealed)));
        assert_eq!(sitting.graded, 0, "a refused grade is not counted");
        assert_eq!(sitting.shown, Some(card), "the card stays on screen");
        assert!(
            review_rows(&coll).is_empty(),
            "a refused grade is not recorded"
        );
    }

    #[test]
    fn grading_with_no_card_on_screen_is_refused() {
        let (mut coll, _card, _d, _s) = one_card();
        let mut sitting = Sitting::new(5, HashSet::new(), 0);
        assert!(matches!(
            sitting.grade(&mut coll, Grade::Good, 1, WALL),
            Err(GradeRefused::NoCardShown)
        ));
        assert_eq!(sitting.reveal(), Err(RevealRefused::NoCardShown));
    }

    #[test]
    fn a_revealed_card_is_graded_recorded_counted_and_cleared() {
        let (mut coll, card, _d, _s) = one_card();
        let mut sitting = Sitting::new(5, HashSet::new(), 0);
        sitting.offer(card, 100);
        sitting.reveal().unwrap();

        let graded = sitting.grade(&mut coll, Grade::Good, 4_000, WALL).unwrap();

        assert!(matches!(graded, Graded::Recorded));
        assert_eq!(sitting.graded, 1);
        assert_eq!(sitting.shown, None);
        assert!(!sitting.revealed, "the next card starts unrevealed");
        assert_eq!(review_rows(&coll).len(), 1);
    }

    /// The answer duration is the monotonic difference, and the row's time is the wall value: the two
    /// clocks are not mixed (ADR-0004 §5). Read back from the log row itself.
    #[test]
    fn the_row_carries_the_wall_time_and_the_monotonic_duration() {
        let (mut coll, card, _d, _s) = one_card();
        let mut sitting = Sitting::new(5, HashSet::new(), 0);
        sitting.offer(card, 1_000);
        sitting.reveal().unwrap();
        sitting.grade(&mut coll, Grade::Easy, 4_250, WALL).unwrap();

        let rows = review_rows(&coll);
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].duration_ms, 3_250,
            "4250 - 1000 on the monotonic clock"
        );
        // 1_700_000_000_000 ms is 14.11.2023 22:13:20 UTC — the wall value, not the monotonic one.
        assert!(
            rows[0].instant.starts_with("2023-11-14T22:13:20"),
            "stamped from the wall clock: {}",
            rows[0].instant
        );
        assert_eq!(rows[0].grade, 4);
    }

    #[test]
    fn the_same_card_offered_again_keeps_its_reveal_and_a_new_one_resets_it() {
        let (_coll, card, _d, _s) = one_card();
        let other = CardRef::new(card.note, 1);
        let mut sitting = Sitting::new(5, HashSet::new(), 0);
        assert!(sitting.offer(card, 10));
        sitting.reveal().unwrap();
        assert!(!sitting.offer(card, 20), "a repaint is not a new card");
        assert!(sitting.revealed);
        assert!(sitting.offer(other, 30));
        assert!(!sitting.revealed);
    }

    /// ADR-0006 §1: the checkpoint is crossed by passing a later monotonic time.
    #[test]
    fn the_ten_minute_checkpoint_is_crossed_by_passing_a_later_time() {
        let sitting = Sitting::new(10, HashSet::new(), 5_000);
        let ten_minutes = Duration::from_secs(600);
        assert!(!sitting.checkpoint_due_after(5_000 + 599_999, ten_minutes));
        assert!(sitting.checkpoint_due_after(5_000 + 600_000, ten_minutes));
        // And through the public call, which reads the bench lever and never shortens past zero:
        // seven hundred seconds on is past the checkpoint whatever the lever says.
        assert!(sitting.checkpoint_due(5_000 + 700_000));
    }

    #[test]
    fn keeping_going_dismisses_the_checkpoint_for_the_rest_of_the_sitting() {
        let mut sitting = Sitting::new(10, HashSet::new(), 0);
        let ten_minutes = Duration::from_secs(600);
        assert!(sitting.checkpoint_due_after(700_000, ten_minutes));
        sitting.keep_going();
        assert!(!sitting.checkpoint_due_after(9_000_000, ten_minutes));
    }
}
