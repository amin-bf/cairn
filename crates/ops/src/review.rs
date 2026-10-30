//! Review as **facts** (ADR-0046 §1, §4) — what a renderer needs to draw the entrance and a running
//! sitting, with every rule already applied and no sentence in sight.
//!
//! Two halves. [`facts`] derives the entrance: which of the five states the collection is in, the
//! counts that go with it, whether the queue is a backlog, which of 5/10/20 are shorter than the
//! queue, and the leech and suspended counts. [`Review`] is a [`Sitting`](crate::sitting::Sitting)
//! together with the collection reads it needs, so a renderer asks *what is the card shown*, *reveal*,
//! *grade*, and never composes the queue or counts a grading itself.
//!
//! **The card shown** ([`Shown`]) carries both faces as [`Face`]s — runs of text with a [`RunStyle`],
//! from the one restricted-Markdown parser ([`markdown`]) with cloze masking already applied by
//! [`deck::render`] — the prompt's direction ([`is_rtl`], its first strong character), the box badge
//! as a fact, and the interval preview in days for each grade ([`session::interval_preview`]).
//!
//! **Time is a value** (ADR-0046 §3): nothing here reads a clock. `now_wall_ms` is wall-clock
//! milliseconds (the review row's *when*, and the device-local day the queue is measured against);
//! `now_mono_ms` is the monotonic count every duration is a difference of.

use std::collections::HashSet;

use cairn_core::content::CardRef;
use cairn_core::log::{DEFAULT_NEW_CARD_RATE, DayScale, day_number};
use cairn_core::replay::{Leech, leeches, replay};
use cairn_core::scheduling::Grade;
use cairn_store::Collection;

use crate::deck;
use crate::markdown;
use crate::session::{self, Offered, Queue, ReviewState};
use crate::sitting::{GradeRefused, Graded, RevealRefused, Sitting};

/// The sittings the entrance may offer besides the whole queue (ADR-0034 §2).
pub const SHORTER_SITTINGS: [usize; 3] = [5, 10, 20];

/// Whether the paragraph `text` begins in is right-to-left, by its first strong character — ADR-0012
/// §7's *dir=auto*. This is the fact the box badge's side and the text alignment follow, and it is
/// asked of the **prompt** and holds for the card's life.
pub fn is_rtl(text: &str) -> bool {
    let info = unicode_bidi::BidiInfo::new(text, None);
    info.paragraphs.first().is_some_and(|p| p.level.is_rtl())
}

/// How a run of text is drawn. [`markdown::Style`] is three independent flags; the card draws code
/// in the monospace face whatever else covers it, and otherwise bold and italic compose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStyle {
    Plain,
    Bold,
    Italic,
    BoldItalic,
    Code,
}

impl From<markdown::Style> for RunStyle {
    fn from(style: markdown::Style) -> Self {
        match (style.code, style.bold, style.italic) {
            (true, _, _) => RunStyle::Code,
            (false, true, true) => RunStyle::BoldItalic,
            (false, true, false) => RunStyle::Bold,
            (false, false, true) => RunStyle::Italic,
            (false, false, false) => RunStyle::Plain,
        }
    }
}

/// A stretch of a face's text in one style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub text: String,
    pub style: RunStyle,
}

/// A field's text as runs: the markers removed, adjacent characters of one style joined into one run.
pub fn runs(text: &str) -> Vec<Run> {
    let marked = markdown::parse(text);
    let mut out: Vec<Run> = Vec::new();
    for (at, c) in marked.plain.char_indices() {
        let style = RunStyle::from(marked.style_at(at));
        match out.last_mut() {
            Some(last) if last.style == style => last.text.push(c),
            _ => out.push(Run {
                text: c.to_string(),
                style,
            }),
        }
    }
    out
}

/// What the entrance is, as facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Facts {
    pub state: ReviewState,
    /// Cards due today (zero in every state but `Due`).
    pub due: usize,
    /// First-time cards the rate would introduce today.
    pub new: usize,
    /// Everything a sitting could draw from: `due + new`.
    pub available: usize,
    /// More due than a comfortable sitting (`session::COMFORTABLE_SITTING`): frame, do not alarm.
    pub backlog: bool,
    /// Which of 5/10/20 are strictly shorter than the queue — an option equal to it is the primary
    /// said twice.
    pub shorter: Vec<usize>,
    /// Cards over the leech floor, suspended ones included (ADR-0010 §6) — carried for the leech
    /// slice.
    pub leeches: usize,
    /// Suspended cards (ADR-0010 §8).
    pub suspended: usize,
}

/// Everything the entrance and a sitting derive from the log, read fresh each time — nothing about
/// the queue is cached, so a force-quit loses nothing (ADR-0006 §6).
struct Derived {
    queue: Queue,
    ranked: Vec<Leech>,
    suspended: usize,
    total: usize,
    reviewed_ever: bool,
    today: i64,
}

fn derive(coll: &Collection, now_wall_ms: i64) -> Result<Derived, String> {
    let today = day_number(now_wall_ms, DayScale::default());
    let current = deck::current_cards(coll).map_err(|e| e.to_string())?;
    let positions = deck::note_positions(coll).map_err(|e| e.to_string())?;
    let rate = coll.new_card_rate().unwrap_or(DEFAULT_NEW_CARD_RATE) as usize;
    let suspended = coll.suspended().map_err(|e| e.to_string())?;
    let lines = coll.log_lines().map_err(|e| e.to_string())?;
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    let replayed = replay(&current, &refs);
    let queue = session::compose(&current, &positions, &replayed, today, rate, &suspended);
    Ok(Derived {
        ranked: leeches(&replayed, today),
        suspended: suspended.len(),
        total: current.len(),
        reviewed_ever: !replayed.cards.is_empty(),
        queue,
        today,
    })
}

/// The entrance, as facts, against the day of `now_wall_ms`.
pub fn facts(coll: &Collection, now_wall_ms: i64) -> Result<Facts, String> {
    let d = derive(coll, now_wall_ms)?;
    let state = ReviewState::of(&d.queue, d.total, d.reviewed_ever);
    let available = d.queue.available();
    let backlog = matches!(state, ReviewState::Due { backlog: true, .. });
    Ok(Facts {
        state,
        due: d.queue.due.len(),
        new: d.queue.new.len(),
        available,
        backlog,
        shorter: SHORTER_SITTINGS
            .into_iter()
            .filter(|option| *option < available)
            .collect(),
        leeches: d.ranked.len(),
        suspended: d.suspended,
    })
}

/// The card on screen, as facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shown {
    pub card: CardRef,
    pub prompt: Vec<Run>,
    pub answer: Vec<Run>,
    /// Whether the prompt begins right-to-left. The badge sits in the corner reading does not begin
    /// at, and **the prompt governs** so it cannot change sides at the reveal.
    pub rtl: bool,
    /// Never reviewed: the badge reads *new*, not *box 1*, which would state a durability nothing
    /// has measured (ADR-0006 §6).
    pub is_new: bool,
    /// The durability box, 1–5.
    pub box_: u8,
    /// The illustrative next interval in days for Forgot, Barely, Good, Easy, in that order.
    pub previews: [u32; 4],
    pub revealed: bool,
}

/// What a permitted grading did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GradeOutcome {
    /// Whether the `reviewed` row was appended. A failed append still counts the grading and moves
    /// the card off the screen (`Graded::NotRecorded`).
    pub recorded: bool,
}

/// A sitting and the reads it needs.
#[derive(Debug)]
pub struct Review {
    sitting: Sitting,
    finished: bool,
}

impl Review {
    /// Begin a sitting of `count` gradings at `now_mono_ms`, snapshotting the leeches that already
    /// exist so the end-of-session count covers only those that cross during it (ADR-0010 §6).
    pub fn start(
        coll: &Collection,
        count: usize,
        now_mono_ms: u64,
        now_wall_ms: i64,
    ) -> Result<Review, String> {
        let before: HashSet<CardRef> = derive(coll, now_wall_ms)?
            .ranked
            .iter()
            .map(|l| l.card)
            .collect();
        Ok(Review {
            sitting: Sitting::new(count, before, now_mono_ms),
            finished: false,
        })
    }

    /// The count chosen.
    pub fn chosen(&self) -> usize {
        self.sitting.chosen()
    }

    /// Gradings made, re-shows included (ADR-0011 §9).
    pub fn graded(&self) -> usize {
        self.sitting.graded()
    }

    /// The card shown — the head of the **live** queue that still renders, re-derived now, so a card
    /// failed a moment ago is back in the running (ADR-0011 §9). `None` once the sitting has ended.
    /// Showing a card is what puts it on screen: the same card across calls keeps its reveal.
    pub fn card(
        &mut self,
        coll: &Collection,
        now_wall_ms: i64,
        now_mono_ms: u64,
    ) -> Result<Option<Shown>, String> {
        if self.finished || self.sitting.graded() >= self.sitting.chosen() {
            return Ok(None);
        }
        let d = derive(coll, now_wall_ms)?;
        let mut head = None;
        for offered in d.queue.sitting(usize::MAX) {
            if let Some(rendered) = deck::render(coll, offered.card).map_err(|e| e.to_string())? {
                head = Some((offered, rendered));
                break;
            }
        }
        let Some((offered, rendered)) = head else {
            return Ok(None);
        };
        self.sitting.offer(offered.card, now_mono_ms);
        Ok(Some(Shown {
            card: offered.card,
            prompt: runs(&rendered.prompt),
            answer: runs(&rendered.answer),
            rtl: is_rtl(&rendered.prompt),
            is_new: offered.is_new,
            box_: offered.box_,
            previews: previews(&offered, d.today),
            revealed: self.sitting.revealed(),
        }))
    }

    /// Show the answer of the card on screen (ADR-0006 §3).
    pub fn reveal(&mut self) -> Result<(), RevealRefused> {
        self.sitting.reveal()
    }

    /// Grade the card on screen — refused unless it is revealed (ADR-0006 §4).
    pub fn grade(
        &mut self,
        coll: &mut Collection,
        grade: Grade,
        now_mono_ms: u64,
        now_wall_ms: i64,
    ) -> Result<GradeOutcome, GradeRefused> {
        let outcome = self.sitting.grade(coll, grade, now_mono_ms, now_wall_ms)?;
        Ok(GradeOutcome {
            recorded: matches!(outcome, Graded::Recorded),
        })
    }

    /// Ten minutes have passed and the reviewer has not waved the check-in away (ADR-0006 §1).
    pub fn checkpoint_due(&self, now_mono_ms: u64) -> bool {
        self.sitting.checkpoint_due(now_mono_ms)
    }

    /// *Keep going*: the checkpoint does not come back this sitting.
    pub fn keep_going(&mut self) {
        self.sitting.keep_going();
    }

    /// *Finish here*: the sitting ends now.
    pub fn finish_here(&mut self) {
        self.finished = true;
    }

    /// Whether the sitting has ended: finished by the reviewer, the chosen count reached, or the
    /// queue emptied before it (ADR-0006 §8).
    pub fn ended(&self, coll: &Collection, now_wall_ms: i64) -> Result<bool, String> {
        if self.finished || self.sitting.graded() >= self.sitting.chosen() {
            return Ok(true);
        }
        let d = derive(coll, now_wall_ms)?;
        for offered in d.queue.sitting(usize::MAX) {
            if deck::render(coll, offered.card)
                .map_err(|e| e.to_string())?
                .is_some()
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// How many leeches crossed the floor during this sitting (ADR-0010 §6) — the leeches now minus
    /// the snapshot taken at its start.
    pub fn leeches_crossed(&self, coll: &Collection, now_wall_ms: i64) -> Result<usize, String> {
        let d = derive(coll, now_wall_ms)?;
        Ok(session::crossed_this_session(self.sitting.leeches_at_start(), &d.ranked).len())
    }
}

fn previews(offered: &Offered, today: i64) -> [u32; 4] {
    [Grade::Forgot, Grade::Barely, Grade::Good, Grade::Easy]
        .map(|grade| session::interval_preview(offered, grade, today))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::Fixture;
    use tempfile::TempDir;

    /// A moment well after the fixtures' own clock origin; the fixtures take `now` as a value.
    const NOW: i64 = 1_760_000_000_000;

    fn installed(fixture: Fixture) -> (Collection, TempDir, TempDir) {
        let data = TempDir::new().unwrap();
        let state = TempDir::new().unwrap();
        let mut coll = Collection::open(data.path(), state.path()).unwrap();
        fixture.install(&mut coll, NOW).unwrap();
        (coll, data, state)
    }

    fn fresh() -> (Collection, TempDir, TempDir) {
        let data = TempDir::new().unwrap();
        let state = TempDir::new().unwrap();
        let coll = Collection::open(data.path(), state.path()).unwrap();
        (coll, data, state)
    }

    #[test]
    fn runs_join_a_style_and_drop_the_markers() {
        assert_eq!(
            runs("a **b** *c* `d` e"),
            vec![
                Run {
                    text: "a ".into(),
                    style: RunStyle::Plain
                },
                Run {
                    text: "b".into(),
                    style: RunStyle::Bold
                },
                Run {
                    text: " ".into(),
                    style: RunStyle::Plain
                },
                Run {
                    text: "c".into(),
                    style: RunStyle::Italic
                },
                Run {
                    text: " ".into(),
                    style: RunStyle::Plain
                },
                Run {
                    text: "d".into(),
                    style: RunStyle::Code
                },
                Run {
                    text: " e".into(),
                    style: RunStyle::Plain
                },
            ]
        );
        assert_eq!(
            runs("**a *b* c**"),
            vec![
                Run {
                    text: "a ".into(),
                    style: RunStyle::Bold
                },
                Run {
                    text: "b".into(),
                    style: RunStyle::BoldItalic
                },
                Run {
                    text: " c".into(),
                    style: RunStyle::Bold
                },
            ]
        );
        assert!(runs("").is_empty());
    }

    #[test]
    fn direction_is_the_first_strong_character() {
        assert!(is_rtl("سلام دنیا"));
        assert!(!is_rtl("bonjour le monde"));
        assert!(!is_rtl(""));
        // Neutral characters lead; the first strong one decides.
        assert!(is_rtl("1) سلام"));
        assert!(!is_rtl("«à» hello سلام"));
    }

    #[test]
    fn an_empty_collection_is_the_empty_state() {
        let (coll, _d, _s) = fresh();
        let facts = facts(&coll, NOW).unwrap();
        assert_eq!(facts.state, ReviewState::Empty);
        assert_eq!(facts.available, 0);
        assert!(facts.shorter.is_empty());
    }

    #[test]
    fn the_caught_up_fixture_is_caught_up_with_no_leeches() {
        let (coll, _d, _s) = installed(Fixture::CaughtUp);
        let facts = facts(&coll, NOW).unwrap();
        assert_eq!(facts.state, ReviewState::CaughtUp);
        assert_eq!((facts.due, facts.new, facts.leeches), (0, 0, 0));
    }

    #[test]
    fn the_leeches_fixture_is_caught_up_and_carries_the_leech_count() {
        let (coll, _d, _s) = installed(Fixture::Leeches);
        let facts = facts(&coll, NOW).unwrap();
        assert_eq!(facts.state, ReviewState::CaughtUp);
        assert_eq!(facts.leeches, 3);
    }

    #[test]
    fn a_backlog_is_framed_and_offers_every_shorter_sitting() {
        let (coll, _d, _s) = installed(Fixture::Backlog);
        let facts = facts(&coll, NOW).unwrap();
        assert!(matches!(facts.state, ReviewState::Due { due: 25, .. }));
        assert!(facts.backlog);
        assert_eq!(facts.shorter, vec![5, 10, 20]);
    }

    #[test]
    fn a_first_seed_is_a_fresh_deck_and_no_sitting_is_shorter_than_five_cards() {
        let (coll, _d, _s) = {
            let data = TempDir::new().unwrap();
            let state = TempDir::new().unwrap();
            let coll = crate::open_collection(data.path(), state.path()).unwrap();
            (coll, data, state)
        };
        let facts = facts(&coll, NOW).unwrap();
        assert!(matches!(facts.state, ReviewState::NewDeck { .. }));
        assert!(!facts.backlog);
        assert!(facts.shorter.is_empty());
    }

    #[test]
    fn only_new_cards_left_after_a_review_is_new_only_never_a_fresh_deck() {
        let (mut coll, _d, _s) = fresh();
        for n in 0..8 {
            coll.create_note("basic", &[("Front", &format!("f{n}")), ("Back", "b")])
                .unwrap();
        }
        let mut review = Review::start(&coll, 1, 0, NOW).unwrap();
        review.card(&coll, NOW, 10).unwrap().unwrap();
        review.reveal().unwrap();
        review.grade(&mut coll, Grade::Easy, 20, NOW).unwrap();
        let facts = facts(&coll, NOW).unwrap();
        assert!(
            matches!(facts.state, ReviewState::NewOnly { .. }),
            "{:?}",
            facts.state
        );
    }

    /// The whole sitting through the operations a renderer calls, on a real store.
    #[test]
    fn a_sitting_runs_from_start_to_end_and_counts_its_gradings() {
        let (mut coll, _d, _s) = installed(Fixture::Backlog);
        let mut review = Review::start(&coll, 3, 1_000, NOW).unwrap();
        assert_eq!((review.chosen(), review.graded()), (3, 0));
        for made in 0..3 {
            assert!(!review.ended(&coll, NOW).unwrap());
            let shown = review.card(&coll, NOW, 2_000).unwrap().unwrap();
            assert!(!shown.revealed);
            review.reveal().unwrap();
            assert!(review.card(&coll, NOW, 2_100).unwrap().unwrap().revealed);
            review.grade(&mut coll, Grade::Good, 3_000, NOW).unwrap();
            assert_eq!(review.graded(), made + 1);
        }
        assert!(review.ended(&coll, NOW).unwrap());
        assert!(review.card(&coll, NOW, 4_000).unwrap().is_none());
    }

    #[test]
    fn a_grade_before_the_reveal_is_refused_and_counts_nothing() {
        let (mut coll, _d, _s) = installed(Fixture::Backlog);
        let mut review = Review::start(&coll, 3, 0, NOW).unwrap();
        review.card(&coll, NOW, 10).unwrap().unwrap();
        let refused = review.grade(&mut coll, Grade::Good, 20, NOW);
        assert_eq!(refused.unwrap_err(), GradeRefused::NotRevealed);
        assert_eq!(review.graded(), 0);
    }

    /// ADR-0011 §9: a lapse returns the same card in the same sitting, and the re-show counts.
    #[test]
    fn a_forgotten_card_is_shown_again_in_the_same_sitting() {
        let (mut coll, _d, _s) = installed(Fixture::Crossing);
        let mut review = Review::start(&coll, 5, 0, NOW).unwrap();
        let first = review.card(&coll, NOW, 10).unwrap().unwrap();
        review.reveal().unwrap();
        review.grade(&mut coll, Grade::Forgot, 20, NOW).unwrap();
        let again = review.card(&coll, NOW, 30).unwrap().unwrap();
        assert_eq!(again.card, first.card, "the lapse comes straight back");
        assert!(!again.revealed, "and starts unrevealed");
        assert_eq!(review.graded(), 1, "the re-show is counted");
    }

    #[test]
    fn a_leech_that_crosses_during_the_sitting_is_counted_at_its_end() {
        let (mut coll, _d, _s) = installed(Fixture::Crossing);
        let mut review = Review::start(&coll, 1, 0, NOW).unwrap();
        assert_eq!(review.leeches_crossed(&coll, NOW).unwrap(), 0);
        review.card(&coll, NOW, 10).unwrap().unwrap();
        review.reveal().unwrap();
        review.grade(&mut coll, Grade::Forgot, 20, NOW).unwrap();
        assert!(review.ended(&coll, NOW).unwrap());
        assert_eq!(review.leeches_crossed(&coll, NOW).unwrap(), 1);
    }

    #[test]
    fn the_checkpoint_is_crossed_by_a_later_time_and_waved_away_by_keep_going() {
        let (coll, _d, _s) = installed(Fixture::Backlog);
        let mut review = Review::start(&coll, 5, 5_000, NOW).unwrap();
        assert!(!review.checkpoint_due(5_000 + 1_000));
        assert!(review.checkpoint_due(5_000 + 700_000));
        review.keep_going();
        assert!(!review.checkpoint_due(5_000 + 9_000_000));
    }

    #[test]
    fn finishing_here_ends_the_sitting_with_cards_still_due() {
        let (coll, _d, _s) = installed(Fixture::Backlog);
        let mut review = Review::start(&coll, 5, 0, NOW).unwrap();
        assert!(!review.ended(&coll, NOW).unwrap());
        review.finish_here();
        assert!(review.ended(&coll, NOW).unwrap());
        assert!(review.card(&coll, NOW, 10).unwrap().is_none());
    }

    #[test]
    fn the_previews_equal_the_sessions_for_the_same_card_and_day() {
        let (coll, _d, _s) = installed(Fixture::Backlog);
        let mut review = Review::start(&coll, 1, 0, NOW).unwrap();
        let shown = review.card(&coll, NOW, 10).unwrap().unwrap();

        let d = derive(&coll, NOW).unwrap();
        let offered = d
            .queue
            .due
            .iter()
            .chain(d.queue.new.iter())
            .find(|o| o.card == shown.card)
            .unwrap();
        let expected = [Grade::Forgot, Grade::Barely, Grade::Good, Grade::Easy]
            .map(|g| session::interval_preview(offered, g, d.today));
        assert_eq!(shown.previews, expected);
        assert!(shown.previews.iter().all(|days| *days >= 1));
        assert!(!shown.is_new && shown.box_ >= 1);
    }

    #[test]
    fn a_cloze_card_arrives_masked_and_its_answer_unmasked() {
        let (coll, _d, _s) = installed(Fixture::Cloze);
        let mut review = Review::start(&coll, 5, 0, NOW).unwrap();
        let shown = review.card(&coll, NOW, 10).unwrap().unwrap();
        let rendered = deck::render(&coll, shown.card).unwrap().unwrap();
        let join = |runs: &[Run]| runs.iter().map(|r| r.text.as_str()).collect::<String>();
        assert_eq!(join(&shown.prompt), markdown::parse(&rendered.prompt).plain);
        assert_eq!(join(&shown.answer), markdown::parse(&rendered.answer).plain);
        assert_ne!(
            join(&shown.prompt),
            join(&shown.answer),
            "the blank is masked"
        );
        assert!(shown.is_new);
    }

    #[test]
    fn a_persian_prompt_is_right_to_left_and_a_french_one_is_not() {
        let (coll, _d, _s) = installed(Fixture::Persian);
        // Walk the whole queue: the Persian fixture holds both scripts.
        let mut review = Review::start(&coll, 100, 0, NOW).unwrap();
        let d = derive(&coll, NOW).unwrap();
        let mut seen = (false, false);
        for offered in d.queue.sitting(usize::MAX) {
            let rendered = deck::render(&coll, offered.card).unwrap().unwrap();
            let rtl = is_rtl(&rendered.prompt);
            let latin = rendered.prompt.chars().any(|c| c.is_ascii_alphabetic());
            seen.0 |= rtl;
            seen.1 |= !rtl && latin;
        }
        assert_eq!(seen, (true, true));
        // And through the card shown: the fact is the prompt's.
        let shown = review.card(&coll, NOW, 10).unwrap().unwrap();
        let rendered = deck::render(&coll, shown.card).unwrap().unwrap();
        assert_eq!(shown.rtl, is_rtl(&rendered.prompt));
    }
}
