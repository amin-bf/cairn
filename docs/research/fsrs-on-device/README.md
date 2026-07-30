# FSRS parameter optimisation on the handset — measured

Evidence for [#20 "Prove FSRS parameter optimisation runs in-client on Android"](https://github.com/amin-bf/leitner/issues/20).
[#2](https://github.com/amin-bf/leitner/issues/2) proved the training path *compiles* for
`aarch64-linux-android`; `cargo check` needs no linker, so nothing about linking, loading or
running was established. This note measures all three on the real device.

**Measured 2026-07-30** on the handset from
[#7](https://github.com/amin-bf/leitner/issues/7): Google Pixel 8 Pro (`husky`), Android 17 /
API 37, `arm64-v8a` only, 4 KB pages. Harness, kept at an archival tag rather than on `main`:
[`prototypes/fsrs-android-bench/`](https://github.com/amin-bf/leitner/tree/prototypes/issue-20/prototypes/fsrs-android-bench),
`fsrs = "=6.6.1"`,
built `--release` with `lto = true`, `codegen-units = 1`, `opt-level = 3`, via
`cargo ndk` on NDK 29.0.13846066.

## The headline

**Optimisation runs in-client on Android, and the cost is not a constraint on the design.** A
decade of the heaviest use this app's own arithmetic contemplates — 730,000 reviews, per
[ADR-0004 §10](../../adr/0004-the-review-event-log.md) — trains in **4.3 seconds** in the
foreground on the handset. A year of heavy use trains in **0.42 s**.

The risk this ticket existed to close is therefore closed: personalised parameters are **not** a
desktop-only feature, and no "Android runs defaults, desktop optimises" fallback is needed.

## What was proven, in order

1. **It links.** A release `aarch64-linux-android` binary and `.so` build through NDK clang.
   `llvm-nm -u` shows no undefined compiler builtins — the `__extenddftf2` class of failure #7
   watched for does not appear here either, and `--link-builtins` was not needed. Every LOAD
   segment is `align 0x4000`, so the 16 KB page requirement is satisfied at link time.
2. **It loads.** `System.loadLibrary("fsrsbench")` succeeds in an app process
   (`primaryCpuAbi=arm64-v8a`), and the JNI entry point returns its report to Java.
3. **It runs, and produces real parameters** — a 21-weight vector that differs from the
   published defaults, on every corpus size, on device.

## Wall clock and memory

Corpus sizes are anchored to [ADR-0004 §10](../../adr/0004-the-review-event-log.md): heavy use is
200 reviews/day ≈ 73,000 a year, so 730,000 is a decade of it. `train` is `compute_parameters`
alone, excluding corpus generation.

### Under `adb shell` (a clean witness — nothing else in the process)

| Items | Scale | `train` | Peak RSS | Threads |
|---|---|---|---|---|
| 5,000 | a few months, typical use | **0.06 s** | 6.2 MB | 1 |
| 20,000 | ~1 year, typical use | **0.15 s** | 12.8 MB | 1 |
| 73,000 | 1 year, heavy use | **0.42 s** | 33.3 MB | 1 |
| 250,000 | ~3.5 years, heavy use | **1.48 s** | 105.4 MB | 1 |
| 730,000 | **a decade of heavy use** | **4.29 s** | 344.1 MB | 1 |

Repeating the 730,000 case three times back to back gives **4.30 / 4.25 / 4.28 s** — no thermal
drift, with the thermal HAL reporting 54–57 °C and no throttling status on any sensor. The
measurement is stable, not a lucky first run.

### Inside a foreground app process

| Items | `train` (in-app) | vs shell | Peak RSS (whole process) |
|---|---|---|---|
| 5,000 | 0.05 s | — | 116.5 MB |
| 73,000 | 0.46 s | +0.04 s | 204.5 MB |
| 730,000 | **4.26 s** | −0.03 s | 502.8 MB |

**An app process is not penalised while it is in front.** The RSS figures are larger only because
they include the ~110 MB an empty Android app process costs before any of our work; the *delta*
tracks the shell figures.

For reference, the same 730,000-item run on the development desktop (x86_64) is **2.73 s**. The
handset is **1.6× slower than a desktop**, not an order of magnitude.

## The optimiser is single-threaded, and that is now measured rather than inferred

[#2](https://github.com/amin-bf/leitner/issues/2) inferred from call-site analysis that
`compute_parameters` never reaches the `rayon` thread pool. The shell runs confirm it at runtime:
the process holds **exactly one worker thread** throughout training (the harness's own sampling
thread is the only other one). `rayon` is linked but never scheduled onto, and no thread pool
needs initialising.

Two consequences. The 4.3 s figure uses **one core of nine** and would not improve with a bigger
device unless upstream parallelises training. And the concern that made the web target's
`wasm-bindgen-rayon` question look threatening never applied to this path at all — which is moot
for the web build, ruled out of scope by [ADR-0007 §1](../../adr/0007-the-local-store.md), but is
worth recording as a settled fact about the training path itself.

## The real constraint is not CPU — it is that Android freezes a backgrounded app

The one place the measurement turns hostile. Running the same ladder in an app that is sent to the
background with HOME:

- The process is moved to the **`/background` cpuset** (little cores), where it burns roughly
  **13× the CPU time** for the same work — 62 s of CPU for a job that costs 4.7 s in front.
- Then Android **freezes it outright**. `dumpsys activity processes` reports `isFrozen=true`, and
  `/proc/<pid>/stat` shows `utime` stopped dead — unchanged across repeated samples minutes apart.
  Training does not slow down; it **stops**.
- It is a freeze and not a hang: bringing the activity back to the foreground resumed the run,
  which then completed needing only 0.83 s of further CPU. The 730,000-item case reported
  **303 s of wall clock** for 4.3 s of work, nearly all of it suspended.

**So the cost that constrains the design is a scheduling-and-UX cost, not a compute cost.** Four
seconds of foreground work is nothing; four seconds of work that may be suspended indefinitely the
moment the user switches apps is a thing the spec has to have an answer for. Nothing here is
specific to this workload — it is how Android treats any long native job in a backgrounded process
— but it lands on this feature first.

## ABI coverage

The `.so` links for **all four** Android ABIs — `arm64-v8a`, `armeabi-v7a`, `x86`, `x86_64` —
each producing genuine Android ELF. ABI selection is therefore a packaging decision, not a
technical constraint, and nothing in the training path restricts it.

Only `arm64-v8a` was **executed**: the test handset is 64-bit only (#7), so the 32-bit ABIs are
proven to build and not proven to run. The emulator could exercise `x86_64` if a reason to care
ever appears.

## What this does *not* establish

Stated plainly, because the numbers above are otherwise easy to over-read.

- **The corpus is synthetic.** Card histories are generated by scheduling with the published
  defaults while drawing each grade against a deliberately different "true" learner model. That
  makes the *shape* of the data realistic — real intervals, real lapse-and-relearn sequences, the
  same one-item-per-review expansion the upstream converter performs — and it makes the **cost**
  measurements sound, since cost is driven by item count and history length. It is not real
  human review data.
- **It says nothing about whether optimising is worth it.** The harness reports a log-loss
  comparison, and on this synthetic corpus optimised and default parameters score within 0.0001 of
  each other — an artefact of a generator whose learner differs from the defaults only in initial
  stability, not evidence about real collections. The real-data answer already exists and is not
  improved on here: **0.3629 default vs 0.3437 optimised**, from
  [#2's findings](../scheduling-algorithms/README.md).
- **One device, one Android version.** A Pixel 8 Pro on API 37 is a fast phone. A five-year-old
  budget device was not measured; the 1.6×-slower-than-desktop ratio should not be assumed to hold
  across the fleet. The 4 KB page caveat from #7 also still applies.

## Consequences for the map

- **[#20](https://github.com/amin-bf/leitner/issues/20)'s worst case is dead.** Optimisation is
  not desktop-only, so the "parameters differ per device" divergence flagged for
  [#9](https://github.com/amin-bf/leitner/issues/9) never arises from a *capability* gap. The
  structural answer in [ADR-0001 §6](../../adr/0001-scheduling-algorithm-and-grade-scale.md) —
  one device optimises and publishes 84 bytes, the others consume it — still stands, and is now a
  convenience rather than a necessity.
- **A new decision is owed**: *when* optimisation runs, and what the user sees while it does. The
  freezer finding makes this sharp — a fire-and-forget background thread is not an option — and it
  was not settled by ADR-0001, which fixed only where the parameter vector lives.
