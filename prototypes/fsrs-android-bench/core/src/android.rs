//! JNI entry point, so the same measurement runs inside a real app process rather
//! than only under `adb shell` — an app is scheduled differently from a shell.

use jni::JNIEnv;
use jni::objects::{JClass, JString};
use jni::sys::jstring;

/// `MainActivity.runBench("5000,20000,73000")` → the report text.
///
/// # Safety
/// Called by the JVM with valid arguments; not callable from Rust.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_leitner_fsrsbench_MainActivity_runBench(
    mut env: JNIEnv,
    _class: JClass,
    spec: JString,
) -> jstring {
    let spec: String = match env.get_string(&spec) {
        Ok(s) => s.into(),
        Err(err) => {
            let msg = format!("FSRSBENCH could not read spec: {err:?}");
            return env
                .new_string(msg)
                .expect("allocating a Java string")
                .into_raw();
        }
    };

    let sizes: Vec<usize> = spec
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    let report = std::panic::catch_unwind(|| crate::run(&sizes, 0.05, 20, 100_000))
        .unwrap_or_else(|_| "FSRSBENCH panicked during the run\n".to_string());

    env.new_string(report)
        .expect("allocating a Java string")
        .into_raw()
}
