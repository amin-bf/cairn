//! **Temporary, and not a specified feature.** The fixture bench: the collection states the capture
//! harness cannot reach on its own, defined once and installable from two directions.
//!
//! # Why this exists
//!
//! The desktop harness creates, redirects and wipes the app's entire data directory per run
//! (`docs/environment/desktop-capture.md`), which is *why* every capture is a first launch and
//! therefore always the same six due cards from [`CairnApp::open_store`](crate::CairnApp). Several
//! decided screens are unreachable from that one fixture — the caught-up floor needs a collection
//! with nothing due, the end-of-session pointer needs a card failed repeatedly, and the entrance's
//! shorter-sitting line needs a queue longer than five. #134 had to photograph the first two inside a
//! *prototype*, so the application shipped decided states whose only pictures were of something that
//! is not the application.
//!
//! **The bench is no longer only about Review.** ADR-0021 §9's deck surface needs a collection with a
//! deck in it, and nothing in the product creates one on its own — ADR-0005 §8 refuses an
//! auto-created default outright, deliberately, so an empty collection holds zero decks and always
//! will. The filter, *New deck*, *Delete deck* and the editor's deck dropdown were therefore drawn in
//! every capture this repository holds with nothing in them at all. So is direction: the seed and
//! every word list here are French, and the one right-to-left row the design pass has seen cost
//! eleven storyboard steps driving `xdotool` into a field. [`Fixture::Decks`] is both.
//!
//! # The route, and the two it is not
//!
//! **A fixture is a pre-made collection, and the shipping seed is left alone.** The harness already
//! owns the data directory, so a state is installed by *dropping in a collection already in it* —
//! the app opens a non-empty store and the seed never fires. Two alternatives were weighed and
//! rejected on #149: **extending the seed** changes what a real user meets on first install and
//! changes what every capture already in the repository is a picture of, so
//! `docs/design/baseline-2026-08-08/` would stop being comparable to anything after it; and a
//! **capture-mode entry point** is app code shaped by the harness's needs, where a state expressed as
//! *data* is what a state actually is.
//!
//! [`CairnApp::open_store`](crate::CairnApp) is therefore untouched, and that is the whole value of
//! the route: a fixture that worked by editing the seed would have solved a different problem.
//!
//! # Two ways in, one definition
//!
//! The states are defined **here**, once. There are two ways to install them:
//!
//! 1. **From outside** — the `cairn-fixture` binary, run by `capture-desktop.sh` against the scratch
//!    profile before the app launches. Desktop only, and sufficient there.
//! 2. **From inside** — the temporary block on Settings. This is not a convenience: Android's
//!    `data_dir` is `getFilesDir()`, which is not writable from outside the app, and #141 found that
//!    an uninstall is not a first launch there either, because ADR-0007 §6 deliberately puts it in
//!    the Auto Backup set. A thumb tapping a button is the only route in on a handset.
//!
//! # A fixture verifies itself
//!
//! [`Fixture::install`] does not merely write rows — it recomputes the review state afterwards and
//! **fails if the collection did not land where the fixture says it lands**. That check is the point
//! rather than a nicety: a storyboard that misses its target fails *silently* and has done so twice
//! (#122, #143), and a fixture that half-installed would produce an entirely plausible picture of the
//! wrong state. The scheduler decides the intervals here, not this module, so "graded `Easy` twice is
//! not due" is a claim about `fsrs` that has to be checked rather than assumed.
//!
//! # The one state that is not data
//!
//! ADR-0006 §1's ten-minute checkpoint hangs off [`Sitting`](crate::Sitting)'s monotonic clock, not
//! off the collection, so no pre-made collection can reach it — it needs ten real minutes that no
//! capture run and no person holding a handset will ever wait for. [`checkpoint_after`] is the one
//! lever this module offers that is not a fixture: a **shorter** checkpoint, set from the environment
//! on desktop and from the same Settings block on the handset. It is subtractive — ADR-0006 §1's six
//! hundred seconds stay named at the call site and this only ever returns something else when a bench
//! override is present.

use std::collections::HashSet;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use cairn_core::content::{CardRef, DeckId, NoteId, cloze_slot};
use cairn_core::log::{DEFAULT_NEW_CARD_RATE, DayScale};
use cairn_core::replay::{leeches, replay};
use cairn_core::scheduling::Grade;
use cairn_store::Collection;

use crate::deck;
use crate::session::{self, ReviewState};

/// One day in milliseconds. Day numbers are a pure shift of the epoch (`log::day_number`), so
/// subtracting whole days from an instant decrements its day number by exactly that many — which is
/// all the backdating below needs, at any rollover hour and any offset.
const DAY_MS: i64 = 86_400_000;

/// The state a fixture put the collection into, recomputed from the rows it wrote — what
/// [`Fixture::install`] checks itself against and what the bench reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reached {
    /// Cards the current content generates.
    pub cards: usize,
    /// Cards due right now.
    pub due: usize,
    /// Cards the rate would introduce right now.
    pub new: usize,
    /// Cards over the leech floor (ADR-0010 §2).
    pub leeches: usize,
    /// What the Review destination will draw.
    pub state: ReviewState,
    /// Decks the collection holds and has not deleted — every value the note list's filter dropdown
    /// offers beside *All decks*, and every value the editor's deck dropdown offers beside *Unfiled*
    /// (ADR-0021 §9).
    pub decks: usize,
    /// Notes the note list draws — its rows, which are notes and never cards (ADR-0021 §2). Not the
    /// same number as `cards`: a cloze note generates one card per blank, and a deleted note
    /// generates neither a card nor a row.
    pub notes: usize,
    /// How many of those rows are **unfiled** (ADR-0005 §8) — carrying no `deck` reference, or one
    /// naming no deck the collection currently holds. Both are legal and still reviewable, and the
    /// note list cannot tell them apart, so neither can this.
    pub unfiled: usize,
    /// **Dormant entries across every note** — slots the log holds with kept reviews that current
    /// content no longer generates (ADR-0018 §2). Each one is a line in some note's card pane and a
    /// row in that note's destructive-edit warning (ADR-0025 §4).
    ///
    /// Counted here because a fixture that reaches dormancy cannot check itself any other way: a
    /// dormant entry is derived per note from the log against the content, so it does not appear in
    /// the card count, the queue, or the note list. Without this number a fixture whose content edit
    /// silently failed would install a perfectly ordinary collection and photograph a pane with
    /// nothing dormant in it — the plausible picture of the wrong state this module exists to refuse.
    pub dormant: usize,
    /// Notes whose pane holds dormant entries and **no live card** — ADR-0018 §6's own state, which
    /// is distinct from the empty note and says so in words.
    ///
    /// Separate from `dormant` because the two can move independently: one note losing a second
    /// blank raises `dormant` and leaves this at zero, and only a note losing *everything* reaches
    /// §6. A fixture promising both has to name both.
    pub no_live_cards: usize,
}

impl std::fmt::Display for Reached {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} cards, {} due, {} new, {} leeches — {:?}",
            self.cards, self.due, self.new, self.leeches, self.state
        )?;
        // The deck clause is written only by a fixture that reached a deck. On the six that reach
        // none, every note is unfiled by ADR-0005 §8's definition and "0 decks, 25 unfiled" is
        // therefore true — and reads as a shortfall in a fixture that was never about the deck
        // surface. Silence is the honest report there.
        if self.decks > 0 {
            write!(
                f,
                ", {} decks, {} of {} notes unfiled",
                self.decks, self.unfiled, self.notes
            )?;
        }
        // Written only by a fixture that reached dormancy, for the same reason the deck clause is:
        // "0 dormant" is true of every other fixture and reads as a shortfall in one that was never
        // about the card pane.
        if self.dormant > 0 {
            write!(
                f,
                ", {} dormant across {} notes, {} of them with nothing live",
                self.dormant, self.notes, self.no_live_cards
            )?;
        }
        Ok(())
    }
}

/// **Temporary, and not a specified feature.** One pre-made collection, named by the state it
/// reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fixture {
    /// Nothing due and nothing left to introduce: the caught-up floor, bare (ADR-0034 §3). No
    /// leeches, so the screen carries the statement and nothing else — which is the screen
    /// [#155](https://github.com/amin-bf/cairn/issues/155) puts the mark above.
    CaughtUp,
    /// Nothing due **and three cards over the leech floor**: the caught-up floor with its one
    /// control under it, and the leech screen behind that. Distinct from [`Fixture::CaughtUp`]
    /// because ADR-0035 §1's reach line was argued on a screen with a card and drawn on exactly one
    /// call site — the second screen to draw a control is this one, and it is only this one when a
    /// leech exists.
    Leeches,
    /// One card due, sitting **one failure day short** of the leech floor. Grading it *Forgot* takes
    /// it over, so the sitting ends with a leech that crossed during it — which is the only way to
    /// reach the end-of-session pointer (ADR-0010 §6), whose contents are `leeches now` minus an
    /// in-memory snapshot taken when the sitting began.
    Crossing,
    /// Twenty-five cards due: past `session::COMFORTABLE_SITTING`, so Review frames a backlog, and
    /// past every shorter sitting, so the entrance's second line offers all three of 5/10/20. That
    /// line is the link accent's only call site (ADR-0034 §2) and is invisible on a fresh collection,
    /// because a first-run queue is five cards and none of 5/10/20 is *shorter* than five.
    Backlog,
    /// Cloze cards due, whose two faces together **overflow** the review card's budget while the
    /// prompt alone does not — the one shape that makes ADR-0033 §4's step-down fire, and the one
    /// the shipping seed cannot produce at all.
    ///
    /// It earns its place the way the other four do: it reaches a decided state nothing else can. A
    /// cloze prompt and its answer are the *same sentence* differing by one masked word, so the card
    /// grows by a whole face at the reveal where a vocabulary card grows by a word — and until
    /// [ADR-0037 §5](../../../docs/adr/0037-motion-and-elevation.md) the tier was recomputed on that
    /// larger content, so the prompt was drawn at **display before the tap and heading after it**.
    /// That shipped from the day ADR-0033 landed, at 560 only, and no capture in this repository
    /// could have shown it: every one of them is of six French words, and no card in that seed is
    /// long enough to step down.
    ///
    /// **The four notes are chosen to span the outcomes, not to look plausible.** One fits on a line
    /// at both judging widths; two wrap at 560 and not at 1280, so the two widths disagree about the
    /// card's height; one has two blanks, so a single note generates two cards and the second card's
    /// prompt shows the first blank already filled.
    Cloze,
    /// Cards **due** *and* leeches at the same time — the picker with the durable entrance under it,
    /// which is the second screen [ADR-0038 §5](../../../docs/adr/0038-the-mark-and-the-icon-rule.md)
    /// moved and the only one it moved without anybody looking.
    ///
    /// **It exists because a decision made a state decided.** Making ADR-0035 §1 a page rule sends
    /// *every* screen's last control to the reach line, and the leech entrance draws in both Review
    /// states, not only the caught-up one. `leeches` cannot reach this — a leech there is
    /// deliberately not due, because a card whose latest grade is a failure is due whatever its
    /// interval says, and a due card would put the screen back into the card state rather than the
    /// picker.
    ///
    /// So it is `Leeches`'s three recovered leeches over `Backlog`'s pile of due cards: the two
    /// halves already exist and neither is re-tuned, which keeps this fixture a *composition* rather
    /// than a third set of intervals to keep true. It is the ordinary case in real use — leeches
    /// earned months ago, cards due today — and it had no picture at all.
    DueWithLeeches,
    /// **Decks with notes in them, and a script the shipping seed has no word in** — the note list's
    /// whole deck surface, and a right-to-left row.
    ///
    /// It is the first fixture that names a state on **Notes** rather than on Review. No other
    /// fixture, and no seed, creates a deck at all, so ADR-0021 §9's filter dropdown, *New deck* and
    /// *Delete deck* have only ever been photographed holding the one value that exists when no deck
    /// does — *All decks* — and the question of whether a row carries its deck has had nothing to
    /// carry. The same holds for direction: every capture in this repository is of French, and the
    /// one right-to-left row the pass has ever seen took eleven storyboard steps driving `xdotool`
    /// into a field, which is how
    /// [#150](https://github.com/amin-bf/cairn/issues/150)'s row defect went eleven months unseen.
    ///
    /// **The four decks are chosen to span the outcomes, not to look plausible.** More than two, so
    /// the dropdown is a list rather than a pair; two named long enough to ask what a deck name does
    /// to whatever has to carry it; one named in Persian, so the dropdown itself holds a
    /// right-to-left entry; and notes left **unfiled** beside the filed ones, which is a legal state
    /// (ADR-0005 §8) with its own value in the editor's dropdown and no picture anywhere.
    ///
    /// **Its deck ids are fixed and known** — see [`DECKS`], which is the whole reason this cannot
    /// be built on [`Collection::create_deck`].
    ///
    /// **It adds no third claim about `fsrs`.** Every note is scheduled ahead by
    /// [`SCHEDULED_AHEAD`], the pair [`Fixture::CaughtUp`] already asserts lands, so Review draws the
    /// caught-up floor and the list is what there is to look at.
    Decks,
    /// **Cards with kept history that current content no longer generates** — the card pane's other
    /// two entry shapes, and the destructive-edit warning above the fields.
    ///
    /// ADR-0018 gives the pane **three** entry shapes where ADR-0012 §1 described one: a card, a
    /// dormant line (§2), and a statement for a note that currently generates none (§6). Two of the
    /// three had never been drawn by anything, and neither had ADR-0025 §4's warning — because
    /// dormancy needs a slot with **kept reviews** that content stopped generating, and every fixture
    /// in the bench was built for a queue state or a card *shape*. `cloze` comes closest and misses:
    /// its cards are new, and `cards::card_pane` skips a logged slot whose kept reviews are zero,
    /// *"there is no history to warn about"*. So switching a `cloze` fixture note's kind produces no
    /// dormant entry, correctly, and photographs nothing.
    ///
    /// That is the bench's own gap rather than a screen's: the state is a collection state, reachable
    /// in principle, and no fixture asked for it. Same shape as
    /// [#153](https://github.com/amin-bf/cairn/issues/153)'s ten-minute checkpoint and the opposite
    /// answer — that one *could* not be a collection, this one simply was not.
    ///
    /// **The three notes span ADR-0018 §3's three naming cases, not a plausible collection.** A cloze
    /// note reviewed on two blanks and edited down to one reaches §2's line beside a live card and
    /// §3's case 2, *"blank 2"*. A `vocab` note reviewed on both directions and switched to an empty
    /// `cloze` reaches §6 — nothing live, everything dormant — and §3's case 1, the field **roles**
    /// *"Term → Meaning"*, which are named rather than shown because the content is exactly what is
    /// gone. And a note carrying history at a slot no shipped kind declares reaches §3's case 3,
    /// *"card 7"* — the unnameable one the ADR says is **shown, never hidden**, which is what a note
    /// switched back out of an acquired kind leaves behind (ADR-0017 §6).
    Dormant,
}

impl Fixture {
    /// Every fixture, in the order the Settings block draws them.
    pub const ALL: [Fixture; 8] = [
        Fixture::CaughtUp,
        Fixture::Leeches,
        Fixture::Crossing,
        Fixture::Backlog,
        Fixture::Cloze,
        Fixture::DueWithLeeches,
        Fixture::Decks,
        Fixture::Dormant,
    ];

    /// The name the harness and the storyboard use. Stable — a storyboard names its fixture, so
    /// renaming one silently changes what a capture is a picture of.
    pub fn key(self) -> &'static str {
        match self {
            Fixture::CaughtUp => "caught-up",
            Fixture::Leeches => "leeches",
            Fixture::Crossing => "crossing",
            Fixture::Backlog => "backlog",
            Fixture::Cloze => "cloze",
            Fixture::DueWithLeeches => "due-with-leeches",
            Fixture::Decks => "decks",
            Fixture::Dormant => "dormant",
        }
    }

    /// The button label on Settings — what the collection will be, not what the button does.
    pub fn label(self) -> &'static str {
        match self {
            Fixture::CaughtUp => "Nothing due",
            Fixture::Leeches => "Leeches",
            Fixture::Crossing => "About to leech",
            Fixture::Backlog => "A backlog",
            Fixture::Cloze => "Cloze cards",
            Fixture::DueWithLeeches => "Due, with leeches",
            Fixture::Decks => "Decks, and Persian",
            Fixture::Dormant => "Dormant cards",
        }
    }

    /// The screen this fixture exists to make reachable, in one line.
    pub fn reaches(self) -> &'static str {
        match self {
            Fixture::CaughtUp => "the caught-up floor, with no control under it",
            Fixture::Leeches => "the caught-up floor with the leech entrance, and the leech screen",
            Fixture::Crossing => "the end-of-session pointer, after one Forgot",
            Fixture::Backlog => "a framed backlog, and the entrance's shorter-sitting line",
            Fixture::Cloze => "a card that steps down, and a reveal that grows by a whole face",
            Fixture::DueWithLeeches => "the picker with the leech entrance on the reach line",
            Fixture::Decks => "the deck filter with decks in it, and a right-to-left row",
            Fixture::Dormant => "a dormant line, a pane with nothing live, and the edit warning",
        }
    }

    /// The fixture a storyboard or a command line named, or `None` — an unknown key is refused
    /// rather than guessed, so a typo aborts the capture run instead of photographing the seed.
    pub fn parse(key: &str) -> Option<Fixture> {
        Fixture::ALL.into_iter().find(|f| f.key() == key)
    }

    /// Write this fixture into an **empty** collection and verify it landed.
    ///
    /// `now_ms` is a value, as everywhere else in this workspace (ADR-0009 §8): the caller reads the
    /// clock. The collection must be empty — the bench replaces a collection rather than adding to
    /// one, and a fixture written on top of existing rows reaches a state nobody named.
    ///
    /// Returns what the collection actually reached, or the mismatch. **The verification is the
    /// interesting half.** The rows written here are chosen so the scheduler lands them where the
    /// fixture says, and that is a claim about `fsrs`'s intervals rather than about this code — a
    /// pinned dependency can still move under a version bump, and the failure mode without this
    /// check is a plausible picture of the wrong screen.
    pub fn install(self, coll: &mut Collection, now_ms: i64) -> Result<Reached, String> {
        if !coll.is_empty().map_err(|e| e.to_string())? {
            return Err("the bench installs into an empty collection — reset first".to_owned());
        }

        let mut history = History::default();
        match self {
            Fixture::CaughtUp => caught_up(coll, &mut history, 12)?,
            Fixture::Leeches => {
                caught_up(coll, &mut history, 9)?;
                // Leeches that are **not due**: failure days inside the ninety-day window, then a
                // recovery long enough to schedule each one ahead. Both halves are needed — a card
                // whose *last* grade is a failure is due whatever its interval says
                // (`session::compose`), so a leech that simply failed its way over the floor cannot
                // coexist with a caught-up screen, and a card that has failed that often still has
                // too little stability to leave the queue on a single pass. The three records
                // differ from each other on purpose; [`LEECHES`] is where that is argued.
                for leech in &LEECHES {
                    let card = note(coll, leech.front, leech.back)?;
                    history.fails(card, leech.fails);
                    history.passes(card, leech.passes);
                }
            }
            Fixture::Crossing => {
                caught_up(coll, &mut history, 5)?;
                // Three failure days, and the last grade a failure — so the card is due today and
                // one more *Forgot*, on a fourth distinct day, takes it over the floor during the
                // sitting rather than before it.
                let card = note(coll, "l'écureuil", "the squirrel")?;
                history.fails(card, &[40, 30, 20]);
            }
            Fixture::Backlog => {
                // One pass, long enough ago that the interval it bought has expired several times
                // over. Nothing here is a leech: a backlog is a person who stopped reviewing, not a
                // person who kept failing.
                for (front, back) in BACKLOG_WORDS {
                    let card = note(coll, front, back)?;
                    history.passes(card, &[(60, Grade::Good)]);
                }
            }
            // **A composition of the two above, deliberately re-tuning neither.** The leeches are
            // `Leeches`'s exactly — [`LEECHES`] entire, ranks and all — so they are leeches *and not
            // due*, which is what lets due cards decide the screen. The due half is `Backlog`'s
            // expired pass. Both sets of intervals are already asserted by their own fixtures, so
            // this one adds no third claim about `fsrs` to keep true.
            Fixture::DueWithLeeches => {
                for (front, back) in BACKLOG_WORDS {
                    let card = note(coll, front, back)?;
                    history.passes(card, &[(60, Grade::Good)]);
                }
                for leech in &LEECHES {
                    let card = note(coll, leech.front, leech.back)?;
                    history.fails(card, leech.fails);
                    history.passes(card, leech.passes);
                }
            }
            Fixture::Cloze => {
                // **No history at all.** The state this fixture exists for is a card's *shape*, not
                // its scheduling, and a new card is the cheapest way to put one on screen: the rate
                // introduces all five, so Review draws a cloze card on the first frame of a capture
                // run. Backdating them would add a claim about `fsrs` that this fixture is not about.
                for text in CLOZE_TEXTS {
                    cloze_note(coll, text)?;
                }
            }
            // **The decks are written before the notes, at ids this module names.** Nothing else in
            // the bench writes a `deck` entity at all, and nothing anywhere writes one at a known
            // id — [`Collection::create_deck`] mints a fresh UUIDv4 per call, which is right for a
            // person making a deck and useless to anything that has to *match* one (see [`DECKS`]).
            //
            // The notes are created in table order, so the `position` keys `create_note` assigns run
            // in that order too and the list draws them as the table reads (ADR-0021 §3). The unfiled
            // ones are interleaved rather than gathered at the end, because a filed run followed by
            // an unfiled run is a picture of the table rather than of a collection.
            Fixture::Decks => {
                let mut ids = Vec::with_capacity(DECKS.len());
                for (id, name) in DECKS {
                    ids.push(deck(coll, id, name)?);
                }
                for (filed_under, front, back) in DECK_NOTES {
                    let card = note(coll, front, back)?;
                    if let Some(index) = filed_under {
                        file(coll, card.note, ids[index])?;
                    }
                    history.passes(card, &SCHEDULED_AHEAD);
                }
            }
            // **The history is written against the content that generated it, and the content is
            // then changed** — which is the whole mechanism, and the reason this fixture cannot be
            // expressed as rows the way the others can. A dormant slot is not a value anybody
            // stores: it is the *difference* between what the log holds and what the note currently
            // says, recomputed per frame (ADR-0012 §5), so reaching one means writing both sides and
            // letting them disagree.
            //
            // The edits below reach past `editor::commit_field` to `mutable_set` for the same reason
            // `deck` reaches past `create_deck`: the editor is what a person uses and a fixture must
            // not need one. What is written is byte-identical to what the editor writes.
            Fixture::Dormant => {
                // Filler, so the collection is a collection and Review has a screen to draw.
                caught_up(coll, &mut history, 6)?;

                // 1. Two blanks reviewed, then the second one taken out of the text. Blank 1 stays
                //    live and blank 2 becomes a line beside it — ADR-0018 §1's interleaving in raw
                //    slot order, with the live card first because 0x8001 sorts below 0x8002.
                let pruned = cloze_note(coll, DORMANT_CLOZE_BEFORE)?;
                history.passes(blank(pruned, 1), &SCHEDULED_AHEAD);
                history.passes(blank(pruned, 2), &SCHEDULED_AHEAD);
                set_field(coll, pruned, "Text", DORMANT_CLOZE_AFTER)?;

                // 2. ADR-0018 §6's own worked example, verbatim: *"switch a reviewed `vocab` note to
                //    `cloze` and type nothing, and every entry is dormant"*. Both directions carry
                //    history, `cloze` with no text generates nothing, so the pane has two lines and
                //    no card. Its `Term` and `Meaning` values stay on the surface untouched — the
                //    fields simply stop being the ones the kind declares, which is what makes the
                //    roles rather than the content the only thing left to name it by.
                let emptied = vocab_note(coll, "la falaise", "the cliff")?;
                history.passes(CardRef::new(emptied, VOCAB_TERM_SLOT), &SCHEDULED_AHEAD);
                history.passes(CardRef::new(emptied, VOCAB_MEANING_SLOT), &SCHEDULED_AHEAD);
                set_field(coll, emptied, "kind", "cloze")?;
                set_field(coll, emptied, "Text", "")?;

                // 3. History at a slot **no shipped kind declares**, which is what a note switched
                //    back out of an acquired kind leaves behind (ADR-0017 §6): the stranger's slot
                //    numbering is not ours, so nothing can name the question it asked. ADR-0018 §3
                //    case 3 says show it anyway — an unnameable dormant card is still history
                //    attached to this note, and dropping it is the header-counter failure at its
                //    limit. The note itself is an ordinary `basic` one, so its own card stays live
                //    beside the line.
                let returned = note(coll, "le hêtre", "the beech")?;
                history.passes(returned, &SCHEDULED_AHEAD);
                history.passes(CardRef::new(returned.note, STRANGER_SLOT), &SCHEDULED_AHEAD);
            }
        }
        history.write(coll, now_ms)?;

        let reached = self.check(coll, now_ms)?;
        Ok(reached)
    }

    /// Recompute the state and hold it against what this fixture promises. Split out of
    /// [`Fixture::install`] so a test can name the expectation it is really asserting.
    fn check(self, coll: &Collection, now_ms: i64) -> Result<Reached, String> {
        let reached = read(coll, now_ms)?;
        let complaint = match self {
            Fixture::CaughtUp => match reached {
                Reached {
                    state: ReviewState::CaughtUp,
                    leeches: 0,
                    ..
                } => None,
                _ => Some("nothing due and no leeches"),
            },
            Fixture::Leeches => match reached {
                Reached {
                    state: ReviewState::CaughtUp,
                    leeches: 3,
                    ..
                } => None,
                _ => Some("nothing due and exactly three leeches"),
            },
            Fixture::Crossing => match reached {
                Reached {
                    state: ReviewState::Due { due: 1, new: 0, .. },
                    leeches: 0,
                    ..
                } => None,
                _ => Some("exactly one card due and no leech yet"),
            },
            Fixture::Backlog => match reached {
                Reached {
                    state:
                        ReviewState::Due {
                            due,
                            new: 0,
                            backlog: true,
                        },
                    leeches: 0,
                    ..
                } if due > 20 => None,
                _ => Some("a backlog of more than twenty due, no new and no leeches"),
            },
            // **Five cards, four offered**, and the gap is the specification rather than a
            // shortfall: ADR-0011 §7 introduces at most **one card per note per day**, so the
            // two-blank note's second card is held back until tomorrow. Both numbers are checked
            // because each pins something different — `cards` pins that cloze generated what its
            // text says it generates (they are computed from content rather than declared, ADR-0002
            // §5, so a change to the blank parser hands this fixture a different collection), and
            // `new` pins the introduction rule that made 5 into 4.
            //
            // Nothing has ever been reviewed here, so the screen is `NewDeck` and not `Due` — the
            // two are indistinguishable from the queue alone (`session`), and this fixture is the
            // one in the bench that reaches the first.
            Fixture::Cloze => match reached {
                Reached {
                    cards: 5,
                    due: 0,
                    new: 4,
                    leeches: 0,
                    state: ReviewState::NewDeck { new: 4 },
                    // Four notes behind five cards, stated rather than implied: it is the whole of
                    // what "one note, two blanks, two cards" means, and `cards` alone cannot say it.
                    // No deck, so every note is unfiled — the definition, not a shortfall
                    // (ADR-0005 §8).
                    decks: 0,
                    notes: 4,
                    unfiled: 4,
                    // Nothing has been reviewed, so no slot can keep history and nothing can be
                    // dormant. Stated rather than elided with `..` because this pattern is
                    // deliberately exhaustive: a field added to `Reached` should have to be answered
                    // here, and that is exactly how `Fixture::Dormant` got these two answered.
                    dormant: 0,
                    no_live_cards: 0,
                } => None,
                _ => Some(
                    "five cloze cards from four notes, four of them offered under the \
                     one-per-note rule, on a deck nothing has been reviewed on",
                ),
            },
            // **Both halves are asserted, and the leech count is the load-bearing one.** A leech
            // that drifted back into the queue would make this `Due` with the right number of cards
            // and no entrance under the picker — the screen this fixture exists for, missing the
            // only thing it exists to show, looking entirely plausible.
            Fixture::DueWithLeeches => match reached {
                Reached {
                    state: ReviewState::Due { due, new: 0, .. },
                    leeches: 3,
                    ..
                } if due > 20 => None,
                _ => Some("more than twenty due, no new, and exactly three leeches beside them"),
            },
            // **Counted off the tables rather than written out, because the tables are the
            // specification.** A literal here would be a second statement of the same three numbers,
            // and the one that goes stale silently when somebody adds a deck — which is exactly the
            // shape #150 named: a rule stated in two places where only one of them is read.
            //
            // All three are load-bearing and each pins something different. `decks` pins that the
            // fixed ids parsed and landed as *held, non-deleted* decks, which is the half
            // [`Collection::decks`] can refuse without anything failing. `notes` pins that every row
            // reached the list — a note filed under a deck flagged deleted is derived deleted and
            // silently absent (ADR-0005 §7). And `unfiled` pins that the notes the table leaves
            // unfiled are still unfiled: it is the number a typo in a deck id would move, because a
            // note filed under an id no deck holds is not an error — it is *unfiled*, legally, with
            // nothing failing (ADR-0005 §8).
            Fixture::Decks => {
                let wanted_unfiled = DECK_NOTES
                    .iter()
                    .filter(|(deck, ..)| deck.is_none())
                    .count();
                match reached {
                    Reached {
                        state: ReviewState::CaughtUp,
                        leeches: 0,
                        decks,
                        notes,
                        unfiled,
                        ..
                    } if decks == DECKS.len()
                        && notes == DECK_NOTES.len()
                        && unfiled == wanted_unfiled =>
                    {
                        None
                    }
                    _ => Some(
                        "every deck and every note the fixture's own tables name, with the \
                         unfiled ones unfiled, on a collection with nothing due",
                    ),
                }
            }
            // **Four dormant entries across three notes, one of them with nothing live.** Both
            // numbers are load-bearing and they fail in different directions, which is why neither
            // is implied by the other.
            //
            // `dormant: 4` pins that all three content edits actually took: the pruned blank, the
            // two vocab directions, and the stranger slot. Any one of them silently not landing —
            // a `mutable_set` that wrote to the wrong attribute, a blank parser that stopped seeing
            // `{{2::…}}`, a kind change that did not re-derive live slots — leaves a collection that
            // installs cleanly and photographs a pane with fewer lines than the fixture is named for.
            //
            // `no_live_cards: 1` pins the one thing a count cannot say: that ADR-0018 §6's state was
            // actually reached rather than approximated. Emptying the vocab note's *fields* instead
            // of its kind would leave a `vocab` note generating two cards with empty faces — four
            // dormant entries collection-wide, `dormant` satisfied, and §6 nowhere on screen. That is
            // the exact shape #163 walked into by hand before this fixture existed.
            Fixture::Dormant => match reached {
                Reached {
                    dormant: 4,
                    no_live_cards: 1,
                    leeches: 0,
                    ..
                } => None,
                _ => Some(
                    "four dormant entries across three notes, one of them a note with \
                     nothing live, and no leeches",
                ),
            },
        };
        match complaint {
            None => Ok(reached),
            Some(wanted) => Err(format!(
                "fixture '{}' wanted {wanted}, reached {reached}",
                self.key()
            )),
        }
    }
}

/// Wipe whatever is in the platform's two directories (ADR-0007 §6) and install `fixture` there —
/// the **outside** way in, and the whole of what the `cairn-fixture` binary does.
///
/// This lives here rather than in `cairn-desktop` because that crate is a shim with no logic
/// (ADR-0003 §5): anything written there is never compiled by the Android build and never exercised
/// on the handset, which is the same class of defect as a runtime platform check. It is not
/// desktop-only by construction — it is simply useless on Android, where nothing outside the app can
/// write `getFilesDir()` and the Settings block is the route instead.
///
/// **The application must not be running.** SQLite's `-wal` and `-shm` siblings are unlinked here,
/// and a live connection would be left checkpointing into inodes nothing can reach. The capture
/// harness satisfies this by installing before it starts the app.
pub fn install_into_platform_dirs(fixture: Fixture) -> Result<Reached, String> {
    let data = cairn_store::platform::data_dir().map_err(|e| e.to_string())?;
    let state = cairn_store::platform::state_dir().map_err(|e| e.to_string())?;
    cairn_store::remove_files(&data, &state);
    let mut coll = Collection::open(&data, &state).map_err(|e| e.to_string())?;
    fixture.install(&mut coll, crate::now_ms())
}

/// What the two destinations with state behind them would draw against this collection right now —
/// the same reads `screens::review` and `screens::notes` make each frame, in one place the bench can
/// assert on.
///
/// The deck half is read the way the **note list** reads it and not the way the store does:
/// [`notes::list`](crate::notes::list) with no filter is the rows, and a row is unfiled when its
/// reference names no deck [`Collection::decks`] returns. That is the definition ADR-0005 §8 gives
/// and the one the screen acts on — a note pointing at a deck that was never held and a note
/// pointing at nothing are the same thing to it, so a fixture cannot be checked against a stricter
/// one.
pub fn read(coll: &Collection, now_ms: i64) -> Result<Reached, String> {
    let today = cairn_core::log::day_number(now_ms, DayScale::default());
    let current = deck::current_cards(coll).map_err(|e| e.to_string())?;
    let positions = deck::note_positions(coll).map_err(|e| e.to_string())?;
    let rate = coll.new_card_rate().unwrap_or(DEFAULT_NEW_CARD_RATE) as usize;
    let suspended = coll.suspended().map_err(|e| e.to_string())?;
    let lines = coll.log_lines().map_err(|e| e.to_string())?;
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    let replayed = replay(&current, &refs);
    let queue = session::compose(&current, &positions, &replayed, today, rate, &suspended);
    let reviewed_ever = !replayed.cards.is_empty();

    let decks = coll.decks().map_err(|e| e.to_string())?;
    let held: HashSet<String> = decks.iter().map(|(id, _)| id.to_canonical()).collect();
    let rows =
        crate::notes::list(coll, &crate::notes::Filter::default()).map_err(|e| e.to_string())?;
    let unfiled = rows
        .iter()
        .filter(|row| !row.deck.as_deref().is_some_and(|d| held.contains(d)))
        .count();

    // Dormancy is read the way the **card pane** reads it, per note, for the same reason the deck
    // half is read the way the note list reads it: a fixture is checked against the screen's own
    // definition or against nothing useful. `card_pane` is the one place that holds the log against
    // the content, and it is what the editor draws.
    let mut dormant = 0;
    let mut no_live_cards = 0;
    for row in &rows {
        let pane = crate::cards::card_pane(coll, row.id).map_err(|e| e.to_string())?;
        dormant += pane.warning.as_ref().map_or(0, |w| w.dormant.len());
        if pane.state == crate::cards::State::NoLiveCards {
            no_live_cards += 1;
        }
    }

    Ok(Reached {
        cards: current.len(),
        due: queue.due.len(),
        new: queue.new.len(),
        leeches: leeches(&replayed, today).len(),
        state: ReviewState::of(&queue, current.len(), reviewed_ever),
        decks: decks.len(),
        notes: rows.len(),
        unfiled,
        dormant,
        no_live_cards,
    })
}

/// The reviews a fixture wants, gathered before any of them is written.
///
/// **Gathering is the mechanism, not a tidiness.** The store guards on write and rewrites any
/// instant at or below the highest already in the log (ADR-0004 §8) — a backwards clock must not sort
/// into the past — so a row written out of order arrives at `highest + 1` and lands on the wrong
/// **day**. A fixture that builds one card's whole history and then starts the next one therefore
/// backdates nothing after the first card: every later row is silently stamped a millisecond after
/// the newest one already there.
///
/// That is not hypothetical. Written per card, the leech fixture's four failure days at 80, 60, 40
/// and 20 days ago all collapsed onto the same recent day, leaving a card with **one** failure day
/// and no leech — a collection that looked entirely plausible and was not the state the fixture
/// names. It was [`Fixture::check`] that caught it, which is the argument for that check in one line.
#[derive(Default)]
struct History {
    /// `(days before now, card, grade)`, in whatever order the fixture happened to build them.
    rows: Vec<(i64, CardRef, Grade)>,
}

impl History {
    /// Grade this card *Forgot* on each of these days — one **failure day** apiece (ADR-0010 §2).
    fn fails(&mut self, card: CardRef, days_ago: &[i64]) {
        for &day in days_ago {
            self.rows.push((day, card, Grade::Forgot));
        }
    }

    /// Grade this card at the given days and grades, oldest first.
    fn passes(&mut self, card: CardRef, reviews: &[(i64, Grade)]) {
        for &(day, grade) in reviews {
            self.rows.push((day, card, grade));
        }
    }

    /// Write every gathered review, **oldest first across the whole collection**, so the store's
    /// write guard never fires and every row keeps the day it was given.
    ///
    /// Rows sharing a day are separated by a millisecond each — not to satisfy the guard, which would
    /// bump them harmlessly, but so the log has a total order that does not depend on it. The offsets
    /// run *backwards* from the anchor, so no fixture row is ever stamped in the future.
    fn write(mut self, coll: &mut Collection, now_ms: i64) -> Result<(), String> {
        self.rows.sort_by_key(|(days_ago, _, _)| -*days_ago);
        let last = self.rows.len().saturating_sub(1) as i64;
        for (index, (days_ago, card, grade)) in self.rows.into_iter().enumerate() {
            let instant = now_ms - days_ago * DAY_MS - (last - index as i64);
            coll.append_review(card, grade, instant, DayScale::default(), 4_200)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

/// **One pass then another, far enough ahead that neither is due again** — the two reviews that put
/// a card out of the queue without putting it near the leech floor.
///
/// Named once because two fixtures now rest on it, and because what it claims is about `fsrs` rather
/// than about this module: that *Good* twenty days ago followed by *Easy* two days ago buys an
/// interval longer than two days. [`Fixture::CaughtUp`]'s check is what asserts it, so a fixture
/// reusing this pair inherits an assertion rather than adding a second one to keep true.
const SCHEDULED_AHEAD: [(i64, Grade); 2] = [(20, Grade::Good), (2, Grade::Easy)];

/// The cloze text [`Fixture::Dormant`] reviews, and the text it is edited down to. **Two constants
/// rather than one and an edit**, so the diff between them is readable as the thing the fixture is
/// about: blank 2 stops being a blank and stays in the sentence as ordinary words, which is the
/// commonest way a card really goes dormant — a person tidying a note, not deleting one.
const DORMANT_CLOZE_BEFORE: &str = "Le renard {{1::traverse}} le {{2::pré}} au crépuscule";
/// The same sentence with the second blank unwrapped. Blank 1 is untouched and stays live, so the
/// pane holds a card and a line at once — ADR-0018 §2's cost, *"two dormant lines above a live card
/// do not cost the pane its job"*, with something to look at rather than an argument.
const DORMANT_CLOZE_AFTER: &str = "Le renard {{1::traverse}} le pré au crépuscule";

/// `vocab`'s two card slots (`content::VOCAB`): Term → Meaning, and Meaning → Term. Named here
/// because [`Fixture::Dormant`] logs history against them *before* the note stops being a `vocab`,
/// and a bare `2` and `3` at that call site would be two numbers with no way to tell they came from
/// a kind definition rather than from nowhere.
const VOCAB_TERM_SLOT: u16 = 2;
/// The reverse direction. See [`VOCAB_TERM_SLOT`].
const VOCAB_MEANING_SLOT: u16 = 3;

/// A fixed-arity slot **no shipped kind declares**, so `cards::dormant_name` can find no roles for
/// it and falls to ADR-0018 §3's case 3, *"card 7"*.
///
/// It stands for an **acquired** kind's slot — a note imported under a stranger's kind definition,
/// reviewed, then switched back to a shipped one (ADR-0017 §6, which permits exactly that and
/// forbids switching *into* a stranger's kind). The slot numbering that produced it was never ours,
/// which is the whole reason nothing can name the question it asked.
///
/// **Below [`CLOZE_SLOT_BIT`], deliberately.** ADR-0017 §3 partitions on the high bit, so a value
/// with it set would be read as a cloze blank and named *"blank 7"* — a picture of case 2 filed
/// under case 3, with nothing failing. 7 is not in `SHIPPED_KINDS`; the test below is what keeps
/// that true when a kind gains a card.
const STRANGER_SLOT: u16 = 7;

/// `count` notes, each reviewed twice and scheduled well ahead — the filler every fixture needs so a
/// screen has a collection behind it rather than one card.
fn caught_up(coll: &mut Collection, history: &mut History, count: usize) -> Result<(), String> {
    for (front, back) in CAUGHT_UP_WORDS.iter().take(count) {
        let card = note(coll, front, back)?;
        history.passes(card, &SCHEDULED_AHEAD);
    }
    Ok(())
}

/// One `basic` note and the single card it generates (`content::BASIC` declares one slot, ordinal 0).
fn note(coll: &mut Collection, front: &str, back: &str) -> Result<CardRef, String> {
    let id: NoteId = coll
        .create_note("basic", &[("Front", front), ("Back", back)])
        .map_err(|e| e.to_string())?;
    Ok(CardRef::new(id, 0))
}

/// One `cloze` note. **No `CardRef` comes back**, and that is the kind's own shape rather than a
/// shortcut: a cloze note's cards are not fixed, being one per numbered blank at `cloze_slot(n)`
/// (ADR-0002 §5, ADR-0017 §3), so the ordinals depend on the text and this helper would have to
/// re-parse it to name them. Nothing in [`Fixture::Cloze`] needs one — it writes no history.
fn cloze_note(coll: &mut Collection, text: &str) -> Result<NoteId, String> {
    coll.create_note("cloze", &[("Text", text)])
        .map_err(|e| e.to_string())
}

/// The card a `cloze` note's blank numbered `n` occupies — `cloze_slot(n)`, the high bit set
/// (ADR-0017 §3). Written out rather than left at the call site because a fixture naming a blank's
/// slot by hand is naming the partition, and the partition is the one thing that must not be
/// restated (ADR-0018 §1: the mask is a name, never a sort key).
fn blank(note: NoteId, n: u16) -> CardRef {
    CardRef::new(note, cloze_slot(n))
}

/// One `vocab` note and the two directions it generates (slots 2 and 3, `content::VOCAB`). The two
/// `shown-with` fields are left empty: they follow `Term` wherever it lands (ADR-0002 §3) and add
/// nothing to a note that exists to lose its cards.
fn vocab_note(coll: &mut Collection, term: &str, meaning: &str) -> Result<NoteId, String> {
    coll.create_note("vocab", &[("Term", term), ("Meaning", meaning)])
        .map_err(|e| e.to_string())
}

/// Write one value onto a note's mutable surface — **the same row the editor writes** through
/// `editor::commit_field` (ADR-0004 §7, ADR-0021 §7), reached directly for the reason [`deck`]
/// reaches past `create_deck`: a fixture must not need the screen it exists to photograph.
///
/// `kind` is an ordinary attribute here and that is not a shortcut — ADR-0017 §5 makes a kind change
/// an ordinary edit rather than a special mechanism, and the editor's own dropdown writes exactly
/// this row. An empty `value` is written as a **SQL NULL**, matching `commit_field`'s clearing arm,
/// so a field emptied by a fixture and a field emptied by a person are the same row.
fn set_field(coll: &mut Collection, note: NoteId, attr: &str, value: &str) -> Result<(), String> {
    let stored = (!value.is_empty()).then_some(value);
    coll.mutable_set("note", &note.0, attr, stored)
        .map_err(|e| e.to_string())
}

/// One deck, **at the id given rather than a minted one** — the whole of what [`DECKS`] buys, and the
/// one place in the workspace a deck comes into being without drawing entropy.
///
/// It writes the same single `name` row [`Collection::create_deck`] writes, onto the same mutable
/// surface, so the deck it makes is not a special kind of deck: ADR-0005 §4's *"a deck is `{ id,
/// name }`"* is satisfied by both, and `create_deck` is simply that plus the mint. This reaches past
/// it rather than through it because minting is exactly the part a fixture must not do.
///
/// An id that is not canonical text is refused rather than skipped. The alternative — filing notes
/// under a deck that was never written — is *legal*: those notes are unfiled (ADR-0005 §8), the list
/// draws them, and nothing fails.
fn deck(coll: &mut Collection, id: &str, name: &str) -> Result<DeckId, String> {
    let deck = DeckId::parse_canonical(id)
        .ok_or_else(|| format!("deck id '{id}' is not canonical UUID text"))?;
    coll.mutable_set("deck", &deck.0, "name", Some(name))
        .map_err(|e| e.to_string())?;
    Ok(deck)
}

/// File a note under a deck — its single `deck` reference, in canonical text (ADR-0005 §2, §8), the
/// same value `editor::set_note_deck` writes when the editor's dropdown changes.
fn file(coll: &mut Collection, note: NoteId, deck: DeckId) -> Result<(), String> {
    coll.mutable_set("note", &note.0, "deck", Some(&deck.to_canonical()))
        .map_err(|e| e.to_string())
}

// --- The checkpoint: the one bench state that is not a collection ------------------------------

/// No override installed. `u64::MAX` rather than zero, because **zero seconds is a legal override** —
/// a checkpoint that is due the moment a sitting starts is the cheapest way to photograph it.
const NO_OVERRIDE: u64 = u64::MAX;

static OVERRIDE_SECONDS: AtomicU64 = AtomicU64::new(NO_OVERRIDE);

/// The environment override, read once. `CAIRN_CHECKPOINT_SECONDS=5` is how the desktop harness
/// reaches ADR-0006 §1's checkpoint without waiting ten minutes for it. Unset, unparsable or absent
/// all mean *no override*, so the shipped behaviour is what you get for doing nothing.
fn env_seconds() -> u64 {
    static FROM_ENV: OnceLock<u64> = OnceLock::new();
    *FROM_ENV.get_or_init(|| {
        std::env::var("CAIRN_CHECKPOINT_SECONDS")
            .ok()
            .and_then(|raw| raw.trim().parse::<u64>().ok())
            .unwrap_or(NO_OVERRIDE)
    })
}

/// **Temporary, and not a specified feature.** The bench's shorter checkpoint, in seconds — long
/// enough that a capture run reaches the card and reveals it first, short enough that nobody waits.
pub const BENCH_CHECKPOINT_SECONDS: u64 = 8;

/// How long a sitting runs before the checkpoint is due: `specified` unless a bench override is in
/// force.
///
/// **Subtractive on purpose.** ADR-0006 §1's ten minutes stay named at the call site and this
/// function has no number of its own, so deleting the whole bench leaves the product behaviour where
/// it is written rather than where it was overridden.
pub fn checkpoint_after(specified: Duration) -> Duration {
    let override_seconds = match OVERRIDE_SECONDS.load(Ordering::Relaxed) {
        NO_OVERRIDE => env_seconds(),
        set => set,
    };
    match override_seconds {
        NO_OVERRIDE => specified,
        seconds => Duration::from_secs(seconds),
    }
}

/// **Temporary, and not a specified feature.** Shorten the checkpoint for the rest of this process —
/// the handset half of [`checkpoint_after`], since a thumb cannot set an environment variable.
pub fn set_checkpoint_after(seconds: u64) {
    OVERRIDE_SECONDS.store(seconds, Ordering::Relaxed);
}

/// Whether a bench override is in force, so the Settings block can say which state its button is in
/// rather than being a control with no readout.
pub fn checkpoint_is_shortened() -> bool {
    OVERRIDE_SECONDS.load(Ordering::Relaxed) != NO_OVERRIDE || env_seconds() != NO_OVERRIDE
}

// --- Content -----------------------------------------------------------------------------------
//
// French to English, the same shape as the shipping seed, so a capture of a fixture reads as the
// same application rather than as a different product. **They are deliberately different words**:
// a screen photographed against `chien`/`chat` is ambiguous between a fixture and the seed, and the
// silent failure this bench is most exposed to is a fixture that did not install.

/// The cloze notes, chosen to span the **outcomes** rather than to look plausible.
///
/// Each line is annotated with what it is for, because a later reader trimming this list to three
/// tidy sentences would remove the coverage without removing anything that looks like coverage —
/// which is the whole failure mode this bench exists downstream of.
const CLOZE_TEXTS: [&str; 4] = [
    // One line at both judging widths: the closest cloze comes to the seed's short card, and the
    // control that shows the step-down below is not simply *what cloze does*.
    "Le chat {{1::mange}} la souris",
    // Wraps at 560 and not at 1280, so the two widths disagree about the card's height — and it is
    // therefore the note that shows ADR-0037 §5's defect at one width and hides it at the other.
    "La Tour Eiffel a été construite pour l'Exposition universelle de {{1::1889}}",
    // Two blanks: one note, two cards, and the second card's prompt shows the first blank filled.
    "{{1::Bonjour}}, je m'appelle Amin et je {{2::travaille}} à Berlin",
    // Long enough to overflow the budget at both widths, so the step-down fires everywhere and the
    // face lands on its floor rather than merely one tier down.
    "Le Traité de Versailles, signé le 28 juin 1919 dans la galerie des Glaces, mit fin à l'état \
     de guerre entre l'Allemagne et les {{1::Alliés}}",
];

/// The decks [`Fixture::Decks`] installs, as `(canonical id, name)` — **fixed, and public because
/// something outside this module has to be able to name them.**
///
/// [`Collection::create_deck`] mints a fresh UUIDv4 per call (ADR-0005 §4), which is correct: a deck
/// is created once, on one device, and its id is what survives export, import and sync. It is also
/// why a fixture cannot use it. **Authority follows deck id** ([ADR-0008
/// §11](../../../docs/adr/0008-the-deck-export-format.md)), so an inbound `.cdeck` exercises
/// ADR-0022's *update* path — *updating a deck you already have*, *N already yours*, *notes moving in
/// from X*, *renaming your X to Y*, *X will be left empty* — only when its deck ids **match ids the
/// collection holds**. Against a collection whose ids were minted at install time, nothing can ever
/// be built to match, and every import plan this repository can reach reads *new deck*: the six lines
/// ADR-0022 exists for have never been drawn by anything, and §3's *a line that does not apply is
/// absent, never shown as zero* is what makes that invisible. From the note list's side the
/// difference is nil, which is why it has to be written down here rather than noticed there.
///
/// The ids are well-formed UUIDv4 text — version `4`, variant `8` — because that is what a deck id is
/// and an ill-formed one would be a second fact about this collection that no real one shares. They
/// are deliberately *legible*: `f1c7` reads as `fixt`, and a deck id in a capture, a `.cdeck` or a
/// log line is then identifiable at a glance as the bench's rather than a person's.
pub const DECKS: [(&str, &str); 4] = [
    ("f1c70000-0001-4000-8000-000000000001", "Français"),
    // Named in Persian, so the **dropdown** carries a right-to-left entry and not only a row. The
    // filter draws deck names through `bidi`; nothing has ever handed it one to draw.
    ("f1c70000-0002-4000-8000-000000000002", "فارسی"),
    // Two long names, which is what asks the question a short one cannot: what a deck name does to
    // whatever has to carry it — the dropdown's own width, and a row, if #162 decides a row shows its
    // deck. Long in different ways, one running to a proper noun and one to a conjunction, so a
    // truncation rule that happens to flatter one is caught by the other.
    (
        "f1c70000-0003-4000-8000-000000000003",
        "Vocabulaire de la Révolution française",
    ),
    (
        "f1c70000-0004-4000-8000-000000000004",
        "Expressions idiomatiques et proverbes",
    ),
];

/// The twenty-five notes [`Fixture::Decks`] installs, as `(deck index into [`DECKS`], Front, Back)`,
/// **in the order the list draws them** — `create_note` assigns `position` at the end of the authored
/// order (ADR-0021 §3), so table order is row order.
///
/// `None` is **unfiled** (ADR-0005 §8): a note carrying no `deck` reference at all, which is what
/// declining the editor's deck dropdown leaves behind and is a legal, still-reviewable state with no
/// picture anywhere.
///
/// Twenty-five because that is `backlog`'s number and the rhythm the pass has already judged a list
/// at — this is the bench's second *list* state, and a list is the one thing that cannot be sized
/// from three rows.
///
/// **The unfiled three are interleaved rather than gathered at the end.** A filed run followed by an
/// unfiled run is a picture of this table; a collection where the two are mixed is a picture of a
/// person's.
const DECK_NOTES: [(Option<usize>, &str, &str); 25] = [
    (Some(0), "le carrefour", "the crossroads"),
    (Some(0), "la serrure", "the lock"),
    (Some(0), "le tiroir", "the drawer"),
    // Unfiled, first of three, and early enough that *All decks* is visibly not one deck.
    (None, "le colporteur", "the pedlar"),
    (Some(0), "la cheminée", "the chimney"),
    (Some(0), "le trottoir", "the pavement"),
    // The Persian deck. Short words first, so a right-to-left row is seen at the length the row was
    // designed for before it is seen at the length that tests it.
    (Some(1), "کتاب", "book"),
    (Some(1), "پنجره", "window"),
    (Some(1), "کتابخانه", "library"),
    // Latin digits inside a right-to-left run: the neutral-run case, where the digits keep their own
    // direction inside a paragraph that does not. It is the shape a bidi bug shows up in first and
    // the one no capture in this repository holds.
    (Some(1), "پرواز شماره 302", "flight number 302"),
    // Long enough to wrap at the judging widths, so the right-to-left row is seen doing the thing
    // #150 measured it failing at — its glyphs drawn 44px outside their own button.
    (
        Some(1),
        "هر که بامش بیش، برفش بیشتر",
        "the higher the roof, the more the snow",
    ),
    (Some(0), "la poignée", "the handle"),
    (Some(0), "le grenier", "the attic"),
    (None, "la girouette", "the weather vane"),
    (Some(2), "la Bastille", "the Bastille"),
    (Some(2), "le tiers état", "the third estate"),
    (
        Some(2),
        "la Convention nationale",
        "the National Convention",
    ),
    (
        Some(2),
        "le Comité de salut public",
        "the Committee of Public Safety",
    ),
    (Some(0), "la lucarne", "the skylight"),
    // The longest preview in the collection, and a whole sentence rather than a term — the row a
    // truncation rule has to answer for, in the deck whose name is also long.
    (
        Some(3),
        "Il ne faut pas vendre la peau de l'ours avant de l'avoir tué",
        "don't count your chickens before they hatch",
    ),
    (
        Some(3),
        "Chacun voit midi à sa porte",
        "everyone sees noon at their own door",
    ),
    (
        Some(3),
        "C'est en forgeant qu'on devient forgeron",
        "practice makes perfect",
    ),
    (Some(0), "le paillasson", "the doormat"),
    (Some(0), "la véranda", "the veranda"),
    (None, "l'arrière-boutique", "the back room of a shop"),
];

const CAUGHT_UP_WORDS: [(&str, &str); 12] = [
    ("la fenêtre", "the window"),
    ("le fleuve", "the river"),
    ("la montagne", "the mountain"),
    ("le brouillard", "the fog"),
    ("la clé", "the key"),
    ("le sentier", "the path"),
    ("la pierre", "the stone"),
    ("le sommeil", "sleep"),
    ("la lumière", "the light"),
    ("le silence", "silence"),
    ("la racine", "the root"),
    ("le seuil", "the threshold"),
];

/// One leech, and the record that ranks it.
struct LeechSpec {
    front: &'static str,
    back: &'static str,
    /// Days before now on which the card was graded *Forgot* — one **failure day** each, and the
    /// primary rank key is how many of them there are (`replay::leeches`).
    fails: &'static [i64],
    /// The recovery that takes the card back out of the queue, oldest first. A card whose latest
    /// grade is a failure is due whatever its interval says, so every leech here has to pass its
    /// way out; the count also feeds the caption's second number, which is **not** a rank key.
    passes: &'static [(i64, Grade)],
}

/// The three leeches, **deliberately unequal, and unequal in two different ways**.
///
/// They were identical until [#160](https://github.com/amin-bf/cairn/issues/160) — four failure days
/// and the same last failure day apiece — so both of `replay::leeches`' rank keys tied and the order
/// fell through to the card-identity tie-break. That key is stable across *devices sharing a
/// collection* and freshly random across *builds of a fixture*, so every capture of this screen the
/// repository held showed an order the run had picked at random, and a re-run produced a diff that
/// read as a change and was not one.
///
/// The spread is chosen to photograph the rank **as it actually behaves**, not to make it look
/// easy. `néanmoins` is worst on the visible key and leads. `désormais` and `d'ailleurs` **tie on
/// it** and are separated only by which failed more recently — a key the screen does not draw — so
/// the pair is the honest test of whether the caption can carry the order at all
/// ([#156](https://github.com/amin-bf/cairn/issues/156)). Giving all three distinct counts would
/// have hidden that question behind a fixture rigged to answer it.
///
/// Review counts differ too, and their **order is chosen against the rank**: the tied pair reads
/// *"4 bad days · 9 reviews"* above *"4 bad days · 10 reviews"*, so one picture shows that the
/// caption's second number is context and not the key that ordered the list. Ordering them the
/// other way would have let a reader read the screen correctly for the wrong reason.
///
/// # The recoveries are sized, not guessed
///
/// Each record ends in enough successful reviews to put the card **7 to 14 days** clear of due, and
/// that margin is the specification rather than slack. `scheduling`'s interval fuzz is seeded from
/// the `CardRef` (ADR-0027 §5, ADR-0001 §7) — the same identity that is stable across devices and
/// **random across builds** — so a leech scheduled close to the edge is due on some builds and not
/// on others, and the fixture's own check then fails intermittently. Measured over 200 builds
/// apiece, the recovery this fixture shipped before #160 left as little as **two days** of margin;
/// these leave seven at worst. A future edit here should re-measure rather than reason: the
/// intervals are `fsrs`'s, not ours.
const LEECHES: [LeechSpec; 3] = [
    // Six failure days: worst on the primary key, and first whatever the other two do.
    LeechSpec {
        front: "néanmoins",
        back: "nevertheless",
        fails: &[86, 71, 57, 43, 29, 15],
        passes: &[
            (36, Grade::Good),
            (25, Grade::Good),
            (16, Grade::Good),
            (9, Grade::Easy),
            (4, Grade::Easy),
            (1, Grade::Easy),
        ],
    },
    // Four failure days, the more recent last failure — second, on nine reviews.
    LeechSpec {
        front: "désormais",
        back: "from now on",
        fails: &[79, 62, 45, 22],
        passes: &[
            (40, Grade::Good),
            (28, Grade::Good),
            (17, Grade::Good),
            (8, Grade::Easy),
            (3, Grade::Easy),
        ],
    },
    // Four failure days, the older last failure — third, on *more* reviews than the row above it,
    // and the screen cannot say why it is third.
    LeechSpec {
        front: "d'ailleurs",
        back: "besides",
        fails: &[84, 68, 52, 36],
        passes: &[
            (30, Grade::Good),
            (20, Grade::Good),
            (13, Grade::Good),
            (7, Grade::Good),
            (3, Grade::Good),
            (1, Grade::Easy),
        ],
    },
];

const BACKLOG_WORDS: [(&str, &str); 25] = [
    ("l'aube", "dawn"),
    ("le crépuscule", "dusk"),
    ("la marée", "the tide"),
    ("le rivage", "the shore"),
    ("la falaise", "the cliff"),
    ("le nuage", "the cloud"),
    ("la neige", "the snow"),
    ("le givre", "the frost"),
    ("la braise", "the ember"),
    ("le cendrier", "the ashtray"),
    ("la charrue", "the plough"),
    ("le moulin", "the mill"),
    ("la grange", "the barn"),
    ("le puits", "the well"),
    ("la clairière", "the clearing"),
    ("le hêtre", "the beech"),
    ("le bouleau", "the birch"),
    ("la mousse", "the moss"),
    ("le lichen", "the lichen"),
    ("la fougère", "the fern"),
    ("le ruisseau", "the stream"),
    ("le gué", "the ford"),
    ("la digue", "the dyke"),
    ("le phare", "the lighthouse"),
    ("la boussole", "the compass"),
];

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use tempfile::TempDir;

    /// A fixed instant, so a test never straddles a rollover. 4am on day 20 514 at the default
    /// scale, which is the same anchor the review screen's own tests use.
    const NOW_MS: i64 = 20_514 * DAY_MS + 4 * 3_600_000;

    fn empty() -> (TempDir, TempDir, Collection) {
        let data = TempDir::new().unwrap();
        let state = TempDir::new().unwrap();
        let coll = Collection::open(data.path(), state.path()).unwrap();
        (data, state, coll)
    }

    /// **Every fixture reaches the state it names.** This is the assertion the whole bench rests on:
    /// the rows each fixture writes are chosen so `fsrs` schedules them where the fixture says, which
    /// is a claim about a pinned dependency rather than about this code. Without it, a fixture that
    /// quietly stopped landing would produce a plausible picture of the wrong screen — which is the
    /// exact failure #122 and #143 both hit from the storyboard side.
    #[test]
    fn every_fixture_lands_where_it_says_it_lands() {
        for fixture in Fixture::ALL {
            let (_d, _s, mut coll) = empty();
            let reached = fixture
                .install(&mut coll, NOW_MS)
                .unwrap_or_else(|e| panic!("{}: {e}", fixture.key()));
            assert!(
                reached.cards > 0,
                "{}: a fixture with no cards is not a fixture",
                fixture.key()
            );
        }
    }

    /// **The three ways ADR-0018 §3 names a dormant entry, all reached by one fixture.** The install
    /// check counts them; only this can say they are *the three cases*, and the cases are the reason
    /// the fixture holds three notes rather than one with three dead slots.
    ///
    /// Nothing fails when this drifts. A stranger slot that a kind later declares still counts as
    /// dormant, still draws a line, and quietly becomes a picture of case 1 filed under case 3 —
    /// which is the same shape as photographing the wrong screen under the right name.
    #[test]
    fn the_dormant_fixture_reaches_all_three_of_adr_0018_s_naming_cases() {
        let (_d, _s, mut coll) = empty();
        Fixture::Dormant.install(&mut coll, NOW_MS).unwrap();

        let rows = crate::notes::list(&coll, &crate::notes::Filter::default()).unwrap();
        let mut names: Vec<String> = Vec::new();
        for row in &rows {
            let pane = crate::cards::card_pane(&coll, row.id).unwrap();
            if let Some(warning) = &pane.warning {
                names.extend(warning.dormant.iter().map(|d| d.name.clone()));
            }
        }
        names.sort();

        assert_eq!(
            names,
            vec![
                // Case 1 — the field **roles** of a slot a held definition declares, twice, because
                // `vocab` asks in both directions. Roles and not content: the content is what is gone.
                "Meaning → Term".to_owned(),
                "Term → Meaning".to_owned(),
                // Case 2 — the high bit set, so a cloze blank in no definition at all.
                "blank 2".to_owned(),
                // Case 3 — neither, so the bare slot. Shown, never hidden (ADR-0018 §3).
                "card 7".to_owned(),
            ],
            "the fixture exists to reach these three cases and nothing else reaches case 3"
        );
    }

    /// **The pruned cloze note keeps a card beside its line.** ADR-0018 §1 interleaves live and
    /// dormant in raw slot order rather than grouping by dormancy, and §2 accepts dormant lines on
    /// the ground that *"two dormant lines above a live card do not cost the pane its job"* — which
    /// is an argument about a pane that still has a card in it. A fixture where every dormant entry
    /// sat on a note with nothing live would photograph the concession and never the case it was
    /// made for.
    #[test]
    fn the_pruned_note_still_generates_the_blank_it_kept() {
        let (_d, _s, mut coll) = empty();
        Fixture::Dormant.install(&mut coll, NOW_MS).unwrap();

        let rows = crate::notes::list(&coll, &crate::notes::Filter::default()).unwrap();
        let pruned = rows
            .iter()
            .find(|r| r.fields.iter().any(|(_, v)| v == DORMANT_CLOZE_AFTER))
            .expect("the edited cloze note is in the list under its new text");
        let pane = crate::cards::card_pane(&coll, pruned.id).unwrap();

        assert_eq!(
            pane.state,
            crate::cards::State::Cards,
            "a note that lost one of two blanks still has one"
        );
        let live = pane
            .entries
            .iter()
            .filter(|e| matches!(e, crate::cards::Entry::Live(_)))
            .count();
        let dormant = pane.entries.len() - live;
        assert_eq!(
            (live, dormant),
            (1, 1),
            "one card and one line, on one note"
        );
    }

    /// [`STRANGER_SLOT`] stands for a slot **no shipped kind declares**, and that is what makes it
    /// case 3 rather than case 1. A kind gaining a card at 7 would move it without touching this file.
    #[test]
    fn the_stranger_slot_is_declared_by_no_shipped_kind() {
        use cairn_core::content::{CLOZE_SLOT_BIT, SHIPPED_KINDS};
        assert!(
            !SHIPPED_KINDS
                .iter()
                .any(|k| k.cards.iter().any(|c| c.slot == STRANGER_SLOT)),
            "slot {STRANGER_SLOT} is declared now, so it names a question and is no longer case 3"
        );
        // And below the partition, or it would be read as a cloze blank and named "blank 7".
        assert_eq!(STRANGER_SLOT & CLOZE_SLOT_BIT, 0);
    }

    /// The caught-up floor is bare: nothing due, and **no leech entrance under it**. The distinction
    /// from [`Fixture::Leeches`] is the whole reason there are two, so it is pinned rather than left
    /// to the reader of the word list.
    #[test]
    fn the_caught_up_fixture_draws_the_floor_with_nothing_under_it() {
        let (_d, _s, mut coll) = empty();
        let reached = Fixture::CaughtUp.install(&mut coll, NOW_MS).unwrap();
        assert_eq!(reached.state, ReviewState::CaughtUp);
        assert_eq!(reached.leeches, 0, "the bare floor has no control under it");
        assert_eq!(reached.due, 0);
        assert_eq!(reached.new, 0);
    }

    /// Caught up **and** leeched — the combination ADR-0035 §1's second call site needs, and the one
    /// that does not fall out of either half on its own: a card whose last grade is a failure is due
    /// whatever its interval says, so a leech has to have passed since crossing to leave the queue.
    #[test]
    fn the_leech_fixture_is_caught_up_with_a_control_under_it() {
        let (_d, _s, mut coll) = empty();
        let reached = Fixture::Leeches.install(&mut coll, NOW_MS).unwrap();
        assert_eq!(reached.state, ReviewState::CaughtUp);
        assert_eq!(reached.leeches, 3);
    }

    /// One ranked leech, read the way `screens::review` reads it — the prompt, the two rank keys,
    /// and the review count the caption's second number comes from.
    #[derive(Debug, PartialEq, Eq)]
    struct RankedLeech {
        prompt: String,
        failure_days: u32,
        last_failure_day: i64,
        reviews: u32,
    }

    fn ranked_leeches(coll: &Collection, now_ms: i64) -> Vec<RankedLeech> {
        let today = cairn_core::log::day_number(now_ms, DayScale::default());
        let current = deck::current_cards(coll).unwrap();
        let lines = coll.log_lines().unwrap();
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        let replayed = replay(&current, &refs);
        leeches(&replayed, today)
            .into_iter()
            .map(|leech| RankedLeech {
                prompt: deck::render(coll, leech.card)
                    .unwrap()
                    .expect("a leech's card is still generated")
                    .prompt,
                failure_days: leech.failure_days,
                last_failure_day: leech.last_failure_day,
                reviews: replayed
                    .cards
                    .get(&leech.card)
                    .map_or(0, |state| state.review_count),
            })
            .collect()
    }

    /// **The leech order is a fact about the collection, and the same fact on every build.**
    ///
    /// Three things are pinned, because the defect [#160](https://github.com/amin-bf/cairn/issues/160)
    /// found had three faces. The **order** is stated, so a fixture edit that reshuffles the screen
    /// says so here rather than in a capture nobody diffs. The two rank keys are **distinct per
    /// card**, which is the property that keeps the order reproducible — `replay::leeches` falls
    /// through to card identity when they tie, and identity is `uuid_v4` per build, so a tied
    /// fixture ranks differently every run. And no two **captions** are the same string, so the
    /// three rows can be told apart in a picture.
    ///
    /// The second install is the half that can actually see the old defect: a single-install
    /// assertion against three tied leeches passes one run in six.
    #[test]
    fn the_leeches_rank_in_a_stated_order_and_the_same_one_every_build() {
        let (_d, _s, mut coll) = empty();
        Fixture::Leeches.install(&mut coll, NOW_MS).unwrap();
        let ranked = ranked_leeches(&coll, NOW_MS);

        let order: Vec<&str> = ranked.iter().map(|l| l.prompt.as_str()).collect();
        assert_eq!(
            order,
            ["néanmoins", "désormais", "d'ailleurs"],
            "worst first (ADR-0010 §4): six failure days, then four and four split by recency"
        );

        let keys: HashSet<(u32, i64)> = ranked
            .iter()
            .map(|l| (l.failure_days, l.last_failure_day))
            .collect();
        assert_eq!(
            keys.len(),
            3,
            "two leeches tying on both rank keys hand the order to a per-build random id"
        );

        let captions: Vec<(u32, u32)> =
            ranked.iter().map(|l| (l.failure_days, l.reviews)).collect();
        assert_eq!(
            captions,
            [(6, 12), (4, 9), (4, 10)],
            "the screen draws '{{days}} bad days · {{reviews}} reviews' and nothing else per row: \
             three distinct strings, and the review count deliberately *ascending* across the \
             tied pair so it cannot be mistaken for what ordered them"
        );

        let (_d2, _s2, mut again) = empty();
        Fixture::Leeches.install(&mut again, NOW_MS).unwrap();
        assert_eq!(
            ranked_leeches(&again, NOW_MS),
            ranked,
            "the same definition installed twice must rank the same way"
        );
    }

    /// **Every leech sits well clear of due, and the margin is the assertion.**
    ///
    /// `Fixture::check` already refuses a `leeches` collection with anything due, but it only ever
    /// sees the one build it ran on. The interval fuzz is seeded from the `CardRef` (ADR-0027 §5),
    /// which is fresh per build, so a leech scheduled one day clear passes that check on most runs
    /// and fails on the rest — an intermittent failure whose cause is three files away. This asserts
    /// the *distance*, which is a property of the record rather than of the build that read it.
    #[test]
    fn every_leech_is_scheduled_days_clear_of_due_not_hours() {
        let (_d, _s, mut coll) = empty();
        Fixture::Leeches.install(&mut coll, NOW_MS).unwrap();
        let today = cairn_core::log::day_number(NOW_MS, DayScale::default());
        let current = deck::current_cards(&coll).unwrap();
        let lines = coll.log_lines().unwrap();
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        let replayed = replay(&current, &refs);

        for leech in leeches(&replayed, today) {
            let state = &replayed.cards[&leech.card];
            let margin = state.due_day - today;
            let prompt = deck::render(&coll, leech.card).unwrap().unwrap().prompt;
            assert!(
                margin >= 5,
                "{prompt} is {margin} days from due — the fuzz swings about three, so this \
                 fixture would install differently on different builds"
            );
        }
    }

    /// [`Fixture::DueWithLeeches`] composes [`Fixture::Leeches`]' leeches unchanged, so it inherits
    /// the order rather than re-tuning one. Pinned because the composition is the *reason* that
    /// fixture exists, and a copy that drifted would put a second, differently-ranked leech screen
    /// behind a picker that looks identical.
    #[test]
    fn the_composed_fixture_ranks_its_leeches_the_same_way() {
        let (_d, _s, mut coll) = empty();
        Fixture::Leeches.install(&mut coll, NOW_MS).unwrap();
        let (_d2, _s2, mut composed) = empty();
        Fixture::DueWithLeeches
            .install(&mut composed, NOW_MS)
            .unwrap();
        assert_eq!(
            ranked_leeches(&composed, NOW_MS),
            ranked_leeches(&coll, NOW_MS)
        );
    }

    /// The crossing fixture leaves its card **one failure day short**, and one *Forgot* today takes
    /// it over. Both halves are asserted: a fixture already over the floor would show the leech
    /// entrance instead of the pointer, and one two days short would end the sitting with nothing.
    #[test]
    fn the_crossing_fixture_crosses_on_the_next_forgot_and_not_before() {
        let (_d, _s, mut coll) = empty();
        let reached = Fixture::Crossing.install(&mut coll, NOW_MS).unwrap();
        assert_eq!(reached.leeches, 0, "it must not have crossed yet");
        assert_eq!(reached.due, 1, "the storyboard grades exactly one card");

        let today = cairn_core::log::day_number(NOW_MS, DayScale::default());
        let lines = coll.log_lines().unwrap();
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        let cards = deck::current_cards(&coll).unwrap();
        let replayed = replay(&cards, &refs);
        let positions = deck::note_positions(&coll).unwrap();
        let queue = session::compose(
            &cards,
            &positions,
            &replayed,
            today,
            DEFAULT_NEW_CARD_RATE as usize,
            &HashSet::new(),
        );
        let due = queue.due[0].card;

        coll.append_review(due, Grade::Forgot, NOW_MS, DayScale::default(), 4_200)
            .unwrap();
        let after = read(&coll, NOW_MS).unwrap();
        assert_eq!(
            after.leeches, 1,
            "one Forgot on a fourth distinct day crosses the floor (ADR-0010 §2)"
        );
    }

    /// A backlog is longer than **every** shorter sitting, not merely longer than a comfortable one.
    /// The entrance draws `5 10 20` only for options strictly below the queue, and that second line
    /// is the link accent's only call site — a fixture of twenty-one would light two thirds of it.
    #[test]
    fn the_backlog_fixture_is_longer_than_every_shorter_sitting() {
        let (_d, _s, mut coll) = empty();
        let reached = Fixture::Backlog.install(&mut coll, NOW_MS).unwrap();
        let ReviewState::Due { due, backlog, .. } = reached.state else {
            panic!("a backlog is due: {reached}");
        };
        assert!(backlog, "past a comfortable sitting, so Review frames it");
        assert!(due > 20, "past the longest shorter sitting: {due}");
    }

    /// **The deck fixture's ids are the ids the collection holds** — the property the whole fixture
    /// exists for and the one thing about it `Fixture::check` cannot see, since a deck at *any* id
    /// counts as a deck. An id that came out different is invisible from the note list (a deck is a
    /// deck) and fatal from the file bench's side, because ADR-0008 §11's authority follows deck id
    /// and an import built to match would take the *new deck* path against a collection that already
    /// holds the deck.
    #[test]
    fn the_deck_fixture_lands_on_the_ids_it_publishes() {
        let (_d, _s, mut coll) = empty();
        Fixture::Decks.install(&mut coll, NOW_MS).unwrap();

        let held: Vec<(String, String)> = coll
            .decks()
            .unwrap()
            .into_iter()
            .map(|(id, name)| (id.to_canonical(), name))
            .collect();
        let published: Vec<(String, String)> = DECKS
            .iter()
            .map(|(id, name)| ((*id).to_owned(), (*name).to_owned()))
            .collect();
        for deck in &published {
            assert!(
                held.contains(deck),
                "DECKS names {deck:?}, which the collection does not hold: {held:?}"
            );
        }
        assert_eq!(held.len(), published.len(), "and holds nothing else");
    }

    /// The deck surface this fixture exists to make drawable, asserted as the **shape** the ticket
    /// asked for rather than as four numbers: more than two decks so the dropdown is a list, two
    /// names long enough to ask a question of whatever carries them, and notes left unfiled beside
    /// the filed ones.
    ///
    /// It is the sizes that are pinned, not the strings — a later reader may swap a word without
    /// consulting anyone, and must not be able to quietly swap the coverage. #133's word list is the
    /// precedent: trimming a list to three tidy entries removes the coverage without removing
    /// anything that *looks* like coverage.
    #[test]
    fn the_deck_fixture_draws_a_dropdown_that_is_a_list_and_names_that_are_long() {
        let (_d, _s, mut coll) = empty();
        let reached = Fixture::Decks.install(&mut coll, NOW_MS).unwrap();

        assert!(
            reached.decks > 2,
            "a dropdown of two decks is a pair, not a list: {reached}"
        );
        let long = DECKS
            .iter()
            .filter(|(_, name)| name.chars().count() > 30)
            .count();
        assert!(
            long >= 2,
            "two names long enough to test a row that carries one, got {long}"
        );
        assert!(
            reached.unfiled > 0 && reached.unfiled < reached.notes,
            "some notes unfiled and some filed, so the two states differ on screen: {reached}"
        );
        assert_eq!(reached.notes, 25, "a list state is sized like `backlog`'s");
    }

    /// **A right-to-left row, which is the second half of why this fixture exists.** The note list
    /// draws a row's *first field* (`NoteRow::preview`), so a Persian note only reaches the surface
    /// this is about if the Persian is in `Front` — a note whose `Back` is Persian would satisfy any
    /// count of Persian notes and draw twenty-five Latin rows.
    ///
    /// Reaching one used to mean eleven storyboard steps driving `xdotool` into a field, which is how
    /// #150's row defect went unseen: the French seed has no right-to-left word in it, so there was
    /// no capture anybody had a reason to take.
    #[test]
    fn the_deck_fixture_puts_persian_in_the_field_a_row_draws() {
        let (_d, _s, mut coll) = empty();
        Fixture::Decks.install(&mut coll, NOW_MS).unwrap();

        // The Arabic script block, which is what "right to left" means for Persian here.
        let rtl = |s: &str| s.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c));
        let rows = crate::notes::list(&coll, &crate::notes::Filter::default()).unwrap();
        let persian: Vec<&crate::notes::NoteRow> =
            rows.iter().filter(|r| rtl(r.preview())).collect();
        assert!(
            persian.len() > 1,
            "one right-to-left row is a specimen; several are a list to judge: {}",
            persian.len()
        );
        assert!(
            persian.iter().any(|r| r.preview().chars().count() > 20),
            "and one of them long enough to wrap at the judging widths"
        );
        assert!(
            DECKS.iter().any(|(_, name)| rtl(name)),
            "the dropdown carries a right-to-left entry too, not only the rows"
        );
    }

    /// Every deck the fixture files a note under is a deck it **holds**. A `deck` reference naming no
    /// held deck is legal and silent — the note is unfiled (ADR-0005 §8), the list draws it, and
    /// nothing fails — so a typo in one id would move a note out of its deck with no error anywhere.
    #[test]
    fn every_deck_the_note_table_files_under_exists() {
        for (filed_under, front, _) in DECK_NOTES {
            if let Some(index) = filed_under {
                assert!(
                    index < DECKS.len(),
                    "'{front}' is filed under deck {index}, and there are {} decks",
                    DECKS.len()
                );
            }
        }
        let ids: HashSet<&str> = DECKS.iter().map(|(id, _)| *id).collect();
        assert_eq!(
            ids.len(),
            DECKS.len(),
            "two decks may share a name, never an id"
        );
        for (id, _) in DECKS {
            assert!(
                cairn_core::content::DeckId::parse_canonical(id).is_some(),
                "'{id}' is not canonical UUID text, so no note can be filed under it"
            );
        }
    }

    /// The bench refuses to write into a collection that already holds rows. A fixture layered on
    /// top of the shipping seed reaches a state nobody named, and it would pass every check above by
    /// accident on at least one fixture.
    #[test]
    fn a_fixture_refuses_a_collection_that_is_not_empty() {
        let (_d, _s, mut coll) = empty();
        Fixture::CaughtUp.install(&mut coll, NOW_MS).unwrap();
        let second = Fixture::Backlog.install(&mut coll, NOW_MS);
        assert!(second.is_err(), "expected a refusal, got {second:?}");
    }

    /// An unknown key is refused rather than guessed — the property that makes a storyboard naming
    /// its own fixture safe, because a typo aborts the run instead of photographing the seed.
    #[test]
    fn an_unknown_fixture_key_is_refused() {
        assert_eq!(Fixture::parse("caught-up"), Some(Fixture::CaughtUp));
        assert_eq!(Fixture::parse("caught_up"), None);
        assert_eq!(Fixture::parse(""), None);
    }

    /// The checkpoint lever is **subtractive**: with no override the caller's own number comes back,
    /// which is what keeps ADR-0006 §1's ten minutes written at the call site rather than here.
    ///
    /// **The override is process-global and the suite runs threaded**, so this test briefly changes
    /// what every other test sees. That is safe only because the one test that reaches
    /// `checkpoint_due` — `the_ten_minute_checkpoint_never_hides_the_card` — winds its sitting back
    /// 700 seconds and asserts the checkpoint *is* due, which every value this sets keeps true. A
    /// test asserting the checkpoint is **not** due would flake against it, and would need the lever
    /// threaded through rather than stored.
    #[test]
    fn the_checkpoint_lever_returns_the_specified_duration_until_it_is_set() {
        let specified = Duration::from_secs(600);
        assert_eq!(checkpoint_after(specified), specified);
        set_checkpoint_after(3);
        assert_eq!(checkpoint_after(specified), Duration::from_secs(3));
        assert!(checkpoint_is_shortened());
        set_checkpoint_after(0);
        assert_eq!(
            checkpoint_after(specified),
            Duration::ZERO,
            "zero is a legal override, not the absence of one"
        );
        OVERRIDE_SECONDS.store(NO_OVERRIDE, Ordering::Relaxed);
        assert_eq!(checkpoint_after(specified), specified);
    }
}
