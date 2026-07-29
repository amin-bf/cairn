//! Variant C — Checkpoint-forward. The newest, least-tested piece of the converged design is the
//! soft time's-up checkpoint, so this variant puts the most design care there: the timer text
//! gently warms in color as it nears zero instead of jumping straight to a banner, and the
//! finish-or-continue choice slides in as a calm footer rather than replacing the card. See
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
                if total_due == 0 {
                    ui.add_space(50.0);
                    ui.vertical_centered(|ui| rtl_label(ui, "Nothing due. Nice work.", 18.0, ACCENT_C));
                    return;
                }
                if let Some(note) = model::scenario_note(scenario) {
                    rtl_label(ui, note, 12.0, DIM_C);
                    ui.add_space(6.0);
                }
                if total_due > core::BACKLOG_THRESHOLD {
                    rtl_label(ui, &format!("{total_due} due — pick a comfortable size, the rest will keep."), 12.0, DIM_C);
                    ui.add_space(6.0);
                }
                ui.add_space(20.0);
                ui.vertical_centered(|ui| {
                    rtl_label(ui, "How many, for the next 10 minutes?", 16.0, FG_C);
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        ui.add_space((ui.available_width() - 3.0 * 64.0 - 16.0).max(0.0) / 2.0);
                        for n in [10usize, 20, 40] {
                            if ui.add(egui::Button::new(format!("{n}")).min_size(egui::vec2(64.0, 44.0))).clicked() {
                                actions.start = Some(n.min(total_due));
                            }
                        }
                    });
                });
            }
            Stage::Reviewing { card, done, total, remaining_secs, checkpoint } => {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));

                ui.vertical_centered(|ui| {
                    rtl_label(ui, &format!("{done} / {total}"), 12.0, DIM_C);
                    ui.add_space(4.0);
                    // Warms from dim to amber in the final 60s instead of a sudden cutover —
                    // the checkpoint should feel like it's arriving, not ambushing.
                    let warm = if remaining_secs <= 60 { 1.0 - (remaining_secs as f32 / 60.0) } else { 0.0 };
                    let timer_color = lerp_color(DIM_C, egui::Color32::from_rgb(0xe0, 0xa8, 0x4a), warm);
                    let m = remaining_secs / 60;
                    let s = remaining_secs % 60;
                    ui.label(egui::RichText::new(format!("{m}:{s:02}")).monospace().size(13.0).color(timer_color));
                });
                ui.add_space(16.0);

                egui::Frame::default()
                    .fill(egui::Color32::from_rgb(0x1c, 0x1f, 0x26))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(0x2a, 0x2f, 0x39)))
                    .corner_radius(14.0)
                    .inner_margin(20.0)
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(10.0);
                            rtl_label(ui, &card.front, 30.0, FG_C);
                            ui.add_space(14.0);

                            if session.revealed {
                                ui.separator();
                                ui.add_space(10.0);
                                rtl_label(ui, &card.back, 19.0, ACCENT_C);
                                ui.add_space(6.0);
                                let box_text = if card.box_num == 0 { "new".to_string() } else { format!("box {}", card.box_num) };
                                ui.label(egui::RichText::new(box_text).size(11.0).monospace().color(DIM_C));
                                ui.add_space(12.0);
                            } else {
                                rtl_label(ui, "(tap to reveal)", 12.0, DIM_C);
                                ui.add_space(6.0);
                            }
                        });

                        if session.revealed {
                            for (g, label) in [(1u8, "Forgot"), (2, "Barely"), (3, "Good"), (4, "Easy")] {
                                let days = model::projected_interval_days(card.box_num, g);
                                let interval = if days == 0 { "again soon".to_string() } else { format!("~{days}d") };
                                let btn = egui::Button::new(format!("{g}   {label}\n{interval}")).min_size(egui::vec2(ui.available_width(), 46.0));
                                if ui.add(btn).clicked() {
                                    actions.grade = Some((card.id, g));
                                }
                            }
                        }
                    });

                if !session.revealed {
                    let full = ui.min_rect();
                    let resp = ui.interact(full, ui.id().with("reveal-area"), egui::Sense::click());
                    if resp.clicked() {
                        actions.reveal = true;
                    }
                }

                // The checkpoint slides in as a calm footer under the card, not a takeover —
                // grading the card you're already looking at still works while it's up.
                if checkpoint {
                    ui.add_space(14.0);
                    egui::Frame::default()
                        .fill(egui::Color32::from_rgb(0x1a, 0x1d, 0x24))
                        .corner_radius(10.0)
                        .inner_margin(egui::vec2(14.0, 10.0))
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal(|ui| {
                                rtl_label(ui, "10 minutes are up.", 13.0, FG_C);
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("Finish here").clicked() {
                                        actions.finish_here = true;
                                    }
                                    if ui.button("Keep going").clicked() {
                                        actions.keep_going = true;
                                    }
                                });
                            });
                        });
                }
            }
            Stage::BatchComplete { total } => {
                ui.add_space(50.0);
                ui.vertical_centered(|ui| {
                    rtl_label(ui, "All done.", 22.0, ACCENT_C);
                    rtl_label(ui, &format!("{total} reviewed"), 13.0, DIM_C);
                });
            }
            Stage::FinishedEarly { done, total, left } => {
                ui.add_space(50.0);
                ui.vertical_centered(|ui| {
                    rtl_label(ui, &format!("Stopped at {done} of {total}."), 18.0, FG_C);
                    rtl_label(ui, &format!("{left} left for next time — no rush."), 13.0, DIM_C);
                });
            }
        }
    }

    core::apply(app, &queue, actions);
}

fn lerp_color(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    egui::Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}
