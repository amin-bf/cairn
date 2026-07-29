//! PROTOTYPE — throwaway. Answers #11 only. See PROTOTYPE.md. Desktop native entry point.
//! Android uses `android_main` in lib.rs.

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([560.0, 860.0]).with_title("review session · #11"),
        ..Default::default()
    };
    eframe::run_native("review session prototype · #11", options, Box::new(|cc| Ok(Box::new(review_session_11::SliceApp::new(cc)))))
}
