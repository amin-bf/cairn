//! **The type scale** — the four sizes this application draws text at. Decided in
//! [#132](https://github.com/amin-bf/cairn/issues/132) and recorded in
//! [ADR-0032](../../../docs/adr/0032-the-type-scale-and-the-rhythm.md).
//!
//! Before this module the app named **no** type size at all. Every tier was stock egui — body and
//! button at 13, small at 9, heading at 18 — inherited rather than chosen, and the card face, the one
//! thing on screen whose whole job is to be read, was drawn at the same 13 as a button label because
//! it shared the button helper.
//!
//! # The numbers are values, not egui
//!
//! They are **logical pixels**, which is the same quantity Android calls `dp` and iOS calls `pt`.
//! Nothing here is expressed in a unit egui owns, so the scale carries to a native client unchanged;
//! `install` is one *binding* of the decision, and the day this app draws through a different
//! renderer the four constants come with it and only `install` is rewritten.
//!
//! # This is the only module in this crate that names a font size
//!
//! ADR-0030 §1's rule for colour and ADR-0031 §1's for layout, repeated a third time and for the same
//! reason: a literal `18.0` passed to a `FontId` on some screen **renders fine to the author and
//! drifts the scale one screen at a time, with nothing failing**. Ask for a *role* —
//! [`TextStyle::Body`], [`TextStyle::Heading`], [`display`] — and never a number.
//!
//! # Control text is an alias of body, and that is a decision
//!
//! [`BODY`] and control text are one value, so egui's `Button` slot is filled *from* `Body` rather
//! than from a fifth constant. Two identical constants are strictly worse than one: they drift apart
//! with nothing failing, and no test can tell a deliberate divergence from a typo. As an alias, "a
//! control's label is prose-sized" is a claim `control_text_is_body_sized` pins, so a later ticket
//! that wants them different has to break that test on purpose.
//!
//! # Installed into **every** theme slot
//!
//! `Style` is per-theme in egui — `style_of(Theme::Dark)` and `style_of(Theme::Light)` are different
//! objects — so `text_styles` sits in exactly the trap [ADR-0030 §2] records for `Visuals`: an
//! untargeted `set_style` writes to whichever slot happens to be active at construction, which is the
//! light one when the OS says light and the pin has not happened yet.
//!
//! Colour genuinely differs between the two, so `theme` was right to target the dark slot. **Type does
//! not** — 15px body is 15px body in either — so this writes to *all* slots through `all_styles_mut`,
//! which makes the trap inapplicable rather than merely avoided, and means the light mode still sitting
//! in #121's fog inherits the scale instead of silently getting stock.
//!
//! [ADR-0030 §2]: ../../../docs/adr/0030-the-first-finish-pass-decisions.md

use std::collections::BTreeMap;

use egui::{FontFamily, FontId, TextStyle};

/// The card face — the text actually being read. **The reason a fifth tier exists at all**: it is the
/// one surface whose job is to be read rather than to label, navigate or footnote.
///
/// Twice the heading, which is the largest step in the scale by a wide margin and is deliberate — see
/// the module note on why the scale accelerates.
pub const DISPLAY: f32 = 40.0;

/// Screen and section titles.
pub const HEADING: f32 = 20.0;

/// Sentences — and, by the alias above, the text inside every control.
pub const BODY: f32 = 15.0;

/// The footnote tier: the box badge, the interval preview, a field's caption.
pub const SMALL: f32 = 12.0;

/// The name egui knows the display tier by. Private on purpose: [`display`] is how a caller reaches
/// it, so the string is written once.
const DISPLAY_SLOT: &str = "display";

/// The display tier as a [`TextStyle`], for the one surface that draws it.
///
/// egui has no `Display` variant, but it does have `TextStyle::Name`, so this needs no mechanism of
/// its own — the tier is an ambient role a screen resolves exactly like `Body`, and the map's open
/// question about values with no ambient slot does **not** reach the type scale.
pub fn display() -> TextStyle {
    TextStyle::Name(DISPLAY_SLOT.into())
}

/// The scale, as egui's text-style table.
///
/// `Button` is filled from [`BODY`] rather than from a constant of its own — the alias of the module
/// header. `Monospace` is body-sized too: it is the `` `code` `` face *inside* a sentence
/// (ADR-0002 §8), so a size that disagreed with the prose around it would break the line it sits in.
pub fn scale() -> BTreeMap<TextStyle, FontId> {
    [
        (TextStyle::Small, FontId::new(SMALL, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(BODY, FontFamily::Proportional)),
        // The alias. Reads `BODY`, never a second literal.
        (TextStyle::Button, FontId::new(BODY, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(BODY, FontFamily::Monospace)),
        (
            TextStyle::Heading,
            FontId::new(HEADING, FontFamily::Proportional),
        ),
        (display(), FontId::new(DISPLAY, FontFamily::Proportional)),
    ]
    .into()
}

/// Install the scale into **every** theme slot. Called once from [`crate::CairnApp::new`].
///
/// Needs no first-frame deferral: a text style allocates no texture, so ADR-0012 §8's font hazard
/// does not reach it. It must run *after* `fonts::install` has registered the families in a given
/// frame only in the sense that a size is meaningless without a face — the two are independent
/// writes and the ordering between them does not matter.
pub fn install(ctx: &egui::Context) {
    let scale = scale();
    ctx.all_styles_mut(|style| style.text_styles = scale.clone());
}

#[cfg(test)]
// `assertions_on_constants` is the point here, not an oversight — as in `frame`, these tests assert
// relationships between compile-time constants precisely so that *changing a constant* is what breaks
// them.
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;
    use egui::{Theme, ThemePreference};

    /// The scale's numbers are a decision and ADR-0032 quotes them. Changing one should be an act
    /// that fails a test rather than a diff nobody notices.
    #[test]
    fn the_scale_is_the_numbers_the_adr_records() {
        assert_eq!(DISPLAY, 40.0);
        assert_eq!(HEADING, 20.0);
        assert_eq!(BODY, 15.0);
        assert_eq!(SMALL, 12.0);
    }

    /// **The alias.** Control text is prose-sized, and that is a decision rather than a coincidence
    /// of today's numbers — so it is pinned. A ticket that wants a control's label smaller than a
    /// sentence has to break this test deliberately, which is the whole point of writing it as an
    /// alias instead of a fifth constant.
    #[test]
    fn control_text_is_body_sized() {
        let scale = scale();
        assert_eq!(
            scale[&TextStyle::Button].size,
            scale[&TextStyle::Body].size,
            "control text is an alias of body (ADR-0032); a fifth constant is not how to diverge it"
        );
    }

    /// The `` `code` `` face inside a sentence matches the sentence. A monospace tier that disagreed
    /// with body would break the line it is set in.
    #[test]
    fn code_is_body_sized() {
        let scale = scale();
        assert_eq!(scale[&TextStyle::Monospace].size, scale[&TextStyle::Body].size);
    }

    /// Four distinct sizes, strictly ascending. Catches a tier edited to collide with its neighbour —
    /// which is invisible on a screen that happens not to draw both.
    #[test]
    fn the_tiers_are_four_distinct_ascending_sizes() {
        assert!(SMALL < BODY, "small must be smaller than body");
        assert!(BODY < HEADING, "body must be smaller than heading");
        assert!(HEADING < DISPLAY, "heading must be smaller than display");
    }

    /// **The scale accelerates**, and that is the shape that was chosen rather than a modular ratio
    /// (ADR-0032 §1): tight where tiers must coexist in a sentence, dramatic at the top where the card
    /// face has one job. Pinned as a relationship so a "tidy it into a uniform ratio" edit fails.
    #[test]
    fn the_scale_accelerates_rather_than_holding_one_ratio() {
        let low = BODY / SMALL; // 1.25
        let mid = HEADING / BODY; // 1.333…
        let high = DISPLAY / HEADING; // 2.0
        assert!(
            low < mid && mid < high,
            "the scale's steps must grow ({low:.3}, {mid:.3}, {high:.3}); a uniform ratio is the \
             shape variant A used, and it lost"
        );
    }

    /// Every tier the app draws is a value this module chose. Guards against a tier silently left on
    /// stock egui — 9, 13 or 18 — which renders fine and is the exact failure this module exists for.
    #[test]
    fn no_tier_is_left_on_stock() {
        let stock = egui::Style::default().text_styles;
        let ours = scale();
        for style in [
            TextStyle::Small,
            TextStyle::Body,
            TextStyle::Button,
            TextStyle::Heading,
        ] {
            assert_ne!(
                ours[&style].size, stock[&style].size,
                "{style:?} is still stock egui's size"
            );
        }
    }

    /// **Installed into every theme slot, not the active one.** This reproduces the bug ADR-0030 §2
    /// records for `Visuals`, one field over: the context launches under a *light* preference, which
    /// is the condition an untargeted `set_style` misfires on. Both slots must come back carrying the
    /// scale — so the light mode sitting in #121's fog inherits it rather than silently getting stock.
    #[test]
    fn the_scale_reaches_every_theme_slot() {
        let ctx = egui::Context::default();
        ctx.set_theme(ThemePreference::Light);

        install(&ctx);

        for theme in [Theme::Dark, Theme::Light] {
            let styles = ctx.style_of(theme);
            assert_eq!(
                styles.text_styles[&TextStyle::Body].size,
                BODY,
                "{theme:?} did not receive the scale"
            );
            assert_eq!(
                styles.text_styles[&display()].size,
                DISPLAY,
                "{theme:?} did not receive the display tier"
            );
        }
    }

    /// The display tier is reachable as an ambient role, which is the fact that kept the type scale
    /// off #121's "values with no ambient slot" question.
    #[test]
    fn the_display_tier_resolves_like_any_other_role() {
        let ctx = egui::Context::default();
        install(&ctx);
        let resolved = ctx.style_of(ctx.theme()).text_styles[&display()].clone();
        assert_eq!(resolved.size, DISPLAY);
    }

    /// **A named tier that was never installed panics — it does not fall back**, unlike every
    /// built-in variant, which cannot be missing. Recorded as a test because it is the one way this
    /// module's failure is *louder* than the defect class it belongs to, and that is worth keeping:
    /// resolving defensively at the call site would draw the 40px card face at stock's 13 on any path
    /// that skipped `install`, with nothing failing and nobody able to see it was wrong.
    ///
    /// It is also why `install` writes to every theme slot rather than the active one — a slot the
    /// scale never reached is not a smaller face, it is a crash the first time a card is drawn in it.
    #[test]
    fn an_uninstalled_display_tier_fails_loudly() {
        let bare = egui::Style::default();
        assert!(
            !bare.text_styles.contains_key(&display()),
            "stock egui has no display tier — if it grows one, this module's claim to name it changes"
        );
        // `AssertUnwindSafe` because `Style` is not `UnwindSafe` and nothing here is observed after
        // the unwind — the value is dropped either way.
        let panicked =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| display().resolve(&bare)))
                .is_err();
        assert!(
            panicked,
            "an unregistered named tier must fail loudly; a silent fallback would draw the card \
             face at stock's size on any path that skipped install"
        );
    }
}
