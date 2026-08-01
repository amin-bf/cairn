# ADR-0005: The deck model

- **Status**: Accepted
- **Date**: 2026-07-29
- **Resolves**: [Decide: the deck model](https://github.com/amin-bf/leitner/issues/10)
- **Map**: [Map: local-first Leitner app spec](https://github.com/amin-bf/leitner/issues/1)
- **Related**: [ADR-0002: The card model](0002-the-card-model.md),
  [ADR-0004: The review event log](0004-the-review-event-log.md)

## Context

Two tickets wait on this one — [new-card rate and daily limits](https://github.com/amin-bf/leitner/issues/21)
and [the deck export format](https://github.com/amin-bf/leitner/issues/13) — and ADR-0002 handed this
ticket three open items directly: whether notes belong to decks (yes, exclusively — ADR-0002 §10),
whether a note may belong to more than one deck, and where deck fields sit relative to the mutable
surface ADR-0004 built.

Standing constraint 2 requires decks to be portable and publishable: stable identity, a self-contained
export, and content cleanly separable from personal review progress. This ADR fixes what a deck *is*
so that constraint has an object to apply to.

While this ticket was open, [ADR-0004](0004-the-review-event-log.md) landed and settled the mutable
surface this ADR depends on: every independently editable thing settles on its own, by a stamp
(counter plus writer id), never by a clock or a last-writer wall-clock timestamp. Deck fields use that
mechanism directly; nothing here reopens it.

## Decision

### 1. A deck is the shipping container, not a filing system

A deck is **the unit of ownership and export** — the thing authored or acquired as a whole, and the
thing handed to someone else. It is not a folder a user reshapes for personal convenience.

Personal organisation — grouping, cross-cutting labels, "cards that keep catching me out" — is
**tags' job**, already established by ADR-0002 §10: tags live on notes, are content, and cut across
whatever decks exist.

Rejected: **deck as filing system**, where a user freely creates and reshuffles decks as their own
hierarchy. Rejected because the two jobs need opposite properties. Export needs an object with stable
identity that persists across updates; personal filing wants to be reshuffled at will. A user who
splits an imported deck to study it in smaller pieces destroys the very identity constraint 2 needs to
match a future update against. Giving each job its own mechanism — tags for organisation, decks for
shipping — costs nothing, because tags already exist and already have the right properties.

**Accepted cost**: a deck acquired from someone else cannot be subdivided without either splitting it
(losing its identity — see §2) or tagging its notes, which is a content edit to material that is not
yours (§9's collision rule governs the same-id case; splitting is a local reorganisation that changes
nothing about the original identity, so it is unaffected by that rule, but it does forfeit clean
future updates from the original).

### 2. A note belongs to exactly one deck

Membership is a **total function**: every note that has a deck reference points at exactly one deck
(see §7 for notes with none).

Rejected: **multi-membership**, where a deck is a set a note can join more than one of. It solves one
real case — a deck forked from another, sharing note ids, where the shared notes could sit in both —
but it does so by making a deck a set a note joins, which is precisely what a tag already is. Building
that mechanism twice erodes the distinction §1 just drew, and the erosion runs one way: decks drift
back into folders. The fork case also needs a scenario this destination has mostly ruled out — one
deck derived from another via export/import — since **hosted deck discovery is out of scope** on the
map; without a library to fork from, decks arrive from one person at a time.

**Import collision rule**: a note whose id already exists in the collection is the same note. It is
not re-imported and does not move; the import reports it. This protects existing review history over
completing an import, which is the right trade — history cannot be reconstructed, an import summary
can be re-read.

**Accepted cost**: an imported deck forked from one you already have can arrive incomplete, missing
whichever notes collide with ones you already hold. The application must report this, not hide it.

### 3. Decks are a flat set

No parent, no subtree, no nesting of any kind. A deck name is a free-form string; `::` or any other
separator carries **no structural meaning**, and the UI must not reconstruct a tree from one.

Four reasons:

1. Nesting is organisation, and §1 already assigned organisation to tags.
2. Nesting is incoherent with "unit of export" (§1): shipping a parent deck forces a choice between
   subsuming the children's identity into it or shipping an empty shell. Neither answer says the tree
   is made of decks.
3. A tree needs merge rules a flat set does not — cycle detection under concurrent offline reparenting,
   and a rule for children when a parent is deleted — bought for a benefit tags already deliver.
4. The common realisation of deck hierarchies in this domain is a name-based naming convention with the
   tree reconstructed for display — itself the tell that the "structure" is presentation, not data.

The one real use nesting serves — studying everything under a broad heading — is served instead by
multi-select at session start (§6), which needs no advance filing into a common ancestor.

### 4. Identity is a minted id, preserved through export and import

A deck's id is a **UUIDv4**, minted once at creation, never changed by renaming, export, or import.
The **name is a mutable, non-unique display label** — two decks may share a name; a uniqueness
constraint would need its own merge rule the moment two offline devices rename toward the same string,
for no benefit a user can't already see.

This mirrors ADR-0002 §6's treatment of note ids for the same reason: a deck is created by a user
action, so minting is correct, and the collection needs no second identity mechanism.

**The deck id travels inside the export file.** This is what makes updates work: importing a file
whose deck id matches one you have is recognised as the same deck, matched against your notes by note
id (ADR-0002 §6), so your review history for its cards survives the update untouched. Without this,
every update would be a new deck and progress would strand.

**Import is therefore not a user choice.** Id matches something you have → update. Id doesn't → new
deck. There is no "import as a separate copy": a second independent copy would need fresh note ids,
which ADR-0002 §6 forbids re-minting, and reusing note ids across two decks collides in the review log
per §2's finding.

**Publishing a modified copy of someone else's deck mints a new deck id but not new note ids** — it
inherits §2's incomplete-import edge for whichever notes collide with the original.

Handed to [#13](https://github.com/amin-bf/leitner/issues/13): a deck **content revision**, so an
import can tell an older file from the one already held and refuse to go backwards. That is what the
export *file* declares, not part of the deck object itself.

### 5. A deck is `{ id, name }` — content, and nothing else

No configuration lives on the deck. Scheduler parameters, algorithm identity, and desired retention
are already collection-wide per ADR-0001 §6 ("Global, not per-deck") and are not reopened here.
New-card rate and daily limits are [#21](https://github.com/amin-bf/leitner/issues/21)'s to decide, not
this ticket's — but this ADR fixes where any such setting may lawfully live, because getting this wrong
would silently break constraint 2.

The controlling test, made explicit because the binary constraint 2 poses — content versus personal
progress — undersells a real three-way split the mutable surface now has:

| | Mutability | Exports with the deck? |
|---|---|---|
| Review log (ADR-0004) | Append-only, immutable | No |
| Notes, tags, deck `{id, name}` | Mutable | **Yes** |
| Personal per-deck preferences | Mutable | **No** |

**Not on the deck**, because the deck exports and a personal setting is a fact about the user, not the
material — putting it there would ship your daily limit to whoever you send the deck to.

**Not in the review log**, despite ADR-0001 §6 putting scheduler configuration there. That precedent
doesn't transfer: the parameter vector is a *replay input* — devices disagreeing about it produce
divergent memory state with no missing event to reveal it. [#2](https://github.com/amin-bf/leitner/issues/2)
found queue composition orthogonal to per-card replay, so a daily limit is not a replay input, and
logging it would write a permanent fact for every nudge from 20 to 25 — the same cost ADR-0002 §7
refused for content edits, and worse, since preference churn has no natural end.

**What's left**: the mutable surface ADR-0004 §7 built, which already holds more than notes — "which
parameter vector is current" is exactly such a register (ADR-0001 §6). A per-deck preference is
another value of the same kind, keyed by deck id, settling per ADR-0004 §7's stamp rule like any other
independently editable thing.

Left open for #21, deliberately: whether such a preference **syncs between your own devices** or stays
device-local. The mutable surface syncs by default, but a commute-sized limit on a phone versus a
desk-sized one on a laptop is a real counter-case, and #21 is where it should be weighed.

> **Amended by [ADR-0008 §9](0008-the-deck-export-format.md)**: the deck-id-keyed slot is not uniformly
> personal. It holds **personal** values, as described here, and **authoring** values such as a deck's
> export revision — which **must** sync, since otherwise an author exporting from a laptop and a phone
> emits conflicting revisions as routine behaviour. Both kinds are alike in never exporting and never
> appearing in the log; they differ in whether the syncing question is open. Only the personal kind is
> #21's to decide.

Also handed onward: **author, description, licence** and similar deck metadata are things an export
*file* declares about itself — [#13](https://github.com/amin-bf/leitner/issues/13)'s concern, not the
deck object's.

> **Widened by [ADR-0022 §8](0022-the-import-preview-and-export-report.md): those three land back
> here, in the authoring half.** They are still what the *file* declares — nothing about them exports
> as deck content — but they have to be **remembered per deck id and synced**, or the same defect
> ADR-0008 §9 named for the revision recurs verbatim: an author publishing updates from two devices
> emits one file crediting them and one anonymous. So the authoring half now holds
> `{revision, digest}` **and** author, description and licence, and the sentence above is right about
> ownership while being incomplete about storage. **The personal half stays empty and stays #21's** —
> ADR-0011 §6 made the new-card rate global, so no personal per-deck preference exists yet, and the
> syncing question survives unanswered.

### 6. Review spans the whole collection; a session is not a domain object

The default and primary flow is **everything due, collection-wide**, in one queue. Per-deck queues
would split a single day's obligations into piles finished separately, one of which quietly
accumulates while the others are worked.

**Narrowing is a filter, not a mode**: any subset of decks, optionally intersected with tags, composes
freely because both are filters over the same queue. This is what serves "study everything under
Spanish" without §3's nesting — multi-select costs nothing extra over a single selection.

**A session has no identity and is recorded nowhere.** ADR-0004 §5 already declined to put a deck on a
review row; giving a session an id would be the same mistake under a different name. It is transient
UI state.

Per-deck due counts are **display, not authority** — following where a card lives *now*, per
ADR-0004 §5 — while the number that governs a day is the collection-wide one. [#2](https://github.com/amin-bf/leitner/issues/2)
and ADR-0001 §7 already established that queue composition cannot corrupt per-card replay, so
narrowing a session is safe by construction.

### 7. Deletion is a flag; note-deletedness derives from it

A deck carries a **`deleted` flag**, settling by ADR-0004 §7 like any other mutable value — not a
plain removal, for the reason ADR-0004 §7 gives for notes: a removal returns from the next device to
sync.

**A note is deleted if its own flag is set, or its deck's flag is set** — derived, not cascaded into a
flag per note. This handles a concurrent edit a cascade gets wrong: deleting a deck on one device while
another, offline, adds a note to it. Under a cascade the new note becomes an orphan with no rule
covering it; under the derived rule it is simply deleted, with no special case required.

**Deleting a deck discards the content of every note it holds at that moment**, matching ADR-0004 §7's
per-note answer ("delete means gone") — a 5,000-note deck's deletion is a decision to want 5,000 notes
gone, and keeping their text would be a different, unrequested decision. The review log is untouched;
those reviews go dormant exactly as ADR-0002 §7 describes.

**Recovery is re-import.** Because deck and note ids are preserved through export and import (§4,
ADR-0002 §6), re-importing a previously-exported copy of a deleted deck restores its notes under the
same ids, and history reattaches by itself through the mechanism ADR-0002 §7 built for restored cloze
blanks. Deleting a deck and re-importing it a year later restores the review history untouched — this
falls out of stable identity rather than requiring its own mechanism.

**Binding on the authoring UI**: state how many notes will lose content before committing to a deck
deletion, and offer "move these notes to another deck" as a non-destructive alternative — the same
obligation ADR-0002 §7 places on a destructive cloze edit.

### 8. Membership is a `deck` reference on the note

A note carries a `deck` field naming the deck it belongs to. This follows ADR-0004 §7 directly — every
independently editable thing settles on its own — so moving a note between decks is one value changing
on one object, using the stamp mechanism already built for note fields and tags.

Rejected: **a member list on the deck**. A list is one value two devices adding different notes to
would contend over, unless decomposed into independently-settling members — arriving at the same place
through more machinery. Membership belongs on the note for the same reason tags do (ADR-0002 §10):
everything about where a note belongs lives in one object.

**A dangling `deck` reference means unfiled, not lost.** Content and decks are both mutable-surface
values settling independently, so nothing guarantees a deck arrives before a note pointing at it (or
survives — see §7). Such a note appears in an unfiled view and remains fully reviewable; it is never
dropped and never silently reassigned.

**No privileged or auto-created default deck.** A built-in "Default" deck, created automatically on
first run, produces a guaranteed rather than merely possible bug: two devices set up before ever
syncing each mint their own UUID for their own "Default", and the moment they meet, two decks exist
with the same name, both genuine, neither wrong — unrecoverable, because §4 correctly made identity a
minted id rather than a name, so no naming rule could ever have unified them after the fact.

A collection may legitimately contain **zero decks**. Deck creation is always an explicit user act, so
every deck has one point of origin. The first attempt to author a note asks which deck, offering to
create one — a small first-run cost in exchange for eliminating a whole class of unrecoverable
duplicate.

### 9. On import, the file wins for everything it carries

Reorganisation an author publishes in an update — commonly a split, one deck becoming two — must
propagate, or an update silently fails to update anything.

**ADR-0004 §7's stamp rule cannot arbitrate this**, and that is worth stating because reaching for it
here is the natural mistake. Its counters are per-collection, incremented against writers in that
collection's own history; comparing a stamp from an imported file to one in your collection compares
two unrelated sequences, and a higher number carries no meaning across them. **Import is a policy
decision, not a merge.**

The policy:

- **Membership of notes the file contains follows the file.** A deck's composition is the author's
  statement about the material, and §1 already established that a user does not file with decks — so
  there is no competing user intent to protect.
- **Notes the file does not mention are untouched.** An update never removes what it does not name;
  your own additions to an imported deck stay where you put them.
- **Decks named in the file that you don't have are created.**
- **A deck left empty by an upstream split is left alone and surfaced, never auto-deleted.** Deleting a
  user-visible object on an author's behalf is not an import's business, and §7 already made deletion
  destructive of content.
- **The deck name is overwritten by the update**, being authored content that exports (§4) — consistent
  with everything else the file carries, even though a user's own rename will feel lost. If this
  matters in practice, the fix is already shaped by §5: a personal display-name override on the mutable
  surface, keyed by deck id, never exported and therefore never overwritten by an update. Not built
  now; recorded so the answer is not reached for later by weakening import instead.
- **Review progress is untouched throughout, structurally.** Progress keys on `CardRef` — a note id and
  an ordinal (ADR-0002 §6) — which no deck reshuffling can reach.

## Requirements this places on downstream tickets

### [#13 — the deck export format](https://github.com/amin-bf/leitner/issues/13)

1. A deck **content revision** (§4), so import can tell an older file from a newer one and refuse to
   go backwards.
2. **Author, description, licence** and similar file-level metadata (§5) — not part of the deck object.
3. **"Does it travel with the deck?"** (§5) is the test for what belongs in an export versus the
   personal mutable surface; apply it to anything new the export format introduces.
4. **Import is policy, not merge** (§9) — do not reach for ADR-0004 §7's stamp rule to arbitrate an
   import; its counters are per-collection and meaningless across collections.
5. §9's policy in full: file wins for membership and name among notes/decks it carries; untouched
   notes stay put; missing decks are created; emptied decks are surfaced, not deleted; progress is
   structurally untouched.

### [#21 — new-card rate and daily limits](https://github.com/amin-bf/leitner/issues/21)

1. A per-deck limit, if wanted, is a **personal preference keyed by deck id** on the mutable surface
   (§5) — never a deck field, never a config-set row in the review log.
2. Decide whether such a preference **syncs between a user's own devices** or stays device-local (§5).
3. Decide how a per-deck limit **composes with the single collection-wide queue** (§6) if one is
   introduced.

## Glossary

**Moved.** These terms are now of record in [`content`](../../crates/core/src/content/CONTEXT.md), per
[ADR-0009 §6](0009-crate-and-workspace-layout.md), which fixed where contexts live. They
were marked provisional here precisely so this could happen: the `CONTEXT.md` is
authoritative, and this ADR keeps the reasoning behind them.

Decks did not earn a context of their own — see ADR-0009 §6. Deck *files* did: see [`export`](../../crates/export/src/CONTEXT.md).

## Consequences

- A deck's only content is its id and name; every other fact a user might associate with "a deck" —
  daily limits, personal renames of an imported deck — lives elsewhere, keyed by deck id, and is
  explicitly excluded from export.
- Decks are flat and unnested; any apparent hierarchy in deck names is presentation a user chose, not
  structure the application maintains or reconstructs.
- A note's deck membership is a single settling value on the note, using the same stamp mechanism as
  its fields and tags — no new merge machinery.
- Deleting a deck is destructive of content by design, symmetric with deleting a note, and fully
  recoverable by re-import because identity is preserved and history reattaches on its own.
- Zero decks, and notes with no deck, are both legal, permanent states — not transient or erroneous —
  because no deck is ever created automatically.
- Updating an imported deck can silently overwrite a user's own rename of it; the fix, if wanted, is a
  personal override that this ADR shapes but does not build.

## Open items handed onward

| Item | Owner |
|---|---|
| Deck content revision; author/description/licence metadata; export policy for §9 | [#13 — the deck export format](https://github.com/amin-bf/leitner/issues/13) |
| ~~Whether a per-deck limit exists, and how it composes with one queue~~ — **answered `no` by [ADR-0011 §6](0011-new-card-rate-and-daily-limits.md)**: the rate is **global**, because with one collection-wide queue per-deck rates make the real daily obligation a **sum shown on no screen**. §5's deck-id slot stays deliberately empty | [#21 — new-card rate and daily limits](https://github.com/amin-bf/leitner/issues/21) |
| **Whether a deck-id-keyed *personal* preference syncs between a user's own devices or stays device-local.** The one part of the row above that survives it, since ADR-0011 never needed such a preference to exist | **Out of scope** — inherited by whatever effort builds *per-deck new-card on/off*, which [the map](https://github.com/amin-bf/leitner/issues/1) ruled out on 2026-07-31 and which names this as its one live sub-question |
| A personal display-name override for an imported deck | **Out of scope** — [the map](https://github.com/amin-bf/leitner/issues/1), 2026-07-31. Nothing left to decide: §11 records the shape (a personal setting keyed by deck id on §5's slot, never exported and therefore never overwritten), so what remains is a build |
