# Dev notes — Leptos + Tauri 2

Everything a developer or agent needs to build and run this stack, learned by doing it. **If this
stack wins #8, this file is the source for the repo `README.md` (setup + commands) and `AGENTS.md`
(the traps and the working rules).** If it loses, delete it.

Nothing here is inferred — every command was run and every trap was hit.

## 1. Prerequisites

**Rust**: stable. Verified on 1.97.0. Targets:

```sh
rustup target add wasm32-unknown-unknown aarch64-linux-android
```

**Two CLIs** — the frontend and the shell are built by different tools:

```sh
cargo install tauri-cli --version "^2" --locked   # verified: 2.11.4
cargo install trunk --locked                      # verified: 0.21.14
```

Pin `tauri-cli ≥ 2.11.3`. Note that **the doubled Rust compile per Android build still happens on
2.11.4** despite that being the documented fix — `slice-core` compiles twice every time.

**Linux system packages** (Arch names):

```sh
sudo pacman -S webkit2gtk-4.1
```

That is the only one. (Dioxus additionally needs `xdotool`; this stack does not.)

**Android**: source the repo's env script before *any* Android command:

```sh
source scripts/android-env.sh
```

Then, once per checkout:

```sh
cd src-tauri && cargo tauri android init    # generates gen/android/
```

## 2. Commands

```sh
cd ui        && trunk serve --port 8111                  # plain web SPA, no Tauri
cd src-tauri && cargo tauri dev                          # desktop (starts trunk itself)
cd src-tauri && cargo tauri android build --debug --apk --target aarch64
cd src-tauri && cargo tauri android build --apk --target aarch64   # release
```

Measured on this machine: web cold 23.6s / incremental 1.2s; Android cold 85s / incremental 5.4s.
**There is no hot reload** — every change is a rebuild plus a page reload.

**A debug binary is not standalone.** It loads `devUrl` (`localhost:8111`), *not* the bundled
frontend, so running `target/debug/slice-core` with no dev server shows
`Could not connect to localhost: Connection refused` inside the window. Release builds embed
`frontendDist`. Always start trunk first, or use `cargo tauri dev`.

## 3. Architecture — the thing to internalise

**Three crates and a JSON boundary.** This is not incidental; it constrains every feature.

| Crate | Compiled for | Rule |
|---|---|---|
| `shared/` | both | Pure domain. **Must not touch the filesystem or `web_sys`.** |
| `ui/` | **always wasm** | View + the storage branch |
| `src-tauri/` | native | `#[tauri::command]`s; everything touching disk |

Consequences an agent must know:

- **The frontend cannot know its platform from a Rust `cfg`** — it is wasm on every target. To tell
  Tauri-desktop from Tauri-Android it sniffs `navigator.userAgent` for `"Android"`.
- **Both storage backends ship in every binary**; the branch is a runtime `if`, not a `cfg`. A
  platform mismatch is a runtime bug, not a compile error.
- **Gate on `window.isTauri`** — never `window.__TAURI__` (defaults off), and never call `invoke`
  unguarded: outside Tauri it throws a raw JS `TypeError` that Rust **cannot** catch as a `Result`.
- Anything shared between frontend and core must be `serde`-serialisable to JSON — that is the IPC
  contract.

## 4. Storage, per platform

| Target | Backend | Location |
|---|---|---|
| Desktop | `invoke` → native file | `~/.local/share/<identifier>/` |
| Android | `invoke` → native file | `/data/user/0/<pkg>/` (`Context.dataDir`) |
| Web | OPFS directly from the frontend | origin-scoped, async API, **main thread — no Web Worker** |

`app.path().app_data_dir()` gives the Android path with **no JNI** — on Android it resolves to
`Context.dataDir`, app-private, no permission needed, removed on uninstall.

**OPFS is origin-scoped.** Changing the origin orphans the data — this includes flipping Tauri's
`useHttpsScheme` after release. Decide it once.

## 5. Traps hit

| Trap | Symptom | Fix |
|---|---|---|
| Two path bases in one config file | `../ui/Trunk.toml is neither a file nor a directory` | `beforeBuildCommand`/`beforeDevCommand` run from the **project root**; `frontendDist` resolves relative to **`src-tauri/`** |
| `version = "0.0.0"` | android build refuses outright | must be ≥ `0.0.1` |
| `invoke` binding not found | IPC silently absent | set `"withGlobalTauri": true` for the `window.__TAURI__.core.invoke` path |
| `versionCode` defaults to `1` | Play rejects the *second* upload | set `tauri.android.versionCode` explicitly |
| `gen/android/` in the repo | 44 non-ignored generated files | regenerable with `android init`; do not hand-edit |
| Debug binary shows connection refused | dev server not running | start trunk, or use `cargo tauri dev` |
| Gradle deprecation warnings | "incompatible with Gradle 9.0" | benign today; will need attention |

**Release APK does work** — signed and ProGuard-minified, it installs, launches and persists across
`invoke`. Sign with `zipalign` + `apksigner`; `isMinifyEnabled = true` did not break IPC.

## 6. Notes for agents

- **Leptos is "lightly maintained"** by a sole principal maintainer, feature-complete by his own
  statement, with a breaking 0.9 in beta. Expect low churn and low odds of a bug being fixed. Pin the
  version.
- **The frontend framework is the swappable part.** The architecture lives in `shared/` +
  `src-tauri/`; the view layer sits behind `store::{append, read_all}` and knows nothing about the
  rest. If Leptos becomes a problem, replacing it does not touch the seam.
- CSR-only Leptos still compiles `server_fn`, `leptos_config`, `serde_qs`, `url` and the whole
  `icu_*`/`idna` chain — 180 crates against Dioxus's 126. Do not expect `csr` to be lean.
- Use `LocalResource` + `<Suspense>`, not `Resource` — the latter carries `Serialize + Send` bounds
  that buy nothing without a server.
- `leptos_router` has **no hash routing**. On a passive static host a deep link 404s. Under Tauri
  this does not bite (custom protocol origin), so it is easy to miss until the web deploy.
- Verify Android changes on the real handset — the emulator is x86_64, the Pixel 8 Pro is arm64-v8a.
