# Research: Rust multi-platform client stacks

**Ticket:** [#3 Research: Rust multi-platform client stacks](https://github.com/amin-bf/cairn/issues/3) ·
**Map:** [#1 Map: local-first Leitner app spec](https://github.com/amin-bf/cairn/issues/1) ·
**Research date:** 2026-07-26 · **Blocks:** [#8 Prototype: pick the client stack](https://github.com/amin-bf/cairn/issues/8)

This document **gathers facts and does not choose**. The choice belongs to #8.

Every version number and capability claim here was checked against a primary source on 2026-07-26 —
crates.io / the sparse index, docs.rs, project GitHub repos (source, releases, CHANGELOGs, issue
trackers), official docs sites, MDN browser-compat-data, and the WHATWG Storage spec. Secondary
write-ups were used only as pointers.

**Full evidence, with a source link on every claim, lives in the appendices:**

| Appendix | Covers |
|---|---|
| [`dioxus.md`](./dioxus.md) | Dioxus: health, Android, rendering, web, storage, dev loop, server story |
| [`leptos-tauri.md`](./leptos-tauri.md) | Leptos + Tauri 2: the same seven areas, plus the two-process architecture |
| [`storage-and-contenders.md`](./storage-and-contenders.md) | Cross-platform storage in depth; egui, Slint, Makepad, Iced, Tauri+other frontends |

Each appendix ends with a register of what its author **could not verify**. Those registers are
part of the finding — read them before treating anything here as settled.

---

## 0. A correction, resolved against the sources

The three investigations disagreed on one load-bearing point, so it was re-checked directly.

The Leptos+Tauri appendix states that `sqlite-wasm-rs` is *"not compatible with rusqlite or diesel"*
and concludes there is **no shared query code** between native and web. **That is wrong as of
today.** Verified directly:

- `rusqlite`'s own `Cargo.toml` on `master` declares `default = ["cache", "ffi-sqlite-wasm-rs"]`,
  `ffi-sqlite-wasm-rs = ["dep:sqlite-wasm-rs"]`, and gates its dependencies by target:
  `libsqlite3-sys` under `cfg(not(all(target_family = "wasm", target_os = "unknown")))`, and
  `sqlite-wasm-rs` under `cfg(all(target_family = "wasm", target_os = "unknown"))`.
- The current `sqlite-wasm-rs` README makes **no** incompatibility claim; it lists `rusqlite` and
  `diesel` under "Related Project".

The incompatibility statement appears to be stale. Treat §5.6 and §5.8 of the Leptos+Tauri appendix
as superseded by §3 of this document and by the storage appendix, which reached the correct
conclusion independently and cited rusqlite PR #1769 and issue #1828.

This matters because it changes the shape of the whole problem: SQL, schema, migrations and
row-mapping code **are** portable across all three targets. What is not portable is where the bytes
land — see §3.4.

---

## 1. Headline findings

The facts most likely to constrain the decision in #8, in rough order of weight.

1. **`rusqlite` is the only storage layer that compiles unchanged for desktop, Android and web.**
   Since 0.38.0 (2025-12-20, PR #1769) the default `ffi-sqlite-wasm-rs` feature swaps
   `libsqlite3-sys` for `sqlite-wasm-rs` on `wasm32-unknown-unknown`. Current: **rusqlite 0.40.1**,
   bundled SQLite **3.53.2**. This is stack-independent — it is true whichever UI framework wins.

2. **`sqlx` cannot follow, and this is now a settled upstream decision.** PR #3994
   ("SQLite-for-WASM support") was **closed unmerged on 2026-07-02**; the maintainer's reason:
   *"Because of the lack of threading, there's no getting around the blocking calls into SQLite which
   breaks the async API as it currently stands."* Any design putting SQLite on a background thread
   (`tokio-rusqlite`, sqlx's connection worker) does not port to web.

3. **Leptos is officially "lightly maintained."** On 2026-05-08 its creator and sole principal
   maintainer opened [leptos#4707](https://github.com/leptos-rs/leptos/issues/4707):
   *"Leptos is not abandoned but will be lightly maintained going forward. I consider it
   feature-complete and do not expect to do significant new development in the future."* Several
   people volunteered in the thread; **no handover was confirmed**. Stable is 0.8.20; a breaking
   0.9.0-beta is out with, in the maintainer's words, *"no urgency"*. Read this as *finished*, not
   *dead* — low churn risk, but low odds a bug you hit gets fixed.

4. **Tauri is the healthier project of the two by every activity measure** — 2.11.5, ~496 commits/yr,
   roughly monthly releases, multiple maintainers, post-1.0 with a stated intent not to break within
   2.x (plugins explicitly excepted). Dioxus sits between: 0.7.9 stable, 341 commits/yr but
   front-loaded, and **no stable release in the ~2.5 months** before the research date.

5. **Both webview stacks have a real Android floor well above their declared `minSdk`.**
   - *Dioxus*: an open `tao` bug (#3401) calls `getCurrentWindowMetrics`, which is API 30+, and
     crashes below it. Fixed upstream in **tao 0.35.2**, but `dioxus-desktop` 0.7.9 pins `tao ^0.34.0`
     — **the fix is in no 0.7.x release**; only 0.8-alpha has it. Effective floor: **Android 11**.
   - *Tauri*: config default is `minSdkVersion` 24, but Tauri's injected script uses ES6+, so on
     WebView 55 (Android 7.1) `window.__TAURI__` never appears and IPC is dead. Practical floor is
     "a modern Play-updated WebView", i.e. Android 9+ in practice.

6. **Tauri's Android *release* path is its sharpest edge, and it is not the runtime.** The generated
   template sets `isMinifyEnabled = true`, and the signed release APK crashes on launch
   ([#13379](https://github.com/tauri-apps/tauri/issues/13379),
   [#15337](https://github.com/tauri-apps/tauri/issues/15337) — both open). Separately
   [#14413](https://github.com/tauri-apps/tauri/issues/14413) (`priority: 1 high`) has `versionCode`
   silently falling back to `1`, which Play rejects on your *second* upload. **The default template
   does not ship a working release build.**

7. **Dioxus and Tauri are the same rendering bet.** Dioxus desktop *and* Android both go through
   wry/tao — confirmed from source, not assumed: `mobile = ["dep:dioxus-desktop"]` in
   `dioxus/Cargo.toml`, and `dx --platform android` expands to `--renderer webview`. Tauri is
   likewise a system-webview shell. The difference between the two stacks is **not** the renderer.

8. **Blitz — the native (non-webview) Dioxus renderer — is not shippable.** Its README:
   *"Blitz is currently in a pre-alpha state… we would not yet recommend building apps with it."*
   `dioxus-native` has ~21k lifetime downloads against `dioxus`'s 2M. Mobile support is roadmapped
   for 0.3-beta, ~Aug 2026.

9. **Web persistence forces a Web Worker, whichever stack you pick.** Full-durability OPFS storage
   needs `FileSystemSyncAccessHandle`, which MDN documents as *"exclusively available in Dedicated
   Web Workers, not on the main thread."* The IndexedDB alternative (`relaxed-idb`) runs anywhere but
   **explicitly gives up full durability**. This boundary is contagious across the whole web app.

10. **Iced is not viable for Android.** Android is absent from its own platform list, there is no
    Android feature flag or tooling, and its DOM runtime `iced_web` has been **archived since 2022**.
    Community winit+wgpu demos exist; that is an integration exercise, not a supported platform.

11. **Fullstack/SSR is genuinely optional in both stacks** — verified from Cargo feature graphs, not
    marketing. Dioxus: `fullstack` and `server` are disjoint from `desktop`/`mobile`/`web`/`router`.
    Leptos: `csr` is the only mode feature pulling in nothing server-side, and the official
    Tauri+Leptos template is CSR-only. Neither loses coherence without a server.

---

## 2. Stack by stack

### 2.1 Dioxus

*Full detail: [`dioxus.md`](./dioxus.md)*

**Health.** Stable **0.7.9** (2026-05-08); newest publish is `0.8.0-alpha.0` (2026-05-19). Ten 0.7.x
patches in ~6 months, then a gap — no stable release in the 2.5 months to the research date, and no
0.8 beta/rc. 341 commits/52wk but trailing weeks near zero; the repo did have commits on the research
date. Pre-1.0, so minor bumps are breaking; 0.8 moved the workspace to edition 2024. Backed by a
small funded team (FutureWei, Satellite.im, GitHub Accelerator).

**Android.** Marketed as first-class (*"Simply run `dx serve --platform android`"*). The tooling
genuinely is integrated — **`cargo-ndk` is not required**, `dx` reimplements its environment setup,
setting `CC`/`CFLAGS`/`AR`/sysroot/linker per triple. Real requirements, read from `dx` source rather
than docs: **JDK 17**, Gradle **9.1.0**, AGP **8.7.0**, Kotlin **2.0.20**, `minSdk` 24 /
`target`+`compile` 34.

Sharp edges: the API-30 crash above; a **config trap** where `[android] min_sdk` (24, for Gradle) and
`[application] android_min_sdk_version` (28, for the NDK clang target) are different keys with
different defaults; `dx` autodetects **JDK 11** on Linux, which won't satisfy `jvmTarget = 17`; and
32-bit Android was **silently dropped in 0.7.4** (#5637) while the docs still tell you to
`rustup target add armv7-linux-androideabi`. 42 open Android issues.

**Rendering.** Webview everywhere — WebView2 / WebKitGTK / WKWebView / Android System WebView. One
CSS model across all three targets, but three different engines whose CSS behaviour differs.

**Web.** `wasm32-unknown-unknown` + `wasm-bindgen` into the real DOM via `web-sys` — no webview. CSR
by default; hydration only arrives via `fullstack`. A no-server build produces a static `public/`
directory deployable anywhere. 0.7 added route-level wasm code splitting (`#[wasm_split(...)]`).
The docs' *"70kb vs React's 65kb"* claim carries no version, compression basis or methodology —
treat it as marketing.

**Storage.** The best-evidenced result of the three reports. Because desktop and mobile Rust code
runs *natively* (it controls a webview, it does not run inside one), `rusqlite` works directly — a
maintainer says so explicitly in discussion #3898. `dx` supplies exactly the environment `cc`-rs
needs, so bundled SQLite should cross-compile for free (**unverified — not actually built**). The
Android data-dir path has **no Dioxus API**; you write JNI yourself, and the cleanest community
pattern is `wry::prelude::dispatch` → `getFilesDir`. `dioxus-sdk-storage` is useless here: on web it
is `localStorage`-only.

**Dev loop.** `dx serve` per platform; Android runs `adb root` → `adb reverse` → `adb install -r` →
push a `.env` → `am start`, and works on physical devices. Three hot-reload tiers: RSX (always on),
assets, and experimental Rust hot-patching via `--hotpatch` — whose source comment warns it *"may
lead to unexpected segfaults."* **The limitation that shapes project layout:** Subsecond only tracks
the *tip* crate. Split a `core`/`domain` crate out of the workspace and you lose Rust hot-patching
for it, though RSX hot-reload still works across the workspace.

**Docs health is itself a risk.** ~55 zero-byte stub files in the 0.7 docs tree — including *every*
deploy guide, `tools/serve.md`, `tools/android.md`, and all `guides/apis/*`. Agents implementing
against this stack will be reading `dx` source, not documentation.

### 2.2 Leptos + Tauri 2

*Full detail: [`leptos-tauri.md`](./leptos-tauri.md)*

**Health.** Split verdict — see headline findings 3 and 4. Leptos 0.8.20, "lightly maintained",
pre-1.0, no stability policy, breaking 0.9 in beta. Tauri 2.11.5, monthly cadence, multi-maintainer,
post-1.0. The two projects are independent; there is an official Tauri template for Leptos.
Note Tauri's published MSRV is 1.77.2 but `dev` already declares 1.90 — plan on a modern toolchain.

**Android.** Tauri never calls it first-class; its own 2.0 post says *"We are not completely happy
about the developer experience at the moment."* 82 open `platform: Android` issues. Concrete
toolchain constants, from CLI source: NDK pin `29.0.13846066`, SDK 36, Gradle 8.14.3, AGP 8.11.0 —
but the CLI actually picks the **lexicographically highest installed NDK**, not the pin. JDK 25/26
breaks the build cryptically and the warning PR (#15780) was still unmerged on the research date;
use Android Studio's bundled JBR. 16 KB page alignment (mandatory for Play uploads since 2025-11-01)
works out of the box **with NDK r28+**.

Of the official plugins, `sql` and `store` are both **full** on Android; `updater`, `single-instance`
and `window-state` have **no** Android support, and `fs` is *"restricted to Application folder by
default."* For an offline app the absences are survivable — you ship through Play.

Beyond the release-path bugs in headline finding 6, three runtime issues matter for *this* app: the
hardware **back button exits the app** ([#14406](https://github.com/tauri-apps/tauri/issues/14406)),
soft-keyboard/viewport behaviour is inconsistent (#13479, #7868 — relevant to any typed-answer
mode), and **`bundle.resources` does not work on Android**
([#8911](https://github.com/tauri-apps/tauri/issues/8911), 35 comments) — which directly blocks
shipping a prepopulated seed deck.

**Architecture — the defining difference from Dioxus.** A Tauri+Leptos app is **two Rust compilation
products**: the Leptos frontend compiled to `wasm32-unknown-unknown` running inside the webview, and
the Tauri core compiled natively. They communicate over a **JSON-RPC-like `invoke` boundary** — *"all
arguments and return data must be serializable to JSON."* The official template is a two-crate
workspace and proves the split. **There is no supported way to run the same Rust code in both as one
compilation unit.** What you can do is factor pure domain logic into a third portable crate that both
depend on; anything touching the filesystem or SQLite lives natively and is reached only across JSON
IPC.

**The plain-web target.** Two traps:
- **`leptos_router` has no hash routing** ([#2184](https://github.com/leptos-rs/leptos/issues/2184),
  open since 2024, two contributors tried and gave up). On a passive static host a direct load of
  `/review/deck-1` 404s. Workarounds are host rewrite rules or copying `index.html` to `404.html`.
  Under Tauri the app is served from a custom protocol origin, so **this bites the web target only**
  and is easy to miss until deploy.
- **Calling `invoke` outside Tauri throws *uncatchably*.** All three Rust binding paths raise a raw
  JS `TypeError` that Rust cannot turn into a `Result` — wasm-bindgen without `catch` skips
  destructors and the exception escapes. Gate on **`window.isTauri`** (always injected) — **never**
  `window.__TAURI__`, which defaults off — or compile the calls out with a Cargo feature.

**Storage.** In the core, `rusqlite`/`sqlx` work directly on desktop and Android; bundling SQLite is
required on Android and `sqlx`'s `sqlite` feature already implies it. On Android, `app_data_dir()`,
`app_local_data_dir()` and `app_config_dir()` **all** resolve to `Context.dataDir` — app-private
internal storage, no permission needed, removed on uninstall. `tauri-plugin-sql` puts `sqlite:x.db`
in **`app_config_dir`** (source-verified; not documented). That plugin is frontend-facing — you send
SQL strings across IPC and get JSON rows back — so a Rust-core-centric app does not need it.
`tauri-plugin-store` is a JSON blob per file: fine for settings, not for a review log.

**One escape hatch worth weighing in #8:** if *all* persistence goes through browser storage on every
target (OPFS/IndexedDB via `web-sys`), Tauri becomes a pure window shell, `invoke` disappears, and
**one wasm binary serves all three targets with nothing to abstract**. The price is giving up native
SQLite and filesystem-level backup. Caveat: the webview origin is platform-dependent, and flipping
`useHttpsScheme` after release **orphans all existing IndexedDB/localStorage data**.

**Dev loop.** `tauri dev` runs `trunk serve`; a change to `src-tauri` is a **full cargo rebuild and
app restart**, not a hot reload. On Android, an emulator works over `adb reverse` + localhost, but a
**physical device** needs `trunk serve` bound to `0.0.0.0`/`TAURI_DEV_HOST` — documented for iOS
only. Pin `tauri-cli ≥ 2.11.3` to avoid a doubled Rust compile per Android deploy. Trunk's last
stable is **0.21.14 (2025-05-08)**, ~14 months old, and the whole Leptos CSR dev loop rests on it.
The Tauri Leptos guide still says *"This guide applies to Leptos version 0.6."*

**Server story.** `csr` is a first-class mode pulling in zero SSR machinery; `leptos_router` needs no
feature flag; `LocalResource` + `<Suspense>`/`<Transition>` is the CSR-native async stack — use it,
not `Resource`, which carries `Serialize + Send` bounds that buy nothing without a server. `#[server]`
is optional to *use*, though `server_fn` compiles unconditionally (cost unmeasured). The Leptos book
explicitly blesses CSR-only as a terminal state, and Tauri's own guidance is *"Use SSG, Tauri doesn't
officially support server based solutions."* The two positions agree.

### 2.3 Other contenders

*Full detail: [`storage-and-contenders.md`](./storage-and-contenders.md)*

An architectural point that frames the whole group: **Tauri is not a web deployment target.** It
*"acts as a static web host"* and produces desktop and mobile binaries only. A "Tauri + X" stack gets
desktop+Android from Tauri and the web target by deploying the same wasm frontend separately as a
static SPA — two pipelines sharing one frontend crate. egui/Slint/Makepad each render one canvas
everywhere from a single toolchain.

- **egui / eframe 0.35.0** — Android support is real (merged 2024-12, via
  `android-game-activity`/`android-native-activity` features), but the PR author's own words:
  *"the development environment setup is completely awful for Android."* Status-bar inset handling is
  a known gap. Its README is unusually candid about the web: you cannot search the page, on-screen
  keyboard handling is faked with invisible DOM elements and *"doesn't always work"*, *"mobile text
  editing is not as good as for a normal web app"*, and AccessKit has no web backend. Its built-in
  web persistence is **`localStorage`** (~5 MiB) — unusable for an event log. Adoption is the
  strongest of the non-webview options (~4.5M downloads/90d).
- **Slint 1.17.1** — Android is supported but Rust-only and routed through **`cargo-apk`, last
  published 2023-11-30** — a stale tool in an otherwise current stack. On web, the vendor says it
  plainly: *"running Slint in the browser is currently not recommended for building general-purpose
  web applications."* Licensing is the distinguishing factor: the royalty-free option **requires
  displaying a Slint badge or the `AboutSlint` widget**; otherwise GPLv3 or paid. That is a product
  decision, not just a legal one.
- **Makepad** — genuinely covers all three targets with in-tree tooling (`cargo makepad android run`,
  `cargo makepad wasm run`), which is rare. But `makepad-widgets` 1.0.0 has **~2,153 downloads/90d**
  against egui's ~4.5M, the only GitHub release tag is `pre-alpha` from 2018, and the default branch
  is `dev`. High ecosystem risk: you would be the one filing the bugs.
- **Iced 0.14.0** — see headline finding 10. The clearest exclusion for a desktop+web+**Android**
  project.
- **Tauri 2 + a non-Leptos frontend** — `create-tauri-app` ships Vanilla, Yew, Leptos and Sycamore
  templates. Yew 0.23.0 (~320k/90d) recently closed a two-year release gap; Sycamore 0.9.2 is thinly
  used (~21.5k/90d vs Leptos's ~1.15M); plain HTML/TS via Vite is the best-documented path and
  carries zero Rust-frontend risk. **This is the option that most directly answers the Leptos
  maintenance finding** — it keeps everything in headline finding 4 and drops everything in
  finding 3.

---

## 3. Cross-cutting: storage

This is the crux, and it is largely **independent of the UI framework**. Whatever #8 picks, these
constraints hold.

### 3.1 Desktop

`rusqlite` **0.40.1** (bundled SQLite 3.53.2, `cc`-compiled from embedded source; `bundled` is *not*
a default feature). `sqlx` **0.9.0** — note its repo moved from `launchbadge` to `transact-rs` and its
MSRV is now 1.94.0. `diesel` **2.3.11** does the same wasm trick as rusqlite but pins
`libsqlite3-sys < 0.38.0`, one minor behind rusqlite 0.40.1 — mixing the two in one graph would
conflict.

Pure-Rust KV alternatives, since the data model is a log: **`redb` 4.1.0** is stable, maintained
(6 open issues), **not mmap-based**, and exposes a five-method synchronous `StorageBackend` trait —
the key architectural affordance. **`fjall` 3.1.8** is LSM-shaped, a natural structural fit for an
append-only log. **`sled` is effectively dead** — no release since 2024-10-11, 171 open issues,
self-described beta with a promised breaking on-disk format change. For an app whose only copy of
user data is local, that rules it out.

Rolling your own append+fsync framing over `postcard`/JSONL is more defensible than adopting a WAL
crate: `okaywal` last shipped 2023-11-26 and `simple-wal` in 2020.

### 3.2 Android

**Bundling SQLite is the only defensible option** — SQLite is not on the NDK's stable-API list, and
since Android N the linker namespace prevents apps loading non-NDK system libraries. Version and
compile options would be device- and OEM-dependent anyway.

Bundled SQLite does cross-compile — proven in shipping software by `tauri-plugin-sql` 2.4.0, whose
platform matrix lists **Android ✓**. Two documented failure modes: NDK clang naming (what `cargo-ndk`
exists to fix) and missing 128-bit float builtins `__extenddftf2`/`__lttf2`/`__trunctfdf2`, for which
cargo-ndk documents `--link-builtins`. Note the reported failure is on the **emulator** triple
`x86_64-linux-android`, so budget CI time for that specifically.

**The pure-Rust KV stores need no NDK C toolchain at all** — their single biggest Android advantage.
Neither redb nor fjall uses mmap, so the classic Android mmap hazards don't apply in app-private
storage. One sharp finding: **fjall fails on Android with `"try_lock() not supported"`** — a Rust std
gap (`target_os = "android"` missing from the `flock` cfg list), fixed by rust-lang/rust PR #157038,
milestone **1.98.0** — which is current stable as of 2026-07-16. **If you use fjall on Android, pin
MSRV ≥ 1.98.0.**

### 3.3 Web — the hard case

`std::fs` always errors on `wasm32-unknown-unknown`. Everything below replaces it.

**OPFS browser support** (from MDN browser-compat-data):

| API | Chrome | **Chrome Android** | Firefox | Safari |
|---|---|---|---|---|
| `getDirectory()` (OPFS root) | 86 | **109** | 111 | 15.2 |
| `createSyncAccessHandle()` | 102 | **109** | 111 | 15.2 |
| sync (non-promise) read/write/flush variants | **108** | **109** | 111 | **16.4** |

**Chrome Android is the lagging surface** — 109, not desktop's 102. And the *synchronous* variants,
the ones an SQLite VFS actually wants, are the later arrivals.

**The `sqlite-wasm-rs` VFS matrix** (VFSes live in the companion crate `sqlite-wasm-vfs` 0.2.0):

| VFS | Storage | Contexts | Multiple connections | Durability |
|---|---|---|---|---|
| `memory` (**default**) | RAM | all | ✗ | full, in-session only |
| `sahpool` | **OPFS** | **Dedicated Worker only** | ✗ | full |
| `relaxed-idb` | **IndexedDB** | all | ✗ | **relaxed only** |

None require COOP/COEP headers — good for plain static hosting, and a real advantage over SQLite's
own official `sqlite-wasm` OPFS VFS, which needs cross-origin isolation and `SharedArrayBuffer`.
`sqlite-wasm-rs` is **not thread-safe** (`-DSQLITE_THREADSAFE=0`, and `JsValue` isn't `Send`).
**No VFS supports multiple connections**, so multi-tab needs your own coordination.

For IndexedDB directly: **`idb` 0.6.5** is the author's recommended crate;
**`indexed_db_futures` 0.6.4** is the most used; **`rexie` is deprecated by its own author** in
favour of `idb`. **wa-sqlite is JavaScript** with no first-party Rust bindings. **Turso** is
promising but not reachable from Rust-on-wasm (issue #5049, "Backlog" milestone, pointer-width
compile errors) and has compatibility gaps that matter here — no `WITH RECURSIVE`, `WITHOUT ROWID`
tables effectively insert-only. Revisit post-1.0.

**Durability is not guaranteed.** Per the WHATWG Storage Standard, default buckets are
`"best-effort"` — *"Data can be cleared by the user agent without user involvement."*
`navigator.storage.persist()` upgrades to `"persistent"`, but MDN warns *"There's no guarantee of
persistence."* Chrome grants silently on heuristics (site engagement, **installed/bookmarked**,
notification permission); Firefox prompts. Eviction is **LRU by origin and deletes all of an
origin's data together**. `persist()` is also **not available in Web Workers**, so you must call it
from the main thread even though OPFS I/O lives in a worker.

**Unverified across two agents:** whether Android Chrome evicts more aggressively than desktop — BCD
records Android as mirroring desktop, and no primary source says otherwise. The practical difference
is that a phone is far likelier to hit the storage-pressure condition at all. **Operating assumption:
the web build's data can vanish; do not treat the browser as the system of record.**

### 3.4 The one-codebase question

**Answer: no. You must abstract behind a trait with at least two backends — but the seam is much
narrower than it was 18 months ago.**

What genuinely is portable: `rusqlite`'s `Connection` API compiles for all three targets (§0), so
**SQL, schema, migrations and row mapping are shared code**. redb's `Database` API is likewise
target-independent.

Four constraints force the split anyway, each independently sufficient:

1. **Storage *location* is not abstracted.** rusqlite-on-wasm defaults to the **in-memory** VFS.
   Persistence means separately depending on `sqlite-wasm-vfs` and registering a VFS at startup — a
   wasm-only code path with no native analogue. redb-on-wasm needs a hand-written `StorageBackend`;
   **redb ships no browser backend**.
2. **The Worker requirement is contagious.** On web, storage sits behind an async, fallible
   message-passing boundary; on native it is an in-process call. That shape difference is exactly
   what the trait has to paper over.
3. **Threading.** wasm SQLite is single-threaded and not thread-safe. Any native design that puts
   SQLite on a background thread does not port.
4. **Multi-tab/multi-connection semantics differ** — normal WAL behaviour natively, single connection
   on web.

**Where to put the seam** — the choice that most shapes the codebase, and a real input to #8:

- **Low seam (bytes).** `trait LogStore { fn append(&self, …); fn read_from(&self, seq: u64); }`.
  Native = append to a file, redb or fjall; web = OPFS via worker, or IndexedDB. Maximises portable
  code (all projection logic is pure Rust over the log), shrinks the platform-specific surface to a
  handful of methods, and is the **only** option that keeps redb/fjall on the table.
- **High seam (SQL).** rusqlite everywhere; the seam is just "how do I open the connection."
  Maximises reuse of SQL and migrations, but forces the *entire* storage-touching call graph on web
  behind a worker RPC, since the connection isn't `Send` and the VFS is worker-only.

No crate found presents one API over both native SQLite and wasm SQLite/OPFS *including* persistence
configuration. The `opfs` crate attempts the unified-filesystem framing but is fully async on both
sides — unusable as a synchronous SQLite VFS or redb backend — and at 0.2.0/27 stars is not a
load-bearing dependency.

---

## 4. Comparison tables

### Storage by target

| Option | Desktop | Android | Browser (wasm32-unknown-unknown) | Needs C toolchain? |
|---|---|---|---|---|
| `rusqlite` 0.40.1 | ✓ `bundled` | ✓ (`cargo ndk`, maybe `--link-builtins`) | ✓ compiles; **memory-only** unless you add `sqlite-wasm-vfs` + Worker | native only |
| `diesel` 2.3.11 | ✓ | ✓ | ✓ same mechanism | native only |
| `sqlx` 0.9.0 (sqlite) | ✓ | ✓ (proven by `tauri-plugin-sql`) | **✗ rejected upstream 2026-07-02** | yes |
| `redb` 4.1.0 | ✓ | ✓ (pure Rust, no mmap) | compiles; **you write the `StorageBackend`** | no |
| `fjall` 3.1.8 | ✓ | ✓ **only on Rust ≥ 1.98.0** | ✗ / unknown | no |
| `sled` | beta-quality | unknown | ✗ | no |
| `turso` 0.7.1 | ✓ | unknown | ✗ from Rust (backlog) | no |
| Plain append-only file | ✓ | ✓ | ✗ — needs OPFS | no |

### UI stacks

| Stack | Version | Android | Web | Rendering | Maintenance signal |
|---|---|---|---|---|---|
| **Dioxus** | 0.7.9 (2026-05-08) | ✓ integrated tooling; real floor **API 30** on 0.7.x | ✓ CSR wasm → real DOM | webview (wry/tao) all targets | funded team; no stable release in 2.5mo; docs full of stubs |
| **Leptos + Tauri 2** | leptos 0.8.20 / tauri 2.11.5 | ✓ most production-proven; **release path broken by default** | ✓ separate static SPA build | system webview | **Leptos "lightly maintained"**; Tauri healthy |
| Tauri 2 + Yew/Sycamore/TS | tauri 2.11.5 | ✓ same as above | ✓ same as above | system webview | Tauri healthy; frontend risk varies |
| egui / eframe | 0.35.0 (2026-06-25) | ✓ rough (insets, IME) | ✓ vendor-documented downsides | immediate-mode canvas | active, high adoption |
| Slint | 1.17.1 (2026-07-07) | ✓ via stale `cargo-apk` | ⚠ *"not recommended"* by vendor | WebGL canvas | active; **badge required** or paid |
| Makepad | 1.0.0 (2025-05-13) | ✓ in-tree tooling | ✓ in-tree tooling | GPU 2D/3D | **~2.1k dl/90d** — very low |
| Iced | 0.14.0 (2025-12-07) | **✗ unsupported** | `iced_web` **archived 2022** | wgpu / tiny-skia | active but self-described *"experimental"* |

---

## 5. What this research does *not* settle

Handing these to #8 explicitly, rather than letting them look answered.

1. **No measured build or iteration times exist for either stack, on any platform.** Neither project
   publishes them; Tauri's docs say only *"several minutes"* for a first build and *"much faster"*
   after. Android is the expensive case for both — every non-hot-patched change pays a Gradle
   assemble plus `adb install`. **This needs an empirical spike, and it is the single biggest
   unknown for day-to-day work.**
2. **Nobody actually compiled `rusqlite` with `bundled` for Android under either stack.** The
   environment is correct by construction and no issue reports the contrary, but it is unproven here.
   The same goes for the full web path — registering an OPFS VFS in a worker and persisting across
   reloads. Ticket [#7](https://github.com/amin-bf/cairn/issues/7) is the natural place to prove
   both.
3. **Whether Dioxus 0.8 stabilises, and whether the tao API-30 fix is backported to 0.7.x.** There is
   no public milestone with dates. Choosing Dioxus today means choosing 0.7.9-with-an-API-30-floor,
   or riding an alpha.
4. **Whether any Leptos co-maintainer actually received commit rights.** No handover was announced.
   This is the fact that would most change the weight of headline finding 3.
5. **Whether Trunk/Leptos hot-reload works inside the Android webview.** No issue exists either way —
   which could mean it works, or that nobody is doing it. Framework-specific HMR-on-Android failures
   are open for Angular, Dioxus, Next and Nuxt.
6. **The seam decision itself** (§3.4) is a design choice, not a fact, and interacts with the deck
   export format ([#13](https://github.com/amin-bf/cairn/issues/13)) and the local store
   ([#12](https://github.com/amin-bf/cairn/issues/12)).

Each appendix carries its own register of unverified claims — 8 items for Dioxus, 13 for
Leptos+Tauri, 12 for storage/contenders. They are worth reading in full before #8 commits.

---

## 6. Method and provenance

Three independent agents worked in parallel against primary sources: one on Dioxus, one on
Leptos+Tauri 2, one on cross-platform storage plus alternative stacks. Splitting the work this way
produced a useful side effect — **the rusqlite-on-wasm finding was reached independently by two
agents from different starting points**, which is why the third agent's contradictory claim (§0) was
caught and re-verified rather than averaged in.

Scope note: the ticket asked where a stack's server-side story is irrelevant to us. It is irrelevant
in both cases, and removing it leaves both stacks coherent — see headline finding 11. That question
is closed.
