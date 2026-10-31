//! Android has no directory lookup here: the Kotlin app passes its directories in.
//!
//! The JNI lookups that lived here served the egui Android build, which is retired (ADR-0045 §8).
//! The Kotlin app hands `filesDir` and `noBackupFilesDir` to the binding itself (ADR-0046), so the
//! split ADR-0007 §6 asks for — the collection in the backup set, the writer marker outside it — is
//! that caller's to keep. This arm still exists because the seam's third arm refuses an unlisted
//! target: it compiles, and it refuses.

use super::PlatformError;
use std::path::PathBuf;

const PASSED_IN: &str = "on Android the app passes its directories in (ADR-0046)";

pub fn data_dir() -> Result<PathBuf, PlatformError> {
    Err(PlatformError(PASSED_IN.to_owned()))
}

pub fn state_dir() -> Result<PathBuf, PlatformError> {
    Err(PlatformError(PASSED_IN.to_owned()))
}
