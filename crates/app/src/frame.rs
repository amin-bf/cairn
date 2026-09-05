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
//! pin them, and not as an invitation to arithmetic at a call site.
//!
//! **There is no third number, and #163 is why.** The editor carried a `TWO_COLUMN_MIN_WIDTH` from
//! #131 — the width at which it stopped folding its two panes into a toggle — and the toggle exists
//! because a soft keyboard eats *height*, so the test named one axis and decided about the other.
//! Judged on a live window dragged down by hand, two columns stayed readable at **118px per pane**,
//! which is a third of the narrowest case anyone had argued for: narrowness is a gradient here and a
//! gradient has no threshold in it. So the width was deleted rather than moved, and the arrangement
//! asks [`editor_is_side_by_side`] instead. The app now has **no width-driven arrangement change
//! anywhere** — the frame is one arrangement at every width (#131), Review refused a second
//! breakpoint (#124), and this was the last one standing.

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
/// frame by putting two real columns in it (see [`editor_is_side_by_side`]). A second caller reaching
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

/// How far above the bottom of the page the last control on a screen sits, when the page is tall
/// enough to place it there.
///
/// **The frame's third number, and the first one a thumb chose.** #131 fixed the two horizontal
/// numbers and left the vertical unasked, because at the 860px window every capture in the design
/// pass was taken at there is no leftover height to place. On a handset there are ~1100px of it, and
/// [#125](https://github.com/amin-bf/cairn/issues/125) found it all pooled below the content, where
/// it costs the reach of every control on the screen.
///
/// **165 is measured, not chosen.** In two sittings on a Pixel 8 Pro the control cluster was dragged
/// into place by thumb, once at 148px tall and once at 184px, and its **bottom edge** landed at 162
/// and 169 — a round apart, with different contents, converging within 7px. So what the thumb picks
/// is a *line above the bottom of the page*, not a gap below the card, and the cluster grows upward
/// from it. That is why this is one number rather than a gap plus a cluster height.
///
/// It is an absolute distance rather than a fraction of the page on purpose: the band it clears is
/// where the hand **grips**, and a grip is physical. A proportion would put the line too high on a
/// tall screen and too low on a short one.
pub const REACH_LINE: f32 = 165.0;

/// How much page is left below the cursor, measured against the **visible viewport**.
///
/// **`Ui::available_height` is the obvious call and it returns zero here.** Inside a `ScrollArea`
/// the content `Ui` is sized to its *content* — that is what scrolling means — so "available" is
/// what the widgets already drawn have claimed, never what the screen has left. The clip rect is the
/// viewport, and the viewport is what a thumb can reach. This was found by a prototype rendering
/// pixel-identically to the thing it was varying, with nothing failing.
///
/// The viewport already excludes the gesture bar: the inset band is a panel reserved *before* the
/// scroll area (`crate::keyboard`, ADR-0025 §1), so this measures usable page and not glass.
pub fn page_room(ui: &egui::Ui) -> f32 {
    (ui.clip_rect().bottom() - ui.cursor().top()).max(0.0)
}

/// The space to leave above a `block` of controls so its bottom edge lands on [`REACH_LINE`] —
/// falling back to `floor` on a page with no room to spare.
///
/// **The fallback is the rule's other half, not defensiveness.** A page too short to place the
/// cluster is a page with no leftover height, which is the state in which this ticket's question
/// does not arise: the controls simply follow the card, exactly as they did before. So one
/// expression covers the handset, the desktop window and everything between, with no breakpoint —
/// the gap absorbs whatever is left over and stops at the stated gap when there is nothing left.
/// Takes the room rather than the `Ui` so the rule is arithmetic a test can state: the one thing
/// worth pinning here is *where the bottom edge lands*, and a test that has to build a window to ask
/// would be testing egui.
pub fn slack_above(room: f32, block: f32, floor: f32) -> f32 {
    (room - block - REACH_LINE).max(floor)
}

/// Whether the note editor draws its two panes **side by side** — true wherever there is no soft
/// keyboard to eat the height, false wherever there is one.
///
/// **This replaced a width threshold, and #163 deleted the width rather than moving it.** The editor
/// carried `TWO_COLUMN_MIN_WIDTH = 900` from #131, and the case against it was never that 900 was the
/// wrong number: the test named a *width* and decided about *vertical* room. The `Write | Cards`
/// toggle exists because a soft keyboard takes 39% of a handset display
/// ([ADR-0025](../../../docs/adr/0025-the-authoring-screen-under-a-soft-keyboard.md) §4), and at 880
/// on a desktop there is no keyboard and 450px of height going spare — and it fired anyway.
///
/// **The width half was tested and there is nothing there.** ADR-0025 §5 had already written
/// *"the failure is vertical, and no width rule addresses it"*, and #131 left standing the worry that
/// two narrow columns might simply be unreadable, which would have given the threshold a reason it
/// did not state. #163 put that in front of a person on a live window dragged down by hand: at
/// **118px per pane** — a third of the narrowest case anyone had argued about — the card face wraps
/// to four lines and stays perfectly readable, nothing clips and nothing overflows. Narrowness
/// produces a *gradient*, not a failure, and a gradient has no threshold in it.
///
/// So there is no width at which the arrangement should change, and the app now has **no
/// width-driven arrangement change anywhere**: #124 refused a second breakpoint on Review, #131 made
/// the frame one arrangement at every width, and this was the last one standing — the one #122
/// recorded as *"the only arrangement change in the app"*.
///
/// **Read off `SoftKeyboard::exists`, which is this client's way of asking *is this operated by a
/// thumb***. [ADR-0035 §3](../../../docs/adr/0035-the-vertical-anchor.md) already reads it for the
/// same kind of question, and its reasoning carries: a platform that raises a keyboard on screen is a
/// platform with no pointer. It is a proxy rather than a truth, exact for the two targets that exist,
/// and stated as *touch* so the native clients coming later inherit the rule without inheriting
/// egui's way of noticing it. Not a compile-time capability constant, deliberately — the one that
/// exists (ADR-0015 §9) exists to make a limitation visible and never to vary behaviour.
///
/// It answers for the **platform**, not for the window, which is why nothing here takes a width.
pub fn editor_is_side_by_side() -> bool {
    !crate::platform::insets().keyboard.exists()
}

/// The widest the editor's two-column frame is drawn. Two columns of a full [`MEASURE`] each would
/// want 1308px including the gutter, which does not fit the 1280 the design pass judges at — so the
/// pair is capped where it stops gaining and each pane simply takes half.
pub const TWO_COLUMN_MEASURE: f32 = 1120.0;

/// The gutter between the editor's two panes. One rhythm unit wider than the page margin would be
/// arbitrary; it is the page margin, so the eye reads the same gap between panes as between content
/// and window.
pub const PANE_GUTTER: f32 = PAGE_MARGIN;

/// The cap the current screen is drawn at — [`MEASURE`] everywhere, except the editor wherever it is
/// drawing two columns.
///
/// **Read by the nav row and by the screen itself, so the two cannot disagree.** The reason the
/// chosen frame beat the others was that the nav pill and the content share one left edge; a nav
/// aligned to a column the screen beneath it is not using is precisely the defect the rejected
/// frames were rejected for, and it would arrive here silently the moment one caller hard-coded
/// `MEASURE` while the other asked a question.
///
/// The row therefore *does* shift when a note is opened on a pointer platform. That is a deliberate
/// trade and the smaller of two: the alternative — pinning the nav to one column while the editor
/// uses another — buys stillness by breaking the alignment on the screen where the eye has two
/// columns to line up against rather than one.
///
/// **It takes no width any more** (#163). It used to, and the window it was handed was the one thing
/// that made this and the screen able to disagree: two call sites, each measuring the window for
/// itself. Now both ask [`editor_is_side_by_side`], which measures nothing.
pub fn cap_for(editor_open: bool) -> f32 {
    if editor_open && editor_is_side_by_side() {
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
        assert_eq!(REACH_LINE, 165.0);
        // There is deliberately no fourth number here. `TWO_COLUMN_MIN_WIDTH` used to be one and
        // #163 deleted it rather than moving it — the editor's arrangement is a question about the
        // platform's input, not about the window, so there is no width left to pin.
    }

    /// **ADR-0035 §1, stated as the thing a thumb actually chose.** On a page with room, the
    /// cluster's *bottom edge* lands on the reach line — whatever the cluster contains and however
    /// tall it is. That invariant is the whole finding: two sittings placed clusters of 148 and 184
    /// and both put the bottom edge in the same place.
    ///
    /// Nothing fails when this drifts. Get the arithmetic wrong by the cluster's height and the
    /// screen still renders, still looks deliberate, and is simply wrong by 184px on the one axis
    /// the ADR is about.
    #[test]
    fn the_cluster_bottom_lands_on_the_reach_line_whatever_it_holds() {
        const ROOM: f32 = 800.0;
        const FLOOR: f32 = 24.0;
        for block in [148.0_f32, 184.0, 36.0] {
            let bottom_edge = ROOM - (slack_above(ROOM, block, FLOOR) + block);
            assert_eq!(
                bottom_edge, REACH_LINE,
                "a {block}px cluster should still end {REACH_LINE}px above the page bottom"
            );
        }
    }

    /// **The other half of the same rule**: a page with no leftover height places nothing, and the
    /// controls follow the card exactly as they did before this ADR.
    ///
    /// This is what makes it one rule rather than a breakpoint — the desktop window the design pass
    /// judges at reaches this arm by arithmetic, not by asking what it is running on.
    #[test]
    fn a_page_with_no_room_falls_back_to_the_stated_gap() {
        const FLOOR: f32 = 24.0;
        assert_eq!(slack_above(300.0, 184.0, FLOOR), FLOOR);
        assert_eq!(slack_above(0.0, 184.0, FLOOR), FLOOR);
        // And the boundary: exactly enough room for the cluster, the line and the gap.
        assert_eq!(slack_above(184.0 + REACH_LINE + FLOOR, 184.0, FLOOR), FLOOR);
    }

    /// **The arrangement asks the platform and never the window** — the one thing left to pin once
    /// the threshold is gone, and the trap it replaces.
    ///
    /// #131's trap was that the editor's side-by-side test could be evaluated against a *column*
    /// rather than the window and pass or fail accidentally, with nothing erroring; the test that
    /// used to sit here pinned `MEASURE` a safe distance below the threshold so the two could not be
    /// mistaken for each other. #163 removed the trap by removing the width: there is no expression
    /// left that could be handed the wrong one.
    ///
    /// What can still go wrong is subtler and this is what catches it — a later ticket reaching for
    /// `viewport_rect().width()` again, to add a floor, a tablet case or a landscape rule. Both
    /// answers here are about the *platform*, so on a desktop they hold at every window size, and a
    /// width creeping back in is what makes one of them move.
    #[test]
    fn the_arrangement_does_not_depend_on_how_wide_the_window_is() {
        // The desktop has no soft keyboard (`platform::desktop`), so the editor is side by side —
        // and stays so at every width, because no width is consulted. 118px per pane was judged by
        // hand and kept; the narrowest arm here is far below anything a window reaches.
        assert!(
            editor_is_side_by_side(),
            "a platform with no soft keyboard draws two columns"
        );
        assert_eq!(
            cap_for(true),
            TWO_COLUMN_MEASURE,
            "the editor takes the wide frame"
        );
        assert_eq!(
            cap_for(false),
            MEASURE,
            "every other screen takes the measure"
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
