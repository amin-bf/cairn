//! Native entry point (desktop). Android uses `android_main` in lib.rs's platform module;
//! web uses the `start` export below.

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([560.0, 820.0])
            .with_title("egui slice · #8"),
        ..Default::default()
    };
    eframe::run_native(
        "egui slice",
        options,
        Box::new(|cc| Ok(Box::new(egui_slice::SliceApp::new(cc)))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    let web_options = eframe::WebOptions::default();
    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window().unwrap().document().unwrap();
        let canvas = document
            .get_element_by_id("the_canvas_id")
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .unwrap();
        eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(egui_slice::SliceApp::new(cc)))),
            )
            .await
            .expect("failed to start eframe");
    });
}
