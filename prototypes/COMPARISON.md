# #8 — measured comparison of three client stacks

Everything below was **run**, not inferred. Where a claim is inherited from
[`docs/research/client-stacks/`](../docs/research/client-stacks/README.md) rather than observed here,
it says so.

- **Machine**: CachyOS, Linux 7.1.5, rustc 1.97.0, JDK 17, NDK 29.0.13846066, Gradle 8.14.3.
- **Handset**: Pixel 8 Pro, Android 17 (API 37), arm64-v8a — the real device from
  [#7](https://github.com/amin-bf/leitner/issues/7), not an emulator.
- **Versions**: dioxus 0.7.9 / dx 0.7.9 · leptos 0.8.20 / tauri 2.11.5 / tauri-cli 2.11.4 /
  trunk 0.21.14. **A** = Dioxus standalone · **B** = Leptos+Tauri · **C** = Dioxus+Tauri.
- **The slice**: 3 hardcoded cards → front → 4-grade answer → back → append event → survive restart.
  Identical behaviour and identical on-disk JSON in all three.

---

## 0. The three options

| | Stack | Shape |
|---|---|---|
| **A** | Dioxus 0.7.9 standalone | one crate, `dx` builds everything |
| **B** | Leptos 0.8.20 + Tauri 2.11.5 | three crates, Trunk builds the frontend |
| **C** | **Dioxus 0.7.9 + Tauri 2.11.5** | three crates, `dx` builds the frontend |

C exists to ask whether Dioxus's UI ergonomics survive being put inside Tauri's shell. They do —
see §8.

---

## 1. Does the slice actually work?

| Target | A · Dioxus | B · Leptos+Tauri | C · Dioxus+Tauri |
|---|---|---|---|
| **Web** — persist + survive reload | ✅ OPFS | ✅ OPFS | ✅ OPFS |
| **Android** — survive force-stop | ✅ Pixel 8 Pro | ✅ Pixel 8 Pro | ✅ Pixel 8 Pro |
| **Desktop (Linux)** — survive restart | ✅ needs `xdotool` | ✅ | ✅ |

**All three targets pass for all three stacks.** Desktop was driven with `xdotool`: click a grade,
kill the process, relaunch, confirm the log reads back.

```
~/.local/share/leitner-dioxus-slice/review-log.jsonl
  {"card_id":0,"grade":3,"at_ms":1785184938597,"device":"dioxus-desktop"}

~/.local/share/dev.leitner.tauri-leptos-slice/review-log.jsonl
  {"card_id":0,"grade":3,"at_ms":1785185065599,"device":"tauri-desktop"}
```

### Desktop needs a different system dependency per stack

With `webkit2gtk-4.1` 2.52.5 installed, **Tauri desktop built and ran on the first try**.

**Dioxus desktop did not link until `xdotool` was also installed:**

```
rust-lld: error: unable to find library -lxdo
```

Traced with `cargo tree -i`:

```
libxdo-sys 0.11.0 → libxdo 0.6.0 → muda 0.17.2 → dioxus-desktop 0.7.9
                                 └→ tray-icon 0.21.3 → dioxus-desktop 0.7.9
```

`muda` (menus) and `tray-icon` are **unconditional** dependencies of `dioxus-desktop` 0.7.9, so a
Linux desktop build requires `xdotool` for features this app never uses. Tauri needed only
`webkit2gtk-4.1`. Small, but it is one more thing every contributor and CI image must install — and
it is visible in the UI too: the Dioxus window carries a native **Window / Edit / Help** menu bar we
did not ask for and would have to remove. The Tauri window has none.

### One architectural wrinkle, observed

A `cargo build` **debug** binary of the Tauri app is **not standalone** — it loads `devUrl`
(`localhost:8111`) rather than the bundled `frontendDist`, so with trunk stopped the window opens
and renders `Could not connect to localhost: Connection refused`. That is the two-process
architecture being literal: in dev, the app is a shell pointed at a web server. Release builds embed
`frontendDist` instead. Dioxus desktop has no equivalent split.

Proof captured on device, read back with `adb run-as`:

```
dioxus : /data/user/0/dev.leitner.dioxusslice/files/review-log.jsonl
tauri  : /data/user/0/dev.leitner.tauri_leptos_slice/review-log.jsonl
```

Both survived `am force-stop` + relaunch with the in-memory card position reset and the log intact.

---

## 2. Web persistence — and a correction to the research

Research headline finding 9 says *"web persistence forces a Web Worker, whichever stack you pick."*

**That is true only for the high (SQL) seam.** It follows from `FileSystemSyncAccessHandle` being
Dedicated-Worker-only, which is what an SQLite VFS needs. At the **low (bytes) seam** this map's data
model actually wants, the *async* OPFS API works on the main thread, and both slices persist through
it with **no worker, no `wasm_bindgen_futures` worker plumbing, and no cross-origin isolation
headers**:

```js
// verified in the running page, both stacks
{"bytes":136,"contents":"{\"card_id\":0,...}\n{...}\n","persisted":false,"quotaMB":10240}
```

Two caveats that hold for both stacks equally, so again not a discriminator:

- `navigator.storage.persisted()` is **`false`** — the default best-effort bucket. Matches research
  §3.3: the browser may evict. The web build must not be the system of record.
- OPFS is **origin-scoped**. The two slices on `:8110` and `:8111` had independent logs. Anything
  that changes the origin (Tauri's `useHttpsScheme`, a domain move) orphans the data.

**This materially lowers the cost of the web target for both stacks**, and it removes what the
research treated as the sharpest storage constraint. It should be re-checked before it is relied on
for anything larger than an append-only log.

---

## 3. Build and iteration times — the research's "single biggest unknown"

No measured times existed for either stack anywhere. These are ours.

| | A · Dioxus | B · Leptos+Tauri | C · Dioxus+Tauri |
|---|---|---|---|
| Web, cold build | **20.0s** | **23.6s** | **23.4s** |
| Web, incremental (Rust change) | 2.6s | **1.2s** | 2.66s |
| Web, incremental (markup only) | **~0s — hot reload, state preserved** | no equivalent | **~0s — hot reload, state preserved, *inside the Tauri window*** |
| Android, cold APK | **71s** | 85s | 36s † |
| Android, incremental APK | 6.8s | **5.4s** | — |
| `adb install` (debug APK) | 10.1s (66 MB) | 6.7s (129 MB) | (137 MB) |
| Unique crates in the wasm graph | **126** | 180 | **127** |

† C's Android build ran with a warm cargo cache from its own web build, so it is **not** comparable
to A's and B's cold numbers. Don't read it as C being twice as fast.

**Neither stack has a painful dev loop.** Both rebuild an Android APK in under 7 seconds
incrementally, which is far better than the "budget a Gradle assemble per change" framing the
research implies. The real day-to-day difference is narrower than expected and comes down to one
thing: **Dioxus hot-reloads markup with no rebuild at all and without losing app state.** For UI
iteration that is a genuine advantage; for logic changes the two are within ~1.5s of each other.

The 180-vs-126 crate gap is the CSR-only Leptos build still pulling in `server_fn`, `leptos_config`,
`serde_qs`, `url` and the whole `icu_*`/`idna` chain — confirming the research's note that
`server_fn` compiles unconditionally, and putting a number on it.

**Observed, contradicting the research:** it says pinning `tauri-cli ≥ 2.11.3` avoids a doubled Rust
compile per Android deploy. On **2.11.4** the doubled compile still happens — `slice-core` compiles
twice on every Android build (1.29s then 0.96s). Cheap here; not free on a real codebase.

---

## 4. Architecture — the difference that does not shrink

This is the finding that outlives every version number above.

**Dioxus is one crate.** The storage backend is a `#[cfg]`:

```rust
#[cfg(target_family = "wasm")]        mod imp { /* OPFS */ }
#[cfg(not(target_family = "wasm"))]   mod imp { /* file */ }
pub use imp::{now_ms, Store};
```

Call sites are identical on all three targets. The platform is known **at compile time**, so
`DEVICE` is a `const` chosen by `cfg`, and a target that does not compile fails the build.

**Leptos + Tauri is three crates and a JSON boundary.** The frontend is *always* wasm, so it cannot
`cfg` on the platform. Both backends compile into every build and the branch is a **runtime** test:

```rust
pub fn is_tauri() -> bool { /* window.isTauri */ }

pub async fn append(ev: &ReviewEvent) -> Result<(), String> {
    if is_tauri() { /* invoke → native */ } else { /* OPFS */ }
}
```

Three consequences, all observed while building it:

1. **The frontend cannot know its own platform in Rust.** To tell Tauri-desktop from Tauri-Android
   the slice has to sniff the user agent for `"Android"`. There is no compile-time answer.
2. **Both storage paths ship in every binary** — the web bundle contains the `invoke` path it will
   never take, and the Android bundle contains the OPFS path it will never take.
3. **A platform mismatch is a runtime bug, not a compile error.** Calling `invoke` outside Tauri
   throws a raw JS `TypeError` that Rust cannot convert to a `Result` (research §2.2, and the reason
   the `is_tauri()` gate exists at all). Get the gate wrong and you learn about it in the browser.

Line counts came out near-identical (**376** vs **365** lines of Rust), so this is not about volume.
It is about where the seam is: Dioxus's is a compile-time `cfg` the compiler checks; Tauri's is a
runtime `if` nobody checks.

### Scaffold and config

| | Dioxus | Leptos + Tauri 2 |
|---|---|---|
| Config files to author | 3 | 9 |
| Generated Android project | in `target/`, regenerated, **0 committed files** | in `src-tauri/gen/android/`, **44 committed files** |

Tauri's `gen/android/` is a real Android Studio project that lives in the repo — Kotlin sources,
Gradle files, manifests, resources. It is regenerable, but once committed it is a surface agents can
edit, drift, and conflict on. Dioxus keeps its equivalent inside `target/` and rebuilds it.

---

## 5. Android sharp edges, checked

| Claim (research) | Verdict here |
|---|---|
| Dioxus `tao` API-30 crash; effective floor Android 11 | **Not reproducible on API 37.** `tao 0.34.8` is what resolves, so the bug is present in the tree; the handset is simply far above the floor. Unchanged as a risk for old devices. |
| Tauri hardware back button exits the app (#14406) | **True — and true of Dioxus too.** Back exited both apps to the previous app/launcher. Shared wry/tao behaviour, **not a Tauri differentiator**. |
| Tauri release APK broken by default (`isMinifyEnabled`) | Template confirmed to set `isMinifyEnabled = true` for release. Build result: see §6. |
| Tauri `versionCode` silently falls back to `1` (#14413) | **Confirmed present**: `versionCode = tauriProperties.getProperty("tauri.android.versionCode", "1")`. |
| Tauri needs `version` ≥ `0.0.1` | Confirmed — `android build` refuses `0.0.0` outright. |
| `dx` autodetects JDK 11 on Linux and breaks | **Did not occur** with `JAVA_HOME` set to JDK 17 by `scripts/android-env.sh`. |

### One new finding: targetSdk, and it cuts against Dioxus

Observed on screen — the Tauri app draws its content **under the status bar**; the Dioxus app does
not. Cause, read from the generated Gradle:

| | compileSdk | targetSdk | minSdk |
|---|---|---|---|
| Dioxus | 34 | **34** | 24 |
| Tauri | 36 | **36** | 24 |

Tauri targets 36, where Android enforces edge-to-edge and the app must apply safe-area insets
itself — so Tauri's is the *correct modern* behaviour with a real layout cost we would have to pay.
Dioxus's tidier result comes from targeting 34, which is **below Google Play's minimum for new
apps**. Dioxus is not avoiding the inset work; it is deferring it.

**Checked, and it is a two-line fix.** `dx` does expose the keys — this in `Dioxus.toml`:

```toml
[android]
target_sdk = 36
compile_sdk = 36
```

produced `targetSdk = 36` / `compileSdk = 36` in the generated Gradle, verified by
`aapt2 dump badging` (`compileSdkVersion='36'`). **So `targetSdk` is not a Dioxus blocker** — treat
it as configuration, and expect to do the inset work either way. The release-variant problem in §6
is a different matter and does *not* have a config fix.

---

## 6. Release build — where both stacks land

This is the most consequential thing the prototype found, and it points the opposite way from what
the research predicted.

### Tauri: the release path works

Research finding 6 says *"The default template does not ship a working release build"* — signed
release APK crashes on launch, citing two open issues.

**It did not reproduce.** `cargo tauri android build --apk --target aarch64` (release,
`isMinifyEnabled = true`) produced a **13 MB** APK in 53s. Signed with a debug keystore via
`zipalign` + `apksigner`, installed on the Pixel 8 Pro, it **launched, rendered, accepted a tap, and
persisted an event across the `invoke` boundary** — meaning ProGuard did not strip the Kotlin plugin
classes that carry IPC. No `FATAL`, no `ClassNotFoundException`.

The two template defects the research names are still *present* — `isMinifyEnabled = true` and
`versionCode` defaulting to `1` — but the crash they were said to cause does not occur on
tauri 2.11.5 / tauri-cli 2.11.4 / API 37.

### Dioxus: the APK path is debug-only, but the AAB path is a real release

> **This section previously claimed Dioxus had no Play-shippable Android build. That was wrong**,
> and it was wrong because only the *APK* package type was tested. Corrected below. The error
> mattered — it was the sole basis of an earlier recommendation.

**What is true:** every APK route emits the **`debug` Gradle variant**.

```
dx build  --platform android --release                        → apk/debug/app-debug.apk
dx build  --platform android --release --device               → apk/debug/app-debug.apk
dx bundle --platform android --release --package-types apk    → apk/debug/app-debug.apk
```

`aapt2 dump badging` on that file reports **`application-debuggable`**, and `adb shell run-as`
succeeds against it. So for sideloaded APKs, Dioxus 0.7.9 gives you a debuggable package.

**What is not true:** that this leaves no shippable artifact. Switching the package type does invoke
Gradle's release task:

```sh
dx bundle --platform android --release --package-types aab --target aarch64-linux-android
```

```
→ app/build/outputs/bundle/release/DioxusSlice-aarch64-linux-android.aab   9.3 MB, 13.5s
```

Verified on that file:

| Check | Result |
|---|---|
| Gradle task | `packageReleaseBundle` — the **release** buildType (`isMinifyEnabled = true`) |
| `debuggable` in the bundle's `AndroidManifest.xml` | **attribute absent → defaults to false** |
| ABI | `base/lib/arm64-v8a/libmain.so` — correct for the handset |

**And AAB is the format that matters.** Google Play has required App Bundles for new apps since
August 2021; the APK is not the upload artifact. So the shipping path exists.

**And the release APK exists too — `dx` just never asks for it.** `dx` generates a real Gradle
project; invoking its unused task directly works:

```sh
cd target/dx/dioxus-slice/release/android/app && ./gradlew assembleRelease
→ app/build/outputs/apk/release/app-release-unsigned.apk    9.0 MB, 11s
```

Signed and **run on the Pixel 8 Pro**: launches, renders, the tap registers, the JNI data-dir path
survives minification, the event persists, and it **survives force-stop + relaunch**. Decisively:

```
$ adb shell run-as dev.leitner.dioxusslice ls
run-as: package not debuggable: dev.leitner.dioxusslice
```

`aapt2 dump badging` shows **no `application-debuggable` line**, `lib/arm64-v8a/`, `compileSdk 36`.

So this was never a capability gap — it is a **CLI gap**. The release buildType works; `dx`'s APK
commands simply do not invoke it, and one `gradlew` call gets you there.

One caveat that remains: **`--target` is required for AAB or you get the host triple.** Without it
the bundle contained `base/lib/x86_64/libmain.so` under the name
`DioxusSlice-x86_64-linux-android.aab`. Easy to ship by accident.

| | Dioxus | Leptos + Tauri 2 |
|---|---|---|
| Release **AAB** — the Play upload format | ✅ 9.3 MB, non-debuggable, correct ABI | ✅ |
| Release **APK**, via `dx` | ❌ debug variant only | ✅ |
| Release **APK**, via `./gradlew assembleRelease` | ✅ 9.0 MB, non-debuggable | ✅ |
| Release artifact run on the handset | ✅ persists, survives restart | ✅ |
| `targetSdk` meets Play minimum | ✅ via 2-line config (default 34) | ✅ 36 by default |

**Knock-on:** once `targetSdk` is raised to 36 — which Play requires anyway — the Dioxus app draws
under the status bar exactly like the Tauri one. The tidier insets noted in §5 were purely an
artefact of targeting 34, **not a property of the stack**. Both stacks owe the same safe-area work.

---

## 6b. A structural point that bears on the Leptos risk

Building the Tauri slice surfaced something the research could not: **the frontend framework is
close to incidental there.** The architecture is carried by the `shared` domain crate and the
`src-tauri` core — 87 lines between them — and Leptos accounts for 128 lines of view code sitting
behind a `store::{append, read_all}` interface that knows nothing about Leptos.

Swapping Leptos for Yew, Sycamore, or plain TypeScript would leave `shared/`, `src-tauri/`, the
`invoke` commands, the OPFS backend and the whole storage seam untouched.

That reframes research finding 3. **Leptos's maintenance risk is a swappable risk; Dioxus's is
not** — in the Dioxus slice the framework *is* the application, and its scaffold is what produces
the Android output. This is the research's own suggestion (its §2.3 closing line: Tauri with a
non-Leptos frontend "most directly answers the Leptos maintenance finding"), now with evidence that
the seam it depends on is real and cheap.

---

## 7. What this does *not* settle

- **Windows and macOS** — never built. Only Linux desktop was proven.
- **Whether OPFS-on-main-thread holds under load** — proven for an append-only log at 136 bytes, not
  for a large log or concurrent tabs. No VFS supports multiple connections; multi-tab is unhandled
  in both slices.
- **Leptos's maintenance risk**, which is a judgement about the future and cannot be prototyped.
  Research finding 3 stands untouched: sole maintainer, "lightly maintained", no confirmed handover.
- **Long-run agent legibility.** Both slices were written by one agent in one session without
  fighting either framework. Dioxus's stub-filled docs (research §2.1) did not bite at this size,
  and would not be expected to.

---

## 8. Option C — Dioxus inside Tauri

Built after A and B, to test the claim §6b made rather than assert it: that in a Tauri app the
frontend framework is nearly incidental and therefore swappable.

### The swap itself is the evidence

`ui/src/store.rs` was copied from the Leptos slice and **2 lines of 145 differ** — both cosmetic
device labels (`"leptos-web"` → `"dxtauri-web"`). No structural change. `shared/` and `src-tauri/`
were copied **unchanged**. Only the view layer was rewritten, from Leptos's `view!` to Dioxus's
`rsx!`.

So the swappability claim holds, measured rather than argued. It also means the same swap to Yew,
Sycamore or TypeScript would be similarly cheap.

### What C gains over B

| | B · Leptos+Tauri | C · Dioxus+Tauri |
|---|---|---|
| Crates in the wasm graph | 180 | **127** |
| Markup-only change | full rebuild + page reload | **hot reload, no rebuild, state preserved** |
| Frontend bundler | Trunk 0.21.14 (last stable 2025-05-08) | `dx` 0.7.9 |
| Web cold / incremental | 23.6s / 1.2s | 23.4s / 2.66s |

**Hot reload works inside the Tauri webview.** Verified on the running desktop app: with the window
open and mid-session — a card already graded and revealed — editing the markup updated the window in
place, kept the revealed state, and never restarted the process. This is the single feature that
made A attractive, and it survives the move into Tauri.

### What C gains over A

| | A · Dioxus | C · Dioxus+Tauri |
|---|---|---|
| Linux system packages | webkit2gtk **+ xdotool** | webkit2gtk |
| Unwanted native menu bar | yes (`muda`) | none — the window is Tauri's |
| Android data dir | 29 lines of hand-written JNI | `app_data_dir()` |
| Release APK via the CLI | ❌ debug variant (AAB or raw `gradlew` only) | ✅ |
| Core project health | pre-1.0, docs are stubs | Tauri post-1.0, documented |

C never depends on `dioxus-desktop`, so it never pulls `muda`/`tray-icon` → `libxdo`. Confirmed:
`cargo tree -i libxdo-sys` finds no match in either the `ui` or the `src-tauri` graph.

### What C costs — the same thing B costs

`ui/` is **always wasm**, so:

- the storage backend is a **runtime `if`** on `window.isTauri`, not a compile-time `#[cfg]`;
- the frontend sniffs the user agent to tell Android from desktop;
- both storage paths ship in every binary;
- a platform mismatch is a runtime bug, not a build failure.

**This is the whole trade.** A's single genuine advantage over B was the compile-time seam. C keeps
everything else about A and gives that up.

### New trap

`identifier` may not contain hyphens by the time it reaches Android:
`dev.leitner.dioxus-tauri-slice` installs as **`dev.leitner.dioxus_tauri_slice`**, so every `adb`
command against the configured name fails silently. Found the hard way.

---

## 9. Three-way summary

| | A · Dioxus | B · Leptos+Tauri | C · Dioxus+Tauri |
|---|---|---|---|
| Web / Android / desktop all pass | ✅ | ✅ | ✅ |
| Storage seam | **compile-time `cfg`** | runtime `if` | runtime `if` |
| Frontend knows its platform in Rust | ✅ | ❌ UA sniff | ❌ UA sniff |
| Crates in wasm graph | 126 | 180 | **127** |
| Rust LOC | 376 | 365 | 365 |
| Crates / config files / committed scaffold | 1 / 3 / 0 | 3 / 9 / 44 | 3 / 9 / 44 |
| Markup hot reload with state | ✅ | ❌ | ✅ |
| Linux system packages | webkit2gtk + xdotool | webkit2gtk | webkit2gtk |
| Android data dir | hand-written JNI | `app_data_dir()` | `app_data_dir()` |
| Release APK from the CLI | ❌ (AAB ✅, `gradlew` ✅) | ✅ | ✅ |
| Native menu bar you didn't ask for | ✅ present | none | none |
| Framework maintenance risk | Dioxus, **not swappable** | Leptos, swappable | Dioxus **frontend only**, swappable |

**How to read this.** C dominates B: everything B offers, plus hot reload and 53 fewer crates, at
identical architectural cost — the only thing B has over C is Leptos's larger ecosystem against
Dioxus's pre-1.0 churn.

A versus C is the real decision, and it is one question: **is the compile-time storage seam worth a
second system dependency, hand-written JNI, a menu bar to remove, a clumsier release path, and
pre-1.0 tooling whose docs are stubs?**

A says the compiler should catch platform mistakes. C says let Tauri own the platform and keep the
nice frontend. Both are defensible; they are not the same bet.

---

## 10. Option D — egui / eframe, the non-webview one

A, B and C are all the same rendering bet: Dioxus desktop/Android and Tauri both go through wry to a
system webview. D is the only slice that isn't — a single canvas drawn from Rust, no HTML, no CSS,
no IPC.

**It gets back the thing B and C gave up.** One crate, one binary per platform, `#[cfg]` picks the
storage backend, `DEVICE` is a `const`. No `invoke`, no `window.isTauri`, no user-agent sniffing.

Desktop and web both persist and survive restart, verified the same way as the others.

### And then the fonts

egui bundles its own fonts. Probed by rendering a string and reading the canvas:

| Script | egui | A / B / C (webview) |
|---|---|---|
| Latin + diacritics — `schön` | ✅ | ✅ |
| Cyrillic — `любовь` | ✅ | ✅ |
| Chinese / Japanese | ❌ boxes | ✅ |
| Arabic | ❌ boxes | ✅ |
| Arrows `→` `⇒` | ❌ boxes | ✅ |

A webview gets the system font stack for free. egui needs `ctx.set_fonts()` and a shipped font file —
a CJK face is 10–20 MB, on top of a debug wasm bundle already at **41 MB**.

**Shaping, however, works.** egui 0.35 uses **harfrust** (a pure-Rust HarfBuzz port) with skrifa, so
Arabic-script letters join correctly. Write-ups saying egui cannot shape are out of date.

**Bidi does not.** epaint's own source: `// TODO(emilk): heed bidi characters`. Probed with Persian
against Chrome as reference, and confirmed by the repo owner, who reads Persian:

| Test | Chrome | Android WebView | egui 0.35 |
|---|---|---|---|
| `فارسی`, `گچپژ`, `پنجره` — letters + joining | ✅ | ✅ | ✅ |
| `این یک جمله است` — sentence | ✅ | ✅ | ❌ wrong word order |
| …the same sentence **alone on its own line** | ✅ | ✅ | ❌ still wrong |
| `۱۲۳۴۵` — Persian digits | ✅ | ✅ | ❌ reversed |

**Not the OS and not the fonts** — same machine, same fonts, same session: Chrome correct, egui
wrong. Only the renderer changed.

Individual words *look* right because HarfBuzz shapes each run; what is missing is the algorithm that
decides the **order runs are placed in**. Not fixable by shipping a font — it needs bidi implemented
upstream in epaint.

**This settles D for this app.** The repo owner's own language is Persian; a flashcard app whose
cards render backwards is not a trade-off, it is a defect.

### Immediate mode versus an async platform API

The webview slices `await` OPFS inside a click handler. egui redraws the whole UI every frame and has
nowhere to await, so the web backend must be **fire-and-forget plus a shared slot the UI polls each
frame** (`INBOX` in `src/store.rs`). That layer exists purely because of immediate mode, and it would
have to wrap every async platform call the app ever makes — not just storage.

Also: no CSS means centring, max-width and responsive layout are all hand-written; the canvas simply
fills the viewport.

### Not attempted

**Android.** The font finding is decisive enough that packaging is moot until the script question is
answered. For the record, `cargo-apk` was last published **2023-11-30** and `xbuild` 0.2.0 is the
alternative — the "completely awful" setup the research quotes from egui's own Android PR author.

---

## 11. Where the four options stand

| | A · Dioxus | B · Leptos+Tauri | C · Dioxus+Tauri | D · egui |
|---|---|---|---|---|
| Rendering | webview | webview | webview | **canvas** |
| Web / Android / desktop verified | ✅✅✅ | ✅✅✅ | ✅✅✅ | ✅ / **not built** / ✅ |
| Storage seam | **compile-time** | runtime | runtime | **compile-time** |
| Async platform APIs | direct `await` | direct `await` | direct `await` | **poll a slot each frame** |
| Non-Latin scripts | ✅ system fonts | ✅ | ✅ | ❌ **CJK/Arabic missing** |
| Text shaping / bidi | ✅ | ✅ | ✅ | ❌ none |
| Markup hot reload | ✅ | ❌ | ✅ | ❌ |
| Layout | CSS | CSS | CSS | hand-written |

**B is dominated by C.** **D is gated on one domain question** — will decks ever be non-Latin? If
yes it is out; if no it is the only option that keeps a compile-time seam without a webview.

That leaves the live shortlist as **A, C, and conditionally D**.
