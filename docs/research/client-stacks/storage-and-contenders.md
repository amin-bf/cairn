# Storage and other client-stack contenders

Research date: **2026-07-26**. All version numbers verified against crates.io / GitHub / official docs on that date.
Consuming project: local-first, offline-by-default, no-server-of-our-own spaced-repetition app; **append-only event log** data model; targets **desktop + web + Android** (no iOS).

Reference points used throughout:
- Current stable Rust is **1.98.0**, channel manifest dated **2026-07-16** ([channel-rust-stable.toml](https://static.rust-lang.org/dist/channel-rust-stable.toml) reports `cargo 0.98.0`; beta manifest dated 2026-07-26 reports `cargo 0.99.0-beta.6`).
- Android target triples throughout: `aarch64-linux-android`, `armv7-linux-androideabi`, `x86_64-linux-android`, `i686-linux-android` ([cargo-ndk README](https://github.com/bbqsrc/cargo-ndk), [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)).
- Web target triple throughout: `wasm32-unknown-unknown`.

---

# Storage

## 1. Desktop

### 1.1 rusqlite

| Fact | Value | Source |
|---|---|---|
| Current version | **0.40.1** (published 2026-06-06) | [crates.io API](https://crates.io/api/v1/crates/rusqlite) |
| Companion sys crate | `libsqlite3-sys` **0.38.1** (2026-06-06) | [crates.io API](https://crates.io/api/v1/crates/libsqlite3-sys) |
| Bundled SQLite version | **3.53.2** | [rusqlite README](https://github.com/rusqlite/rusqlite#notes-on-building-rusqlite-and-libsqlite3-sys) |
| Minimum system SQLite (non-bundled) | 3.34.1 | [rusqlite README](https://github.com/rusqlite/rusqlite#supported-sqlite-versions) |
| Edition | 2024 | [Cargo.toml](https://raw.githubusercontent.com/rusqlite/rusqlite/master/Cargo.toml) |
| Maintenance badge | `maintenance = { status = "actively-developed" }` | [Cargo.toml](https://raw.githubusercontent.com/rusqlite/rusqlite/master/Cargo.toml) |
| Recent downloads | ~27.2M/90d | [crates.io API](https://crates.io/api/v1/crates/rusqlite) |

**What `bundled` compiles.** `bundled` maps to `libsqlite3-sys?/bundled` + `modern_sqlite` ([Cargo.toml features](https://raw.githubusercontent.com/rusqlite/rusqlite/master/Cargo.toml)). The README states: *"If you use the `bundled`, `bundled-sqlcipher`, or `bundled-sqlcipher-vendored-openssl` features, `libsqlite3-sys` will use the [cc](https://crates.io/crates/cc) crate to compile SQLite or SQLCipher from source and link against that. This source is embedded in the `libsqlite3-sys` crate and is currently SQLite 3.53.2 (as of `rusqlite` 0.40.1 / `libsqlite3-sys` 0.38.1)."* ([README](https://github.com/rusqlite/rusqlite#notes-on-building-rusqlite-and-libsqlite3-sys)). So `bundled` requires a working **C toolchain** on the build host for the target — this is the crux of the Android story (§2).

`bundled` is **not** a default feature. Defaults are `["cache", "ffi-sqlite-wasm-rs"]` ([crates.io version API for 0.40.1](https://crates.io/api/v1/crates/rusqlite/0.40.1)). The README explains why: *"it's not ideal for all scenarios and in particular, generic libraries built around `rusqlite` should probably not enable it, which is why it is not a default feature."* ([README](https://github.com/rusqlite/rusqlite#usage))

Build knobs relevant to constrained targets: the build script honours `SQLITE_MAX_VARIABLE_NUMBER`, `SQLITE_MAX_EXPR_DEPTH`, and a free-form `LIBSQLITE3_FLAGS` (e.g. `"-USQLITE_ALPHA -DSQLITE_BETA"`) ([README](https://github.com/rusqlite/rusqlite#notes-on-building-rusqlite-and-libsqlite3-sys)). Non-bundled builds respect `SQLITE3_LIB_DIR`, `SQLITE3_INCLUDE_DIR`, `SQLITE3_STATIC` (same section).

**Big finding for a three-target project:** rusqlite has had **first-class `wasm32-unknown-unknown` support since 0.38.0** (2025-12-20), via PR [#1769](https://github.com/rusqlite/rusqlite/pull/1769), which closed the long-standing issues #488 and #827 ([v0.38.0 release notes](https://github.com/rusqlite/rusqlite/releases)). The mechanism is the `ffi-sqlite-wasm-rs` feature, which is **on by default**: *"`ffi-sqlite-wasm-rs` switches to using the `sqlite-wasm-rs` crate (instead of `libsqlite3-sys`) on `wasm32-unknown-unknown` builds. This is enabled by default and can be opted out by setting `default-features = false`."* ([README](https://github.com/rusqlite/rusqlite#optional-features)). The PR author describes it as: *"The main diff is to replace `libsqlite3-sys` with `sqlite-wasm-rs` on the wasm platform, while keeping everything else unchanged."* ([PR #1769](https://github.com/rusqlite/rusqlite/pull/1769)). See §3.3 and §4 for what this does and does not buy you.

**Caveat on CI coverage:** rusqlite's only CI workflow (`.github/workflows/main.yml`) has a test matrix of `x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`, `x86_64-pc-windows-gnu` — **no Android and no wasm target is built or tested in CI** ([main.yml](https://raw.githubusercontent.com/rusqlite/rusqlite/master/.github/workflows/main.yml); it is also the only file in `.github/workflows`). Treat Android/wasm as supported-but-not-CI-gated.

**Async wrapper.** `tokio-rusqlite` **0.7.0** (2025-11-16) wraps rusqlite on a background thread ([crates.io](https://crates.io/crates/tokio-rusqlite)). Note this thread-based approach is exactly what breaks on wasm (see sqlx below).

### 1.2 sqlx (SQLite backend)

| Fact | Value | Source |
|---|---|---|
| Current version | **0.9.0** (published 2026-05-21; changelog dates it 2026-05-06) | [crates.io API](https://crates.io/api/v1/crates/sqlx), [CHANGELOG](https://raw.githubusercontent.com/launchbadge/sqlx/main/CHANGELOG.md) |
| MSRV | **1.94.0** | [crates.io version API](https://crates.io/api/v1/crates/sqlx/0.9.0), [CHANGELOG 0.9.0 "Breaking"](https://raw.githubusercontent.com/launchbadge/sqlx/main/CHANGELOG.md) |
| Governance | Repository transferred from `launchbadge` to **`transact-rs`** with the 0.9.0 release | [CHANGELOG 0.9.0 "New Github Organization"](https://raw.githubusercontent.com/launchbadge/sqlx/main/CHANGELOG.md) |
| Recent downloads | ~31.5M/90d | [crates.io API](https://crates.io/api/v1/crates/sqlx) |

The CHANGELOG is explicit about the ownership change: *"SQLx has not been owned or maintained by LaunchBadge, LLC. for a few years now, and has since been informally transferred to the collective ownership of its principal authors."* ([CHANGELOG](https://raw.githubusercontent.com/launchbadge/sqlx/main/CHANGELOG.md))

**Feature layout (0.9.0).** The `sqlite` feature now expands to `["sqlite-bundled", "sqlite-deserialize", "sqlite-load-extension", "sqlite-unlock-notify"]`, i.e. **bundled C SQLite by default**; `sqlite-unbundled` is the opt-out that links a system SQLite. `sqlite-bundled` pulls `sqlx-sqlite/bundled` ([crates.io version API for 0.9.0](https://crates.io/api/v1/crates/sqlx/0.9.0)). So sqlx-with-SQLite has the *same* C-toolchain cross-compilation requirement as rusqlite.

**Compile-time query checking.** `macros` feature → `derive` + `sqlx-macros/macros` + `sqlx-core/offline`; 0.9.0 adds per-crate `sqlx.toml` configuration (feature `sqlx-toml`, not enabled by default in the `sqlx` library) for renaming `DATABASE_URL`, global type overrides, relocating `_sqlx_migrations`, etc. ([CHANGELOG 0.9.0](https://raw.githubusercontent.com/launchbadge/sqlx/main/CHANGELOG.md)). Note 0.9.0 introduced a broad breaking change: *"all `query*()` functions now take `impl SqlSafeStr` which is only implemented for `&'static str` and `AssertSqlSafe`"* ([CHANGELOG, #3723](https://raw.githubusercontent.com/launchbadge/sqlx/main/CHANGELOG.md)).

**Decisive negative for this project: sqlx does not and will not support SQLite on wasm.** PR [#3994 "Feature: SQLite-for-WASM support"](https://github.com/transact-rs/sqlx/pull/3994) was **closed unmerged on 2026-07-02**. Collaborator `abonander`: *"I looked into the viability of supporting SQLite for WASM and I wasn't happy with what I saw. Because of the lack of threading, there's no getting around the blocking calls into SQLite which breaks the async API as it currently stands."* The underlying problem, per a tester on the same PR, is that sqlx's SQLite connection worker relies on a background thread, which crashes on wasm. The older tracking issue [#2985 "SQLite-for-WASM with SQLx"](https://github.com/transact-rs/sqlx/issues/2985) remains open and opens with *"To the best of my knowledge, SQLx currently does not support wasm32-unknown-unknown in any way."*

sqlx's wasm work is instead directed at **`wasm32-wasip2`** for Postgres/MySQL (PRs [#4058](https://github.com/transact-rs/sqlx/pull/4058), [#4309](https://github.com/transact-rs/sqlx/pull/4309), closed 2026-07-01), which is irrelevant to a browser deployment.

### 1.3 diesel

| Fact | Value | Source |
|---|---|---|
| Current version | **2.3.11** (2026-07-10) | [crates.io API](https://crates.io/api/v1/crates/diesel) |
| MSRV | **1.86.0** | [crates.io version API](https://crates.io/api/v1/crates/diesel/2.3.11) |

Included here because it is the *other* Rust SQL layer that solved wasm. Its `sqlite` feature is `["dep:libsqlite3-sys", "dep:sqlite-wasm-rs", ...]`, and the dependency table shows the target split explicitly:
- `libsqlite3-sys >=0.17.2, <0.38.0`, optional, `cfg(not(all(target_family = "wasm", target_os = "unknown")))`
- `sqlite-wasm-rs >=0.4.0, <0.6.0`, optional, `cfg(all(target_family = "wasm", target_os = "unknown"))`

([crates.io dependency API for diesel 2.3.11](https://crates.io/api/v1/crates/diesel/2.3.11/dependencies)). Same architecture as rusqlite 0.38+. Note diesel is pinned to `libsqlite3-sys < 0.38.0`, i.e. **one minor behind** what rusqlite 0.40.1 uses — mixing rusqlite and diesel in one dependency graph would currently duplicate/conflict on `libsqlite3-sys`.

### 1.4 Embedded key-value stores

#### redb

| Fact | Value | Source |
|---|---|---|
| Current version | **4.1.0** (2026-04-19); 3.1.3 also published 2026-04-02 | [crates.io API](https://crates.io/api/v1/crates/redb) |
| MSRV | **1.89** | [crates.io version API](https://crates.io/api/v1/crates/redb/4.1.0) |
| Status | *"Stable and maintained."* | [README](https://github.com/cberner/redb#status) |
| Repo activity | last commit 2026-07-17, **6 open issues**, 4,688 stars | [GitHub API](https://api.github.com/repos/cberner/redb) |
| Runtime deps | only `libc` (wasi-only), plus optional `chrono`/`log`/`uuid` | [crates.io dependency API](https://crates.io/api/v1/crates/redb/4.1.0/dependencies) |

*"redb is written in pure Rust and is loosely inspired by [lmdb](http://www.lmdb.tech/doc/). Data is stored in a collection of copy-on-write B+trees."* and *"The file format is stable, and a reasonable effort will be made to provide an upgrade path if there are any future changes to it."* ([README](https://github.com/cberner/redb)). Features: *"Zero-copy, thread-safe, `BTreeMap` based API / Fully ACID-compliant transactions / MVCC support for concurrent readers & writer, without blocking / Crash-safe by default / Savepoints and rollbacks"* ([README](https://github.com/cberner/redb#features)).

**Not mmap-based (important for Android/wasm).** The 4.1.0 dependency list contains **no `memmap`/`memmap2`** ([crates.io dependency API](https://crates.io/api/v1/crates/redb/4.1.0/dependencies)). Durability model per the design doc: the primary commit path is *"a single `fsync`"* after writing data and checksums, with an optional *"2-phase commit strategy"* using two fsyncs; media assumptions are (1) single-byte writes are atomic, (2) writes are durable after `fsync`, (3) "powersafe overwrite" ([design.md](https://github.com/cberner/redb/blob/master/docs/design.md)).

**Pluggable storage — the key architectural affordance.** `redb::StorageBackend` is a **synchronous** trait requiring `Send + Sync + 'static + Debug` and five methods:

```rust
fn len(&self) -> Result<u64, Error>;
fn read(&self, offset: u64, out: &mut [u8]) -> Result<(), Error>;
fn set_len(&self, len: u64) -> Result<(), Error>;
fn sync_data(&self) -> Result<(), Error>;
fn write(&self, offset: u64, data: &[u8]) -> Result<(), Error>;
```

plus an optional `close()`. Built-in implementors are `FileBackend` and `InMemoryBackend` ([docs.rs StorageBackend](https://docs.rs/redb/latest/redb/trait.StorageBackend.html)).

**wasm:** redb *compiles* for `wasm32-unknown-unknown`. PR [#1084](https://github.com/cberner/redb/pull/1084) (merged 2025-09-20) fixed the missing `FileBackend::new_internal` in the non-Windows/non-Unix/non-WASI fallback: *"causing compilation failures on targets like WASM."* The parallel PR [#1065](https://github.com/cberner/redb/pull/1065) was closed as redundant, with its author confirming *"It works, thanks!"* against master; that PR's framing is the correct mental model: *"redb doesn't really care about the storage backend; it just needs something that implements the `StorageBackend` trait."* You would supply your own OPFS- or IndexedDB-backed `StorageBackend` — **redb ships no browser backend of its own.**

**Android:** two historical Android issues, both **closed**: [#510 "Android compatible"](https://github.com/cberner/redb/issues/510) (2023-01-28) and [#556 "Android Probability Crash"](https://github.com/cberner/redb/issues/556) (2023-04-16, closed with PR #573). **Unverified:** I could not retrieve the comment thread of #556 to confirm the root cause; the fetched page rendered only the title/metadata. No open Android issues exist today (redb has 6 open issues total).

#### sled

| Fact | Value | Source |
|---|---|---|
| Latest published | **1.0.0-alpha.124**, published **2024-10-11** | [crates.io API](https://crates.io/api/v1/crates/sled) |
| Latest *stable* | **0.34.7** — the 1.0 line has been in alpha for years | [crates.io API](https://crates.io/api/v1/crates/sled) |
| Repo activity | last commit 2026-04-04, **171 open issues**, 9,054 stars | [GitHub API](https://api.github.com/repos/spacejam/sled) |

The repository tagline is literally *"the champagne of beta embedded databases"* ([GitHub](https://github.com/spacejam/sled)). The README warns: *"if reliability is your primary constraint, use SQLite. sled is beta"*, and *"the on-disk format is going to change in ways that require manual migrations before the `1.0.0` release"* ([README](https://github.com/spacejam/sled)). Also flagged: excessive disk usage vs RocksDB in some scenarios, and unsuitability for write-sparse multi-process workloads.

**Assessment:** no crates.io release in ~21 months, 171 open issues, self-declared beta with a promised breaking on-disk format change. For an app whose *only* copy of user data is local, this is the weakest of the three KV options.

#### fjall

| Fact | Value | Source |
|---|---|---|
| Current version | **3.1.8** (2026-07-18) | [crates.io API](https://crates.io/api/v1/crates/fjall) |
| MSRV | **1.90.0** | [README](https://github.com/fjall-rs/fjall) |
| Repo activity | last commit 2026-07-22, 33 open issues, 2,237 stars | [GitHub API](https://api.github.com/repos/fjall-rs/fjall) |
| Note | **3.1.3, 3.1.4, 3.1.5 are all yanked** on crates.io | [crates.io API](https://crates.io/api/v1/crates/fjall) |

*"Log-structured, embeddable key-value storage engine written in Rust"*, LSM-tree based, *"similar to RocksDB"*, *"100% safe & stable Rust"*, with a *"Stable disk format"* where *"Future breaking changes will result in a major version bump and a migration path"* ([README](https://github.com/fjall-rs/fjall)). Not mmap-based (LSM/SSTable design).

Because the data model here is an append-only log, fjall's LSM shape is a natural structural fit — but see §2.4 for its Android file-locking issue, which is currently **open**.

**Unverified:** fjall's README makes no statement about `wasm32-unknown-unknown`, and a GitHub issue search for "wasm" in `fjall-rs/fjall` returns **0 results** ([GitHub issue search](https://api.github.com/search/issues?q=repo:fjall-rs/fjall+wasm)). Assume no browser story.

#### native_db (built on redb)

`native-db` **0.8.2**, last published **2025-07-08**, ~8.3k recent downloads ([crates.io API](https://crates.io/api/v1/crates/native-db)). Low adoption and 12 months without a release; mentioned for completeness only.

### 1.5 Plain append-only file formats

Since the data model is a log, "no database at all" is a live option on desktop and Android. The relevant primitives, all current:

| Crate | Version | Last publish | Notes | Source |
|---|---|---|---|---|
| `serde_json` (JSON Lines) | 1.0.151 | 2026-07-20 | Human-inspectable log; trivially appendable | [crates.io](https://crates.io/api/v1/crates/serde_json) |
| `postcard` | 1.1.3 | 2025-07-24 | Compact, no-std, stable wire format | [crates.io](https://crates.io/api/v1/crates/postcard) |
| `bincode` | 3.0.0 | 2025-12-16 | 3.0 is a recent major bump | [crates.io](https://crates.io/api/v1/crates/bincode) |
| `rkyv` | 0.8.17 | 2026-07-02 | Zero-copy deserialization | [crates.io](https://crates.io/api/v1/crates/rkyv) |

Write-ahead-log crates specifically are **not** in good shape: `okaywal` is at **0.3.1, last published 2023-11-26**, with 708 recent downloads ([crates.io](https://crates.io/api/v1/crates/okaywal)); `simple-wal` is at **0.3.0 from 2020-10-25** ([crates.io](https://crates.io/api/v1/crates/simple-wal)). Rolling your own append+fsync framing over `postcard`/JSONL is more defensible than adopting either.

**Adjacent option — CRDT libraries.** If the "append-only event log" is intended to converge across devices later, these are actively maintained and all have first-class wasm builds:

| Crate | Version | Last publish | Recent downloads | Source |
|---|---|---|---|---|
| `automerge` | 0.10.0 | 2026-06-05 | ~154k | [crates.io](https://crates.io/api/v1/crates/automerge) |
| `loro` | 1.13.7 | 2026-07-15 | ~270k | [crates.io](https://crates.io/api/v1/crates/loro) |
| `yrs` | 0.27.3 | 2026-07-13 | ~668k | [crates.io](https://crates.io/api/v1/crates/yrs) |

**Unverified:** I did not verify each CRDT crate's persistence-layer story per target; they are document/state libraries and still require you to choose where bytes land on each platform.

---

## 2. Android

### 2.1 The general shape of the problem

Two things must be true: (a) the Rust target triple must build, and (b) any **C** dependency must be compiled by the NDK's clang for that triple. (a) is trivial (`rustup target add`); (b) is where SQLite-based options get interesting.

The mainstream tool is **`cargo-ndk` 4.1.2** (published 2025-08-09, [crates.io](https://crates.io/api/v1/crates/cargo-ndk)). Per its README it *"handles all the environment configuration needed for successfully building libraries or binaries for Android from a Rust codebase"*, offers `cargo ndk`, `cargo ndk-test` (runs tests via adb), and `cargo ndk-env`, and *"will automatically detect the most recent NDK version and use it"* when the NDK is installed via Android Studio ([cargo-ndk README](https://github.com/bbqsrc/cargo-ndk)). Supported build hosts: Linux, macOS (x86_64 and arm64), Windows (same README).

Alternative/older tooling is stale: `cargo-apk` **0.10.0, last published 2023-11-30** ([crates.io](https://crates.io/api/v1/crates/cargo-apk)); `xbuild` **0.2.0, last published 2022-12-21**, 625 recent downloads ([crates.io](https://crates.io/api/v1/crates/xbuild)). Supporting crates that *are* current: `android-activity` **0.6.1** (2026-03-24), `jni` **0.22.4** (2026-03-16), `android_logger` **0.15.1** (2025-06-29) ([crates.io](https://crates.io/api/v1/crates/android-activity)). Note `ndk` is at **0.9.0 from 2024-04-26** and `ndk-context` at **0.1.1 from 2022-04-19** ([crates.io](https://crates.io/api/v1/crates/ndk), [crates.io](https://crates.io/api/v1/crates/ndk-context)).

### 2.2 Does `rusqlite` + `bundled` cross-compile to Android?

**Yes in practice, but it is not CI-verified by rusqlite and there are two well-documented failure modes.**

Strongest positive evidence: Tauri's official SQL plugin, **`tauri-plugin-sql` 2.4.0** (2026-04-04), is built on sqlx-with-bundled-SQLite and its README ships a platform matrix listing **Android ✓** (and iOS ✗) ([tauri-plugin-sql README](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/sql/README.md)). Since `sqlx`'s `sqlite` feature implies `sqlite-bundled` ([sqlx 0.9.0 features](https://crates.io/api/v1/crates/sqlx/0.9.0)), the bundled `sqlite3.c` amalgamation demonstrably compiles and links on Android in shipping software.

**Failure mode 1 — toolchain naming / missing `AR_<triple>` and `CC_<triple>`.** rusqlite issue [#503 "Android: during bundled compile, cannot find aarch64-linux-android-clang"](https://github.com/rusqlite/rusqlite/issues/503) (now **closed**) is the canonical report: the `cc` crate looks for an unversioned `aarch64-linux-android-clang` while the modern NDK only ships API-versioned drivers like `aarch64-linux-android28-clang`. `cargo-ndk` exists precisely to set these variables for you. **Unverified:** the fetched issue page did not render the resolution comments, so I cannot cite the exact accepted fix from that thread — but `cargo ndk build` is the documented modern answer.

**Failure mode 2 — missing compiler builtins (`__extenddftf2` et al.).** sqlx issue [#2299 "android compilation fails"](https://github.com/transact-rs/sqlx/issues/2299) is **still open** (last updated 2024-12-19). It reports, for a Tauri Android project with NDK 25.1.8937393 targeting `x86_64-linux-android`: `undefined symbol: __extenddftf2 ... referenced by sqlite3.c:29950 (sqlite3/sqlite3.c:29950)`, alongside `__lttf2`, `__trunctfdf2`. These are 128-bit (`long double`) soft-float runtime routines that SQLite's printf implementation pulls in and that the NDK's default link set omits. **`cargo-ndk` documents the fix directly:** *"### The build is complaining that some compiler builtins are missing. What do I do? — Add `--link-builtins` to your `cargo ndk build` command and you should be happy."* ([cargo-ndk README, Troubleshooting](https://github.com/bbqsrc/cargo-ndk#troubleshooting)). A sibling flag `--link-libcxx-shared` handles `libc++_shared.so` (same section).

Note the reported failure is on **`x86_64-linux-android`** — the emulator triple. Real devices are `aarch64-linux-android`. Budget CI time for the emulator triple specifically.

**Practical checklist for agents:**
1. `rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android`
2. Install NDK "side by side" via Android Studio SDK Manager ([Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)).
3. Build via `cargo ndk -t arm64-v8a -t armeabi-v7a build --release`, adding `--link-builtins` if `__extenddftf2`/`__lttf2`/`__trunctfdf2` appear.
4. Set `ANDROID_HOME`, `ANDROID_NDK_ROOT`, `JAVA_HOME` ([Slint Android docs](https://docs.rs/slint/latest/slint/android/index.html) documents exactly these three).

### 2.3 Linking Android's *system* SQLite

**Not a supported option.** SQLite is **not** on the Android NDK's list of stable native APIs; the NDK stable-API surface is enumerated at [Native APIs | Android NDK](https://developer.android.com/ndk/guides/stable_apis). Every device ships `/system/lib/libsqlite.so` (and Android's Java `SQLiteDatabase` uses it), but it is not headers-and-stub-library exposed to apps, and since Android N the linker namespace restrictions actively prevent apps loading non-NDK system libraries. Dart's own SQLite FFI guidance reaches the same conclusion — before Android N a native library could rely on the system `libsqlite.so`, *"but this was not officially supported, and sqlite was not one of the stable APIs"*, so you must ship your own build ([dart-lang/sdk samples/ffi/sqlite/docs/android.md](https://github.com/dart-lang/sdk/blob/main/samples/ffi/sqlite/docs/android.md)).

**Pitfalls even if you tried:** version is device- and OEM-dependent (no control over which SQLite version, which compile-time options, or whether FTS5/JSON1 are present); it is not in the NDK's linker-visible namespace; and you get zero reproducibility across the device fleet. **Bundling is the only defensible choice** — which is also what `rusqlite --features bundled` and `sqlx --features sqlite` do by default.

### 2.4 KV stores on Android

**Neither redb nor fjall uses mmap** (§1.4), so the classic "mmap on Android storage" hazards (emulated `/sdcard` FUSE layers, `MAP_SHARED` write-back semantics) do not apply if the DB lives in the app's private `filesDir`, which is real ext4/f2fs. All three are pure Rust and therefore need **no NDK C toolchain at all** — this is their single biggest advantage over SQLite on Android.

**The real Android hazard is `std::fs::File::lock()`, and it was fixed in Rust 1.98.0.**

- fjall issue [#225 "`File::lock()` and related functions are unsupported on some platforms"](https://github.com/fjall-rs/fjall/issues/225) (opened 2026-01-03, **still open**, labelled "platform support") states: *"std file-locking APIs are not supported on some platforms like Android"*, and reports the concrete failure: `Io(Error { kind: Unsupported, message: "try_lock() not supported" })`. The reporter's suggested workaround is to *"skip locking and log the event when locking is unsupported."*
- Root cause was in Rust std, not fjall: the `flock`-based implementation's `cfg` list did not include `target_os = "android"`, falling through to `Err(io::const_error!(io::ErrorKind::Unsupported, "try_lock() not supported"))`. Tracked as rust-lang/rust issue [#148325 "`File::lock` does not work on Android"](https://github.com/rust-lang/rust/issues/148325) (opened 2025-10-31).
- **Fixed by** rust-lang/rust PR [#157038 "android: implement file locking by calling flock"](https://github.com/rust-lang/rust/pull/157038), merged 2026-05-29, **milestone 1.98.0**. Current master's `library/std/src/sys/fs/unix.rs` now lists `target_os = "android"` in the `flock(LOCK_EX | LOCK_NB)` arm ([unix.rs, lines ~1542-1563](https://raw.githubusercontent.com/rust-lang/rust/master/library/std/src/sys/fs/unix.rs)).
- Rust **1.98.0 is current stable** (channel manifest dated 2026-07-16, [channel-rust-stable.toml](https://static.rust-lang.org/dist/channel-rust-stable.toml)).

**Consequence:** if you use fjall (or anything else using `std` file locks) on Android, **pin MSRV ≥ 1.98.0**. On Rust ≤ 1.97 it fails at database-open time on Android. Note also the caveat surfaced in the community discussion that some Android 14 devices reportedly do not support the `flock` syscall itself and need `fcntl(F_SETLKW)` instead ([internals.rust-lang.org: "Is `File::lock` really unsupported on Android?"](https://internals.rust-lang.org/t/is-file-lock-really-unsupported-on-android/23711)) — **Unverified:** I could not confirm this device-level claim against an Android/AOSP primary source.

**fsync on Android:** redb's durability rests on the three media assumptions quoted in §1.4 ([design.md](https://github.com/cberner/redb/blob/master/docs/design.md)); Android's app-private storage on ext4/f2fs satisfies these to the same degree desktop Linux does. **Unverified:** I found no primary source specifically measuring `fsync` honesty on Android device flash, so treat "fsync is durable" as the same act of faith it is on desktop.

**sled on Android:** no current data. Given §1.4's maintenance findings, out of scope.

---

## 3. Web / wasm — the hard case

There is no filesystem: the `wasm32-unknown-unknown` target has std, but *"many parts do not work, such as `std::fs` which always returns errors"* ([The rustc book, wasm32-unknown-unknown](https://doc.rust-lang.org/beta/rustc/platform-support/wasm32-unknown-unknown.html)). Everything below is about replacing that.

### 3.1 OPFS (Origin Private File System)

**Exact browser support**, from MDN's browser-compat-data (the machine-readable primary source behind MDN's tables):

| API | Chrome | Chrome **Android** | Firefox | Firefox Android | Safari | Edge |
|---|---|---|---|---|---|---|
| `StorageManager.getDirectory()` (OPFS root) | **86** | **109** | **111** | mirrors desktop (111) | **15.2** | mirrors Chrome |
| `FileSystemFileHandle` | 86 | 109 | 111 | mirrors desktop | 15.2 | mirrors Chrome |
| `FileSystemFileHandle.createSyncAccessHandle()` | **102** | **109** | **111** | mirrors desktop | **15.2** | mirrors Chrome |
| `FileSystemSyncAccessHandle` (interface) | **102** | **109** | **111** | mirrors desktop | **15.2** | mirrors Chrome |
| `FileSystemSyncAccessHandle.close/flush/getSize/truncate` — **sync (non-promise) variants** | **108** | **109** | **111** | mirrors desktop | **16.4** | mirrors Chrome |
| `FileSystemFileHandle.createWritable()` | 86 | 109 | 111 | mirrors desktop | **26** | mirrors Chrome |
| `createSyncAccessHandle(mode)` (multi-reader modes) | 121, **experimental** | mirrors Chrome | ✗ | ✗ | ✗ | mirrors Chrome |

Source: [mdn/browser-compat-data `api/FileSystemSyncAccessHandle.json`](https://raw.githubusercontent.com/mdn/browser-compat-data/main/api/FileSystemSyncAccessHandle.json), [`api/FileSystemFileHandle.json`](https://raw.githubusercontent.com/mdn/browser-compat-data/main/api/FileSystemFileHandle.json), [`api/StorageManager.json`](https://raw.githubusercontent.com/mdn/browser-compat-data/main/api/StorageManager.json). ("mirror" in BCD means the value is inherited from the corresponding desktop/Chromium entry.)

**Three things worth flagging for an Android-targeting project:**
1. **Chrome Android landed OPFS at 109, not 86/102.** Android is the *lagging* Chromium surface here, not desktop.
2. The **synchronous** `read`/`write`/`getSize`/`flush`/`truncate` signatures — the ones an SQLite VFS actually wants — required Chrome **108** / Safari **16.4**. Safari 15.2–16.3 has the promise-returning variants only.
3. Safari only got `createWritable()` in **26**, so a non-SAH write path is very new there. (Not relevant if you don't target iOS/macOS Safari, but relevant if the web build is meant to be universal.)

**Worker requirement is absolute.** MDN: `FileSystemSyncAccessHandle` is *"exclusively available in Dedicated Web Workers, not on the main thread"*, and is *"available only in secure contexts (HTTPS)"* ([MDN FileSystemSyncAccessHandle](https://developer.mozilla.org/en-US/docs/Web/API/FileSystemSyncAccessHandle)). MDN dates the feature as *"Baseline Widely available"*, available across browsers **since March 2023** (same page). Note the "widely available" badge is computed from desktop+mobile *core* browsers and glosses over the Chrome-Android-109 detail above.

**Any OPFS-based storage therefore forces your persistence layer into a dedicated Web Worker**, with your UI thread talking to it over `postMessage`/Comlink-style RPC. This is an architectural constraint on the *whole app*, not a storage detail.

### 3.2 IndexedDB from Rust

| Crate | Version | Last publish | Repo last commit | Recent dl | Verdict |
|---|---|---|---|---|---|
| `indexed_db_futures` | **0.6.4** | 2025-05-11 | 2025-05-11 | ~710k | Most-used; 0.6 was a full API rewrite |
| `idb` | **0.6.5** | 2025-12-29 | 2025-12-29 | ~180k | Most recently released; author's recommended crate |
| `rexie` | **0.6.2** | 2024-08-12 | 2026-05-07 (README-only) | ~87k | **Effectively deprecated by its own author** |

Sources: [crates.io idb](https://crates.io/api/v1/crates/idb), [crates.io indexed_db_futures](https://crates.io/api/v1/crates/indexed_db_futures), [crates.io rexie](https://crates.io/api/v1/crates/rexie), [GitHub API idb](https://api.github.com/repos/devashishdxt/idb), [GitHub API rust-indexed-db](https://api.github.com/repos/Alorel/rust-indexed-db), [GitHub API rexie](https://api.github.com/repos/devashishdxt/rexie).

**`rexie` is a dead end and says so.** Its README (last touched 2026-05-07, commit titled "Clarify future plans and recommend 'idb' crate (#53)"): *"I don't plan to add new features to this crate, but I'll continue to fix bugs as they come up. For new projects, I recommend considering `idb`, which provides a similar API and is the crate I plan to maintain more actively going forward."* ([rexie README](https://github.com/devashishdxt/rexie#readme)). Same author as `idb`.

**`idb`** — `Factory::open()` → awaitable request, schema via `on_upgrade_needed`, `Database::transaction()` with `ReadOnly`/`ReadWrite`, async/await by default via the `futures` feature (disabling it gives callback style with `on_success`/`on_error`). Deps: `wasm-bindgen ^0.2`, `web-sys ^0.3`, `js-sys ^0.3` ([docs.rs idb](https://docs.rs/idb/latest/idb/)). **Unverified:** the docs do not state Web Worker compatibility or an MSRV.

**`indexed_db_futures`** — builder-pattern async API mirroring the JS shape, with traits `BuildPrimitive`/`BuildSerde`/`Build`, and 11 optional features including `async-upgrade`, `cursors`, `dates`, `indices`, `serde`, `streams`, `typed-arrays`. Two documented gotchas: *"Unlike Javascript, transactions will roll back by default instead of committing"* (deliberate, to cooperate with `?`), and a warning that apps compiled with `#[cfg(target_feature = "atomics")]` *"may encounter problems due to transaction auto-commit timing conflicts in multi-threaded environments"* ([docs.rs indexed_db_futures](https://docs.rs/indexed_db_futures/latest/indexed_db_futures/)). That atomics caveat matters if you ever enable wasm threads. **Unverified:** MSRV not documented.

**Ergonomics note:** IndexedDB is asynchronous *by specification*, so any Rust wrapper is async, and you cannot drive it from a synchronous SQLite VFS callback without either (a) accepting relaxed durability by buffering in memory (what `relaxed-idb` does, §3.3) or (b) blocking a worker on `Atomics.wait`.

### 3.3 SQLite compiled to wasm

Four distinct things, often conflated:

#### (a) `sqlite-wasm` — SQLite's own official WASM build (JavaScript)

Repo [sqlite/sqlite-wasm](https://github.com/sqlite/sqlite-wasm) — *"SQLite Wasm conveniently wrapped as an ES Module"*, 1,028 stars, last commit 2026-07-13 ([GitHub API](https://api.github.com/repos/sqlite/sqlite-wasm)). This is a **JS/ESM** distribution, not a Rust crate. Its two persistence VFSes, from the official docs:

**OPFS VFS** — *"This support is only available when `sqlite3.js` is loaded from a Worker thread"*, and *"JavaScript's `SharedArrayBuffer` type is required for the OPFS VFS, and that class is only available if the web server includes"* the COOP/COEP headers. Browser support given as *"Chromium-derived browsers released since approximately mid-2022"*, Firefox v111+, Safari 16.4+ (with versions <17 flagged as having incompatibility issues). Locking: *"no two database handles can have the same OPFS-hosted database open at one time"* and *"there's no such thing as 'N concurrent readers' in OPFS-via-VFS"*; testing in 2026 *"was consistently able to handle 8-10 concurrent workers for long periods, provided (A) all keep their locking to a minimum."* ([sqlite.org/wasm persistence.md](https://sqlite.org/wasm/doc/trunk/persistence.md))

**OPFS SAHPool VFS** — Worker-only, but **"Does not require COOP/COEP HTTP headers"** and no `SharedArrayBuffer`. *"Should work on all major browsers released since March 2023."* *"Does not support multiple simultaneous connections"* at the library level, *"though client-level solutions exist via WebLocks coordination."* (same page)

**This is the single most important trade-off on the web target:** OPFS VFS buys you better concurrency at the price of **COOP/COEP cross-origin isolation headers on your hosting**, which for a "no server of our own" app means whatever static host you pick must let you set `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp`. SAHPool avoids the header requirement entirely at the price of single-connection-only.

#### (b) `sqlite-wasm-rs` — the real Rust binding

| Fact | Value | Source |
|---|---|---|
| Version | **0.5.5** (2026-05-25) | [crates.io](https://crates.io/api/v1/crates/sqlite-wasm-rs) |
| Recent downloads | **~3.89M/90d** | [crates.io](https://crates.io/api/v1/crates/sqlite-wasm-rs) |
| MSRV | **1.85.0** | [README](https://github.com/Spxg/sqlite-wasm-rs#minimum-supported-rust-version-msrv) |
| Target | `wasm32-unknown-unknown` exclusively | [README](https://github.com/Spxg/sqlite-wasm-rs) |
| License | MIT | [README](https://github.com/Spxg/sqlite-wasm-rs) |
| Repo | 96 stars, **1 open issue**, last commit 2026-06-26 | [GitHub API](https://api.github.com/repos/Spxg/sqlite-wasm-rs) |

*"`wasm32-unknown-unknown` bindings to the libsqlite3 library."* The VFS implementations live in a **separate companion crate, `sqlite-wasm-vfs` 0.2.0** (2026-01-17), with the README noting *"It requires sqlite-wasm-rs 0.5.2 or higher to be used, for version 0.5.1, use version 0.1 instead."* ([README](https://github.com/Spxg/sqlite-wasm-rs#about-vfs), [crates.io sqlite-wasm-vfs](https://crates.io/api/v1/crates/sqlite-wasm-vfs)). A helper crate `rsqlite-vfs` **0.1.1** (2026-05-19) exists for writing your own ([crates.io](https://crates.io/api/v1/crates/rsqlite-vfs)).

The README's VFS comparison table, verbatim:

| | MemoryVFS | SyncAccessHandlePoolVFS | RelaxedIdbVFS |
|-|-|-|-|
| Storage | RAM | **OPFS** | **IndexedDB** |
| Contexts | All | **Dedicated Worker** | All |
| Multiple connections | ✗ | ✗ | ✗ |
| Full durability | ✓ | ✓ | ✗ |
| Relaxed durability | ✗ | ✗ | ✓ |
| Multi-database transactions | ✓ | ✓ | ✓ |
| **No COOP/COEP requirements** | ✓ | **✓** | ✓ |

([sqlite-wasm-rs README](https://github.com/Spxg/sqlite-wasm-rs#vfs-comparison))

Read this table carefully — it encodes the two real decisions:
- **`sahpool` (OPFS)**: full durability, no COOP/COEP needed, **but requires a Dedicated Worker**.
- **`relaxed-idb` (IndexedDB)**: runs in *any* context including the main thread, **but explicitly gives up full durability**.
- **Neither supports multiple connections**, so multi-tab requires your own coordination (Web Locks / BroadcastChannel / a single SharedWorker owner).

**Threading:** *"This library is not thread-safe: `JsValue` is not cross-threaded ... sqlite is compiled with `-DSQLITE_THREADSAFE=0`."* ([README, "About multithreading"](https://github.com/Spxg/sqlite-wasm-rs#about-multithreading)). Encryption is available via the `sqlite3mc` feature (SQLite3MultipleCiphers), and a `sqlite-vec` extension is packaged (same README).

**Unverified:** the exact SQLite version bundled by sqlite-wasm-rs 0.5.5. The README does not state it and I did not resolve it from the build script.

#### (c) `wa-sqlite`

[rhashimoto/wa-sqlite](https://github.com/rhashimoto/wa-sqlite) — 1,390 stars, last commit 2026-07-21, actively maintained ([GitHub API](https://api.github.com/repos/rhashimoto/wa-sqlite)). It is a **JavaScript** project ("WebAssembly SQLite with support for browser storage extensions") and is listed by sqlite-wasm-rs only as a "Related Project" ([sqlite-wasm-rs README](https://github.com/Spxg/sqlite-wasm-rs#related-project)). **No first-party Rust bindings.** Not a Rust-side candidate.

#### (d) Diesel-specific: `sqlite-web-rs` / `diesel-wasm-sqlite`

[xmtp/sqlite-web-rs](https://github.com/xmtp/sqlite-web-rs), crate `sqlite-web` at **0.0.1, published 2024-12-09** ([crates.io](https://crates.io/api/v1/crates/sqlite-web)). Superseded by diesel's own built-in `sqlite-wasm-rs` support (§1.3). Ignore.

#### (e) Turso (formerly Limbo) — pure-Rust SQLite rewrite

Worth evaluating because a pure-Rust SQLite would in principle solve *both* the Android C-toolchain problem and the wasm problem at once. It currently does **not**, for Rust-on-`wasm32-unknown-unknown`.

| Fact | Value | Source |
|---|---|---|
| Crate `turso` | **0.7.1** stable (2026-07-22); `0.8.0-pre.2` (2026-07-26) | [crates.io](https://crates.io/api/v1/crates/turso) |
| Recent downloads | ~353k/90d | [crates.io](https://crates.io/api/v1/crates/turso) |
| Status | *"Yes — Turso powers production applications today at multiple organizations"* but *"we have not yet reached 1.0."* | [README](https://github.com/tursodatabase/turso) |
| Compatibility | *"compatible with SQLite at the SQL dialect, file format, and C API levels, and existing SQLite database files work as-is. We are not at 100% yet"* | [README](https://github.com/tursodatabase/turso) |

Its browser story is real but **not reachable from a Rust wasm build**: Turso-in-the-browser ships as a JS SDK compiled to **`wasm32-wasip1-threads`** with a Web Worker doing synchronous OPFS I/O and `SharedArrayBuffer` shared between worker and main thread ([Introducing Turso in the Browser](https://turso.tech/blog/introducing-turso-in-the-browser)) — note the `SharedArrayBuffer` dependency implies COOP/COEP. The request to embed Turso in a **Rust** library compiled to wasm is issue [#5049](https://github.com/tursodatabase/turso/issues/5049), **open**, assigned to the *"Backlog"* milestone, currently failing to compile with pointer-width errors (*"attempt to compute `8_usize - 16_usize`, which would overflow"* in `bindings.rs`, i.e. 64-bit pointer assumptions vs wasm32).

Compatibility gaps that matter for an event-log app ([COMPAT.md](https://github.com/tursodatabase/turso/blob/main/COMPAT.md)): `WITH RECURSIVE` not supported; `WITHOUT ROWID` tables *"effectively insert-only"* (UPDATE/DELETE/UPSERT rejected); limited foreign-key support; no custom collations; no BLOB I/O API; no backup API; no `CREATE VIRTUAL TABLE`; and a behavioural divergence where Turso returns `SQLITE_BUSY` for multiple active writes on one connection.

**Verdict:** promising, not yet a fit. Revisit post-1.0.

### 3.4 Persistence durability in the browser

**What `navigator.storage.persist()` actually guarantees — per spec:** the WHATWG Storage Standard defines two bucket modes, `"best-effort"` (the default; *"Data can be cleared by the user agent without user involvement"*) and `"persistent"` (*"persistent buckets cannot be cleared without consent by the user"*). Persistence *"can be used to protect storage from the user agent's clearing policies. The user agent cannot clear storage marked as persistent without involvement from the origin or user."* Under pressure, UAs should clear network state and best-effort buckets first, and only clear persistent buckets *"after informing the user and obtaining their consent."* Quota calculation and eviction heuristics are left **implementation-defined**. ([WHATWG Storage Standard](https://storage.spec.whatwg.org/))

MDN restates the API contract: `persist()` resolves `true` only if *"Permission granted AND bucket mode is persistent (storage won't be cleared except by explicit user action)"*, and warns *"The browser may or may not honor the request depending on browser-specific rules. There's no guarantee of persistence."* It is **secure-contexts-only** and, per MDN, **not available in Web Workers** ([MDN StorageManager.persist](https://developer.mozilla.org/en-US/docs/Web/API/StorageManager/persist)) — which means you must call it from the main thread even though your OPFS I/O lives in a worker.

`StorageManager.persist` support: Chrome **55**, Chrome Android mirrors desktop, Firefox **57**, Safari **15.2** ([BCD api/StorageManager.json](https://raw.githubusercontent.com/mdn/browser-compat-data/main/api/StorageManager.json)).

**How grants are decided:**
- **Chrome (and Chromium/Edge)**: no prompt is ever shown; grant is silent and heuristic. web.dev lists the criteria as: *"How high is the level of site engagement?"*, *"Has the site been installed or bookmarked?"*, *"Has the site been granted permission to show notifications?"* ([web.dev — Persistent storage](https://web.dev/articles/persistent-storage)). MDN concurs that Safari and Chrome/Edge *"auto-approve/deny based on user interaction history, no popup shown"* ([MDN Storage quotas and eviction criteria](https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria)).
- **Firefox**: *"When persistent storage is requested, it prompts the user with a UI popup asking if they will allow the site to store data in persistent storage."* ([web.dev](https://web.dev/articles/persistent-storage))

**What persistence protects:** Cache API, cookies, DOM/Local Storage, File System API (**this includes OPFS**), IndexedDB, service workers ([web.dev](https://web.dev/articles/persistent-storage)). So a single successful `persist()` covers whichever of OPFS/IndexedDB you pick.

**Quotas** ([MDN Storage quotas and eviction criteria](https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria)):
- Chrome/Chromium (both modes): up to **60% of total disk size** per origin.
- Firefox best-effort: min(10% of disk, **10 GiB group limit**); persistent: up to **50% of disk, capped at 8 TiB**, exempt from the group limit.
- Safari browser apps: ~**60%** of disk, overall browser cap 80%.
- `localStorage`: hard **~5 MiB** per origin (10 MiB total across local+session) on all browsers. *This rules localStorage out as a real event-log store — relevant to eframe-on-web, see §B.1.*

**Eviction** (same MDN page): LRU by origin under storage pressure, skipping origins with persistence granted; **all** of an origin's data is deleted together (IndexedDB + Cache API + OPFS), not selectively, *"prevent[ing] data inconsistency"*. Safari additionally applies a proactive 7-day rule: *"If an origin has no user interaction, such as click or tap, in the last seven days of browser use, its data created from script will be deleted."*

**How this differs on Android Chrome specifically.** BCD records Chrome Android as *mirroring* desktop Chrome for `StorageManager.persist`/`persisted`/`estimate` ([BCD](https://raw.githubusercontent.com/mdn/browser-compat-data/main/api/StorageManager.json)), and neither the WHATWG spec nor MDN documents a distinct Android eviction policy. **The material Android differences are practical, not spec-level**, and I want to be precise about what I could and could not verify:
- **Verified:** OPFS/`createSyncAccessHandle` arrived at **Chrome Android 109**, materially later than desktop Chrome 102 ([BCD](https://raw.githubusercontent.com/mdn/browser-compat-data/main/api/FileSystemSyncAccessHandle.json)).
- **Verified:** Chrome's grant heuristics include *"Has the site been installed"* — on Android, installing as a PWA / adding to home screen is the most reliable way to satisfy this ([web.dev](https://web.dev/articles/persistent-storage)).
- **Unverified:** I found no primary source stating that Android Chrome evicts *more aggressively* than desktop, nor documenting Android's OS-level "clear cached data"/app-storage-manager interaction with Chrome's origin storage. A phone is simply far more likely to hit the storage-pressure condition that triggers LRU eviction in the first place. Treat "the web build's data can vanish on a full phone unless persisted" as the operating assumption, and do **not** treat the browser as the system of record.

---

## 4. The one-codebase question

**Short answer: there is no crate today that gives you one unchanged storage API across desktop, Android, and wasm. You must abstract behind a trait with (at least) two backends. But the seam is much narrower than it was 18 months ago, and where you put it changes a lot.**

### 4.1 What genuinely *does* compile unchanged on all three

**`rusqlite` is the closest thing that exists.** Since 0.38.0 the *same* `rusqlite::Connection` API compiles for `x86_64-unknown-linux-gnu`, `aarch64-linux-android`, and `wasm32-unknown-unknown`, because the `ffi-sqlite-wasm-rs` default feature transparently swaps `libsqlite3-sys` for `sqlite-wasm-rs` on wasm ([README](https://github.com/rusqlite/rusqlite#optional-features), [PR #1769](https://github.com/rusqlite/rusqlite/pull/1769)). Diesel does the identical trick via cfg-gated dependencies ([diesel 2.3.11 deps](https://crates.io/api/v1/crates/diesel/2.3.11/dependencies)). Your **SQL, schema, migrations, and row-mapping code are genuinely portable.**

**Similarly, redb's `Database` API is target-independent** — redb 4.1.0 has no platform-specific runtime dependency except `libc` on WASI, and compiles for `wasm32-unknown-unknown` since PR [#1084](https://github.com/cberner/redb/pull/1084).

### 4.2 What forces the split anyway

Four hard constraints, each independently sufficient:

1. **Storage *location* is not abstracted.** rusqlite-on-wasm defaults to the **in-memory VFS**, which does not persist. Persistence requires separately depending on `sqlite-wasm-vfs` 0.2 and registering `sahpool` or `relaxed-idb` at startup — a wasm-only code path with no native analogue ([sqlite-wasm-rs README](https://github.com/Spxg/sqlite-wasm-rs#about-vfs)). Likewise redb-on-wasm requires you to hand-write a `StorageBackend` over OPFS/IndexedDB; **redb ships none** ([StorageBackend docs](https://docs.rs/redb/latest/redb/trait.StorageBackend.html)).

2. **The Worker requirement is contagious.** Full-durability OPFS storage is only reachable from a Dedicated Web Worker (`FileSystemSyncAccessHandle` is *"exclusively available in Dedicated Web Workers"* — [MDN](https://developer.mozilla.org/en-US/docs/Web/API/FileSystemSyncAccessHandle); sqlite-wasm-rs's sahpool row says "Dedicated Worker" — [README](https://github.com/Spxg/sqlite-wasm-rs#vfs-comparison)). So on web your storage layer lives behind a message-passing boundary, whereas on desktop/Android it's an in-process call. That boundary is **async and fallible** on web and **sync and infallible-ish** on native, which is precisely the shape difference a trait has to paper over.

3. **Threading.** sqlite-wasm-rs: *"This library is not thread-safe ... sqlite is compiled with `-DSQLITE_THREADSAFE=0`."* ([README](https://github.com/Spxg/sqlite-wasm-rs#about-multithreading)). This is exactly what killed sqlx's wasm support (*"Because of the lack of threading, there's no getting around the blocking calls into SQLite which breaks the async API as it currently stands"* — [sqlx PR #3994](https://github.com/transact-rs/sqlx/pull/3994)). Any native design that puts SQLite on a background thread (`tokio-rusqlite`, sqlx's connection worker) does not port.

4. **Multi-tab/multi-connection semantics differ.** All three sqlite-wasm-rs VFSes are marked "Multiple connections: ✗" ([README](https://github.com/Spxg/sqlite-wasm-rs#vfs-comparison)), and the official SQLite WASM docs say *"no two database handles can have the same OPFS-hosted database open at one time"* ([sqlite.org/wasm](https://sqlite.org/wasm/doc/trunk/persistence.md)). On desktop/Android, SQLite's normal multi-connection/WAL behaviour applies. Concurrency invariants therefore cannot be assumed uniform.

### 4.3 The two seam positions worth considering

Given an **append-only event log**, the trait boundary can sit at very different altitudes:

- **Low seam (bytes):** define your own `trait LogStore { fn append(&self, records: &[Event]) -> Result<()>; fn read_from(&self, seq: u64) -> Result<Vec<Event>>; }`. Native impl = append to a file (or redb/fjall); wasm impl = OPFS file via worker, or IndexedDB. This maximises portable code (all query/projection logic is pure Rust over the log) and reduces the platform-specific surface to ~5 methods. It is also the *only* option that keeps redb/fjall on the table, since neither has a browser backend.
- **High seam (SQL):** use rusqlite everywhere and keep the seam at "how do I open the connection" — native `Connection::open(path)`, wasm register-VFS-then-`Connection::open("app.db")` inside a worker. Maximises reuse of SQL/migrations but forces the *entire* storage-touching call graph on web behind a worker RPC, since the connection isn't `Send` and the VFS is worker-only.

**Nothing verified in this research presents one API over both native SQLite and wasm SQLite/OPFS *including* the persistence configuration.** The `opfs` crate ([anchpop/opfs](https://github.com/anchpop/opfs), **0.2.0**, published 2026-03-27, 27 stars, ~10.7k recent downloads — [crates.io](https://crates.io/api/v1/crates/opfs)) is the only crate found that even attempts the unified-filesystem framing: *"when compiling to native platforms, it will use `tokio::fs` instead of browser APIs"*, with *"Write once, run anywhere"* and an in-memory impl for tests ([README](https://github.com/anchpop/opfs#readme)). But its API is **fully async on both sides** (`create_writable_with_options(...).await`), which makes it unusable as a drop-in for a synchronous SQLite VFS or redb `StorageBackend`, and at 27 stars / 0.2.0 it is not a load-bearing dependency. **Unverified:** whether `opfs` 0.2.0 uses `createSyncAccessHandle` or the async `createWritable` path under the hood, and therefore what its actual durability and worker requirements are.

---

# Other contenders

Scope note: Dioxus and Leptos+Tauri are covered by other agents and are excluded here.

An architectural point that applies to the whole section: **Tauri is not a web deployment target.** Its docs state *"Tauri acts as a static web host"* and *"Tauri does not natively support server based alternatives (such as SSR)"* ([Tauri frontend docs](https://v2.tauri.app/start/frontend/)), and it produces desktop and mobile binaries only. A "Tauri + <Rust web framework>" stack gets you desktop + Android from Tauri, and the *web* target by deploying the same wasm frontend separately as a static SPA — two build pipelines sharing one frontend crate. By contrast egui/Slint/Makepad each render one canvas everywhere from a single toolchain.

## B.1 egui / eframe

| Fact | Value | Source |
|---|---|---|
| Version | **egui 0.35.0**, **eframe 0.35.0**, both 2026-06-25 | [crates.io egui](https://crates.io/api/v1/crates/egui), [crates.io eframe](https://crates.io/api/v1/crates/eframe) |
| MSRV | **1.92** (eframe 0.35.0) | [crates.io version API](https://crates.io/api/v1/crates/eframe/0.35.0) |
| Recent downloads | egui ~4.55M/90d, eframe ~3.75M/90d | [crates.io](https://crates.io/api/v1/crates/egui) |
| Rendering | Immediate-mode; textured triangles via **wgpu** (default) or **glow**; on web, **WebGPU when available, else WebGL2** | [egui README](https://github.com/emilk/egui), [eframe README](https://github.com/emilk/egui/blob/main/crates/eframe/README.md) |
| Licence | MIT / Apache-2.0 | [eframe README](https://github.com/emilk/egui/blob/main/crates/eframe/README.md) |

**Android: real but rough.** eframe's README claims Android among supported platforms: *"`eframe` is the official framework library for writing apps using egui. The app can be compiled both to run natively (for Linux, Mac, Windows, and Android) or as a web app"* ([eframe README](https://github.com/emilk/egui/blob/main/crates/eframe/README.md)). Concretely, eframe 0.35.0 exposes two mutually-exclusive Cargo features — `android-game-activity` and `android-native-activity` — each forwarding to `egui-winit` ([crates.io features](https://crates.io/api/v1/crates/eframe/0.35.0), [docs.rs eframe](https://docs.rs/eframe/latest/eframe/)).

The support landed in PR [#5318 "Android support for eframe"](https://github.com/emilk/egui/pull/5318), **merged 2024-12-12**. Its mechanism and its caveats, from the author: *"allowing `eframe` to be used on Android. It works by smugling the `AndroidApp` required by `winit` through `NativeOptions`"*; *"The example isn't great because it doesn't leave space on the display for Android's top status bar or the lower navigation bar"* (tracked upstream at `rust-windowing/winit#3910`); and *"the development environment setup is completely awful for Android unless you happen to already be a full-time Android developer with everything configured."*

**Web: works, with documented and structural downsides.** eframe's README has a dedicated "Limitations when running egui on the web" section that is unusually candid and directly relevant to a flashcard app with lots of text entry:

> *"`eframe` and egui compiles to Wasm using either WebGPU (when available) or WebGL2 for rendering, and almost nothing else from the web tech stack."*
> - *"Search: you cannot search an egui web page like you would a normal web page."*
> - *"Bringing up an on-screen keyboard on mobile: there is no JS function to do this, so `eframe` fakes it by adding some invisible DOM elements. It doesn't always work."*
> - *"Mobile text editing is not as good as for a normal web app."*
> - *"Accessibility: There is an experimental screen reader for `eframe`, but it has to be enabled explicitly... `egui` supports AccessKit, but as of early 2024, AccessKit lacks a Web backend."*
>
> *"The suggested use for `eframe` are for web apps where performance and responsiveness are more important than accessibility and mobile text editing."*
> ([eframe README](https://github.com/emilk/egui/blob/main/crates/eframe/README.md))

egui's own README also states: *"egui is in active development. It works well for what it does, but it lacks many features and the interfaces are still in flux."* ([egui README](https://github.com/emilk/egui))

**Storage story — the notable gotcha.** eframe's `persistence` feature is backed by **`localStorage` on web**. The implementation is unambiguous:

```rust
fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}
```

with `local_storage_get`/`local_storage_set`/`local_storage_remove` and `load_memory` reading the key `"egui_memory_ron"` ([eframe `src/web/storage.rs`](https://raw.githubusercontent.com/emilk/egui/main/crates/eframe/src/web/storage.rs)). `localStorage` is capped at **~5 MiB per origin** ([MDN](https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria)). So **eframe's built-in persistence is only suitable for UI state, never for the event log** — you would bring your own storage layer per §3 regardless. On native, `persistence` pulls the `home` crate for a platform data directory ([crates.io features](https://crates.io/api/v1/crates/eframe/0.35.0)).

**Viable for Android?** Yes, with caveats about status-bar insets and the text-input experience. **Nothing about storage is provided for you on any target.**

## B.2 Slint

| Fact | Value | Source |
|---|---|---|
| Version | **1.17.1** (2026-07-07) | [crates.io](https://crates.io/api/v1/crates/slint) |
| Recent downloads | ~366k/90d | [crates.io](https://crates.io/api/v1/crates/slint) |
| API stability | *"Slint follows a stable 1.x API"* | [README](https://github.com/slint-ui/slint) |

**Licensing — the distinguishing factor.** Three options ([Slint README](https://github.com/slint-ui/slint#license), [LICENSE.md](https://github.com/slint-ui/slint/blob/master/LICENSE.md)):
1. **Royalty-free License** — *"Build proprietary desktop, mobile, or web applications for free"*. Per LICENSE.md, it *"permits use in **proprietary** desktop, mobile, and web applications **at no cost**. Use in embedded systems is excluded."* Crucially it carries an **attribution requirement**: free *"as long as you disclose that you use Slint (for example with the `AboutSlint` widget or the Slint badge); without that disclosure, use the Commercial license."*
2. **GNU GPLv3** — free for open-source on any platform including embedded.
3. **Commercial License** — proprietary on all platforms including embedded.

For a personal/open-source flashcard app on desktop+web+Android, option 1 or 2 applies; **option 1 obliges you to display a Slint badge or the `AboutSlint` widget somewhere in the UI.** That is a product decision, not just a legal one.

**Android: supported, Rust-only, `cargo-apk`-based.** The Rust `slint::android` module docs say: *"To build and deploy your application, we suggest the usage of [cargo-apk](https://github.com/rust-mobile/cargo-apk)"*, with *"Slint does not require a specific build tool and can work with others, such as xbuild"*. The documented invocation is `cargo apk run --target aarch64-linux-android --lib`, requiring the `backend-android-activity-06` feature and a `cdylib` crate type, plus `ANDROID_HOME`, `ANDROID_NDK_ROOT`, and (optionally) `JAVA_HOME` ([docs.rs slint::android](https://docs.rs/slint/latest/slint/android/index.html)). Note the recommended tool, **`cargo-apk`, was last published 2023-11-30** ([crates.io](https://crates.io/api/v1/crates/cargo-apk)) — a stale dependency in an otherwise current stack. Android support is Rust-only; C++ is not supported there ([Slint discussion #10425](https://github.com/slint-ui/slint/discussions/10425)).

**Web/wasm: explicitly discouraged for general web apps.** Slint's own web platform guide: *"Only Rust supports using Slint with WebAssembly"*; Slint *"renders your UI into a HTML `<canvas>` element using WebGL"* without using the DOM or CSS; and, decisively, ***"running Slint in the browser is currently not recommended for building general-purpose web applications."*** Listed limitations: text rendering bypasses browser capabilities, screen readers unavailable, non-standard UI behaviour ([Slint Web platform docs](https://docs.slint.dev/latest/docs/slint/guide/platforms/web/)).

**Storage story:** none provided; Slint is a UI toolkit. You bring §3's stack yourself, and on web you'd be pairing a WebGL canvas with an OPFS worker.

**Viable for Android?** Yes. **Viable for web?** The vendor says not for general-purpose web apps — take that at face value.

## B.3 Makepad

| Fact | Value | Source |
|---|---|---|
| `makepad-widgets` version | **1.0.0**, published **2025-05-13** | [crates.io](https://crates.io/api/v1/crates/makepad-widgets) |
| Recent downloads | **~2,153/90d** | [crates.io](https://crates.io/api/v1/crates/makepad-widgets) |
| GitHub | ~6,500 stars, 345 forks, **default branch `dev`**, ~104 open issues | [makepad/makepad](https://github.com/makepad/makepad) |
| GitHub releases | only a single **`pre-alpha`** tag, dated **2018-12-08** | [Releases page](https://github.com/makepad/makepad/releases) |

*"A cross-platform UI runtime for native and web targets"*, *"a Rust-first framework with a scriptable UI DSL"*, self-described as *"an AI-accelerated application development environment for Rust"* ([README](https://github.com/makepad/makepad)).

**Platforms:** macOS (Metal), Windows (DirectX 11), Linux (OpenGL), Web (WebGL/WASM), iOS, tvOS, **Android** ([README](https://github.com/makepad/makepad)). Build commands are first-class:
- Android: `cargo run -p cargo-makepad --release -- android run -p makepad-example-ironfish`
- WASM: `cargo makepad wasm run -p makepad-example-splash --release`, after `cargo makepad wasm install-toolchain`
([README](https://github.com/makepad/makepad))

**Assessment:** genuinely covers all three targets with in-tree tooling, which is rare. But the adoption signal is very weak — **~2.1k downloads per 90 days**, versus egui's ~4.5M — and the crates.io `1.0.0` sits oddly against a releases page whose only tag is `pre-alpha` from 2018 and a `dev` default branch. The README declares no stability level. **Unverified:** I could not retrieve the last-commit date for `makepad/makepad` (GitHub API rate limit was hit); the repo page rendered 1,785 commits on `dev` without a timestamp, so I cannot state current activity precisely.

**Storage story:** none provided.

**Viable for Android?** On paper yes, with the best-integrated tooling of the group. In practice the ecosystem/adoption risk for a project that agents will implement is high — expect to be the one filing the bugs.

## B.4 Iced

| Fact | Value | Source |
|---|---|---|
| Version | **0.14.0**, published **2025-12-07** | [crates.io](https://crates.io/api/v1/crates/iced) |
| Prior release | 0.13.1 (2024-09-19) — i.e. ~15 months between minor releases | [crates.io](https://crates.io/api/v1/crates/iced) |
| Recent downloads | ~606k/90d | [crates.io](https://crates.io/api/v1/crates/iced) |
| Rendering | `iced_wgpu` (Vulkan/Metal/DX12) + `iced_tiny_skia` software fallback | [README](https://github.com/iced-rs/iced) |
| Self-description | ***"Iced is currently experimental software."*** | [README](https://github.com/iced-rs/iced) |

**Android: no official support.** The README's feature list is *"Cross-platform support (Windows, macOS, Linux, and the Web)"* — **Android is absent** ([README](https://github.com/iced-rs/iced)). There is no Android feature flag, no documented target, and no in-tree Android tooling. What exists is community demonstration only: because `winit` and `wgpu` support Android via `android-activity`, people have layered Iced onto a winit+wgpu Android pipeline (e.g. [ibaryshnikov/android-iced-example](https://github.com/ibaryshnikov/android-iced-example)). That is an integration exercise, not a supported platform.

**Web: the situation is worse than the README implies.** The README lists "the Web" as cross-platform support, but the DOM-targeting runtime, **`iced_web`, is at 0.4.0 published 2021-03-31, and its GitHub repository is ARCHIVED** (last push 2022-10-08) ([crates.io iced_web](https://crates.io/api/v1/crates/iced_web), [GitHub API iced-rs/iced_web](https://api.github.com/repos/iced-rs/iced_web)). Web today would mean `iced_wgpu` on WebGL/WebGPU via `wasm32-unknown-unknown`. **Unverified:** I could not find an official, current Iced statement on the maturity of the wgpu-on-wasm path in the 0.14 line; the README's "the Web" claim links to a *desktop screenshot*, which is not encouraging.

**Verdict: NOT viable for Android** as a supported platform, and its web claim is not backed by a maintained runtime. Of the five contenders in this section, Iced is the clearest exclusion for a desktop+web+**Android** project.

## B.5 Tauri 2 with a non-Leptos frontend (Yew / Sycamore / plain HTML+TS)

| Fact | Value | Source |
|---|---|---|
| Tauri version | **2.11.5** (2026-07-01) | [crates.io](https://crates.io/api/v1/crates/tauri) |
| Recent downloads | **~8.6M/90d** — the most-used crate in this whole section | [crates.io](https://crates.io/api/v1/crates/tauri) |
| Mobile support since | Tauri **2.0 stable, 2024-10-02** | [Tauri 2.0 Stable Release](https://v2.tauri.app/blog/tauri-20/) |
| Android target triples | `aarch64-linux-android`, `armv7-linux-androideabi`, `i686-linux-android`, `x86_64-linux-android` | [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) |
| Android toolchain | Android Studio + SDK Platform, Platform-Tools, **NDK (Side by side)**, Build-Tools, Command-line Tools; `JAVA_HOME` must be set | [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) |
| Desktop minimums | macOS Catalina (10.15)+, Windows 7+ | [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) |

**Rendering:** system WebView. On Android that is the Chromium-based Android System WebView, whose version varies per device; Tauri documents how to check it and maintains a webview-versions reference ([Webview Versions | Tauri](https://v2.tauri.app/reference/webview-versions/)). **Unverified:** I could not confirm Tauri 2's minimum Android API level from a Tauri-owned primary source; secondary sources cite API 24 (Android 7.0), and the official [Google Play distribution guide](https://v2.tauri.app/distribute/google-play/) exists but I did not extract a minSdk figure from it.

**Frontend options — Yew and Sycamore are first-class in the scaffolder.** `create-tauri-app`'s Rust UI template menu is exactly:

```
? Choose your UI template ›
Vanilla
Yew
Leptos
Sycamore
```

([create-tauri-app README](https://github.com/tauri-apps/create-tauri-app#readme)). Note the asymmetry: the **scaffolder** ships Yew and Sycamore templates, but the **documentation site's** framework-configuration guides cover Next.js, Nuxt, Qwik, SvelteKit, Vite, **Leptos**, and **Trunk** — Yew and Sycamore are not separately documented there ([Tauri frontend docs](https://v2.tauri.app/start/frontend/)). In practice Yew/Sycamore both build with Trunk, so the Trunk guide covers them.

Frontend crate health:
- **Yew 0.23.0** (2026-03-10), ~320k recent downloads. Note **0.22.1 is yanked**; 0.22.0 was 2025-12-08, and before that 0.21.0 was **2023-09-29** — a two-year gap that has recently closed ([crates.io](https://crates.io/api/v1/crates/yew)).
- **Sycamore 0.9.2** (2025-09-23), **~21.5k** recent downloads — an order of magnitude less used than Yew and two orders less than Leptos (~1.15M) ([crates.io sycamore](https://crates.io/api/v1/crates/sycamore), [crates.io leptos](https://crates.io/api/v1/crates/leptos)).
- **Plain HTML/TS** via Vite is the best-documented path of all and carries zero Rust-frontend risk.

**Storage story — the strongest in this section.** Tauri is the only contender that ships first-party persistence plugins, all current:
- **`tauri-plugin-sql` 2.4.0** (2026-04-04) — *"Interface with SQL databases through [sqlx](https://github.com/launchbadge/sqlx). It supports the `sqlite`, `mysql` and `postgres` drivers"*, with a platform table listing Linux ✓, Windows ✓, macOS ✓, **Android ✓**, iOS ✗ ([README](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/sql/README.md)). Requires Rust ≥ 1.77.2 per that README.
- **`tauri-plugin-store` 2.4.4** (2026-07-18) — *"Simple, persistent key-value store"* ([crates.io](https://crates.io/api/v1/crates/tauri-plugin-store)).
- **`tauri-plugin-fs` 2.5.1** (2026-05-02) ([crates.io](https://crates.io/api/v1/crates/tauri-plugin-fs)).

**But note the seam does not disappear.** `tauri-plugin-sql` gives you SQLite on desktop *and Android* through one API — genuinely solving two of your three targets. It does **not** exist in the browser, because there is no Tauri runtime there. The plain-web build of the same Yew/Sycamore/TS frontend must talk to §3's wasm stack (sqlite-wasm-rs + OPFS worker, or IndexedDB) instead. So this stack converts the problem from "three backends" into "**one Tauri-side backend covering desktop+Android, plus one browser backend**" — which is the cleanest 2-backend split found in this research.

**Viable for Android?** Yes — this is the most production-proven Android path of the five, by download volume and by the existence of an officially-supported, Android-tested storage plugin.

---

## Summary tables

### Storage options by target

| Option | Desktop | Android | Browser (wasm32-unknown-unknown) | Needs C toolchain? |
|---|---|---|---|---|
| `rusqlite` 0.40.1 | ✓ `bundled` | ✓ (`cargo ndk`, maybe `--link-builtins`) | ✓ compiles; **memory-only unless you add `sqlite-wasm-vfs` + Worker** | Yes (native only) |
| `diesel` 2.3.11 | ✓ | ✓ | ✓ same mechanism as rusqlite | Yes (native only) |
| `sqlx` 0.9.0 (sqlite) | ✓ | ✓ (proven by tauri-plugin-sql; open issue #2299) | **✗ rejected upstream 2026-07-02** | Yes |
| `redb` 4.1.0 | ✓ | ✓ (pure Rust, no mmap) | Compiles; **you must write the `StorageBackend`** | No |
| `fjall` 3.1.8 | ✓ | ✓ **only on Rust ≥ 1.98.0** (file-lock fix) | ✗ / unknown | No |
| `sled` 1.0.0-alpha.124 | Beta-quality | Unknown | ✗ | No |
| `turso` 0.7.1 | ✓ | Unknown | ✗ from Rust (issue #5049, backlog) | No |
| Plain append-only file | ✓ | ✓ | ✗ (`std::fs` errors) — needs OPFS | No |

### UI contenders

| Framework | Version | Android | Web | Rendering | Storage provided |
|---|---|---|---|---|---|
| egui / eframe | 0.35.0 (2026-06-25) | ✓ (feature flags; inset + IME rough edges) | ✓ (WebGPU/WebGL2; vendor-documented downsides) | Immediate-mode canvas | Only `localStorage` (~5 MiB) on web |
| Slint | 1.17.1 (2026-07-07) | ✓ Rust-only, via stale `cargo-apk` | ✓ but ***"not recommended for building general-purpose web applications"*** | WebGL `<canvas>` | None |
| Makepad | `makepad-widgets` 1.0.0 (2025-05-13) | ✓ in-tree tooling | ✓ in-tree tooling | GPU 2D/3D | None |
| Iced | 0.14.0 (2025-12-07) | **✗ not supported** (community hacks only) | Claimed; `iced_web` **archived** since 2022 | wgpu / tiny-skia | None |
| Tauri 2 + Yew/Sycamore/TS | tauri 2.11.5 (2026-07-01) | ✓ most proven | Frontend redeploys as static SPA (Tauri itself is not a web target) | System WebView | ✓ `tauri-plugin-sql` incl. **Android** |

---

## Consolidated list of things I could NOT verify

1. **rusqlite issue #503 resolution comments** — the page rendered title/metadata only; the accepted fix is inferred from `cargo-ndk`'s existence and docs, not quoted from that thread.
2. **redb issue #556 (Android crash) root cause** — closed with PR #573, but the comment thread did not render.
3. **SQLite version bundled inside `sqlite-wasm-rs` 0.5.5** — not stated in its README.
4. **fjall on `wasm32-unknown-unknown`** — no statement either way; issue search for "wasm" returns 0 results.
5. **The claim that some Android 14 devices lack the `flock` syscall** (requiring `fcntl(F_SETLKW)`) — sourced only to an internals.rust-lang.org discussion, not to AOSP/Android docs.
6. **Android-specific browser eviction aggressiveness** — no primary source found showing Android Chrome evicts differently from desktop Chrome; BCD records Android as mirroring desktop for all `StorageManager` members.
7. **`fsync` durability on Android device flash** — no primary measurement found.
8. **Tauri 2's minimum Android API level** from a Tauri-owned source (secondary sources say API 24 / Android 7.0).
9. **Makepad's current commit activity** — GitHub API rate-limited; the repo page showed 1,785 commits on `dev` without a date.
10. **Iced 0.14's wgpu-on-wasm maturity** — no current official statement found; the README's "the Web" claim links to a desktop screenshot.
11. **`opfs` crate 0.2.0 internals** — whether it uses `createSyncAccessHandle` or `createWritable`, and hence its true durability/worker requirements.
12. **MSRV for `idb` and `indexed_db_futures`** — not documented on either crate's docs.rs landing page.
