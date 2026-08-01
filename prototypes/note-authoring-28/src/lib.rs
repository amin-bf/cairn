//! PROTOTYPE — throwaway. Answers #28 only. See PROTOTYPE.md.

pub mod app;
pub mod bidi;
pub mod core;
pub mod insets;
pub mod markdown;
pub mod model;
pub mod variant_a;
pub mod variant_b;
pub mod variant_c;
pub mod variant_d;

#[cfg(target_os = "android")]
pub mod android;

pub use app::ProtoApp;

/// Android entry point, same shape as `prototypes/review-session-11` (tag `prototypes/issue-11`):
/// `NativeActivity` hosts the app directly, no Java/Kotlin, just this `.so` and a manifest.
///
/// Note for anyone running this on the handset: **text input here is ASCII-only**, because winit's
/// Android backend has no IME path (AGENTS.md, client-stack rule 8). The Persian scenario is
/// desktop-only by construction — on the phone, judge layout, not typing.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: android_activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid as _;

    // The activity handle, for `insets::read`. `ndk_context`'s context is the *Application* and has
    // no `getWindow()`; this is the only handle in the process that does.
    insets::ACTIVITY.store(app.activity_as_ptr(), std::sync::atomic::Ordering::Relaxed);

    let options = eframe::NativeOptions {
        android_app: Some(app.clone()),
        event_loop_builder: Some(Box::new(move |b| {
            b.with_android_app(app.clone());
        })),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "note authoring · #28",
        options,
        Box::new(|cc| Ok(Box::new(ProtoApp::new(cc)))),
    );
}
