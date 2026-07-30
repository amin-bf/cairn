//! The entire platform surface, and it is two functions wide.
//!
//! ADR-0003 makes the platform seam a compile-time `#[cfg]` and forbids ever replacing it with a
//! runtime check — the whole stack choice rests on wrong platform code failing the build. ADR-0007
//! §442 then shrank what has to cross the seam to two directory lookups; everything else in this
//! crate is portable.
//!
//! **The third arm is the point.** A binary `android` / `not(android)` partition is tidier and can
//! never fail to compile — which is exactly its defect: a new target would silently take the
//! desktop arm and fail on a device instead of in CI. The `compile_error!` is what makes the rule
//! real rather than stylistic.
//!
//! **If a third function ever appears here, the seam is eroding.** That is the signal to stop and
//! ask why, not to add it.

use std::fmt;
use std::path::PathBuf;

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
    "unsupported target: add an arm to leitner_store::platform. \
     Do not widen an existing arm to cover it — see ADR-0003 and ADR-0009 §4."
);

/// Where `collection.db` and `derived.db` live (ADR-0007 §6).
///
/// On Android this is `getFilesDir()`, which **is** in the Auto Backup set. That is deliberate: the
/// collection should be restored onto a replacement phone. What must not be restored is the writer
/// identity, which is why [`state_dir`] exists.
pub fn data_dir() -> Result<PathBuf, PlatformError> {
    imp::data_dir()
}

/// Where the writer marker lives — **outside the backup set** (ADR-0007 §6).
///
/// This is load-bearing, not tidiness. Auto Backup defaults to on and restores `getFilesDir()`
/// wholesale, which would carry `local.writer_id` onto a replacement phone and make two devices the
/// same writer — the duplicate-writer failure ADR-0004 §2 and §3 exist to prevent, arriving through
/// a platform default nobody opted into. A marker held here turns a restore into a clean fork.
///
/// Moving this into the data directory reintroduces the bug silently, and sync would not notice
/// until the two devices eventually met.
pub fn state_dir() -> Result<PathBuf, PlatformError> {
    imp::state_dir()
}

/// A platform directory could not be resolved. Not recoverable in-process: without a data directory
/// there is nowhere to put the collection.
#[derive(Debug)]
pub struct PlatformError(pub String);

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "platform directory unavailable: {}", self.0)
    }
}

impl std::error::Error for PlatformError {}
