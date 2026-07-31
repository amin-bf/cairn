//! Variant C — **Inline, one column**. There is no preview pane, on any screen size. Each field
//! renders directly beneath its own input, so the rendered result is never somewhere else.
//!
//! Its three positions, each of which A and B disagree with:
//!
//! - **"No room for two panes" is answered by never having two.** A gets there by folding the
//!   second pane into a toggle on small screens; C says a second pane was the wrong shape to begin
//!   with, and puts each field's rendering under that field. The phone layout and the desktop
//!   layout are then the same layout, not two.
//! - **Blanks are typed, not inserted, and inspected in a list.** The syntax is visible in the
//!   input and rendered underneath, and every blank gets a row naming its number and what it
//!   hides. This is the strongest answer to "check the set at a glance", and the one that asks the
//!   most of the author.
//! - **The destructive-edit warning fires at the edit, not at the save**, as a strip pinned under
//!   the field that caused it, with an undo that puts the text back. B shows the same fact as a
//!   greyed card in a stack; C shows it as a consequence of the keystroke you just made.

use crate::app::{self, Width, ACCENT, DIM, FG, WARN};
use crate::core::{self, Editor};
use crate::markdown::{self, Cloze};
use crate::model::{self, Role};
use eframe::egui;

pub fn ui(ui: &mut egui::Ui, ed: &mut Editor, _width: Width) {
    kind_line(ui, ed);

    // Where the strip goes is the whole point of C, so it has to be attached to the edit that
    // actually caused it. For a cloze note that is the `Text` field; for every other kind the card
    // set is fixed by the kind, so the only edit that can retire a card is the kind change — and
    // the strip belongs up here under the kind, not repeated beneath each field in turn.
    let dormant = ed.dormant();
    if !dormant.is_empty() && !ed.kind_def().is_cloze() {
        warning_strip(ui, ed, &dormant);
    }
    ui.add_space(12.0);

    let k = ed.kind_def();
    for f in k.fields {
        field_block(ui, ed, f);
        ui.add_space(16.0);
    }

    core::mono(ui, "TAGS", 10.0, DIM);
    let mut tags = ed.tags.clone();
    if core::text_field(ui, egui::Id::new("c-tags"), &mut tags, false, 1, 13.0, FG).changed {
        ed.tags = tags;
    }
    core::mono(ui, "travel with the deck when you share it", 10.0, DIM);

    ui.add_space(16.0);
    ui.horizontal(|ui| {
        if ui.add(egui::Button::new("Save").min_size(egui::vec2(90.0, 34.0))).clicked() {
            ed.saved_note = Some("Saved.".to_string());
        }
        if let Some(note) = &ed.saved_note {
            core::mono(ui, note, 11.0, ACCENT);
        }
        core::mono(ui, &format!("· {} cards", ed.cards().len()), 11.0, DIM);
    });
}

/// The kind, plus an expandable panel that says exactly what changing it would do — to the fields
/// and to the cards. ADR-0002 leaves "what happens when a user switches kind" open; C's answer is
/// that you are shown the mapping before you commit to it, because two different things happen at
/// once (values stop being displayed, cards stop being generated) and only one is reversible in
/// the obvious way.
fn kind_line(ui: &mut egui::Ui, ed: &mut Editor) {
    let id = egui::Id::new("c-kind-open");
    let mut open = ui.data_mut(|d| d.get_temp::<bool>(id).unwrap_or(false));

    ui.horizontal_wrapped(|ui| {
        core::mono(ui, "KIND", 10.0, DIM);
        core::label(ui, ed.kind_def().label, 14.0, FG);
        if ui.small_button(if open { "close" } else { "change" }).clicked() {
            open = !open;
        }
    });
    core::mono(ui, ed.kind_def().blurb, 10.0, DIM);
    ui.data_mut(|d| d.insert_temp(id, open));

    if !open {
        return;
    }

    ui.add_space(8.0);
    let mut chosen: Option<&'static str> = None;
    for k in model::KINDS {
        if k.id == ed.kind {
            continue;
        }
        app::panel_frame().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                core::label(ui, k.label, 13.0, FG);
                if ui.small_button("switch to this").clicked() {
                    chosen = Some(k.id);
                }
            });

            // What each field value would do. Nothing is destroyed — ADR-0002 §4 stores a note as
            // a name→value map, so a value the new kind does not declare simply stops being read.
            for (name, value) in ed.values.iter().filter(|(_, v)| !v.trim().is_empty()) {
                let short: String = value.chars().take(28).collect();
                match k.field(name) {
                    Some(_) => core::mono(ui, &format!("{name}  →  kept, still shown"), 10.0, DIM),
                    None => core::mono(
                        ui,
                        &format!("{name}  →  kept but not shown  (\"{short}\")"),
                        10.0,
                        WARN,
                    ),
                }
            }

            let would_go_dormant = model::dormant(ed.history(), &model::ordinals(k.id, &ed.values));
            if would_go_dormant.is_empty() {
                core::mono(ui, "no card loses its history", 10.0, DIM);
            } else {
                for d in &would_go_dormant {
                    core::mono(
                        ui,
                        &format!(
                            "card {} goes dormant — {}, box {}",
                            d.ordinal,
                            core::reviews_phrase(d.reviews),
                            d.box_num
                        ),
                        10.0,
                        WARN,
                    );
                }
            }
        });
        ui.add_space(6.0);
    }

    if let Some(id_) = chosen {
        ed.set_kind(id_);
        ui.data_mut(|d| d.insert_temp(id, false));
    }
}

fn field_block(ui: &mut egui::Ui, ed: &mut Editor, f: &'static model::FieldDef) {
    let is_cloze = ed.kind_def().is_cloze();

    ui.horizontal_wrapped(|ui| {
        core::mono(ui, &f.name.to_uppercase(), 10.0, DIM);
        if let Role::ShownWith(anchor) = f.role {
            core::mono(ui, &format!("· never asked · appears with {anchor}"), 10.0, DIM);
        }
    });

    let id = egui::Id::new(("c-field", f.name));
    let mut val = ed.value(f.name);
    let out = core::text_field(ui, id, &mut val, f.multiline, 4, 15.0, FG);
    if out.changed {
        // Record before overwriting, so the strip below can offer a one-step undo of the very
        // keystroke that retired a card.
        ed.record_undo(f.name, "that edit");
        ed.values.insert(f.name.to_string(), val);
    }

    // The rendering, directly under its own input — the whole of variant C in one line of layout.
    let value = ed.value(f.name);
    if !value.trim().is_empty() {
        ui.add_space(4.0);
        let cloze = if is_cloze { Cloze::Marked } else { Cloze::Off };
        let theme = markdown::Theme::new(14.0, FG, DIM, ACCENT);
        core::render(ui, markdown::job(&value, cloze, theme));
    }

    if is_cloze {
        blank_list(ui, ed, f.name);

        // Recomputed from the draft every frame, so the strip appears on the same frame the blank
        // disappeared — not at save time — and stays for as long as the card is dormant.
        let dormant = ed.dormant();
        if !dormant.is_empty() {
            warning_strip(ui, ed, &dormant);
        }
    }
}

/// One row per blank: its number, the text it hides, how much history it carries, and a remove
/// button. This is C's answer to proofreading `{{1::…}}` — the raw syntax stays in the input, and
/// the list is what you actually read.
fn blank_list(ui: &mut egui::Ui, ed: &mut Editor, field: &'static str) {
    let text = ed.value(field);
    let numbers = model::blank_numbers(&text);

    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        core::mono(ui, "BLANKS", 10.0, DIM);
        core::mono(ui, &format!("· {} card{}", numbers.len(), if numbers.len() == 1 { "" } else { "s" }), 10.0, DIM);
        let next = model::next_blank_number(&text);
        core::mono(ui, &format!("· type {{{{{next}::…}}}} for the next one"), 10.0, DIM);
    });

    if numbers.is_empty() {
        core::mono(ui, "no blanks yet — this note makes no cards", 11.0, WARN);
        return;
    }

    let mut remove: Option<u16> = None;
    for n in &numbers {
        let inner = model::blank_inner(&text, *n);
        let occurrences = model::blank_occurrences(&text, *n);
        let history = model::history_for(ed.history(), *n);
        ui.horizontal_wrapped(|ui| {
            core::mono(ui, &format!("{n}"), 12.0, ACCENT);
            core::label(ui, &inner, 12.0, FG);
            if occurrences > 1 {
                core::mono(ui, &format!("· hidden in {occurrences} places", ), 10.0, DIM);
            }
            match history {
                Some(h) => core::mono(ui, &format!("· {}", core::reviews_phrase(h.reviews)), 10.0, ACCENT),
                None => core::mono(ui, "· new", 10.0, DIM),
            }
            if ui.small_button("remove").clicked() {
                remove = Some(*n);
            }
        });
    }

    // Numbers are never tidied, so the list must explain the hole rather than close it (§5).
    let highest = numbers.last().copied().unwrap_or(0) as usize;
    if numbers.len() < highest {
        let missing: Vec<String> = (1..=highest as u16)
            .filter(|n| !numbers.contains(n))
            .map(|n| n.to_string())
            .collect();
        core::mono(
            ui,
            &format!(
                "no blank {} — deleted earlier. The number is not reused, so its reviews can still find their way home.",
                missing.join(", ")
            ),
            10.0,
            DIM,
        );
    }

    if let Some(n) = remove {
        ed.unblank(field, n);
    }
}

fn warning_strip(ui: &mut egui::Ui, ed: &mut Editor, dormant: &[model::Dormant]) {
    ui.add_space(8.0);
    app::warn_frame().show(ui, |ui| {
        for d in dormant {
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
            "Nothing is deleted: the reviews stay in the log and reattach if the content returns.",
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
