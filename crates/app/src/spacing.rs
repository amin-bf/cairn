//! **The rhythm** — the one unit every gap in this application is a multiple of. Decided in
//! [#132](https://github.com/amin-bf/cairn/issues/132) and recorded in
//! [ADR-0032](../../../docs/adr/0032-the-type-scale-and-the-rhythm.md).
//!
//! Like [`crate::typography`], the number is a **logical pixel** — Android's `dp`, iOS's `pt` — so the
//! rhythm carries to a native client and only [`install`] is rewritten.
//!
//! # The rhythm is *stated*, and that is the whole decision
//!
//! egui adds `item_spacing` between consecutive widgets **before** any gap a caller states; its own
//! documentation says a stated gap is *"in addition to the `item_spacing` that is always added"*. With
//! stock's `vec2(8.0, 3.0)`, the app's seventy `add_space` calls read as a tidy 4/8/12 grid in the
//! source and **drew 7, 11 and 15** — a grid of nothing — while every horizontal row silently overran
//! its column by 8 per gap. That was not a hypothesis: the two-column editor #131 landed was measured
//! 8px wider than its own page frame, off-centre with 80px of margin on the left and 72 on the right,
//! and nothing failed.
//!
//! So [`install`] **zeroes `item_spacing`** and every gap is stated through [`gap`]. The number in the
//! source is then the number on the screen, and width arithmetic like `(available - gutter) / 2`
//! becomes correct rather than approximately correct.
//!
//! **This deliberately does not follow the ambient-role pattern** of ADR-0030 §1 and ADR-0031 §1, and
//! the reason is worth keeping because it looks like an inconsistency otherwise. Naming a value once
//! only helps when call sites can then stop naming values. An ambient gap of 8 cannot do that here:
//! the fourteen sites that want 16 would have to write `8` to reach it, and a gap *smaller* than the
//! ambient is not expressible at all, because there is no negative space. Colour and the page frame
//! could be made ambient; a rhythm of many different gaps cannot.
//!
//! # A half-step will not compile
//!
//! [`gap`] takes a whole number of units. A unit that permits halves is a four-unit wearing an eight
//! label, which is the same untruth as the invisible 3 this module exists to remove. Making the wrong
//! value unrepresentable is cheaper than a rule that asks people not to write it.
//!
//! # What the rhythm does **not** govern
//!
//! The page frame is its own family: [`crate::frame::PAGE_MARGIN`] is 28, which is not a multiple of
//! [`UNIT`], and that is correct rather than an oversight. A margin is the distance from content to
//! the *window edge*, judged against captures at three widths (ADR-0031 §2); a gap is the distance
//! between two things *inside* the column. The rhythm has no more claim on the margin than it has on
//! the 640 measure or the 36px control height, and bending 28 to fit a grid it was never judged
//! against would re-open a decided ADR to buy tidiness.

use egui::{Vec2, vec2};

/// The base unit. **Every gap between two things inside the column is a whole multiple of this.**
///
/// Eight rather than four because a unit earns its keep by *refusing* things, and at the range of
/// gaps this application draws — one to four units — a four-grid refuses almost nothing. Variant A
/// used four and lost; the direction that won used eight, with sixteen and twenty-four as its
/// commonest gaps.
pub const UNIT: f32 = 8.0;

/// `units` whole units of the rhythm.
///
/// **Takes an integer so a half-step cannot be written.** `gap(1.5)` is not a value this application
/// has; it is a four-unit rhythm mislabelled, and the type is what says so.
pub const fn gap(units: u32) -> f32 {
    UNIT * units as f32
}

/// Install the rhythm into **every** theme slot: `item_spacing` to zero, so a stated gap is the gap.
///
/// Every slot for [`crate::typography::install`]'s reason — `Style` is per-theme, spacing is not, and
/// writing to all of them makes the active-slot trap of ADR-0030 §2 inapplicable rather than merely
/// avoided.
pub fn install(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| style.spacing.item_spacing = Vec2::ZERO);
}

/// A horizontal row whose controls are separated by `units` of the rhythm.
///
/// **This is what replaces the ambient horizontal gap**, and it exists so a row asks for a rhythm
/// rather than a number — [`crate::frame::column`]'s discipline, one value family over. Zeroing
/// `item_spacing` globally is what makes stated gaps true, and the cost is that a row of buttons would
/// otherwise touch; this pays it in one place instead of at eleven call sites holding a literal each.
pub fn row<R>(ui: &mut egui::Ui, units: u32, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap(units);
        add(ui)
    })
    .inner
}

/// [`row`], for a row allowed to wrap onto further lines.
///
/// The gap applies in **both** axes here, unlike [`row`]: once a row wraps, the distance between its
/// lines is as visible as the distance between its items, and leaving the vertical at zero glues the
/// wrapped lines together.
pub fn row_wrapped<R>(ui: &mut egui::Ui, units: u32, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(gap(units), gap(units));
        add(ui)
    })
    .inner
}

/// The item spacing an egui **composite** draws its own internals with — a combo-box dropdown, a
/// scroll area — restored locally inside the widget that needs it.
///
/// These are the parts of the interface this application does not lay out itself, so the global zero
/// leaves their rows touching. One unit in both axes is the rhythm's smallest step and is what they
/// are given; it is set through this function rather than inline so a reader can find every place the
/// global zero is stepped around.
pub fn composite_spacing() -> Vec2 {
    vec2(gap(1), gap(1))
}

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;
    use egui::{Theme, ThemePreference};

    /// The unit is a decision and ADR-0032 quotes it.
    #[test]
    fn the_unit_is_the_number_the_adr_records() {
        assert_eq!(UNIT, 8.0);
    }

    /// Gaps are whole units, and the commonest ones are the values the direction was judged with.
    #[test]
    fn a_gap_is_whole_units_of_the_rhythm() {
        assert_eq!(gap(0), 0.0);
        assert_eq!(gap(1), 8.0);
        assert_eq!(gap(2), 16.0);
        assert_eq!(gap(3), 24.0);
    }

    /// **The load-bearing assertion of this module.** A stated gap is only the drawn gap while the
    /// ambient is zero; the moment `item_spacing` is non-zero again, every gap in the application is
    /// wrong by that amount and every width partition overruns its column — silently, which is how
    /// the editor came to hang 8px past its own page frame.
    #[test]
    fn install_zeroes_item_spacing_in_every_theme_slot() {
        let ctx = egui::Context::default();
        // Launch "in light mode" — the condition an untargeted setter misfires on (ADR-0030 §2).
        ctx.set_theme(ThemePreference::Light);

        install(&ctx);

        for theme in [Theme::Dark, Theme::Light] {
            assert_eq!(
                ctx.style_of(theme).spacing.item_spacing,
                Vec2::ZERO,
                "{theme:?} still adds an invisible gap before every stated one"
            );
        }
    }

    /// Stock egui is what this replaces, and it is non-zero in both axes — recorded so the test above
    /// is visibly guarding against something rather than asserting a default.
    #[test]
    fn stock_item_spacing_is_what_this_removes() {
        assert_eq!(egui::Style::default().spacing.item_spacing, vec2(8.0, 3.0));
    }

    /// **A row of *n* items with *n−1* stated gaps must fit exactly.** This is the arithmetic that was
    /// false before the ambient was zeroed, and it is the whole reason the editor's two panes hung
    /// past their frame. Expressed as the editor's own split so the regression has a name.
    #[test]
    fn a_stated_split_fits_its_column_exactly() {
        let available = crate::frame::TWO_COLUMN_MEASURE;
        let gutter = crate::frame::PANE_GUTTER;
        let each = (available - gutter) / 2.0;

        // Two panes and the gutter between them, plus the ambient egui would have added between each
        // pair of items — which must now be nothing.
        let ambient = Vec2::ZERO.x;
        let drawn = each + ambient + gutter + each;

        assert_eq!(
            drawn,
            available,
            "the split overruns its column by {}px",
            drawn - available
        );
    }

    /// The page frame is a different value family and is **not** on the rhythm's grid. Pinned so the
    /// mismatch reads as the decision it is (ADR-0032 §3) rather than as a defect someone tidies.
    #[test]
    fn the_page_margin_is_deliberately_off_the_grid() {
        let margin = crate::frame::PAGE_MARGIN;
        assert!(
            margin % UNIT != 0.0,
            "the page margin is now a whole number of units, so ADR-0032 §3's carve-out reads as \
             an oversight rather than a decision — re-judge it rather than deleting the note"
        );
    }
}
