//! The egui application: every screen, and both entry points.
//!
//! **This crate deliberately has no `src/main.rs`.** `cargo-apk` panics after signing
//! (`Bin is not compatible with Cdylib`) when one crate has both a cdylib and a bin — the APK comes
//! out correct but the exit code does not, and CI breaks. The desktop binary is `leitner-desktop`,
//! which is a shim with no logic (ADR-0003 §5, ADR-0009 §3).
//!
//! **The Android entry point is `platform::android`**, not this file. The activity handle the inset
//! seam needs originates in `android_main` and nothing else can hand it over — `ndk_context` holds
//! the `Application` — so keeping the two together is what leaves `platform` one function wide and
//! this file free of `#[cfg(target_os)]` (ADR-0025 §2, client-stack rule 3).
//!
//! See `CONTEXT.md` beside this file, [ADR-0003](../../../docs/adr/0003-client-stack.md) and
//! [ADR-0006](../../../docs/adr/0006-the-review-session-experience.md).

pub mod bidi;
pub mod cards;
pub mod deck;
pub mod editor;
pub mod fonts;
pub mod keyboard;
pub mod notes;
pub mod optimise;
pub mod platform;
pub mod session;
pub mod sync;

use std::collections::HashSet;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use leitner_core::content::{CardRef, DeckId, NoteId};
use leitner_core::log::{DEFAULT_NEW_CARD_RATE, DayScale, day_number};
use leitner_core::replay::{Leech, Replayed, leeches, replay};
use leitner_core::scheduling::Grade;
use leitner_store::Collection;

use notes::Filter;
use session::{Offered, ReviewState};

/// Re-exported so `leitner-desktop` needs no `eframe` dependency of its own — it cannot then
/// resolve a different feature set from the one this crate was built with, and it has no route to
/// grow real code unnoticed.
pub use eframe;

/// A sitting of review, held **only in memory** (ADR-0006 §6, issue #94): nothing about it is stored,
/// so a force-quit loses nothing — relaunch re-derives the queue from the log and every already-graded
/// card is simply no longer due. The queue is **re-derived every frame** rather than snapshotted, so
/// a card failed mid-sitting returns the same session (ADR-0011 §9) and an edit that makes a card
/// dormant drops it without ceremony.
///
/// The size the user picked counts **gradings, not distinct cards** (ADR-0011 §9): every graded event
/// advances `graded`, re-shows included, so the progress bar always moves when the user acts. A
/// sitting ends when `graded` reaches `chosen` or the queue empties, whichever comes first.
struct Sitting {
    /// The count the user chose — the number of **gradings** this sitting runs to.
    chosen: usize,
    /// Gradings performed so far. This is the progress numerator, and it counts every grade press,
    /// including a lapsed card's same-session re-show (ADR-0011 §9).
    graded: usize,
    /// The card currently on screen, so a reveal survives a frame and resets when the card changes.
    shown: Option<CardRef>,
    revealed: bool,
    /// When the sitting began — the quiet 10-minute timer runs from here (issue #94).
    started: Instant,
    /// When the current card came on screen, so the row can record how long the answer took
    /// (ADR-0004 §5).
    card_shown: Instant,
    /// Set once the user answers the 10-minute checkpoint's "keep going", so it does not nag again.
    checkpoint_dismissed: bool,
    /// The set of cards that were already leeches when this sitting began — the `before` half of the
    /// end-of-session pointer (ADR-0010 §6). Held **in memory only**, like everything else about a
    /// sitting: the pointer covers cards that crossed *this* session, which is exactly `leeches now`
    /// minus this snapshot, and it needs no stored dismissal or last-seen state.
    leeches_at_start: HashSet<CardRef>,
}

impl Sitting {
    fn new(chosen: usize, leeches_at_start: HashSet<CardRef>) -> Self {
        let now = Instant::now();
        Sitting {
            chosen,
            graded: 0,
            shown: None,
            revealed: false,
            started: now,
            card_shown: now,
            checkpoint_dismissed: false,
            leeches_at_start,
        }
    }

    /// The checkpoint is due once ten minutes have passed and the user has not already waved it away.
    /// It is a **courtesy**, never an enforcement — reaching the chosen count is what ends a session
    /// (issue #94).
    fn checkpoint_due(&self) -> bool {
        !self.checkpoint_dismissed && self.started.elapsed() >= Duration::from_secs(600)
    }
}

/// The three top-level destinations (ADR-0021 §1, `ui` `CONTEXT.md`): the smallest set that makes
/// every already-specified screen reachable. The leech screen hangs off review's end-of-session
/// pointer and enrolment sits inside settings, so those are not destinations of their own. How the
/// three are *rendered* — a tab bar, a drawer, something else — is the visual design pass's; what is
/// fixed is that all three are reachable from a persistent affordance, which the nav row is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Destination {
    Review,
    Notes,
    Settings,
}

/// The editor's live state (ADR-0012 §2, ADR-0021 §7): one editor, four entrances. `note` is `None`
/// for an uncommitted new draft — under autosave the note is not born until its first non-empty field
/// (ADR-0021 §7), so before that there is nothing for a kill to lose. `fields` are the per-field text
/// buffers in kind-definition order.
struct Editing {
    note: Option<NoteId>,
    kind: String,
    fields: Vec<(String, String)>,
    /// The deck this note is filed under, chosen in the editor's deck dropdown beside the kind one
    /// (ADR-0021 §9). `None` is unfiled — a legal, still-reviewable state (ADR-0005 §8). On a draft
    /// the choice is held here until the note is born on its first non-empty field, then written once.
    deck: Option<DeckId>,
    /// The new-deck name buffer for *create a new deck*, available from the deck dropdown (ADR-0021
    /// §9): the moment you need a deck that does not exist is while filing the note that wants it.
    new_deck: String,
    /// Which of the two panes is showing when the screen is too narrow to hold both — the phone's
    /// `Write | Cards` toggle (ADR-0012 §1, ADR-0025 §5). `false` is *Write*, the form. On a wide
    /// screen both panes show and this is ignored.
    show_cards: bool,
}

impl Editing {
    /// A fresh draft of `kind`, carrying that kind's fields as empty buffers. Used by **create** and
    /// by the *New note* chord, which carries the current kind forward (ADR-0021 §8).
    fn new_draft(kind: &str) -> Self {
        Editing {
            note: None,
            kind: kind.to_owned(),
            fields: editor::field_names(kind)
                .into_iter()
                .map(|name| (name.to_owned(), String::new()))
                .collect(),
            deck: None,
            new_deck: String::new(),
            show_cards: false,
        }
    }

    /// Load a stored note into the editor — the **edit** entrance from the note list, the leech
    /// screen, or the review screen (ADR-0021 §5). Reads the note's kind and each declared field's
    /// current value; a value replay never denies, since the editor reads the same mutable surface.
    fn for_note(coll: &Collection, note: NoteId) -> Self {
        let kind = coll
            .mutable_get("note", &note.0, "kind")
            .ok()
            .flatten()
            .unwrap_or_default();
        let fields = editor::field_names(&kind)
            .into_iter()
            .map(|name| {
                let value = coll
                    .mutable_get("note", &note.0, name)
                    .ok()
                    .flatten()
                    .unwrap_or_default();
                (name.to_owned(), value)
            })
            .collect();
        let deck = coll
            .mutable_get("note", &note.0, "deck")
            .ok()
            .flatten()
            .and_then(|d| DeckId::parse_canonical(&d));
        Editing {
            note: Some(note),
            kind,
            fields,
            deck,
            new_deck: String::new(),
            show_cards: false,
        }
    }

    /// Rebuild the field buffers for a newly chosen kind, carrying forward any value whose field name
    /// the new kind shares (ADR-0012 §2). A kind change is an ordinary edit — the cards it makes
    /// dormant are §5's ambient warning's concern, not a special mechanism (ADR-0017 §5).
    fn switch_kind(&mut self, kind: &str) {
        let carried = std::mem::take(&mut self.fields);
        self.kind = kind.to_owned();
        self.fields = editor::field_names(kind)
            .into_iter()
            .map(|name| {
                let prior = carried
                    .iter()
                    .find(|(n, _)| n == name)
                    .map_or_else(String::new, |(_, v)| v.clone());
                (name.to_owned(), prior)
            })
            .collect();
    }
}

/// The application: an open collection (or the message saying why it would not open), the current
/// destination, the transient review sitting, and the editor's live state when one is open.
pub struct LeitnerApp {
    store: Result<Collection, String>,
    dest: Destination,
    sitting: Option<Sitting>,
    /// The open editor, or `None` when the note list is showing its list rather than the form.
    editing: Option<Editing>,
    /// The note list's text-search buffer, held across frames (ADR-0021 §2).
    search: String,
    /// True while the enrolment screen (the *surface* of "Set up sync") is showing, reached from the
    /// Settings destination (ADR-0015 §7). The device flow that would follow it carries the network
    /// and is deferred (see `sync`), so this gates only what the screen *says*, not a grant.
    setting_up_sync: bool,
    /// The note list's deck filter, held across frames — one of ADR-0005 §6's three composable
    /// filters. `None` narrows by no deck (every note, filed or not).
    deck_filter: Option<DeckId>,
    /// The note list's *new deck* name buffer: decks are created where they are filtered (ADR-0021
    /// §9), so the create control sits beside the deck filter.
    new_deck: String,
    /// The settings screen's new-card-rate edit buffer, held across frames. `None` until the settings
    /// screen first reads the stored rate into it; committed back to the mutable surface on change
    /// (ADR-0011 §3, §5). Kept as text so a mid-edit empty field does not read as zero.
    new_card_rate: Option<String>,
    /// True while the **leech screen** is showing (ADR-0010 §4, §6). It hangs off the Review
    /// destination rather than being one of its own — reached from the end-of-session pointer and from
    /// a durable entry on the picker — so it is a sub-state of Review, not a fourth `Destination`.
    showing_leeches: bool,
    /// The count of leeches that **crossed the floor during the sitting just finished** — the
    /// end-of-session pointer (ADR-0010 §6). `Some(n)` shows the pointer once; dismissing it or tapping
    /// through clears it. Never a nag: a card ignored here is not raised again, only kept on the leech
    /// screen.
    session_pointer: Option<usize>,
    /// The optimisation run in progress, or `None` when none is (ADR-0014 §3). A worker thread the
    /// frame loop polls: while it is `Some`, settings shows the two-phase progress and a Cancel; on
    /// completion the fitted vector is written (or skipped, if unchanged) and this returns to `None`.
    /// Nothing is persisted until it completes, so a frozen or killed run leaves it holding nothing to
    /// resume (client-stack rule 10).
    optimise_job: Option<optimise::OptimiseJob>,
    /// True from the moment a run completes until another is started — the settings screen's cue to
    /// show the completion message (ADR-0014 §4). Frame-local, never persisted.
    optimise_done: bool,
    /// Cleared until the shipped font set is installed. The install happens on the **first frame**,
    /// not in `CreationContext` — see `fonts` and the note on `new`.
    fonts_installed: bool,
    /// The band the platform's chrome and soft keyboard are sitting on, and the state its guards
    /// need across frames (ADR-0025 §1, §3). Held here because two of the guards are differential:
    /// they read the previous frame's geometry and must act before this frame lays anything out.
    band: keyboard::Band,
    /// **Temporary** — the hand-off specimen's state (see [`handoff_specimen`]).
    handoff: HandOff,
}

/// **Temporary, and not a specified feature.** What the hand-off specimen carries between frames.
#[derive(Default)]
struct HandOff {
    /// The name the platform reported for the last successful [`leitner_export::platform::put`] —
    /// **the written one, never the requested one** (ADR-0022 §10). This is what `hand_off` is then
    /// asked for, so the specimen exercises the read-back rather than asserting it.
    written: Option<String>,
    /// The last thing either button had to say, verbatim: a read-back name or a refusal. Held rather
    /// than logged because a handset run has no console the person holding it can read.
    said: String,
}

impl LeitnerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Fonts are installed on the **first frame**, never here (see `ui` and the `fonts` module).
        // Registering a face during creation was found in #8 to break rendering on some backends;
        // deferring it one frame fixes it, and a newly-named family (bold) is not referenceable on
        // the frame it is registered anyway (ADR-0012 §8).
        let store = Self::open_store();
        Self {
            store,
            dest: Destination::Review,
            sitting: None,
            editing: None,
            search: String::new(),
            setting_up_sync: false,
            deck_filter: None,
            new_deck: String::new(),
            new_card_rate: None,
            showing_leeches: false,
            session_pointer: None,
            optimise_job: None,
            optimise_done: false,
            fonts_installed: false,
            band: keyboard::Band::default(),
            handoff: HandOff::default(),
        }
    }

    /// Open the collection under the platform's two directories (ADR-0007 §6) and, on a first launch,
    /// seed a few `basic` notes so the walking skeleton has cards to review — issue #94's opening line.
    ///
    /// **More than one note, deliberately.** A single card cannot exercise the session at all: the count
    /// picker collapses to one `All 1` button with none of ADR-0006 §1's choices reachable, and a sitting
    /// ends on the first grading — so §2's *position is derived, never stored* cannot be demonstrated,
    /// because there is no mid-session to be force-quit out of (issue #96). One note past
    /// [`DEFAULT_NEW_CARD_RATE`] keeps the rate itself visible as the binding limit rather than the
    /// seed's size.
    fn open_store() -> Result<Collection, String> {
        const SEED: [(&str, &str); DEFAULT_NEW_CARD_RATE as usize + 1] = [
            ("chien", "dog"),
            ("chat", "cat"),
            ("livre", "book"),
            ("eau", "water"),
            ("pain", "bread"),
            ("maison", "house"),
        ];

        let data = leitner_store::platform::data_dir().map_err(|e| e.to_string())?;
        let state = leitner_store::platform::state_dir().map_err(|e| e.to_string())?;
        let mut coll = Collection::open(&data, &state).map_err(|e| e.to_string())?;
        if coll.is_empty().map_err(|e| e.to_string())? {
            for (front, back) in SEED {
                coll.create_note("basic", &[("Front", front), ("Back", back)])
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok(coll)
    }

    /// **Temporary** — the other half of [`reset_control`]. Return this device to a first launch: the
    /// store removes both databases and the writer marker, then reopening reseeds, because the
    /// collection comes back empty. Every in-memory screen state goes with them, since all of it
    /// describes rows that no longer exist.
    ///
    /// **This takes the fitted scheduler parameters and the new-card rate with it, and that is a
    /// first-launch reset rather than a side effect.** Neither is a setting kept off to one side: an
    /// optimisation run records its vector as a `config-set` **log row** (ADR-0014 §5, ADR-0004 §6), and
    /// the rate is a value on ADR-0004 §7's mutable surface — both live inside `collection.db`, so
    /// deleting the log is what deletes them. Afterwards the scheduler is back on the published defaults
    /// and the settings nudge reads *"Using the standard parameters"* again, which is true. Anyone
    /// wanting a reset that spares them is asking to keep a projection of a log that no longer exists.
    ///
    /// **The open connection is dropped first, and that ordering is the whole of the correctness here.**
    /// SQLite holds the database plus its `-wal` and `-shm` siblings open; unlinking them underneath a
    /// live connection leaves a checkpoint writing into inodes nothing can reach, and the reopened
    /// collection then races the old one's flush. Assigning `store` is what closes it.
    ///
    /// The marker goes too, so the device **mints a fresh writer id** — correct here and the opposite of
    /// what a *restore* may do (store rule 5, [ADR-0016 §2](../../../docs/adr/0016-backup-and-restore.md):
    /// a writer id is never adopted). A reset device is a new writer, not a returning one.
    fn reset_collection(&mut self) {
        self.store = Err("Resetting…".to_owned());

        let dirs = leitner_store::platform::data_dir()
            .and_then(|data| leitner_store::platform::state_dir().map(|state| (data, state)));
        if let Ok((data, state)) = dirs {
            leitner_store::remove_files(&data, &state);
        }

        self.store = Self::open_store();
        self.sitting = None;
        self.editing = None;
        self.showing_leeches = false;
        self.session_pointer = None;
        self.new_card_rate = None;
        self.deck_filter = None;
        self.search.clear();
        self.dest = Destination::Review;
    }
}

impl eframe::App for LeitnerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // The shipped font set is installed here, on the first frame, and this frame draws nothing:
        // `set_fonts` applies at the start of the *next* pass, so the newly-named bold family is not
        // referenceable yet and any text drawn now would use the stock faces (ADR-0012 §8, ADR-0003
        // §7, client-stack rule 7). One repaint is requested so the deferral costs no input event.
        if !self.fonts_installed {
            fonts::install(ui.ctx());
            self.fonts_installed = true;
            ui.ctx().request_repaint();
            return;
        }

        let now_ms = now_ms();
        // "Due today" is the **device's local** day (replay `CONTEXT.md`), which the walking skeleton
        // reads at the default 4am scale; a real device timezone is a later ticket.
        let today = day_number(now_ms, DayScale::default());

        let coll = match self.store.as_mut() {
            Err(message) => {
                heading(ui, "Leitner");
                body(ui, message);
                return;
            }
            Ok(coll) => coll,
        };

        // ---- The band the platform's chrome and keyboard are sitting on (ADR-0025 §1) -----------
        //
        // Nothing below this application reports the soft keyboard, so it asks — and un-asked the
        // failure is *unreachability*, not occlusion: egui is handed a viewport taller than the
        // visible one, the content fits inside it, and there is no scroll range over the covered
        // band at all. Reserving it is what gives the scroll area a real range, and reserving the
        // top is what stops the first line of text being drawn under the status bar.
        let bands = self.band.read(ui.ctx());
        if keyboard::Bands::is_worth_reserving(bands.top) {
            egui::Panel::top("inset-top")
                .exact_size(bands.top)
                .frame(egui::Frame::NONE)
                .show(ui, |_| {});
        }

        // **Before the band is reserved, not after.** The focused field has to be inside the
        // viewport on the *same* frame it shrinks, or its IME output lapses for one frame and the
        // keyboard is gone before the next one — see `keyboard` for the loop this breaks.
        let viewport_bottom = ui.available_rect_before_wrap().bottom() - bands.bottom;
        self.band
            .keep_focus_visible(ui.ctx(), bands, viewport_bottom);
        self.band.settle_focus_scrolled_away(ui.ctx());

        if keyboard::Bands::is_worth_reserving(bands.bottom) {
            egui::Panel::bottom("inset-bottom")
                .exact_size(bands.bottom)
                .frame(egui::Frame::NONE)
                .show(ui, |_| {});
        }

        // Everything the destination draws scrolls, which is what makes the covered band reachable
        // rather than merely clipped. The nav row scrolls with it deliberately: pinning it would
        // spend the form pane's first screen, which is the resource ADR-0025's consequences name —
        // under a keyboard that screen is all the user has, and the destructive-edit warning was
        // moved into it because nowhere else works.
        //
        // The area is taken *before* the closure so it carries guard 1's forced offset without
        // holding a borrow of `band` across the frame the destinations are drawn in.
        let area = self.band.scroll_area();
        let mut reset_requested = false;
        let out = area.show(ui, |ui| {
            // The persistent affordance that makes all three destinations reachable (ADR-0021 §1):
            // a destination reachable only by completing a session is not reachable, so the nav row
            // is drawn every frame, above whatever the current destination shows.
            nav_bar(ui, &mut self.dest);
            ui.separator();
            ui.add_space(4.0);

            match self.dest {
                Destination::Review => {
                    // Opening the editor from the review screen counts as a reveal (ADR-0021 §6):
                    // the request carries the note, and `review` has already flipped the card face
                    // over. The edit entrance is shared by the leech screen, which also hangs off
                    // Review (ADR-0010 §7).
                    if let Some(note) = review(
                        ui,
                        coll,
                        &mut self.sitting,
                        &mut self.showing_leeches,
                        &mut self.session_pointer,
                        now_ms,
                        today,
                    ) {
                        self.editing = Some(Editing::for_note(coll, note));
                        self.dest = Destination::Notes;
                    }
                }
                Destination::Notes => {
                    notes_screen(
                        ui,
                        coll,
                        &mut self.editing,
                        &mut self.search,
                        &mut self.deck_filter,
                        &mut self.new_deck,
                    );
                }
                Destination::Settings => {
                    // The wipe cannot happen here: `coll` is borrowed for the whole frame, and the
                    // reset closes that connection. So the control only *asks*, and the app acts once
                    // the borrow has ended, below.
                    reset_requested = settings_screen(
                        ui,
                        coll,
                        &mut self.setting_up_sync,
                        &mut self.new_card_rate,
                        &mut self.optimise_job,
                        &mut self.optimise_done,
                        &mut self.handoff,
                        now_ms,
                    );
                }
            }
        });
        self.band.record(&out);

        if reset_requested {
            self.reset_collection();
        }
    }
}

/// The nav row: three buttons, the current one marked. This is the persistent affordance ADR-0021 §1
/// fixes; its *appearance* (a tab bar, a drawer) is the visual design pass's, and a row of buttons is
/// the honest floor.
fn nav_bar(ui: &mut egui::Ui, dest: &mut Destination) {
    ui.horizontal(|ui| {
        for (target, label) in [
            (Destination::Review, "Review"),
            (Destination::Notes, "Notes"),
            (Destination::Settings, "Settings"),
        ] {
            if ui
                .selectable_label(*dest == target, text(ui, label))
                .clicked()
            {
                *dest = target;
            }
        }
    });
}

/// Draw the whole review destination for this frame: the count picker when no sitting is running,
/// otherwise the current card. Returns the note the user asked to **edit**, if any — the review
/// screen is one of the editor's four entrances (ADR-0021 §5), and opening it counts as a reveal
/// (ADR-0021 §6), which is why the card is flipped here before the request leaves.
fn review(
    ui: &mut egui::Ui,
    coll: &mut Collection,
    sitting: &mut Option<Sitting>,
    showing_leeches: &mut bool,
    session_pointer: &mut Option<usize>,
    now_ms: i64,
    today: i64,
) -> Option<NoteId> {
    // Everything on screen is derived from the log this frame — there is no cached session state to
    // fall out of step with it. The new-card rate and the notes' authored positions ride the mutable
    // surface (ADR-0011 §5, §7), read fresh alongside the log; suspension is the per-card flag beside
    // them (ADR-0010 §8), so the queue now refuses a suspended card wherever it would appear.
    let current = deck::current_cards(coll).unwrap_or_default();
    let positions = deck::note_positions(coll).unwrap_or_default();
    let rate = coll.new_card_rate().unwrap_or(DEFAULT_NEW_CARD_RATE) as usize;
    let suspended = coll.suspended().unwrap_or_default();
    let lines = coll.log_lines().unwrap_or_default();
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    let replayed = replay(&current, &refs);
    let queue = session::compose(&current, &positions, &replayed, today, rate, &suspended);
    // The leech list is a query over replayed history (ADR-0010 §1, §4), derived fresh every frame —
    // never stored, always current, self-clearing. Suspension does not touch detection: a suspended
    // card is still a leech if its record says so, and has its permanent home below.
    let ranked = leeches(&replayed, today);
    let total = current.len();

    // The leech screen and the end-of-session pointer both hang off Review (ADR-0010 §6), and only
    // when no sitting is running: mid-sitting the card is the sole speaker (ADR-0006). The picker sits
    // last, so a durable entry to the leech screen can precede it.
    if sitting.is_none() {
        if *showing_leeches {
            heading(ui, "Leeches");
            ui.add_space(8.0);
            let edit = leech_screen(ui, coll, &ranked, &suspended, &replayed);
            ui.add_space(8.0);
            if full_width_button(ui, "Back to review").clicked() {
                *showing_leeches = false;
            }
            return edit;
        }

        heading(ui, "Review");
        ui.add_space(8.0);

        // The end-of-session pointer: a plain statement that N cards are costing a lot, with a way
        // through to the list where the decision is made — never a decision point itself (ADR-0010
        // §6). Dismissing it or tapping through clears it; it is shown once and never nags.
        if let Some(count) = *session_pointer {
            body(ui, &pointer_wording(count));
            ui.add_space(8.0);
            if full_width_button(ui, "Show me").clicked() {
                *showing_leeches = true;
                *session_pointer = None;
            }
            if full_width_button(ui, "Not now").clicked() {
                *session_pointer = None;
            }
            return None;
        }

        // Whether the collection has *any* history — the one fact the queue cannot carry, and what
        // separates a first look from a finished day (`ReviewState::of`). A card appears in
        // `replayed.cards` exactly when it has been reviewed.
        let reviewed_ever = !replayed.cards.is_empty();
        if let Some(count) = picker(ui, &queue, total, reviewed_ever) {
            // Snapshot the cards that are already leeches, so the pointer at this sitting's end covers
            // only what crosses during it (ADR-0010 §6).
            let before = ranked.iter().map(|l| l.card).collect();
            *sitting = Some(Sitting::new(count, before));
        }
        // The durable way back to the leech screen (ADR-0010 §6): the notice is the discovery path,
        // this is the place to return to. Offered whenever any card is a leech or is suspended (whose
        // permanent home this is, ADR-0010 §8), below the picker so it never competes with it.
        if !ranked.is_empty() || !suspended.is_empty() {
            ui.add_space(12.0);
            if full_width_button(ui, &leech_entry_wording(ranked.len(), suspended.len())).clicked()
            {
                *showing_leeches = true;
            }
        }
        return None;
    }

    heading(ui, "Review");
    ui.add_space(8.0);

    // A running sitting: keep the frame ticking so the 10-minute checkpoint can surface without an
    // input event (immediate mode has nowhere to wait — client-stack rule 4).
    ui.ctx().request_repaint_after(Duration::from_secs(1));

    // The next card is the head of the **live** queue that still renders — re-derived this frame, so
    // a card failed a moment ago is back in the running (ADR-0011 §9) and a card just made dormant is
    // gone. `find` stops at the first that renders, which is virtually always the head; the scan only
    // matters for the mid-edit race where the head went dormant this very frame.
    let next = queue.sitting(usize::MAX).into_iter().find_map(|offered| {
        deck::render(coll, offered.card)
            .ok()
            .flatten()
            .map(|rendered| (offered, rendered))
    });

    let mut end_sitting = false;
    let mut edit_request: Option<NoteId> = None;
    {
        let s = sitting.as_mut().expect("just checked it is Some");

        if s.graded >= s.chosen {
            // Reaching the chosen count of **gradings** ends the sitting (ADR-0011 §9, issue #94).
            end_sitting = true;
        } else if s.checkpoint_due() {
            body(ui, "You've been reviewing for 10 minutes.");
            ui.add_space(8.0);
            if full_width_button(ui, "Finish here").clicked() {
                end_sitting = true;
            }
            if full_width_button(ui, "Keep going").clicked() {
                s.checkpoint_dismissed = true;
            }
        } else if let Some((offered, rendered)) = next {
            // A new card on screen resets the reveal and the answer-timer; the same card across
            // frames keeps them, so a reveal survives a repaint.
            if s.shown != Some(offered.card) {
                s.shown = Some(offered.card);
                s.revealed = false;
                s.card_shown = Instant::now();
            }

            // Progress counts gradings against the chosen count (ADR-0011 §9), so the bar moves on
            // every grade press — a lapse re-show included — never freezing when the user struggles.
            body(ui, &format!("{} of {}", s.graded, s.chosen));
            ui.add_space(8.0);

            // Reveal is tap-the-card: the prompt is one wide button, and clicking it shows the back.
            // Identical by touch and by mouse — egui does not distinguish them.
            if card_face(ui, &rendered.prompt).clicked() {
                s.revealed = true;
            }

            // Edit this note, at any point in the card's life (ADR-0021 §6): the honest diagnosis of
            // most leeches is a defective card, and the moment to fix it is when it is in front of
            // you, not twenty cards later. Opening the editor **counts as a reveal** — the editor
            // shows the back, so without flipping the card here ADR-0006 §4's "no grading before the
            // answer is seen" would be quietly false. An edit that makes the card dormant needs no
            // mechanism: the next frame re-derives the queue and simply does not offer it.
            ui.add_space(4.0);
            if full_width_button(ui, "Edit note").clicked() {
                s.revealed = true;
                edit_request = Some(offered.card.note);
            }

            if s.revealed {
                ui.add_space(4.0);
                card_face(ui, &rendered.answer);

                // The box badge appears only after reveal, is non-interactive, and reports
                // durability — never a queue (scheduling `CONTEXT.md`). A card with no review history
                // reads `new`, never `Box 1`, which would state a durability nothing has measured
                // (ADR-0006 §6).
                ui.add_space(4.0);
                badge(ui, &box_badge_wording(!offered.is_new, offered.box_));

                ui.add_space(12.0);
                if let Some(grade) = grade_buttons(ui, &offered, today) {
                    let duration_ms = s.card_shown.elapsed().as_millis() as u64;
                    // A failed append drops this one review rather than wedging the session: the next
                    // frame re-derives the queue from whatever did commit. Surfacing write errors is
                    // a later ticket.
                    let _ = coll.append_review(
                        offered.card,
                        grade,
                        now_ms,
                        DayScale::default(),
                        duration_ms,
                    );
                    // A grading advances the count whatever the grade — the re-derived queue decides
                    // whether the card returns (a lapse) or leaves (a pass), not this counter.
                    s.graded += 1;
                    s.revealed = false;
                    s.shown = None;
                }
            }
        } else {
            // The queue emptied before the chosen count — every due card passed and the day's new
            // cards are used up. Nothing is stored; the sitting simply ends (ADR-0006 §8).
            end_sitting = true;
        }
    }

    if end_sitting {
        // The end-of-session pointer covers only leeches that crossed **this** sitting (ADR-0010 §6):
        // the leeches now, minus those already crossed when it began. The `before` snapshot lived in
        // the sitting, which is in-memory, so this needs no stored dismissal or last-seen marker.
        if let Some(s) = sitting.as_ref() {
            let crossed = session::crossed_this_session(&s.leeches_at_start, &ranked);
            *session_pointer = (!crossed.is_empty()).then_some(crossed.len());
        }
        *sitting = None;
    }
    edit_request
}

/// The end-of-session pointer's wording (ADR-0010 §6): a plain statement of cost and an offer to look,
/// never a decision made in the moment. Kept pure so the sentence is testable without a window.
fn pointer_wording(count: usize) -> String {
    if count == 1 {
        "1 card is costing you a lot. Take a look?".to_owned()
    } else {
        format!("{count} cards are costing you a lot. Take a look?")
    }
}

/// The durable leech-screen entry's label (ADR-0010 §6, §8): names how many cards are leeches and how
/// many are suspended, so the button that leads to their permanent home says what it holds.
fn leech_entry_wording(leeches: usize, suspended: usize) -> String {
    match (leeches, suspended) {
        (l, 0) => format!("Leeches ({l})"),
        (0, s) => format!("Suspended ({s})"),
        (l, s) => format!("Leeches ({l}) · suspended ({s})"),
    }
}

/// The leech screen (ADR-0010 §4, §6, §7, §8): the ranked list of cards costing the user, worst first,
/// each offering **edit** (primary), **suspend** and **delete** — and never a tag, which would publish
/// a private struggle into a deck (ADR-0010 §7); plus the **permanent** section of suspended cards,
/// each with **unsuspend** (ADR-0010 §8). Returns the note to edit, if any — the shared editor
/// entrance (ADR-0021 §5). The exact visual design is the pass's; what is fixed is the ranking, the
/// three actions, the never-a-tag, and the suspended section's permanence.
///
/// A suspended card is not listed among the active leeches even when its history still qualifies — it
/// lives in the suspended section instead, so it is named once, not twice. Actions are collected while
/// drawing and applied after, so the immutable reads that render each card's preview do not fight the
/// mutable writes.
fn leech_screen(
    ui: &mut egui::Ui,
    coll: &mut Collection,
    ranked: &[Leech],
    suspended: &HashSet<CardRef>,
    replayed: &Replayed,
) -> Option<NoteId> {
    // Render each card's preview text up front, while the collection is only read; the owned strings
    // then outlive the immutable borrow so the action writes below are free to take `&mut coll`.
    let active: Vec<(CardRef, String, u32, u32)> = ranked
        .iter()
        .filter(|leech| !suspended.contains(&leech.card))
        .map(|leech| {
            let reviews = replayed
                .cards
                .get(&leech.card)
                .map_or(0, |state| state.review_count);
            (
                leech.card,
                card_preview(coll, leech.card),
                leech.failure_days,
                reviews,
            )
        })
        .collect();
    // The suspended section is ordered by card identity so two devices show it the same way.
    let mut suspended_sorted: Vec<CardRef> = suspended.iter().copied().collect();
    suspended_sorted.sort_by_key(|card| card.encode());
    let suspended_rows: Vec<(CardRef, String)> = suspended_sorted
        .iter()
        .map(|&card| (card, card_preview(coll, card)))
        .collect();

    let mut edit: Option<NoteId> = None;
    let mut suspend: Option<CardRef> = None;
    let mut unsuspend: Option<CardRef> = None;
    let mut delete: Option<NoteId> = None;

    // The floor is what lets the empty state speak plainly (ADR-0010 §4): nothing is hurting.
    if active.is_empty() && suspended_rows.is_empty() {
        body(ui, "Nothing is costing you a lot right now.");
        return None;
    }

    if !active.is_empty() {
        body(ui, "These keep catching you out — worst first.");
        ui.add_space(4.0);
        for (card, preview, days, reviews) in &active {
            // The cost, made concrete (ADR-0010 §6): failure days and how many reviews they took.
            badge(ui, &format!("{days} bad days · {reviews} reviews"));
            ui.horizontal(|ui| {
                if ui.button(text(ui, preview)).clicked() {
                    edit = Some(card.note); // edit is the primary action (ADR-0010 §7)
                }
                if ui.button(text(ui, "Suspend")).clicked() {
                    suspend = Some(*card);
                }
                if ui.button(text(ui, "Delete")).clicked() {
                    delete = Some(card.note);
                }
            });
        }
    }

    if !suspended_rows.is_empty() {
        ui.add_space(12.0);
        // Suspended cards have a permanent home here (ADR-0010 §8) — their own section, always, with
        // unsuspend available. Never a one-way door.
        body(ui, "Suspended — not shown in review.");
        ui.add_space(4.0);
        for (card, preview) in &suspended_rows {
            ui.horizontal(|ui| {
                if ui.button(text(ui, preview)).clicked() {
                    edit = Some(card.note);
                }
                if ui.button(text(ui, "Unsuspend")).clicked() {
                    unsuspend = Some(*card);
                }
            });
        }
    }

    // Apply the one action taken this frame. Each is a mutable-surface write (suspend/unsuspend) or the
    // note delete (ADR-0004 §7); a failed write is dropped and the next frame re-derives, as elsewhere.
    if let Some(card) = suspend {
        let _ = coll.suspend(card);
    }
    if let Some(card) = unsuspend {
        let _ = coll.unsuspend(card);
    }
    if let Some(note) = delete {
        let _ = coll.mutable_set("note", &note.0, "deleted", Some("true"));
    }
    edit
}

/// One card's prompt, for a leech row — the card the user will recognise. A dormant card (its content
/// no longer generated, ADR-0002 §7) renders nothing, so it falls back to a plain marker rather than a
/// blank row: a suspended card whose note was edited away still needs a line to carry its unsuspend.
fn card_preview(coll: &Collection, card: CardRef) -> String {
    deck::render(coll, card)
        .ok()
        .flatten()
        .map(|rendered| rendered.prompt)
        .filter(|prompt| !prompt.trim().is_empty())
        .unwrap_or_else(|| "(card no longer generated)".to_owned())
}

/// The count picker and the explicit worded states (issue #94). Returns the chosen sitting size when
/// the user starts one.
fn picker(
    ui: &mut egui::Ui,
    queue: &session::Queue,
    total: usize,
    reviewed_ever: bool,
) -> Option<usize> {
    let available = queue.available();
    match ReviewState::of(queue, total, reviewed_ever) {
        ReviewState::Empty => {
            body(ui, "No cards yet. Add a note to start reviewing.");
            None
        }
        ReviewState::CaughtUp => {
            body(ui, "All caught up — nothing is due right now.");
            None
        }
        ReviewState::NewDeck { new } => {
            body(
                ui,
                "A fresh deck. These cards are new — start whenever you like.",
            );
            count_buttons(ui, new)
        }
        // Nothing due in a collection that *has* been reviewed: the day's repeats are done and the
        // new-card rate still has room. Never "a fresh deck", which is a false statement about a
        // collection with history behind it (ADR-0006 §8, whose two states predate ADR-0011's rate).
        ReviewState::NewOnly { new } => {
            body(ui, &new_only_wording(new));
            count_buttons(ui, new)
        }
        ReviewState::Due { due, new, backlog } => {
            if backlog {
                // Backlog is framed, never a bare number (issue #94, ADR-0001 §3).
                body(
                    ui,
                    "Plenty due — pick a comfortable size, the rest will keep.",
                );
            } else if new > 0 {
                body(ui, &format!("{due} due, plus {new} new. Pick a size."));
            } else {
                body(ui, &format!("{due} due. Pick a size."));
            }
            count_buttons(ui, available)
        }
    }
}

/// A row of sitting-size choices, each capped by what is actually available so the picker never
/// offers more work than exists.
fn count_buttons(ui: &mut egui::Ui, available: usize) -> Option<usize> {
    let mut chosen = None;
    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        for option in [5usize, 10, 20] {
            if option <= available && ui.button(text(ui, &option.to_string())).clicked() {
                chosen = Some(option);
            }
        }
        // "All" is always meaningful when anything is available.
        if available > 0 && ui.button(text(ui, &format!("All {available}"))).clicked() {
            chosen = Some(available);
        }
    });
    chosen
}

/// The four grade buttons: full-width, stacked, with a visual break between 1 and 2 and an
/// illustrative interval preview on each (issue #94). Returns the grade pressed, if any.
fn grade_buttons(ui: &mut egui::Ui, offered: &Offered, today: i64) -> Option<Grade> {
    let mut pressed = None;
    let mut button = |ui: &mut egui::Ui, grade: Grade, label: &str| {
        let days = session::interval_preview(offered, grade, today);
        if full_width_button(ui, &format!("{label}   ·   {days}d")).clicked() {
            pressed = Some(grade);
        }
    };
    button(ui, Grade::Forgot, "Forgot");
    // The visual break between the failure grade and the passes.
    ui.add_space(12.0);
    button(ui, Grade::Barely, "Barely");
    button(ui, Grade::Good, "Good");
    button(ui, Grade::Easy, "Easy");
    pressed
}

/// The **Notes** destination (ADR-0021 §2): the browse surface and the app's authoring home. Shows
/// the editor when one is open, otherwise the note list — create, the text search, and the rows,
/// each row offering **edit and delete** (never suspend, which is the leech screen's, ADR-0021 §2).
fn notes_screen(
    ui: &mut egui::Ui,
    coll: &mut Collection,
    editing: &mut Option<Editing>,
    search: &mut String,
    deck_filter: &mut Option<DeckId>,
    new_deck: &mut String,
) {
    if let Some(ed) = editing {
        if full_width_button(ui, "Done").clicked() {
            *editing = None;
            return;
        }
        ui.add_space(8.0);
        editor_pane(ui, coll, ed);
        return;
    }

    heading(ui, "Notes");
    ui.add_space(8.0);

    // Create opens a fresh draft; the note is not committed until its first non-empty field
    // (ADR-0021 §7). A new note defaults to `basic` — the shipped kind that is a plain front/back. It
    // opens already filed under the deck the list is filtered to, if any — the deck you are looking at
    // is the likeliest one for the note you are about to write.
    if full_width_button(ui, "Create note").clicked() {
        let mut draft = Editing::new_draft("basic");
        draft.deck = *deck_filter;
        *editing = Some(draft);
        return;
    }

    // The empty state is the empty *collection* (ADR-0015 §7): shown when there is no note at all, not
    // when a filter happens to match nothing.
    if !notes::any_notes(coll).unwrap_or(false) {
        ui.add_space(8.0);
        body(ui, notes::EMPTY_STATE);
        return;
    }

    // The deck filter and the deck authoring surface (ADR-0021 §9): decks are **created where they
    // are filtered**, so the filter dropdown, *new deck*, and the delete of the filtered deck all sit
    // together here. Deletion is ADR-0005 §7's flag, deriving through to the deck's notes.
    ui.add_space(8.0);
    deck_controls(ui, coll, deck_filter, new_deck);

    // Text search — the load-bearing filter (ADR-0021 §2), a plain substring over field values,
    // composing with the deck filter above (deck ∩ text; the tag filter shares the vocabulary and is
    // set on notes but has no dedicated control yet).
    ui.add_space(8.0);
    field_label(ui, "Search");
    text_field(ui, search);
    let filter = Filter {
        deck: deck_filter.map(|d| d.to_canonical()),
        text: (!search.trim().is_empty()).then(|| search.trim().to_owned()),
        ..Filter::default()
    };

    ui.add_space(8.0);
    let rows = notes::list(coll, &filter).unwrap_or_default();
    if rows.is_empty() {
        body(ui, "No notes match.");
        return;
    }
    // The list's own sequence is the rendering of `position` order — there is no sort control and the
    // key is never shown (ADR-0021 §4). No row carries schedule information (ADR-0021 §2).
    let mut open: Option<NoteId> = None;
    let mut delete: Option<NoteId> = None;
    for row in &rows {
        ui.horizontal(|ui| {
            if ui.button(text(ui, row.preview())).clicked() {
                open = Some(row.id);
            }
            if ui.button(text(ui, "Delete")).clicked() {
                delete = Some(row.id);
            }
        });
    }
    if let Some(id) = delete {
        // ADR-0004 §7's delete: a marker on the mutable surface that discards the content. There is no
        // undelete here — recovery is ADR-0016's restore (ADR-0021 §2).
        let _ = coll.mutable_set("note", &id.0, "deleted", Some("true"));
    }
    if let Some(id) = open {
        *editing = Some(Editing::for_note(coll, id));
    }
}

/// The note list's deck surface (ADR-0021 §9): the deck **filter** dropdown, *new deck* creation
/// beside it, and — for the deck currently filtered to — a delete. A deck is `{ id, name }` with a
/// minted id (ADR-0005 §4); the dropdown shows names but the filter is by id, so two decks may share
/// a name without merging. *All decks* and *Unfiled* are filter values, not decks: **no deck is ever
/// auto-created** (ADR-0005 §8), so a collection may legitimately hold none.
fn deck_controls(
    ui: &mut egui::Ui,
    coll: &mut Collection,
    deck_filter: &mut Option<DeckId>,
    new_deck: &mut String,
) {
    let decks = coll.decks().unwrap_or_default();

    field_label(ui, "Deck");
    let selected = deck_filter
        .and_then(|id| decks.iter().find(|(d, _)| *d == id).map(|(_, n)| n.clone()))
        .unwrap_or_else(|| "All decks".to_owned());
    egui::ComboBox::from_id_salt("deck-filter")
        .selected_text(text(ui, &selected))
        .show_ui(ui, |ui| {
            ui.selectable_value(deck_filter, None, text(ui, "All decks"));
            for (id, name) in &decks {
                ui.selectable_value(deck_filter, Some(*id), text(ui, name));
            }
        });

    ui.horizontal(|ui| {
        let created = ui.button(text(ui, "New deck")).clicked();
        text_field(ui, new_deck);
        // Create the deck and immediately filter to it — you made it to use it (ADR-0021 §9).
        if created
            && !new_deck.trim().is_empty()
            && let Ok(id) = coll.create_deck(new_deck.trim())
        {
            *deck_filter = Some(id);
            new_deck.clear();
        }
    });

    // Delete is reachable from the same place (ADR-0021 §9); it flags the filtered deck deleted
    // (ADR-0005 §7), which derives its notes deleted too. The binding warning naming how many notes
    // lose content, and the *move to another deck* alternative (ADR-0005 §7), are the visual pass's.
    if let Some(id) = *deck_filter
        && ui.button(text(ui, "Delete deck")).clicked()
    {
        let _ = coll.mutable_set("deck", &id.0, "deleted", Some("true"));
        *deck_filter = None;
    }
}

/// The editor's deck dropdown (ADR-0021 §9): the note's single deck (ADR-0005 §2), *Unfiled* for
/// none, and *create a new deck* inline. A change to a **stored** note is written at once; on a draft
/// it is held in `ed.deck` and applied when the note is born (see [`editor_pane`]). Creating a deck
/// here files the note under it immediately — the one reason you made it (ADR-0021 §9).
fn editor_deck_dropdown(ui: &mut egui::Ui, coll: &mut Collection, ed: &mut Editing) {
    let decks = coll.decks().unwrap_or_default();
    let current = ed
        .deck
        .and_then(|id| decks.iter().find(|(d, _)| *d == id).map(|(_, n)| n.clone()))
        .unwrap_or_else(|| "Unfiled".to_owned());

    field_label(ui, "Deck");
    let mut chosen = ed.deck;
    egui::ComboBox::from_id_salt("note-deck")
        .selected_text(text(ui, &current))
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut chosen, None, text(ui, "Unfiled"));
            for (id, name) in &decks {
                ui.selectable_value(&mut chosen, Some(*id), text(ui, name));
            }
        });
    if chosen != ed.deck {
        ed.deck = chosen;
        if let Some(id) = ed.note {
            let _ = editor::set_note_deck(coll, id, ed.deck);
        }
    }

    ui.horizontal(|ui| {
        let created = ui.button(text(ui, "New deck")).clicked();
        text_field(ui, &mut ed.new_deck);
        if created
            && !ed.new_deck.trim().is_empty()
            && let Ok(id) = coll.create_deck(ed.new_deck.trim())
        {
            ed.deck = Some(id);
            ed.new_deck.clear();
            if let Some(note) = ed.note {
                let _ = editor::set_note_deck(coll, note, ed.deck);
            }
        }
    });
}

/// The editor's two panes (ADR-0012 §1): the header (kind and deck dropdowns), then the **form body**
/// — the fields, each autosaved per blur (ADR-0021 §7), with the destructive-edit warning above them
/// (ADR-0025 §4) — and the **card body**, *"what will I be asked"* (`cards`). On a narrow screen the
/// two bodies are a `Write | Cards` toggle (ADR-0012 §1); where both fit they show together.
fn editor_pane(ui: &mut egui::Ui, coll: &mut Collection, ed: &mut Editing) {
    heading(
        ui,
        if ed.note.is_some() {
            "Edit note"
        } else {
            "New note"
        },
    );
    ui.add_space(8.0);

    // The standing quiet line stating the Android text-input limitation *in advance* (ADR-0015 §9):
    // the failure is silence — composed non-Latin text never reaches the app — so it can only be
    // told, never detected. Off the one sanctioned `cfg(target_os)` capability constant (ADR-0015
    // §15), so on desktop this compiles to nothing rather than a statement about a limitation the
    // reader does not have.
    if sync::LATIN_INPUT_ONLY {
        field_label(ui, sync::DESKTOP_AUTHORING_LINE);
        ui.add_space(8.0);
    }

    // The kind dropdown: the shipped kinds plus this note's own current kind when acquired, and never
    // another acquired one (ADR-0012 §2, ADR-0017 §6).
    let options = editor::kind_options(&ed.kind);
    let mut chosen = ed.kind.clone();
    field_label(ui, "Kind");
    egui::ComboBox::from_id_salt("note-kind")
        .selected_text(text(ui, &ed.kind))
        .show_ui(ui, |ui| {
            for option in &options {
                ui.selectable_value(&mut chosen, option.clone(), text(ui, option));
            }
        });
    if chosen != ed.kind {
        ed.switch_kind(&chosen);
        // On a stored note a kind change is an ordinary edit (ADR-0017 §5); on a draft it only
        // re-shapes the buffers, and the note is still born on its first non-empty field.
        if let Some(id) = ed.note {
            let _ = coll.mutable_set("note", &id.0, "kind", Some(&chosen));
        }
    }

    ui.add_space(8.0);

    // The deck dropdown, beside the kind one (ADR-0021 §9): the note's one deck (ADR-0005 §2), with
    // *create a new deck* right here — the moment you need a deck that does not exist is while filing
    // the note that wants it. On a draft the choice is held until the note is born, then written once.
    editor_deck_dropdown(ui, coll, ed);

    ui.add_space(8.0);

    // The card pane and the ambient destructive-edit warning are recomputed from current content
    // every frame (ADR-0012 §5): dormancy holds no before-state, so there is no "just became dormant"
    // and nothing to auto-scroll to (ADR-0018 §4). A draft not yet born has no stored note, so no
    // cards and no history — its pane is empty until its first field commits (ADR-0021 §7).
    let pane = ed.note.and_then(|id| cards::card_pane(coll, id).ok());

    // On a phone the two panes are a `Write | Cards` toggle (ADR-0012 §1); where both fit they show
    // together (ADR-0025 §5). Width is the only signal, and it is enough: the soft-keyboard failure
    // ADR-0025 addresses is vertical, so the toggle stands on its own merits rather than necessity.
    let both_fit = ui.available_width() >= TWO_PANE_MIN_WIDTH;
    if !both_fit {
        pane_toggle(ui, &mut ed.show_cards);
        ui.add_space(8.0);
    }

    if both_fit || !ed.show_cards {
        editor_form_body(ui, coll, ed, pane.as_ref());
    }
    if both_fit || ed.show_cards {
        if both_fit {
            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);
        }
        editor_cards_body(ui, pane.as_ref());
    }
}

/// Below this width the two editor panes cannot sit side by side, and the screen shows the
/// `Write | Cards` toggle instead (ADR-0012 §1, ADR-0025 §5). A layout threshold, never a device check.
const TWO_PANE_MIN_WIDTH: f32 = 640.0;

/// The phone's `Write | Cards` pane toggle (ADR-0012 §1): two mutually exclusive choices, the current
/// one marked. Which pane is showing is the only thing it changes — there is no third state.
fn pane_toggle(ui: &mut egui::Ui, show_cards: &mut bool) {
    ui.horizontal(|ui| {
        if ui
            .selectable_label(!*show_cards, text(ui, "Write"))
            .clicked()
        {
            *show_cards = false;
        }
        if ui
            .selectable_label(*show_cards, text(ui, "Cards"))
            .clicked()
        {
            *show_cards = true;
        }
    });
}

/// The editor's **form body** (ADR-0012 §1): the destructive-edit warning **above** the fields
/// (ADR-0025 §4), then the fields, each autosaved per blur (ADR-0021 §7), plus *Blank it* for a
/// `cloze` selection and the *New note* chord. The warning sits above the fields because under a soft
/// keyboard only the form's first screen shows, and after the last field the warning is off it,
/// leaving only a counter — which does not warn (ADR-0018 §4, ADR-0025 §4).
fn editor_form_body(
    ui: &mut egui::Ui,
    coll: &mut Collection,
    ed: &mut Editing,
    pane: Option<&cards::CardPane>,
) {
    // The ambient warning, above the fields. It names each dormant card and its kept history and
    // offers Undo — the form pane's speaker (the card pane demonstrates), and not a bare count.
    if let Some(warning) = pane.and_then(|p| p.warning.as_ref()) {
        warning_banner(ui, warning);
        ui.add_space(8.0);
    }

    // Bare Enter is inert in every single-line field, the last one included (ADR-0012 §7, ADR-0021
    // §8); the *New note* rhythm is a modifier chord, never bare Enter — `cloze`'s multiline field
    // would need Enter for a newline anyway, so "Enter on the last field" could never be uniform.
    let bare_enter = ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.command);
    let new_note_chord = ui.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.command);

    let kind = ed.kind.clone();
    let was_committed = ed.note.is_some();
    let mut note = ed.note;
    for idx in 0..ed.fields.len() {
        let name = ed.fields[idx].0.clone();
        // `cloze`'s Text field is the one multiline field — Enter inserts a newline there, and it
        // carries the *Blank it* action.
        let cloze_text = kind == "cloze" && name == "Text";
        field_label(ui, &name);
        let resp = if cloze_text {
            cloze_text_field(ui, &mut ed.fields[idx].1, pane)
        } else {
            text_field(ui, &mut ed.fields[idx].1)
        };
        // Enter keeps focus in a single-line field: egui treats it as submit-and-blur, so re-grab
        // focus and let nothing else happen (ADR-0012 §7).
        if !cloze_text && bare_enter && resp.lost_focus() {
            resp.request_focus();
        }
        // Autosave on blur (ADR-0021 §7): a field settles as one row when it loses focus, and the
        // note is created here if this is its first non-empty field.
        if resp.lost_focus() {
            let value = ed.fields[idx].1.clone();
            if let Ok(committed) = editor::commit_field(coll, note, &kind, &name, &value) {
                note = committed;
            }
        }
    }
    // A draft born this frame carries the deck chosen before it existed (ADR-0021 §9): apply it once,
    // on the None→Some transition, so a note filed at creation lands under its deck.
    if !was_committed && let Some(id) = note {
        let _ = editor::set_note_deck(coll, id, ed.deck);
    }
    ed.note = note;

    // *New note* (ADR-0021 §8): commit the current buffers, then start a fresh draft carrying the kind
    // forward — under autosave, that is all "save and add another" ever meant. Bound to the modifier
    // chord, so it can never collide with a field's own Enter.
    if new_note_chord {
        for (field, value) in ed.fields.clone() {
            if let Ok(committed) = editor::commit_field(coll, note, &kind, &field, &value) {
                note = committed;
            }
        }
        *ed = Editing::new_draft(&kind);
    }
}

/// The `cloze` Text field plus its *Blank it* action (ADR-0012 §3): wrap the current selection as
/// `{{n::…}}`, numbered **one above the highest ever used** — including this note's dormant blanks,
/// which the text alone cannot show (`cards::next_blank_number`) — never the lowest free one, so a new
/// blank can never reclaim a deleted card's identity. Enabled only while text is selected, since a
/// blank is *made from a selection*. Returns the field's response for the autosave path.
fn cloze_text_field(
    ui: &mut egui::Ui,
    buffer: &mut String,
    pane: Option<&cards::CardPane>,
) -> egui::Response {
    let output = multiline_field_output(ui, buffer);
    let selection = output.cursor_range.filter(|r| !r.is_empty());
    let clicked = ui
        .add_enabled_ui(selection.is_some(), |ui| {
            full_width_button(ui, "Blank it").clicked()
        })
        .inner;
    if clicked && let Some(range) = selection {
        blank_selection(buffer, range, pane);
    }
    output.response.response
}

/// Wrap the selected `range` of `buffer` as a new `{{n::…}}` blank (ADR-0012 §3). The number is
/// [`cards::next_blank_number`] when the note is stored (so its dormant blanks count as "ever used"),
/// or the text-only rule for an unborn draft.
fn blank_selection(
    buffer: &mut String,
    range: egui::text::CCursorRange,
    pane: Option<&cards::CardPane>,
) {
    let chars = range.as_sorted_char_range();
    let start = byte_index(buffer, chars.start.0);
    let end = byte_index(buffer, chars.end.0);
    let number = match pane {
        Some(p) => cards::next_blank_number(p, buffer),
        None => leitner_core::content::next_blank_number(buffer),
    };
    let selected = buffer[start..end].to_owned();
    buffer.replace_range(start..end, &format!("{{{{{number}::{selected}}}}}"));
}

/// The byte offset of the `char_index`-th character in `s`, or `s.len()` past the end — the bridge
/// from egui's character-indexed cursor to a byte range `String::replace_range` accepts.
fn byte_index(s: &str, char_index: usize) -> usize {
    s.char_indices().nth(char_index).map_or(s.len(), |(b, _)| b)
}

/// The ambient destructive-edit warning, above the fields (ADR-0012 §5, ADR-0025 §4). It names each
/// dormant card and its **kept** history and states Undo — it is not the counter ADR-0018 §4 forbids.
fn warning_banner(ui: &mut egui::Ui, warning: &cards::Warning) {
    body(ui, "This edit made cards dormant:");
    for entry in &warning.dormant {
        badge(ui, &entry.history());
    }
    body(ui, cards::UNDO_COPY);
}

/// The editor's **card body**: the cards this note currently generates, in raw-slot order, live and
/// dormant interleaved (ADR-0018 §1). A live entry is a card; a **dormant entry is a single line**
/// (ADR-0018 §2). A pane with nothing live is its own state, distinct from the empty note (ADR-0018
/// §6). What each row *looks* like is the visual design pass's.
fn editor_cards_body(ui: &mut egui::Ui, pane: Option<&cards::CardPane>) {
    field_label(ui, "Cards");
    ui.add_space(4.0);
    let Some(pane) = pane else {
        body(ui, "No cards yet.");
        return;
    };
    if pane.entries.is_empty() {
        body(ui, "No cards yet.");
        return;
    }
    for entry in &pane.entries {
        match entry {
            cards::Entry::Live(card) => {
                card_face(ui, &card.prompt);
                if !card.answer.is_empty() {
                    card_face(ui, &card.answer);
                }
                badge(ui, &box_badge_wording(card.reviews > 0, card.box_));
                ui.add_space(8.0);
            }
            // A dormant entry is a single line — its name, *dormant*, its kept history (ADR-0018 §2).
            cards::Entry::Dormant(dormant) => badge(ui, &dormant.history()),
        }
    }
    if pane.state == cards::State::NoLiveCards {
        ui.add_space(8.0);
        body(ui, "This note currently generates no cards.");
    }
}

/// The `cloze` Text field rendered through the bidi layouter, returning the full [`egui::text_edit::
/// TextEditOutput`] so the caller can read the current selection for *Blank it*. Mirrors [`text_field`]
/// for the multiline case; the shared helper returns only a `Response` and cannot carry the cursor.
fn multiline_field_output(
    ui: &mut egui::Ui,
    buffer: &mut String,
) -> egui::text_edit::TextEditOutput {
    let rtl = bidi::is_rtl(buffer);
    let mut layouter = bidi_layouter;
    let out = egui::TextEdit::multiline(buffer)
        .desired_width(f32::INFINITY)
        .horizontal_align(if rtl {
            egui::Align::RIGHT
        } else {
            egui::Align::LEFT
        })
        .layouter(&mut layouter)
        .show(ui);
    // The keyboard raise rides here too (ADR-0026 §4). The two wrappers are one wrapper in the sense
    // that matters — they share `bidi_layouter` and every field goes through one of them — and a
    // promise made to "every field" that skipped `cloze`'s Text is not a promise.
    raise_keyboard(ui.ctx(), &out.response);
    out
}

/// The **Settings** destination (ADR-0021 §1), holding the sync surface (ADR-0015 §12, ADR-0019 §1).
///
/// This renders the *surface* — the words and the refusals — for the not-yet-enrolled device: the
/// promise, the entry to enrolment, and the durable removal route. The enrolled surface (the resting
/// "Last caught up ⟨when⟩", the connected account, Sync now, the device list, Disconnect and the
/// history cutoff) is modelled and proven in `sync`, but it needs a live grant, and the device flow
/// that obtains one carries the network this environment lacks (ADR-0013 §11) — so it is wired when
/// that mechanism lands, not faked here. What is fixed now is what each surface *says*.
// Each screen threads its own `&mut` slice of `LeitnerApp` state plus the frame's `now_ms`; grouping
// them behind a struct would only relocate the same fields, not reduce them.
#[allow(clippy::too_many_arguments)]
fn settings_screen(
    ui: &mut egui::Ui,
    coll: &mut Collection,
    setting_up: &mut bool,
    rate_buffer: &mut Option<String>,
    optimise_job: &mut Option<optimise::OptimiseJob>,
    optimise_done: &mut bool,
    handoff: &mut HandOff,
    now_ms: i64,
) -> bool {
    heading(ui, "Settings");
    ui.add_space(8.0);

    if *setting_up {
        enrolment_screen(ui, setting_up);
        return false;
    }

    new_card_rate_control(ui, coll, rate_buffer);
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);

    optimise_control(ui, coll, optimise_job, optimise_done, now_ms);
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);

    // The promise, worded once (ADR-0015 §3) — never "automatic", never "in the background".
    body(ui, sync::PROMISE);
    ui.add_space(8.0);

    // "Set up sync" is the entry, not "login" or "pairing" (ADR-0015 §7): there is no account of ours
    // and no device-to-device step.
    if full_width_button(ui, sync::SET_UP_SYNC).clicked() {
        *setting_up = true;
    }

    ui.add_space(12.0);
    // The removal route and the app name, kept permanently because the folder is hidden and cannot be
    // navigated to (ADR-0015 §10, ADR-0020 §4). Disconnect is the only control this app owns.
    body(ui, &sync::revocation_and_removal());

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    let reset = reset_control(ui);

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    rendering_specimen(ui);

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    handoff_specimen(ui, handoff);

    reset
}

/// **Temporary, and not a specified feature.** The rendering specimen: every script the shipped faces
/// exist for, drawn in every family they are registered into, so issue #97's criteria can be read off
/// one screen by someone who reads the script.
///
/// **It is here because the handset cannot be asked any other way.** Client-stack rule 8 makes Android
/// text input ASCII-only — there is no IME path, so a Persian sentence can never be *typed* on the
/// device — and a screenshot compared against a reference image only tells a non-reader that something
/// shaped like Persian appeared. So the strings ship in the binary, each above what it must read, and
/// the judgement handed over is the one only a reader can make.
///
/// **Three families, drawn one under the other on purpose.** A face is resolved per family and per
/// character, so the same string can be right in `Proportional` and wrong in `Monospace` or in
/// [`fonts::bold_family`] with nothing failing anywhere (client-stack rule 7). Stacking them puts the
/// three renderings of one string side by side, which is the only way a wrong *face* — as opposed to a
/// missing glyph — shows up at all: it draws, it just draws in the wrong hand.
///
/// It goes through [`bidi::job`] like every other string in the app, because half of what is being
/// checked is the ordering that helper produces (client-stack rule 1) rather than the glyphs alone.
fn rendering_specimen(ui: &mut egui::Ui) {
    body(
        ui,
        "Development control — every script the shipped faces exist for, in every family. Each line \
         below is the same text drawn by a different family; check it against the caption above it.",
    );
    ui.add_space(8.0);

    for (caption, specimen) in fonts::SPECIMENS {
        field_label(ui, caption);
        for family in fonts::families() {
            ui.horizontal(|ui| {
                // The family's own name, in the family itself: a tag drawn in some *other* face
                // would be naming a rendering it is not part of.
                ui.label(bidi::job(
                    &family_tag(&family),
                    egui::FontId::new(11.0, family.clone()),
                    ui.visuals().weak_text_color(),
                ));
                ui.label(bidi::job(
                    specimen,
                    egui::FontId::new(20.0, family.clone()),
                    ui.visuals().text_color(),
                ));
            });
        }
        ui.add_space(10.0);
    }
}

/// **Temporary, and not a specified feature.** The hand-off specimen: the two user-files calls issue
/// #98 asks to be verified on the handset, behind **two separate buttons**.
///
/// **It is here because nothing else reaches them.** [#88](https://github.com/amin-bf/leitner/issues/88)
/// landed `leitner-export` and its four-operation seam but deferred the export *screen* to the visual
/// pass, so `put` and `hand_off` have no call site in this crate — and every one of #98's criteria is
/// about what those two calls do at runtime on a real `MediaStore`. A seam with no caller cannot be
/// verified by holding the phone.
///
/// **Two buttons rather than one, and that is the point rather than a convenience.**
/// [ADR-0023 §5](../../../docs/adr/0023-sending-a-written-file.md) says the affordance *never fires by
/// itself*: nothing opens when an export finishes. A specimen that wrote and then shared in one press
/// would satisfy every other criterion while making that one unobservable — the sheet would appear
/// either way, and no one watching could tell which rule was in force.
///
/// **It reports the name it was given back, never the one it asked for**
/// ([ADR-0022 §10](../../../docs/adr/0022-the-import-preview-and-export-report.md)), and it shows both
/// so the difference is legible: press it twice and the second write collides, which is the whole of
/// [ADR-0024 §4](../../../docs/adr/0024-identifying-a-written-file.md)'s claim that declaring no media
/// type is what keeps the extension. The bytes are identical across presses on purpose — same name,
/// same content — so the collision is the one that ADR's probe measured and not a different event.
fn handoff_specimen(ui: &mut egui::Ui, state: &mut HandOff) {
    body(
        ui,
        "Development control — the two user-files calls, one per button. Write puts a real .ldeck \
         through the seam and states the name the platform wrote back. Hand off opens the system \
         share sheet for it, and only when pressed: writing never opens anything.",
    );
    ui.add_space(4.0);

    if full_width_button(ui, "Write a deck file (temporary)").clicked() {
        state.said = match specimen_deck() {
            Err(e) => format!("Could not build the file: {e}"),
            Ok(bytes) => {
                let requested = leitner_export::export_filename(&[SPECIMEN_DECK_NAME]);
                match leitner_export::platform::put(&requested, &bytes) {
                    Err(e) => format!("Could not write it: {e}"),
                    Ok(written) => {
                        let said = format!(
                            "Asked for \"{requested}\" — written as \"{}\".",
                            written.name
                        );
                        state.written = Some(written.name);
                        said
                    }
                }
            }
        };
    }

    ui.add_space(4.0);

    if full_width_button(ui, "Hand it off (temporary)").clicked() {
        state.said = match &state.written {
            None => "Nothing written yet — write a deck file first.".to_owned(),
            Some(name) => match leitner_export::platform::hand_off(name) {
                Ok(()) => format!("Handed \"{name}\" onward. Nothing is reported after this."),
                Err(e) => format!("Could not hand it off: {e}"),
            },
        };
    }

    if !state.said.is_empty() {
        ui.add_space(8.0);
        body(ui, &state.said);
    }
}

/// The specimen deck's display name — the filename derives from it, sanitised outbound.
const SPECIMEN_DECK_NAME: &str = "Specimen";

/// Fixed ids, so every press builds **byte-identical** content and a second write is a true
/// same-name collision rather than a new file. `leitner-core` never mints an id (ADR-0009 §8), and a
/// specimen has no collection to take one from.
const SPECIMEN_DECK_ID: DeckId = DeckId([
    0x98, 0x0d, 0xec, 0x00, 0x40, 0x00, 0x40, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
]);
const SPECIMEN_NOTE_ID: NoteId = NoteId([
    0x98, 0x0d, 0xec, 0x00, 0x40, 0x00, 0x40, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02,
]);

/// A real `.ldeck` — the actual container, not a stand-in. What is being verified is what the
/// platform does with the bytes and the name, so a placeholder payload would still exercise the
/// seam; a real one additionally lets whoever receives the share open it.
fn specimen_deck() -> Result<Vec<u8>, leitner_export::ExportError> {
    let content = leitner_export::DeckContent {
        id: SPECIMEN_DECK_ID,
        name: SPECIMEN_DECK_NAME.to_owned(),
        notes: vec![leitner_export::NoteContent {
            id: SPECIMEN_NOTE_ID,
            position: "n".to_owned(),
            kind: "basic".to_owned(),
            fields: vec![
                ("Front".to_owned(), "specimen front".to_owned()),
                ("Back".to_owned(), "specimen back".to_owned()),
            ],
        }],
        tombstones: Vec::new(),
    };
    let digest = leitner_export::deck_digest(&content)?;
    let revision = leitner_export::next_revision(None, &digest);
    leitner_export::build_deck(
        &leitner_export::Metadata::default(),
        &[leitner_export::DeckExport { content, revision }],
    )
}

/// The short name of a family, for the specimen's row tag.
fn family_tag(family: &egui::FontFamily) -> String {
    match family {
        egui::FontFamily::Proportional => "prop".to_owned(),
        egui::FontFamily::Monospace => "mono".to_owned(),
        egui::FontFamily::Name(name) => name.to_string(),
    }
}

/// **Temporary, and not a specified feature.** A development control that returns this device to a
/// **first launch** — the collection deleted and reseeded exactly as [`LeitnerApp::open_store`] does it
/// on a fresh install — so an on-handset verification run does not need a cable and `run-as` to get back
/// to a known state. Returns whether it was pressed.
///
/// **It is a reset, not a delete, and it is not a step towards a user-facing one.** Nothing in this design
/// removes data: [ADR-0016 §1](../../../docs/adr/0016-backup-and-restore.md) establishes that restore is
/// a merge and a replace is *not implementable*, because every device holds the whole log and merge is
/// set union — so a wipe here is undone by the next sync from any peer that still holds those rows. It
/// is honest only as what it says it is: a local reset on a device being tested against.
/// [ADR-0015 §10](../../../docs/adr/0015-the-sync-experience.md) separately forbids a control that
/// deletes *published* data, which this does not touch.
fn reset_control(ui: &mut egui::Ui) -> bool {
    body(
        ui,
        "Development control — returns this device to a first launch, seed and all. Rows other \
         devices hold come back on the next sync.",
    );
    ui.add_space(4.0);
    full_width_button(ui, "Reset the collection (temporary)").clicked()
}

/// The new-card-rate control (ADR-0011 §3): a plain integer field, with the consequence explained
/// where it is set — no modal, no automatic mode. The buffer is seeded from the stored rate on first
/// show and committed on a completed edit (blur), clamped and defaulted in the store; **zero is a
/// legal value and the backlog answer**, so an empty or unparsable field is left for the user to
/// finish rather than snapped to a number. It never enters the log and never exports (ADR-0011 §5).
fn new_card_rate_control(
    ui: &mut egui::Ui,
    coll: &mut Collection,
    rate_buffer: &mut Option<String>,
) {
    // Seed the buffer from the stored rate the first time this screen is shown.
    let buffer = rate_buffer.get_or_insert_with(|| {
        coll.new_card_rate()
            .unwrap_or(DEFAULT_NEW_CARD_RATE)
            .to_string()
    });

    field_label(ui, "New cards a day");
    let resp = text_field(ui, buffer);
    // Commit on blur: a completed edit that parses writes the (clamped) rate back; zero is kept.
    if resp.lost_focus()
        && let Ok(rate) = buffer.trim().parse::<u32>()
    {
        // A failed write is dropped rather than surfaced: the re-read below then reflects the
        // unchanged stored value, so the field simply shows the edit did not take. Surfacing write
        // errors is a later ticket, as at the review grade site.
        let _ = coll.set_new_card_rate(rate);
        // Reflect the clamp back into the buffer so an out-of-range entry shows what was stored.
        *buffer = coll
            .new_card_rate()
            .unwrap_or(DEFAULT_NEW_CARD_RATE)
            .to_string();
    }
    ui.add_space(4.0);
    // The consequence, stated where the choice is (ADR-0011 §3, §4): this is the only enforced limit,
    // and zero is how a backlog is cleared before turning it back on.
    body(
        ui,
        "The only limit in the app. Set it to zero to clear a backlog, then turn it back on.",
    );
}

/// The parameter-optimisation control (ADR-0014 §2, §3, §4). **The action is always present** — a
/// button that is sometimes absent teaches the feature does not exist — with the fact-only nudge
/// beneath it. Pressing it starts a worker thread the frame loop polls; while it runs, the button is
/// replaced in place by the two-phase progress and a Cancel (§4), and **nothing is written until it
/// completes**. On completion the fitted vector is written — skipped if unchanged (§5) — and the
/// factual completion message shown, which makes no quality claim (§4).
///
/// The words and the run's shape are proven in `optimise`; this is the egui wiring the visual pass
/// refines. ADR-0014 §7's *sync, then train* is a no-op here: no transport is enrolled in this build,
/// and an offline device optimising on local history is a fine outcome — the leading sync is a
/// sequence, never a gate.
fn optimise_control(
    ui: &mut egui::Ui,
    coll: &mut Collection,
    job: &mut Option<optimise::OptimiseJob>,
    done: &mut bool,
    now_ms: i64,
) {
    field_label(ui, "Scheduler");

    if let Some(running) = job.as_mut() {
        // A run is in flight: keep the frame loop turning so `poll` is reached, then render the phase.
        ui.ctx().request_repaint();
        match running.phase() {
            optimise::Phase::Preparing => {
                ui.horizontal(|ui| {
                    ui.add(egui::Spinner::new());
                    body(ui, "Preparing…");
                });
            }
            optimise::Phase::Training { current, total } => {
                let fraction = if total == 0 {
                    0.0
                } else {
                    current as f32 / total as f32
                };
                ui.add(egui::ProgressBar::new(fraction).show_percentage());
            }
        }
        if full_width_button(ui, "Cancel").clicked() {
            running.cancel();
        }
        // Poll once this frame. On completion, write the vector (unchanged ones write nothing, §5) and
        // drop the job. A cancelled or failed run yields `None`: nothing to write, recover by pressing
        // the button again.
        if let Some(result) = running.poll() {
            if let Some(outcome) = result {
                // A failed write is dropped rather than surfaced, matching the review-grade site; the
                // nudge simply re-reads the unchanged row next frame.
                let _ = coll.set_scheduler_parameters(
                    outcome.parameters.weights(),
                    outcome.fitted_over,
                    now_ms,
                    DayScale::default(),
                );
                *done = true;
            }
            *job = None;
        }
        return;
    }

    // At rest: the always-present action, the fact-only nudge, and the completion message if a run
    // just finished (ADR-0014 §2, §4).
    if full_width_button(ui, "Optimise").clicked() {
        *done = false;
        let lines = coll.log_lines().unwrap_or_default();
        *job = Some(optimise::OptimiseJob::start(lines));
        ui.ctx().request_repaint();
    }
    ui.add_space(4.0);

    let nudge = coll
        .log_lines()
        .map(|lines| {
            let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
            optimise::nudge_text(&leitner_core::replay::optimisation_nudge(&refs))
        })
        .unwrap_or_default();
    body(ui, &nudge);

    if *done {
        ui.add_space(4.0);
        body(ui, optimise::COMPLETION_MESSAGE);
    }
}

/// The enrolment screen's surface (ADR-0015 §7, ADR-0019 §4): what it states *before* the grant. The
/// device flow, the credential file and the UserInfo fetch that would follow are the deferred network
/// mechanism (see `sync` and ADR-0013 §11); this screen owns the plain-words scope and the one-time
/// disclosure, which are decided and need no network to state.
fn enrolment_screen(ui: &mut egui::Ui, setting_up: &mut bool) {
    heading(ui, sync::SET_UP_SYNC);
    ui.add_space(8.0);

    // The scope, in plain words (ADR-0015 §7, ADR-0019 §4): the consent screen asks for two things.
    body(ui, sync::SCOPE_PLAIN_WORDS);
    ui.add_space(8.0);
    // The promise again — it appears at enrolment and in settings, and nowhere else (ADR-0015 §3).
    body(ui, sync::PROMISE);
    ui.add_space(8.0);
    // What leaves the device, stated once (ADR-0020 §7): not a status message, never promoted to a
    // resting surface.
    body(ui, sync::DISCLOSURE_CLAUSE);

    ui.add_space(12.0);
    // The device flow itself needs the network and a handset (ADR-0013 §11): the surface is settled
    // here, the grant is its own step. Stated plainly rather than offered as a control that cannot
    // complete.
    field_label(
        ui,
        "Granting access uses the device flow, which needs a network connection — not available in \
         this build.",
    );

    ui.add_space(8.0);
    if full_width_button(ui, "Back").clicked() {
        *setting_up = false;
    }
}

// --- small rendering helpers, every one through the bidi layout so no screen holds a bare label ---

fn now_ms() -> i64 {
    // The one clock read on the review path — an edge value, never reached from `leitner-core`
    // (ADR-0009 §8). A clock before the epoch is not a real handset state; clamp rather than wrap.
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn text(ui: &egui::Ui, s: &str) -> egui::text::LayoutJob {
    bidi::job(
        s,
        egui::TextStyle::Button.resolve(ui.style()),
        ui.visuals().text_color(),
    )
}

fn heading(ui: &mut egui::Ui, s: &str) {
    ui.label(bidi::job(
        s,
        egui::TextStyle::Heading.resolve(ui.style()),
        ui.visuals().text_color(),
    ));
}

fn body(ui: &mut egui::Ui, s: &str) {
    ui.label(bidi::job(
        s,
        egui::TextStyle::Body.resolve(ui.style()),
        ui.visuals().text_color(),
    ));
}

/// The box badge: a small, non-interactive indicator, weaker than body text so it never reads as a
/// call to action.
/// The picker's statement when nothing is due but cards have never been seen: the fact, then the
/// invitation, and **no claim that the deck is fresh** — the collection has history behind it.
///
/// It states nothing about being behind, because the reviewer is not: reaching this state means the
/// day's repeats are finished and only ADR-0011 §2's rate stands between them and the rest.
fn new_only_wording(new: usize) -> String {
    if new == 1 {
        "Nothing due right now. One new card, whenever you like.".to_string()
    } else {
        format!("Nothing due right now. {new} new cards, whenever you like.")
    }
}

/// What the **box badge** reads: the durability box, or `new` for a card with **no review history**
/// (ADR-0006 §6).
///
/// The `new` case is not a nicety, and it is the one every call site is liable to drop, because
/// [`box_of`](leitner_core::scheduling::box_of) is total and answers `1` for a card it has never seen.
/// `1` is *also* the honest answer for a card reviewed thirty times and never retained — so rendering
/// the number regardless makes the badge state a durability nothing has measured, on the one card where
/// the user can tell it is wrong. A first introduction then reads as *bottom box*, a position in a queue
/// of boxes, which is precisely the reading [ADR-0001 §3] forbids the badge from acquiring; `new` states
/// the absence of history instead, which is what is true.
///
/// It is one function because the badge means one thing wherever it is drawn — the review screen and the
/// card pane both come through here, and a second `format!("Box {}")` anywhere is this defect returning.
///
/// [ADR-0001 §3]: ../../../docs/adr/0001-scheduling-algorithm-and-grade-scale.md
fn box_badge_wording(reviewed: bool, box_: u8) -> String {
    if reviewed {
        format!("Box {box_}")
    } else {
        "new".to_string()
    }
}

/// A small, non-interactive, weak-coloured footnote carrying bidi-laid text.
fn badge(ui: &mut egui::Ui, s: &str) {
    ui.label(bidi::job(
        s,
        egui::TextStyle::Small.resolve(ui.style()),
        ui.visuals().weak_text_color(),
    ));
}

/// A full-width button carrying bidi-laid text.
fn full_width_button(ui: &mut egui::Ui, s: &str) -> egui::Response {
    let job = text(ui, s);
    ui.add_sized([ui.available_width(), 36.0], egui::Button::new(job))
}

/// The card face — a wide, tall clickable surface. Tapping the prompt reveals; the answer face is
/// drawn the same way for visual consistency, its click ignored.
fn card_face(ui: &mut egui::Ui, s: &str) -> egui::Response {
    let job = text(ui, s);
    ui.add_sized([ui.available_width(), 96.0], egui::Button::new(job))
}

/// A small label for a field or control — a form-pane caption, weaker than body text.
fn field_label(ui: &mut egui::Ui, s: &str) {
    ui.label(bidi::job(
        s,
        egui::TextStyle::Small.resolve(ui.style()),
        ui.visuals().weak_text_color(),
    ));
}

/// A text field routed through the bidi layouter (`AGENTS.md` client-stack rule 2): a `TextEdit` lays
/// out its own text and otherwise bypasses the helper, so Persian would render with the words
/// backwards. The layouter resets `halign` to `LEFT` — an RTL job otherwise spans negative x and the
/// field clips its last character (see `bidi::job`) — and the field's own `horizontal_align` carries
/// the direction, chosen per the buffer's first strong character (ADR-0012 §7's `dir="auto"`).
///
/// Single-line only: Enter is a submit egui turns into a blur, which the caller re-grabs to keep Enter
/// inert (ADR-0012 §7, ADR-0021 §8). The one multiline field — `cloze`'s Text — goes through
/// [`multiline_field_output`] instead, which shares [`bidi_layouter`] but exposes the cursor for
/// *Blank it*.
fn text_field(ui: &mut egui::Ui, buffer: &mut String) -> egui::Response {
    let rtl = bidi::is_rtl(buffer);
    let mut layouter = bidi_layouter;
    let response = ui.add(
        egui::TextEdit::singleline(buffer)
            .desired_width(f32::INFINITY)
            .horizontal_align(if rtl {
                egui::Align::RIGHT
            } else {
                egui::Align::LEFT
            })
            .layouter(&mut layouter),
    );
    raise_keyboard(ui.ctx(), &response);
    response
}

/// **Guard 3** — raise the soft keyboard when the user clicks a text field and it is down
/// (ADR-0026 §4).
///
/// This is the recovery half of the vendored adapter patch (`vendor/PATCH.md`), and it is not
/// optional. Once the per-tap interrupt is suppressed, nothing re-asserts *show* after the user
/// dismisses the keyboard with the IME's own chevron: the adapter debounces its allow-IME flag
/// against its own previous value, and that value never changed — only the platform's did. The
/// interrupt block was the only thing re-asserting it, so removing the defect removes recovery with
/// it. An implementation that takes the patch without this ships a keyboard the user cannot get
/// back.
///
/// It goes through `ViewportCommand::IMEAllowed(true)`, which the adapter maps straight onto the
/// window without touching that debounced flag — public API, and no state to desync.
///
/// **Keyed on the field's own discrete click**, never on a per-frame "something is focused and the
/// pointer went down": `request_focus` fires while *dragging* too, and the version that hung off it
/// issued **72 show requests from a single scroll gesture**. Both wrong versions made the same
/// mistake — hanging behaviour off a per-frame flag when the thing being modelled is a discrete
/// event.
///
/// **Gated on a keyboard that exists and is currently down**, read from the seam at the moment of the
/// click rather than from a cached frame value, so "is it actually down" is answered by the platform.
/// The gate is what keeps the measured zero-hides-zero-shows shape: without it every click sends a
/// redundant show. And it is the seam's honest return type that makes the gate correct off Android —
/// a platform with no soft keyboard answers `Absent`, not "down" (ADR-0026 §5).
fn raise_keyboard(ctx: &egui::Context, response: &egui::Response) {
    if response.clicked() && crate::platform::insets().keyboard.is_down() {
        ctx.send_viewport_cmd(egui::ViewportCommand::IMEAllowed(true));
    }
}

/// The bidi text-edit layouter shared by every field (`text_field`, `multiline_field_output`): it
/// runs the field's contents through the `bidi` helper, left-aligned within the edit, so untrusted
/// mixed-script text lays out correctly wherever it is typed.
fn bidi_layouter(
    ui: &egui::Ui,
    text: &dyn egui::TextBuffer,
    wrap_width: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = bidi::job(
        text.as_str(),
        egui::TextStyle::Body.resolve(ui.style()),
        ui.visuals().text_color(),
    );
    job.halign = egui::Align::LEFT;
    job.wrap.max_width = wrap_width;
    ui.fonts_mut(|f| f.layout_job(job))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pointer_states_a_cost_and_is_singular_or_plural() {
        // ADR-0010 §6: the pointer is a plain statement of cost, not a bare count and not a decision.
        assert_eq!(
            pointer_wording(1),
            "1 card is costing you a lot. Take a look?"
        );
        assert_eq!(
            pointer_wording(3),
            "3 cards are costing you a lot. Take a look?"
        );
    }

    #[test]
    fn the_leech_entry_names_leeches_suspended_or_both() {
        // ADR-0010 §6, §8: the durable entry to the leech screen says what it holds — the active
        // leeches, the suspended cards whose permanent home it is, or both.
        assert_eq!(leech_entry_wording(2, 0), "Leeches (2)");
        assert_eq!(leech_entry_wording(0, 3), "Suspended (3)");
        assert_eq!(leech_entry_wording(2, 3), "Leeches (2) · suspended (3)");
    }

    #[test]
    fn nothing_due_with_new_cards_left_never_calls_the_deck_fresh() {
        // The sentence a reviewer with history gets when the day's repeats are done. It states the
        // fact and invites, and it must not claim freshness (ADR-0006 §8) or imply falling behind.
        assert_eq!(
            new_only_wording(1),
            "Nothing due right now. One new card, whenever you like."
        );
        assert_eq!(
            new_only_wording(4),
            "Nothing due right now. 4 new cards, whenever you like."
        );
    }

    #[test]
    fn a_card_with_no_review_history_badges_new_not_box_one() {
        // ADR-0006 §6. `box_of` is total and answers 1 for a never-reviewed card, so the badge has to
        // ask whether there is any history at all — otherwise a first introduction claims a durability
        // nothing measured, and reads as the bottom of a queue of boxes (ADR-0001 §3).
        assert_eq!(box_badge_wording(false, 1), "new");
        assert_eq!(box_badge_wording(true, 1), "Box 1");
        assert_eq!(box_badge_wording(true, 4), "Box 4");
    }
}
