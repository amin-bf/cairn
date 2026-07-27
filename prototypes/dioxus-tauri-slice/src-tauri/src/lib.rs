//! The native Tauri core. Everything that touches the filesystem lives here and is reachable from
//! the frontend ONLY across the JSON `invoke` boundary.

use slice_shared::ReviewEvent;
use std::io::Write;
use std::path::PathBuf;
use tauri::Manager;

fn log_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    // On Android this resolves to Context.dataDir — app-private, no permission needed.
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("review-log.jsonl"))
}

#[tauri::command]
fn append_event(app: tauri::AppHandle, ev: ReviewEvent) -> Result<(), String> {
    let path = log_path(&app)?;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    writeln!(f, "{}", serde_json::to_string(&ev).unwrap()).map_err(|e| e.to_string())?;
    f.sync_all().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn read_all(app: tauri::AppHandle) -> Result<Vec<ReviewEvent>, String> {
    let path = log_path(&app)?;
    match std::fs::read_to_string(&path) {
        Ok(t) => Ok(slice_shared::parse_log(&t)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
fn backend(app: tauri::AppHandle) -> String {
    match log_path(&app) {
        Ok(p) => format!("invoke → native file — {}", p.display()),
        Err(e) => format!("invoke → UNRESOLVED: {e}"),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![append_event, read_all, backend])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
