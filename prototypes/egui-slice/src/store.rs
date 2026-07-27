//! The storage seam again, at bytes — but note what immediate mode does to it.
//!
//! The webview stacks could `await` inside an event handler. egui redraws the whole UI every frame
//! and has nowhere to await, so the web backend cannot be called directly: it has to be
//! **fire-and-forget plus a shared slot the UI polls each frame**. That extra layer is the price
//! immediate mode charges for an async platform API, and it is the one real structural difference
//! this slice surfaced.

use crate::model::ReviewEvent;

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use super::ReviewEvent;
    use std::io::Write;
    use std::path::PathBuf;

    pub fn data_dir() -> Result<PathBuf, String> {
        #[cfg(target_os = "android")]
        {
            crate::android::files_dir()
        }
        #[cfg(not(target_os = "android"))]
        {
            let base = std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
                .ok_or("no HOME")?;
            Ok(base.join("leitner-egui-slice"))
        }
    }

    fn path() -> Result<PathBuf, String> {
        let d = data_dir()?;
        std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
        Ok(d.join("review-log.jsonl"))
    }

    /// Synchronous on native — the shape immediate mode actually wants.
    pub fn read_all() -> Vec<ReviewEvent> {
        match path().and_then(|p| std::fs::read_to_string(p).map_err(|e| e.to_string())) {
            Ok(t) => super::parse(&t),
            Err(_) => Vec::new(),
        }
    }

    pub fn append(ev: &ReviewEvent) {
        if let Ok(p) = path() {
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&p) {
                let _ = writeln!(f, "{}", serde_json::to_string(ev).unwrap());
                let _ = f.sync_all();
            }
        }
    }

    pub fn backend() -> String {
        data_dir()
            .map(|d| format!("file — {}/review-log.jsonl", d.display()))
            .unwrap_or_else(|e| format!("file — UNRESOLVED: {e}"))
    }

    pub fn now_ms() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }

    /// Native needs no polling — this is a no-op so the UI code stays identical.
    pub fn poll() -> Option<Vec<ReviewEvent>> {
        None
    }

    pub fn kick_off_load() {}
}

#[cfg(target_arch = "wasm32")]
mod imp {
    use super::ReviewEvent;
    use std::cell::RefCell;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::{spawn_local, JsFuture};

    const FILE: &str = "review-log.jsonl";

    thread_local! {
        /// The shared slot. The UI drains this on the next frame.
        static INBOX: RefCell<Option<Vec<ReviewEvent>>> = const { RefCell::new(None) };
    }

    async fn handle() -> Result<web_sys::FileSystemFileHandle, String> {
        let storage = web_sys::window().ok_or("no window")?.navigator().storage();
        let dir = JsFuture::from(storage.get_directory())
            .await
            .map_err(|e| format!("getDirectory: {e:?}"))?
            .unchecked_into::<web_sys::FileSystemDirectoryHandle>();
        let opts = web_sys::FileSystemGetFileOptions::new();
        opts.set_create(true);
        JsFuture::from(dir.get_file_handle_with_options(FILE, &opts))
            .await
            .map(|h| h.unchecked_into())
            .map_err(|e| format!("getFileHandle: {e:?}"))
    }

    async fn read_inner() -> Result<Vec<ReviewEvent>, String> {
        let h = handle().await?;
        let file = JsFuture::from(h.get_file())
            .await
            .map_err(|e| format!("getFile: {e:?}"))?
            .unchecked_into::<web_sys::File>();
        let text = JsFuture::from(file.text())
            .await
            .map_err(|e| format!("text: {e:?}"))?
            .as_string()
            .unwrap_or_default();
        Ok(super::parse(&text))
    }

    async fn append_inner(ev: ReviewEvent) -> Result<(), String> {
        let h = handle().await?;
        let file = JsFuture::from(h.get_file())
            .await
            .map_err(|e| format!("getFile: {e:?}"))?
            .unchecked_into::<web_sys::File>();
        let size = file.size();
        let opts = web_sys::FileSystemCreateWritableOptions::new();
        opts.set_keep_existing_data(true);
        let stream = JsFuture::from(h.create_writable_with_options(&opts))
            .await
            .map_err(|e| format!("createWritable: {e:?}"))?
            .unchecked_into::<web_sys::FileSystemWritableFileStream>();
        JsFuture::from(stream.seek_with_f64(size).map_err(|e| format!("seek: {e:?}"))?)
            .await
            .map_err(|e| format!("seek: {e:?}"))?;
        let line = format!("{}\n", serde_json::to_string(&ev).unwrap());
        JsFuture::from(stream.write_with_str(&line).map_err(|e| format!("write: {e:?}"))?)
            .await
            .map_err(|e| format!("write: {e:?}"))?;
        JsFuture::from(stream.close()).await.map_err(|e| format!("close: {e:?}"))?;
        Ok(())
    }

    /// Fire-and-forget: start a load, drop the future, let `poll()` pick up the result.
    pub fn kick_off_load() {
        spawn_local(async {
            if let Ok(evs) = read_inner().await {
                INBOX.with(|c| *c.borrow_mut() = Some(evs));
            }
        });
    }

    pub fn append(ev: &ReviewEvent) {
        let ev = ev.clone();
        spawn_local(async move {
            let _ = append_inner(ev).await;
            if let Ok(evs) = read_inner().await {
                INBOX.with(|c| *c.borrow_mut() = Some(evs));
            }
        });
    }

    /// Called once per frame by the UI. This is the polling layer immediate mode forces.
    pub fn poll() -> Option<Vec<ReviewEvent>> {
        INBOX.with(|c| c.borrow_mut().take())
    }

    /// Web cannot answer synchronously, so the UI starts empty and fills in a frame later.
    pub fn read_all() -> Vec<ReviewEvent> {
        Vec::new()
    }

    pub fn backend() -> String {
        format!("OPFS (main thread, async → polled each frame) — {FILE}")
    }

    pub fn now_ms() -> i64 {
        js_sys::Date::now() as i64
    }
}

pub use imp::{append, backend, kick_off_load, now_ms, poll, read_all};

fn parse(text: &str) -> Vec<ReviewEvent> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}
