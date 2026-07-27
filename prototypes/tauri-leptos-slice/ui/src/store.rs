//! The storage seam in the Tauri+Leptos shape.
//!
//! The decisive difference from the Dioxus slice: this crate is ALWAYS wasm, so the choice of
//! backend cannot be a `#[cfg]`. Both backends are compiled into every build and the branch is a
//! **runtime** test on `window.isTauri`.
//!
//! `window.isTauri` — never `window.__TAURI__`, which defaults off, and never a bare `invoke` call,
//! which throws a raw JS `TypeError` that Rust cannot catch.

use slice_shared::ReviewEvent;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn tauri_invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

pub fn is_tauri() -> bool {
    web_sys::window()
        .and_then(|w| js_sys::Reflect::get(&w, &JsValue::from_str("isTauri")).ok())
        .map(|v| v.is_truthy())
        .unwrap_or(false)
}

/// The frontend is one wasm binary for every platform, so it cannot know its platform from a Rust
/// `cfg`. Under Tauri it has to sniff the user agent to tell desktop from Android.
pub fn device() -> String {
    if !is_tauri() {
        return "leptos-web".into();
    }
    let ua = web_sys::window()
        .map(|w| w.navigator().user_agent().unwrap_or_default())
        .unwrap_or_default();
    if ua.contains("Android") { "tauri-android".into() } else { "tauri-desktop".into() }
}

pub fn now_ms() -> i64 {
    js_sys::Date::now() as i64
}

pub async fn append(ev: &ReviewEvent) -> Result<(), String> {
    if is_tauri() {
        let args = serde_wasm_bindgen::to_value(&serde_json::json!({ "ev": ev }))
            .map_err(|e| e.to_string())?;
        tauri_invoke("append_event", args)
            .await
            .map(|_| ())
            .map_err(|e| format!("invoke append_event failed: {e:?}"))
    } else {
        opfs::append(ev).await
    }
}

pub async fn read_all() -> Result<Vec<ReviewEvent>, String> {
    if is_tauri() {
        let v = tauri_invoke("read_all", JsValue::UNDEFINED)
            .await
            .map_err(|e| format!("invoke read_all failed: {e:?}"))?;
        serde_wasm_bindgen::from_value(v).map_err(|e| e.to_string())
    } else {
        opfs::read_all().await
    }
}

pub async fn backend() -> String {
    if is_tauri() {
        tauri_invoke("backend", JsValue::UNDEFINED)
            .await
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| "invoke backend failed".into())
    } else {
        "OPFS (main thread, async) — review-log.jsonl".into()
    }
}

/// Byte-for-byte the same OPFS code as the Dioxus slice — duplicated rather than shared, because
/// the `shared` crate is also compiled natively for the Tauri core and must not touch `web_sys`.
mod opfs {
    use super::{JsCast, JsFuture, JsValue, ReviewEvent};

    const FILE: &str = "review-log.jsonl";

    async fn dir() -> Result<web_sys::FileSystemDirectoryHandle, String> {
        let storage = web_sys::window().ok_or("no window")?.navigator().storage();
        JsFuture::from(storage.get_directory())
            .await
            .map(|d| d.unchecked_into())
            .map_err(|e| format!("OPFS getDirectory failed: {e:?}"))
    }

    async fn handle() -> Result<web_sys::FileSystemFileHandle, String> {
        let opts = web_sys::FileSystemGetFileOptions::new();
        opts.set_create(true);
        JsFuture::from(dir().await?.get_file_handle_with_options(FILE, &opts))
            .await
            .map(|h| h.unchecked_into())
            .map_err(|e| format!("getFileHandle failed: {e:?}"))
    }

    pub async fn append(ev: &ReviewEvent) -> Result<(), String> {
        let h = handle().await?;
        let file = JsFuture::from(h.get_file())
            .await
            .map_err(|e| format!("getFile failed: {e:?}"))?
            .unchecked_into::<web_sys::File>();
        let size = file.size();

        let opts = web_sys::FileSystemCreateWritableOptions::new();
        opts.set_keep_existing_data(true);
        let stream = JsFuture::from(h.create_writable_with_options(&opts))
            .await
            .map_err(|e| format!("createWritable failed: {e:?}"))?
            .unchecked_into::<web_sys::FileSystemWritableFileStream>();

        JsFuture::from(stream.seek_with_f64(size).map_err(|e: JsValue| format!("seek: {e:?}"))?)
            .await
            .map_err(|e| format!("seek failed: {e:?}"))?;
        let line = format!("{}\n", serde_json::to_string(ev).unwrap());
        JsFuture::from(stream.write_with_str(&line).map_err(|e: JsValue| format!("write: {e:?}"))?)
            .await
            .map_err(|e| format!("write failed: {e:?}"))?;
        JsFuture::from(stream.close())
            .await
            .map_err(|e| format!("close failed: {e:?}"))?;
        Ok(())
    }

    pub async fn read_all() -> Result<Vec<ReviewEvent>, String> {
        let h = handle().await?;
        let file = JsFuture::from(h.get_file())
            .await
            .map_err(|e| format!("getFile failed: {e:?}"))?
            .unchecked_into::<web_sys::File>();
        let text = JsFuture::from(file.text())
            .await
            .map_err(|e| format!("text failed: {e:?}"))?
            .as_string()
            .unwrap_or_default();
        Ok(slice_shared::parse_log(&text))
    }
}
