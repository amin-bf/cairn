# Dioxus + Tauri 2 slice — PROTOTYPE, throwaway

Option **C**. Answers [#8](https://github.com/amin-bf/leitner/issues/8). Measured results:
[`../COMPARISON.md`](../COMPARISON.md).

Built to ask one question: **can you keep Dioxus's UI ergonomics while getting Tauri's shell and
shipping story?** Short answer: yes, and it costs you the compile-time storage seam.

## Run it

```sh
source ../../scripts/android-env.sh              # android target only

cd ui        && dx serve --platform web --port 8112     # plain web SPA, no Tauri
cd src-tauri && cargo tauri dev                          # desktop
cd src-tauri && cargo tauri android build --debug --apk --target aarch64
```

## Shape

Same three crates as the Leptos slice — `shared/`, `ui/`, `src-tauri/` — with Dioxus in place of
Leptos in `ui/`. `dx` compiles the frontend to wasm; Tauri serves it and owns the disk.

| Crate | Compiled for | Role |
|---|---|---|
| `shared/` | both | Pure domain. Identical to the Leptos slice's. |
| `ui/` | **always wasm** | Dioxus CSR view + the runtime storage branch |
| `src-tauri/` | native | `#[tauri::command]`s. Identical to the Leptos slice's. |

## What building it proved

- **`store.rs` ported from the Leptos slice with 2 lines changed** out of 145 — and both were
  cosmetic device labels, not structure. That is the "the frontend is swappable" claim tested rather
  than asserted.
- **`dx` hot reload works inside the Tauri webview, with app state preserved.** Edited the markup
  with the desktop window open and mid-session; the window updated in place and stayed on the
  revealed card. Neither of the other two slices can do this.
- **No `xdotool`.** `ui/` depends on `dioxus` with only the `web` feature, so it never pulls
  `dioxus-desktop` → `muda`/`tray-icon` → `libxdo`. The window comes from Tauri, so there is also no
  unwanted native menu bar.
- **127 crates in the wasm graph** — level with standalone Dioxus (126), against Leptos's 180.

## What it costs

The same thing the Leptos slice costs, and for the same reason: `ui/` is **always wasm**, so the
storage backend is a **runtime** `if` on `window.isTauri`, not a compile-time `#[cfg]`. The frontend
cannot know its platform from Rust and sniffs the user agent. Both storage paths ship in every
binary. A platform mismatch is a runtime bug.

## Verified

- Web: OPFS, survived reload, `isTauri === false` correctly detected.
- Android (Pixel 8 Pro, API 37): persisted via `invoke` → `app_data_dir()`, survived force-stop.
- Desktop: persisted, survived restart.
- Hot reload inside the Tauri window, with state preserved.

## Trap

`identifier` may not contain hyphens once it reaches Android — `dev.leitner.dioxus-tauri-slice`
installs as **`dev.leitner.dioxus_tauri_slice`**, so `adb` commands against the configured name
silently fail. Resolve with `adb shell pm list packages`.
