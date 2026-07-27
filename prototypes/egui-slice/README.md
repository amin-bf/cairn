# egui / eframe slice — PROTOTYPE, throwaway

Option **D**, and the only **non-webview** slice. Answers
[#8](https://github.com/amin-bf/leitner/issues/8). Measured results:
[`../COMPARISON.md`](../COMPARISON.md).

Built because A, B and C are all the same rendering bet — Dioxus desktop/Android and Tauri both go
through wry to a system webview. This one draws a single canvas from Rust on every platform: no
HTML, no CSS, no IPC.

## Run it

```sh
cargo run                        # desktop
trunk serve --port 8113          # web
```

Under Wayland, winit ignores `GDK_BACKEND`; use `env -u WAYLAND_DISPLAY WINIT_UNIX_BACKEND=x11` if
you need an X11 window (e.g. to drive it with `xdotool`).

## What it gets right

- **The compile-time seam is back.** One crate, one binary per platform, `#[cfg]` picks the storage
  backend, `DEVICE` is a `const`. No `invoke`, no `window.isTauri`, no user-agent sniffing — the
  thing A had and B and C gave up.
- **No webview**, so no three-CSS-engine divergence and no wasm/native split in the UI layer.
- Desktop and web both persist and survive restart, verified.

## Text: what actually works, tested against Chrome

egui bundles only **Hack, Ubuntu-Light and Noto Emoji**, so out of the box anything outside
Latin/Cyrillic is a missing-glyph box — including `→`. That part **is fixable**: `ctx.set_fonts()`
with a supplied face makes CJK, Arabic and symbols render.

And egui 0.35 is better than its reputation here: it shapes text with **harfrust** (a pure-Rust
HarfBuzz port) plus **skrifa**. Arabic-script letters join correctly. This is recent — older write-ups
saying egui cannot shape are out of date.

**Persian, probed against Chrome as the reference rendering:**

| Test | Chrome | egui 0.35 |
|---|---|---|
| `فارسی` — plain word | ✅ | ✅ joined correctly |
| `گچپژ` — the four letters Arabic lacks | ✅ | ✅ all present |
| `پنجره` — joining | ✅ | ✅ |
| **`این یک جمله است` — pure-RTL sentence** | **✅** | ❌ **words in the wrong order** |
| **`۱۲۳۴۵` — Persian digits** | **۱۲۳۴۵** | ❌ **renders reversed as ۵۴۳۲۱** |
| mixed Latin + Persian | ✅ | ❌ |
| same sentence **alone on its own line** | ✅ | ❌ **still wrong** |

> **Corrected by the repo owner, who reads Persian.** I first marked the sentence row as correct by
> comparing pixel shapes. It is not — the *words* come out in the wrong order on egui while Chrome
> renders them correctly. Do not trust glyph-shape eyeballing for a script you cannot read; this
> single correction moves egui from "mostly fine, numbers are broken" to "not usable for Persian".

**The gap is bidi, not fonts and not shaping.** epaint's own source says so:

```rust
// TODO(emilk): heed bidi characters
/// need script-aware splitting once RTL/bidi support is added
```

HarfBuzz shapes each run, so individual words *look* right — letters join, the four Persian letters
are there. What is missing is the Unicode bidirectional algorithm, which decides the **order things
are placed in**. Without it, egui lays runs out in logical order, left to right. So a Persian
sentence reads backwards, and numbers come out reversed.

That is not fixable by shipping a font, and it is not a styling problem. It needs the bidi algorithm
implemented upstream in epaint.

**For a Persian flashcard app, this disqualifies egui today.** Not "adds a caveat" — the primary
content renders in the wrong order.

### It is not the OS, and it is not the fonts

Worth stating because it is the obvious suspect. The controlled comparison:

| | Same machine, same fonts, same session |
|---|---|
| Chrome | ✅ correct |
| **egui** | ❌ wrong |
| Android System WebView (Pixel 8 Pro) | ✅ correct |

Only the renderer changed. Confirmed twice by the repo owner reading the actual output, including
with the sentence **standalone on its own line** with no Latin text beside it — so it is not a
mixed-run artefact of how the probe was laid out either.

### The cost of the font fix

| | Size |
|---|---|
| `NotoSansArabic-Regular.ttf` | 232 KB |
| `NotoSansMath-Regular.ttf` (arrows) | 968 KB |
| `NotoSansCJK-Regular.ttc` | **19 MB** |
| egui's entire bundled set, for scale | ~1.4 MB |
| current debug wasm, before any of this | **41 MB** |

Arabic-script support is cheap. CJK is not — 19 MB embedded in the wasm bundle, which browsers must
download before the first frame. The webview stacks get every script from the system font stack for
nothing.

## Other costs observed

- **Immediate mode has nowhere to `await`.** The webview slices could await OPFS inside a click
  handler. egui redraws every frame, so the web backend has to be fire-and-forget plus a shared slot
  the UI polls each frame — see the `INBOX` in `src/store.rs`. That layer exists only because of
  immediate mode.
- **No CSS means layout is your code.** The canvas fills the viewport; centring, max-width and
  responsive behaviour are all hand-written.
- Typography is visibly egui's own and does not match a native or web app.
- Android was **not built** — the font finding above is decisive enough that packaging is moot until
  the script question is answered. `cargo-apk` was last published 2023-11-30; `xbuild` 0.2.0 is the
  alternative.

## Verified

- Desktop: persisted to `~/.local/share/leitner-egui-slice/`, survived restart.
- Web: persisted to OPFS through the polling layer, survived reload.
- Android: **not attempted** — see above.
