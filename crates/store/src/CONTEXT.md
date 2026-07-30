# Store

Where a collection is kept on disk, and where the platform seam lives. Two SQLite files via
`rusqlite` with `bundled`, plus the two directory lookups that locate them.

**Bound by** [ADR-0007](../../../docs/adr/0007-the-local-store.md), whose glossary this file
supersedes; also by [ADR-0004 §11](../../../docs/adr/0004-the-review-event-log.md) (relay byte for
byte) and [ADR-0003 §5](../../../docs/adr/0003-client-stack.md) (the platform seam).

## Language

**Collection database** (`collection.db`):
The authoritative file. Three tables: `log`, `mutable`, `local`. Nothing here can be lost except by
a failure of SQLite itself.

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

**Line**:
The `log.line` column: the interchange row exactly as received. **The only authoritative column** —
every other column in the table, and everything in `derived.db`, is derived from it and may be
dropped and rebuilt.

## Rules that are easy to break silently

- **`INSERT OR IGNORE` is for merge-ingest only. Our own writes use plain `INSERT`.** On another
  device's rows it is the union merge; on our own it silently drops a review.
- **Never take the next sequence number from `MAX(seq) WHERE writer = me`.** After a merge that
  continues *another* device's numbering. Use `local.seq_highwater`.
- **Every write transaction is `BEGIN IMMEDIATE`.** Sequence allocation is a read-modify-write; a
  deferred transaction loses updates between two processes.
- **Derived columns do not have to round-trip. Only `line` does.**
- **The writer marker lives outside the backup set** — never move it into the data directory.
- **WAL on both files; `synchronous=FULL` on the collection, `OFF` on the cache.**

## The platform seam

`platform/` is the entire platform surface of the whole application, and it is two functions wide:
`data_dir()` and `state_dir()`. Three `#[cfg]` arms, the third a `compile_error!` so an unrecognised
target fails the build rather than silently taking the desktop path.

**A third function appearing here means the seam is eroding.** Everything else in this crate is
portable — `rusqlite` with `bundled` compiles unchanged for desktop and Android, proven on the
handset in #7.
