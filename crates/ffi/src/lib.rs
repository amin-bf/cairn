//! The Kotlin binding over `cairn-ops` (ADR-0046 §2) — thin on purpose.
//!
//! What crosses is an operation and the facts it returns. This crate exports **no platform function
//! and calls none** (ADR-0046 §5): Kotlin passes `filesDir` and `noBackupFilesDir` to [`open`](CollectionHandle::open),
//! and never reaches `cairn_store::platform`, whose Android arm panics in a library no
//! `android-activity` loaded. Time is a value here as everywhere: where an operation needs *now*, the
//! caller passes it.
//!
//! **A panic reaches Kotlin as an exception only while the release profile unwinds** (ADR-0046 §7), so
//! `panic = "abort"` must never be added to the workspace's `[profile.release]`.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use cairn_core::content::CardRef;
use cairn_core::scheduling::Grade as CoreGrade;
use cairn_ops::review::{self, Review};
use cairn_ops::session::ReviewState as CoreState;
use cairn_store::Collection;

uniffi::setup_scaffolding!();

/// The one error that crosses, as Kotlin's `CairnException`. The field is `reason` and never
/// `message`: a field called `message` generates Kotlin that clashes with `Throwable.message`.
#[derive(Debug, uniffi::Error)]
pub enum CairnError {
    /// An operation failed; `reason` is the operation's own account of why, for a log, not for the
    /// screen (facts cross, words do not — ADR-0046 §4).
    Failed { reason: String },
}

impl std::fmt::Display for CairnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let CairnError::Failed { reason } = self;
        write!(f, "{reason}")
    }
}

impl std::error::Error for CairnError {}

impl From<cairn_store::StoreError> for CairnError {
    fn from(e: cairn_store::StoreError) -> Self {
        CairnError::Failed {
            reason: e.to_string(),
        }
    }
}

impl From<String> for CairnError {
    fn from(reason: String) -> Self {
        CairnError::Failed { reason }
    }
}

/// The opened collection. Kotlin holds exactly one per process (ADR-0046 §7). Objects are shared
/// through `Arc` so every method takes `&self`, and `Collection` is `Send` but not `Sync`, hence the
/// `Mutex`: calls run one at a time.
#[derive(uniffi::Object)]
pub struct CollectionHandle {
    /// `None` only while [`install_fixture`](CollectionHandle::install_fixture) has the files closed
    /// to replace them; every other call finds a collection.
    collection: Mutex<Option<Collection>>,
    data_dir: PathBuf,
    state_dir: PathBuf,
}

/// The locked collection, dereferencing to it.
struct Locked<'a>(MutexGuard<'a, Option<Collection>>);

impl std::ops::Deref for Locked<'_> {
    type Target = Collection;
    fn deref(&self) -> &Collection {
        self.0
            .as_ref()
            .expect("the collection is open between calls")
    }
}

impl std::ops::DerefMut for Locked<'_> {
    fn deref_mut(&mut self) -> &mut Collection {
        self.0
            .as_mut()
            .expect("the collection is open between calls")
    }
}

impl CollectionHandle {
    /// A panic inside one call poisons the lock; the collection itself is not left half-written by
    /// that (every write is a transaction), so later calls carry on rather than all panicking too.
    fn collection(&self) -> Locked<'_> {
        Locked(
            self.collection
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
        )
    }
}

#[uniffi::export]
impl CollectionHandle {
    /// Open the collection under the two directories the caller passes and, on a first launch, seed
    /// the starter notes — `cairn_ops::open_collection`, the one place the seed is written, so both
    /// renderers seed alike.
    #[uniffi::constructor]
    pub fn open(data_dir: String, state_dir: String) -> Result<Self, CairnError> {
        let coll = cairn_ops::open_collection(Path::new(&data_dir), Path::new(&state_dir))?;
        Ok(Self {
            collection: Mutex::new(Some(coll)),
            data_dir: data_dir.into(),
            state_dir: state_dir.into(),
        })
    }

    /// Replace the collection with the named fixture (`caught-up`, `leeches`, `crossing`, `backlog`,
    /// `cloze`, `persian`, …) — the bench, reached by name. The files are wiped and reopened (a fixture
    /// installs only into an empty collection, and `open` would seed one), so nothing of the old
    /// collection survives. An unknown name is refused and touches nothing. Any sitting begun before
    /// is stale after this and should be dropped.
    pub fn install_fixture(&self, name: String, now_ms: i64) -> Result<(), CairnError> {
        let fixture = cairn_ops::fixtures::Fixture::parse(&name)
            .ok_or_else(|| CairnError::from(format!("no fixture is named {name:?}")))?;
        let mut slot = self
            .collection
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        // Close the files before unlinking them: a live connection would keep checkpointing into
        // inodes nothing can reach.
        drop(slot.take());
        cairn_store::remove_files(&self.data_dir, &self.state_dir);
        let mut coll = Collection::open(&self.data_dir, &self.state_dir)?;
        let installed = fixture.install(&mut coll, now_ms);
        *slot = Some(coll);
        installed?;
        Ok(())
    }

    /// The entrance as facts, against the day of `now_wall_ms`.
    pub fn review_state(&self, now_wall_ms: i64) -> Result<ReviewFacts, CairnError> {
        Ok(cairn_ops::review::facts(&self.collection(), now_wall_ms)?.into())
    }

    /// Begin a sitting of `count` gradings. `now_mono_ms` is the monotonic count every duration in
    /// the sitting is a difference of; `now_wall_ms` is the review rows' *when* and the day's bound.
    pub fn start_sitting(
        self: Arc<Self>,
        count: u32,
        now_mono_ms: u64,
        now_wall_ms: i64,
    ) -> Result<Arc<SittingHandle>, CairnError> {
        let review = Review::start(&self.collection(), count as usize, now_mono_ms, now_wall_ms)?;
        Ok(Arc::new(SittingHandle {
            owner: self,
            review: Mutex::new(review),
        }))
    }

    /// How many cards are due at `now_ms` (wall-clock milliseconds, handed over by the caller).
    pub fn due_count(&self, now_ms: i64) -> Result<u64, CairnError> {
        Ok(cairn_ops::session::due_count(&self.collection(), now_ms)? as u64)
    }

    /// How many notes the collection holds — what a first launch's seed shows up as.
    pub fn note_count(&self) -> Result<u64, CairnError> {
        let rows =
            cairn_ops::notes::list(&self.collection(), &cairn_ops::notes::Filter::default())?;
        Ok(rows.len() as u64)
    }
}

/// Which of the five states the entrance is in, with the counts that belong to it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum ReviewStateKind {
    /// No cards exist at all.
    Empty,
    /// Cards exist and nothing has ever been reviewed.
    NewDeck { new: u32 },
    /// Nothing due; what is left is cards never yet seen.
    NewOnly { new: u32 },
    /// Everything reviewed is scheduled ahead.
    CaughtUp,
    /// Cards are due; `new` first-time cards are also available.
    Due { due: u32, new: u32, backlog: bool },
}

/// The entrance, as facts — no sentence (ADR-0046 §4).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReviewFacts {
    pub state: ReviewStateKind,
    pub due: u32,
    pub new: u32,
    /// `due + new`: everything a sitting could draw from.
    pub available: u32,
    pub backlog: bool,
    /// Which of 5/10/20 are strictly shorter than the queue.
    pub shorter: Vec<u32>,
    /// Cards over the leech floor, suspended ones included.
    pub leeches: u32,
    pub suspended: u32,
}

impl From<review::Facts> for ReviewFacts {
    fn from(f: review::Facts) -> Self {
        let state = match f.state {
            CoreState::Empty => ReviewStateKind::Empty,
            CoreState::NewDeck { new } => ReviewStateKind::NewDeck { new: new as u32 },
            CoreState::NewOnly { new } => ReviewStateKind::NewOnly { new: new as u32 },
            CoreState::CaughtUp => ReviewStateKind::CaughtUp,
            CoreState::Due { due, new, backlog } => ReviewStateKind::Due {
                due: due as u32,
                new: new as u32,
                backlog,
            },
        };
        ReviewFacts {
            state,
            due: f.due as u32,
            new: f.new as u32,
            available: f.available as u32,
            backlog: f.backlog,
            shorter: f.shorter.into_iter().map(|n| n as u32).collect(),
            leeches: f.leeches as u32,
            suspended: f.suspended as u32,
        }
    }
}

/// How a run of text is drawn. `BoldItalic` is bold text inside italic (or the reverse): the egui card
/// draws it, so the binding carries it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum RunStyle {
    Plain,
    Bold,
    Italic,
    BoldItalic,
    Code,
}

impl From<review::RunStyle> for RunStyle {
    fn from(s: review::RunStyle) -> Self {
        match s {
            review::RunStyle::Plain => RunStyle::Plain,
            review::RunStyle::Bold => RunStyle::Bold,
            review::RunStyle::Italic => RunStyle::Italic,
            review::RunStyle::BoldItalic => RunStyle::BoldItalic,
            review::RunStyle::Code => RunStyle::Code,
        }
    }
}

/// A stretch of one face in one style, markers already dropped and cloze masking applied.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Run {
    pub text: String,
    pub style: RunStyle,
}

/// The four grades, in the order the buttons run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum Grade {
    Forgot,
    Barely,
    Good,
    Easy,
}

impl From<Grade> for CoreGrade {
    fn from(g: Grade) -> Self {
        match g {
            Grade::Forgot => CoreGrade::Forgot,
            Grade::Barely => CoreGrade::Barely,
            Grade::Good => CoreGrade::Good,
            Grade::Easy => CoreGrade::Easy,
        }
    }
}

/// The illustrative next interval in days for each grade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct IntervalPreview {
    pub forgot: u32,
    pub barely: u32,
    pub good: u32,
    pub easy: u32,
}

/// The card on screen, as facts. Ids cross as the canonical string and the ordinal.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ShownCard {
    pub note_id: String,
    pub ordinal: u32,
    pub prompt: Vec<Run>,
    pub answer: Vec<Run>,
    /// The prompt begins right-to-left (its first strong character); the answer follows the prompt.
    pub rtl: bool,
    /// Never reviewed: the badge reads *new*, not *box n*.
    pub is_new: bool,
    /// The durability box, 1–5; meaningful when `is_new` is false.
    pub box_number: u8,
    pub previews: IntervalPreview,
    pub revealed: bool,
}

fn runs(from: Vec<review::Run>) -> Vec<Run> {
    from.into_iter()
        .map(|r| Run {
            text: r.text,
            style: r.style.into(),
        })
        .collect()
}

impl From<review::Shown> for ShownCard {
    fn from(s: review::Shown) -> Self {
        let CardRef { note, ordinal } = s.card;
        let [forgot, barely, good, easy] = s.previews;
        ShownCard {
            note_id: note.to_canonical(),
            ordinal: u32::from(ordinal),
            prompt: runs(s.prompt),
            answer: runs(s.answer),
            rtl: s.rtl,
            is_new: s.is_new,
            box_number: s.box_,
            previews: IntervalPreview {
                forgot,
                barely,
                good,
                easy,
            },
            revealed: s.revealed,
        }
    }
}

/// What a permitted grading did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct GradeOutcome {
    /// Whether the review row was appended. A failed append still counts the grading and moves the
    /// card off the screen (`Graded::NotRecorded`).
    pub recorded: bool,
}

/// One sitting: the cairn-ops sitting over the collection it was begun on. Every time is passed in.
#[derive(uniffi::Object)]
pub struct SittingHandle {
    owner: Arc<CollectionHandle>,
    review: Mutex<Review>,
}

impl SittingHandle {
    fn review(&self) -> MutexGuard<'_, Review> {
        self.review.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[uniffi::export]
impl SittingHandle {
    /// The count the sitting was started with.
    pub fn chosen(&self) -> u32 {
        self.review().chosen() as u32
    }

    /// Gradings made, re-shows included.
    pub fn graded(&self) -> u32 {
        self.review().graded() as u32
    }

    /// The card shown, or `None` once the sitting has ended.
    pub fn card(
        &self,
        now_wall_ms: i64,
        now_mono_ms: u64,
    ) -> Result<Option<ShownCard>, CairnError> {
        let shown = self
            .review()
            .card(&self.owner.collection(), now_wall_ms, now_mono_ms)?;
        Ok(shown.map(ShownCard::from))
    }

    /// Show the answer of the card on screen.
    pub fn reveal(&self) -> Result<(), CairnError> {
        self.review()
            .reveal()
            .map_err(|e| CairnError::from(format!("{e:?}")))
    }

    /// Grade the card on screen; refused (as `CairnException`) before the reveal.
    pub fn grade(
        &self,
        grade: Grade,
        now_mono_ms: u64,
        now_wall_ms: i64,
    ) -> Result<GradeOutcome, CairnError> {
        let outcome = self
            .review()
            .grade(
                &mut self.owner.collection(),
                grade.into(),
                now_mono_ms,
                now_wall_ms,
            )
            .map_err(|e| CairnError::from(e.to_string()))?;
        Ok(GradeOutcome {
            recorded: outcome.recorded,
        })
    }

    /// Ten minutes have passed and the check-in has not been waved away.
    pub fn checkpoint_due(&self, now_mono_ms: u64) -> bool {
        self.review().checkpoint_due(now_mono_ms)
    }

    /// *Keep going*: the checkpoint does not come back this sitting.
    pub fn keep_going(&self) {
        self.review().keep_going();
    }

    /// *Finish here*: the sitting ends now.
    pub fn finish_here(&self) {
        self.review().finish_here();
    }

    /// Whether the sitting has ended.
    pub fn ended(&self, now_wall_ms: i64) -> Result<bool, CairnError> {
        Ok(self.review().ended(&self.owner.collection(), now_wall_ms)?)
    }

    /// How many leeches crossed the floor during this sitting.
    pub fn leeches_crossed(&self, now_wall_ms: i64) -> Result<u32, CairnError> {
        Ok(self
            .review()
            .leeches_crossed(&self.owner.collection(), now_wall_ms)? as u32)
    }
}

/// Its own block, because the export macro expands every method it sees whatever `cfg` the method
/// carries — the cfg has to sit on the whole block.
#[cfg(feature = "panic-probe")]
#[uniffi::export]
impl CollectionHandle {
    /// Panic on purpose, holding the lock, so a test can see the panic arrive as an exception and the
    /// handle survive it.
    pub fn panic_probe(&self) {
        let _held = self.collection();
        panic!("panic probe");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn dirs() -> (TempDir, TempDir) {
        (TempDir::new().unwrap(), TempDir::new().unwrap())
    }

    fn open(data: &TempDir, state: &TempDir) -> CollectionHandle {
        CollectionHandle::open(
            data.path().display().to_string(),
            state.path().display().to_string(),
        )
        .unwrap()
    }

    const NOW: i64 = 1_760_000_000_000;

    /// A handle over a store in a temp dir with `fixture` installed through it, as Kotlin would.
    fn benched(fixture: &str) -> (Arc<CollectionHandle>, TempDir, TempDir) {
        let (data, state) = dirs();
        let handle = Arc::new(open(&data, &state));
        handle.install_fixture(fixture.to_owned(), NOW).unwrap();
        (handle, data, state)
    }

    fn text(runs: &[Run]) -> String {
        runs.iter().map(|r| r.text.as_str()).collect()
    }

    #[test]
    fn each_review_state_is_reached_through_a_fixture_by_name() {
        let (handle, _d, _s) = benched("caught-up");
        let facts = handle.review_state(NOW).unwrap();
        assert_eq!(facts.state, ReviewStateKind::CaughtUp);
        assert_eq!((facts.due, facts.new, facts.leeches), (0, 0, 0));

        let (handle, _d, _s) = benched("leeches");
        let facts = handle.review_state(NOW).unwrap();
        assert_eq!(facts.state, ReviewStateKind::CaughtUp);
        assert_eq!(facts.leeches, 3);

        let (handle, _d, _s) = benched("backlog");
        let facts = handle.review_state(NOW).unwrap();
        assert!(matches!(
            facts.state,
            ReviewStateKind::Due {
                due: 25,
                backlog: true,
                ..
            }
        ));
        assert!(facts.backlog);
        assert_eq!(facts.shorter, vec![5, 10, 20]);

        let (handle, _d, _s) = benched("persian");
        let facts = handle.review_state(NOW).unwrap();
        assert_eq!(facts.state, ReviewStateKind::NewDeck { new: 5 });
        assert!(facts.shorter.is_empty());
    }

    #[test]
    fn the_empty_and_new_only_states_are_reached_too() {
        let (data, state) = dirs();
        let handle = open(&data, &state);
        // A first launch seeds: a fresh deck.
        assert!(matches!(
            handle.review_state(NOW).unwrap().state,
            ReviewStateKind::NewDeck { .. }
        ));

        // Review one card; the rest of the rate's new cards are left: new only, not a fresh deck.
        let handle = Arc::new(handle);
        let sitting = handle.clone().start_sitting(1, 0, NOW).unwrap();
        sitting.card(NOW, 10).unwrap().unwrap();
        sitting.reveal().unwrap();
        sitting.grade(Grade::Good, 20, NOW).unwrap();
        assert!(matches!(
            handle.review_state(NOW).unwrap().state,
            ReviewStateKind::NewOnly { .. }
        ));
    }

    #[test]
    fn an_unknown_fixture_is_refused_and_touches_nothing() {
        let (data, state) = dirs();
        let handle = open(&data, &state);
        let before = handle.note_count().unwrap();
        let CairnError::Failed { reason } = handle
            .install_fixture("no-such-fixture".to_owned(), NOW)
            .unwrap_err();
        assert!(reason.contains("no-such-fixture"));
        assert_eq!(handle.note_count().unwrap(), before);
    }

    #[test]
    fn a_sitting_runs_start_to_end_through_the_handle() {
        let (handle, _d, _s) = benched("backlog");
        let sitting = handle.clone().start_sitting(3, 1_000, NOW).unwrap();
        assert_eq!((sitting.chosen(), sitting.graded()), (3, 0));
        for made in 0..3 {
            assert!(!sitting.ended(NOW).unwrap());
            let shown = sitting.card(NOW, 2_000).unwrap().unwrap();
            assert!(!shown.revealed);
            assert!(!shown.is_new && (1..=5).contains(&shown.box_number));
            sitting.reveal().unwrap();
            assert!(sitting.card(NOW, 2_100).unwrap().unwrap().revealed);
            let outcome = sitting.grade(Grade::Good, 3_000, NOW).unwrap();
            assert!(outcome.recorded);
            assert_eq!(sitting.graded(), made + 1);
        }
        assert!(sitting.ended(NOW).unwrap());
        assert!(sitting.card(NOW, 4_000).unwrap().is_none());
        // The three gradings are in the log: three fewer are due.
        assert!(matches!(
            handle.review_state(NOW).unwrap().state,
            ReviewStateKind::Due { due: 22, .. }
        ));
    }

    #[test]
    fn a_grade_before_the_reveal_crosses_as_the_one_error_and_counts_nothing() {
        let (handle, _d, _s) = benched("backlog");
        let sitting = handle.start_sitting(3, 0, NOW).unwrap();
        // Nothing on screen yet, and then a card that is not revealed: both refused.
        assert!(sitting.grade(Grade::Good, 5, NOW).is_err());
        sitting.card(NOW, 10).unwrap().unwrap();
        let CairnError::Failed { reason } = sitting.grade(Grade::Good, 20, NOW).unwrap_err();
        assert!(!reason.is_empty());
        assert_eq!(sitting.graded(), 0);
    }

    #[test]
    fn a_lapse_is_shown_again_in_the_same_sitting() {
        let (handle, _d, _s) = benched("crossing");
        let sitting = handle.start_sitting(5, 0, NOW).unwrap();
        let first = sitting.card(NOW, 10).unwrap().unwrap();
        sitting.reveal().unwrap();
        sitting.grade(Grade::Forgot, 20, NOW).unwrap();
        let again = sitting.card(NOW, 30).unwrap().unwrap();
        assert_eq!(
            (&again.note_id, again.ordinal),
            (&first.note_id, first.ordinal)
        );
        assert!(!again.revealed);
        assert_eq!(sitting.graded(), 1);
    }

    #[test]
    fn the_checkpoint_and_the_leeches_cross() {
        let (handle, _d, _s) = benched("crossing");
        let sitting = handle.start_sitting(1, 0, NOW).unwrap();
        assert!(!sitting.checkpoint_due(599_999));
        assert!(sitting.checkpoint_due(600_000));
        sitting.keep_going();
        assert!(!sitting.checkpoint_due(10_000_000));

        assert_eq!(sitting.leeches_crossed(NOW).unwrap(), 0);
        sitting.card(NOW, 10).unwrap().unwrap();
        sitting.reveal().unwrap();
        sitting.grade(Grade::Forgot, 20, NOW).unwrap();
        assert!(sitting.ended(NOW).unwrap());
        assert_eq!(sitting.leeches_crossed(NOW).unwrap(), 1);
    }

    #[test]
    fn finishing_here_ends_the_sitting() {
        let (handle, _d, _s) = benched("backlog");
        let sitting = handle.start_sitting(10, 0, NOW).unwrap();
        sitting.card(NOW, 10).unwrap().unwrap();
        sitting.finish_here();
        assert!(sitting.ended(NOW).unwrap());
        assert!(sitting.card(NOW, 20).unwrap().is_none());
    }

    #[test]
    fn the_previews_equal_the_ops_for_the_same_card_and_day() {
        let (handle, _d, _s) = benched("backlog");
        let sitting = handle.clone().start_sitting(1, 0, NOW).unwrap();
        let shown = sitting.card(NOW, 10).unwrap().unwrap();

        let mut ops = Review::start(&handle.collection(), 1, 0, NOW).unwrap();
        let expected = ops.card(&handle.collection(), NOW, 10).unwrap().unwrap();
        let p = shown.previews;
        assert_eq!([p.forgot, p.barely, p.good, p.easy], expected.previews);
        assert_eq!(shown.note_id, expected.card.note.to_canonical());
        assert_eq!(shown.ordinal, u32::from(expected.card.ordinal));
        assert!(p.forgot >= 1 && p.easy >= p.good);
    }

    #[test]
    fn a_cloze_card_crosses_masked_with_its_answer_unmasked() {
        let (handle, _d, _s) = benched("cloze");
        let sitting = handle.start_sitting(5, 0, NOW).unwrap();
        let shown = sitting.card(NOW, 10).unwrap().unwrap();
        assert!(shown.is_new);
        assert_ne!(text(&shown.prompt), text(&shown.answer));
    }

    #[test]
    fn a_persian_prompt_is_right_to_left_and_a_french_one_is_not() {
        let (handle, _d, _s) = benched("persian");
        let sitting = handle.start_sitting(5, 0, NOW).unwrap();
        let (mut persian, mut french) = (false, false);
        for _ in 0..5 {
            let shown = sitting.card(NOW, 10).unwrap().unwrap();
            let arabic = text(&shown.prompt)
                .chars()
                .any(|c| ('\u{0600}'..='\u{06FF}').contains(&c));
            if arabic {
                assert!(shown.rtl);
                persian = true;
            } else {
                assert!(!shown.rtl);
                french = true;
            }
            sitting.reveal().unwrap();
            sitting.grade(Grade::Good, 20, NOW).unwrap();
        }
        assert!(persian && french, "the fixture holds both scripts");
    }

    #[test]
    fn markup_arrives_as_styled_runs_without_its_markers() {
        // A caught-up collection has nothing waiting, so the one note added is the whole queue.
        let (handle, _d, _s) = benched("caught-up");
        handle
            .collection()
            .create_note("basic", &[("Front", "a **b** *c* `d`"), ("Back", "plain")])
            .unwrap();
        let sitting = handle.start_sitting(1, 0, NOW).unwrap();
        let shown = sitting.card(NOW, 10).unwrap().unwrap();
        assert_eq!(text(&shown.prompt), "a b c d");
        let styles: Vec<RunStyle> = shown.prompt.iter().map(|r| r.style).collect();
        assert_eq!(
            styles,
            [
                RunStyle::Plain,
                RunStyle::Bold,
                RunStyle::Plain,
                RunStyle::Italic,
                RunStyle::Plain,
                RunStyle::Code
            ]
        );
    }

    #[test]
    fn opening_lands_the_files_in_the_two_directories_and_seeds_a_first_launch() {
        let (data, state) = dirs();
        let handle = open(&data, &state);
        assert!(data.path().join("collection.db").exists());
        assert!(state.path().join("derived.db").exists());
        assert!(state.path().join("writer.id").exists(), "the writer marker");
        assert!(handle.note_count().unwrap() > 0, "a first launch seeds");
        // The seed is all new cards, none yet due.
        assert_eq!(handle.due_count(1_700_000_000_000).unwrap(), 0);
    }

    #[test]
    fn a_failed_open_crosses_as_the_one_error_with_a_reason() {
        let data = TempDir::new().unwrap();
        let file = data.path().join("not-a-directory");
        std::fs::write(&file, b"x").unwrap();
        let err = CollectionHandle::open(
            file.join("data").display().to_string(),
            file.join("state").display().to_string(),
        )
        .err()
        .expect("opening under a file must fail");
        let CairnError::Failed { reason } = err;
        assert!(!reason.is_empty());
    }

    /// ADR-0046 §7: a panic reaches Kotlin as an exception only while the profile unwinds.
    #[test]
    fn the_release_profile_never_aborts_on_panic() {
        let manifest = include_str!("../../../Cargo.toml");
        assert!(
            !manifest.lines().any(|l| {
                let l = l.split('#').next().unwrap_or("").replace(' ', "");
                l.starts_with("panic=")
            }),
            "`panic = \"abort\"` in any profile turns a Rust panic into a process abort"
        );
    }

    #[cfg(feature = "panic-probe")]
    #[test]
    fn a_panic_unwinds_and_the_handle_keeps_working() {
        let (data, state) = dirs();
        let handle = open(&data, &state);
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            handle.panic_probe();
        }));
        assert!(caught.is_err());
        assert_eq!(handle.due_count(1_700_000_000_000).unwrap(), 0);
    }
}
