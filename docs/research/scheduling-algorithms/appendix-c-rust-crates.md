# Rust crate landscape: spaced-repetition scheduling algorithms

Research date: **2026-07-26**. All version/download/licence data pulled live from the
crates.io JSON API; commit recency and dependents from the GitHub API; build results from
locally executed `cargo build` / `cargo check` (Rust 1.97.0, cargo 1.97.0).

No files in the leitner-app working tree were modified. Build experiments were run in
throwaway crates under `/tmp/wasmtest` and `/tmp/andtest`.

---

## 0. Headline facts (the two that change the decision)

1. **`fsrs` no longer depends on `burn` (or any ML/tensor framework) at runtime.**
   `burn` was demoted to a `[dev-dependencies]` entry in
   [PR #411](https://github.com/open-spaced-repetition/fsrs-rs/pull/411)
   ("Speed up FSRS inference, evaluation, and training without production Burn dependency",
   merged 2026-06-04, commit `ca31a0d`). First published version without `burn` as a normal
   dependency is **6.3.0** (2026-06-04); 6.2.0 and earlier still had it. Verified per-version
   via `https://crates.io/api/v1/crates/fsrs/<v>/dependencies`.
   The 21-parameter model, inference and gradient-based training are now hand-written over
   `ndarray` (`src/analytic.rs`, `src/training.rs`).

2. **`fsrs` 6.6.1 compiles for both `wasm32-unknown-unknown` and `aarch64-linux-android`,
   including the parameter-training path.** Verified by building locally (details in §1.6).
   A release `cdylib` exposing *both* `next_states` and `compute_parameters`, built with
   `opt-level="z"`, `lto=true`, `strip=true`, `panic="abort"`, is **164 KB of `.wasm`**
   (pre-`wasm-opt`, pre-gzip).

---

## Fact table

| crate | latest version | release date | downloads (total / last 90d) | licence (SPDX as published) | maintenance status |
|---|---|---|---|---|---|
| [`fsrs`](https://crates.io/crates/fsrs) | **6.6.1** | 2026-06-09 | 339,191 / 51,424 | `BSD-3-Clause` | **Actively maintained.** 29 commits in last 90d, 58 in last 365d. Anki depends on it. |
| [`rs-fsrs`](https://crates.io/crates/rs-fsrs) | 1.2.1 | 2024-10-28 | 13,663 / 6,622 | MIT (`license-file`, so crates.io shows `non-standard`) | Repo alive (commits to 2026-06-23) but **no crates.io release in 21 months**. Scheduler only, no optimiser. |
| [`fsrsrs`](https://crates.io/crates/fsrsrs) | 1.1.0 | 2025-01-05 | 3,884 / 27 | `non-standard` | No repository field. 152 LOC. Effectively abandoned. |
| [`sm-2`](https://crates.io/crates/sm-2) | 0.2.0 | 2025-08-18 | 1,366 / 24 | `MIT` | Dormant since 2025-08-18. 199 LOC, 1 GitHub star. Same org as fsrs-rs. |
| [`srs-sm2`](https://crates.io/crates/srs-sm2) | 0.1.1 | 2026-07-21 | 48 / 48 | `MIT OR Apache-2.0` (GitHub detects only Apache-2.0) | **5 days old.** 0 stars, 155 LOC, `no_std`. Unproven. |
| [`spaced-rs`](https://crates.io/crates/spaced-rs) | 0.3.1 | 2022-08-28 | 6,072 / 19 | **`GPL-3.0-only`** | Abandoned (4 years). ⚠️ copyleft |
| [`memo_rs`](https://crates.io/crates/memo_rs) | 1.0.0 | 2023-04-09 | 1,491 / 7 | `MIT` | Abandoned. 95 LOC, single release. |
| [`sra`](https://crates.io/crates/sra) | 0.1.0 | 2021-08-23 | 2,389 / 81 | `MIT OR Apache-2.0` | Abandoned (5 years). 139 LOC. |
| [`spaced-repetition`](https://crates.io/crates/spaced-repetition) | 1.1.0 | 2022-04-16 | 2,800 / 15 | `BSD-2-Clause` | Abandoned. 734 LOC. |
| [`spaced-repetition-rs`](https://crates.io/crates/spaced-repetition-rs) | 0.1.0 | 2022-02-19 | 1,600 / 8 | `BSD-2-Clause` | Abandoned; superseded by `spaced-repetition` (same author). |
| [`ebirsu`](https://crates.io/crates/ebirsu) | 0.2.0 | 2023-08-17 | 4,113 / 15 | **`GPL-3.0-or-later`** | Abandoned. 478 LOC. ⚠️ copyleft |
| [`fitts`](https://crates.io/crates/fitts) | 0.2.1 | 2025-12-28 | 1,455 / 12 | `MIT OR Apache-2.0` | Low activity. Fitts'-Law difficulty + SM-2 intervals. 1,021 LOC. |
| [`learnforge-core`](https://crates.io/crates/learnforge-core) | 0.1.0 | 2026-06-19 | 18 / 18 | `MIT` | Single release, 1 month old. Claims WASM-portable BKT + SM-2. Unproven. |
| [`ebisu`](https://crates.io/crates/ebisu) | 0.0.0 | 2022-01-24 | 1,628 / 7 | `Unlicense` | **Empty name squat — 0 lines of code, 828-byte crate.** |
| [`anki`](https://crates.io/crates/anki) | 0.0.1 | 2019-12-31 | 2,143 / 14 | `AGPL-3.0-or-later` | **Placeholder — 0 LOC.** Description: "This crate is currently only available through the GitHub repo." |
| `leitner` | — | — | — | — | **Does not exist on crates.io** (404). No Leitner-box crate found by any search. |

---

## 1. `fsrs` (repo: `open-spaced-repetition/fsrs-rs`)

### 1.1 Identity, version, licence

- crates.io name is **`fsrs`**, not `fsrs-rs` (`fsrs-rs` and `fsrs-optimizer` are 404 on crates.io).
  <https://crates.io/api/v1/crates/fsrs>
- Latest published: **6.6.1**, published **2026-06-09T06:24:14Z** by `asukaminato0721`.
  Checksum `b8a99ea3dec9af37c9ed3835463ff6820a578a0026a8e370b4c1c0db48dfab3f`, crate size 126,197 bytes, edition **2024**.
- Repo `main` branch already carries `version = "6.6.2"` (unpublished at time of research).
- **Licence: `BSD-3-Clause`** — consistent on crates.io (every version back to 1.0.0) and in
  the GitHub licence detection. Permissive, no copyleft.
- Total downloads **339,191**; last-90-day downloads **51,424**. 73 published versions since 2024-01-27.
- docs.rs builds successfully: <https://docs.rs/crate/fsrs/latest> returns HTTP 200.

### 1.2 Maintenance status

<https://api.github.com/repos/open-spaced-repetition/fsrs-rs>

- 398 stars, 42 forks, created 2023-08-16, `pushed_at` **2026-07-20**, **not archived**.
- **4 open issues** and **4 open PRs** (GitHub's `open_issues_count: 8` combines both).
- Commit volume: **29 commits in the last 90 days**, **58 in the last 365 days**.
  Latest commit on `main`: `87c9a28`, 2026-07-08, "Bump the non-breaking group with 2 updates (#431)".
- Substantive (non-dependabot) work in June/July 2026 includes: #411 burn removal, #419
  "Speed up compute_parameters without SIMD", #420 "Expose training hyperparameters",
  #421 "Support card IDs in time-series evaluation", #424 "Skip validation during parameter
  training", #433 "Fix Cost ADR seed isolation".
- Dependabot is configured and active — dependency bumps land within days.

### 1.3 Anki dependency (strong maintenance evidence) — CONFIRMED

Anki's workspace manifest pins the published crate:

`https://raw.githubusercontent.com/ankitects/anki/main/Cargo.toml`
```toml
[workspace.dependencies.fsrs]
version = "6.6.1"
# git = "https://github.com/open-spaced-repetition/fsrs-rs.git"
# path = "../open-spaced-repetition/fsrs-rs"
```

`rslib/Cargo.toml` line 66: `fsrs.workspace = true`.
`Cargo.lock` line 1404 confirms `name = "fsrs"`, `version = "6.6.1"`,
`source = "registry+https://github.com/rust-lang/crates.io-index"` with the same checksum as
the crates.io release. Anki consumes the *published crate*, not a git pin — so releases are
production-exercised.

Note: only **5 reverse dependencies** are recorded on crates.io
(`flashy`, plus others pinning `^1`, `^1.3.4`, `^5.2.0`, `^6.6.1`). Anki itself is not
published to crates.io, so this number massively understates real-world usage.

### 1.4 Public API surface (read from `src/` at commit `87c9a28`)

Everything below is re-exported from the crate root (`src/lib.rs`). Modules are private;
`Model<B: Backend>` and the `burn` batcher types are `#[cfg(test)]`-gated and **not** public.

**Review-history item type** — `src/dataset.rs`:
```rust
pub struct FSRSItem {
    pub reviews: Vec<FSRSReview>,   // chronological; one FSRSItem per review, carrying its own prefix
}

pub struct FSRSReview {
    pub rating: u32,   // 1=Again 2=Hard 3=Good 4=Easy
    pub delta_t: u32,  // days since previous review; MUST be 0 for the first review
}
```
Both derive `Debug, Clone, Deserialize, Serialize, PartialEq, Eq` (`FSRSReview` also `Copy`).
`FSRSItem::long_term_review_cnt() -> usize` is public. Also public: `filter_outlier(...)`.

**Memory-state type** — `src/inference.rs`:
```rust
pub struct MemoryState {   // Debug, PartialEq, Clone, Copy, Serialize  (note: NOT Deserialize)
    pub stability: f32,
    pub difficulty: f32,
}
```

**The main handle** — `src/model.rs`:
```rust
pub struct FSRS { parameters: [f32; 21] }   // that is the entire struct: 84 bytes, no backend, no device
impl FSRS { pub fn new(parameters: &Parameters) -> Result<Self> }  // &[] => defaults
impl Default for FSRS                                              // DEFAULT_PARAMETERS
pub fn check_and_fill_parameters(parameters: &Parameters) -> Result<Vec<f32>, FSRSError>
pub type Parameters = [f32];                                       // always length 21
pub static DEFAULT_PARAMETERS: [f32; 21];
pub const FSRS5_DEFAULT_DECAY: f32 = 0.5;
pub const FSRS6_DEFAULT_DECAY: f32 = 0.1542;
```

**(a) Scheduling / next intervals** — `src/inference.rs`:
```rust
impl FSRS {
    pub fn next_states(
        &self,
        current_memory_state: Option<MemoryState>,  // None = new card
        desired_retention: f32,
        days_elapsed: u32,
    ) -> Result<NextStates>;

    pub fn next_interval(
        &self,
        stability: Option<f32>,     // None => init stability from rating
        desired_retention: f32,
        rating: u32,
    ) -> f32;                        // note: infallible, returns f32 days (fractional)
}

pub struct NextStates { pub again: ItemState, pub hard: ItemState, pub good: ItemState, pub easy: ItemState }
pub struct ItemState  { pub memory: MemoryState, pub interval: f32 }

pub fn current_retrievability(state: MemoryState, days_elapsed: f32, decay: f32) -> f32;
```

**(b) Optimising / training parameters from history** — `src/training.rs`:
```rust
pub fn compute_parameters(input: ComputeParametersInput) -> Result<Vec<f32>>;   // free function, not a method

pub struct ComputeParametersInput {
    pub train_set: Vec<FSRSItem>,
    pub card_ids: Option<Vec<i64>>,
    pub progress: Option<Arc<Mutex<CombinedProgressState>>>,
    pub enable_short_term: bool,                  // default true
    pub num_relearning_steps: Option<usize>,
    pub training_config: Option<TrainingConfig>,
}
impl Default for ComputeParametersInput { .. }

pub struct TrainingConfig {                       // Default: 5 epochs, batch 512, seed 2023,
    pub num_epochs: usize,                        //          lr 4e-2, max_seq_len 256, gamma 1.0
    pub batch_size: usize,
    pub seed: u64,
    pub learning_rate: f64,
    pub max_seq_len: usize,
    pub gamma: f64,
}

pub struct CombinedProgressState { .. }           // new_shared() -> Arc<Mutex<Self>>, current(), total(), finished()
pub fn benchmark(..) -> ..;                       // testing/quick-validation counterpart; panics instead of Err
```

**(c) Memory state from a review sequence** — `src/inference.rs`:
```rust
impl FSRS {
    pub fn memory_state(&self, item: FSRSItem, starting_state: Option<MemoryState>) -> Result<MemoryState>;
    pub fn memory_state_batch(&self, items: Vec<FSRSItem>, starting_states: Vec<Option<MemoryState>>) -> Result<Vec<MemoryState>>;
    pub fn historical_memory_states(&self, item: FSRSItem, starting_state: Option<MemoryState>) -> Result<Vec<MemoryState>>;
    pub fn historical_memory_state_batch(&self, items: Vec<FSRSItem>, starting_states: Option<Vec<Option<MemoryState>>>) -> Result<Vec<Vec<MemoryState>>>;

    /// SM-2 → FSRS migration path for cards with truncated history
    pub fn memory_state_from_sm2(&self, ease_factor: f32, interval: f32, sm2_retention: f32) -> Result<MemoryState>;
}
```

**Evaluation:**
```rust
impl FSRS {
    pub fn evaluate<F: FnMut(ItemProgress) -> bool>(&self, items: Vec<FSRSItem>, progress: F) -> Result<ModelEvaluation>;
    pub fn universal_metrics<F>(..) -> ..;
}
pub struct ModelEvaluation { pub log_loss: f32, pub rmse_bins: f32 }
pub struct ItemProgress   { pub current: usize, pub total: usize }
pub fn evaluate_with_time_series_splits<F>(..) -> ..;
```

**Simulation** (re-exported from `src/simulation.rs`):
`Card`, `RevlogEntry`, `RevlogReviewKind`, `SimulatorConfig`, `SimulationResult`,
`PostSchedulingContext`, `PostSchedulingFn`, `ReviewPriorityFn`, `CMRRTargetFn`,
`simulate`, `optimal_retention`, `expected_workload`, `expected_workload_with_existing_cards`,
`extract_simulator_config`.

**Errors:** `pub use error::{FSRSError, Result}` (`src/error.rs` is 15 lines; `snafu`-based).

Source size: 11,925 lines across `src/*.rs` including `#[cfg(test)]` modules
(`convertor_tests.rs` 774, `batch_shuffle.rs` 176, `test_helpers.rs` 57 are test-only).
Largest real modules: `simulation.rs` 2,876, `training.rs` 1,721, `cost_adr.rs` 1,632,
`inference.rs` 1,517, `analytic.rs` 978, `model.rs` 878.

### 1.5 Dependencies and build weight

Full `[dependencies]` of 6.6.1 / repo `main`
(<https://raw.githubusercontent.com/open-spaced-repetition/fsrs-rs/main/Cargo.toml>):

```toml
itertools      = "0.15.0"   # 0.14.0 in the published 6.6.1
log            = "0.4"
ndarray        = "0.17.2"
priority-queue = "=2.7.0"
rand           = "0.10.1"
rand_distr     = { version = "0.6.0", optional = true }
rayon          = "1.12.0"
serde          = { version = "1.0.228", features = ["derive"] }
snafu          = "0.9.1"
strum          = { version = "0.28.0", features = ["derive"] }
```

Features:
```toml
[features]
default = []
experimental_cost_adr = ["dep:rand_distr"]   # "APIs may change without semver compatibility guarantees"
```

`[dev-dependencies]`: `burn 0.17.1` (default-features off, features `std, train, ndarray,
sqlite-bundled, metrics`), `chrono`, `chrono-tz`, `criterion`, `csv`, `fern`, `rusqlite`.

**Answers to the specific questions:**

- **Does it pull in a tensor/ML framework?** **No** (as of 6.3.0+). `burn` is dev-only.
  Only `ndarray` — a plain N-d array crate, no autodiff, no GPU, no `build.rs` codegen.
- **Which backend(s)?** N/A. There is no backend abstraction in the public API any more.
  `Model<B: Backend>` and the `burn` imports in `dataset.rs`/`training.rs`/`model.rs` are all
  `#[cfg(test)]`. Historically (≤6.2.0) it was `burn` with the `ndarray` backend.
- **Does a training feature gate away the ML dependency — i.e. can you use scheduling
  without the training dependency?** **The question is now moot: there is no feature gate,
  and none is needed.** `compute_parameters` is unconditionally compiled and adds no extra
  dependencies. The whole crate — scheduling *and* training — sits on the 10 crates above.
  There is *no* way to feature-gate training out, but there is also no cost to it: it is
  pure `ndarray`/`rayon` arithmetic. See §0 for the measured 164 KB wasm figure covering both.
- **Separate crate for optimizer vs scheduler?** **No.** One crate, `fsrs`, contains both
  ("FSRS for Rust, including Optimizer and Scheduler"). There is no `fsrs-optimizer` crate on
  crates.io. `rs-fsrs` is a *scheduler-only* alternative (see §3.1) but is not the optimiser
  half of a pair.

**`rayon` caveat (the one real multi-platform gotcha).** `rayon` is a hard, non-optional
dependency, but it is used in only three places (`grep` over `src/`, excluding tests):

| file | usage | reachable from |
|---|---|---|
| `src/inference.rs:659` | `rayon::spawn` | `evaluate_with_time_series_splits` only |
| `src/simulation.rs:1396` | `.into_par_iter()` | `optimal_retention` / `simulate` machinery |
| `src/cost_adr.rs:438,466,659` | `.par_iter()` / `.into_par_iter()` | `experimental_cost_adr` feature only |

`src/model.rs`, `src/training.rs` and `src/analytic.rs` contain **zero** `rayon` references,
and `inference.rs` has exactly one (in the time-series-split evaluator). So
`next_states`, `next_interval`, `memory_state*`, `current_retrievability` and
`compute_parameters` do **not** touch the rayon thread pool. On `wasm32-unknown-unknown`
without `wasm-bindgen-rayon`, `std::thread::spawn` is unsupported at runtime — so the
*simulation / optimal-retention / time-series-eval* paths are the ones at runtime risk, not
the scheduling or training paths. **This runtime distinction is inferred from static call-site
analysis; I did not execute the wasm binary to confirm it. See open questions.**

### 1.6 wasm and Android support — VERIFIED BY BUILDING

**Build evidence I produced locally** (throwaway crate `/tmp/wasmtest`, `fsrs = "6.6.1"`):

- `cargo build --target wasm32-unknown-unknown` initially fails, but **only** inside
  `getrandom 0.4.3`:
  `error: The wasm32/64-unknown-unknown are not supported by default; you may need to enable the "wasm_js" crate feature`.
  `rayon-core 1.13.0` compiled for wasm32 without complaint.
- With `getrandom = { version = "0.4", features = ["wasm_js"] }` for the wasm target plus
  `RUSTFLAGS='--cfg getrandom_backend="wasm_js"'`, the build **succeeds** — `fsrs v6.6.1`
  and the wrapper compile clean, debug and `--release`.
- The wrapper exercised `FSRS::default().next_states(..)`, `FSRS::memory_state(..)` **and
  `compute_parameters(..)`** — i.e. the training path is wasm-compilable, not just scheduling.
- Release `cdylib` with `opt-level="z"`, `lto=true`, `strip=true`, `panic="abort"`, exporting
  both a scheduling and a training entry point: **164 KB `.wasm`** before `wasm-opt`/gzip.
- `cargo check --target aarch64-linux-android` (after `rustup target add aarch64-linux-android`):
  **passes clean**, scheduling and training entry points included. (`check` needs no NDK
  linker, so this proves source/target compatibility, not final linking.)

**Upstream / documented evidence:**

- **`fsrs-browser` is the org's official wasm packaging** —
  <https://github.com/open-spaced-repetition/fsrs-browser> (52 stars, BSD-3-Clause,
  `pushed_at` 2026-06-14, not archived), published to npm as
  [`fsrs-browser`](https://www.npmjs.com/package/fsrs-browser) **v6.6.0, 2026-06-13,
  BSD-3-Clause**. Its `Cargo.toml` is `crate-type = ["cdylib"]` and depends on
  `fsrs = "6.6.0"`, `getrandom = { version = "0.4", features = ["wasm_js"] }`,
  `wasm-bindgen = "=0.2.100"`, **`wasm-bindgen-rayon = "1.3.0"`**, `rayon = "1.12.0"`,
  `serde-wasm-bindgen`. It re-exports `pub use wasm_bindgen_rayon::init_thread_pool;` —
  independent confirmation that `wasm-bindgen-rayon` is the sanctioned way to make the
  rayon paths work in a browser.
  Its README: *"This project runs fsrs-rs in the browser with support for training FSRS
  parameters"* and *"training 24,394 revlogs on `./dev` takes days, while `./prod.sh` takes
  3.5 seconds"* — so **in-browser parameter training is a supported, benchmarked use case**,
  but only with a release/optimised build.
  `fsrs-browser` versions track `fsrs-rs` major.minor: npm has 2.0.3, 2.0.4, 3.0.0, 4.1.1,
  5.2.0, 6.6.0.
- `fsrs-rs` has a long-lived `fsrs-browser` git branch with a dedicated
  `.github/workflows/auto-merge.yml` ("Auto merge main into fsrs-browser") that fires on
  release — i.e. wasm packaging is wired into the release process.
- Issue [#99 "support WASM"](https://github.com/open-spaced-repetition/fsrs-rs/issues/99) and
  PR [#89 "wasm support"](https://github.com/open-spaced-repetition/fsrs-rs/pull/89) are both
  **closed**.
- **Android:** AnkiDroid's backend
  [`ankidroid/Anki-Android-Backend`](https://github.com/ankidroid/Anki-Android-Backend)
  (88 stars, `pushed_at` 2026-07-23) builds `rslib-bridge` as
  `crate-type = ["cdylib"]` with `anki = { path = "../anki/rslib", features = ["rustls"] }`
  + `jni`. Since Anki's `rslib` depends on `fsrs = 6.6.1`, **`fsrs` ships inside every
  AnkiDroid release as an Android `.so`**. This is the strongest available Android evidence.
- **Not verified:** `fsrs-rs`'s own CI does *not* test wasm or Android. `.github/workflows/`
  contains only `autofix.yml`, `auto-merge.yml`, `check.yml` + `check.sh`; `check.yml` runs
  `build-linux` / `build-macos` (and reportedly Windows) on the host target only.
  So there is no upstream CI gate protecting the wasm/Android builds from regressing.

---

## 2. SM-2 in Rust

### 2.1 Candidates found (exhaustive crates.io search)

Searched `spaced repetition`, `flashcard`, `srs`, `fsrs`, `supermemo`, `leitner`,
`half-life regression`, `memory model` (20 results each) via
`https://crates.io/api/v1/crates?q=...`. Library candidates (CLI/TUI apps such as
`forne`, `carddown`, `speki`, `flashed`, `crablit`, `hashcards`, `vultan`, `deckster`,
`spaced-review`, `melete`, `kbsr`, `reps`, `okul`, `fastcards`, `alix`, `ankr` excluded —
they are binaries, not schedulers you can depend on):

**`sm-2` 0.2.0** — MIT, 2025-08-18, 1,366 downloads (24 in 90d), 199 LOC.
<https://github.com/open-spaced-repetition/sm-2-rs> — **1 star, 0 open issues,
`pushed_at` 2025-08-18** (dormant ~11 months). Same org as fsrs-rs; author `joshdavham`
(also the py-fsrs maintainer). API:
```rust
pub struct Card { pub card_id: u128, pub n: u32, pub ef: f32, pub i: u32,
                  pub due: DateTime<Utc>, pub needs_extra_review: bool }
pub struct ReviewLog { .. }
pub struct Scheduler;   // stateless unit struct
impl Scheduler {
    pub fn review_card(card: &Card, rating: u8, review_datetime: Option<DateTime<Utc>>,
                       review_duration: Option<u32>) -> Result<(Card, ReviewLog), SchedulerError>;
}
pub enum SchedulerError { NotDue }
```
Pulls in `chrono` + `serde` + `serde_json`. Rating scale is 0–5. Includes a
`needs_extra_review` same-day re-ask for ratings 3. `Card::default()` derives `card_id` from
`SystemTime::now()` millis — a wasm/portability smell (works on wasm32 with `js-sys` time,
but no monotonicity or collision protection). **Returns `Err(NotDue)` if you review early**,
which is an opinionated constraint you may not want.

**`srs-sm2` 0.1.1** — `MIT OR Apache-2.0` on crates.io (GitHub detects Apache-2.0 only —
licence metadata is inconsistent), 2026-07-21, 48 downloads, 155 LOC.
<https://github.com/suradet-ps/srs-sm2> — created **2026-07-20 (5 days before this
research)**, 0 stars. Genuinely dependency-free, `#![no_std]`, no clock access:
```rust
pub struct Schedule { pub interval_days: u32, pub ease_factor: f32 }
pub fn schedule_next(card: Schedule, quality: u8) -> Schedule;
pub const MIN_EASE_FACTOR: f32 = 1.3;
```
Excellent shape (pure function, no I/O, no allocation, no time) but **zero track record**.
Docs are unusually good for its age. Would be a fine *reference implementation to copy*.

**`spaced-rs` 0.3.1** — **GPL-3.0-only**, 2022-08-28, 6,072 downloads, 109 LOC.
"a sm2 *inspired* SR algorithm" — not faithful SM-2. Abandoned. Copyleft.

**`memo_rs` 1.0.0** — MIT, 2023-04-09, 1,491 downloads, 95 LOC, single release, abandoned.
"An implementation of supermemo2 for rust".

**`sra` 0.1.0** — `MIT OR Apache-2.0`, 2021-08-23, 2,389 downloads, 139 LOC, single release,
abandoned 5 years. "A collection of spaced repetition algorithms".

**`spaced-repetition` 1.1.0** — BSD-2-Clause, 2022-04-16, 2,800 downloads, 734 LOC,
<https://github.com/ISibboI/spaced-repetition-rs>. "based on anki and supermemo". Abandoned.
(`spaced-repetition-rs` 0.1.0 is the same author's earlier name — superseded.)

**`fitts` 0.2.1** — `MIT OR Apache-2.0`, 2025-12-28, 1,455 downloads, 1,021 LOC.
Fitts'-Law difficulty prediction + SM-2 intervals. Idiosyncratic, low adoption.

**`learnforge-core` 0.1.0** — MIT, 2026-06-19, 18 downloads, 3,199 LOC. Claims
"BKT, SM-2, threshold, microlearning selection, signing, packs — desktop/web/WASM portable".
Single release, one month old, part of a larger product (`agentixgarage/learnforge`) — pulls
in far more than a scheduler (156 KB crate).

### 2.2 Honest assessment: is any production-viable?

**No, none of them is production-viable for a shipped client**, on the evidence:

- The two with real download counts (`spaced-rs` 6,072 and `ebirsu` 4,113) are both
  **GPL-licensed and abandoned**.
- `sm-2` is the most credible (open-spaced-repetition org, MIT, sane API) but has **1 GitHub
  star, 24 downloads in 90 days, and no commits in 11 months**. It is a hobby crate, not
  infrastructure. Its `Err(NotDue)` policy and `SystemTime`-derived IDs would need working
  around.
- `srs-sm2` has the best *design* but is **5 days old with 0 stars** — depending on it is a
  bet on one unknown maintainer.
- Every one of them is 95–200 lines. There is no meaningful volume of hard-won logic to
  inherit, and no test-suite reputation to lean on.

**SM-2's actual size if implemented directly.** The entire algorithm is ~15 lines. From
`sm-2-rs`'s `scheduler.rs` (MIT), the *complete* core is:

```rust
if rating >= 3 {                                    // correct response
    card.ef = (card.ef + (0.1 - (5.0 - rating) * (0.08 + (5.0 - rating) * 0.02))).max(1.3);
    card.i = match card.n { 0 => 1, 1 => 6, _ => (card.i as f32 * card.ef).ceil() as u32 };
    card.n += 1;
} else {                                            // incorrect response
    card.n = 0;
    card.i = 0;                                     // ef deliberately unchanged
}
```

That is it: two state fields (`ease_factor`, `interval_days`) plus a repetition counter, one
EF update formula `EF' = EF + (0.1 - (5-q)(0.08 + (5-q)·0.02))` clamped at 1.3, and the
`1 → 6 → i·EF` interval ladder. Realistically **40–80 lines including a `Rating` enum,
`serde` derives and a table-driven test** — the kind of test vector `sm-2-rs` already
publishes (`ratings [4,3,3,4,5,3,0,1,3,3,4,5,3]` → intervals
`[1,0,0,6,15,0,0,0,0,0,35,85,0]`) and which `srs-sm2` documents in its doctests. Both can be
used as free cross-check oracles without taking a dependency.

**Conclusion for the record:** taking a dependency for SM-2 buys ~50 lines of arithmetic and
costs a supply-chain edge, a licence to audit, and (for `sm-2`) a forced `chrono` +
`serde_json` dependency and an opinionated "not due" error. Writing it directly is the
lower-risk option on the facts. The interesting design questions in SM-2 (same-day
relearning steps, fuzz, leech handling, timezone/day-boundary policy, capping) are **not**
answered by any of these crates anyway — every one of them punts on them.

---

## 3. Other relevant crates

### 3.1 FSRS alternatives

**`rs-fsrs` 1.2.1** — <https://github.com/open-spaced-repetition/rs-fsrs>
(48 stars, 3 open issues, `pushed_at` 2026-07-20, default branch **`master`**).
Licence: `license-file = "LICENSE"` in `Cargo.toml`, so crates.io reports `non-standard`;
the file itself is **MIT ("Copyright (c) 2023 Open Spaced Repetition")** and GitHub detects
MIT. 13,663 downloads (6,622 in 90d), 1,271 LOC.

**Scheduler only — no optimiser/training.** Dependencies are just `chrono` (+ optional
`serde`/`serde_json`) — much lighter than `fsrs`, but you cannot compute personalised
parameters with it. API:
```rust
pub use algo::FSRS;
pub use scheduler::{ImplScheduler, Scheduler};
pub use scheduler_basic::BasicScheduler;
pub use scheduler_longterm::LongtermScheduler;
pub use models::{Card, Rating, RecordLog, ReviewLog, SchedulingInfo, State};
pub use parameters::{Parameters, Seed};
pub use alea::{Alea, AleaState, Prng, alea};
pub use fractional_days::FractionalDays;

pub struct Card { pub due: DateTime<Utc>, pub stability: f64, pub difficulty: f64,
                  pub elapsed_days: i64, pub scheduled_days: i64, pub reps: i32,
                  pub lapses: i32, pub state: State, pub last_review: DateTime<Utc> }
impl Card { pub fn get_retrievability(&self, now: DateTime<Utc>) -> f64 }
pub enum State { .. }  pub enum Rating { .. }
```
Note the different shape from `fsrs`: a *stateful, date-carrying* `Card` (f64 stability /
difficulty) versus `fsrs`'s stateless `FSRS` + `MemoryState` (f32). It has `Alea`/`Prng` for
interval fuzz, which `fsrs` does not expose.
⚠️ **The crate is 21 months stale on crates.io (1.2.1, 2024-10-28) despite an active repo** —
recent repo commits (e.g. #61 "Add FractionalDays trait", 2026-01-04) are **unpublished**.
Also: `#![deny(warnings)]` at crate root, which can break your build on a new compiler.

**`fsrsrs` 1.1.0** — 2025-01-05, 3,884 downloads (27 in 90d), 152 LOC, `non-standard`
licence, **no repository field on crates.io**. Unverifiable provenance; do not use.

### 3.2 Other scheduler families — largely absent from Rust

- **Leitner boxes:** **no crate named `leitner` exists** (crates.io 404), and no crate in any
  of my searches implements a Leitner box scheduler as a library. The only search hit for
  "leitner" was `reps`, a TUI app. **You would be implementing Leitner from scratch.**
- **SM-17 / SM-18:** **no Rust crate found.** These are SuperMemo's proprietary, unpublished
  algorithms — no open specification exists, so this is expected.
- **Ebisu (Bayesian half-life):** the crate name `ebisu` is **squatted by an empty
  placeholder** — v0.0.0, 2022-01-24, **0 lines of code**, 828-byte crate, `Unlicense`,
  <https://github.com/xorshift/ebisu>. No usable Rust Ebisu implementation exists.
  (Upstream Ebisu is Python: `fasiha/ebisu`.)
- **Half-life regression (Duolingo's HLR):** **no Rust crate found.** My "half-life
  regression" search returned only unrelated finance/agent-memory crates.
- **Memory models generally:** `ebirsu` 0.2.0 (GPL-3.0-or-later, abandoned 2023, 478 LOC,
  <https://gitlab.com/antnm/ebirsu/>) is a generic "flashcard quiz scheduling" crate;
  `learnforge-core` includes Bayesian Knowledge Tracing (BKT). Neither is credible
  infrastructure. Most `q=memory model` hits are LLM-agent memory crates
  (`engram-lib`, `alaya`, `pensyve-core`, `kleos-lib`, `mnemo-engine`, `zeph-memory`) —
  unrelated to SRS despite keyword collisions.
- **`anki` on crates.io is a 0-LOC placeholder** (v0.0.1, 2019-12-31, AGPL-3.0-or-later,
  description: "This crate is currently only available through the GitHub repo"). You cannot
  depend on Anki's `rslib` from crates.io — and you would not want to (see §4).

### 3.3 FSRS parameter optimisation outside `fsrs-rs`

- **`fsrs-optimizer` (PyPI) 6.5.0** — <https://github.com/open-spaced-repetition/fsrs-optimizer>.
  `requires_dist`: **`torch`**, `numpy`, `pandas>=3.0.0`, `matplotlib`, `scikit-learn`,
  `scipy`, `statsmodels`, `pytz`, `tqdm`. `license` field is **`None`** and there are **no
  License classifiers** on the PyPI metadata — licence must be read from the repo, not the
  package index. Using this would force a **Python + PyTorch** runtime on your build or
  server — categorically unshippable in a wasm/Android client, and a heavy server dependency.
- **`fsrs` (PyPI, "py-fsrs") 6.3.1** — MIT, © 2022 Open Spaced Repetition. A pure-Python
  scheduler. Same problem: non-Rust runtime.
- **`fsrs-browser` (npm) 6.6.0** — BSD-3-Clause, 2026-06-13. *Not* a non-Rust dependency:
  it is `fsrs` compiled to wasm by the same org, and it **does** expose
  `compute_parameters` + `TrainingConfig` to JS. Relevant as a packaging precedent, or as an
  off-the-shelf answer if your web client is JS-first rather than Rust-first.

**Bottom line:** `fsrs` the Rust crate is the *only* way to optimise FSRS parameters without
introducing a non-Rust runtime. Nothing else in the Rust ecosystem trains FSRS parameters.

---

## 4. Licence implications

| Licence | Crates | Character | Implication for an Android/web client |
|---|---|---|---|
| **`BSD-3-Clause`** | **`fsrs` 6.6.1** (all versions), `fsrs-browser` | Permissive | ✅ No copyleft. Requires retaining the copyright notice + disclaimer in distributed documentation/binaries, and the "no endorsement" clause forbids using "Open Spaced Repetition" or contributor names to promote your app without permission. **No source-disclosure obligation.** Fine for a closed-source Play Store / web app; just ship an attribution/licences screen. |
| **`MIT`** | `sm-2`, `memo_rs`, `rs-fsrs` (via `license-file`), `learnforge-core` | Permissive | ✅ Attribution only. |
| **`MIT OR Apache-2.0`** | `srs-sm2`, `sra`, `fitts` | Permissive, dual | ✅ Rust-ecosystem standard; pick either. |
| **`BSD-2-Clause`** | `spaced-repetition`, `spaced-repetition-rs` | Permissive | ✅ Attribution only. |
| **`GPL-3.0-only`** | **`spaced-rs`** | **Strong copyleft** | ⚠️ **Avoid.** Linking it into your client would oblige you to license the whole app GPL-3.0 and offer corresponding source. Also creates friction with Apple's App Store terms (less so Google Play). |
| **`GPL-3.0-or-later`** | **`ebirsu`** | **Strong copyleft** | ⚠️ **Avoid.** Same as above. |
| **`AGPL-3.0-or-later`** | **`anki`** (0-LOC placeholder) | **Network copyleft** | ⚠️ **Avoid.** Anki's real `rslib` is AGPL-3.0. Do **not** vendor or link Anki's Rust library — AGPL §13 extends the source obligation to users interacting over a network, which is fatal for a hosted/sync web client. The `fsrs` crate is deliberately BSD-3-Clause precisely so it can be reused independently; use `fsrs`, never `rslib`. |
| **`Unlicense`** | `ebisu` (empty) | Public-domain dedication | Irrelevant — 0 LOC. |
| **`non-standard`** | `rs-fsrs`, `fsrsrs`, `flashy` | Unresolved metadata | ⚠️ `rs-fsrs` verified MIT by reading its `LICENSE` file. **`fsrsrs` has no repository and no readable licence — treat as unlicensed / do not use.** Automated licence scanners (`cargo-deny`, `cargo-about`) will flag `non-standard`; you may need an explicit allow-entry if you use `rs-fsrs`. |

**Transitive licences of `fsrs`'s 10 runtime deps** — `itertools`, `log`, `ndarray`,
`priority-queue`, `rand`, `rayon`, `serde`, `snafu`, `strum` are all MIT/Apache-2.0 or
BSD-family by long-standing convention. **I did not individually verify each transitive
licence** — run `cargo deny check licenses` or `cargo about` on the real dependency graph
before shipping. Nothing in the tree looked copyleft during the build.

**Net conclusion on licensing:** the FSRS path is clean. `fsrs` (BSD-3-Clause) has no
copyleft obligation and is safe for a proprietary, distributed Android/web client with a
standard attribution screen. The only copyleft traps in this space are `spaced-rs`,
`ebirsu` (GPL) and Anki's own `rslib` (AGPL) — all avoidable.

---

## 5. Unverified / open questions

1. **Runtime behaviour of `rayon` on `wasm32-unknown-unknown`.** I proved the code
   *compiles* and showed by static call-site analysis that the scheduling and
   `compute_parameters` paths contain no `rayon` calls. I did **not execute** the wasm binary,
   so I cannot certify that `compute_parameters` runs without a thread pool, nor confirm that
   `simulate` / `optimal_retention` / `evaluate_with_time_series_splits` panic or hang on wasm
   without `wasm-bindgen-rayon`. That `fsrs-browser` bundles `wasm-bindgen-rayon` and
   re-exports `init_thread_pool` is strong circumstantial evidence that at least *some* path
   needs real threads. **Worth a 30-minute spike: run `compute_parameters` in
   `wasm32-unknown-unknown` under `wasm-bindgen-test`/node without a thread pool.**
2. **Android link step.** `cargo check --target aarch64-linux-android` passed, which proves
   source compatibility but not final `.so` linking (no NDK installed here). The AnkiDroid
   backend shipping `fsrs` inside `anki/rslib` is strong evidence linking works, but I did not
   produce an Android artifact myself. Other Android ABIs (`armv7-linux-androideabi`,
   `x86_64-linux-android`, `i686-linux-android`) were not tested at all.
3. **`compute_parameters` wall-clock cost on real client hardware.** `fsrs-browser`'s README
   gives one anecdote (24,394 revlogs, 3.5 s in an optimised browser build; "days" in a debug
   build). I have no measurement for a mid-range Android device, and no measurement at all for
   the `--release` vs `opt-level="z"` trade-off I used for the size figure. The debug/release
   gap is enormous — any in-app training must be release-built.
4. **Serde asymmetry.** `MemoryState`, `NextStates` and `ItemState` derive `Serialize` but
   **not `Deserialize`** (`FSRSItem`/`FSRSReview` derive both). If you persist `MemoryState`
   you must define your own local struct or a `Deserialize` shim. I confirmed this from the
   derive attributes but did not check whether it is intentional or an upstream oversight
   worth filing.
5. **fsrs 6.x semver churn.** Nine minor/major versions shipped in the eleven days
   2026-05-29 → 2026-06-09 (6.0.0 … 6.6.1), and `main` already sits at an unpublished 6.6.2.
   The 6.x line is moving fast and 6.0→6.6 included breaking API changes. I did **not**
   enumerate the breaking changes or check for a CHANGELOG. Pin exactly and read release
   notes before bumping. `experimental_cost_adr` is explicitly documented as exempt from
   semver.
6. **`fsrs`'s `priority-queue = "=2.7.0"` is an exact pin.** This will hard-conflict if
   anything else in your graph needs a different `priority-queue` version. Not hit in my test
   build (which had almost no other deps).
7. **Transitive licence audit** not performed per-crate (see §4).
8. **`rs-fsrs`'s unpublished repo state.** Whether the maintainers intend to publish 1.3.x, or
   whether `rs-fsrs` is being wound down now that `fsrs` is dependency-light, is unknown — I
   found no statement either way.
9. **FSRS algorithm-version alignment.** `fsrs` 6.6.1 exposes both `FSRS5_DEFAULT_DECAY` and
   `FSRS6_DEFAULT_DECAY`, and `DEFAULT_PARAMETERS` is 21 values (FSRS-6). I did not verify
   which FSRS spec revision `rs-fsrs` 1.2.1 implements, so the two crates may not be
   parameter-compatible with each other.
10. **crates.io `recent_downloads` semantics** assumed to be a 90-day window (the documented
    convention). Not independently confirmed against the API docs.

---

## Appendix: primary sources

- fsrs crate API: <https://crates.io/api/v1/crates/fsrs> · versions · `/6.6.1/dependencies`
- fsrs-rs repo: <https://github.com/open-spaced-repetition/fsrs-rs> ·
  `main/Cargo.toml` · `main/src/lib.rs` · `main/src/{dataset,inference,model,training}.rs` ·
  `main/README.md` · `.github/workflows/`
- burn removal: <https://github.com/open-spaced-repetition/fsrs-rs/pull/411> (commit `ca31a0d`)
- wasm issue/PR: <https://github.com/open-spaced-repetition/fsrs-rs/issues/99> ·
  <https://github.com/open-spaced-repetition/fsrs-rs/pull/89>
- docs.rs: <https://docs.rs/crate/fsrs/latest> (HTTP 200)
- Anki: <https://github.com/ankitects/anki/blob/main/Cargo.toml> (workspace dep, `version = "6.6.1"`) ·
  `rslib/Cargo.toml:66` · `Cargo.lock:1404`
- AnkiDroid backend: <https://github.com/ankidroid/Anki-Android-Backend> ·
  `rslib-bridge/Cargo.toml`
- fsrs-browser: <https://github.com/open-spaced-repetition/fsrs-browser> ·
  `main/Cargo.toml` · `main/src/lib.rs` · <https://www.npmjs.com/package/fsrs-browser>
- rs-fsrs: <https://github.com/open-spaced-repetition/rs-fsrs> (`master` branch) ·
  `master/Cargo.toml` · `master/LICENSE` · `master/src/{lib,models}.rs`
- sm-2-rs: <https://github.com/open-spaced-repetition/sm-2-rs> · `main/src/{scheduler,card}.rs`
- srs-sm2: <https://github.com/suradet-ps/srs-sm2> · `main/src/lib.rs`
- PyPI: <https://pypi.org/pypi/fsrs-optimizer/json> · <https://pypi.org/pypi/fsrs/json>
- crates.io search: `https://crates.io/api/v1/crates?q=<term>&per_page=20` for
  `spaced repetition`, `flashcard`, `srs`, `fsrs`, `supermemo`, `leitner`,
  `half-life regression`, `memory model`
