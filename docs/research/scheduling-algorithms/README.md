# Scheduling algorithms: SM-2, FSRS, graded Leitner

Research resolving [Research: scheduling algorithms — SM-2, FSRS, graded Leitner](https://github.com/amin-bf/cairn/issues/2),
a ticket on [Map: local-first Leitner app spec](https://github.com/amin-bf/cairn/issues/1).

**Research date: 2026-07-26.** Every version and status claim was verified against a live primary
source on that date, not from model memory.

**This document gathers facts; it does not choose.** The choice belongs to
[Decide: scheduling algorithm and grade scale](https://github.com/amin-bf/cairn/issues/5). Where
this document says "note that", it is flagging a constraint the decision must account for, not
recommending an outcome.

Appendices hold the full evidence with per-claim source URLs:

- [Appendix A — SM-2 and FSRS](./appendix-a-sm2-and-fsrs.md): algorithm text, formulas, Anki source, parameter counts, replay analysis.
- [Appendix B — Leitner boxes as a UI](./appendix-b-leitner-boxes.md): the original system, the graded-mapping search, shipped-app behaviour, the documented failure mode.
- [Appendix C — Rust crate landscape](./appendix-c-rust-crates.md): crates.io/GitHub fact tables, `fsrs` API surface, build verification, licences.

---

## Executive summary

1. **SM-2 is fully specified, tiny, and 39 years old.** The published algorithm is four state fields
   and two formulas ([Appendix A §1–2](./appendix-a-sm2-and-fsrs.md)). It is ~15 lines of code.
2. **"Ease hell" is Anki's bug, not SM-2's.** Anki replaced SM-2's bounded `f(EF,q)` ease update with
   fixed additive deltas and no mean reversion. Criticism of "SM-2" in the Anki community is usually
   criticism of Anki's variant ([Appendix A §3a](./appendix-a-sm2-and-fsrs.md)).
3. **FSRS has no minimum-review threshold any more, and defaults work from zero history.** The
   1000-review and 400-review figures that circulate are historical; Anki's own FAQ says the optimizer
   works with any number of reviews as of Anki 24.06.3. Default parameters were fitted on ~727M reviews
   from ~10k users and degrade gracefully ([Appendix A §5](./appendix-a-sm2-and-fsrs.md)). **This
   removes the main practical argument for shipping SM-2 first and migrating later.**
4. **FSRS is still opt-in in Anki, not the default** — verified against Anki `main` at 2026-07-25 two
   independent ways. Secondary write-ups frequently get this wrong ([Appendix A §7](./appendix-a-sm2-and-fsrs.md)).
5. **There is no published mapping of a 4-point grade scale onto box movement. Graded Leitner is
   always invented per project.** Stronger than mere absence: both academic treatments of Leitner
   deliberately collapse grades to binary, and the reference open-source implementation is binary and
   treats even the *demotion* rule as a config flag ([Appendix B §2](./appendix-b-leitner-boxes.md)).
6. **The box-display-is-a-lie failure mode is documented in a peer-reviewed paper by the vendor who
   hit it** — Duolingo, ACL 2016 ([Appendix B §4.1](./appendix-b-leitner-boxes.md)). This is direct
   evidence for the map's standing constraint 4, not speculation.
7. **`fsrs` (the Rust crate) is production-grade, BSD-3-Clause, and no longer pulls in an ML
   framework.** It compiles for `wasm32-unknown-unknown` and `aarch64-linux-android`, training path
   included; 164 KB of wasm for a build exporting both scheduling and training
   ([Appendix C §0–1](./appendix-c-rust-crates.md)).
8. **No production-viable SM-2 crate exists, and no Leitner crate exists at all.** Both would be
   written by hand — SM-2 is ~50 lines ([Appendix C §2–3](./appendix-c-rust-crates.md)).
9. **Pure replay works for both algorithms' cores, but a specific, enumerable list of things breaks
   it** — chiefly load balancing, calendar-dependent adjustments, day-boundary/timezone config, and
   the parameter vector itself ([§5](#5-data-requirements-what-the-review-log-must-record) below).

---

## 1. SM-2

Source: Woźniak's own published text, <https://super-memory.com/english/ol/sm2.htm>, fetched and
quoted verbatim in [Appendix A §1](./appendix-a-sm2-and-fsrs.md).

### The algorithm

```
I(1) = 1 day
I(2) = 6 days
I(n) = I(n-1) * EF          for n > 2, rounded up to whole days

EF  initial 2.5, floor 1.3
EF' = EF + (0.1 - (5-q) * (0.08 + (5-q) * 0.02))     applied after every repetition
    ≡ EF - 0.8 + 0.28q - 0.02q²

grades q ∈ 0..5   (5 perfect … 0 complete blackout)
q < 3  ⇒  restart the interval ladder at I(1); the reset itself does not touch EF
```

Per-grade EF deltas: q=5 → +0.10, q=4 → 0 (fixed point), q=3 → −0.14, q=2 → −0.32, q=1 → −0.54,
q=0 → −0.80.

### State per card

`ef` (f32, init 2.5, floor 1.3), `n` (repetition count), `interval` (days — required because `I(n)`
is recursive), and `due`/`last_review`. **That is all.** The published algorithm has no lapse counter,
no leech handling, no sub-day scheduling, and no notion of lateness.

### Weaknesses — and precisely whose they are

| Property | SM-2 as published | Anki's variant |
|---|---|---|
| Ease update | bounded quadratic `f(EF, q)`; q=4 is a fixed point | fixed additive deltas: Again −0.20, Hard −0.15, Easy +0.15, floor 1.3, **no mean reversion** |
| Grade scale | 6 points (0–5) | 4 buttons, one failure option |
| Sub-day learning | none | configurable learning steps (default `1m 10m`) and relearning steps (default `10m`) |
| Lateness | ignored | `good = (ivl + days_late/2) * EF` |
| Lapses | no counter | `lapses` counter, leech tag/suspend at 8 |
| Fuzz | none | on for intervals ≥ 2.5 days, range-dependent |
| Load balancing | none | **on by default** |

**"Ease hell" is caused by the Anki row, not the SM-2 row.** Repeated `Again`/`Hard` walk ease
monotonically down to the 1.3 floor with nothing pushing it back up, after which intervals only ever
grow ×1.3. The term itself appears in the Anki manual, which also states FSRS does not have the
problem. FSRS's difficulty update adds mean reversion explicitly to avoid it. Full mechanism and
source lines in [Appendix A §3a](./appendix-a-sm2-and-fsrs.md).

A second, separate weakness of SM-2 proper: it is a **1987–1989** algorithm with no published
empirical validation against modern benchmarks, and it models difficulty with a single scalar that
doubles as the interval multiplier.

---

## 2. FSRS

Sources: the FSRS algorithm wiki (`open-spaced-repetition/awesome-fsrs`), `fsrs-rs` source at commit
`87c9a28`, Anki `main` at 2026-07-25, `srs-benchmark`. See
[Appendix A §4–8](./appendix-a-sm2-and-fsrs.md).

### The memory model

Three variables per card, collectively the **memory state**:

- **S — stability**: days for recall probability to fall from 100% to 90%.
- **D — difficulty**: how hard it is to *increase* stability, clamped to [1, 10].
- **R — retrievability**: current recall probability, a function of elapsed time and S.

FSRS-6 uses a **power-law** forgetting curve with a *trainable* decay exponent (`w20`, default
0.1542) — not exponential; that was FSRS v3:

```
R(t, S) = (1 + factor · t/S)^(-w20),   factor = 0.9^(-1/w20) - 1   (so that R(S,S) = 0.9)
```

S and D update only on review; R changes continuously. FSRS-6 uses **all** reviews including
same-day ones (`delta_t == 0` takes a separate short-term stability path). Full update formulas,
transcribed from `fsrs-rs/src/model.rs`, are in [Appendix A §4](./appendix-a-sm2-and-fsrs.md).

### Parameters and optimisation

- **FSRS-6: 21 trainable parameters.** Historical counts: v1=7, v4=17, FSRS-5=19, FSRS-6=21.
- "Optimisation" is gradient-descent fitting of those 21 weights to minimise log-loss on the recall
  outcomes implied by *your own* review log.
- Recommended cadence (Anki FAQ): monthly, or every time the review count doubles.

### The minimum-review question — the historical answer is wrong

> "That was the case in earlier versions of Anki. In **Anki 24.06.3** (and newer versions), the
> optimizer can be used with any number of reviews."
> — <https://faqs.ankiweb.net/frequently-asked-questions-about-fsrs.html>

Historical thresholds were 1000 reviews (pre-24.04) and 400 (24.04); the 400 figure survives only as
a stale comment in Anki source. Current Anki has **no numeric threshold in the UI**. The only
surviving guidance is qualitative — the optional health check flags "less than a few hundred"
reviews.

### Zero history

**FSRS works from a cold start and degrades gracefully.** `FSRS::new(&[])` substitutes
`DEFAULT_PARAMETERS`; the defaults were fitted by running the optimizer on **several hundred million
reviews from ~10k users** and the project states plainly that even with defaults FSRS beats SM-2.
Quantitatively, the benchmark's 0-trainable-parameter row sits at log-loss 0.3629 versus 0.3437
optimised — worse, same league. Anki additionally has a `historical_retention` knob (default 0.9) to
fill gaps in imported history, and a `memory_state_from_sm2` conversion for cards that only have
SM-2 `interval`/`ease`.

### Version and status

- **Shipped production version is FSRS-6.** `fsrs-rs` published 6.6.2 (2026-07-08); Anki pins 6.6.1.
- **FSRS-7 exists but only in the research benchmark repo** — 35 parameters, dual power-law curve,
  fractional intervals. Not in `fsrs-rs`, not in `py-fsrs`, not in Anki, and **no primary write-up of
  its formulas exists yet**. Nothing suggests it lands in Anki soon.
- **FSRS is opt-in in Anki, not the default.** Verified two ways against Anki `main` @ 2026-07-25:
  the manual calls it "an alternative to Anki's legacy SuperMemo 2 (SM-2) algorithm", and
  `BoolKey::Fsrs` falls through to `false` with no code path enabling it for new collections.
  Available since Anki/AnkiMobile 23.10, AnkiDroid 2.17+.
- Licences: **`fsrs-rs` BSD-3-Clause**; `py-fsrs` and `fsrs4anki` MIT; Anki itself AGPL-3.0-or-later.

### Grade scale

Four ratings: 1 Again, 2 Hard, 3 Good, 4 Easy. **Hard is a passing grade** — it takes the success
branch with a penalty multiplier (`w15`). The Anki manual is emphatic that this is the one habit FSRS
cannot adapt to: pressing Hard when you actually failed inflates every interval. FSRS-6 has **no
lapse-count term** in the model at all; post-lapse stability depends on D, S and R, and is capped so
a lapse can never raise stability.

---

## 3. Graded Leitner

Full evidence: [Appendix B](./appendix-b-leitner-boxes.md).

### The original specifies capacity, not intervals

The best available description of Leitner's own system (5 partitions of **1, 2, 5, 8, 14 cm**) says
review is triggered when *"a partition became full"* — a capacity/backpressure rule. Intervals are
**emergent** from box geometry and study rate, not specified. The German description gives a third
mechanism again: proportional sampling (all of box 1, ½ of box 2, ¼ of box 3 per session) with a
minimum 8-hour session gap.

Two caveats that matter:

- **Leitner's book was not read.** Every claim about "the original" is second-hand, and the specific
  passages relied on are *uncited* Wikipedia prose on an article carrying a "more citations needed"
  tag. Publication year is genuinely disputed in citable literature (1970 / 1972 / 1974).
- **The famous 1-2-4-8-16 day table is not Leitner's.** It traces to C. A. Mace, *The Psychology of
  Study* (1932) — 40 years earlier. Duolingo's ACL 2016 figure captioned "The Leitner System" uses
  exactly Mace's schedule. This is the clearest documented case of Leitner folklore.

### There is no published 4-point → box mapping

The answer to the ticket's question is a plain **no**, and the evidence is stronger than absence:

- **Reddy et al., KDD 2016** (the "Leitner queue network" paper) had Mnemosyne's 0–5 grades available
  and threw them away: *"We discretize grades into binary outcomes, where recall ≜ grade ≥ 2."* Their
  box rule is binary, one step in each direction. They also state flatly that *"all existing schemes
  for assigning review frequencies to decks in the Leitner system … are based on heuristics with no
  formal optimality guarantees."*
- **Settles & Meeder, ACL 2016** (Duolingo) formalise Leitner as a two-feature binary special case of
  half-life regression: `half_life = 2^(#correct − #incorrect)`. No place for Hard or Easy.
- **`open-spaced-repetition/leitner-box`**, from the same org that maintains FSRS, ships
  `Rating.{Fail, Pass}` only — and makes even *demotion* a config flag (`on_fail: "first_box" |
  "prev_box"`), i.e. the canonical implementers consider the rule undetermined by the source material.
- A Java library implementing Leitner, SM-2 and FSRS side by side keeps a 4-point rating for FSRS and
  gives Leitner a bare `boolean`.
- **The two graded implementations found in real code contradict each other:**

  | | Again | Hard | Good | Easy |
  |---|---|---|---|---|
  | `yro7/panglot-public` (Rust) | box 0 (reset) | **demote one** | promote one | **same as Good** |
  | `SouichiroTsujimoto/xanki` (spec) | relearning phase | **box unchanged** | box +1 | **box +2** |

  Hard is a demotion in one and a no-op in the other; Easy is information-free in one and a
  double-promotion in the other.
- Woźniak's "Normalized Leitner" — the one attempt at a normative spec — is also binary, and
  explicitly calls one-step-back demotion an *"incorrect mutation"* of the system.

**Conclusion: if this project defines an Again/Hard/Good/Easy → box mapping, it is inventing one, and
the spec should say so outright rather than implying a pedigree.**

### What shipped apps that show boxes actually do

| App | Box/level shown? | Underlying scheduler | Display ↔ state |
|---|---|---|---|
| **Anki** | No box. New/Learning/Review/Relearn phases; Young/Mature | SM-2 variant, or FSRS (opt-in) | Young/Mature is a **pure interval threshold at 21 days** |
| **WaniKani** | **Yes — 9 named stages, prominently** | stage → fixed interval table | **1:1 — the stage *is* the state** |
| **Duolingo 2012–16** | Yes — 4-bar strength meters | Leitner variant | projection; **documented as failing** |
| **Duolingo 2016–18** | Yes — same meters | half-life regression | bars = predicted recall probability |
| **Duolingo 2018+** | Crowns | progress counter | **decoupled from memory state by design** |
| **Memrise** | Levels | 8-rung fixed ladder, reset on failure | effectively a real box system |
| **Mnemosyne** | No box, 0–5 grades | SM-2 variant | n/a |
| **Quizlet Learn** | Rounds/progress | undisclosed ML | **could not verify** |

Three findings worth carrying into the decision:

1. **Anki's learning/relearning steps *are* a Leitner ladder** — "Good moves to the next step, Again
   goes back to the first step" — while its review phase is a continuous interval. So Anki is a
   bounded Leitner ladder up front and an interval scheduler afterwards.
2. **WaniKani is the trustworthy-box counter-example, and it runs the arrow the other way**: stage is
   primary state, interval is derived from stage. Its demotion is the only formula-driven graded box
   rule found in any shipped app — and its "grade" is error count within a review, not self-report.
3. **Duolingo's eventual resolution to the box-vs-memory tension was to stop claiming the level means
   memory state at all.**

### The failure mode is documented, in a refereed venue, by the vendor

> "In fact, when it first launched, Duolingo used a variant similar to Figure 3 to manage skill meter
> decay and practice. **The present research was motivated by the need for a more accurate model, in
> response to student complaints that the Leitner-based skill meters did not adequately reflect what
> they had learned.**"
> — Settles & Meeder, ACL 2016, <https://research.duolingo.com/papers/settles.acl16.pdf>

Quantitatively, Leitner's mean absolute error on predicted recall was **0.235 vs HLR's 0.128** — the
display was roughly twice as far from the truth. The same paper records the behavioural cost: users
optimised for the meter, practising *"just to keep the tree gold"* while feeling the practice
reviewed the wrong things.

A second, mechanically distinct instance from Anki: **a displayed interval can decrease after a
passing grade** (fuzz plus the `good ≥ hard + 1` minimum-gap constraint). Any box number derived from
a *displayed interval* inherits that non-monotonicity and can go **down** after a successful review.

### If a box must be derived, the candidate derivations disagree

| Derivation | Form | Source |
|---|---|---|
| Duolingo HLR (published) | `interval ∝ 2^box`, `box = log₂(half_life)` | Settles & Meeder, ACL 2016 |
| Woźniak "Normalized Leitner" | `box = log_EF(interval / Int1)`; conceptually box discretises **stability** | supermemo.guru |
| Reddy et al. (empirically fitted, 859k reviews) | `interval ∝ box` — **linear** | KDD 2016 |
| Anki Young/Mature (shipped) | threshold: two buckets at 21 days | Anki manual |

The only *empirically fitted* relationship is linear; the two normative ones are exponential. No
paper reconciles this. Note also Woźniak's framing — *"well known cards are shunted to boxes
corresponding with higher memory stability"* — which points at discretising **stability** rather than
the scheduled interval. Stability is monotone under successful review; the scheduled interval
demonstrably is not (see the Anki case above).

**There is no canonical box count.** 3, 5, 8, 9 and 12 all appear in real sources. Only two
Leitner repos on GitHub clear 20 stars, so there is no authoritative open-source convention to copy.

---

## 4. Rust availability

Full fact tables, API surface and build evidence: [Appendix C](./appendix-c-rust-crates.md).

### `fsrs` — the crate is called `fsrs`, not `fsrs-rs`

| | |
|---|---|
| crates.io | `fsrs` **6.6.1**, published 2026-06-09 (`fsrs-rs` and `fsrs-optimizer` are 404) |
| Downloads | 339,191 total / 51,424 in 90 days |
| Licence | **BSD-3-Clause** — permissive, attribution + no-endorsement, **no source-disclosure obligation** |
| Maintenance | 29 commits in 90 days, 4 open issues, not archived, active dependabot |
| Production use | **Anki pins the published crate** (`version = "6.6.1"` in its workspace manifest, matching `Cargo.lock` checksum) |
| Scope | **One crate holds both scheduler and optimizer.** There is no separate optimizer crate. |

**Two facts that overturn common assumptions:**

1. **`burn` is gone.** As of 6.3.0 (2026-06-04), `burn` is a *dev-dependency only*; training was
   rewritten with hand-rolled analytic gradients over `ndarray`. The 10 runtime dependencies are
   `itertools, log, ndarray, priority-queue, rand, rayon, serde, snafu, strum`. There is no feature
   gate separating training from scheduling — and none is needed, because training costs no extra
   dependencies.
2. **It builds for both target platforms, training path included** (verified locally, not inferred):
   `wasm32-unknown-unknown` builds clean once `getrandom` has the `wasm_js` feature; `cargo check
   --target aarch64-linux-android` passes. A release `cdylib` exporting *both* `next_states` and
   `compute_parameters` is **164 KB of `.wasm`** at `opt-level="z"` with LTO and strip, before
   `wasm-opt`.

Multi-platform corroboration: `fsrs-browser` (npm, BSD-3-Clause, v6.6.0) is the org's official wasm
packaging and does **in-browser parameter training** — 24,394 revlogs in 3.5 s in an optimised build,
"days" in a debug build. AnkiDroid ships `fsrs` inside an Android `.so` via Anki's `rslib`.

### API surface (real names)

```rust
pub struct FSRSReview { pub rating: u32, pub delta_t: u32 }   // that is the entire model input
pub struct FSRSItem   { pub reviews: Vec<FSRSReview> }        // one card's whole ordered history
pub struct MemoryState { pub stability: f32, pub difficulty: f32 }
pub struct FSRS { /* [f32; 21] — 84 bytes, no backend, no device */ }

impl FSRS {
    fn next_states(&self, Option<MemoryState>, desired_retention: f32, days_elapsed: u32) -> Result<NextStates>;
    fn next_interval(&self, Option<f32>, desired_retention: f32, rating: u32) -> f32;
    fn memory_state(&self, FSRSItem, starting_state: Option<MemoryState>) -> Result<MemoryState>;
    fn historical_memory_states(&self, FSRSItem, Option<MemoryState>) -> Result<Vec<MemoryState>>;
    fn memory_state_from_sm2(&self, ease_factor: f32, interval: f32, sm2_retention: f32) -> Result<MemoryState>;
    fn evaluate<F>(&self, Vec<FSRSItem>, F) -> Result<ModelEvaluation>;
}
pub fn compute_parameters(input: ComputeParametersInput) -> Result<Vec<f32>>;   // free fn, not a method
```

`FSRS::new(&[])` and `FSRS::default()` both give the fitted default parameters.

### Caveats to carry into the decision

- **`rayon` is a hard dependency**, but call-site analysis shows it is reached only from `simulate`,
  `optimal_retention`, `cost_adr` and time-series evaluation — **not** from scheduling or
  `compute_parameters`. That `fsrs-browser` bundles `wasm-bindgen-rayon` is evidence some path needs
  real threads. *Not runtime-verified* — worth a short spike before relying on in-app training on wasm.
- **`MemoryState` derives `Serialize` but not `Deserialize`.** Persisting it needs a local shim.
- **`priority-queue = "=2.7.0"` is an exact pin** — will hard-conflict with any other consumer.
- **6.x moves fast**: nine versions in the eleven days 2026-05-29 → 2026-06-09, with breaking API
  changes inside the 6.x line. Pin exactly.
- `rs-fsrs` (MIT, scheduler-only, `chrono` and little else) is a lighter alternative but is **21
  months stale on crates.io** despite an active repo, and has no optimizer. `fsrsrs` has no
  repository and no readable licence — treat as unlicensed.

### SM-2 and Leitner in Rust: nothing worth depending on

- **No production-viable SM-2 crate.** The two with real download counts (`spaced-rs` 6k,
  `ebirsu` 4k) are **GPL and abandoned**. `sm-2` (MIT, same org as fsrs) has 1 GitHub star, 24
  downloads in 90 days, no commits in 11 months, forces `chrono` + `serde_json`, and returns
  `Err(NotDue)` if you review early. `srs-sm2` (MIT/Apache, `no_std`, pure function) has the best
  shape but was **5 days old** at research time with 0 stars. All are 95–200 LOC.
- **Writing SM-2 directly is the lower-risk option on the facts**: ~15 lines of core arithmetic,
  40–80 with a `Rating` enum, serde and tests. Both crates above publish usable test vectors that can
  serve as free cross-check oracles without taking the dependency.
- **No Leitner crate exists** — `leitner` is a 404 on crates.io and no crate in any search implements
  a box scheduler as a library. Leitner would be written from scratch.
- **Nothing exists in Rust** for SM-17/18 (proprietary and unpublished), Ebisu (the crate name is a
  0-LOC squat), or half-life regression.
- **`fsrs` is the only way to optimise FSRS parameters without introducing a non-Rust runtime.** The
  Python `fsrs-optimizer` requires PyTorch.

### Licences

Clean path: **`fsrs` BSD-3-Clause** — fine for a distributed Android/web client with an attribution
screen. Traps to avoid: `spaced-rs` (GPL-3.0-only), `ebirsu` (GPL-3.0-or-later), and above all
**Anki's own `rslib` (AGPL-3.0)** — §13 network copyleft would be fatal for a synced client. `fsrs`
is deliberately BSD so it can be reused independently of Anki. A transitive licence audit
(`cargo deny check licenses`) has not been run.

---

## 5. Data requirements: what the review log must record

This is the section that feeds **standing constraint 1** on the map ("the log records raw grades and
timestamps, never only derived state") and constraint 3 (device identity and ordering).

### What each algorithm actually consumes

| Algorithm | Model input | Derived state |
|---|---|---|
| **SM-2** | ordered grades `q ∈ 0..5`; timestamps only needed if lateness is honoured (Anki's addition, not SM-2's) | `ef`, `n`, `interval` |
| **FSRS-6** | `{rating: 1..4, delta_t: days}` per review, chained per card. **Nothing else.** Not answer time. | `MemoryState { stability, difficulty }` |
| **Graded Leitner** | ordered grades + the invented mapping | `box` |

FSRS's minimal input is confirmed three ways: `fsrs-rs/src/dataset.rs`, the benchmark's input-feature
legend (interval lengths, grades, same-day reviews — no answer time), and the Anki FAQ's *"FSRS only
uses interval lengths and grades."*

**But `delta_t` is derived, not recorded.** Anki computes it as a difference of *day buckets*, where
the bucket boundary is the user's configured rollover hour in their timezone. So the log must carry
enough to reconstruct the bucketing — an absolute instant plus the rollover/timezone in force —
or the derived `delta_t` will silently change if either is ever adjusted.

### The recommended event shape

Superset of everything SM-2, FSRS-6 and a graded box system need, with the distinctions Anki learned
the hard way that it needed:

| Field | Why |
|---|---|
| `card_id` | which card |
| `reviewed_at` | absolute instant (ms precision), so day bucketing is reconstructible |
| `tz_offset` / rollover config | `delta_t` is bucket-relative; without this the same instants bucket differently |
| `rating` | 1–4 (or 0–5 if SM-2's scale is kept) — the raw grade, per constraint 1 |
| `event_kind` | graded review vs **manual reschedule** vs **reset** vs **cram/preview**. Anki's `review_kind` is load-bearing: the optimiser walks history backwards to find the start of the current memory trace and must drop resets and crams. A log of only graded reviews cannot reproduce state after a user manually sets a due date. |
| `state_before` | new / learning / review / relearning — distinguishes intraday from interday |
| `scheduled_interval_before`, `scheduled_due_before` | recovers lateness; also records what *was* chosen |
| `duration_ms` | not used by either memory model; useful for retention-cost tuning and analytics |
| `device_id`, ordering marker | map constraint 3 — a device must be able to tell it is ahead of or behind another |

Anki's own `RevlogEntry` and `py-fsrs`'s `ReviewLog` are transcribed field-by-field in
[Appendix A §6](./appendix-a-sm2-and-fsrs.md) for comparison.

### Replay: what works, and exactly what breaks it

**The cores are pure.** SM-2's `(EF, n, I)` is a deterministic function of the grade sequence alone.
FSRS-6's `step` function is pure with no RNG and no global state; `forward_reviews` recomputes
`(S, D)` from a review sequence — which is precisely how Anki recomputes memory state after
re-optimisation. So `(card, timestamp, grade)` ⇒ scheduling state, **conditional on** the following:

| Breaker | Replayable? | What it means for us |
|---|---|---|
| **The parameter vector** | Must be versioned | Re-optimising changes every card's computed `(S, D)`. Bit-exact replay requires pinning/versioning the 21 weights alongside the log. |
| **Interval fuzz** | **Yes, if seeded from card identity** | Anki seeds from `card_id + reps`, so every device recomputes the identical date and fuzz survives replay. `py-fsrs` uses unseeded `random()` and does not. **Requirement: seed from card identity and review count; never from an ambient RNG.** |
| **Load balancing** | **No — depends on the whole collection** | Picks a day inside the fuzz range weighted by how many *other* cards are due each day. On by default in Anki. Not recoverable from one card's log. |
| **Easy Days** | **No — depends on calendar weekday** | The adjustment depends on which weekday each candidate due date lands on. |
| **Sibling avoidance** | **No — depends on other cards** | Biases against days already holding a card from the same note. |
| **Day cutoff / rollover / timezone** | Only if recorded | Change the rollover hour or timezone and identical instants bucket into different days ⇒ different `delta_t` ⇒ different state. |
| **Manual / reset / cram events** | Only if logged as first-class events | Otherwise state after a manual due-date change cannot be reproduced. |
| **Config drift** | Must be versioned | Interval modifier, easy/hard/lapse multipliers, max/min intervals, learning steps, desired retention, `historical_retention` are all *current-config* inputs to the interval, not stored per event. |
| **Answer wall-clock beyond the timestamp** | No hidden dependence found | The only clock inputs are the logged instant and the day-timing config. Answer duration feeds neither memory model. |

**The design rule this implies:** any scheduling adjustment that depends on state outside the card's
own history — load balancing, calendar shaping, sibling spreading — must either be **disabled**, or
have its *chosen* outcome **persisted on the event** so the non-replayable decision becomes recorded
data rather than something recomputed. This is a real tension with map constraint 1 and belongs in
the event-log format decision, not just the algorithm decision.

**Swappability by replay** (the second half of constraint 1) holds if the log records raw grades,
absolute timestamps, the day-bucketing config, and the event-kind discriminator. That set is a strict
superset of SM-2's, FSRS-6's, and any box system's inputs — so a later algorithm change is a
re-derivation, not a migration. The one thing that does *not* survive a swap is the *scheduled dates*
already shown to the user; those change under the new algorithm by construction.

---

## 6. Open questions this research did not close

Carried forward honestly; the full per-appendix lists are longer.

1. **Leitner's book was never read.** Every claim about the original system is second-hand from
   uncited Wikipedia prose. Unverified: whether the 1/2/5/8/14 cm partitions are actually in the book,
   whether Leitner states any day-interval, whether demotion is "back one" or "back to box 1", and
   whether he even says five boxes. ISBN 978-3-451-05060-2 (18th ed., Herder 2011) if this matters.
2. **FSRS-7's formulas have no primary write-up.** Only a qualitative benchmark-README description
   exists. Pinning them would mean reading `srs-benchmark` source.
3. **Runtime `rayon` behaviour on wasm is unverified.** Compilation was proven; execution was not.
   A short spike — run `compute_parameters` under `wasm-bindgen-test` without a thread pool — would
   settle whether in-app optimisation works on web.
4. **`compute_parameters` wall-clock on a mid-range Android device is unmeasured.** The only anecdote
   is 3.5 s for 24k revlogs in an optimised browser build, with a *catastrophic* debug/release gap.
5. **Android linking was not proven end-to-end** — `cargo check` passed, but no `.so` was produced,
   and only `aarch64` was tested. (AnkiDroid shipping `fsrs` is strong indirect evidence.)
6. **The linear-vs-exponential box↔interval contradiction is unreconciled** by any paper.
7. **No cited instance of "card shown in a high box but actually due now."** The mechanism is
   established (non-monotone displayed intervals); the manifestation through a box UI is predicted,
   not observed — plausibly because few apps ship a box UI over an interval scheduler at all.
8. **supermemo.guru returns HTTP 403 to automated fetchers**, so Woźniak's own retrospective
   criticism of SM-2 could not be quoted directly (quotes used were retrieved with a browser
   user-agent).
