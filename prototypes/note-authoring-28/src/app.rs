//! PROTOTYPE app shell — throwaway. Answers #28 only. See PROTOTYPE.md.
//!
//! Owns the variant switch, the scenario switch, and the phone/desktop width toggle. Each variant
//! module renders one structurally different answer to "what does authoring a note look like?"
//! against the same draft.

use crate::core::{self, Editor};
use crate::model::{self, Scenario};
use crate::{variant_a, variant_b, variant_c, variant_d};
use eframe::egui;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Variant {
    /// The graft chosen after round 1, and the one to judge now.
    D,
    A,
    B,
    C,
}

impl Variant {
    // D first, so the app opens on it and the arrow keys start there.
    const ALL: [Variant; 4] = [Variant::D, Variant::A, Variant::B, Variant::C];

    pub fn key(self) -> &'static str {
        match self {
            Variant::D => "D",
            Variant::A => "A",
            Variant::B => "B",
            Variant::C => "C",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Variant::D => "Split + cards (round 2)",
            Variant::A => "Split preview",
            Variant::B => "Cards-first",
            Variant::C => "Inline, one column",
        }
    }

    /// One line on what this variant is actually proposing, shown in the bar so a judge does not
    /// have to remember which is which.
    pub fn pitch(self) -> &'static str {
        match self {
            Variant::D => "A's split view + A's kind dropdown + B's card visuals",
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
    /// Live platform insets, re-read every frame. See `insets.rs` for why the app has to ask.
    insets: crate::insets::Insets,
    /// **The #67 switch.** Off is what the app does today: no inset is applied, so egui is handed a
    /// screen rect that includes the band the keyboard is drawn over. On is what it would do if it
    /// read the insets itself. Both are on the handset at once so the difference is judged rather
    /// than described.
    apply_insets: bool,
    /// The prototype's own controls are collapsed by default on the handset — they are not the
    /// surface being judged, and with a keyboard up every line they take is a line the editor
    /// does not get.
    controls_open: bool,
    /// Live scroll offset of the editor's `ScrollArea`, read back each frame so a forced offset can
    /// be expressed relative to where the user actually is.
    scroll_offset: f32,
    /// Set for exactly one frame, to pin the offset before layout. See `keep_focus_visible`.
    forced_scroll: Option<f32>,
    /// Last frame's visible viewport, in screen coordinates.
    viewport: egui::Rect,
    /// Last frame's reserved bottom band, so a *growing* band can be told from a steady one.
    prev_bottom_pts: f32,
}

impl ProtoApp {
    /// `PROTO_VARIANT=B PROTO_SCENARIO=cloze PROTO_WIDTH=phone cargo run` opens straight onto one
    /// combination. Only here so screenshots can be captured deterministically without
    /// synthesising clicks — the switcher bar is the real way to drive this.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let env = |k: &str| std::env::var(k).unwrap_or_default().to_ascii_lowercase();

        let variant = match env("PROTO_VARIANT").as_str() {
            "a" => Variant::A,
            "b" => Variant::B,
            "c" => Variant::C,
            _ => Variant::D,
        };
        // On the handset the phone preset is the *default*, not a toggle to remember to press:
        // #28's pass had to fake phone width on a desktop, and this one does not have to fake
        // anything. Desktop still opens on the desktop preset.
        let default_width = if cfg!(target_os = "android") { Width::Phone } else { Width::Desktop };
        let width = match env("PROTO_WIDTH").as_str() {
            "phone" => Width::Phone,
            "desktop" => Width::Desktop,
            _ => default_width,
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
        // photograph a draft with a dormant card. Interactively you just click the kind.
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
            insets: Default::default(),
            apply_insets: true,
            controls_open: !cfg!(target_os = "android"),
            scroll_offset: 0.0,
            forced_scroll: None,
            viewport: egui::Rect::NOTHING,
            prev_bottom_pts: 0.0,
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

        // ---- #67: what the platform says, and whether we listen to it -------------------------
        //
        // Re-read every frame. The keyboard animates in over ~250ms and `getRootWindowInsets`
        // reports the *current* frame of that animation, so a value cached at focus time would be
        // whatever the inset happened to be one frame in — usually near zero.
        let previous = self.insets;
        self.insets = crate::insets::read();
        if self.insets != previous {
            // Immediate mode only repaints on input, and the keyboard sliding up is not input as
            // far as winit is concerned — without this the reflow lands one stray touch later.
            ui.ctx().request_repaint();
        }
        let ppp = ui.ctx().pixels_per_point();
        let top_pts = if self.apply_insets { self.insets.top / ppp } else { 0.0 };
        let bottom_pts =
            if self.apply_insets { self.insets.bottom_occluded() / ppp } else { 0.0 };

        // The status bar. Unapplied, the app draws its first line of text *under* the clock — which
        // is true of this prototype on `main` today and is a finding in its own right.
        if top_pts > 0.5 {
            egui::Panel::top("inset-top")
                .exact_size(top_pts)
                .frame(egui::Frame::NONE)
                .show(ui, |_| {});
        }

        // The controls live at the **top** on the handset. At the bottom they are the first thing
        // the keyboard eats, and then there is no way to turn the inset handling back on — the
        // switch being judged would be unreachable in exactly the state it exists to fix.
        let controls_at_top = cfg!(target_os = "android");
        let controls = |app: &mut Self, ui: &mut egui::Ui| {
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                // The #67 switch, first and always visible.
                let label = if app.apply_insets { "insets: ON" } else { "insets: OFF" };
                let tint = if app.apply_insets { ACCENT } else { WARN };
                if ui.add(egui::Button::new(egui::RichText::new(label).color(tint))).clicked() {
                    app.apply_insets = !app.apply_insets;
                }
                core::mono(
                    ui,
                    &format!(
                        "kbd {:.0}px · bars {:.0}/{:.0} · ppp {ppp:.2}",
                        app.insets.ime, app.insets.top, app.insets.bottom
                    ),
                    10.0,
                    if app.insets.keyboard_is_up() { ACCENT } else { DIM },
                );
                if ui.button(if app.controls_open { "▾" } else { "▸" }).clicked() {
                    app.controls_open = !app.controls_open;
                }
            });
            if !app.controls_open {
                ui.add_space(4.0);
                return;
            }
            ui.horizontal_wrapped(|ui| {
                if ui.button("◀").clicked() {
                    app.variant = app.variant.prev();
                }
                ui.label(
                    egui::RichText::new(format!("{} — {}", app.variant.key(), app.variant.name()))
                        .strong()
                        .color(FG),
                );
                if ui.button("▶").clicked() {
                    app.variant = app.variant.next();
                }
                ui.separator();
                for s in Scenario::ALL {
                    let selected = s == app.editor.scenario;
                    if ui.selectable_label(selected, s.label()).clicked() && !selected {
                        app.reload(s);
                    }
                }
                ui.separator();
                for (w, name) in [(Width::Phone, "phone width"), (Width::Desktop, "desktop width")] {
                    let selected = w == app.width;
                    if ui.selectable_label(selected, name).clicked() {
                        app.width = w;
                    }
                }
                ui.separator();
                if ui.button("⟲ reset draft").clicked() {
                    let s = app.editor.scenario;
                    app.reload(s);
                }
            });
            core::mono(ui, app.variant.pitch(), 10.0, DIM);
            ui.add_space(6.0);
        };

        if controls_at_top {
            egui::Panel::top("switcher").show(ui, |ui| controls(self, ui));
        } else {
            egui::Panel::bottom("switcher").show(ui, |ui| controls(self, ui));
        }

        // Before the band is reserved, not after: the focused field has to be inside the viewport
        // on the *same* frame it shrinks, or the keyboard is gone before the next one.
        //
        // **Only while the band is growing.** Running it every frame would drag the focused field
        // back the instant the user scrolled away from it — a different bug wearing the same fix.
        if bottom_pts > self.prev_bottom_pts + 0.5 {
            // `ui` has already been shrunk by the panels shown above, so this is the region the
            // editor is about to get — minus the band that is about to be reserved below it.
            let bottom = ui.available_rect_before_wrap().bottom() - bottom_pts;
            self.keep_focus_visible(ui.ctx(), bottom);
        }
        self.prev_bottom_pts = bottom_pts;

        // The band the keyboard is sitting on. Reserving it is the entire difference between the
        // two states of the switch: with it, `CentralPanel` gets a viewport that matches what the
        // user can see and the `ScrollArea` inside it gains a real scroll range; without it, the
        // content below the fold is unreachable rather than scrollable.
        if bottom_pts > 0.5 {
            egui::Panel::bottom("inset-bottom")
                .exact_size(bottom_pts)
                .frame(egui::Frame::NONE)
                .show(ui, |_| {});
        }

        egui::CentralPanel::default().show(ui, |ui| {
            // **Force the offset on the frame the band appears**, rather than asking for a scroll
            // and letting it land next frame. One frame with the focused field outside the viewport
            // is all it takes to lose the keyboard — see `keep_focus_visible`.
            let mut area = egui::ScrollArea::vertical().auto_shrink([false, false]);
            if let Some(offset) = self.forced_scroll.take() {
                area = area.vertical_scroll_offset(offset);
            }
            let out = area.show(ui, |ui| {
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
                            Variant::D => variant_d::ui(ui, &mut self.editor, self.width),
                            Variant::A => variant_a::ui(ui, &mut self.editor, self.width),
                            Variant::B => variant_b::ui(ui, &mut self.editor, self.width),
                            Variant::C => variant_c::ui(ui, &mut self.editor, self.width),
                        }
                        ui.add_space(24.0);
                    });
                });
            });
            self.scroll_offset = out.state.offset.y;
            self.viewport = out.inner_rect;
        });
    }
}

impl ProtoApp {
/// Keep the focused text field inside the viewport the keyboard leaves — and **not** as a nicety.
///
/// `TextEdit` publishes `output.ime` only from inside `if ui.is_rect_visible(inner_rect)`
/// (egui 0.35, `widgets/text_edit/builder.rs:832`). `egui-winit` turns the *absence* of that output
/// into `set_ime_allowed(false)`, and winit's Android backend turns that into `hide_soft_input`.
///
/// So reserving the keyboard's band naively closes a loop:
///
/// > band reserved → focused field clipped → no `ime` output → keyboard hidden → inset drops to 0
/// > → viewport grows → field visible → keyboard shown → band reserved → …
///
/// which on the handset is a **continuously flickering keyboard**, reported the first time this was
/// driven by hand. Tapping a field low enough to be covered enters the loop; scrolling a focused
/// field out of view with the keyboard already up runs one lap of it, which reads as the field
/// "springing back" — that is the viewport growing again, not a scroll.
///
/// The fix is to make the antecedent false: the focused field never leaves the viewport, so the
/// `ime` output never lapses. It has to be applied **in the same frame** the band appears —
/// `scroll_to_rect` lands one frame later, and one frame without the output is one hide.
///
/// This is a finding about the real client, not about the prototype. Any implementation that reads
/// IME insets and shrinks its viewport owes this, or it oscillates.
fn keep_focus_visible(&mut self, ctx: &egui::Context, viewport_bottom: f32) {
    let Some(focused) = ctx.memory(|m| m.focused()) else {
        return;
    };
    // Last frame's rect, in screen coordinates — which is what we need, since this runs before
    // the field is laid out again.
    let Some(rect) = ctx.read_response(focused).map(|r| r.rect) else {
        return;
    };

    // A little air under the field, so the caret is not flush against the keyboard.
    let overshoot = rect.bottom() + 8.0 - viewport_bottom;
    if overshoot > 0.5 {
        self.forced_scroll = Some(self.scroll_offset + overshoot);
    }
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
