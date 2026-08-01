# Store

Where a collection is kept on disk, and where the platform seam lives. Two SQLite files via
`rusqlite` with `bundled`, plus the two directory lookups that locate them.

**Bound by** [ADR-0007](../../../docs/adr/0007-the-local-store.md), whose glossary this file
supersedes; also by [ADR-0004 §11](../../../docs/adr/0004-the-review-event-log.md) (relay byte for
byte), [ADR-0003 §5](../../../docs/adr/0003-client-stack.md) (the platform seam) and
[ADR-0020 §3 §4](../../../docs/adr/0020-protection-at-rest.md) (nothing here is encrypted, and no
secret is asked for).

## Language

**Collection database** (`collection.db`):
The authoritative file. Three tables: `log`, `mutable`, `local`. Nothing here can be lost except by
a failure of SQLite itself. **Unencrypted, permanently and by decision** — on the handset the platform
already answers every adversary an encryption layer would, and on the desktop any process running as
the user reads it. That desktop exposure is *conceded*, not overlooked
([ADR-0020 §2](../../../docs/adr/0020-protection-at-rest.md)); closing it needs a secret the user
supplies, which ADR-0020 §3 refuses because there is nowhere to recover a forgotten one from. The
drive credential lies in the same directory under the same rule, which is
[ADR-0013 §9](../../../docs/adr/0013-the-sync-transport.md)'s reason for declining a keyring.

**Derived database** (`derived.db`):
The disposable cache, attached to the same connection. Deletable at any time; losing it costs a full
replay and nothing else.

**Sequence high-water** (`local.seq_highwater`):
The highest sequence number *this install* has written. Both the source of the next sequence number
and — inverted — the self-heal detector: a log row above it means someone else is writing as us.
_Avoid_: Max seq, last seq — the names that invite the `MAX(seq) WHERE writer = me` bug.

**Writer marker**:
The copy of the writer id held outside the backup set. Its absence or disagreement means this
collection was copied here, and a fresh writer id must be minted.

**Collection id**:
A UUIDv4 naming *this collection*, held in `local`, minted once at first launch beside the writer
marker. **Adopted and never re-minted** — the exact opposite of the writer id, and the two rules must
never be swapped:

| | Writer id | Collection id |
|---|---|---|
| On finding one you did not mint | **never adopt** — mint fresh | **always adopt** — never re-mint |
| Why | a sequence number promises sole authorship | every device of one collection must agree |

It is **not** on the mutable surface — it must never settle — and **not** in the log, since it is not
an input to replay.

**Empty collection**:
One that has authored **no log rows under this device's own writer id and nothing on the mutable
surface**. The test that decides whether a device adopts a collection id it meets or refuses it. Not
"has no notes" — an imported deck with no reviews is still empty by this test.

**Line**:
The `log.line` column: the interchange row exactly as received. **The only authoritative column** —
every other column in the table, and everything in `derived.db`, is derived from it and may be
dropped and rebuilt.

**Clock-skew guard**:
The pair of measures from [ADR-0004 §8](../../../docs/adr/0004-the-review-event-log.md) that this
crate is the sole home of, because it is the edge where the wall clock is read. *Guard on write*: a
new row's instant is never at or below the highest already in the log — if the clock reads earlier
(the flat-battery boot), it is lifted to that highest plus a millisecond and the day is stamped to
match, so a bad clock cannot write rows that sort into an order that never happened. *Detect on
merge*: a merged `reviewed` row dated more than a day ahead of this device's clock is reported as a
`SkewWarning` and **never blocked** — someone is wrong, though never who. The remedy for skew already
written is the **history cutoff**, a collection-wide `history-cutoff-set` row that makes replay
disown every earlier `reviewed` row.

## Rules that are easy to break silently

- **`INSERT OR IGNORE` is for merge-ingest only. Our own writes use plain `INSERT`.** On another
  device's rows it is the union merge; on our own it silently drops a review.
- **Never take the next sequence number from `MAX(seq) WHERE writer = me`.** After a merge that
  continues *another* device's numbering. Use `local.seq_highwater`.
- **Every write transaction is `BEGIN IMMEDIATE`.** Sequence allocation is a read-modify-write; a
  deferred transaction loses updates between two processes.
- **Derived columns do not have to round-trip. Only `line` does.**
- **The writer marker lives outside the backup set** — never move it into the data directory.
- **`derived.db` lives outside the backup set too.** It is disposable by design, so backing it up
  protects nothing while burning the 25 MB platform quota and hastening the silent cutoff.
- **A writer id is never adopted; a collection id is never re-minted.** Applying either rule to the
  other identity is silent and destructive in opposite directions. At the seam where a device *meets*
  an identity (restore, enrolment), one rule decides both: an **empty** collection adopts the id it
  meets, a **non-empty** one refuses — and a refusal names the mismatch **and** the way out, never
  only "no" (ADR-0016 §10).
- **A new row's instant never sorts at or below the log's highest.** The clock-skew guard on write
  (ADR-0004 §8) lifts a backwards clock above the log rather than writing a row into an order that
  never happened. It needs every row's instant, so `ingest` populates the derived `instant` column
  for absorbed rows too — best-effort, NULL when the token is not the canonical form.
- **A cache that cannot prove its derivation is discarded, not trusted.** `derived.db` is stamped
  with `leitner_core::replay::DERIVATION_VERSION`; a missing or mismatched stamp clears it on open.
  The derivation is versioned, the projection is not — there is no cache migration (ADR-0004 §9).
- **WAL on both files; `synchronous=FULL` on the collection, `OFF` on the cache.**

## The platform seam

`platform/` is the entire platform surface of the whole application, and it is two functions wide:
`data_dir()` and `state_dir()`. Three `#[cfg]` arms, the third a `compile_error!` so an unrecognised
target fails the build rather than silently taking the desktop path.

**A third function appearing here means the seam is eroding.** Everything else in this crate is
portable — `rusqlite` with `bundled` compiles unchanged for desktop and Android, proven on the
handset in #7.

**The seam rule is per crate, not per workspace.** [ADR-0013 §12](../../../docs/adr/0013-the-sync-transport.md)
recorded a contradiction — ADR-0009's handoff sends every platform capability *here*, while the rule
above forbids a third arriving — and [ADR-0016 §5](../../../docs/adr/0016-backup-and-restore.md)
resolved it: a crate that must touch the platform for an unrelated reason gets **its own** `platform`
module under the same three-arm discipline. `leitner-export` has one (put/get/list for user-visible
files). **This module still stays at two functions**, and that limit is now load-bearing rather than
merely tidy — it is the reason the erosion signal still means anything.
