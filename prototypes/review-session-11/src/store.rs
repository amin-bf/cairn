//! PROTOTYPE persistence — throwaway. A flat JSON-lines file, synchronous. This is *not*
//! ADR-0004's event log — it exists only to make "kill the app mid-session, does it resume
//! correctly" concrete and demoable. Wipe the file to reset.

use crate::model::ReviewEvent;
use std::io::Write as _;
use std::path::PathBuf;

const LOG_FILE: &str = "PROTOTYPE-review-session-11-log.jsonl";

fn log_path() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        // A relative path resolves nowhere writable under NativeActivity — the app's private
        // files dir is the only place the process can both write and re-read across a kill.
        if let Ok(dir) = crate::android::files_dir() {
            return dir.join(LOG_FILE);
        }
    }
    PathBuf::from(LOG_FILE)
}

pub fn read_all() -> Vec<ReviewEvent> {
    let Ok(text) = std::fs::read_to_string(log_path()) else { return Vec::new() };
    text.lines().filter_map(|line| serde_json::from_str(line).ok()).collect()
}

pub fn append(ev: &ReviewEvent) {
    let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(log_path()) else { return };
    if let Ok(line) = serde_json::to_string(ev) {
        let _ = writeln!(f, "{line}");
    }
}

pub fn wipe() {
    let _ = std::fs::remove_file(log_path());
}
