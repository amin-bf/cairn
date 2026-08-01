//! PROTOTYPE shared mechanics — throwaway. Answers #28 only. See PROTOTYPE.md.
//!
//! What lives here is everything the three variants must **agree** on so they stay comparable:
//! the draft being edited, the bidi-correct text input, and the two operations ADR-0002 names as
//! dangerous (adding a blank, removing one). What does *not* live here is any layout — where the
//! preview goes, how a warning is delivered, and how blanks are inspected are exactly the
//! questions #28 asks, so each variant answers them its own way.

use crate::model::{self, History, Scenario, Values};
use eframe::egui;

/// One reversible edit. Every variant records it; only variant C puts it on screen, which is
/// itself one of the things being judged.
#[derive(Clone)]
pub struct Undo {
    pub field: String,
    pub before: String,
    pub what: String,
}

/// The note being authored. `values` is keyed by field name and deliberately keeps values for
/// fields the *current* kind does not declare: ADR-0002 §4 stores a note as its kind plus a name→
/// value map, so switching `vocab` → `basic` leaves Pronunciation sitting there unread rather than
/// destroying it. Whether the editor should *say* so is a live question — the variants disagree.
pub struct Editor {
    pub scenario: Scenario,
    pub kind: String,
    pub values: Values,
    pub tags: String,
    /// ADR-0021 §9 puts a deck dropdown beside the kind dropdown. Landed after #28 was judged, so
    /// round 2 had no such field.
    pub deck: String,
    pub undo: Option<Undo>,
    /// Set by a variant that wants a modal confirm before committing (variant A's answer).
    pub pending_save: bool,
    pub saved_note: Option<String>,
    /// Which blank number the author last touched, so a variant can point at it.
    pub focus_blank: Option<u16>,
}

impl Editor {
    pub fn load(scenario: Scenario) -> Self {
        Editor {
            scenario,
            kind: scenario.kind_id().to_string(),
            values: scenario.values(),
            tags: scenario.tags().to_string(),
            deck: "German A1".to_string(),
            undo: None,
            pending_save: false,
            saved_note: None,
            focus_blank: None,
        }
    }

    pub fn kind_def(&self) -> &'static model::KindDef {
        model::kind(&self.kind)
    }

    pub fn value(&self, field: &str) -> String {
        self.values.get(field).cloned().unwrap_or_default()
    }

    pub fn history(&self) -> &'static [History] {
        self.scenario.history()
    }

    pub fn cards(&self) -> Vec<model::GenCard> {
        model::generate(&self.kind, &self.values)
    }

    pub fn live_ordinals(&self) -> Vec<u16> {
        model::ordinals(&self.kind, &self.values)
    }

    /// Cards with review history that this draft no longer generates — ADR-0002 §7's dormant
    /// cards. Recomputed every frame from the draft, so it is live, not a save-time check.
    pub fn dormant(&self) -> Vec<model::Dormant> {
        model::dormant(self.history(), &self.live_ordinals())
    }

    /// Field values held by the note but not declared by the current kind. Never destroyed.
    pub fn orphaned_values(&self) -> Vec<(String, String)> {
        let k = self.kind_def();
        self.values
            .iter()
            .filter(|(name, v)| !v.trim().is_empty() && k.field(name).is_none())
            .map(|(name, v)| (name.clone(), v.clone()))
            .collect()
    }

    pub fn record_undo(&mut self, field: &str, what: impl Into<String>) {
        self.undo = Some(Undo {
            field: field.to_string(),
            before: self.value(field),
            what: what.into(),
        });
    }

    pub fn apply_undo(&mut self) {
        if let Some(u) = self.undo.take() {
            self.values.insert(u.field, u.before);
        }
    }

    /// Wraps `char_range` of `field` as a new blank and returns the number it was given.
    ///
    /// The number always comes from `next_blank_number` — one above the highest ever used, never
    /// the lowest free one (see that function for why filling a gap is a history-corrupting act).
    pub fn blank_selection(&mut self, field: &str, char_range: std::ops::Range<usize>) -> u16 {
        self.record_undo(field, "added a blank");
        let mut text = self.value(field);
        let n = model::next_blank_number(&text);

        let start = byte_at(&text, char_range.start);
        let end = byte_at(&text, char_range.end);
        let inner = text[start..end].to_string();
        text.replace_range(start..end, &format!("{{{{{n}::{inner}}}}}"));

        self.values.insert(field.to_string(), text);
        self.focus_blank = Some(n);
        n
    }

    /// Removes blank `n`, leaving its text in place. This is the destructive operation: the card
    /// at that ordinal stops being generated, and its reviews go dormant (§7).
    pub fn unblank(&mut self, field: &str, n: u16) {
        self.record_undo(field, format!("removed blank {n}"));
        let text = self.value(field);
        let mut out = String::with_capacity(text.len());
        for piece in model::parse_cloze(&text) {
            match piece {
                model::Piece::Text(t) => out.push_str(&t),
                model::Piece::Blank { n: m, inner } if m == n => out.push_str(&inner),
                model::Piece::Blank { n: m, inner } => {
                    out.push_str(&format!("{{{{{m}::{inner}}}}}"))
                }
            }
        }
        self.values.insert(field.to_string(), out);
        self.focus_blank = None;
    }

    pub fn set_kind(&mut self, id: &str) {
        self.kind = id.to_string();
    }
}

fn byte_at(s: &str, char_idx: usize) -> usize {
    s.char_indices().nth(char_idx).map(|(b, _)| b).unwrap_or(s.len())
}

// ---------------------------------------------------------------------------------------------
// Widgets every variant shares — input mechanics, not layout
// ---------------------------------------------------------------------------------------------

pub struct FieldOutput {
    pub changed: bool,
    /// Selected range in **characters** — live while the field has focus, and otherwise the last
    /// selection it had. The remembered one matters: clicking a "blank the selection" button takes
    /// focus away from the field, so on the frame the click is handled the live selection is
    /// already gone. Every variant with such a button depends on this.
    pub selection: Option<std::ops::Range<usize>>,
    pub has_focus: bool,
}

/// A bidi-correct text input.
///
/// AGENTS.md rule 2: `TextEdit` lays out its own text and otherwise bypasses the bidi helper, so
/// it needs `.layouter()`. Two consequences, and only one of them is inherent:
///
/// - **Inherent, and accepted:** on *RTL* text the caret and selection are in visual order while
///   the buffer is logical, so the caret is imprecise. That is the cost of the approach, recorded
///   in `AGENTS.md` — judge RTL *rendering* here, not RTL caret precision.
/// - **Not inherent, and fixed:** the caret was also wrong on plain **LTR** text, one position per
///   preceding line break, because the helper emitted a doubled newline and egui maps cursors
///   through the galley. See `bidi::job` — the laid-out text must stay byte-identical to the
///   buffer, which is now a test.
///
/// A single-line field is a real `TextEdit::singleline`, so Enter does not insert a newline into a
/// `Term`, and its text scrolls rather than wrapping inside a one-row box.
pub fn text_field(
    ui: &mut egui::Ui,
    id: egui::Id,
    value: &mut String,
    multiline: bool,
    rows: usize,
    size: f32,
    color: egui::Color32,
) -> FieldOutput {
    let rtl = crate::bidi::is_rtl(value);
    let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
        let mut job = crate::bidi::job(buf.as_str(), egui::FontId::proportional(size), color);
        // A single-line field must not wrap, or a long term folds into a box one row tall and the
        // caret goes hunting for rows that are not drawn.
        job.wrap.max_width = if multiline { wrap_width } else { f32::INFINITY };
        // **Drop the job's own alignment inside a `TextEdit`.** `bidi::job` sets `halign = Max` for
        // RTL, which lays the galley out at *negative* x — its rect runs (-109, 0) for a Persian
        // line. A label survives that because it allocates from the galley's size, but a `TextEdit`
        // draws at a fixed origin and clips, so the overhang is simply cut off. Ordering does not
        // depend on halign; alignment inside the widget is `horizontal_align`'s job, set below.
        job.halign = egui::Align::LEFT;
        ui.fonts_mut(|f| f.layout_job(job))
    };

    let mut edit = if multiline {
        egui::TextEdit::multiline(value).desired_rows(rows)
    } else {
        egui::TextEdit::singleline(value)
    };
    edit = edit
        .id(id)
        .desired_width(f32::INFINITY)
        .font(egui::FontId::proportional(size))
        .layouter(&mut layouter);
    if rtl {
        edit = edit.horizontal_align(egui::Align::RIGHT);
    }

    let out = edit.show(ui);
    // egui 0.35 indexes cursors with a `CharIndex` newtype; unwrap it so variants can slice by
    // character without importing egui's text-selection types.
    let live = out.cursor_range.map(|r| {
        let r = r.as_sorted_char_range();
        r.start.0..r.end.0
    });
    let mem_id = id.with("last-selection");
    if let Some(r) = live.clone() {
        ui.data_mut(|d| d.insert_temp(mem_id, (r.start, r.end)));
    }
    let selection = live.or_else(|| {
        ui.data_mut(|d| d.get_temp::<(usize, usize)>(mem_id)).map(|(s, e)| s..e)
    });

    // **Enter in a single-line field does nothing.** egui treats it as a submit and surrenders
    // focus, which leaves the caret nowhere and forces a click to get back in. Taking focus
    // straight back is the whole handling — no field order to thread through the variants, and no
    // opinion imposed about what Enter "should" do.
    //
    // Only a single-line field ever gets here: a multiline `TextEdit` consumes Enter as a newline
    // and never loses focus, which is what the cloze `Text` field wants.
    if out.response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        ui.memory_mut(|m| m.request_focus(id));
    }

    FieldOutput {
        changed: out.response.changed(),
        has_focus: out.response.has_focus(),
        selection,
    }
}

/// Drops the remembered selection for a field.
///
/// Must be called whenever something other than typing rewrites the text — blanking a selection
/// does exactly that. Otherwise the remembered range still describes the *old* string, the button
/// stays enabled, and a second click blanks whatever now happens to sit at those offsets.
pub fn forget_selection(ui: &mut egui::Ui, id: egui::Id) {
    ui.data_mut(|d| d.remove::<(usize, usize)>(id.with("last-selection")));
}

/// Draws a laid-out job at the full available width.
///
/// The width is not cosmetic. `bidi::job` sets `halign = RIGHT` on RTL text, and `ui.label` sizes
/// itself to its content — so a right-aligned galley that is exactly as wide as its own text has
/// nowhere to align to, and Persian ends up hugging the left edge with the alignment silently
/// doing nothing. Giving the galley the full width is what makes the alignment visible.
pub fn render(ui: &mut egui::Ui, job: egui::text::LayoutJob) {
    // `bidi::job` marks RTL by setting halign, so a standalone string aligns by its own direction.
    let rtl = job.halign == egui::Align::RIGHT;
    render_aligned(ui, job, rtl);
}

/// Draws a laid-out job flushed to one edge, with the edge chosen by the **caller**.
///
/// Callers pass the direction of the line's own first strong character — the `dir="auto"` rule —
/// so a Latin pronunciation under a Persian term sits left while the term sits right. Giving the
/// whole card one shared edge was tried and rejected: it holds the block together, but pushes
/// Latin text to the right, where it reads as misplaced.
///
/// The flush is done by laying the label out right-to-left rather than by the job's `halign`.
/// `halign = Max` produces a galley whose rect runs from negative x to 0 — epaint aligns the rows
/// against the origin, not against the wrap width — and a widget that allocates from that rect
/// then draws the text off its own left edge. Forcing `halign` back to `LEFT` keeps the galley in
/// positive space, and the layout does the alignment.
pub fn render_aligned(ui: &mut egui::Ui, mut job: egui::text::LayoutJob, rtl: bool) {
    let avail = ui.available_width();
    job.wrap.max_width = avail;
    job.halign = egui::Align::LEFT;
    let galley = ui.fonts_mut(|f| f.layout_job(job));

    if !rtl {
        ui.label(galley);
        return;
    }

    // Lay the galley out flush left and pad in front of it, rather than asking a layout or the
    // job's `halign` to do the alignment. Both of those were tried and neither puts the text where
    // it says: `halign = Max` builds a galley spanning negative x, and a right-to-left `Layout`
    // placed the label's *left* edge on the container's right, so the line ran off the far side.
    // Measuring the galley and spacing by the difference is the one version that is checkable.
    let pad = (avail - galley.rect.width()).max(0.0);
    ui.horizontal(|ui| {
        ui.add_space(pad);
        ui.label(galley);
    });
}

/// Bidi-correct plain label. Nothing user-visible uses `ui.label(&str)` directly — AGENTS.md
/// rule 1.
pub fn label(ui: &mut egui::Ui, text: &str, size: f32, color: egui::Color32) {
    render(ui, crate::bidi::job(text, egui::FontId::proportional(size), color));
}

/// Monospace chrome — numbers, counts, badges. Never card content, so it needs no bidi pass.
pub fn mono(ui: &mut egui::Ui, text: &str, size: f32, color: egui::Color32) {
    ui.label(egui::RichText::new(text).size(size).monospace().color(color));
}

pub fn reviews_phrase(reviews: u32) -> String {
    match reviews {
        0 => "no reviews yet".to_string(),
        1 => "1 review".to_string(),
        n => format!("{n} reviews"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Where the text is actually drawn, per line, in a container `width` wide.
    fn draw_extents(lines: &[(String, bool)], width: f32) -> Vec<(f32, f32)> {
        let ctx = egui::Context::default();
        crate::app::install_fonts(&ctx);
        let _ = ctx.run_ui(Default::default(), |_| {});
        let out = ctx.run_ui(Default::default(), |ui| {
            ui.set_max_width(width);
            for (text, rtl) in lines {
                let theme = crate::markdown::Theme::new(
                    15.0,
                    egui::Color32::WHITE,
                    egui::Color32::GRAY,
                    egui::Color32::GREEN,
                );
                let job = crate::markdown::job(text, crate::markdown::Cloze::Off, theme);
                render_aligned(ui, job, *rtl);
            }
        });
        out.shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::epaint::Shape::Text(t) => {
                    Some((t.pos.x, t.pos.x + t.galley.rect.width()))
                }
                _ => None,
            })
            .collect()
    }

    /// Each line takes its own direction, so a Persian card with a Latin pronunciation under it
    /// puts the term on the right and the pronunciation on the left. Judged live: one shared edge
    /// per card was the alternative, and pushing Latin text rightwards read as misplaced.
    #[test]
    fn each_line_aligns_by_its_own_direction_not_the_cards() {
        let term = "سگ".to_string();
        let pronunciation = "sag".to_string();
        let lines = vec![
            (term.clone(), crate::bidi::is_rtl(&term)),
            (pronunciation.clone(), crate::bidi::is_rtl(&pronunciation)),
        ];
        let drawn = draw_extents(&lines, 400.0);
        assert_eq!(drawn.len(), 2);

        let (_, term_end) = drawn[0];
        assert!((term_end - 400.0).abs() < 0.5, "Persian must end at the right edge, got {term_end}");

        let (latin_start, _) = drawn[1];
        assert!(
            latin_start.abs() < 0.5,
            "a Latin pronunciation must start at the left edge even inside a Persian card, got {latin_start}"
        );
    }

    #[test]
    fn an_ltr_line_stays_flush_left() {
        let lines = vec![("der Hund".to_string(), false), ("hʊnt".to_string(), false)];
        for (start, _) in draw_extents(&lines, 400.0) {
            assert!(start.abs() < 0.5, "LTR must still start at the left edge, got {start}");
        }
    }

    #[test]
    fn blanking_a_selection_takes_the_next_number_up() {
        let mut ed = Editor::load(Scenario::Cloze);
        ed.values.insert("Text".into(), "alpha beta".into());
        // chars 0..5 = "alpha"
        assert_eq!(ed.blank_selection("Text", 0..5), 1);
        assert_eq!(ed.value("Text"), "{{1::alpha}} beta");
        // and the second blank does not reuse 1
        let text = ed.value("Text");
        let start = text.chars().count() - 4;
        assert_eq!(ed.blank_selection("Text", start..start + 4), 2);
        assert_eq!(ed.value("Text"), "{{1::alpha}} {{2::beta}}");
    }

    #[test]
    fn blanking_works_on_multibyte_text() {
        let mut ed = Editor::load(Scenario::Cloze);
        ed.values.insert("Text".into(), "سگ در خانه".into());
        ed.blank_selection("Text", 6..10); // "خانه"
        assert_eq!(ed.value("Text"), "سگ در {{1::خانه}}");
    }

    #[test]
    fn unblanking_leaves_the_text_and_the_other_blanks_alone() {
        let mut ed = Editor::load(Scenario::Cloze);
        ed.values.insert("Text".into(), "{{1::a}} and {{2::b}}".into());
        ed.unblank("Text", 1);
        assert_eq!(ed.value("Text"), "a and {{2::b}}");
    }

    #[test]
    fn removing_a_blank_is_visible_as_a_dormant_card_immediately() {
        let mut ed = Editor::load(Scenario::Cloze);
        assert!(ed.dormant().is_empty());
        ed.unblank("Text", 2);
        let d = ed.dormant();
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].ordinal, 2);
        assert_eq!(d[0].reviews, 40);
    }

    #[test]
    fn undo_restores_the_blank_and_the_card_with_it() {
        let mut ed = Editor::load(Scenario::Cloze);
        ed.unblank("Text", 2);
        ed.apply_undo();
        // ADR-0002 §7: identity was derived from content, so the history reattaches by itself.
        assert!(ed.dormant().is_empty());
    }

    #[test]
    fn changing_kind_keeps_values_the_new_kind_does_not_declare() {
        let mut ed = Editor::load(Scenario::Vocab);
        ed.set_kind("basic");
        let orphans = ed.orphaned_values();
        assert!(orphans.iter().any(|(n, _)| n == "Pronunciation"));
        assert!(orphans.iter().any(|(n, _)| n == "Term"));
        // Nothing was destroyed — switching back finds them intact.
        ed.set_kind("vocab");
        assert_eq!(ed.value("Pronunciation"), "deːɐ̯ hʊnt");
    }
}
