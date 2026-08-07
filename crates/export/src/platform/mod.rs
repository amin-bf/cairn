//! The user-files seam — **put, get, list, hand_off** — and it is four functions wide.
//!
//! This is `cairn-export`'s own `platform` module, a peer of `cairn-store`'s two-function seam
//! under the same compile-time `#[cfg]` discipline (ADR-0009 §4, ADR-0016 §5, ADR-0023 §1). It moves
//! an artifact to a place the user can see, reads one back, lists the ones we recognise, and hands
//! one onward. The invariant is **opaque, minimal, enumerable** — *not* a function count: ADR-0016
//! §5's *"three operations, not four"* was an argument about **delete**, which is still absent, and
//! [`hand_off`] is the fourth operation ADR-0023 added on the deck file's own purpose.
//! `cairn-store::platform` still keeps exactly two, so its erosion signal is intact.
//!
//! **The third arm is the point.** A binary `android` / `not(android)` split can never fail to
//! compile, which is its defect: a new target would silently take the desktop arm. The
//! `compile_error!` third arm is what makes the seam rule real.

use std::fmt;

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
    "unsupported target: add an arm to cairn_export::platform. \
     Do not widen an existing arm to cover it — see ADR-0009 §4 and ADR-0023 §1."
);

/// The outcome of a [`put`]: **the name the platform actually wrote**, which a collision may have
/// deduped away from the one requested (ADR-0022 §10). The report reads this back and never echoes
/// the request — on Android the user chose neither the name nor the location, so it is the only way
/// they can find the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    pub name: String,
}

/// Write `bytes` to the user-visible files area under `requested_name`, returning the name actually
/// written. Desktop writes into the documents folder; Android inserts a `MediaStore` row **declaring
/// no media type**, which is what keeps the extension on a collision (ADR-0024 §4).
pub fn put(requested_name: &str, bytes: &[u8]) -> Result<Written, PlatformError> {
    imp::put(requested_name, bytes)
}

/// Read back a file this application wrote or recognised, by the name [`list`] reports.
pub fn get(name: &str) -> Result<Vec<u8>, PlatformError> {
    imp::get(name)
}

/// The names of the recognised files the application can see — `.cdeck` and `.ccoll`. On Android
/// this is the `MediaStore` rows we wrote, and **only** those: a file another application dropped in
/// `Downloads` is invisible to the query, not merely unreadable (ADR-0024 §3).
pub fn list() -> Result<Vec<String>, PlatformError> {
    imp::list()
}

/// Hand the written file to the surface the platform provides for passing a file onward, and stop
/// there (ADR-0023 §1). Android launches the system share sheet; the desktop reveals the file
/// **selected** in the file manager. It **never fires by itself** — the caller invokes it from a
/// user action, never on an export completing (ADR-0023 §5).
pub fn hand_off(name: &str) -> Result<(), PlatformError> {
    imp::hand_off(name)
}

/// A user-files operation failed. Recoverable at the call site — an export that could not be written
/// is reported, not fatal.
#[derive(Debug)]
pub struct PlatformError(pub String);

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "user-files operation failed: {}", self.0)
    }
}

impl std::error::Error for PlatformError {}
