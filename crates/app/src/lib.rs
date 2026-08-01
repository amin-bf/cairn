//! The egui application: every screen, and both entry points.
//!
//! **This crate deliberately has no `src/main.rs`.** `cargo-apk` panics after signing
//! (`Bin is not compatible with Cdylib`) when one crate has both a cdylib and a bin — the APK comes
//! out correct but the exit code does not, and CI breaks. The desktop binary is `leitner-desktop`,
//! which is a shim with no logic (ADR-0003 §5, ADR-0009 §3).
//!
//! See `CONTEXT.md` beside this file, [ADR-0003](../../../docs/adr/0003-client-stack.md) and
//! [ADR-0006](../../../docs/adr/0006-the-review-session-experience.md).

pub mod bidi;
pub mod deck;
pub mod editor;
pub mod fonts;
pub mod notes;
pub mod session;

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use leitner_core::content::{DeckId, NoteId};
use leitner_core::log::{DayScale, day_number};
use leitner_core::replay::replay;
use leitner_core::scheduling::Grade;
use leitner_store::Collection;

use notes::Filter;
use session::{Offered, ReviewState};

/// Re-exported so `leitner-desktop` needs no `eframe` dependency of its own — it cannot then
/// resolve a different feature set from the one this crate was built with, and it has no route to
/// grow real code unnoticed.
pub use eframe;

/// A sitting of review, held **only in memory** (ADR-0006 §6, issue #94): its position is never
/// stored, so a force-quit loses nothing — relaunch re-derives the queue from the log and every
/// already-graded card is simply no longer due. The chosen cards are snapshotted at the start; the
/// index walks them; grading appends a row and advances.
struct Sitting {
    cards: Vec<Offered>,
    index: usize,
    revealed: bool,
    /// When the sitting began — the quiet 10-minute timer runs from here (issue #94).
    started: Instant,
    /// When the current card came on screen, so the row can record how long the answer took
    /// (ADR-0004 §5).
    card_shown: Instant,
    /// Set once the user answers the 10-minute checkpoint's "keep going", so it does not nag again.
    checkpoint_dismissed: bool,
}

impl Sitting {
    fn new(cards: Vec<Offered>) -> Self {
        let now = Instant::now();
        Sitting {
            cards,
            index: 0,
            revealed: false,
            started: now,
            card_shown: now,
            checkpoint_dismissed: false,
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
    /// The note list's deck filter, held across frames — one of ADR-0005 §6's three composable
    /// filters. `None` narrows by no deck (every note, filed or not).
    deck_filter: Option<DeckId>,
    /// The note list's *new deck* name buffer: decks are created where they are filtered (ADR-0021
    /// §9), so the create control sits beside the deck filter.
    new_deck: String,
    /// Cleared until the shipped font set is installed. The install happens on the **first frame**,
    /// not in `CreationContext` — see `fonts` and the note on `new`.
    fonts_installed: bool,
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
            deck_filter: None,
            new_deck: String::new(),
            fonts_installed: false,
        }
    }

    /// Open the collection under the platform's two directories (ADR-0007 §6) and, on a first launch,
    /// seed one `basic` note so the walking skeleton has a card to review — issue #94's opening line.
    fn open_store() -> Result<Collection, String> {
        let data = leitner_store::platform::data_dir().map_err(|e| e.to_string())?;
        let state = leitner_store::platform::state_dir().map_err(|e| e.to_string())?;
        let mut coll = Collection::open(&data, &state).map_err(|e| e.to_string())?;
        if coll.is_empty().map_err(|e| e.to_string())? {
            coll.create_note("basic", &[("Front", "chien"), ("Back", "dog")])
                .map_err(|e| e.to_string())?;
        }
        Ok(coll)
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

        // The persistent affordance that makes all three destinations reachable (ADR-0021 §1): a
        // destination reachable only by completing a session is not reachable, so the nav row is drawn
        // every frame, above whatever the current destination shows.
        nav_bar(ui, &mut self.dest);
        ui.separator();
        ui.add_space(4.0);

        match self.dest {
            Destination::Review => {
                // Opening the editor from the review screen counts as a reveal (ADR-0021 §6): the
                // request carries the note, and `review` has already flipped the card face over.
                if let Some(note) = review(ui, coll, &mut self.sitting, now_ms, today) {
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
            Destination::Settings => settings_screen(ui),
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
    now_ms: i64,
    today: i64,
) -> Option<NoteId> {
    // Everything on screen is derived from the log this frame — there is no cached session state to
    // fall out of step with it.
    let current = deck::current_cards(coll).unwrap_or_default();
    let lines = coll.log_lines().unwrap_or_default();
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    let replayed = replay(&current, &refs);
    let queue = session::compose(&current, &replayed, today);
    let total = current.len();

    heading(ui, "Review");
    ui.add_space(8.0);

    if sitting.is_none() {
        if let Some(count) = picker(ui, &queue, total) {
            *sitting = Some(Sitting::new(queue.sitting(count)));
        }
        return None;
    }

    // A running sitting: keep the frame ticking so the 10-minute checkpoint can surface without an
    // input event (immediate mode has nowhere to wait — client-stack rule 4).
    ui.ctx().request_repaint_after(Duration::from_secs(1));

    let mut end_sitting = false;
    let mut edit_request: Option<NoteId> = None;
    {
        let s = sitting.as_mut().expect("just checked it is Some");

        if s.checkpoint_due() {
            body(ui, "You've been reviewing for 10 minutes.");
            ui.add_space(8.0);
            if full_width_button(ui, "Finish here").clicked() {
                end_sitting = true;
            }
            if full_width_button(ui, "Keep going").clicked() {
                s.checkpoint_dismissed = true;
            }
        } else if let Some(offered) = s.cards.get(s.index).copied() {
            match deck::render(coll, offered.card).ok().flatten() {
                // A card that no longer renders (its note went dormant mid-sitting) is skipped.
                None => {
                    s.index += 1;
                    s.revealed = false;
                    s.card_shown = Instant::now();
                }
                Some(rendered) => {
                    let progress = format!("{} of {}", s.index + 1, s.cards.len());
                    body(ui, &progress);
                    ui.add_space(8.0);

                    // Reveal is tap-the-card: the prompt is one wide button, and clicking it shows
                    // the back. Identical by touch and by mouse — egui does not distinguish them.
                    if card_face(ui, &rendered.prompt).clicked() {
                        s.revealed = true;
                    }

                    // Edit this note, at any point in the card's life (ADR-0021 §6): the honest
                    // diagnosis of most leeches is a defective card, and the moment to fix it is when
                    // it is in front of you, not twenty cards later. Opening the editor **counts as a
                    // reveal** — the editor shows the back, so without flipping the card here
                    // ADR-0006 §4's "no grading before the answer is seen" would be quietly false. An
                    // edit that makes the card dormant needs no mechanism: the next frame re-derives
                    // the queue and simply does not offer it (ADR-0021 §6).
                    ui.add_space(4.0);
                    if full_width_button(ui, "Edit note").clicked() {
                        s.revealed = true;
                        edit_request = Some(offered.card.note);
                    }

                    if s.revealed {
                        ui.add_space(4.0);
                        card_face(ui, &rendered.answer);

                        // The box badge appears only after reveal, is non-interactive, and reports
                        // durability — never a queue (scheduling `CONTEXT.md`).
                        ui.add_space(4.0);
                        badge(ui, &format!("Box {}", offered.box_));

                        ui.add_space(12.0);
                        if let Some(grade) = grade_buttons(ui, &offered, today) {
                            let duration_ms = s.card_shown.elapsed().as_millis() as u64;
                            // A failed append drops this one review rather than wedging the
                            // session: the card advances, and the next frame re-derives the queue
                            // from whatever did commit. Surfacing write errors is a later ticket.
                            let _ = coll.append_review(
                                offered.card,
                                grade,
                                now_ms,
                                DayScale::default(),
                                duration_ms,
                            );
                            s.index += 1;
                            s.revealed = false;
                            s.card_shown = Instant::now();
                        }
                    }
                }
            }
            // Reaching the chosen count ends the sitting (issue #94).
            if s.index >= s.cards.len() {
                end_sitting = true;
            }
        } else {
            end_sitting = true;
        }
    }

    if end_sitting {
        *sitting = None;
    }
    edit_request
}

/// The count picker and the explicit worded states (issue #94). Returns the chosen sitting size when
/// the user starts one.
fn picker(ui: &mut egui::Ui, queue: &session::Queue, total: usize) -> Option<usize> {
    let available = queue.available();
    match ReviewState::of(queue, total) {
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
    text_field(ui, search, false);
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
        text_field(ui, new_deck, false);
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
        text_field(ui, &mut ed.new_deck, false);
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

/// The editor's **form pane** (ADR-0012 §1, §2): the kind dropdown and the note's fields, each field
/// autosaved per blur (ADR-0021 §7). The card pane — *"what will I be asked"* — and the
/// destructive-edit warning above the fields are #83's, not here.
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
        // `cloze`'s Text field is the one multiline field — Enter inserts a newline there.
        let multiline = kind == "cloze" && name == "Text";
        field_label(ui, &name);
        let resp = {
            let buffer = &mut ed.fields[idx].1;
            text_field(ui, buffer, multiline)
        };
        // Enter keeps focus in a single-line field: egui treats it as submit-and-blur, so re-grab
        // focus and let nothing else happen (ADR-0012 §7).
        if !multiline && bare_enter && resp.lost_focus() {
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

/// The **Settings** destination (ADR-0021 §1): the home enrolment and sync settings hang off. Their
/// surfaces are their own tickets (#84, #85); this is the reachable door the shell owes them.
fn settings_screen(ui: &mut egui::Ui) {
    heading(ui, "Settings");
    ui.add_space(8.0);
    body(ui, "Sync and backup settings will live here.");
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
/// `multiline` is the one `cloze` Text field; every other field is single-line, so Enter is a submit
/// egui turns into a blur, which the caller re-grabs to keep Enter inert (ADR-0012 §7, ADR-0021 §8).
fn text_field(ui: &mut egui::Ui, buffer: &mut String, multiline: bool) -> egui::Response {
    let rtl = bidi::is_rtl(buffer);
    let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
        let mut job = bidi::job(
            text.as_str(),
            egui::TextStyle::Body.resolve(ui.style()),
            ui.visuals().text_color(),
        );
        job.halign = egui::Align::LEFT;
        job.wrap.max_width = wrap_width;
        ui.fonts_mut(|f| f.layout_job(job))
    };
    let field = if multiline {
        egui::TextEdit::multiline(buffer)
    } else {
        egui::TextEdit::singleline(buffer)
    };
    ui.add(
        field
            .desired_width(f32::INFINITY)
            .horizontal_align(if rtl {
                egui::Align::RIGHT
            } else {
                egui::Align::LEFT
            })
            .layouter(&mut layouter),
    )
}

/// Android entry point. `NativeActivity` hosts the app directly: the APK is this `.so` plus a
/// manifest, with no Java, no Kotlin and no Gradle project in the repository.
///
/// GameActivity was built and tested in #8 and reverted — it implements IME correctly, but winit's
/// Android backend never reads it, so non-Latin text input stays unavailable at any packaging cost.
/// Never design a feature that requires typing non-Latin text on Android (`AGENTS.md` rule 8).
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: android_activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid as _;

    let options = eframe::NativeOptions {
        android_app: Some(app.clone()),
        event_loop_builder: Some(Box::new(move |b| {
            b.with_android_app(app.clone());
        })),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "Leitner",
        options,
        Box::new(|cc| Ok(Box::new(LeitnerApp::new(cc)))),
    );
}
