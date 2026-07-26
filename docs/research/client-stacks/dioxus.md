# Dioxus as a Rust multi-platform client stack

Research date: **2026-07-26**. Target consumer: local-first, offline-by-default, no-server-of-our-own spaced-repetition flashcard app; desktop + web + Android (no iOS).

All version facts were checked against crates.io API, the GitHub repo at tag `v0.7.9`, and the docsite source repo on this date. Where a claim rests on reading source rather than a doc page, the source file link is given.

---

## 1. Version & health

**Current stable: `dioxus` 0.7.9, published 2026-05-08.** Latest published version overall is `0.8.0-alpha.0` (2026-05-19), a prerelease. ([crates.io API for `dioxus`](https://crates.io/api/v1/crates/dioxus) — `max_stable_version: 0.7.9`, `newest_version: 0.8.0-alpha.0`; [git tags](https://github.com/DioxusLabs/dioxus/tags))

The `dioxus-cli` crate (which provides the `dx` binary) tracks the same version numbers: `max_stable 0.7.9`, `newest 0.8.0-alpha.0`. ([crates.io API for `dioxus-cli`](https://crates.io/api/v1/crates/dioxus-cli))

**Release cadence, last ~12 months** ([crates.io API for `dioxus`](https://crates.io/api/v1/crates/dioxus)):

| Version | Date |
|---|---|
| 0.7.0-alpha.0 | 2025-05-14 |
| 0.7.0-rc.0 | 2025-08-11 |
| 0.7.0 | 2025-10-31 |
| 0.7.1 | 2025-11-06 |
| 0.7.2 | 2025-12-05 |
| 0.7.3 | 2026-01-17 |
| 0.7.4 | 2026-03-27 |
| 0.7.5 | 2026-04-07 |
| 0.7.6 | 2026-04-22 |
| 0.7.7 | 2026-05-01 |
| 0.7.8 | 2026-05-07 |
| 0.7.9 | 2026-05-08 |
| 0.8.0-alpha.0 | 2026-05-19 |

Pattern: a long alpha/rc runway for 0.7 (May–Oct 2025), then ten patch releases in ~6 months, then a gap. **No stable release has shipped in the ~2.5 months since 2026-05-08**, and no 0.8 beta/rc since the single alpha on 2026-05-19.

**Commit activity**: 341 commits over the last 52 weeks, but heavily front-loaded — the trailing weeks of the GitHub participation series are `[..., 5, 1, 1, 0, 2, 2, 0, 1]`. ([GitHub participation stats API](https://api.github.com/repos/DioxusLabs/dioxus/stats/participation)) The repo is nevertheless live: the most recent commits are dated **2026-07-26** (`fix(cli): prevent duplicate SSG output on rebuild (#5702)`, `pin kstring and bump the lightningcss minimum (#5708)`). Contributors are a mix of core maintainers (`jkelleyrtp`, `ealmloff`, `nicoburns`) and many outside contributors. ([commits API](https://api.github.com/repos/DioxusLabs/dioxus/commits))

**Downloads**: 2,062,048 all-time, 670,619 recent. ([crates.io API for `dioxus`](https://crates.io/api/v1/crates/dioxus))

**Pre-1.0 / API stability**: Dioxus is pre-1.0 (0.7.x) and follows the cargo 0.x convention where minor bumps are breaking. The project states this explicitly for the 0.8 line: *"This release is the first in the 0.8 series. We've merged a number of breaking changes to internal APIs and slight behavior changes."* ([v0.8.0-alpha.0 release notes](https://github.com/DioxusLabs/dioxus/releases/tag/v0.8.0-alpha.0)) The 0.8 alpha also moved the workspace to Rust **edition 2024** ([PR #5502](https://github.com/DioxusLabs/dioxus/pull/5502)) and re-derives `#[non_exhaustive]` on `#[component]` props ([PR #5422](https://github.com/DioxusLabs/dioxus/pull/5422)).

Funding/maintainer note: the README says Dioxus grew "from a side project to a small team of fulltime engineers" funded by FutureWei, Satellite.im, and the GitHub Accelerator, with a stated goal of becoming "self-sustaining by providing paid high-quality enterprise tools." ([README](https://github.com/DioxusLabs/dioxus/blob/main/README.md))

**Documentation health is a real risk.** A large fraction of the 0.7 documentation tree consists of **zero-byte stub files**, including every deployment guide and every platform-API reference. Verified via the docsite git tree (size == 0):
`guides/deploy/android.md`, `guides/deploy/ios.md`, `guides/deploy/linux.md`, `guides/deploy/macos.md`, `guides/deploy/web.md`, `guides/deploy/windows.md`, `guides/deploy/ssg.md`, `guides/tools/android.md`, `guides/tools/serve.md`, `guides/tools/bundle.md`, `guides/apis/native.md`, `guides/apis/desktop.md`, `guides/apis/window.md`, `guides/apis/document.md`, `guides/apis/sdk.md`, `essentials/setup/devserver.md`, `essentials/setup/configuration.md`, `essentials/setup/tooling.md`, plus most of `essentials/advanced/` and `essentials/router/`. ([docsite git tree](https://github.com/DioxusLabs/docsite/tree/main/docs-src/0.7/src)) Agents implementing against this stack will frequently have to read `dx` source rather than docs.

**Unverified:** whether a 0.7.10 or 0.8.0-beta is imminent — there is no public roadmap issue or milestone page I could locate with dates for the 0.8 stable release.

---

## 2. Android

### What the project claims

The README markets it as first-class: *"## First-class Android and iOS support — Dioxus is the fastest way to build native mobile apps with Rust. Simply run `dx serve --platform android` and your app is running in an emulator or on device in seconds. Call directly into JNI and Native APIs."* ([README](https://github.com/DioxusLabs/dioxus/blob/main/README.md))

The mobile guide says: *"The Rust ecosystem for mobile continues to mature, with Dioxus offering strong support for mobile applications. Mobile is a first-class target for Dioxus apps, with a robust WebView implementation that supports CSS animations and transparency effects. … While native Android animations and widgets aren't currently supported, CSS-based animations and styling provide a powerful alternative."* ([mobile.md source](https://github.com/DioxusLabs/docsite/blob/main/docs-src/0.7/src/guides/platforms/mobile.md))

### Official setup, quoted verbatim

From [`docs-src/0.7/src/guides/platforms/mobile.md`](https://github.com/DioxusLabs/docsite/blob/main/docs-src/0.7/src/guides/platforms/mobile.md):

> First, install the Rust Android targets:
> ```sh
> rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
> ```
> To develop on Android, you will need to [install Android Studio](https://developer.android.com/studio).
> Once you have installed Android Studio, you will need to install the Android SDK and NDK:
> 1. Create a blank Android project
> 2. Select `Tools > SDK manager`
> 3. Navigate to the `SDK tools` window
>
> Then select:
> - The SDK
> - The SDK Command line tools
> - The NDK (side by side)
> - CMAKE
>
> Next set the Java, Android, NDK, and PATH variables:
>
> Mac:
> ```sh
> export JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home"
> export ANDROID_HOME="$HOME/Library/Android/sdk"
> export NDK_HOME="$ANDROID_HOME/ndk/25.2.9519653"
> export PATH="$PATH:$ANDROID_HOME/emulator:$ANDROID_HOME/platform-tools"
> ```
> Windows:
> ```powershell
> [System.Environment]::SetEnvironmentVariable("JAVA_HOME", "C:\Program Files\Android\Android Studio\jbr", "User")
> [System.Environment]::SetEnvironmentVariable("ANDROID_HOME", "$env:LocalAppData\Android\Sdk", "User")
> [System.Environment]::SetEnvironmentVariable("NDK_HOME", "$env:LocalAppData\Android\Sdk\ndk\25.2.9519653", "User")
> ```
> > The NDK version in the paths should match the version you installed in the last step

Note there is **no Linux example** in the docs, and the NDK version `25.2.9519653` shown is only an illustrative path, not a required minimum. The docs give **no minimum NDK version, no minimum JDK version, and no minimum SDK/API level.**

Then, running:

> Starting with Dioxus 0.6, `dx` ships with built-in support for mobile.
> ```sh
> dx new my-app
> ```
> ```sh
> emulator -avd Pixel_6_API_34  -netdelay none -netspeed full
> ```
> ```sh
> cd my-app
> dx serve
> ```

`dx doctor` is offered as a self-check: *"You can use the `dx doctor` command to see if `dx` can properly understand your install. This command helps provide insight into missing toolchains and tools required for cross-platform development."* ([getting_started/index.md](https://github.com/DioxusLabs/docsite/blob/main/docs-src/0.7/src/getting_started/index.md))

### What the toolchain *actually* requires (read from `dx` v0.7.9 source)

The docs are thin, so these are the real, hard numbers baked into the CLI.

**Rust target triples** — `dx` maps device architecture to triples in [`packages/cli/src/build/android.rs`](https://github.com/DioxusLabs/dioxus/blob/v0.7.9/packages/cli/src/build/android.rs): `aarch64-linux-android` → `arm64-v8a`, `armv7-linux-androideabi` → `armeabi-v7a`, `i686-linux-android` → `x86`, `x86_64-linux-android` → `x86_64`. Autodetection defaults to `aarch64-linux-android` and then queries `adb shell uname -m` to refine.

**`cargo-ndk` is NOT required.** `dx` reimplements it. The source comment is explicit:

> ```
> /// We pulled the environment setup from `cargo ndk` and attempt to mimic its behavior to retain
> /// compatibility with existing crates that work with `cargo ndk`.
> /// <https://github.com/bbqsrc/cargo-ndk/blob/1d1a6dc70a99b7f95bc71ed07bf893ef37966efc/src/cargo.rs#L97-L102>
> ```
> ([android.rs](https://github.com/DioxusLabs/dioxus/blob/v0.7.9/packages/cli/src/build/android.rs))

`dx` sets, per build: `JAVA_HOME`, `ANDROID_NDK_HOME`, `ANDROID_NDK_ROOT`, `ANDROID_SDK_ROOT`, `ANDROID_HOME`, `NDK_HOME`, `CC`/`CXX`/`AR`/`RANLIB`/`CFLAGS`/`CXXFLAGS` (triple-suffixed, `cc`-crate style), `CARGO_NDK_SYSROOT_PATH`, `CARGO_NDK_SYSROOT_TARGET`, `CARGO_NDK_SYSROOT_LIBS_PATH`, `ANDROID_NATIVE_API_LEVEL`, `CARGO_TARGET_<TRIPLE>_LINKER`, `BINDGEN_EXTRA_CLANG_ARGS_<triple>`, `OPENSSL_LIB_DIR`/`OPENSSL_INCLUDE_DIR`/`OPENSSL_LIBS`, and `WRY_ANDROID_PACKAGE=dev.dioxus.main`. This matters directly for §5: **C dependencies that build via the `cc` crate (e.g. bundled SQLite) get a correctly configured cross-compiler for free.** ([android.rs](https://github.com/DioxusLabs/dioxus/blob/v0.7.9/packages/cli/src/build/android.rs))

**JDK**: the generated Gradle module sets `jvmTarget = "17"`, `sourceCompatibility = JavaVersion.VERSION_17`, `targetCompatibility = JavaVersion.VERSION_17`. So **JDK 17** is the effective requirement. ([app/build.gradle.kts.hbs](https://github.com/DioxusLabs/dioxus/blob/v0.7.9/packages/cli/assets/android/gen/app/build.gradle.kts.hbs)) `dx` auto-detects `JAVA_HOME` from `/Applications/Android Studio.app/Contents/jbr/Contents/Home` (macOS), `C:\Program Files\Android\Android Studio\jbr` (Windows), and — notably — `/usr/lib/jvm/java-11-openjdk-amd64` on Linux, which is **JDK 11 and will not satisfy the JDK-17 `jvmTarget`**; set `JAVA_HOME` yourself on Linux. ([android.rs](https://github.com/DioxusLabs/dioxus/blob/v0.7.9/packages/cli/src/build/android.rs))

**Gradle / AGP / Kotlin** (all vendored into `dx`, downloaded by the wrapper):
- Gradle **9.1.0** — `distributionUrl=...gradle-9.1.0-bin.zip` ([gradle-wrapper.properties](https://github.com/DioxusLabs/dioxus/blob/v0.7.9/packages/cli/assets/android/gen/gradle/wrapper/gradle-wrapper.properties))
- Android Gradle Plugin **8.7.0**, Kotlin Gradle plugin **2.0.20** ([gen/build.gradle.kts](https://github.com/DioxusLabs/dioxus/blob/v0.7.9/packages/cli/assets/android/gen/build.gradle.kts))
- Runtime deps injected: `androidx.webkit:webkit:1.13.0`, `androidx.appcompat:appcompat:1.7.1`, `com.google.android.material:material:1.13.0` ([app/build.gradle.kts.hbs](https://github.com/DioxusLabs/dioxus/blob/v0.7.9/packages/cli/assets/android/gen/app/build.gradle.kts.hbs))

**SDK levels — and a config trap.** There are **two different `min_sdk` config keys with two different defaults**:
- `[android] min_sdk` → Gradle `minSdk`, **default 24**; `[android] target_sdk` → **34**; `[android] compile_sdk` → **34**. ([android.rs `AndroidHandlebarsObjects`](https://github.com/DioxusLabs/dioxus/blob/v0.7.9/packages/cli/src/build/android.rs), [manifest.rs `AndroidConfig`](https://github.com/DioxusLabs/dioxus/blob/v0.7.9/packages/cli/src/config/manifest.rs))
- `[application] android_min_sdk_version` → the **NDK/clang API level** used for `--target=<triple><api>` and `ANDROID_NATIVE_API_LEVEL`, **default 28**. Its own doc comment is wrong: it says *"If not set 24 is returned as a default"* while the code returns `28`. ([android.rs `min_sdk_version_or_default`](https://github.com/DioxusLabs/dioxus/blob/v0.7.9/packages/cli/src/build/android.rs))

So an untouched project declares `minSdk = 24` to the Play Store while compiling native code against API 28 headers — and, per the issue below, actually crashes below API 30.

### What the issue tracker says (docs vs. reality)

42 open issues match `android` on the repo. ([issue search](https://github.com/DioxusLabs/dioxus/issues?q=is%3Aissue+is%3Aopen+android)) The decision-relevant ones:

- **[#3401 "Not working on Android 9.0"](https://github.com/DioxusLabs/dioxus/issues/3401)** — open since 2024-12-19, 14 comments, last activity 2026-03-01. The app crashes at startup on API 28/29 with `java.lang.NoSuchMethodError: no non-static method "Landroid/view/WindowManagerImpl;.getCurrentWindowMetrics()..."`. Root cause is in `tao`: `getCurrentWindowMetrics` is API 30+. Community workaround is a `[patch.crates-io] tao = { path = "../tao" }` fork. **Effective minimum runtime is Android 11 / API 30 on Dioxus 0.7.x, not the declared 24/28.**
  - Upstream fix landed 2026-05-04 as [tao PR #1211 "fix(android): don't panic on getCurrentWindowMetrics on API<30"](https://github.com/tauri-apps/tao/pull/1211), released in **tao 0.35.2** ([tao CHANGELOG](https://github.com/tauri-apps/tao/blob/dev/CHANGELOG.md)).
  - But `dioxus-desktop` 0.7.9 depends on **`tao ^0.34.0`**, which cannot resolve to 0.35.x — **so the fix is not in any 0.7.x release.** ([crates.io deps for dioxus-desktop 0.7.9](https://crates.io/api/v1/crates/dioxus-desktop/0.7.9/dependencies)) `dioxus-desktop` 0.8.0-alpha.0 uses `tao ^0.35.2` / `wry ^0.55.1` and does get it. ([crates.io deps for dioxus-desktop 0.8.0-alpha.0](https://crates.io/api/v1/crates/dioxus-desktop/0.8.0-alpha.0/dependencies))
- **[#5637 "Support for armv7-linux-androideabi target was dropped starting with Dioxus 0.7.4"](https://github.com/DioxusLabs/dioxus/issues/5637)** — open, 2026-06-18. `dx bundle --android --release --target armv7-linux-androideabi` fails with *"Only 64-bit Android targets are supported"*, from a compile-time assertion added in manganis [PR #4842](https://github.com/DioxusLabs/dioxus/pull/4842). **This directly contradicts the official docs**, which still tell you to `rustup target add armv7-linux-androideabi i686-linux-android`. Reporter is pinned to 0.7.3; no fix.
- **[#5661](https://github.com/DioxusLabs/dioxus/issues/5661)** (2026-07-03) — on Windows, the `dx` linker response file uses backslashes and clang fails cross-compiling to Android.
- **[#5628](https://github.com/DioxusLabs/dioxus/issues/5628)** (2026-06-15) — Dioxus CSS asset fails to load during `dx serve` on Android.
- **[#5565](https://github.com/DioxusLabs/dioxus/issues/5565)** (2026-05-16) — `dx build --platform android`: OpenSSL `.so` not bundled in APK.
- **[#5356](https://github.com/DioxusLabs/dioxus/issues/5356)** (2026-03-10) — `with_activity` Android utility wrongly assumes `ndk-context` provides an Activity ref.
- **[#3685](https://github.com/DioxusLabs/dioxus/issues/3685)** — Android app icon configuration not working; open since 2025-02-04, still active 2026-07-21.
- **[#5118](https://github.com/DioxusLabs/dioxus/issues/5118)** (2025-12-19) — "Failed to build the template for Android" (Windows).
- **[#3762](https://github.com/DioxusLabs/dioxus/issues/3762)** — android platform fails to bundle on NixOS.
- **[#5653](https://github.com/DioxusLabs/dioxus/issues/5653)** (2026-06-28) — missing context menu and native text-selection toolbar on Android/iOS.
- **[#3849](https://github.com/DioxusLabs/dioxus/issues/3849)** — no mobile native file selection (open since 2025-03-09).

**Assessment:** Android is a *supported and actively worked-on* target with a genuinely integrated toolchain (no `cargo-ndk`, no hand-written Gradle), but the tail of open, long-lived platform bugs — a hard API-30 floor unfixed in the stable line, a silently dropped 32-bit target that contradicts the docs, Windows cross-compile breakage, asset-serving bugs — makes it "best-effort with strong tooling" rather than the "first-class" of the marketing copy.

---

## 3. Desktop rendering

**Desktop is webview-based, via wry/tao.** Confirmed in the docs:

> *"Apps built with Dioxus desktop use the system WebView to render the page. This makes the final size of application much smaller than other WebView renderers (typically under 5MB). … Dioxus desktop is built on top of [wry](https://github.com/tauri-apps/wry), a Rust library for creating desktop applications with a WebView."*
> — [desktop.md](https://github.com/DioxusLabs/docsite/blob/main/docs-src/0.7/src/guides/platforms/desktop.md)

Platform webviews: **WebView2 on Windows**, **WebKitGTK (`libwebkit2gtk-4.1-dev`) plus `xdotool`/`libxdo-dev` on Linux**, system WebKit on macOS with no extra deps. ([getting_started/index.md](https://github.com/DioxusLabs/docsite/blob/main/docs-src/0.7/src/getting_started/index.md))

Dependency versions for 0.7.9: `dioxus-desktop` → `wry ^0.53.5`, `tao ^0.34.0`, `muda ^0.17.0`. ([crates.io deps](https://crates.io/api/v1/crates/dioxus-desktop/0.7.9/dependencies)) 0.8.0-alpha.0 → `wry ^0.55.1`, `tao ^0.35.2`.

**Confirmed: desktop and mobile both go through wry.** This is not an assumption — it is a single crate. In `packages/dioxus/Cargo.toml` at v0.7.9:

```toml
desktop = ["dep:dioxus-desktop", "dioxus-config-macro/desktop"]
mobile  = ["dep:dioxus-desktop", "dioxus-config-macro/mobile"]
native  = ["dep:dioxus-native", "dioxus-config-macro/native"] # todo(jon): decompose the desktop crate such that "webview" is the default and native is opt-in
```
([dioxus/Cargo.toml @ v0.7.9](https://github.com/DioxusLabs/dioxus/blob/v0.7.9/packages/dioxus/Cargo.toml))

`dx` reinforces this: `--platform android` is *"Alias for `--target <device-triple> --renderer webview --bundle-format android`"*, and `--platform desktop` resolves to macos/windows/linux, each *"`--renderer webview`"*. ([packages/cli/src/platform.rs @ v0.7.9](https://github.com/DioxusLabs/dioxus/blob/v0.7.9/packages/cli/src/platform.rs)) Consequence: **one CSS/DOM rendering model across desktop, Android, and web** — but three different webview engines (WebView2, WebKitGTK, WKWebView, Android System WebView) whose CSS behaviour differs.

The legacy `dioxus-mobile` crate is stale at **0.6.2** and is not part of the 0.7 story. ([crates.io `dioxus-mobile`](https://crates.io/crates/dioxus-mobile))

### Blitz / `dioxus-native` — the native renderer

**Blitz is explicitly pre-alpha and the maintainers say do not build apps with it.** From the repo README:

> *"## Status — Blitz is currently in a **pre-alpha** state. It already has a very capable renderer, but there are also still many bugs and missing features. We are actively working on bringing it into a usable state but we would not yet recommend building apps with it."*
> — [DioxusLabs/blitz README](https://github.com/DioxusLabs/blitz/blob/main/README.md)

The 0.7 blog post is more optimistic but still hedges: *"Not every CSS feature is supported yet, with some bugs like incorrect writing direction or the occasional layout quirk"* and *"Bear in mind that Blitz is still considered a 'work in progress.' We have not focused on performance."* ([Dioxus 0.7 release post source](https://github.com/DioxusLabs/docsite/blob/main/docs-src/blog/src/release-070.md)) A live CSS support matrix is published at [blitz.is/status/css](https://blitz.is/status/css).

Versions: `blitz-dom` max stable **0.2.4** (2025-10-22), newest **0.3.0-beta.1** (2026-07-10). ([crates.io `blitz-dom`](https://crates.io/api/v1/crates/blitz-dom)) `dioxus-native` max stable **0.7.9**, newest 0.8.0-alpha.0, with only **21,470 total downloads** — i.e. essentially unused compared to `dioxus`'s 2M. ([crates.io `dioxus-native`](https://crates.io/api/v1/crates/dioxus-native)) The repo is actively developed (last push 2026-07-25; Blitz 0.3.0-beta.1 synced into Dioxus on 2026-07-11 via [PR #5673](https://github.com/DioxusLabs/dioxus/pull/5673)).

Roadmap: mobile (Android/iOS) support for Blitz is slated for **0.3 Beta, estimated August 2026**; 1.0 has no date and still lists unfinished basics — `position: static`/`fixed`, floats, shadow DOM, form controls (`select`, color picker, range), and the accessibility tree. ([Blitz roadmap issue #119](https://github.com/DioxusLabs/blitz/issues/119))

The desktop guide frames Blitz as future work: *"In the future, we plan to move to a custom web renderer-based DOM renderer with WGPU integrations (Blitz)."* ([desktop.md](https://github.com/DioxusLabs/docsite/blob/main/docs-src/0.7/src/guides/platforms/desktop.md))

**Bottom line: for a shipping app today, treat Dioxus as webview-only (`--renderer webview`). `--renderer native` exists and builds, but the upstream project says not to ship on it, and it has no Android support yet.**

---

## 4. Web target

**Mechanism: compile to `wasm32-unknown-unknown` with `wasm-bindgen`, render into the real DOM via `web-sys`.**

> *"To run on the Web, your app must be compiled to WebAssembly and depend on the `dioxus` and `dioxus-web` crates."*
> — [web.md](https://github.com/DioxusLabs/docsite/blob/main/docs-src/0.7/src/guides/platforms/web.md)

`dioxus-web` 0.7.9's non-optional dependencies confirm a pure wasm-bindgen/web-sys renderer with no webview involved: `wasm-bindgen ^0.2.100`, `web-sys ^0.3.77`, `js-sys ^0.3.77`, `wasm-bindgen-futures ^0.4.50`, `wasm-streams ^0.4.2`, plus the Dioxus core crates. ([crates.io deps](https://crates.io/api/v1/crates/dioxus-web/0.7.9/dependencies))

`dx --platform web` is *"Alias for `--target wasm32-unknown-unknown --renderer websys --bundle-format web`"*. ([platform.rs](https://github.com/DioxusLabs/dioxus/blob/v0.7.9/packages/cli/src/platform.rs)) Prerequisite: `rustup target add wasm32-unknown-unknown`. ([getting_started/index.md](https://github.com/DioxusLabs/docsite/blob/main/docs-src/0.7/src/getting_started/index.md))

**CSR vs SSR/hydration.** CSR is the default and works standalone. Hydration/SSR is a *fullstack* feature: *"Dioxus provides hydration to resume apps that are rendered on the server. See the fullstack reference."* ([web.md](https://github.com/DioxusLabs/docsite/blob/main/docs-src/0.7/src/guides/platforms/web.md)) In `dioxus`'s Cargo.toml, `hydrate` is pulled in only through the `fullstack` feature (`"dioxus-web?/hydrate"`), so a non-fullstack web build is plain CSR. ([dioxus/Cargo.toml @ v0.7.9](https://github.com/DioxusLabs/dioxus/blob/v0.7.9/packages/dioxus/Cargo.toml))

**Static output — no server required.** The `dx` bundler explicitly handles the SPA case:

> ```
> //! ### Web:
> //! Create a folder that is somewhat similar to an app-image (exe + asset)
> //! The server is dropped into the `web` folder, even if there's no `public` folder.
> //! If there's no server (SPA), we still use the `web` folder, but it only contains the public folder.
> //! web/
> //!     server
> //!     assets/
> //!     public/
> //!         index.html
> //!         wasm/
> //!            app.wasm
> //!            glue.js
> //!            snippets/
> //!         assets/
> //!            logo.png
> ```
> — [packages/cli/src/build/web.rs @ v0.7.9](https://github.com/DioxusLabs/dioxus/blob/v0.7.9/packages/cli/src/build/web.rs)

So a no-server build produces a static `public/` directory deployable to any static host or packaged offline.

**Bundle size.** The only official number is a doc claim, and it is old and unqualified:

> *"A build of Dioxus for the web will be roughly equivalent to the size of a React build (70kb vs 65kb) but it will load significantly faster because WebAssembly can be compiled as it is streamed."*
> — [web.md](https://github.com/DioxusLabs/docsite/blob/main/docs-src/0.7/src/guides/platforms/web.md)

**Unverified:** that 70 kb figure carries no compression basis, no Dioxus version, and no measurement methodology, and it appears unchanged from older docs. Treat it as marketing, not a budget. I found no primary-source measured bundle sizes for a 0.7.x app.

**Build pipeline (what actually determines size).** `dx bundle_web` runs, in order: `wasm-bindgen` (version auto-detected from the dependency graph and verified/installed by `dx` — `WasmBindgen::verify_install`), then optional bundle-splitting, then **`wasm-opt`**, then asset registration. In dev builds `dx` deliberately keeps debug symbols and names (`keep_debug`, `keep_names`), so dev `.wasm` is much larger than release. `esbuild` is auto-downloaded for JS asset processing. ([build/web.rs @ v0.7.9](https://github.com/DioxusLabs/dioxus/blob/v0.7.9/packages/cli/src/build/web.rs))

**Startup / lazy loading.** 0.7 shipped route-level WASM code splitting:

> *"Dioxus 0.7 introduces automatic code splitting and lazy loading for WebAssembly apps. Instead of shipping a single monolithic `.wasm` binary to the browser, `dx` now splits your app into smaller chunks based on your router. Each route's code is loaded on-demand as the user navigates."*
> ```rust
> #[derive(Routable, Clone, PartialEq)]
> enum Route {
>     #[route("/")] Home,
>     #[wasm_split("/dashboard")] Dashboard,
>     #[wasm_split("/settings")] Settings,
> }
> ```
> — [0.7 release post](https://github.com/DioxusLabs/docsite/blob/main/docs-src/blog/src/release-070.md)

This needs the `wasm-split` cargo feature on `dioxus`, and the release post notes *"to turn on the router splitter, you need to manually enable wasm-split on the router."* ([dioxus/Cargo.toml @ v0.7.9](https://github.com/DioxusLabs/dioxus/blob/v0.7.9/packages/dioxus/Cargo.toml))

Caveat carried by all wasm targets: *"Because of the limitations of Wasm, not every crate will work with your web apps, so you'll need to make sure that your crates work without native system calls (timers, IO, etc)."* ([web.md](https://github.com/DioxusLabs/docsite/blob/main/docs-src/0.7/src/guides/platforms/web.md))

A custom `index.html` is supported (must contain `<div id="main">`) and still hot-reloads. ([web.md](https://github.com/DioxusLabs/docsite/blob/main/docs-src/0.7/src/guides/platforms/web.md))

---

## 5. Local storage from Rust

### Desktop and Android: yes, directly — this is the officially endorsed path for local-first

A Dioxus maintainer answered exactly this question in [discussion #3898, "Desktop/Mobile apps with persistent local storage"](https://github.com/DioxusLabs/dioxus/discussions/3898):

> **@ealmloff**: *"Desktop and mobile apps already run using native code. They control a webview instance, but they don't run their code inside of a webview. That means you can use native APIs like reading the filesystem or creating a database directly from the desktop or mobile code. If you also want the option to ship a separate server binary for your web build, you can move your server functions behind a config flag."*

The desktop guide says the same: *"Although desktop apps are rendered in a WebView, your Rust code runs natively. This means that browser APIs are not available … However, native system APIs are accessible, so streaming, WebSockets, the filesystem, etc are all easily accessible."* ([desktop.md](https://github.com/DioxusLabs/docsite/blob/main/docs-src/0.7/src/guides/platforms/desktop.md))

**`rusqlite` with `bundled`**: current stable is **rusqlite 0.40.1** (2026-06-06), on `libsqlite3-sys ^0.38.1`. ([crates.io `rusqlite`](https://crates.io/api/v1/crates/rusqlite)) `libsqlite3-sys` builds SQLite's C source via the `cc` crate (`cc ^1.2.27` build-dep) under the `bundled` feature. ([crates.io deps for libsqlite3-sys 0.38.1](https://crates.io/api/v1/crates/libsqlite3-sys/0.38.1/dependencies))

**Does NDK cross-compilation of the C library "just work"?** Yes, in the sense that `dx` supplies exactly the environment `cc`-rs needs — `CC_aarch64-linux-android`, `CFLAGS_...` containing `--target=aarch64-linux-android<api>`, `AR`, `RANLIB`, `CARGO_NDK_SYSROOT_PATH`, `CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER`, plus `-lstatic=clang_rt.builtins-aarch64-android` rustflags. ([android.rs `android_env_vars`](https://github.com/DioxusLabs/dioxus/blob/v0.7.9/packages/cli/src/build/android.rs)) I found **zero open issues in the Dioxus tracker mentioning sqlite/rusqlite/libsqlite3**, which is consistent with this working. **Unverified:** I did not compile it; treat this as "the environment is correct by construction" rather than an empirically confirmed build. Note the related, *unresolved* [#5565 "OpenSSL .so not bundled in APK"](https://github.com/DioxusLabs/dioxus/issues/5565) shows that native-lib packaging into the APK is not always seamless — though `bundled` SQLite is statically linked into `libmain.so` and so should not hit that class of bug.

**Android app-private data dir path from Rust.** There is **no Dioxus API for this** — you obtain it via JNI. Two community patterns from [discussion #3475](https://github.com/DioxusLabs/dioxus/discussions/3475):

```rust
// Pattern A — ndk-context + jni (unsafe raw handles)
fn get_files_dir() -> Result<String> {
    let ctx = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }?;
    let mut env = vm.attach_current_thread()?;
    let ctx = unsafe { jni::objects::JObject::from_raw(ctx.context().cast()) };
    let dir = env.call_method(ctx, "getFilesDir", "()Ljava/io/File;", &[])?.l()?;
    let dir: jni::objects::JString =
        env.call_method(&dir, "toString", "()Ljava/lang/String;", &[])?.l()?.try_into()?;
    Ok(env.get_string(&dir)?.to_str()?.to_string())
}
```

```rust
// Pattern B — wry::prelude::dispatch, no unsafe, mirrors Tauri's approach
#[cfg(target_os = "android")]
fn app_local_data_dir() -> Result<PathBuf> {
    let (tx, rx) = std::sync::mpsc::channel();
    fn run(env: &mut JNIEnv<'_>, activity: &JObject<'_>) -> Result<PathBuf> {
        let files_dir = env.call_method(activity, "getFilesDir", "()Ljava/io/File;", &[])?.l()?;
        let files_dir: JString<'_> = env
            .call_method(files_dir, "getAbsolutePath", "()Ljava/lang/String;", &[])?.l()?.into();
        Ok(PathBuf::from(env.get_string(&files_dir)?.to_string_lossy().to_string()))
    }
    wry::prelude::dispatch(move |env, activity, _webview| tx.send(run(env, activity)).unwrap());
    rx.recv().unwrap()
}
```

Both are user-contributed, not official. Relevant crates: `ndk-context` **0.1.1** (last published 2022-04-19 — unmaintained but ubiquitous, 37M downloads) and `jni` 0.21.x. ([crates.io `ndk-context`](https://crates.io/api/v1/crates/ndk-context)) Caveat: [#5356](https://github.com/DioxusLabs/dioxus/issues/5356) reports that Dioxus's own `with_activity` helper "assumes `ndk-context` provides Activity ref" and is buggy — Pattern B via `wry::prelude::dispatch` is the safer bet.

`android_activity::AndroidApp::internal_data_path` is the "obvious" API but the discussion's opening question is precisely that **there is no way to get the `AndroidApp` handle from a Dioxus app**; it went unanswered. **Unverified:** whether 0.7/0.8 exposes `AndroidApp` anywhere.

**What `dioxus-sdk` gives you (and why it is not enough).** `dioxus-sdk` 0.7.0 (2025-11-06) ships `dioxus-sdk-storage` with `use_persistent` / `use_singleton_persistent`. Its backends, read from its Cargo.toml:
- non-wasm: the **`directories` crate 4.0.1**, writing serialized (ciborium + `yazi` compression) blobs to a data directory; exposes `data_directory`, `set_dir_name`, `set_directory`;
- Android: additionally `jni 0.21.1` + `ndk-context 0.1.1`;
- wasm: `web-sys` with only the `Window`, `Storage`, `StorageEvent` features — i.e. **`localStorage`/`sessionStorage` only** (`pub use client_storage::{LocalStorage, SessionStorage}`).
([packages/storage/Cargo.toml](https://github.com/DioxusLabs/sdk/blob/main/packages/storage/Cargo.toml), [packages/storage/src/lib.rs](https://github.com/DioxusLabs/sdk/blob/main/packages/storage/src/lib.rs))

The SDK README warns: *"These crates are still under development. Expect breaking changes!"* ([SDK README](https://github.com/DioxusLabs/sdk/blob/main/README.md)) For a flashcard corpus, `localStorage` (~5 MB, synchronous, string-only) is not a viable web backend.

### Web: no filesystem — the options, and which have working Rust bindings

**The single most important finding here: `rusqlite` itself supports `wasm32-unknown-unknown` out of the box as of rusqlite 0.38.** In `rusqlite`'s Cargo.toml:

```toml
# On wasm32-unknown-unknown builds use the sqlite-wasm-rs crate instead of libsqlite3-sys
ffi-sqlite-wasm-rs = ["dep:sqlite-wasm-rs"]
default = ["cache", "ffi-sqlite-wasm-rs"]
```
with `libsqlite3-sys` gated to `cfg(not(all(target_family = "wasm", target_os = "unknown")))` and `sqlite-wasm-rs ^0.5.1` gated to `cfg(all(target_family = "wasm", target_os = "unknown"))`.
([rusqlite Cargo.toml](https://github.com/rusqlite/rusqlite/blob/master/Cargo.toml), [crates.io deps for rusqlite 0.40.1](https://crates.io/api/v1/crates/rusqlite/0.40.1/dependencies))

Corroborated by [rusqlite issue #1828](https://github.com/rusqlite/rusqlite/issues/1828) (closed 2026-04-15): *"starting with `rusqlite` 0.38 `wasm32-unknown-unknown` builds now unconditionally depend on `sqlite-wasm-rs`"* — the issue was a request to opt *out*, which is what the `ffi-sqlite-wasm-rs` default feature now allows. **Practical consequence: the same `rusqlite` query code can compile for desktop, Android, and web wasm.**

**`sqlite-wasm-rs`** — current **0.5.5** (2026-05-25), 4.97M total / 3.89M recent downloads, MSRV 1.85.0. ([crates.io](https://crates.io/api/v1/crates/sqlite-wasm-rs)) VFS backends live in the companion crate **`sqlite-wasm-vfs` 0.2.0** (2026-01-17, requires sqlite-wasm-rs ≥ 0.5.2):

| VFS | Storage | Contexts | Multiple connections | Full durability | Relaxed durability |
|---|---|---|---|---|---|
| `memory` (default) | RAM | All | ✗ | ✅ | ✗ |
| `sahpool` (SyncAccessHandlePoolVFS) | **OPFS** | **Dedicated Worker only** | ✗ | ✅ | ✗ |
| `relaxed-idb` (RelaxedIdbVFS) | **IndexedDB** | All | ✗ | ✗ | ✅ |

([sqlite-wasm-rs README](https://github.com/Spxg/sqlite-wasm-rs/blob/master/README.md), [docs.rs](https://docs.rs/sqlite-wasm-rs/latest/sqlite_wasm_rs/))

Key constraints for the app design:
- **No COOP/COEP headers or SharedArrayBuffer needed** for any of the three VFS — good for plain static hosting. ([sqlite-wasm-rs README](https://github.com/Spxg/sqlite-wasm-rs/blob/master/README.md))
- **Not thread-safe** — SQLite is compiled with `-DSQLITE_THREADSAFE=0` and JsValue is not `Send`. ([docs.rs](https://docs.rs/sqlite-wasm-rs/latest/sqlite_wasm_rs/))
- **`sahpool`/OPFS requires running in a dedicated Web Worker.** A Dioxus web app runs on the main thread by default, so OPFS-backed SQLite means either moving DB work into a worker (non-trivial with Dioxus's main-thread signal model) or falling back to `relaxed-idb`.
- `relaxed-idb` runs on the main thread and needs no worker, at the cost of **relaxed durability** — acceptable for flashcard review state, less so for anything you cannot re-derive.
- Encryption is available via the `sqlite3mc` feature (SQLite3MultipleCiphers).

**Other web options and their Rust-binding status:**
- **`sqlite-wasm-vfs` 0.2.0** — 115,993 downloads; author self-describes as *"some experimental VFS implementations"*. ([crates.io](https://crates.io/api/v1/crates/sqlite-wasm-vfs))
- **`diesel-wasm-sqlite`** — a Diesel backend that *"allows SQLite instantiation in web workers to take advantage of OPFS."* ([crates.io](https://crates.io/crates/diesel-wasm-sqlite))
- **wa-sqlite** — JavaScript, not Rust; the project's own discussion notes that a pure-Rust IDBBatchAtomicVFS is blocked because there are *"no direct WebAssembly bindings for IndexedDB, making it impossible to use IndexedDB for SQLite storage entirely in Rust without involving JavaScript."* ([wa-sqlite discussion #154](https://github.com/rhashimoto/wa-sqlite/discussions/154))
- **`dioxus-client-storage`** — *"Unified storage API for Dioxus (IndexedDB, LocalStorage, SessionStorage)"*, but v**0.0.3** with **81 total downloads**, third-party (`eftech93`), last published 2026-05-03. Not production-grade. ([crates.io](https://crates.io/api/v1/crates/dioxus-client-storage))
- **`dioxus-local-storage`** — third-party, last published **2024-06-04** (v0.4.0), 1,493 downloads. Stale relative to 0.7. ([crates.io](https://crates.io/api/v1/crates/dioxus-local-storage))

**What the official docs recommend — and why it's the wrong shape for this project.** The Dioxus "Working with Databases" tutorial does use SQLite, but gates it to the server:

> *"To add sqlite functionality to HotDog, we'll pull in the `rusqlite` crate. Note that `rusqlite` is only meant to be compiled on the server, so we'll feature gate it behind the `"server"` feature in our Cargo.toml."*
> ```toml
> rusqlite = { version = "0.32.1", optional = true }
> [features]
> server = ["dioxus/server", "dep:rusqlite"]
> ```
> — [tutorial/databases.md](https://github.com/DioxusLabs/docsite/blob/main/docs-src/0.7/src/tutorial/databases.md)

That is a client/server example (and pins a rusqlite version, 0.32.1, that is eight releases stale). There is **no official local-first storage guide**; the closest thing is the maintainer's discussion answer quoted above.

**`sqlx`**: mentioned by name in the fullstack docs as a third-party option you must bring yourself (*"You'll need to pull in 3rd-party crates like `Sqlx` and `tower-sessions`"*), always in a server context. ([fullstack/index.md](https://github.com/DioxusLabs/docsite/blob/main/docs-src/0.7/src/essentials/fullstack/index.md)) **Unverified:** I found no primary source on `sqlx` + Android NDK cross-compilation under `dx`, and note the open upstream [rusqlite #1735 "Cross-compiling target is not respected when using bundled `libsqlite-sys` (via sqlx)"](https://github.com/rusqlite/rusqlite/issues/1735). `sqlx` also has no `wasm32-unknown-unknown` SQLite story comparable to rusqlite's. For a three-target local-first app, rusqlite is the better-evidenced choice.

---

## 6. Dev loop

### What `dx serve` does per platform

`dx serve` takes cargo-style args and a platform alias. ([packages/cli/src/cli/serve.rs @ v0.7.9](https://github.com/DioxusLabs/dioxus/blob/v0.7.9/packages/cli/src/cli/serve.rs)) The aliases expand as ([platform.rs](https://github.com/DioxusLabs/dioxus/blob/v0.7.9/packages/cli/src/platform.rs)):

- `--platform web` → `--target wasm32-unknown-unknown --renderer websys --bundle-format web`
- `--platform desktop` → resolves to `macos` / `windows` / `linux` by host; each → `--target <host> --renderer webview --bundle-format <os>`
- `--platform android` → `--target <device-triple> --renderer webview --bundle-format android`
- also: `ios`, `server`, `liveview`

Each platform gets its own cargo profile, so builds don't invalidate each other: `web-dev`/`web-release`, `desktop-dev`/`desktop-release`, `android-dev`/`android-release`, `ios-dev`/`ios-release`, `server-dev`/`server-release`. ([fullstack/project_setup.md](https://github.com/DioxusLabs/docsite/blob/main/docs-src/0.7/src/essentials/fullstack/project_setup.md)) `dx` also auto-enables the matching cargo feature (`web`/`desktop`/`mobile`/`server`) if you declare it.

**Android specifically**, `dx` generates a full Gradle project tree (`build.gradle.kts`, `gradle.properties`, `gradlew`, `settings.gradle`, wrapper jar, `AndroidManifest.xml`, `MainActivity.kt`) into a cache dir, then runs `gradle assembleDebug`. ([android.rs](https://github.com/DioxusLabs/dioxus/blob/v0.7.9/packages/cli/src/build/android.rs)) It then, per the documented sequence:

> 1. **Enable Root Access**: `adb root`
> 2. **Port Forwarding**: `adb reverse tcp:<port> tcp:<port>` — *"Forwards the development server port from the host machine to the Android simulator, enabling communication between the app and the dev server."*
> 3. **APK Installation**: `adb install -r <apk_path>`
> 4. **Environment Variables**: writes a `.env` file and `adb push`es it to the device
> 5. **App Launch**: `adb shell am start -n <package_name>/<activity_name>`
>
> — [packages/cli/src/build/builder.rs @ v0.7.9](https://github.com/DioxusLabs/dioxus/blob/v0.7.9/packages/cli/src/build/builder.rs)

The 0.7 post confirms this works on physical devices: *"Thanks to community contributions, `dx serve --platform android` now supports Android devices! You can edit markup, modify assets, and even hot-patch on a real Android device without needing to boot a simulator. This works by leveraging `adb reverse`."* ([0.7 release post](https://github.com/DioxusLabs/docsite/blob/main/docs-src/blog/src/release-070.md))

**Web**: `dx bundle_web` runs wasm-bindgen (version auto-detected and auto-installed), optional wasm-split, wasm-opt, esbuild for JS assets. ([build/web.rs](https://github.com/DioxusLabs/dioxus/blob/v0.7.9/packages/cli/src/build/web.rs))

### Hot reload — three tiers, with real limits

From [essentials/ui/hotreload.md](https://github.com/DioxusLabs/docsite/blob/main/docs-src/0.7/src/essentials/ui/hotreload.md):

**1. RSX hot-reload** (always on). Add/remove/modify elements, all string attributes, formatted strings, component-prop *literals* (numbers, bools, strings), and the bodies of `for`/`if` blocks — no recompile.

**2. Asset hot-reload**. CSS, images, SCSS (auto-recompiled) via `asset!()`. Tailwind is auto-downloaded and watched if `tailwind.css` exists in the project root.

**3. Rust hot-patching ("Subsecond")** — experimental, opt-in via `dx serve --hotpatch`:

> *"**New in Dioxus 0.7**, you can enable experimental Rust code hot-reloading using the `--hotpatch` flag. … The extra flag is required while hot-patching is still experimental."*

The `dx` source is blunter: *"This is quite experimental and may lead to unexpected segfaults or crashes in development."* ([cli/serve.rs](https://github.com/DioxusLabs/dioxus/blob/v0.7.9/packages/cli/src/cli/serve.rs))

**Documented Subsecond limitations** ([hotreload.md](https://github.com/DioxusLabs/docsite/blob/main/docs-src/0.7/src/essentials/ui/hotreload.md)):
- *"You may add new globals at runtime, but their destructors will never be called."*
- *"Globals are tracked across patches, but … renames are observed as introducing a new global."*
- *"Changes to static initializers will not be observed."*
- **The big one:** *"Rust hot-patching currently only tracks the 'tip' crate in your project. If you edit code in any of your dependencies — which might be *your* crate in a workspace — DX does *not* register that change. While RSX hot-reloading works across a workspace, Subsecond currently does not."*

That last point is a direct constraint on project layout: **a workspace that splits `core`/`domain` from `ui` loses Rust hot-patching for everything outside the UI crate.** For a flashcard app where scheduling logic lives in a separate crate, this matters.

**Always requires a full rebuild or hot-patch**: new variables/expressions, logic changes outside RSX, component signature changes, import/module changes, complex Rust expressions in attributes.

In **0.8.0-alpha.0, hot-patching became default-on** (PR #5506, *"enable hotpatch by default, hotreload cargo.toml, dynamic file watcher"*). ([v0.8.0-alpha.0 release notes](https://github.com/DioxusLabs/dioxus/releases/tag/v0.8.0-alpha.0))

Other dev-loop niceties: press `r` in the `dx` TUI for a manual full rebuild; press `d` to attach an LLDB debugger (VSCode-family editors only — *"we'd happily accept contributions to expand our support to Neovim, Zed, etc."*); `dx` prints per-stage timings (e.g. `asset optimization: 2s, linking: 1s, wasm-bindgen: 4s`); desktop and mobile show rebuild toasts. ([0.7 release post](https://github.com/DioxusLabs/docsite/blob/main/docs-src/blog/src/release-070.md))

### Known pain points

- Subsecond does not cross workspace-crate boundaries (above).
- `--hotpatch` may segfault (source comment above).
- Android: [#5628](https://github.com/DioxusLabs/dioxus/issues/5628) CSS assets fail to load during `dx serve` on Android; [#3867](https://github.com/DioxusLabs/dioxus/issues/3867) a conditional target dependency in Cargo.toml causes `dx serve` Android deployment to hang indefinitely.
- 0.8 alpha regression: *"due to changes upstream, the cli must be installed via `--locked` if you wish to build from source."* ([v0.8.0-alpha.0 notes](https://github.com/DioxusLabs/dioxus/releases/tag/v0.8.0-alpha.0))
- `cargo install dioxus-cli` from source *"can take up to 10 minutes"*; prebuilt binaries via `curl -sSL https://dioxus.dev/install.sh | bash` or `cargo binstall dioxus-cli --force` are "strongly" recommended. ([getting_started/index.md](https://github.com/DioxusLabs/docsite/blob/main/docs-src/0.7/src/getting_started/index.md))
- `dx --version` reported a stale git SHA, blocking self-update — fixed only in 0.7.9. ([v0.7.9 release notes](https://github.com/DioxusLabs/dioxus/releases/tag/v0.7.9))

**Unverified: actual iteration wall-clock times.** The marketing claims are *"Edit your markup, styles, and see changes in milliseconds"* ([README](https://github.com/DioxusLabs/dioxus/blob/main/README.md)) and *"running in an emulator or on device in seconds"* — but I found **no primary source with measured cold-build, incremental-rebuild, or hotpatch timings** for any platform, and none for Android in particular (where every non-hot-patched change also pays a Gradle `assembleDebug` + `adb install` cycle). Budget an empirical spike.

---

## 7. Server story

**Fullstack is optional and opt-in via a cargo feature.** In `packages/dioxus/Cargo.toml` @ v0.7.9, `fullstack` and `server` are ordinary optional features, disjoint from the client renderers and from `router`:

```toml
default = ["launch", "devtools", "logger", "lib"]
router  = ["dep:dioxus-router"]
fullstack = ["dep:dioxus-fullstack", "dioxus-config-macro/fullstack", "dep:serde",
             "dioxus-web?/document", "dioxus-web?/hydrate", "dioxus-server?/document",
             "dioxus-web?/devtools", "dioxus-web?/mounted"]
server  = ["dep:dioxus-server", "dioxus-fullstack?/server", "dep:dioxus-fullstack-macro",
           "ssr", "dioxus-liveview?/axum"]
desktop = ["dep:dioxus-desktop", "dioxus-config-macro/desktop"]
mobile  = ["dep:dioxus-desktop", "dioxus-config-macro/mobile"]
web     = ["dep:dioxus-web", "dioxus-fullstack?/web", ...]
```
([dioxus/Cargo.toml @ v0.7.9](https://github.com/DioxusLabs/dioxus/blob/v0.7.9/packages/dioxus/Cargo.toml))

Every `dioxus-fullstack?/...` and `dioxus-web?/hydrate` is an **optional-dependency-conditional** (`?`) feature — they activate only if fullstack is already enabled. Nothing in `desktop`, `mobile`, `web`, or `router` pulls in fullstack.

The build tool agrees, and detects fullstack rather than assuming it:

> *"Under the hood, DX automatically detects if the target app has a server variant by checking its `Cargo.toml` for a Cargo feature called 'server'. … If your `dioxus` dependency enables the `fullstack` feature, DX recognizes this app is a fullstack app and then creates two builds."*
> — [fullstack/project_setup.md](https://github.com/DioxusLabs/docsite/blob/main/docs-src/0.7/src/essentials/fullstack/project_setup.md)

No `fullstack` feature ⇒ one build, one binary, no server.

### Is anything entangled?

- **Routing** — no. `dioxus-router` is its own feature and its own crate, independent of `fullstack`/`server`. ([dioxus/Cargo.toml](https://github.com/DioxusLabs/dioxus/blob/v0.7.9/packages/dioxus/Cargo.toml)) The router's SSG/static-generation and "typed routing" extras are the fullstack-flavoured parts; plain client-side routing is not.
- **State** — no. Signals, hooks, stores, and context live in `dioxus-signals` / `dioxus-hooks` / `dioxus-core`, all in the `lib`/default feature set.
- **Assets** — no. The `asset!()` macro comes from the `asset` feature (`dep:manganis`, `dep:dioxus-asset-resolver`), in the default `lib` set, independent of fullstack. `dx bundle` handles hashing/optimizing assets for every bundle format including `android` and `web`. ([dioxus/Cargo.toml](https://github.com/DioxusLabs/dioxus/blob/v0.7.9/packages/dioxus/Cargo.toml), [build/web.rs](https://github.com/DioxusLabs/dioxus/blob/v0.7.9/packages/cli/src/build/web.rs))
- **Hydration/SSR** — enabled only through `fullstack`; a non-fullstack web build is plain CSR (see §4).

**One real gotcha, from the maintainer's own answer.** The stock `dx new` fullstack templates (e.g. `hot_dog`) *do* produce a separate server binary that the desktop client requires at runtime. From [discussion #3898](https://github.com/DioxusLabs/dioxus/discussions/3898):

> *"In the hot_dog example, when you bundle the app for desktop, the server is a separate binary. Installing the .exe installs the client on your machine, but the client needs the server process to be running separately … and the client will panic if you try to hit the 'favorites' route without the server process running."*

So the mitigation is: **do not scaffold from a fullstack template.** Pick a client-only template, or strip `fullstack`/`server` from the generated Cargo.toml. If you later want an *optional* server for the web build only, the maintainer's suggested pattern is to conditionally compile the same function as a server function:

```rust
// If we are not using fullstack, define our own server function error type
#[not(any(feature = "web", feature = "server"))]
type ServerFnError = dioxus::CapturedError;
// If we are using fullstack, annotate this as a server function
#[cfg_attr(any(feature = "web", feature = "server"), server)]
async fn init_db() -> Result<Vec<Item>, ServerFnError> { ... }
```
([@ealmloff, discussion #3898](https://github.com/DioxusLabs/dioxus/discussions/3898))

Also note that Dioxus explicitly does **not** provide database/session/cache infrastructure even in fullstack mode: *"Currently, Dioxus Fullstack does not provide built-in utilities for things like Databases, Caches, Sessions, and Mailers … You'll need to pull in 3rd-party crates like `Sqlx` and `tower-sessions`."* ([fullstack/index.md](https://github.com/DioxusLabs/docsite/blob/main/docs-src/0.7/src/essentials/fullstack/index.md)) So a no-server project loses nothing it would otherwise have gotten for free.

**Verdict: removing fullstack leaves the stack fully coherent.** Router, signals/state, assets, hot-reload, and all three bundle targets work without it.

---

## Summary of `**Unverified:**` items

1. Timeline for 0.8.0 stable, or any further 0.7.x patch — no public milestone with dates.
2. The "70 kb, comparable to React" web bundle figure — no compression basis, no version, no methodology; likely stale.
3. Measured build/iteration times on any platform, especially Android (Gradle + `adb install` on every non-hot-patched change).
4. Whether `rusqlite` with `bundled` actually compiles and runs on Android under `dx` — the env vars `dx` sets are demonstrably correct for `cc`-rs cross-compilation, and no issues report the contrary, but I did not build it.
5. Whether Dioxus exposes an `android_activity::AndroidApp` handle anywhere (the JNI workarounds exist precisely because the discussion question went unanswered).
6. `sqlx` on Android via `dx`, and `sqlx` SQLite on `wasm32-unknown-unknown` — no primary source either way; upstream [rusqlite #1735](https://github.com/rusqlite/rusqlite/issues/1735) suggests sqlx's bundled-sqlite cross-compilation is itself shaky.
7. Whether the `armv7`/32-bit Android drop ([#5637](https://github.com/DioxusLabs/dioxus/issues/5637)) is intentional policy or a bug — no maintainer response on the issue. (Not blocking for this project: 64-bit-only is fine for phones.)
8. Whether the tao API<30 fix (tao 0.35.2) will be backported to the Dioxus 0.7.x line, or whether Android 9/10 support requires waiting for 0.8 stable.
