//! **The page frame** — the gutter between content and the window edge, and the widest a column is
//! ever drawn. Decided in [#131](https://github.com/amin-bf/cairn/issues/131) against thirty-two
//! captures of four candidate frames, preserved as the tag `prototypes/issue-131`.
//!
//! Before this module the app had **no frame at all**: content ran edge to edge at every width, so
//! 1280px of window bought 1280px of button, a card with one word centred in it, and a Settings
//! paragraph drawn as one 150-character line. #124 found, while judging something else, that a
//! margin and a capped measure account for most of the visible distance between the baseline and
//! every Review variant it tried — and that neither was on the foundations list it was written with.
//!
//! # The two numbers live here and nowhere else
//!
//! This module is to layout what `theme` is to colour: **one naming site**. ADR-0030 §1 put the
//! palette behind a single function because a `Color32::from_rgb` elsewhere "renders fine to the
//! author and drifts the palette one screen at a time, with nothing failing". A literal `28.0` of
//! horizontal padding or a hand-rolled `min(available, 640.0)` on some screen is the same defect
//! wearing different units, and #123 found the app is *already* paying that cost for spacing at
//! around sixty literal call sites.
//!
//! So: ask [`column`] for a frame, never a number. The two constants are `pub` for the tests that
//! pin them and for the one place that legitimately needs the raw measure — the editor's
//! side-by-side threshold — and not as an invitation to arithmetic at a call site.

use eframe::egui::{self, Align, Layout, vec2};

/// The gutter between content and the window edge, on both sides, at **every** width.
///
/// Nearly invisible at 1280 and the whole difference at 560 — which is the reverse of how a margin
/// and a measure are usually argued about. The 560×860 captures are where it is unarguable: without
/// it, text touches both window edges and full-width buttons bleed off the frame.
pub const PAGE_MARGIN: f32 = 28.0;

/// The widest a column is ever drawn, however wide the window gets.
///
/// **640, reused rather than invented.** It is the number the editor already broke at, and the app
/// gains nothing from carrying two neighbouring values that mean roughly the same thing. The
/// prototype proposed 620; the difference is 20px and the reuse is worth more than the 20px.
pub const MEASURE: f32 = 640.0;

/// Draw `add` inside the page frame: the margin on both sides, centred, and never wider than
/// [`MEASURE`].
///
/// **One arrangement, centred, at every width — on every destination.** #124 settled that for Review
/// and #131 extended it: at 1280 roughly half the window is empty *by design*, and the note list and
/// Settings want the same treatment a card does. The alternatives were photographed and lost — giving
/// rows and forms a wider column leaves every Settings section ragged between its button and its
/// paragraph, and letting them spend the whole window puts 1100px between a note's title and its
/// *Delete*.
pub fn column<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    frame_of_width(ui, MEASURE, add)
}

/// The same frame, at a caller-chosen cap. **One caller only** — the editor, which earns a wider
/// frame by putting two real columns in it (see [`TWO_COLUMN_MIN_WIDTH`]). A second caller reaching
/// for this is the frame eroding into a per-screen preference, which is exactly what #131 refused.
pub fn wide_column<R>(ui: &mut egui::Ui, cap: f32, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    frame_of_width(ui, cap, add)
}

fn frame_of_width<R>(ui: &mut egui::Ui, cap: f32, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let available = ui.available_width();
    // `max(80.0)` is not defensive rounding: on a window narrower than two margins the subtraction
    // goes negative, and a negative width silently inverts the centring rather than failing.
    let width = (available - PAGE_MARGIN * 2.0).min(cap).max(80.0);
    let side = ((available - width) / 2.0).max(0.0);
    ui.horizontal(|ui| {
        ui.add_space(side);
        ui.allocate_ui_with_layout(
            vec2(width, ui.available_height()),
            Layout::top_down(Align::Min),
            |ui| {
                ui.set_width(width);
                add(ui)
            },
        )
        .inner
    })
    .inner
}

/// The **window** width at or above which the note editor draws its two panes side by side.
///
/// This is measured against the window and never against a column, and the distinction is the whole
/// reason it is written down. The threshold it replaces tested `ui.available_width()`, which was the
/// window's width only because the app had no frame; the moment one exists that expression becomes
/// *the column's* width, and a 640px measure is not `>= 640` once the margin is taken off it. The
/// desktop would then have shown the phone's `Write | Cards` toggle at every window size, with
/// nothing failing and no test covering it.
///
/// 900 is [#124](https://github.com/amin-bf/cairn/issues/124)'s number for the same shape of
/// question — the width at which a two-column arrangement stops being cramped — reused rather than
/// invented, and it leaves each pane around 430px inside a 1280 window.
pub const TWO_COLUMN_MIN_WIDTH: f32 = 900.0;

/// The widest the editor's two-column frame is drawn. Two columns of a full [`MEASURE`] each would
/// want 1308px including the gutter, which does not fit the 1280 the design pass judges at — so the
/// pair is capped where it stops gaining and each pane simply takes half.
pub const TWO_COLUMN_MEASURE: f32 = 1120.0;

/// The gutter between the editor's two panes. One rhythm unit wider than the page margin would be
/// arbitrary; it is the page margin, so the eye reads the same gap between panes as between content
/// and window.
pub const PANE_GUTTER: f32 = PAGE_MARGIN;

/// The cap the current screen is drawn at — [`MEASURE`] everywhere, except the editor once the
/// window can hold its two columns.
///
/// **Read by the nav row and by the screen itself, so the two cannot disagree.** The reason the
/// chosen frame beat the others was that the nav pill and the content share one left edge; a nav
/// aligned to a column the screen beneath it is not using is precisely the defect the rejected
/// frames were rejected for, and it would arrive here silently the moment one caller hard-coded
/// `MEASURE` while the other asked a question.
///
/// The row therefore *does* shift when a note is opened on a wide window. That is a deliberate
/// trade and the smaller of two: the alternative — pinning the nav to one column while the editor
/// uses another — buys stillness by breaking the alignment on the screen where the eye has two
/// columns to line up against rather than one.
pub fn cap_for(editor_open: bool, window_width: f32) -> f32 {
    if editor_open && window_width >= TWO_COLUMN_MIN_WIDTH {
        TWO_COLUMN_MEASURE
    } else {
        MEASURE
    }
}

#[cfg(test)]
// **`assertions_on_constants` is the point here, not an oversight.** These tests assert
// relationships between compile-time constants precisely so that *changing a constant* is what
// breaks them — a test that could not be folded away would be a test of something else.
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    /// The frame's numbers are a decision, not a preference, and an ADR quotes them. Changing one
    /// should be an act that fails a test rather than a diff nobody notices.
    #[test]
    fn the_frame_is_the_numbers_the_adr_records() {
        assert_eq!(PAGE_MARGIN, 28.0);
        assert_eq!(MEASURE, 640.0);
        assert_eq!(TWO_COLUMN_MIN_WIDTH, 900.0);
    }

    /// **The measure must be under the two-column threshold, and by a real margin.** This is the
    /// trap #131 found: if the editor's side-by-side test were ever evaluated against a column
    /// rather than the window, a measure at or above the threshold would make it accidentally pass
    /// and a measure below it would make it always fail — and either way nothing errors. Pinning the
    /// relationship means a later ticket that moves the measure toward the threshold has to read
    /// this comment first.
    #[test]
    fn the_measure_cannot_be_confused_with_the_two_column_threshold() {
        assert!(
            MEASURE < TWO_COLUMN_MIN_WIDTH,
            "a measure at or above the two-column threshold makes the two indistinguishable"
        );
        assert!(
            TWO_COLUMN_MIN_WIDTH - MEASURE >= 200.0,
            "the two numbers are close enough to be mistaken for each other"
        );
    }

    /// Two panes at the two-column cap must each still clear the width at which a form is usable,
    /// and must fit inside the 1280 the design pass judges at once the margins are taken off.
    #[test]
    fn the_two_column_frame_fits_the_judging_width() {
        let pane = (TWO_COLUMN_MEASURE - PANE_GUTTER) / 2.0;
        assert!(pane >= 400.0, "each editor pane is cramped at {pane}px");
        assert!(
            TWO_COLUMN_MEASURE <= 1280.0 - PAGE_MARGIN * 2.0,
            "the two-column frame does not fit the judging width"
        );
    }
}
