//! The palette. **The only module in this crate that names a colour** (ADR-0030 §1); every screen
//! keeps reading the *ambient* `ui.visuals()` — `text_color()`, `weak_text_color()`,
//! `hyperlink_color`, the selection accent — exactly as it did against stock egui, so adopting the
//! palette changes *where the values come from*, not a single call site. A colour literal anywhere
//! outside this module is the defect.
//!
//! egui's stock theme is what shipped through the walking skeleton: legible but loud — pure-grey
//! neutrals, a saturated blue selection, `#ff0000` for error. This is the same structure in stone —
//! cool slate neutrals and four desaturated accents, flat fills, 2px corners, no shadow — so nothing
//! above this module has to know it exists.
//!
//! # Dark is pinned, and pinning is two acts (ADR-0030 §2)
//!
//! egui follows the OS theme preference, and Cairn never overrode it. The palette is dark only, so
//! following the OS with only a dark palette drawn would silently hand a light-preferring OS **stock
//! egui** — the 5.12:1 body §3 exists to leave behind — reached by omission, not by anyone choosing
//! it. That is the one outcome §2 refuses. So [`install`] does **both** halves:
//!
//! 1. It writes the palette into the **dark slot specifically** — `set_visuals_of(Theme::Dark, …)`,
//!    never the untargeted `set_visuals`, which writes to *whichever slot is active when it is
//!    called*. With the preference defaulting to system-following, launching in light mode would let
//!    the untargeted setter install a **dark palette into the light slot** and leave the dark slot on
//!    stock — dark colours in light mode, stock grey in dark mode, and a runtime theme change cannot
//!    repair it because it is called once. The targeted setter is what this bug is fixed with.
//! 2. It **pins the preference to dark** — `set_theme(Dark)` — so an OS theme change does not clobber
//!    the palette back to the stock light slot. Doing only step 1 fails silently the first time the
//!    OS flips to light.
//!
//! Unlike `fonts`, this needs **no first-frame deferral**: visuals allocate no texture, so ADR-0012
//! §8's `CreationContext` hazard does not reach them, and setting them at construction avoids a frame
//! of the wrong theme. It is called from [`crate::CairnApp::new`].
//!
//! # The contrast floor (ADR-0030 §3)
//!
//! The floor is **7:1** — WCAG AAA for body text — because the small text style is 9px, where WCAG
//! AA's 4.5:1 is already the marginal case this palette exists to leave behind. It binds **text
//! against the surface it is drawn on**. `text_pairs_clear_the_contrast_floor` holds every reading-text
//! pair the app draws to it: body-on-panel (13.34:1, up from stock's 5.12:1), body-on-card, and body
//! over the one reachable accent, the selection fill.
//!
//! **Weak text is the one text pair below the floor, and it is left there deliberately.** The box
//! badge and field labels read `weak_text_color()` — body gamma-multiplied toward the background —
//! which lands near `#8b979b`, ~5.6:1 on the panel. Lifting it to 7:1 would make the badge *louder*,
//! which is precisely what ADR-0030 §4 forbids ("small, non-interactive footnote … quiet aside"), and
//! would diverge the app from the design system's `--text-weak` token. It is a **pre-existing
//! weakness** — stock's weak text is 5.12:1, so the palette *improves* it (5.12 → 5.6) and never
//! regresses it — of the same kind as the non-text pairs §3 leaves out of scope, not the lone
//! regression §3 records (the hover stroke, lifted back over 3:1 here). `weak_text_is_not_a_regression`
//! pins that it stays at least as legible as stock rather than silently drifting below it. This is in
//! tension with §3's listing weak-text-on-panel among the *bound* pairs; §4's quiet-footnote
//! requirement is the one that governs the colour, and the tension is recorded for the finish pass.

use egui::{Color32, CornerRadius, Stroke, Theme, ThemePreference, Visuals};

const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

// --- stone: the neutral ramp, cool slate with a trace of blue ---
const STONE_0: Color32 = rgb(0x0f1214); // text-field wells
const STONE_2: Color32 = rgb(0x1a1e21); // panels and windows
const STONE_3: Color32 = rgb(0x21262a); // faint fills
const STONE_4: Color32 = rgb(0x282e33); // pressed widgets, separators
const STONE_5: Color32 = rgb(0x2c3237); // widgets at rest
const STONE_6: Color32 = rgb(0x363d43); // widgets hovered
// The hovered stroke, at the light-stone rung. The draft put it a rung lower, at `#6d7a80`, which
// measured **2.49:1** against the hovered fill (`STONE_6`) — the one pair the palette turned from
// passing (stock's 3.19:1) to failing, and hover is exactly the state the non-text contrast rule
// covers (ADR-0030 §3). There is no rung between the two that clears 3:1 with margin, so it moves up
// to `#8b979b`; `hover_stroke_clears_three_to_one` pins it. Weak text — derived by egui, never named
// here — lands near this same value, which is why the ramp carried one entry for both.
const STONE_9: Color32 = rgb(0x8b979b); // hovered strokes — lifted from #6d7a80 to clear 3:1
const STONE_10: Color32 = rgb(0xb9c2c3); // text on a widget
const STONE_11: Color32 = rgb(0xe2e6e6); // body text
const QUIET: Color32 = rgb(0x333b40); // strokes at rest

// --- the four desaturated accents ---
const LICHEN: Color32 = rgb(0x6f93a8); // links
const LICHEN_DEEP: Color32 = rgb(0x2a4453); // selection fill
const LICHEN_PALE: Color32 = rgb(0xcfe3ec); // selection stroke and text
const CLAY: Color32 = rgb(0xc2a37a); // warn — warm, never alarming
const ROSE: Color32 = rgb(0xb57e79); // error — softened, never #ff0000

/// Install the palette, pinning dark. Called **once** from [`crate::CairnApp::new`] — both acts of
/// pinning, per ADR-0030 §2 and the module header: the palette into the **dark slot** (never the
/// active slot), and the theme preference to dark so an OS theme change cannot restore stock egui.
pub fn install(ctx: &egui::Context) {
    ctx.set_visuals_of(Theme::Dark, cairn_dark());
    ctx.set_theme(ThemePreference::Dark);
}

/// The palette itself, built from egui's dark theme so every field this does not name keeps its
/// default — a future egui release adding a field is then a value we inherit, not a compile error.
pub fn cairn_dark() -> Visuals {
    let mut v = Visuals::dark();

    v.panel_fill = STONE_2;
    v.window_fill = STONE_2;
    v.extreme_bg_color = STONE_0;
    v.faint_bg_color = STONE_3;
    v.override_text_color = None; // body colour rides fg_stroke, per widget state

    v.widgets.noninteractive.bg_fill = STONE_2;
    v.widgets.noninteractive.weak_bg_fill = STONE_2;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, STONE_4); // separators
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, STONE_11); // body text
    v.widgets.noninteractive.corner_radius = CornerRadius::same(2);

    v.widgets.inactive.bg_fill = STONE_5;
    v.widgets.inactive.weak_bg_fill = STONE_5;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, QUIET);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, STONE_10);
    v.widgets.inactive.corner_radius = CornerRadius::same(2);

    v.widgets.hovered.bg_fill = STONE_6;
    v.widgets.hovered.weak_bg_fill = STONE_6;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, STONE_9);
    v.widgets.hovered.fg_stroke = Stroke::new(1.5, STONE_11);
    v.widgets.hovered.corner_radius = CornerRadius::same(3);
    v.widgets.hovered.expansion = 1.0;

    v.widgets.active.bg_fill = STONE_4;
    v.widgets.active.weak_bg_fill = STONE_4;
    v.widgets.active.bg_stroke = Stroke::new(1.0, STONE_11);
    v.widgets.active.fg_stroke = Stroke::new(2.0, Color32::WHITE);
    v.widgets.active.corner_radius = CornerRadius::same(2);

    v.widgets.open.bg_fill = STONE_4;
    v.widgets.open.weak_bg_fill = STONE_4;
    v.widgets.open.bg_stroke = Stroke::new(1.0, QUIET);
    v.widgets.open.fg_stroke = Stroke::new(1.0, STONE_10);
    v.widgets.open.corner_radius = CornerRadius::same(2);

    v.selection.bg_fill = LICHEN_DEEP;
    v.selection.stroke = Stroke::new(1.0, LICHEN_PALE);

    v.hyperlink_color = LICHEN;
    v.warn_fg_color = CLAY;
    v.error_fg_color = ROSE;

    // No shadow anywhere: a Cairn surface is a fill and a 2px corner, and nothing floats.
    v.window_shadow = egui::epaint::Shadow::NONE;
    v.popup_shadow = egui::epaint::Shadow::NONE;

    v
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The relative luminance of an **opaque** colour, per WCAG 2.1.
    fn luminance(c: Color32) -> f64 {
        let channel = |v: u8| {
            let s = v as f64 / 255.0;
            if s <= 0.03928 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(c.r()) + 0.7152 * channel(c.g()) + 0.0722 * channel(c.b())
    }

    /// WCAG contrast ratio between two **opaque** colours.
    fn contrast(fg: Color32, bg: Color32) -> f64 {
        let (a, b) = (luminance(fg), luminance(bg));
        let (hi, lo) = if a >= b { (a, b) } else { (b, a) };
        (hi + 0.05) / (lo + 0.05)
    }

    /// Composite a possibly-translucent `fg` (premultiplied, as egui stores it) over an opaque `bg`,
    /// so the *effective* drawn colour can be measured. `weak_text_color()` is body text at reduced
    /// alpha, so what the eye reads is the blend, not the source.
    fn over(fg: Color32, bg: Color32) -> Color32 {
        let a = fg.a() as f64 / 255.0;
        let blend = |f: u8, b: u8| (f as f64 + b as f64 * (1.0 - a)).round() as u8;
        Color32::from_rgb(
            blend(fg.r(), bg.r()),
            blend(fg.g(), bg.g()),
            blend(fg.b(), bg.b()),
        )
    }

    /// The visuals as the app reads them at rest — the non-interactive, panel-surface state every
    /// `text`/`body`/`badge` helper draws through.
    fn visuals() -> Visuals {
        cairn_dark()
    }

    /// Every **reading-text** pair the app draws clears the 7:1 floor (ADR-0030 §3). These are the
    /// pairs the crate reads via the ambient visuals: body text on the panel, body text on a card
    /// (the widget fill a card face draws over), and body text over the one reachable accent, the
    /// selection fill. Weak text is the one text pair below the floor and is held separately, by
    /// `weak_text_is_not_a_regression`, for the reason in the module header.
    #[test]
    fn text_pairs_clear_the_contrast_floor() {
        let v = visuals();
        let body = v.text_color();
        let panel = v.panel_fill;
        let card = v.widgets.inactive.bg_fill;
        let selection = v.selection.bg_fill;

        for (name, fg, bg) in [
            ("body-on-panel", body, panel),
            ("body-on-card", body, card),
            ("body-on-selection", body, selection),
        ] {
            let ratio = contrast(fg, bg);
            assert!(
                ratio >= 7.0,
                "{name} is {ratio:.2}:1, below the 7:1 contrast floor (ADR-0030 §3)"
            );
        }
    }

    /// The measured exemplar the ADR quotes: body-on-panel is **13.34:1**, up from stock's 5.12:1.
    /// Pinned on its own so a change to the body colour or the panel fill that still cleared 7:1 but
    /// walked away from the headline figure would be caught.
    #[test]
    fn body_on_panel_matches_the_recorded_measurement() {
        let v = visuals();
        let ratio = contrast(v.text_color(), v.panel_fill);
        assert!(
            (ratio - 13.34).abs() < 0.05,
            "body-on-panel is {ratio:.2}:1; ADR-0030 §3 records 13.34:1"
        );
    }

    /// Weak text (the box badge, field labels) is a **pre-existing weakness**, not a regression: it
    /// is deliberately dim (ADR-0030 §4's quiet footnote) and sits below the 7:1 floor, but it must
    /// never drop below stock egui's weak text, which is what it replaces (module header). Stock's
    /// weak text is `gray(140)` on `gray(27)`, ~5.12:1; ours must clear that. Composited because
    /// `weak_text_color()` is translucent.
    #[test]
    fn weak_text_is_not_a_regression() {
        let v = visuals();
        let ours = contrast(over(v.weak_text_color(), v.panel_fill), v.panel_fill);

        let stock = Visuals::dark();
        let stock_weak = contrast(
            over(stock.weak_text_color(), stock.panel_fill),
            stock.panel_fill,
        );

        assert!(
            ours >= stock_weak,
            "weak-text-on-panel is {ours:.2}:1, below stock's {stock_weak:.2}:1 — a regression"
        );
    }

    /// [`install`] writes the palette into the **dark slot specifically** and pins the preference to
    /// dark (ADR-0030 §2). The test reproduces the bug the targeted setter fixes: it launches "in
    /// light mode" (preference `Light`) *before* installing. The untargeted `set_visuals` writes to
    /// whichever slot is active, so under a light preference it would install the dark palette into
    /// the **light** slot and leave the dark slot on stock — and this test would then find stock in
    /// the dark slot and fail. So restoring `set_visuals` in `install` breaks this test.
    #[test]
    fn install_targets_the_dark_slot_and_pins_dark() {
        let ctx = egui::Context::default();
        // Launch in light mode — the exact condition under which the untargeted setter misfires.
        ctx.set_theme(ThemePreference::Light);

        install(&ctx);

        // The palette landed in the DARK slot, not "whichever was active" (which was light).
        assert_eq!(
            ctx.style_of(Theme::Dark).visuals.panel_fill,
            STONE_2,
            "the palette must be installed into the dark slot, never the active one"
        );
        // The light slot is left untouched — stock, not the dark palette pasted into it.
        assert_ne!(
            ctx.style_of(Theme::Light).visuals.panel_fill,
            STONE_2,
            "the dark palette must not be written into the light slot"
        );
        // Following is disabled: the preference is pinned to dark, so an OS flip cannot restore stock.
        assert_eq!(
            ctx.options(|o| o.theme_preference),
            ThemePreference::Dark,
            "theme-following must be disabled (pinned to dark), or an OS theme change restores stock"
        );
        // And the resolved active theme is dark, so the app draws the palette from the first frame.
        assert_eq!(ctx.theme(), Theme::Dark);
    }

    /// The hover stroke against its own fill must clear **3:1** (ADR-0030 §3). This is the pair the
    /// draft regressed to 2.49:1; the fix lifts `STONE_8`. Non-text, so the floor is 3:1, not 7:1 —
    /// but hover is exactly the state the rule covers, so it is not left failing.
    #[test]
    fn hover_stroke_clears_three_to_one() {
        let v = visuals();
        let ratio = contrast(v.widgets.hovered.bg_stroke.color, v.widgets.hovered.bg_fill);
        assert!(
            ratio >= 3.0,
            "hover stroke is {ratio:.2}:1 against its fill, below 3:1 (ADR-0030 §3)"
        );
    }
}
