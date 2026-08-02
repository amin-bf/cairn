//! The Android arm: `WindowInsets` over JNI, and the entry point the activity handle comes from.
//!
//! This is the one call chain available to an application whose APK is a manifest plus a shared
//! object, with no Java and no Gradle project (ADR-0003 §2) — the same shape as
//! `leitner_store::platform`'s directory lookups.
//!
//! Two traps live here, both paid for on the handset, and both of the kind that present as a crash
//! somewhere else entirely. They are written out because neither is visible from the code that
//! trips them.

use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

use jni::objects::{JObject, JValue};

use super::{Insets, SoftKeyboard};

/// Below this many physical pixels the keyboard is treated as **down**.
///
/// The platform reports the *current frame* of the show/hide animation, so the inset passes through
/// small values on the way in and out. One pixel is not a keyboard, and a band that thin would
/// reserve nothing while making the raise gate flicker.
const KEYBOARD_UP_THRESHOLD_PX: f32 = 1.0;

/// The activity `jobject`, stashed by [`android_main`].
///
/// **Not `ndk_context::android_context().context()`.** That handle is the
/// `android.app.Application` — `android-activity` initialises it from `getApplication()` — and
/// `getWindow()` is an `Activity` method, so looking it up there throws `NoSuchMethodError` and
/// aborts the process. The existing JNI in `leitner-store` gives no warning about this, because
/// `getFilesDir()` is a `Context` method that `Application` has too: the wrong handle works
/// everywhere it has been used so far and fails on the first activity-shaped question.
static ACTIVITY: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

/// Read the live insets from the activity's decor view.
///
/// `getInsets(int)` and `WindowInsets.Type` are API 30+; `min_sdk_version` is 24, so every failure
/// degrades to "no insets" rather than refusing to run — on an older handset the application draws
/// as it did before this seam existed, which is the pre-ADR-0025 behaviour and not a crash.
pub fn insets() -> Insets {
    let read = read_insets();
    // **Clear before returning, always.** A failed JNI lookup leaves a Java exception *pending* on
    // the thread, and the next call made with one armed aborts the process rather than returning an
    // error — so the crash surfaces inside a later, unrelated call and looks like that call's fault.
    // `?` on a JNI result is only safe when something clears up behind it.
    clear_pending_exception();
    read.unwrap_or_default()
}

fn read_insets() -> Option<Insets> {
    let activity = ACTIVITY.load(Ordering::Relaxed);
    if activity.is_null() {
        return None;
    }

    let ctx = ndk_context::android_context();
    // SAFETY: `ndk_context` hands back the VM `android-activity` registered at startup, and the
    // activity pointer is a global ref taken by `android-activity` for the process's lifetime.
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.ok()?;
    let mut env = vm.attach_current_thread().ok()?;
    let activity = unsafe { JObject::from_raw(activity.cast()) };

    let window = env
        .call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])
        .ok()?
        .l()
        .ok()?;
    let decor = env
        .call_method(&window, "getDecorView", "()Landroid/view/View;", &[])
        .ok()?
        .l()
        .ok()?;
    // Null until the view is attached — the first frame or two after launch.
    let root = env
        .call_method(
            &decor,
            "getRootWindowInsets",
            "()Landroid/view/WindowInsets;",
            &[],
        )
        .ok()?
        .l()
        .ok()?;
    if root.is_null() {
        return None;
    }

    // `WindowInsets.Type.ime()` and `.systemBars()` are static ints naming the inset families.
    let types = env.find_class("android/view/WindowInsets$Type").ok()?;
    let ime_type = env
        .call_static_method(&types, "ime", "()I", &[])
        .ok()?
        .i()
        .ok()?;
    let bars_type = env
        .call_static_method(&types, "systemBars", "()I", &[])
        .ok()?
        .i()
        .ok()?;

    let ime = family(&mut env, &root, ime_type)?;
    let bars = family(&mut env, &root, bars_type)?;

    let ime_height = int_field(&mut env, &ime, "bottom")?;
    let keyboard = if ime_height > KEYBOARD_UP_THRESHOLD_PX {
        SoftKeyboard::Up { height: ime_height }
    } else {
        // Down — and this arm says *down*, never `Absent`. The platform has a soft keyboard whether
        // or not it is currently showing, and conflating the two is what ADR-0026 §5 forbids.
        SoftKeyboard::Down
    };

    Some(Insets {
        top: int_field(&mut env, &bars, "top")?,
        // The gesture bar, with the keyboard **excluded** — `systemBars()` is its own inset family.
        // `Insets::bottom_occluded` is what decides between them, and it is a max rather than a sum
        // because the keyboard is drawn over the bar.
        bottom: int_field(&mut env, &bars, "bottom")?,
        keyboard,
    })
}

/// One `WindowInsets.Type` family — the `android.graphics.Insets` object for `ty`.
fn family<'a>(env: &mut jni::JNIEnv<'a>, root: &JObject<'a>, ty: i32) -> Option<JObject<'a>> {
    env.call_method(
        root,
        "getInsets",
        "(I)Landroid/graphics/Insets;",
        &[JValue::Int(ty)],
    )
    .ok()?
    .l()
    .ok()
}

/// `android.graphics.Insets` exposes `left`/`top`/`right`/`bottom` as public `int` fields rather
/// than getters, so this is a field read and not a call.
fn int_field(env: &mut jni::JNIEnv, obj: &JObject, name: &str) -> Option<f32> {
    env.get_field(obj, name, "I")
        .ok()?
        .i()
        .ok()
        .map(|v| v as f32)
}

fn clear_pending_exception() {
    let ctx = ndk_context::android_context();
    let Ok(vm) = (unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }) else {
        return;
    };
    let Ok(env) = vm.attach_current_thread() else {
        return;
    };
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
}

/// Android entry point. `NativeActivity` hosts the application directly: the APK is this `.so` plus
/// a manifest, with no Java, no Kotlin and no Gradle project in the repository.
///
/// GameActivity was built and tested in #8 and reverted — it implements IME correctly, but winit's
/// Android backend never reads it, so non-Latin text input stays unavailable at any packaging cost.
/// Never design a feature that requires typing non-Latin text on Android (`AGENTS.md` rule 8).
///
/// **It lives in this arm because the activity handle originates here**, and nowhere else can hand
/// it over: `ndk_context` holds the `Application`. Stashing it from the seam's own arm is what keeps
/// [`super::insets`] the module's one function — there is no second thing for a caller to reach, and
/// no `#[cfg(target_os)]` anywhere above this file (client-stack rule 3).
#[unsafe(no_mangle)]
fn android_main(app: android_activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid as _;

    ACTIVITY.store(app.activity_as_ptr().cast(), Ordering::Relaxed);

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
        Box::new(|cc| Ok(Box::new(crate::LeitnerApp::new(cc)))),
    );
}
