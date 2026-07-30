# ADR-0007: The local store

- **Status**: Accepted
- **Date**: 2026-07-30
- **Resolves**: [Decide: the local store](https://github.com/amin-bf/leitner/issues/12)
- **Map**: [Map: local-first Leitner app spec](https://github.com/amin-bf/leitner/issues/1)
- **Evidence**: [`docs/research/client-stacks/storage-and-contenders.md`](../research/client-stacks/storage-and-contenders.md),
  plus the Android storage proof recorded in
  [`docs/environment/android-toolchain.md`](../environment/android-toolchain.md)
- **Related**: [ADR-0002: The card model](0002-the-card-model.md),
  [ADR-0003: The client stack](0003-client-stack.md),
  [ADR-0004: The review event log](0004-the-review-event-log.md),
  [ADR-0005: The deck model](0005-the-deck-model.md)

## Context

ADR-0004 fixed the log's logical content and one canonical interchange form, and explicitly did not
own where any of it lands on a device. It handed this ADR four requirements by name: round-trip the
interchange form exactly *including fields it does not understand*; own the disposable cache and its
invalidation; provide efficient append and per-card review lookup; and give the mutable surface
per-**value** stamps rather than per-record ones.

Three earlier decisions constrain the answer. ADR-0003 chose a stack whose central property is a
**compile-time storage seam** — a target that does not compile fails the build — and warned that
introducing a runtime platform branch would erode the thing the whole stack choice rests on.
ADR-0002 split content from progress into two stores with different rules. ADR-0005 put deck
membership on the note and left a slot for per-deck preferences on the mutable surface.

Research ([#3](https://github.com/amin-bf/leitner/issues/3)) left one question open for this
ticket in as many words: *where the seam sits* — at bytes or at SQL.

The answer turned out to depend on a scoping question nobody had asked, so this ADR settles that
first.

## Decision

### 1. The web target is dropped, and this is where that was decided

**The clients are desktop and Android. The web target is out of scope for this destination**, and
returns only if a server of our own is ever built — which is itself out of scope, so it does not
graduate as fog. The map records it under **Out of scope**.

The reason is not that browser storage is awkward. It is that **for an app whose only copy of the
data is local, the browser is the one platform where "local" is not reliably durable**:

- Origin storage is evicted under storage pressure, and eviction deletes *all* of an origin's data
  together rather than selectively.
- The API that asks for exemption is heuristic and documented as guaranteeing nothing: it resolves
  true only if permission is granted, and *"the browser may or may not honor the request depending
  on browser-specific rules. There's no guarantee of persistence."*
- One major browser proactively deletes script-created data after seven days without user
  interaction.

The storage mechanics compound it rather than causing it. Appending without a dedicated Worker
rewrites the whole file, because the async writable stream *"is typically implemented by writing
data to a temporary file, and only replacing the file… when the stream is closed"*, with
`keepExistingData` meaning *"the existing file is first copied to the temporary file"* — so one
review appended to a 100 MB log costs 100 MB of copying. The durable path (`SyncAccessHandle`) is
*"exclusively available in Dedicated Web Workers"*, while our UI loop is on the main thread. And the
no-Worker database path supports only `synchronous=off`, writes changed blocks asynchronously with
no cross-block atomicity, and warns that *"using it on multiple pages may cause DB corruption."*

A web client is coherent when a **server** is the system of record and incoherent when the browser
is. Given the map fixes *no server of our own*, the honest move is to drop the target rather than
ship a third platform whose data can vanish.

**Two knock-ons, recorded rather than left implicit:**

- **ADR-0003 is not reopened, and is mildly strengthened.** Most of what egui knowingly gave up was
  web-only — accessibility, text selection, find-in-page. What does *not* change is Android's
  missing IME, which is a winit limitation and has nothing to do with the web target.
- **Desktop is now the sole authoring surface for non-Latin content.** ADR-0003 §6 accepted
  Latin-only Android input because authoring could happen "on desktop or web". It is now desktop,
  full stop, and sync remains the only route by which Persian content reaches the phone. This
  tightens the map's sync fog; it does not loosen it.

### 2. One SQLite database per collection, and the raw interchange line is the authoritative column

**The store is SQLite, via `rusqlite` with the `bundled` feature, on both targets.** It is already
proven rather than assumed: cross-compiled for arm64-v8a and run on the real handset with a
persisted database, packaged into an APK whose `.so` loads in-process
([#7](https://github.com/amin-bf/leitner/issues/7)).

The log table stores **the interchange line verbatim**, and everything else is derived from it:

```sql
CREATE TABLE log (
    writer   BLOB    NOT NULL,       -- 16 bytes, not 36 characters of text
    seq      INTEGER NOT NULL,
    line     BLOB    NOT NULL,       -- authoritative: the §11 interchange row, byte for byte
    kind     TEXT    NOT NULL,       -- everything below is derived from `line`
    note     BLOB,                   -- NULL for rows that are not `reviewed`
    ordinal  INTEGER,
    day      INTEGER,
    instant  INTEGER,                -- milliseconds since epoch
    PRIMARY KEY (writer, seq)
) WITHOUT ROWID;

CREATE INDEX log_replay ON log (note, ordinal, day, instant, writer, seq);
```

Three properties fall out of this shape rather than being separately engineered:

- **Round-tripping unknown fields is structural, not a discipline.** ADR-0004 §11 requires a row to
  be relayed byte for byte and never re-encoded. Shredding JSON into typed columns can only honour
  that by *also* keeping the original line — so the line has to be there regardless. Once it is, an
  old build physically cannot strip a field a newer build wrote, because it stores the bytes it
  received.
- **Merging is a primary key conflict.** ADR-0004 §2's set-union-with-duplicates-dropped is
  `INSERT OR IGNORE` on `(writer, seq)`; two rows with the same pair *are* the same row.
- **`log_replay` is ADR-0004 §9's sort order exactly** — day, then instant, then writer, then
  sequence — so a card's replay is one index scan already in the right order, with no sorting step
  and no separate mechanism for the per-card lookup requirement.

**The derived columns do not have to round-trip; only `line` does.** This is stated explicitly
because it is the sort of asymmetry a later reader "fixes". Reducing an ISO instant to integer
milliseconds may not re-serialise byte-identically, and that is fine: the canonical bytes are in
`line`, untouched. The derived columns exist to be indexed, not to be authoritative.

`grade` and `duration` are deliberately **not** derived into columns. The only thing that reads them
is replay, which is reading the line anyway.

#### What was rejected, and why

- **A byte-level seam** — `trait LogStore { append(bytes); read_from(seq); }` over an append-only
  file, with all projection logic pure Rust. This was genuinely attractive while web was in scope,
  because it made the platform surface three methods and put SQLite on an in-memory VFS everywhere.
  With web dropped, its entire justification evaporated: both remaining targets have real
  filesystems and a durable SQLite. What remained of its case — that the only data-losing code would
  be twenty lines of append-and-`fsync` — is real but not worth a second artefact and a hand-rolled
  durability layer.
- **SQLite in a dedicated Worker on OPFS**, the conventional web answer. Moot once web was dropped,
  but recorded because it was the specific cost that provoked the scoping question: the Worker
  boundary is *contagious*, making every storage call async-and-fallible on one target and
  synchronous on the others — which is exactly the shape difference ADR-0003 congratulated this
  stack for not having.
- **redb.** Pure Rust, ACID, crash-safe, stable file format, tiny dependency tree. Its one advantage
  here is needing no C toolchain — and #7 already proved that toolchain works, in 27 seconds. So the
  advantage is worth approximately nothing, while the cost is real: per-card lookup, the cache and
  the mutable store all become hand-maintained key ranges instead of an index and two tables.
- **fjall.** The same trade as redb, plus weaker maintenance signals (33 open issues, three yanked
  releases) and an Android database-open failure that is only fixed at Rust ≥ 1.98.
- **sled.** Self-declared beta with a promised breaking on-disk format change. Not a candidate for
  the only copy of a user's data.

### 3. Two files: one authoritative, one disposable

`collection.db` holds the log, the mutable store and local device state. `derived.db` holds the
cache. The app opens the first and `ATTACH`es the second onto the same connection.

Four independent arguments landed on the same boundary, which is the main reason to trust it:

1. **Write amplification.** The log is append-only; the cache is update-heavy, since every review
   rewrites that card's cached state. One file means the churny workload keeps rewriting pages in
   the file holding the one thing that cannot be regenerated. Two files mean the irreplaceable file
   sees appends and almost nothing else.
2. **Reset is a file delete**, not a `DELETE FROM` sweep followed by a `VACUUM` to reclaim space.
3. **Atomicity wants the same line** (§5): the log row, its sequence number and the mutable store
   must commit together, and the cache must commit with nothing.
4. **Durability wants it too** (§7): the two files take opposite `synchronous` settings, which is
   impossible within one file.

**No cross-file transaction is needed, and this is what makes the split safe.** The two files can
only disagree in one dangerous direction — a cache *ahead* of the log would hold state derived from
a review that does not exist, while a cache *behind* the log is merely work not yet done. So the
rule is **log first, cache second**, and the cache carries its own high-water mark: the
`(writer, sequence)` it has consumed through. A cache that is behind is caught up by replaying
forward; a cache that cannot prove where it is up to is discarded. This matters because SQLite does
not offer atomic commit across attached databases in WAL mode anyway.

Alongside the high-water mark, `derived.db` carries a **derivation version** — our replay code plus
the exact pinned `fsrs` version. A mismatch deletes the file. That is ADR-0004 §9's *"there is no
projection versioning… throw the cache away and rebuild"* made concrete: an application update that
changes the arithmetic migrates nothing, it deletes a file.

**Accepted cost**: a rebuild after a wipe is O(whole log) and lands on launch. It cannot be
amortised away, because *"due today"* needs every card's due date, so laziness does not help the
first launch. Accepted as rare-and-measured rather than designed around.

### 4. The mutable store is one attribute table with the stamp on the row

ADR-0004 §7 states **one rule for the entire mutable surface** — every independently editable thing
settles on its own, ordered by a `(counter, writer id)` stamp, never by a clock. The store is shaped
so that rule has exactly one implementation:

```sql
CREATE TABLE mutable (
    entity    TEXT    NOT NULL,      -- 'note' | 'deck' | 'writer'
    entity_id BLOB    NOT NULL,
    attr      TEXT    NOT NULL,      -- 'front' | 'kind' | 'deck' | 'deleted' | 'tag:physics' | 'label'
    value     TEXT,                  -- JSON scalar
    counter   INTEGER NOT NULL,      -- the stamp
    writer    BLOB    NOT NULL,      -- stamp tiebreak
    PRIMARY KEY (entity, entity_id, attr)
) WITHOUT ROWID;
```

Settling is: compare `(counter, writer)`, keep the higher. Five lines, one code path, used by every
value in the application.

**The argument is agent-legibility in the specific sense the map fixes as non-negotiable.** Typed
tables with per-column stamps are more idiomatic SQL, but they give the settling logic a case per
column, and the failure mode is an agent adding a twelfth field and forgetting its stamp comparison
— so that one value silently stops settling and quietly loses edits. That is the class of defect the
map says the spec must be free of, and this shape makes it unreachable.

Three things follow:

- **Removal is a value change, not a row deletion** — exactly what §7 demanded when it answered
  ADR-0002 §10's open question about tag removal. A removed tag is `tag:physics = false` at a higher
  stamp. Nothing here is ever deleted, so nothing can resurrect.
- **Adding an attribute is not a migration.** A new note kind with new field names, a new per-deck
  preference (ADR-0005's open slot), a new device attribute — none touch the schema.
- **Sync is not foreclosed.** ADR-0004 leaves *"snapshot or change stream"* to the sync work; this
  shape makes a snapshot the table and a change stream `WHERE counter > N`.

**Accepted cost**: every read is a pivot, and "all notes in deck X" cannot use a natural index. The
queryable shape is therefore typed tables in `derived.db`, rebuilt from these rows by the same
disposable machinery §3 already requires.

The per-kind layout data of ADR-0002 §4 is **read-only, code-defined, and not part of this table** —
it carries no stamp because nothing edits it.

### 5. Device identity lives in `local`, and the high-water mark is the self-heal detector

```sql
CREATE TABLE local (key TEXT PRIMARY KEY, value TEXT) WITHOUT ROWID;
--  writer_id      this install's writer id (ADR-0004 §2)
--  seq_highwater  the highest sequence THIS install has written
--  lamport        the §7 stamp counter, jumps above any counter it sees
```

**The next sequence number comes from `seq_highwater`, never from the log.** Deriving it as
`MAX(seq) WHERE writer = me` looks strictly better — gap-freeness would become structural — and it
is a trap. After a merge the log may contain rows bearing our own writer id that we did not write,
and `MAX + 1` would cheerfully continue *someone else's* numbering: precisely the duplicate-writer
failure ADR-0004 §2 and §3 exist to prevent, arriving through the back door of a convenience.

Inverted, the same comparison becomes the guard:

> The stored high-water is authoritative for *what I wrote*. If the log contains our writer id at a
> sequence **above** our high-water, someone else is writing as us — mint a new writer id
> immediately.

That is §2's **self-heal** made mechanical, and it has no other implementation: self-heal requires
knowing "I did not write that", and only locally-held state knows it. The high-water is therefore
not a cached convenience but the detector, and is as durable as the log.

**The sequence bump commits in the same transaction as the row it numbers.** A crash between them
would either skip a number — and §2 makes gap-freeness load-bearing, because a gap breaks the
ahead/behind arithmetic *silently* — or reuse one, which is worse. One transaction makes both
unreachable. This is also why `local` lives in `collection.db` rather than the cache.

The Lamport counter is here for the mirror-image reason: if it were lost or reset, this device's
edits would lose every stamp contest until real activity carried it back above its peers, which is
the "my edit keeps silently losing with no explanation" failure §7 rejected wall clocks to avoid.

### 6. Where the files live, and surviving Android Auto Backup

| Target | Location |
|---|---|
| Desktop | `$XDG_DATA_HOME/leitner/` (falling back to `~/.local/share/leitner/`) |
| Android | `getFilesDir()`, reached by the hand-written JNI proven in the #8 slice |

Android exposes no data directory to this stack, so the ~29 lines of `ndk_context` + `jni` calling
`getFilesDir().getAbsolutePath()` carry forward as validated code, per ADR-0003 §5.

What survives on Android: an **app update**, yes; an **uninstall**, no; a user "clear data", no.

**And then there is backup, which silently manufactures ADR-0004 §2's duplicate-writer failure.**
Verified against the platform documentation: `allowBackup` **defaults to `true`**; Auto Backup
includes *"files saved to app's internal storage accessed by `getFilesDir()`"* and *"database files
returned by `getDatabasePath()`"*; and *"data is restored whenever the app is installed… during
device setup… after the APK is installed but before the app is available to be launched."*

So replacing a phone restores the collection **including `local.writer_id`**, and two devices are
now the same writer. Both emit sequence 501 for different reviews; the union drops one. Because sync
does not exist yet, self-heal cannot fire until they eventually meet — by which time divergent rows
already collide and the loss is silent. This is the exact scenario §2 and §3 were built against,
arriving through a platform default nobody opted into.

**The fix: keep Auto Backup on, and make restore safe with a writer-identity marker held outside the
backup set.**

- A copy of the writer id is written to a marker in a location that is deliberately *not* portable:
  `getNoBackupFilesDir()` on Android — excluded from backup by default, requiring no configuration —
  and `$XDG_STATE_HOME` on desktop, which is XDG's slot for state that persists but is not the
  user's portable data.
- On launch: **marker absent, or disagreeing with the database → this collection arrived from
  somewhere else. Mint a fresh writer id, reset `seq_highwater` to zero, rewrite the marker.** The
  Lamport counter is *not* reset; it must stay above everything already in the store.

This turns a restore into a clean fork. The restored device keeps the old writer's rows correctly
attributed to that writer and writes its own under a new id, so the two devices diverge under
*different* ids and union merges them losslessly. It is §3's rule — never adopt, always mint —
enforced at the one moment the platform tries to violate it.

It also generalises past Android for free: the desktop version of this hazard is a collection in a
cloud-synced folder or copied to a second machine, and the marker catches those identically. Its
failure mode is benign in the safe direction — a lost marker mints a writer id we did not need,
costing about 24 bytes in the version summary, which §2 already budgets for.

**Two constraints recorded here because they are easy to trip over later:**

- **`getNoBackupFilesDir()` is the *only* exclusion mechanism available to us.** XML backup rules
  require `@xml/…` under `res/`, and ADR-0003 §2's no-Gradle-project property rests on the APK being
  a manifest plus a `.so`, with no Android resources at all. The no-backup directory needs no XML.
- **Auto Backup is not our backup story, and stops working without telling anyone.** The quota is
  25 MB per app: *"if the amount of data is over 25 MB, the system calls `onQuotaExceeded()` and
  doesn't back up data to the cloud."* At §10's projections we cross that in about two years of
  heavy use. This is a fact for the map's **Backup and restore** fog, not something this ADR solves.

### 7. Durability: WAL on both files, `FULL` on the collection, `OFF` on the cache

```
PRAGMA journal_mode  = WAL;
PRAGMA main.synchronous    = FULL;   -- collection.db
PRAGMA derived.synchronous = OFF;    -- derived.db
PRAGMA busy_timeout  = 5000;
PRAGMA application_id = <constant>;
```

For the authoritative file, WAL with `FULL` is both the safer *and* the cheaper option, which is
unusual enough to spell out: a rollback-journal commit syncs twice — journal, then database —
whereas a WAL commit at `FULL` syncs once, appending sequentially. Durability per commit for half
the syncs, on a workload that is overwhelmingly appends. On a phone that is milliseconds against a
review that takes seconds.

`synchronous=NORMAL` was considered and rejected. It is *permitted* by this ticket's own criterion —
a review lost to a crash is a small harm, a corrupted log is a large one, and WAL at `NORMAL` cannot
corrupt, only lose recent commits — but it buys nothing we need at a cost we can trivially pay.

WAL also handles the Android reality that backgrounded apps are killed routinely: the `-wal` is left
behind and the next open recovers it, with no special-case code.

**The cache takes the opposite setting, which is the second dividend of splitting the files.**
`derived.db` is disposable by construction, so syncing it is pure waste — every cached state lost is
one that can be recomputed.

**One transaction per review**, carrying the log row, its derived columns and the `seq_highwater`
bump together.

**Caveat, not solved here**: with WAL, `collection.db` alone is not a complete copy, since recent
commits live in the `-wal` until checkpoint. Exports are unaffected — #13 exports interchange lines,
not files — but a user hand-copying the file would silently lose recent reviews. The correct way to
produce a single-file copy is `VACUUM INTO`, and that belongs to the **Backup and restore** fog.

### 8. Concurrency: `BEGIN IMMEDIATE`, and two different inserts

Two processes may open the same collection — a second desktop instance, most obviously. The store is
made safe by construction rather than by preventing it:

- **Every write transaction is `BEGIN IMMEDIATE`.** Sequence allocation is a read-modify-write on
  `seq_highwater`; under a deferred transaction two processes can both read 500 and both write row
  501. Taking the write lock up front serialises the whole read-modify-write, making the lost update
  unreachable rather than unlikely.
- **Our own writes use plain `INSERT`; only merge-ingest uses `INSERT OR IGNORE`.** The distinction
  carries more weight than it appears to. `INSERT OR IGNORE` is the correct expression of §2's union
  merge when absorbing another device's rows. Applied to our own writes it would convert a sequence
  collision into a **silently dropped review**. Same statement, opposite meaning, depending on
  provenance — and precisely the kind of thing that gets unified for tidiness later.
- **`busy_timeout`** so a second writer waits rather than failing.

**No single-instance lock.** It would add a failure mode of its own — stale locks after a crash — to
prevent something already prevented. Whether the *application* should allow a second window is a UX
question (two windows grading the same card), not a storage one, and is left to the UI work.

**A hazard from the research that does not apply**: the Android `File::lock` failure, unsupported
before Rust 1.98 and the reason a pure-Rust engine fails at database-open time on Android, is a
`std::fs` matter. SQLite takes POSIX `fcntl` locks through its own VFS. It was a real argument
against the key-value engines and is not one against this design.

### 9. The log has no version; the schema has two mechanisms

**The interchange form is never versioned as a whole and never migrated.** A version number presumes
the log was written by one thing at one time; ours is written by several devices on several
application versions and merged by union, so a single log routinely contains rows written by builds
that never met. There is no coherent value to put in such a field.

ADR-0004 §1 already put versioning where it belongs — **on the row, as its kind**, with *skip kinds
you do not recognise* as the rule that keeps an old build working against a newer one's rows. So:

- **Evolution is additive only.** New fields on an existing kind are ignored by old builds and
  preserved byte for byte by them, which §11's no-re-encoding rule makes structural rather than
  disciplined.
- **A genuinely incompatible change introduces a new kind**, rather than reinterpreting an old one.
- **The log is never migrated**, because migration means rewriting rows, which §11 forbids.

The database schema is a different question, splitting three ways:

| | Migration story |
|---|---|
| `log` | Only `line` is irreplaceable; every other column is `DROP` and re-derive by re-parsing lines |
| `mutable`, `local` | The only authoritative-and-underivable tables — and §4 made them attribute-agnostic, so the domain can grow without touching their shape |
| `derived.db` | Never migrated. Derivation-version mismatch deletes the file |

So `PRAGMA user_version` on `collection.db` covers the rare real schema change, and the derivation
version in `derived.db` covers everything else. That the migration-prone surface is approximately
empty is a payoff of §4, not an accident.

**Downgrade refuses rather than guesses.** A build meeting a `user_version` higher than it knows
declines to open the collection, with a clear message. The *rows* are forward-compatible by §11; the
schema is not, and a write from a build that misunderstands the schema is unrecoverable in a way
skipping a row never is.

### 10. Size, and why ADR-0004 §10's never-compact answer stands

§10 projected the interchange form. On disk we also store the derived columns and their B-tree, so a
realistic per-row cost is **400–450 bytes** against ~150 raw — roughly a factor of three, recorded
here so the raw figure is not quietly inherited:

| | Heavy use (200 reviews/day) | Typical (~50/day) |
|---|---|---|
| Raw interchange (§10) | 11 MB/yr, 110 MB/decade | 3 MB/yr, 27 MB/decade |
| **In `collection.db`** | **~33 MB/yr, ~330 MB/decade** | **~8 MB/yr, ~80 MB/decade** |

330 MB for a decade of heavy use, on a phone, is not a problem, so §10's reasoning survives intact
along with all of its arguments against trimming: the optimiser trains on full histories, a trimmed
log cannot merge with an old fork, and garbage collection would need a horizon measured in how long
a device might sit switched off.

The encodings in §2 — 16-byte id BLOBs, integer instants, a minimal derived column set — cut most of
the multiplier, and ADR-0004 §11 explicitly permits them: local storage *"may be anything (database
columns, for instance) provided it round-trips exactly"*.

**§10's optional local discard below the cutoff is specified but not built.** The schema makes it
trivial — delete rows below the cutoff, then `VACUUM` — but it irreversibly destroys data, is off by
default, and solves a problem a decade of heavy use does not produce. The door stays open.

**Encryption is untouched.** The map holds *local encryption / device passcode* as fog, and nothing
here forecloses it.

## Requirements this places on downstream tickets

### [#13 — the deck export format](https://github.com/amin-bf/leitner/issues/13)

1. A progress export is **`SELECT line FROM log`** — the bytes as received, never re-encoded.
2. Content export reads the `mutable` table; §4's stamps travel with it or are reset on import,
   which #13 still decides.
3. **Export is not a file copy.** WAL means `collection.db` alone omits recent commits (§7).

### [#14 — crate and workspace layout](https://github.com/amin-bf/leitner/issues/14)

1. The platform surface is now small and concrete: two directory lookups (data and state) and the
   Android JNI shim. Everything else is portable.
2. The seam stays a compile-time `#[cfg]` per ADR-0003; the web arm disappears entirely.

### [#20 — FSRS optimisation in-client](https://github.com/amin-bf/leitner/issues/20)

1. **The wasm half is out of scope** (§1). Only the Android proof remains live.

### [#21 — new-card rate and daily limits](https://github.com/amin-bf/leitner/issues/21)

1. Anything counted per day reads the cache, which is disposable — a daily counter must be derivable
   from the log, never stored only in `derived.db`.
   > **Discharged by [ADR-0011 §5](0011-new-card-rate-and-daily-limits.md).** The only daily counter
   > is *"cards whose earliest `reviewed` row falls in the device's local day"* — a query over
   > replayed history, stored nowhere, so the cache may hold it freely and losing the cache loses
   > nothing. This warning has now been honoured twice, ADR-0010's leech rule being the other.

### [#26 — leeches](https://github.com/amin-bf/leitner/issues/26)

1. ~~Suspension as a fourth row kind needs no schema change: `kind` is a column and unknown kinds are
   skipped (§9).~~
   > **Amended by [ADR-0010 §5](0010-leeches.md).** There is no fourth row kind. Suspension is a
   > value on ADR-0004 §7's mutable surface — which §7 here made **one attribute table with the
   > stamp on the row**, so it needs no schema change *there* either. The "no schema change"
   > observation was true of both homes and therefore never discriminated between them; the log was
   > ruled out on wall-clock settling instead.

## Glossary

**Moved.** These terms are now of record in [`store`](../../crates/store/src/CONTEXT.md), per
[ADR-0009 §6](0009-crate-and-workspace-layout.md), which fixed where contexts live. They
were marked provisional here precisely so this could happen: the `CONTEXT.md` is
authoritative, and this ADR keeps the reasoning behind them.

**Derivation version** and **cache high-water** moved to [`replay`](../../crates/core/src/replay/CONTEXT.md), which owns the arithmetic they version.

## Consequences

- **The application is desktop and Android.** A third target returns only with a server, as a fresh
  effort.
- **Nothing in the store can lose a review except a failure of SQLite itself.** Every other artefact
  is derived, and derived means deletable.
- **A restored or copied collection always forks its writer id**, so device replacement is safe
  before sync exists — at the cost of one extra writer id per restore.
- **The migration surface is approximately empty**: the log is never migrated, the cache is deleted,
  and the two authoritative tables are attribute-agnostic.
- **A cache wipe costs a full replay on next launch**, and that cost grows with history. Rare, but
  it will be felt on a phone one day.
- **Auto Backup will silently stop** once the collection passes 25 MB, which the **Backup and
  restore** fog now has to answer.
- **Two files means two things to keep together.** Losing `derived.db` is free; losing
  `collection.db` is everything.

## Open items handed onward

| Item | Owner |
|---|---|
| Whether a collection has an identity of its own, so a device can tell a *different* collection from a divergent copy | Sync transport; map fog |
| A real backup and restore story, given Auto Backup's 25 MB silent cutoff and WAL's `VACUUM INTO` caveat | Map fog: **Backup and restore** |
| Encryption of the store at rest | Map fog: **Local encryption / device passcode** |
| Whether the application permits a second window on one collection | UI work; the store is safe either way |
| Building §10's optional discard below the cutoff | Deferred until someone needs the space |
