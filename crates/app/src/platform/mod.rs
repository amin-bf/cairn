//! This crate's platform surface. It holds **two functions** — the window's insets, and the file
//! the platform launched us pointing at — for the same reason: both are questions about *this
//! crate's own window and activity* that nothing below it can answer.
//!
//! An inset is a fact about the window this crate is drawing into, so it is asked here rather than
//! routed through `cairn-store`, which would then be answering a question about layout. ADR-0016
//! §5 settled that the platform-seam rule is **per crate** rather than per workspace:
//! `cairn_store::platform` keeps exactly its two directory lookups, `cairn_export::platform` has
//! its four file operations, and this is the third such module
//! ([ADR-0025 §2](../../../../docs/adr/0025-the-authoring-screen-under-a-soft-keyboard.md)). The
//! count is not the invariant — *opaque, minimal, enumerable* is.
//!
//! **[`launch_file`] is the second function, and it is here for the same reason the first is: it
//! needs the Activity.** `getIntent()` is an `Activity` method, and this crate is the sole holder of
//! the activity handle — the user-files seam in `cairn-export` only ever holds the
//! `android.app.Application` ([ADR-0023 §7](../../../../docs/adr/0023-sending-a-written-file.md),
//! the reason [`android.rs`](android.rs)'s `ACTIVITY` is stashed separately), so an inbound read
//! cannot live there and ADR-0016 §5 deliberately keeps that seam at put/get/list/hand_off. The
//! read is a genuine platform question — *did the OS start us pointing at a file, and if so its
//! bytes and how it came* — opaque, minimal and answered honestly per arm. It is **not** a widening
//! of the file seam; it is this crate answering a question that is genuinely its own.
//!
//! **Why the application has to ask at all.** Nothing below it reports the soft keyboard. winit's
//! Android backend handles only motion and key events; `AGENTS.md` client-stack rule 8 records the
//! consequence for composed text, and this is the same gap's other half — it reports no **insets**
//! either. The window is enforced edge-to-edge, under which the resize soft-input mode is inert,
//! because an edge-to-edge window is expected to read the inset and lay itself out rather than be
//! resized under it. So raising the keyboard works and nothing comes back.
//!
//! **And the consequence is unreachability, not occlusion.** Un-asked, egui is handed a viewport
//! taller than the one the user can see; the content fits inside it; so the scroll area has no
//! scroll range and the covered band cannot be reached at all. Measured on the Pixel 8 Pro:
//! **923dp of usable height down to 565dp — 39% of the screen — with no notification, no reflow and
//! nothing to scroll.**
//!
//! **The third arm is the point.** A binary `android` / `not(android)` partition is tidier and can
//! never fail to compile, which is exactly its defect: a new target would silently take the desktop
//! arm and fail on a device instead of in CI. The `compile_error!` is what makes the rule real
//! rather than stylistic. Same discipline as `cairn_store::platform`, deliberately.
//!
//! **A *third* function appearing here is the erosion signal** — that is where to stop and ask why,
//! not to add it. The two that are here each answer a question only this crate's window or activity
//! can, and each was forced by a ticket that could not be resolved anywhere else (ADR-0025 §2 for
//! insets, #107 for the launch file); a third wants the same bar cleared. Note what is *not* a
//! function of the seam at all: the Android arm also holds `android_main`, because the entry point
//! is where the activity handle originates and it answers no portable question — there is nothing
//! there for a caller to reach — and desktop drag-and-drop is not here either, because egui surfaces
//! dropped files directly with no seam function (ADR-0016 §5, [`crate::inbound::take_dropped`]).

#[cfg(target_os = "android")]
#[path = "android.rs"]
mod imp;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
#[path = "desktop.rs"]
mod imp;

#[cfg(not(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    target_os = "windows"
)))]
compile_error!(
    "unsupported target: add an arm to cairn_app::platform. \
     Do not widen an existing arm to cover it — see ADR-0003 and ADR-0009 §4."
);

/// What the platform's window costs this frame: the system bars, and the soft keyboard.
///
/// Read **every frame**, never cached at focus time — the keyboard animates in over roughly 250ms
/// and the platform reports the current frame of that animation, so a value read once at focus is
/// whatever the inset happened to be one frame in, usually near zero.
pub fn insets() -> Insets {
    imp::insets()
}

/// The file the platform launched this process pointing at, read from the **activity's** launch
/// intent — `getData()` for `ACTION_VIEW`, the `EXTRA_STREAM` extra for `ACTION_SEND` (ADR-0024 §2)
/// — or `None` when it was an ordinary launch.
///
/// **Read once, at startup, and this is where cold start is satisfied** (ADR-0016 §5): the intent
/// is on the activity from the first frame the process runs for, whether or not the application was
/// already alive, so consulting it as the app comes up is what makes a file-manager open work on a
/// cold process rather than only on a warm one. The caller consults it once and holds the resulting
/// [`crate::inbound::Inbound`] — the *file*, never a derived plan (ADR-0022 §5).
///
/// The bytes are opened through the content resolver under the read grant the intent carries; the
/// display name is **not** required and may be absent (ADR-0024 §1). `None` on the desktop, which
/// has no launch intent — a desktop file arrives by drag-and-drop, read separately and directly off
/// egui's input with no seam function ([`crate::inbound::take_dropped`], ADR-0016 §5).
pub fn launch_file() -> Option<crate::inbound::Inbound> {
    imp::launch_file()
}

/// The platform's insets, in **physical pixels**. Divide by `pixels_per_point` for egui points.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Insets {
    /// Status bar or display cutout at the top.
    pub top: f32,
    /// Gesture bar or navigation bar at the bottom, the keyboard **excluded**.
    pub bottom: f32,
    /// The soft keyboard — present or not, up or down. See [`SoftKeyboard`].
    pub keyboard: SoftKeyboard,
}

impl Insets {
    /// What the bottom of the window is actually costing right now: the keyboard when it is up, the
    /// gesture bar when it is not.
    ///
    /// A **max, not a sum** — the keyboard is drawn *over* the gesture bar, so adding them reserves
    /// a band taller than the one that is covered and leaves a strip of dead space under the
    /// keyboard.
    pub fn bottom_occluded(self) -> f32 {
        self.keyboard.height().max(self.bottom)
    }
}

/// The soft keyboard's state, and the reason this is an enum rather than a height.
///
/// [ADR-0025 §2](../../../../docs/adr/0025-the-authoring-screen-under-a-soft-keyboard.md) first
/// specified this seam as returning insets *with a non-Android implementation that returns zero*.
/// **Zero is also what a down keyboard reports**, so off Android the two states are
/// indistinguishable and every gate on "the keyboard is down" is permanently true — which would make
/// the desktop send a redundant IME-enable on every pointer press with any widget focused, including
/// when the adapter has deliberately disabled IME because the focused widget is not a text field
/// ([ADR-0026 §5](../../../../docs/adr/0026-the-per-tap-keyboard-re-pop.md)).
///
/// Collapsing the two states was the error, so the type says which it is. This is **not** a widening
/// of the seam: still one function, and the type it returns is honest.
///
/// It is deliberately not a second compile-time capability constant either. The existing one —
/// ADR-0015 §9's non-Latin-input constant, client-stack rule 3's one sanctioned exception — exists
/// *to make a limitation visible, never to vary behaviour*, and this varies behaviour.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum SoftKeyboard {
    /// This platform has no soft keyboard at all. Not the same as one that is down, and the whole
    /// reason this type is not a bare `f32`.
    #[default]
    Absent,
    /// The platform has one and it is currently down.
    Down,
    /// It is up, covering this many **physical pixels** at the bottom of the window.
    Up { height: f32 },
}

impl SoftKeyboard {
    /// True only where a soft keyboard exists *and* is currently down — the state in which asking
    /// the platform to raise it is meaningful (ADR-0026 §4, §5).
    ///
    /// False on a platform that has none, which is the distinction this type exists to carry.
    pub fn is_down(self) -> bool {
        matches!(self, Self::Down)
    }

    /// True while the keyboard is up and covering the window.
    pub fn is_up(self) -> bool {
        matches!(self, Self::Up { .. })
    }

    /// What it is covering, in physical pixels — zero when it is down or does not exist.
    pub fn height(self) -> f32 {
        match self {
            Self::Up { height } => height,
            Self::Absent | Self::Down => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The distinction ADR-0026 §5 exists for: a platform with no soft keyboard must not answer
    /// "yes" to *is it down*, or every gate on that question is permanently true off Android.
    #[test]
    fn absent_is_not_down() {
        assert!(!SoftKeyboard::Absent.is_down());
        assert!(SoftKeyboard::Down.is_down());
        assert!(!SoftKeyboard::Up { height: 1145.0 }.is_down());
    }

    /// Desktop returns `Absent`, so the raise never fires there. The gate is read off the seam, so
    /// this is the seam's own guarantee rather than the caller's.
    #[cfg(not(target_os = "android"))]
    #[test]
    fn a_platform_without_a_soft_keyboard_says_so() {
        assert_eq!(insets().keyboard, SoftKeyboard::Absent);
        assert!(!insets().keyboard.is_down());
    }

    /// The keyboard is drawn *over* the gesture bar, so the occluded band is a max. Summing them
    /// reserves more than is covered and leaves dead space under the keyboard.
    #[test]
    fn the_keyboard_and_the_gesture_bar_overlap_rather_than_stack() {
        let bars = Insets {
            top: 151.0,
            bottom: 72.0,
            keyboard: SoftKeyboard::Down,
        };
        assert_eq!(bars.bottom_occluded(), 72.0);

        let typing = Insets {
            keyboard: SoftKeyboard::Up { height: 1145.0 },
            ..bars
        };
        assert_eq!(typing.bottom_occluded(), 1145.0);
    }
}
