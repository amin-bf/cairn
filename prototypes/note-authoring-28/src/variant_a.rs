//! Variant A — **Split preview**. The conventional answer, and the one ADR-0002 §8 describes
//! most literally: fields on one side, the rendered result on the other.
//!
//! Its three positions, each of which B and C disagree with:
//!
//! - **The preview is about markup**, not about cards. It shows what the text will look like once
//!   rendered; what cards come out is a one-line summary. So the preview answers "did I get the
//!   markup right", which is the question §8 says a live preview exists to answer.
//! - **The phone answer is a toggle.** There is no room for two panes, so on a narrow screen the
//!   two panes become `Write | Preview` and you switch. Nothing is dropped; it is serialised.
//! - **Destructive edits are caught at save time, in a modal.** An edit is not committed yet, so
//!   the warning arrives at the moment of commitment, where it can still be declined outright.

use crate::app::{self, Width, ACCENT, DIM, FG, WARN};
use crate::core::{self, Editor};
use crate::markdown::{self, Cloze};
use crate::model::{self, Role};
use eframe::egui;

pub fn ui(ui: &mut egui::Ui, ed: &mut Editor, width: Width) {
    kind_row(ui, ed);
    ui.add_space(10.0);

    if width.is_phone() {
        let id = egui::Id::new("a-pane");
        let mut showing_preview = ui.data_mut(|d| d.get_temp::<bool>(id).unwrap_or(false));
        ui.horizontal(|ui| {
            for (flag, name) in [(false, "Write"), (true, "Preview")] {
                if ui.selectable_label(showing_preview == flag, name).clicked() {
                    showing_preview = flag;
                }
            }
        });
        ui.data_mut(|d| d.insert_temp(id, showing_preview));
        ui.add_space(8.0);

        if showing_preview {
            preview(ui, ed);
        } else {
            form(ui, ed);
        }
    } else {
        ui.columns(2, |cols| {
            form(&mut cols[0], ed);
            preview(&mut cols[1], ed);
        });
    }

    ui.add_space(14.0);
    save_row(ui, ed);
    save_modal(ui, ed);
}

fn kind_row(ui: &mut egui::Ui, ed: &mut Editor) {
    app::panel_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            core::mono(ui, "kind", 11.0, DIM);
            let current = ed.kind_def().label;
            egui::ComboBox::from_id_salt("a-kind").selected_text(current).show_ui(ui, |ui| {
                for k in model::KINDS {
                    let selected = k.id == ed.kind;
                    if ui.selectable_label(selected, k.label).clicked() && !selected {
                        ed.set_kind(k.id);
                    }
                }
            });
        });
        core::label(ui, ed.kind_def().blurb, 11.0, DIM);
    });
}

fn form(ui: &mut egui::Ui, ed: &mut Editor) {
    let k = ed.kind_def();
    for f in k.fields {
        ui.horizontal(|ui| {
            core::mono(ui, f.name, 11.0, FG);
            // The `shown-with` question, answered in the field label: a passenger says so where
            // you are typing it, rather than being explained somewhere else.
            if let Role::ShownWith(anchor) = f.role {
                core::mono(ui, &format!("· shown with {anchor}, never asked"), 10.0, DIM);
            }
        });

        let id = egui::Id::new(("a-field", f.name));
        let mut val = ed.value(f.name);
        let out = core::text_field(ui, id, &mut val, f.multiline, 4, 15.0, FG);
        if out.changed {
            ed.values.insert(f.name.to_string(), val);
        }

        if k.is_cloze() {
            blank_toolbar(ui, ed, f.name, out.selection);
        }
        ui.add_space(10.0);
    }

    core::mono(ui, "tags", 11.0, FG);
    let id = egui::Id::new("a-tags");
    let mut tags = ed.tags.clone();
    if core::text_field(ui, id, &mut tags, false, 1, 13.0, FG).changed {
        ed.tags = tags;
    }
}

/// A toolbar button, plus a chip per blank. The chips are variant A's answer to "check the set at
/// a glance": a compact row of the numbers that exist, so a gap is visible as a gap.
fn blank_toolbar(
    ui: &mut egui::Ui,
    ed: &mut Editor,
    field: &'static str,
    selection: Option<std::ops::Range<usize>>,
) {
    let text = ed.value(field);
    let next = model::next_blank_number(&text);
    let has_selection = selection.as_ref().is_some_and(|r| r.start < r.end);

    ui.horizontal_wrapped(|ui| {
        let btn = egui::Button::new(format!("{{{{ }}}}  blank the selection → {next}"));
        if ui.add_enabled(has_selection, btn).clicked() {
            if let Some(r) = selection {
                ed.blank_selection(field, r);
            }
        }
        if !has_selection {
            core::mono(ui, "select some text first", 10.0, DIM);
        }
    });

    let numbers = model::blank_numbers(&text);
    if numbers.is_empty() {
        return;
    }
    ui.horizontal_wrapped(|ui| {
        core::mono(ui, "blanks", 10.0, DIM);
        for n in &numbers {
            let occurrences = model::blank_occurrences(&text, *n);
            let reviews = model::history_for(ed.history(), *n).map(|h| h.reviews).unwrap_or(0);
            let mut chip = format!("{n}");
            if occurrences > 1 {
                chip.push_str(&format!("×{occurrences}"));
            }
            if reviews > 0 {
                chip.push_str(&format!(" · {reviews}r"));
            }
            let colour = if reviews > 0 { ACCENT } else { DIM };
            ui.label(egui::RichText::new(chip).size(10.0).monospace().color(colour));
        }
    });

    // ADR-0002 §5: gaps are normal — they are what deleting a blank leaves behind. Saying so is
    // the whole answer to "why is there no 3?", and it is the alternative to tidying the numbers.
    let highest = numbers.last().copied().unwrap_or(0) as usize;
    if numbers.len() < highest {
        core::mono(ui, "gaps are normal — a deleted blank leaves its number behind", 10.0, DIM);
    }
}

fn preview(ui: &mut egui::Ui, ed: &mut Editor) {
    let k = ed.kind_def();
    app::panel_frame().show(ui, |ui| {
        ui.set_min_height(120.0);
        core::mono(ui, "PREVIEW", 10.0, DIM);
        ui.add_space(6.0);
        for f in k.fields {
            let value = ed.value(f.name);
            core::mono(ui, f.name, 10.0, DIM);
            if value.trim().is_empty() {
                core::mono(ui, "—", 13.0, DIM);
            } else {
                let cloze = if k.is_cloze() { Cloze::Marked } else { Cloze::Off };
                core::render(ui, markdown::job(&value, cloze, app::body_theme(15.0)));
            }
            ui.add_space(8.0);
        }
    });

    ui.add_space(8.0);

    // Cards get a summary line, not a rendering. Variant A's bet is that while you are writing,
    // the markup is the live question and the card set is not.
    let cards = ed.cards();
    let labels: Vec<String> = cards.iter().map(|c| c.label.clone()).collect();
    core::mono(
        ui,
        &format!(
            "{} card{} — {}",
            cards.len(),
            if cards.len() == 1 { "" } else { "s" },
            if labels.is_empty() { "none yet".to_string() } else { labels.join(", ") }
        ),
        11.0,
        DIM,
    );
}

fn save_row(ui: &mut egui::Ui, ed: &mut Editor) {
    ui.horizontal(|ui| {
        if ui.add(egui::Button::new("Save").min_size(egui::vec2(90.0, 34.0))).clicked() {
            if ed.dormant().is_empty() {
                ed.saved_note = Some("Saved.".to_string());
            } else {
                ed.pending_save = true;
            }
        }
        if let Some(note) = &ed.saved_note {
            core::mono(ui, note, 11.0, ACCENT);
        }
    });
}

/// The destructive-edit warning, variant A's way: a modal at the moment of commitment, naming
/// every card that will stop being generated and what it is carrying. ADR-0002 §7 requires the
/// authoring UI to say this, because nothing downstream can.
fn save_modal(ui: &mut egui::Ui, ed: &mut Editor) {
    if !ed.pending_save {
        return;
    }
    let dormant = ed.dormant();
    let mut close = false;
    let mut commit = false;

    egui::Modal::new(egui::Id::new("a-save-modal")).show(ui.ctx(), |ui| {
        ui.set_max_width(420.0);
        core::label(ui, "This edit retires cards that have history", 16.0, WARN);
        ui.add_space(8.0);
        for d in &dormant {
            core::label(
                ui,
                &format!(
                    "Card {} — {}, box {}",
                    d.ordinal,
                    core::reviews_phrase(d.reviews),
                    d.box_num
                ),
                13.0,
                FG,
            );
        }
        ui.add_space(8.0);
        core::label(
            ui,
            "Their reviews stay in the log and are not lost — the cards simply stop being asked. \
             Put the content back and the history reattaches by itself.",
            12.0,
            DIM,
        );
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui.add(egui::Button::new("Save anyway").min_size(egui::vec2(110.0, 32.0))).clicked() {
                commit = true;
            }
            if ui.add(egui::Button::new("Go back").min_size(egui::vec2(90.0, 32.0))).clicked() {
                close = true;
            }
        });
    });

    if commit {
        ed.pending_save = false;
        ed.saved_note = Some(format!("Saved — {} card(s) retired.", dormant.len()));
    } else if close {
        ed.pending_save = false;
    }
}
