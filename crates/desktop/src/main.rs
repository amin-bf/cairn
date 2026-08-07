//! The desktop binary.
//!
//! This crate exists for one reason: `cargo-apk` panics after signing when a single crate has both
//! a cdylib and a bin, so the desktop entry point cannot live in `cairn-app` (ADR-0003 §5).
//!
//! **Keep it this short.** Logic added here is never compiled by the Android build and never
//! exercised on the handset — a silent desktop-only path, which is the same class of defect as a
//! runtime platform check and is exactly what ADR-0003's compile-time seam exists to prevent.
//! Anything that looks like it belongs here belongs in `cairn_app`.

use cairn_app::eframe;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([560.0, 860.0])
            .with_title("Cairn"),
        ..Default::default()
    };
    eframe::run_native(
        "Cairn",
        options,
        Box::new(|cc| Ok(Box::new(cairn_app::CairnApp::new(cc)))),
    )
}
