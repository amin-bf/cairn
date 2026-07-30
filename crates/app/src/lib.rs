//! The egui application: every screen, and both entry points.
//!
//! **This crate deliberately has no `src/main.rs`.** `cargo-apk` panics after signing
//! (`Bin is not compatible with Cdylib`) when one crate has both a cdylib and a bin — the APK comes
//! out correct but the exit code does not, and CI breaks. The desktop binary is `leitner-desktop`,
//! which is a shim with no logic (ADR-0003 §5, ADR-0009 §3).
//!
//! See `CONTEXT.md` beside this file, [ADR-0003](../../../docs/adr/0003-client-stack.md) and
//! [ADR-0006](../../../docs/adr/0006-the-review-session-experience.md).

pub mod bidi;

/// Re-exported so `leitner-desktop` needs no `eframe` dependency of its own — it cannot then
/// resolve a different feature set from the one this crate was built with, and it has no route to
/// grow real code unnoticed.
pub use eframe;

/// The application. No behaviour has landed yet — ADR-0009 laid out the workspace and stopped at
/// the seam, so this opens a window and renders one string through the bidi helper.
pub struct LeitnerApp;

impl LeitnerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Fonts are installed on the **first frame**, never here. Registering a face during
        // creation was found in #8 to break rendering on some backends; deferring it one frame
        // fixes it. When the Arabic face lands, it goes in `update`, guarded by a `bool` — and it
        // must be registered into *every* family including `Monospace`, or text silently renders
        // as boxes (ADR-0003 §4).
        Self
    }
}

impl eframe::App for LeitnerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Through the helper, not `ui.label("…")` — even here, so the skeleton demonstrates the
        // rule rather than modelling its violation.
        ui.label(bidi::job(
            "Leitner",
            egui::TextStyle::Heading.resolve(ui.style()),
            ui.visuals().text_color(),
        ));
    }
}

/// Android entry point. `NativeActivity` hosts the app directly: the APK is this `.so` plus a
/// manifest, with no Java, no Kotlin and no Gradle project in the repository.
///
/// GameActivity was built and tested in #8 and reverted — it implements IME correctly, but winit's
/// Android backend never reads it, so non-Latin text input stays unavailable at any packaging cost.
/// Never design a feature that requires typing non-Latin text on Android (`AGENTS.md` rule 8).
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: android_activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid as _;

    let options = eframe::NativeOptions {
        android_app: Some(app.clone()),
        event_loop_builder: Some(Box::new(move |b| {
            b.with_android_app(app.clone());
        })),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "Leitner",
        options,
        Box::new(|cc| Ok(Box::new(LeitnerApp::new(cc)))),
    );
}
