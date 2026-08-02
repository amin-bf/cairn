# leitner-app

A local-first, offline-by-default spaced-repetition flashcard app in Rust, for **desktop and
Android**, with no server of our own.

Agent instructions live in [`AGENTS.md`](./AGENTS.md). The codebase entry point is
[`CONTEXT-MAP.md`](./CONTEXT-MAP.md). Decisions live in [`docs/adr/`](./docs/adr).

## Layout

A six-crate workspace, laid out in
[ADR-0009](./docs/adr/0009-crate-and-workspace-layout.md) and extended by
[ADR-0013 §11](./docs/adr/0013-the-sync-transport.md):

| Crate | What it is |
|---|---|
| `leitner-core` | The domain, entire and pure. One dependency, `fsrs` ([ADR-0027](./docs/adr/0027-the-scheduler-dependency.md)) — testable with no database, window or handset. |
| `leitner-store` | SQLite persistence and the two directory lookups (its platform seam is two functions wide). |
| `leitner-export` | The `.ldeck` deck-file container and import policy. Holds the zip dependency. |
| `leitner-sync` | Publishing the log to the remote and reading it back. Holds the network dependencies. |
| `leitner-app` | The egui application. `lib` + `cdylib`; the Android entry point and the window's inset seam live here. |
| `leitner-desktop` | A twenty-line shim, forced by `cargo-apk`. Keep it empty. |

## Stack

**egui / eframe** — one binary per platform, no webview, no IPC. Chosen in
[ADR-0003](./docs/adr/0003-client-stack.md) after building the same slice four ways and measuring
them on real hardware.

The web target was ruled out of scope in [ADR-0007 §1](./docs/adr/0007-the-local-store.md): for an
app whose only copy of the data is local, the browser is the one platform where "local" is not
reliably durable.

**One crate of that stack is not taken as published.** `vendor/egui-winit` is a verbatim copy of
`egui-winit` 0.35.0 with one block guarded off Android, wired in by `[patch.crates-io]`: as shipped,
every tap into a text field there dismisses and reopens the soft keyboard, for a composition that
platform cannot produce. So **a version bump of the stack is no longer only a version change** — see
[`vendor/PATCH.md`](./vendor/PATCH.md) and `scripts/verify-vendor.sh`
([ADR-0026](./docs/adr/0026-the-per-tap-keyboard-re-pop.md)).

## Prerequisites

```sh
rustup target add aarch64-linux-android
cargo install cargo-apk --locked  # Android packaging
```

**Linux desktop** needs no webkit — egui renders to a canvas, so a working GPU/OpenGL stack is all
it takes.

**Android** — source the environment script before *any* Android command. It sets `JAVA_HOME`,
`ANDROID_HOME` and `NDK_HOME` (see
[`docs/environment/android-toolchain.md`](./docs/environment/android-toolchain.md)):

```sh
source scripts/android-env.sh
```

Skipping it produces `Failed to get android tools`, which does not say what is missing.

## Running

```sh
cargo run -p leitner-desktop          # desktop
cargo test --workspace                # everything verifiable without hardware
cargo test -p leitner-core            # the domain alone: no database, no window, no handset
scripts/verify-vendor.sh              # the vendored adapter: verbatim plus exactly one change

source scripts/android-env.sh
cd crates/app && cargo apk build      # APK: a manifest and one .so, no classes.dex
```

`cargo apk build --release` compiles but stops at signing — no release keystore is configured, since
the only one available is a local developer debug key and deployment is out of scope.

Under Wayland, winit ignores `GDK_BACKEND`. For an X11 window (e.g. to drive it with `xdotool`):

```sh
env -u WAYLAND_DISPLAY WINIT_UNIX_BACKEND=x11 cargo run -p leitner-desktop
```

Verify UI judgements on the **real handset** — the emulator is x86_64 and the Pixel 8 Pro is
arm64-v8a only.

## Where data lives

Resolved by `leitner_store::platform`, which is two functions wide and stays that way.

**The platform surface is per crate, not per workspace** ([ADR-0016 §5](./docs/adr/0016-backup-and-restore.md)).
There are three modules under the same three-arm `#[cfg]` discipline, each answering a question its own
crate owns: `leitner_store::platform` the two directories below, `leitner_export::platform` the
user-visible files, and `leitner_app::platform` the window's insets — an inset being a fact about the
window the UI draws into, which the store has no business answering
([ADR-0025 §2](./docs/adr/0025-the-authoring-screen-under-a-soft-keyboard.md)). A **fourth function
appearing in any one of them** is the erosion signal, not a fourth module.

| Target | Collection (`collection.db`, `derived.db`) | Writer marker |
|---|---|---|
| Desktop | `$XDG_DATA_HOME/leitner/` | `$XDG_STATE_HOME/leitner/` |
| Android | `getFilesDir()`, via JNI | `getNoBackupFilesDir()`, via JNI |

The two are separate on purpose. Android's Auto Backup is on by default and restores the data
directory onto a replacement phone — including the writer identity, which would make two devices the
same writer and silently drop reviews. The marker sits outside the backup set so a restore becomes a
clean fork instead. See [ADR-0007 §6](./docs/adr/0007-the-local-store.md).
