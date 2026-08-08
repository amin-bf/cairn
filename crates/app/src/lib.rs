//! The egui application: the application state and frame loop, the shared text-layout helpers, and
//! both entry points. The destinations' screens live one module each under [`screens`].
//!
//! **This crate deliberately has no `src/main.rs`.** `cargo-apk` panics after signing
//! (`Bin is not compatible with Cdylib`) when one crate has both a cdylib and a bin — the APK comes
//! out correct but the exit code does not, and CI breaks. The desktop binary is `cairn-desktop`,
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
pub mod inbound;
pub mod keyboard;
pub mod listing;
pub mod markdown;
pub mod notes;
pub mod optimise;
pub mod platform;
mod screens;
pub mod session;
pub mod sync;

use std::collections::HashSet;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use cairn_core::content::{CardRef, DeckId, NoteId};
use cairn_core::log::{DEFAULT_NEW_CARD_RATE, DayScale, day_number};
use cairn_store::Collection;

use screens::notes::notes_screen;
use screens::review::review;
use screens::settings::{FileList, HandOff, settings_screen};

/// Re-exported so `cairn-desktop` needs no `eframe` dependency of its own — it cannot then
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
pub(crate) struct Sitting {
    /// The count the user chose — the number of **gradings** this sitting runs to.
    pub(crate) chosen: usize,
    /// Gradings performed so far. This is the progress numerator, and it counts every grade press,
    /// including a lapsed card's same-session re-show (ADR-0011 §9).
    pub(crate) graded: usize,
    /// The card currently on screen, so a reveal survives a frame and resets when the card changes.
    pub(crate) shown: Option<CardRef>,
    pub(crate) revealed: bool,
    /// When the sitting began — the quiet 10-minute timer runs from here (issue #94).
    started: Instant,
    /// When the current card came on screen, so the row can record how long the answer took
    /// (ADR-0004 §5).
    pub(crate) card_shown: Instant,
    /// Set once the user answers the 10-minute checkpoint's "keep going", so it does not nag again.
    pub(crate) checkpoint_dismissed: bool,
    /// The set of cards that were already leeches when this sitting began — the `before` half of the
    /// end-of-session pointer (ADR-0010 §6). Held **in memory only**, like everything else about a
    /// sitting: the pointer covers cards that crossed *this* session, which is exactly `leeches now`
    /// minus this snapshot, and it needs no stored dismissal or last-seen state.
    pub(crate) leeches_at_start: HashSet<CardRef>,
}

impl Sitting {
    pub(crate) fn new(chosen: usize, leeches_at_start: HashSet<CardRef>) -> Self {
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
    pub(crate) fn checkpoint_due(&self) -> bool {
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
pub(crate) struct Editing {
    pub(crate) note: Option<NoteId>,
    pub(crate) kind: String,
    pub(crate) fields: Vec<(String, String)>,
    /// The deck this note is filed under, chosen in the editor's deck dropdown beside the kind one
    /// (ADR-0021 §9). `None` is unfiled — a legal, still-reviewable state (ADR-0005 §8). On a draft
    /// the choice is held here until the note is born on its first non-empty field, then written once.
    pub(crate) deck: Option<DeckId>,
    /// The new-deck name buffer for *create a new deck*, available from the deck dropdown (ADR-0021
    /// §9): the moment you need a deck that does not exist is while filing the note that wants it.
    pub(crate) new_deck: String,
    /// Which of the two panes is showing when the screen is too narrow to hold both — the phone's
    /// `Write | Cards` toggle (ADR-0012 §1, ADR-0025 §5). `false` is *Write*, the form. On a wide
    /// screen both panes show and this is ignored.
    pub(crate) show_cards: bool,
}

impl Editing {
    /// A fresh draft of `kind`, carrying that kind's fields as empty buffers. Used by **create** and
    /// by the *New note* chord, which carries the current kind forward (ADR-0021 §8).
    pub(crate) fn new_draft(kind: &str) -> Self {
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
    pub(crate) fn for_note(coll: &Collection, note: NoteId) -> Self {
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
    pub(crate) fn switch_kind(&mut self, kind: &str) {
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
pub struct CairnApp {
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
    /// The note being reordered, or `None` when the list is not in its two-tap placement state
    /// (ADR-0021 §4). `Some(id)` after **Move**: the list then offers a gap target between every
    /// visible pair, and one tap places the note there. Held across frames; cleared on place or
    /// cancel, and dropped if a filter change hides the moving note.
    moving: Option<NoteId>,
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
    /// **Temporary** — the file-list specimen's state (see [`file_list_specimen`]).
    file_list: FileList,
    /// The file the platform handed us — a launch intent read once at startup, or the last file
    /// dropped on the window — held so the inbound specimen can **re-derive** its plan every frame
    /// (ADR-0022 §5). This is the *file*, never a cached plan: `inbound::read` runs the whole
    /// gate-then-describe read against the live collection each time the specimen draws.
    inbound: Option<inbound::Inbound>,
    /// Whether the launch intent has been consulted yet (`platform::launch_file` is a one-shot read,
    /// so it is asked once as the app comes up — which is where cold start is satisfied, ADR-0016 §5).
    launch_checked: bool,
}

impl CairnApp {
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
            moving: None,
            new_card_rate: None,
            showing_leeches: false,
            session_pointer: None,
            optimise_job: None,
            optimise_done: false,
            fonts_installed: false,
            band: keyboard::Band::default(),
            handoff: HandOff::default(),
            file_list: FileList::default(),
            inbound: None,
            launch_checked: false,
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

        let data = cairn_store::platform::data_dir().map_err(|e| e.to_string())?;
        let state = cairn_store::platform::state_dir().map_err(|e| e.to_string())?;
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

        let dirs = cairn_store::platform::data_dir()
            .and_then(|data| cairn_store::platform::state_dir().map(|state| (data, state)));
        if let Ok((data, state)) = dirs {
            cairn_store::remove_files(&data, &state);
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

impl eframe::App for CairnApp {
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

        // ---- The inbound file, read before the collection is borrowed (ADR-0016 §5, #107) --------
        //
        // Two routes converge on one held [`inbound::Inbound`]: the Android launch intent, consulted
        // once as the app comes up (cold-start capable — the intent is on the activity from the
        // first frame), and a desktop drop, surfaced by egui directly on the frame it lands. The
        // *file* is held; its plan is derived fresh every frame by the specimen (ADR-0022 §5).
        if !self.launch_checked {
            self.launch_checked = true;
            if let Some(launched) = platform::launch_file() {
                self.inbound = Some(launched);
            }
        }
        if let Some(dropped) = inbound::take_dropped(ui.ctx()) {
            self.inbound = Some(dropped);
        }

        let now_ms = now_ms();
        // "Due today" is the **device's local** day (replay `CONTEXT.md`), which the walking skeleton
        // reads at the default 4am scale; a real device timezone is a later ticket.
        let today = day_number(now_ms, DayScale::default());

        let coll = match self.store.as_mut() {
            Err(message) => {
                heading(ui, "Cairn");
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

        // ---- The navigation shell (ADR-0021 §1; `CONTEXT.md`'s *Top-level destination*) ----------
        //
        // **Pinned, and it yields the screen to the soft keyboard.** Two accepted constraints pull
        // against each other here, and neither document recorded which won. ADR-0021 §1 wants a
        // *persistent* affordance, because "a destination reachable only by completing a session is
        // not reachable" — and a row that scrolls away is not persistent. ADR-0025's consequences
        // make the form pane's **first screen** a specified resource, holding the destructive-edit
        // warning because nowhere else works — and a row that is always pinned spends that resource
        // out of the 565dp a keyboard leaves of 923dp.
        //
        // So it is drawn whenever the keyboard is **not up**: full reachability while the user is
        // reading, the whole first screen while they are typing, and the row is back the moment the
        // keyboard goes down. **One rule reading one fact.** It is expressible only because
        // ADR-0026 §5 made the seam distinguish *no soft keyboard on this platform* from *keyboard
        // down* — off Android the seam reports the first, `keyboard_is_up` is permanently false, and
        // the row is simply always pinned. One rule reaching two answers is **not**
        // platform-conditional behaviour, so client-stack rule 3 is untouched.
        //
        // It sits below the status-bar band and outside the scroll area, which is what makes it
        // pinned rather than merely first.
        if !self.band.keyboard_is_up() {
            egui::Panel::top("nav")
                .resizable(false)
                .show(ui, |ui| nav_bar(ui, &mut self.dest));
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
        // rather than merely clipped. The nav row is **not** in here — it is the pinned panel above,
        // so scrolling a long note list never carries the way out off the screen with it.
        //
        // The area is taken *before* the closure so it carries guard 1's forced offset without
        // holding a borrow of `band` across the frame the destinations are drawn in.
        let area = self.band.scroll_area();
        let mut reset_requested = false;
        let out = area.show(ui, |ui| {
            ui.add_space(4.0);

            match self.dest {
                Destination::Review => {
                    // The review screen's edit entrance, offered only on a **revealed** card
                    // (ADR-0029 §1) — so nothing needs flipping on the way through here, and
                    // ADR-0021 §6's "counts as a reveal" is retired along with the pre-reveal
                    // control it existed for. The entrance is shared by the leech screen, which
                    // also hangs off Review (ADR-0010 §7) and has no reveal to spend.
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
                        &mut self.moving,
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
                        &mut self.inbound,
                        &mut self.file_list,
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
/// fixes. **Where it sits is settled** — pinned above the scroll area, yielding to the soft keyboard
/// (see the call site) — which is the *layout pass*'s half; how it **looks** is the *finish pass*'s,
/// and a row of buttons is the honest floor.
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

// --- small rendering helpers, every one through the bidi layout so no screen holds a bare label ---

fn now_ms() -> i64 {
    // The one clock read on the review path — an edge value, never reached from `cairn-core`
    // (ADR-0009 §8). A clock before the epoch is not a real handset state; clamp rather than wrap.
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub(crate) fn text(ui: &egui::Ui, s: &str) -> egui::text::LayoutJob {
    bidi::job(
        s,
        egui::TextStyle::Button.resolve(ui.style()),
        ui.visuals().text_color(),
    )
}

pub(crate) fn heading(ui: &mut egui::Ui, s: &str) {
    ui.label(bidi::job(
        s,
        egui::TextStyle::Heading.resolve(ui.style()),
        ui.visuals().text_color(),
    ));
}

pub(crate) fn body(ui: &mut egui::Ui, s: &str) {
    ui.label(bidi::job(
        s,
        egui::TextStyle::Body.resolve(ui.style()),
        ui.visuals().text_color(),
    ));
}

/// What the **box badge** reads: the durability box, or `new` for a card with **no review history**
/// (ADR-0006 §6).
///
/// The `new` case is not a nicety, and it is the one every call site is liable to drop, because
/// [`box_of`](cairn_core::scheduling::box_of) is total and answers `1` for a card it has never seen.
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
pub(crate) fn box_badge_wording(reviewed: bool, box_: u8) -> String {
    if reviewed {
        format!("Box {box_}")
    } else {
        "new".to_string()
    }
}

/// A small, non-interactive, weak-coloured footnote carrying bidi-laid text.
pub(crate) fn badge(ui: &mut egui::Ui, s: &str) {
    ui.label(bidi::job(
        s,
        egui::TextStyle::Small.resolve(ui.style()),
        ui.visuals().weak_text_color(),
    ));
}

/// A full-width button carrying bidi-laid text.
pub(crate) fn full_width_button(ui: &mut egui::Ui, s: &str) -> egui::Response {
    let job = text(ui, s);
    ui.add_sized([ui.available_width(), 36.0], egui::Button::new(job))
}

/// The card face — a wide, tall clickable surface. Tapping the prompt reveals; the answer face is
/// drawn the same way for visual consistency, its click ignored.
pub(crate) fn card_face(ui: &mut egui::Ui, s: &str) -> egui::Response {
    // Card content is the one surface that renders the restricted Markdown subset (ADR-0002 §8):
    // `**bold**` in the shipped face, never a literal `**` (issue #104).
    let job = bidi::markdown_job(
        s,
        egui::TextStyle::Button.resolve(ui.style()),
        ui.visuals().text_color(),
    );
    ui.add_sized([ui.available_width(), 96.0], egui::Button::new(job))
}

/// A small label for a field or control — a form-pane caption, weaker than body text.
pub(crate) fn field_label(ui: &mut egui::Ui, s: &str) {
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
pub(crate) fn text_field(ui: &mut egui::Ui, buffer: &mut String) -> egui::Response {
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
pub(crate) fn raise_keyboard(ctx: &egui::Context, response: &egui::Response) {
    if response.clicked() && crate::platform::insets().keyboard.is_down() {
        ctx.send_viewport_cmd(egui::ViewportCommand::IMEAllowed(true));
    }
}

/// The bidi text-edit layouter shared by every field (`text_field`, `multiline_field_output`): it
/// runs the field's contents through the `bidi` helper, left-aligned within the edit, so untrusted
/// mixed-script text lays out correctly wherever it is typed.
pub(crate) fn bidi_layouter(
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
    fn a_card_with_no_review_history_badges_new_not_box_one() {
        // ADR-0006 §6. `box_of` is total and answers 1 for a never-reviewed card, so the badge has to
        // ask whether there is any history at all — otherwise a first introduction claims a durability
        // nothing measured, and reads as the bottom of a queue of boxes (ADR-0001 §3).
        assert_eq!(box_badge_wording(false, 1), "new");
        assert_eq!(box_badge_wording(true, 1), "Box 1");
        assert_eq!(box_badge_wording(true, 4), "Box 4");
    }
}
