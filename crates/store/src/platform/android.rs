//! Android directories, reached by hand-written JNI.
//!
//! Android exposes no data directory to this stack — ADR-0003 §5 records that Tauri's
//! `app_data_dir()` was the only candidate that avoided this, and it lost. So we call
//! `Context.getFilesDir()` and `Context.getNoBackupFilesDir()` ourselves. The `getFilesDir` half is
//! carried forward from the #8 slice, where it was proven on the handset.
//!
//! **Ordering hazard.** The JVM handle comes from `ndk_context`, which is populated by
//! `android-activity` inside `cairn-app`. So the store cannot be opened before the activity
//! exists. Under `android_main` that is automatic; it does mean `cairn-store` is not
//! independently runnable on Android, and store tests run on desktop.

use super::PlatformError;
use std::path::PathBuf;

/// `Context.getFilesDir()` — app-private internal storage.
///
/// Survives an app update. Does not survive uninstall or "clear data". **Is** included in Auto
/// Backup, which is why the writer marker lives in [`state_dir`] instead.
pub fn data_dir() -> Result<PathBuf, PlatformError> {
    context_dir("getFilesDir")
}

/// `Context.getNoBackupFilesDir()` — app-private storage excluded from Auto Backup by default.
///
/// This is the **only** exclusion mechanism available to us: XML backup rules require `@xml/…`
/// under `res/`, and ADR-0003 §2's no-Gradle-project property rests on the APK being a manifest
/// plus a `.so`, with no Android resources at all. The no-backup directory needs no XML.
pub fn state_dir() -> Result<PathBuf, PlatformError> {
    context_dir("getNoBackupFilesDir")
}

/// Both lookups are the same three JNI calls with a different method name: ask the activity for a
/// `java.io.File`, then ask that for its absolute path.
fn context_dir(method: &str) -> Result<PathBuf, PlatformError> {
    let err = |e: jni::errors::Error| PlatformError(format!("{method}: {e}"));

    let ctx = ndk_context::android_context();
    // SAFETY: `ndk_context` hands back the JavaVM and Activity pointers that `android-activity`
    // stored during `android_main`. Valid for the process lifetime once the activity exists.
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.map_err(err)?;
    let mut env = vm.attach_current_thread().map_err(err)?;
    let activity = unsafe { jni::objects::JObject::from_raw(ctx.context().cast()) };

    let file = env
        .call_method(&activity, method, "()Ljava/io/File;", &[])
        .map_err(err)?
        .l()
        .map_err(err)?;
    let path = env
        .call_method(&file, "getAbsolutePath", "()Ljava/lang/String;", &[])
        .map_err(err)?
        .l()
        .map_err(err)?;
    let path: String = env
        .get_string(&jni::objects::JString::from(path))
        .map_err(err)?
        .into();

    Ok(PathBuf::from(path))
}
