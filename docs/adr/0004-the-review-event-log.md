# ADR-0004: The review event log

- **Status**: Accepted
- **Date**: 2026-07-28
- **Resolves**: [Decide: the review event log format](https://github.com/amin-bf/leitner/issues/9)
- **Map**: [Map: local-first Leitner app spec](https://github.com/amin-bf/leitner/issues/1)
- **Evidence**: [`docs/research/local-first-event-log/`](../research/local-first-event-log/README.md)
- **Related**: [ADR-0001: Scheduling algorithm and grade scale](0001-scheduling-algorithm-and-grade-scale.md),
  [ADR-0002: The card model](0002-the-card-model.md)

## Context

Standing constraint 1 makes the review log the source of truth: raw grades and timestamps are
recorded, and every piece of scheduling state — memory state, due date, box — is *replayed* from
them rather than stored. ADR-0001 chose the scheduler that consumes the replay and stated what it
needs. ADR-0002 fixed what a review event points at and split content from progress. This ADR fixes
the log itself.

It is the decision the rest of the map has been waiting on, and it has to answer four things that
have been deferred to it by name:

- **What a row is**, and which facts belong in this log rather than beside it.
- **How rows are ordered across devices with no server** — standing constraint 3.
- **How mutable data settles**, handed over by ADR-0002 §7 and §10.
- **Clock skew**, which the map calls its sharpest conflict and instructed this ticket to confront
  rather than assume away.

The scoping principle throughout: this ADR owns the log's *logical* content and one canonical
interchange form. It does not own local storage
([#12](https://github.com/amin-bf/leitner/issues/12)), the export container
([#13](https://github.com/amin-bf/leitner/issues/13)), or sync transport, which remains deliberately
deferred.

## Decision

### 1. The log holds inputs to replay, and there are three row kinds

The membership test is **"is this an input to replay?"** — not "is this immutable?". The latter reads
naturally but is wrong: scheduler configuration is plainly mutable and ADR-0001 §6 nonetheless
requires it in the log, because two devices replaying the same history under different parameters
compute different memory states with no event missing and nothing to detect it.

| Kind | Written when | Replay meaning |
|---|---|---|
| `reviewed` | the user presses a grade button | accumulates — one fact per recall attempt |
| `config-set` | a setting changes | supplies the current value of that setting (§6, §7) |
| `history-cutoff-set` | the user disowns bad history | replay ignores every `reviewed` row before the given instant |

**One widening, made explicitly.** §5 records how long an answer took, which FSRS does not consume.
The rule is therefore: *the log holds inputs to replay, plus raw facts about the act of reviewing
that cannot be reconstructed later.* The second clause is bounded deliberately to **what the user
did and when** — not device state, not app version, not which deck the card was in at the time.

**No separate learning, relearning, cramming or manual-reschedule kinds.** Prior art needs six such
variants because it has cram decks and manual rescheduling; we have neither. ADR-0001 §5 requires
same-day relearning re-shows to be real logged events, and they are — ordinary `reviewed` rows whose
day gap is zero. The distinction between a first review, an inter-day review and a same-day re-show
falls straight out of replay, so authoring it into the log would be recording a conclusion instead
of the evidence.

**The kind discriminator earns its place for forward compatibility, not for filtering.** The log is
immutable and cannot be edited, so a device running an old build *will* encounter rows written by a
newer one. A discriminator plus "skip kinds you do not recognise" is the only thing that stops a
future row kind from breaking an old client's replay (§11).

**`history-cutoff-set` is the only eraser**, and it is collection-wide rather than per-card.
Per-card would be finer, but an escape hatch that is easy to reason about is worth more than one
that is precise, and a per-card version invites use as routine editing — which is the append-only
property leaking away one exception at a time.

Rejected: **a per-row undo**, meaning a row that says "row X did not count". It needs no new
addressing (§2 provides it) but it makes each row's meaning depend on other rows, and the realistic
case is covered by the cutoff. **Accepted cost**: a mis-pressed grade button is permanent. Its
influence on the card is a few percent of one stability update (§8), and the only remedy is a cutoff
that discards good history alongside bad.

**Suspension** — "stop showing me this card" — is deliberately **not** decided here. It is personal
progress so it cannot live in the note store, but it is a mutable flag rather than a fact that
happened. It belongs with [#26](https://github.com/amin-bf/leitner/issues/26), which owns the leech
policy that needs it, and it is additive: a fourth row kind costs nothing under the rule above.

> **Settled by [ADR-0010 §5](0010-leeches.md): it is *not* a fourth row kind.** The reasoning in the
> first two sentences here was correct and the last clause did not follow from it. A mutable flag
> that toggles is settled in the log by *timestamp* order (§9), which §7 exists to forbid; on the
> mutable surface it is settled by a counter carrying real causality. Suspension is a per-`CardRef`
> value on that surface, syncing but never exporting. **The three row kinds above are final.**

### 2. A row is identified by its writer and that writer's own sequence number

Without a server there is no authority handing out row numbers, so no global sequence exists to
compare. Every production local-first system converged independently on the same primitive, and we
take it:

> **Each device numbers its own rows.** A row carries a **writer id** and a **sequence number** that
> increments by exactly one per row that device writes.

**The pair is the row's identity.** There is no separate event id. Merging two logs is set union with
duplicate pairs dropped — no conflict resolution, because two rows with the same pair are the same
row.

**The gap-free requirement is load-bearing.** A device reconstructs `{writer → highest sequence}` by
scanning its own log; comparing two of those summaries answers *"what have you got that I haven't?"*
exactly, in both directions. That is the only mechanism that satisfies standing constraint 3 — a
scalar clock gives a total order but cannot distinguish *behind* from *concurrent*, and the prior art
that answers the question with a single counter can only do so because a server owns it. A gap in a
sequence breaks the arithmetic silently.

**The summary is computed, never stamped on rows.** Stamping a full version summary on every row
would double or triple the log and grow with device count; carrying the pair costs a couple of dozen
bytes and lets any summary be reconstructed by scanning. The summary belongs in the sync handshake,
which is where the libraries that pioneered this put it.

**Writer ids are random, minted locally, never derived from hardware.** Platform guidance is against
hardware identifiers, and on web they do not exist at all.

#### The duplicate-writer failure, and why both guards are kept

Restore a backup onto a *second* device and two devices believe they are the same writer. Both emit
sequence 89 for different reviews; the union drops one, and the summary reports "I have everything
through 89" so the survivor is never asked for. Silent, unwarned loss.

Two guards, both cheap, both adopted:

- **Self-heal**: a device that sees a row bearing its own writer id which it knows it did not write
  immediately mints itself a new one. This is the only self-healing behaviour found in production
  libraries.
- **A new writer id per install**, including a reinstall on the same physical device.

**Accepted cost**: writer ids accumulate — perhaps a few dozen across a lifetime. Each costs about
24 bytes in the summary, so the whole summary stays around a kilobyte, exchanged once per sync. That
is the correct thing to spend to make silent loss of reviews impossible.

### 3. Writer ids are machine-owned; device labels are human-owned

Recognising a device by name must never let a user **adopt** an existing writer id.

The failure is concrete. A laptop is writer `L`, up to sequence 500. It is reinstalled and restores a
backup that only reached `L:480`. Invited to say "yes, `L` is this laptop", the user answers
correctly — and the device resumes at 481 while another device still holds `L:481–500`. Two different
reviews now claim the same identity, the union drops one, and the summary never asks for the real
one.

The reason no user answer can be trusted here is that **a sequence number promises "I wrote exactly
rows 1..N and nobody else ever will."** Only a device that has continuously held the log can keep
that promise; recognising a name establishes nothing about it. Reusing an identifier without also
restoring the matching local state is documented as producing inconsistent replicas.

So the one identity is split in two:

| | Owner | Rules |
|---|---|---|
| **Writer id** | the machine | random, never reused, never adopted, not shown to the user |
| **Device label** | the user | free text; **several writer ids may share one label** |

A device meeting an unfamiliar writer asks what to call it; answering *"that is this same laptop,
from before I reinstalled"* files that writer id under the existing label. The log's arithmetic never
changes — the summary still holds three entries where the UI shows one device.

Labels are not replay inputs, so they are **not log rows**. They live on the mutable surface and
settle by §7. They are also what makes the deferred sync work expressible at all: *"you are behind
your laptop"* is a sentence; *"you are behind `7f3a-b21c`"* is not.

Rejected: **requiring every device to be online at sync time**, which would make adoption safe by
guaranteeing the true highest sequence is visible. It fails on three counts. Presence requires a
rendezvous point, which is a server; the realistic transport for a serverless app is a dumb store
where devices are *never* online simultaneously, and that asynchrony is the feature. A quorum that
cannot be assembled — a phone in a drawer, a device that is lost — blocks syncing indefinitely, and
declaring a device dead is its own unsolved problem. And what it purchases is a smaller summary
table: about a kilobyte, ever.

### 4. A day is 4am to 4am, stamped at write time and frozen

FSRS's only time input is `delta_t`, a whole number of **days**, so the app must convert instants
into day buckets and that requires a boundary.

**Midnight is wrong, for a concrete reason.** ADR-0001 §5 requires a failed card's same-session
re-show to be a real logged event with `delta_t = 0`. Fail at 23:58 and re-see at 00:04 and a
midnight boundary reports a full day's gap for six minutes — crediting a day of memory decay that
did not happen. **The boundary is 4am**, where nobody is reviewing.

**The day scale is collection-wide** — one timezone and one rollover hour for the whole collection,
not per device. It is carried by a `config-set` row; this widens what configuration means rather than
adding a fourth row kind, which is right, because a day boundary is a scheduler input in exactly the
sense ADR-0001 §6 established.

**Every `reviewed` row carries both the absolute instant and the day number the writing device
assigned it, and replay uses the stamped day number without ever recomputing it.**

This is the decision, and the alternative is worse than it looks. If replay recomputed day numbers
from raw instants under current settings, then nudging the rollover hour from 4am to 3am would
**silently re-bucket the entire history** — every card's stability and every due date moving, as a
side effect of a preference. Freezing the stamp means a settings change affects only reviews written
after it. **The past is not editable by a setting.**

The absolute instant is retained regardless, so nothing raw is lost and a different bucketing scheme
remains derivable — constraint 1 is honoured in full.

**Accepted cost**: a *wrong* stamp is frozen too. Reviewing on a device whose timezone or clock is
wrong writes wrong day numbers permanently. §8 confronts this.

**"Due today" and daily limits are a different question and use the device's *local* day**, not the
collection day. `delta_t` is a difference, so it needs only a consistent scale; "today" needs to
match the day the user is living in. Binding on
[#21](https://github.com/amin-bf/leitner/issues/21).

### 5. What a review row carries

```
kind        reviewed
writer      7f3a…                 §2
sequence    412                   §2
card        note abc, ordinal 0   ADR-0002 §6 — CardRef, no standalone card id exists
grade       3                     ADR-0001 §2 — raw 1–4, exactly as pressed
instant     2026-03-01T09:14:22.418Z   UTC, millisecond precision
day         20514                 §4 — stamped, frozen
duration    4200 ms               how long the answer took
```

**Duration is recorded although FSRS does not consume it.** ADR-0001 §2 argued for keeping all four
grades even behind a UI that hides some, because *a grade never recorded cannot be recovered*.
Duration is the same class of fact: nobody can reconstruct next year how long a card was sat on
tonight. It has near-term consumers — time-studied statistics, and a leech signal for
[#26](https://github.com/amin-bf/leitner/issues/26), since a card always answered correctly after
twenty seconds of grinding is a bad card. It costs a handful of bytes.

**Deliberately absent:**

- **The card's memory state or scheduled interval before the review.** The most mature prior art
  stores these; for us they are recomputable, and storing them is precisely what constraint 1
  forbids. Considered and rejected as a *witness* — a way to detect the invisible failure ADR-0001 §6
  names, where two devices replay the same log and disagree. Rejected because it roughly doubles the
  row, because the disagreement it detects is caused by configuration drift which §6 has already
  closed off, and because the residual cause (differing app versions) leaves no permanent trace and
  heals on its own (§9).
- **Which deck the card was in.** Deck membership is mutable content
  ([#10](https://github.com/amin-bf/leitner/issues/10)); per-deck statistics should follow where a
  card lives *now*.
- **Flags such as "this was a relearning re-show" or "this was the first ever review".** Both fall out
  of replay (§1).

### 6. Configuration rows

Configuration is carried by `config-set` rows, and settles **per setting** rather than as one blob.

The case that forces it: the optimiser runs on a laptop while, on a phone, the rollover hour changes
from 4am to 3am — both offline. Under whole-blob last-write-wins one of those silently reverts the
other, and nothing reports it. Per-setting settling means concurrent changes to *different* settings
both survive. This is the same principle ADR-0002 §7 reached for note fields.

The settings, and their groupings:

| Setting | Contents | Why grouped this way |
|---|---|---|
| **Scheduler parameters** | 21-weight vector **+ algorithm identity** (`fsrs-6` and the exact pinned crate version) **+ fitted-over count** | Twenty-one numbers are meaningless without knowing which formulas consume them (ADR-0001 §6). Split them and a valid-looking vector can be read by the wrong version |
| **Day scale** | timezone **+** rollover hour | Together they define one boundary; neither is meaningful alone |
| **Desired retention** | 0.9 | Fixed and not user-exposed (ADR-0001 §6), but **recorded explicitly**, so exposing it later is a value change rather than a format change |

> **Amended by [ADR-0014 §6](0014-when-parameter-optimisation-runs.md): the scheduler-parameters
> setting gains a third member, the fitted-over count** — how many reviews the vector was trained
> on, **frozen at write time** like §4's day bucketing. It is not derivable: a device that trained
> while behind on sync fitted over fewer reviews than a later scan of the *merged* log around that
> row would count, so derivation reports a fit that never happened, exactly where the number
> matters. The setting still settles as one unit, and arbitration is unchanged — values settle by
> stamp (§7), never by which was fitted over more history.

**The log carries only changes.** Starting values are the published defaults built into the
application, so a fresh collection needs no `config-set` rows at all.

**An application update writes nothing to the log.** The algorithm identity travels with the weights,
so it is recorded when the user *optimises* — where it is genuinely informative, stating which
version fitted those numbers. Were an update to write a row, two devices updating to different
versions would fight over it indefinitely.

### 7. One rule for everything that settles

> **Every independently editable thing settles on its own.**

That single rule covers the whole mutable surface, and the special cases disappear:

- **Note fields** — per field, as ADR-0002 §7 recommended, so editing the front on one device and the
  back on another loses neither.
- **Tags** — each tag settles on its own. ADR-0002 §10 asked for set union and this delivers it for
  free, because two different tags are two different things and never compete. It also answers the
  removal question §10 left open: removing a tag is a value change on that one thing, not a deletion
  that set union has no way to express.
- **Device labels** (§3) — one per writer id.
- **Configuration settings** (§6) — one per setting.

#### "Later" is decided by a counter, never by a clock

If the winner were chosen by wall-clock timestamp, a device with a fast clock would win every contest
until real time caught up — pinning a note's text or, far worse, a parameter vector, while the user
re-edits and watches it lose with no explanation.

So every mutable value carries a **stamp**: a counter plus the writer id. The counter obeys one rule
— *on seeing any counter greater than your own, jump above it*. Values are compared by stamp, never
by time.

This yields exactly what is needed. **If a device had seen the competing edit before making its own,
its edit wins** — real causality, no clock involved. When two edits are genuinely simultaneous the
writer id breaks the tie identically on every device, so no two devices diverge.

Note the deliberate division of labour with §2: the version summary answers *"am I behind?"*, which a
counter of this kind cannot; the stamp answers *"which of these two values is later?"*, which the
summary does not. Two mechanisms because there are two questions.

#### Deleting a note

Not previously owned by any ticket. ADR-0002 §7 established that *cards* need no deletion, but a user
deleting a whole note was never addressed, and it cannot be a plain removal — the note returns from
the next device to sync.

**Deletion is a flag, not a removal.** A note keeps its id and carries a `deleted` marker that settles
like any other value. This slots into the existing machinery exactly: the note's reviews stay in the
log, dormant, projecting onto nothing, and undeleting **reattaches the history by itself** — the same
mechanism ADR-0002 §7 built for restored cloze blanks.

**A deleted note keeps only its marker**, not its content: an id, a flag and a stamp, around forty
bytes. **Accepted cost**: undeleting restores the schedule but not the text, which must come from a
backup or an export. Delete means gone, which is also the right answer for a user deleting something
they want rid of.

> **Discharged by [ADR-0016 §4](0016-backup-and-restore.md)**, which specifies the backup this
> sentence spends. The mechanism is worth knowing because it looks as though it should not work: the
> content above is **discarded, not superseded by a competing value**, so an archive predating the
> delete carries those field values with old stamps and meets nothing to lose to under the counter
> rule. Undelete the note, restore, and the text returns — ADR-0002 §7's *"history reattaches by
> itself"* applied to content instead of schedule.
>
> **The limit that follows from the same rule**: a note whose text was *overwritten* rather than
> deleted cannot be recovered this way, because the newer stamp must win or the causality rule above
> is broken. Backup protects against loss, not against unwanted change.

> **Amended by [ADR-0008 §5 and §8](0008-the-deck-export-format.md)**: a deleted note also retains its
> `deck` reference — id, flag, deck reference, stamp, roughly sixteen bytes above the figure quoted
> here. Without it a retraction cannot be attributed to a deck, so a deck-scoped export cannot select
> its tombstones and an author cannot withdraw a note from a published deck. No content is retained,
> so "delete means gone" is untouched.

**Concurrent add-and-remove of the same tag resolves as add-wins.** A spurious tag is a nuisance; a
silently-lost one is a bug that is never found.

### 8. Clock skew is guarded and detected, never prevented

The map calls this its sharpest conflict, and this ADR neither solves it nor assumes it away.

**The damage is smaller than it appears, and for a reason worth naming.** Because state is replayed
rather than stored, one wrong day-stamp among a card's thirty reviews perturbs its stability by a few
percent and every subsequent correct review pulls it back. **No card can be permanently poisoned by a
single bad stamp.** That is a dividend of constraint 1 — the same constraint that created the
exposure.

Severity is strongly asymmetric:

| Skew | Effect |
|---|---|
| Seconds to hours | None, unless it straddles the 4am boundary; then one day gap is off by one |
| A day or two, over a stretch of reviews | A block of gaps off by one; small and self-correcting |
| **Years** | **Serious.** Rows sort into an order that never happened, so grades replay in a sequence the user never performed |

The realistic cause of the third row is mundane: a phone that goes flat and boots before reaching the
network.

**It cannot be prevented.** A device that is offline has no reference to check itself against — that
is what being offline means. The mature prior art refuses to sync beyond five minutes of skew, but it
measures against a *server's* clock, which this design does not have. Three things are therefore done
instead:

1. **Guard on write.** Never emit an instant at or below the highest already in the log; if the clock
   says earlier, use the highest plus a millisecond and stamp the day to match. Free, and it
   completely fixes the flat-battery case — the device boots believing it is 1970 while holding a log
   full of 2026, so it writes 2026. It is a memory, not an oracle: **it cannot help a device that has
   never synced.**
2. **Detect on merge, warn, do not block.** The newest row held is a lower bound on the true time, so
   a device can establish that *someone* is wrong — though never *who* — and say so specifically:
   *"this device says 3 March; another device recorded a review on 5 March."* An application that
   refuses to let the user study because of a clock is worse than a slightly wrong interval.
3. **No repair mechanism, for now.** The cutoff (§1) is the escape hatch. A precise alternative — a
   row reading *"shift writer L's rows 400–460 forward one day"* — remains cheap to add later
   **because §2 gave every row an address**, and it is an append rather than an edit, so nothing about
   append-only would break. It is not built before the failure has been seen once.

Rejected: **refusing to sync on skew** (requires the server clock we do not have); **a logical clock
that runs ahead of physical time** (one bad device drags every other forward, and with day-stamps
frozen we would be freezing fiction); **storing derived state as a safety net** (constraint 1, and it
trades a rare bounded error for a permanent one).

**No constraint bends.** The residual, stated plainly: **a device that has never synced, with a badly
wrong clock, writes wrong day-stamps permanently, and the only remedy discards good history along
with the bad.**

### 9. Replay: order, cache, and no projection versioning

**The order is: day number, then instant, then writer id, then sequence.** Every device must sort
identically or it computes a different answer from the same log; the last two components are a
deterministic tiebreak available everywhere.

Sorting by day first has a quiet benefit: **the gap between consecutive reviews can never be
negative**, which matters because FSRS's day-gap input is unsigned — a negative gap is not merely
wrong but unrepresentable. A bad clock can therefore produce a wrong *order*, which §8 accepts, but
never an input the scheduler cannot consume.

**Replay silently ignores rows whose `CardRef` the current content does not generate.** ADR-0002 §7
requires this and stresses it must not warn or discard. It looks like an error condition and is not.

**Derived state may be cached, and the cache is disposable.** Five years of heavy use is roughly
365,000 rows; the arithmetic is trivial — a couple of dozen operations per review — but reading and
unpacking the rows is not, and it lands on every launch, on a phone, on battery. So the app caches
what it computed, under a hard rule: **never the source of truth, never synced, never exported,
rebuilt without ceremony whenever it is missing or suspect.** Invalidation falls out of the structure
— a card's replay depends only on its own reviews plus current configuration, so new rows invalidate
that card and a configuration change invalidates everything.

**There is no projection versioning.** When the derivation changes — a crate upgrade, a fix to our own
day arithmetic, or one day a different scheduler entirely — the answer is *throw the cache away and
rebuild*. This is the entire dividend constraint 1 was bought for: the map promised that changing the
algorithm would be a re-derivation rather than a data migration, and this is where that is cashed.

**Accepted cost**: two devices on different application versions compute slightly different due dates
from the same log — the divergence ADR-0001 §6 feared, arriving through application versions rather
than configuration. It is accepted because **it leaves no trace.** Unlike a wrong day-stamp it lives
entirely in the disposable cache; the moment both devices are on the same build they agree again,
with nothing to repair.

### 10. The log is never compacted

Growth is bounded by arithmetic, not by policy. Heavy use — 200 reviews a day — is about 73,000 rows
a year: roughly 11 MB a year in the interchange form of §11, or under 2 MB compressed, and a packed
local representation ([#12](https://github.com/amin-bf/leitner/issues/12)'s choice) would be smaller
still. A decade of heavy use is therefore around 110 MB raw and 15 MB compressed. Typical use is a
quarter of that. Against the storage of any device we target, this is not a problem in any timeframe
the application will exist for.

> **Amended by [ADR-0013 §12](0013-the-sync-transport.md)**: the raw figure is confirmed
> independently to within 1%, but **the compressed figure carries the same condition as §11's ratio**
> — it silently assumed a large-window compressor over large blocks. 15 MB is right for `zstd` over
> writer-year-sized blocks; it is roughly 27 MB under `gzip` and roughly 22 MB under daily
> segmentation. The never-compact conclusion is untouched: it never depended on the ratio.

Trimming would also cost real things:

- **The optimiser trains on full review histories.** Discarding old reviews degrades the parameter
  fitting, which is the one feature that makes the scheduler personal rather than generic.
- **A trimmed log cannot merge with an old fork.** The one library shipping history truncation states
  the constraint plainly: peers can only sync if they hold versions after the truncation point. A
  phone that has been in a drawer for a year would be unmergeable.
- **It drags in machinery we otherwise never need**: tombstones, and a garbage-collection horizon
  measured in *how long a device might be switched off* — a quantity with no upper bound.

**Snapshots for speed, yes (§9); trimming for space, no.** The two are separate ideas and only the
first is adopted.

**Accepted cost**: data cannot be made *gone*, only hidden. A cutoff hides rows from replay without
removing them, and deleting them locally is futile while another device still holds copies — they
return on the next merge.

One partial out is permitted. Because a cutoff lives *in the log* and therefore syncs, it stays in
force everywhere; a device that physically discards rows below the cutoff to reclaim space stays
correct, since anything returning is hidden again by the same cutoff. This is allowed as a **purely
local, optional** choice, **off by default**, with one warning attached: **it makes the cutoff
irreversible on that device.** Moving the cutoff back later cannot recover what was discarded.

### 11. The interchange form

This ADR owns the row schema and **one canonical interchange form** — how a row is written when it
moves between devices or into an export. [#12](https://github.com/amin-bf/leitner/issues/12) owns
local storage, which may be anything (database columns, for instance) provided it round-trips
exactly. The interchange form is the log's real identity: what a future implementation must agree
with, and what makes device-scoped segments on a commodity store possible.

**One JSON object per line.**

```
{"k":"rev","w":"7f3a…","s":412,"n":"abc…","o":0,"g":3,"t":"2026-03-01T09:14:22.418Z","d":20514,"ms":4200}
```

Larger than a packed binary row — roughly 150 bytes against 56 — but it **compresses about ten to
one**, because every line repeats the same keys and the same handful of writer ids, so compressed it
is smaller than raw binary while remaining something a person can open and read when something has
gone wrong. This is the same trade ADR-0002 §8 made for note fields, for the same reason: a format
that can be inspected and repaired by hand is worth real bytes.

> **Amended by [ADR-0013 §12](0013-the-sync-transport.md)**: **"about ten to one" is conditional on
> two things this section does not fix — the compressor's window and the block size.** Measured on
> rows in exactly the shape above: a decade compresses **11.76× with `zstd -19`** but only **3.99×
> with `gzip -9`**, because gzip's 32 KiB window cannot reach back to the repeated writer ids; and
> block size moves it just as far — **12.01×** as one file per writer-year against **5.02×** as daily
> segments and **3.04×** for a sync-sized chunk. So the ratio belongs to a *transport* choice this
> ADR deliberately deferred. ADR-0013 §4 fixes `zstd`, and its §5 roll-up ladder carries blocks up
> that curve. **The row size needs no amendment**: two independent measurements of this exact shape
> gave 151.4 B and 152.5 B against "roughly 150 bytes".

**Two rules matter more than the format choice:**

1. **A row is relayed byte for byte and never re-encoded.** An old build receiving a row containing a
   field it has never heard of stores and forwards *the original line*, so it physically cannot strip
   data a newer build wrote. Because a row is immutable and addressed by writer and sequence, there is
   never a reason to rewrite one — forward compatibility comes out free rather than depending on
   "preserve unknown fields" discipline in every code path.
2. **Unknown row kinds are skipped, not errors.** Newline framing makes a row skippable without
   parsing it, which is what §1's discriminator exists for.

**Two details pinned because both are silent-corruption risks:**

- **The card reference is a bijection with ADR-0002 §6's canonical 18 bytes.** On the wire it is the
  note's UUID in RFC 9562 canonical text form plus the ordinal as a number; both are reversible, and
  the interval-fuzz seeding of ADR-0001 §7 uses the 18 bytes rebuilt from them. Two devices must
  compute the same due date, so this cannot be left to each call site.
- **Floating-point weights must round-trip exactly.** ADR-0001 pins the parameter vector precisely; a
  weight losing its last digit yields a different schedule. Written with full round-trip precision,
  always.

**Malformed rows do not abort replay.** A malformed *final* line is discarded silently — it is an
incomplete write from a crash mid-append, not data loss. A malformed line in the *middle* is skipped
with a warning and replay continues. Every row is independent, so continuing is safe, and a single
bad byte must never render the application unusable.

## Requirements this places on downstream tickets

### [#12 — the local store](https://github.com/amin-bf/leitner/issues/12)

1. Must **round-trip the interchange form of §11 exactly**, including fields it does not understand.
2. Owns the **disposable cache** of §9 — including its invalidation and the guarantee that losing it
   is never a correctness problem.
3. Needs efficient **append** and **per-card review lookup**; replay is per card.
4. The **mutable store** of §7 needs per-value stamps, not per-record ones.

### [#13 — the deck export format](https://github.com/amin-bf/leitner/issues/13)

1. A progress export is **log rows**, in the interchange form.
2. **Writer ids are a device fingerprint.** Exporting progress exports them. This needs scrubbing or
   an explicit warning; it is created by §2 and owned here.
3. Content export is the note store; §7's stamps travel with it or are reset on import, which #13
   decides.

### [#10 — the deck model](https://github.com/amin-bf/leitner/issues/10)

1. **Deck membership is mutable content and never appears in the log.** A review row does not record
   which deck the card was in (§5).
2. Deck fields settle by §7 like any other mutable value.

### [#21 — new-card rate and daily limits](https://github.com/amin-bf/leitner/issues/21)

1. **"Today" is the device's local day**, not the collection day scale used for `delta_t` (§4).
   > **Discharged by [ADR-0011 §5](0011-new-card-rate-and-daily-limits.md).** The daily new-card
   > count is derived against the device's local day, and **nothing from that ticket enters the
   > log** — the rate is a value on §7's mutable surface, failing §1's *"is this an input to
   > replay?"* test for the same two reasons ADR-0010 §5 gave for suspension. Constraint 1 needs no
   > third widening.

### [#26 — leeches](https://github.com/amin-bf/leitner/issues/26)

1. ~~**Owns suspension**, which is additive as a fourth row kind (§1).~~
   > **Amended by [ADR-0010 §5](0010-leeches.md).** Suspension is a value on the **§7 mutable
   > surface**, not a row kind — this table was wrong and §1's prose ("a mutable flag rather than a
   > fact that happened") was right. A flag that toggles has its winner picked by *timestamp* order
   > in the log (§9), which is exactly what §7 forbids; and it fails §1's own membership test, since
   > suspension is not an input to replay. The three row kinds stand unchanged.
2. **Answer duration is available** as a leech signal (§5) — taken up by ADR-0010 §6 as a *cost
   display*, having been rejected as a trigger for being too noisy (ADR-0010 §3).

### [#11 — the review session prototype](https://github.com/amin-bf/leitner/issues/11)

1. A same-session re-show after a lapse is a **real logged review** with a zero day gap, not a UI-only
   loop (ADR-0001 §5, §1 here).

## Glossary

**Moved.** These terms are now of record in [`log`](../../crates/core/src/log/CONTEXT.md), per
[ADR-0009 §6](0009-crate-and-workspace-layout.md), which fixed where contexts live. They
were marked provisional here precisely so this could happen: the `CONTEXT.md` is
authoritative, and this ADR keeps the reasoning behind them.

**Cache** moved to [`replay`](../../crates/core/src/replay/CONTEXT.md), which decides what it holds.

## Consequences

- **Scheduling state exists nowhere but in a cache that may be deleted at any time.** Changing the
  scheduler is a rebuild, not a migration.
- **No event can ever be removed.** Corrections are coarse — a collection-wide cutoff — and a
  mis-pressed grade is permanent. This is the price of union-merge without tombstones, and it is paid
  knowingly.
- **A never-synced device with a badly wrong clock writes permanently wrong day-stamps.** Guarded
  against the common causes, detected after the fact, never prevented.
- **The log grows forever, and that is fine** — around 110 MB per decade of heavy use, raw.
- **Two devices on different application versions may disagree about due dates.** Transient by
  construction, because nothing about it is written down.
- **Writer ids accumulate and are a fingerprint.** Cheap to hold; a privacy consideration for export.
- **Nothing here needs a server**, and the shape it produces — each device appending only to its own
  rows — is exactly the device-scoped-segment transport the map is keeping open.

## Open items handed onward

| Item | Owner |
|---|---|
| Local storage engine; the cache; per-value stamps | [#12 — the local store](https://github.com/amin-bf/leitner/issues/12) |
| Export container; scrubbing writer ids from a progress export | [#13 — the deck export format](https://github.com/amin-bf/leitner/issues/13) |
| ~~Suspension as a fourth row kind~~ — **closed by [ADR-0010 §5](0010-leeches.md)**: it is a value on the §7 mutable surface, keyed by `CardRef`, and no fourth row kind exists | [#26 — leeches](https://github.com/amin-bf/leitner/issues/26) |
| ~~How the mutable store moves between devices — snapshot or change stream~~ — **closed by [ADR-0013 §7](0013-the-sync-transport.md)**: the question dissolves, because a writer's own counter is monotone, so compacting its change stream to the latest value per key *is* a per-writer snapshot. Deltas per sync, snapshot as the roll-up result. Publishing it **per writer** rather than as one shared document is what keeps conditional writes out of the design | [#39 — the sync transport](https://github.com/amin-bf/leitner/issues/39) |
| Whether a precise clock-correction row is ever needed | Deferred until the failure is seen |
| "Everything is merged, you are safe" reassurance in the UI | [#40 — the sync experience](https://github.com/amin-bf/leitner/issues/40) |
