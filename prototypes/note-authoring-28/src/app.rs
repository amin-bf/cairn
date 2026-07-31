//! PROTOTYPE app shell — throwaway. Answers #28 only. See PROTOTYPE.md.
//!
//! Owns the variant switch, the scenario switch, and the phone/desktop width toggle. Each variant
//! module renders one structurally different answer to "what does authoring a note look like?"
//! against the same draft.

use crate::core::{self, Editor};
use crate::model::{self, Scenario};
use crate::{variant_a, variant_b, variant_c};
use eframe::egui;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Variant {
    A,
    B,
    C,
}

impl Variant {
    const ALL: [Variant; 3] = [Variant::A, Variant::B, Variant::C];

    pub fn key(self) -> &'static str {
        match self {
            Variant::A => "A",
            Variant::B => "B",
            Variant::C => "C",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Variant::A => "Split preview",
            Variant::B => "Cards-first",
            Variant::C => "Inline, one column",
        }
    }

    /// One line on what this variant is actually proposing, shown in the bar so a judge does not
    /// have to remember which is which.
    pub fn pitch(self) -> &'static str {
        match self {
            Variant::A => "form | rendered preview · blanks via toolbar · confirm modal on save",
            Variant::B => "form above the cards it generates · dormancy shown in the stack, always",
            Variant::C => "no preview pane at all · render under each field · live warning + undo",
        }
    }

    fn next(self) -> Variant {
        let all = Self::ALL;
        let i = all.iter().position(|&v| v == self).unwrap();
        all[(i + 1) % all.len()]
    }

    fn prev(self) -> Variant {
        let all = Self::ALL;
        let i = all.iter().position(|&v| v == self).unwrap();
        all[(i + all.len() - 1) % all.len()]
    }
}

/// Column width. The phone preset is what makes "there is no room for two panes" judgeable
/// without the handset — but it fakes only the *width*. It cannot fake a soft keyboard taking
/// half the screen, which is the part of that question only the Pixel can answer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Width {
    Phone,
    Desktop,
}

impl Width {
    pub fn max_px(self) -> f32 {
        match self {
            Width::Phone => 400.0,
            Width::Desktop => 1100.0,
        }
    }

    pub fn is_phone(self) -> bool {
        self == Width::Phone
    }
}

pub const BG: egui::Color32 = egui::Color32::from_rgb(0x14, 0x16, 0x1a);
pub const PANEL: egui::Color32 = egui::Color32::from_rgb(0x1c, 0x1f, 0x26);
pub const LINE: egui::Color32 = egui::Color32::from_rgb(0x2a, 0x2f, 0x39);
pub const FG: egui::Color32 = egui::Color32::from_rgb(0xe6, 0xe8, 0xec);
pub const DIM: egui::Color32 = egui::Color32::from_rgb(0x7f, 0x88, 0x94);
pub const ACCENT: egui::Color32 = egui::Color32::from_rgb(0x7e, 0xe2, 0xb8);
pub const WARN: egui::Color32 = egui::Color32::from_rgb(0xe2, 0xb4, 0x6a);
pub const WARN_BG: egui::Color32 = egui::Color32::from_rgb(0x26, 0x20, 0x14);
pub const WARN_LINE: egui::Color32 = egui::Color32::from_rgb(0x45, 0x39, 0x22);

pub fn body_theme(size: f32) -> crate::markdown::Theme {
    crate::markdown::Theme::new(size, FG, DIM, ACCENT)
}

pub struct ProtoApp {
    fonts_installed: bool,
    variant: Variant,
    width: Width,
    editor: Editor,
}

impl ProtoApp {
    /// `PROTO_VARIANT=B PROTO_SCENARIO=cloze PROTO_WIDTH=phone cargo run` opens straight onto one
    /// combination. Only here so screenshots can be captured deterministically without
    /// synthesising clicks — the switcher bar is the real way to drive this.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let env = |k: &str| std::env::var(k).unwrap_or_default().to_ascii_lowercase();

        let variant = match env("PROTO_VARIANT").as_str() {
            "b" => Variant::B,
            "c" => Variant::C,
            _ => Variant::A,
        };
        let width = match env("PROTO_WIDTH").as_str() {
            "phone" => Width::Phone,
            _ => Width::Desktop,
        };
        let scenario = match env("PROTO_SCENARIO").as_str() {
            "new" => Scenario::NewNote,
            "cloze" => Scenario::Cloze,
            "persian" => Scenario::Persian,
            "kind" => Scenario::KindChange,
            _ => Scenario::Vocab,
        };

        let mut editor = Editor::load(scenario);
        // `PROTO_KIND=basic` opens with the kind already switched, which is the cheapest way to
        // photograph a draft that has retired a card. Interactively you just click the kind.
        let forced_kind = env("PROTO_KIND");
        if !forced_kind.is_empty() && model::KINDS.iter().any(|k| k.id == forced_kind) {
            editor.set_kind(&forced_kind);
        }
        // `PROTO_DROP_BLANK=2` opens with that blank already deleted, for the same reason.
        if let Ok(n) = std::env::var("PROTO_DROP_BLANK").unwrap_or_default().parse::<u16>() {
            editor.unblank("Text", n);
        }

        ProtoApp {
            fonts_installed: false,
            variant,
            width,
            editor,
        }
    }

    pub fn width(&self) -> Width {
        self.width
    }

    fn reload(&mut self, scenario: Scenario) {
        self.editor = Editor::load(scenario);
    }
}

/// Fonts, installed on the first frame rather than in `CreationContext` (AGENTS.md, client-stack
/// rule 7) and registered in **every** family including Monospace, or text silently renders as
/// boxes.
///
/// Two faces, for two different gaps found by running this prototype rather than by reasoning
/// about it:
///
/// - **Arabic**, without which Persian is boxes. Known already, and why #11 shipped the same file.
/// - **DejaVu Sans**, without which the `vocab` kind's own `Pronunciation` field is boxes.
///   egui ships Hack and Ubuntu-Light, and neither covers the IPA extensions — so `deːɐ̯ hʊnt`
///   rendered as `de□ □ h□nt` on the first run. That is a finding about the spec, not about this
///   prototype: ADR-0002 §9 defers audio on the grounds that the motivating case "is already
///   solved as text" by a written `Pronunciation` field, and a field the app cannot draw does not
///   solve anything. See PROTOTYPE.md.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "ar".into(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/NotoSansArabic-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "dejavu".into(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!("../assets/DejaVuSans.ttf"))),
    );
    // Appended, so they are fallbacks: egui's own faces still win wherever they have the glyph.
    for fam in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        let list = fonts.families.entry(fam).or_default();
        list.push("ar".into());
        list.push("dejavu".into());
    }

    // A **real bold family**, because there is no other way to draw bold. egui bundles no bold
    // face, and its own `RichText::strong` only brightens the colour — which is invisible here,
    // since the body colour is already near-white. ADR-0002 §8 puts `**bold**` in the Markdown
    // subset, so the app has to ship a face for it. Arabic first so Persian in bold is bold too
    // rather than falling back to tofu.
    fonts.font_data.insert(
        "dejavu-bold".into(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/DejaVuSans-Bold.ttf"
        ))),
    );
    fonts.font_data.insert(
        "ar-bold".into(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/NotoSansArabic-Bold.ttf"
        ))),
    );
    fonts
        .families
        .insert(crate::markdown::bold_family(), vec!["dejavu-bold".into(), "ar-bold".into()]);

    ctx.set_fonts(fonts);
}

impl eframe::App for ProtoApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if !self.fonts_installed {
            ui.ctx().set_visuals(egui::Visuals::dark());
            ui.ctx().all_styles_mut(|style| {
                style.visuals.panel_fill = BG;
                style.visuals.extreme_bg_color = egui::Color32::from_rgb(0x10, 0x12, 0x16);
                style.spacing.item_spacing = egui::vec2(8.0, 8.0);
                style.spacing.button_padding = egui::vec2(12.0, 8.0);
            });
            install_fonts(ui.ctx());
            self.fonts_installed = true;

            // **Draw nothing this frame.** `set_fonts` applies at the *start of the next* pass, so
            // the families it declares are not bound yet — and the very first bold word would hit
            // `FontFamily::Name("bold") is not bound to any fonts` and abort. Appending fallbacks
            // to `Proportional` and `Monospace` survived this because those families already
            // exist; a brand-new named family does not, which is what makes bold the case that
            // exposes it.
            //
            // This is the same one-frame deferral `AGENTS.md` client-stack rule 7 already records,
            // now for a second reason: not just *when* a face is registered, but when a family
            // becomes referenceable.
            ui.ctx().request_repaint();
            return;
        }

        // Arrow keys cycle variants — but never while a field has focus, or ← / ← in a text box
        // would throw the judge onto another variant mid-sentence.
        let typing = ui.ctx().memory(|m| m.focused().is_some());
        if !typing {
            ui.ctx().input(|i| {
                if i.key_pressed(egui::Key::ArrowRight) {
                    self.variant = self.variant.next();
                }
                if i.key_pressed(egui::Key::ArrowLeft) {
                    self.variant = self.variant.prev();
                }
            });
        }

        egui::Panel::bottom("switcher").show(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                if ui.button("◀").clicked() {
                    self.variant = self.variant.prev();
                }
                ui.label(
                    egui::RichText::new(format!("{} — {}", self.variant.key(), self.variant.name()))
                        .strong()
                        .color(FG),
                );
                if ui.button("▶").clicked() {
                    self.variant = self.variant.next();
                }
                ui.separator();
                for s in Scenario::ALL {
                    let selected = s == self.editor.scenario;
                    if ui.selectable_label(selected, s.label()).clicked() && !selected {
                        self.reload(s);
                    }
                }
                ui.separator();
                for (w, name) in [(Width::Phone, "phone width"), (Width::Desktop, "desktop width")] {
                    let selected = w == self.width;
                    if ui.selectable_label(selected, name).clicked() {
                        self.width = w;
                    }
                }
                ui.separator();
                if ui.button("⟲ reset draft").clicked() {
                    let s = self.editor.scenario;
                    self.reload(s);
                }
            });
            core::mono(ui, self.variant.pitch(), 10.0, DIM);
            ui.add_space(6.0);
        });

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                let avail = ui.available_width();
                let w = self.width.max_px().min(avail);
                let pad = ((avail - w) / 2.0).max(0.0);
                ui.horizontal(|ui| {
                    ui.add_space(pad);
                    ui.vertical(|ui| {
                        ui.set_max_width(w);
                        core::mono(
                            ui,
                            &format!(
                                "NOTE AUTHORING PROTOTYPE · #28 · {} · {}",
                                self.editor.scenario.label(),
                                if self.width.is_phone() { "phone width" } else { "desktop width" }
                            ),
                            10.0,
                            DIM,
                        );
                        core::label(ui, self.editor.scenario.note(), 11.0, DIM);
                        ui.add_space(12.0);

                        match self.variant {
                            Variant::A => variant_a::ui(ui, &mut self.editor, self.width),
                            Variant::B => variant_b::ui(ui, &mut self.editor, self.width),
                            Variant::C => variant_c::ui(ui, &mut self.editor, self.width),
                        }
                        ui.add_space(24.0);
                    });
                });
            });
        });
    }
}

// ---------------------------------------------------------------------------------------------
// Small shared chrome. Anything below is deliberately *not* layout — a frame and a chip, so the
// three variants are judged on structure rather than on which one got nicer borders.
// ---------------------------------------------------------------------------------------------

pub fn card_frame(fill: egui::Color32, stroke: egui::Color32) -> egui::Frame {
    egui::Frame::default()
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, stroke))
        .corner_radius(10.0)
        .inner_margin(14.0)
}

pub fn panel_frame() -> egui::Frame {
    card_frame(PANEL, LINE)
}

pub fn warn_frame() -> egui::Frame {
    card_frame(WARN_BG, WARN_LINE)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bold must be a **heavier face**, not a brighter colour.
    ///
    /// The first attempt brightened the body colour, which on this near-white palette moved
    /// `#e6e8ec` to about `#f3f3f4` — invisible, and reported as "I can't see bold". Measuring the
    /// laid-out width is the cheapest way to prove a different face is really being selected, and
    /// it needs no window: egui lays text out on the CPU.
    ///
    /// It guards a crash too — `FontFamily::Name` panics at draw time if nothing is bound to it —
    /// but only the *registration*, **not the timing**, and the timing is what actually shipped
    /// broken. Running a pass between `install_fonts` and the measurement is exactly what the real
    /// app could not do: `set_fonts` applies at the start of the next pass, so drawing bold in the
    /// same frame it was installed aborted. That is handled by returning early from the first
    /// frame in `ui`, and catching it here would need a two-frame `eframe` harness.
    #[test]
    fn bold_is_a_heavier_face_than_the_body_text() {
        let ctx = egui::Context::default();
        install_fonts(&ctx);
        let _ = ctx.run_ui(Default::default(), |_| {});

        let width = |family: egui::FontFamily| {
            let mut job = egui::text::LayoutJob::default();
            job.append(
                "strong",
                0.0,
                egui::TextFormat {
                    font_id: egui::FontId::new(15.0, family),
                    color: FG,
                    ..Default::default()
                },
            );
            ctx.fonts_mut(|f| f.layout_job(job)).rect.width()
        };

        let normal = width(egui::FontFamily::Proportional);
        let bold = width(crate::markdown::bold_family());
        assert!(
            bold > normal * 1.05,
            "bold ({bold}px) must be visibly heavier than normal ({normal}px)"
        );
    }
}
