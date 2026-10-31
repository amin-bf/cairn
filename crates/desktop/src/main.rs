//! The desktop binary.
//!
//! The crate was split out of `cairn-app` for the egui Android build's packager (ADR-0003 §5); that
//! build is retired (ADR-0045 §8) and the split stays, because the scripts and docs run
//! `-p cairn-desktop`.
//!
//! **Keep it this short.** Anything that looks like it belongs here belongs in `cairn_app`, where
//! the tests reach it.

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
