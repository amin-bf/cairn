# Cairn

A spaced-repetition flashcard app for **desktop and Android**, written in Rust. Local-first,
offline by default, with no server of ours anywhere in it.

## The name

**A cairn is a marker built one stone at a time.** Nobody raises one in a hurry. It is not a
signal and it asks nothing of you — it simply stands where it was put, outlasts the weather, and
tells whoever comes next that the path goes this way.

Four properties of this application are in that image, and each is a rule somewhere in
[`docs/adr/`](./docs/adr) rather than an aspiration:

- **Accretive.** A review is one line appended to a log that is never rewritten and never
  compacted. Everything else — what is due, what state a card is in — is *derived* by replaying
  it, so there is no summary that can drift away from what actually happened.
- **Durable, not urgent.** A card sits in a box from 1 to 5, computed from how long you would
  remember it rather than how soon you should act. Boxes are never counted, never sorted, and
  never shown as a queue: the app will not manufacture a backlog to push you through.
- **Quiet.** Nothing speaks unbidden — no streaks, no notifications, and where sync is concerned,
  no status icon, no badge and no success toast. Exactly two things are permitted to say anything
  about sync at all, and both are real problems you would want to know about. Even a backlog is
  framed rather than brandished: the wording at the end of a session is *"N still waiting, that's
  fine"*.
- **Local.** Your collection is a file on your device and that copy is the authoritative one.
  Sync is optional and goes to your own cloud drive; the remote is a meeting point for your
  devices, not a system of record, so deleting it costs one republish and no data.

The scheduler underneath is FSRS-6 — the boxes are a way of *reading* your memory, not the
mechanism driving it. The name was chosen after that decision rather than before it, and
[ADR-0028](./docs/adr/0028-the-application-is-named-cairn.md) records why the previous one had
to go: it promised a mechanism this design had already rejected.

---

Agent instructions live in [`AGENTS.md`](./AGENTS.md). The codebase entry point is
[`CONTEXT-MAP.md`](./CONTEXT-MAP.md). Decisions live in [`docs/adr/`](./docs/adr).

## Layout

A six-crate workspace, laid out in
[ADR-0009](./docs/adr/0009-crate-and-workspace-layout.md) and extended by
[ADR-0013 §11](./docs/adr/0013-the-sync-transport.md):

| Crate | What it is |
|---|---|
| `cairn-core` | The domain, entire and pure. One dependency, `fsrs` ([ADR-0027](./docs/adr/0027-the-scheduler-dependency.md)) — testable with no database, window or handset. |
| `cairn-store` | SQLite persistence and the two directory lookups (its platform seam is two functions wide). |
| `cairn-export` | The `.cdeck` deck-file container and import policy. Holds the zip dependency. |
| `cairn-sync` | Publishing the log to the remote and reading it back. Holds the network dependencies. |
| `cairn-app` | The egui application. `lib` + `cdylib`; the Android entry point and the window's inset seam live here. |
| `cairn-desktop` | A twenty-line shim, forced by `cargo-apk`. Keep it empty. |

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
cargo run -p cairn-desktop          # desktop
cargo test --workspace                # everything verifiable without hardware
cargo test -p cairn-core            # the domain alone: no database, no window, no handset
scripts/verify-vendor.sh              # the vendored adapter: verbatim plus exactly one change

source scripts/android-env.sh
cd crates/app && cargo apk build      # APK: a manifest and one .so, no classes.dex
```

`cargo apk build --release` compiles but stops at signing — no release keystore is configured, since
the only one available is a local developer debug key and deployment is out of scope.

Under Wayland, winit ignores `GDK_BACKEND`. For an X11 window (e.g. to drive it with `xdotool`):

```sh
env -u WAYLAND_DISPLAY WINIT_UNIX_BACKEND=x11 cargo run -p cairn-desktop
```

Verify UI judgements on the **real handset** — the emulator is x86_64 and the Pixel 8 Pro is
arm64-v8a only.

## Where data lives

Resolved by `cairn_store::platform`, which is two functions wide and stays that way.

**The platform surface is per crate, not per workspace** ([ADR-0016 §5](./docs/adr/0016-backup-and-restore.md)).
There are three modules under the same three-arm `#[cfg]` discipline, each answering a question its own
crate owns: `cairn_store::platform` the two directories below, `cairn_export::platform` the
user-visible files, and `cairn_app::platform` the window's insets — an inset being a fact about the
window the UI draws into, which the store has no business answering
([ADR-0025 §2](./docs/adr/0025-the-authoring-screen-under-a-soft-keyboard.md)). A **fourth function
appearing in any one of them** is the erosion signal, not a fourth module.

| Target | Collection (`collection.db`, `derived.db`) | Writer marker |
|---|---|---|
| Desktop | `$XDG_DATA_HOME/cairn/` | `$XDG_STATE_HOME/cairn/` |
| Android | `getFilesDir()`, via JNI | `getNoBackupFilesDir()`, via JNI |

The two are separate on purpose. Android's Auto Backup is on by default and restores the data
directory onto a replacement phone — including the writer identity, which would make two devices the
same writer and silently drop reviews. The marker sits outside the backup set so a restore becomes a
clean fork instead. See [ADR-0007 §6](./docs/adr/0007-the-local-store.md).
