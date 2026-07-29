//! Variant A — Card-first. Chrome recedes to a thin corner readout; the card takes almost the
//! whole screen. Box badge and interval preview are understated, small monospace text, so they
//! read as a quiet fact rather than a headline. See ../PROTOTYPE.md and core.rs for the shared
//! session mechanics — this module is presentation only.

use crate::app::{rtl_label, SliceApp, ACCENT_C, DIM_C, FG_C};
use crate::core::{self, Actions, Stage};
use crate::model::{self, Card};
use eframe::egui;

pub fn ui(ui: &mut egui::Ui, app: &mut SliceApp, queue: Vec<Card>) {
    let scenario = app.scenario();
    let mut actions = Actions::default();

    {
        let session = app.session_mut();
        match core::stage(session, &queue) {
            Stage::PickCount { total_due } => {
                if total_due == 0 {
                    ui.add_space(60.0);
                    ui.vertical_centered(|ui| rtl_label(ui, "Nothing due.", 20.0, FG_C));
                    return;
                }
                if let Some(note) = model::scenario_note(scenario) {
                    rtl_label(ui, note, 11.0, DIM_C);
                }
                ui.add_space(30.0);
                ui.vertical_centered(|ui| {
                    rtl_label(ui, &format!("{total_due} due"), 12.0, DIM_C);
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.add_space((ui.available_width() - 3.0 * 60.0 - 16.0).max(0.0) / 2.0);
                        for n in [10usize, 20, 40] {
                            if ui.add(egui::Button::new(format!("{n}")).min_size(egui::vec2(60.0, 40.0))).clicked() {
                                actions.start = Some(n.min(total_due));
                            }
                        }
                    });
                });
            }
            Stage::Reviewing { card, done, total, remaining_secs, checkpoint } => {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("{done}/{total}")).size(11.0).monospace().color(DIM_C));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let m = remaining_secs / 60;
                        let s = remaining_secs % 60;
                        ui.label(egui::RichText::new(format!("{m}:{s:02}")).size(11.0).monospace().color(DIM_C));
                    });
                });
                ui.add_space(30.0);

                if checkpoint {
                    ui.vertical_centered(|ui| {
                        rtl_label(ui, "Time's up.", 18.0, FG_C);
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            if ui.button("Finish here").clicked() {
                                actions.finish_here = true;
                            }
                            if ui.button("Keep going").clicked() {
                                actions.keep_going = true;
                            }
                        });
                    });
                    return;
                }

                ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));

                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    rtl_label(ui, &card.front, 34.0, FG_C);
                    ui.add_space(40.0);

                    if session.revealed {
                        rtl_label(ui, &card.back, 20.0, ACCENT_C);
                        ui.add_space(6.0);
                        let box_text = if card.box_num == 0 { "new".to_string() } else { format!("box {}", card.box_num) };
                        ui.label(egui::RichText::new(box_text).size(10.0).monospace().color(DIM_C));
                        ui.add_space(24.0);
                        for (g, label) in [(1u8, "Forgot"), (2, "Barely"), (3, "Good"), (4, "Easy")] {
                            let days = model::projected_interval_days(card.box_num, g);
                            let interval = if days == 0 { "again soon".to_string() } else { format!("~{days}d") };
                            let btn = egui::Button::new(format!("{g}  {label}   ")).min_size(egui::vec2(220.0, 38.0));
                            ui.horizontal(|ui| {
                                ui.add_space((ui.available_width() - 220.0 - 40.0).max(0.0) / 2.0);
                                if ui.add(btn).clicked() {
                                    actions.grade = Some((card.id, g));
                                }
                                ui.label(egui::RichText::new(interval).size(10.0).monospace().color(DIM_C));
                            });
                        }
                    } else {
                        rtl_label(ui, "(tap the card)", 11.0, DIM_C);
                    }
                });

                // The whole remaining central area is tappable to reveal, so the card feels like
                // the primary object rather than a button hiding inside it.
                if !session.revealed {
                    let full = ui.max_rect();
                    let resp = ui.interact(full, ui.id().with("reveal-area"), egui::Sense::click());
                    if resp.clicked() {
                        actions.reveal = true;
                    }
                }
            }
            Stage::BatchComplete { total } => {
                ui.add_space(60.0);
                ui.vertical_centered(|ui| {
                    rtl_label(ui, "Done.", 22.0, ACCENT_C);
                    rtl_label(ui, &format!("{total} reviewed"), 12.0, DIM_C);
                });
            }
            Stage::FinishedEarly { done, total, left } => {
                ui.add_space(60.0);
                ui.vertical_centered(|ui| {
                    rtl_label(ui, &format!("Stopped at {done}/{total}."), 18.0, FG_C);
                    rtl_label(ui, &format!("{left} left whenever you're back."), 12.0, DIM_C);
                });
            }
        }
    }

    core::apply(app, &queue, actions);
}
