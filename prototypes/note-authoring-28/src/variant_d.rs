//! Variant D — **the graft**, and the one to judge in round 2.
//!
//! Chosen live after round 1: **A's split view, B's visuals, A's kind selector.** Concretely —
//!
//! - **A's frame.** Two panes on a desktop, folded into a `Write | Cards` toggle on a phone.
//! - **B's content.** The right pane is not a rendering of the fields, it is the **cards the note
//!   generates**, drawn by literally the same functions variant B uses (`variant_b::live_card` and
//!   `variant_b::dormant_card`), so "the visuals of B" means the visuals of B rather than a copy
//!   that will drift.
//! - **A's kind selector.** `variant_a::kind_row` — the dropdown, not B's chip row.
//!
//! Two things follow from the graft that neither parent had to decide, both flagged in PROTOTYPE.md
//! rather than settled here:
//!
//! - **Dormant cards sit in ordinal position**, not appended after the live ones. This is the fix
//!   to round 1's open problem: in B the retired card was the last thing in a scrolling column and
//!   fell below the fold, so the count in the header did all the warning. Beside the form in its
//!   own pane, in the slot the card actually occupied, it is where you are already looking.
//! - **The destructive-edit warning also appears in the *form* pane**, compactly. On a phone the
//!   cards pane is behind a toggle, so an ambient warning that lives only in the stack can be
//!   invisible exactly when it matters. The form pane is the one that is always on screen.

use crate::app::{self, Width, ACCENT, DIM, FG, WARN};
use crate::core::{self, Editor};
use crate::model::{self, Role};
use crate::{variant_a, variant_b};
use eframe::egui;

pub fn ui(ui: &mut egui::Ui, ed: &mut Editor, width: Width) {
    // A's kind selector, reused rather than reimplemented.
    variant_a::kind_row(ui, ed);
    ui.add_space(10.0);

    if width.is_phone() {
        let id = egui::Id::new("d-pane");
        let mut showing_cards = ui.data_mut(|d| d.get_temp::<bool>(id).unwrap_or(false));
        ui.horizontal(|ui| {
            for (flag, name) in [(false, "Write"), (true, "Cards")] {
                if ui.selectable_label(showing_cards == flag, name).clicked() {
                    showing_cards = flag;
                }
            }
            let dormant = ed.dormant().len();
            if dormant > 0 && !showing_cards {
                core::mono(ui, &format!("· {dormant} dormant"), 10.0, WARN);
            }
        });
        ui.data_mut(|d| d.insert_temp(id, showing_cards));
        ui.add_space(8.0);

        if showing_cards {
            cards(ui, ed);
        } else {
            form(ui, ed, true);
        }
    } else {
        ui.columns(2, |cols| {
            form(&mut cols[0], ed, false);
            cards(&mut cols[1], ed);
        });
    }

    ui.add_space(14.0);
    ui.horizontal(|ui| {
        if ui.add(egui::Button::new("Save").min_size(egui::vec2(90.0, 34.0))).clicked() {
            let n = ed.dormant().len();
            ed.saved_note = Some(if n == 0 {
                "Saved.".to_string()
            } else {
                format!("Saved — {n} card(s) now dormant.")
            });
        }
        if let Some(note) = &ed.saved_note {
            core::mono(ui, note, 11.0, ACCENT);
        }
    });
}

fn form(ui: &mut egui::Ui, ed: &mut Editor, phone: bool) {
    // In the same panel frame B uses, so the two panes read as two cards side by side rather than
    // as loose fields next to a card stack.
    app::panel_frame().show(ui, |ui| form_inner(ui, ed, phone));
}

fn form_inner(ui: &mut egui::Ui, ed: &mut Editor, phone: bool) {
    let k = ed.kind_def();
    for f in k.fields {
        ui.horizontal_wrapped(|ui| {
            core::mono(ui, f.name, 11.0, FG);
            if let Role::ShownWith(anchor) = f.role {
                core::mono(ui, &format!("· shown with {anchor}, never asked"), 10.0, DIM);
            }
        });

        let id = egui::Id::new(("d-field", f.name));
        let mut val = ed.value(f.name);
        let out = core::text_field(ui, id, &mut val, f.multiline, 4, 15.0, FG);
        if out.changed {
            ed.values.insert(f.name.to_string(), val);
        }

        if k.is_cloze() {
            blank_row(ui, ed, f.name, id, out.selection, phone);
        }
        ui.add_space(10.0);
    }

    core::mono(ui, "tags", 11.0, FG);
    let mut tags = ed.tags.clone();
    if core::text_field(ui, egui::Id::new("d-tags"), &mut tags, false, 1, 13.0, FG).changed {
        ed.tags = tags;
    }

    // The warning lives in the pane you are typing in, as well as in the stack. See the module
    // note: on a phone the stack is behind a toggle, so on its own it is not enough.
    let dormant = ed.dormant();
    if !dormant.is_empty() {
        ui.add_space(10.0);
        app::warn_frame().show(ui, |ui| {
            for d in &dormant {
                core::label(
                    ui,
                    &format!(
                        "Card {} is no longer generated — {}, box {}.",
                        d.ordinal,
                        core::reviews_phrase(d.reviews),
                        d.box_num
                    ),
                    12.0,
                    WARN,
                );
            }
            core::label(
                ui,
                "Nothing is deleted: the reviews stay in the log and reattach if the content \
                 returns.",
                11.0,
                DIM,
            );
            if ed.undo.is_some() {
                ui.add_space(6.0);
                if ui.button("Undo").clicked() {
                    ed.apply_undo();
                }
            }
        });
    }
}

/// The blank control. The chip row is **phone-only**: with the card stack sitting beside the field
/// on a desktop, a list of numbers repeats what the stack already shows — but behind a toggle the
/// stack is not visible while typing, and then the chips are the only way to check the set.
fn blank_row(
    ui: &mut egui::Ui,
    ed: &mut Editor,
    field: &'static str,
    field_id: egui::Id,
    selection: Option<std::ops::Range<usize>>,
    phone: bool,
) {
    let text = ed.value(field);
    let next = model::next_blank_number(&text);
    let has_selection = selection.as_ref().is_some_and(|r| r.start < r.end);

    ui.horizontal_wrapped(|ui| {
        let btn = egui::Button::new(format!("Blank it → makes card {next}"));
        if ui.add_enabled(has_selection, btn).clicked() {
            if let Some(r) = selection {
                ed.blank_selection(field, r);
                core::forget_selection(ui, field_id);
            }
        }
        core::mono(
            ui,
            if has_selection { "the new card appears beside" } else { "select the words to hide" },
            10.0,
            DIM,
        );
    });

    if !phone {
        return;
    }

    let numbers = model::blank_numbers(&text);
    if numbers.is_empty() {
        return;
    }

    // One line per blank, not a row of chips. The chip row read as a *count* — "blanks 1" looks
    // like "one blank" rather than "blank number 1" — and its inner separator was the same weight
    // as the gap between chips, so `1 · 5r  2` had no unambiguous parse. Naming the number and
    // spelling out the history removes both problems, and showing the hidden text answers the
    // ticket's actual question: proofreading a set of blanks you cannot read in the raw syntax.
    ui.add_space(4.0);
    core::mono(ui, &format!("blanks — {} card{}", numbers.len(), if numbers.len() == 1 { "" } else { "s" }), 10.0, DIM);
    for n in &numbers {
        ui.horizontal_wrapped(|ui| {
            core::mono(ui, &format!("{n}"), 11.0, ACCENT);
            core::label(ui, &model::blank_inner(&text, *n), 11.0, FG);
            let occurrences = model::blank_occurrences(&text, *n);
            if occurrences > 1 {
                core::mono(ui, &format!("· hidden in {occurrences} places"), 10.0, DIM);
            }
            match model::history_for(ed.history(), *n) {
                Some(h) => core::mono(ui, &format!("· {}", core::reviews_phrase(h.reviews)), 10.0, ACCENT),
                None => core::mono(ui, "· new", 10.0, DIM),
            }
        });
    }
}

/// B's stack, with one change: dormant cards are interleaved by ordinal rather than appended.
fn cards(ui: &mut egui::Ui, ed: &mut Editor) {
    let live = ed.cards();
    let dormant = ed.dormant();

    ui.horizontal_wrapped(|ui| {
        core::mono(
            ui,
            &format!("THIS NOTE MAKES {} CARD{}", live.len(), if live.len() == 1 { "" } else { "S" }),
            10.0,
            DIM,
        );
        if !dormant.is_empty() {
            core::mono(ui, &format!("· {} DORMANT", dormant.len()), 10.0, WARN);
        }
    });
    ui.add_space(8.0);

    if live.is_empty() && dormant.is_empty() {
        app::panel_frame().show(ui, |ui| {
            core::label(
                ui,
                if ed.kind_def().is_cloze() {
                    "No blanks yet — select some text and blank it to make the first card."
                } else {
                    "Fill the fields and the cards appear here."
                },
                13.0,
                DIM,
            );
        });
        return;
    }

    // One ordinal-ordered sequence, so a dormant card sits in the slot it used to occupy rather
    // than at the end of the list. Round 1's failure was positional, not visual.
    let mut slots: Vec<(u16, Option<usize>)> = live
        .iter()
        .enumerate()
        .map(|(i, c)| (c.ordinal, Some(i)))
        .chain(dormant.iter().map(|d| (d.ordinal, None)))
        .collect();
    slots.sort_by_key(|(ordinal, _)| *ordinal);

    for (ordinal, live_index) in slots {
        match live_index {
            Some(i) => variant_b::live_card(ui, ed, &live[i]),
            None => {
                if let Some(d) = dormant.iter().find(|d| d.ordinal == ordinal) {
                    variant_b::dormant_card(ui, ed, d);
                }
            }
        }
        ui.add_space(8.0);
    }
}
