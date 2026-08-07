# Log

The append-only, immutable half of a collection, and the identity scheme that makes two devices'
logs mergeable by set union. Holds the **inputs to replay** — not "immutable facts", which scheduler
configuration violates.

Depends on `content` (a row carries a `CardRef`). Depends on nothing else.

**Bound by** [ADR-0004](../../../../docs/adr/0004-the-review-event-log.md), whose glossary this file
supersedes; also by [ADR-0002 §7](../../../../docs/adr/0002-the-card-model.md) and
[ADR-0001 §6](../../../../docs/adr/0001-scheduling-algorithm-and-grade-scale.md).

> **Naming hazard.** This module is called `log` and would shadow the `log` crate. It still cannot
> collide — but the guarantee **narrowed** when ADR-0027 admitted `fsrs`, which itself depends on
> `log`. What prevents the shadowing now is that `log` is not a **direct** dependency of
> `cairn-core`: only direct dependencies enter a crate's extern prelude, so a transitive one is
> invisible to our source (ADR-0009 §6, as amended by ADR-0027 §4). Adding `log` here for tracing
> would break it immediately — logging belongs at the edges, in `cairn-store` and `cairn-app`.

## Language

### Rows

**Review log**:
The append-only half of a collection. Reviews and scheduler configuration. Never compacted, never
migrated, never edited. Contrast **note store**, `content`'s term for the mutable half.

**Row**:
One entry in the review log. Immutable, self-contained, identified by `(writer id, sequence
number)`.
_Avoid_: Event — preferred only when speaking about the abstract occurrence, never the stored
artefact.

**Row kind**:
The discriminator naming what a row is: `reviewed`, `config-set`, `history-cutoff-set`. Unknown
kinds are **skipped, never errors** — this is what lets an old build relay a newer one's data.

**History cutoff**:
The instant before which replay ignores every `reviewed` row. The only repair available for a
clock-skew corrupted history, and it is collection-wide.

### Identity and ordering

**Writer id**:
The machine-owned random identifier of one sequential writer. Never reused, never adopted, not shown
to the user. **Not a device** — one device may own several over its life.

**Sequence number**:
A writer's own gap-free counter, incremented once per row it writes. Together with the writer id it
*is* the row's identity, so there is no separate event id.

**Version summary**:
`{writer id → highest sequence}`, computed by scanning a log. Answers "am I ahead of or behind that
device?". Lives in the sync handshake, never on a row.
_Avoid_: Vector clock — accurate but imports guarantees we do not make.

**Device label**:
The user-owned name for a device, grouping one or more writer ids. Mutable content, not a log row.

### Time

**Day number**:
The day bucket a review fell in, computed at write time under the collection day scale and **frozen**.
What replay uses — never recomputed, so changing the rollover hour cannot retroactively re-bucket
history.

**Day scale**:
The collection-wide timezone and rollover hour defining where a day starts. 4am.
Note that "due today" and daily limits use the **device's local** day instead, not this.

### The mutable surface

**Stamp**:
The `(counter, writer id)` pair attached to every mutable value, deciding which of two competing
values is later. The counter jumps above any counter it sees. **Never a wall clock.**
_Avoid_: Timestamp, version, mtime.

**Suspension**:
"Stop showing me this card" — a per-`CardRef` boolean on the mutable surface, settling by stamp like
any other value (ADR-0010 §5). **Syncs between the user's own devices; never exported**, because it
is personal progress and a `.cdeck` file carries only content. It is **not** a row kind and not an
input to replay: memory state is exactly what the reviews say it is, and suspension changes only
what is *offered*.
_Avoid_: Buried, archived, disabled, leech flag — and never "suspend event", which is the row kind
ADR-0010 §5 ruled out.

**New-card rate**:
How many cards may be **introduced** per day — a user-set integer, default 5, zero legal and the
intended answer to a backlog. A single **global** value on the mutable surface (ADR-0011 §3, §6):
syncs between the user's own devices, never exported, and **never a log row**, failing the same
membership test suspension fails. The *count* it bounds is derived by `replay`, never stored.
_Avoid_: New card limit, daily limit — there is no daily *review* limit at all (ADR-0011 §1), and
using "limit" for both invites one.

### Interchange

**Interchange form**:
The canonical JSON-lines encoding of ADR-0004 §11. Local storage may differ; it must round-trip.
Relayed **byte for byte and never re-encoded**, so an old build cannot strip a newer one's field.

## Rules that are easy to break silently

- **Never take the next sequence number from `MAX(seq) WHERE writer = me`.** After a merge that
  continues *another* device's numbering — the exact duplicate-writer failure this scheme exists to
  prevent. Use the sequence high-water in `store`.
- **Never adopt a writer id.** A sequence number promises "I wrote exactly rows 1..N and nobody else
  ever will", which only continuous possession establishes.
- **Guard writes against the log's own contents.** A device whose clock is years wrong writes rows
  that sort into an order that never happened; the guard is the only thing that catches it before
  the data is permanent (ADR-0004 §8).
- **Never add a row kind for something that toggles.** Across writers the log is ordered by
  *timestamp*, so a value that flips on and off would be settled by wall clock — the precise thing
  the stamp exists to prevent. Anything the user can turn off again belongs on the mutable surface.
  This is why suspension is not a row kind (ADR-0010 §5).
