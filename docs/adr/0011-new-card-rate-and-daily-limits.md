# ADR-0011: New-card introduction rate and daily limits

- **Status**: Accepted
- **Date**: 2026-07-30
- **Resolves**: [Decide: new-card introduction rate and daily limits](https://github.com/amin-bf/leitner/issues/21)
- **Map**: [Map: local-first Leitner app spec](https://github.com/amin-bf/leitner/issues/1)
- **Related**: [ADR-0001](0001-scheduling-algorithm-and-grade-scale.md) (FSRS-6, lapses, replay
  purity), [ADR-0002](0002-the-card-model.md) (notes, cards, siblings),
  [ADR-0004](0004-the-review-event-log.md) (the day scale, the mutable surface),
  [ADR-0005](0005-the-deck-model.md) (one collection-wide queue),
  [ADR-0006](0006-the-review-session-experience.md) (the session count),
  [ADR-0008](0008-the-deck-export-format.md) (deterministic emission),
  [ADR-0010](0010-leeches.md) (suspension)

## Context

The ticket asked how many new cards enter review per day and what limits a day has. It arrived
carrying five sub-questions, and by the time it was worked, two of them had already been answered
elsewhere:

- **What a day is** was settled by [ADR-0004 §4](0004-the-review-event-log.md): the collection-wide
  4am–4am scale exists only to stamp `delta_t` at write time, while *"due today" and daily limits
  use the device's local day*. That is binding here and is not reopened.
- **Whether queue composition corrupts replay** was settled by
  [Research: scheduling algorithms](https://github.com/amin-bf/leitner/issues/2): it does not.
  Which cards are *offered* never changes what the log records or what any interval computes. That
  finding is what licenses most of this ADR, and it is also what stops one argument from being
  reused where it does not apply — see §8.

What remained was genuinely open, and the ticket's own framing understated it. Two decisions
elsewhere had quietly removed the usual instruments:

- [ADR-0001 §7](0001-scheduling-algorithm-and-grade-scale.md) disabled load balancing, calendar
  shaping and sibling avoidance to buy replay purity. **That removes every mechanism that smooths
  daily workload after the fact.** Whatever enters the collection propagates forward almost
  undamped; interval fuzz survives, but it jitters a due date and cannot flatten a spike.
- [ADR-0001 §6](0001-scheduling-algorithm-and-grade-scale.md) fixed desired retention at 0.9,
  global and not user-exposed, because devices disagreeing about it compute different memory
  states. **So the workload knob most tools expose is unavailable by construction.**

The new-card rate is therefore not one lever among several. It is the only one left.

## Decision

### 1. There is no daily review limit; the session count stands alone

[ADR-0006 §1](0006-the-review-session-experience.md) already ships a bound on review work: the user
picks a session size, and a courtesy timer asks rather than tells. A daily cap would be a second,
overlapping mechanism, and where the two differ the daily cap is the worse half.

- **The session count is a choice made with today's appetite in hand. A daily cap is a prediction**
  entered once in a settings screen, which then overrides the appetite it was guessing at.
- **A daily cap has to refuse a user who wants to keep going.** ADR-0006 already faced that exact
  moment and decided against it — its timer was deliberately softened from a wall into a
  checkpoint, on the operative rule that *time's up leaves the user to decide*. A daily cap
  reintroduces the wall one level up.
- **Every hard question the ticket raised about limits exists only because the limit does.** What
  happens to overflow — deferred to tomorrow, or shown anyway? What does the box display say about
  a card that is due but withheld? Those are not solved here. They never arise.

**Therefore: no daily review limit. A user may start as many sessions in a day as they like, and
each is bounded only by the size they chose for it.**

**Accepted cost**: a user returning to a 400-card backlog gets no automatic protection. What they
get is ADR-0006 §7's framing — *"pick a comfortable size, the rest will keep"* at the picker and
*"N still waiting, that's fine"* at the end — plus §3's zero setting below. Nothing stops someone
grinding 400 cards in one sitting if they choose to, and nothing should: the design's position is
that an adult who wants to clear their backlog is not making a mistake that software should
prevent.

### 2. New-card introduction is capped, and the asymmetry with §1 is the decision

The symmetric reading of §1 is tempting and wrong: let the session count bound new cards too, and
take whatever is available. The two supplies are structurally different.

- **Due cards are self-limiting.** Today's due set is finite and was fixed by reviews already
  performed. Clear it and it is empty.
- **New cards are unbounded.** Import a 5,000-card deck and 5,000 cards are available on day one.
  Nothing in the system says no.
- **The two decisions have different horizons.** The session count decides *today's* effort.
  Introducing a card decides **every future day**, because the card returns for years. Enthusiasm
  on a Sunday afternoon is a poor instrument for setting next March's workload.
- **Nothing downstream smooths it**, per the Context above.

There is also a failure mode specific to leaving it uncapped, and it is invisible while it is
happening. A fresh deck offers a full session of nothing but new cards; those come due together a
few days later, on top of the next batch. Week one feels excellent and week three is a wall — and
by then the cards are in the log, where no setting reaches them.

**Therefore: new-card introduction is capped, and it is the only enforced limit in this design.**

The asymmetry is deliberate and is recorded as such so it does not read later as an inconsistency:
**reviews are bounded by choice, new cards by rule.** The rule exists exactly where the user cannot
see the consequence from inside the moment of choosing.

### 3. The cap is a user-set integer, and zero is the backlog escape hatch

Three forms were available.

**A derived rate is declined, and not for the obvious reason.** The `fsrs` crate ships
`expected_workload` and `optimal_retention`, and the ticket noted this is precisely what they exist
for. The argument that does *not* apply is ADR-0001 §7's — a derived rate would not break replay,
because queue composition is orthogonal to memory state. Declining it has to stand on its own:

- **The binding input is not in the collection.** The right rate is a function of how many minutes a
  day this person has. `expected_workload` predicts the *consequence* of a rate; it cannot know the
  *budget*. Deriving the rate from collection state answers the question with data that does not
  contain it.
- **A derived rate moves without the user acting**, which is the exact property that makes a
  workload spike feel unattributable — the thing they would adjust is the thing that adjusted
  itself.
- **`optimal_retention` is not a lever available here.** ADR-0001 §6 fixed retention at 0.9 for
  correctness, not taste; acting on the optimiser's advice automatically would unfix it.

**A fixed constant is declined** because available study time varies by an order of magnitude
between people, so any constant is wrong for nearly everyone.

**Therefore: the cap is a plain user-set integer — new cards per day — in the range 0 to 9,999.**
No automatic mode, no derived mode, no warning dialog: the setting is visible and the consequence
is explained where it is set, and a user who types 50 has made a legible choice that a modal would
only insult.

**Zero is a legal value and is the backlog answer.** A user buried in reviews sets new cards to
zero, clears the backlog, and turns it back on. This is a better instrument than any automatic
suppression rule, because it is visible, deliberate and reversible, where an automatic rule is a
behaviour the user has to reverse-engineer.

### 4. The default is five a day, derived from this design's own session budget

The number cannot be picked by feel, because the cost of a rate is not intuitive. Reviews accumulate
**logarithmically** per card and **linearly** in the rate.

With FSRS-6 published defaults at 0.9 retention and mostly-`3 Good` grading, a card's intervals run
roughly 3 → 8 → 20 → 50 → 130 → 330 days, so a card accrues about six reviews in its first year.
Modelling accrued reviews as `N(t) ≈ ln(1+t)` makes a card of age `t` contribute `1/(1+t)` reviews
per day, and summing over a stock introduced at `n` per day gives a daily load of approximately
`n × ln(T)`:

| new cards/day | after 1 month | after 1 year | after 3 years |
|---|---|---|---|
| 5 | ~17/day | ~30/day | ~35/day |
| 10 | ~34/day | ~59/day | ~70/day |
| 20 | ~68/day | ~118/day | ~140/day |

**This is an estimate from published default intervals, not a measurement.** `expected_workload`
would give the exact figure; the order of magnitude is what the decision needs, and it is not in
doubt.

Set that against the session budget this project already designed. ADR-0006 §1 pairs a 10-minute
timer with the count picker; at 20–30 seconds a card that is **roughly 25 reviews per session**. So
ten new cards a day implies two to three full sessions every day indefinitely, and twenty implies
four to five.

**Therefore: the default is five new cards a day** — the rate whose settled load fits inside about
one session of the shape ADR-0006 designed. The default is derived from this design's own stated
budget rather than inherited from convention, which also means it stays justified if the session
shape changes.

**The decisive property is that the error is asymmetric and one direction is irreversible.** Raising
the rate is free and takes effect tomorrow. Lowering it does **nothing** to cards already
introduced: they are in the log and they return for years. A default that is too low costs a few
days of mild impatience before the user changes it. A default that is too high costs a workload wall
three weeks in, whose only remedies are grinding it out or a `history-cutoff-set` that discards good
history along with the bad.

**ADR-0006's numbers are not touched.** 10/20/40 and the 10-minute timer stay as they are; §10 of
that ADR already records them as illustrative. This ADR notes only the relationship: **any rate
above roughly seven a day makes the 40-card ceiling too small to clear a day's settled load in one
sitting**, which is a fact to revisit against real usage rather than a reason to change either
number now.

### 5. The rate syncs on the mutable surface; the count is derived, and it is per device

Two questions usually conflated: where the *setting* lives, and where the *count* lives.

**The setting is not a log row.** ADR-0004 §1 fixed the log's membership test as *"is this an input
to replay?"*, and the new-card rate is not one — it decides what is offered, never what happened or
what any interval computes. This is the same test [ADR-0010 §5](0010-leeches.md) applied to
suspension and reached the same answer, and for the same second reason: a value the user toggles up
and down, placed in an append-only log, has its winner picked by a wall clock, which ADR-0004 §7
exists to forbid.

**Therefore: the rate is one value on ADR-0004 §7's mutable surface** — synced, settling by the
counter that jumps above any counter it sees, **never logged and never exported**. Settling
per-setting means optimising parameters on a laptop cannot silently revert a rate change made on a
phone.

**The count is derived, never stored.** A card counts against today if its **earliest `reviewed`
row falls in the device's local day**. Nothing new is persisted. This is the same move ADR-0006 §2
proved out for session position — derive from the log, never invent a second source of truth — and
it satisfies [ADR-0007's requirement on this ticket](0007-the-local-store.md) directly: *a daily
counter must be derivable from the log, never stored only in `derived.db`.* The cache may hold it
for speed; losing the cache loses nothing.

Two riders fall out of that definition rather than needing rules of their own:

- **A lapse re-show never counts against the cap**, because it is not an *earliest* row.
- **A card introduced, then failed and re-shown, counts once.**

**Accepted cost, recorded rather than fixed: two devices that have not synced today can each
introduce up to the full rate.** Enforcing a collection-wide daily count across devices that have
not met requires a rendezvous point, which is a server, which this destination does not contain —
and [Research: sync transport](https://github.com/amin-bf/leitner/issues/33) is unambiguous that a
device cannot learn what it has not received. The rate converges on the intended figure for anyone
who syncs; a two-device user who never syncs should set two or three rather than five.

**The carry-forward fix is explicitly rejected.** Treating the overshoot as a debt that suppresses
tomorrow's introductions would make today's new cards depend on *when the user happened to sync*,
so two devices replaying the same collection would offer different cards — and it quietly redefines
a daily rate into a running average. This is written down because it is the obvious-looking
improvement and it is a trap.

### 6. The rate is global, not per deck

[ADR-0005 §5](0005-the-deck-model.md) carved a slot for exactly this — *"a per-deck preference, if
#21 wants one, sits on the mutable surface keyed by deck id"* — so using it would cost nothing
structurally. It is declined anyway.

- **It would break the thing the cap exists to do.** ADR-0005 §6 fixed review as spanning the
  **whole collection in one queue**; there is no per-deck session. With per-deck rates the user's
  real daily obligation is the *sum* across every deck — a number that appears in no settings
  screen. Add a fourth deck at five a day and the workload rises by five with nothing on screen
  changing. The one figure this ADR protects becomes emergent.
- **It cuts against the established grain.** ADR-0005 §4 states no configuration lives on the deck,
  and ADR-0001 §6 made scheduler configuration global for correctness. A per-deck rate would be the
  first per-deck configuration in the system.
- **The underlying want is usually about order, not rate.** *"I want the language deck, not the
  trivia deck"* is a question about which new cards are picked, which §7 answers once instead of
  through N settings.

**Therefore: one global rate.**

**The known shape of the fix, if use proves it necessary**: a **per-deck new-card on/off** — a
boolean on ADR-0005 §5's slot, excluding a deck from introduction without fragmenting the total.
Recorded, not built, in the same spirit as ADR-0005's own deferred display-name override.

### 7. Notes carry a position, and new cards are introduced in that order

With one global rate drawing from the whole collection, selection order determines what the user
actually experiences. Two facts about the existing model shape this:

- **Notes carry no creation time, and their ids are deliberately random.** ADR-0002 §6 chose UUIDv4
  *"random rather than time-ordered"*, partly because the map's unresolved clock-skew tension is a
  standing reason to prefer identifiers that read no wall clock. So *"introduce them in the order I
  added them"* is not available — the information does not exist.
- **ADR-0008 §12 has a latent gap.** It requires the export to be byte-for-byte deterministic, which
  forces `notes.jsonl` lines into some fixed order — but no ADR says what that order is.

The cheap option is an arbitrary but stable order: sort by the 18-byte `CardRef`, or shuffle seeded
from card identity the way ADR-0001 §7 seeds fuzz. It costs nothing and every device agrees. Its
price is that **a published deck's authored sequence is destroyed** — a frequency-ordered vocabulary
course, the most common shape of shared deck there is, would introduce rare words before common
ones. Constraint 2 exists to make decks portable and publishable, and author-chosen sequence is a
large part of what a good shared deck *is*.

**Therefore: a note carries a `position`, a plain integer**, and never-reviewed cards are introduced
in `(position, ordinal)` order.

- **Assigned from a local high-water counter on creation**, from the `notes.jsonl` line index on
  import, ties broken by note id.
- **It need not be dense or globally unique** — only to sort. Two offline devices will assign
  overlapping positions; the tie-break makes that harmless, and no user-visible promise depends on
  the interleaving being what either user imagined.
- **It is a mutable value on the note**, settling by ADR-0004 §7 like any other, and **it travels in
  the export** — it is authored content, not a personal preference, by ADR-0005 §5's own test.

This closes ADR-0008 §12's gap rather than adding an obligation: the export already needed a fixed
emission order for determinism to be honest, and `position` is now it. One concept serves both.

> **Amended by [ADR-0021 §3](0021-note-ordering-saving-and-the-note-list.md): `position` is an
> **order key with infill**, not a plain integer — and the user *can* reorder notes, which is the
> question this section handed onward and is now discharged.**
>
> **The bullet above granted a freedom this section then specified away.** *"Need not be dense"* is a
> permission that the assignment rule never lets anyone exercise: a local high-water counter and a
> `notes.jsonl` line index both produce **consecutive** integers, so *"put this note between those
> two"* has no value to write. Every way of writing it by renumbering costs N writes on ADR-0004 §7's
> surface, each settling independently — and **order is a gestalt, so one lost value scrambles the
> whole list**: two devices reordering concurrently produce neither device's order, and nothing
> reports it.
>
> So `position` becomes a key that always admits a value between any two neighbours (a fractional
> index). **A move writes exactly one value, forever.** The three rules above survive with only the
> type changed — a key after the current last on creation, keys in line order on import, ties broken
> by note id — and *"need not be dense or globally unique"* becomes true and load-bearing rather than
> decorative. **Nothing reaches the export**: the file carries line order, not the value, exactly as
> the import rule above already said, so ADR-0008 §12's emission clause is textually unchanged and
> still byte-for-byte deterministic. *One concept serves both* survives the field becoming editable,
> which only this option allows — the two readers want different things, introduction order needing
> only "what comes next" where emission order needs the whole authored sequence.

### 8. The cap counts cards, and at most one card per note is introduced per day

**Counting cards rather than notes is forced by the cap's purpose.** An eight-blank `cloze` note is
eight cards of permanent load. Counting notes would let a single note consume eight days of budget
while reporting one.

Whether a note's cards may arrive together is the real question, and ADR-0002 §4 makes it
unavoidable: `vocab` and `basic-reverse` both generate two cards — `Term → Meaning` at ordinal 0 and
`Meaning → Term` at ordinal 1 — so most notes in a real collection generate siblings.

**Introducing siblings together corrupts the one measurement that has no history to dilute it.** The
two cards exist because recognition and production are genuinely different skills, worth scheduling
separately (ADR-0002 §1). But shown in the same session, the second card does not measure
production — it measures whether the user remembers a phrase they read ninety seconds ago. The
honest grade is `4 Easy`, and FSRS's initial stability for Easy is roughly four times its initial
stability for `3 Good`. That is a card's **first** review, so nothing averages it down. It
self-corrects at the eventual lapse, but the error is **systematic rather than random** — it applies
to every reverse and every cloze sibling in the collection — so it does not cancel out. It just
produces a predictable wave of failures a fortnight later.

**This is permitted where ADR-0001 §7's sibling avoidance was not, and the distinction is exact.**
§7 disabled sibling avoidance as a **due-date adjustment**: it read collection-wide state to move a
scheduled date, which breaks replay. This is **queue composition** — it decides which card is
*offered*, never what the log records or what any interval computes. Research established that
boundary and this sits cleanly on the permitted side. The distinction is written out because
"sibling handling is off" is the natural misreading, and it would forfeit the decision above.

**Therefore: the cap counts cards, and at most one card per note is introduced per day.**

Selection walks the `(position, ordinal)`-ordered list of never-reviewed cards, skipping any card
whose note already had a card introduced today, and takes up to the cap. **A suspended card is
skipped and never counted**, per [ADR-0010's requirement on this ticket](0010-leeches.md), for the
same reason §8 there removes it from the due count.

Two accepted costs: on a collection of two-card notes the days alternate between fresh notes and
yesterday's reverses — harmless, but visible; and a collection with very few notes introduces fewer
than the cap on some days, because there are not enough distinct notes to draw from.

### 9. A lapse re-show consumes the session count, and the session queue excludes only passes

Resolving the ticket's lapse question exposed a **contradiction between two accepted ADRs**:

- [ADR-0001 §5](0001-scheduling-algorithm-and-grade-scale.md) requires that *"a failed card returns
  within the same session, and that re-show is a real, logged, graded event"* with `delta_t = 0`,
  explicitly not a UI-only loop.
- [ADR-0006 §2](0006-the-review-session-experience.md) defines the session queue as *"due cards
  minus cards with a log entry"*.

Read literally, ADR-0006 excludes the failed card the instant it is graded, so the re-show ADR-0001
mandates can never happen. ADR-0006 was written from a prototype with no lapse path, so it was never
noticed.

**The queue is due cards minus cards whose latest review was a pass** (grade 2, 3 or 4). A failed
card stays because it genuinely is still due — FSRS gives it a sub-day interval — and leaves once
passed, with ADR-0001 §5's one-day floor applying from there.

**The session count counts gradings, not distinct cards.** Every graded event advances it, re-shows
included, so choosing 20 means at most 20 grade presses.

- **The count is a proxy for effort**, paired with a timer. A grading is the unit of effort; a
  distinct card is not, since a card failed three times is three cards' worth of work.
- **ADR-0006 §7's progress bar has to move when the user acts.** Counting distinct cards freezes the
  bar exactly on the sessions where the user is struggling — the worst possible moment for the
  interface to look stuck or punitive.
- **Ending a session with a lapsed card unresolved costs nothing.** There are no relearning steps in
  this design: ADR-0004 §1 has no relearning row kind and ADR-0001 §5 writes no lapse rule. A lapsed
  card is simply a card due very soon, reappearing next session or tomorrow under the one-day floor.
  There is no ladder position to lose and nothing to resume — which is the property ADR-0006 §2
  proved by force-stopping the app.

## Amendments to accepted ADRs

Following the precedent [ADR-0008](0008-the-deck-export-format.md) set, and
[ADR-0010](0010-leeches.md) followed.

| ADR | What changes | Why |
|---|---|---|
| [ADR-0002 §6](0002-the-card-model.md) | A note gains **`position`**, an integer used for introduction order and export emission order. Note ids stay random UUIDv4 and are **not** made sortable. | §7 above: authored sequence is a large part of what a shared deck is, and no existing field can express it. |
| [ADR-0006 §1](0006-the-review-session-experience.md) | The chosen session count counts **gradings**, re-shows included. | §9 above: a grading is the unit of effort, and the progress bar must move when the user acts. |
| [ADR-0006 §2](0006-the-review-session-experience.md) | The session queue is due cards minus cards **whose latest review was a pass**, not minus any card with a log entry. | §9 above: as written it made ADR-0001 §5's mandated same-session re-show impossible. |
| [ADR-0008 §12](0008-the-deck-export-format.md) | `notes.jsonl` emission order is **`position`, then note id**. | §7 above: §12 demanded byte-for-byte determinism without ever fixing the line order it depends on. |

No amendment is needed to ADR-0001 (§7's disabled adjustments are due-date adjustments; §8 above is
queue composition and does not touch them), ADR-0004 (nothing here enters the log; the rate rides
§7's existing mutable surface), ADR-0005 (§5's per-deck slot is left unused, deliberately), or
ADR-0010 (its requirement on this ticket is honoured in §8 unchanged).

## Requirements this places on downstream tickets

### [#28 — the note authoring and editing experience](https://github.com/amin-bf/leitner/issues/28)

1. **`position` is authored data, and whether the user can reorder notes is that ticket's call.**
   This ADR fixes only that the field exists, sorts, and travels. If reordering is offered, it is an
   ordinary mutable-surface edit settling by ADR-0004 §7.
2. **Introduction order is visible to the author**, so a deck built for sequence needs the authoring
   surface to show what that sequence is.

> **Both discharged by [ADR-0021](0021-note-ordering-saving-and-the-note-list.md), not by #28** —
> which closed without reaching either, which is why the map re-owned them.
>
> Requirement 1: **yes, and it is one mutable-surface edit exactly as anticipated** — but only because
> §7's type changed. Under the plain integer it would have been N edits, which is the hazard ADR-0021
> §3 closes.
>
> Requirement 2 is met **somewhere this ADR did not expect**. Sequence is visible on the **note
> list**, not in the editor: ADR-0021 §4 shows order as the list's own sequence and never as a number,
> because order is a property of the collection rather than of a note in isolation. *"How `position`
> is surfaced while authoring"* therefore has the answer **not in the editor at all**.

### [#37 — backup and restore](https://github.com/amin-bf/leitner/issues/37)

1. **The new-card rate is personal, not content**: it belongs to the progress profile, must survive
   a backup and restore, and must never appear in a `.ldeck` export (§5).
2. **`position` is content** and travels in the export (§7) — the opposite side of the same line.

### Sync transport ([#39](https://github.com/amin-bf/leitner/issues/39), [#40](https://github.com/amin-bf/leitner/issues/40))

1. The rate rides the **mutable surface**, so it inherits whatever answer sync gives it. Before sync
   exists the rate is per device, like every other mutable value.
2. **§5's per-device overshoot is a promise sync changes**: the sooner devices merge, the closer the
   effective rate is to the configured one. Worth stating to the user rather than leaving as a
   surprise.

## Glossary

New terms are of record in the `CONTEXT.md` files, per
[ADR-0009 §6](0009-crate-and-workspace-layout.md): **position** in
[`content`](../../crates/core/src/content/CONTEXT.md), which owns authored data; **new-card rate**
in [`log`](../../crates/core/src/log/CONTEXT.md), which owns the mutable surface it lives on;
**introduced** and **introduction candidate** in
[`replay`](../../crates/core/src/replay/CONTEXT.md), which owns the derived query; **session count**
in [`ui`](../../crates/app/src/CONTEXT.md), which owns the session.

## Consequences

- **The only enforced limit in the application is the new-card cap.** Everything else the user does
  is bounded by choices they make in the moment. That is a small enough surface to state in one
  sentence in a settings screen, which is the point.
- **Nothing in this ADR enters the event log**, so the ticket's closing question is answered
  negatively and constraint 1 needs no third widening. Queue composition and replay stay separate,
  which is what let §3, §7 and §8 be decided on their merits instead of on replay purity.
- **No card is ever due-but-withheld**, so nothing here can make the box display lie. The only cards
  held back are new ones, which have no box at all — ADR-0006 §6 shows them as `new`. Constraint 4
  is untouched.
- **A daily counter now exists that must be derived, not stored.** It is the second such query after
  ADR-0010's leech rule, and both answer to ADR-0007's warning: always current, incapable of going
  stale, and free to be cached because losing the cache loses nothing.
- **The default is folklore with an argument, and is labelled as such.** Five a day has a
  derivation from ADR-0006's session budget but no measurement behind it. Like ADR-0010's
  four-in-ninety, it is expected to move once there is real usage.
- **A note gains a field for the first time since ADR-0002.** The mechanism was always there —
  ADR-0002 §4's *"fields may be added; a note that predates the field reads it as empty"* — and an
  empty `position` sorting alongside a note id is a defined state, not a migration.

## Open items handed onward

| Item | Owner |
|---|---|
| **Workload prediction as advice** — showing *"at this rate, expect roughly N reviews a day once it settles"* using `expected_workload`. Useful, and explicitly never allowed to *control* the rate (§3). | **Out of scope** — [the map](https://github.com/amin-bf/leitner/issues/1), 2026-07-31. §3 already fixed the hard part, and the interim answer — this ADR's own estimate table — ships; what remains is a read-only figure beside the rate setting |
| **Per-deck new-card on/off** — shape known (§6), not built. | **Out of scope** — [the map](https://github.com/amin-bf/leitner/issues/1), 2026-07-31, on **scope not sharpness**: the decision was taken (defer) and the mechanism is written down, so what remains is a build. It carries one live sub-question a fresh effort inherits — [ADR-0005](0005-the-deck-model.md)'s row on whether such a preference syncs or stays device-local |
| ~~**Whether notes are user-reorderable**, and how `position` is surfaced while authoring~~ — **answered by [ADR-0021 §3 and §4](0021-note-ordering-saving-and-the-note-list.md)**: they are, `position` becomes an **order key with infill** so a move is one write rather than a renumber, and it is surfaced as the note list's own sequence — never as a number, and not in the editor at all | — *(was [#66](https://github.com/amin-bf/leitner/issues/66), **re-owned on 2026-08-01**: [#28](https://github.com/amin-bf/leitner/issues/28) was named here and closed without touching it, which the* Open items *sweep caught)* |
| **Revisiting 10/20/40 and five a day against real usage** — §4 records the relationship between them; neither number is measured. | Post-implementation |
