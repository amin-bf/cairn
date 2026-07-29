//! PROTOTYPE app shell — throwaway. Owns the variant/scenario switch and the log; each variant
//! module renders one structurally different review session against the same due queue.

use crate::model::{self, Card, ReviewEvent, Scenario};
use crate::{variant_a, variant_b, variant_c};
use eframe::egui;

/// Round 2 — converged live on the session mechanics (see `core.rs`); these three differ only in
/// presentation.
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
            Variant::A => "Card-first",
            Variant::B => "Dashboard header",
            Variant::C => "Checkpoint-forward",
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

/// Everything a variant needs that is NOT stored in the log — i.e. everything that is lost on a
/// real process kill. Resetting this struct (on variant switch, scenario switch, or "simulate
/// kill & restart") is how the prototype models that loss honestly instead of faking a resume
/// the real app wouldn't actually have.
#[derive(Default)]
pub struct SessionState {
    pub revealed: bool,
    pub batch_size: Option<usize>,
    pub batch: Vec<Card>,
    pub started_at: Option<std::time::Instant>,
    /// Set by "keep going" at a checkpoint — the session pushes past the timer.
    pub continue_past_timer: bool,
    /// Set by "finish here" at a checkpoint — ends the batch early, cards or no cards left.
    pub ended_early: bool,
}

impl SessionState {
    fn reset(&mut self) {
        *self = SessionState::default();
    }
}

pub struct SliceApp {
    fonts_installed: bool,
    variant: Variant,
    scenario: Scenario,
    log: Vec<ReviewEvent>,
    session: SessionState,
}

impl SliceApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            fonts_installed: false,
            variant: Variant::A,
            scenario: Scenario::Normal,
            log: crate::store::read_all(),
            session: SessionState::default(),
        }
    }

    fn simulate_kill_and_restart(&mut self) {
        // Reload exactly what a real relaunch would see: the on-disk log, nothing else.
        self.log = crate::store::read_all();
        self.session.reset();
    }

    /// Shared by every variant: append to the in-memory log and persist it, so the due queue
    /// (recomputed from the log on every frame) drops this card everywhere at once.
    pub fn grade(&mut self, card_id: u32, grade: u8) {
        let ev = ReviewEvent { card_id, grade, at_ms: model::now_ms() };
        crate::store::append(&ev);
        self.log.push(ev);
        self.session.revealed = false;
    }
}

/// Bidi-correct label — every user-visible string in this prototype goes through this, per
/// AGENTS.md rule 1, not `ui.label` directly.
pub fn rtl_label(ui: &mut egui::Ui, text: &str, size: f32, color: egui::Color32) {
    ui.label(crate::bidi::job(text, egui::FontId::proportional(size), color));
}

const FG: egui::Color32 = egui::Color32::from_rgb(0xe6, 0xe8, 0xec);
const DIM: egui::Color32 = egui::Color32::from_rgb(0x7f, 0x88, 0x94);
const ACCENT: egui::Color32 = egui::Color32::from_rgb(0x7e, 0xe2, 0xb8);
pub const FG_C: egui::Color32 = FG;
pub const DIM_C: egui::Color32 = DIM;
pub const ACCENT_C: egui::Color32 = ACCENT;

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "ar".into(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!("../assets/NotoSansArabic-Regular.ttf"))),
    );
    for fam in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts.families.entry(fam).or_default().push("ar".into());
    }
    ctx.set_fonts(fonts);
}

impl eframe::App for SliceApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if !self.fonts_installed {
            ui.ctx().set_visuals(egui::Visuals::dark());
            ui.ctx().all_styles_mut(|style| {
                style.visuals.panel_fill = egui::Color32::from_rgb(0x14, 0x16, 0x1a);
                style.spacing.item_spacing = egui::vec2(8.0, 8.0);
                style.spacing.button_padding = egui::vec2(14.0, 12.0);
            });
            install_fonts(ui.ctx());
            self.fonts_installed = true;
        }

        // Keyboard: arrow keys cycle variant, unless a variant is mid-typing (none of these
        // variants have a text field, but keep the guard for the pattern's sake).
        ui.ctx().input(|i| {
            if i.key_pressed(egui::Key::ArrowRight) {
                self.variant = self.variant.next();
                self.session.reset();
            }
            if i.key_pressed(egui::Key::ArrowLeft) {
                self.variant = self.variant.prev();
                self.session.reset();
            }
        });

        egui::Panel::bottom("switcher").show(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                if ui.button("◀").clicked() {
                    self.variant = self.variant.prev();
                    self.session.reset();
                }
                ui.label(
                    egui::RichText::new(format!("{} — {}", self.variant.key(), self.variant.name())).strong().color(FG),
                );
                if ui.button("▶").clicked() {
                    self.variant = self.variant.next();
                    self.session.reset();
                }
                ui.separator();
                for s in Scenario::ALL {
                    let selected = s == self.scenario;
                    if ui.selectable_label(selected, s.label()).clicked() && !selected {
                        self.scenario = s;
                        self.session.reset();
                    }
                }
                ui.separator();
                if ui.button("⟲ Simulate kill & restart").clicked() {
                    self.simulate_kill_and_restart();
                }
                if ui.button("🗑 wipe log").clicked() {
                    crate::store::wipe();
                    self.log.clear();
                    self.session.reset();
                }
            });
            ui.add_space(6.0);
        });

        egui::CentralPanel::default().show(ui, |ui| {
            rtl_label(ui, &format!("REVIEW SESSION PROTOTYPE · #11 · {} scenario", self.scenario.label()), 11.0, DIM);
            ui.add_space(10.0);

            let queue = model::due_queue(self.scenario, &self.log);

            match self.variant {
                Variant::A => variant_a::ui(ui, self, queue),
                Variant::B => variant_b::ui(ui, self, queue),
                Variant::C => variant_c::ui(ui, self, queue),
            }
        });
    }
}

// Small accessors variant modules need without borrowing all of SliceApp's private fields.
impl SliceApp {
    pub fn session_mut(&mut self) -> &mut SessionState {
        &mut self.session
    }
    pub fn scenario(&self) -> Scenario {
        self.scenario
    }
}
