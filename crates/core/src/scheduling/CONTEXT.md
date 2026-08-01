# Scheduling

FSRS-6 arithmetic: given a card's ordered grades and day numbers, what is its memory state, when is
it next due, and which box does it show. Pure — no clock, no randomness, no storage.

Depends on `content` for `CardRef` (the fuzz seed). Deliberately does **not** depend on `log`: this
context takes grades and day numbers as values, so its arithmetic is testable against a hand-written
list with no rows, no writers and no merge.

**Bound by** [ADR-0001](../../../../docs/adr/0001-scheduling-algorithm-and-grade-scale.md), whose
glossary this file supersedes; also by
[ADR-0004 §4 and §5](../../../../docs/adr/0004-the-review-event-log.md) and by
[ADR-0027](../../../../docs/adr/0027-the-scheduler-dependency.md), which admits the `fsrs` crate into
`leitner-core` for this context's sake and states what that does **not** license.

> **This context owns the crate's only dependency, so two of its rules are about the crate rather
> than about the arithmetic.** `fsrs` brings `rand`, `serde`, `rayon` and `ndarray` transitively;
> none of them is available to our code (ADR-0027 §3). **The fuzz is ours, not the crate's** — the
> crate ships its own over `rand`, and ADR-0001 §7 seeds ours from `CardRef`, so the implementation
> takes the *un-fuzzed* interval and fuzzes it itself. **`rayon` is compiled in and never reached**:
> `compute_parameters` is single-threaded, inferred in
> [#2](https://github.com/amin-bf/leitner/issues/2) and measured in
> [#20](https://github.com/amin-bf/leitner/issues/20). A future version that does reach it is a
> change to notice, not a default to absorb.

## Language

### Grading

**Grade**:
The user's rating of a single recall attempt, one of `Forgot`, `Barely`, `Good`, `Easy`, encoded
1–4. `Forgot` is the only failure.
_Avoid_: Rating, score, answer, ease.

**Lapse**:
A review graded `Forgot`. Collapses stability per the model; we define no rule of our own for it.

### Memory

**Memory state**:
A card's `(stability, difficulty)` pair, derived by replaying its review history. Never authored
directly, never the source of truth.

**Stability**:
Days for recall probability to fall to 90%. Provably non-decreasing on a pass and non-increasing on
a lapse — which is what makes it safe to build a box on.

**Difficulty**:
How hard it is to increase a card's stability, clamped to `[1, 10]`.

**Retrievability**:
Current recall probability, a function of stability and elapsed time.

### The user-facing bucket

**Box**:
A UI-level bucket, 1 to 5, computed from **stability alone** — thresholds 1 / 7 / 30 / 180 days.
Expresses durability, never urgency.
_Avoid_: Level, stage, bucket, interval band.

**Durability**:
The property a box reports: how long you would remember this. Distinct from **due-ness**, which is
separate and is never expressed as a box.

### Configuration

**Scheduler parameters**:
The 21-weight vector, algorithm identity and fitted-over count. Collection state carried in the log,
not a device setting — otherwise two devices replay the same log and compute different memory
states.
_Avoid_: Weights (too narrow — the algorithm identity travels with them).

**Optimisation run**:
Fitting a new parameter vector to this collection's own review history. Always started by the user
(ADR-0014 §1), never on a threshold or a schedule.
_Avoid_: Training, syncing parameters, recalculating.

**Fitted-over count**:
How many reviews an optimisation run trained on, recorded on the `config-set` row and **frozen at
write time**. Never recomputed — see the rule below.

**Desired retention**:
The target recall probability at the scheduled due date. Fixed at 0.9.

## Rules that are easy to break silently

- **The box is derived from stability, never from the scheduled interval.** Fuzz and minimum-gap
  make intervals non-monotone, so a box built on them could *fall after a pass*.
- **Three UI rules bind anything that displays a box** (constraint 4): boxes are never sorted,
  counted, or presented as a review queue; "due today" is a separate number never expressed as a
  box; and nothing may state or imply that lower boxes come up more often.
- **Lateness is never penalised.** `delta_t` is actual elapsed days, and a low retrievability
  *increases* the stability gain. Do not add a catch-up rule.
- **Load balancing, calendar shaping and sibling avoidance stay disabled.** They are what buys
  replay purity; enabling one makes a card's schedule depend on the rest of the collection, and
  replay stops being a function of the card's own history.
- **Fuzz is seeded from `CardRef`, never from an RNG.** Two devices replaying the same log must
  reach the same answer.
- **Never derive the fitted-over count by counting rows around the `config-set` row.** A device that
  optimised while behind on sync trained on fewer reviews than the merged log shows there, so the
  derived number reports a fit that never happened — and it does so precisely when the vector is
  stale, which is the case the number exists to expose (ADR-0014 §6).
- **An optimisation run that produces the current vector writes nothing.** A value-less `config-set`
  row still enters the settling contest, so it can displace a better-fitted vector while changing
  nothing (ADR-0014 §5).
