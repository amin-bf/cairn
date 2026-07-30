//! Variant B — **Cards-first**. The preview is not a rendering of the fields; it is the **cards
//! the note generates**, drawn the way review will draw them, stacked under the form.
//!
//! Its three positions, each of which A and C disagree with:
//!
//! - **The unit of preview is the card, not the field.** ADR-0002 §1 says editing a note changes
//!   every card drawn from it at once; this makes that literal. It is also the only layout in
//!   which §3's `shown-with` rule explains itself — you watch the pronunciation move from prompt
//!   to answer between card 0 and card 1 without anybody writing a sentence about it.
//! - **There is no second pane to find room for.** The form and the stack are one column that
//!   scrolls, so the phone layout is the desktop layout. That is B's whole answer to "no room for
//!   two panes": do not have two panes.
//! - **Destructive edits are ambient and continuous, never modal.** A card whose blank you deleted
//!   *stays in the stack*, greyed, labelled with what it is holding. The warning is a permanent
//!   property of the screen rather than an interruption at save time — which is honest to §7,
//!   where retirement is not an event but a card the content no longer generates.

use crate::app::{self, Width, ACCENT, DIM, FG, LINE, PANEL, WARN};
use crate::core::{self, Editor};
use crate::markdown::{self, Cloze};
use crate::model::{self, GenCard, Role, SideLine};
use eframe::egui;

pub fn ui(ui: &mut egui::Ui, ed: &mut Editor, _width: Width) {
    kind_chips(ui, ed);
    ui.add_space(10.0);
    form(ui, ed);
    ui.add_space(16.0);
    stack(ui, ed);
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
        // No confirmation dialog. B's bet is that a warning you have been staring at for the whole
        // edit does not need repeating at the moment you press Save.
        if let Some(note) = &ed.saved_note {
            core::mono(ui, note, 11.0, ACCENT);
        }
    });
}

/// Kinds as a row of chips rather than a dropdown — the set is closed and small (ADR-0002 §2), so
/// showing all four costs one line and makes "these are all the shapes there are" visible.
fn kind_chips(ui: &mut egui::Ui, ed: &mut Editor) {
    ui.horizontal_wrapped(|ui| {
        core::mono(ui, "kind", 11.0, DIM);
        for k in model::KINDS {
            let selected = k.id == ed.kind;
            if ui.selectable_label(selected, k.label).clicked() && !selected {
                ed.set_kind(k.id);
            }
        }
    });
    core::mono(ui, ed.kind_def().blurb, 10.0, DIM);
}

fn form(ui: &mut egui::Ui, ed: &mut Editor) {
    let k = ed.kind_def();
    app::panel_frame().show(ui, |ui| {
        for f in k.fields {
            ui.horizontal(|ui| {
                core::mono(ui, f.name, 11.0, FG);
                if let Role::ShownWith(anchor) = f.role {
                    core::mono(ui, &format!("· rides with {anchor}"), 10.0, DIM);
                }
            });

            let id = egui::Id::new(("b-field", f.name));
            let mut val = ed.value(f.name);
            let out = core::text_field(ui, id, &mut val, f.multiline, 3, 15.0, FG);
            if out.changed {
                ed.values.insert(f.name.to_string(), val);
            }

            if k.is_cloze() {
                let text = ed.value(f.name);
                let next = model::next_blank_number(&text);
                let has_selection = out.selection.as_ref().is_some_and(|r| r.start < r.end);
                ui.horizontal_wrapped(|ui| {
                    let btn = egui::Button::new(format!("Blank it → makes card {next}"));
                    if ui.add_enabled(has_selection, btn).clicked() {
                        if let Some(r) = out.selection.clone() {
                            ed.blank_selection(f.name, r);
                        }
                    }
                    core::mono(
                        ui,
                        if has_selection {
                            "the new card appears below"
                        } else {
                            "select the words to hide"
                        },
                        10.0,
                        DIM,
                    );
                });
            }
            ui.add_space(8.0);
        }

        core::mono(ui, "tags", 11.0, FG);
        let mut tags = ed.tags.clone();
        if core::text_field(ui, egui::Id::new("b-tags"), &mut tags, false, 1, 13.0, FG).changed {
            ed.tags = tags;
        }
    });
}

fn stack(ui: &mut egui::Ui, ed: &mut Editor) {
    let cards = ed.cards();
    let dormant = ed.dormant();

    ui.horizontal_wrapped(|ui| {
        core::mono(
            ui,
            &format!("THIS NOTE MAKES {} CARD{}", cards.len(), if cards.len() == 1 { "" } else { "S" }),
            10.0,
            DIM,
        );
        if !dormant.is_empty() {
            core::mono(ui, &format!("· {} DORMANT", dormant.len()), 10.0, WARN);
        }
    });
    ui.add_space(8.0);

    if cards.is_empty() {
        app::panel_frame().show(ui, |ui| {
            core::label(
                ui,
                if ed.kind_def().is_cloze() {
                    "No blanks yet — select some text above and blank it to make the first card."
                } else {
                    "Fill the fields above and the cards appear here."
                },
                13.0,
                DIM,
            );
        });
    }

    for card in &cards {
        live_card(ui, ed, card);
        ui.add_space(8.0);
    }

    // The heart of variant B: a retired card does not vanish, it greys. You cannot delete a blank
    // and fail to notice what it cost, because the cost is sitting in the stack where the card was.
    for d in &dormant {
        dormant_card(ui, ed, d);
        ui.add_space(8.0);
    }
}

fn live_card(ui: &mut egui::Ui, ed: &Editor, card: &GenCard) {
    let history = model::history_for(ed.history(), card.ordinal);
    app::card_frame(PANEL, LINE).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            core::mono(ui, &format!("card {} · {}", card.ordinal, card.label), 10.0, DIM);
            match history {
                Some(h) => core::mono(
                    ui,
                    &format!("· {} · box {}", core::reviews_phrase(h.reviews), h.box_num),
                    10.0,
                    ACCENT,
                ),
                None => core::mono(ui, "· new", 10.0, DIM),
            }
        });
        ui.add_space(8.0);

        match &card.cloze_text {
            Some(text) => {
                core::render(ui, markdown::job(text, Cloze::Hide(card.ordinal), app::body_theme(15.0)));
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);
                core::mono(ui, "answer", 10.0, DIM);
                core::render(ui, markdown::job(text, Cloze::Reveal(card.ordinal), app::body_theme(15.0)));
            }
            None => {
                side(ui, &card.prompt, 17.0, FG);
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);
                core::mono(ui, "answer", 10.0, DIM);
                side(ui, &card.answer, 15.0, ACCENT);
            }
        }
    });
}

/// One side of a card. Passenger fields (§3) render smaller and dimmer beneath their anchor, which
/// is what makes "never asked" legible without a caption explaining it.
fn side(ui: &mut egui::Ui, lines: &[SideLine], size: f32, color: egui::Color32) {
    for line in lines {
        if line.text.trim().is_empty() {
            continue;
        }
        if line.passenger {
            let theme = markdown::Theme::new(12.0, DIM, DIM, DIM);
            core::render(ui, markdown::job(&line.text, Cloze::Off, theme));
        } else {
            let theme = markdown::Theme::new(size, color, DIM, ACCENT);
            core::render(ui, markdown::job(&line.text, Cloze::Off, theme));
        }
    }
}

fn dormant_card(ui: &mut egui::Ui, ed: &mut Editor, d: &model::Dormant) {
    app::warn_frame().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            core::mono(ui, &format!("card {} · dormant", d.ordinal), 10.0, WARN);
            core::mono(ui, &format!("· {} · box {}", core::reviews_phrase(d.reviews), d.box_num), 10.0, DIM);
        });
        ui.add_space(6.0);
        core::label(
            ui,
            "This card is no longer generated, so it will stop being asked. Its reviews stay in \
             the log and reattach by themselves if the content comes back.",
            12.0,
            DIM,
        );
        if let Some(undo) = &ed.undo {
            let what = undo.what.clone();
            ui.add_space(6.0);
            if ui.button(format!("Undo — {what}")).clicked() {
                ed.apply_undo();
            }
        }
    });
}
