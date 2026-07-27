# Dioxus slice — PROTOTYPE, throwaway

Answers [#8](https://github.com/amin-bf/leitner/issues/8). Measured results:
[`../COMPARISON.md`](../COMPARISON.md).

## Run it

```sh
source ../../scripts/android-env.sh     # required for the android target only

dx serve --platform web                 # http://localhost:8080
dx serve --platform desktop             # needs webkit2gtk-4.1 — NOT INSTALLED, unproven
dx build --platform android --device    # then: adb install -r <apk>
```

## Shape

One crate, three targets. The storage backend is a **compile-time** `#[cfg]` in
[`src/store.rs`](./src/store.rs); call sites are identical everywhere:

```rust
let store = Store::open().await;
store.append(&ev).await;
let all = store.read_all().await;
```

| File | What it is |
|---|---|
| `src/model.rs` | The 3 cards, the 4 grades, the event |
| `src/store.rs` | The seam: OPFS on wasm, file on native |
| `src/android.rs` | **The entire Android-only surface** — 29 lines of hand-written JNI |
| `src/main.rs` | The view |

## What Android cost

Dioxus exposes no data-dir API, so `src/android.rs` calls `getFilesDir()` over JNI via
`ndk_context` + `jni`. It works — resolved to `/data/user/0/dev.leitner.dioxusslice/files/` on the
Pixel 8 Pro — and it is 29 lines you own forever. The Tauri slice gets the same path from
`app.path().app_data_dir()`.

## Verified

- Web: events persisted to OPFS and survived a full reload; bytes confirmed on disk.
- Android (Pixel 8 Pro, API 37): events persisted and survived `am force-stop` + relaunch.
- Desktop: **not run** — `webkit2gtk-4.1` missing.

## Not verified / found broken

- Every **APK** route emits the debug Gradle variant (`application-debuggable`, unminified) —
  `dx build --release`, with or without `--device`, and `dx bundle --package-types apk`.
- The **AAB** route does produce a real release build, and AAB is what Google Play requires:
  `dx bundle --platform android --release --package-types aab --target aarch64-linux-android`
  → `outputs/bundle/release/*.aab`, non-debuggable, `lib/arm64-v8a/`. **Pass `--target` or you get
  the host triple silently.** Not yet launched on device — `bundletool` is not installed.
- `targetSdk` defaults to 34, below Google Play's minimum — **two-line fix**
  (`[android] target_sdk = 36`, `compile_sdk = 36`), verified in `Dioxus.toml` here.
