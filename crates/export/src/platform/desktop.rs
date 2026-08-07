//! Desktop user files: a real directory the user can open, and a reveal-in-file-manager hand-off.
//!
//! The write goes to the documents folder (ADR-0022 §10's `~/Documents/French A1.cdeck`), resolved
//! by the XDG user-directories convention with a `~/Documents` fallback. No crate dependency for the
//! lookup — it is a few lines, matching `cairn-store::platform`'s reasoning for the data dirs.
//!
//! **`hand_off` reveals the file selected in the file manager** (ADR-0023 §4), because no
//! `org.freedesktop.portal.Share` exists on the desktop. It prefers `FileManager1.ShowItems`, which
//! selects the file, and falls back to opening the containing directory — a degradation ADR-0023 §4
//! deliberately does not surface to the user.

use super::{PlatformError, Written};
use crate::files::{dedupe_name, is_recognised};
use std::path::PathBuf;

fn err(context: &str, e: impl std::fmt::Display) -> PlatformError {
    PlatformError(format!("{context}: {e}"))
}

/// The user-visible documents directory: `$XDG_DOCUMENTS_DIR` when set and absolute, else
/// `$HOME/Documents`. Created on demand so the first export does not fail on a fresh account.
fn files_dir() -> Result<PathBuf, PlatformError> {
    let dir = match std::env::var_os("XDG_DOCUMENTS_DIR") {
        Some(v) if !v.is_empty() && PathBuf::from(&v).is_absolute() => PathBuf::from(v),
        _ => {
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .filter(|p| !p.as_os_str().is_empty())
                .ok_or_else(|| PlatformError("HOME is unset".into()))?;
            home.join("Documents")
        }
    };
    std::fs::create_dir_all(&dir).map_err(|e| err("create documents dir", e))?;
    Ok(dir)
}

pub fn put(requested_name: &str, bytes: &[u8]) -> Result<Written, PlatformError> {
    let dir = files_dir()?;
    let name = dedupe_name(requested_name, |n| dir.join(n).exists());
    std::fs::write(dir.join(&name), bytes).map_err(|e| err("write export", e))?;
    Ok(Written { name })
}

pub fn get(name: &str) -> Result<Vec<u8>, PlatformError> {
    let dir = files_dir()?;
    std::fs::read(dir.join(name)).map_err(|e| err("read export", e))
}

pub fn list() -> Result<Vec<String>, PlatformError> {
    let dir = files_dir()?;
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .map_err(|e| err("list exports", e))?
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| is_recognised(name))
        .collect();
    names.sort();
    Ok(names)
}

pub fn hand_off(name: &str) -> Result<(), PlatformError> {
    let path = files_dir()?.join(name);
    let uri = format!("file://{}", path.to_string_lossy());

    // Preferred: FileManager1.ShowItems selects the file (ADR-0023 §4). D-Bus-activatable, takes
    // URIs, so no file descriptor crosses the boundary.
    let shown = std::process::Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            "org.freedesktop.FileManager1",
            "--object-path",
            "/org/freedesktop/FileManager1",
            "--method",
            "org.freedesktop.FileManager1.ShowItems",
            &format!("['{uri}']"),
            "",
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if shown {
        return Ok(());
    }

    // Fallback: open the containing directory without selecting the file. ADR-0023 §4 says nothing
    // about this degradation to the user — it is invisible unless you already knew.
    let dir = path.parent().unwrap_or(&path);
    std::process::Command::new("xdg-open")
        .arg(dir)
        .status()
        .map_err(|e| err("reveal in file manager", e))?;
    Ok(())
}
