# Replay

The join. Given the current content and the whole review log, replay computes what the collection
looks like right now: which cards exist, each card's memory state, and what is due. Everything it
produces is derived and disposable — the log is the only authority.

Depends on all three of `content`, `log` and `scheduling`, and nothing depends on it inside `core`.

**This context has no ADR of its own, and that is the reason it exists as a context.** Its rules were
each written for another purpose and are scattered across four documents; gathered here they are one
coherent mechanism, and separated they get reimplemented wrongly. Read
[ADR-0001 §7](../../../../docs/adr/0001-scheduling-algorithm-and-grade-scale.md) (replay purity),
[ADR-0002 §7](../../../../docs/adr/0002-the-card-model.md) (content and progress live apart),
[ADR-0004 §9](../../../../docs/adr/0004-the-review-event-log.md) (order, cache, no projection
versioning) and [ADR-0007 §2](../../../../docs/adr/0007-the-local-store.md) (the stored line is
authoritative).

## Language

**Replay**:
Deriving current state by reading the log in order and applying each row. The one mechanism that
keeps the scheduler swappable: changing the algorithm means re-deriving, never migrating.
_Avoid_: Projection, rebuild, fold, materialisation.

**Cache**:
Locally computed scheduling state. Disposable, never authoritative, never synced, never exported.
Lives in `derived.db` (see `store`), but *what* it holds is this context's decision.

**Derivation version**:
The constant identifying our replay arithmetic plus the pinned scheduler version. A mismatch deletes
the cache and forces a full replay. Bump it whenever a change would make old cached state disagree
with fresh arithmetic.

**Cache high-water**:
The `(writer, sequence)` the cache has consumed through. Behind is recoverable by replaying forward;
unprovable is discarded.

**Dormant card**:
A `CardRef` with events in the log that the current content no longer generates. **Not a stored
state** — it is the absence of a generated card, and it reattaches by itself if the content returns.
_Avoid_: Retired, deleted, orphaned, tombstoned — all of which imply a stored lifecycle that
deliberately does not exist.

**Due**:
A card whose next scheduled day has arrived, measured against the **device's local** day — not the
collection day scale, which is only ever used for stamping rows at write time. **A suspended card is
excluded from every due count** (ADR-0010 §8), or the count could never reach zero.
_Avoid_: Overdue, pending, scheduled.

**Failure day**:
A distinct day on which a card was graded `1 Forgot` at least once. The unit the leech rule counts,
**not** the grade-1 row — same-session re-shows are real logged rows with a zero day gap (ADR-0001
§5), so counting rows would treat one act of forgetting as three.
_Avoid_: Lapse count, failure count — both of which invite counting rows.

**Leech**:
A card with **four or more failure days in the trailing ninety**, measured with the device's local
day as the right edge (ADR-0010 §2). **Derived, never stored** — a query over replayed history, so
it is always current and self-clearing: learn the card and it leaves the list unaided. Carries no
scheduling consequence whatever.
_Avoid_: Difficult card, problem card — and never derive it from FSRS difficulty, which is a
scheduler parameter and would re-couple the surface to a scheduler the design keeps swappable.

## The mechanism, in one paragraph

The card set is computed from **current content**. The log is read in `(day number, then stable tie
break)` order, and each `reviewed` row is applied to the card its `CardRef` names; rows whose
`CardRef` names no currently-generated card are **retained and simply not projected**. `config-set`
rows change scheduler parameters from that point forward; rows before a `history-cutoff-set` are
ignored entirely. The result is memory state per card, from which `scheduling` derives the box and
the due day.

That paragraph is the whole prize of ADR-0002 §7: **card retirement does not exist**. There are no
tombstones, no lifecycle and no deletion events, because a card that stops being generated stops
being projected, and starts again if it returns.

## Rules that are easy to break silently

- **Replay takes no clock and no randomness.** Day numbers are frozen on the row at write time
  (ADR-0004 §4) and fuzz is seeded from `CardRef` (ADR-0001 §7). If replay ever reads the wall
  clock, two devices stop agreeing and the entire merge design is void.
- **Never delete a row because nothing projects it.** An unmatched `CardRef` is the mechanism, not
  garbage.
- **A cache that cannot prove how far it got is discarded, not trusted.** Losing the cache costs a
  full replay; trusting a stale one costs wrong memory state that looks right.
- **The projection is not versioned; the derivation is.** There is no migration path for cached
  state — bump the derivation version and let it rebuild.
- **No leech signal may reach memory state.** Leech-ness is read *out* of replayed history and never
  fed back into it (ADR-0010 §1). A lapse counter that nudges scheduling is folklore overriding the
  model, and ADR-0001 §5 rejected it before ADR-0010 existed.

## The highest-value test in the repository

ADR-0004 §2 makes merging set union with duplicates dropped. The claim the whole sync design rests
on is therefore: **any interleaving of two devices' rows replays to the same state.** That is a
property test over shuffled row sets, it needs no sync implementation and no second device, and it
is verifiable today.
