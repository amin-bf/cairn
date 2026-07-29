//! Variant B — Dashboard header. A prominent strip up top always shows progress, time left and
//! backlog context together, so the session never has to be inferred. The interval preview is a
//! single aligned row under the grade buttons rather than embedded per-button. See
//! ../PROTOTYPE.md and core.rs for the shared session mechanics — this module is presentation
//! only.

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
                dashboard(ui, None, None, total_due);
                if total_due == 0 {
                    ui.add_space(30.0);
                    ui.vertical_centered(|ui| rtl_label(ui, "Nothing due right now.", 16.0, FG_C));
                    return;
                }
                if let Some(note) = model::scenario_note(scenario) {
                    ui.add_space(6.0);
                    rtl_label(ui, note, 12.0, DIM_C);
                }
                ui.add_space(16.0);
                ui.vertical_centered(|ui| {
                    rtl_label(ui, "Pick a session size", 14.0, FG_C);
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.add_space((ui.available_width() - 3.0 * 70.0 - 16.0).max(0.0) / 2.0);
                        for n in [10usize, 20, 40] {
                            if ui.add(egui::Button::new(format!("{n} cards")).min_size(egui::vec2(70.0, 42.0))).clicked() {
                                actions.start = Some(n.min(total_due));
                            }
                        }
                    });
                });
            }
            Stage::Reviewing { card, done, total, remaining_secs, checkpoint } => {
                dashboard(ui, Some((done, total)), Some(remaining_secs), queue.len());
                ui.add_space(14.0);

                if checkpoint {
                    egui::Frame::default()
                        .fill(egui::Color32::from_rgb(0x22, 0x1f, 0x16))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(0x40, 0x37, 0x24)))
                        .corner_radius(10.0)
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                rtl_label(ui, "Time's up — finish here, or keep going?", 14.0, FG_C);
                                ui.add_space(10.0);
                                ui.horizontal(|ui| {
                                    ui.add_space((ui.available_width() - 220.0).max(0.0) / 2.0);
                                    if ui.add(egui::Button::new("Finish here").min_size(egui::vec2(100.0, 36.0))).clicked() {
                                        actions.finish_here = true;
                                    }
                                    if ui.add(egui::Button::new("Keep going").min_size(egui::vec2(100.0, 36.0))).clicked() {
                                        actions.keep_going = true;
                                    }
                                });
                            });
                        });
                    return;
                }

                ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));

                egui::Frame::default()
                    .fill(egui::Color32::from_rgb(0x1c, 0x1f, 0x26))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(0x2a, 0x2f, 0x39)))
                    .corner_radius(14.0)
                    .inner_margin(20.0)
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(8.0);
                            rtl_label(ui, &card.front, 28.0, FG_C);
                            ui.add_space(12.0);
                        });

                        if session.revealed {
                            ui.separator();
                            ui.vertical_centered(|ui| {
                                ui.add_space(8.0);
                                rtl_label(ui, &card.back, 18.0, ACCENT_C);
                                ui.add_space(6.0);
                                let box_text = if card.box_num == 0 { "new".to_string() } else { format!("box {}", card.box_num) };
                                ui.label(egui::RichText::new(box_text).size(11.0).monospace().color(DIM_C));
                                ui.add_space(14.0);
                            });

                            for (g, label) in [(1u8, "Forgot"), (2, "Barely"), (3, "Good"), (4, "Easy")] {
                                let days = model::projected_interval_days(card.box_num, g);
                                let interval = if days == 0 { "again soon".to_string() } else { format!("~{days}d") };
                                let btn = egui::Button::new(format!("{g}   {label}\n{interval}")).min_size(egui::vec2(ui.available_width(), 46.0));
                                if ui.add(btn).clicked() {
                                    actions.grade = Some((card.id, g));
                                }
                            }
                        } else {
                            rtl_label(ui, "tap the card to reveal", 12.0, DIM_C);
                            ui.add_space(4.0);
                        }
                    });

                if !session.revealed {
                    let full = ui.max_rect();
                    let resp = ui.interact(full, ui.id().with("reveal-area"), egui::Sense::click());
                    if resp.clicked() {
                        actions.reveal = true;
                    }
                }
            }
            Stage::BatchComplete { total } => {
                dashboard(ui, Some((total, total)), None, 0);
                ui.add_space(40.0);
                ui.vertical_centered(|ui| {
                    rtl_label(ui, "Session complete.", 20.0, ACCENT_C);
                    rtl_label(ui, &format!("{total} reviewed"), 13.0, DIM_C);
                });
            }
            Stage::FinishedEarly { done, total, left } => {
                dashboard(ui, Some((done, total)), None, left);
                ui.add_space(40.0);
                ui.vertical_centered(|ui| {
                    rtl_label(ui, "Ended early — that's fine.", 18.0, FG_C);
                    rtl_label(ui, &format!("{left} left for next time."), 13.0, DIM_C);
                });
            }
        }
    }

    core::apply(app, &queue, actions);
}

fn dashboard(ui: &mut egui::Ui, progress: Option<(usize, usize)>, remaining_secs: Option<u64>, backlog_total: usize) {
    egui::Frame::default()
        .fill(egui::Color32::from_rgb(0x1a, 0x1d, 0x24))
        .corner_radius(10.0)
        .inner_margin(egui::vec2(14.0, 10.0))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                if let Some((done, total)) = progress {
                    ui.label(egui::RichText::new(format!("{done} / {total}")).strong().color(FG_C));
                    ui.add(egui::ProgressBar::new(if total == 0 { 1.0 } else { done as f32 / total as f32 }).desired_width(120.0));
                } else {
                    ui.label(egui::RichText::new("no batch yet").color(DIM_C).size(12.0));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(secs) = remaining_secs {
                        let m = secs / 60;
                        let s = secs % 60;
                        ui.label(egui::RichText::new(format!("{m}:{s:02} left")).monospace().color(DIM_C));
                    }
                });
            });
            if backlog_total > core::BACKLOG_THRESHOLD {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!("{backlog_total} due overall — this session covers a slice of it"))
                        .size(11.0)
                        .color(DIM_C),
                );
            }
        });
}
