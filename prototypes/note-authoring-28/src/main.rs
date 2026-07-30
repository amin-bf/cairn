//! PROTOTYPE — throwaway. Answers #28 only. See PROTOTYPE.md. Desktop native entry point.
//! Android uses `android_main` in lib.rs.

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 900.0])
            .with_title("note authoring · #28"),
        ..Default::default()
    };
    eframe::run_native(
        "note authoring prototype · #28",
        options,
        Box::new(|cc| Ok(Box::new(note_authoring_28::ProtoApp::new(cc)))),
    )
}
