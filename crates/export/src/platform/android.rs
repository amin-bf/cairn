//! Android has no user-files seam here: the Kotlin app moves the bytes itself.
//!
//! The `MediaStore` and share-sheet calls that lived here served the egui Android build, which is
//! retired (ADR-0045 §8). The Kotlin app reads and writes files through the system picker and save
//! dialog and hands the binding bytes (ADR-0047 §6). This arm still exists because the seam's third
//! arm refuses an unlisted target: it compiles, and it refuses.

use super::{PlatformError, Written};

const BYTES_IN: &str = "on Android the app moves the file and passes the bytes (ADR-0047 §6)";

pub fn put(_requested_name: &str, _bytes: &[u8]) -> Result<Written, PlatformError> {
    Err(PlatformError(BYTES_IN.to_owned()))
}

pub fn get(_name: &str) -> Result<Vec<u8>, PlatformError> {
    Err(PlatformError(BYTES_IN.to_owned()))
}

pub fn list() -> Result<Vec<String>, PlatformError> {
    Err(PlatformError(BYTES_IN.to_owned()))
}

pub fn hand_off(_name: &str) -> Result<(), PlatformError> {
    Err(PlatformError(BYTES_IN.to_owned()))
}
