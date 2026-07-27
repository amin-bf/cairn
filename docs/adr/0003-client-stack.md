# ADR-0003: The client stack

- **Status**: Accepted
- **Date**: 2026-07-28
- **Resolves**: [Prototype: pick the client stack](https://github.com/amin-bf/leitner/issues/8)
- **Map**: [Map: local-first Leitner app spec](https://github.com/amin-bf/leitner/issues/1)
- **Evidence**: [`docs/research/client-stacks/`](../research/client-stacks/README.md) and the four
  prototypes measured in `prototypes/COMPARISON.md` (branch `worktree-worktree-client-stack-8`,
  PR [#29](https://github.com/amin-bf/leitner/pull/29))
- **Related**: [ADR-0001](0001-scheduling-algorithm-and-grade-scale.md),
  [ADR-0002](0002-the-card-model.md)

## Context

The map fixes Rust, three targets (desktop, web, Android), offline-by-default, no server of our own,
and — decisively for this choice — **that agents implement the app**, so the stack must be
agent-legible rather than merely pleasant for a human who can intuit around gaps.

Research ([#3](https://github.com/amin-bf/leitner/issues/3)) gathered facts and deliberately chose
nothing. It left the single biggest unknown as *"no measured build or iteration times exist for
either stack, on any platform."*

Four candidates were built as the **same thin slice** — show a card front, take a graded answer, show
the back, append the event to a local log, survive a restart — and run on real hardware: a Pixel 8
Pro (Android 17, API 37, arm64-v8a), a Linux desktop, and Chrome.

| | Stack | Rendering |
|---|---|---|
| A | Dioxus 0.7.9 standalone | webview (wry) |
| B | Leptos 0.8.20 + Tauri 2.11.5 | webview |
| C | Dioxus 0.7.9 + Tauri 2.11.5 | webview |
| **D** | **egui / eframe 0.35** | **canvas** |

**All four passed all three targets.** Correctness did not decide this.

Two structural facts emerged that the research could not have supplied:

- **A, B and C are the same rendering bet.** Dioxus desktop/Android and Tauri both go through wry to
  a system webview. Choosing among them is choosing an arrangement of one architecture.
- **C dominates B.** Swapping Leptos for Dioxus inside Tauri cost 2 changed lines out of 145 in the
  storage layer and nothing in `shared/` or `src-tauri/`, while gaining hot reload and dropping 53
  crates. B is therefore not a live option.

## Decision

**egui / eframe (option D).**

### 1. Why not the webview stacks

The seam is the reason. In A the storage backend is a compile-time `#[cfg]`; a target that does not
compile fails the build. In B and C the frontend is *always* wasm, so it cannot know its platform
from Rust — it sniffs the user agent, both storage paths ship in every binary, and a platform
mismatch is a runtime bug in a webview rather than a compile error. Given that agents write this
code, a seam the compiler checks is worth more than one that reads nicely.

A keeps that seam but carries the webview anyway, plus an extra Linux system dependency
(`xdotool`, via `muda`/`tray-icon`, surfacing as an untraceable `-lxdo` link error), a native menu
bar we did not ask for, and no release-variant APK from its own CLI.

### 2. What egui buys

Measured, not inferred:

- **Compile-time storage seam**, one crate, one binary per platform, no IPC boundary, no dead code
  shipped, no user-agent sniffing.
- **Leanest dependency graph of the four** — 119 crates in the wasm graph (against 126 / 180 / 127)
  and 268 native (against A's 352).
- **Smallest release artifact** — a 5.4 MB Android APK, against A's 9.0 MB and B's 13 MB.
- **No Gradle project in the repository** — *if* we stay on `NativeActivity`. Packaged via
  `cargo-apk` with `android-native-activity`, the APK is a manifest and a `.so`: no Java, no Kotlin,
  no `classes.dex`, against 44 committed generated files for each Tauri option. **This advantage is
  conditional — see §6.**
- No webkit2gtk, and therefore no three-CSS-engine divergence.

### 3. Bidi is patched in our application, not upstream

egui shapes text correctly — harfrust infers direction from script, so Arabic-script letters join.
What epaint does not do is **order the runs**: it places them left-to-right in logical order, so a
Persian sentence renders with its words backwards and Persian digits reverse. Confirmed against
Chrome on the same machine and on the handset, so it is neither the OS nor the fonts.

Two workarounds were built and rejected before the working one was found:

| Attempt | Result |
|---|---|
| Reorder runs with `BidiInfo::visual_runs`, feeding a plain string | **No-op** — a pure-RTL sentence is one run, and epaint re-splits it |
| Reorder characters with `BidiInfo::reorder_line` | **Worse** — breaks harfrust's joining |

The fix that works exploits epaint's own contract that *"each section is an independent shaping
run"*, and that sections are laid out in the order given:

- run the Unicode bidirectional algorithm over the string;
- emit a `LayoutJob` whose **sections are in visual order**;
- inside an RTL run, emit its **words in reverse**, each word keeping logical character order so
  shaping is untouched;
- reverse runs of Arabic-Indic digits, which epaint emits RTL because they carry the Arabic script
  property. Digits have no joining behaviour, so reversing them is safe where reversing letters is
  not.

**~60 lines, no fork of epaint**, verified on Persian sentences, mixed Latin/Persian, and digits, and
confirmed by a Persian reader. See `prototypes/egui-slice/src/bidi.rs`.

**All card and UI text must be rendered through this helper.** Text rendered with a plain
`RichText`/`&str` bypasses it and will be wrong for RTL content. This is the single most important
rule in this ADR.

### 4. What we are knowingly giving up

These are properties of a canvas, not defects of egui, and no non-webview stack has them. A
hand-rolled UI would not restore them either — it would lose them by the same mechanism and cost
months.

- **No accessibility on web.** AccessKit has no web backend; a canvas exposes nothing to a screen
  reader.
- **No text selection and no find-in-page** on web. The web build is a single `<canvas>`.
- **Typed answers work, but RTL editing is awkward.** Tested directly: `TextEdit` accepts Latin and
  Persian input and stores it correctly. Its *display* bypasses the bidi helper unless you pass a
  custom `.layouter()` that routes through the same `LayoutJob` — do that everywhere. What remains
  is that the galley is then in **visual** order while the buffer is in **logical** order, so caret
  movement, selection and click-to-position are wrong for RTL text. [#11](https://github.com/amin-bf/leitner/issues/11)
  must be designed against that: prefer short single-line answers, avoid mid-string editing
  affordances, and do not assume a native text field's behaviour. IME composition (CJK) is separately
  weak, per egui's own docs — less relevant since Persian input is direct key mapping.
- **No hot reload.** A and C have it; egui does not.
- **Layout is hand-written** — centring, max-width and responsive behaviour are our code.
- **Fonts are ours to ship.** egui bundles only Hack, Ubuntu-Light and Noto Emoji. Arabic costs
  232 KB; **CJK costs 19 MB**, which is the practical bar on ever supporting CJK decks.
  **Install them on the first frame, never in `CreationContext`** — registering a font during
  creation breaks the *web* build on both renderers: `egui-wgpu` panics with *"Tried to update a
  texture that has not been allocated yet"*, and `glow` renders the entire UI near-black. Deferring
  the install one frame fixes both, and the default wgpu renderer is then fine. Found by testing;
  it costs an afternoon if you meet it cold.
- **An async platform call cannot be awaited in the frame.** Immediate mode redraws every frame, so
  results arrive via a handle polled per frame; the context must be woken with `request_repaint()`
  or a completed task sits unseen until the next input event.

### 5. Toolchain constraints this locks in

- **`cargo-apk`** for Android packaging. It works on NDK 29 / API 37 but was last published
  **2023-11-30** and is unmaintained. It also **panics after signing** (`Bin is not compatible with
  Cdylib`) when a crate has both a cdylib and a bin — the APK is correct, the exit code is not.
  **The desktop binary must therefore live in its own crate**, or CI breaks.
- **`eframe`'s dependency must be split per target.** Its default features include `accesskit`,
  which it refuses alongside `android-native-activity`.
- The Android data directory needs **hand-written JNI** (`ndk_context` + `jni`, ~29 lines) — Tauri's
  `app_data_dir()` was the only thing that avoided it.
- Under Wayland, winit ignores `GDK_BACKEND`; use `WINIT_UNIX_BACKEND=x11` when an X11 window is
  needed.

### 6. Android text input forces a choice we have not made yet

Found by typing into the app on the handset: **only Latin can be entered.** The cause is not egui.
`android-activity` 0.6.1's `NativeActivity` backend implements no input method at all — its
`set_text_input_state` and `set_ime_editor_info` are literally `// NOP: Unsupported`. Latin arrives
only because, in the library's own words, *"some soft keyboards will deliver physical key events for
basic ascii input"*, which it calls adequate *"for prototyping"* but *"unlikely to be sufficient for
production applications."* Persian is delivered via `InputConnection.commitText` and never reaches us.

`GameActivity` looked like the answer — real IME through GameTextInput. **It was built and tested,
and it does not help.** The Gradle project is kept at `prototypes/egui-slice/android/` so nobody
repeats the experiment.

**winit is the break, not the activity backend.**
`winit-0.30.13/src/platform_impl/android/mod.rs` handles exactly two input events —
`InputEvent::MotionEvent` and `InputEvent::KeyEvent`. There is no `Ime` handling and no call to
`text_input_state`. `set_ime_allowed` merely calls `show_soft_input`, raising the keyboard and then
discarding whatever it composes. GameActivity implements IME correctly underneath; winit never reads
it. Verified on the handset: Persian still could not be typed.

| | NativeActivity | GameActivity (tested) |
|---|---|---|
| Non-Latin text input | ❌ | ❌ **still broken** |
| APK | **5.4 MB**, manifest + `.so` | 19 MB, incl. 5.7 MB `classes.dex` |
| Packaging | `cargo-apk` | Gradle project + AAR + Kotlin conflict to resolve |
| Native accessibility | ❌ | ✅ accesskit permitted |

**So this is not a choice we get to make.** Non-Latin text input on Android is unavailable to any
egui/winit app today, at any packaging cost. Fixing it means implementing Android IME in **winit** —
two dependency layers below us, real platform-integration work, and far larger than the bidi patch.

Reverted to NativeActivity, since GameActivity's only remaining prize is native-only accessibility.

**Authoring is therefore a desktop/web activity.** Both accept Persian correctly — verified by
typing `من ۳ کتاب فارسی دارم` into the web build, right-aligned and correctly ordered, and Persian
sentences into the desktop build. Cards get authored there and reach the phone by sync. That is a
workable answer, but it **promotes sync from deferred to load-bearing**: it is no longer only a
multi-device convenience, it is the only route by which non-Latin content reaches Android at all.

**The consequence for [#11](https://github.com/amin-bf/leitner/issues/11) is concrete:** typed
answers on Android can only be entered in Latin. For German-answer decks that is survivable. For
Persian-answer decks it is not, and no amount of work inside this repository changes it. **This is
the strongest argument against this stack that the whole exercise produced**, and it is recorded
here rather than buried, because the webview options do not have it — a DOM text field gets full IME
for free.

## Consequences

- The storage seam is a compile-time `#[cfg]`, so platform mistakes fail the build. This is the
  property the whole choice rests on, and it must not be eroded by introducing a runtime platform
  branch later.
- **Every string shown to a user goes through the bidi helper.** A plain `ui.label("…")` on card
  content is a bug for RTL decks. This is the most likely way for an agent to silently break the app.
- We own a piece of text layout. If epaint implements bidi upstream, the helper should be deleted
  rather than kept alongside — two bidi passes would double-reverse.
- CJK decks are effectively out until someone accepts a 19 MB font in the bundle. Latin, Cyrillic and
  Arabic-script decks are supported.
- Typed-answer review ([#11](https://github.com/amin-bf/leitner/issues/11)) must be designed against
  weak IME support rather than assuming a native text field.
- Accessibility is not available on web. If it becomes a requirement, this ADR must be reopened —
  it cannot be added to a canvas cheaply.
- **Android typed answers are Latin-only until we move to GameActivity** (§6), and moving costs the
  no-Gradle-project property. This is the one open question this ADR does not close.
- `cargo-apk` is an unmaintained single point of failure for Android releases. If it breaks,
  `xbuild` 0.2.0 is the fallback, and the manifest-plus-`.so` APK is simple enough to assemble by
  hand.
