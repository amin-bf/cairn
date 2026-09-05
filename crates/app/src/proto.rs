//! **The mark and the icon rule** — the throwaway prototype for
//! [#155](https://github.com/amin-bf/cairn/issues/155), split out of
//! [The Craft](https://github.com/amin-bf/cairn/issues/149).
//!
//! **This never merges into `main`.** It is preserved as the tag `prototypes/issue-155`, the repo's
//! standing convention (`AGENTS.md`, *Landing work*). Reachable from any clone without merging:
//!
//! ```sh
//! git show prototypes/issue-155:docs/design/prototype-155/README.md
//! git checkout prototypes/issue-155 -- crates/app/src/proto.rs
//! ```
//!
//! # What is already decided, and is therefore not a candidate here
//!
//! The route — **icons are glyphs in the font stack** — and the placement — **the mark goes large
//! above *All caught up.*** — were both settled by [The Craft](https://github.com/amin-bf/cairn/issues/149).
//! This prototype does not re-open either. The face is **built**: `crates/app/assets/CairnIcons-Regular.ttf`,
//! generated from the launcher's own monochrome drawable by `scripts/build-icon-face.py`, registered
//! into all three families by `fonts::install`, drawing in the running application through
//! `ui.visuals().text_color()` with no image, no crate and no layout code. That is the ticket's
//! first question answered before the sitting starts, and it is answered *yes*.
//!
//! What is left is what the ticket says is left: **what the mark weighs**.
//!
//! # The finding this prototype opened with, before any candidate was drawn
//!
//! **A glyph's line box is the family's, not the glyph's**, and it costs 63px of unchosen space.
//!
//! `ui.label` allocates `Fonts::row_height` — the tallest face in the family at that size. The icon
//! face declares a cap-height ascent and a shallow descent and gets no say in it: at a size of 150
//! the stones are **109px inside a 172px row**, sitting at the top, with 10px of air above them and
//! **53px below** before anything else is drawn. Then the stated gap is added to that.
//!
//! So a knob that set the mark's size would have been dragging *two* distances at once — the picture
//! and an invisible skirt under it that grows with it — and the gap the ADR ended up naming would
//! have been a number that is not the gap on the screen. That is ADR-0032 §2's *a stated gap is the
//! whole gap*, broken by a quantity nobody wrote down.
//!
//! `crate::icon` is the answer: allocate the **ink**, and paint the galley offset so the ink lands
//! in it. Inline — an icon beside the word it illustrates — the line box is exactly what you want,
//! which is why this is not a retreat from the route but the one case the route does not cover.
//!
//! # The knobs
//!
//! **Size and weight are both distances, so both are dragged** — #141's finding, applied a third
//! time. Neither has a candidate set worth photographing: the question is not *which of these three*
//! but *how big* and *how loud*, and a menu of three answers a question nobody asked.
//!
//! | knob | starts at | what the sitting is reading off it |
//! |---|---|---|
//! | **size** | 150 | the font size, and the height of stones it actually draws (0.72 of it) |
//! | **gap** | `gap(3)` | the rhythm units between the stones and *All caught up.* |
//! | **ink** | 255 | the alpha on `text_color()` — **153 is `weak_text_color()`** |
//!
//! **The ink knob is an alpha rather than a choice of two roles, and that is deliberate.** The two
//! roles the palette already has are `text_color()` at full and `weak_text_color()` at 0.6, which is
//! **153**. Both are marked on the readout. If the thumb lands on one of them the answer is *an
//! existing role serves*, which is the cheapest outcome available and cannot be reached by a menu
//! that assumed it. If it lands between them, the mark needs a weight of its own and the ADR has the
//! number.
//!
//! # The two toggles
//!
//! **Where the block sits** is a question the mark creates rather than one it inherits. Today the
//! statement hangs at `gap(8)` under the heading, with the rest of the page empty; that reads as a
//! sentence tucked under a title when the sentence is all there is, and it reads differently once
//! there is a 109px picture above it. So: today's anchor, or optically centred in the page room.
//!
//! **Where the entrance sits** is [ADR-0035 §1](../../../docs/adr/0035-the-vertical-anchor.md), and
//! this screen is the first with cause to apply or ignore it anywhere but Review. §1 says *the last
//! control on **a screen** sits on a reach line*; `frame::slack_above` has exactly one call site and
//! it is `screens/review.rs`'s grade cluster. The caught-up screen with a leech has a control under
//! it — ADR-0034 §2's durable entrance — and draws it **directly under the statement, at y=252 of
//! 800**. One of the two has to give, and the toggle is which.
//!
//! Reach it with the leech fixture, which is the only state where this screen carries a control:
//!
//! ```sh
//! cargo build -p cairn-desktop && ./target/debug/cairn-fixture leeches
//! ```

use std::sync::Once;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::{field_label, frame, spacing, typography};

/// **The knobs' starting positions, settable from the environment**, so the capture harness can
/// photograph a ladder without choreographing a drag through `xdotool`.
///
/// The sitting is still the point — a size and a weight are judged by dragging them, not by picking
/// between stills. What this is for is the *other* half: a ladder in both themes at both widths is
/// what makes the sitting's answer checkable afterwards, and a still of a knob position nobody can
/// reproduce is worth nothing.
///
/// `CAIRN_PROTO=size,gap,ink,centred,reach` — e.g. `CAIRN_PROTO=110,3,153,1,1`. Read once, on the
/// first frame that asks.
fn from_env() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let Ok(spec) = std::env::var("CAIRN_PROTO") else {
            return;
        };
        let fields: Vec<&str> = spec.split(',').collect();
        let number = |i: usize| fields.get(i).and_then(|f| f.trim().parse::<u32>().ok());
        if let Some(v) = number(0) {
            SIZE.store(v.clamp(SIZE_MIN as u32, SIZE_MAX as u32), Ordering::Relaxed);
        }
        if let Some(v) = number(1) {
            GAP.store(v.min(GAP_MAX), Ordering::Relaxed);
        }
        if let Some(v) = number(2) {
            INK.store(v.min(255), Ordering::Relaxed);
        }
        if let Some(v) = number(3) {
            CENTRED.store(v != 0, Ordering::Relaxed);
        }
        if let Some(v) = number(4) {
            ON_REACH_LINE.store(v != 0, Ordering::Relaxed);
        }
    });
}

// --- the size knob --------------------------------------------------------------------------------

/// **150** — no argument behind it, and that is the honest starting point for a knob. It is roughly
/// the size at which the stones stop reading as a bullet and start reading as a picture, which is a
/// place to begin dragging from rather than a proposal.
static SIZE: AtomicU32 = AtomicU32::new(150);

/// The knob's ends. The bottom is `DISPLAY`, which is the size the ticket said not to assume and
/// therefore the one the sitting should be able to see and reject; the top is a mark taller than the
/// sentence under it by a factor of five, which is past anything anyone would choose.
pub const SIZE_MIN: f32 = 40.0;
pub const SIZE_MAX: f32 = 300.0;

/// The size the mark is asked for this frame.
pub fn size() -> f32 {
    from_env();
    SIZE.load(Ordering::Relaxed) as f32
}

/// The height of stones that size actually draws — the glyph's ink is one cap height, so this is
/// 0.72 of the number the ADR would name. Both go in the readout, because the eye is judging this
/// one and the code names the other.
pub fn drawn_height() -> f32 {
    size() * 0.72
}

/// One pixel of drag is one pixel of size. The whole range is then 260px of travel, which crosses
/// the column in one sweep at either judging width — a size is judged coarsely and a finer scale
/// would only make the sweep longer.
pub fn drag_size(delta_x: f32) {
    if delta_x != 0.0 {
        let next = (size() + delta_x).clamp(SIZE_MIN, SIZE_MAX);
        SIZE.store(next as u32, Ordering::Relaxed);
    }
}

// --- the gap knob ---------------------------------------------------------------------------------

/// **`gap(3)`** — 24px, the gap ADR-0032 §2 uses to separate one thing from another thing.
static GAP: AtomicU32 = AtomicU32::new(3);

/// The knob's top. Eight units is 64px, which is the lead the statement already has under the
/// heading, so it is the largest gap on this screen and therefore the largest worth reaching.
pub const GAP_MAX: u32 = 8;

/// The units between the stones and the statement.
pub fn gap() -> u32 {
    from_env();
    GAP.load(Ordering::Relaxed)
}

/// **Dragged, but snapped to whole units**, and both halves of that matter. A gap is a distance, so
/// #141 says drag it; the rhythm admits only whole multiples of eight (ADR-0032 §2, and `spacing::gap`
/// takes an integer so a half-step will not compile), so a continuous knob would produce an answer
/// the application cannot express. 24px of travel per unit.
pub fn drag_gap(delta_x: f32) {
    if delta_x != 0.0 {
        let next = (gap() as f32 * 24.0 + delta_x).clamp(0.0, GAP_MAX as f32 * 24.0);
        GAP.store((next / 24.0).round() as u32, Ordering::Relaxed);
    }
}

// --- the ink knob ---------------------------------------------------------------------------------

/// **255** — `text_color()` unmodified, which is what the ticket predicted the mark would need and
/// therefore the claim the sitting is testing rather than the answer it is starting from.
static INK: AtomicU32 = AtomicU32::new(255);

/// The alpha `weak_text_color()` lands on: egui's `weak_text_alpha` is 0.6, and #143 found that the
/// same 0.6 weighs differently on a light ground than a dark one — so this is where the *dark* role
/// sits and the sitting has to look at both themes to know whether one number serves.
pub const WEAK_ALPHA: u32 = 153;

/// The mark's ink this frame: the ambient text colour at the knob's alpha.
///
/// **One construction, read off the ambient role**, which is the property the ticket asked to
/// confirm rather than assume: the same expression has to produce a mark that works in both themes,
/// or the mark needs a colour of its own and ADR-0030 §1 gains a family.
pub fn ink(visuals: &egui::Visuals) -> egui::Color32 {
    visuals
        .text_color()
        .gamma_multiply(ink_alpha() as f32 / 255.0)
}

/// The knob's value, for the readout and for the ADR.
pub fn ink_alpha() -> u32 {
    from_env();
    INK.load(Ordering::Relaxed)
}

/// One pixel of drag is one step of alpha. 255px of travel, and the two roles the palette already
/// has sit at 153 and 255 — a hundred pixels apart, which is enough to stop on either deliberately.
pub fn drag_ink(delta_x: f32) {
    if delta_x != 0.0 {
        let next = (ink_alpha() as f32 + delta_x).clamp(0.0, 255.0);
        INK.store(next as u32, Ordering::Relaxed);
    }
}

// --- the two toggles ------------------------------------------------------------------------------

/// Whether the mark-and-statement block is centred in the page room rather than anchored under the
/// heading at `gap(8)`.
static CENTRED: AtomicBool = AtomicBool::new(false);

pub fn centred() -> bool {
    from_env();
    CENTRED.load(Ordering::Relaxed)
}

/// The space above the block: today's fixed lead, or whatever centres it in what is left of the page.
///
/// **Optically centred, not arithmetically** — the block is placed at 40% of the slack rather than
/// 50%, because a thing centred by measurement in a page with a heading above it reads as sitting
/// low. That fraction is itself a thing the sitting can reject, and if it does, the answer is that
/// this screen wants a stated lead after all.
pub fn lead(ui: &egui::Ui, block: f32) -> f32 {
    if centred() {
        ((frame::page_room(ui) - block) * 0.4).max(spacing::gap(2))
    } else {
        spacing::gap(8)
    }
}

/// The height of the whole block, which [`lead`] needs **before** any of it is drawn.
///
/// Arithmetic rather than a remembered measurement, for the reason `grade_cluster_height` gives: a
/// composition that varied would need last frame's number and would carry a frame of lag.
pub fn block_height(ui: &egui::Ui) -> f32 {
    let row = |size: f32| {
        ui.ctx()
            .fonts_mut(|f| f.row_height(&egui::FontId::proportional(size)))
    };
    drawn_height()
        + spacing::gap(gap())
        + row(typography::DISPLAY)
        + spacing::gap(2)
        + row(typography::SMALL)
}

/// Whether the durable leech entrance sits on ADR-0035 §1's reach line rather than directly under
/// the statement.
static ON_REACH_LINE: AtomicBool = AtomicBool::new(false);

pub fn on_reach_line() -> bool {
    from_env();
    ON_REACH_LINE.load(Ordering::Relaxed)
}

/// The space above the entrance: today's `gap(3)`, or whatever lands its bottom edge on the reach
/// line. `frame::slack_above` already falls back to the stated gap on a page with no room, so the
/// second arm is §1 applied verbatim and nothing else.
pub fn entrance_lead(ui: &egui::Ui) -> f32 {
    if on_reach_line() {
        frame::slack_above(
            frame::page_room(ui),
            crate::controls::HEIGHT,
            spacing::gap(3),
        )
    } else {
        spacing::gap(3)
    }
}

// --- the switcher ---------------------------------------------------------------------------------

/// A horizontal drag surface with a live readout, drawn as one full-width row. #141's finding,
/// applied a third time, and #154's widget carried over unchanged.
fn knob(ui: &mut egui::Ui, readout: &str) -> f32 {
    let height = typography::BODY * 2.4;
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::click_and_drag(),
    );
    ui.painter().rect_filled(
        rect,
        egui::CornerRadius::same(2),
        crate::theme::control_fill(ui.visuals()),
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        readout,
        egui::FontId::proportional(typography::SMALL),
        ui.visuals().text_color(),
    );
    response.drag_delta().x
}

/// The switcher, drawn on Settings **directly under the heading**, above everything else.
///
/// Deliberately ugly and deliberately labelled: it is a harness control and nothing about it is
/// being judged. **Above Appearance**, for the reason #154 recorded and measured — that control's
/// sentence wraps at 560 and one at 1280, so everything below it sits 17px lower at the narrow
/// width, and anything a storyboard must click at both judging widths has to be above it.
pub fn switcher(ui: &mut egui::Ui) {
    field_label(
        ui,
        "PROTOTYPE #155 — the mark. Set it here, then go to Review with nothing due.",
    );
    ui.add_space(spacing::gap(1));

    let delta = knob(
        ui,
        &format!(
            "drag — size {} of {SIZE_MAX} ({:.0}px of stones)",
            size(),
            drawn_height()
        ),
    );
    drag_size(delta);

    ui.add_space(spacing::gap(1));
    let delta = knob(
        ui,
        &format!(
            "drag — gap to the sentence: gap({}) = {:.0}px",
            gap(),
            spacing::gap(gap())
        ),
    );
    drag_gap(delta);

    ui.add_space(spacing::gap(1));
    let delta = knob(
        ui,
        &format!(
            "drag — ink {} of 255   (weak_text_color is {WEAK_ALPHA}, text_color is 255)",
            ink_alpha()
        ),
    );
    drag_ink(delta);

    ui.add_space(spacing::gap(2));
    field_label(ui, "where the block sits");
    ui.add_space(spacing::gap(1));
    for (centre, label) in [
        (false, "today — gap(8) under the heading"),
        (true, "centred in the page room"),
    ] {
        if ui
            .selectable_label(centred() == centre, crate::text(ui, label))
            .clicked()
        {
            CENTRED.store(centre, Ordering::Relaxed);
        }
    }

    ui.add_space(spacing::gap(2));
    field_label(
        ui,
        "the leech entrance — ADR-0035 §1's second call site (needs the `leeches` fixture)",
    );
    ui.add_space(spacing::gap(1));
    for (reach, label) in [
        (false, "today — gap(3) under the statement"),
        (true, "ADR-0035 §1 — the bottom edge on the reach line"),
    ] {
        if ui
            .selectable_label(on_reach_line() == reach, crate::text(ui, label))
            .clicked()
        {
            ON_REACH_LINE.store(reach, Ordering::Relaxed);
        }
    }
}
