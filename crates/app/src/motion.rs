//! **Motion** — how long a transition takes, what shape it has, and the one place a thing is
//! allowed to move. Decided in [#154](https://github.com/amin-bf/cairn/issues/154) against a running
//! sitting, and recorded in [ADR-0037](../../../docs/adr/0037-motion-and-elevation.md).
//!
//! The fifth per-family module, after `theme`, `frame`, `typography` and `spacing`. All five now
//! answer *should there be one shared token module?* the same way: **no**. A family owns its own
//! constants and its own `install`, and the day this app draws through a different renderer the
//! constants come with it and only `install` is rewritten.
//!
//! # The duration is ambient; the curve cannot be
//!
//! [`DURATION`] is written into `Style::animation_time`, which every `animate_bool` in the crate
//! reads from the **global** style — so a screen cannot give itself its own tempo, which is the
//! wanted behaviour rather than a limitation. A screen that wants a different speed is a screen
//! inventing a value the system already names ([ADR-0030 §1](../../../docs/adr/0030-the-first-finish-pass-decisions.md),
//! a fifth time).
//!
//! The **curve** has no such slot, and it is worth knowing why rather than assuming an oversight.
//! It is not a value the renderer holds anywhere: it is *which function you call*. `animate_bool` is
//! linear, `animate_bool_responsive` is the only one in the library that picks a curve, and the
//! easing parameter is a bare `fn(f32) -> f32` with no closures and no cubic-Bézier constructor.
//! [`EASING`] therefore sits here as a constant beside the duration, so the choice is stated once
//! even though the renderer will not carry it ambiently. **A call site that names its own easing
//! function is the defect** — the same defect as a stray `Color32::from_rgb`.
//!
//! # They are not independent knobs to the eye
//!
//! At `cubic_out`, half the *time* is 87.5% of the *distance* — `1 − (1−0.5)³`. Most of a
//! `cubic_out` transition is over in its first quarter, so shortening the duration and softening the
//! curve are two ways of asking for the same thing, and 240ms was dragged against this curve rather
//! than chosen beside it.
//!
//! # Cost is not a constraint, and that is measured
//!
//! [#123](https://github.com/amin-bf/cairn/issues/123) computed that tessellation is not cached
//! between frames, so a transition is a full layout-and-tessellation pass over the whole viewport
//! per frame, and said explicitly that nothing had been run. It has: in release at 1280×800, through
//! `eframe`'s own `cpu_usage`, a reveal is **twelve frames at 0.51ms** — about 6.1ms of CPU spread
//! over the transition, roughly 3% of one core. An animating review frame is in fact *cheaper* than
//! a resting one, because a resting frame carries the once-a-second queue re-derivation and an
//! animating one re-uses the card it already has.

use egui::emath::easing;

/// How long a transition takes, in seconds. **240ms** (ADR-0037 §2).
///
/// Stock egui's `animation_time` is 0.2, and this is not a tweak of it: 0.2 was inherited, and this
/// is the number a person arrived at by dragging a knob with a live readout while watching the
/// reveal, which is [ADR-0035](../../../docs/adr/0035-the-vertical-anchor.md)'s rule — *judging a
/// distance wants a knob, not a menu* — applied to a distance measured in time.
pub const DURATION: f32 = 0.24;

/// The shape of a transition. **`cubic_out`** (ADR-0037 §2).
///
/// The one curve egui itself picks anywhere, which made it the starting position a choice had to
/// beat rather than a preference; six were offered and it won. Held as a `fn` pointer because that
/// is exactly what the renderer's easing parameter is.
pub const EASING: fn(f32) -> f32 = easing::cubic_out;

/// Install the duration into **every** theme slot. Called once from [`crate::CairnApp::new`].
///
/// Every slot for [`crate::typography::install`]'s reason: `Style` is per-theme, motion is not, and
/// a slot left unwritten is stock egui's 0.2 lying in wait for whoever switches theme. That failure
/// is invisible until it happens and then indistinguishable from a judgement, which is the class
/// [ADR-0036](../../../docs/adr/0036-the-light-palette.md) exists downstream of.
pub fn install(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| style.animation_time = DURATION);
}

/// How far through the reveal this frame is, from 0 (unrevealed) to 1 (fully turned over).
///
/// # Why this takes `card_changed` rather than keying on the card
///
/// Both traps [#123](https://github.com/amin-bf/cairn/issues/123) named live in the choice of
/// animation id, and they pull in **opposite directions** (ADR-0037 §4).
///
/// Keyed on the `Ui`'s own id the state is *too stable*: it survives the card changing, so grading
/// leaves `revealed` false with the value still at 1.0, and **the next card's answer is drawn,
/// fading out, for the whole duration** — a card nobody has turned over showing the one thing the
/// application exists to withhold, with nothing failing. It was found by counting frames, not by
/// looking: a gated run reported 24 animating frames per reveal where twelve were expected.
///
/// Keyed on the card's own `CardRef` that is fixed and a second leak opens: egui's animation state
/// is an `IdMap<BoolAnim>` **inserted into and never removed from** — there is no eviction anywhere
/// in `animation_manager.rs`, and `Context::clear_animations` drops the whole map or nothing — so a
/// per-card id retains one entry for every card ever reviewed, for the life of the process. Nothing
/// is at risk at twelve bytes an entry; it is growth with no ceiling in the loop the application
/// runs all day, arrived at by accident.
///
/// So: **one id, reset when the card changes.** The reset animates to `false` with a duration of
/// **zero**, which snaps — the manager's step divides by the duration, the result is infinite, it
/// fails its own `is_finite()` check and falls through to the target. That is the documented
/// behaviour of a zero-length animation, reached deliberately. O(1) memory, a new card that starts
/// unrevealed with no fade, and both traps discharged rather than traded against each other.
pub fn reveal_progress(ui: &egui::Ui, card_changed: bool, revealed: bool) -> f32 {
    let id = ui.id().with("reveal");
    if card_changed {
        ui.ctx().animate_bool_with_time(id, false, 0.0);
    }
    ui.ctx()
        .animate_bool_with_time_and_easing(id, revealed, DURATION, EASING)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Context, Id};

    /// Step a context forward one frame so the animation manager sees time pass.
    fn frame(ctx: &Context) {
        let input = egui::RawInput {
            predicted_dt: 1.0 / 60.0,
            ..Default::default()
        };
        let _ = ctx.run_ui(input, |_| {});
    }

    /// **Trap one, from the side that produces no motion at all.** An id seen for the first time
    /// snaps to its target rather than animating from zero — so an *unstable* id (one rebuilt every
    /// frame) is permanently on its first sight and there is no animation anywhere, with nothing
    /// failing and no error to read. Pinned against `Context` rather than against this module,
    /// because what it describes is what the renderer does.
    #[test]
    fn a_first_sight_id_snaps_to_its_target() {
        let ctx = Context::default();
        let value = ctx.animate_bool_with_time_and_easing(Id::new("fresh"), true, DURATION, EASING);
        assert_eq!(
            value, 1.0,
            "an id the manager has never seen must snap, which is what makes an unstable id silent"
        );
    }

    /// The companion to the test above: a **stable** id does animate, which is what makes that one
    /// a statement about the id rather than about the duration.
    #[test]
    fn a_stable_id_animates_rather_than_snapping() {
        let ctx = Context::default();
        let id = Id::new("stable");
        ctx.animate_bool_with_time_and_easing(id, false, DURATION, EASING);
        frame(&ctx);
        let value = ctx.animate_bool_with_time_and_easing(id, true, DURATION, EASING);
        assert!(
            value > 0.0 && value < 1.0,
            "a stable id must be mid-transition after one frame, got {value}"
        );
    }

    /// **Trap two.** Animation state is retained indefinitely — the map cannot be counted from
    /// outside, so this pins the *consequence*: an id untouched for 600 frames **resumes** rather
    /// than snapping, and has not advanced. A retained entry is frozen, not stale, and it is still
    /// there. That is why [`reveal_progress`] uses one id and resets it instead of minting one per
    /// card.
    #[test]
    fn animation_state_is_retained_indefinitely() {
        let ctx = Context::default();
        let id = Id::new("retained");
        ctx.animate_bool_with_time_and_easing(id, false, DURATION, EASING);
        frame(&ctx);
        let midway = ctx.animate_bool_with_time_and_easing(id, true, DURATION, EASING);

        for _ in 0..600 {
            frame(&ctx);
        }

        let resumed = ctx.animate_bool_with_time_and_easing(id, true, DURATION, EASING);
        assert!(
            resumed > midway,
            "the entry survived 600 untouched frames and resumed from where it was ({midway} -> {resumed})"
        );
    }

    /// The mechanism [`reveal_progress`]' reset depends on: a **zero** duration snaps even a stable
    /// id. If a later egui made a zero-length animation interpolate instead, the reset would stop
    /// working and a new card would fade its predecessor's answer out — so this is pinned.
    #[test]
    fn a_zero_duration_snaps_a_stable_id() {
        let ctx = Context::default();
        let id = Id::new("zero");
        ctx.animate_bool_with_time_and_easing(id, true, DURATION, EASING);
        frame(&ctx);
        let snapped = ctx.animate_bool_with_time(id, false, 0.0);
        assert_eq!(
            snapped, 0.0,
            "a zero-length animation must fall through to its target"
        );
    }

    /// `install` writes every slot, not the active one. A slot left stock is 0.2 waiting for a
    /// theme switch, which is exactly the failure ADR-0036 was written downstream of.
    #[test]
    fn install_writes_the_duration_into_both_theme_slots() {
        let ctx = Context::default();
        install(&ctx);
        for theme in [egui::Theme::Dark, egui::Theme::Light] {
            let installed = ctx.style_of(theme).animation_time;
            assert_eq!(
                installed, DURATION,
                "{theme:?} kept {installed} rather than the installed duration"
            );
        }
    }

    /// The curve and the duration are not independent to the eye, and the ADR quotes the figure.
    /// Changing `EASING` should be an act that fails a test rather than a diff nobody notices.
    #[test]
    fn half_the_time_is_most_of_the_distance() {
        let halfway = EASING(0.5);
        assert!(
            (halfway - 0.875).abs() < 1e-6,
            "cubic_out at t=0.5 is 0.875; got {halfway}"
        );
    }
}
