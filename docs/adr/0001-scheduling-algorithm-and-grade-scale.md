# ADR-0001: Scheduling algorithm and grade scale

- **Status**: Accepted
- **Date**: 2026-07-27
- **Resolves**: [Decide: scheduling algorithm and grade scale](https://github.com/amin-bf/cairn/issues/5)
- **Map**: [Map: local-first Leitner app spec](https://github.com/amin-bf/cairn/issues/1)
- **Evidence**: [`docs/research/scheduling-algorithms/`](../research/scheduling-algorithms/README.md)

## Context

The app is Leitner-branded and shows users **boxes**, but the map's standing constraint 4
declares the box metaphor to be UI-level and leaves the scheduler free. Standing constraint 1
requires the review event log to record raw grades and timestamps rather than derived state, so
that scheduling state is a *replay* of the log and the algorithm stays swappable.

This ADR chooses the scheduler, fixes the grade scale, defines how a box is computed, and states
what the choice requires of the event log.

Prior research established the facts this decision rests on, including two that removed the usual
reasons to start simple: FSRS has no minimum-review threshold and its default parameters work from
a cold start, and the `fsrs` crate no longer pulls in an ML framework and builds for
`wasm32-unknown-unknown` and `aarch64-linux-android`. It also established, more strongly than mere
absence, that **no published mapping of a four-point grade onto box movement exists** — the two
academic treatments of Leitner deliberately collapse grades to binary, and the two graded
implementations found in real code contradict each other on what the second grade even does. Any
grade-to-box rule we write is an invention and owns its own correctness.

## Decision

### 1. FSRS-6 is the scheduler, via the `fsrs` crate, pinned exactly

The memory model is FSRS-6: per card a **memory state** of stability `S` and difficulty `D`, with
retrievability `R` derived from elapsed time. The crate is BSD-3-Clause and carries both the
scheduler and the optimiser.

Pin the version exactly. The 6.x line shipped nine releases in eleven days with breaking API
changes *inside* the major version.

Rejected: **SM-2** — fully specified and trivial to write, but a 1987 algorithm with no published
validation against modern benchmarks, modelling difficulty with a single scalar that doubles as the
interval multiplier. Rejected: **graded Leitner as the engine** — it would require inventing the
core of the product's correctness with no evidence behind it, which constraint 4 already frees us
from needing to do.

### 2. Four grades, with the pass/fail boundary made explicit

The model input is `FSRSReview { rating: 1..4, delta_t }` and the default parameters were fitted
against four-point semantics, so the scale is four points or the model is being fed something it
was not fitted for.

| Value | Label | Meaning presented to the user |
|---|---|---|
| 1 | **Forgot** | I could not recall it. If you had to see the answer to know it, this is the one. |
| 2 | **Barely** | I recalled it, but slowly or unsure. |
| 3 | **Good** | I recalled it correctly. |
| 4 | **Easy** | Instant, no effort. |

Grades 2, 3 and 4 all take the **success** branch of the model; only grade 1 is a failure. The UI
renders a **visual break between 1 and 2**, not a smooth four-point spectrum.

The wording is load-bearing, not cosmetic. Because grade 2 is a *passing* grade, a user who
reaches for it after actually failing inflates stability and every interval that follows — and
this is the one habit the model cannot adapt to, because it has been told the recall succeeded.
A label naming *felt difficulty* invites exactly that error, since "I got it, but it was hard" and
"I did not really get it" both feel like a difficulty complaint. Labels naming the **recall
outcome** do not: "Barely" cannot be reached for by someone who did not recall the card at all.

Grades are not collapsed to pass/fail. Grades 2 and 4 carry distinct multipliers in the model, and
constraint 1's logic — record raw, derive later — argues for capturing the finer signal even if an
early UI chooses to hide it. A grade never recorded cannot be recovered.

### 3. A box is discretised stability, and it means durability — not urgency

```
box(card) = 1  if the card has no memory state (never reviewed), else
            1  if S <   1 day
            2  if S <   7 days
            3  if S <  30 days
            4  if S < 180 days
            5  otherwise
```

**The box is derived from stability, never from the scheduled interval or the due date.**
Scheduled intervals are not monotone under success: interval fuzz and minimum-gap constraints mean
a displayed interval can come out *lower* after a passing grade, and a box derived from it would
inherit that — a user could pass a card and watch it drop a box. That is not an approximation, it
is a visible malfunction.

Stability has no such defect, and the guarantee is structural rather than incidental:

- **Success, inter-day**: `S' = S × (bracket + 1.0)` where every factor of the bracket is
  non-negative given `D ∈ [1, 10]` (so `11 − D ≥ 1`) and `R ∈ (0, 1]` (so
  `exp((1 − R)·w10) − 1 ≥ 0`). The multiplier is **≥ 1 by construction** for grades 2, 3 and 4.
- **Success, same-day**: explicitly clamped — `if G >= 2 { max(sinc, 1.0) }`.
- **Failure**: post-lapse stability is **capped** at `S / exp(w17·w18)`, so a lapse can never
  raise stability.

Therefore **stability is non-decreasing on every passing grade and non-increasing on failure**,
and so is the box. We enforce no monotonicity rule of our own; the model provides it.

Stability is defined as *the number of days for recall probability to fall to 90%*, which makes
the ladder directly and honestly explainable to a user: box 3 means "you would remember this for
about a week".

Note a convenience this produces: at the default desired retention of 0.9 the scheduled interval is
**numerically equal to stability** — the retention formula reduces to `interval = S` at exactly
that target. So the thresholds above can be read either way at scheduling time. This does *not*
mean the box could equivalently be derived from the interval: the equality holds only at the
instant of scheduling and only at 0.9, and the interval we then *display* has fuzz and the
minimum-gap constraint applied on top, which is precisely what destroys its monotonicity. The box
is defined on stability so that it inherits stability's guarantee rather than the interval's
defects.

#### The failure mode this design confronts

The known way a box display becomes a lie is a meter that **claims to reflect memory while being
driven by queue position**. The clearest documented case is a language-learning vendor whose
Leitner-derived strength meters were replaced after student complaints that the meters did not
reflect what they had learned; in the paper reporting the replacement, the Leitner-style predictor
had roughly twice the mean absolute error of the model that replaced it, and users optimised for
the meter rather than for learning.

This design inverts that relationship. The box is a function of the memory model directly, and it
makes **no claim about the queue at all**.

Two cards can therefore sit in the same box with very different due dates, and **this is not a
lie**. Due-ness is a function of stability, elapsed time and desired retention; the box is a
function of stability alone. A card with `S = 10 d` reviewed yesterday and a card with `S = 25 d`
reviewed 24 days ago are both in box 3 and both genuinely have week-to-month durability; one is
due now and one is not. The box never spoke about that.

This holds **only** if the UI never implies otherwise. The following are binding constraints on
[Prototype: the review session experience](https://github.com/amin-bf/cairn/issues/11):

- Boxes are never sorted, counted, or presented as a review queue.
- "Due today" is a separate number and is never expressed as a box.
- No copy states or implies that lower boxes come up more often. Under an interval scheduler that
  is statistically true but not guaranteed per card, and asserting it recreates exactly the failure
  mode above.

**Accepted cost**: we give up the classic Leitner promise that low boxes come round sooner. That
promise cannot be kept over an interval-based scheduler. The only shipped counter-example found in
research keeps it by making the stage primary and deriving the interval from it — that is, by not
having a memory model at all.

### 4. Lateness is native and never penalised

`delta_t` is the **actual** number of days elapsed since the previous review, not the interval that
was scheduled. Lateness is therefore not a special case; it is the only time signal the model
consumes.

It also flows in the correct direction. Low `R` (the consequence of being late) *increases* the
stability gain via `exp((1 − R)·w10) − 1`: still recalling a card 30 days late is evidence it was
more durable than believed, and the model credits it. This is the spacing effect expressed in the
formula rather than in a heuristic.

Consequently we implement **none** of the following:

- No lateness penalty. A late success is good news.
- No overdue decay and no expiry. A card 400 days late is a card with very low `R`.
- No clamping of `delta_t`. `S` is clamped internally to `[0.001, 36500]`, which is sufficient.
- No "overdue" state in the model. Due or not due; how long it has been due is presentation.

A very overdue card that is passed will cross several thresholds at once and **jump several
boxes**. This is correct under the durability reading, and is accepted.

### 5. We write no lapse rule; the model already has one

Post-lapse stability is a function of `D`, `S` and `R` with no lapse-count term anywhere in the
model. Computed with the published FSRS-6 default parameters, for a card failed at its due time:

| Box before | `S` before | `S` after lapse | Box after |
|---|---|---|---|
| 5 | 200 d | 4.8 d | **2** |
| 5 | 1000 d | 8.2 d | **3** |
| 4 | 60 d | 3.1 d | **2** |
| 3 | 15 d | 1.7 d | **2** |
| 2 | 3 d | 0.7 d | **1** |

Difficulty moves these by roughly 5% across its whole range. Retrievability moves them
substantially — a lapse on a *very* overdue card is treated about twice as gently, which is right:
failing something unseen for months is weaker evidence of fragility than failing something seen
last week.

The consequence is that **the traditional "fail returns the card to the start" rule is
approximately emergent** from the model rather than imposed by us. The metaphor and the model agree
here, so we honour the metaphor without inventing anything.

Therefore:

- **No reset-to-box-1 rule and no drop-one-step rule.** `S` collapses per the model; the box
  follows. We never write a demotion rule and never have to defend one.
- **No lapse counter influencing scheduling.** Adding one would be folklore overriding the model.
- **A failed card returns within the same session, and that re-show is a real, logged, graded
  event** with `delta_t = 0`. FSRS-6 models same-day reviews on a dedicated short-term path.
  Relearning must **not** be a UI-only loop, or the log loses events the model was fed.
- **A floor of 1 day on the next inter-day interval** after the card leaves the session. Post-lapse
  `S ≈ 3–5 d` makes this rarely bind; it exists to prevent a pathological tiny `S` scheduling an
  inter-day repeat hours later.

**Accepted cost**: the ladder is asymmetric — a slow climb and a sudden collapse. Reaching box 5
takes months of successful review; a single lapse returns the card to box 2. Under the durability
reading this is true, because the user has just demonstrated they had forgotten it. If the top end
ever needs to feel less brittle, the lever is the **thresholds in §3**, not a softened lapse rule —
softening the lapse would make the display lie.

**Not decided here**: leeches — what happens to a card failed repeatedly. That is card and deck
management, not scheduling, and the model has no opinion on it.

> **Since decided by [ADR-0010](0010-leeches.md), and this section is untouched by it.** Leeches are
> detected and surfaced, never acted on automatically, and no leech signal reaches memory state —
> *"no lapse counter influencing scheduling"* stands exactly as written above. ADR-0010 §2 leans on
> this section twice: on the 3–5 day post-lapse collapse measured in the table, and on same-session
> re-shows being real logged rows, which is why it counts failure *days* rather than failure rows.

### 6. Scheduler configuration is collection state carried in the log, never a device setting

The parameter vector is a replay input: re-optimising changes every card's computed `(S, D)`. With
several devices this is a correctness problem, not a preference one. If one device optimises to a
personal 21-weight vector while another runs the published defaults, **both replay the same log and
compute different memory states** — different stability, different boxes, different due dates. The
divergence is invisible to any log-merge machinery, because no event is missing; the two replays
disagreed about the function.

Therefore:

- **The 21-weight parameter vector is collection state**, initialised to the published FSRS-6
  defaults. A device that optimises writes the resulting vector to the log as a config-change
  event; every device replays with it. Parameters are data that syncs, not a local preference.
- **Replay applies the current parameter vector over the whole history**, not a timeline of which
  weights were live when. This matches established practice after re-optimisation, is far simpler,
  and reduces cross-device agreement to "which vector is current" — a small last-writer-wins
  register on the mutable surface, the shape already established as necessary for card edits. No
  new machinery.
- **The algorithm identity is recorded alongside the weights** — `fsrs-6` plus the exact pinned
  crate version. Twenty-one floats are meaningless without knowing which formulas consume them.
- **Desired retention is fixed at 0.9 and not user-exposed.** It is the same class of thing: a
  current-config input to every interval, breaking replay identically if devices disagree. It is
  also the knob most easily misused: raising it from 0.9 to 0.97 shortens the interval for a card
  of stability 10 days from 10.0 days to 3.6 days — very nearly a tripling of review workload — for
  a small accuracy gain. Global, not per-deck. Exposing it later makes it another config-change
  event.

**Accepted cost**: optimisation becomes a syncing action rather than a local one. A user who
optimises on a laptop while offline will not see their phone's schedule change until the log
merges. The alternative is a phone that silently disagrees with the laptop about what the user
knows, which is worse and far harder to notice.

**Consequence for [Prove FSRS parameter optimisation runs in-client](https://github.com/amin-bf/cairn/issues/20)**:
that ticket's worst case was personalised parameters becoming desktop-only, with no server to
compute them on. Under this decision that is no longer a divergence risk — **any one device that
can run the optimiser publishes the result and the others consume it**. A client that cannot run
`compute_parameters` needs only to read 84 bytes. #20 still determines whether optimisation is
possible anywhere, but it can no longer fracture the collection.

### 7. Replay purity is bought by disabling collection-dependent adjustments

Research enumerated exactly what breaks pure replay. For v1:

| Adjustment | Decision | Why |
|---|---|---|
| Load balancing | **Off** | Picks a due date weighted by how many *other* cards are due nearby — a function of the whole collection, unrecoverable from one card's history. |
| Calendar shaping ("easy days") | **Off** | Depends on which weekday a candidate date falls on. |
| Sibling avoidance | **Off** | Depends on other cards derived from the same note. |
| Interval fuzz | **On, seeded from `(card_id, review_count)`** | Without fuzz, cards introduced together stay clumped indefinitely. Seeding from card identity makes every device compute the same date. |

Fuzz is doubly safe: it shifts only a **future** due date, while the quantity the model consumes is
the **elapsed** time actually recorded. Fuzz can therefore never perturb replayed memory state — it
only affects the date displayed.

**The general rule, which outlives these four cases**: any scheduling adjustment depending on state
outside a single card's own history is either **disabled**, or has its chosen outcome **persisted
on the event** so the non-replayable decision becomes recorded data. For v1 we take the first
branch in every case.

## Requirements this places on the event log

[Decide: the review event log format](https://github.com/amin-bf/cairn/issues/9) owns the field
list. This ADR constrains it:

1. **Grade 1–4, raw**, exactly as the user gave it.
2. **An absolute instant** for each review, at millisecond precision.
3. **The day-bucketing configuration in force** — see the constraint amendment below.
4. **An event kind discriminator** that distinguishes graded reviews from manual reschedules,
   resets and cramming, and that admits **same-day relearning reviews** as first-class events.
5. **Scheduler configuration changes as first-class events** — parameter vector, algorithm
   identity, and later desired retention if it is ever exposed.

### Standing constraint 1 needs widening a second time

The ticket asked whether choosing an algorithm forces the log to record more than raw grade and
timestamp, and instructed us to verify rather than assume. **It does.**

`delta_t` is **not** a difference of timestamps. It is a difference of **day buckets**, and the
bucket boundary is a rollover hour in a timezone. Two devices — or one device before and after a
settings change — will bucket *identical absolute instants* into different days and compute
different `delta_t`, different `(S, D)`, and different boxes.

So a log of `(card_id, grade, instant)` is **not replayable** under FSRS. The log must carry the
absolute instant *and the day-bucketing configuration in force at that moment*. This is the second
widening of constraint 1; the first added the event-kind discriminator.

**Clock skew** is unchanged by this decision but its stakes are confirmed: the projection is
strictly timestamp-ordered with `delta_t` in days, so a device with a wrong wall clock writes wrong
facts into a log that is immutable by design. That tension remains #9's to confront.

## Glossary

**Moved.** These terms are now of record in [`scheduling`](../../crates/core/src/scheduling/CONTEXT.md), per
[ADR-0009 §6](0009-crate-and-workspace-layout.md), which fixed where contexts live. They
were marked provisional here precisely so this could happen: the `CONTEXT.md` is
authoritative, and this ADR keeps the reasoning behind them.

## Consequences

- The Leitner promise "low boxes come round sooner" is **not** made. Boxes report durability only.
- Box movement is not a one-step ladder: passes on very overdue cards can jump several boxes, and a
  lapse drops several at once.
- Users who lapse often will live mostly in boxes 1–3. Thresholds are the lever if this needs
  changing.
- Personalised parameters require at least one device capable of running the optimiser; all others
  benefit without running it.
- Every card's schedule is reproducible from the log alone given the current parameter vector,
  provided the day-bucketing configuration is recorded.
- Changing the scheduler later is a re-derivation from the log, not a data migration. Scheduled
  dates already shown to the user do change — that is inherent to any swap.

## Open items handed onward

| Item | Owner |
|---|---|
| Event field list, encoding, clock skew, compaction | [Decide: the review event log format](https://github.com/amin-bf/cairn/issues/9) |
| What a "day" is — rollover hour and timezone | [Decide: the review event log format](https://github.com/amin-bf/cairn/issues/9) |
| New-card rate, daily limits, backlog pacing | [Decide: new-card introduction rate and daily limits](https://github.com/amin-bf/cairn/issues/21) |
| Whether the optimiser runs on web and Android, and at what cost | [Prove FSRS parameter optimisation runs in-client](https://github.com/amin-bf/cairn/issues/20) |
| Grade button presentation, where boxes appear, backlog feel | [Prototype: the review session experience](https://github.com/amin-bf/cairn/issues/11) |
| ~~Leech handling for repeatedly failed cards~~ — **closed by [ADR-0010](0010-leeches.md)**: detect and surface, never intervene. **The scheduler is untouched** — a leech is four failure *days* in a trailing ninety, and FSRS difficulty was rejected as the signal precisely because binding a user-facing surface to a scheduler parameter re-couples what constraint 4 decoupled | [#26 — leeches](https://github.com/amin-bf/cairn/issues/26) |
