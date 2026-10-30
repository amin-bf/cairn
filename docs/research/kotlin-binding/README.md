# Binding the Rust core to Kotlin — uniffi, cargo-ndk and Gradle today

Evidence for [#194 "Native Android: Binding the Rust Core to Kotlin — uniffi, cargo-ndk and Gradle
Today"](https://github.com/amin-bf/cairn/issues/194), on the map
[#193](https://github.com/amin-bf/cairn/issues/193).

[ADR-0045 §2](../../adr/0045-the-native-android-client.md) makes the Kotlin app a UI over
`cairn-core`, `cairn-store` and `cairn-export`, called through generated bindings, and §3 has the
Gradle project build the Rust core itself. This note answers what that costs today: what uniffi
generates for Kotlin, how Gradle drives the NDK build for `arm64-v8a` and `x86_64`, what the C
dependency needs, which thread a call runs on, how big the APK gets, and what in the crates' present
shape does not survive being a library loaded by a Kotlin process.

**Read 29.10.2026** from uniffi's changelog and manual at the 0.32.2 tag, cargo-ndk's README, the
Android developer documentation and the AGP API jar. **Measured 29.10.2026** with a throwaway spike
kept out of the repository (reproduced in §9): a wrapper crate over the three crates, bound with
uniffi 0.32.2, built with cargo-ndk 4.1.2 against NDK 29, packaged by AGP 9.4.1 on Gradle 9.7.1
with JDK 17, and run on an `x86_64` emulator image at API 36. The `arm64-v8a` library was built and
inspected but not run on a handset. [`AGENTS.md`](../../../AGENTS.md) rule 9 holds for platform
behaviour; nothing below turns on a difference between the emulator and a handset except where §6
says so.

## The headline

**The binding works today, with no plugin and no Rust runtime, but not from the crates as they
are.** Three things have to change, and they are #195's to decide:

1. **The binding lives in its own crate.** uniffi's derives have to sit on the types they export,
   or the defining crate has to use uniffi too. `cairn-core` admits a dependency only through
   [ADR-0027 §2](../../adr/0027-the-scheduler-dependency.md)'s five-part test, whose second part
   (*computation and nothing else … no platform*) an FFI runtime fails. A wrapper
   crate that mirrors the types it exposes avoids this, and it also absorbs the types uniffi cannot
   carry (§2).
2. **Kotlin passes the directories.** `cairn_store::platform` and `cairn_export::platform` reach the
   Android API through `ndk_context`, which only `android-activity` initialises. In a library loaded
   by a Kotlin activity, the first call panics (§6). `Collection::open` already takes both paths.
3. **The egui-free application logic leaves `cairn-app`.** The session, the editor, the note list
   and the rest live in a crate that also pulls in eframe, winit and android-activity. A binding
   cannot depend on it without shipping all three (§7).

What it costs is about 1.9 MB of native library per ABI, plus about 0.2 MB of JNA. Both are 16 KB
aligned, and the bundled SQLite builds under cargo-ndk with no extra flags.

## 1. uniffi today

**The current release is 0.32.2, released 22.09.2026** ([changelog][uniffi-changelog]). The releases
that matter for a Kotlin consumer:

| Release | What it changed for Kotlin |
| --- | --- |
| 0.30.0 | Bindings call through **JNA direct mapping** rather than interface mapping |
| 0.32.0 (30.06.2026) | `[ByRef] bytes` arguments need a *direct* `ByteBuffer`; an `async` primary constructor is a bindgen error; objects gain `uniffiIsDestroyed` |
| 0.32.1 (08.09.2026) | Fixes a Kotlin checksum mismatch on `aarch64` |

Unreleased on `main`: an **experimental JNI-based Kotlin generator** (`uniffi-bindgen-kotlin-jni`,
[#2911][uniffi-2911]) that would drop JNA. It is worth watching but not worth building on.

**The Kotlin runtime needs JNA ≥ 5.12 and kotlinx-coroutines ≥ 1.6** ([Kotlin
manual][uniffi-kotlin]). The current JNA release is 5.19.1 (12.06.2026), published as an `@aar`
that carries `libjnidispatch.so` for each ABI. With `android = true` in `uniffi.toml`, the generated
code imports `androidx.annotation`, so that dependency is required too. Objects are released through
a cleaner: `android.system.SystemCleaner` on API 34 and later, JNA's own cleaner below that.

**Generation runs in library mode, once, on the host:** `uniffi-bindgen generate <host .so>
--language kotlin --out-dir <dir>`. It reads the metadata out of a host build of the same crate, so
one generated source tree serves every Android ABI. Formatting goes through ktlint when it is
installed; `--no-format` skips it.

## 2. What the generated Kotlin looks like, and what cannot cross

Observed in the spike's generated `cairn_ffi.kt`:

| Rust | Kotlin |
| --- | --- |
| `#[derive(uniffi::Record)] struct` | `data class` with `var` fields (`val` when `generate_immutable_records = true`) |
| `#[derive(uniffi::Enum)]` with fields | `sealed class` whose variants are data classes |
| `#[derive(uniffi::Error)] enum CairnError` | `sealed class CairnException : kotlin.Exception()`; every fallible function is `@Throws(CairnException::class)` |
| a Rust panic | `InternalException` carrying the panic message |
| `#[derive(uniffi::Object)]` | `open class X : Disposable, AutoCloseable`, released with `close()` or `use { }` |
| `#[uniffi::export(callback_interface)] trait` | a Kotlin `interface`, called on the caller's thread |
| `async fn` | `suspend fun`, driven by `suspendCancellableCoroutine` |
| `u32`, `u64` | `UInt`, `ULong` |
| `Vec<u8>` | `ByteArray` |

**Async needs no Rust executor.** The Kotlin coroutine polls the Rust future, so nothing like tokio
goes into the library. uniffi has no built-in cancellation: *"There's no builtin way to cancel a
future"*, and the manual says to expose a library-specific `cancel()` instead ([futures
manual][uniffi-futures]).

**Panics only reach Kotlin as exceptions while the profile unwinds.** uniffi's scaffolding wraps
every call in `catch_unwind`. The workspace's `[profile.release]` has no `panic = "abort"`, and it
must not gain one for the binding, or a panic would abort the process instead of throwing.

**What the boundary refuses**, measured or read from the [builtin types table][uniffi-builtin]:

- **`usize` is not a uniffi type.** Mirroring `cairn_export::Summary` as a remote enum failed to
  compile with `E0277: the trait bound usize: uniffi::TypeId is not satisfied`, because its counts
  are `usize`. The spike's wrapper enum widens them to `u64`.
- **Fixed-size arrays and tuples are absent from the table.** Anything shaped like `[u8; 16]` (an id)
  or `(A, B)` crosses as a `Vec`, a string or a record. The spike's `decks()` turns
  `(DeckId, String)` into a `Deck` record and the id into its canonical string.
- **Objects are shared through `Arc`, so every method takes `&self`, and every exported type must
  be `Send + Sync`.** `Collection`'s `&mut self` methods need a `Mutex` around it (§5).
- **An error variant field called `message` generates Kotlin that does not compile**: it clashes
  with `Throwable.message` ("Overload resolution ambiguity"). The spike calls it `reason`.
- **External types need uniffi in the defining crate.** Using a type from another crate in an
  exported signature requires that crate to derive the uniffi traits itself, or the wrapper to
  restate it with `#[uniffi::remote(...)]`. A remote type has to match the original's shape
  exactly, so the `usize` limit above applies to it too.

## 3. Driving the Rust build from Gradle

**uniffi ships no Gradle plugin.** Its manual shows a plain `Exec` task ([Gradle
manual][uniffi-gradle]). The two tools in use:

- **cargo-ndk 4.1.2 (09.08.2025)** ([README][cargo-ndk]) runs cargo once per target with the NDK's
  clang, and exports `CC_<target>`, `AR_<target>` and the rest in the form the `cc` crate reads.
  The spike's command:

  ```sh
  cargo ndk -t arm64-v8a -t x86_64 -P 24 -o jniLibs build --release --lib
  ```

  `-o` writes `jniLibs/<ABI>/libcairn_ffi.so`, which is exactly the layout AGP takes prebuilt
  libraries from. `--link-builtins` and `--link-libcxx-shared` are there for libraries that need
  them, and this one needs neither (§4).
- **rust-android-gradle 0.10.0 (21.08.2026)** (`org.mozilla.rust-android-gradle.rust-android`,
  [README][rust-android-gradle]) is a Gradle plugin that drives cargo against the NDK toolchains
  itself, without cargo-ndk. 0.10.0 added AGP 8 support. Nothing in its README says it supports
  AGP 9.

**AGP 9.4.0 (September 2026)** needs Gradle 9.6.0 or later, JDK 17 and Build Tools 36. Its default
NDK is 28.2.13676358, and it supports API levels up to 37 ([AGP release notes][agp-notes]). Since
AGP 9.0, Kotlin support is built in: a project applies no `org.jetbrains.kotlin.android` plugin,
and that plugin is incompatible with the new DSL ([built-in Kotlin][agp-builtin-kotlin]). The docs
recommend pinning `ndkVersion` explicitly instead of relying on the default.

**The wiring AGP offers**: the Variant API's `Sources` has `getJniLibs()`, `getKotlin()` and
`getJava()` (read from the 9.4.1 `gradle-api` jar), and a task's output directory joins a source
set with `addGeneratedSourceDirectory(task, Task::outputDir)` ([extend AGP][agp-extend]). A cargo-ndk
task registered that way, and a bindgen task beside it, run before packaging without any hand-written
`dependsOn`. The spike took the shorter route of static `srcDir`s (§9), which only works because it
ran the two commands by hand first. Prebuilt libraries go under `src/main/jniLibs/<ABI>`
([jniLibs][agp-jnilibs]). `ndk.abiFilters` limits what is packaged, and an App Bundle installs only
the device's ABI ([ABIs][ndk-abis]).

**The local toolchain is behind.** [`docs/environment/android-toolchain.md`](../../environment/android-toolchain.md)
records AGP 8.11 and Gradle 8.14.3. It pinned the NDK 29 beta for a Tauri reason that no longer
applies. The spike used AGP 9.4.1 with Gradle 9.7.1 and the same NDK 29 for cargo-ndk, without
conflict, because AGP itself compiled no native code.

## 4. The C dependencies

**The bundled SQLite is the only C in the library.** `cargo tree -i cc` on the wrapper crate lists
one dependent: `libsqlite3-sys` 0.38.2 (from rusqlite 0.40 with `bundled`), which compiles SQLite
3.53.2 with `SQLITE_THREADSAFE=1`. `zip` decompresses through `zlib-rs` 0.6.8, which is pure Rust.
`zstd` only sits in `cairn-sync`, and that crate is outside this binding.

Under cargo-ndk it built for both ABIs with no extra flags or environment. `readelf -d` on the
result shows only `libc.so`, `libm.so` and `libdl.so` as `NEEDED`, and `nm -D` shows no undefined
`__*tf2` soft-float builtins, which is the case `--link-builtins` exists for.

**Every `LOAD` segment is aligned to `0x4000` (16 KB)**, in `libcairn_ffi.so` and in JNA's
`libjnidispatch.so`, for both ABIs. That meets Google Play's 16 KB page-size requirement
([page sizes][page-sizes]) with no linker flag: NDK r28 and later align to 16 KB by default.

## 5. Threading

**A synchronous uniffi call blocks the thread it is made on.** No thread is created for it. The
callback in the spike ran on the thread that called `countWithProgress`, a plain background
`Thread-2`. Android raises an ANR after 5 seconds without a response to input ([ANRs][anr]). Its
coroutine guidance is to make I/O and database work main-safe, moving it off the UI with
`withContext(Dispatchers.IO)` in a repository or ViewModel ([coroutine best
practices][coroutines-bp]). StrictMode is the documented way to catch work that slips onto the main
thread. Under ADR-0045 §7 this is established practice, and Cairn needs no decision of its own.

**One `Collection` per process, behind one lock.** `Collection` owns one rusqlite `Connection`, with
`derived.db` attached to it. `Connection` is `Send` but not `Sync`, so the exported handle has to
wrap it in a `Mutex`, as the spike does, and calls into it run one at a time. Async exports would not
help here: they would wait on the same lock.

## 6. What a bound library cannot keep

**`cairn_store::platform` panics.** Its Android arm reads `getFilesDir()` and `getNoBackupFilesDir()`
through `ndk_context::android_context()`, which `expect`s a context that only android-activity
installs. The spike called `platform::data_dir()` from Kotlin and got back
`InternalException: android context was not initialized`. `Collection::open(data_dir, state_dir)`
already takes both paths, so Kotlin passes `context.filesDir` and `context.noBackupFilesDir`. Measured:
`collection.db` was created under `files/`, and `derived.db` and `writer.id` under `no_backup/`,
the same split the store's `data_dir()` and `state_dir()` give. Those two functions are the whole of
the platform seam that [ADR-0009 §4](../../adr/0009-crate-and-workspace-layout.md) puts in the store,
so on Android the seam moves into Kotlin. As long as the Android arm stays in the crate, it links
`jni` and `ndk-context` into the library as dead code.

**`cairn_export::platform`'s Android arm panics for the same reason.** `put`, `get`, `list` and
`hand_off` call `MediaStore` and the share sheet through JNI on the same context. Under ADR-0045 §6
that seam moves to Kotlin anyway: the system picker and `ContentResolver` hold the file, and export
takes and returns bytes. The spike exported one bytes-level function, `summarise`, and it worked.
Which of the others to export is #195's question.

**The `cli` feature does not belong in the Android build.** `uniffi = { features = ["cli"] }` pulls
`uniffi_bindgen` into every target, so the bindgen binary should sit in a host-only crate or behind a
feature the NDK build leaves off. Measured on `arm64-v8a`, it costs 24 KB of `.so` (1,942,848 B with
it, 1,918,384 B without).

## 7. The logic that sits in `cairn-app`

The egui-free application logic is in `cairn-app`, not in the crates ADR-0045 §2 names:
`session.rs` (676 lines), `cards.rs`, `deck.rs`, `editor.rs`, `notes.rs`, `listing.rs`,
`optimise.rs` (which spawns its own thread), `sync.rs`, `inbound.rs`, `file_bench.rs` and
`fixtures.rs`. `cairn-app` depends on eframe, winit and android-activity, so a binding that depends on
it ships all three, along with a NativeActivity entry point the Kotlin app never uses. ADR-0045 §2 asks for one implementation, so the choice is to move
these modules into a crate the binding and the desktop both depend on. Rewriting them in Kotlin is
the other option, and §2 refuses it. Which crate, and where the line falls, is #195.

## 8. Size

Measured on the spike (release profile `opt-level = "z"`, `lto = true`, `strip = true`, as in the
workspace):

| Item | `arm64-v8a` | `x86_64` |
| --- | --- | --- |
| `libcairn_ffi.so` (store, export and core, with `cli`) | 1,942,848 B | 1,985,896 B |
| the same, `gzip -9` | ≈ 1.01 MB | — |
| JNA's `libjnidispatch.so` | 176,520 B | 126,912 B |

The JNA `.aar` is 525 KB in all, of which `classes.jar` is 223 KB. The libraries are **stored
uncompressed** in the APK, so the APK carries their full size and the download is what compresses
them. The two-ABI debug APK came to 5,833,230 B. The unminified release APK came to 5,402,986 B, of
which 3,439,612 B is `classes.dex`: the Kotlin stdlib, coroutines, JNA and the generated bindings,
with no R8. An App Bundle delivers one ABI, so a handset receives one `libcairn_ffi.so`.

**R8 needs keep rules for JNA, and the `.aar` ships none.** JNA's FAQ ([JNA FAQ][jna-faq]) gives:

```
-dontwarn java.awt.*
-keep class com.sun.jna.* { *; }
-keep class * extends com.sun.jna.* { *; }
-keepclassmembers class * extends com.sun.jna.* { public *; }
```

The minified release was not measured.

## 9. The spike

Kept out of the repository. Paths below are relative to the repository root, with the spike crate
beside `crates/`.

`Cargo.toml`:

```toml
[package]
name = "cairn-ffi-spike"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[lib]
name = "cairn_ffi"
crate-type = ["cdylib", "lib"]

[[bin]]
name = "uniffi-bindgen"
path = "src/bin/uniffi-bindgen.rs"   # fn main() { uniffi::uniffi_bindgen_main() }

[dependencies]
uniffi = { version = "=0.32.2", features = ["cli"] }
cairn-core = { path = "../crates/core" }
cairn-store = { path = "../crates/store" }
cairn-export = { path = "../crates/export" }

[profile.release]
opt-level = "z"
lto = true
strip = true
```

`uniffi.toml`:

```toml
[bindings.kotlin]
package_name = "dev.cairn.core"
android = true
```

`src/lib.rs`:

```rust
use std::path::Path;
use std::sync::Mutex;

uniffi::setup_scaffolding!();

#[derive(Debug, uniffi::Error)]
pub enum CairnError {
    Store { reason: String },
}
// Display, std::error::Error and From<cairn_store::StoreError> omitted.

#[derive(uniffi::Record)]
pub struct Deck { pub id: String, pub name: String }

#[derive(uniffi::Enum)]
pub enum FileSummary {
    Deck { decks: u64, notes: u64, retractions: u64 },
    Collection { created: String, notes: u64, reviews: u64 },
    Other { media_type: String },
    Unreadable,
}

#[uniffi::export]
pub fn summarise(bytes: Vec<u8>) -> FileSummary { /* widens Summary's usize counts to u64 */ }

#[derive(uniffi::Object)]
pub struct CollectionHandle(Mutex<cairn_store::Collection>);

#[uniffi::export]
impl CollectionHandle {
    #[uniffi::constructor]
    pub fn open(data_dir: String, state_dir: String) -> Result<Self, CairnError> {
        Ok(Self(Mutex::new(cairn_store::Collection::open(Path::new(&data_dir), Path::new(&state_dir))?)))
    }
    pub fn is_empty(&self) -> Result<bool, CairnError> { Ok(self.0.lock().unwrap().is_empty()?) }
    pub fn create_deck(&self, name: String) -> Result<String, CairnError> {
        Ok(self.0.lock().unwrap().create_deck(&name)?.to_canonical())
    }
    pub fn decks(&self) -> Result<Vec<Deck>, CairnError> {
        Ok(self.0.lock().unwrap().decks()?.into_iter()
            .map(|(id, name)| Deck { id: id.to_canonical(), name }).collect())
    }
}

#[uniffi::export]
pub fn platform_data_dir() -> Result<String, CairnError> {
    cairn_store::platform::data_dir()
        .map(|p| p.display().to_string())
        .map_err(|e| CairnError::Store { reason: e.to_string() })
}

#[uniffi::export(callback_interface)]
pub trait Progress: Send + Sync {
    fn step(&self, done: u32, total: u32);
}

#[uniffi::export]
pub fn count_with_progress(total: u32, progress: Box<dyn Progress>) -> u32 {
    for i in 1..=total { progress.step(i, total); }
    total
}

#[uniffi::export]
pub async fn ping() -> u32 { 1 }
```

Build and generate:

```sh
cargo build --release --lib
cargo run --bin uniffi-bindgen -- generate target/release/libcairn_ffi.so \
    --language kotlin --out-dir out --no-format
cargo ndk -t arm64-v8a -t x86_64 -P 24 -o jniLibs build --release --lib
```

The Android project, AGP 9.4.1 with built-in Kotlin. `app/build.gradle.kts`:

```kotlin
plugins { id("com.android.application") }

android {
    namespace = "dev.cairn.kbspike"
    compileSdk = 36
    defaultConfig {
        applicationId = "dev.cairn.kbspike"
        minSdk = 24
        targetSdk = 36
        ndk { abiFilters += listOf("arm64-v8a", "x86_64") }
    }
    sourceSets["main"].kotlin.srcDir("../spike/out")       // java.srcDir is not compiled as Kotlin
    sourceSets["main"].jniLibs.srcDir("../spike/jniLibs")
}

dependencies {
    implementation("net.java.dev.jna:jna:5.19.1@aar")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.10.2")
    implementation("androidx.annotation:annotation:1.9.1")
}
```

`MainActivity.onCreate` makes every call on one background `Thread`:

```kotlin
CollectionHandle.open(filesDir.absolutePath, noBackupFilesDir.absolutePath).use { c ->
    Log.i(t, "open ok, empty=${c.isEmpty()}")
    c.createDeck("Spike")
    Log.i(t, "decks=${c.decks()}")
}
Log.i(t, "summarise(empty)=${summarise(ByteArray(0))}")
countWithProgress(2u, object : Progress {
    override fun step(done: UInt, total: UInt) =
        Log.i(t, "progress $done/$total on ${Thread.currentThread().name}").let {}
})
Log.i(t, "ping=${runBlocking { ping() }}")
// then platformDataDir() and CollectionHandle.open("/nonexistent/x", "/nonexistent/y"), each in try/catch
```

What it logged:

```
open ok, empty=true
decks=[Deck(id=…, name=Spike)]
summarise(empty)=Unreadable
progress 1/2 on Thread-2
progress 2/2 on Thread-2
ping=1
platformDataDir threw dev.cairn.core.InternalException: android context was not initialized
bad open threw dev.cairn.core.CairnException$Store: reason=io: Read-only file system (os error 30)
```

## Open for #195

- Which crate holds the binding, and under what name. It cannot be `cairn-core`.
- Where the egui-free modules of `cairn-app` move (§7).
- Whether the Android arms of `cairn_store::platform` and `cairn_export::platform` are deleted or
  kept as dead code until the egui build retires (ADR-0045 §5).
- The surface of the export binding once the user-files seam moves to Kotlin.
- Which NDK to pin. AGP 9.4 defaults to 28.2, and the local toolchain has the 29 beta for a reason
  that no longer holds. Moving the local toolchain doc to AGP 9.4 with built-in Kotlin goes with it.
- The `Collection` owner on the Kotlin side: one instance, with calls serialised off the main thread.
- R8 keep rules, and a measured minified size.
- Whether to watch the JNI generator (#2911) and switch to it once it is released, dropping JNA.

## Sources

- uniffi changelog: [uniffi-changelog]
- uniffi manual, Kotlin: [uniffi-kotlin]; Gradle: [uniffi-gradle]; builtin types: [uniffi-builtin];
  futures: [uniffi-futures]
- uniffi JNI-based Kotlin bindgen: [uniffi-2911]
- cargo-ndk: [cargo-ndk]
- rust-android-gradle: [rust-android-gradle]
- AGP release notes: [agp-notes]; built-in Kotlin: [agp-builtin-kotlin]; extending AGP: [agp-extend];
  prebuilt native libraries: [agp-jnilibs]
- Android ABIs: [ndk-abis]; 16 KB page sizes: [page-sizes]
- ANRs: [anr]; coroutine best practices: [coroutines-bp]
- JNA FAQ: [jna-faq]

[uniffi-changelog]: https://github.com/mozilla/uniffi-rs/blob/main/CHANGELOG.md
[uniffi-kotlin]: https://mozilla.github.io/uniffi-rs/latest/kotlin/configuration.html
[uniffi-gradle]: https://mozilla.github.io/uniffi-rs/latest/kotlin/gradle.html
[uniffi-builtin]: https://mozilla.github.io/uniffi-rs/latest/types/builtin_types.html
[uniffi-futures]: https://mozilla.github.io/uniffi-rs/latest/futures.html
[uniffi-2911]: https://github.com/mozilla/uniffi-rs/pull/2911
[cargo-ndk]: https://github.com/bbqsrc/cargo-ndk
[rust-android-gradle]: https://github.com/mozilla/rust-android-gradle
[agp-notes]: https://developer.android.com/build/releases/gradle-plugin
[agp-builtin-kotlin]: https://developer.android.com/build/migrate-to-built-in-kotlin
[agp-extend]: https://developer.android.com/build/extend-agp
[agp-jnilibs]: https://developer.android.com/studio/projects/gradle-external-native-builds#jniLibs
[ndk-abis]: https://developer.android.com/ndk/guides/abis
[page-sizes]: https://developer.android.com/guide/practices/page-sizes
[anr]: https://developer.android.com/topic/performance/vitals/anr
[coroutines-bp]: https://developer.android.com/kotlin/coroutines/coroutines-best-practices
[jna-faq]: https://github.com/java-native-access/jna/blob/master/www/FrequentlyAskedQuestions.md
