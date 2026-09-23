//! **Throwaway measurement for #186** — never merged; preserved as `prototypes/issue-186`.
//!
//! Question: when Android backgrounds the app (Home, power button), does egui run a frame carrying
//! `Event::WindowFocused(false)` *before* eframe's `suspended()` drops the window? If it does, the
//! editor can settle in that frame. Each frame appends one line to `<data_dir>/probe-186.log`,
//! read back with `adb shell run-as dev.cairn.app cat files/probe-186.log`.

use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use cairn_store::Collection;

use eframe::egui;

use crate::{Editing, editor};

static FRAME: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn log(line: &str) {
    let Ok(dir) = cairn_store::platform::data_dir() else {
        return;
    };
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("probe-186.log"))
    {
        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis());
        let _ = writeln!(f, "{ms} {line}");
    }
}

/// Called at the top of every frame with the collection and the open editor, if any.
pub(crate) fn frame(ctx: &egui::Context, coll: &mut Collection, editing: Option<&mut Editing>) {
    let n = FRAME.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let (focus_events, focused) = ctx.input(|i| {
        let ev: Vec<bool> = i
            .events
            .iter()
            .filter_map(|e| match e {
                egui::Event::WindowFocused(f) => Some(*f),
                _ => None,
            })
            .collect();
        (ev, i.focused)
    });
    let unsettled = editing.as_ref().map(|ed| {
        ed.fields
            .iter()
            .filter(|(f, v)| editor::is_unsettled(coll, ed.note, f, v))
            .map(|(f, _)| f.clone())
            .collect::<Vec<_>>()
    });
    let lost = focus_events.contains(&false);
    let mut settled = false;
    if lost && let Some(ed) = editing {
        ed.note = editor::settle_all(coll, ed.note, &ed.kind, &ed.fields, ed.deck);
        settled = true;
    }
    log(&format!(
        "frame={n} focused={focused} focus_events={focus_events:?} unsettled={unsettled:?} settled={settled}"
    ));
}

/// eframe's `App::save` — on `Suspended`, and every `auto_save_interval`. Logs the unsettled
/// fields it found, then settles them.
pub(crate) fn save(coll: &mut Collection, editing: Option<&mut Editing>) {
    let Some(ed) = editing else {
        log("save editing=None");
        return;
    };
    let unsettled: Vec<String> = ed
        .fields
        .iter()
        .filter(|(f, v)| editor::is_unsettled(coll, ed.note, f, v))
        .map(|(f, _)| f.clone())
        .collect();
    ed.note = editor::settle_all(coll, ed.note, &ed.kind, &ed.fields, ed.deck);
    log(&format!("save unsettled={unsettled:?} note_after={:?}", ed.note.is_some()));
}

/// eframe's `on_exit`, logged so a clean shutdown is distinguishable from a freeze.
pub(crate) fn exit() {
    log("on_exit");
}
