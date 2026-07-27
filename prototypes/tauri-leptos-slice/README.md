# Leptos + Tauri 2 slice — PROTOTYPE, throwaway

Answers [#8](https://github.com/amin-bf/leitner/issues/8). Measured results:
[`../COMPARISON.md`](../COMPARISON.md).

## Run it

```sh
source ../../scripts/android-env.sh              # android target only

cd ui && trunk serve --port 8111                 # plain web SPA, no Tauri
cd src-tauri && cargo tauri dev                  # desktop — needs webkit2gtk-4.1, unproven
cd src-tauri && cargo tauri android build --debug --apk --target aarch64
```

## Shape

**Three crates and a JSON boundary** — the architecture the research described, built out:

| Crate | Compiled for | Role |
|---|---|---|
| `shared/` | both | Pure domain. Must not touch the filesystem. |
| `ui/` | **always wasm** | Leptos CSR view + the storage branch |
| `src-tauri/` | native | The `invoke` commands that touch the disk |

Because `ui/` is always wasm, the backend cannot be a `#[cfg]`. Both paths compile into every build
and the branch is a **runtime** test on `window.isTauri` — see [`ui/src/store.rs`](./ui/src/store.rs).
The frontend also cannot know its own platform in Rust, so it sniffs the user agent for `"Android"`.

## Config traps hit while building this

- `beforeBuildCommand` runs from the **project root**; `frontendDist` resolves relative to
  **`src-tauri/`**. Two different bases in one file.
- `version` must be ≥ `0.0.1` — `android build` refuses `0.0.0` outright.
- `withGlobalTauri: true` is required for the `window.__TAURI__.core.invoke` binding used here.

## Verified

- Web: events persisted to OPFS, survived reload, `window.isTauri === false` correctly detected.
- Android (Pixel 8 Pro, API 37): persisted via `invoke` → `app_data_dir()`, survived force-stop.
- **Release APK**: signed + minified, installs and runs correctly — the research's finding 6 did not
  reproduce. See COMPARISON §6.
- Desktop: **not run** — `webkit2gtk-4.1` missing.

## Note on `src-tauri/gen/`

`cargo tauri android init` generates a real Android Studio project there. 44 of those files are not
gitignored, so they land in the repo. They are regenerable but they are a surface that can drift.
