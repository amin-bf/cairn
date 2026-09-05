//! The **Notes** destination: the note list and deck controls, and the editor pane — its form and
//! card bodies, the pane toggle, the cloze field and blank selection, and the warning banner.

use cairn_core::content::NoteId;
use cairn_store::Collection;

use eframe::egui::{Align, Layout, vec2};

use crate::notes::{self, DeckFilter, Filter};
use crate::{
    Editing, badge, bidi, bidi_layouter, body, box_badge_wording, cards, compact_button, controls,
    editor, field_label, fonts, frame, full_width_button, heading, raise_keyboard, sync, text,
    text_field,
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
    deck: &mut notes::DeckBar,
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
        note_list(ui, coll, editing, search, deck, moving);
    });
}

/// The editor's heading — *Edit note* or *New note*, drawn by the screen rather than by the pane
/// since #163 put *Done* at the foot of the page.
fn editor_heading(ui: &mut egui::Ui, ed: &Editing) {
    heading(
        ui,
        if ed.note.is_some() {
            "Edit note"
        } else {
            "New note"
        },
    );
}

/// The editor inside its own frame. **Whether the panes sit side by side is asked once and handed
/// down** — never re-derived further in, where the answer would be about the column rather than the
/// screen.
fn editor_screen(ui: &mut egui::Ui, coll: &mut Collection, editing: &mut Option<Editing>) {
    // **No width is measured here any more** (#163). This used to read `viewport_rect().width()` and
    // compare it against a 900px threshold, and the width was never what the question was about: the
    // toggle below exists because a soft keyboard eats *height*, and at 880 on a desktop there is no
    // keyboard and 450px going spare. Dragged down by hand to 118px per pane the two columns stayed
    // readable, so there is no width to find — `frame::editor_is_side_by_side` asks the platform.
    let two_column = frame::editor_is_side_by_side();
    let cap = frame::cap_for(true);
    frame::wide_column(ui, cap, |ui| {
        // **The heading leads the screen, because *Done* no longer does** (#163). It used to sit
        // above the heading, so the first thing on the editor was the way out of it.
        if let Some(ed) = editing.as_ref() {
            editor_heading(ui, ed);
            ui.add_space(spacing::gap(2));
        }
        if let Some(ed) = editing.as_mut() {
            editor_pane(ui, coll, ed, two_column);
        }

        // ***Done* sits on the reach line** — [ADR-0035 §1]'s **fourth** call site, and the first on
        // Notes. Three tickets inherited *apply §1 here* and none could, because in every
        // arrangement that kept *Done* at the top the screen's last control was the **Back field**,
        // and a form whose inputs float at the foot of the page is not an arrangement anyone wants.
        // So the rule had no target on this screen until the exit moved, which is why #163 judged
        // the placement and the anchoring as one question rather than two.
        //
        // **It also makes §1 say which of two things it means.** Every call site before this one
        // places something the reader is meant to press *next* — a grade cluster, the leech
        // entrance, *Back to review*. *Done* is what you press when you are finished. §1 is
        // therefore read as *the last control on the page* rather than *the way forward*, which is
        // a distinction it never had to make while it only ever had the first kind.
        //
        // The fallback arm needs no branch: `slack_above` returns the stated gap on a page with no
        // room left, so a long `cloze` note and a short window reach it by arithmetic. Measured at
        // 166px above the page bottom — §1's 165 plus the stroke — at 1280×800 and 560×860 alike.
        //
        // [ADR-0035 §1]: ../../../../docs/adr/0035-the-vertical-anchor.md
        ui.add_space(frame::slack_above(
            frame::page_room(ui),
            controls::HEIGHT,
            spacing::gap(2),
        ));
        // Full width is a target on a phone and a distance on a desktop: at 1120 it drew a *Done*
        // wider than the two columns of content beneath it. Same 36px height either way — the map
        // holds hit targets to touch, so only the stretching goes.
        let done = if two_column {
            compact_button(ui, "Done")
        } else {
            full_width_button(ui, "Done")
        };
        if done.clicked() {
            // **Settle before letting go of the buffers.** Clearing `editing` throws away the field
            // the user is still inside, which never produced the response its autosave is read from
            // (ADR-0021 §7). That lost the last field typed, silently, on both arrangements.
            if let Some(ed) = editing {
                editor::settle_all(coll, ed.note, &ed.kind, &ed.fields, ed.deck);
            }
            *editing = None;
        }
    });
}

/// The note list: create, the deck controls, the text search, and the rows.
#[allow(clippy::too_many_arguments)]
fn note_list(
    ui: &mut egui::Ui,
    coll: &mut Collection,
    editing: &mut Option<Editing>,
    search: &mut String,
    deck: &mut notes::DeckBar,
    moving: &mut Option<NoteId>,
) {
    heading(ui, "Notes");
    ui.add_space(spacing::gap(2));

    // ***Create note* is not drawn here.** It is pinned below the scroll, on ADR-0035 §1's reach
    // line (`CairnApp::ui`, ADR-0039 §8) — this screen's one primary action used to sit at the very
    // top of the page, which is the furthest point from a thumb, and a list has no leftover height
    // for the page rule to spend inside the scroll.

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
    deck_controls(ui, coll, deck);

    // Text search — the load-bearing filter (ADR-0021 §2), a plain substring over field values,
    // composing with the deck filter above (deck ∩ text; the tag filter shares the vocabulary and is
    // set on notes but has no dedicated control yet).
    ui.add_space(spacing::gap(2));
    field_label(ui, "Search");
    text_field(ui, search);
    let filter = Filter {
        deck: deck.filter.clone(),
        text: (!search.trim().is_empty()).then(|| search.trim().to_owned()),
        ..Filter::default()
    };

    // **The chrome stops here, and a line says so** (ADR-0039 §2). The gap either side of it is the
    // `gap(2)` that already separated the three chrome groups — the boundary was a missing line,
    // not a missing distance.
    ui.add_space(spacing::gap(2));
    frame::rule(ui);
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

    // **The deck is a caption on the row exactly when the list is not narrowed to a deck**
    // (ADR-0039 §3). Under *All decks* it is the only thing that tells a filed note from an unfiled
    // one; under a named deck every row would repeat the name the filter already states, which is
    // the same line saying nothing twenty-five times.
    let held = coll.decks().unwrap_or_default();
    // **And only when the collection holds a deck at all.** A caption earns its line by telling one
    // row from another; in a collection with no decks every note is unfiled by definition, so the
    // caption would read *Unfiled* twenty-five times and distinguish nothing — which is the same
    // redundancy §3 refuses under a named filter, arriving from the empty end. That is the state
    // every collection starts in, and the one the shipping seed is in.
    let captioned = matches!(deck.filter, DeckFilter::All) && !held.is_empty();
    let deck_of = |row: &notes::NoteRow| -> Option<String> {
        if !captioned {
            return None;
        }
        // A reference naming no deck the collection holds is **unfiled**, not broken (ADR-0005 §8),
        // so this lookup is allowed to miss and a miss reads *Unfiled* — the rule the editor's own
        // dropdown already applies.
        Some(
            row.deck
                .as_ref()
                .and_then(|id| {
                    held.iter()
                        .find(|(d, _)| d.to_canonical() == *id)
                        .map(|(_, n)| n.clone())
                })
                .unwrap_or_else(|| notes::UNFILED.to_owned()),
        )
    };

    // **Move** enters the two-tap placement state (ADR-0021 §4): a tap on the row's move control,
    // then a tap on a gap. No drag and no long-press — the two taps behave identically under touch
    // and mouse, which is the finding ADR-0006 §5 recorded and this must not break. The pictures
    // stand alone here under ADR-0039 §1's exception, which is what twenty-five repetitions buys.
    let actions = [
        controls::Action {
            glyph: fonts::MOVE,
            word: "Move",
        },
        controls::Action {
            glyph: fonts::DELETE,
            word: "Delete",
        },
    ];

    for (i, row) in rows.iter().enumerate() {
        // **The gap between rows is stated, one unit.** Before ADR-0032 these rows leaned on egui's
        // ambient 3px and were fused the moment it went to zero.
        if i > 0 {
            ui.add_space(spacing::gap(1));
        }
        let press = controls::row(ui, row.preview(), deck_of(row).as_deref(), &actions);
        if press.opened {
            open = Some(row.id);
        }
        match press.action {
            Some(0) => start_move = Some(row.id),
            Some(1) => delete = Some(row.id),
            _ => {}
        }
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

    // **The note being placed is held, not named** (ADR-0039 §7). It is drawn as the row it is, on
    // the one material in the system that means *temporarily on top* (ADR-0037 §2) — which is
    // exactly what a note in mid-move is, and the only place in the product where that material has
    // ever described its own contents rather than a popup's.
    let name = rows
        .iter()
        .find(|r| r.id == mid)
        .map_or("", |r| r.preview());
    field_label(ui, "Placing");
    ui.add_space(spacing::gap(1));
    controls::held(ui, name);
    ui.add_space(spacing::gap(2));

    // The gaps run among the *other* visible rows — the moving note removed so it is never offered a
    // place beside itself. Gap `i` sits before `visible[i]`, and the last gap (past the final row) is
    // the end; the open ends send the note to either extreme (ADR-0021 §4).
    //
    // **The notes keep the weight they wear everywhere else and the targets give it up.** Before
    // ADR-0039 §7 this was the other way round — twenty-six identical full-width slabs with the
    // notes set as plain body text between them, so the screen read as a list of buttons with
    // captions rather than a list of notes with gaps between them.
    let visible: Vec<&notes::NoteRow> = rows.iter().filter(|r| r.id != mid).collect();
    let mut place: Option<usize> = None;
    for gap in 0..=visible.len() {
        if controls::quiet_target(ui, "Place here").clicked() {
            place = Some(gap);
        }
        if let Some(row) = visible.get(gap) {
            ui.add_space(spacing::gap(1));
            controls::row_inert(ui, row.preview(), None);
            ui.add_space(spacing::gap(1));
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
fn deck_controls(ui: &mut egui::Ui, coll: &mut Collection, deck: &mut notes::DeckBar) {
    let decks = coll.decks().unwrap_or_default();

    field_label(ui, "Deck");
    let selected = match &deck.filter {
        DeckFilter::All => ALL_DECKS.to_owned(),
        DeckFilter::Unfiled => notes::UNFILED.to_owned(),
        DeckFilter::Deck(id) => decks
            .iter()
            .find(|(d, _)| d == id)
            .map(|(_, n)| n.clone())
            // A filter naming a deck the collection no longer holds is the same nothing a note's
            // dangling reference is (ADR-0005 §8), so it reads the same word.
            .unwrap_or_else(|| notes::UNFILED.to_owned()),
    };
    egui::ComboBox::from_id_salt("deck-filter")
        .selected_text(text(ui, &selected))
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut deck.filter, DeckFilter::All, text(ui, ALL_DECKS));
            // ***Unfiled* is a filter value, and until ADR-0039 §5 it was not expressible.** The
            // filter was an `Option` whose `None` meant *narrow nothing*, so the state ADR-0005 §8
            // calls "an unfiled view" had no way to be asked for and the dropdown offered no such
            // entry. It sits directly under *All decks* rather than after the named decks, because
            // it narrows the collection rather than naming one of its parts.
            ui.selectable_value(
                &mut deck.filter,
                DeckFilter::Unfiled,
                text(ui, notes::UNFILED),
            );
            for (id, name) in &decks {
                ui.selectable_value(&mut deck.filter, DeckFilter::Deck(*id), text(ui, name));
            }
        });

    spacing::row(ui, 1, |ui| {
        let created = controls::snug(ui, "New deck").clicked();
        text_field(ui, &mut deck.new_name);
        // Create the deck and immediately filter to it — you made it to use it (ADR-0021 §9).
        if created
            && !deck.new_name.trim().is_empty()
            && let Ok(id) = coll.create_deck(deck.new_name.trim())
        {
            deck.filter = DeckFilter::Deck(id);
            deck.new_name.clear();
        }
    });

    // **Delete names what it destroys before it destroys it** (ADR-0021 §9, ADR-0039 §6). Flagging a
    // deck deleted derives every note in it deleted (ADR-0005 §7) and there is no undelete
    // (ADR-0021 §2) — recovery is a restore from backup. So the control asks, and the question
    // carries the **count**, which is the one fact that makes the difference between an empty deck
    // and a year of authoring legible before the tap rather than after it.
    //
    // The weight does not change and that was decided rather than defaulted: the palette holds a
    // dormant error accent (ADR-0030 §5) and waking it here would make *every* destructive control
    // in the product a palette question. What is dangerous about this control is that it is silent,
    // not that it is quiet.
    let Some(id) = deck.filter.named_deck() else {
        deck.confirming = None;
        return;
    };
    if deck.confirming == Some(id) {
        let count = notes::count_in_deck(coll, id).unwrap_or(0);
        ui.add_space(spacing::gap(1));
        body(ui, &deck_delete_warning(&selected, count));
        ui.add_space(spacing::gap(1));
        spacing::row(ui, 1, |ui| {
            if controls::snug(ui, "Delete the deck and its notes").clicked() {
                let _ = coll.mutable_set("deck", &id.0, "deleted", Some("true"));
                deck.filter = DeckFilter::All;
                deck.confirming = None;
            }
            if controls::snug(ui, "Keep it").clicked() {
                deck.confirming = None;
            }
        });
    } else if controls::snug(ui, "Delete deck").clicked() {
        deck.confirming = Some(id);
    }
}

/// The word for the filter value that narrows by no deck at all.
const ALL_DECKS: &str = "All decks";

/// What the delete asks, naming the deck and the count.
///
/// A separate function so the sentence is testable without a `Ui`: the number is the whole point of
/// the warning, and "0 notes" and "1 note" are the two readings a plural-by-`s` gets wrong.
fn deck_delete_warning(name: &str, notes_in_deck: usize) -> String {
    match notes_in_deck {
        0 => format!("Delete {name}? It has no notes in it."),
        1 => format!("Delete {name}? Its 1 note is deleted with it, and cannot be undeleted."),
        n => format!("Delete {name}? Its {n} notes are deleted with it, and cannot be undeleted."),
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

    // **The gap the rest of the form has.** Every other pair in this column is one `gap(2)` apart and
    // this one was zero, so *New deck* sat welded to the deck dropdown and the two read as one
    // control with a stray label between them. Nothing failed; it is the rhythm ADR-0032 states,
    // missing at one call site.
    ui.add_space(spacing::gap(2));
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
/// **The weight was upside down, and that was the bug.** ADR-0018 §4's whole argument is that *a
/// count is not a warning* and the **names** are — and the shipped drawing put the names at the small
/// tier in weak ink while the lead-in and the reassurance took body. So the quietest thing in the
/// block was the only part that said which cards had gone dormant, and the loudest was the sentence
/// telling you not to worry. #163 turned it back up the right way: the names take body, the
/// reassurance takes the aside weight it is.
///
/// **The boundary is the second half.** The block ran out of the deck row and into the *Text* label
/// with one gap either side, so it read as three paragraphs of loose prose inside a form rather than
/// an ambient statement about it — the only such statement in the app with nothing around it. A
/// **left rule** at the separator rung is the cheapest boundary available: it adds no fill and no new
/// value, where a filled block would read as a control the size of a paragraph and #149 left
/// elevation to popups.
fn warning_banner(ui: &mut egui::Ui, warning: &cards::Warning) {
    let rule = ui.visuals().widgets.noninteractive.bg_stroke;
    egui::Frame::new()
        .inner_margin(egui::Margin {
            left: spacing::gap(2) as i8,
            ..Default::default()
        })
        .show(ui, |ui| {
            let top = ui.cursor().top();
            body(ui, "This edit made cards dormant:");
            ui.add_space(spacing::gap(1));
            for entry in &warning.dormant {
                // **The names take body**, which is what ADR-0018 §4 says the warning *is*.
                body(ui, &entry.history());
            }
            ui.add_space(spacing::gap(1));
            // The reassurance is the aside, so it takes the aside's weight.
            badge(ui, cards::UNDO_COPY);
            let bottom = ui.min_rect().bottom();
            let x = ui.min_rect().left() - spacing::gap(2);
            ui.painter().vline(x, top..=bottom, rule);
        });
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
            cards::Entry::Dormant(dormant) => dormant_entry(ui, &dormant.history()),
        }
    }
    if pane.state == cards::State::NoLiveCards {
        ui.add_space(spacing::gap(2));
        // **ADR-0018 §6's statement is a fact about the pane, not an error** (#163). At body over two
        // entries in weak small it was the loudest thing in a column headed *Cards* that has none —
        // so it read as a complaint where §6 specified a statement. It takes the same aside weight
        // the entries above it carry.
        badge(ui, "This note currently generates no cards.");
    }
}

/// A dormant entry, drawn as a **peer of the card** rather than as a caption under it (#163).
///
/// ADR-0018 §4 gives the pane's entry the job of **demonstration** — *"which card and how much
/// history, in the place the pane already puts that card"* — and the position is the demonstration.
/// Drawn as a bare line under a card it does not read as a position: it reads as a caption belonging
/// to the card above, which is why the shipped screen shows the identical sentence twice, 500px
/// apart, and reads as a duplication defect rather than as ADR-0012 §5's deliberate second speaker.
///
/// So it takes the card's **footprint and corner** with no fill — an outline where a card is a well.
/// A card is a specimen; this is the empty place a specimen used to be, which is what a dormant slot
/// is. Nothing new is named: the stroke is `card_stroke`, the corner `surface::RADIUS`.
///
/// **The shape is doing the work, and that was checked rather than asserted.** Whether an outline at
/// the card's footprint reads as *a card that is not there* or merely as a box is not a thing a
/// measurement can answer and not a thing the author of it can judge — it was put in front of the
/// repo owner beside a real card at 2× and came back as an absence. It also depends on #163's answer
/// to what the pane **is**: in a *preview*, a specimen case, an entry that is not a specimen has to
/// look like one missing. In a listing it would only have been a row.
fn dormant_entry(ui: &mut egui::Ui, line: &str) {
    egui::Frame::new()
        .stroke(crate::theme::card_stroke(ui.visuals()))
        .corner_radius(egui::CornerRadius::same(surface::RADIUS))
        .inner_margin(spacing::gap(2) as i8)
        .show(ui, |ui| {
            ui.set_width(ui.available_width() - spacing::gap(4));
            badge(ui, line);
        });
    ui.add_space(spacing::gap(2));
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

    /// **The warning names the dormant cards louder than it reassures** (#163, ADR-0018 §4).
    ///
    /// §4's argument is that *a count is not a warning* and the **names** are. The shipped drawing
    /// inverted exactly that: the lead-in and the *"nothing is deleted"* reassurance took body while
    /// the names — the only part that says *which* card went dormant — took the small tier in weak
    /// ink. So the block said the true thing quietly and the comforting thing loudly.
    ///
    /// Asserted from the **galleys that came out**, by size, because that is the claim: no branch is
    /// wrong when this drifts, no test of a call site would see it, and the screen renders perfectly
    /// either way. The number is the count of glyph runs at each tier, which is coarse on purpose —
    /// what matters is that the names are not the quietest thing in the block.
    #[test]
    fn the_warning_names_the_cards_at_the_louder_tier() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx, crate::theme::ThemeChoice::Dark);
        crate::typography::install(&ctx);
        spacing::install(&ctx);

        let warning = cards::Warning {
            dormant: vec![cards::DormantEntry {
                slot: 7,
                name: "card 7".to_owned(),
                reviews: 2,
            }],
        };
        let out = ctx.run_ui(Default::default(), |ui| {
            ui.set_width(500.0);
            warning_banner(ui, &warning);
        });

        // Every galley the frame drew, as (text, font size).
        let mut drawn: Vec<(String, f32)> = Vec::new();
        fn walk(shape: &egui::Shape, into: &mut Vec<(String, f32)>) {
            match shape {
                egui::Shape::Text(t) => {
                    let size = t
                        .galley
                        .job
                        .sections
                        .first()
                        .map_or(0.0, |s| s.format.font_id.size);
                    into.push((t.galley.text().to_owned(), size));
                }
                egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, into)),
                _ => {}
            }
        }
        for clipped in &out.shapes {
            walk(&clipped.shape, &mut drawn);
        }

        let size_of = |needle: &str| {
            drawn
                .iter()
                .find(|(t, _)| t.contains(needle))
                .unwrap_or_else(|| panic!("the warning never drew {needle:?}; it drew {drawn:?}"))
                .1
        };
        let names = size_of("card 7");
        let reassurance = size_of("Nothing is deleted");
        assert!(
            names > reassurance,
            "the names of the dormant cards are drawn at {names}px and the reassurance at \
             {reassurance}px — ADR-0018 §4 says the names are the warning"
        );
        assert_eq!(names, crate::typography::BODY);
        assert_eq!(reassurance, crate::typography::SMALL);
    }

    /// **ADR-0035 §1 on the editor, asserted from where the pixels landed** (#163).
    ///
    /// *Done* is the screen's last control and sits on the reach line — its **bottom edge** a stated
    /// distance above the bottom of the page, whatever the note above it contains. That invariant is
    /// the whole of §1, and three tickets inherited *apply it here* without being able to, because
    /// while *Done* sat at the top the screen's last control was a text field and §1 had no sensible
    /// target.
    ///
    /// **Nothing fails when this drifts.** Draw *Done* immediately under the form instead and the
    /// editor renders perfectly, looks deliberate, and is simply wrong by the height of the empty
    /// page — which is the state the app was in, on this screen, for three tickets running. So the
    /// assertion is about the rect that came out rather than about the call being present: the
    /// `slack_above` arithmetic is already pinned in `frame`, and what was missing was a caller.
    #[test]
    fn done_sits_on_the_reach_line_whatever_the_note_holds() {
        let data = TempDir::new().unwrap();
        let state = TempDir::new().unwrap();
        let mut coll = Collection::open(data.path(), state.path()).unwrap();
        let note = coll
            .create_note("basic", &[("Front", "l'aube"), ("Back", "dawn")])
            .unwrap();

        // The bottom of the lowest rect drawn — *Done* is the last thing on the page, so this is its
        // bottom edge. Read off the shapes rather than off a returned `Response`, so a rearrangement
        // that leaves the call in place and the button somewhere else still fails.
        fn lowest_rect_bottom(out: &egui::FullOutput) -> f32 {
            fn walk(shape: &egui::Shape, low: &mut f32) {
                match shape {
                    egui::Shape::Rect(r) => *low = low.max(r.rect.bottom()),
                    egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, low)),
                    _ => {}
                }
            }
            let mut low = f32::MIN;
            for clipped in &out.shapes {
                walk(&clipped.shape, &mut low);
            }
            low
        }

        let ctx = egui::Context::default();
        crate::theme::install(&ctx, crate::theme::ThemeChoice::Dark);
        crate::typography::install(&ctx);
        spacing::install(&ctx);
        let mut editing = Some(Editing::for_note(&coll, note));
        let mut page_bottom = 0.0;
        let out = ctx.run_ui(Default::default(), |ui| {
            // The page is the clip rect the screen is handed, which is what `frame::page_room`
            // measures against — take it from inside the same `Ui` rather than guessing a window.
            page_bottom = ui.clip_rect().bottom();
            editor_screen(ui, &mut coll, &mut editing);
        });

        let clearance = page_bottom - lowest_rect_bottom(&out);
        assert!(
            (clearance - frame::REACH_LINE).abs() <= 2.0,
            "the editor's last control should end {}px above the page bottom; it ended {clearance:.1}px",
            frame::REACH_LINE
        );
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
        let mut deck = notes::DeckBar::default();

        let mut frame = |moving: &mut Option<NoteId>, coll: &mut Collection| {
            ctx.run_ui(Default::default(), |ui| {
                notes_screen(ui, coll, &mut editing, &mut search, &mut deck, moving);
            })
        };

        // Normal mode: the row offers *Move*, and there is no placement control yet.
        let mut moving = None;
        let listed = drawn_text(&frame(&mut moving, &mut coll));
        assert!(
            listed.contains(fonts::MOVE),
            "a row offers the reorder entrance, now as a glyph standing alone \
             (ADR-0039 §1) — drew: {listed}"
        );
        assert!(
            !listed.contains("Place here"),
            "the gaps only appear once a move is under way — drew: {listed}"
        );

        // Tapping *Move* on the first row is modelled by the state it sets; the next frame is the
        // placement list.
        moving = Some(first);
        let placing = drawn_text(&frame(&mut moving, &mut coll));
        // **Held, not named** (ADR-0039 §7): the state is labelled once and the note is drawn as
        // the row it is, so the two facts are a caption and an object rather than one sentence.
        assert!(
            placing.contains("Placing"),
            "the placement state says what it is — drew: {placing}"
        );
        assert!(
            placing.contains("alpha"),
            "the note being placed is held as a row — drew: {placing}"
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
        let mut deck = notes::DeckBar::default();
        let mut moving = Some(hidden);

        let _ = ctx.run_ui(Default::default(), |ui| {
            notes_screen(
                ui,
                &mut coll,
                &mut editing,
                &mut search,
                &mut deck,
                &mut moving,
            );
        });

        assert_eq!(
            moving, None,
            "a move whose note the filter hides is cancelled"
        );
    }
    /// **The delete warning names the count, and names it in a sentence that survives one note**
    /// (ADR-0021 §9, ADR-0039 §6).
    ///
    /// The number is the whole point of the warning — an empty deck and a year of authoring are the
    /// same tap otherwise — and `1 notes` is the reading a plural-by-`s` produces on the one deck
    /// size where a person is most likely to shrug and confirm.
    #[test]
    fn the_delete_warning_names_what_is_lost() {
        assert!(
            deck_delete_warning("Français", 0).contains("no notes"),
            "an empty deck says so: {}",
            deck_delete_warning("Français", 0)
        );
        let one = deck_delete_warning("Français", 1);
        assert!(one.contains("1 note is"), "singular: {one}");
        let many = deck_delete_warning("Français", 25);
        assert!(many.contains("25 notes are"), "plural: {many}");
        for count in [0, 1, 25] {
            assert!(
                deck_delete_warning("Français", count).contains("Français"),
                "the deck is named at {count}"
            );
        }
        assert!(
            deck_delete_warning("Français", 25).contains("cannot be undeleted"),
            "there is no undelete here (ADR-0021 §2), and the warning is where that is said"
        );
    }
}
