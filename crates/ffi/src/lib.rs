//! The Kotlin binding over `cairn-ops` (ADR-0046 §2) — thin on purpose.
//!
//! What crosses is an operation and the facts it returns. This crate exports **no platform function
//! and calls none** (ADR-0046 §5): Kotlin passes `filesDir` and `noBackupFilesDir` to [`open`](CollectionHandle::open),
//! and never reaches `cairn_store::platform`, whose Android arm refuses (ADR-0045 §8). Time is a
//! value here as everywhere: where an operation needs *now*, the caller passes it.
//!
//! **A panic reaches Kotlin as an exception only while the release profile unwinds** (ADR-0046 §7), so
//! `panic = "abort"` must never be added to the workspace's `[profile.release]`.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use cairn_core::content::{CardRef, DeckId, NoteId};
use cairn_core::scheduling::Grade as CoreGrade;
use cairn_ops::cards;
use cairn_ops::editor::{self, Editing};
use cairn_ops::inbound::{self, Plan, Refusal};
use cairn_ops::leeches;
use cairn_ops::notes::{self, DeckFilter, Filter};
use cairn_ops::optimise;
use cairn_ops::restore;
use cairn_ops::review::{self, Review};
use cairn_ops::session::ReviewState as CoreState;
use cairn_ops::sitting::{GradeRefused, RevealRefused};
use cairn_store::Collection;

uniffi::setup_scaffolding!();

/// The one error that crosses, as Kotlin's `CairnException`. A field is never called `message`: it
/// generates Kotlin that clashes with `Throwable.message`.
///
/// **A refusal is a fact, not a sentence** (ADR-0046 §4): each way an operation says no has its own
/// variant, so Kotlin catches `CairnException.NotRevealed` and says what it needs to in its own words.
/// `Failed` is what is left — a store or I/O fault nobody can act on by name.
#[derive(Debug, uniffi::Error)]
pub enum CairnError {
    /// An operation failed; `reason` is the operation's own account of why, for a log, not for the
    /// screen.
    Failed { reason: String },
    /// A reveal or a grade was asked of a sitting with no card on screen.
    NoCardShown,
    /// A grade was asked of a card whose answer has not been revealed (ADR-0006 §4).
    NotRevealed,
    /// `install_fixture` was given a name no fixture has; nothing was touched.
    UnknownFixture { name: String },
}

impl std::fmt::Display for CairnError {
    /// For a log; nothing here is shown to a person.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CairnError::Failed { reason } => write!(f, "{reason}"),
            CairnError::NoCardShown => f.write_str("no card is on screen"),
            CairnError::NotRevealed => f.write_str("the answer has not been revealed"),
            CairnError::UnknownFixture { name } => write!(f, "no fixture is named {name:?}"),
        }
    }
}

impl From<GradeRefused> for CairnError {
    fn from(refused: GradeRefused) -> Self {
        match refused {
            GradeRefused::NoCardShown => CairnError::NoCardShown,
            GradeRefused::NotRevealed => CairnError::NotRevealed,
        }
    }
}

impl From<RevealRefused> for CairnError {
    fn from(refused: RevealRefused) -> Self {
        match refused {
            RevealRefused::NoCardShown => CairnError::NoCardShown,
        }
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

/// Which theme the user asked for (ADR-0036 §3): the mirror of `cairn_ops::appearance::ThemeChoice`,
/// so Kotlin never parses the stored string itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ThemeChoice {
    System,
    Light,
    Dark,
}

impl From<cairn_ops::appearance::ThemeChoice> for ThemeChoice {
    fn from(choice: cairn_ops::appearance::ThemeChoice) -> Self {
        use cairn_ops::appearance::ThemeChoice as Stored;
        match choice {
            Stored::System => Self::System,
            Stored::Light => Self::Light,
            Stored::Dark => Self::Dark,
        }
    }
}

impl From<ThemeChoice> for cairn_ops::appearance::ThemeChoice {
    fn from(choice: ThemeChoice) -> Self {
        match choice {
            ThemeChoice::System => Self::System,
            ThemeChoice::Light => Self::Light,
            ThemeChoice::Dark => Self::Dark,
        }
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
    ///
    /// **This destroys the collection under the two directories `open` was given.** It is the bench:
    /// only a debug or test build may call it, never a path a user reaches.
    ///
    /// Returns what the install reached, in the bench's own words (`Reached`'s display), recomputed
    /// from the rows it wrote.
    pub fn install_fixture(&self, name: String, now_ms: i64) -> Result<String, CairnError> {
        let fixture = cairn_ops::fixtures::Fixture::parse(&name)
            .ok_or(CairnError::UnknownFixture { name })?;
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
        Ok(installed?.to_string())
    }

    /// Return the device to a first launch: close the collection, remove its files and open it again
    /// through `open_collection`, so it is seeded as a fresh install is. Any sitting begun before is
    /// stale after this and should be dropped.
    ///
    /// **This destroys the collection under the two directories `open` was given**, like
    /// [`install_fixture`](CollectionHandle::install_fixture): the Kotlin block that reaches it exists
    /// only in a development build.
    pub fn reset(&self) -> Result<(), CairnError> {
        let mut slot = self
            .collection
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let Some(coll) = slot.take() else {
            // Only a refused reopen below leaves the slot empty; try again from the files.
            *slot = Some(cairn_ops::fixtures::reset_files(
                &self.data_dir,
                &self.state_dir,
            )?);
            return Ok(());
        };
        *slot = Some(cairn_ops::fixtures::reset(
            coll,
            &self.data_dir,
            &self.state_dir,
        )?);
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

    /// The stored appearance choice, read before the first frame (ADR-0036 §3, ADR-0046 §6).
    /// Reading only: changing it is the Settings slice's. Unset or unrecognised is `System`.
    pub fn theme_choice(&self) -> Result<ThemeChoice, CairnError> {
        Ok(cairn_ops::appearance::theme_choice(&self.collection())?.into())
    }

    /// Record the appearance choice (ADR-0036 §3). Device-local; the read above decides what it means.
    pub fn set_theme_choice(&self, choice: ThemeChoice) -> Result<(), CairnError> {
        cairn_ops::appearance::set_theme_choice(&mut self.collection(), choice.into())?;
        Ok(())
    }

    /// The new-card rate (ADR-0011 §3): the stored value, or the default when unset.
    pub fn new_card_rate(&self) -> Result<u32, CairnError> {
        Ok(cairn_ops::new_card_rate::rate(&self.collection())?)
    }

    /// Set the new-card rate and return **the value stored after the clamp**, so the field can show
    /// what the app did with an out-of-range entry. Zero is kept (ADR-0011 §5).
    pub fn set_new_card_rate(&self, rate: u32) -> Result<u32, CairnError> {
        Ok(cairn_ops::new_card_rate::set_rate(
            &mut self.collection(),
            rate,
        )?)
    }

    /// The fact Settings' nudge states (ADR-0014 §2): either the standard parameters with the review
    /// total, or the fit with the reviews since. Kotlin words it; no sentence crosses.
    pub fn optimisation_nudge(&self) -> Result<OptimisationNudge, CairnError> {
        Ok(optimise::nudge(&self.collection())?.into())
    }

    /// The collection as an archive made at `created_ms`, with the name the save dialog suggests,
    /// dated in local time `offset_minutes` east of UTC (ADR-0053 §4). Records nothing: Kotlin calls
    /// [`record_backup`](Self::record_backup) once the provider took the bytes.
    pub fn backup(&self, created_ms: i64, offset_minutes: i32) -> Result<BackupFacts, CairnError> {
        let built = cairn_ops::backup::build(&self.collection(), created_ms, offset_minutes)?;
        Ok(BackupFacts {
            bytes: built.bytes,
            name: built.name,
            notes: built.notes,
            reviews: built.reviews,
        })
    }

    /// Note that the archive made at `created_ms` was written. Device-local; nothing settles.
    pub fn record_backup(&self, created_ms: i64) -> Result<(), CairnError> {
        cairn_ops::backup::record(&mut self.collection(), created_ms)?;
        Ok(())
    }

    /// The facts the backup nudge states (ADR-0016 §6). Kotlin words it; no sentence crosses.
    pub fn backup_nudge(&self) -> Result<BackupNudge, CairnError> {
        let nudge = cairn_ops::backup::nudge(&self.collection())?;
        Ok(BackupNudge {
            last_backup_ms: nudge.last_backup_ms,
            reviews_since: nudge.reviews_since,
            reviews: nudge.reviews,
            notes: nudge.notes,
        })
    }

    /// Snapshot the log and start an optimise run on a Rust thread (ADR-0014 §3). The handle outlives
    /// the call: Kotlin polls it, and nothing is written until its `finish` finds the run over.
    pub fn start_optimise(self: Arc<Self>) -> Result<Arc<OptimiseHandle>, CairnError> {
        let job = optimise::start(&self.collection())?;
        Ok(Arc::new(OptimiseHandle {
            owner: self,
            job: Mutex::new(job),
        }))
    }

    /// The note list under a deck selector and a search text (ADR-0021 §2): every row in list order,
    /// the decks the collection holds, and the two facts the screen's states turn on. The search
    /// text is `cairn_ops::notes::search_text`'s.
    pub fn note_list(&self, deck: DeckSelector, text: String) -> Result<NoteListFacts, CairnError> {
        let coll = self.collection();
        let held = coll.decks()?;
        let filter = Filter {
            deck: deck.into_filter()?,
            text: notes::search_text(&text),
            ..Filter::default()
        };
        let rows = notes::list(&coll, &filter)?
            .into_iter()
            .map(|row| {
                let preview = row.preview().to_owned();
                // A reference naming no held deck is unfiled (ADR-0005 §8).
                let deck = row.deck.as_deref().and_then(|id| {
                    held.iter()
                        .find(|(held_id, _)| held_id.to_canonical() == id)
                });
                NoteRowFact {
                    note_id: row.id.to_canonical(),
                    rtl: review::is_rtl(&preview),
                    preview,
                    deck_id: deck.map(|(id, _)| id.to_canonical()),
                    deck_name: deck.map(|(_, name)| name.clone()),
                }
            })
            .collect();
        Ok(NoteListFacts {
            rows,
            captioned: matches!(filter.deck, DeckFilter::All) && !held.is_empty(),
            has_notes: notes::any_notes(&coll)?,
            decks: held
                .into_iter()
                .map(|(id, name)| DeckFact {
                    id: id.to_canonical(),
                    name,
                })
                .collect(),
        })
    }

    /// How many notes the deck holds — what the delete warning names (ADR-0021 §9).
    pub fn notes_in_deck(&self, deck: String) -> Result<u64, CairnError> {
        let deck = parse_deck(&deck)?;
        Ok(notes::count_in_deck(&self.collection(), deck)? as u64)
    }

    /// Write one position value placing `moving` at `gap` among the `visible` notes (the list as
    /// drawn, without the moving note) — `cairn_ops::notes::place_between`.
    pub fn place_note(
        &self,
        moving: String,
        visible: Vec<String>,
        gap: u32,
    ) -> Result<(), CairnError> {
        let moving = parse_note(&moving)?;
        let visible = visible
            .iter()
            .map(|id| parse_note(id))
            .collect::<Result<Vec<_>, _>>()?;
        notes::place_between(&mut self.collection(), moving, &visible, gap as usize)?;
        Ok(())
    }

    /// Delete a note; there is no undelete (ADR-0021 §2).
    pub fn delete_note(&self, note: String) -> Result<(), CairnError> {
        let note = parse_note(&note)?;
        notes::delete_note(&mut self.collection(), note)?;
        Ok(())
    }

    /// The leech screen's two sections against the day of `now_wall_ms` — `cairn_ops::leeches::list`.
    pub fn leech_list(&self, now_wall_ms: i64) -> Result<LeechListFacts, CairnError> {
        let list = leeches::list(&self.collection(), now_wall_ms)?;
        let row = |card: CardRef,
                   prompt: Option<Vec<review::Run>>,
                   note_preview: String,
                   cost: Option<LeechCost>| {
            let generated = prompt.is_some();
            let prompt = runs(prompt.unwrap_or_default());
            LeechRowFact {
                note_id: card.note.to_canonical(),
                slot: card.ordinal,
                rtl: review::is_rtl(&prompt.iter().map(|r| r.text.as_str()).collect::<String>()),
                prompt,
                generated,
                note_preview_rtl: review::is_rtl(&note_preview),
                note_preview,
                cost,
            }
        };
        Ok(LeechListFacts {
            active: list
                .active
                .into_iter()
                .map(|r| {
                    let cost = LeechCost {
                        failure_days: r.failure_days,
                        reviews: r.reviews,
                        answer_minutes: r.answer_minutes,
                    };
                    row(r.card, r.prompt, r.note_preview, Some(cost))
                })
                .collect(),
            suspended: list
                .suspended
                .into_iter()
                .map(|r| row(r.card, r.prompt, r.note_preview, None))
                .collect(),
        })
    }

    /// Suspend one card of a note (ADR-0010 §8).
    pub fn suspend_card(&self, note: String, slot: u16) -> Result<(), CairnError> {
        let card = CardRef::new(parse_note(&note)?, slot);
        leeches::suspend(&mut self.collection(), card)?;
        Ok(())
    }

    /// Unsuspend one card of a note.
    pub fn unsuspend_card(&self, note: String, slot: u16) -> Result<(), CairnError> {
        let card = CardRef::new(parse_note(&note)?, slot);
        leeches::unsuspend(&mut self.collection(), card)?;
        Ok(())
    }

    /// Create a deck and return its id; a blank name makes nothing and returns `None`.
    pub fn create_deck(&self, name: String) -> Result<Option<String>, CairnError> {
        Ok(notes::create_deck(&mut self.collection(), &name)?.map(|id| id.to_canonical()))
    }

    /// Delete a deck, returning how many notes went with it.
    pub fn delete_deck(&self, deck: String) -> Result<u64, CairnError> {
        let deck = parse_deck(&deck)?;
        Ok(notes::delete_deck(&mut self.collection(), deck)? as u64)
    }

    /// Open the editor on a stored note. Refused for an id that names no note.
    pub fn edit_note(self: Arc<Self>, note: String) -> Result<Arc<EditingHandle>, CairnError> {
        let id = parse_note(&note)?;
        let editing = Editing::for_note(&self.collection(), id);
        if editing.kind.is_empty() {
            return Err(CairnError::Failed {
                reason: format!("{note:?} is not a stored note"),
            });
        }
        Ok(EditingHandle::wrap(self, editing, false))
    }

    /// Open the editor on a new draft of `kind`, filed to `deck` when it is born. Nothing is stored
    /// until a field has text (ADR-0021 §7).
    pub fn new_draft(
        self: Arc<Self>,
        kind: String,
        deck: Option<String>,
    ) -> Result<Arc<EditingHandle>, CairnError> {
        Ok(EditingHandle::draft(
            self,
            &kind,
            parse_optional_deck(deck)?,
        ))
    }
}

/// One field of the editor: its name, what the store holds, and whether Enter types a new line in it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FieldFact {
    pub name: String,
    pub value: String,
    pub multiline: bool,
}

/// A field's text as Kotlin holds it, handed over to settle.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FieldValue {
    pub name: String,
    pub value: String,
}

/// The editor on one note, as the sitting is a handle (ADR-0046 §7). The text being typed is Kotlin's;
/// this holds the rules around it: when a note is born, what settles, when the idle window falls due.
#[derive(uniffi::Object)]
pub struct EditingHandle {
    owner: Arc<CollectionHandle>,
    editing: Mutex<Editing>,
    /// Opened as a draft: the heading's fact, fixed for the screen's life.
    new: bool,
}

impl EditingHandle {
    fn wrap(owner: Arc<CollectionHandle>, editing: Editing, new: bool) -> Arc<Self> {
        Arc::new(Self {
            owner,
            editing: Mutex::new(editing),
            new,
        })
    }

    fn draft(owner: Arc<CollectionHandle>, kind: &str, deck: Option<DeckId>) -> Arc<Self> {
        let mut editing = Editing::new_draft(kind);
        editing.deck = deck;
        Self::wrap(owner, editing, true)
    }

    fn editing(&self) -> MutexGuard<'_, Editing> {
        self.editing.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Put Kotlin's buffers into the editor's, for the names the kind has.
    fn take_values(editing: &mut Editing, values: &[FieldValue]) {
        for (name, buffer) in &mut editing.fields {
            if let Some(v) = values.iter().find(|v| &v.name == name) {
                buffer.clone_from(&v.value);
            }
        }
    }

    fn settle(&self, editing: &mut Editing) {
        editing.note = editor::settle_all(
            &mut self.owner.collection(),
            editing.note,
            &editing.kind,
            &editing.fields,
            editing.deck,
        );
    }
}

#[uniffi::export]
impl EditingHandle {
    /// Whether this editor was opened as a draft — the heading reads *New note* or *Edit note*.
    pub fn is_new(&self) -> bool {
        self.new
    }

    /// The note's id once it exists: always for a stored note, after its first field for a draft.
    pub fn note_id(&self) -> Option<String> {
        self.editing().note.map(|n| n.to_canonical())
    }

    pub fn kind(&self) -> String {
        self.editing().kind.clone()
    }

    /// The kinds the dropdown offers (ADR-0012 §2).
    pub fn kind_options(&self) -> Vec<String> {
        editor::kind_options(&self.editing().kind)
    }

    /// The deck the note is filed under, by id.
    pub fn deck(&self) -> Option<String> {
        self.editing().deck.map(|d| d.to_canonical())
    }

    /// The fields in kind order, each with the value the editor holds.
    pub fn fields(&self) -> Vec<FieldFact> {
        let editing = self.editing();
        editing
            .fields
            .iter()
            .map(|(name, value)| FieldFact {
                multiline: editor::field_is_multiline(&editing.kind, name),
                name: name.clone(),
                value: value.clone(),
            })
            .collect()
    }

    /// A field changed to `value` at `now_mono_ms`: restarts the idle window (ADR-0021 §7).
    pub fn touched(&self, field: String, value: String, now_mono_ms: u64) {
        let mut editing = self.editing();
        Self::take_values(&mut editing, &[FieldValue { name: field, value }]);
        editing.touched(now_mono_ms);
    }

    /// The decks the collection holds, for the deck dropdown.
    pub fn decks(&self) -> Result<Vec<DeckFact>, CairnError> {
        Ok(self
            .owner
            .collection()
            .decks()?
            .into_iter()
            .map(|(id, name)| DeckFact {
                id: id.to_canonical(),
                name,
            })
            .collect())
    }

    /// A field lost focus: commit its value. A draft is born here if the value is not empty. A field
    /// the store already holds as typed is left alone — no row, no stamp.
    pub fn commit_field(&self, field: String, value: String) -> Result<(), CairnError> {
        let mut editing = self.editing();
        if !editor::is_unsettled(&self.owner.collection(), editing.note, &field, &value) {
            return Ok(());
        }
        Self::take_values(
            &mut editing,
            &[FieldValue {
                name: field.clone(),
                value: value.clone(),
            }],
        );
        let born_before = editing.note.is_some();
        let note = editor::commit_field(
            &mut self.owner.collection(),
            editing.note,
            &editing.kind,
            &field,
            &value,
        )?;
        editing.note = note;
        if !born_before && let Some(id) = note {
            editor::set_note_deck(&mut self.owner.collection(), id, editing.deck)?;
        }
        Ok(())
    }

    /// Settle every field, writing only where the value differs from the store. Returns the note id.
    pub fn settle_all(&self, values: Vec<FieldValue>) -> Result<Option<String>, CairnError> {
        let mut editing = self.editing();
        Self::take_values(&mut editing, &values);
        self.settle(&mut editing);
        Ok(editing.note.map(|n| n.to_canonical()))
    }

    /// The ticking check: settle once the buffers have sat still for the idle window. Returns the
    /// milliseconds until it falls due, or `None` when nothing is pending.
    pub fn settle_if_idle(&self, now_mono_ms: u64) -> Result<Option<u64>, CairnError> {
        let left = self.editing().settle_if_idle(
            &mut self.owner.collection(),
            now_mono_ms,
            editor::SETTLE_AFTER_IDLE,
        );
        Ok(left.map(|d| d.as_millis() as u64))
    }

    /// Choose another kind. A stored note is written at once; the fields rebuild, carrying the values
    /// whose names the new kind shares.
    pub fn switch_kind(&self, kind: String) -> Result<(), CairnError> {
        let mut editing = self.editing();
        if editing.kind == kind {
            return Ok(());
        }
        self.settle(&mut editing);
        if let Some(id) = editing.note {
            notes::set_kind(&mut self.owner.collection(), id, &kind)?;
        }
        editing.switch_kind(&kind);
        Ok(())
    }

    /// Choose a deck (or none). A stored note is written at once; a draft holds it until it is born.
    pub fn set_deck(&self, deck: Option<String>) -> Result<(), CairnError> {
        let deck = parse_optional_deck(deck)?;
        let mut editing = self.editing();
        if let Some(id) = editing.note {
            editor::set_note_deck(&mut self.owner.collection(), id, deck)?;
        }
        editing.deck = deck;
        Ok(())
    }

    /// The cards this note generates and the dormant ones it keeps, as the store holds them now (the
    /// caller settles first). A draft not yet born has none.
    pub fn card_pane(&self) -> Result<CardPaneFacts, CairnError> {
        let Some(note) = self.editing().note else {
            return Ok(CardPaneFacts {
                entries: Vec::new(),
                state: PaneState::Empty,
                warning: Vec::new(),
            });
        };
        Ok(cards::card_pane(&self.owner.collection(), note)?.into())
    }

    /// The number *Blank it* gives the next blank of `text`: one above the highest ever used, the
    /// note's dormant blanks included (ADR-0012 §3).
    pub fn next_blank_number(&self, text: String) -> Result<u32, CairnError> {
        let Some(note) = self.editing().note else {
            return Ok(u32::from(cairn_core::content::next_blank_number(&text)));
        };
        let pane = cards::card_pane(&self.owner.collection(), note)?;
        Ok(u32::from(cards::next_blank_number(&pane, &text)))
    }

    /// *New note*: settle this one, then open a fresh draft carrying its kind and deck.
    pub fn new_note(&self, values: Vec<FieldValue>) -> Result<Arc<EditingHandle>, CairnError> {
        let mut editing = self.editing();
        Self::take_values(&mut editing, &values);
        self.settle(&mut editing);
        Ok(Self::draft(self.owner.clone(), &editing.kind, editing.deck))
    }
}

/// What a dormant entry is named by (ADR-0018 §3), as facts. The sentence around them is Kotlin's.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum DormantName {
    /// A slot a kind declares: the field roles of its question.
    Roles {
        prompt: Vec<String>,
        answer: Vec<String>,
    },
    /// A cloze blank, by its number.
    Blank { number: u32 },
    /// Neither: the bare slot.
    Slot { slot: u32 },
}

impl From<cards::DormantName> for DormantName {
    fn from(n: cards::DormantName) -> Self {
        match n {
            cards::DormantName::Roles { prompt, answer } => DormantName::Roles { prompt, answer },
            cards::DormantName::Blank(number) => DormantName::Blank {
                number: u32::from(number),
            },
            cards::DormantName::Slot(slot) => DormantName::Slot {
                slot: u32::from(slot),
            },
        }
    }
}

/// A card the note generates, as Review's card is: runs for both faces, the prompt's direction and
/// the box badge's fact.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct LiveCardFact {
    pub slot: u32,
    pub prompt: Vec<Run>,
    pub answer: Vec<Run>,
    pub rtl: bool,
    /// Never reviewed: the badge reads *new*, not *box n*.
    pub is_new: bool,
    pub box_number: u8,
}

/// A dormant entry: what it is named by and how many reviews are kept.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct DormantFact {
    pub slot: u32,
    pub name: DormantName,
    pub reviews: u32,
}

/// One entry of the card pane.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum PaneEntry {
    Live { card: LiveCardFact },
    Dormant { entry: DormantFact },
}

/// Which of the pane's three states the note is in (ADR-0018 §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum PaneState {
    Cards,
    NoLiveCards,
    Empty,
}

/// The card pane's facts: entries in raw slot order, the state, and the dormant entries the warning
/// above the fields names.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct CardPaneFacts {
    pub entries: Vec<PaneEntry>,
    pub state: PaneState,
    pub warning: Vec<DormantFact>,
}

fn dormant_fact(d: &cards::DormantEntry) -> DormantFact {
    DormantFact {
        slot: u32::from(d.slot),
        name: cards::dormant_name(d.slot).into(),
        reviews: d.reviews,
    }
}

impl From<cards::CardPane> for CardPaneFacts {
    fn from(pane: cards::CardPane) -> Self {
        let entries = pane
            .entries
            .iter()
            .map(|e| match e {
                cards::Entry::Live(c) => PaneEntry::Live {
                    card: LiveCardFact {
                        slot: u32::from(c.slot),
                        prompt: runs(review::runs(&c.prompt)),
                        answer: runs(review::runs(&c.answer)),
                        rtl: review::is_rtl(&c.prompt),
                        is_new: c.reviews == 0,
                        box_number: c.box_,
                    },
                },
                cards::Entry::Dormant(d) => PaneEntry::Dormant {
                    entry: dormant_fact(d),
                },
            })
            .collect();
        CardPaneFacts {
            entries,
            state: match pane.state {
                cards::State::Cards => PaneState::Cards,
                cards::State::NoLiveCards => PaneState::NoLiveCards,
                cards::State::Empty => PaneState::Empty,
            },
            warning: pane
                .warning
                .map(|w| w.dormant.iter().map(dormant_fact).collect())
                .unwrap_or_default(),
        }
    }
}

fn parse_note(id: &str) -> Result<NoteId, CairnError> {
    NoteId::parse_canonical(id).ok_or_else(|| CairnError::Failed {
        reason: format!("{id:?} is not a note id"),
    })
}

fn parse_optional_deck(id: Option<String>) -> Result<Option<DeckId>, CairnError> {
    id.as_deref().map(parse_deck).transpose()
}

fn parse_deck(id: &str) -> Result<DeckId, CairnError> {
    DeckId::parse_canonical(id).ok_or_else(|| CairnError::Failed {
        reason: format!("{id:?} is not a deck id"),
    })
}

/// Which decks the note list admits: the mirror of `cairn_ops::notes::DeckFilter`, the deck by its
/// canonical id text.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum DeckSelector {
    All,
    Deck { id: String },
    Unfiled,
}

impl DeckSelector {
    fn into_filter(self) -> Result<DeckFilter, CairnError> {
        Ok(match self {
            DeckSelector::All => DeckFilter::All,
            DeckSelector::Deck { id } => DeckFilter::Deck(parse_deck(&id)?),
            DeckSelector::Unfiled => DeckFilter::Unfiled,
        })
    }
}

/// A deck the collection holds.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct DeckFact {
    pub id: String,
    pub name: String,
}

/// One note's row as facts. No schedule information, by construction (ADR-0001 §3).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct NoteRowFact {
    pub note_id: String,
    /// The note's first field value, whole: truncating it is the row's drawing.
    pub preview: String,
    /// Whether the preview's first strong character runs right to left.
    pub rtl: bool,
    /// The deck the note is filed under; both `None` when it is unfiled.
    pub deck_id: Option<String>,
    pub deck_name: Option<String>,
}

/// The note list and what the screen's states turn on.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct NoteListFacts {
    pub rows: Vec<NoteRowFact>,
    pub decks: Vec<DeckFact>,
    /// Whether the collection holds any note at all: false is the empty state, true with no rows is
    /// "no match".
    pub has_notes: bool,
    /// Whether rows carry their deck as a caption (ADR-0039 §3): under *All decks*, in a collection
    /// that holds a deck.
    pub captioned: bool,
}

/// What a leech's cost line is made of; a suspended row has none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct LeechCost {
    pub failure_days: u32,
    pub reviews: u32,
    /// Whole minutes of answer time, rounded half up.
    pub answer_minutes: u32,
}

/// One card on the leech screen, as facts. An unsuspended card is one slot of one note.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct LeechRowFact {
    pub note_id: String,
    pub slot: u16,
    /// The prompt as Review's runs; empty when the card is no longer generated.
    pub prompt: Vec<Run>,
    /// Whether the prompt begins right to left.
    pub rtl: bool,
    /// False when the card renders nothing (ADR-0002 §7).
    pub generated: bool,
    /// The note's preview, whole, for the delete confirmation, and its direction.
    pub note_preview: String,
    pub note_preview_rtl: bool,
    /// `Some` for an active row, `None` for a suspended one.
    pub cost: Option<LeechCost>,
}

/// The leech screen's sections: active leeches worst first, then suspended cards by identity.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct LeechListFacts {
    pub active: Vec<LeechRowFact>,
    pub suspended: Vec<LeechRowFact>,
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
    /// Cards over the leech floor and not suspended: the leeches the leech screen lists.
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
    /// Monospace and slanted: an italic that wraps a code span, drawn so by the egui card too.
    CodeItalic,
}

impl From<review::RunStyle> for RunStyle {
    fn from(s: review::RunStyle) -> Self {
        match s {
            review::RunStyle::Plain => RunStyle::Plain,
            review::RunStyle::Bold => RunStyle::Bold,
            review::RunStyle::Italic => RunStyle::Italic,
            review::RunStyle::BoldItalic => RunStyle::BoldItalic,
            review::RunStyle::Code => RunStyle::Code,
            review::RunStyle::CodeItalic => RunStyle::CodeItalic,
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

/// A built backup, not yet written: the archive's bytes, the suggested name and the counts the
/// report states (ADR-0053 §4).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct BackupFacts {
    pub bytes: Vec<u8>,
    pub name: String,
    pub notes: u64,
    pub reviews: u64,
}

/// What the backup nudge states (ADR-0016 §6): the mirror of `cairn_ops::backup::Nudge`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct BackupNudge {
    /// The last backup's creation instant in epoch milliseconds, or none before the first.
    pub last_backup_ms: Option<i64>,
    /// Reviews after the last backup; every review when there is none.
    pub reviews_since: u64,
    pub reviews: u64,
    pub notes: u64,
}

/// The name a backup made at `created_ms` suggests, `offset_minutes` east of UTC — what the save
/// dialog opens with, known before the archive is built (ADR-0053 §4).
#[uniffi::export]
pub fn backup_filename(created_ms: i64, offset_minutes: i32) -> String {
    cairn_ops::backup::filename(created_ms, offset_minutes)
}

/// What the optimise nudge states (ADR-0014 §2): the mirror of `cairn_ops::optimise::OptimisationNudge`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum OptimisationNudge {
    /// No run has written a vector: the standard parameters, over this many reviews in all.
    Standard { reviews_total: u64 },
    /// A run fitted the vector over `fitted_over` reviews, and `reviews_since` have been done since.
    Fitted {
        fitted_over: u64,
        reviews_since: u64,
    },
}

impl From<optimise::OptimisationNudge> for OptimisationNudge {
    fn from(nudge: optimise::OptimisationNudge) -> Self {
        match nudge {
            optimise::OptimisationNudge::Standard { reviews_total } => {
                Self::Standard { reviews_total }
            }
            optimise::OptimisationNudge::Fitted {
                fitted_over,
                reviews_since,
            } => Self::Fitted {
                fitted_over,
                reviews_since,
            },
        }
    }
}

/// Where a run is (ADR-0014 §4): the uncancellable lead-in, or the determinate training loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum OptimisePhase {
    Preparing,
    Training { current: u64, total: u64 },
}

/// What `finish` found: the run going on, over with nothing written (cancelled, or the vector it
/// fitted is the one already held), or over with a new vector written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum OptimiseFinish {
    StillRunning,
    NothingWritten,
    Written,
}

/// One optimise run over the collection it was started on. Kotlin never holds a parameter: `finish`
/// does the write itself.
#[derive(uniffi::Object)]
pub struct OptimiseHandle {
    owner: Arc<CollectionHandle>,
    job: Mutex<optimise::OptimiseJob>,
}

impl OptimiseHandle {
    fn job(&self) -> MutexGuard<'_, optimise::OptimiseJob> {
        self.job.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[uniffi::export]
impl OptimiseHandle {
    /// *Preparing*, or *training* with the steps done and the steps in all.
    pub fn phase(&self) -> OptimisePhase {
        match self.job().phase() {
            optimise::Phase::Preparing => OptimisePhase::Preparing,
            optimise::Phase::Training { current, total } => OptimisePhase::Training {
                current: current as u64,
                total: total as u64,
            },
        }
    }

    /// Ask the run to stop; it writes nothing.
    pub fn cancel(&self) {
        self.job().cancel();
    }

    /// Collect the run if it is over and write its outcome, stamped `now_wall_ms`.
    pub fn finish(&self, now_wall_ms: i64) -> Result<OptimiseFinish, CairnError> {
        let mut coll = self.owner.collection();
        let found = optimise::finish(&mut coll, &mut self.job(), now_wall_ms)?;
        Ok(match found {
            optimise::Finish::StillRunning => OptimiseFinish::StillRunning,
            optimise::Finish::NothingWritten => OptimiseFinish::NothingWritten,
            optimise::Finish::Written => OptimiseFinish::Written,
        })
    }
}

/// One sitting: the cairn-ops sitting over the collection it was begun on. Every time is passed in.
#[derive(uniffi::Object)]
pub struct SittingHandle {
    owner: Arc<CollectionHandle>,
    review: Mutex<Review>,
}

/// One fixture on the development bench: the key `install_fixture` takes and what it reaches.
///
/// `label` and `reaches` are the bench's own text, as in the egui build, so they cross as they are
/// and are not a Kotlin string resource: the bench is a development tool, not a user-facing screen,
/// so ADR-0046 §4's words-stay-in-Kotlin rule is not applied to it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct BenchFixture {
    pub key: String,
    pub label: String,
    pub reaches: String,
}

/// The development bench, in its own order: every fixture with its key and what it reaches.
#[uniffi::export]
pub fn fixtures() -> Vec<BenchFixture> {
    cairn_ops::fixtures::Fixture::ALL
        .into_iter()
        .map(|f| BenchFixture {
            key: f.key().to_owned(),
            label: f.label().to_owned(),
            reaches: f.reaches().to_owned(),
        })
        .collect()
}

/// How a file reached the application: the mirror of `cairn_ops::inbound::Arrival` for the three
/// ways the Kotlin app is handed one. It names the arrival in the log and changes nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FileArrival {
    /// Chosen in the system picker (*Open a file…*).
    Picked,
    /// Opened from a file manager (`ACTION_VIEW`).
    Opened,
    /// Shared from another application (`ACTION_SEND`).
    Shared,
}

impl From<FileArrival> for inbound::Arrival {
    fn from(arrival: FileArrival) -> Self {
        match arrival {
            FileArrival::Picked => Self::Picked,
            FileArrival::Opened => Self::Opened,
            FileArrival::Shared => Self::Shared,
        }
    }
}

/// Which gate a preview is owed (decided in `cairn-ops`, shared with the desktop).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ImportGate {
    /// *Import* and *Cancel*.
    Decide,
    /// A lone *Close*.
    Dismiss,
}

impl From<inbound::Gate> for ImportGate {
    fn from(gate: inbound::Gate) -> Self {
        match gate {
            inbound::Gate::Decide => Self::Decide,
            inbound::Gate::Dismiss => Self::Dismiss,
        }
    }
}

/// Why a file cannot be previewed, as a fact; Kotlin chooses the sentence (ADR-0046 §4).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum FileRefusal {
    /// Not an archive this application can read, or a read that stopped short of a deck.
    Unreadable,
    /// A `format` this build cannot read.
    NeedsNewerVersion,
    /// An absolute path, a `..` segment, a symlink or an unknown member.
    NotADeck,
    /// An older revision of a deck the collection holds, named by the held deck's name.
    OlderCopy { deck: String },
    /// A collection archive from a collection other than the one this device holds (ADR-0016 §10).
    AnotherCollection,
    /// A read stopped at its ceiling whose head is a collection archive's.
    TooLargeToRead,
}

impl From<Refusal> for FileRefusal {
    fn from(refusal: Refusal) -> Self {
        match refusal {
            Refusal::Unreadable => Self::Unreadable,
            Refusal::UnknownFormat(_) => Self::NeedsNewerVersion,
            // An archive is routed to `restore` before `inbound` reads it, so a deck read never
            // meets the other profile; were one to, it is not a deck this application can read.
            Refusal::WrongProfile => Self::Unreadable,
            Refusal::BrokenPath => Self::NotADeck,
            Refusal::Older { deck } => Self::OlderCopy { deck },
        }
    }
}

impl From<restore::Refusal> for FileRefusal {
    fn from(refusal: restore::Refusal) -> Self {
        match refusal {
            // A broken archive is not a deck put together wrongly: it is a file this application
            // cannot read.
            restore::Refusal::Unreadable
            | restore::Refusal::WrongProfile
            | restore::Refusal::BrokenPath => Self::Unreadable,
            restore::Refusal::UnknownFormat(_) => Self::NeedsNewerVersion,
            restore::Refusal::AnotherCollection => Self::AnotherCollection,
            restore::Refusal::TooLarge => Self::TooLargeToRead,
        }
    }
}

/// Whether a deck in the file is one the collection holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum DeckPath {
    /// The collection holds the deck, and the file updates it.
    Update,
    /// The collection holds no such deck, and the file creates it.
    Create,
}

impl From<inbound::Path> for DeckPath {
    fn from(path: inbound::Path) -> Self {
        match path {
            inbound::Path::Update => Self::Update,
            inbound::Path::Create => Self::Create,
        }
    }
}

/// A count from a file's plan, as it crosses. A count no `u64` holds saturates rather than wraps.
fn count(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

/// Notes moving into a deck from one the collection holds; `from` is `None` for unfiled notes.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MovingInFacts {
    pub from: Option<String>,
    pub count: u64,
}

/// What importing would do to one deck of the file.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct DeckEffects {
    pub name: String,
    pub path: DeckPath,
    pub new_notes: u64,
    pub already_yours: u64,
    pub moving_in: Vec<MovingInFacts>,
    pub deleted: u64,
    /// The collection's own name for the deck, when the file renames it.
    pub renamed_from: Option<String>,
    pub no_change: bool,
    /// The same revision as the held deck, with different content.
    pub revision_conflict: bool,
}

/// What importing a file would do, as facts. The header's fields are empty when the file has none.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ImportPlanFacts {
    pub author: String,
    pub licence: String,
    pub description: String,
    pub decks: Vec<DeckEffects>,
    /// Card types the file adds that this build does not have.
    pub adopted_kinds: Vec<String>,
    /// Held decks the import leaves empty.
    pub emptied_decks: Vec<String>,
    pub gate: ImportGate,
}

/// What restoring a collection archive would bring in, as its manifest declares it (ADR-0016 §11).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RestorePreviewFacts {
    /// The archive's creation instant in epoch milliseconds, for Kotlin to show in local time;
    /// `None` when the archive states it in a form this application does not write.
    pub created_ms: Option<i64>,
    pub notes: u64,
    pub reviews: u64,
    /// Always *decide*: a restore adds and never removes, so there is always a choice to make.
    pub gate: ImportGate,
}

/// A refusal, an import plan or a restore preview, each with its gate.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum FilePreview {
    Refused {
        refusal: FileRefusal,
        gate: ImportGate,
    },
    Plan {
        plan: ImportPlanFacts,
    },
    Restore {
        restore: RestorePreviewFacts,
    },
}

/// What applying a file did.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum FileApplied {
    /// Written. `filter_to` is the deck id to filter the note list to, `None` when the file held
    /// several decks.
    Landed { filter_to: Option<String> },
    /// The collection changed under the preview and the file is now refused.
    Refused { refusal: FileRefusal },
}

impl FilePreview {
    fn of_restore(outcome: Result<restore::Preview, restore::Refusal>) -> Self {
        match outcome {
            Ok(preview) => Self::Restore {
                restore: RestorePreviewFacts {
                    created_ms: preview.created_ms,
                    notes: preview.notes,
                    reviews: preview.reviews,
                    gate: ImportGate::Decide,
                },
            },
            Err(refusal) => Self::Refused {
                refusal: refusal.into(),
                gate: ImportGate::Dismiss,
            },
        }
    }

    fn of(outcome: Result<Plan, Refusal>) -> Self {
        let gate = inbound::gate_for(&outcome).into();
        match outcome {
            Err(refusal) => Self::Refused {
                refusal: refusal.into(),
                gate,
            },
            Ok(plan) => Self::Plan {
                plan: ImportPlanFacts {
                    author: plan.header.author,
                    licence: plan.header.licence,
                    description: plan.header.description,
                    decks: plan
                        .decks
                        .into_iter()
                        .map(|d| DeckEffects {
                            name: d.name,
                            path: d.path.into(),
                            new_notes: count(d.new_notes),
                            already_yours: count(d.already_yours),
                            moving_in: d
                                .moving_in
                                .into_iter()
                                .map(|m| MovingInFacts {
                                    from: m.from,
                                    count: count(m.count),
                                })
                                .collect(),
                            deleted: count(d.deleted),
                            renamed_from: d.renamed_from,
                            no_change: d.no_change,
                            revision_conflict: d.revision_conflict,
                        })
                        .collect(),
                    adopted_kinds: plan.adopted_kinds,
                    emptied_decks: plan.emptied_decks,
                    gate,
                },
            },
        }
    }
}

/// A file Kotlin read and handed over. The bytes are held here, in Rust, so they cross once however
/// often the preview is read again; the plan is derived on every `preview()` and never held
/// (ADR-0022 §5).
#[derive(uniffi::Object)]
pub struct InboundFileHandle {
    owner: Arc<CollectionHandle>,
    inbound: inbound::Inbound,
    /// False when Kotlin stopped at the read ceiling: only the head is held.
    complete: bool,
    /// Whether the head is a collection archive's, sniffed once when the file is opened: an archive
    /// is read by `restore`, anything else by `inbound` (ADR-0053 §2).
    archive: bool,
}

impl InboundFileHandle {
    /// The refusal an incomplete read meets before anything is planned; the rule is `cairn-ops`'s.
    fn head_refusal(&self) -> Refusal {
        inbound::head_refusal(&self.inbound.bytes)
    }

    /// The restore preview, derived against the collection as it stands now.
    fn restore(&self) -> Result<Result<restore::Preview, restore::Refusal>, CairnError> {
        Ok(restore::read(
            &self.inbound.bytes,
            self.complete,
            &self.owner.collection(),
        )?)
    }
}

#[uniffi::export]
impl InboundFileHandle {
    /// What importing or restoring the file would do to the collection as it stands now, or why it
    /// can't.
    pub fn preview(&self) -> Result<FilePreview, CairnError> {
        if self.archive {
            return Ok(FilePreview::of_restore(self.restore()?));
        }
        if !self.complete {
            return Ok(FilePreview::of(Err(self.head_refusal())));
        }
        let report = inbound::read(&self.inbound, &self.owner.collection())?;
        Ok(FilePreview::of(report.outcome))
    }

    /// Re-derive the plan and write it. A store failure is the binding's one error.
    ///
    /// **Restoring is not built yet** (#253): an archive the re-read refuses says why, and one this
    /// device would accept is an error, with nothing written either way.
    pub fn apply(&self) -> Result<FileApplied, CairnError> {
        if self.archive {
            return match self.restore()? {
                Err(refusal) => Ok(FileApplied::Refused {
                    refusal: refusal.into(),
                }),
                Ok(_) => Err(CairnError::Failed {
                    reason: "restoring an archive is not built yet".to_owned(),
                }),
            };
        }
        if !self.complete {
            return Ok(FileApplied::Refused {
                refusal: self.head_refusal().into(),
            });
        }
        let applied = inbound::apply(&self.inbound, &mut self.owner.collection())?;
        Ok(match applied {
            Ok(applied) => FileApplied::Landed {
                filter_to: applied.filter_to.map(|id| id.to_canonical()),
            },
            Err(refusal) => FileApplied::Refused {
                refusal: refusal.into(),
            },
        })
    }
}

#[uniffi::export]
impl CollectionHandle {
    /// Hold a file the platform handed over. `name` is the display name it carried, if any;
    /// `complete` is false when the read stopped at its ceiling and `bytes` is only the head.
    pub fn open_file(
        self: Arc<Self>,
        arrival: FileArrival,
        name: Option<String>,
        bytes: Vec<u8>,
        complete: bool,
    ) -> Arc<InboundFileHandle> {
        Arc::new(InboundFileHandle {
            owner: self,
            archive: restore::is_archive(&bytes),
            inbound: inbound::Inbound {
                arrival: arrival.into(),
                name,
                bytes,
            },
            complete,
        })
    }
}

/// One of the bench's deck files: its name and its bytes.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct BenchFileFact {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// The bench's files, built against `Fixture::Decks`. Like [`fixtures`], in every build of the
/// binding, so the tests and the handset session use the files the desktop bench writes.
///
/// `scratch_dir` is a directory the app may write to — its cache directory on Android, where the
/// system temp directory is not the app's: the archive is a backup of a store opened and removed
/// again under it.
#[uniffi::export]
pub fn bench_files(scratch_dir: String) -> Vec<BenchFileFact> {
    let set = cairn_ops::file_bench::FileSet::Imports;
    set.files_in(Path::new(&scratch_dir))
        // The set is built from the bench's own constant fixtures, never from input, and
        // `file_bench`'s tests build it on every run: a failure here is a broken build, not a state.
        .expect("the bench's files build")
        .into_iter()
        .map(|f| BenchFileFact {
            name: f.name,
            bytes: f.bytes,
        })
        .collect()
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

    /// Re-read the card on screen after its note was edited: the same card with its new content and
    /// reveal, or the next card if the edit stopped it generating (no grade recorded, no count
    /// advanced), or `None` once the queue is empty. The rule is cairn-ops'.
    pub fn refresh(
        &self,
        now_wall_ms: i64,
        now_mono_ms: u64,
    ) -> Result<Option<ShownCard>, CairnError> {
        let shown = self
            .review()
            .refresh(&self.owner.collection(), now_wall_ms, now_mono_ms)?;
        Ok(shown.map(ShownCard::from))
    }

    /// Show the answer of the card on screen.
    pub fn reveal(&self) -> Result<(), CairnError> {
        Ok(self.review().reveal()?)
    }

    /// Grade the card on screen; refused as `NotRevealed` before the reveal, `NoCardShown` with none.
    pub fn grade(
        &self,
        grade: Grade,
        now_mono_ms: u64,
        now_wall_ms: i64,
    ) -> Result<GradeOutcome, CairnError> {
        let outcome = self.review().grade(
            &mut self.owner.collection(),
            grade.into(),
            now_mono_ms,
            now_wall_ms,
        )?;
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

        // `open` seeds a first-launch deck, so the empty collection is only reached by name.
        let (handle, _d, _s) = benched("empty");
        let facts = handle.review_state(NOW).unwrap();
        assert_eq!(facts.state, ReviewStateKind::Empty);
        assert_eq!((facts.due, facts.new, facts.available), (0, 0, 0));
        assert_eq!(handle.note_count().unwrap(), 0);

        let (handle, _d, _s) = benched("new-only");
        let facts = handle.review_state(NOW).unwrap();
        assert_eq!(facts.state, ReviewStateKind::NewOnly { new: 2 });
        assert_eq!((facts.due, facts.new, facts.available), (0, 2, 2));
    }

    #[test]
    fn the_fresh_deck_and_new_only_states_are_reached_by_reviewing() {
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
        let refused = handle
            .install_fixture("no-such-fixture".to_owned(), NOW)
            .unwrap_err();
        assert!(
            matches!(&refused, CairnError::UnknownFixture { name } if name == "no-such-fixture"),
            "{refused:?}"
        );
        assert_eq!(handle.note_count().unwrap(), before);
    }

    #[test]
    fn reset_returns_the_handle_to_a_first_launch() {
        let (data, state) = dirs();
        let handle = open(&data, &state);
        let seeded = handle.note_count().unwrap();
        handle.install_fixture("backlog".to_owned(), NOW).unwrap();
        assert_ne!(handle.note_count().unwrap(), seeded);

        handle.reset().unwrap();

        assert_eq!(handle.note_count().unwrap(), seeded);
        // The handle is usable afterwards: it holds the reopened collection.
        assert!(handle.due_count(NOW).is_ok());
    }

    #[test]
    fn the_bench_lists_every_fixture_with_its_key_and_what_it_reaches() {
        let listed = fixtures();
        assert_eq!(listed.len(), cairn_ops::fixtures::Fixture::ALL.len());
        let caught_up = listed.iter().find(|f| f.key == "caught-up").unwrap();
        assert_eq!(
            caught_up.reaches,
            cairn_ops::fixtures::Fixture::CaughtUp.reaches()
        );
        // Each key installs, so the list is the bench and not a copy of it.
        for f in &listed {
            assert!(
                cairn_ops::fixtures::Fixture::parse(&f.key).is_some(),
                "{}",
                f.key
            );
        }
    }

    #[test]
    fn an_install_says_what_it_reached() {
        let (data, state) = dirs();
        let handle = open(&data, &state);
        let reached = handle.install_fixture("backlog".to_owned(), NOW).unwrap();
        let expected = cairn_ops::fixtures::read(&handle.collection(), NOW).unwrap();
        assert_eq!(reached, expected.to_string());
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
    fn a_refused_grade_or_reveal_crosses_as_a_typed_reason_and_counts_nothing() {
        let (handle, _d, _s) = benched("backlog");
        let sitting = handle.start_sitting(3, 0, NOW).unwrap();
        // Nothing on screen yet: neither a reveal nor a grade has a card to act on.
        assert!(matches!(sitting.reveal(), Err(CairnError::NoCardShown)));
        assert!(matches!(
            sitting.grade(Grade::Good, 5, NOW),
            Err(CairnError::NoCardShown)
        ));
        // A card that is not revealed: the grade is refused for that reason.
        sitting.card(NOW, 10).unwrap().unwrap();
        assert!(matches!(
            sitting.grade(Grade::Good, 20, NOW),
            Err(CairnError::NotRevealed)
        ));
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
    fn the_leech_list_crosses_as_facts_and_suspending_moves_a_row() {
        let (handle, _d, _s) = benched("leeches");
        let list = handle.leech_list(NOW).unwrap();
        assert_eq!(list.active.len(), 3);
        let worst = &list.active[0];
        assert_eq!(text(&worst.prompt), "néanmoins");
        assert!(worst.generated && !worst.rtl);
        assert_eq!(
            worst.cost,
            Some(LeechCost {
                failure_days: 6,
                reviews: 12,
                answer_minutes: 4
            })
        );
        assert_eq!(worst.note_preview, "néanmoins");

        handle
            .suspend_card(worst.note_id.clone(), worst.slot)
            .unwrap();
        let after = handle.leech_list(NOW).unwrap();
        assert_eq!(after.active.len(), 2);
        assert_eq!(after.suspended.len(), 1);
        assert_eq!(after.suspended[0].cost, None);
        assert_eq!(after.suspended[0].note_id, worst.note_id);
        let facts = handle.review_state(NOW).unwrap();
        assert_eq!((facts.leeches, facts.suspended), (2, 1));

        handle
            .unsuspend_card(worst.note_id.clone(), worst.slot)
            .unwrap();
        assert_eq!(handle.leech_list(NOW).unwrap(), list);
    }

    #[test]
    fn a_suspended_card_that_no_longer_generates_is_flagged_and_a_bad_id_is_refused() {
        let (handle, _d, _s) = benched("cloze");
        let note = handle
            .note_list(DeckSelector::All, String::new())
            .unwrap()
            .rows[0]
            .note_id
            .clone();
        // Blank 9 does not exist on the note: the card is dormant.
        handle.suspend_card(note, 0x8000 | 9).unwrap();
        let list = handle.leech_list(NOW).unwrap();
        assert_eq!(list.suspended.len(), 1);
        assert!(!list.suspended[0].generated);
        assert!(list.suspended[0].prompt.is_empty());
        assert!(matches!(
            handle.suspend_card("nope".to_owned(), 0),
            Err(CairnError::Failed { .. })
        ));
        assert!(matches!(
            handle.unsuspend_card("nope".to_owned(), 0),
            Err(CairnError::Failed { .. })
        ));
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

        // Every cloze card the fixture offers: the blank is masked as "[…]" on the prompt, and the
        // answer shows the word the fixture's own text hides, with no mask left.
        let blanks = ["mange", "1889", "Bonjour", "travaille", "Alliés"];
        for _ in 0..4 {
            let shown = sitting.card(NOW, 10).unwrap().unwrap();
            let (prompt, answer) = (text(&shown.prompt), text(&shown.answer));
            assert_eq!(prompt.matches("[…]").count(), 1, "{prompt:?}");
            assert!(!answer.contains("[…]"), "{answer:?}");
            assert!(
                blanks.iter().any(|word| answer.contains(word)),
                "{answer:?} shows none of the hidden words"
            );
            sitting.reveal().unwrap();
            sitting.grade(Grade::Good, 20, NOW).unwrap();
        }
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
            .create_note(
                "basic",
                &[("Front", "a **b** *c* `d` *`e`*"), ("Back", "plain")],
            )
            .unwrap();
        let sitting = handle.start_sitting(1, 0, NOW).unwrap();
        let shown = sitting.card(NOW, 10).unwrap().unwrap();
        assert_eq!(text(&shown.prompt), "a b c d e");
        let styles: Vec<RunStyle> = shown.prompt.iter().map(|r| r.style).collect();
        // An italic around a code span stays slanted, as the egui card draws it.
        assert_eq!(
            styles,
            [
                RunStyle::Plain,
                RunStyle::Bold,
                RunStyle::Plain,
                RunStyle::Italic,
                RunStyle::Plain,
                RunStyle::Code,
                RunStyle::Plain,
                RunStyle::CodeItalic
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
    fn the_new_card_rate_is_read_set_clamped_and_zero_is_kept() {
        let (data, state) = dirs();
        let handle = open(&data, &state);
        assert_eq!(
            handle.new_card_rate().unwrap(),
            5,
            "unset reads the default"
        );
        assert_eq!(handle.set_new_card_rate(20).unwrap(), 20);
        assert_eq!(handle.new_card_rate().unwrap(), 20);
        assert_eq!(
            handle.set_new_card_rate(100_000).unwrap(),
            9_999,
            "the clamp"
        );
        assert_eq!(handle.set_new_card_rate(0).unwrap(), 0, "zero is kept");
        assert_eq!(handle.new_card_rate().unwrap(), 0);
    }

    #[test]
    fn a_set_theme_choice_is_read_back_by_the_one_reader() {
        let (data, state) = dirs();
        let handle = open(&data, &state);
        for choice in [ThemeChoice::Dark, ThemeChoice::Light, ThemeChoice::System] {
            handle.set_theme_choice(choice).unwrap();
            assert_eq!(handle.theme_choice().unwrap(), choice);
        }
    }

    #[test]
    fn the_stored_appearance_choice_is_read_and_unrecognised_follows_the_platform() {
        let (data, state) = dirs();
        let handle = open(&data, &state);
        assert_eq!(handle.theme_choice().unwrap(), ThemeChoice::System, "unset");
        for (stored, expected) in [
            ("light", ThemeChoice::Light),
            ("dark", ThemeChoice::Dark),
            ("system", ThemeChoice::System),
            ("sepia", ThemeChoice::System),
        ] {
            handle.collection().set_theme_preference(stored).unwrap();
            assert_eq!(
                handle.theme_choice().unwrap(),
                expected,
                "stored {stored:?}"
            );
        }
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
        let CairnError::Failed { reason } = err else {
            panic!("a store fault is a logged reason, not a refusal: {err:?}");
        };
        assert!(!reason.is_empty());
    }

    fn previews(facts: &NoteListFacts) -> Vec<String> {
        facts.rows.iter().map(|r| r.preview.clone()).collect()
    }

    fn all(handle: &CollectionHandle, search: &str) -> NoteListFacts {
        handle
            .note_list(DeckSelector::All, search.to_owned())
            .unwrap()
    }

    #[test]
    fn the_note_list_is_every_note_in_list_order_with_the_decks_beside_it() {
        let (handle, _d, _s) = benched("decks");
        let facts = all(&handle, "");
        assert_eq!(facts.rows.len(), 25);
        assert_eq!(facts.rows[0].preview, "le carrefour");
        assert_eq!(facts.rows[0].deck_name.as_deref(), Some("Français"));
        assert!(!facts.rows[0].rtl);
        // The fourth note is unfiled: a row with no deck.
        assert_eq!(facts.rows[3].preview, "le colporteur");
        assert_eq!(facts.rows[3].deck_name, None);
        assert_eq!(facts.rows[3].deck_id, None);
        assert!(facts.rows[6].rtl);
        assert_eq!(facts.decks.len(), 4);
        assert!(facts.decks.iter().any(|d| d.name == "فارسی"));
        assert!(facts.has_notes);
        assert!(facts.captioned, "a collection with decks captions its rows");
    }

    #[test]
    fn the_caption_is_only_for_all_decks_in_a_collection_that_holds_a_deck() {
        let (handle, _d, _s) = benched("decks");
        let french = handle
            .note_list(DeckSelector::All, String::new())
            .unwrap()
            .decks
            .iter()
            .find(|d| d.name == "Français")
            .unwrap()
            .id
            .clone();
        let narrowed = handle
            .note_list(DeckSelector::Deck { id: french }, String::new())
            .unwrap();
        assert!(!narrowed.captioned);
        assert_eq!(narrowed.rows.len(), 10);

        let (handle, _d, _s) = benched("caught-up");
        let facts = all(&handle, "");
        assert!(facts.decks.is_empty());
        assert!(!facts.captioned, "no decks: every row would say Unfiled");
    }

    #[test]
    fn unfiled_narrows_to_the_notes_with_no_deck() {
        let (handle, _d, _s) = benched("decks");
        let facts = handle
            .note_list(DeckSelector::Unfiled, String::new())
            .unwrap();
        assert_eq!(
            previews(&facts),
            vec!["le colporteur", "la girouette", "l'arrière-boutique"]
        );
    }

    #[test]
    fn search_finds_persian_through_its_letter_variants() {
        let (handle, _d, _s) = benched("decks");
        // Arabic kaf and yeh, typed on an Arabic keyboard, find the Persian kaf and yeh (ADR-0049).
        let facts = all(&handle, "\u{0643}\u{062A}\u{0627}\u{0628}");
        assert_eq!(previews(&facts), vec!["کتاب", "کتابخانه"]);
        let none = all(&handle, "zzzz");
        assert!(none.rows.is_empty());
        assert!(none.has_notes, "no match is not an empty collection");
    }

    #[test]
    fn an_empty_collection_has_no_notes() {
        let (handle, _d, _s) = benched("empty");
        assert!(!all(&handle, "").has_notes);
    }

    #[test]
    fn deleting_a_note_removes_it_from_the_list() {
        let (handle, _d, _s) = benched("decks");
        let first = all(&handle, "").rows[0].note_id.clone();
        handle.delete_note(first).unwrap();
        let facts = all(&handle, "");
        assert_eq!(facts.rows.len(), 24);
        assert_eq!(facts.rows[0].preview, "la serrure");
    }

    #[test]
    fn creating_a_deck_returns_its_id_and_lists_it() {
        let (handle, _d, _s) = benched("decks");
        let id = handle
            .create_deck("  Géographie ".to_owned())
            .unwrap()
            .unwrap();
        let facts = all(&handle, "");
        assert!(
            facts
                .decks
                .iter()
                .any(|d| d.id == id && d.name == "Géographie")
        );
        assert_eq!(handle.notes_in_deck(id).unwrap(), 0);
    }

    #[test]
    fn a_blank_deck_name_and_a_search_of_spaces_are_the_operations_rules() {
        let (handle, _d, _s) = benched("decks");
        let before = all(&handle, "").decks.len();
        assert_eq!(handle.create_deck("   ".to_owned()).unwrap(), None);
        assert_eq!(all(&handle, "").decks.len(), before);
        assert_eq!(all(&handle, "   ").rows, all(&handle, "").rows);
    }

    #[test]
    fn deleting_a_deck_takes_its_notes_and_says_how_many() {
        let (handle, _d, _s) = benched("decks");
        let facts = all(&handle, "");
        let french = facts
            .decks
            .iter()
            .find(|d| d.name == "Français")
            .unwrap()
            .id
            .clone();
        assert_eq!(handle.notes_in_deck(french.clone()).unwrap(), 10);
        assert_eq!(handle.delete_deck(french).unwrap(), 10);
        let after = all(&handle, "");
        assert_eq!(after.rows.len(), 15);
        assert_eq!(after.decks.len(), 3);
    }

    #[test]
    fn a_malformed_id_is_a_refusal_not_a_panic() {
        let (handle, _d, _s) = benched("decks");
        assert!(matches!(
            handle.delete_note("not-an-id".to_owned()),
            Err(CairnError::Failed { .. })
        ));
        assert!(handle.delete_deck("nope".to_owned()).is_err());
        assert!(
            handle
                .note_list(DeckSelector::Deck { id: "nope".into() }, String::new())
                .is_err()
        );
    }

    #[test]
    fn placing_a_note_writes_one_value_and_a_hidden_note_keeps_its_place() {
        let (handle, _d, _s) = benched("decks");
        let french = all(&handle, "")
            .decks
            .iter()
            .find(|d| d.name == "Français")
            .unwrap()
            .id
            .clone();
        let sel = DeckSelector::Deck { id: french };
        let rows = handle.note_list(sel.clone(), String::new()).unwrap().rows;
        let order: Vec<String> = rows.iter().map(|r| r.note_id.clone()).collect();
        // Move the first French note to the end of the filtered list.
        let moving = order[0].clone();
        let others: Vec<String> = order[1..].to_vec();
        handle
            .place_note(moving, others.clone(), others.len() as u32)
            .unwrap();
        let after = handle.note_list(sel, String::new()).unwrap();
        assert_eq!(after.rows.last().unwrap().preview, "le carrefour");
        assert_eq!(after.rows[0].preview, "la serrure");
        // The unfiled note that sat among them did not move.
        let whole = all(&handle, "");
        let colporteur = whole
            .rows
            .iter()
            .position(|r| r.preview == "le colporteur")
            .unwrap();
        assert!(
            colporteur
                < whole
                    .rows
                    .iter()
                    .position(|r| r.preview == "le carrefour")
                    .unwrap()
        );
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

    fn blank_bench() -> (Arc<CollectionHandle>, TempDir, TempDir) {
        benched("empty")
    }

    fn value(name: &str, text: &str) -> FieldValue {
        FieldValue {
            name: name.to_owned(),
            value: text.to_owned(),
        }
    }

    fn stored(handle: &Arc<CollectionHandle>, name: &str) -> String {
        let note = handle
            .note_list(DeckSelector::All, String::new())
            .unwrap()
            .rows[0]
            .note_id
            .clone();
        let editing = handle.clone().edit_note(note).unwrap();
        editing
            .fields()
            .into_iter()
            .find(|f| f.name == name)
            .unwrap()
            .value
    }

    #[test]
    fn an_untouched_draft_leaves_nothing_behind() {
        let (handle, _d, _s) = blank_bench();
        let editing = handle.clone().new_draft("basic".into(), None).unwrap();
        assert!(editing.is_new());
        assert_eq!(editing.note_id(), None);
        let born = editing
            .settle_all(vec![value("Front", ""), value("Back", "")])
            .unwrap();
        assert_eq!(born, None);
        assert_eq!(handle.note_count().unwrap(), 0);
    }

    #[test]
    fn a_draft_is_born_on_its_first_field_and_reopening_shows_the_text() {
        let (handle, _d, _s) = blank_bench();
        let editing = handle.clone().new_draft("basic".into(), None).unwrap();
        editing
            .commit_field("Front".into(), "le chat".into())
            .unwrap();
        assert_eq!(handle.note_count().unwrap(), 1);
        let id = editing.note_id().expect("born");
        editing
            .commit_field("Back".into(), "the cat".into())
            .unwrap();
        let again = handle.clone().edit_note(id).unwrap();
        assert!(!again.is_new());
        let fields = again.fields();
        assert_eq!(
            fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
            ["Front", "Back"]
        );
        assert_eq!(fields[0].value, "le chat");
        assert_eq!(fields[1].value, "the cat");
    }

    #[test]
    fn persian_text_is_stored_byte_for_byte() {
        let (handle, _d, _s) = blank_bench();
        let editing = handle.clone().new_draft("basic".into(), None).unwrap();
        let persian = "سلام \u{200c}دنیا";
        editing
            .commit_field("Front".into(), persian.into())
            .unwrap();
        assert_eq!(stored(&handle, "Front").as_bytes(), persian.as_bytes());
    }

    #[test]
    fn the_idle_settle_falls_due_two_seconds_after_the_last_edit() {
        let (handle, _d, _s) = blank_bench();
        let editing = handle.clone().new_draft("basic".into(), None).unwrap();
        editing.touched("Front".into(), "mot".into(), 10_000);
        assert_eq!(editing.settle_if_idle(11_000).unwrap(), Some(1_000));
        assert_eq!(handle.note_count().unwrap(), 0);
        // Another edit restarts the window.
        editing.touched("Front".into(), "mots".into(), 11_500);
        assert_eq!(editing.settle_if_idle(13_000).unwrap(), Some(500));
        assert_eq!(editing.settle_if_idle(13_500).unwrap(), None);
        assert_eq!(handle.note_count().unwrap(), 1);
        assert_eq!(stored(&handle, "Front"), "mots");
        // It fires once per pause.
        assert_eq!(editing.settle_if_idle(20_000).unwrap(), None);
    }

    #[test]
    fn a_draft_holds_its_deck_until_the_note_is_born() {
        let (handle, _d, _s) = blank_bench();
        let deck = handle
            .create_deck("Français".into())
            .unwrap()
            .expect("a deck");
        let editing = handle
            .clone()
            .new_draft("basic".into(), Some(deck.clone()))
            .unwrap();
        assert_eq!(editing.deck(), Some(deck.clone()));
        editing.commit_field("Front".into(), "x".into()).unwrap();
        let rows = handle
            .note_list(DeckSelector::Deck { id: deck }, String::new())
            .unwrap()
            .rows;
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn changing_deck_and_kind_on_a_stored_note_writes_at_once() {
        let (handle, _d, _s) = blank_bench();
        let deck = handle
            .create_deck("Persian".into())
            .unwrap()
            .expect("a deck");
        let editing = handle.clone().new_draft("basic".into(), None).unwrap();
        editing.commit_field("Front".into(), "x".into()).unwrap();
        editing.set_deck(Some(deck.clone())).unwrap();
        assert_eq!(
            handle
                .note_list(DeckSelector::Deck { id: deck }, String::new())
                .unwrap()
                .rows
                .len(),
            1
        );
        assert!(editing.kind_options().contains(&"cloze".to_owned()));
        editing.switch_kind("cloze".into()).unwrap();
        assert_eq!(editing.kind(), "cloze");
        let fields = editing.fields();
        assert_eq!(fields.len(), 1);
        assert!(fields[0].multiline);
        let again = handle
            .clone()
            .edit_note(editing.note_id().unwrap())
            .unwrap();
        assert_eq!(again.kind(), "cloze");
        assert!(!editing_fields_multiline(&handle, "basic"));
    }

    fn editing_fields_multiline(handle: &Arc<CollectionHandle>, kind: &str) -> bool {
        handle
            .clone()
            .new_draft(kind.into(), None)
            .unwrap()
            .fields()
            .iter()
            .any(|f| f.multiline)
    }

    #[test]
    fn new_note_settles_this_one_and_opens_a_draft_with_the_same_kind_and_deck() {
        let (handle, _d, _s) = blank_bench();
        let deck = handle
            .create_deck("Français".into())
            .unwrap()
            .expect("a deck");
        let editing = handle
            .clone()
            .new_draft("basic".into(), Some(deck.clone()))
            .unwrap();
        editing.touched("Front".into(), "un".into(), 1);
        let fresh = editing.new_note(vec![value("Front", "un")]).unwrap();
        assert_eq!(handle.note_count().unwrap(), 1);
        assert!(fresh.is_new());
        assert_eq!(fresh.note_id(), None);
        assert_eq!(fresh.kind(), "basic");
        assert_eq!(fresh.deck(), Some(deck));
        assert!(fresh.fields().iter().all(|f| f.value.is_empty()));
    }

    #[test]
    fn leaving_a_field_that_holds_what_is_stored_writes_nothing() {
        let (handle, _d, _s) = blank_bench();
        let editing = handle.clone().new_draft("basic".into(), None).unwrap();
        editing.commit_field("Front".into(), "a".into()).unwrap();
        let id = editing.note_id().unwrap();
        // Moved away by hand in the store; a blur over the same text must not write it back.
        let rival = handle.clone().edit_note(id).unwrap();
        rival.commit_field("Front".into(), "b".into()).unwrap();
        editing.commit_field("Front".into(), "b".into()).unwrap();
        assert_eq!(stored(&handle, "Front"), "b");
        // A blank draft field leaves the draft unborn.
        let blank = handle.clone().new_draft("basic".into(), None).unwrap();
        blank.commit_field("Front".into(), String::new()).unwrap();
        assert_eq!(blank.note_id(), None);
    }

    #[test]
    fn the_editor_lists_the_decks() {
        let (handle, _d, _s) = blank_bench();
        let deck = handle
            .create_deck("Français".into())
            .unwrap()
            .expect("a deck");
        let editing = handle.clone().new_draft("basic".into(), None).unwrap();
        let decks = editing.decks().unwrap();
        assert_eq!(decks.len(), 1);
        assert_eq!(
            (decks[0].id.as_str(), decks[0].name.as_str()),
            (deck.as_str(), "Français")
        );
    }

    #[test]
    fn a_missing_note_is_refused() {
        let (handle, _d, _s) = blank_bench();
        assert!(handle.clone().edit_note("nonsense".into()).is_err());
    }

    /// Every note the bench holds, opened as Kotlin would, with its pane.
    fn panes(handle: &Arc<CollectionHandle>) -> Vec<(Arc<EditingHandle>, CardPaneFacts)> {
        handle
            .note_list(DeckSelector::All, String::new())
            .unwrap()
            .rows
            .into_iter()
            .map(|row| {
                let editing = handle.clone().edit_note(row.note_id).unwrap();
                let pane = editing.card_pane().unwrap();
                (editing, pane)
            })
            .collect()
    }

    #[test]
    fn a_draft_with_nothing_stored_has_the_empty_pane() {
        let (handle, _d, _s) = blank_bench();
        let editing = handle.clone().new_draft("basic".into(), None).unwrap();
        let pane = editing.card_pane().unwrap();
        assert_eq!(pane.state, PaneState::Empty);
        assert!(pane.entries.is_empty() && pane.warning.is_empty());
    }

    #[test]
    fn a_live_entry_is_drawn_as_review_draws_it_and_a_persian_prompt_is_right_to_left() {
        let (handle, _d, _s) = blank_bench();
        let editing = handle.clone().new_draft("basic".into(), None).unwrap();
        editing
            .settle_all(vec![value("Front", "**سلام**"), value("Back", "hello")])
            .unwrap();
        let pane = editing.card_pane().unwrap();
        assert_eq!(pane.state, PaneState::Cards);
        let [PaneEntry::Live { card }] = pane.entries.as_slice() else {
            panic!("one live entry, got {:?}", pane.entries);
        };
        assert_eq!(card.slot, 0);
        assert_eq!(text(&card.prompt), "سلام");
        assert_eq!(card.prompt[0].style, RunStyle::Bold);
        assert_eq!(text(&card.answer), "hello");
        assert!(card.rtl);
        assert!(card.is_new);
    }

    #[test]
    fn the_dormant_fixture_gives_outlines_names_and_a_warning() {
        let (handle, _d, _s) = benched("dormant");
        let all = panes(&handle);

        // The pruned cloze: blank 1 live, blank 2 dormant, in raw slot order.
        let (_, pruned) = all
            .iter()
            .find(|(_, p)| matches!(p.entries.first(), Some(PaneEntry::Live { card }) if card.slot == 0x8001))
            .expect("the pruned cloze");
        assert_eq!(pruned.state, PaneState::Cards);
        assert!(matches!(
            &pruned.entries[1],
            PaneEntry::Dormant { entry } if entry.name == DormantName::Blank { number: 2 } && entry.reviews == 2
        ));
        assert_eq!(pruned.warning.len(), 1);

        // The emptied vocab: nothing live, two dormant, named by roles.
        let (_, emptied) = all
            .iter()
            .find(|(_, p)| p.state == PaneState::NoLiveCards)
            .expect("a pane with nothing live");
        let names: Vec<_> = emptied.warning.iter().map(|d| d.name.clone()).collect();
        assert_eq!(
            names,
            vec![
                DormantName::Roles {
                    prompt: vec!["Term".into()],
                    answer: vec!["Meaning".into()]
                },
                DormantName::Roles {
                    prompt: vec!["Meaning".into()],
                    answer: vec!["Term".into()]
                },
            ]
        );

        // The stranger's slot is shown by its bare number beside the live card.
        assert!(all.iter().any(|(_, p)| {
            p.warning
                .iter()
                .any(|d| d.name == DormantName::Slot { slot: 7 })
        }));
    }

    #[test]
    fn a_note_with_nothing_dormant_has_no_warning() {
        let (handle, _d, _s) = benched("caught-up");
        assert!(panes(&handle).iter().all(|(_, p)| p.warning.is_empty()));
    }

    #[test]
    fn the_next_blank_is_one_above_the_highest_ever_used_dormant_included() {
        let (handle, _d, _s) = benched("dormant");
        let all = panes(&handle);
        let (pruned, _) = all
            .iter()
            .find(|(_, p)| matches!(p.entries.first(), Some(PaneEntry::Live { card }) if card.slot == 0x8001))
            .unwrap();
        // The text holds only blank 1; blank 2 is dormant with kept reviews, so the next is 3.
        assert_eq!(pruned.next_blank_number("Le {{1::x}} y".into()).unwrap(), 3);
        // A fresh draft has no history: the text alone decides.
        let draft = handle.clone().new_draft("cloze".into(), None).unwrap();
        assert_eq!(draft.next_blank_number("a {{4::b}}".into()).unwrap(), 5);
        assert_eq!(draft.next_blank_number(String::new()).unwrap(), 1);
    }

    /// A cloze note first in authored order and a basic one after it, both stored.
    fn cloze_then_basic() -> (Arc<CollectionHandle>, String, TempDir, TempDir) {
        let (handle, d, s) = blank_bench();
        let cloze = handle.clone().new_draft("cloze".into(), None).unwrap();
        cloze
            .commit_field("Text".into(), "Paris is in {{1::France}}".into())
            .unwrap();
        let id = cloze.note_id().unwrap();
        let basic = handle.clone().new_draft("basic".into(), None).unwrap();
        basic.commit_field("Front".into(), "chien".into()).unwrap();
        (handle, id, d, s)
    }

    fn reviewed_rows(handle: &CollectionHandle) -> usize {
        handle
            .collection()
            .log_lines()
            .unwrap()
            .iter()
            .filter(|line| line.contains("reviewed"))
            .count()
    }

    #[test]
    fn the_sittings_refresh_shows_an_edit_on_the_same_card_still_revealed() {
        let (handle, id, _d, _s) = cloze_then_basic();
        let sitting = handle.clone().start_sitting(5, 0, NOW).unwrap();
        let before = sitting.card(NOW, 10).unwrap().unwrap();
        assert_eq!(before.note_id, id);
        sitting.reveal().unwrap();

        let editing = handle.clone().edit_note(id.clone()).unwrap();
        editing
            .commit_field("Text".into(), "Lyon is in {{1::France}}".into())
            .unwrap();
        let after = sitting.refresh(NOW, 20).unwrap().unwrap();

        assert_eq!((&after.note_id, after.ordinal), (&id, before.ordinal));
        assert!(text(&after.prompt).starts_with("Lyon is in"));
        assert!(after.revealed);
    }

    #[test]
    fn an_edit_that_stops_the_card_generating_moves_on_with_no_review_row_and_no_count() {
        let (handle, id, _d, _s) = cloze_then_basic();
        let sitting = handle.clone().start_sitting(5, 0, NOW).unwrap();
        sitting.card(NOW, 10).unwrap().unwrap();
        sitting.reveal().unwrap();
        let rows = reviewed_rows(&handle);

        let editing = handle.clone().edit_note(id.clone()).unwrap();
        editing
            .commit_field("Text".into(), "no blank any more".into())
            .unwrap();
        let next = sitting.refresh(NOW, 20).unwrap().unwrap();

        assert_ne!(next.note_id, id);
        assert!(!next.revealed);
        assert_eq!(sitting.graded(), 0);
        assert_eq!(reviewed_rows(&handle), rows);
        assert!(!sitting.ended(NOW).unwrap());
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

    /// Call the handle as Kotlin's tick does until the run is over.
    fn finish_the_run(run: &OptimiseHandle) -> OptimiseFinish {
        loop {
            match run.finish(NOW).unwrap() {
                OptimiseFinish::StillRunning => std::thread::yield_now(),
                over => return over,
            }
        }
    }

    #[test]
    fn the_nudge_reads_standard_with_the_total_until_a_run_writes_and_fitted_after() {
        let (handle, _d, _s) = benched("history");
        let OptimisationNudge::Standard { reviews_total } = handle.optimisation_nudge().unwrap()
        else {
            panic!("no run has written a vector");
        };
        assert_eq!(reviews_total, 225, "25 cards at nine reviews each");

        let run = handle.clone().start_optimise().unwrap();
        assert_eq!(finish_the_run(&run), OptimiseFinish::Written);
        assert_eq!(
            handle.optimisation_nudge().unwrap(),
            OptimisationNudge::Fitted {
                fitted_over: 225,
                reviews_since: 0
            }
        );
    }

    #[test]
    fn a_run_reports_its_phase_and_a_second_run_over_the_same_history_writes_nothing() {
        let (handle, _d, _s) = benched("history");
        let run = handle.clone().start_optimise().unwrap();
        match run.phase() {
            OptimisePhase::Preparing => {}
            OptimisePhase::Training { current, total } => assert!(current <= total),
        }
        assert_eq!(finish_the_run(&run), OptimiseFinish::Written);
        // Finished: the handle keeps saying so.
        assert_eq!(run.finish(NOW).unwrap(), OptimiseFinish::NothingWritten);

        let again = handle.clone().start_optimise().unwrap();
        assert_eq!(finish_the_run(&again), OptimiseFinish::NothingWritten);
    }

    #[test]
    fn a_cancelled_run_writes_nothing_and_leaves_the_nudge_standard() {
        let (handle, _d, _s) = benched("history");
        let run = handle.clone().start_optimise().unwrap();
        run.cancel();
        assert_eq!(finish_the_run(&run), OptimiseFinish::NothingWritten);
        assert!(matches!(
            handle.optimisation_nudge().unwrap(),
            OptimisationNudge::Standard { .. }
        ));
    }

    // ---- The file surface (ADR-0022, ADR-0052) ---------------------------------------------------

    /// The bench's files in the order `FileSet::Imports` builds them: the update deck, the
    /// three-deck file, a collection archive, and a file this application wrote and cannot parse.
    fn bench() -> Vec<BenchFileFact> {
        let files = bench_files(std::env::temp_dir().display().to_string());
        assert_eq!(files.len(), 4);
        files
    }

    fn open_bench_file(
        handle: &Arc<CollectionHandle>,
        file: &BenchFileFact,
    ) -> Arc<InboundFileHandle> {
        handle.clone().open_file(
            FileArrival::Picked,
            Some(file.name.clone()),
            file.bytes.clone(),
            true,
        )
    }

    fn plan_of(preview: FilePreview) -> ImportPlanFacts {
        match preview {
            FilePreview::Plan { plan, .. } => plan,
            other => panic!("expected a plan, got {other:?}"),
        }
    }

    #[test]
    fn the_bench_serves_its_deck_files_in_every_build() {
        let files = bench();
        assert!(
            files
                .iter()
                .all(|f| !f.name.is_empty() && !f.bytes.is_empty())
        );
        let mut names: Vec<_> = files.iter().map(|f| f.name.clone()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 4);
    }

    #[test]
    fn the_update_deck_previews_its_plan_with_the_decide_gate() {
        let (handle, _d, _s) = benched("decks");
        let file = open_bench_file(&handle, &bench()[0]);
        let plan = plan_of(file.preview().unwrap());
        assert_eq!(plan.gate, ImportGate::Decide);
        assert_eq!(plan.author, "Marjan Rahimi");
        assert_eq!(plan.licence, "CC BY-SA 4.0");
        assert_eq!(
            plan.description,
            "A1 vocabulary for the first ten chapters."
        );
        assert_eq!(plan.decks.len(), 1);
        let deck = &plan.decks[0];
        assert_eq!(deck.path, DeckPath::Update);
        assert!(deck.new_notes > 0);
        assert!(deck.already_yours > 0);
        assert!(deck.deleted > 0);
        assert!(!deck.moving_in.is_empty());
        assert!(!deck.no_change);
    }

    #[test]
    fn the_three_deck_file_carries_its_persian_name_and_no_header() {
        let (handle, _d, _s) = benched("decks");
        let plan = plan_of(open_bench_file(&handle, &bench()[1]).preview().unwrap());
        assert_eq!(plan.decks.len(), 3);
        assert_eq!(plan.decks[0].name, "فارسی");
        assert_eq!(plan.decks[0].path, DeckPath::Update);
        assert_eq!(plan.decks[1].path, DeckPath::Create);
        assert_eq!(
            (
                plan.author.as_str(),
                plan.licence.as_str(),
                plan.description.as_str()
            ),
            ("", "", "")
        );
        assert_eq!(plan.gate, ImportGate::Decide);
    }

    fn restore_of(preview: FilePreview) -> RestorePreviewFacts {
        match preview {
            FilePreview::Restore { restore } => restore,
            other => panic!("expected a restore preview, got {other:?}"),
        }
    }

    #[test]
    fn the_bench_archive_previews_a_restore_over_a_collection_holding_nothing() {
        // `empty` wipes the store and authors nothing — what a fresh install would be without the
        // starter notes `open_collection` seeds.
        let (handle, _d, _s) = benched("empty");
        let restore = restore_of(open_bench_file(&handle, &bench()[2]).preview().unwrap());
        assert_eq!(
            restore,
            RestorePreviewFacts {
                created_ms: Some(cairn_ops::file_bench::ARCHIVE_CREATED_MS),
                notes: 25,
                reviews: 50,
                gate: ImportGate::Decide,
            }
        );
    }

    #[test]
    fn the_bench_archive_over_an_installed_decks_fixture_is_another_collections() {
        let (handle, _d, _s) = benched("decks");
        let preview = open_bench_file(&handle, &bench()[2]).preview().unwrap();
        assert_eq!(
            preview,
            FilePreview::Refused {
                refusal: FileRefusal::AnotherCollection,
                gate: ImportGate::Dismiss
            }
        );
    }

    #[test]
    fn an_archive_is_previewed_again_once_the_collection_stops_being_empty() {
        let (handle, _d, _s) = benched("empty");
        let file = open_bench_file(&handle, &bench()[2]);
        assert_eq!(restore_of(file.preview().unwrap()).gate, ImportGate::Decide);
        handle.install_fixture("decks".to_owned(), NOW).unwrap();
        assert_eq!(
            file.preview().unwrap(),
            FilePreview::Refused {
                refusal: FileRefusal::AnotherCollection,
                gate: ImportGate::Dismiss
            }
        );
    }

    #[test]
    fn applying_an_archive_writes_nothing_whether_or_not_it_is_refused() {
        let (handle, _d, _s) = benched("decks");
        let before = handle.note_list(DeckSelector::All, String::new()).unwrap();
        assert_eq!(
            open_bench_file(&handle, &bench()[2]).apply().unwrap(),
            FileApplied::Refused {
                refusal: FileRefusal::AnotherCollection
            }
        );
        assert_eq!(
            handle.note_list(DeckSelector::All, String::new()).unwrap(),
            before
        );

        // Restoring is the next ticket's: an archive this device would accept is not written yet.
        let (empty, _d2, _s2) = benched("empty");
        assert!(matches!(
            open_bench_file(&empty, &bench()[2]).apply(),
            Err(CairnError::Failed { .. })
        ));
        assert_eq!(empty.note_count().unwrap(), 0);
    }

    #[test]
    fn a_file_that_cannot_be_parsed_is_unreadable() {
        let (handle, _d, _s) = benched("decks");
        let preview = open_bench_file(&handle, &bench()[3]).preview().unwrap();
        assert_eq!(
            preview,
            FilePreview::Refused {
                refusal: FileRefusal::Unreadable,
                gate: ImportGate::Dismiss
            }
        );
    }

    #[test]
    fn an_incomplete_read_is_told_apart_by_its_head() {
        let (handle, _d, _s) = benched("decks");
        let files = bench();
        // The head of an archive still says what it is: the sniff reads only the first member, so
        // a read stopped at the ceiling is too large, not unreadable.
        let head = files[2].bytes[..512.min(files[2].bytes.len())].to_vec();
        let archive = handle
            .clone()
            .open_file(FileArrival::Picked, None, head, false);
        assert_eq!(
            archive.preview().unwrap(),
            FilePreview::Refused {
                refusal: FileRefusal::TooLargeToRead,
                gate: ImportGate::Dismiss
            }
        );
        assert_eq!(
            archive.apply().unwrap(),
            FileApplied::Refused {
                refusal: FileRefusal::TooLargeToRead
            }
        );
        // The head of a deck is not enough to plan from, so it is unreadable, never a plan.
        let head = files[0].bytes[..512.min(files[0].bytes.len())].to_vec();
        let deck = handle
            .clone()
            .open_file(FileArrival::Picked, None, head, false);
        assert_eq!(
            deck.preview().unwrap(),
            FilePreview::Refused {
                refusal: FileRefusal::Unreadable,
                gate: ImportGate::Dismiss
            }
        );
    }

    #[test]
    fn applying_the_update_lands_on_its_deck_and_a_re_read_changes_nothing() {
        let (handle, _d, _s) = benched("decks");
        let file = open_bench_file(&handle, &bench()[0]);
        let landed = file.apply().unwrap();
        let FileApplied::Landed { filter_to } = landed else {
            panic!("expected the import to land, got {landed:?}");
        };
        assert_eq!(filter_to.as_deref(), Some(cairn_ops::fixtures::DECKS[0].0));
        // The preview is re-derived against the collection as it now stands: the same file changes
        // nothing, so its gate is a lone close.
        let plan = plan_of(file.preview().unwrap());
        assert_eq!(plan.gate, ImportGate::Dismiss);
        assert!(plan.decks[0].no_change);
    }

    #[test]
    fn applying_a_several_deck_file_names_no_deck() {
        let (handle, _d, _s) = benched("decks");
        let file = open_bench_file(&handle, &bench()[1]);
        assert_eq!(
            file.apply().unwrap(),
            FileApplied::Landed { filter_to: None }
        );
    }

    #[test]
    fn applying_a_refused_file_says_why_and_writes_nothing() {
        let (handle, _d, _s) = benched("decks");
        let before = handle.note_list(DeckSelector::All, String::new()).unwrap();
        let file = open_bench_file(&handle, &bench()[3]);
        assert_eq!(
            file.apply().unwrap(),
            FileApplied::Refused {
                refusal: FileRefusal::Unreadable
            }
        );
        assert_eq!(
            handle.note_list(DeckSelector::All, String::new()).unwrap(),
            before
        );
    }

    #[test]
    fn the_preview_is_re_derived_when_the_collection_changes_under_it() {
        let (handle, _d, _s) = benched("decks");
        let file = open_bench_file(&handle, &bench()[0]);
        let before = plan_of(file.preview().unwrap());
        // Another door lands the same file first.
        open_bench_file(&handle, &bench()[0]).apply().unwrap();
        let after = plan_of(file.preview().unwrap());
        assert_ne!(before, after);
        assert_eq!(after.gate, ImportGate::Dismiss);
    }

    #[test]
    fn applying_after_a_newer_revision_lands_underneath_refuses_and_writes_nothing() {
        let (handle, _d, _s) = benched("decks");
        let file = open_bench_file(&handle, &bench()[0]);
        assert_eq!(plan_of(file.preview().unwrap()).gate, ImportGate::Decide);
        let before = handle.note_list(DeckSelector::All, String::new()).unwrap();
        // Under the preview, a newer revision of the deck arrives — as a sync from a device that
        // imported a later copy would leave it. The attribute is the one an import writes back
        // (ADR-0008 §9).
        let (deck, name) = cairn_ops::fixtures::DECKS[0];
        let id = DeckId::parse_canonical(deck).unwrap();
        handle
            .collection()
            .mutable_set_if_changed("deck", &id.0, cairn_export::DECK_REVISION_ATTR, Some("9"))
            .unwrap();
        // Import re-derives, so the file it was shown is now an older copy and nothing is written.
        assert_eq!(
            file.apply().unwrap(),
            FileApplied::Refused {
                refusal: FileRefusal::OlderCopy {
                    deck: name.to_owned()
                }
            }
        );
        assert_eq!(
            handle.note_list(DeckSelector::All, String::new()).unwrap(),
            before
        );
    }

    #[test]
    fn a_backup_is_built_named_and_counted_and_only_recording_moves_the_nudge() {
        let (handle, _d, _s) = benched("history");
        let notes = handle.note_count().unwrap();
        let before = handle.backup_nudge().unwrap();
        assert_eq!(before.last_backup_ms, None);
        assert!(before.reviews > 0);
        assert_eq!(before.reviews_since, before.reviews);
        assert_eq!(before.notes, notes);

        // 2025-10-09T08:53:20Z: an hour east of UTC, the same day.
        let backup = handle.backup(NOW, 60).unwrap();
        assert_eq!(backup.name, "Cairn backup 2025-10-09.ccoll");
        assert_eq!(backup.name, backup_filename(NOW, 60));
        assert_eq!((backup.notes, backup.reviews), (notes, before.reviews));
        // Building wrote nothing down.
        assert_eq!(handle.backup_nudge().unwrap(), before);

        // Opened again over the collection it came from, the archive previews its restore.
        let opened =
            handle
                .clone()
                .open_file(FileArrival::Opened, Some(backup.name), backup.bytes, true);
        assert_eq!(
            opened.preview().unwrap(),
            FilePreview::Restore {
                restore: RestorePreviewFacts {
                    created_ms: Some(NOW),
                    notes,
                    reviews: before.reviews,
                    gate: ImportGate::Decide,
                }
            }
        );

        handle.record_backup(NOW).unwrap();
        let after = handle.backup_nudge().unwrap();
        assert_eq!(after.last_backup_ms, Some(NOW));
        assert_eq!(after.reviews_since, 0);
        assert_eq!((after.reviews, after.notes), (before.reviews, before.notes));
    }

    #[test]
    fn an_arrival_makes_no_difference_to_the_plan() {
        let (handle, _d, _s) = benched("decks");
        let file = &bench()[0];
        let picked = plan_of(open_bench_file(&handle, file).preview().unwrap());
        for arrival in [FileArrival::Opened, FileArrival::Shared] {
            let other = handle
                .clone()
                .open_file(arrival, None, file.bytes.clone(), true);
            assert_eq!(plan_of(other.preview().unwrap()), picked);
        }
    }
}
