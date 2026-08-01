//! PROTOTYPE — throwaway. Answers #67: the authoring screen under a soft keyboard.
//!
//! **Why this file exists at all.** Nothing between the app and Android reports the soft keyboard.
//! `AGENTS.md` client-stack rule 8 already records that winit's Android backend handles only motion
//! and key events and has no IME path — that rule is about *composed text*, and this is its other,
//! unrecorded half: winit also never reports the **insets** the keyboard occupies. The window is
//! `EDGE_TO_EDGE_ENFORCED` (API 35+), under which the old `adjustResize` no longer resizes the
//! window either, so the frame stays the full display and `eframe` hands egui a screen rect that
//! includes the region the keyboard is sitting on top of.
//!
//! Measured on the Pixel 8 Pro before writing a line of this: `mInputShown=true` while the app
//! window frame stayed `[0,0][1344,2992]`, `mImeHeight=1145` — **38% of the display**, invisible to
//! the app. The consequence is worse than occlusion: egui sizes its `ScrollArea` to a viewport that
//! is 1145px taller than the one the user can see, the content fits inside it, so there is **no
//! scroll range** and the covered band is unreachable rather than merely scrolled off.
//!
//! So the app must ask the platform itself. This reads `WindowInsets` over JNI, the same shape as
//! `android.rs`'s `files_dir` — the one call chain available to a `NativeActivity` app with no Java
//! in the APK.

/// Platform insets, in **physical pixels**. Divide by `pixels_per_point` for egui points.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Insets {
    /// Status bar / display cutout at the top.
    pub top: f32,
    /// Gesture bar or navigation bar at the bottom, keyboard *excluded*.
    pub bottom: f32,
    /// The soft keyboard. Zero when it is down.
    pub ime: f32,
}

impl Insets {
    /// What the bottom of the window is actually costing right now: the keyboard when it is up,
    /// the gesture bar when it is not. They overlap rather than stack — the keyboard is drawn over
    /// the gesture bar — so this is a max, not a sum.
    pub fn bottom_occluded(self) -> f32 {
        self.ime.max(self.bottom)
    }

    pub fn keyboard_is_up(self) -> bool {
        self.ime > 1.0
    }
}

#[cfg(not(target_os = "android"))]
pub fn read() -> Insets {
    // Desktop has no soft keyboard, and this prototype's whole point is that the desktop pass
    // could not answer the question. Zero here keeps the same code path running on both.
    Insets::default()
}

/// Read the live insets from the activity's decor view.
///
/// `getInsets(int)` and `WindowInsets.Type` are API 30+; the handset is API 37. Every failure
/// degrades to `Insets::default()` — a prototype that silently reports "no keyboard" is a visibly
/// wrong layout, which is a better failure than a crash mid-judgement.
#[cfg(target_os = "android")]
pub fn read() -> Insets {
    let out = read_inner();
    // **Clear before returning, always.** A failed lookup leaves a Java exception *pending* on the
    // thread, and the next JNI call made with one armed aborts the process rather than returning an
    // error — which is how the `getWindow` mistake above presented: not as a `None`, but as
    // `SIGABRT` inside the following frame's unrelated call. `?` on a JNI result is only safe when
    // something clears up after it.
    clear_pending_exception();
    out.unwrap_or_default()
}

#[cfg(target_os = "android")]
fn clear_pending_exception() {
    let ctx = ndk_context::android_context();
    let Ok(vm) = (unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }) else {
        return;
    };
    let Ok(mut env) = vm.attach_current_thread() else {
        return;
    };
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
}

/// The activity `jobject`, stashed by `android_main`.
///
/// **Not `ndk_context::android_context().context()`.** That is the *Application*, which the first
/// build of this file learned the hard way: `getWindow()` is an `Activity` method, so the lookup
/// threw `NoSuchMethodError` on `android.app.Application` and the process aborted. `android.rs`
/// works off the same handle only because `getFilesDir()` is a `Context` method, which
/// `Application` also has — so the existing JNI in this prototype gave no warning that the handle
/// was the wrong one for anything activity-shaped.
#[cfg(target_os = "android")]
pub static ACTIVITY: std::sync::atomic::AtomicPtr<std::ffi::c_void> =
    std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());

#[cfg(target_os = "android")]
fn read_inner() -> Option<Insets> {
    use jni::objects::{JObject, JValue};

    let activity_ptr = ACTIVITY.load(std::sync::atomic::Ordering::Relaxed);
    if activity_ptr.is_null() {
        return None;
    }

    let ctx = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.ok()?;
    let mut env = vm.attach_current_thread().ok()?;
    let activity = unsafe { JObject::from_raw(activity_ptr.cast()) };

    let window = env.call_method(&activity, "getWindow", "()Landroid/view/Window;", &[]).ok()?.l().ok()?;
    let decor = env.call_method(&window, "getDecorView", "()Landroid/view/View;", &[]).ok()?.l().ok()?;
    // Null before the view is attached — the first frame or two after launch.
    let root = env
        .call_method(&decor, "getRootWindowInsets", "()Landroid/view/WindowInsets;", &[])
        .ok()?
        .l()
        .ok()?;
    if root.is_null() {
        return None;
    }

    // `WindowInsets.Type.ime()` / `.systemBars()` are static ints naming the inset families.
    let type_class = env.find_class("android/view/WindowInsets$Type").ok()?;
    let ime_type = env.call_static_method(&type_class, "ime", "()I", &[]).ok()?.i().ok()?;
    let bars_type = env.call_static_method(&type_class, "systemBars", "()I", &[]).ok()?.i().ok()?;

    let ime = env
        .call_method(&root, "getInsets", "(I)Landroid/graphics/Insets;", &[JValue::Int(ime_type)])
        .ok()?
        .l()
        .ok()?;
    let bars = env
        .call_method(&root, "getInsets", "(I)Landroid/graphics/Insets;", &[JValue::Int(bars_type)])
        .ok()?
        .l()
        .ok()?;

    Some(Insets {
        top: int_field(&mut env, &bars, "top")?,
        bottom: int_field(&mut env, &bars, "bottom")?,
        ime: int_field(&mut env, &ime, "bottom")?,
    })
}

/// `android.graphics.Insets` exposes `left`/`top`/`right`/`bottom` as public `int` fields rather
/// than getters, so this is a field read and not a call.
#[cfg(target_os = "android")]
fn int_field(env: &mut jni::JNIEnv, obj: &jni::objects::JObject, name: &str) -> Option<f32> {
    env.get_field(obj, name, "I").ok()?.i().ok().map(|v| v as f32)
}
