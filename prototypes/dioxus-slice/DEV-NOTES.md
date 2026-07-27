# Dev notes — Dioxus

Everything a developer or agent needs to build and run this stack, learned by doing it. **If Dioxus
wins #8, this file is the source for the repo `README.md` (setup + commands) and `AGENTS.md` (the
traps and the working rules).** If it loses, delete it.

Nothing here is inferred — every command was run and every trap was hit.

## 1. Prerequisites

**Rust**: stable. Verified on 1.97.0. Targets:

```sh
rustup target add wasm32-unknown-unknown aarch64-linux-android
```

**The `dx` CLI** — this is the whole build system; `cargo build` alone will not produce a web or
Android app:

```sh
cargo install dioxus-cli --locked   # verified: dx 0.7.9
```

**Linux system packages** (Arch names):

```sh
sudo pacman -S webkit2gtk-4.1 xdotool
```

- `webkit2gtk-4.1` — the desktop webview. Without it the desktop target does not build.
- `xdotool` — provides `libxdo`. **Not optional and not obvious**: it arrives via
  `muda` (menus) → `dioxus-desktop` and `tray-icon` → `dioxus-desktop`, both unconditional. Without
  it the link fails with `rust-lld: error: unable to find library -lxdo`, which names no crate and
  is hard to trace. Put it in the README, and in any CI image.

**Android**: source the repo's env script before *any* Android command. It sets `JAVA_HOME`,
`ANDROID_HOME`, `NDK_HOME` and the SDK paths from [#7](https://github.com/amin-bf/leitner/issues/7):

```sh
source scripts/android-env.sh
```

Skipping it produces `Failed to get android tools`, which does not say what is missing.

## 2. Commands

```sh
dx serve --platform web                  # http://localhost:8080, hot reload
dx serve --platform desktop
dx build --platform android --device     # dev APK; then adb install -r <apk>
dx serve --platform android              # build + install + launch on a connected device

# shipping: AAB, not APK — every APK route is the debug Gradle variant
dx bundle --platform android --release --package-types aab --target aarch64-linux-android
```

**`--target` is not optional for release bundles.** Omit it and `dx` silently builds the *host*
triple — you get `lib/x86_64/` inside a file named `…-x86_64-linux-android.aab`.

Measured on this machine: web cold 20s / incremental 2.6s; Android cold 71s / incremental 6.8s.

**Markup-only edits hot-reload with no rebuild and without losing app state.** This is the biggest
day-to-day advantage of the stack — prefer iterating in `rsx!` over restarting.

**Limitation that shapes crate layout**: Subsecond (the Rust hot-patcher) only tracks the *tip*
crate. Splitting a `core`/`domain` crate out of the workspace loses Rust hot-patching for it,
although `rsx!` hot-reload still works across the workspace. Weigh this when deciding
[#14](https://github.com/amin-bf/leitner/issues/14).

## 3. Storage, per platform

The seam is a compile-time `#[cfg]` in `src/store.rs` — one `Store` type, identical call sites:

| Target | Backend | Location |
|---|---|---|
| Desktop | file | `$XDG_DATA_HOME/leitner-dioxus-slice/` |
| Android | file | `/data/user/0/<pkg>/files/` |
| Web | OPFS | origin-scoped, async API, **main thread — no Web Worker needed** |

**Android has no data-dir API.** You write JNI yourself — see `src/android.rs`, 29 lines using
`ndk_context` + `jni` to call `getFilesDir()`. Budget for this; it is the one Android-only surface.

**`std::time::SystemTime::now()` panics on wasm.** Use `js_sys::Date::now()`. `store::now_ms()`
exists solely to hide this — anything time-related must go through a `cfg`-split helper.

**`dioxus-sdk-storage` is not usable here** — on web it is `localStorage`-only.

## 4. Traps hit

| Trap | Symptom | Fix |
|---|---|---|
| APK routes are debug-only | `--release` still emits `apk/debug/app-debug.apk`, `application-debuggable` | **Ship the AAB, not the APK** — see below. Play requires AAB anyway. |
| AAB silently built for the host triple | bundle contains `lib/x86_64/`, named `*-x86_64-linux-android.aab` | **always pass `--target aarch64-linux-android`** |
| `[android.signing]` appears to do nothing | parses, but no `signingConfig` in the generated Gradle | fields are `jks_file`/`jks_password`/`key_alias`/`key_password`; sign the AAB yourself if needed |
| `targetSdk` defaults to 34 | Below Play's minimum | `[android] target_sdk = 36` / `compile_sdk = 36` — verified to work |
| Missing `libxdo` | `unable to find library -lxdo` | `pacman -S xdotool` |
| Android env not sourced | `Failed to get android tools` | `source scripts/android-env.sh` |
| Native menu bar appears | Window / Edit / Help on desktop | comes from `muda`; must be removed deliberately |
| Launcher activity is `dev.dioxus.main.MainActivity` | `am start -n <pkg>/.MainActivity` fails | use `adb shell monkey -p <pkg> -c android.intent.category.LAUNCHER 1` |

## 5. Notes for agents

- **Read `dx` source, not the docs.** The 0.7 docs tree has ~55 zero-byte stub files, including every
  deploy guide and `tools/android.md`. Config keys are discoverable from the binary
  (`strings $(which dx)`) and from the parser's `missing field` errors, which is how
  `[android.signing]`'s four field names were found — they appear in no documentation.
- **Pre-1.0**: minor bumps are breaking. Pin the version.
- The whole app is one crate, so a target that does not compile **fails the build** rather than
  failing at runtime. Lean on that — prefer `#[cfg]` over runtime platform checks.
- Verify Android changes on the real handset. The emulator is x86_64; the Pixel 8 Pro is
  arm64-v8a only.
