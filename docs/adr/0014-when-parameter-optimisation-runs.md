# ADR-0014: When parameter optimisation runs

- **Status**: Accepted
- **Date**: 2026-07-31
- **Resolves**: [Decide: when parameter optimisation runs](https://github.com/amin-bf/leitner/issues/42)
- **Map**: [Map: local-first Leitner app spec](https://github.com/amin-bf/leitner/issues/1)
- **Related**: [ADR-0001](0001-scheduling-algorithm-and-grade-scale.md) (the parameter vector is
  collection state), [ADR-0003](0003-client-stack.md) (the Gradle-free APK, immediate mode),
  [ADR-0004](0004-the-review-event-log.md) (`config-set` rows, the settling rule),
  [ADR-0006](0006-the-review-session-experience.md) (the session, the header),
  [ADR-0008](0008-the-deck-export-format.md) (advance only on change),
  [ADR-0009](0009-crate-and-workspace-layout.md) (the platform seam),
  [ADR-0010](0010-leeches.md) (detect and surface, never intervene),
  [ADR-0011](0011-new-card-rate-and-daily-limits.md) (what the application enforces),
  [ADR-0013](0013-the-sync-transport.md) (the rendezvous point, and the foreground budget)

## Context

[ADR-0001 §6](0001-scheduling-algorithm-and-grade-scale.md) settled **where the parameter vector
lives** — collection state, published to the log as a `config-set` row, replayed by every device —
and deliberately said nothing about **when it is computed**. Nothing owned that question until
[Prove FSRS parameter optimisation runs in-client on Android](https://github.com/amin-bf/leitner/issues/20)
made it sharp rather than theoretical.

That ticket's measurements are the ground this ADR stands on, and two of them matter more than the
headline:

- **Compute cost is not the constraint.** A decade of the heaviest use
  [ADR-0004 §10](0004-the-review-event-log.md) contemplates — 730,000 reviews — trains in **4.3 s**
  in the foreground on the handset; a year of heavy use in **0.42 s**; a year of typical use in
  **0.15 s**. Stable across repeats, no thermal throttling, single-threaded, and only 1.6× slower
  than the development desktop.
- **The constraint is that Android freezes a backgrounded app.** The process moves to the
  `/background` cpuset (~13× the CPU time) and is then frozen outright — `isFrozen=true`, `utime`
  stopped dead. Training *stops*; it resumes only on return to the foreground. The 730,000 case
  reported **303 s of wall clock for 4.3 s of work**. A fire-and-forget background thread is
  therefore not an option.

Evidence: [`docs/research/fsrs-on-device/`](../research/fsrs-on-device/README.md).

Two further facts were established while working this ticket, by reading the pinned crate rather
than by measurement, and both remove guesswork from the design:

- `ComputeParametersInput` carries an optional **progress handle** exposing `current()` and
  `total()`. Progress is determinate; an indeterminate spinner is not forced on us.
- `CombinedProgressState` carries **`want_abort`**. Cancellation is cooperative and already
  supported; it costs one `bool`.

Three sub-questions had to be answered together, because each constrains the others: what triggers a
run, where it runs given the freezer, and what the user sees while it happens.

## Decision

### 1. Optimisation is explicit, and never automatic

**A run happens when the user asks for it.** Not on a review-count threshold, not on elapsed time,
not at app start, not on a schedule.

The obvious case for automation is that the user has nothing to decide with. Better-fitted
parameters are strictly an improvement, so asking is asking for a rubber stamp — and the objection
that a silent re-optimisation "rewrites the user's day behind their back" is weaker here than it
looks, because [ADR-0011 §1](0011-new-card-rate-and-daily-limits.md) removed the daily review limit
entirely. With the session count chosen at the picker each time, a schedule that shifts changes a
header number, not an obligation.

**The deciding argument is contention, not compute or courtesy.** Optimising is not a local
computation; it is a **synced write**. Each run emits a `config-set` row, and
[ADR-0004 §7](0004-the-review-event-log.md) settles competing values by counter-stamp. On a
threshold trigger, every device crosses the same threshold at roughly the same time, every device
trains, and every device writes — so the stamp contest of §6 below happens *every time the threshold
comes round, on every device, forever*, and all but one device's work is discarded. On an explicit
trigger it requires a human to deliberately press the button in two places inside a sync gap.

Automation would have made a rare, bounded failure into a routine one. That is the whole case.

**Accepted cost: personalised parameters are opt-in, and some users will never claim them.** The
measured benefit is real but modest — log-loss 0.3629 with the published defaults against 0.3437
optimised, from [#2's findings](../research/scheduling-algorithms/README.md) — and the defaults were
themselves fitted on several hundred million reviews, so the floor is a good one. A user who never
finds the button is not left somewhere bad.

### 2. The action is always present; the nudge states a fact and nothing more

**The action lives in settings and is never conditioned.** A button that is sometimes absent is
worse than one that is sometimes pointless: a user who goes looking and finds nothing learns the
feature does not exist.

**The nudge is a subtitle beneath it that reports two counts:**

> Fitted over 3,120 reviews. You've reviewed 1,400 times since.

and, where no run has ever happened — which
[ADR-0004 §6](0004-the-review-event-log.md) makes the *absence* of a `config-set` row, not a
default-valued one:

> Using the standard parameters. You've reviewed 4,200 times.

It carries **no threshold, no badge, no colour and no verb**. The user infers "that sounds like a
lot" without the application making a claim it cannot support. There is no floor below which it
stays silent, because there is no defensible number to use: the historical 1000- and 400-review
minimums are stale, the optimiser's own documentation states it works with any number of reviews,
and the only surviving guidance is the qualitative observation that fewer than a few hundred is
thin.

**It appears nowhere else — specifically not at the end of a session.**
[ADR-0010 §9](0010-leeches.md) already placed a pointer at that moment, and a second one competing
for it devalues both. End of session is also when the user is least interested in a settings chore.

This is the same shape [ADR-0010](0010-leeches.md) chose for leeches — *detect and surface, never
intervene* — and it transfers for the same reason: **noticing is the part the user cannot do;
acting is the part they can.**

### 3. It runs on a worker thread, and the freezer is not engineered against

Three candidates; two are disqualified by decisions already made.

| Where | Verdict | Why |
|---|---|---|
| Blocking the frame | **Rejected** | 4.3 s of training plus an uncosted corpus-build pass (see *Open items*), against Android's 5 s input-dispatch ANR threshold. The tail case risks the system killing the app, not merely a rude pause. |
| A foreground service or scheduled job | **Rejected** | [ADR-0003](0003-client-stack.md)'s measured prize is an APK that is a manifest plus one `.so` — no Java, no dex, no Gradle project in the repo, against 44 committed generated files for each alternative stack. A scheduler for a 4.3 s job spends that outright. |
| A worker thread, polled on the frame loop | **Chosen** | Already the house pattern: spawn it, store a handle, read the result on a later frame, call `ctx.request_repaint()`. The crate supplies the progress handle to read and the abort flag to set. |

**The freezer is not designed around, because the log write is the last step.** Nothing is persisted
until a complete vector exists, so a run that is frozen — or killed — holds **no partial state**.
There is nothing to resume, repair, roll back, or detect on next launch. If the user backgrounds the
app the run stops; if they return it continues and finishes. That is a slow outcome, not a broken
one, and the recovery action is the same button.

So: **no wakelock, no foreground service, no scheduled work, no keep-screen-on.** The user pressed a
button and is looking at the screen — the one state the freezer does not punish.

**Resumption is not promised.** At 730,000 reviews the process sits ~390 MB above baseline, which
makes a backgrounded process a prime candidate for the low-memory killer. It may be killed rather
than frozen. The conclusion is unchanged — nothing was written, press it again — but the user must
not be told the run is safely waiting for them.

### 4. What the user sees, and what is never claimed

**While it runs**, progress renders **inline in settings, in place of the button** — not a modal,
not a full-screen treatment. Four seconds is long enough to need feedback and short enough that
taking over the screen for it is disproportionate; at the typical scale measured (20,000 items,
0.15 s) it will often be gone before it is read. A **Cancel** control sets `want_abort`.

**On completion**, one factual sentence:

> Parameters updated. Due dates have been recalculated.

The second half is not decoration. [ADR-0001 §6](0001-scheduling-algorithm-and-grade-scale.md)
applies the current vector over the *whole* history, so every card's `(S, D)` is recomputed and
**every due date in the collection moves**. A user who is not told this sees the number in
[ADR-0006 §7](0006-the-review-session-experience.md)'s header change for no visible reason.

**Two things are never shown.** The 21 weights, before or after — twenty-one floats are unreadable
and, per ADR-0001 §6, meaningless without the algorithm identity anyway. And **any claim about
quality**: the 0.3629 → 0.3437 figure is a population average from someone else's benchmark, and we
have no instrument that tells *this* user whether *their* collection improved. Saying "your
scheduling is now more accurate" would be an unfalsifiable claim the application cannot back.

The §2 subtitle then re-reads on its own from the newly written row; no separate reset is needed.

### 5. An unchanged vector writes nothing

If the fitted vector is identical to the current one, **no `config-set` row is emitted**.

This is [ADR-0008 §11](0008-the-deck-export-format.md)'s shape applied to a second artifact: the
deck revision advances only when the content digest changes, so relaying an unmodified deck emits no
phantom revision. Here the cost avoided is larger than a wasted row — a value-less write still
enters the stamp contest of §6, so it can *displace* a genuinely better vector while changing
nothing.

It also disposes of the degenerate cases without a special path. A collection with no review history
fits the defaults, the result equals what is current, and nothing is written — so §2's "the button is
always present" needs no zero-history guard.

### 6. The cross-device race is accepted, and the fitted-over count is recorded, not derived

**The race.** [ADR-0004 §7](0004-the-review-event-log.md) settles competing values by counter-stamp
— real causality, and deliberately not a wall clock — but the counter knows nothing about **merit**.
A device that has not merged another's recent reviews can fit a vector on a partial log and, if its
counter is higher, overwrite one fitted on the full history.

**It is accepted.** Three things bound it. §1 already did most of the work: an explicit trigger
makes this need a human pressing the button in two places inside a sync gap, where a threshold
trigger made it routine. It is **self-correcting** — the next run on any device, over a merged log,
replaces it. And the magnitude is small: the entire distance between the published defaults and an
optimised fit is 0.3629 → 0.3437, and a fit over 95% of the history sits far inside that.

**The obvious mitigation — "refuse to optimise while behind" — is rejected, but not for the reason
it first appeared.** This ADR was drafted arguing that knowing you are behind requires a rendezvous
point the destination does not contain. **That argument is wrong, and
[ADR-0013](0013-the-sync-transport.md) is why**: it chose a rendezvous point, and made the listing
*itself* the version summary, so "am I behind?" costs one round trip. The finding that looked
decisive — that a transport family **cannot answer "am I behind?" even in principle**, a listing
reporting what arrived and never what exists elsewhere — belongs to
[Research: sync transport](https://github.com/amin-bf/leitner/issues/33) and applies to the
*folder-sync* family, which was rejected. Applying it to the transport that won was an
over-generalisation, and it is recorded here rather than quietly deleted because the corrected
version changes what we build: §7 below.

It is still rejected **as a refusal**. A device may be un-enrolled, offline, or hit a failed sync,
and an offline device optimising on its own history is a perfectly good outcome — blocking it would
withhold a feature that works fine without a network to serve a race that is bounded and
self-correcting. What the cheap answer buys is *ordering*, not *veto*.

The residual therefore stands, and it is the same class of loss
[ADR-0011 §5](0011-new-card-rate-and-daily-limits.md) accepted when it let two unsynced devices each
introduce a full day of new cards.

**What is added instead: the `config-set` row records how many reviews the vector was fitted over.**

This is not a convenience, and the reasoning is the point. It looks derivable — find the row, count
the review rows before it — and **it is not, precisely because of the race.** A device that trained
while behind wrote a vector fitted over 3,120 reviews; a later scan of the *merged* log around that
same row counts 8,000 and reports a fit that never happened. Derivation overstates the fit exactly
in the case where honesty matters most. So the count is **frozen at write time**, for the same
reason [ADR-0004 §4](0004-the-review-event-log.md) freezes the day-bucketing on each row: a value
that describes the moment of writing cannot be recovered by looking at the world afterwards.

The payoff is that **the race becomes self-diagnosing at no extra cost.** §2's subtitle reads
*"fitted over 3,120 reviews"* while the collection holds 8,000; the user sees a stale fit and presses
the button, which is the correcting action anyway. The instrument that reports the problem and the
instrument that fixes it are the same one.

Arbitration is **not** changed: values still settle by stamp, never by fitted-over count. Making
merit arbitrate would be a second settling rule, and ADR-0004 §7 exists to have exactly one.

### 7. Pressing Optimise syncs first, where a transport is enrolled and reachable

[ADR-0013](0013-the-sync-transport.md) handed this ADR one item by name: **publishing and optimising
compete for the same foreground window.** Android freezes a backgrounded app, so both want the
moment the user has the app open — sync in seconds, optimisation in up to 4.3 s at decade scale, out
of one budget. It assigned the ordering here.

> **Sync, then train.**

The ordering is not housekeeping, and that is why it belongs in this ADR rather than in the sync
experience: **it is the cheapest available reduction of the race §6 accepts.** ADR-0013 §6 makes
"am I behind?" one round trip, and a round trip against a 0.15–4.3 s job is noise. Optimising on a
knowingly stale log when refreshing it costs one request is not defensible — the whole value of a
fitted vector is the history it saw.

**It is a sequence, never a gate.** If no transport is enrolled, if the device is offline, or if the
sync fails, **training proceeds anyway** on local history, and the completion message makes no
different claim. The only thing the user loses is the reduction, which is exactly the case §6's
fitted-over count exists to make visible afterwards.

**No new UI state.** The sync is the leading part of §4's indeterminate first phase, alongside the
corpus build. The user sees one action with one progress treatment, because from their side it is
one action.

**What this does not decide**: when sync runs *otherwise* — on open, on a timer, on a gesture —
which is [#40](https://github.com/amin-bf/leitner/issues/40)'s, untouched. This settles only what
happens inside the one action this ADR owns.

### 8. Desktop and Android do not diverge

**Both platforms get the same answer, and get it by construction rather than by effort.**

Every mechanism above is platform-neutral — a button in settings, a worker thread polled on the
frame loop, an inline progress bar, a `config-set` row. None has a platform arm. The single
asymmetry is the freezer, which exists only on Android, and §3's answer to it is *do not engineer
against it* — a no-op on desktop.

That matters beyond tidiness. [ADR-0009](0009-crate-and-workspace-layout.md) fixes the platform
surface at **two functions and three `#[cfg]` arms**, and records that **a third function appearing
there means the seam is eroding**. A per-platform optimisation path would want exactly that third
function. Divergence is not merely unnecessary here; it costs something the spec has already priced.

**The softer divergence is rejected too**: keeping the trigger on both but nudging only on desktop,
on the theory that a laptop is the more comfortable place for a 4.3 s job. The measurements refute
the premise — 1.6× slower, and not penalised at all inside a foreground app process — and a
desktop-only nudge would quietly reintroduce the desktop-only feature
[#20](https://github.com/amin-bf/leitner/issues/20) spent a whole ticket killing.

ADR-0001 §6's publish-84-bytes mechanism is untouched and still does real work. It is simply what
happens on **every device that is not the one where the button was pressed** — which is the normal
case, not the fallback case.

## Amendments to accepted ADRs

Following the precedent [ADR-0008](0008-the-deck-export-format.md) set, and
[ADR-0010](0010-leeches.md) and [ADR-0011](0011-new-card-rate-and-daily-limits.md) followed.

| ADR | What changes | Why |
|---|---|---|
| [ADR-0004 §6](0004-the-review-event-log.md) | The **scheduler parameters** setting gains a third member: the **fitted-over count**, the number of reviews the vector was trained on, frozen at write time. The setting still settles as one unit. | §6 above: the count is not derivable from the merged log without reporting a fit that never happened, and it is what makes the accepted race self-diagnosing. |

**No amendment is needed elsewhere.** ADR-0001 §6 is confirmed rather than changed — this ADR
answers the *when* it explicitly left open, and its publish-and-consume mechanism is untouched (§8).
ADR-0006 is untouched: nothing here changes the session, and the due-count movement described in §4
is the header reporting a real change, not a new display rule. ADR-0010 §9's end-of-session slot is
deliberately left to it (§2). ADR-0003's Gradle-free APK is preserved by §3 rather than spent.

## Requirements this places on downstream tickets

### [#40 — the sync experience](https://github.com/amin-bf/leitner/issues/40)

1. **A parameter vector changing mid-session shifts the queue under the user, and no local
   mechanism can prevent it.** [ADR-0006 §2](0006-the-review-session-experience.md) derives session
   state from the log every frame, so a new vector recomputes every `(S, D)` while a session is
   open. The tempting fix — block review while a run is in flight — **cannot work**, because
   ADR-0001 §6 makes the vector *collection state that arrives by sync*: a merge landing another
   device's vector is the identical event, and it has no local trigger to gate on. This is
   structurally a sync-experience question and is handed over whole.
2. **Optimising is a synced action, and the user should be told so.** ADR-0001 §6 already accepted
   that a user who optimises on a laptop while offline will not see their phone's schedule change
   until the log merges. §2's fitted-over subtitle is the surface where that latency becomes
   visible.
3. **§7 settles the ordering ADR-0013 assigned here, and nothing more.** Pressing Optimise syncs
   first; *when sync runs otherwise* — on open, on a timer, on a gesture — is untouched and remains
   this ticket's. If that answer makes a sync already-recent at the moment the button is pressed,
   §7's leading sync is free to be skipped: it is there to make the training input current, not to
   be a ritual.

### [#37 — backup and restore](https://github.com/amin-bf/leitner/issues/37)

1. **The fitted-over count is part of a `config-set` row**, so it travels wherever the log travels
   and needs no separate handling. Stated only so it is not mistaken for a derived column that a
   restore may recompute — §6 is explicit that recomputation produces a wrong answer.

## Glossary

New terms are of record in the `CONTEXT.md` files, per
[ADR-0009 §6](0009-crate-and-workspace-layout.md): **optimisation run** and **fitted-over count** in
[`scheduling`](../../crates/core/src/scheduling/CONTEXT.md), which already owns **scheduler
parameters**.

## Consequences

- **Personalised parameters are an opt-in feature.** This is the largest thing given up, and it is
  given up knowingly: the alternative made a bounded cross-device failure routine, and the floor —
  published defaults fitted on several hundred million reviews — is good.
- **The application still enforces exactly one limit.**
  [ADR-0011](0011-new-card-rate-and-daily-limits.md) recorded that the new-card cap is the only
  enforced limit; nothing here adds a second. Optimisation is a thing the user does, not a rule the
  application applies.
- **A second acceptance of the same shape is now on record.** ADR-0011 §5 accepted two unsynced
  devices each introducing a day's new cards; §6 here accepts a behind device overwriting a
  better-fitted vector. Both are self-correcting on the next merge. Note the roots differ now that a
  transport exists: ADR-0011's is structural, while §6's residual survives only where sync is
  absent, offline or failed — §7 removes the rest. If a third appears, the pattern is worth naming
  rather than re-argued.
- **An argument in this ADR was falsified by an ADR that landed while it was being written**, and
  the correction is left visible in §6 rather than edited out. ADR-0013 arrived mid-session and
  turned "you cannot know you are behind" from true into false, which changed a rejection into a
  sequencing decision (§7). The reading order in `CONTEXT-MAP.md` exists for exactly this.
- **A value is frozen at write time for the second time in this spec.** ADR-0004 §4 froze day
  bucketing; §6 here freezes the fitted-over count. Both for the same reason: a fact about the
  moment of writing cannot be recovered by inspecting the world later. This is now a shape to reach
  for, not a one-off.
- **Nothing new is stored outside the log.** The trigger is a button, the progress is frame-local
  state, and the only persistence is a row that ADR-0004 §6 already defined. No new table, no new
  cache, no new mutable-surface value.
- **The `AGENTS.md` client-stack section gains a rule** — a backgrounded Android app is frozen, not
  slowed. It is a fact no test catches and that an agent would otherwise design straight into.

## Open items handed onward

| Item | Owner |
|---|---|
| **The corpus-build pass is uncosted.** The measured 4.3 s is `compute_parameters` alone; turning log rows into `FSRSItem`s expands *one item per review carrying its full prefix*, and at 730,000 reviews is not obviously the smaller half. No decision here depends on the answer — a worker thread does not care how long the work is, and §4 makes no time promise — but it must be measured before the progress treatment is tuned. | Implementation |
| **Progress is two-phase, and the first phase cannot be cancelled.** The crate's progress handle and `want_abort` cover *training* only, so the corpus build has no `current()`/`total()` to read and no abort to honour: an indeterminate lead-in, then the determinate bar. Recorded here so it is not discovered halfway through building §4. | Implementation |
| **A run completing mid-session shifts the queue**, and no local lockout can fix it (see Requirements). | [#40](https://github.com/amin-bf/leitner/issues/40) |
| **Whether the nudge is discoverable enough in practice.** §1 accepts that opt-in leaves the improvement unclaimed by users who never look; §2 deliberately declines to be pushier about it. Worth revisiting against real usage, like ADR-0010's four-in-ninety and ADR-0011's five a day. | Post-implementation |
