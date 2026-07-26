# Leptos + Tauri 2 as a Rust multi-platform client stack

> **Editor's note — §5.6 and §5.8 are superseded.** This report states that `sqlite-wasm-rs` is
> *"not compatible with rusqlite or diesel"* and concludes there is no shared query code between
> native and web. That was re-checked directly against primary sources on 2026-07-26 and is **wrong**:
> `rusqlite`'s `Cargo.toml` declares `default = ["cache", "ffi-sqlite-wasm-rs"]` and depends on
> `sqlite-wasm-rs` under `cfg(all(target_family = "wasm", target_os = "unknown"))`, and the current
> `sqlite-wasm-rs` README makes no such incompatibility claim — it lists rusqlite and diesel as
> "Related Project". SQL, schema, migrations and row-mapping code **are** portable across all three
> targets. See §0 and §3.4 of [`../client-stacks.md`](../client-stacks.md). The rest of this report
> stands.

Research date: **2026-07-26**. Every claim links to a primary source (crates.io API / crates.io sparse index, docs.rs, the projects' own GitHub source & issue trackers, or the official docs site). Secondary sources are not used as evidence.

Consuming context: local-first, offline-by-default, **no server of our own**, spaced-repetition flashcard app, targeting **desktop + web + Android** (no iOS).

---

## 1. Versions & health

### 1.1 Leptos

| Fact | Value | Source |
|---|---|---|
| Latest stable | **0.8.20**, published **2026-06-25** | [crates.io API](https://crates.io/api/v1/crates/leptos) |
| Latest overall | **0.9.0-beta**, published **2026-07-18** | [crates.io API](https://crates.io/api/v1/crates/leptos) |
| Repo | github.com/leptos-rs/leptos, MIT, not archived | [GitHub API](https://api.github.com/repos/leptos-rs/leptos) |
| Stars / open issues / contributors | 21,121 stars, 127 open issues, 372 contributors | [GitHub API](https://api.github.com/repos/leptos-rs/leptos), [contributors pagination](https://api.github.com/repos/leptos-rs/leptos/contributors?per_page=1) |
| Commits, last 52 weeks | **334** | [GitHub participation stats](https://api.github.com/repos/leptos-rs/leptos/stats/participation) |
| MSRV | **1.88**, edition 2021 | [crates.io sparse index for leptos 0.8.20](https://index.crates.io/le/pt/leptos); [workspace Cargo.toml](https://github.com/leptos-rs/leptos/blob/main/Cargo.toml) |
| Downloads | 3,963,639 all-time / 1,148,666 recent | [crates.io API](https://crates.io/api/v1/crates/leptos) |

**Release cadence, last ~12 months** (all from [crates.io API](https://crates.io/api/v1/crates/leptos)):
`0.8.3` 2025-07-13 · `0.8.4` 2025-07-20 · `0.8.5` 2025-07-21 · `0.8.6` 2025-07-27 · `0.8.7` 2025-08-26 (**yanked**) · `0.8.8` 2025-08-26 · `0.8.9` 2025-09-18 · `0.8.10` 2025-09-29 · `0.8.11` 2025-10-24 · `0.8.12` 2025-10-27 · `0.8.13` 2025-11-22 · `0.8.14` 2025-11-25 · `0.8.15` 2025-12-19 · `0.8.16` 2026-02-16 · `0.8.17` 2026-03-01 · `0.8.19` 2026-04-16 · `0.9.0-alpha` 2026-05-19 · `0.8.20` 2026-06-25 · `0.9.0-beta` 2026-07-18.

Note the gap: **no stable release between 2025-12-19 and 2026-02-16** (~2 months), and patch releases have thinned out through 2026.

#### 1.1.1 THE decision-relevant fact: Leptos is now "lightly maintained"

On **2026-05-08** the creator and principal maintainer (`gbj`) opened [issue #4707 "Status Update - May 2026"](https://github.com/leptos-rs/leptos/issues/4707), still open and last updated 2026-07-08. Verbatim:

> **tl:dr;** Leptos is not abandoned but will be lightly maintained going forward. I consider it feature-complete and do not expect to do significant new development in the future. I am open to additional maintainers who want to take a more active role.

and:

> I consider Leptos complete. I have shipped every major feature on any of my roadmaps.

and on 0.9:

> There has been some ongoing, slow work on a `leptos_0.9` branch, which includes some cleanup and fixes that require a semver-breaking release. At this point, I'd love some help reviewing any additional PRs that should be merged into that branch, and fixing any remaining issues in that milestone, while building toward a release. **I don't feel much urgency about it, and it doesn't contain any significant new features.**

and:

> I'm likely going to engage a little less with issues/discussions/PRs than I have historically

[Source: leptos-rs/leptos#4707](https://github.com/leptos-rs/leptos/issues/4707)

In the comment thread several people volunteered to help maintain (`EvanCarroll` 2026-05-16, `LeoniePhiline` 2026-05-18, `leechristophermurray` 2026-06-18, `aekasitt` 2026-06-21), and one commenter argued for cutting a 1.0 (`jberkenbilt`, 2026-07-08). **Unverified:** whether any of these volunteers were actually granted commit rights — the issue thread contains no maintainer reply confirming a handover, and I found no announcement of new maintainers. ([comment thread](https://github.com/leptos-rs/leptos/issues/4707))

Interpretation for a consuming project: the framework is *stable and finished*, not *actively developed*. That cuts both ways — low churn risk going forward, but also low likelihood that a bug you hit gets fixed promptly.

#### 1.1.2 Pre-1.0 status and API stability policy

Leptos is **pre-1.0** (0.8.x). Cargo's semver rules for 0.x mean **the minor version is the breaking-change axis**: `0.8 → 0.9` is a breaking release by construction. There is **no published formal API-stability policy document** for Leptos.

**Unverified:** I could not find any page on leptos.dev, book.leptos.dev, or in the repo stating a stability guarantee or a 1.0 timeline. The only forward-looking statement is #4707's "cleanup and fixes that require a semver-breaking release" for 0.9 ([#4707](https://github.com/leptos-rs/leptos/issues/4707)).

#### 1.1.3 Major-version churn: what actually breaks

**0.6 → 0.7** was a near-total rewrite of the reactive system and renderer. Per the [v0.7.0 release notes](https://github.com/leptos-rs/leptos/releases/tag/v0.7.0), breaking changes included:
- Module reorganisation — you must switch to `use leptos::prelude::*`.
- API renames — `create_signal()` → `signal()`.
- **Views became statically typed** — branching now requires `Either` enums or `.into_any()`. This is the one that mechanically breaks most view code.
- **Thread-safety by default** — non-`Send` values now need `signal_local()`.
- **Router route definitions changed shape** — string paths replaced by `StaticSegment`/`ParamSegment` or the `path!()` macro.
- SSR boilerplate changes (`get_configuration` became synchronous).
New in 0.7: `Resource` supports `.await` inside `<Suspense/>`, `ArcRwSignal` ref-counted signals, `.read()`/`.write()` guards, `bind:` two-way binding, `#[derive(Store)]` reactive stores.
([leptos v0.7.0 release notes](https://github.com/leptos-rs/leptos/releases/tag/v0.7.0))

**Major-release timeline** ([crates.io versions API](https://crates.io/api/v1/crates/leptos/versions)): `0.5.0` 2023-09-29 → `0.7.0` 2024-11-30 → `0.8.0` 2025-05-01 → `0.9.0-beta` 2026-07-18. So historically a **breaking minor roughly every 6–14 months**; 0.8 has now been the stable line for ~15 months.

**0.7 → 0.8** ([v0.8.0 release notes](https://github.com/leptos-rs/leptos/releases/tag/v0.8.0), released **2025-05-01**) was much smaller and mostly server-side:
- Axum 0.8 support — "This alone required a major version bump, as we reexport some Axum types."
- `LocalResource` dropped `SendWrapper` from its public interface, so `.as_deref()` calls must be removed. **This is the one breaking change that touches a CSR-only app.**
- Server-function custom errors now need `FromServerFnError` instead of `ServerFnError`.
- `LeptosOptions` and `ConfFile` no longer implement `Default`.
- `PossibleRouteMatch` made dyn-safe.
The notes describe these as "technically semver-breaking but should not meaningfully affect user code".

**Unverified:** the release page rendered its date ambiguously; the 2025-05-01 date above comes from the [crates.io versions API](https://crates.io/api/v1/crates/leptos/versions), which is authoritative.

**0.8 → 0.9** (currently `0.9.0-beta`, 2026-07-18). Per the [0.9.0-beta](https://github.com/leptos-rs/leptos/releases) and [0.9.0-alpha](https://github.com/leptos-rs/leptos/releases) notes:
- Signals **no longer implement the `Fn()` traits directly on nightly**; they deref to a `dyn Fn()` instead. Component props that took `impl Fn() -> T` should move to `impl SignalOrFn<Output = T>`; call sites may need `foo=*signal` instead of `foo=signal`.
- Lazy-loading now sits behind a new `lazy` Cargo feature (binary-size win for projects not using it).
- The maintainer describes the migration as intentionally minimal.

**Within 0.8.x**, one behaviour change worth knowing: as of **0.8.19**, server-function encodings now respect Axum/Actix request body size limits rather than ignoring them, so >2 MB POSTs may need the framework limit raised ([0.8.19 release notes](https://github.com/leptos-rs/leptos/releases)). Irrelevant to a no-server project.

### 1.2 Tauri

| Fact | Value | Source |
|---|---|---|
| Latest stable `tauri` | **2.11.5**, published **2026-07-01** | [crates.io API](https://crates.io/api/v1/crates/tauri) |
| Latest stable `tauri-cli` | **2.11.4**, published **2026-06-28** | [crates.io API](https://crates.io/api/v1/crates/tauri-cli) |
| Repo | github.com/tauri-apps/tauri, `Apache-2.0 OR MIT` | [workspace Cargo.toml](https://github.com/tauri-apps/tauri/blob/dev/Cargo.toml) |
| Commits, last 52 weeks | **496** | [GitHub participation stats](https://api.github.com/repos/tauri-apps/tauri/stats/participation) |
| Published MSRV (2.11.5) | **1.77.2**, edition 2021 | [crates.io sparse index](https://index.crates.io/ta/ur/tauri) |
| MSRV on `dev` branch (upcoming) | **1.90** | [workspace Cargo.toml](https://github.com/tauri-apps/tauri/blob/dev/Cargo.toml) |
| Downloads | 23,205,450 all-time / 8,595,514 recent | [crates.io API](https://crates.io/api/v1/crates/tauri) |

**Release cadence, last ~12 months** (from [crates.io API](https://crates.io/api/v1/crates/tauri)):
`2.6.1` 2025-06-26 · `2.6.2` 2025-06-27 · `2.7.0` 2025-07-20 · `2.8.0`–`2.8.2` 2025-08-18/19 · `2.8.3` 2025-08-24 · `2.8.4` 2025-08-25 · `2.8.5` 2025-09-01 · `2.9.0` 2025-10-20 · `2.9.1` 2025-10-22 · `2.9.2` 2025-10-29 · `2.9.3` 2025-11-13 · `2.9.4` 2025-11-30 · `2.9.5` 2025-12-09 · `2.10.0`/`2.10.1` 2026-02-02 · `2.10.2` 2026-02-04 · `2.10.3` 2026-03-04 · `2.11.0` 2026-04-30 · `2.11.1` 2026-05-06 · `2.11.2` 2026-05-16 · `2.11.3` 2026-06-17 · `2.11.4` 2026-06-30 · `2.11.5` 2026-07-01.

That is a **steady ~monthly minor/patch cadence with an active multi-person team** — visibly healthier than Leptos on this axis. The [tauri CHANGELOG](https://github.com/tauri-apps/tauri/blob/dev/crates/tauri/CHANGELOG.md) shows routine bugfix throughput (deadlock fixes, perf work, dependency pins) through July 2026.

#### 1.2.1 Post-1.0 status and stability policy

Tauri is **post-1.0** (2.x). The [Tauri 2.0 stable release announcement](https://v2.tauri.app/blog/tauri-20/) commits to fixing rather than redesigning in the 2.x line:

> We mainly want to focus on improving this major version with a better developer experience, better documentation and less impactful bugs.

The important carve-out is **plugins**, which are versioned separately and are explicitly *not* covered by the same guarantee:

> Each plugin's stableness is defined per plugin and documented (soon) in the plugin documentation. **The plugin API can possibly break in minor versions**, but we will try to keep these changes to a minimum, especially for plugins considered stable.

([Tauri 2.0 Stable Release](https://v2.tauri.app/blog/tauri-20/))

**Unverified:** there is no separate formal semver/stability policy document in the repo; the statements above from the release blog are the strongest commitment I could find. There is an open issue noting that CI does not machine-check for semver breakage: [tauri-apps/tauri#12465](https://github.com/tauri-apps/tauri/issues/12465).

**MSRV discrepancy to be aware of:** published `tauri` 2.11.5 declares `rust-version = 1.77.2` ([sparse index](https://index.crates.io/ta/ur/tauri)), but the `dev` branch workspace now declares `rust-version = "1.90"` ([Cargo.toml](https://github.com/tauri-apps/tauri/blob/dev/Cargo.toml)), i.e. a large MSRV bump is queued for the next release. Separately, `tauri-build 1.5.7-edition2024.0` raised MSRV to **1.85** for edition-2024 projects ([tauri releases](https://github.com/tauri-apps/tauri/releases)). Plan on a **modern Rust toolchain**, not the declared 1.77.2 floor.

### 1.3 Combined health verdict (facts only)

- Tauri: ~500 commits/yr, monthly releases, multi-maintainer, post-1.0 with a stated no-breaking-changes-in-2.x intent.
- Leptos: ~334 commits/yr and falling, pre-1.0, explicitly declared **feature-complete and lightly maintained** by its sole principal maintainer, with a breaking 0.9 in beta and no urgency behind it.
- The two projects are **independent**; nothing in Tauri depends on Leptos or vice versa. There is an official Tauri template for Leptos (see §3).

---

## 2. Android via Tauri 2

### 2.0 Toolchain constants baked into the CLI

These are the numbers that actually govern your build, read from the CLI source at tag `tauri-cli-v2.11.4` ([crates/tauri-cli/src/mobile/android/mod.rs](https://raw.githubusercontent.com/tauri-apps/tauri/tauri-cli-v2.11.4/crates/tauri-cli/src/mobile/android/mod.rs), lines 51–52):

```rust
const NDK_VERSION: &str = "29.0.13846066";
const SDK_VERSION: u8 = 36;
```

Gradle template versions in the same tag ([templates/mobile/android/](https://github.com/tauri-apps/tauri/tree/dev/crates/tauri-cli/templates/mobile/android)):
- Gradle wrapper **8.14.3** (`gradle/wrapper/gradle-wrapper.properties`)
- Android Gradle Plugin **8.11.0**, Kotlin plugin **1.9.25** (`build.gradle.kts`)
- `compileSdk = 36`, `targetSdk = 36`, `jvmTarget = JVM_1_8` (`app/build.gradle.kts`)

Also note: `tauri-cli` is **2.11.4** while the `tauri` runtime crate is **2.11.5** — the CLI trails by one patch and `tauri android build` carries an `--ignore-version-mismatches` flag precisely because it version-checks ([CLI reference](https://v2.tauri.app/reference/cli/)).

### 2.1 Is Android first-class?

**No — not by Tauri's own wording.** The [Tauri 2.0 Stable Release](https://v2.tauri.app/blog/tauri-20/) says only:

> A very much awaited part of this release is the mobile operating system support.

> We are not completely happy about the developer experience at the moment but are actively improving to bring it up to par with the desktop experience.

> On mobile **not all of the official plugins are supported**. Some are by design not a good fit for mobile and some are just not implemented to support mobile yet.

The words "first-class", "stable", or "production-ready" do not appear in reference to mobile. **There is no global OS-level platform-support matrix page on v2.tauri.app** — the only global table is the per-plugin one at [v2.tauri.app/plugin/](https://v2.tauri.app/plugin/#support-table).

**Official plugin support on Android** (parsed from the [plugin support table](https://v2.tauri.app/plugin/#support-table)):

- **No Android support at all:** `autostart`, `cli`, `global-shortcut`, `localhost`, `positioner`, `process`, `single-instance`, **`updater`**, `window-state`, `system-tray`, `window-customization`.
- **Partial on Android:** `clipboard-manager` ("Only plain-text content support"), `deep-link` ("Deep links must be registered in config. Dynamic registration [not supported]"), `dialog` ("Does not support folder picker"), **`fs` ("Access is restricted to Application folder by default")**, `opener`/`shell` ("Only allows to open URLs via `open`").
- **Full on Android:** `barcode-scanner`, `biometric`, `geolocation`, `haptics`, `http`, `log`, `nfc`, `notification`, `os`, `persisted-scope`, **`sql`**, **`store`**, `stronghold`, `upload`, `websocket`.

For an offline flashcard app the relevant absences are **no `updater`** (you ship through the Play Store instead), **no `single-instance`**, and **no `window-state`**. `sql` and `store` — the two you'd actually want — are both full.

So: Android is a **shipped, supported target of the stable 2.x line**, but the project's own words put its DX below desktop, and a visible chunk of the official plugin surface is desktop-only.

### 2.2 Exact prerequisites (quoted from the official page)

From [v2.tauri.app/start/prerequisites/](https://v2.tauri.app/start/prerequisites/) ([raw source](https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/start/prerequisites.mdx)):

1. **Android Studio** — "Download and install [Android Studio from the Android Developers website](https://developer.android.com/studio)".

2. **`JAVA_HOME`** must point at Android Studio's bundled JBR (JetBrains Runtime). The docs give no standalone JDK version number; they tell you to use the one shipped with Android Studio:
   - Linux: `export JAVA_HOME=/opt/android-studio/jbr`
   - macOS: `export JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home"`
   - Windows: `[System.Environment]::SetEnvironmentVariable("JAVA_HOME", "C:\Program Files\Android\Android Studio\jbr", "User")`

3. **SDK components**, installed via Android Studio's SDK Manager — verbatim list:
   > Android SDK Platform, Android SDK Platform-Tools, NDK (Side by side), Android SDK Build-Tools, Android SDK Command-line Tools

4. **`ANDROID_HOME` and `NDK_HOME`**:
   - Linux: `export ANDROID_HOME="$HOME/Android/Sdk"` and `export NDK_HOME="$ANDROID_HOME/ndk/$(ls -1 $ANDROID_HOME/ndk)"`
   - macOS: `export ANDROID_HOME="$HOME/Library/Android/sdk"` and the same `NDK_HOME` line
   - Windows: PowerShell setting `ANDROID_HOME` to `"$env:LocalAppData\Android\Sdk"` plus a computed NDK version

   Note the `NDK_HOME` recipe **assumes exactly one NDK is installed** — `$(ls -1 $ANDROID_HOME/ndk)` returns multiple lines otherwise and silently produces a broken path.

5. **Rust target triples** — verbatim:
   ```
   rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
   ```

   (`aarch64` = modern devices, `armv7` = old 32-bit devices, `i686`/`x86_64` = emulators.)

**Unverified:** the prerequisites page states **no exact NDK version** and **no exact JDK version number** — it defers both to whatever Android Studio installs. That is a real reproducibility gap for an agent-driven build. However, see §2.3 for a concrete NDK floor stated elsewhere in the docs.

### 2.3 NDK version: how it's actually chosen, and 16 KB page alignment

**Which NDK the CLI uses** — from [crates/tauri-cli/src/mobile/android/mod.rs](https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-cli/src/mobile/android/mod.rs) (~lines 600–685):
1. If `NDK_HOME` is set → validated and used, else `"Android NDK invalid. Make sure the NDK_HOME environment variable has correct value."`
2. Otherwise it scans `$ANDROID_HOME/ndk` and picks the **lexicographically highest** installed NDK:
   ```rust
   installed_ndks.sort();
   if let Some(ndk) = installed_ndks.last() {
       log::info!("Using installed NDK: {}", ndk.display());
       std::env::set_var("NDK_HOME", ndk);
   }
   ```
   **Not the pinned `NDK_VERSION`.** With several side-by-side NDKs you silently get the newest.
3. If none found: non-interactive → `bail!("Android NDK not found...")`; interactive → offers to `sdkmanager --install ndk;29.0.13846066`.

**Known-good: `29.0.13846066`** (the pin). Recent fixes, all landed: [#15344](https://github.com/tauri-apps/tauri/pull/15344) "Fix NDK_HOME environment variable not honored when set" (shipped in **tauri-cli 2.11.3**), [#15360](https://github.com/tauri-apps/tauri/pull/15360) (normalize NDK_HOME slashes on Windows), [#14398](https://github.com/tauri-apps/tauri/pull/14398).
**Still open:** [#15345](https://github.com/tauri-apps/tauri/issues/15345) (Windows mixed path separators, `status: upstream`), [#11841](https://github.com/tauri-apps/tauri/issues/11841) (16 comments, macOS: `NDK_HOME isn't set` despite installation, open since 2024-12).

**Unverified/inconsistent:** there are **no** issues referencing NDK release letters (r27/r28/r29) — Tauri pins by full package version only.

**16 KB page alignment (Android 15+, mandatory for Play uploads from 2025-11-01): handled, provided the NDK is r28+.** Evidence from [#14895 "[feat] Support 16 KB page sizes"](https://github.com/tauri-apps/tauri/issues/14895), opened 2026-02-05, **closed 2026-02-28**:
- Maintainer *FabianLars* (2026-02-05): "tauri itself and our official plugins should support 16kib out of the box with a recent enough ndk."
- Reporter *chrox* (2026-02-26): "It's been confirmed that this is not related to Tauri… **after upgrading the NDK to 28.2.13676358, 16 KB page size support worked out of the box. No additional configuration is required on the Tauri side.**"

Corroborating documentation statement, from the [mobile plugin dev guide](https://v2.tauri.app/develop/plugins/develop-mobile/): building requires **NDK 28 or higher** for 16 KB page alignment. This is not repeated on the prerequisites page — a doc inconsistency.

**Residual risk (unrebutted in the thread):** the Tauri Gradle `rust` plugin overrides `.cargo/config.toml` rustflags, so the usual `-Wl,-z,max-page-size=16384` workaround has to go through `build.rs` if a Rust dependency ships an oddly-linked `.so`. Verify with `llvm-objdump -p <lib>.so | grep LOAD` (want `align 2**14`). **Unverified:** v2.tauri.app says nothing about 16 KB page sizes anywhere; it is handled implicitly by the NDK pin, not documented.

### 2.3b JDK / Gradle version compatibility — an active, unfixed footgun

**Not documented on v2.tauri.app.** The prerequisites page only says to point `JAVA_HOME` at Android Studio's JBR; no version ceiling is stated anywhere. The real requirement is implicit in the template (Gradle 8.14.3 + AGP 8.11.0 + Kotlin 1.9.25, `JvmTarget.JVM_1_8`).

PR [#15780 "feat(cli): warn when Java is too new for the bundled Gradle"](https://github.com/tauri-apps/tauri/pull/15780) was opened **2026-07-26 — today — and is still open/unmerged**. From its body:

> Installing a very recent JDK (Java 25/26) while Tauri ships Gradle 8.14 makes Android builds fail with a cryptic `Failed to assemble APK with Io...` error that gives no hint about the real cause.

> This adds an upfront check … `log::warn!` when it's newer than the bundled Gradle can run on… **It's a warning only — nothing is blocked.**

**Action:** use the JDK bundled with Android Studio (currently JBR ≈ JDK 21) and set `JAVA_HOME` to it. Do not let a system JDK 25/26 win. Reference: [Gradle compatibility matrix](https://docs.gradle.org/current/userguide/compatibility.html). Related open issue: [#15385](https://github.com/tauri-apps/tauri/issues/15385); still-open Gradle download timeout: [#13148](https://github.com/tauri-apps/tauri/issues/13148).

### 2.4 Minimum Android API level / WebView floor

- **Config default:** `bundle > android > minSdkVersion` defaults to **24** (Android 7.0). Type `uint32`. Description: "The minimum API level required for the application to run." ([Tauri config reference](https://v2.tauri.app/reference/config/))
- **Maintainer's practical answer**, from Tauri maintainer *FabianLars* in [discussion #11843](https://github.com/tauri-apps/tauri/discussions/11843):
  > In theory 7 and above, but i only saw 9 and above being tested by someone i think.
- Same discussion: Android 10 devices shipping **Chrome/WebView 77** lack the features Vite's dev runtime needs, so remote dev/testing does not work there. Maintainer also notes older Android **emulators bundle outdated WebView versions** that do not match real-world devices, making them unreliable for testing. ([#11843](https://github.com/tauri-apps/tauri/discussions/11843))
**The empirical floor is much higher than SDK 24.** [#8788 "New app fails on Android version < 8"](https://github.com/tauri-apps/tauri/issues/8788) is open with 33 comments (2024-02-05 → last activity 2026-07-17):
- *mertushka* (2025-08-12): "[docs say] The minimum supported Android version for Tauri apps is Android 7.0 (SDK 24). Might want to update the docs as well because I thought it would work on Android 7.1.1 (SDK 25) but ended up here."
- *abpdf* (2026-07-17): "the application can start and render pages normally on Android 7.1.1 (WebView 55). But `window.__TAURI__` is still unavailable because **the script injected automatically by Tauri contains … ES6+ syntax, causing SyntaxError to be thrown on Chrome 55**, blocking all its front-end and back-end [IPC]."
- Same user's resolution: "**Manually updated Android System WebView to 117.0.5938.60 — this was the critical step to make `window.__TAURI__` appear and IPC work, since the default WebView 55 lacks ES6 syntax support.**"

**Practical floor: a WebView with full ES6+ support.** Because Android System WebView updates via Play Store independently of the OS, any modern Android 8+ device with Play Services is fine; the risk is old / Play-less / China-market devices and bare AOSP emulator images. **Unverified:** Tauri publishes no explicit minimum Chromium/WebView version number anywhere.

**Related hard-crash hazard:** [wry#1785](https://github.com/tauri-apps/wry/issues/1785) (open, filed 2026-07-24) — if no WebView provider is available, `MissingWebViewPackageException` is silently discarded, leaving an uncleared JNI exception and causing a **hard process abort** rather than a recoverable `Err`:
> `WebViewBuilder::build()` returns `Ok` before the WebView is actually created on Android… there is currently no way for an app embedding wry on Android to detect or recover from a missing/disabled/updating WebView provider — the app simply crashes on launch, with no opportunity to show recovery UI.

This bites on emulator images without Play Services, and on real devices while the WebView is mid-update. **Use a Google-APIs / Play-enabled emulator system image.**

Other relevant `bundle > android` config keys ([config reference](https://v2.tauri.app/reference/config/)):
- `versionCode` — `uint32`, 1..2,100,000,000 (Google Play limit); default `major*1000000 + minor*1000 + patch`.
- `autoIncrementVersionCode` — boolean, reads/writes `tauri.properties`.

### 2.5 Android native-command threading constraint

> On Android native commands are scheduled on the main thread. Performing long-running operations will cause the UI to freeze and potentially "Application Not Responding" (ANR) error.

([v2.tauri.app/develop/plugins/develop-mobile/](https://v2.tauri.app/develop/plugins/develop-mobile/)). This applies to Kotlin-side *plugin* commands; ordinary `#[tauri::command] async fn` still runs on the Tauri async runtime (§3.3).

### 2.5b The Android CLI surface (exact commands)

From the [CLI reference](https://v2.tauri.app/reference/cli/):

- **`tauri android init`** — "Initialize Android target in the project". Flags: `--ci` (skip prompts), `-v/--verbose`, `--skip-targets-install` (don't run rustup), `-c/--config`.

  **What it generates** is *not documented* ([CLI reference](https://v2.tauri.app/reference/cli/) gives one line; `gen/android` appears only incidentally in the signing/icons/env-var pages). Verified from the [CLI template tree](https://github.com/tauri-apps/tauri/tree/dev/crates/tauri-cli/templates/mobile/android), it renders into `src-tauri/gen/android/`:
  ```
  .editorconfig, .gitignore
  settings.gradle, build.gradle.kts, gradle.properties
  gradlew, gradlew.bat, gradle/wrapper/{gradle-wrapper.jar, gradle-wrapper.properties}
  buildSrc/build.gradle.kts
  buildSrc/src/main/kotlin/{BuildTask.kt, RustPlugin.kt}   <- the custom `rust` Gradle plugin
  app/.gitignore
  app/build.gradle.kts
  app/proguard-rules.pro
  app/src/main/AndroidManifest.xml
  app/src/main/MainActivity.kt
  app/src/main/res/{drawable,drawable-v24,layout,mipmap-*,values,values-night,xml}/...
  ```
  This is a **real, standalone Android Studio Gradle project**; Rust compilation is driven *from Gradle* by `buildSrc/.../RustPlugin.kt` + `BuildTask.kt`. The [Google Play guide](https://v2.tauri.app/distribute/google-play/) confirms: "Tauri uses an Android Studio project under the hood, so any official practice for building and publishing Android apps also apply to your app." The path is exposed as `TAURI_ANDROID_PROJECT_PATH` ([env var reference](https://v2.tauri.app/reference/environment-variables/)).

  **What is re-synced on every `dev`/`build` without a re-init** (from `mobile/android/dev.rs`): `tauri.properties` (`generate_tauri_properties`) and the debug `applicationIdSuffix` (`sync_debug_application_id_suffix`). Everything else — `minSdkVersion`, identifier, package name, your signing block — is **baked at init time**. Known divergence: [#14813](https://github.com/tauri-apps/tauri/issues/14813) reports `android init` producing different files on Windows vs Ubuntu.
- **`tauri android dev`** — runs on device/emulator with hot-reloading. Flags: `--release`, `-f/--features`, `--device [DEVICE]` ("Runs on the given device name"), `--open` ("Open Android Studio instead of trying to run on a connected device"), `--host [<HOST>]` ("Use the public network address for the development server"), `--no-dev-server-wait`, `--port <PORT>`.
- **`tauri android build`** — release build producing APKs/AABs. Flags: `-d/--debug`, `-t/--target [<TARGETS>...]` ("Which targets to build" — `aarch64`, `armv7`, `i686`, `x86_64`), `--apk`, `--aab`, `--split-per-abi` ("Whether to split the APKs and AABs per ABIs"), `-o/--open`.

**Google Play distribution** ([docs](https://v2.tauri.app/distribute/google-play/)): build with `tauri android build --aab`; the Play Store wants AAB. Confirms the minimum: "The minimum supported Android version for Tauri apps is Android 7.0 (codename Nougat, SDK 24)". `versionCode` comes from `tauri.conf.json > bundle > android > versionCode`. `--split-per-abi` produces per-ABI artifacts.

**Release signing** ([docs](https://v2.tauri.app/distribute/sign/android/)) — this IS documented and concrete:
```sh
keytool -genkey -v -keystore ~/upload-keystore.jks -keyalg RSA -keysize 2048 -validity 10000 -alias upload
```
then create `src-tauri/gen/android/keystore.properties`:
```
password=<your-password>
keyAlias=upload
storeFile=<path-to-keystore.jks>
```
and hand-edit `src-tauri/gen/android/app/build.gradle.kts` — add `import java.io.FileInputStream`, add a `signingConfigs` block before `buildTypes`, and set `signingConfig = signingConfigs.getByName("release")` on the release build type.

**Sharp edge 1 — signing config lives in generated code.** The generated `.gitignore` ([templates/mobile/android/.gitignore](https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-cli/templates/mobile/android/.gitignore)) excludes `key.properties`, `keystore.properties`, `/local.properties`, `build`, `/.tauri`, `/tauri.settings.gradle` — but **not** `app/build.gradle.kts`. So you must **commit `src-tauri/gen/android/`** to preserve your signing block, and then manage CLI regeneration of that directory in git. There is **no `tauri.conf.json` key for Android signing**.

**Sharp edge 2 — the default release build crashes on launch.** The generated `app/build.gradle.kts` sets `isMinifyEnabled = true` in the `release` buildType, and multiple users report the signed release APK dying immediately while debug builds work. From [#15337](https://github.com/tauri-apps/tauri/issues/15337) (17 comments, open): the reporter's own fix was *"I just changed the `isMinifyEnabled = true` to `isMinifyEnabled = false` in the `build.gradle.kts` file. and now the app is working"*, and maintainer *FabianLars* replied *"Linking #13379 since both are caused by that and should be fixed together."* Both [#13379](https://github.com/tauri-apps/tauri/issues/13379) and [#15337](https://github.com/tauri-apps/tauri/issues/15337) remain **open**. **The default template does not ship a working release build.**

**Sharp edge 3 — versionCode regression.** [#14413](https://github.com/tauri-apps/tauri/issues/14413), labelled **`priority: 1 high`**, still open: `versionName`/`versionCode` silently fall back to `1.0 (1)` instead of the `tauri.conf.json` values — a regression since 2.9.2, suspected to come from [#14194](https://github.com/tauri-apps/tauri/pull/14194) moving the logic from `tauri-build` to `tauri-cli`. Play rejects duplicate version codes, so this bites on your **second** upload, not your first. Verify the produced AAB before every upload.

**No automated release path:** from the [Google Play guide](https://v2.tauri.app/distribute/google-play/) — "The first upload must be made manually in the website so it can verify your app signature and bundle identifier. **Tauri currently does not offer a way to automate the process of creating Android releases**, which must leverage the Google Play Developer API, but it is a work in progress."

### 2.5c Reaching the dev server from a device: `adb reverse` + conditional LAN-IP rewrite

**This is not in the docs** — the entire "Development Server" section of [v2.tauri.app/develop/](https://v2.tauri.app/develop/#developing-your-mobile-application) is written for iOS. Verified from CLI source instead.

**Mechanism 1 — `adb reverse` (always runs).** From [crates/tauri-cli/src/mobile/android/android_studio_script.rs](https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-cli/src/mobile/android/android_studio_script.rs) (~lines 190–300): the CLI logs `"Forwarding port {port} with adb"`, runs `adb reverse tcp:<port> tcp:<port>`, re-checks with `adb reverse --list`, and loops `"waiting for the port to be forwarded to {}..."` until it appears. If no device is listed it warns `"ADB device list is empty, waiting a few seconds to see if there's any booting device..."` and retries for 5 s. With more than one device it errors:
> Multiple Android devices are connected ({}), please disconnect devices you do not intend to use so Tauri can determine which to use

This is why **an emulator works with a plain `http://localhost:1420` devUrl** — the port is tunnelled device-localhost → host-localhost over adb.

**Mechanism 2 — LAN-IP rewrite (conditional).** From [crates/tauri-cli/src/mobile/android/dev.rs](https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-cli/src/mobile/android/dev.rs) (~lines 241–260):
```rust
// when --host is provided or running on a physical device or resolving 0.0.0.0 we must use the network IP
if options.host.0.is_some()
  || device.as_ref().map(|d| !d.serial_no().starts_with("emulator")).unwrap_or(false)
  || tauri_config.build.dev_url.as_ref().is_some_and(|url| matches!(url.host(), Some(Host::Ipv4(i)) if i == Ipv4Addr::UNSPECIFIED))
{
  use_network_address_for_dev_url(...)?;
}
```
i.e. **emulator → localhost + `adb reverse`; physical device → devUrl host replaced with your machine's LAN IP.** The CLI then logs (from `mobile/mod.rs::use_network_address_for_dev_url`):
> Replacing devUrl host with {ip}. If your frontend is not listening on that address, try configuring your development server to use the `TAURI_DEV_HOST` environment variable or 0.0.0.0 as host

**Consequence for a Trunk/Leptos setup:** on a **physical Android device** your `trunk serve` must bind to `TAURI_DEV_HOST` / `0.0.0.0`, not `127.0.0.1` — even though the docs only spell this out for iOS. Relevant `android dev` flags ([CLI reference](https://v2.tauri.app/reference/cli/)): `--host` ("Use public network address for development server"), `--force-ip-prompt`, `--no-dev-server`, `--port` (defaults to 1430), `--root-certificate-path`.

### 2.6 Issue-tracker reality check

Query `repo:tauri-apps/tauri is:issue is:open label:"platform: Android"` returns **82 open issues** as of 2026-07-26 ([GitHub search](https://github.com/tauri-apps/tauri/issues?q=is%3Aissue+is%3Aopen+label%3A%22platform%3A+Android%22)).

**Build / toolchain**
[#15345](https://github.com/tauri-apps/tauri/issues/15345) (Windows NDK path separators, `status: upstream`) · [#15385](https://github.com/tauri-apps/tauri/issues/15385) (no Java/Gradle compat hint) · [#11841](https://github.com/tauri-apps/tauri/issues/11841) (16💬, `NDK_HOME isn't set` though NDK installed) · [#11967](https://github.com/tauri-apps/tauri/issues/11967) (missing `libandroid.so`) · [#10886](https://github.com/tauri-apps/tauri/issues/10886) (Linux `:app:rustBuildArmDebug` failure) · [#10937](https://github.com/tauri-apps/tauri/issues/10937) (Windows symlink) · [#11780](https://github.com/tauri-apps/tauri/issues/11780) (`ring_core_0_17_8_` static lib) · [#14813](https://github.com/tauri-apps/tauri/issues/14813) (`android init` differs Windows vs Ubuntu) · [#13148](https://github.com/tauri-apps/tauri/issues/13148) (Gradle download timeout) · [#8559](https://github.com/tauri-apps/tauri/issues/8559) (2y7mo — build fails if the project path contains a space).

**Signing / release / store**
[#15337](https://github.com/tauri-apps/tauri/issues/15337) (**17💬** — Android 15 release crash) · [#13379](https://github.com/tauri-apps/tauri/issues/13379) (`isMinifyEnabled` crashes the app) · [#14413](https://github.com/tauri-apps/tauri/issues/14413) (**`priority: 1 high`** — versionName/versionCode not populated since 2.9.2) · [#13201](https://github.com/tauri-apps/tauri/issues/13201) (no `tauri.properties` when version derives from `Cargo.toml`) · [#15561](https://github.com/tauri-apps/tauri/issues/15561) (`applicationIdSuffix` inserted into wrong block) · [#13357](https://github.com/tauri-apps/tauri/issues/13357) (can't build aab/apk) · [#13435](https://github.com/tauri-apps/tauri/issues/13435) (`dev` OK but `build --apk --target aarch64` fails).

**WebView / runtime**
[#8788](https://github.com/tauri-apps/tauri/issues/8788) (**33💬** — fails on Android < 8) · [#8911](https://github.com/tauri-apps/tauri/issues/8911) (**35💬** — `bundle.resources` doesn't work on Android) · [#14694](https://github.com/tauri-apps/tauri/issues/14694) (**`priority: 0 crash`** — freeze on navigation) · [#15506](https://github.com/tauri-apps/tauri/issues/15506) (**`priority: 0 crash`** — `requestPermissions` / `ActivityResultLauncher`) · [#14406](https://github.com/tauri-apps/tauri/issues/14406) (back button exits the app) · [#15671](https://github.com/tauri-apps/tauri/issues/15671) (blank webview after task removal) · [#13479](https://github.com/tauri-apps/tauri/issues/13479) & [#7868](https://github.com/tauri-apps/tauri/issues/7868) (soft-keyboard/viewport resize inconsistency) · [#13554](https://github.com/tauri-apps/tauri/issues/13554) (memory overflow loading image/audio assets) · [#14776](https://github.com/tauri-apps/tauri/issues/14776) (`convertFileSrc` broken for `app_data_dir`) · [#11907](https://github.com/tauri-apps/tauri/issues/11907) (armv7 Samsung A13) · [#12019](https://github.com/tauri-apps/tauri/issues/12019) (excessive range requests for video) · [#15748](https://github.com/tauri-apps/tauri/issues/15748) (custom-protocol authority mismatch vs WebView2/CEF).

**Plugin / API gaps**
[#14695](https://github.com/tauri-apps/tauri/issues/14695) (plugins can't call MainActivity fns) · [#14270](https://github.com/tauri-apps/tauri/issues/14270) (no `onCreate` hook) · [#12588](https://github.com/tauri-apps/tauri/issues/12588) (fullscreen) · [#13408](https://github.com/tauri-apps/tauri/issues/13408) (orientation) · [#11475](https://github.com/tauri-apps/tauri/issues/11475) (SafeArea) · [#13063](https://github.com/tauri-apps/tauri/issues/13063) (direct plugin calls from Rust crash) · [#12741](https://github.com/tauri-apps/tauri/issues/12741) (Logcat level).

**Dev loop:** [#11494](https://github.com/tauri-apps/tauri/issues/11494), [#11108](https://github.com/tauri-apps/tauri/issues/11108), [#15142](https://github.com/tauri-apps/tauri/issues/15142), [#11821](https://github.com/tauri-apps/tauri/issues/11821), [#15153](https://github.com/tauri-apps/tauri/issues/15153), [#15379](https://github.com/tauri-apps/tauri/issues/15379), [#14099](https://github.com/tauri-apps/tauri/issues/14099), [#13739](https://github.com/tauri-apps/tauri/issues/13739), [#12698](https://github.com/tauri-apps/tauri/issues/12698), [#12175](https://github.com/tauri-apps/tauri/issues/12175) — detailed in §6.4.

#### The most decision-relevant for a small offline app

1. **[#13379](https://github.com/tauri-apps/tauri/issues/13379) + [#15337](https://github.com/tauri-apps/tauri/issues/15337) — release builds crash on launch out of the box** (`isMinifyEnabled = true` in the generated `release` buildType). Both open. **The default template does not ship a working release build.** See §2.5b.
2. **[#14413](https://github.com/tauri-apps/tauri/issues/14413) (`priority: 1 high`)** — versionCode silently falls back to `1`; Play rejects duplicates, so it bites on your *second* upload.
3. **[#8788](https://github.com/tauri-apps/tauri/issues/8788)** — the documented SDK 24 floor is not real; see §2.4.
4. **[#14694](https://github.com/tauri-apps/tauri/issues/14694)** (`priority: 0 crash`) — freeze on navigation.
5. **[#14406](https://github.com/tauri-apps/tauri/issues/14406)** — the hardware back button **exits the app** rather than navigating back. For a card-review SPA this is an immediately visible UX defect you must work around yourself.
6. **[#13479](https://github.com/tauri-apps/tauri/issues/13479) / [#7868](https://github.com/tauri-apps/tauri/issues/7868)** — soft-keyboard/viewport behaviour is inconsistent. Relevant to any typed-answer flashcard mode.
7. **[#8911](https://github.com/tauri-apps/tauri/issues/8911)** (35💬) — `bundle.resources` does not work on Android. **Directly relevant if you want to ship a seed deck / prepopulated SQLite file.**
8. **[#15385](https://github.com/tauri-apps/tauri/issues/15385) / PR [#15780](https://github.com/tauri-apps/tauri/pull/15780)** — JDK/Gradle mismatch fails cryptically; the warning PR is still unmerged (§2.3b).

The shape of this list: **no fundamental "Android is broken" blocker** — the runtime is fine — but the sharp edges cluster in the **release path** and in **observability** (no Rust logs in `android dev`, no logcat level control).

---

## 3. Architecture reality-check: where does Rust run?

### 3.1 The two processes

Tauri's own process-model page ([v2.tauri.app/concept/process-model/](https://v2.tauri.app/concept/process-model/)):

> Each Tauri application has a **core process**, which acts as the application's entry point and which is the only component with full access to the operating system.

> The Core process doesn't render the actual user interface (UI) itself; it spins up **WebView processes** that leverage WebView libraries provided by the operating system.

> the WebView libraries are **not** included in your final executable but dynamically linked at runtime

So in a Tauri+Leptos app there are **two Rust compilation products**:

| | Tauri core | Leptos frontend |
|---|---|---|
| Compiles to | native host target (`x86_64-unknown-linux-gnu`, …) or Android target (`aarch64-linux-android`, …) | `wasm32-unknown-unknown` |
| Runs in | the Core process | inside the system WebView |
| Built by | `cargo` (driven by `tauri-cli`) | `trunk` |
| Has | filesystem, OS APIs, native deps, threads, tokio | browser sandbox only |

### 3.2 The official template proves the two-crate split

The official [`create-tauri-app` Leptos template](https://github.com/tauri-apps/create-tauri-app/tree/dev/templates/template-leptos) is a **cargo workspace with two crates**:

`templates/template-leptos/Cargo.toml.lte` ([source](https://github.com/tauri-apps/create-tauri-app/blob/dev/templates/template-leptos/Cargo.toml.lte)):
```toml
[package]
name = "{% package_name %}-ui"
version = "0.1.0"
edition = "2021"

[dependencies]
leptos = { version = "0.8", features = ["csr"] }
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
js-sys = "0.3"
serde = { version = "1", features = ["derive"] }
serde-wasm-bindgen = "0.6"
console_error_panic_hook = "0.1.7"

[workspace]
members = ["src-tauri"]
```

Note: `features = ["csr"]` — **the official Leptos-on-Tauri template is CSR-only**, no SSR, no server functions. That directly matches the "no server of our own" requirement.

The template also ships a `.taurignore` ([source](https://github.com/tauri-apps/create-tauri-app/blob/dev/templates/template-leptos/.taurignore)) containing:
```
/src
/public
/Cargo.toml
```
i.e. frontend edits are deliberately excluded from the Tauri core's rebuild watcher.

### 3.3 How they communicate

**Mechanism: JSON-RPC-like message passing, not FFI.** From [v2.tauri.app/concept/inter-process-communication/](https://v2.tauri.app/concept/inter-process-communication/):

> The primary API, `invoke`, is similar to the browser's `fetch` API and allows the Frontend to invoke Rust functions, pass arguments, and receive data.

> Because this mechanism uses a **JSON-RPC like protocol** under the hood to serialize requests and responses, **all arguments and return data must be serializable to JSON**.

There are two primitives: **Commands** (request/response, frontend → core) and **Events** (fire-and-forget, bidirectional). Commands are explicitly *not* FFI — the core is free to reject requests.

From [v2.tauri.app/develop/calling-rust/](https://v2.tauri.app/develop/calling-rust/):
- `#[tauri::command]` + `tauri::generate_handler!` registers a function.
- "Async commands are executed on a separate async task using `async_runtime::spawn`." Sync commands run on the main thread unless marked `#[tauri::command(async)]`.
- Async commands cannot take borrowed args (`&str`, `State<'_, T>`) — use owned types or wrap in `Result`.
- "Everything returned from commands must implement `serde::Serialize`, including errors."
- **Streaming:** "The Tauri channel is the recommended mechanism for streaming data … to the frontend."
- **Escape hatch for large payloads:** "Return values that implement `serde::Serialize` are serialized to JSON … this can slow down your application if you try to return large data." Use `tauri::ipc::Response` to return raw bytes and bypass JSON.

**IPC cost, concretely:** every core↔frontend call round-trips through JSON serialize → JS string/value → deserialize. For a flashcard app this is fine at the granularity of "give me the next due card" or "record this review"; it is *not* fine at the granularity of "stream 50k rows to the UI". If you ever need bulk transfer, `tauri::ipc::Response` with a binary encoding is the documented workaround.

### 3.4 Wasm-side bindings: what actually exists

The **official template does not use a binding crate at all.** It hand-declares the binding via `wasm-bindgen` ([src/app.rs.lte](https://github.com/tauri-apps/create-tauri-app/blob/dev/templates/template-leptos/src/app.rs.lte)):

```rust
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}
```
with `serde_wasm_bindgen::to_value(&GreetArgs { name: &name })` to build the args and `.as_string().unwrap()` to read the result. This requires `app.withGlobalTauri = true` in `tauri.conf.json` so `window.__TAURI__` exists — the [Leptos frontend guide](https://v2.tauri.app/start/frontend/leptos/) states:

> Enable `withGlobalTauri` to ensure that Tauri APIs are available in the `window.__TAURI__` variable and can be imported using `wasm-bindgen`.

and the template's `.manifest` sets `withGlobalTauri = true` accordingly ([source](https://github.com/tauri-apps/create-tauri-app/blob/dev/templates/template-leptos/.manifest)). **Note `withGlobalTauri` defaults to `false`** — see §4.3 for why this matters and which global to detect on.

Third-party alternatives (full comparison in §4.6):
- **`tauri-sys`** — "Bindings to the Tauri API for projects using wasm-bindgen". 123 stars, Apache-2.0, branch `v2`, pushed **2026-07-19**, 23 open issues ([GitHub API](https://api.github.com/repos/JonasKruckenberg/tauri-sys)). **NOT published on crates.io** — [`crates.io/api/v1/crates/tauri-sys` returns 404](https://crates.io/api/v1/crates/tauri-sys); git dependency only, and it **requires a global `esbuild` install**. Has an `examples/leptos` crate.
- **`tauri-wasm`** — on crates.io at **0.2.0 (2025-04-07)**, no release in 15 months, 11 stars ([crates.io](https://crates.io/crates/tauri-wasm), [docs.rs](https://docs.rs/tauri-wasm/latest/tauri_wasm/)).
- **`tauri-ipc-macros`** — "IPC bindings for using Tauri with a Rust Frontend (e.g. leptos)" ([crates.io](https://crates.io/crates/tauri-ipc-macros/0.1.3), [repo](https://github.com/jvatic/tauri-ipc-macros)).

**Verdict on the "one codebase in both" question:** there is **no supported way to run the *same* Rust code in both the core and the webview as one compilation unit**. The wasm side and the native side are separate crates with different target triples and different dependency sets. What you *can* do — and what a well-structured project would do — is factor **pure domain logic (scheduling algorithm, card model, serde types) into a third `no_std`-ish/portable crate** that both the UI crate and the `src-tauri` crate depend on. Anything touching the filesystem, SQLite, or OS APIs can only live on the native side, and reaching it from the UI **always** crosses the JSON IPC boundary. The two-crate split and the serialization boundary are forced, not optional.

---

## 4. The plain-web target (no Tauri shell)

### 4.1 `leptos_router` has no hash routing — a real constraint for static hosting

[leptos-rs/leptos#2184 "Support hashstyle routing"](https://github.com/leptos-rs/leptos/issues/2184) has been **open since 2024-01-13**, last updated 2025-06-26, 12 comments. The maintainer (`gbj`, 2025-01-13) explains the architecture:

> The router is designed to support multiple different "location providers," but it has only ever had one (using the browser's History API). … The only place I'm aware that it's hardcoded to use that location provider is [here] …

Two independent contributors attempted implementations and gave up. `mohe2015` (2026-06-26):

> I experimented a lot with this but failed to implement this in a way that I felt confident with and that was working properly. I think the main problem is that **many places in the code currently assume the routing url to be equal to the browser url**.

Consequences for the plain-web build:
- Deploying a Leptos CSR app to a **passive static host** (GitHub Pages, plain S3, `python -m http.server`) means a direct load of `/review/deck-1` 404s, because the path doesn't exist as a file. The issue's original report says exactly this.
- Workarounds documented in the thread: server rewrite rules (`RewriteEngine on`), or for GitHub Pages, **copying `index.html` to `404.html`** (`kahboon0425`, 2025-05-05).
- Under Tauri the app is served from a custom protocol origin rather than a static file server, so History-API routing works there — meaning **this constraint bites the web target only**, and it is easy to miss until you deploy.

**Unverified:** whether the `leptos_0.9` branch changes the `LocationProvider` design. #2184 is not marked as fixed and the 0.9 notes don't mention it.

Leptos's own [CSR deployment chapter](https://book.leptos.dev/deployment/csr.html) documents the workarounds explicitly: Netlify `[[redirects]] from = "/*" to = "/index.html" status = 200`; GitHub Pages `cp dist/index.html dist/404.html`. **This divergence applies to the web build only** — the Tauri build is served from `tauri://localhost`, where pushState routing works.

### 4.2 What `trunk` produces

| Fact | Value | Source |
|---|---|---|
| Latest stable | **0.21.14**, 2025-05-08 (~14 months old) | [crates.io](https://crates.io/api/v1/crates/trunk) |
| Prereleases | `0.22.0-beta.1` 2026-03-10, `0.22.0-beta.2` 2026-07-24 | same |
| Repo activity | pushed 2026-07-24, 4357 stars | [trunk-rs/trunk](https://github.com/trunk-rs/trunk) |

Commands, verbatim from the [Trunk guide](https://trunkrs.dev/commands/):
> `trunk build` runs a cargo build targeting the wasm32 instruction set, runs `wasm-bindgen` on the built WASM, and spawns asset build pipelines for any assets defined in the target `index.html`.
> `trunk watch` does the same thing as `trunk build`, but also watches the filesystem for changes…
> `trunk serve` does the same thing as `trunk watch`, but also spawns a web server.
> `trunk clean` cleans up any build artifacts generated from earlier builds.

`--release` is a real flag on `build` ([src/cmd/build.rs](https://github.com/trunk-rs/trunk/blob/main/src/cmd/build.rs)); `--dist` (`-d`) controls the output dir; `--public-url` handles sub-path deploys.

**Output:** `dist/` containing a rewritten `index.html` plus content-hashed `.js` glue and `_bg.wasm`, with SRI integrity hashes ([trunk site source](https://github.com/trunk-rs/trunk/blob/main/site/content/_index.md)):
```html
<script type="module">
import init, * as bindings from '/my_program_name-905e0077a27c1ab6.js';
const wasm = await init('/my_program_name-905e0077a27c1ab6_bg.wasm');
…
</script>
```
The Leptos book's [CSR deployment chapter](https://book.leptos.dev/deployment/csr.html): "`trunk build` will create a number of build artifacts in a `dist/` directory. **Publishing `dist` somewhere online should be all you need to deploy your app.**"

Note: Tauri's own config points `frontendDist` at `../dist` and `beforeBuildCommand` at `trunk build` (§6.2) — **the same `dist/` output feeds both targets**.

### 4.3 The global-detection trap: `isTauri` vs `__TAURI_INTERNALS__` vs `__TAURI__`

This is the sharpest finding for a dual build. There are three globals and they behave differently.

From Tauri's webview manager ([crates/tauri/src/manager/webview.rs](https://github.com/tauri-apps/tauri/blob/dev/crates/tauri/src/manager/webview.rs), ~lines 167–179), injected **unconditionally** into the main frame:
```js
Object.defineProperty(window, 'isTauri', { value: true });
if (!window.__TAURI_INTERNALS__) {
  Object.defineProperty(window, '__TAURI_INTERNALS__', { value: { plugins: {} } })
}
```

But `window.__TAURI__` is **opt-in**. From [crates/tauri-utils/src/config.rs](https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-utils/src/config.rs) (~line 3154):
```rust
/// Whether we should inject the Tauri API on `window.__TAURI__` or not.
#[serde(default, alias = "with-global-tauri")]
pub with_global_tauri: bool,
```
`#[serde(default)]` on a `bool` ⇒ **defaults to `false`**. Maintainer *FabianLars* in [discussion #6119](https://github.com/tauri-apps/tauri/discussions/6119) (2024-03-19): "we only set `__TAURI__` now if withGlobalTauri is enabled. Try `__TAURI_INTERNALS__` instead."

| Global | Present in Tauri v2? | Present in a plain browser? |
|---|---|---|
| `window.isTauri` | **always** — the correct detection flag | no |
| `window.__TAURI_INTERNALS__` | **always** (carries `invoke`, `convertFileSrc`, `transformCallback`, `metadata`) | no |
| `window.__TAURI__` | **only when `withGlobalTauri: true`** | no |

The official JS detection API ([packages/api/src/core.ts:337](https://github.com/tauri-apps/tauri/blob/dev/packages/api/src/core.ts)):
```ts
function isTauri(): boolean {
  return !!((globalThis as any) || window).isTauri
}
```
It is documented public API: [JS API reference](https://v2.tauri.app/reference/javascript/api/namespacecore/).

### 4.4 What happens if you call `invoke` with no Tauri host: it throws *uncatchably*

**All three mainstream Rust paths throw a raw JS `TypeError` that Rust cannot recover from**, unless the binding was declared `catch`.

**(a) The official Tauri Leptos template — throws, uncaught.** [src/app.rs.lte](https://github.com/tauri-apps/create-tauri-app/blob/dev/templates/template-leptos/src/app.rs.lte) declares `invoke` with **no `catch`** on `js_namespace = ["window", "__TAURI__", "core"]`. In a plain browser `window.__TAURI__` is `undefined`, so the call raises `TypeError: Cannot read properties of undefined`. Per the [wasm-bindgen `catch` docs](https://rustwasm.github.io/docs/wasm-bindgen/reference/attributes/on-js-imports/catch.html):
> By default `wasm-bindgen` will take no action when Wasm calls a JS function which ends up throwing an exception. The Wasm spec right now doesn't support stack unwinding and as a result Rust code **will not execute destructors**.

So the exception propagates through the wasm boundary, destructors are skipped, and you cannot turn it into a `Result` in Rust. (The template additionally does `.as_string().unwrap()`, which would panic anyway.)

**(b) `tauri-sys` — depends which function you call.** [src/core.rs](https://github.com/JonasKruckenberg/tauri-sys/blob/v2/src/core.rs):
```rust
#[wasm_bindgen(module = "/src/core.js")]
extern "C" {
    pub async fn invoke(cmd: &str, args: JsValue) -> JsValue;                        // NO catch
    #[wasm_bindgen(js_name = "invoke", catch)]
    pub async fn invoke_result(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>; // catch
    #[wasm_bindgen(js_name = "isTauri")]
    pub fn is_tauri() -> bool;
}
```
**Use `invoke_result`, never `invoke`, in a dual build.** Its JS shim binds `window.__TAURI_INTERNALS__.invoke` and implements `isTauri()` as `'isTauri' in window && !!window.isTauri` ([src/core.js](https://github.com/JonasKruckenberg/tauri-sys/blob/v2/src/core.js)).

**(c) `tauri-wasm` — its `Result` cannot catch this case at all.** [tauri-wasm/src/invoke.rs](https://github.com/nanoqsh/tauri-wasm/blob/main/tauri-wasm/src/invoke.rs) maps only **promise rejection** into `Error`; the undefined-namespace `TypeError` fires inside `into_future()` *synchronously, before a `Promise` exists*, so it escapes as a raw JS exception. It also binds through `window.__TAURI__`, so it **requires `withGlobalTauri = true`** even inside a real Tauri app — its own example sets `with-global-tauri = true` in [examples/app-tauri/Tauri.toml](https://github.com/nanoqsh/tauri-wasm/blob/main/examples/app-tauri/Tauri.toml).

**Rule for implementers:** guard every `invoke` behind `is_tauri()` (`window.isTauri`), or compile the calls out entirely via a Cargo feature. **Never** gate on `window.__TAURI__`.

### 4.5 Official guidance on conditional compilation vs runtime detection

- **Runtime detection:** maintainer *FabianLars* in [discussion #6119](https://github.com/tauri-apps/tauri/discussions/6119) evolved from "check if `window.__TAURI__` exists" (2023) → "`__TAURI__` only if withGlobalTauri; try `__TAURI_INTERNALS__`" (2024) → "Tauri already provides this through: `import { isTauri } from \"@tauri-apps/api/core\"`".
- **Conditional compilation:** [discussion #2725](https://github.com/tauri-apps/tauri/discussions/2725):
  > The most reliable solution is some manner of conditional compilation of your code, whether you do it with Webpack, Vite or WASM is entirely up to you.

**The Leptos-side reference project goes all-in on Cargo features**, not runtime detection: [leptos-rs/leptos/projects/tauri-from-scratch](https://github.com/leptos-rs/leptos/tree/main/projects/tauri-from-scratch) uses one crate with three feature sets:
```toml
csr     = ["leptos/csr", "dep:server_fn"]
hydrate = ["leptos/hydrate", "dep:leptos_meta", "dep:console_error_panic_hook", "dep:wasm-bindgen"]
ssr     = ["dep:axum", "leptos/ssr", "dep:leptos_axum", ...]
```
with `"beforeBuildCommand": "trunk build --no-default-features -v --features \"csr\""` in `tauri.conf.json`.

**But note what that project actually is:** it ships an SSR web app *and* a Tauri CSR client that talks to the same HTTP server — it **never calls `invoke` at all**:
```rust
#[cfg(feature = "csr")]
pub fn main() {
    server_fn::client::set_server_url("http://127.0.0.1:3000");
    leptos::mount::mount_to_body(App);
}
```
> Here we're setting the server functions to use the url base that we access in our browser. … Otherwise our tauri app will try to route server function network requests using it's own idea of what it's url is. Which is `tauri://localhost` on macOS, and something else on windows.

It also requires CORS (`allow_origin("tauri://localhost")`) and warns "If you are on windows the origin of your app will be different than `tauri://localhost`." **That architecture is the opposite of "no server of our own"** — it is not a model for this project, but it *is* the only in-repo Leptos+Tauri reference.

Trust level caveat, from [projects/README.md](https://github.com/leptos-rs/leptos/blob/main/projects/README.md): "The barrier to entry for the `projects` directory is intended to be lower: Example projects will generally be built against a particular version, and **not regularly linted or updated**." This one pins `rev = "v0.8.2"`.

**No official Leptos↔Tauri SSR integration exists and none is planned.** [leptos-rs/leptos#4043](https://github.com/leptos-rs/leptos/issues/4043) was closed 2025-06-19 with gbj replying:
> I'm not sure there's value in SSR per se, as in rendering HTML in the Tauri part of the app and sending it to the web view rather than just doing rendering in the webview

**Searching `leptos-rs/book` for "tauri" returns zero hits — the Leptos book contains no Tauri guidance at all.**

### 4.6 `tauri-sys` vs `tauri-wasm`, side by side

| | `tauri-sys` | `tauri-wasm` |
|---|---|---|
| On crates.io | **No** — [404](https://crates.io/api/v1/crates/tauri-sys) | **Yes** — [crates.io](https://crates.io/crates/tauri-wasm) |
| Version | `0.2.4` (in-repo, git-only) | **`0.2.0`** |
| Published / pushed | repo pushed **2026-07-19** | crate **2025-04-07**; repo pushed 2025-08-16 |
| Downloads | n/a | 5,142 total / 1,095 recent |
| Stars | 123 | 11 |
| Extra build requirement | **requires a global `esbuild` install** | none |
| Binds via | `window.__TAURI_INTERNALS__` | `window.__TAURI__.core` (needs `withGlobalTauri`) |
| API surface | app, core, dpi, event, image, partial menu/window. Missing: mocks, path, tray, webview, webviewWindow | `invoke`, `emit`, `event`, `is_tauri`, `args` |

`tauri-sys`'s README: "This crate is not yet published to crates.io, so you need to use it from git. **You also need a global installation of `esbuild`**." `tauri-wasm`'s pitch: "Interact with a Tauri backend using the pure Rust library. You don't need NPM or any other JavaScript tools to build a frontend, use Cargo instead."

**Neither is a safe long-term bet on its own:** `tauri-wasm` has had **no crates.io release in 15 months** and 11 stars; `tauri-sys` is more active but is a git dependency requiring `esbuild` in CI. The **official template's hand-rolled `#[wasm_bindgen]` `extern "C"` block** (§3.4) has no such dependency and is ~10 lines — for a handful of commands that is arguably the lowest-risk option.

### 4.7 Summary: what the code must abstract over

If persistence and logic live **in the Tauri core**, the web build has no equivalent, so you need:
1. A **storage trait** with two impls: `invoke`-backed (Tauri) and wasm-SQLite/IndexedDB-backed (web) — see §5.6.
2. A **Cargo feature** (`--features tauri`) selecting the impl at compile time, because runtime `invoke` failures are uncatchable (§4.4). Runtime `is_tauri()` detection is possible but only as a *guard*, not as error recovery.
3. **Routing rewrite rules** for the web host (§4.1) that the Tauri build does not need.
4. `withGlobalTauri: true` if you use the official template's binding or `tauri-wasm`.

If instead the app is **fully browser-local** (all storage via `web-sys`/OPFS/IndexedDB, Tauri used purely as a window shell), then **there is nothing to abstract** — one `trunk build`, one `dist/`, the identical wasm runs in both. That is dramatically cheaper, at the cost of giving up native SQLite in the core.

---

## 5. Local storage from Rust

### 5.1 Tauri core, desktop: yes, `rusqlite`/`sqlx` work directly

The Tauri core is an ordinary native Rust binary, so any native crate works. Current versions:

| Crate | Latest stable | Date | Source |
|---|---|---|---|
| `rusqlite` | **0.40.1** | 2026-06-06 | [crates.io](https://crates.io/api/v1/crates/rusqlite) |
| `libsqlite3-sys` | **0.38.1** | 2026-06-06 | [crates.io](https://crates.io/api/v1/crates/libsqlite3-sys) |
| `sqlx` | **0.9.0** | 2026-05-21 | [crates.io](https://crates.io/api/v1/crates/sqlx) |
| `diesel` | **2.3.11** | 2026-07-10 | [crates.io](https://crates.io/api/v1/crates/diesel) |

### 5.2 Android: what cross-compilation requires

**Android does not ship a stable, linkable `libsqlite3.so` for NDK use.** The consequence is that you must **compile SQLite from source into your binary** — i.e. enable the `bundled` feature, which builds the amalgamation with the `cc` crate using the NDK's clang.

- `rusqlite`: enable `features = ["bundled"]`, which enables `libsqlite3-sys/bundled`. The known failure mode without it is a linker error for missing SQLite; see [rusqlite#503 "Android: during bundled compile, cannot find aarch64-linux-android-clang"](https://github.com/rusqlite/rusqlite/issues/503) and [tauri discussion #7340 "sqlite and android? missing libsqlite3.so.0"](https://github.com/tauri-apps/tauri/discussions/7340).
- `sqlx`: in **0.8.x**, the `sqlite` feature already implies bundling — verbatim from [sqlx v0.8.6 Cargo.toml](https://github.com/launchbadge/sqlx/blob/v0.8.6/Cargo.toml#L111):
  ```toml
  sqlite = ["_sqlite", "sqlx-sqlite/bundled", "sqlx-macros?/sqlite"]
  ```
  and [sqlx-sqlite v0.8.6 Cargo.toml](https://github.com/launchbadge/sqlx/blob/v0.8.6/sqlx-sqlite/Cargo.toml#L28): `bundled = ["libsqlite3-sys/bundled"]`.
- **Changed in sqlx 0.9.0**: the root `sqlite` feature now expands to `["sqlite-bundled", "sqlite-deserialize", "sqlite-load-extension", "sqlite-unlock-notify"]`, and `sqlite-bundled` / `sqlite-unbundled` are separate opt-ins; the base `sqlx-sqlite` dependency declares `libsqlite3-sys` with only `["pkg-config", "vcpkg"]` ([sqlx main Cargo.toml](https://github.com/launchbadge/sqlx/blob/main/Cargo.toml), [sqlx-sqlite main Cargo.toml](https://github.com/launchbadge/sqlx/blob/main/sqlx-sqlite/Cargo.toml)). Bundling is still the default *via the `sqlite` feature*, but if you hand-pick `sqlite-unbundled` on Android you will fail to link. Also note sqlx 0.9.0's MSRV is **1.94.0** ([sqlx main Cargo.toml](https://github.com/launchbadge/sqlx/blob/main/Cargo.toml)).
- Requirements that follow: the **NDK's clang must be reachable** (`NDK_HOME` set, and cargo configured with the right linker per target). `tauri-cli` sets this up for you when you build through `cargo tauri android build`; a bare `cargo build --target aarch64-linux-android` will typically fail on linking unless you configure `.cargo/config.toml` yourself — the symptom reported in [tauri#6405 "Various Troubles Cross-Compiling to Android"](https://github.com/tauri-apps/tauri/issues/6405).
- Because SQLite is compiled into the binary, remember the **NDK r28+ / 16 KB page alignment** requirement from §2.3 ([mobile plugin docs](https://v2.tauri.app/develop/plugins/develop-mobile/)).

### 5.3 Getting the app-private data directory

`tauri::path::BaseDirectory` has 23 variants. App-scoped ones, quoted from [docs.rs](https://docs.rs/tauri/latest/tauri/path/enum.BaseDirectory.html):
- `AppData` — "The default app data directory. Resolves to `BaseDirectory::Data`/`{bundle_identifier}`."
- `AppLocalData` — "Resolves to `BaseDirectory::LocalData`/`{bundle_identifier}`."
- `AppConfig` — "Resolves to `BaseDirectory::Config`/`{bundle_identifier}`."
- `AppCache` — "Resolves to `BaseDirectory::Cache`/`{bundle_identifier}`."
- `Desktop`, `Executable`, `Font`, `Runtime`, `Template` are "Available on **non-Android** only" — do not use them in shared code.

**On Android specifically**, the Rust resolver calls into a Kotlin plugin ([crates/tauri/src/path/android.rs](https://github.com/tauri-apps/tauri/blob/dev/crates/tauri/src/path/android.rs)) and the Kotlin side resolves both `getConfigDir` and `getDataDir` to `Context.dataDir` ([PathPlugin.kt](https://github.com/tauri-apps/tauri/blob/dev/crates/tauri/mobile/android/src/main/java/app/tauri/PathPlugin.kt)):
```kotlin
@Command
fun getConfigDir(invoke: Invoke) {
  if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
    resolvePath(invoke, activity.dataDir.absolutePath)
  } else {
    resolvePath(invoke, activity.applicationInfo.dataDir)
  }
}
```
`getDataDir` is byte-identical. So on Android, `app_data_dir()`, `app_local_data_dir()` and `app_config_dir()` all land under the **app-private internal storage** directory (`/data/user/0/<package>/…` or `/data/data/<package>/…`), namespaced by `{bundle_identifier}`. That is exactly where an offline SQLite file belongs — no runtime storage permission needed, and it is removed on uninstall.

Usage in the core: `app.path().app_data_dir()?` then `std::fs::create_dir_all(&dir)` before opening the DB (the SQL plugin does exactly this, §5.4).

### 5.4 `tauri-plugin-sql`

| Fact | Value | Source |
|---|---|---|
| Latest | **2.4.0**, 2026-04-04 | [crates.io](https://crates.io/api/v1/crates/tauri-plugin-sql) |
| Backing library | `sqlx` **0.8** with `["json","time","uuid","rust_decimal"]` | [plugins/sql/Cargo.toml](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/sql/Cargo.toml) |
| Feature map | `sqlite = ["sqlx/sqlite", "sqlx/runtime-tokio"]` | same |
| Declared platform support | `android = { level = "full" }` in Cargo metadata | same |
| Stated MSRV | 1.77.2 | [plugin README](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/sql/README.md) |

What it provides: a `Database` class for the **JS frontend** (`Database.load(connectionString)`, `db.execute(query, params)`, `db.select(...)`), plus a Rust-side `Builder` for registering **migrations** at app startup; "all migrations…[execute] within a transaction" ([docs](https://v2.tauri.app/plugin/sql/)).

**Where the DB file lands** — read from the source, not the docs. [plugins/sql/src/wrapper.rs](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/sql/src/wrapper.rs):
```rust
"sqlite" => {
    let app_path = _app
        .path()
        .app_config_dir()
        .expect("No App config path was found!");
    create_dir_all(&app_path).expect("Couldn't create app config dir");
    let conn_url = &path_mapper(app_path, conn_url);
    ...
```
`path_mapper` pushes everything after the `:` onto that path. So `sqlite:leitner.db` becomes `<app_config_dir>/leitner.db` — **`app_config_dir`, not `app_data_dir`**. On Android that is `Context.dataDir/{bundle_identifier}/leitner.db` (§5.3).

**Limits of `tauri-plugin-sql`:**
- Because it depends on `sqlx 0.8` and `sqlx 0.8`'s `sqlite` feature implies `sqlx-sqlite/bundled` (§5.2), **SQLite is bundled/compiled from source** transitively — good for Android, but it means the plugin pins you to sqlx 0.8 and its bundled SQLite version even though sqlx 0.9.0 is out.
- It is designed as a **frontend-facing** plugin: you send SQL strings across the IPC boundary and get JSON rows back. That is the opposite of putting your domain logic in the core.
- No compile-time-checked queries (you're not using the `sqlx::query!` macros through the plugin), no typed row mapping — results arrive as `serde_json` values.
- **Documentation inconsistency:** the plugin README's support table says **iOS: x** while its own `Cargo.toml` metadata says `ios = { level = "full" }` ([README](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/sql/README.md) vs [Cargo.toml](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/sql/Cargo.toml)). Irrelevant here (no iOS) but a signal about doc drift.
- The `[features]` block carries a literal `# TODO: bundled-cipher etc` comment — no SQLCipher/encryption feature exists yet ([Cargo.toml](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/sql/Cargo.toml)).

**For a Rust-core-centric app, you do not need this plugin at all** — depend on `rusqlite`/`sqlx` directly in `src-tauri` and expose domain-level `#[tauri::command]`s. The plugin only earns its keep if the *frontend* wants to issue SQL.

### 5.5 `tauri-plugin-store`

| Fact | Value | Source |
|---|---|---|
| Latest | **2.4.4**, 2026-07-18 | [crates.io](https://crates.io/api/v1/crates/tauri-plugin-store) |
| Declared platform support | windows/linux/macos/**android**/ios all `level = "full"` | [plugins/store/Cargo.toml](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/store/Cargo.toml) |
| Native deps | pure Rust — `serde`, `serde_json`, `tokio` (sync/time/macros), `dunce` | same |

What it is: "a persistent key-value store" that lets you "persist state to a file which can be saved and loaded on demand including between app restarts" ([docs](https://v2.tauri.app/plugin/store/)). Values are **JSON** (`serde_json::Value`). Rust API is synchronous (`store.set()`, `store.get()`); the JS API is async, with a `LazyStore` that "only loads the store on first access".

**Limits:** it is a JSON blob keyed by filename — no queries, no indexes, no transactions, no partial writes; you load and save whole documents. Values "must be `serde_json::Value` instances, otherwise, they will not be compatible with the JavaScript bindings" ([docs](https://v2.tauri.app/plugin/store/)). Suitable for **settings/preferences**, not for a flashcard corpus with a review schedule.

**Unverified:** the store docs do not state the default on-disk directory; you pass a filename and the plugin resolves it. **Unverified** exactly which `BaseDirectory` it uses.

### 5.6 Plain-web target: no core process, no filesystem

On the pure-web build there is no Tauri core at all — the Leptos wasm bundle is the whole app, sandboxed in the browser. Options with **working Rust/wasm bindings today**:

**A. SQLite compiled to wasm — `sqlite-wasm-rs`**
| Fact | Value | Source |
|---|---|---|
| Latest | **0.5.5**, 2026-05-25 | [crates.io](https://crates.io/api/v1/crates/sqlite-wasm-rs) |
| Recent downloads | 3,894,614 | same |
| Description | "`wasm32-unknown-unknown` bindings to the libsqlite3 library" | same |
| MSRV | 1.85.0 | [README](https://github.com/Spxg/sqlite-wasm-rs/blob/master/README.md) |

Three VFS backends ([README](https://github.com/Spxg/sqlite-wasm-rs/blob/master/README.md)):
| Backend | Storage | Durability | Multi-connection | Notes |
|---|---|---|---|---|
| Memory (default) | RAM | full (in-session) | ❌ | no setup |
| `SyncAccessHandlePoolVFS` | **OPFS** | full | ❌ | **requires a dedicated worker** |
| `RelaxedIdbVFS` | **IndexedDB**, block-based | **relaxed only** | ❌ | |

None require COOP/COEP headers. Build options: bundled compilation (needs the **emscripten toolchain**) or a `precompiled`/prebuilt `libsqlite3.a` via a build-script override.

**Two hard caveats:**
1. > This library is **not compatible with rusqlite or diesel** directly — it's a low-level FFI binding (`use sqlite_wasm_rs as ffi`).
   So the web build cannot reuse `rusqlite` query code from the native build. Any shared data-access layer must be written against your own trait, with two implementations.
2. > This library is **not thread-safe** — due to `JsValue` limitations and SQLite compiled with `-DSQLITE_THREADSAFE=0`.

Encryption is available via a `sqlite3mc` feature ([README](https://github.com/Spxg/sqlite-wasm-rs/blob/master/README.md)).

**B. IndexedDB directly**
| Crate | Latest | Date | Recent DL | Source |
|---|---|---|---|---|
| `indexed_db_futures` | **0.6.4** | 2025-05-11 | 709,719 | [crates.io](https://crates.io/api/v1/crates/indexed_db_futures) |
| `idb` | **0.6.5** | 2025-12-29 | 180,359 | [crates.io](https://crates.io/api/v1/crates/idb) |
| `indexed_db` | 0.4.2 stable (0.5.0 **yanked** 2026-05-24; 0.5.0-alpha.1 2025-03-29) | | 59,393 | [crates.io](https://crates.io/api/v1/crates/indexed_db) |
| `rexie` | 0.6.2 | **2024-08-12** (stale) | 86,866 | [crates.io](https://crates.io/api/v1/crates/rexie) |

`indexed_db_futures` ("Future bindings for IndexedDB via web_sys") has the widest usage; `idb` is the most recently released. All are thin wrappers over `web-sys`. Note that **none of the four has shipped a release in 2026 except `idb` (2025-12-29) and the yanked `indexed_db` 0.5.0** — this corner of the ecosystem is quiet.

**C. OPFS directly** — available through `web-sys` (**0.3.103**, 2026-06-24, [crates.io](https://crates.io/api/v1/crates/web-sys)), which is "Bindings for all Web APIs, a procedurally generated crate from WebIDL". Using OPFS's synchronous access handles requires running in a **Web Worker**, which is the same constraint `sqlite-wasm-rs`'s OPFS-SAHPool VFS documents.

**D. `localStorage`/`sessionStorage`** — `gloo-storage` **0.4.0**, 2026-03-25, 710,361 recent downloads ([crates.io](https://crates.io/api/v1/crates/gloo-storage)), "Convenience crate for working with local and session storage in browser". Only useful for small settings blobs (~5 MB quota, string values, synchronous).

### 5.7 If you use browser storage *inside* Tauri too, the origin is load-bearing

A tempting way to collapse the two storage implementations is to use OPFS/IndexedDB **in both** targets — the Tauri webview is a real browser engine, so `web-sys` storage works there. That is viable, but the persistence is keyed to the webview's **origin**, and Tauri's origin is configurable and platform-dependent.

From [crates/tauri-utils/src/config.rs](https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-utils/src/config.rs) (line 2180ff), verbatim:

> Sets whether the custom protocols should use `https://<scheme>.localhost` instead of the default `http://<scheme>.localhost` **on Windows and Android**. Defaults to `false`.
>
> ## Note
> Using a `https` scheme will NOT allow mixed content when trying to fetch `http` endpoints and therefore will not match the behavior of the `<scheme>://localhost` protocols used **on macOS and Linux**.
>
> ## Warning
> **Changing this value between releases will change the IndexedDB, cookies and localstorage location and your app will not be able to access the old data.**

So the frontend origin is `tauri://localhost` on macOS/Linux and `http://tauri.localhost` (or `https://…` if `useHttpsScheme` is set) on Windows/Android. **Flipping `useHttpsScheme` after shipping silently orphans all user data.** Treat it as immutable once released. Corroborating: [tauri#14367](https://github.com/tauri-apps/tauri/issues/14367) reports the browser saving credentials against `https://tauri.localhost`.

### 5.8 The structural consequence

Desktop/Android store via **native SQLite in the core process behind JSON IPC**; web stores via **wasm SQLite or IndexedDB in-process with no IPC**. Two storage implementations, two concurrency models (native tokio + threads vs. single-threaded, non-thread-safe wasm), and — because `sqlite-wasm-rs` is explicitly *"not compatible with rusqlite or diesel"* — **no shared query code**. The only genuinely portable layer is your **domain types plus a storage trait**.

The alternative that avoids all of this: put persistence in **browser storage on every target** (OPFS or IndexedDB via `web-sys`/`sqlite-wasm-rs`), and use Tauri purely as a window shell with no `invoke` at all. Then one wasm binary serves all three targets (§4.7), at the price of giving up native SQLite, giving up filesystem-level backup/export without a plugin, and inheriting the origin caveat above.

---

## 6. Dev loop

### 6.1 `tauri dev` (desktop)

From [v2.tauri.app/develop/](https://v2.tauri.app/develop/):
1. Runs your `beforeDevCommand` (for Leptos: `trunk serve`) and points the webview at `devUrl` (default `http://localhost:1420`).
2. Frontend hot reload is the frontend tool's job: "You can make changes to your web app, and if your tooling supports it, the webview should update automatically, just like a browser."
3. Rust core: "`tauri dev` watches your `src-tauri` folder and its dependent crates in the workspace for changes, so your application is automatically rebuilt and restarted whenever you modify them." — i.e. **a core change is a full cargo rebuild + app restart**, not a hot reload.
4. `.taurignore` files "work like regular `.gitignore` files" to exclude paths from triggering rebuilds (the Leptos template ships one — §3.2).
5. `--no-watch` disables auto-rebuild.

### 6.2 Required Leptos/Trunk config

From [v2.tauri.app/start/frontend/leptos/](https://v2.tauri.app/start/frontend/leptos/):

`src-tauri/tauri.conf.json`:
```json
{
  "build": {
    "beforeDevCommand": "trunk serve",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "trunk build",
    "frontendDist": "../dist"
  },
  "app": { "withGlobalTauri": true }
}
```
`Trunk.toml`:
```toml
[build]
target = "./index.html"

[watch]
ignore = ["./src-tauri"]

[serve]
port = 1420
open = false
ws_protocol = "ws"
```

Three caveats stated verbatim on that page:
> Use SSG, Tauri doesn't officially support server based solutions.

> Use `serve.ws_protocol = "ws"` so that the hot-reload websocket can connect properly for mobile development.

> Enable `withGlobalTauri` to ensure that Tauri APIs are available in the `window.__TAURI__` variable and can be imported using `wasm-bindgen`.

**Doc/template drift to watch:** that guide says "This guide applies to Leptos version **0.6**" while stable Leptos is 0.8.20 ([guide](https://v2.tauri.app/start/frontend/leptos/)). And the shipped `create-tauri-app` template's `Trunk.toml.lte` **omits `ws_protocol = "ws"`** ([template source](https://github.com/tauri-apps/create-tauri-app/blob/dev/templates/template-leptos/Trunk.toml.lte)) even though the docs say it is required for mobile hot-reload — so a freshly scaffolded project will likely have broken hot-reload on Android until you add that line yourself.

### 6.3 `tauri android dev`

From [v2.tauri.app/develop/](https://v2.tauri.app/develop/#developing-your-mobile-application):
> By default, the mobile dev command tries to run your application on a connected device, and falls back to prompting you to select a simulator to use.

You can pass a device name as an argument, or use `--open` to hand off to Android Studio — but "the Tauri CLI process **must** be running and **cannot** be killed."

Debugging: the web inspector is Chrome DevTools; "enabled by default for Android emulators, but you must enable it for physical devices" (requires USB Debugging in Developer Options).

Performance work landed recently: 2.11.3 included "Reuse proxy reqwest client in mobile dev, improving the dev load speed" ([#15444](https://github.com/tauri-apps/tauri/pull/15444), [CHANGELOG](https://github.com/tauri-apps/tauri/blob/dev/crates/tauri/CHANGELOG.md)) — confirming the mobile dev server proxies through the CLI process.

Actual `android dev` sequence, read from [crates/tauri-cli/src/mobile/android/dev.rs](https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-cli/src/mobile/android/dev.rs):
1. Resolve target device (`adb device_list`, filtered to `Connected`).
2. Conditionally rewrite `devUrl` to the LAN IP (§2.5c).
3. `crate::dev::setup(...)` → run `beforeDevCommand`, wait for the dev server (unless `--no-dev-server-wait`).
4. Acquire a file lock at `<out_dir>/lock.android`.
5. `configure_cargo`, `generate_tauri_properties`, `sync_debug_application_id_suffix`.
6. `rustup target add <triple>` if missing.
7. An initial Rust `target.build(...)` "to initialize plugins".
8. Gradle assemble + install + launch; then the file-watcher loop.

Device selection ([docs](https://v2.tauri.app/develop/#device-selection)):
```
cargo tauri android dev 'Pixel_7_API_36'
cargo tauri android dev --open     # hands off to Android Studio
```
> To use Xcode or Android Studio, the Tauri CLI process **must** be running and **cannot** be killed. It is recommended to use the `tauri [android|ios] dev --open` command and keep the process alive until you close the IDE.

The CLI **errors if more than one device is connected** (quoted in §2.5c). tauri-cli 2.11.0 added "Prompt to restart the Android emulator if it is not connected to adb" ([#14313](https://github.com/tauri-apps/tauri/pull/14313)).

### 6.4 Known pain points (from the tracker)

- **First build is slow by design.** From [v2.tauri.app/develop/](https://v2.tauri.app/develop/): "The first time you run this command, the Rust package manager may need **several minutes** to download and build all the required packages. Since they are cached, subsequent builds are much faster, as only your code needs rebuilding."
- **Doubled Rust compile on Android deploy — fixed, but only in CLI ≥ 2.11.3.** [#15419](https://github.com/tauri-apps/tauri/issues/15419): "`tauri android run` builds both APK and AAB, causing ~50% slower deploy… 3 Gradle invocations: APK build → Rust compiles (~16s); AAB build → Rust compiles **again** (~15s) ← unnecessary; Install." Closed 2026-06-17 by [#15473](https://github.com/tauri-apps/tauri/pull/15473), released in **tauri-cli 2.11.3**. **Pin CLI ≥ 2.11.3.**
- **Blank page / hangs on `tauri android dev`** — [#11494](https://github.com/tauri-apps/tauri/issues/11494) (9 comments, open since 2024-10-25); [#11108](https://github.com/tauri-apps/tauri/issues/11108) (15 comments, `android dev` hangs on Ubuntu 24.04, open since 2024-09).
- **No Rust debug logs shown by `tauri android dev`** — [#15142](https://github.com/tauri-apps/tauri/issues/15142), open since 2025-12-17: "Almost impossible to develop something if you can't have the most basic log working." Maintainer *Legend-Master* (2026-06-09): "Does it still happen to you on latest? `pnpm tauri android dev` and `cargo tauri android dev` both worked for me with tauri-cli 2.11.2" — likely improved, but the issue is still open. No logcat level control either: [#12741](https://github.com/tauri-apps/tauri/issues/12741).
- **Framework-specific HMR failures on Android** (all open): [#11821](https://github.com/tauri-apps/tauri/issues/11821) (Angular — `[vite] failed to connect to websocket`; a commenter notes "I just tried with svelte framework, it works in android", so it is dev-server-config-specific), [#15153](https://github.com/tauri-apps/tauri/issues/15153) (Dioxus), [#15379](https://github.com/tauri-apps/tauri/issues/15379) (Next 16 App Router — blank screen), [#11165](https://github.com/tauri-apps/tauri/issues/11165) (Nuxt). **Unverified:** no Trunk/Leptos-specific HMR-on-Android issue exists in the tracker — could mean it works, or that nobody is doing it.
- **Emulator breakage:** [#14099](https://github.com/tauri-apps/tauri/issues/14099) (won't launch on Arch Linux), [#12698](https://github.com/tauri-apps/tauri/issues/12698) (emulator opens, app never installs), [#13739](https://github.com/tauri-apps/tauri/issues/13739) (white screen, React+Vite). Combined with the WebView-provider abort ([wry#1785](https://github.com/tauri-apps/wry/issues/1785)), **use a Google-APIs/Play-enabled emulator image, and test on a physical device**. Maintainer note that emulators bundle unrepresentative WebViews: [discussion #11843](https://github.com/tauri-apps/tauri/discussions/11843).
- **IDE/debugging quality is a tracked gap:** [#12175](https://github.com/tauri-apps/tauri/issues/12175) "[feat] Improve IDE toolchain and debugging on Android" (open).
- **Trunk itself is in a slow patch:** last stable is **0.21.14 (2025-05-08)** — ~14 months old — with `0.22.0-beta.1` (2026-03-10) and `0.22.0-beta.2` (2026-07-24) still prerelease ([crates.io](https://crates.io/api/v1/crates/trunk)). The repo is alive (pushed 2026-07-24, 4357 stars), just slow to cut stables. The whole Leptos-CSR dev loop rests on this tool.
- **Rebuild cost is structural:** any change to `src-tauri` triggers a full cargo rebuild and app restart on desktop (§6.1); on Android it additionally cross-compiles and re-packages through Gradle. Mitigation: build a single ABI during development (`--target aarch64`) instead of all four.
- **Unverified:** there are **no published Android rebuild-time figures** anywhere in Tauri's docs — only "several minutes" for the first build and "much faster" afterwards.

---

## 7. Server story: is `#[server]`/SSR optional?

**Short answer from primary sources: yes. CSR-only Leptos is fully coherent, and it is what the official Tauri template ships.**

### 7.1 `csr` is a first-class, mutually exclusive mode

Verbatim from [leptos/Cargo.toml](https://github.com/leptos-rs/leptos/blob/main/leptos/Cargo.toml) (`main` @ 0.8.20):
```toml
hydration = ["reactive_graph/hydration", "leptos_server/hydration", "hydration_context/browser", "leptos_dom/hydration"]
csr       = ["leptos_macro/csr", "reactive_graph/effects", "getrandom?/wasm_js"]
hydrate   = ["leptos_macro/hydrate", "hydration", "tachys/hydrate", "reactive_graph/effects", "getrandom?/wasm_js"]
ssr       = ["leptos_macro/ssr", "leptos_server/ssr", "server_fn/ssr", "hydration", "tachys/ssr"]
```
`csr` is the **only** mode feature that pulls in nothing server-side. `ssr` and `hydrate` both pull `hydration`; `ssr` additionally activates `server_fn/ssr`. **There is no `default` feature** — you must pick one explicitly.

From [leptos/src/lib.rs](https://github.com/leptos-rs/leptos/blob/main/leptos/src/lib.rs) (lines 84–107):
> - **`csr`** Client-side rendering: Generate DOM nodes in the browser.
> - **`ssr`** Server-side rendering: Generate an HTML string (typically on the server).
> - **`hydrate`** Hydration: use this to add interactivity to an SSRed Leptos app.
>
> **Important Note:** You must enable one of `csr`, `hydrate`, or `ssr` to tell Leptos which mode your app is operating in. You should only enable one of these per build target…

**0.9 parity:** [v0.9.0-beta leptos/Cargo.toml](https://github.com/leptos-rs/leptos/blob/v0.9.0-beta/leptos/Cargo.toml) has an identical feature set plus `lazy = ["tachys/lazy"]`. `csr` is unchanged — **no CSR-relevant break in 0.9.**

### 7.2 `leptos_router` needs no server and no feature flag

From [router/Cargo.toml](https://github.com/leptos-rs/leptos/blob/main/router/Cargo.toml):
```toml
[features]
tracing = ["dep:tracing"]
ssr = ["dep:percent-encoding"]
nightly = []
```
**There is no `csr` feature on `leptos_router` — CSR is the default, ungated build**, and `ssr` is purely additive. `generate_route_list` lives in `leptos_axum`/`leptos_actix`, not in `leptos_router`, so **no SSR-side route generation is required**.

Version note: `leptos_router` is **0.8.15 (2026-07-21)** — *not* lockstep with `leptos` 0.8.20 ([crates.io](https://crates.io/api/v1/crates/leptos_router)).

History mechanism: `router/src/location/` contains exactly three files — `history.rs`, `server.rs`, `mod.rs`. The client provider `BrowserUrl` reads `window().location()` and navigates via `push_state_with_url` / `replace_state_with_url`, i.e. the **HTML5 History API** ([location/history.rs](https://github.com/leptos-rs/leptos/blob/main/router/src/location/history.rs)); docs.rs confirms `BrowserUrl` is the sole client implementor of [`LocationProvider`](https://docs.rs/leptos_router/0.8.15/leptos_router/location/trait.LocationProvider.html). **No hash-based provider exists** — see §4.1. The only `hash` handling in `BrowserUrl` is `scroll_to_el()`, which is anchor behaviour, not routing.

**Corollary for Tauri:** a `file://`-served build could not use pushState routing; the Tauri app must be served over `tauri://localhost` (which it is by default).

### 7.3 `Resource` vs `LocalResource` — the one real CSR gotcha

`Resource`'s serialization bounds are **not `cfg`-gated** and apply in a pure CSR build. From [leptos_server/src/resource.rs](https://github.com/leptos-rs/leptos/blob/main/leptos_server/src/resource.rs) (lines 457–486), the impl block providing `new`:
```rust
impl<T> ArcResource<T, JsonSerdeCodec>
where
    JsonSerdeCodec: Encoder<T> + Decoder<T>,
    …
{
    pub fn new<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
```
`JsonSerdeCodec: Encoder<T> + Decoder<T>` resolves to `T: Serialize + DeserializeOwned` via `codee`. The **`Fut: Send` bound is the real pain in CSR**, since most browser APIs (`web-sys`, `gloo-net`, `reqwasm`) produce `!Send` futures. `Resource` still *functions* — `Owner::current_shared_context()` returns `None` in CSR so the fetcher just runs client-side, and the serialization write path is `#[cfg(feature = "ssr")]`-gated (resource.rs:334) — you simply pay bounds you get nothing for.

**`LocalResource` drops every one of them.** From [leptos_server/src/local_resource.rs](https://github.com/leptos-rs/leptos/blob/main/leptos_server/src/local_resource.rs):
```rust
pub fn new<Fut>(fetcher: impl Fn() -> Fut + 'static) -> Self
where
    T: 'static,
    Fut: Future<Output = T> + 'static,
```
No `Serialize`, no `Deserialize`, no `Send`, no `Sync`. ([docs.rs](https://docs.rs/leptos/0.8.20/leptos/prelude/struct.LocalResource.html))

The book states the rule directly ([async/resources](https://book.leptos.dev/async/10_resources.html)):
> Resources come in two primary flavors: `Resource` and `LocalResource`. If you're using server-side rendering …, you should default to using `Resource`. If you're using client-side rendering with a `!Send` API (like many of the browser APIs) … then you should use `LocalResource`.

**Rule for this project: use `LocalResource` everywhere.** `Resource`'s reason to exist is serializing a value from server to client, which is inert with no server.

### 7.4 `<Suspense/>` and `<Transition/>` are not SSR-entangled

`leptos/src/suspense_component.rs` contains **no `csr`/`ssr`/`hydrate` cfg gating at all** (only a `#[cfg(feature = "nonce")]` block); `leptos/src/transition.rs` has none. Proof by maintained example: [examples/fetch/src/lib.rs](https://github.com/leptos-rs/leptos/blob/main/examples/fetch/src/lib.rs), from a crate declared `features = ["csr", "tracing"]`, combines `LocalResource::new(...)` + `<Transition fallback=…>` + `<ErrorBoundary>` + `Suspend::new(async move { … })` with `reqwasm` doing a real network fetch. That is exactly the CSR-only async stack, kept in-repo.

### 7.5 Is `#[server]` optional? Yes to *use*; no to *compile*

From [leptos/Cargo.toml](https://github.com/leptos-rs/leptos/blob/main/leptos/Cargo.toml) `[dependencies]`:
```toml
server_fn = { workspace = true, features = ["form-redirects", "browser"] }
```
**No `optional = true`** — compare `leptos-spin-macro`, `rand`, `getrandom`, `tracing`, `subsecond`, which *are* optional. And `leptos/src/lib.rs:254` does an unconditional `pub use server_fn;`, with the prelude re-exporting `server_fn::{self, error::{FromServerFnError, ServerFnError, ServerFnErrorErr}}`.

So `server_fn` (plus its `browser` client feature) is **always compiled**, even in a CSR-only build, and there is no feature flag to remove it. Nothing breaks if you never write `#[server]` — it is dead weight in the dependency graph and build time, nothing more. None of the CSR examples (`counter`, `router`, `todomvc`, `fetch`) use `#[server]`.

**Unverified:** I did not measure the wasm-size or compile-time cost that the non-optional `server_fn` imposes on a CSR-only build.

### 7.6 Official CSR examples (all trunk-built, all in-repo)

| Example | Manifest |
|---|---|
| [`counter`](https://github.com/leptos-rs/leptos/tree/main/examples/counter) | `leptos = { features = ["csr"] }` |
| [`router`](https://github.com/leptos-rs/leptos/tree/main/examples/router) | `["csr", "tracing"]` + `leptos_router` — no server crate |
| [`todomvc`](https://github.com/leptos-rs/leptos/tree/main/examples/todomvc) | `["csr"]`, `web-sys` `Storage` |
| [`fetch`](https://github.com/leptos-rs/leptos/tree/main/examples/fetch) | `["csr", "tracing"]`, `LocalResource` + `<Transition/>` |
| [`tailwind_csr`](https://github.com/leptos-rs/leptos/tree/main/examples/tailwind_csr) | — |
| [`todo_app_sqlite_csr`](https://github.com/leptos-rs/leptos/tree/main/examples/todo_app_sqlite_csr) | — |

[examples/README.md](https://github.com/leptos-rs/leptos/blob/main/examples/README.md): "Most of the examples use either `trunk` (a simple build system and dev server for client-side-rendered apps) or `cargo-leptos`…"

Note `todomvc` uses `web-sys` `Storage` — an in-repo precedent for browser-local persistence in a CSR Leptos app.

### 7.7 The book explicitly blesses CSR-only as a terminal state

Chapter 13, ["Client-Side Rendering: Wrapping Up"](https://book.leptos.dev/csr_wrapping_up.html) — note the book lives in [`leptos-rs/book`](https://github.com/leptos-rs/book), the main repo's copy is a redirect stub:

> When the JS and WASM have loaded, Leptos will render your app into the `<body>`. This means that nothing appears on the screen until JS/WASM have loaded and run. This has some drawbacks:
> 1. It increases load time, as your user's screen is blank until additional resources have been downloaded.
> 2. It's bad for SEO, as load times are longer and the HTML you serve has no meaningful content.
> 3. It's broken for users for whom JS/WASM don't load for some reason…
>
> **However, depending on the requirements of your project, you may be fine with these limitations.**
>
> **If you just want to deploy your Client-Side Rendered website, skip ahead to the chapter on ["Deployment"]** — there, you'll find directions on how best to deploy your Leptos CSR site.

Chapter 14, ["Part 2: Server Side Rendering"](https://book.leptos.dev/ssr/), in full:
> As you read in the last chapter, there are some limitations to using client-side rendered web applications. This second part of the book will discuss how to use server-side rendering to overcome these limitations and get the best performance and SEO out of your Leptos apps.

It imposes nothing on CSR-only beyond ch. 13. And from [Getting Started](https://book.leptos.dev/getting_started/):
> **Client-side rendering (CSR) with Trunk** - a great option if you just want to make a snappy website with Leptos, **or work with a pre-existing server or API**. … The advantages of Leptos CSR include faster build times and a quicker iterative development cycle, as well as a simpler mental model and more options for deploying your app. … Also note that, under the hood, an auto-generated snippet of JS is used to load the Leptos WASM bundle, so **JS *must* be enabled** on the client device for your CSR app to display properly.

Of the three stated CSR drawbacks, **none binds a local-first offline flashcard app**: load time is local, SEO is irrelevant for a Tauri shell, and the no-JS case does not arise inside a webview.

### 7.8 Tauri's own position

> Use SSG, Tauri doesn't officially support server based solutions.

([v2.tauri.app/start/frontend/leptos/](https://v2.tauri.app/start/frontend/leptos/) and the [Trunk frontend guide](https://v2.tauri.app/start/frontend/trunk/))

The two positions agree: **CSR-only is the supported, intended configuration for Leptos-inside-Tauri**, and it is exactly what a no-server-of-our-own project wants.

---

## Cross-cutting: unverified items

Collected for convenience — every one is also flagged inline above.

- Whether any of the volunteers on [leptos#4707](https://github.com/leptos-rs/leptos/issues/4707) actually received commit rights; no handover announcement found (§1.1.1).
- Any formal Leptos API-stability policy document or 1.0 timeline — none exists (§1.1.2).
- Any formal Tauri semver/stability policy document beyond the 2.0 release blog; [#12465](https://github.com/tauri-apps/tauri/issues/12465) notes CI does not machine-check semver (§1.2.1).
- Exact NDK and JDK version numbers on the [prerequisites page](https://v2.tauri.app/start/prerequisites/) — both deferred to "whatever Android Studio installs" (§2.2). NDK ≥ 28 appears only in the [mobile-plugin guide](https://v2.tauri.app/develop/plugins/develop-mobile/) (§2.3).
- Any documented minimum Chromium/WebView version for Tauri on Android (§2.4).
- Whether 16 KB page alignment can still fail via a Rust dependency's prebuilt `.so`; the Gradle-overrides-rustflags claim in [#14895](https://github.com/tauri-apps/tauri/issues/14895) was never rebutted (§2.3).
- Whether the `leptos_0.9` branch changes the `LocationProvider` design to permit hash routing; [#2184](https://github.com/leptos-rs/leptos/issues/2184) is not marked fixed (§4.1).
- Which `BaseDirectory` `tauri-plugin-store` uses by default — undocumented (§5.5).
- Published Android rebuild-time figures — none exist anywhere in Tauri's docs (§6.4).
- Whether Trunk/Leptos HMR works inside the Android webview — no Trunk-specific issue exists in the tracker, which is ambiguous evidence (§6.4).
- The wasm-size / compile-time cost of the non-optional `server_fn` dependency in a CSR-only build (§7.5).
- Contradiction, unresolved: `tauri-plugin-sql`'s README says **iOS: x** while its own `Cargo.toml` metadata says `ios = { level = "full" }` (§5.4). Irrelevant here (no iOS) but a doc-drift signal.

