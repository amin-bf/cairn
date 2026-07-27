# Dev notes — Dioxus + Tauri 2

Option **C**. **If this stack wins #8, this file is the source for the repo `README.md` (setup +
commands) and `AGENTS.md` (traps + working rules).** If it loses, delete it.

Nothing here is inferred — every command was run and every trap was hit.

## 1. Prerequisites

```sh
rustup target add wasm32-unknown-unknown aarch64-linux-android
cargo install dioxus-cli --locked                 # dx 0.7.9 — builds the frontend
cargo install tauri-cli --version "^2" --locked   # 2.11.4 — builds the shell
sudo pacman -S webkit2gtk-4.1                     # Linux desktop webview
```

**No `xdotool`, and no Trunk.** `ui/` uses `dioxus` with only the `web` feature, so it never pulls
`dioxus-desktop` → `muda`/`tray-icon` → `libxdo`. `dx` replaces Trunk as the frontend bundler.

**Android**: source the env script before *any* Android command, then init once per checkout:

```sh
source scripts/android-env.sh
cd src-tauri && cargo tauri android init
```

## 2. Commands

```sh
cd ui        && dx serve --platform web --port 8112   # plain web SPA
cd src-tauri && cargo tauri dev                       # desktop (runs dx serve itself)
cd src-tauri && cargo tauri android build --debug --apk --target aarch64
cd src-tauri && cargo tauri android build --apk --target aarch64        # release
```

Measured: web cold 23.4s, web incremental (Rust change) 2.66s, Android APK 36s with a warm cargo
cache.

**Hot reload works inside the Tauri webview and preserves app state.** Markup edits appear in the
running desktop window without a rebuild or a restart, mid-session. This is the reason to pick this
stack; iterate in `rsx!` rather than restarting.

## 3. Architecture — what you must internalise

Three crates and a JSON IPC boundary. `ui/` is **always wasm**, on every platform.

- **The frontend cannot know its platform from a Rust `cfg`.** It sniffs `navigator.userAgent` for
  `"Android"` to tell desktop from mobile.
- **The storage branch is a runtime `if`**, not a `#[cfg]`. Both backends compile into every binary.
  A platform mismatch is a runtime bug, not a compile error — no compiler safety net here.
- **Gate on `window.isTauri`** — never `window.__TAURI__` (defaults off), and never call `invoke`
  unguarded: outside Tauri it throws a raw JS `TypeError` Rust cannot catch as a `Result`.
- Everything crossing the boundary must be `serde`-serialisable to JSON.
- `shared/` must not touch the filesystem or `web_sys` — it compiles both natively and to wasm.

## 4. Storage, per platform

| Target | Backend | Location |
|---|---|---|
| Desktop | `invoke` → native file | `~/.local/share/<identifier>/` |
| Android | `invoke` → native file | `/data/user/0/<pkg>/` (`Context.dataDir`) |
| Web | OPFS from the frontend | origin-scoped, async API, **main thread — no Web Worker** |

`app.path().app_data_dir()` resolves the Android path with **no JNI** — that is the piece the
standalone Dioxus slice has to hand-write.

**OPFS is origin-scoped.** Flipping Tauri's `useHttpsScheme` after release orphans existing data.

## 5. Traps hit

| Trap | Symptom | Fix |
|---|---|---|
| Hyphens in `identifier` | `dev.leitner.dioxus-tauri-slice` installs as `dev.leitner.dioxus_tauri_slice`; `adb` commands silently fail | resolve with `adb shell pm list packages`; prefer an underscore-free identifier |
| Two path bases in one config file | `beforeBuildCommand` runs from the **project root**; `frontendDist` resolves relative to **`src-tauri/`** | write `cd ui && dx build …` and `../target/dx/<ui-crate>/debug/web/public` |
| `frontendDist` points into `target/` | `dx` writes the bundle to `target/dx/<crate>/{debug,release}/web/public`, so the path is profile-dependent | keep debug and release configs straight |
| `version = "0.0.0"` | android build refuses outright | must be ≥ `0.0.1` |
| `invoke` binding missing | IPC silently absent | `"withGlobalTauri": true` |
| `versionCode` defaults to `1` | Play rejects the *second* upload | set `tauri.android.versionCode` |
| Debug binary shows connection refused | dev server not running | start `dx serve`, or use `cargo tauri dev` |

## 6. Notes for agents

- **This stack has no compiler safety net for platform code.** The standalone Dioxus slice fails the
  *build* when a target is wrong; here you find out at runtime, in a webview. Test all three targets.
- **`src-tauri/gen/` is generated** — 44 non-ignored files land in the repo. Regenerate with
  `cargo tauri android init`; never hand-edit.
- Dioxus is **pre-1.0** — minor bumps are breaking, pin it. Its 0.7 docs tree has ~55 zero-byte stub
  files; config keys are discoverable via `strings $(which dx)` and the parser's `missing field`
  errors.
- Tauri is post-1.0 and stable within 2.x (plugins excepted), and its behaviour is documented — when
  the two disagree, trust Tauri's docs and read `dx`'s source.
- Verify Android on the real handset: the emulator is x86_64, the Pixel 8 Pro is arm64-v8a only.
