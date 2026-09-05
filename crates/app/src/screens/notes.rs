//! The **Notes** destination: the note list and deck controls, and the editor pane — its form and
//! card bodies, the pane toggle, the cloze field and blank selection, and the warning banner.

use cairn_core::content::{DeckId, NoteId};
use cairn_store::Collection;

use eframe::egui::{Align, Layout, vec2};

use crate::notes::{self, Filter};
use crate::{
    Editing, badge, bidi, bidi_layouter, body, box_badge_wording, cards, compact_button, editor,
    field_label, frame, full_width_button, heading, raise_keyboard, sync, text, text_field,
};
use crate::{spacing, surface};

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
    moving: &mut Option<NoteId>,
) {
    // **The two surfaces take different frames, and this is the only screen in the app that does.**
    // The note list is a column like every other destination (#131); the editor earns a wider one by
    // putting two real columns in it, which is the thing ADR-0012 §1 always said the wide layout was
    // and the implementation never did.
    if editing.is_some() {
        editor_screen(ui, coll, editing);
        return;
    }
    frame::column(ui, |ui| {
        note_list(ui, coll, editing, search, deck_filter, new_deck, moving);
    });
}

/// The editor inside its own frame. **Whether the panes sit side by side is decided here, against
/// the window, and handed down** — never re-derived from `ui.available_width()` further in, which
/// under the page frame is the *column's* width and would put every desktop into the phone's toggle
/// (`frame::TWO_COLUMN_MIN_WIDTH`).
fn editor_screen(ui: &mut egui::Ui, coll: &mut Collection, editing: &mut Option<Editing>) {
    // `viewport_rect`, not `available_width` and not `content_rect`. The width the whole window has
    // is the thing this decision is about; `available_width` is the frame's column, which is the
    // trap, and the safe-area insets `content_rect` subtracts are vertical on every device that has
    // them — so taking them off would make an arrangement decision out of a notch.
    let window = ui.ctx().viewport_rect().width();
    let two_column = window >= frame::TWO_COLUMN_MIN_WIDTH;
    let cap = frame::cap_for(true, window);
    frame::wide_column(ui, cap, |ui| {
        // Full width is a target on a phone and a distance on a desktop: at 1120 it drew a *Done*
        // wider than the two columns of content beneath it. Same 36px height either way — the map
        // holds hit targets to touch, so only the stretching goes.
        let done = if two_column {
            compact_button(ui, "Done")
        } else {
            full_width_button(ui, "Done")
        };
        if done.clicked() {
            // **Settle before letting go of the buffers.** On the frame *Done* is clicked the panes
            // below are never drawn, so the field the user is inside never produces the response its
            // autosave is read from (ADR-0021 §7) — and clearing `editing` then throws the edit away
            // with the buffer. That lost the last field typed, silently, on both arrangements.
            if let Some(ed) = editing {
                editor::settle_all(coll, ed.note, &ed.kind, &ed.fields, ed.deck);
            }
            *editing = None;
            return;
        }
        // PROTOTYPE #163: the two readouts. The width one turns the window edge into a knob — drag
        // and read the pane width off it; the fill one is a knob proper. Both are here rather than on
        // a debug screen because a knob you have to leave the screen to reach is not a knob.
        width_readout(ui, cap);
        field_fill_knob(ui);
        ui.add_space(spacing::gap(2));
        if let Some(ed) = editing {
            editor_pane(ui, coll, ed, two_column);
        }
    });
}

/// PROTOTYPE #163. The width readout — **window, frame and each pane**, so a person dragging the
/// window edge can say where two columns stop working *in numbers* rather than in gestures.
///
/// The knob here is the window itself, which is why there is no slider: the variable being judged is
/// the thing the user is already holding. What was missing was only the readout.
///
/// **What it found is that there is nothing to find.** Dragged down by hand, two columns held at
/// 398px per pane (an 880 window), then 151, then **118** — a third of the narrowest case the ticket
/// had argued about — with the card face wrapping to four lines and staying perfectly readable.
/// Narrowness is a *gradient* here, not a failure, and a gradient has no threshold in it. So the
/// answer was to delete `TWO_COLUMN_MIN_WIDTH` rather than move it, and to fold the panes on the
/// platform's soft keyboard, which is the axis ADR-0025 §4 always said the toggle was about.
fn width_readout(ui: &mut egui::Ui, cap: f32) {
    let window = ui.ctx().viewport_rect().width();
    let frame_width = cap.min(window - frame::PAGE_MARGIN * 2.0);
    let pane = (frame_width - frame::PANE_GUTTER) / 2.0;
    badge(
        ui,
        &format!("window {window:.0}  ·  frame {frame_width:.0}  ·  each pane {pane:.0}"),
    );
}

/// PROTOTYPE #163. The field-fill knob: one slider and a readout, on the one screen that draws a
/// card and a text field side by side.
///
/// **A knob rather than a menu**, which is the map's standing lesson from #141 and #155: the question
/// is a *distance* — how far a field's material has to move before it stops reading as the same thing
/// as the card — and a menu of three candidate rungs answers a question nobody asked. #155's ink knob
/// stopped six of 255 away from a role that already existed, which a menu would have hidden.
///
/// **It moves the field and holds the card still**, deliberately. The card's fill is ADR-0033's
/// decided well and #125 banked a result on it; the field's fill has no argument behind it at all.
///
/// The readout carries the numbers so the screenshot does not have to. **field : card** is the one
/// that matters and it starts at 1.000:1; the other two are there so a move that buys separation by
/// spending the card's own well against the page is visible while it is being made.
///
/// **What it found**: both themes stopped at **0.55** and gave different answers. Dark landed on
/// `#15191b`, one unit per channel off the middle of the ramp's only double step — the `STONE_1` the
/// ramp had numbered and never filled, so #143's recorded cost of *"a rung the dark ramp had none to
/// spare of"* was not paid. Light landed on `#d2d6d7`, four of 255 from `STONE_L_EDGE`, and minted
/// its own rather than reusing a rung that means *pressed widget*.
fn field_fill_knob(ui: &mut egui::Ui) {
    let mut t = crate::theme::field_knob();
    let card = crate::theme::card_fill(ui.visuals());
    let field = crate::theme::field_fill(ui.visuals());
    let page = ui.visuals().panel_fill;
    let hex = |c: egui::Color32| format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b());
    if ui
        .add(egui::Slider::new(&mut t, 0.0..=1.0).text("field fill → page"))
        .changed()
    {
        crate::theme::set_field_knob(t);
    }
    badge(
        ui,
        &format!(
            "knob {t:.3}   field {}  card {}  page {}   ·   field:card {:.3}:1   field:page {:.3}:1   card:page {:.3}:1",
            hex(field),
            hex(card),
            hex(page),
            crate::theme::contrast(field, card),
            crate::theme::contrast(field, page),
            crate::theme::contrast(card, page),
        ),
    );
}

/// The note list: create, the deck controls, the text search, and the rows.
#[allow(clippy::too_many_arguments)]
fn note_list(
    ui: &mut egui::Ui,
    coll: &mut Collection,
    editing: &mut Option<Editing>,
    search: &mut String,
    deck_filter: &mut Option<DeckId>,
    new_deck: &mut String,
    moving: &mut Option<NoteId>,
) {
    heading(ui, "Notes");
    ui.add_space(spacing::gap(2));

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
        ui.add_space(spacing::gap(2));
        body(ui, notes::EMPTY_STATE);
        return;
    }

    // The deck filter and the deck authoring surface (ADR-0021 §9): decks are **created where they
    // are filtered**, so the filter dropdown, *new deck*, and the delete of the filtered deck all sit
    // together here. Deletion is ADR-0005 §7's flag, deriving through to the deck's notes.
    ui.add_space(spacing::gap(2));
    deck_controls(ui, coll, deck_filter, new_deck);

    // Text search — the load-bearing filter (ADR-0021 §2), a plain substring over field values,
    // composing with the deck filter above (deck ∩ text; the tag filter shares the vocabulary and is
    // set on notes but has no dedicated control yet).
    ui.add_space(spacing::gap(2));
    field_label(ui, "Search");
    text_field(ui, search);
    let filter = Filter {
        deck: deck_filter.map(|d| d.to_canonical()),
        text: (!search.trim().is_empty()).then(|| search.trim().to_owned()),
        ..Filter::default()
    };

    ui.add_space(spacing::gap(2));
    let rows = notes::list(coll, &filter).unwrap_or_default();
    if rows.is_empty() {
        // A move whose target view is now empty has no gaps to offer; drop it rather than strand it.
        *moving = None;
        body(ui, "No notes match.");
        return;
    }

    // A move stands only while its note is on screen: placement is *between visible neighbours*
    // (ADR-0021 §4), so a filter change that hides the moving note leaves nothing to name or place
    // against, and the mode is cancelled rather than carried in a state the user cannot see.
    if let Some(mid) = *moving
        && !rows.iter().any(|r| r.id == mid)
    {
        *moving = None;
    }

    // In the placement state every gap between the visible rows is a one-tap target (ADR-0021 §4);
    // otherwise the list offers each row's open, move and delete.
    if let Some(mid) = *moving {
        placement_list(ui, coll, moving, &rows, mid);
        return;
    }

    // The list's own sequence is the rendering of `position` order — there is no sort control and the
    // key is never shown (ADR-0021 §4). No row carries schedule information (ADR-0021 §2).
    let mut open: Option<NoteId> = None;
    let mut delete: Option<NoteId> = None;
    let mut start_move: Option<NoteId> = None;
    for (i, row) in rows.iter().enumerate() {
        // **The gap between rows is stated, one unit, and it is the same unit that separates the
        // controls *within* a row.** Before ADR-0032 these rows leaned on egui's ambient 3px and were
        // fused the moment it went to zero. Stating it equal in both axes is deliberate rather than
        // convenient: a row is a group of three controls, and a vertical gap smaller than the
        // horizontal one would make a *column* of Deletes read as a group before the row does.
        if i > 0 {
            ui.add_space(spacing::gap(1));
        }
        spacing::row(ui, 1, |ui| {
            if ui.button(text(ui, row.preview())).clicked() {
                open = Some(row.id);
            }
            // **Move** enters the two-tap placement state (ADR-0021 §4): a tap here, then a tap on a
            // gap. No drag and no long-press — the two taps behave identically under touch and mouse,
            // which is the finding ADR-0006 §5 recorded and this must not break.
            if ui.button(text(ui, "Move")).clicked() {
                start_move = Some(row.id);
            }
            if ui.button(text(ui, "Delete")).clicked() {
                delete = Some(row.id);
            }
        });
    }
    if let Some(id) = start_move {
        *moving = Some(id);
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

/// The note list in its **placement state** (ADR-0021 §4). After **Move**, the moving note is named
/// and every gap between the *other* visible rows becomes a one-tap **Place here** target; **Cancel**
/// leaves the order untouched. There is no drag, no long-press and no auto-scroll — two taps that
/// behave identically under touch and mouse (ADR-0006 §5). Placing writes **exactly one** `position`
/// value ([`notes::place_between`], ADR-0021 §3), and because the neighbours are the two *visible*
/// notes flanking the gap, a hidden note between them keeps its place (ADR-0021 §4).
fn placement_list(
    ui: &mut egui::Ui,
    coll: &mut Collection,
    moving: &mut Option<NoteId>,
    rows: &[notes::NoteRow],
    mid: NoteId,
) {
    if full_width_button(ui, "Cancel move").clicked() {
        *moving = None;
        return;
    }
    ui.add_space(spacing::gap(2));
    let name = rows
        .iter()
        .find(|r| r.id == mid)
        .map_or("", |r| r.preview());
    field_label(ui, &format!("Placing: {name}"));
    ui.add_space(spacing::gap(2));

    // The gaps run among the *other* visible rows — the moving note removed so it is never offered a
    // place beside itself. Gap `i` sits before `visible[i]`, and the last gap (past the final row) is
    // the end; the open ends send the note to either extreme (ADR-0021 §4).
    let visible: Vec<&notes::NoteRow> = rows.iter().filter(|r| r.id != mid).collect();
    let mut place: Option<usize> = None;
    for gap in 0..=visible.len() {
        if full_width_button(ui, "Place here").clicked() {
            place = Some(gap);
        }
        if let Some(row) = visible.get(gap) {
            body(ui, row.preview());
        }
    }
    if let Some(gap) = place {
        // One write, and the mode ends. A failed write drops the state too: there is no half-move to
        // recover, and the list is re-read from the surface next frame regardless.
        let ids: Vec<NoteId> = visible.iter().map(|r| r.id).collect();
        let _ = notes::place_between(coll, mid, &ids, gap);
        *moving = None;
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

    spacing::row(ui, 1, |ui| {
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

    spacing::row(ui, 1, |ui| {
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
fn editor_pane(ui: &mut egui::Ui, coll: &mut Collection, ed: &mut Editing, two_column: bool) {
    heading(
        ui,
        if ed.note.is_some() {
            "Edit note"
        } else {
            "New note"
        },
    );
    ui.add_space(spacing::gap(2));

    // The card pane and the ambient destructive-edit warning are recomputed from current content
    // every frame (ADR-0012 §5): dormancy holds no before-state, so there is no "just became dormant"
    // and nothing to auto-scroll to (ADR-0018 §4). A draft not yet born has no stored note, so no
    // cards and no history — its pane is empty until its first field commits (ADR-0021 §7).
    let mut pane = ed.note.and_then(|id| cards::card_pane(coll, id).ok());

    // On a phone the two bodies are a `Write | Cards` toggle (ADR-0012 §1); where both fit they show
    // together (ADR-0025 §5). **Where both fit, they now sit side by side** — which is what ADR-0012
    // §1 has always described and what the implementation never did: the old wide layout stacked
    // them vertically with a rule between, so its `640` was a *width* test gating a decision about
    // *vertical* room. #131 made that latent oddity load-bearing, because a page frame changes what
    // "the width" means, and #131's answer is this: the width test now decides the thing it names.
    //
    // `two_column` arrives from `editor_screen`, measured against the **window**. Re-deriving it from
    // `ui.available_width()` here is the defect the whole change exists to remove.
    if two_column {
        // **The header travels with the form**, not across the top. Kind, deck and *new deck* are
        // properties of the note being written, so side by side the left column reads as *the note*
        // and the right as *its cards* — whereas a header stretched over both columns makes a 1050px
        // text field out of *new deck* and belongs to neither.
        let gutter = frame::PANE_GUTTER;
        let each = ((ui.available_width() - gutter) / 2.0).max(1.0);
        ui.horizontal_top(|ui| {
            pane_column(ui, each, |ui| {
                editor_header(ui, coll, ed);
                editor_form_body(ui, coll, ed, pane.as_ref());
            });
            ui.add_space(gutter);
            pane_column(ui, each, |ui| editor_cards_body(ui, pane.as_ref()));
        });
        return;
    }

    editor_header(ui, coll, ed);
    // Switching to *Cards* stops drawing the form on the same frame, so the field being typed in
    // never sees its blur — the same loss *Done* had, and worse in kind: the pane the tap asked
    // *"what will I be asked"* of would answer with a card missing the half just written.
    if pane_toggle(ui, &mut ed.show_cards) && ed.show_cards {
        ed.note = editor::settle_all(coll, ed.note, &ed.kind, &ed.fields, ed.deck);
        // Re-read the pane after settling, not on the next frame. It was computed above from the
        // content the store held *before* this tap, and egui repaints on demand — so leaving it
        // stale draws the card the tap was asking about one edit behind, until something unrelated
        // asks for another frame.
        pane = ed.note.and_then(|id| cards::card_pane(coll, id).ok());
    }
    ui.add_space(spacing::gap(2));
    if ed.show_cards {
        editor_cards_body(ui, pane.as_ref());
    } else {
        editor_form_body(ui, coll, ed, pane.as_ref());
    }
}

/// The editor's header: the Android input caveat, then the kind and deck dropdowns. Above the fields
/// in one column, and at the top of the **form** column when there are two.
fn editor_header(ui: &mut egui::Ui, coll: &mut Collection, ed: &mut Editing) {
    // The standing quiet line stating the Android text-input limitation *in advance* (ADR-0015 §9):
    // the failure is silence — composed non-Latin text never reaches the app — so it can only be
    // told, never detected. Off the one sanctioned `cfg(target_os)` capability constant (ADR-0015
    // §15), so on desktop this compiles to nothing rather than a statement about a limitation the
    // reader does not have.
    if sync::LATIN_INPUT_ONLY {
        field_label(ui, sync::DESKTOP_AUTHORING_LINE);
        ui.add_space(spacing::gap(2));
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

    ui.add_space(spacing::gap(2));

    // The deck dropdown, beside the kind one (ADR-0021 §9): the note's one deck (ADR-0005 §2), with
    // *create a new deck* right here — the moment you need a deck that does not exist is while filing
    // the note that wants it. On a draft the choice is held until the note is born, then written once.
    editor_deck_dropdown(ui, coll, ed);

    ui.add_space(spacing::gap(2));
}

/// One of the editor's two side-by-side panes: a fixed-width, top-down child so the pane's own
/// full-width controls size to the **pane** rather than to the row they sit in.
fn pane_column(ui: &mut egui::Ui, width: f32, add: impl FnOnce(&mut egui::Ui)) {
    ui.allocate_ui_with_layout(
        vec2(width, ui.available_height()),
        Layout::top_down(Align::Min),
        |ui| {
            ui.set_width(width);
            add(ui);
        },
    );
}

/// The phone's `Write | Cards` pane toggle (ADR-0012 §1): two mutually exclusive choices, the current
/// one marked. Which pane is showing is the only thing it changes — there is no third state.
///
/// Returns whether this tap **moved** it. The caller needs the transition rather than the state,
/// because leaving *Write* is what strands the edit in the field being typed in, and re-settling on
/// every frame spent on the *Cards* tab would ask the store the same question forever.
fn pane_toggle(ui: &mut egui::Ui, show_cards: &mut bool) -> bool {
    let was = *show_cards;
    spacing::row(ui, 1, |ui| {
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
    *show_cards != was
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
        ui.add_space(spacing::gap(2));
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
        // Through `settle_all` rather than its own loop, which is where this fix started: the chord
        // was the one exit that already committed its buffers, and it did so **without** the filing
        // line above it — so a note born by the chord under an active deck filter landed unfiled,
        // where the same note born by a blur landed filed.
        // ADR-0021 §8 carries the *kind* forward and says nothing about the deck, so the fresh draft
        // is left unfiled exactly as before — that is a design question for #163, not a defect.
        // The id it returns is deliberately dropped: the draft replacing `ed` is a *different* note,
        // so carrying the settled one forward is what would be wrong here.
        editor::settle_all(coll, note, &kind, &ed.fields, ed.deck);
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
    ui.add_space(spacing::gap(1));
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
                // **The same card the review screen draws** (ADR-0012 §1) — one object with two
                // faces, the badge in its own corner — which is now literally true rather than
                // approximately: both come through `surface::card`, so the material cannot drift
                // between the two screens the way the old pair of `card_face` calls could.
                //
                // The *height* is the one thing that differs, and it is the caller's: a list of
                // four cards at the review card's floor would be 1,200px of mostly nothing.
                surface::card(
                    ui,
                    &card.prompt,
                    (!card.answer.is_empty()).then_some(card.answer.as_str()),
                    Some(&box_badge_wording(card.reviews > 0, card.box_)),
                    surface::FIT,
                    // Fully open: the card pane shows a card, not a card being turned over. The
                    // reveal is the review screen's event and belongs to it (ADR-0037 §3).
                    1.0,
                );
                ui.add_space(spacing::gap(2));
            }
            // A dormant entry is a single line — its name, *dormant*, its kept history (ADR-0018 §2).
            cards::Entry::Dormant(dormant) => badge(ui, &dormant.history()),
        }
    }
    if pane.state == cards::State::NoLiveCards {
        ui.add_space(spacing::gap(2));
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
        // PROTOTYPE #163: the field-fill knob, on the multiline field too — `cloze`'s Text is the
        // largest field in the app and the one where a fill has the most area to be judged on.
        .background_color(crate::theme::field_fill(ui.visuals()))
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Every string a frame actually drew — the galley text inside its shapes, walked recursively —
    /// so a control's presence is **asserted** from what the user sees, not from the branch it sits in.
    fn drawn_text(out: &egui::FullOutput) -> String {
        fn walk(shape: &egui::Shape, into: &mut String) {
            match shape {
                egui::Shape::Text(t) => {
                    into.push_str(t.galley.text());
                    into.push('\n');
                }
                egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, into)),
                _ => {}
            }
        }
        let mut text = String::new();
        for clipped in &out.shapes {
            walk(&clipped.shape, &mut text);
        }
        text
    }

    /// **ADR-0021 §4, and the half that fails in silence.** *Move* is present on a row, and taking it
    /// turns the list into the placement state: the moving note is named, the sort control is *not*
    /// reintroduced, and gap targets appear. Nothing else exercises the state, so a regression that
    /// dropped the entrance or never swapped in the gaps would pass every other test.
    #[test]
    fn move_opens_a_two_tap_placement_state_with_gap_targets() {
        let data = TempDir::new().unwrap();
        let state = TempDir::new().unwrap();
        let mut coll = Collection::open(data.path(), state.path()).unwrap();
        let first = coll.create_note("basic", &[("Front", "alpha")]).unwrap();
        coll.create_note("basic", &[("Front", "beta")]).unwrap();

        let ctx = egui::Context::default();
        let mut editing = None;
        let mut search = String::new();
        let mut deck_filter = None;
        let mut new_deck = String::new();

        let mut frame = |moving: &mut Option<NoteId>, coll: &mut Collection| {
            ctx.run_ui(Default::default(), |ui| {
                notes_screen(
                    ui,
                    coll,
                    &mut editing,
                    &mut search,
                    &mut deck_filter,
                    &mut new_deck,
                    moving,
                );
            })
        };

        // Normal mode: the row offers *Move*, and there is no placement control yet.
        let mut moving = None;
        let listed = drawn_text(&frame(&mut moving, &mut coll));
        assert!(
            listed.contains("Move"),
            "a row offers the reorder entrance — drew: {listed}"
        );
        assert!(
            !listed.contains("Place here"),
            "the gaps only appear once a move is under way — drew: {listed}"
        );

        // Tapping *Move* on the first row is modelled by the state it sets; the next frame is the
        // placement list.
        moving = Some(first);
        let placing = drawn_text(&frame(&mut moving, &mut coll));
        assert!(
            placing.contains("Placing: alpha"),
            "the moving note is named — drew: {placing}"
        );
        assert!(
            placing.contains("Cancel move"),
            "cancel leaves the order untouched (ADR-0021 §4) — drew: {placing}"
        );
        assert!(
            placing.contains("Place here"),
            "every gap between the visible rows is a one-tap target — drew: {placing}"
        );
    }

    /// A move whose note the filter hides is dropped: placement is *between visible neighbours*
    /// (ADR-0021 §4), so once the note leaves the view there is nothing to place it against and the
    /// state must not survive into a frame the user cannot see.
    #[test]
    fn a_move_is_dropped_when_its_note_leaves_the_filtered_view() {
        let data = TempDir::new().unwrap();
        let state = TempDir::new().unwrap();
        let mut coll = Collection::open(data.path(), state.path()).unwrap();
        let hidden = coll.create_note("basic", &[("Front", "alpha")]).unwrap();
        coll.create_note("basic", &[("Front", "beta")]).unwrap();

        let ctx = egui::Context::default();
        let mut editing = None;
        // A text filter that the moving note does not match — it cannot be placed against neighbours
        // the user cannot see, so the mode must not survive (ADR-0021 §4).
        let mut search = "beta".to_owned();
        let mut deck_filter = None;
        let mut new_deck = String::new();
        let mut moving = Some(hidden);

        let _ = ctx.run_ui(Default::default(), |ui| {
            notes_screen(
                ui,
                &mut coll,
                &mut editing,
                &mut search,
                &mut deck_filter,
                &mut new_deck,
                &mut moving,
            );
        });

        assert_eq!(
            moving, None,
            "a move whose note the filter hides is cancelled"
        );
    }
}
