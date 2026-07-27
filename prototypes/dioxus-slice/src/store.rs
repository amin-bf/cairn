//! The storage seam, at bytes (research §3.4 "low seam").
//!
//! Two implementations, one call-site shape:
//!
//! ```ignore
//! let store = Store::open().await;
//! store.append(&ev).await;
//! let all = store.read_all().await;
//! ```
//!
//! Both are `async` even though the native one does blocking IO — that is the seam paying for
//! itself. The web backend is unavoidably async (every OPFS call is a `Promise`), so the trait
//! shape has to be the web one and native has to widen to meet it.

use crate::model::ReviewEvent;

#[cfg(target_family = "wasm")]
mod imp {
    use super::ReviewEvent;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    const FILE: &str = "review-log.jsonl";

    pub struct Store {
        dir: web_sys::FileSystemDirectoryHandle,
    }

    impl Store {
        pub async fn open() -> Result<Self, String> {
            let storage = web_sys::window().ok_or("no window")?.navigator().storage();
            let dir = JsFuture::from(storage.get_directory())
                .await
                .map_err(|e| format!("OPFS getDirectory failed: {e:?}"))?
                .unchecked_into::<web_sys::FileSystemDirectoryHandle>();
            Ok(Self { dir })
        }

        async fn handle(&self) -> Result<web_sys::FileSystemFileHandle, String> {
            let opts = web_sys::FileSystemGetFileOptions::new();
            opts.set_create(true);
            JsFuture::from(self.dir.get_file_handle_with_options(FILE, &opts))
                .await
                .map(|h| h.unchecked_into())
                .map_err(|e| format!("getFileHandle failed: {e:?}"))
        }

        pub async fn append(&self, ev: &ReviewEvent) -> Result<(), String> {
            let handle = self.handle().await?;
            // Read the current size so we can seek to the end: OPFS `createWritable` truncates by
            // default, and `keepExistingData` still starts the cursor at 0.
            let file = JsFuture::from(handle.get_file())
                .await
                .map_err(|e| format!("getFile failed: {e:?}"))?
                .unchecked_into::<web_sys::File>();
            let size = file.size();

            let opts = web_sys::FileSystemCreateWritableOptions::new();
            opts.set_keep_existing_data(true);
            let stream = JsFuture::from(handle.create_writable_with_options(&opts))
                .await
                .map_err(|e| format!("createWritable failed: {e:?}"))?
                .unchecked_into::<web_sys::FileSystemWritableFileStream>();

            JsFuture::from(stream.seek_with_f64(size).map_err(|e| format!("seek: {e:?}"))?)
                .await
                .map_err(|e| format!("seek failed: {e:?}"))?;
            let line = format!("{}\n", serde_json::to_string(ev).unwrap());
            JsFuture::from(
                stream.write_with_str(&line).map_err(|e| format!("write: {e:?}"))?,
            )
            .await
            .map_err(|e| format!("write failed: {e:?}"))?;
            JsFuture::from(stream.close())
                .await
                .map_err(|e| format!("close failed: {e:?}"))?;
            Ok(())
        }

        pub async fn read_all(&self) -> Result<Vec<ReviewEvent>, String> {
            let handle = self.handle().await?;
            let file = JsFuture::from(handle.get_file())
                .await
                .map_err(|e| format!("getFile failed: {e:?}"))?
                .unchecked_into::<web_sys::File>();
            let text = JsFuture::from(file.text())
                .await
                .map_err(|e| format!("text failed: {e:?}"))?
                .as_string()
                .unwrap_or_default();
            Ok(super::parse(&text))
        }

        pub fn backend() -> String {
            format!("OPFS (main thread, async) — {FILE}")
        }
    }

    pub fn now_ms() -> i64 {
        js_sys::Date::now() as i64
    }
}

#[cfg(not(target_family = "wasm"))]
mod imp {
    use super::ReviewEvent;
    use std::io::Write;
    use std::path::PathBuf;

    pub struct Store {
        path: PathBuf,
    }

    impl Store {
        pub async fn open() -> Result<Self, String> {
            let dir = data_dir()?;
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            Ok(Self { path: dir.join("review-log.jsonl") })
        }

        pub async fn append(&self, ev: &ReviewEvent) -> Result<(), String> {
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
                .map_err(|e| e.to_string())?;
            writeln!(f, "{}", serde_json::to_string(ev).unwrap()).map_err(|e| e.to_string())?;
            f.sync_all().map_err(|e| e.to_string())?;
            Ok(())
        }

        pub async fn read_all(&self) -> Result<Vec<ReviewEvent>, String> {
            match std::fs::read_to_string(&self.path) {
                Ok(t) => Ok(super::parse(&t)),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
                Err(e) => Err(e.to_string()),
            }
        }

        pub fn backend() -> String {
            data_dir()
                .map(|d| format!("file — {}/review-log.jsonl", d.display()))
                .unwrap_or_else(|e| format!("file — UNRESOLVED: {e}"))
        }
    }

    /// The one place desktop and Android actually diverge.
    #[cfg(target_os = "android")]
    fn data_dir() -> Result<PathBuf, String> {
        // Dioxus exposes no data-dir API, so this is hand-written JNI against the Activity —
        // the cost the research predicted. See README "What Android cost".
        crate::android::files_dir()
    }

    #[cfg(not(target_os = "android"))]
    fn data_dir() -> Result<PathBuf, String> {
        let base = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
            .ok_or("no HOME")?;
        Ok(base.join("leitner-dioxus-slice"))
    }

    pub fn now_ms() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }
}

pub use imp::{now_ms, Store};

fn parse(text: &str) -> Vec<ReviewEvent> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}
