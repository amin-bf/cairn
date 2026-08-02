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
marker. The type is `leitner_core::identity::CollectionId` (shared with `export`'s restore and
`sync`'s enrolment); this crate is the one place it is **minted**. **Adopted and never re-minted** —
the exact opposite of the writer id, and the two rules must never be swapped:

| | Writer id | Collection id |
|---|---|---|
| On finding one you did not mint | **never adopt** — mint fresh | **always adopt** — never re-mint |
| Why | a sequence number promises sole authorship | every device of one collection must agree |

It is **not** on the mutable surface — it must never settle — and **not** in the log, since it is not
an input to replay.

**Deck**:
A `{ id, name }` on the mutable surface (`entity = "deck"`): the `name` attribute, and a `deleted`
flag settling like any other value (ADR-0005 §4, §5, §7). Minted only by `create_deck` — **never
auto-created** (ADR-0005 §8), so a collection may hold zero. A note's membership is a single `deck`
attribute on the *note*, holding the deck's canonical id, so moving a note between decks is one
value changing (ADR-0005 §8); an id naming no held deck is **unfiled**, and a note derives deleted
from its **own** flag or its **deck's** (ADR-0005 §7).

**New-card rate**:
The single global integer bounding introductions (ADR-0011 §3), stored on the mutable surface under a
distinct `entity = "setting"` singleton (one row, a fixed all-zero id, attr `new_card_rate`).
`new_card_rate`/`set_new_card_rate` read and write it; the default (five), the range and the parse
live in `leitner-core::log`, so the store holds no domain rule of its own. A **personal setting**, not
content: it **syncs between a user's own devices but never exports** — the separate entity keeps it out
of any export that enumerates content by kind — and it **never enters the log** (ADR-0011 §5). Zero is
legal, the backlog escape hatch. The per-deck slot ADR-0005 §5 reserved stays empty (ADR-0011 §6).

**Scheduler parameters**:
The fitted 21-weight vector an optimisation run produces, recorded by `set_scheduler_parameters` as a
`config-set` parameter row — the **one place** the vector is persisted (ADR-0001 §6), and a **log
row, not the mutable surface**: it must replay in causal order so every device computes the same
memory state, which is exactly what the mutable surface's wall-clock settling would break.
`current_scheduler_parameters` reads the latest such row's weights (or the published defaults) —
what the write compares against. **An unchanged vector writes nothing** (ADR-0014 §5): a value-less
row still enters ADR-0004 §7's stamp contest and could displace a better fit, so `set_...` returns
`None` and emits no row when the vector equals what is current — which also disposes of a
history-less collection fitting the defaults, with no zero-history guard. The **fitted-over count**
travels on the row under `fov` and is **frozen at write time** (ADR-0014 §6), never re-derived here;
`leitner-core::interchange` emits the line and `leitner_core::replay::optimisation_nudge` reads the
count back. This crate holds no domain rule about *what* to fit — that is `leitner-core::scheduling`.
_Avoid_: Weights setting, a mutable-surface `params` attribute — settling the vector by stamp
recomputes memory state under the wall clock.

**Suspension**:
"Stop showing me this card" (ADR-0010 §5), stored on the mutable surface under a distinct
`entity = "suspension"` keyed by the `CardRef`'s canonical 18-byte encoding (attr `suspended`, `"true"`
while suspended, cleared to NULL to unsuspend). `suspend`/`unsuspend`/`is_suspended`/`suspended` are
the four operations; the last returns the set the review queue excludes from **every** due count and
introduction (ADR-0010 §8). **Per card, not per note** — one cloze blank or one direction of a pair may
be agony while its sibling is solid. Like the new-card rate it is **personal**: it settles by stamp,
**syncs between a user's own devices but never exports** (the separate entity keeps it out of any
content export), and it is **not a log row** — a toggle in the log would be settled by wall clock, the
one thing the stamp exists to prevent. A suspension whose card stops being generated goes dormant and
reattaches by itself, exactly as review history does; cleaning it up would be a bug.
_Avoid_: Suspend event, leech flag, buried, archived — the row kind ADR-0010 §5 ruled out.

**Reorder**:
Moving a note in authored order is `move_note_between`, which writes **exactly one** `position` value
between two neighbours (`leitner_core::content::order::between`) and **never renumbers** (ADR-0021 §3,
§4). Touching only the moved note is what makes reordering inside a *filtered* list well-defined:
hidden notes keep their keys and stay between the neighbours they were between. `create_note` places a
new note after `MAX(position)`; the two are the only writers of a note's place, and both are one write.

**Tag row**:
One tag on a note, stored as its **own** mutable attribute — `tag:<name>` set to `"true"`, cleared
to NULL to remove (see [`TAG_ATTR_PREFIX`]). One attribute per tag is what makes tags settle by
**set union** (ADR-0002 §10): two devices adding different tags offline write different attributes
and both survive, where a single joined `tags` value would let one addition overwrite the other —
the same trap ADR-0005 §8 named for a member list on a deck.
_Avoid_: A single joined `tags` value — it contends and loses additions.

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
- **A tag is one attribute per tag, never a joined `tags` value.** Set union is the required merge
  (ADR-0002 §10); it only holds because each tag settles independently. Collapsing them into one
  value silently reintroduces last-write-wins, and one device's added tag vanishes on the next merge.
- **No deck is ever auto-created**, and there is no default deck (ADR-0005 §8). Two never-synced
  devices would each mint a different id for a built-in default, producing an unmergeable duplicate
  the first time they meet. `create_deck` is the sole origin; zero decks is a legal resting state.

## The platform seam

`platform/` is this crate's whole platform surface, and it is two functions wide: `data_dir()` and
`state_dir()`. Three `#[cfg]` arms, the third a `compile_error!` so an unrecognised target fails the
build rather than silently taking the desktop path.

**A third function appearing here means the seam is eroding.** Everything else in this crate is
portable — `rusqlite` with `bundled` compiles unchanged for desktop and Android, proven on the
handset in #7.

**The seam rule is per crate, not per workspace.** [ADR-0013 §12](../../../docs/adr/0013-the-sync-transport.md)
recorded a contradiction — ADR-0009's handoff sends every platform capability *here*, while the rule
above forbids a third arriving — and [ADR-0016 §5](../../../docs/adr/0016-backup-and-restore.md)
resolved it: a crate that must touch the platform for an unrelated reason gets **its own** `platform`
module under the same three-arm discipline. `leitner-export` has one (put/get/list/hand_off for
user-visible files), and `leitner-app` has the third — one function returning the window's insets,
since an inset is a fact about the window the UI draws into and routing it here would make this crate
answer a question about layout ([ADR-0025 §2](../../../docs/adr/0025-the-authoring-screen-under-a-soft-keyboard.md)).
**This module still stays at two functions**, and that limit is now load-bearing rather than merely
tidy — it is the reason the erosion signal still means anything. The signal is a **fourth function
here**, never a fourth module elsewhere.
