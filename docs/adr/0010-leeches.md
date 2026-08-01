# ADR-0010: Leeches — cards the user keeps failing

- **Status**: Accepted
- **Date**: 2026-07-30
- **Resolves**: [Decide: leeches — cards the user keeps failing](https://github.com/amin-bf/leitner/issues/26)
- **Map**: [Map: local-first Leitner app spec](https://github.com/amin-bf/leitner/issues/1)
- **Related**: [ADR-0001](0001-scheduling-algorithm-and-grade-scale.md) (no lapse rule of our own,
  the box means durability), [ADR-0002](0002-the-card-model.md) (cards are generated views; tags are
  content), [ADR-0004](0004-the-review-event-log.md) (the log's membership test; the mutable
  surface), [ADR-0006](0006-the-review-session-experience.md) (the session and its dashboard),
  [ADR-0007](0007-the-local-store.md) (one attribute table for the mutable surface)
- **Amends**: [ADR-0004](0004-the-review-event-log.md) (handoff table),
  [ADR-0006 §7](0006-the-review-session-experience.md) (the due count),
  [ADR-0007](0007-the-local-store.md) (§ requirements on this ticket)

## Context

[ADR-0001 §5](0001-scheduling-algorithm-and-grade-scale.md) deliberately declined this question.
FSRS-6 has **no lapse-count term anywhere in the model** — post-lapse stability is a function of
difficulty, stability and retrievability only — so the scheduler has no opinion about a card failed
repeatedly and never will. Whatever we do is card and deck management layered on top, not a
scheduling rule.

The problem is real regardless, and ADR-0001 §5 measured it: a lapse collapses stability to roughly
3–5 days from *any* box. A card the user genuinely cannot learn therefore returns every few days
forever. It never ages out on its own, and nothing in the model will ever make it.

Two prior decisions lean on the answer before it is given. [ADR-0002 §10](0002-the-card-model.md)
declined to serve *"cards that keep catching me out"* with tags, on the ground that it is
*"a query over history: always current, never needing maintenance"* — a position this ADR takes
literally. And [ADR-0004 §1](0004-the-review-event-log.md) left **suspension** explicitly undecided,
observing that it *"is a mutable flag rather than a fact that happened"* while its own handoff table
promised it as a fourth log row kind. Those two statements are incompatible; §5 below settles which
survives.

## Decision

### 1. There is a leech concept, and it only ever detects and surfaces

The app identifies cards that are hurting the user and shows them. It never suspends, reschedules,
tags or hides anything on its own. Every action is the user's.

**Doing nothing was the honest alternative** — no detection, no threshold, no new state, trusting the
user to notice a card that keeps coming back and edit or delete it. Both of those already exist and
cost the spec nothing. It is rejected because **noticing is the hard part, not acting**.
[ADR-0005 §6](0005-the-deck-model.md) puts every card in one collection-wide queue with no session
entity, so a leech never presents itself *as* a leech: it presents as a diffuse sense that reviews
are annoying, spread across a queue where the annoyance cannot be attributed to any particular card.
A card failed twenty times looks exactly like a card failed once when it comes up.

**Intervening automatically is rejected from the other side.** An automatic suspension the user does
not understand is worse than no feature — it makes cards vanish for reasons the user cannot see, in
an app whose entire display discipline (constraint 4, ADR-0001 §3) is built on never implying
something the data does not say.

The scheduler is untouched. ADR-0001 §5's *"no lapse counter influencing scheduling"* stands exactly
as written; nothing here feeds back into memory state.

### 2. A leech is four failure days in a trailing ninety — episodes, not rows

**Counted in failure *days*, not failure rows.** ADR-0001 §5 requires same-session relearning
re-shows to be real logged events with a zero day gap, so failing a card and then failing its
re-shows twice more that evening produces three grade-1 rows for **one act of forgetting**. Counting
rows would triple-count a single episode and make any threshold meaningless. Counting distinct
failure days fixes that, and fixes a second thing for free: a brand-new card fumbled four times on
its first evening is one episode, not four, so the rule cannot mistake ordinary learning for a leech.

**Measured over a trailing window of days, not over the card's lifetime.** A lifetime lapse count is
monotone in age and never forgives, so every mature card eventually crosses any fixed threshold and
the list slowly fills with cards that are fine. A card the user finally learned would stay flagged
forever. That is exactly the staleness ADR-0002 §10 rejected tags for — *always current, never
needing maintenance* is the bar, and a lifetime counter fails it. A trailing window makes the flag
**self-clearing**: learn the card and it leaves the list with nobody doing anything.

**A window in days rather than in reviews**, because the harm is a **budget** harm. A leech is a card
eating review time. A mature card failed twice a decade apart costs nothing, and a day window
excludes it for free — where "k of the last n reviews" would flag it, since a card reviewed twice a
year cannot accumulate recent failures but can easily accumulate a bad ratio.

**Four failure days within the last ninety.** The window's right edge is the **device's local day**,
consistent with ADR-0004 §4's handoff on "due today"; the failure days themselves are the frozen
collection-scale day numbers already stamped on the rows.

**The number is an invention, and is recorded as one.** ADR-0002 §10 already noted that no published
grade→box mapping exists; there is no better-founded leech threshold either. The one published
figure found is **eight lifetime lapses, with tagging rather than suspension as the default action**
— documented in an application manual and confirmed in that application's source, and it is folklore
rather than a measured quantity ([`docs/research/scheduling-algorithms/appendix-a-sm2-and-fsrs.md`](../research/scheduling-algorithms/appendix-a-sm2-and-fsrs.md)).
It is also a *lifetime* count, which this section rejects on its own merits.

What can be said for four-in-ninety is that it is roughly calibrated at both ends. At desired
retention 0.9 (ADR-0001 §6) a correctly scheduled card is *supposed* to lapse about a tenth of the
time, so a healthy box-3 card seeing 5–10 reviews in ninety days carries under one expected failure
— four is clear of noise. Meanwhile a card stuck in the post-lapse limit cycle returns every 3–5
days and trips the rule in about two to three weeks, which is soon enough to be useful.

**Both numbers are free to move.** Nothing in this design depends on them, in the same sense
[ADR-0006 §10](0006-the-review-session-experience.md) recorded for its own illustrative figures.

### 3. Rejected signals: difficulty, and the failure-to-review ratio

**FSRS difficulty is rejected, and this is the load-bearing rejection.** `D` is a **scheduler
parameter**. Binding the leech definition to it re-couples the user-facing surface to FSRS, which
constraint 4 and ADR-0001 §3 spent real effort decoupling — swap the scheduler and the concept
breaks with it, in an app whose whole storage design (ADR-0004 §1) exists to keep the scheduler
swappable.

It is also the wrong meaning. `D` says *"this card needs short intervals"*, not *"you are
suffering"*: a card at `D` near 10 that the user passes every time at four-day intervals is working
exactly as designed, not a leech. And it is a blunted signal by construction — FSRS-6 deliberately
adds mean reversion to the difficulty update to avoid the runaway-ease failure mode, so `D` drifts
toward the population mean. ADR-0001 §5 measured the practical consequence: difficulty moves
post-lapse stability by only about 5% across its entire range, so it is weakly coupled to the pain
it would be standing in for.

**The failure-to-review ratio is rejected** for the lifetime-monotonicity reason above, plus one of
its own: at desired retention 0.9 the "normal" value of the ratio is a function of a *setting*
rather than a constant, so the threshold would silently change meaning if retention were ever
retuned.

**Answer duration is not a trigger.** ADR-0004 §5 records it and offers it here as a leech signal.
It is too noisy for the definition — a user who puts the phone down mid-card records minutes of
"thinking" — but it has a real job in §6.

### 4. The list is ranked, not filtered

Because §1 makes nothing automatic, the threshold's only job is deciding what goes **on a list** —
and a list can be ordered rather than cut. Cards past the §2 floor are shown **ranked by recent
failure cost**, worst first, so there is no bright line to defend and no false-positive cost.

**The floor is still needed.** A pure ranking with no floor always shows the worst card in the
collection, presenting a healthy collection's worst card as a problem. The floor is what lets the
empty state say plainly that nothing is hurting the user.

### 5. Suspension is a per-card value on the mutable surface, never a log row kind

This overturns ADR-0004's handoff table and ADR-0007's requirement on this ticket, both of which
promised a fourth row kind. ADR-0004 §1's *prose* — *"a mutable flag rather than a fact that
happened"* — was right and its table was wrong.

**A toggleable flag in the log has its winner picked by a wall clock, which is the one thing
ADR-0004 §7 exists to forbid.** Suspension can be turned off and on again. In an append-only log
with no removal, the current value is "the latest such row", and across writers the log's order is
timestamp order (ADR-0004 §9). ADR-0004 §7 is explicit about why that is unacceptable: *"a device
with a fast clock would win every contest until real time caught up"* — with §8 already conceding
that a never-synced device with a bad clock writes permanently wrong stamps. Suspend on a laptop,
unsuspend on a phone, and a skewed clock decides. The mutable surface instead settles by **a counter
that jumps above any counter it sees**, which carries real causality and reads no clock. Putting
suspension in the log would be reimplementing §7 inside the log, badly.

**It also fails the log's own membership test.** ADR-0004 §1 fixed that test as *"is this an input to
replay?"* Suspension is not: a suspended card's stability, difficulty and box are exactly what its
reviews say they are, and nothing about memory state changes. It changes what is **offered**, which
is a queue question. The single widening §1 allowed — *raw facts about the act of reviewing that
cannot be reconstructed later* — is bounded to what the user did and when, and suspending is not an
act of reviewing.

**"No schema change" never discriminated.** ADR-0007 §7 made the mutable surface **one attribute
table with the stamp on the row**, so suspension costs no schema change there either; the argument
ADR-0007 offered in favour of the row kind applies equally to the alternative. Meanwhile ADR-0004
§10 never compacts, so a user toggling suspension would write permanent dead weight into a log
committed to being relayed byte for byte forever.

Three consequences:

- **Keyed by `CardRef`** — the derived `(note id, ordinal)` of ADR-0002 §6, in its canonical 18-byte
  encoding, already load-bearing as ADR-0001 §7's fuzz seed. ADR-0004 §7's rule that *every
  independently editable thing settles on its own* then extends with no special case, and one card's
  suspension never competes with another's.
- **Per card, not per note.** A leech is a specific card — one cloze blank, one direction of a
  vocabulary pair. ADR-0002 §1 makes those genuinely separate schedules, and the reverse direction
  may be solid while the forward one is agony. Note deletion is per note (ADR-0004 §7); suspension
  cannot inherit that shape.
- **It syncs, and it never exports.** Both of one user's devices must agree; a published deck must
  not carry it. ADR-0008 already amended ADR-0005 §5 to recognise that this slot holds authoring
  values that travel *and* personal ones that do not — suspension is squarely the second kind.

**A suspension whose card stops being generated goes dormant and reattaches by itself**, exactly as
review history does under ADR-0002 §7. It needs no cleanup, and cleaning it up would be a bug.

**`history-cutoff-set` needs no special handling.** A cutoff makes replay ignore earlier reviews, so
failures before it stop counting and the leech clears. If the user disowned the history, they
disowned the failures.

**No new event kind is required, so nothing returns to
[#9](https://github.com/amin-bf/leitner/issues/9).** The three row kinds of ADR-0004 §1 stand
unchanged.

### 6. Where the user meets the list

**A dedicated screen alone would fail.** §1's entire justification is that the user cannot notice on
their own; a view they must already suspect exists and go looking for requires exactly the noticing
that does not happen. Passive-only reproduces "do nothing" with extra code.

**Prompting inline during review is worse.** It is tempting — the card was just failed, the pain is
live — but it demands a considered judgement at the moment the user is most frustrated and least
able to make one, and decisions made in irritation route to *delete*. That is precisely the failure
mode §7 introduces suspension to prevent, so inline prompting would maximise it. It also cuts across
ADR-0006's deliberately low-friction session.

Therefore: **an end-of-session notice is the discovery path, and a dedicated screen is the durable
place to return to.** End of session is the right moment because the frame has changed — the count
is met, the user is reflecting rather than grinding — and ADR-0006 §1 set the precedent for exactly
this shape, where the ten-minute timer is *"a courtesy check-in, not an enforcement mechanism"*. The
notice is informational, dismissible and never blocking.

**The notice covers only cards that crossed the floor during the session just finished.** A notice
every session becomes wallpaper, but "cards new since you last looked" is not available to us:
ADR-0006 §2 stores no session position and ADR-0005 §6 has no session entity, so there is no "last
time" to compare against. Restricting the notice to cards whose crossing was *caused by a failure
logged in this session* needs none of that — it is a function of the rows the running app just
wrote. **Zero new state: no dismissal flag, no last-seen marker.** A card the user saw and chose to
ignore does not nag; it remains on the dedicated screen whenever they go looking.

**The notice is a pointer, not a decision point** — *"3 cards are costing you a lot"*, tap through to
see them. Same principle: do not invite a judgement in the moment. The list, where the card, its
failure history and its cost are all visible, is where the decision is made.

**This is where answer duration earns its keep.** Rejected as a trigger in §3, it does on the list
the one job nothing else can — it makes the cost **concrete**. *"22 reviews, 14 minutes, still
failing"* converts a vague annoyance into an actual decision in a way *"4 lapses"* does not.

### 7. Three actions — edit, suspend, delete — and never a tag

**Tagging is ruled out, and it would be a correctness bug rather than a preference.** ADR-0002 §10
made tags **content**, and ADR-0008 makes content travel in the `.ldeck` export. A leech tag would
publish the user's personal struggle into a deck someone else downloads. It is progress wearing
content's clothes, on the wrong side of constraint 2's split.

**Edit is the primary action.** The honest diagnosis of most leeches is a **defective card**:
ambiguous, too large, or testing two facts at once. Editing fixes the cause; everything else hides
the symptom. It already exists, and ADR-0002 §7 means an edit that changes the card set leaves the
history to reattach by itself.

**Suspension earns its place on a narrow argument: it is the safe version of the impulse that would
otherwise be a delete.** The realistic response at the end of a long session is not "let me rewrite
this properly" but "get this out of my face". Without suspension that impulse routes to deletion,
and ADR-0004 §7 is explicit that deletion discards content permanently — undeleting restores the
schedule but not the text. The choice is therefore not suspend-versus-nothing but
suspend-versus-losing-the-card.

**Delete remains available and unchanged.**

### 8. What suspension does to the numbers already on screen

**A suspended card is excluded from "due today".** ADR-0006 §7 puts a due count in a persistent
header dashboard and §8 makes *"nothing due right now"* an explicit worded state. A suspended card
left in that count while never being offered would make the number unable to reach zero and the
empty state unreachable — a header permanently reporting work the user is structurally unable to do.

This is **not** the box question. ADR-0001 §3's prohibitions are on boxes being sorted, counted, or
implying review frequency, and the box goes on meaning durability: a suspended card in box 2 is
still telling the truth about how durable that memory is, and makes no claim about the queue. This
is precisely the case constraint 4 was written to survive. The figure that has to change is the
separate due count ADR-0006 §6 explicitly carved out.

**Suspension is visible on the card itself**, so the user is never left wondering why a card is not
appearing. A display marker, not a box change.

**Suspended cards have a permanent home on the leech screen.** Once out of the due count, that
surface is the only place they exist, and suspend-and-forget would leave content quietly rotting
with no way back short of hunting through a browser. They appear as their own section, always, with
unsuspend available. **Suspension is never a one-way door.**

**Unsuspending needs no catch-up rule, and adding one would be a mistake.** A card unsuspended after
a year returns enormously overdue, and ADR-0001 §4 already handles exactly this: lateness is native
and unpenalised, `delta_t` is actual elapsed days, and low retrievability *increases* the stability
gain. The model absorbs it correctly. This is written down explicitly so that nobody later adds a
reset-on-unsuspend rule out of nervousness.

## Amendments to accepted ADRs

Following the precedent [ADR-0008](0008-the-deck-export-format.md) set in amending three accepted
ADRs.

| ADR | What changes | Why |
|---|---|---|
| [ADR-0004](0004-the-review-event-log.md), handoff table and §26 requirements | *"Suspension as a fourth row kind"* is **withdrawn**. Suspension is a value on the §7 mutable surface. | §5 above: a toggleable flag in the log is settled by wall clock, which §7 forbids, and suspension fails §1's own membership test. ADR-0004 §1's prose already said so. |
| [ADR-0007](0007-the-local-store.md), §26 requirements | *"Suspension as a fourth row kind needs no schema change"* is **withdrawn**; it needs no schema change on the §7 attribute table either. | §5 above: the argument never discriminated between the two homes. |
| [ADR-0006 §7](0006-the-review-session-experience.md) | The header's due count **excludes suspended cards**. | §8 above: otherwise the count cannot reach zero and §8's *"nothing due right now"* state is unreachable. |

No amendment is needed to ADR-0001 (§5 explicitly deferred this and its *"no lapse counter
influencing scheduling"* is preserved), ADR-0002 (§10's *"a query over history"* is honoured
literally), or ADR-0008 (nothing new enters the export).

## Requirements this places on downstream tickets

### [#21 — new-card rate and daily limits](https://github.com/amin-bf/leitner/issues/21)

1. **A suspended card is not introduced and is not counted** against any daily limit, for the same
   reason §8 removes it from the due count.
   > **Discharged by [ADR-0011 §8](0011-new-card-rate-and-daily-limits.md)**, which skips suspended
   > cards in the introduction walk. Note that the *daily limit* this anticipated turned out to be
   > singular: ADR-0011 §1 declines a daily review limit entirely, so the new-card cap is the only
   > count a suspended card could have entered.

### [#37 — backup and restore](https://github.com/amin-bf/leitner/issues/37)

1. **Suspension is part of the progress profile**, not of deck content — it must survive a backup and
   restore, and must never appear in a `.ldeck` export (§5).

### Sync transport ([#39](https://github.com/amin-bf/leitner/issues/39), [#40](https://github.com/amin-bf/leitner/issues/40))

1. Suspension rides the **mutable surface**, so it inherits whatever answer sync gives to
   ADR-0004's open item *"how the mutable store moves between devices — snapshot or change stream"*.
   It adds no new transport requirement.

## Glossary

New terms are of record in the `CONTEXT.md` files, per
[ADR-0009 §6](0009-crate-and-workspace-layout.md): **leech** and **failure day** in
[`replay`](../../crates/core/src/replay/CONTEXT.md), which owns the derived query; **suspension** in
[`log`](../../crates/core/src/log/CONTEXT.md), which owns the mutable surface it lives on.

## Consequences

- **The scheduler is untouched.** No leech signal reaches memory state, and ADR-0001 §5 stands
  unamended.
- **Leech-ness is stored nowhere.** It is a query over replayed history — always current, incapable
  of going stale, and satisfying ADR-0007's warning to #21 that anything counted must be derivable
  from the log rather than living only in the disposable cache.
- **The one new piece of state in this ADR is a per-card boolean with a stamp**, on machinery that
  already exists.
- **The thresholds are folklore, and are labelled as such.** Four-in-ninety has a calibration
  argument but no measurement behind it; it is expected to move once there is real usage.
- **A user who ignores the end-of-session notice is never nagged again about that card.** Deliberate
  — the dedicated screen is the recourse — and it means a leech can be silently tolerated
  indefinitely, which is the user's right.
- **Cards suspended on one device stop appearing on the other**, once the mutable surface syncs.
  Before sync exists, suspension is per-device, like every other mutable value.

## Open items handed onward

| Item | Owner |
|---|---|
| The exact visual design of the leech screen and the end-of-session notice | **Out of scope** — *the visual design pass*, which [ADR-0006 §10](0006-the-review-session-experience.md) opened and ADR-0015, ADR-0017, ADR-0018 and ADR-0019 have joined; [the map](https://github.com/amin-bf/leitner/issues/1) ruled it out on 2026-07-31, on the ground that these surfaces have specified existence, content and wording, so an agent fleet handed the spec today is not blocked |
| Whether the four-in-ninety thresholds survive real usage | Post-implementation, not a spec question |
| ~~How the mutable surface (and so suspension) moves between devices~~ — **answered by [ADR-0013 §7](0013-the-sync-transport.md)**: published **per writer**, which is what keeps conditional writes out of the design, and a writer's own counter being monotone means compacting its change stream to the latest value per key *is* a per-writer snapshot | [#39 — the sync transport](https://github.com/amin-bf/leitner/issues/39) |
