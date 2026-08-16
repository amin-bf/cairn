//! **The vertical anchor** — the throwaway prototype for
//! [#141](https://github.com/amin-bf/cairn/issues/141), graduated out of the fog by
//! [#125](https://github.com/amin-bf/cairn/issues/125)'s sitting on the physical handset.
//!
//! **This never merges into `main`.** It is preserved as the tag `prototypes/issue-141`, the repo's
//! standing convention (`AGENTS.md`, *Rules that are easy to break silently* 3). Reachable from any
//! clone without merging:
//!
//! ```sh
//! git show prototypes/issue-141:docs/design/prototype-141/README.md
//! git checkout prototypes/issue-141 -- crates/app/src/proto.rs
//! ```
//!
//! # Why this one is not a desktop binary
//!
//! Every prototype in this map so far has been a `cairn-desktop` bin driven by a capture script,
//! because every question so far could be settled from a pair of stills at 560×860 and 1280×800.
//! This one cannot be. The ticket's question only *exists* where there is leftover height — at the
//! 860px window every capture in the map was taken at, there is none — and its answer is a
//! judgement about **reach**, which a mouse cannot make. So the prototype is the application
//! itself, varied, built for the handset and cycled in the hand.
//!
//! `cargo apk` packages `cairn-app`'s cdylib, so there is no second Android binary to put this in;
//! the variants live behind this module and are switched from Settings. **The switcher is on
//! Settings and not on Review on purpose** — anything added to the review screen changes the
//! arrangement being judged, which is the whole subject of the ticket.
//!
//! # What is held constant
//!
//! Everything the four Review ADRs fixed. The frame ([ADR-0031](../../../docs/adr/0031-the-page-frame.md)),
//! the scale ([ADR-0032](../../../docs/adr/0032-the-type-scale-and-the-rhythm.md)), the card
//! ([ADR-0033](../../../docs/adr/0033-the-card.md)) and the controls
//! ([ADR-0034](../../../docs/adr/0034-the-controls.md)) are drawn through the shipped modules,
//! unchanged. **Nothing here resizes anything**: #125 found no target undersized, so every variant
//! draws the same 36px controls and the same 300px card, and varies only *where on the page they
//! sit*.
//!
//! # The candidates
//!
//! Two of them are the ticket's, in its words. The rest are the pair it implies once you notice
//! that *where the slack goes* and *where **Edit note** goes* are two questions rather than one.
//!
//! | | *Edit note* | the slack falls | the thumb zone holds |
//! |---|---|---|---|
//! | **A** today | below the grades | below everything | nothing |
//! | **B** bottom | below the grades | above the card | *Edit note* |
//! | **C** reach | above the card | between *Edit note* and the card | the grades |
//! | **D** split | below the grades | between the card and the grades | *Edit note* |
//! | **E** split, edit up | above the card | between the card and the grades | the grades |
//!
//! **B is the ticket's *reading order held*** — the existing stack, unchanged, pushed to the
//! bottom. **C is its *frequency maps to reach*** — the control pressed on every card gets the zone
//! the thumb owns, the one pressed rarely takes the worst reach. The two they bracket are the ones
//! that separate the question: D and E keep the card where reading wants it and move only the
//! *controls* down, which is the arrangement neither candidate proposes and which preserves
//! prompt → answer → grade adjacency **as an order** while breaking it as a *distance*.
//!
//! B and D both hand the bottom of the screen to *Edit note* — the rarest control on the screen —
//! which is the inversion the ticket complains about, drawn so it can be seen rather than argued.
//!
//! # The horizontal axis is a toggle, not a variant
//!
//! #125 found a second and independent problem: *Barely* and *Easy* sit at the two extremes of the
//! segmented row and flip between comfortable and a stretch depending on which hand holds the
//! phone. [`snug`] narrows the whole grade cluster to [`SNUG_FRACTION`] of the column and centres
//! it, bringing both extremes inside the thumb's arc — at the cost of a grade row that no longer
//! agrees with the card above it. It is a toggle because it composes with all five anchors, and
//! because it is a **symmetric** answer: the row is mirrored for a right-to-left prompt, so
//! anything that favours one edge cannot be right in both scripts.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

/// Where the Review slice's content sits vertically. See the module header for the table.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    /// What the application draws today: the stack top-anchored, all the slack below it.
    Today,
    /// The existing stack unchanged, pushed to the bottom — the ticket's *reading order held*.
    Bottom,
    /// *Edit note* to the top, the card between, the grades at the bottom — the ticket's
    /// *frequency maps to reach*.
    Reach,
    /// The card stays where reading wants it; only the controls come down.
    Split,
    /// [`Anchor::Split`] with *Edit note* lifted above the card, so the bottom belongs to the
    /// grades alone.
    SplitEditTop,
    /// **The second round's answer.** *Edit note* sits directly under the card — out of the thumb's
    /// zone without needing a reserved row above the card — and the four grades are **stacked**
    /// rather than segmented, because a thumb travels up and down freely and sideways badly.
    StackedEditUnderCard,
}

impl Anchor {
    /// The cycle order, which is also the order they are judged in.
    pub const ALL: [Anchor; 6] = [
        Anchor::Today,
        Anchor::Bottom,
        Anchor::Reach,
        Anchor::Split,
        Anchor::SplitEditTop,
        Anchor::StackedEditUnderCard,
    ];

    /// The letter the write-up and the switcher both name it by.
    pub fn letter(self) -> &'static str {
        match self {
            Anchor::Today => "A",
            Anchor::Bottom => "B",
            Anchor::Reach => "C",
            Anchor::Split => "D",
            Anchor::SplitEditTop => "E",
            Anchor::StackedEditUnderCard => "F",
        }
    }

    /// One line, for the switcher — what the eye should check on this variant.
    pub fn label(self) -> &'static str {
        match self {
            Anchor::Today => "A — today: everything up top",
            Anchor::Bottom => "B — the whole stack at the bottom",
            Anchor::Reach => "C — edit up, grades at the bottom",
            Anchor::Split => "D — card stays, controls at the bottom",
            Anchor::SplitEditTop => "E — D, with edit above the card",
            Anchor::StackedEditUnderCard => "F — edit under the card, grades stacked",
        }
    }

    /// Whether *Edit note* is drawn above the card rather than under the grades.
    pub fn edit_on_top(self) -> bool {
        matches!(self, Anchor::Reach | Anchor::SplitEditTop)
    }

    /// Whether *Edit note* rides directly under the card, leaving the anchored block to the grades.
    ///
    /// It answers the same question [`Anchor::edit_on_top`] does — keep the rarest control out of
    /// the zone the thumb owns — and pays less for it: above the card the control has to reserve an
    /// empty row before the reveal or the card moves when it is tapped, and under the card it
    /// simply appears, because nothing above it shifts.
    pub fn edit_under_card(self) -> bool {
        matches!(self, Anchor::StackedEditUnderCard)
    }

    /// Whether the four grades are stacked full-width rather than *Forgot* over a segmented row.
    ///
    /// **This reopens [ADR-0034 §1](../../../docs/adr/0034-the-controls.md)** and it is the second
    /// round's second finding: *a thumb travels up and down freely and sideways badly*. §1 chose the
    /// segmented row on an argument about the **scale** — four stacked controls read as four rungs
    /// of one ladder, which puts the failure grade on a scale it is not on — and reasoned its
    /// widths on a desktop window, where a pointer crosses 208px for nothing. Neither half of that
    /// is wrong; what §1 never had was a thumb, and the axis it chose is the axis a thumb is worst
    /// on. *Forgot* is still held apart, by a gap rather than by a change of shape.
    pub fn grades_stacked(self) -> bool {
        matches!(self, Anchor::StackedEditUnderCard)
    }

    /// Whether the card comes down with the controls, or stays where the reading order puts it.
    pub fn card_comes_down(self) -> bool {
        matches!(self, Anchor::Bottom | Anchor::Reach)
    }

    /// Whether anything is bottom-anchored at all.
    pub fn anchors_low(self) -> bool {
        !matches!(self, Anchor::Today)
    }
}

/// **F** — the third round opens on the arrangement the second one asked for.
static ANCHOR: AtomicUsize = AtomicUsize::new(5);
static SNUG: AtomicBool = AtomicBool::new(false);

/// The anchor being drawn this frame.
pub fn anchor() -> Anchor {
    Anchor::ALL[ANCHOR.load(Ordering::Relaxed).min(Anchor::ALL.len() - 1)]
}

/// Choose the anchor. Called from the Settings switcher only.
pub fn set_anchor(anchor: Anchor) {
    let i = Anchor::ALL.iter().position(|a| *a == anchor).unwrap_or(0);
    ANCHOR.store(i, Ordering::Relaxed);
}

/// Whether the grade cluster is narrowed and centred — the horizontal axis.
pub fn snug() -> bool {
    SNUG.load(Ordering::Relaxed)
}

/// Toggle the horizontal axis. Called from the Settings switcher only.
pub fn set_snug(on: bool) {
    SNUG.store(on, Ordering::Relaxed);
}

/// How much of the column a narrowed grade cluster keeps.
///
/// The handset draws the column at 448dp less two 28dp margins — 392dp — so this is 282dp, which
/// puts both extremes of the segmented row about 55dp closer to the middle than they are today.
/// The number is a starting point for the sitting to move, not a measured one: the thumb's arc is
/// what it is judged against and nobody has traced that arc.
pub const SNUG_FRACTION: f32 = 0.72;

/// The width the grade cluster is drawn at, given the column it sits in.
pub fn grade_width(column: f32) -> f32 {
    if snug() {
        column * SNUG_FRACTION
    } else {
        column
    }
}

/// How much page is left below the cursor, measured against **the visible viewport**.
///
/// `ui.available_height()` is the obvious call and it returns **zero** here, which cost a capture
/// run to find. Inside a `ScrollArea` the content `Ui` is sized to its *content* — that is what
/// scrolling means — so "available" is what the widgets already drawn have claimed, not what the
/// screen has left. The clip rect is the viewport, and the viewport is what a thumb reaches.
pub fn page_room(ui: &egui::Ui) -> f32 {
    (ui.clip_rect().bottom() - ui.cursor().top() - BOTTOM_MARGIN).max(0.0)
}

/// The gutter an anchored block keeps between itself and the bottom of the page.
///
/// **This is a decision the prototype had to make to draw anything at all**, and it belongs in the
/// answer rather than in the harness. [ADR-0031](../../../docs/adr/0031-the-page-frame.md) fixed a
/// 28px gutter on the left and the right and never named a bottom one, because until something is
/// anchored low there is nothing to keep off the edge. Anchored without it, the grade row sits
/// flush against the window on the desktop and against the gesture bar's band on the handset, which
/// reads as content that overran rather than content that was placed.
///
/// It is [`crate::frame::PAGE_MARGIN`] and not a number of its own: the eye reads the same gap
/// below the content as it does beside it, and a second neighbouring value would be the drift
/// `frame`'s one-naming-site rule exists to refuse.
pub const BOTTOM_MARGIN: f32 = crate::frame::PAGE_MARGIN;

/// The height a page needs before any of this means anything.
///
/// **The fallback is the point, not defensiveness.** Below this the page has no leftover height to
/// place, so an anchored variant would either draw on top of the heading or invent a scroll range —
/// and on the 860px window every capture in this map was taken at, that is the state. Every variant
/// falls back to `Today` there, which is also the honest answer: the question the ticket asks does
/// not exist on a screen with no slack in it.
pub const MIN_ANCHORABLE_HEIGHT: f32 = 560.0;

// --- last frame's measured height -----------------------------------------------------------------
//
// Bottom-anchoring in immediate mode needs the height of a block *before* it is drawn. The two ways
// to get it are a bottom-up layout, which nests badly around `spacing::row` and the segmented row's
// width arithmetic, and remembering what the block measured last frame — which is what this does.
//
// The lag is one frame, and it is visible in exactly one place: the frame a card is revealed on,
// where the block grows by the grades. A prototype is allowed to have that; a decision that comes
// out of this and ships would want the layout done properly.

/// One slot per (anchor, revealed) pair, so the reveal's growth does not read as the unrevealed
/// block's height.
static MEASURED: [AtomicU32; 16] = [const { AtomicU32::new(0) }; 16];

fn slot(revealed: bool) -> usize {
    ANCHOR.load(Ordering::Relaxed).min(7) * 2 + usize::from(revealed)
}

/// Record what the anchored block actually measured this frame.
pub fn remember(revealed: bool, height: f32) {
    MEASURED[slot(revealed)].store(height.to_bits(), Ordering::Relaxed);
}

/// The space to leave above the anchored block so it lands [`lift`] above the bottom of the page.
///
/// Zero on the first frame of a variant — the block simply draws where it would have anyway, and
/// the frame after puts it in place.
pub fn slack(revealed: bool, room: f32) -> f32 {
    let measured = f32::from_bits(MEASURED[slot(revealed)].load(Ordering::Relaxed));
    if measured <= 0.0 {
        return 0.0;
    }
    (room - measured - clamped_lift(room, measured)).max(0.0)
}

// --- the lift -------------------------------------------------------------------------------------
//
// **The second round's axis, and it exists because the first round's answer was "not that far".**
// Bottom-anchoring was judged in the hand and the verdict was that the card belongs up top and the
// grades at the very bottom are *still* a stretch — which makes the remaining question a distance,
// and nobody has a number for it. Rather than guess three and photograph them, the cluster is
// **dragged** into place and the number is read off afterwards.
//
// This is the harness half of the ticket, not a candidate: nothing shipping is draggable.

/// **134** — where the second round's thumb put it, kept as the starting point so a rebuild does
/// not throw away a placement that took a sitting to find. The lift is measured from the bottom, so
/// a taller cluster keeps the *bottom edge* the thumb chose and grows upward into the slack.
static LIFT: AtomicU32 = AtomicU32::new(1_124_466_688); // 134.0f32.to_bits()

/// How far above the bottom margin the anchored block currently sits.
pub fn lift() -> f32 {
    f32::from_bits(LIFT.load(Ordering::Relaxed))
}

/// The lift actually applied, held so the block can never ride up into the card.
fn clamped_lift(room: f32, measured: f32) -> f32 {
    lift().clamp(0.0, (room - measured - MIN_GAP).max(0.0))
}

/// The smallest gap kept between the card and the block, so a drag cannot close the two together
/// and make the grades read as part of the card.
const MIN_GAP: f32 = 24.0;

/// Move the block by a drag's vertical delta. Dragging **up** raises it, which is why the sign
/// flips: a finger moving toward the top of the screen reports a negative `y`.
pub fn drag_lift(delta_y: f32) {
    if delta_y != 0.0 {
        LIFT.store((lift() - delta_y).max(0.0).to_bits(), Ordering::Relaxed);
    }
}

/// The readout, drawn **into the empty space itself** so it costs the arrangement no room.
///
/// It is what makes the sitting produce a number rather than an impression: the screenshot taken
/// afterwards carries the distance the thumb chose, in the same units the ADR would name it in.
pub fn readout(ui: &egui::Ui, rect: egui::Rect, room: f32, revealed: bool) {
    let measured = f32::from_bits(MEASURED[slot(revealed)].load(Ordering::Relaxed));
    let lift = clamped_lift(room, measured);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        format!("drag to place — lift {lift:.0} of {room:.0}"),
        egui::FontId::proportional(crate::typography::SMALL),
        ui.visuals().weak_text_color(),
    );
}

/// The switcher, drawn at the top of Settings.
///
/// Deliberately ugly and deliberately labelled: it is a harness control, and nothing about it is
/// being judged. It sits here rather than on Review because a control added to the review screen
/// changes the arrangement the ticket is asking about.
pub fn switcher(ui: &mut egui::Ui) {
    use crate::{controls, field_label, spacing};

    field_label(ui, "PROTOTYPE #141 — the vertical anchor. Pick one, then go to Review.");
    ui.add_space(spacing::gap(1));

    let current = anchor();
    for candidate in Anchor::ALL {
        if ui
            .selectable_label(current == candidate, crate::text(ui, candidate.label()))
            .clicked()
        {
            set_anchor(candidate);
        }
    }

    ui.add_space(spacing::gap(2));
    let label = if snug() {
        "grades: narrowed and centred"
    } else {
        "grades: the full column"
    };
    if controls::wide(ui, label).clicked() {
        set_snug(!snug());
    }
}
