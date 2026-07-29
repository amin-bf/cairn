# leitner-app

A local-first, offline-by-default spaced-repetition flashcard app in Rust, for **desktop, web and
Android**, with no server of our own.

Agent instructions live in [`AGENTS.md`](./AGENTS.md). Decisions live in [`docs/adr/`](./docs/adr).

## Stack

**egui / eframe** — one crate, one binary per platform, no webview, no IPC. Chosen in
[ADR-0003](./docs/adr/0003-client-stack.md) after building the same slice four ways and measuring
them on real hardware.

## Prerequisites

```sh
rustup target add wasm32-unknown-unknown aarch64-linux-android

cargo install trunk --locked      # web bundler
cargo install cargo-apk --locked  # Android packaging
```

**Linux desktop** needs no webkit — egui renders to a canvas:

```sh
# nothing beyond a working GPU/OpenGL stack
```

**Android** — source the environment script before *any* Android command. It sets `JAVA_HOME`,
`ANDROID_HOME` and `NDK_HOME` (see [`docs/environment/android-toolchain.md`](./docs/environment/android-toolchain.md)):

```sh
source scripts/android-env.sh
```

Skipping it produces `Failed to get android tools`, which does not say what is missing.

## Running

```sh
cargo run                                  # desktop
trunk serve                                # web
cargo apk build && adb install -r <apk>    # Android
cargo apk build --release                  # Android release (~5.4 MB)
```

Under Wayland, winit ignores `GDK_BACKEND`. For an X11 window (e.g. to drive it with `xdotool`):

```sh
env -u WAYLAND_DISPLAY WINIT_UNIX_BACKEND=x11 cargo run
```

## Where data lives

| Target | Location |
|---|---|
| Desktop | `$XDG_DATA_HOME/leitner/` |
| Android | `/data/user/0/<pkg>/files/` (via JNI — see `AGENTS.md`) |
| Web | OPFS, origin-scoped |

Web storage is a **best-effort** bucket — `navigator.storage.persisted()` is `false` by default and
the browser may evict it. The web build is not the system of record.
