//! The reserved band, and the guards that keep it from oscillating (ADR-0025 §1 §3, ADR-0026 §4).
//!
//! `platform` answers *what is covered*. This decides *what the application does about it*, and the
//! two are separate because the second is where every subtle failure lives.
//!
//! **Reserving the band is not the whole instruction, and an implementation that stops there is
//! visibly broken.** A `TextEdit` publishes its IME output only while its rect is visible; the
//! windowing adapter turns the *absence* of that output into hide-the-keyboard; hiding the keyboard
//! collapses the inset; collapsing the inset restores the viewport; which shows the field again,
//! which raises the keyboard. On the handset that closed loop is a continuously flickering keyboard.
//! Two of the three guards below exist to break it — from each end — and the third is the recovery
//! half of the vendored adapter patch.
//!
//! Both loop-breaking guards are here rather than in the editor because **every screen with a text
//! field inherits them**, which is why ADR-0025 §3 writes them as rules.

use crate::platform::{self, Insets};

/// A little air between the focused field and the top of the keyboard, so the caret is not flush
/// against it.
const CARET_CLEARANCE: f32 = 8.0;

/// A band change smaller than this is animation noise, not a keyboard arriving. Used to tell a
/// *growing* band from a steady one, which is what scopes guard 1 (see [`Band::keep_focus_visible`]).
const BAND_EPSILON: f32 = 0.5;

/// The band the platform's chrome and keyboard are sitting on, and the frame state the guards need.
///
/// Held by the application across frames because two of the guards are differential: one fires only
/// while the band is *growing*, and both read the previous frame's geometry — the focused field's
/// rect and the viewport — since they must act *before* this frame lays anything out.
#[derive(Debug)]
pub struct Band {
    /// What the platform said this frame.
    insets: Insets,
    /// Last frame's reserved bottom band in points, so a growing band can be told from a steady one.
    prev_bottom_pts: f32,
    /// Live scroll offset, read back each frame so a forced offset is expressed relative to where
    /// the user actually is.
    scroll_offset: f32,
    /// Set for exactly one frame, to pin the offset *before* layout. See [`Band::keep_focus_visible`].
    forced_scroll: Option<f32>,
    /// Last frame's visible viewport, in screen coordinates.
    viewport: egui::Rect,
}

impl Default for Band {
    fn default() -> Self {
        Self {
            insets: Insets::default(),
            prev_bottom_pts: 0.0,
            scroll_offset: 0.0,
            forced_scroll: None,
            viewport: egui::Rect::NOTHING,
        }
    }
}

impl Band {
    /// Read the platform and return the bands to reserve, in **egui points**.
    ///
    /// Re-read every frame, never cached at focus time: the keyboard animates in and the platform
    /// reports the current frame of that animation. A repaint is requested whenever the value moves,
    /// because immediate mode only repaints on input and a keyboard sliding up is not input as far
    /// as the windowing layer is concerned — without it the reflow lands one stray touch later.
    pub fn read(&mut self, ctx: &egui::Context) -> Bands {
        let previous = self.insets;
        self.insets = platform::insets();
        if self.insets != previous {
            ctx.request_repaint();
        }
        let ppp = ctx.pixels_per_point();
        Bands {
            top: self.insets.top / ppp,
            bottom: self.insets.bottom_occluded() / ppp,
        }
    }

    /// Whether the soft keyboard is **up** right now, from the insets [`Band::read`] last took.
    ///
    /// The navigation shell reads this to decide whether to pin itself (`ui` `CONTEXT.md`'s
    /// *Top-level destination*), and the question is deliberately *is it up* rather than *is it
    /// down*: the two are not complements. A platform with **no** soft keyboard answers `false` to
    /// both, which is the distinction [`SoftKeyboard`] exists to carry (ADR-0026 §5) — so the shell
    /// is simply always pinned off Android, from the same rule rather than from a second one.
    ///
    /// It reads the stored insets rather than the platform, so it is free and agrees with the bands
    /// this frame actually reserved. Call it after [`Band::read`].
    pub(crate) fn keyboard_is_up(&self) -> bool {
        self.insets.keyboard.is_up()
    }

    /// **Guard 1** — keep the focused field inside the viewport, *in the same frame it shrinks*.
    ///
    /// Not a nicety. Reserving the band clips the focused field, which stops its IME output, which
    /// the adapter turns into hide-the-keyboard, which collapses the inset, which restores the
    /// viewport, which shows the field, which raises the keyboard again. The fix is to make the
    /// antecedent false: the focused field never leaves the viewport, so the output never lapses.
    ///
    /// **The timing is the guard.** Asking for a scroll lands a frame later, and one frame without
    /// the output is one hide — so this computes an offset to be applied *before* layout, and the
    /// caller must run it before reserving the band.
    ///
    /// **Scoped to a growing band**, via `bands.bottom` against the previous frame's. Run every
    /// frame it would drag the focused field back the instant the user scrolled away from it — a
    /// different bug wearing the same fix, and the one guard 2 exists to handle instead.
    pub fn keep_focus_visible(&mut self, ctx: &egui::Context, bands: Bands, viewport_bottom: f32) {
        if bands.bottom <= self.prev_bottom_pts + BAND_EPSILON {
            self.prev_bottom_pts = bands.bottom;
            return;
        }
        self.prev_bottom_pts = bands.bottom;

        let Some((_, rect)) = focused_field(ctx) else {
            return;
        };
        if let Some(overshoot) = overshoot(rect, viewport_bottom, CARET_CLEARANCE) {
            self.forced_scroll = Some(self.scroll_offset + overshoot);
        }
    }

    /// **Guard 2** — surrender focus when the user scrolls a focused field **completely** out of
    /// view.
    ///
    /// The same loop as guard 1, entered from the other end: the field is clipped, so its IME output
    /// lapses, so the keyboard hides, so the band collapses, so the viewport grows and the field is
    /// on screen again — and the keyboard comes straight back.
    ///
    /// Dragging it back is wrong here, because scrolling away is deliberate. Surrendering focus makes
    /// the state consistent instead: no focus, no IME output, no keyboard, nothing to oscillate
    /// between. Tapping any field brings it back, which is what guard 3 is for.
    ///
    /// **Completely** out of view, not merely clipped — a field half off the edge is still being
    /// typed into.
    pub fn settle_focus_scrolled_away(&self, ctx: &egui::Context) {
        let Some((focused, rect)) = focused_field(ctx) else {
            return;
        };
        if fully_out_of_view(rect, self.viewport) {
            ctx.memory_mut(|m| m.surrender_focus(focused));
        }
    }

    /// The scroll area for this frame, carrying guard 1's forced offset when there is one.
    ///
    /// The offset is **set**, not requested: `scroll_to_rect` and friends land a frame later, and one
    /// frame with the focused field outside the viewport is one hide.
    pub fn scroll_area(&mut self) -> egui::ScrollArea {
        let area = egui::ScrollArea::vertical().auto_shrink([false, false]);
        match self.forced_scroll.take() {
            Some(offset) => area.vertical_scroll_offset(offset),
            None => area,
        }
    }

    /// Record where the scroll area ended up, for the next frame's guards.
    pub fn record<R>(&mut self, out: &egui::scroll_area::ScrollAreaOutput<R>) {
        self.scroll_offset = out.state.offset.y;
        self.viewport = out.inner_rect;
    }
}

/// The bands to reserve this frame, in egui points.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Bands {
    /// The status bar or display cutout. Unreserved, the application draws its first line of text
    /// under the clock — true of this application before ADR-0025 and a defect in its own right.
    pub top: f32,
    /// What the bottom is costing: the keyboard when it is up, the gesture bar when it is not.
    pub bottom: f32,
}

impl Bands {
    /// Whether a band is worth reserving at all. Below half a point it is noise, and an exact-size
    /// panel of zero is a separator nobody asked for.
    pub fn is_worth_reserving(size: f32) -> bool {
        size > BAND_EPSILON
    }
}

/// The focused **text field** and last frame's rect for it, in screen coordinates — which is what
/// the guards need, since they run before the widget is laid out again.
///
/// **A text field, not any focused widget**, and both guards are narrowed by it. What they exist to
/// break is the IME loop, and only a text field publishes the IME output that drives it — so a
/// focused button scrolled out of view is nothing to act on. Widening this would make the desktop
/// silently drop keyboard focus whenever a focused control scrolled away, which is a behaviour
/// change nothing here asked for.
///
/// A widget's text-edit state is what identifies it: `TextEdit` stores one under its own id, and
/// nothing else does.
fn focused_field(ctx: &egui::Context) -> Option<(egui::Id, egui::Rect)> {
    let focused = ctx.memory(|m| m.focused())?;
    egui::text_edit::TextEditState::load(ctx, focused)?;
    let rect = ctx.read_response(focused)?.rect;
    Some((focused, rect))
}

/// How far the scroll offset must move to bring `field`'s bottom edge — plus `clearance` — back
/// inside a viewport ending at `viewport_bottom`. `None` when it is already inside.
fn overshoot(field: egui::Rect, viewport_bottom: f32, clearance: f32) -> Option<f32> {
    let overshoot = field.bottom() + clearance - viewport_bottom;
    (overshoot > BAND_EPSILON).then_some(overshoot)
}

/// Whether `field` has left `viewport` **entirely**, top or bottom.
///
/// Clipped is not gone: a field half off the bottom edge is still being typed into, and surrendering
/// focus there would take the keyboard away mid-sentence.
fn fully_out_of_view(field: egui::Rect, viewport: egui::Rect) -> bool {
    if viewport == egui::Rect::NOTHING {
        return false;
    }
    field.bottom() < viewport.top() || field.top() > viewport.bottom()
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Rect, pos2};

    fn rect(top: f32, bottom: f32) -> Rect {
        Rect::from_min_max(pos2(0.0, top), pos2(400.0, bottom))
    }

    /// The field is below the shrunken viewport, so the offset moves by exactly what it overhangs
    /// plus the caret clearance — no more, or a deliberate scroll is overridden further than the
    /// keyboard justifies.
    #[test]
    fn a_field_the_band_would_cover_is_pulled_back_by_what_it_overhangs() {
        // Viewport ends at 565; the field's bottom sits at 600.
        assert_eq!(overshoot(rect(560.0, 600.0), 565.0, 8.0), Some(43.0));
    }

    /// Already inside, with room for the caret: nothing moves. Guard 1 firing here would fight the
    /// user's own scroll on every frame the band is steady.
    #[test]
    fn a_field_already_inside_the_viewport_is_left_alone() {
        assert_eq!(overshoot(rect(100.0, 300.0), 565.0, 8.0), None);
    }

    /// **Completely** out of view, not merely clipped — ADR-0025 §3. A field half off the bottom
    /// edge is still being typed into, and surrendering focus there takes the keyboard away
    /// mid-sentence.
    #[test]
    fn a_field_clipped_by_the_edge_still_holds_focus_but_one_fully_gone_does_not() {
        let viewport = rect(0.0, 565.0);

        // Half off the bottom, and half off the top: still being typed into.
        assert!(!fully_out_of_view(rect(540.0, 620.0), viewport));
        assert!(!fully_out_of_view(rect(-20.0, 40.0), viewport));

        // Entirely past either edge.
        assert!(fully_out_of_view(rect(600.0, 680.0), viewport));
        assert!(fully_out_of_view(rect(-90.0, -10.0), viewport));
    }

    /// Before the first frame there is no viewport, and a widget cannot be judged against one that
    /// does not exist — every field would read as gone and lose focus on the frame it gained it.
    #[test]
    fn nothing_is_out_of_view_before_there_is_a_viewport() {
        assert!(!fully_out_of_view(rect(600.0, 680.0), Rect::NOTHING));
    }

    /// Guard 1 fires only while the band is **growing**. A steady band means the keyboard is already
    /// up and any scrolling is the user's, which guard 2 handles by surrendering focus — not by
    /// dragging the field back.
    #[test]
    fn the_focus_pin_is_scoped_to_a_growing_band() {
        let ctx = egui::Context::default();
        let mut band = Band::default();

        let reserving = |bottom| Bands { top: 0.0, bottom };

        // The band appears: 0 → 400pt.
        band.keep_focus_visible(&ctx, reserving(400.0), 565.0);
        assert_eq!(band.prev_bottom_pts, 400.0);

        // Steady, then shrinking: neither is a frame guard 1 may act on. With no focused widget
        // nothing would be forced anyway, so the state under test is the bookkeeping the scope
        // decision is made from.
        band.keep_focus_visible(&ctx, reserving(400.0), 565.0);
        assert_eq!(band.prev_bottom_pts, 400.0);
        band.keep_focus_visible(&ctx, reserving(0.0), 965.0);
        assert_eq!(band.prev_bottom_pts, 0.0);
        assert_eq!(band.forced_scroll, None);
    }

    /// The navigation shell yields only to a keyboard that is **actually up**, and the three states
    /// are not two (`ui` `CONTEXT.md`'s *Top-level destination*, ADR-0026 §5).
    ///
    /// This is the silent half of the pinning rule. *Absent* and *Down* must answer alike — the row
    /// is pinned in both — so a platform with no soft keyboard gets a permanently pinned shell from
    /// the **same** rule rather than from a platform branch. Written as `!is_down()` instead, the
    /// desktop would answer *"the keyboard is not down"* and hide the only way out of a destination,
    /// forever, with nothing failing anywhere. That is the exact collapse ADR-0025 §2 first shipped
    /// and ADR-0026 §5 corrected, arriving here through a different caller.
    #[test]
    fn the_shell_yields_only_to_a_keyboard_that_is_actually_up() {
        let mut band = Band::default();

        band.insets.keyboard = platform::SoftKeyboard::Absent;
        assert!(!band.keyboard_is_up(), "no soft keyboard: the shell stays");

        band.insets.keyboard = platform::SoftKeyboard::Down;
        assert!(!band.keyboard_is_up(), "keyboard down: the shell stays");

        band.insets.keyboard = platform::SoftKeyboard::Up { height: 400.0 };
        assert!(
            band.keyboard_is_up(),
            "keyboard up: the shell yields its band to the form pane's first screen"
        );
    }

    /// Off Android the seam reports no keyboard and no bars, so there is nothing to reserve and the
    /// layout is the one this crate had before the seam existed. The *raise* gate is the seam's own
    /// guarantee and is tested there (ADR-0026 §5).
    ///
    /// The `cfg` is **not** client-stack rule 3's defect signal: it gates a *test* to the arm whose
    /// behaviour it asserts, exactly as `cairn-export`'s desktop-seam test does. Nothing here
    /// varies at run time.
    #[cfg(not(target_os = "android"))]
    #[test]
    fn a_platform_without_a_soft_keyboard_reserves_nothing() {
        let ctx = egui::Context::default();
        let mut band = Band::default();
        let bands = band.read(&ctx);

        assert_eq!(
            bands,
            Bands {
                top: 0.0,
                bottom: 0.0
            }
        );
        assert!(!Bands::is_worth_reserving(bands.top));
        assert!(!Bands::is_worth_reserving(bands.bottom));
    }
}
