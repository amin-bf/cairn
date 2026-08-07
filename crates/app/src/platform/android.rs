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

use jni::objects::{JObject, JString, JValue};

use super::{Insets, SoftKeyboard};
use crate::inbound::{Arrival, Inbound};

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

/// The file this process was launched pointing at, read from the **activity's** launch intent, or
/// `None` for an ordinary launch (`super::launch_file`'s Android arm).
///
/// `getIntent()` is an `Activity` method, which is why this reads [`ACTIVITY`] rather than the
/// `Application` handle `ndk_context` hands back — looking an `Activity` method up on the
/// `Application` throws `NoSuchMethodError` and aborts the process, the same trap [`read_insets`]
/// records. The bytes are opened through the content resolver under the read grant the intent
/// carries; the display name is asked for but **not required** (ADR-0024 §1) — a share may carry
/// none, and a provider URI carries a row id where the name would be.
pub fn launch_file() -> Option<Inbound> {
    let read = read_launch_file();
    // **Clear before returning, always** — the same discipline as `insets`. A failed JNI lookup
    // leaves a Java exception pending, and the next call armed with one aborts the process inside an
    // unrelated frame. `?` above is only safe because this runs behind it (acceptance of #107).
    clear_pending_exception();
    read
}

fn read_launch_file() -> Option<Inbound> {
    let activity = ACTIVITY.load(Ordering::Relaxed);
    if activity.is_null() {
        return None;
    }

    let ctx = ndk_context::android_context();
    // SAFETY: `ndk_context` hands back the VM `android-activity` registered at startup, and both the
    // activity and the application context are global refs valid for the process lifetime.
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.ok()?;
    let mut env = vm.attach_current_thread().ok()?;
    let activity = unsafe { JObject::from_raw(activity.cast()) };

    let intent = env
        .call_method(&activity, "getIntent", "()Landroid/content/Intent;", &[])
        .ok()?
        .l()
        .ok()?;
    if intent.is_null() {
        return None;
    }

    let action = env
        .call_method(&intent, "getAction", "()Ljava/lang/String;", &[])
        .ok()?
        .l()
        .ok()?;
    if action.is_null() {
        return None;
    }
    let action: String = env.get_string(&JString::from(action)).ok()?.into();

    // `ACTION_VIEW` carries the URI in `getData()`; `ACTION_SEND` carries it in `EXTRA_STREAM`
    // (ADR-0024 §2). Any other action — the app was launched, not handed a file — is `None`.
    let (arrival, uri) = match action.as_str() {
        "android.intent.action.VIEW" => {
            let uri = env
                .call_method(&intent, "getData", "()Landroid/net/Uri;", &[])
                .ok()?
                .l()
                .ok()?;
            (Arrival::Opened, uri)
        }
        "android.intent.action.SEND" => {
            let key = env.new_string("android.intent.extra.STREAM").ok()?;
            // The single-argument `getParcelableExtra` is deprecated at API 33+ but present and
            // functional from 24; `min_sdk_version` is 24, so it is the portable call.
            let uri = env
                .call_method(
                    &intent,
                    "getParcelableExtra",
                    "(Ljava/lang/String;)Landroid/os/Parcelable;",
                    &[JValue::Object(&JObject::from(key))],
                )
                .ok()?
                .l()
                .ok()?;
            (Arrival::Shared, uri)
        }
        _ => return None,
    };
    if uri.is_null() {
        return None;
    }

    let context = ndk_context::android_context();
    // SAFETY: valid for the process lifetime; the resolver is borrowed for this one read.
    let context = unsafe { JObject::from_raw(context.context().cast()) };
    let resolver = env
        .call_method(
            &context,
            "getContentResolver",
            "()Landroid/content/ContentResolver;",
            &[],
        )
        .ok()?
        .l()
        .ok()?;

    let bytes = read_stream(&mut env, &resolver, &uri)?;
    // Best-effort only: identity is in the bytes, so a missing name is no obstacle (ADR-0024 §1).
    let name = inbound_display_name(&mut env, &resolver, &uri);

    Some(Inbound {
        arrival,
        name,
        bytes,
    })
}

/// Read every byte behind a `content://` URI through `openInputStream`, under the read grant the
/// launch intent carries. `None` if the stream will not open — a refusal the caller surfaces, never
/// a crash.
fn read_stream(env: &mut jni::JNIEnv, resolver: &JObject, uri: &JObject) -> Option<Vec<u8>> {
    let stream = env
        .call_method(
            resolver,
            "openInputStream",
            "(Landroid/net/Uri;)Ljava/io/InputStream;",
            &[JValue::Object(uri)],
        )
        .ok()?
        .l()
        .ok()?;
    if stream.is_null() {
        return None;
    }

    let mut out = Vec::new();
    let buffer = env.new_byte_array(8192).ok()?;
    loop {
        let read = env
            .call_method(&stream, "read", "([B)I", &[JValue::Object(&buffer)])
            .ok()?
            .i()
            .ok()?;
        if read < 0 {
            break;
        }
        let mut chunk = vec![0i8; read as usize];
        env.get_byte_array_region(&buffer, 0, &mut chunk).ok()?;
        out.extend(chunk.into_iter().map(|b| b as u8));
    }
    let _ = env.call_method(&stream, "close", "()V", &[]);
    Some(out)
}

/// The `OpenableColumns.DISPLAY_NAME` a provider exposes for the URI, or `None` when it exposes none
/// — which is legal and common for a share (ADR-0024 §1). Every failure degrades to `None`, so this
/// never turns a nameless-but-readable file into a refusal.
fn inbound_display_name(
    env: &mut jni::JNIEnv,
    resolver: &JObject,
    uri: &JObject,
) -> Option<String> {
    let null = JObject::null();
    let cursor = env
        .call_method(
            resolver,
            "query",
            "(Landroid/net/Uri;[Ljava/lang/String;Ljava/lang/String;[Ljava/lang/String;Ljava/lang/String;)Landroid/database/Cursor;",
            &[
                JValue::Object(uri),
                JValue::Object(&null),
                JValue::Object(&null),
                JValue::Object(&null),
                JValue::Object(&null),
            ],
        )
        .ok()?
        .l()
        .ok()?;
    if cursor.is_null() {
        return None;
    }
    let name = if env
        .call_method(&cursor, "moveToFirst", "()Z", &[])
        .ok()?
        .z()
        .ok()?
    {
        let column = env.new_string("_display_name").ok()?;
        let index = env
            .call_method(
                &cursor,
                "getColumnIndex",
                "(Ljava/lang/String;)I",
                &[JValue::Object(&JObject::from(column))],
            )
            .ok()?
            .i()
            .ok()?;
        if index < 0 {
            None
        } else {
            let value = env
                .call_method(
                    &cursor,
                    "getString",
                    "(I)Ljava/lang/String;",
                    &[JValue::Int(index)],
                )
                .ok()?
                .l()
                .ok()?;
            (!value.is_null())
                .then(|| env.get_string(&JString::from(value)).ok().map(Into::into))
                .flatten()
        }
    } else {
        None
    };
    let _ = env.call_method(&cursor, "close", "()V", &[]);
    name
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
