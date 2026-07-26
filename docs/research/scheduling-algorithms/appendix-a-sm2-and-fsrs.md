# SM-2 and FSRS: primary-source research notes

Research date: **2026-07-26**. All version claims below were verified on the web / against source
checkouts on that date, not from memory.

Source checkouts used (so claims are reproducible):

- `ankitects/anki` @ `e10ce15` (2026-07-25), sparse clone of `rslib`, `proto`, `ftl`, `ts/routes/deck-options` — https://github.com/ankitects/anki
- `ankitects/anki-manual` @ HEAD (2026-07) — https://github.com/ankitects/anki-manual
- `open-spaced-repetition/fsrs-rs` @ `87c9a28` (2026-07-08), version **6.6.2** — https://github.com/open-spaced-repetition/fsrs-rs
- `open-spaced-repetition/py-fsrs` @ `3abe686` (2026-03-10), version **6.3.1** — https://github.com/open-spaced-repetition/py-fsrs
- `open-spaced-repetition/awesome-fsrs` wiki (the *current* FSRS algorithm wiki; the old `fsrs4anki` wiki pages now redirect here) — https://github.com/open-spaced-repetition/awesome-fsrs/wiki/The-Algorithm
- `open-spaced-repetition/srs-benchmark` README — https://github.com/open-spaced-repetition/srs-benchmark

---

## 1. SM-2: the actual published algorithm

Primary source: Piotr Woźniak, *"SuperMemo 2: Algorithm"* / "Application of a computer to improve the
results obtained in working with the SuperMemo method", extracted from his 1990 Master's Thesis
(University of Technology in Poznań), published on the web 1998.

- https://super-memory.com/english/ol/sm2.htm (Woźniak's own site — canonical)
- https://www.supermemo.com/en/archives1990-2015/english/ol/sm2 (same text on supermemo.com, dated 11.08.1990)

Both URLs were fetched and contain byte-for-byte the same algorithm text. Verbatim quotes below.

### Provenance / age

> "I wrote the first SuperMemo program in December 1987 (Turbo Pascal 3.0, IBM PC)."

> "Let us now consider the final form of the SM-2 algorithm that with minor changes was used in the
> SuperMemo programs, versions 1.0-3.0 between December 13, 1987 and March 9, 1989 (the name SM-2 was
> chosen because of the fact that SuperMemo 2.0 was by far the most popular version implementing this
> algorithm)."

So SM-2 is a **1987–1989 algorithm**, ~39 years old as of 2026.
(Source: https://super-memory.com/english/ol/sm2.htm)

### The seven published steps (verbatim)

> 1. Split the knowledge into smallest possible items.
> 2. With all items associate an E-Factor equal to 2.5.
> 3. Repeat items using the following intervals:
>    `I(1):=1`
>    `I(2):=6`
>    `for n>2: I(n):=I(n-1)*EF`
>    where: I(n) - inter-repetition interval after the n-th repetition (in days), EF - E-Factor of a given item
>    If interval is a fraction, round it up to the nearest integer.
> 4. After each repetition assess the quality of repetition response in 0-5 grade scale:
>    5 - perfect response
>    4 - correct response after a hesitation
>    3 - correct response recalled with serious difficulty
>    2 - incorrect response; where the correct one seemed easy to recall
>    1 - incorrect response; the correct one remembered
>    0 - complete blackout.
> 5. After each repetition modify the E-Factor of the recently repeated item according to the formula:
>    `EF':=EF+(0.1-(5-q)*(0.08+(5-q)*0.02))`
>    where: EF' - new value of the E-Factor, EF - old value of the E-Factor, q - quality of the response in the 0-5 grade scale.
>    If EF is less than 1.3 then let EF be 1.3.
> 6. If the quality response was lower than 3 then start repetitions for the item from the beginning
>    **without changing the E-Factor** (i.e. use intervals I(1), I(2) etc. as if the item was memorized anew).
> 7. After each repetition session of a given day repeat again all items that scored below four in the
>    quality assessment. Continue the repetitions until all of these items score at least four.

(Source: https://super-memory.com/english/ol/sm2.htm)

### Additional facts stated on the same page

- The EF update is also given in the equivalent reduced form `EF':=EF-0.8+0.28*q-0.02*q*q`, and:
  "Note, that for q=4 the E-Factor does not change." (So q=5 raises EF by +0.1, q=4 leaves it, q=3
  lowers it by −0.14, q=2 by −0.32, q=1 by −0.54, q=0 by −0.80.)
- Initial EF = 2.5; hard floor 1.3. Rationale for the 1.3 floor, verbatim:
  > "Items having E-Factors lower than 1.3 were repeated annoyingly often and always seemed to have
  > inherent flaws in their formulation (usually they did not conform to the minimum information
  > principle). Thus not letting E-Factors fall below 1.3 substantially improved the throughput of the
  > process and provided an indicator of items that should be reformulated."
- Note that the original text also says the *earlier* SM-2 era allowed EF between 1.1 and 2.5 before
  the 1.3 floor was introduced; the published final algorithm uses 1.3.

### Failure rule (q < 3) — be precise

On q < 3 the repetition count `n` is reset to 1 (next interval `I(1)=1` day, then `I(2)=6`), and
**EF is explicitly NOT changed** by the reset step. EF *is* still modified by step 5 for every
repetition including failed ones (step 5 says "After each repetition"), so a failing grade does lower
EF via the formula; it is only the *reset* that is EF-neutral. Sources disagree on nothing here, but
note that many reimplementations (including Anki) deviate — see §3.

---

## 2. SM-2: state that must be stored per card

To run SM-2 exactly as published you need, per item:

| Field | Type | Why |
|---|---|---|
| `ef` (E-Factor) | float, init 2.5, floor 1.3 | multiplier in `I(n)=I(n-1)*EF` |
| `n` (repetition number) | int, ≥0 | selects `I(1)=1`, `I(2)=6`, or `I(n-1)*EF`; reset to 0 on q<3 |
| `interval` (last `I(n)`) | int days | needed because `I(n)` is recursive on `I(n-1)` |
| `due` / `last_review` date | date | to know when the item is due (the paper works in whole days) |

Notes:
- `interval` is strictly redundant *if* you keep the full grade history and replay from scratch, but
  it is required for incremental operation.
- The published algorithm has **no** lapse counter, no leech handling, no per-item difficulty beyond
  EF, and no sub-day scheduling.
- Step 7 (same-day repetition of anything graded <4) is part of the published algorithm but is a
  *session* rule, not per-card persistent state.

(Source: https://super-memory.com/english/ol/sm2.htm)

---

## 3. SM-2: known weaknesses, and precisely what is Anki's vs SM-2 proper

### 3a. "Ease hell"

The term is an Anki-community term, **not** a SuperMemo one. The most authoritative source that uses
it is the Anki manual itself:

> "(Re)learning steps of 1 day or greater are not recommended when using FSRS. The main reason they
> were popular with the legacy SM-2 algorithm is because repeatedly failing a card after it has
> graduated from the learning phase could reduce its ease a lot, leading to what some people called
> **"ease hell"**. This is not a problem that FSRS suffers from."
> — `src/deck-options.md` line 543, https://github.com/ankitects/anki-manual/blob/main/src/deck-options.md
> (rendered: https://docs.ankiweb.net/deck-options.html#learning-and-relearning-steps)

**Precise mechanism, and it is Anki-specific.** In *published* SM-2 a failure resets `n` but the
reset itself does not touch EF, and EF changes are bounded by the `f(EF,q)` formula. In **Anki**,
review-state answers apply *fixed additive deltas* to ease, floored at 1.3:

```rust
// rslib/src/scheduler/states/review.rs (Anki @ e10ce15)
pub const INITIAL_EASE_FACTOR: f32 = 2.5;
pub const MINIMUM_EASE_FACTOR: f32 = 1.3;
pub const EASE_FACTOR_AGAIN_DELTA: f32 = -0.2;
pub const EASE_FACTOR_HARD_DELTA: f32 = -0.15;
pub const EASE_FACTOR_EASY_DELTA: f32 = 0.15;
```
(https://github.com/ankitects/anki/blob/main/rslib/src/scheduler/states/review.rs)

Because `Again` is −0.20 and `Hard` is −0.15 with no upward pressure other than `Easy` (+0.15), a card
that is failed or "Hard"-ed repeatedly monotonically walks its ease down to the 1.3 floor, at which
point its interval only ever grows by ×1.3 and it recurs indefinitely often. There is no mean
reversion. FSRS's difficulty update deliberately adds mean reversion "to avoid 'ease hell'" — that
phrase appears verbatim in the FSRS v3 and v4 sections of the algorithm wiki:
https://github.com/open-spaced-repetition/awesome-fsrs/wiki/The-Algorithm

Anki also documents a *mitigation* it added: successive failures while a card is in the learning
phase do **not** decrease ease. Anki FAQ, verbatim:

> "successive failures while cards are in learning do not result in further decreases to the card's ease"
> — https://faqs.ankiweb.net/what-spaced-repetition-algorithm.html

Verified in code: `ease_factor` is only mutated in `ReviewState::answer_*` (review.rs); `LearnState`
only ever sets `ctx.initial_ease_factor`, never decrements
(`rslib/src/scheduler/states/learning.rs`).

Note on secondary sources: popular explanations of ease hell (e.g. https://readbroca.com/anki/ease-hell/)
describe the same mechanism, but they are **secondary** — I have cited the Anki manual/source instead.

### 3b. Anki is not raw SM-2 — the enumerated differences

Anki FAQ, "What spaced repetition algorithm does Anki use?", verbatim points:

> "As of Anki 23.10, Anki has two available algorithms. The first one is based on the SuperMemo 2
> algorithm, and the second one is called FSRS."
> …Anki gives users "complete control over initial learning step lengths" rather than SM-2's fixed 1-day
> and 6-day intervals; offers "four response choices instead of six, with only one failure option";
> "answering cards later than scheduled will be factored into the next interval calculation";
> and "successive failures while cards are in learning do not result in further decreases to the card's ease."
> — https://faqs.ankiweb.net/what-spaced-repetition-algorithm.html

Verified in Anki source (`rslib/src/deckconfig/mod.rs`, `rslib/src/scheduler/states/*`):

| Anki addition | Default | Where | Not in SM-2 proper? |
|---|---|---|---|
| **Learning steps** (sub-day, before graduation) | `learn_steps: vec![1.0, 10.0]` minutes | `deckconfig/mod.rs:96` | Yes — SM-2 has only `I(1)=1d`, `I(2)=6d` |
| **Relearning steps** | `relearn_steps: vec![10.0]` minutes | `deckconfig/mod.rs:97` | Yes |
| **Graduating interval** | `graduating_interval_good: 1`, `graduating_interval_easy: 4` days | `deckconfig/mod.rs:52-53` | Yes |
| **Starting ease** | `initial_ease: 2.5` | `deckconfig/mod.rs:46` | Same value as SM-2, but configurable |
| **Easy bonus** | `easy_multiplier: 1.3` | `deckconfig/mod.rs:47` | Yes — SM-2 has no per-grade multiplier |
| **Hard interval** | `hard_multiplier: 1.2` (applied to *previous* interval, not `EF`) | `deckconfig/mod.rs:48` | Yes |
| **New interval** (on lapse) | `lapse_multiplier: 0.0` | `deckconfig/mod.rs:49` | SM-2 resets to `I(1)`; Anki lets you retain a fraction. Manual recommends leaving it at 0 and cites SuperMemo on post-lapse stability: https://supermemo.guru/wiki/Post-lapse_stability |
| **Interval modifier** | `interval_multiplier: 1.0` | `deckconfig/mod.rs:50` | Yes — a global retention knob; manual gives the SuperMemo-derived formula `log(desired retention%)/log(current retention%)` |
| **Maximum interval** | `maximum_review_interval: 36_500` days (100 y) | `deckconfig/mod.rs:51` | Yes |
| **Minimum lapse interval** | `minimum_lapse_interval: 1` day | `deckconfig/mod.rs:52` | Yes |
| **Leech threshold / action** | `leech_threshold: 8`, `leech_action: TagOnly` | `deckconfig/mod.rs:60-61` | Yes — SM-2 has no lapse counter at all |
| **Fuzz** | always on for intervals ≥ 2.5 d | `scheduler/states/fuzz.rs` | Yes |
| **Late-review bonus** | `good = (ivl + days_late/2) * EF`, `easy = (ivl + days_late) * EF * easy_mult` | `states/review.rs:220-254` | Yes — SM-2 ignores lateness |
| **Monotonicity guard** | hard < good < easy enforced; "Anki forces a new interval to be at least 1 day longer than it was previously" | `states/review.rs`, manual line ~698 | Yes |
| **Easy Days / load balancer** | load balancer default **on** | `config/bool.rs:69` (`LoadBalancerEnabled` defaults `true`), `states/load_balancer.rs` | Yes |

Leech behaviour, verbatim from the manual:

> "Each time a review card 'lapses' (is failed while it is in review mode), a counter increases. When
> this counter reaches 8, Anki tags the note as a leech and suspends the card. … If you keep failing
> that card, Anki will continue to alert you about the leech periodically. These warnings occur at half
> the initial leech threshold."
> — https://github.com/ankitects/anki-manual/blob/main/src/leeches.md (rendered https://docs.ankiweb.net/leeches.html)

Confirmed in code (`leech_threshold_met` in `states/review.rs:294`): fires at `lapses >= threshold`
and every `ceil(threshold/2)` thereafter.

Anki's fuzz (exact, from `rslib/src/scheduler/states/fuzz.rs`):

```rust
static FUZZ_RANGES: [FuzzRange; 3] = [
    FuzzRange { start: 2.5,  end: 7.0,      factor: 0.15 },
    FuzzRange { start: 7.0,  end: 20.0,     factor: 0.1  },
    FuzzRange { start: 20.0, end: f32::MAX, factor: 0.05 },
];
fn fuzz_delta(interval: f32) -> f32 {
    if interval < 2.5 { 0.0 } else {
        FUZZ_RANGES.iter().fold(1.0, |delta, range|
            delta + range.factor * (interval.min(range.end) - range.start).max(0.0))
    }
}
```
i.e. no fuzz below 2.5 days; otherwise ±(1 day + 0.15·days in 2.5–7 + 0.1·days in 7–20 + 0.05·days above 20).
The manual's rationale: "Anki also applies a small amount of random 'fuzz' to prevent cards that were
introduced at the same time and given the same ratings from sticking together and always coming up for
review on the same day." — https://github.com/ankitects/anki-manual/blob/main/src/studying.md#fuzz-factor

---

## 4. FSRS: the memory model (DSR)

Primary source: the FSRS algorithm wiki (now under `awesome-fsrs`;
https://github.com/open-spaced-repetition/awesome-fsrs/wiki/The-Algorithm) and the ABC of FSRS
(https://github.com/open-spaced-repetition/awesome-fsrs/wiki/ABC-of-FSRS), cross-checked against
`fsrs-rs` source.

> "FSRS springs from [DHP model](https://www.maimemo.com/paper/), which is based on the 'Three
> Component Model of Memory'. … These three variables include:
> * **Retrievability (R)**: The probability that the person can successfully recall a particular piece
>   of information at a given moment. It depends on the time elapsed since the last review and the
>   memory stability (S).
> * **Stability (S)**: The time, in days, required for R to decrease from 100% to 90%. For example,
>   S = 365 means that an entire year will pass before the probability of recalling a particular card
>   drops to 90%.
> * **Difficulty (D)**: The inherent complexity of a particular information. It represents how difficult
>   it is to increase memory stability after a review. In FSRS, it affects how fast stability (and hence
>   intervals) grows after each review."
> — https://github.com/open-spaced-repetition/awesome-fsrs/wiki/ABC-of-FSRS

> "In FSRS, these three values are collectively called the 'memory state'. The value of R changes daily,
> while D and S change only after a card has been reviewed. Older versions of FSRS take into account only
> the first review of the day, **FSRS-6 uses all reviews**. Each card has its own DSR values."
> — ibid.

`D ∈ [1, 10]`; in `fsrs-rs`: `S_MIN = 0.001`, `S_MAX = 36500.0`, `D_MIN = 1.0`, `D_MAX = 10.0`
(`src/simulation.rs:58-61`).

### Forgetting curve form: POWER function (not exponential)

FSRS-6 uses a **power-law** forgetting curve with a *trainable* decay exponent `w20`. Wiki, verbatim:

> "The forgetting curve's decay is trainable:
> R(t,S) = (1 + factor · t/S)^(−w₂₀), where factor = 0.9^(−1/w₂₀) − 1 to ensure R(S,S) = 90%."
> — https://github.com/open-spaced-repetition/awesome-fsrs/wiki/The-Algorithm

Confirmed in `fsrs-rs/src/model.rs`:
```rust
pub(crate) fn power_forgetting_curve(w: &[f32], t: f32, s: f32) -> f32 {
    let decay = -w[20];
    let factor = (0.9f32.ln() / decay).exp() - 1.0;
    (t / s * factor + 1.0).powf(decay)
}
pub(crate) fn next_interval(w: &[f32], stability: f32, desired_retention: f32) -> f32 {
    let decay = -w[20];
    let factor = (0.9f32.ln() / decay).exp() - 1.0;
    stability / factor * (desired_retention.powf(1.0 / decay) - 1.0)
}
```
History (from the same wiki page): FSRS v3 used the **exponential** `R = 0.9^(t/S)`; v4 used power with
`DECAY = −1, FACTOR = 1/9`; FSRS-4.5 used `DECAY = −0.5, FACTOR = 19/81`; **FSRS-6 made the decay a
trainable parameter `w20`** (default 0.1542). Note the sign convention differs between the wiki
(`^(−w20)`) and the code (`decay = -w[20]`, then `.powf(decay)`) — they are the same thing.
> "The new forgetting curve drops sharply before S and flatly after S." — wiki, FSRS-4.5 section.

### Stability and difficulty updates (FSRS-6, verbatim from `fsrs-rs/src/model.rs`)

```rust
init_stability(w, rating)   = w[rating-1]                     // rating 1..4 -> w0..w3
init_difficulty(w, rating)  = w[4] - exp(w[5] * (rating-1)) + 1.0

linear_damping(dD, D)       = (10.0 - D) * dD / 9.0
next_difficulty(w, D, G)    = D + linear_damping(-w[6] * (G - 3.0), D)
mean_reversion(w, D')       = w[7] * (init_difficulty(w, 4) - D') + D'   // target = D0(Easy)

// success (G >= 2), inter-day
stability_after_success(w, S, D, R, G) =
    S * ( exp(w[8]) * (11.0 - D) * S^(-w[9]) * (exp((1.0 - R) * w[10]) - 1.0)
          * hard_penalty(w[15] if G==2 else 1) * easy_bonus(w[16] if G==4 else 1) + 1.0 )

// failure (G == 1), inter-day  (post-lapse stability, capped)
stability_after_failure(w, S, D, R) =
    min( w[11] * D^(-w[12]) * ((S + 1.0)^w[13] - 1.0) * exp((1.0 - R) * w[14]),
         S / exp(w[17] * w[18]) )

// same-day / short-term (delta_t == 0)
stability_short_term(w, S, G) =
    S * { let sinc = exp(w[17] * (G - 3.0 + w[18])) * S^(-w[19]);
          if G >= 2 { max(sinc, 1.0) } else { sinc } }
```
Driver (`fn step`): compute `R = power_forgetting_curve(w, delta_t, S)`; pick
`stability_after_failure` if `G==1` else `stability_after_success`; **override with
`stability_short_term` if `delta_t == 0`**; `D' = clamp(mean_reversion(next_difficulty(D,G)), 1, 10)`;
on the very first review (`nth==0 && S==0`) use `init_stability`/`init_difficulty` instead;
`G==0` (manual reschedule) leaves state untouched. Final `S` is clamped to `[0.001, 36500]`.

Interpretive notes given on the wiki (still valid for the success formula): SInc decreases with D,
decreases with S, increases as R decreases (spacing effect), and is always ≥ 1 for a successful
review. — https://github.com/open-spaced-repetition/awesome-fsrs/wiki/The-Algorithm

---

## 5. FSRS parameters, optimisation, and the minimum-review threshold

### Parameter count

- **FSRS-6 has 21 trainable parameters.** Wiki: "This version uses 21 parameters." —
  https://github.com/open-spaced-repetition/awesome-fsrs/wiki/The-Algorithm ; also
  `fsrs-rs/src/inference.rs`: `pub static DEFAULT_PARAMETERS: [f32; 21]`.
- Historical counts (srs-benchmark README): v1=7, v2=14, v3=13, v4=17, 4.5=17, FSRS-5=19, FSRS-6=21,
  FSRS-7=35. — https://github.com/open-spaced-repetition/srs-benchmark

### Default parameters (FSRS-6)

```
[0.212, 1.2931, 2.3065, 8.2956, 6.4133, 0.8334, 3.0194, 0.001, 1.8722, 0.1666,
 0.796, 1.4835, 0.0614, 0.2629, 1.6483, 0.6014, 1.8729, 0.5425, 0.0912, 0.0658, 0.1542]
```
Identical in the wiki (`## Default parameters`, FSRS-6 section) and in
`fsrs-rs/src/inference.rs` (`DEFAULT_PARAMETERS`, with `w20 = FSRS6_DEFAULT_DECAY = 0.1542`).
Doc comment in `fsrs-rs`: *"The default parameters. Fits the average person's learning habits."*

Provenance of the defaults, verbatim:
> "If a user doesn't have enough reviews yet, the default parameters are used instead. They have been
> found by running the FSRS optimizer on **several hundred million reviews from ~10k users**. Even with
> the default parameters, FSRS is better than SM-2 algorithm."
> — https://github.com/open-spaced-repetition/awesome-fsrs/wiki/ABC-of-FSRS

### What "optimisation" means

> "The FSRS optimizer uses machine learning to learn your memory patterns and find parameters that best
> fit your review history."
> — Anki manual, `deck-options.md` (https://docs.ankiweb.net/deck-options.html#fsrs-parameters)

> "When you click the Optimize button, FSRS will analyze your review history, and generate parameters that
> are optimal for your memory and your cards."
> — `ftl/core/deck-config.ftl:463`

It is gradient-descent fitting of the 21 weights to minimise log-loss on the recall outcomes implied
by the user's own review log (see `fsrs-rs/src/training.rs`, and
https://github.com/open-spaced-repetition/awesome-fsrs/wiki/The-mechanism-of-optimization).
Manual: "Do not change the parameters manually or copy them from someone else." Parameters and
desired retention are **per-preset**; enabling FSRS at all is **global**.

### MINIMUM number of reviews before optimisation is meaningful — the actual stated threshold

This is the question where the historical answer and the current answer differ, so be careful:

**Current official answer: there is no hard minimum since Anki 24.06.3.** Anki FSRS FAQ, verbatim:

> Q: (do I need N reviews to optimise?) — "That was the case in earlier versions of Anki. In
> **Anki 24.06.3** (and newer versions), the optimizer can be used with any number of reviews."
> — https://faqs.ankiweb.net/frequently-asked-questions-about-fsrs.html

Historical thresholds (for context, from fsrs4anki docs / older manual text, recovered via search —
treat the exact version boundaries as **secondary**): **1000 reviews** in versions before Anki 24.04,
**400 reviews** in Anki 24.04. The 400 figure still survives as a comment in current Anki source:

> `/// Note this does not return an error if there are less than 400 items - the caller should instead
> /// check the fsrs_items count in the return value.`
> — `rslib/src/scheduler/fsrs/params.rs:78`, https://github.com/ankitects/anki/blob/main/rslib/src/scheduler/fsrs/params.rs

Current Anki has **no numeric threshold in the UI**. The only guidance is qualitative, in the
optional health check:

> Common reasons why FSRS may not perform well include "**Low number of reviews (less than a few
> hundred)** … Like any machine learning algorithm, FSRS needs data to learn from."
> — Anki manual, `deck-options.md` (https://docs.ankiweb.net/deck-options.html#fsrs-parameters)

Code behaviour: `Collection::compute_params` returns the *current* params unchanged if
`fsrs_items == 0`; otherwise it trains on whatever it has (`rslib/src/scheduler/fsrs/params.rs`).
The UI message when nothing is found is `deck-config-fsrs-params-no-reviews = No reviews found.…`
(`ftl/core/deck-config.ftl:492`).

Re-optimisation cadence, verbatim from the FAQ:
> "Once per month should be more than enough. A more sophisticated rule is to optimize every time the
> number of reviews doubles: after you did 100 reviews, then after 200, then after 400, etc."
> — https://faqs.ankiweb.net/frequently-asked-questions-about-fsrs.html
The manual says "once every month is sufficient."

### Does FSRS work with ZERO history using defaults? Does it degrade gracefully?

**Yes on both counts, and this is stated explicitly.**

- `fsrs-rs`: `FSRS::new(&[])` is valid and substitutes `DEFAULT_PARAMETERS`
  (`check_and_fill_parameters`, `src/model.rs:336`: `0 => DEFAULT_PARAMETERS.to_vec()`);
  `impl Default for FSRS` = "parameters that fit the average person's learning habits".
- Anki manual setup step: "Click the 'Optimize' button … If you see a message that says 'The FSRS
  parameters currently appear to be optimal', that's fine." — i.e. defaults are a legitimate resting state.
- Wiki: "Even with the default parameters, FSRS is better than SM-2 algorithm."
  — https://github.com/open-spaced-repetition/awesome-fsrs/wiki/ABC-of-FSRS
- Quantitatively, the benchmark reports `FSRS-7 default param.` (0 trainable params) at
  log-loss 0.3629 vs optimised `FSRS-7` at 0.3437 and `FSRS-5` (19 optimised params) at 0.3560 —
  i.e. default-parameter FSRS is worse than optimised FSRS but in the same league, and the degradation
  is graceful. (The benchmark does not publish a `FSRS-6 default param.` row; the 0-parameter row is
  for FSRS-7.) — https://github.com/open-spaced-repetition/srs-benchmark
- Anki also has a `historical_retention` knob (default 0.9) used to fill gaps when review history is
  missing: "When some of your review history is missing, FSRS needs to fill in the gaps. By default, it
  will assume that when you did those old reviews, you remembered 90% of the material."
  — `deck-options.md`, and `historical_retention: 0.9` in `rslib/src/deckconfig/mod.rs`.
- When a card has *no* FSRS memory state but has SM-2 `ivl`/`factor`, FSRS converts them:
  `S = Interval / IntervalModifier` and `D = 11 − (factor−1)/(e^{w8}·S^{−w9}·(e^{w10(1−R)}−1))`.
  — https://github.com/open-spaced-repetition/fsrs4anki/wiki/How-does-the-scheduler-work%3F
  (that wiki page is flagged as outdated for the *scheduler* mechanics but the conversion formulas are
  the ones still described; treat as **weaker evidence** than the source code path
  `rslib/src/scheduler/fsrs/memory_state.rs`.)

---

## 6. What a review log must record for FSRS (MOST IMPORTANT)

### The model's actual input is minimal

`fsrs-rs` trains and infers on `FSRSItem { reviews: Vec<FSRSReview> }` where a review is **just two
fields**:

```rust
// fsrs-rs/src/dataset.rs
pub struct FSRSReview {
    /// 1 = Again, 2 = Hard, 3 = Good, 4 = Easy
    pub rating: u32,
    /// The number of days that passed. `delta_t` for item first(initial) review must be 0
    pub delta_t: u32,
}
```
An `FSRSItem` is the **whole ordered review history of one card**; for training, Anki emits one
`FSRSItem` per review (a growing prefix of the card's history).
— https://github.com/open-spaced-repetition/fsrs-rs/blob/main/src/dataset.rs

Corroborated by the benchmark's "Input features" column: FSRS-6 / FSRS-rs are marked
**`IL, G, SR`** = **i**nterval **l**engths (in days), **g**rades, **s**ame-day reviews. They do
**not** use `AT` (answer time). And the FSRS FAQ says so explicitly:
> "FSRS only uses interval lengths and grades."
> — https://faqs.ankiweb.net/frequently-asked-questions-about-fsrs.html
(Answer time is used only by "Compute minimum recommended retention", not by the memory model.)
— https://github.com/open-spaced-repetition/srs-benchmark

### But `delta_t` is *derived*, so the log must record enough to derive it

Anki computes `delta_t` in whole days from revlog timestamps bucketed by the user's day cutoff:

```rust
// rslib/src/scheduler/fsrs/params.rs
let delta_ts = iter::once(0).chain(entries.iter().tuple_windows().map(|(previous, current)| {
    previous.days_elapsed(next_day_at) - current.days_elapsed(next_day_at)
})).collect_vec();
// where days_elapsed(next_day_at) = (next_day_at.elapsed_secs_since(self.id.as_secs()) / 86_400).max(0)
```
So: **timestamp per review + the day-cutoff/rollover configuration** ⇒ `delta_t`.

### Anki's actual revlog row — the concrete field list to copy

```rust
// rslib/src/revlog/mod.rs
pub struct RevlogEntry {
    pub id: RevlogId,          // epoch MILLISECONDS of the review == the timestamp (also the PK)
    pub cid: CardId,           // card id
    pub usn: Usn,              // sync marker (not algorithmic)
    pub button_chosen: u8,     // "ease": 1..4; 0 == manual rescheduling
    pub interval: i32,         // new interval; positive = days, negative = seconds
    pub last_interval: i32,    // previous interval; positive = days, negative = seconds
    pub ease_factor: u32,      // SM-2 ease ×1000 (2500 = 250%). Under FSRS, difficulty normalised
                               // to 100-1100 so that difficulty 0 is distinguishable from SM-2 learning
    pub taken_millis: u32,     // answer duration in ms
    pub review_kind: RevlogReviewKind, // 0 Learning, 1 Review, 2 Relearning, 3 Filtered, 4 Manual, 5 Rescheduled
}
```
— https://github.com/ankitects/anki/blob/main/rslib/src/revlog/mod.rs

Why each non-obvious field matters for FSRS fitting:
- `review_kind` is load-bearing. Anki walks the history backwards to find the *last* `Learning` entry
  (start of the current memory trace) and to detect `Manual` resets and filtered-deck "cramming":
  `is_reset()` = `Manual` + `ease_factor == 0`; `is_cramming()` = `Filtered` + `ease_factor == 0`;
  `has_rating_and_affects_scheduling()` filters out both. Entries before a reset are dropped.
  (`rslib/src/revlog/mod.rs`, `rslib/src/scheduler/fsrs/params.rs::reviews_for_fsrs`)
- `interval` / `last_interval` with their sign convention distinguish **intraday** (`|ivl| < 86400 s`)
  from **interday** reviews: `let interday = entry.interval >= 1 || entry.interval <= -86400;`.
- `button_chosen == 0` marks non-graded (manual) entries which must be excluded.
- `ignore_revlogs_before` (a per-preset date) can exclude old history from fitting
  (`ignore_revlogs_before_date` in `DeckConfigInner`).

### The reference "minimal" log, per py-fsrs

`py-fsrs` (v6.3.1, MIT) defines the log a client should keep as exactly:

```python
@dataclass
class ReviewLog:
    card_id: int
    rating: Rating              # 1..4
    review_datetime: datetime
    review_duration: int | None # ms, optional; not used by the model
```
and the per-card state as:
```python
class Card:
    card_id: int
    state: State                # Learning / Review / Relearning
    step: int | None            # index into learning/relearning steps
    stability: float | None
    difficulty: float | None
    due: datetime
    last_review: datetime | None
```
— https://github.com/open-spaced-repetition/py-fsrs/blob/main/fsrs/review_log.py ,
  https://github.com/open-spaced-repetition/py-fsrs/blob/main/fsrs/card.py

### The published research dataset schema (a good sanity check on "what to record")

`open-spaced-repetition/anki-revlogs-10k` (727 M reviews, 10k users) exposes per review:
`card_id`, `day_offset` (days since the user's first review), `rating` (1 again / 2 hard / 3 good /
4 easy), `state` (0 new / 1 learning / 2 review / 3 relearning), `duration` (ms, 0–60000),
`elapsed_days` (−1 for new), `elapsed_seconds` (−1 for new).
— https://huggingface.co/datasets/open-spaced-repetition/anki-revlogs-10k

### Recommendation for our own review log (synthesis, not a source claim)

Record per review event: `card_id`, `reviewed_at` (absolute instant, ms + timezone offset),
`rating` (1–4), `state_before` (new/learning/review/relearning), `scheduled_interval_before`
(signed days/seconds or an explicit unit), `scheduled_due_before`, `duration_ms`, and an
`event_kind` discriminator (graded review vs manual reschedule vs reset vs cram/preview).
That is a strict superset of everything FSRS-6 and SM-2 need and it preserves the
reset/cram/manual distinctions that Anki learned the hard way it needs.

---

## 7. Current FSRS version, status in Anki, licence

### Version

- **Shipped / production version is FSRS-6 (21 parameters).** `fsrs-rs` released version is **6.6.2**
  (`Cargo.toml`, 2026-07-08); `py-fsrs` is **6.3.1**; the algorithm wiki's newest documented version is
  FSRS-6. — https://github.com/open-spaced-repetition/fsrs-rs , https://github.com/open-spaced-repetition/py-fsrs ,
  https://github.com/open-spaced-repetition/awesome-fsrs/wiki/The-Algorithm
- **FSRS-7 exists but only in the research/benchmark repo as of 2026-07-26.** srs-benchmark README:
  > "FSRS-7: the newest version. Unlike all previous versions, which have been designed to work with
  > integer interval lengths, FSRS-7 has been designed to work with fractional interval lengths. It is
  > the only version that can give realistic predictions of probability of recall for same-day reviews.
  > The biggest change is that the forgetting curve now has 8 optimizable parameters and uses a rather
  > complex formula."
  > — https://github.com/open-spaced-repetition/srs-benchmark
  It has **35** trainable parameters and beats FSRS-6/FSRS-rs on log-loss (0.3437 vs 0.3443). It is
  **not** in `fsrs-rs`, not in `py-fsrs`, and not in Anki. The algorithm wiki has no FSRS-7 section yet,
  so I have **no primary write-up of the FSRS-7 formulas** — see open questions.

### Status in Anki: OPT-IN, not the default

This contradicts a lot of secondary writing, so it is worth stating carefully. Verified two ways
against Anki main @ `e10ce15` (2026-07-25):

1. Manual, verbatim: "The Free Spaced Repetition Scheduler (FSRS) is **an alternative to Anki's legacy
   SuperMemo 2 (SM-2) algorithm**." and the setup instruction "**Enable FSRS** under the 'FSRS' section,
   at the bottom of the deck options page. FSRS can only be enabled globally; you cannot enable it for
   some presets and disable it for others."
   — https://docs.ankiweb.net/deck-options.html#fsrs
2. Source: `BoolKey::Fsrs` is not in the list of keys that default to `true`, so it falls through to
   `other => self.get_config_default(other)` = **false**
   (`rslib/src/config/bool.rs:62-75`). The only places that set it true are unit tests
   (`storage/sqlite.rs:682`, `scheduler/answering/mod.rs:710`). There is no code path that enables FSRS
   for new collections.

Availability, verbatim: "Ensure all of your Anki clients support FSRS. **Anki 23.10, AnkiMobile 23.10**,
and AnkiWeb support it. **AnkiDroid supports it in 2.17+**." — https://docs.ankiweb.net/deck-options.html#fsrs
Also: "As of Anki 23.10, Anki has two available algorithms." — https://faqs.ankiweb.net/what-spaced-repetition-algorithm.html

FSRS version inside Anki: Anki main pins `fsrs = "6.6.1"` (`Cargo.toml` / `Cargo.lock`), and
`DeckConfigInner` carries `fsrs_params_4`, `fsrs_params_5`, `fsrs_params_6` with `fsrs_params()`
preferring the 6 field — so **Anki ships FSRS-6**. Latest Anki tags on the remote:
stable **26.05**, betas **26.08b1 / 26.08b2** (`git ls-remote --tags ankitects/anki`).

### Licences

| Component | Licence | Source |
|---|---|---|
| `fsrs-rs` (the implementation Anki links) | **BSD-3-Clause** | `Cargo.toml` `license = "BSD-3-Clause"` + `LICENSE` ("BSD 3-Clause License, Copyright (c) 2023, Open Spaced Repetition") — https://github.com/open-spaced-repetition/fsrs-rs/blob/main/LICENSE |
| `py-fsrs` | **MIT** | https://github.com/open-spaced-repetition/py-fsrs/blob/main/LICENSE |
| `fsrs4anki` (the add-on/custom-scheduler project) | **MIT** | https://github.com/open-spaced-repetition/fsrs4anki/blob/main/LICENSE |
| Anki itself | **AGPL-3.0-or-later** | file headers: "License: GNU AGPL, version 3 or later" |

For a Rust project, `fsrs-rs` under BSD-3-Clause is the permissively-licensed reference
implementation; Anki's AGPL applies to Anki's own scheduler code, not to `fsrs-rs`.

---

## 8. FSRS grade scale and lapse / relearning handling

**Four buttons.** `fsrs-rs/src/dataset.rs` doc comment:
> "Rating scale: 1 = Again (forgot anything you want to remember); 2 = Hard (remembered with
> difficulty); 3 = Good (remembered correctly); 4 = Easy (remembered effortlessly)"

Wiki symbol table: `G`: 1 `again`, 2 `hard`, 3 `good`, 4 `easy`.
— https://github.com/open-spaced-repetition/awesome-fsrs/wiki/The-Algorithm

Anki's user-facing definitions (`studying.md`):
> **Again**: "your answer is incorrect or when you couldn't recall the answer … You'll typically use this
> button about 5-20% of the time." **Hard**: "your answer is correct, but you had doubts about it or it
> took a long time to recall." **Good**: "correct, but it took some mental effort … should be the most
> commonly used button. You'll typically use this button about 80-95% of the time." **Easy**: "correct and
> it took no mental effort."
> — https://github.com/ankitects/anki-manual/blob/main/src/studying.md#answer-buttons

**Hard is a PASSING grade.** This matters:
> "FSRS can adapt to almost any habit, except for one: pressing 'Hard' instead of 'Again' when you forget
> the information. When you press 'Hard', FSRS assumes you have recalled the information correctly
> (though with hesitation and a lot of mental effort). If you press 'Hard' when you have failed to recall
> the information, all intervals will be unreasonably high."
> — https://docs.ankiweb.net/deck-options.html#fsrs
Confirmed in code: `stability_after_failure` is used only when `rating == 1.0`; Hard takes the success
branch with a `w[15]` "hard penalty" multiplier.

**"Again" handling / lapses / relearning:**
- The memory-model effect is `stability_after_failure` (post-lapse stability), **capped** at
  `S / exp(w17·w18)` so a lapse cannot *increase* stability, and it depends on `D`, `S` and `R` —
  not on a lapse counter. FSRS-6 has **no** `lapses` term in the model (FSRS v1 did; v2 onward dropped it).
  — https://github.com/open-spaced-repetition/awesome-fsrs/wiki/The-Algorithm , `fsrs-rs/src/model.rs`
- Anki still tracks `lapses` on the card and still applies **leech** tagging/suspension under FSRS
  (`Card.lapses`, `leech_threshold: 8` — these are Anki bookkeeping, not FSRS).
- Relearning steps under FSRS: Anki puts a failed card through the configured relearning steps, and
  "In FSRS, fuzz is applied when the card leaves the relearning stage"
  (`rslib/src/scheduler/states/review.rs:81-83`). Anki recommends relearning steps < 1 day, and in the
  latest version FSRS can own short-term scheduling entirely if the steps field is left empty
  ("This is an experimental feature.") — https://docs.ankiweb.net/deck-options.html#learning-and-relearning-steps
- FSRS-6 also models **same-day** re-reviews (`delta_t == 0` ⇒ `stability_short_term`), which is what
  makes intraday relearning meaningful to the model. FSRS-5 introduced same-day data; FSRS-6 improved
  the formula. — srs-benchmark README, ABC of FSRS.

---

## 9. CROSS-CUTTING: is scheduling state re-derivable by replaying (card, timestamp, grade)?

Short answer: **for the pure algorithms, yes. For Anki's implementations of them, not quite — and the
things that break it are specific and enumerable.**

### 9a. SM-2 proper — fully replayable

`(EF, n, I)` are a deterministic function of the ordered grade sequence alone. Intervals are whole
days and lateness is ignored, so you don't even need timestamps to recover `EF`/`n`/`I` — only to know
whether a review happened. Fully replayable. (https://super-memory.com/english/ol/sm2.htm)

### 9b. FSRS-6 core model — fully replayable

`FSRSReview { rating, delta_t }` is the entire input; `fn step` is pure and deterministic (no RNG, no
global state). `FSRS::forward_reviews(reviews, starting_state)` recomputes `(S, D)` from a review
sequence — this is exactly how Anki recomputes memory state after re-optimisation
(`rslib/src/scheduler/fsrs/memory_state.rs`). So `(card, timestamp, grade)` + parameters + desired
retention ⇒ `(S, D)` and the ideal interval, deterministically.
Caveat: **the parameters themselves are part of the state.** Re-optimising changes every card's
computed `(S, D)`. If you want bit-exact reproduction you must version/pin the parameter vector
alongside the log.

### 9c. Things that break pure replay in practice

| Breaker | Replayable? | Detail / source |
|---|---|---|
| **Anki's fuzz** | **YES — deterministic** | Anki seeds the RNG from card identity, not wall-clock: `get_fuzz_seed_for_id_and_reps(card_id, reps) = Some(card_id + reps)`, then `StdRng::seed_from_u64(seed).random_range(0.0..1.0)` (`rslib/src/scheduler/answering/mod.rs:669-691`). Given `card_id` and the rep count you reproduce the fuzz exactly. (It is also disabled entirely under test.) |
| **py-fsrs fuzz** | **NO** | `_get_fuzzed_interval` calls bare `random()` with no seed (`py-fsrs/fsrs/scheduler.py:846`). Unseeded ⇒ unreproducible. Can be disabled via `enable_fuzzing=False`. |
| **Load balancer** | **NO — depends on the whole collection** | Enabled **by default** (`BoolKey::LoadBalancerEnabled` is in the defaults-to-`true` list, `rslib/src/config/bool.rs:69`). `find_interval` picks a day inside the fuzz range weighted by `(1/cards_due)^2 * (1/target_interval)`, using `days_by_preset` — the count of *other* cards due on each candidate day (`rslib/src/scheduler/states/load_balancer.rs:192-265`). The chosen interval therefore depends on the due-date distribution of every other card in the preset at answer time. Not recoverable from one card's log. |
| **Easy Days** | **NO — depends on calendar weekday** | `calculate_easy_days_modifiers(easy_days_load, &weekdays, &review_counts)` with `interval_to_weekday(...)` — the result depends on which weekday each candidate due date lands on, i.e. on absolute calendar position. Manual: "After the interval is calculated, it will be adjusted by a small amount to change the due date. This feature works with both FSRS and the legacy SM-2 algorithm." (https://docs.ankiweb.net/deck-options.html#easy-days) |
| **Sibling avoidance** | **NO — depends on sibling cards** | `calculate_sibling_modifiers(..., note_id)` biases against days that already hold a card from the same note (`load_balancer.rs:247`). Cross-card coupling. |
| **Day cutoff / rollover hour / timezone** | **Partially — must be recorded** | Everything day-shaped goes through `timing.days_elapsed` / `next_day_at`. `delta_t` for FSRS is `previous.days_elapsed(next_day_at) - current.days_elapsed(next_day_at)` (`fsrs/params.rs`), and SM-2 `elapsed_days` is `interval - (due - timing.days_elapsed)` (`answering/current.rs:104-106`). Change the rollover hour or the timezone and the *same* absolute timestamps bucket into different days ⇒ different `delta_t` ⇒ different state. So the rollover hour and the timezone in force must be part of the log or a pinned config. |
| **Lateness (SM-2, Anki)** | **YES if timestamps are logged** | `days_late = elapsed_days - scheduled_days`, feeding `good = (ivl + days_late/2)*EF`. Deterministic from timestamps + prior scheduled interval — but it does mean SM-2-in-Anki needs *timestamps*, not just grade order. |
| **Non-graded / manual events** | **Must be in the log** | `Manual` (set due date), `Rescheduled`, `Filtered`+cram entries, and `is_reset()` truncate or alter the effective history (`reviews_for_fsrs` walks backwards and drops everything before a reset). If your log only records graded reviews you cannot reproduce state after a user has manually set a due date or reset a card. |
| **Configuration drift** | **Must be versioned** | Interval modifier, easy/hard/lapse multipliers, max/min intervals, learning & relearning steps, leech threshold, desired retention, `historical_retention`, `ignore_revlogs_before_date`, and the FSRS parameter vector are all *current-config* inputs to the interval, not stored per event. A replay under today's config will not reproduce yesterday's scheduling if any of them changed. |
| **Wall-clock at answer time beyond the timestamp** | No hidden dependence found | The scheduler's only clock inputs are `self.now` and `timing` (both derivable from the logged instant + rollover config). Answer duration (`taken_millis`) is recorded but does not feed either algorithm. |
| **Queue composition (which cards you see today)** | Not part of card state | Daily limits, gathering/sorting order, burying etc. determine *whether* a card was answered on a given day. Since your log records what actually happened, this doesn't affect replay of state — but it does mean you cannot replay a *simulation* of what would have happened. |

### 9d. Practical conclusion for a replayable design

You get pure, exact replay from `(card_id, timestamp, grade)` if you (a) pin/version the parameter
vector and the per-preset config, (b) record the rollover hour + timezone (or store `delta_t`
alongside), (c) log manual/reset/cram events as first-class entries, and (d) either disable load
balancing / easy-days / sibling avoidance, or persist the *chosen* interval on each event so the
non-replayable choice is recorded as data rather than recomputed. Fuzz specifically is *not* a problem
if you copy Anki's approach of seeding from `(card_id, reps)`.

---

## Unverified / open questions

1. **FSRS-7 formulas.** I could not find a primary write-up. The `awesome-fsrs` wiki "The Algorithm"
   page documents only up to FSRS-6; the srs-benchmark README describes FSRS-7 qualitatively (35
   params, dual power-law forgetting curve, fractional intervals, 8 curve parameters) and DeepWiki
   summaries mention "a weighted combination of two power-law curves" — **DeepWiki is a generated
   secondary source and I did not treat it as authoritative.** To pin the formulas you'd need to read
   `srs-benchmark/src/*` directly. Nothing suggests FSRS-7 will land in Anki soon.
2. **"FSRS-7 is likely the final version."** This appeared in a search-result summary attributed to the
   OSR team; I could not locate the primary statement. Treat as unverified.
3. **Historical minimum-review thresholds (1000 → 400 → none).** The *current* "no minimum since Anki
   24.06.3" is primary (Anki FSRS FAQ). The 1000-and-400 history came from search summaries of
   `fsrs4anki` docs/older manual revisions; the `400` figure is corroborated by a live comment in Anki
   source, but the exact version boundaries are **secondary**.
4. **Whether any Anki fork/client defaults FSRS on.** I verified only Anki desktop main
   (`BoolKey::Fsrs` ⇒ false). AnkiDroid/AnkiMobile defaults were not checked; AnkiDroid reads the same
   collection config so it should inherit, but I did not verify their new-profile behaviour.
5. **supermemo.guru pages** (e.g. `Algorithm_SM-2`, `Post-lapse_stability`) return **HTTP 403** to
   automated fetches, so I could not quote Woźniak's own retrospective criticism of SM-2. The Anki
   manual cites `supermemo.guru/wiki/Post-lapse_stability` for the claim that preserving part of the
   interval after a lapse is counter-productive; I am relaying Anki's citation, not the page itself.
6. **Anki 26.05/26.08 release notes.** GitHub API rate-limited; I read tags via `git ls-remote` and the
   source tree at `main` (2026-07-25) instead. I did not read per-release changelogs, so if FSRS default-on
   is announced in an unreleased beta I would have missed it — though the `main` branch code says otherwise.
7. **`FSRS-6 default param.` benchmark row** does not exist; the 0-parameter benchmark row is FSRS-7.
   So the exact quantitative penalty for running FSRS-**6** with defaults instead of optimised params
   is not directly published in that table.
8. **Anki's SM-2 "early review" path** (`passing_early_review_intervals`) is flagged in-source as
   needing rework ("FIXME: this needs reworking in the future; it overly penalizes reviews done shortly
   before the due date"). Behaviour is deterministic and replayable but is an acknowledged wart.
