//! PROTOTYPE — throwaway. Answers #28 only. See PROTOTYPE.md. Desktop native entry point.
//! Android uses `android_main` in lib.rs.

/// Where the window opens: centred on the **landscape** monitor.
///
/// This desk has a portrait screen too, and a prototype that opens on it is unjudgeable — the
/// editor is a two-pane layout. `PROTO_POS=x,y` overrides if the desk is rearranged.
///
/// Native Wayland ignores client-requested positions — xdg-shell has no absolute placement — so
/// this is honoured under X11/XWayland and is advisory otherwise, where the compositor places the
/// window on its primary output instead. To force X11, unset `WAYLAND_DISPLAY`
/// (`env -u WAYLAND_DISPLAY cargo run`): winit 0.29 removed `WINIT_UNIX_BACKEND` in favour of the
/// standard `WAYLAND_DISPLAY` / `DISPLAY` variables.
fn window_position() -> [f32; 2] {
    if let Ok(spec) = std::env::var("PROTO_POS") {
        if let Some((x, y)) = spec.split_once(',') {
            if let (Ok(x), Ok(y)) = (x.trim().parse(), y.trim().parse()) {
                return [x, y];
            }
        }
    }
    // HDMI-A-1 is the landscape output at 0,436 1920x1080; centre 1180x900 inside it.
    [370.0, 526.0]
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 900.0])
            .with_position(window_position())
            .with_title("note authoring · #28"),
        ..Default::default()
    };
    eframe::run_native(
        "note authoring prototype · #28",
        options,
        Box::new(|cc| Ok(Box::new(note_authoring_28::ProtoApp::new(cc)))),
    )
}
