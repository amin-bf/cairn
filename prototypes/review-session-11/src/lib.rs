//! PROTOTYPE — throwaway. Answers #11 only. See PROTOTYPE.md.

pub mod app;
pub mod bidi;
pub mod core;
pub mod model;
pub mod store;
pub mod variant_a;
pub mod variant_b;
pub mod variant_c;

#[cfg(target_os = "android")]
pub mod android;

pub use app::SliceApp;

/// Android entry point, same shape as `prototypes/egui-slice` (tag `prototypes/issue-8`):
/// `NativeActivity` hosts the app directly, no Java/Kotlin, just this `.so` and a manifest.
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
    let _ = eframe::run_native("review session · #11", options, Box::new(|cc| Ok(Box::new(SliceApp::new(cc)))));
}
