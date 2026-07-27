# #8 — measured comparison of the two client stacks

Everything below was **run**, not inferred. Where a claim is inherited from
[`docs/research/client-stacks/`](../docs/research/client-stacks/README.md) rather than observed here,
it says so.

- **Machine**: CachyOS, Linux 7.1.5, rustc 1.97.0, JDK 17, NDK 29.0.13846066, Gradle 8.14.3.
- **Handset**: Pixel 8 Pro, Android 17 (API 37), arm64-v8a — the real device from
  [#7](https://github.com/amin-bf/leitner/issues/7), not an emulator.
- **Versions**: dioxus 0.7.9 / dx 0.7.9 · leptos 0.8.20 / tauri 2.11.5 / tauri-cli 2.11.4 /
  trunk 0.21.14.
- **The slice**: 3 hardcoded cards → front → 4-grade answer → back → append event → survive restart.
  Identical behaviour and identical on-disk JSON in both.

---

## 1. Does the slice actually work?

| Target | Dioxus | Leptos + Tauri 2 |
|---|---|---|
| **Web** — persist + survive reload | ✅ OPFS, verified bytes on disk | ✅ OPFS, verified bytes on disk |
| **Android** — persist + survive force-stop | ✅ on the Pixel 8 Pro | ✅ on the Pixel 8 Pro |
| **Desktop (Linux)** — persist + survive restart | ✅ once `xdotool` is installed | ✅ |

**All three targets pass for both stacks.** Desktop was driven with `xdotool`: click a grade, kill
the process, relaunch, confirm the log reads back.

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

| | Dioxus | Leptos + Tauri 2 |
|---|---|---|
| Web, cold build | **20.0s** (175 crates) | **23.6s** (trunk) |
| Web, incremental (Rust change) | **2.6s** | **1.2s** |
| Web, incremental (markup only) | **~0s — RSX hot-reload, no rebuild, state preserved** | no equivalent; full rebuild + page reload |
| Android, cold APK | **71s** (16s Rust + 55s Gradle) | **85s** |
| Android, incremental APK | **6.8s** | **5.4s** |
| `adb install` (debug APK) | 10.1s (66 MB) | 6.7s (129 MB) |
| Unique crates in the wasm graph | **126** | **180** |

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
