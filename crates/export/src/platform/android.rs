//! Android user files: `MediaStore` for put/get/list, the system share sheet for hand-off — reached
//! by hand-written JNI, the same shim `cairn-store::platform::android` established (ADR-0007 §6).
//!
//! **The measured names below say `.ldeck`, and are left as run.** That was the extension in force
//! when these figures were taken; [ADR-0028 §3](../../../docs/adr/0028-the-application-is-named-cairn.md)
//! renamed it to `.cdeck` and §4 does not rewrite a record of a handset run. Nothing in the findings
//! depends on the letters — what was measured is where the dedupe suffix lands and what type
//! `MediaStore` stores.
//!
//! **Verified on the handset** — a Pixel 8 Pro, through this code rather than through a probe
//! ([#98](https://github.com/amin-bf/cairn/issues/98)). Both decisions the shape encodes held:
//!
//! - **The write declares no `mime_type`** ([ADR-0024 §4](../../../docs/adr/0024-identifying-a-written-file.md)).
//!   A declared type that disagrees with the name is the *only* reason a collision produces
//!   `French A1.ldeck (1)` instead of `French A1 (1).ldeck`; `MediaStore` stores
//!   `application/octet-stream` either way, so declaring one costs the extension and buys nothing.
//!   **Measured**: the same name written twice stored as `Specimen.ldeck` then
//!   `Specimen (1).ldeck`, both `application/octet-stream`. **And measured twice, at both ends of
//!   the supported range** — identically at API 37 and at API **29**, the level where
//!   `MediaStore.Downloads` and the permission-free insert first exist. So this is a property of the
//!   collection rather than of a recent platform, and the window ADR-0023 left unmeasured is
//!   **24–28** specifically, not "below the handset we own".
//! - **`hand_off`'s flags go on the chooser, not the inner intent** (ADR-0023 §7).
//!   `Intent.createChooser` returns a fresh intent inheriting neither `FLAG_ACTIVITY_NEW_TASK` —
//!   mandatory because the context is an `Application`, not an Activity — nor
//!   `FLAG_GRANT_READ_URI_PERMISSION`, whose absence fails only *after* the user has picked a target.
//!   **Measured**: the launch carries `flg=0x10000001`, which is exactly those two bits, and the
//!   chooser drew a populated sheet — under **two different choosers**, the framework's own
//!   `com.android.internal.app.ChooserActivity` on API 29 and `com.android.intentresolver` on API
//!   37. The `createChooser` route is not tied to one chooser generation.
//!
//! **And the bytes arrive.** ADR-0023 left *"whether a recipient can read the bytes"* open because
//! completing a share meant sending a file to a real contact; a second handset the owner controls
//! removes that, and the transfer was run. A device that has never held this application received
//! the file **byte for byte** — same SHA-256, same four members, `mimetype` still first and
//! uncompressed. So the grant is not merely accepted, it is honoured, and ADR-0008 §2's *"arrives
//! with someone who does not have our application"* is a measured claim rather than a design intent.
//!
//! **The display name survives the hand-off too**, dedupe suffix included, which upgrades ADR-0023
//! §7's fourth fact from an argument to an observation: the chooser previewing a bare row id is
//! cosmetic *because* a recipient resolves the real name, and now something has.

use super::{PlatformError, Written};
use crate::files::is_recognised;
use jni::objects::{JObject, JString, JValue};
use jni::{JNIEnv, JavaVM};

const DECK_MEDIA_TYPE: &str = crate::container::DECK_MEDIA_TYPE;

fn err(context: &str, e: impl std::fmt::Display) -> PlatformError {
    PlatformError(format!("{context}: {e}"))
}

/// The JVM this process runs under. The pointer is the one `android-activity` stored in
/// `android_main`; valid for the process lifetime once the activity exists.
fn java_vm() -> Result<JavaVM, PlatformError> {
    let ctx = ndk_context::android_context();
    // SAFETY: see above — `ndk_context` hands back the pointer stored during `android_main`.
    unsafe { JavaVM::from_raw(ctx.vm().cast()) }.map_err(|e| err("JavaVM", e))
}

/// The Application context handle. **Not an Activity** — hence `hand_off`'s `FLAG_ACTIVITY_NEW_TASK`.
///
/// SAFETY: the caller holds an attached `env` for the same thread, and the handle is valid for the
/// process lifetime once the activity exists.
unsafe fn app_context() -> JObject<'static> {
    let ctx = ndk_context::android_context();
    unsafe { JObject::from_raw(ctx.context().cast()) }
}

/// `context.getContentResolver()`, borrowed from `env` for the length of one operation.
fn content_resolver<'a>(
    env: &mut JNIEnv<'a>,
    context: &JObject,
) -> Result<JObject<'a>, PlatformError> {
    env.call_method(
        context,
        "getContentResolver",
        "()Landroid/content/ContentResolver;",
        &[],
    )
    .and_then(|v| v.l())
    .map_err(|e| err("getContentResolver", e))
}

/// The `MediaStore.Downloads.EXTERNAL_CONTENT_URI` collection every write lands in and every list
/// reads from — the permission-free, scoped-storage collection (ADR-0016 §5, API 29+).
fn downloads_uri<'a>(env: &mut JNIEnv<'a>) -> Result<JObject<'a>, PlatformError> {
    env.get_static_field(
        "android/provider/MediaStore$Downloads",
        "EXTERNAL_CONTENT_URI",
        "Landroid/net/Uri;",
    )
    .and_then(|v| v.l())
    .map_err(|e| err("MediaStore.Downloads URI", e))
}

pub fn put(requested_name: &str, bytes: &[u8]) -> Result<Written, PlatformError> {
    let vm = java_vm()?;
    let mut env = vm.attach_current_thread().map_err(|e| err("attach", e))?;
    let context = unsafe { app_context() };
    let resolver = content_resolver(&mut env, &context)?;

    // ContentValues with DISPLAY_NAME only — **no MIME_TYPE** (ADR-0024 §4).
    let values = env
        .new_object("android/content/ContentValues", "()V", &[])
        .map_err(|e| err("ContentValues", e))?;
    let key = env
        .new_string("_display_name")
        .map_err(|e| err("name key", e))?;
    let name = env
        .new_string(requested_name)
        .map_err(|e| err("name value", e))?;
    env.call_method(
        &values,
        "put",
        "(Ljava/lang/String;Ljava/lang/String;)V",
        &[JValue::Object(&key), JValue::Object(&name)],
    )
    .map_err(|e| err("ContentValues.put", e))?;

    let collection = downloads_uri(&mut env)?;
    let uri = env
        .call_method(
            &resolver,
            "insert",
            "(Landroid/net/Uri;Landroid/content/ContentValues;)Landroid/net/Uri;",
            &[JValue::Object(&collection), JValue::Object(&values)],
        )
        .and_then(|v| v.l())
        .map_err(|e| err("MediaStore insert", e))?;
    if uri.is_null() {
        return Err(PlatformError("MediaStore insert returned null".into()));
    }

    let stream = env
        .call_method(
            &resolver,
            "openOutputStream",
            "(Landroid/net/Uri;)Ljava/io/OutputStream;",
            &[JValue::Object(&uri)],
        )
        .and_then(|v| v.l())
        .map_err(|e| err("openOutputStream", e))?;
    let buffer = env
        .byte_array_from_slice(bytes)
        .map_err(|e| err("bytes", e))?;
    env.call_method(
        &stream,
        "write",
        "([B)V",
        &[JValue::Object(&JObject::from(buffer))],
    )
    .map_err(|e| err("write", e))?;
    env.call_method(&stream, "close", "()V", &[])
        .map_err(|e| err("close", e))?;

    // Read back the name the platform actually wrote (ADR-0022 §10) — a collision deduped it.
    let written =
        display_name_of(&mut env, &resolver, &uri)?.unwrap_or_else(|| requested_name.to_owned());
    Ok(Written { name: written })
}

/// The `_display_name` a `MediaStore` row carries, by querying the row's own URI.
fn display_name_of(
    env: &mut JNIEnv,
    resolver: &JObject,
    uri: &JObject,
) -> Result<Option<String>, PlatformError> {
    let cursor = query_all(env, resolver, uri)?;
    if cursor.is_null() {
        return Ok(None);
    }
    let moved = env
        .call_method(&cursor, "moveToFirst", "()Z", &[])
        .and_then(|v| v.z())
        .map_err(|e| err("moveToFirst", e))?;
    let name = if moved {
        column_string(env, &cursor, "_display_name")?
    } else {
        None
    };
    env.call_method(&cursor, "close", "()V", &[])
        .map_err(|e| err("cursor.close", e))?;
    Ok(name)
}

/// `resolver.query(uri, null, null, null, null)` — every column, every row, no filter.
fn query_all<'a>(
    env: &mut JNIEnv<'a>,
    resolver: &JObject,
    uri: &JObject,
) -> Result<JObject<'a>, PlatformError> {
    let null = JObject::null();
    env.call_method(
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
    .and_then(|v| v.l())
    .map_err(|e| err("query", e))
}

/// The string value of a named column in the cursor's current row, or `None` if absent.
fn column_string(
    env: &mut JNIEnv,
    cursor: &JObject,
    column: &str,
) -> Result<Option<String>, PlatformError> {
    let col_name = env.new_string(column).map_err(|e| err("column", e))?;
    let index = env
        .call_method(
            cursor,
            "getColumnIndex",
            "(Ljava/lang/String;)I",
            &[JValue::Object(&col_name)],
        )
        .and_then(|v| v.i())
        .map_err(|e| err("getColumnIndex", e))?;
    if index < 0 {
        return Ok(None);
    }
    let value = env
        .call_method(
            cursor,
            "getString",
            "(I)Ljava/lang/String;",
            &[JValue::Int(index)],
        )
        .and_then(|v| v.l())
        .map_err(|e| err("getString", e))?;
    if value.is_null() {
        return Ok(None);
    }
    let s: String = env
        .get_string(&JString::from(value))
        .map_err(|e| err("jstring", e))?
        .into();
    Ok(Some(s))
}

pub fn get(name: &str) -> Result<Vec<u8>, PlatformError> {
    let vm = java_vm()?;
    let mut env = vm.attach_current_thread().map_err(|e| err("attach", e))?;
    let context = unsafe { app_context() };
    let resolver = content_resolver(&mut env, &context)?;

    let uri = match uri_for(&mut env, &resolver, name)? {
        Some(uri) => uri,
        None => return Err(PlatformError(format!("no file named {name}"))),
    };
    let stream = env
        .call_method(
            &resolver,
            "openInputStream",
            "(Landroid/net/Uri;)Ljava/io/InputStream;",
            &[JValue::Object(&uri)],
        )
        .and_then(|v| v.l())
        .map_err(|e| err("openInputStream", e))?;

    let mut out = Vec::new();
    let buffer = env.new_byte_array(8192).map_err(|e| err("buffer", e))?;
    loop {
        let read = env
            .call_method(&stream, "read", "([B)I", &[JValue::Object(&buffer)])
            .and_then(|v| v.i())
            .map_err(|e| err("read", e))?;
        if read < 0 {
            break;
        }
        let mut chunk = vec![0i8; read as usize];
        env.get_byte_array_region(&buffer, 0, &mut chunk)
            .map_err(|e| err("region", e))?;
        out.extend(chunk.into_iter().map(|b| b as u8));
    }
    env.call_method(&stream, "close", "()V", &[])
        .map_err(|e| err("close", e))?;
    Ok(out)
}

pub fn list() -> Result<Vec<String>, PlatformError> {
    let vm = java_vm()?;
    let mut env = vm.attach_current_thread().map_err(|e| err("attach", e))?;
    let context = unsafe { app_context() };
    let resolver = content_resolver(&mut env, &context)?;
    let collection = downloads_uri(&mut env)?;

    let cursor = query_all(&mut env, &resolver, &collection)?;
    if cursor.is_null() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    loop {
        let more = env
            .call_method(&cursor, "moveToNext", "()Z", &[])
            .and_then(|v| v.z())
            .map_err(|e| err("moveToNext", e))?;
        if !more {
            break;
        }
        if let Some(name) = column_string(&mut env, &cursor, "_display_name")?
            && is_recognised(&name)
        {
            names.push(name);
        }
    }
    env.call_method(&cursor, "close", "()V", &[])
        .map_err(|e| err("cursor.close", e))?;
    names.sort();
    Ok(names)
}

/// The `content://` URI of the row whose display name is `name`, resolved by scanning the Downloads
/// collection and appending the matched row id.
///
/// **It scans by the *written* name, which is why the dedupe cannot make it ambiguous.** The caller
/// holds what [`put`] read back, never what it asked for (ADR-0022 §10) — and the scan sees only rows
/// this application owns, since `MediaProvider` filters the query by `owner_package_name`
/// (ADR-0024 §3). Verified on the handset resolving a deduped `Specimen (1).ldeck` (#98).
fn uri_for<'a>(
    env: &mut JNIEnv<'a>,
    resolver: &JObject,
    name: &str,
) -> Result<Option<JObject<'a>>, PlatformError> {
    let collection = downloads_uri(env)?;
    let cursor = query_all(env, resolver, &collection)?;
    if cursor.is_null() {
        return Ok(None);
    }
    let mut found: Option<i64> = None;
    loop {
        let more = env
            .call_method(&cursor, "moveToNext", "()Z", &[])
            .and_then(|v| v.z())
            .map_err(|e| err("moveToNext", e))?;
        if !more {
            break;
        }
        if column_string(env, &cursor, "_display_name")?.as_deref() == Some(name)
            && let Some(id) = column_string(env, &cursor, "_id")?.and_then(|s| s.parse().ok())
        {
            found = Some(id);
            break;
        }
    }
    env.call_method(&cursor, "close", "()V", &[])
        .map_err(|e| err("cursor.close", e))?;

    match found {
        None => Ok(None),
        Some(id) => {
            let base = downloads_uri(env)?;
            let uri = env
                .call_static_method(
                    "android/content/ContentUris",
                    "withAppendedId",
                    "(Landroid/net/Uri;J)Landroid/net/Uri;",
                    &[JValue::Object(&base), JValue::Long(id)],
                )
                .and_then(|v| v.l())
                .map_err(|e| err("withAppendedId", e))?;
            Ok(Some(uri))
        }
    }
}

pub fn hand_off(name: &str) -> Result<(), PlatformError> {
    let vm = java_vm()?;
    let mut env = vm.attach_current_thread().map_err(|e| err("attach", e))?;
    let context = unsafe { app_context() };
    let resolver = content_resolver(&mut env, &context)?;

    let uri = match uri_for(&mut env, &resolver, name)? {
        Some(uri) => uri,
        None => return Err(PlatformError(format!("no file named {name}"))),
    };

    // ACTION_SEND carrying the URI as EXTRA_STREAM, declaring the deck media type for the chooser
    // (accepted fine; the stored type is separate — ADR-0023 §3).
    let action = env
        .new_string("android.intent.action.SEND")
        .map_err(|e| err("action", e))?;
    let intent = env
        .new_object(
            "android/content/Intent",
            "(Ljava/lang/String;)V",
            &[JValue::Object(&JObject::from(action))],
        )
        .map_err(|e| err("Intent", e))?;
    let mime = env
        .new_string(DECK_MEDIA_TYPE)
        .map_err(|e| err("mime", e))?;
    env.call_method(
        &intent,
        "setType",
        "(Ljava/lang/String;)Landroid/content/Intent;",
        &[JValue::Object(&JObject::from(mime))],
    )
    .map_err(|e| err("setType", e))?;
    let extra_stream = env
        .new_string("android.intent.extra.STREAM")
        .map_err(|e| err("EXTRA_STREAM", e))?;
    env.call_method(
        &intent,
        "putExtra",
        "(Ljava/lang/String;Landroid/os/Parcelable;)Landroid/content/Intent;",
        &[
            JValue::Object(&JObject::from(extra_stream)),
            JValue::Object(&uri),
        ],
    )
    .map_err(|e| err("putExtra", e))?;

    // The chooser is a *fresh* intent; the flags go on it, not the inner intent (ADR-0023 §7).
    let chooser = env
        .call_static_method(
            "android/content/Intent",
            "createChooser",
            "(Landroid/content/Intent;Ljava/lang/CharSequence;)Landroid/content/Intent;",
            &[JValue::Object(&intent), JValue::Object(&JObject::null())],
        )
        .and_then(|v| v.l())
        .map_err(|e| err("createChooser", e))?;
    // FLAG_ACTIVITY_NEW_TASK (0x1000_0000) | FLAG_GRANT_READ_URI_PERMISSION (0x0000_0001).
    env.call_method(
        &chooser,
        "addFlags",
        "(I)Landroid/content/Intent;",
        &[JValue::Int(0x1000_0000 | 0x0000_0001)],
    )
    .map_err(|e| err("addFlags", e))?;
    env.call_method(
        &context,
        "startActivity",
        "(Landroid/content/Intent;)V",
        &[JValue::Object(&chooser)],
    )
    .map_err(|e| err("startActivity", e))?;
    Ok(())
}
