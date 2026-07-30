//! Desktop directories, per the XDG Base Directory specification (ADR-0007 §6).
//!
//! No crate dependency for this: the two lookups plus their documented fallbacks are a dozen lines,
//! and `leitner-store`'s dependency list is short enough to be worth keeping that way.

use super::PlatformError;
use std::path::PathBuf;

const APP_DIR: &str = "leitner";

fn home() -> Result<PathBuf, PlatformError> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| PlatformError("HOME is unset".into()))
}

/// `$XDG_DATA_HOME/leitner`, falling back to `~/.local/share/leitner`.
pub fn data_dir() -> Result<PathBuf, PlatformError> {
    xdg("XDG_DATA_HOME", ".local/share")
}

/// `$XDG_STATE_HOME/leitner`, falling back to `~/.local/state/leitner`.
///
/// XDG's slot for state that persists but is not the user's portable data — which is exactly the
/// writer marker's requirement. The desktop form of the Android backup hazard is a collection in a
/// cloud-synced folder or copied to a second machine, and this catches those identically.
pub fn state_dir() -> Result<PathBuf, PlatformError> {
    xdg("XDG_STATE_HOME", ".local/state")
}

fn xdg(var: &str, fallback: &str) -> Result<PathBuf, PlatformError> {
    let base = match std::env::var_os(var) {
        // The spec requires an absolute path; a relative value is defined to be ignored.
        Some(v) if !v.is_empty() && PathBuf::from(&v).is_absolute() => PathBuf::from(v),
        _ => home()?.join(fallback),
    };
    Ok(base.join(APP_DIR))
}
