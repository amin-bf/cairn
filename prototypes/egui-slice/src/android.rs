//! Android's app-private data dir.
//!
//! Dioxus exposes no API for this, so it is hand-written JNI against the Activity. Kept in its own
//! file precisely so the comparison can point at it: this is the whole Android-only surface.

use std::path::PathBuf;

pub fn files_dir() -> Result<PathBuf, String> {
    let ctx = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.map_err(|e| e.to_string())?;
    let mut env = vm.attach_current_thread().map_err(|e| e.to_string())?;
    let activity = unsafe { jni::objects::JObject::from_raw(ctx.context().cast()) };

    let file = env
        .call_method(&activity, "getFilesDir", "()Ljava/io/File;", &[])
        .map_err(|e| e.to_string())?
        .l()
        .map_err(|e| e.to_string())?;
    let path = env
        .call_method(&file, "getAbsolutePath", "()Ljava/lang/String;", &[])
        .map_err(|e| e.to_string())?
        .l()
        .map_err(|e| e.to_string())?;
    let s: String = env
        .get_string(&jni::objects::JString::from(path))
        .map_err(|e| e.to_string())?
        .into();
    Ok(PathBuf::from(s))
}
