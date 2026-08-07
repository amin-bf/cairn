//! The desktop arm: no soft keyboard, and no window chrome the compositor makes us lay out around.
//!
//! A desktop window's client area is already the area the application draws into — there is no
//! status bar over it and no gesture bar under it — so both bars are zero here and that is a fact
//! rather than a stub.
//!
//! **The keyboard is `Absent`, not zero-height, and that is the whole point of the return type.**
//! Zero is what a *down* keyboard reports, so an implementation returning it here would make every
//! gate on "the keyboard is down" permanently true on desktop, and the raise in the text-field
//! wrapper would fire on every click into a field (ADR-0026 §5).

use super::{Insets, SoftKeyboard};
use crate::inbound::Inbound;

pub fn insets() -> Insets {
    Insets {
        top: 0.0,
        bottom: 0.0,
        keyboard: SoftKeyboard::Absent,
    }
}

/// The desktop has no launch intent: a file opened from a file manager arrives by drag-and-drop,
/// which egui surfaces directly with no seam function (ADR-0016 §5). So this is always `None`, and
/// that is a fact rather than a stub — [`crate::inbound::take_dropped`] is the desktop inbound path,
/// read off the frame's raw input, and this seam exists only for the arm that genuinely has one.
pub fn launch_file() -> Option<Inbound> {
    None
}
