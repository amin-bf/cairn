# ADR-0008: The deck export format

- **Status**: Accepted
- **Date**: 2026-07-30
- **Resolves**: [Decide: the deck export format](https://github.com/amin-bf/leitner/issues/13)
- **Map**: [Map: local-first Leitner app spec](https://github.com/amin-bf/leitner/issues/1)
- **Related**: [ADR-0002: The card model](0002-the-card-model.md),
  [ADR-0004: The review event log](0004-the-review-event-log.md),
  [ADR-0005: The deck model](0005-the-deck-model.md),
  [ADR-0007: The local store](0007-the-local-store.md)
- **Amends**: [ADR-0002 §4](0002-the-card-model.md), [ADR-0004 §7](0004-the-review-event-log.md),
  [ADR-0005 §5](0005-the-deck-model.md) — see [Amendments to earlier ADRs](#amendments-to-earlier-adrs)

## Context

Standing constraint 2 requires decks to be **portable and publishable**: stable identity, a
self-contained export, and card content cleanly separable from personal review progress. Four ADRs
have handed this one work by name:

- **ADR-0002 §4 and §9** — the container must carry the kind definitions its notes use, so a deck file
  explains itself; and it must be **capable of carrying binary files from the first version**, even
  though it carries none today, so adding audio later fills an open slot instead of invalidating every
  deck ever exported.
- **ADR-0004 §11** — a progress export is log rows in the interchange form, and **writer ids are a
  device fingerprint**, so exporting progress needs scrubbing or a warning. Whether the mutable
  surface's stamps travel or reset on import was left here.
- **ADR-0005 §4, §5 and §9** — a deck **content revision** so an import can refuse to go backwards;
  author, description and licence as things the *file* declares about itself; and the standing rule
  that **import is policy, not merge**.
- **ADR-0007** — a progress export is `SELECT line FROM log`, never re-encoded, and **export is not a
  file copy**, because WAL leaves recent commits outside `collection.db`.

Since those were written, [Decide: backup and restore](https://github.com/amin-bf/leitner/issues/37)
was graduated from the map's fog and states in its own body that an explicit export's *container is
this ticket's*. That draws the scope boundary this ADR opens with.

## Decision

### 1. One container, two profiles; only the deck profile is specified here

The container defined below carries a **declared profile**. This ADR specifies the **deck** profile in
full. A **progress** profile — ADR-0004 §11 interchange lines — is *reserved*: the container admits it,
and [#37](https://github.com/amin-bf/leitner/issues/37) decides whether and how a whole-collection
artifact is offered.

> **Amended by [ADR-0016 §2](0016-backup-and-restore.md): the reserved profile is specified as
> `collection`, not `progress`.** It carries content *and* the log — unfiled notes, the whole mutable
> surface with its stamps intact, per-deck revisions and suspensions, none of which the deck profile
> exports — so the reserved name understates the payload by half. That matters beyond naming: the
> selection rule is *everything that settles, plus the log, minus device identity and credentials*,
> and a profile named for half its contents is how the wrong rule gets implemented. The extension is
> `.lcoll` (ADR-0016 §9), discharging §10's handoff.

Rejected: **one artifact with progress as an inclusion choice** ("export deck ☑ include my progress").
Not on privacy grounds, though those apply, but because the option is **not expressible**:

> **A per-deck progress export does not exist.** The log has no deck column, deliberately — ADR-0004 §5
> declined to put one on a review row — and ADR-0005 §6 makes per-deck due counts display-only,
> following where a card lives *now*. Scoping progress to a deck therefore means filtering by *current*
> membership, so the same log exported twice a week apart, with one note moved between decks, yields
> different "progress for the same deck". Progress is coherent collection-wide and incoherent
> deck-wide.

ADR-0004 §11's writer-id disclosure disappears as a **side effect** of that, rather than needing a
policy to contain it: the deck profile carries no log, so it carries no writer ids.

Also rejected: **letting a backup artifact invent its own container later**. Two containers for one
application is two parsers, two versioning stories and two sets of the path-traversal rules in §6.

### 2. The container is a zip archive

A deck file is a **zip archive** whose payload members are text.

This repo has twice traded bytes for inspectability, and for the same stated reason each time:
ADR-0002 §8 kept note fields as plain strings so *"a broken note can be repaired in a text editor"*,
and ADR-0004 §11 chose JSON lines over packed binary because *"a format that can be inspected and
repaired by hand is worth real bytes"*. A deck file is the one artifact that **leaves the machine and
arrives with someone who does not have our application**. If inspectability is ever worth paying for,
it is here.

Zip is the only candidate that is simultaneously an archive with a **directory readable without
inflating the payload** (§6 depends on this) and a format every desktop operating system opens by
double-click.

**Dependency, verified rather than assumed** — `zip` 8.6.0 with
`--no-default-features --features deflate-flate2-zlib-rs` builds from **nine transitive crates with no
`-sys` crate among them**, so deflate needs no C toolchain. Recorded because the obvious-looking
feature name is the broken one:

> `--features deflate-flate2` **does not compile**: `error: You need to choose a zlib backend` /
> `No compression backend selected; enable one of 'zlib', 'zlib-ng', 'zlib-rs', or the default
> 'rust_backend' feature.` The `deflate` umbrella feature does build, but additionally pulls zopfli and
> three more crates for a denser, far slower encoder we do not need.

Rejected alternatives:

- **tar + zstd.** Its one advantage is solid compression across members, and it does not apply. The
  12.01× measured in [`docs/research/sync-transport/`](../research/sync-transport/README.md) came from
  repeated writer identifiers across a decade of *log rows*; deck content is a single notes member, so
  the compressor window spans it inside one zip entry anyway. Audio, the binary this container exists
  to admit, does not compress at all. Against that: stream-only, so §6's read-the-manifest-first
  property is unavailable, and `zstd-sys` is a C dependency.
- **A SQLite file.** Tempting, because ADR-0007 already ships `rusqlite` with `bundled`, so it costs
  no new dependency. Rejected because a recipient cannot look inside one without a tool, and because it
  would give us a schema to migrate in the single artifact that must stay readable by old builds.
- **One JSON document with base64 blobs.** Inflates binary by a third and puts megabytes of audio
  inside a text field — which is ADR-0002 §9's requirement met in name only.

**Accepted cost, stated because §1 reserves a progress profile:** zip's deflate has the same 32 KiB
window as gzip, so log rows in a progress member would compress roughly **4×** rather than zstd's
**11.76×** — about 27 MB per decade against 15 MB. This is recoverable without changing the container,
by placing a zstd-compressed payload in a `stored` (uncompressed) zip entry. That choice belongs to
[#37](https://github.com/amin-bf/leitner/issues/37) and is not made here.

### 3. Stamps do not travel; import restamps only what changes

The ADR-0004 §7 mutable-surface stamps are **not written into the file**.

ADR-0005 §9 already made them unusable across this boundary: import is **policy, not merge**, because
§7's counters are per-collection and *"a higher number carries no meaning"* between two of them. A
stamp in the file is therefore a number no reader may lawfully consult — which is worse than dead
weight, because it is a standing invitation to the exact comparison ADR-0005 §9 spends a paragraph
forbidding.

**On import, values are written with fresh local stamps**, by ADR-0004 §7's existing rule — on seeing
any counter greater than your own, jump above it. No new mechanism, and locally well-defined in a way
an imported stamp never was.

**Import restamps only values whose content actually differs.** Restamping everything would make
importing a 5,000-note deck write 5,000 stamped values onto the mutable surface, which then propagate
to the user's own devices as a wall of edits — and re-importing an unchanged file would do it again.
With this rule, **re-importing the same file is a genuine no-op**: silent, idempotent, and producing
nothing to sync.

**Import is not restore, and the distinction is the stamp rule.** An import crosses a collection
boundary, so stamps must reset. A restore re-enters *the same* collection, so its stamps must travel
**byte for byte** — otherwise a restored device's fresh counters could outrank genuinely later edits
still held on another device. Same container, opposite rule, decided by whether a collection boundary
is crossed. Handed to [#37](https://github.com/amin-bf/leitner/issues/37), and it is a live argument
that the map's **collection identity** fog is load-bearing: telling the two operations apart requires
knowing which collection a payload came from.

### 4. The revision is a plain integer; the digest detects change

Each deck the file carries declares a **revision**: a monotonically increasing integer, and a
**content digest** over that deck's payload.

- **Not a timestamp.** ADR-0004 §7 removed wall clocks from every "which of these is later?" decision,
  and ADR-0004 §8 records why. A file claiming to be newer because the authoring machine's clock runs
  fast would be that same defect in a new place.
- **Not a bare content hash.** A hash answers *same or different*, never *older or newer*, and
  ADR-0005 §4's requirement is specifically to **refuse to go backwards**, which needs an order.
- **A plain counter is legal here although a §7 stamp is not**, and the distinction is worth stating
  because it reads at first like a contradiction with §3. A per-value stamp is compared *across*
  collections, where counters are meaningless. A deck revision is only ever compared *within one deck
  id's lineage*, and a deck id belongs to exactly one collection of origin — ADR-0005 §4 mints a new
  deck id when a modified copy is published. Same kind of number; one crosses the boundary that voids
  it, the other does not.
- **It carries no writer id**, unlike a §7 stamp. That is precisely the device fingerprint ADR-0004 §11
  flags, and a published deck goes to strangers.

**Import refuses a strictly lower revision and accepts an equal one.** Accepting equal is required, not
conceded: §3's idempotent re-import depends on it, and an equal-revision import falls through to
ADR-0005 §9's file-wins policy having changed nothing.

**Accepted cost of dropping the writer id.** A §7 stamp would have tie-broken deterministically; a bare
counter cannot. An author who exports the same deck from two of their own devices while those devices
are offline from each other can emit two different files both claiming revision 4. The digest makes
this **reportable rather than silent** — same revision, different bytes — and the consequence needs no
new rule, because equal revisions are already allowed and resolve by file-wins.

### 5. Note deletions travel; deck deletion never does

**A retracted note travels as a tombstone**: its id, marked deleted, and nothing else. There is no
content left to carry — ADR-0004 §7 already reduced a deleted note to a marker.

Without this, a factual error in a published deck is **unretractable by construction**: ADR-0005 §9
holds that *"notes the file does not mention are untouched"*, so omitting a bad note leaves it in every
recipient's collection forever.

Carrying it is consistent with ADR-0005 §9's own principle. A deck's composition is *"the author's
statement about the material"*, with *"no competing user intent to protect"* — and whether a note
exists at all is that same statement, not a filing choice the recipient made. The blast radius is
small and recoverable: the recipient loses the author's **text**, their review log is untouched
structurally, and ADR-0002 §7's reattachment restores the history if the note ever returns.

**A deck's own `deleted` flag never travels.** The asymmetry is deliberate and rests on blast radius
plus existing precedent, not on a taste for symmetry: a deck tombstone would let a file the recipient
merely opened discard the content of every note in it, and ADR-0005 §9 already refuses this exact shape
— *"a deck left empty by an upstream split is left alone and surfaced, never auto-deleted."* An author
retiring a deck publishes an update that empties it, and §9 already says what happens next.

**Import reports the tombstone count it applied.** A destructive update is never silent.

### 6. Layout

```
mimetype               stored   — first member, uncompressed (§10)
manifest.json          deflate  — readable alone, from the central directory
notes.jsonl            deflate  — one note or tombstone per line
kinds/<kind-id>.json   deflate  — one member per kind the notes use
media/                 stored   — reserved prefix, empty today
```

`manifest.json` declares the format version (§7), the profile (§1), file-level metadata (§12), the
decks carried — each with its **own** revision and digest (§11) — the kind ids used, and **counts of
notes and tombstones**.

**JSON Lines for notes, not one JSON array.** Straight from ADR-0004 §11's framing precedent: line
framing makes a malformed line skippable rather than fatal, a large deck streams instead of parsing
whole, and a deck under version control diffs one line per note. A tombstone is a line bearing an id
and a deleted marker with no fields.

**The manifest is what earns the container choice.** Deck names, counts and required kinds are readable
from the zip central directory **without inflating the payload**, so the application can show *"3 decks,
1,240 notes, 12 retractions, requires kinds: vocab, cloze"* before committing to an import.

**One member per kind, not one combined file.** The central directory listing alone then answers "what
does this file need in order to render?", and because ADR-0002 §4 makes kind definitions evolve under
append-only rules, a per-kind member shows exactly which definition changed.

**`media/` is a reserved prefix that is genuinely empty.** Nothing references it — ADR-0002 §8
deliberately excluded image and link syntax from the Markdown subset — so the slot is structural, which
is exactly what ADR-0002 §9 asked for. When it is used, entries are `stored`: audio is already
compressed.

**Path-traversal rules, stated because "agents implement this".** The importer accepts **only** the
member names above and the `media/` prefix. It rejects absolute paths, any `..` path segment, and
symlink entries outright. Zip path traversal is the classic defect of this container and must not be
left to an implementer's judgement.

### 7. Versioning: one hard gate, additive inside

ADR-0007 settled the log with *"the log has no version — each row does"*, so evolution is additive and
the migration-prone surface is approximately empty. The same shape applies here, with one exception the
log does not have: a file has **structure**, and structure cannot be guessed at.

1. **An unknown `format` integer → refuse, with a plain message.** The member layout is the only thing
   a reader cannot skip past safely, so it is the only hard gate. Guessing risks a silent partial
   import.
2. **Unknown keys in the manifest or on a note line → ignored, and preserved on re-export** if the note
   is otherwise untouched. This is ADR-0004 §11's rule, and it is what makes additions free.
3. **An unknown kind → imported and rendered from the file's own definition.** This is the payoff
   ADR-0002 §4 paid for: *"a reader that has never heard of `vocab` can still render the cards
   correctly, because the file explains itself."* Refusing here would waste the mechanism.

**A shipped kind definition always wins, and a file may never overwrite one.** The file's definition is
used **only** for kinds the running build does not ship. This rule is load-bearing rather than
defensive: ADR-0002 §4 calls reordering a kind's `cards` list *"the single most destructive edit
available in this codebase"* — it silently retypes every accumulated review onto the wrong card, and the
log cannot be edited to repair it. Without this rule, importing a file is a remote path to exactly that
edit, and nothing downstream can catch it.

### 8. Export takes a deck selection

An export is over **one or more selected decks** — ADR-0005 §9 requires an upstream split to propagate
and speaks of *"decks named in the file"*, so multi-deck files are a consequence of an accepted
decision rather than a new capability.

The file carries exactly the notes whose `deck` reference names a selected deck, plus the tombstones
(§5) of deleted notes that named one.

**Unfiled notes are never exported.** A note whose `deck` reference names no deck the collection holds
(ADR-0005 §8) is in no deck, so a deck-scoped export cannot reach it. **The export screen states how
many unfiled notes exist**, so they are not silently missed by a user who believes they exported
everything.

This creates a requirement on ADR-0004 §7 — see [Amendments](#amendments-to-earlier-adrs): a deleted
note must retain its `deck` reference, or §5's tombstones cannot be selected at all.

### 9. `{revision, digest}` per deck id: synced, never exported

The revision and digest of §4 are held as **one value per deck id on the ADR-0004 §7 mutable surface**.

- **Never exported.** ADR-0005 §4 was explicit that the revision is something the **file** declares, not
  part of the deck object. Were it to travel as deck content, an importer would adopt the author's
  counter into a collection where it means nothing.
- **It syncs between the user's own devices**, and there is no judgement call in this one: if it did
  not, an author exporting from a laptop and from a phone would emit conflicting revision-4 files as
  *routine behaviour* rather than as §4's rare offline edge.
- **One value serves both authoring and importing.** A deck id is normally either yours or acquired,
  but not always — deleting your own deck and re-importing your own file crosses over. A single
  "revision last emitted or seen for this deck" handles both: export stamps it, import refuses anything
  strictly lower and otherwise adopts it.

**The counter advances only when the digest changes.** Not on every export. This buys the relay case for
free: passing someone an unmodified copy of a deck emits the byte-identical file at the **same**
revision, instead of inflating the counter and creating a phantom revision that competes with the
original author's next real one.

### 10. `.ldeck`, self-identifying from its first bytes

The extension is **`.ldeck`**. The first member of the archive is **`mimetype`**, `stored` with no
compression, containing `application/vnd.leitner.deck+zip`.

The mechanism: a zip archive's own header says nothing about what the archive *contains*, so a
container built on zip is unidentifiable from its bytes unless it puts a type marker at a known
position. Storing an uncompressed `mimetype` entry **first** places the media type string at a fixed
byte offset, where content sniffing and `file(1)`-style tools can read it without parsing the archive
at all. It costs about forty bytes and means a deck file arriving with a mangled or stripped extension
is still recognisable.

This is a settled convention rather than an invention: the EPUB Open Container Format requires exactly
this of its own zip container — the `mimetype` file first, uncompressed, with no extra field
([EPUB 3.3, W3C](https://www.w3.org/TR/epub-33/)).

**Renaming a deck file to `.zip` and looking inside is intended**, not an accident to design against —
it is the inspectability §2 chose the container for.

**On Android the intent filter matches a `pathPattern` for the extension alongside the media type.**
Custom extensions have no reliable extension-to-type mapping there, and getting this wrong means the
file will not open from a file manager or a mail attachment — which is the entire distribution channel
for a deck.

A distinct extension per profile, sharing one container format, is how the operating system and the
user tell a deck file from a whole-collection artifact **before** opening it. Naming the progress
profile's extension belongs to [#37](https://github.com/amin-bf/leitner/issues/37).

> **Discharged by [ADR-0016 §9](0016-backup-and-restore.md)**: the extension is **`.lcoll`**, with
> `application/vnd.leitner.collection+zip` in a `stored` `mimetype` member first in the archive, by
> the mechanism above unchanged. ADR-0016 §5 also supplies what this ADR never had — **how a file
> reaches the user's filesystem at all**, which was unowned for `.ldeck` too.

### 11. Authority follows deck id

Read literally, ADR-0005 §2 and §9 disagree, and an implementer meeting a file that names a note held
in a different deck has two rules and no tiebreak:

- **§2**: a note whose id already exists *"is not re-imported and does not move; the import reports it."*
- **§9**: *"membership of notes the file contains follows the file."*

They describe different cases — §2 a fork, §9 an update — but neither says so. The rule that separates
them:

> **A file may reorganise notes only into a deck whose identity it already shares.**

- **The file's deck id matches one held → update path.** ADR-0005 §9 applies in full: the file wins,
  notes move into that deck if the file says so, the deck name is overwritten, tombstones apply, and
  §4's revision gate governs.
- **The file's deck id is new → create path.** ADR-0005 §2 applies in full: notes already held are
  never touched and never moved, only genuinely new ids are created, and the import **reports what it
  skipped**. A file with no established identity cannot reach into decks the user already holds and
  take notes out of them.

**The revision gate is therefore per deck, and the manifest carries revision and digest per deck**
rather than one pair per file. A single file-level revision cannot work: a deck at revision 8 exported
alongside a freshly split deck at revision 1 has no honest single number — and the split case is
exactly why multi-deck files exist, so this is not an edge.

Each deck's digest covers **that deck's own** notes, tombstones and kind definitions.

### 12. Deterministic emission, minimal disclosure

**Author, description and licence are optional, default to empty, and are never auto-populated** from
an operating-system user name, a device label, or any other ambient identity. ADR-0005 §5 hands these
to this ADR as file-level metadata; a name in a file sent to strangers is identity, not content, so it
is typed deliberately or it is absent.

**Export is byte-for-byte deterministic**: fixed member order, all member timestamps pinned to a
constant, a fixed deflate level, no extra fields, and no platform-dependent creator or attribute
variance.

> **Amended by [ADR-0016 §11](0016-backup-and-restore.md): determinism binds the `deck` profile
> only.** The reasoning above is entirely about an artifact **sent to strangers** — build time must
> not leak, and §9's "same revision, same file" must be a property rather than an approximation. The
> `collection` profile is the opposite artifact: it goes to nobody, it has no revision, and a backup
> without a date is close to useless, since a user with three archives in a folder must tell them
> apart before restoring the wrong one. It therefore **carries a creation timestamp in its manifest**
> and does not inherit this paragraph. **Minimal disclosure above still binds both profiles** — no
> author name, no device label, no ambient identity ever auto-populated.

> **Amended by [ADR-0011 §7](0011-new-card-rate-and-daily-limits.md): `notes.jsonl` lines are
> emitted in `(position, note id)` order.** Determinism above fixes the order of zip *members* but
> never said what order the *lines inside* `notes.jsonl` take — so the strongest claim in this
> section rested on an order no ADR specified. ADR-0011 adds `position` to the note (amending
> ADR-0002 §6) so that new cards are introduced in the order their author intended, and that same
> field fixes emission order here. One concept serves both, and a deck under version control now
> diffs one line per note against a stable sequence rather than an incidental one. Zip entries otherwise carry per-member modification times and creator fields, so exporting
identical content twice would yield different bytes — leaking build time, making a deck under version
control diff as changed when nothing did, and weakening §9's "same revision, same file" from a property
to an approximation.

> **Touched but unchanged by [ADR-0021 §3](0021-note-ordering-saving-and-the-note-list.md), recorded
> so nobody re-derives it.** That ADR makes `position` an **order key with infill** rather than a plain
> integer, so that a user reordering notes writes one value instead of renumbering a run of them. **No
> byte of this format moves.** Emission stays `(position, note id)` and stays byte-for-byte
> deterministic, because the file carries **line order** rather than the value — ADR-0011 §7 already
> specified import as reading the `notes.jsonl` line index, so the representation was never in the
> container to begin with. A reader who notices `position`'s type changing and comes looking for a
> format version bump should stop here: there isn't one, and there does not need to be.

**Verification of constraint 2's content/progress split, which the ticket asked for.** Every channel is
clean: writer ids are absent because the deck profile carries no log; device labels are never exported;
scheduler configuration lives in the log, not in content; personal per-deck preferences are excluded by
ADR-0005 §5's own test. And one leak was closed **incidentally** — an ADR-0004 §7 stamp *is* a counter
plus a writer id, so §3's removal of stamps for a correctness reason removed a device fingerprint at
the same time. Recorded because a future change reintroducing stamps "to improve merging" would reopen
both at once.

## Amendments to earlier ADRs

This ADR is the first to amend accepted ones. Each amended section carries a pointer back here.

### [ADR-0004 §7](0004-the-review-event-log.md) — a deleted note keeps its `deck` reference

§7 specifies a deleted note as *"an id, a flag and a stamp, around forty bytes."* Read strictly, the
`deck` reference — a mutable value on the note per ADR-0005 §8 — is discarded with the content, and
nothing then records which deck the note was in.

**Amendment**: a deleted note retains its `deck` reference and nothing else — id, deleted flag, deck
reference, stamp. Roughly sixteen bytes above the stated figure.

**Why**: without it §5's tombstones cannot be selected by a deck-scoped export, so retraction is
impossible. No content is retained, so §7's "delete means gone" property and its privacy consequence
are untouched.

### [ADR-0002 §4](0002-the-card-model.md) — a collection may hold acquired kind definitions

§4 describes kind definitions as *"read-only data shipped with the application"*. After §7 above, a
collection may also hold definitions **acquired from an imported file**, for kinds the running build
does not ship.

**Why this is a widening and not a reopening**: ADR-0002 §2 rejected *user-authored* note types because
*"a user-editable schema is the one part of this model that provably does not merge"* — the objection
was user-editability, not provenance. An acquired definition is authored by a build of this
application, remains read-only, and is trustworthy under §4's own evolution rules: a kind identifier is
never reused for a different shape, and the `cards` list is append-only. §7's shipped-definition-wins
rule ensures an acquired definition can never displace one the build already has.

### [ADR-0005 §5](0005-the-deck-model.md) — the deck-id-keyed slot holds authoring values too

§5 describes the mutable-surface slot keyed by deck id as holding **personal** preferences, and defers
to [#21](https://github.com/amin-bf/leitner/issues/21) the question of whether such a preference syncs.

**Amendment**: the slot holds two kinds of value — **personal** ones (#21's, whose syncing remains
open) and **authoring** ones such as §9's `{revision, digest}`, which **must** sync. Both are alike in
never being exported and never appearing in the review log; they differ in whether the question of
syncing is open at all.

## Requirements this places on downstream tickets

### [#37 — backup and restore](https://github.com/amin-bf/leitner/issues/37)

1. The **progress profile** is reserved but unspecified (§1). Its payload is ADR-0004 §11 interchange
   lines; the container, versioning, path rules and determinism of this ADR apply unchanged.
2. **Restore keeps stamps byte for byte** (§3), the opposite of import, because it does not cross a
   collection boundary. Distinguishing the two operations needs collection identity, which no ticket
   yet owns.
3. The **compression escape hatch** (§2): a zstd-compressed payload inside a `stored` entry recovers
   roughly 12× where deflate gives roughly 4×, without changing the container.
4. The **extension** for a whole-collection artifact (§10).

### [#21 — new-card rate and daily limits](https://github.com/amin-bf/leitner/issues/21)

1. ADR-0005 §5's deck-id-keyed slot is **not uniformly personal** (see Amendments). A daily limit is a
   personal value and its syncing remains #21's to decide; §9's revision is an authoring value and
   already syncs.

### The authoring/editing experience ([#28](https://github.com/amin-bf/leitner/issues/28))

1. The export screen **states the count of unfiled notes** (§8), so a user cannot silently omit them.
2. An import **reports** the tombstones it applied (§5) and the notes it skipped on a create path
   (§11), before or immediately after committing.
3. Author, description and licence are **typed deliberately or left empty** (§12) — never
   auto-populated.

## Glossary

**Moved.** These terms are now of record in [`export`](../../crates/export/src/CONTEXT.md), per
[ADR-0009 §6](0009-crate-and-workspace-layout.md), which fixed where contexts live. They
were marked provisional here precisely so this could happen: the `CONTEXT.md` is
authoritative, and this ADR keeps the reasoning behind them.

**Acquired kind definition** is also noted in [`content`](../../crates/core/src/content/CONTEXT.md), since it is a kind definition first and an import artefact second.

## Consequences

- A deck file is inspectable by anyone who receives it, with or without this application, and repairable
  by hand — the same trade ADR-0002 §8 and ADR-0004 §11 made, applied where the recipient is a stranger.
- Progress can never be exported per deck, so the obvious privacy hazard of a deck-sharing feature does
  not exist to be mitigated.
- Re-importing an unchanged file changes nothing and syncs nothing; relaying an unmodified deck emits
  the identical file at the identical revision.
- An old build can import a deck built by a newer one, including one using a kind it has never heard of,
  and render it correctly from the file's own definitions — but can never have its own kind definitions
  rewritten by a file.
- An author can retract a note from a published deck; an author cannot delete a recipient's deck.
- Adding audio later fills the reserved `media/` prefix without invalidating a single deck already
  exported.
- Two decks exported together carry independent revisions, so an upstream split propagates without
  either deck's history of updates being renumbered.

## Open items handed onward

| Item | Owner |
|---|---|
| Progress profile, restore-keeps-stamps, compression hatch, backup extension | [#37 — backup and restore](https://github.com/amin-bf/leitner/issues/37) |
| Whether a *personal* deck-id-keyed preference syncs | **Out of scope** — [#21](https://github.com/amin-bf/leitner/issues/21) never needed such a preference to exist (the new-card rate is global), so the question survives it unanswered and is now [ADR-0005](0005-the-deck-model.md)'s open row, inherited by whatever effort builds *per-deck new-card on/off* |
| ~~Collection identity, needed to tell import from restore~~ — **settled by [ADR-0016 §4](0016-backup-and-restore.md)**: a UUIDv4 adopted and never re-minted, which also **upgrades [ADR-0013 §10](0013-the-sync-transport.md) from a structural accident to a checked invariant** | — |
| **Export/import reporting surfaces** — what the user is shown when a deck is exported, and what an import preview says. §5 makes the manifest readable from the central directory *without inflating the payload* precisely so an import can be previewed, but no ADR says what the preview states | [Decide: what an import preview states, and what export reports back](https://github.com/amin-bf/leitner/issues/68) — **re-owned on 2026-08-01**: [#28](https://github.com/amin-bf/leitner/issues/28) was named here and closed without reaching it, which the *Open items* sweep caught |
