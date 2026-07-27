//! PROTOTYPE — throwaway. Option D: egui / eframe. Answers #8 only.
//!
//! The one non-webview slice. No HTML, no CSS, no IPC — a single canvas drawn by Rust on every
//! platform, and the storage seam back to a compile-time `#[cfg]`.

pub mod bidi;
pub mod model;
pub mod store;

#[cfg(target_os = "android")]
pub mod android;

use model::{ReviewEvent, CARDS, GRADES};

#[cfg(target_os = "android")]
pub const DEVICE: &str = "egui-android";
#[cfg(target_arch = "wasm32")]
pub const DEVICE: &str = "egui-web";
#[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
pub const DEVICE: &str = "egui-desktop";

pub struct SliceApp {
    idx: usize,
    revealed: bool,
    log: Vec<ReviewEvent>,
    /// Typed-answer probe for #11 — is a real text field usable here?
    typed: String,
}

impl SliceApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Match the other three slices' look as closely as egui allows.
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        cc.egui_ctx.all_styles_mut(|style| {
            style.visuals.panel_fill = egui::Color32::from_rgb(0x14, 0x16, 0x1a);
            style.visuals.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(0x25, 0x29, 0x32);
            style.visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(0x2f, 0x35, 0x41);
            style.visuals.widgets.active.weak_bg_fill = egui::Color32::from_rgb(0x33, 0x39, 0x45);
            style.spacing.item_spacing = egui::vec2(8.0, 8.0);
            style.spacing.button_padding = egui::vec2(14.0, 12.0);
        });

        {
            // Embedded, not read from a system path — Android has no /usr/share/fonts.
            // egui ships only Hack, Ubuntu-Light and Noto Emoji, so Arabic script needs a face.
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "ar".into(),
                std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
                    "../assets/NotoSansArabic-Regular.ttf"
                ))),
            );
            for fam in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                fonts.families.entry(fam).or_default().push("ar".into());
            }
            cc.egui_ctx.set_fonts(fonts);
        }
        store::kick_off_load();
        Self { idx: 0, revealed: false, log: store::read_all(), typed: String::new() }
    }
}

impl eframe::App for SliceApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // The polling layer. On native this is a no-op; on web it is how OPFS results arrive.
        if let Some(evs) = store::poll() {
            self.log = evs;
        }

        {
            ui.add_space(14.0);
            ui.label(
                egui::RichText::new(format!("EGUI SLICE · {}", DEVICE.to_uppercase()))
                    .size(11.0)
                    .color(egui::Color32::from_rgb(0x7f, 0x88, 0x94)),
            );
            ui.add_space(16.0);

            let card = &CARDS[self.idx % CARDS.len()];

            egui::Frame::central_panel(&ui.style().clone())
                .fill(egui::Color32::from_rgb(0x1c, 0x1f, 0x26))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(0x2a, 0x2f, 0x39)))
                .corner_radius(14.0)
                .inner_margin(20.0)
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(10.0);
                        ui.label(bidi::job(card.front, egui::FontId::proportional(30.0), egui::Color32::from_rgb(0xe6,0xe8,0xec)));
                        ui.add_space(14.0);
                    });

                    if self.revealed {
                        ui.separator();
                        ui.vertical_centered(|ui| {
                            ui.add_space(10.0);
                            ui.label(bidi::job(
                                card.back,
                                egui::FontId::proportional(19.0),
                                egui::Color32::from_rgb(0x7e, 0xe2, 0xb8),
                            ));
                            ui.add_space(12.0);
                        });
                        let b = egui::Button::new(
                            egui::RichText::new("Next card")
                                .strong()
                                .color(egui::Color32::from_rgb(0x14, 0x16, 0x1a)),
                        )
                        .fill(egui::Color32::from_rgb(0x7e, 0xe2, 0xb8))
                        .corner_radius(10.0)
                        .min_size(egui::vec2(ui.available_width(), 44.0));
                        if ui.add(b).clicked() {
                            self.idx += 1;
                            self.revealed = false;
                        }
                    } else {
                        for (g, label) in GRADES {
                            let fill = if g == 1 {
                                egui::Color32::from_rgb(0x2a, 0x1e, 0x21)
                            } else {
                                egui::Color32::from_rgb(0x25, 0x29, 0x32)
                            };
                            let b = egui::Button::new(egui::RichText::new(format!("  {g}   {label}")))
                                .fill(fill)
                                .corner_radius(10.0)
                                .min_size(egui::vec2(ui.available_width(), 46.0));
                            if ui.add(b).clicked() {
                                let ev = ReviewEvent {
                                    card_id: self.idx as u32,
                                    grade: g,
                                    at_ms: store::now_ms(),
                                    device: DEVICE.to_string(),
                                };
                                store::append(&ev);
                                self.log.push(ev);
                                self.revealed = true;
                            }
                            if g == 1 {
                                ui.add_space(4.0);
                            }
                        }
                    }
                });

            ui.add_space(16.0);
            ui.label(
                egui::RichText::new("TYPED ANSWER PROBE (#11)")
                    .size(10.0)
                    .color(egui::Color32::from_rgb(0x7f, 0x88, 0x94)),
            );
            // TextEdit lays out its own text, so it bypasses the bidi helper unless we hand it
            // a custom layouter that routes through the same LayoutJob.
            let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
                let mut job = bidi::job(
                    text.as_str(),
                    egui::FontId::proportional(20.0),
                    egui::Color32::from_rgb(0xe6, 0xe8, 0xec),
                );
                job.wrap.max_width = wrap_width;
                ui.ctx().fonts_mut(|f| f.layout_job(job))
            };
            let rtl = bidi::is_rtl(&self.typed);
            ui.add(
                egui::TextEdit::singleline(&mut self.typed)
                    .horizontal_align(if rtl { egui::Align::RIGHT } else { egui::Align::LEFT })
                    .hint_text("type here — Latin, Persian, anything")
                    .desired_width(f32::INFINITY)
                    .font(egui::FontId::proportional(20.0))
                    .layouter(&mut layouter),
            );
            // Through the helper — a plain RichText here renders RTL backwards. This debug line
            // was itself the first thing to get it wrong.
            ui.label(bidi::job(
                &format!("{} chars: {}", self.typed.chars().count(), self.typed),
                egui::FontId::monospace(11.0),
                egui::Color32::from_rgb(0x9a, 0xa3, 0xb0),
            ));

            ui.add_space(16.0);
            ui.label(
                egui::RichText::new(format!("Persisted log — {} events", self.log.len())).strong(),
            );
            ui.label(
                egui::RichText::new(store::backend())
                    .size(10.0)
                    .color(egui::Color32::from_rgb(0x7f, 0x88, 0x94)),
            );
            ui.add_space(6.0);
            for ev in self.log.iter().rev().take(6) {
                ui.label(bidi::job(
                    &format!(
                        "card {} · grade {} · {} · {}",
                        ev.card_id, ev.grade, ev.at_ms, ev.device
                    ),
                    egui::FontId::monospace(10.0),
                    egui::Color32::from_rgb(0x9a, 0xa3, 0xb0),
                ));
            }
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new("Restart the app. The log must survive.")
                    .size(11.0)
                    .color(egui::Color32::from_rgb(0x7f, 0x88, 0x94)),
            );
        }
    }
}


/// Android entry point. `android-native-activity` means the framework's own `NativeActivity`
/// hosts us, so the APK needs no Java or Kotlin at all — just this `.so` and a manifest.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: android_activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid as _;

    let options = eframe::NativeOptions {
        android_app: Some(app.clone()),
        event_loop_builder: Some(Box::new(move |b| {
            b.with_android_app(app.clone());
        })),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "egui slice",
        options,
        Box::new(|cc| Ok(Box::new(SliceApp::new(cc)))),
    );
}
