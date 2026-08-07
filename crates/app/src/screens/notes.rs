//! The **Notes** destination: the note list and deck controls, and the editor pane — its form and
//! card bodies, the pane toggle, the cloze field and blank selection, and the warning banner.

use cairn_core::content::{DeckId, NoteId};
use cairn_store::Collection;

use crate::notes::{self, Filter};
use crate::{
    Editing, badge, bidi, bidi_layouter, body, box_badge_wording, card_face, cards, editor,
    field_label, full_width_button, heading, raise_keyboard, sync, text, text_field,
};

/// The **Notes** destination (ADR-0021 §2): the browse surface and the app's authoring home. Shows
/// the editor when one is open, otherwise the note list — create, the text search, and the rows,
/// each row offering **edit and delete** (never suspend, which is the leech screen's, ADR-0021 §2).
pub(crate) fn notes_screen(
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
        None => cairn_core::content::next_blank_number(buffer),
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
