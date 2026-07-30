# ADR-0002: The card model

- **Status**: Accepted
- **Date**: 2026-07-27
- **Resolves**: [Decide: the card model](https://github.com/amin-bf/leitner/issues/6)
- **Map**: [Map: local-first Leitner app spec](https://github.com/amin-bf/leitner/issues/1)
- **Evidence**: [`docs/research/local-first-event-log/`](../research/local-first-event-log/README.md)
- **Related**: [ADR-0001: Scheduling algorithm and grade scale](0001-scheduling-algorithm-and-grade-scale.md)

## Context

Three tickets wait on this one — [the review event log format](https://github.com/amin-bf/leitner/issues/9),
[the deck model](https://github.com/amin-bf/leitner/issues/10) and
[the deck export format](https://github.com/amin-bf/leitner/issues/13) — and all three wait on the
same thing: **what a review event points at, and whether that thing holds still.**

The map's standing constraint 1 makes the review log append-only and immutable, and derives all
scheduling state by replaying it. Every event names a card. If a card's identity is unstable, or if
two devices can disagree about which cards exist, the log accumulates events that project onto
nothing, or onto the wrong thing — and because the log cannot be edited, that damage is permanent.

Constraint 2 additionally requires card **content** to be cleanly separable from personal **review
progress**, so decks can be exported and shared without carrying one person's memory state.

This ADR fixes the shape of authored material and the identity of the things that get scheduled. It
does not decide what a deck is (#10), how events are encoded (#9), or where any of it is stored
(#12).

## Decision

### 1. A note holds content; a card is a generated view of it

Authoring produces a **note**. A note is not scheduled and is never shown to the user as a question.

A **card** is a question generated from a note by rule. Each card carries its own independent
schedule, so the two directions of a vocabulary pair — recognising a word and producing it — track
separately, which is correct because they are separately learnable skills.

One note yields one or more cards. Editing the note changes every card drawn from it at once.

Rejected: **a flat model** in which a card is its own content. It is meaningfully simpler and it
eliminates sibling cards entirely — which is attractive, because collection-dependent scheduling
adjustments are precisely what ADR-0001 §7 had to disable to keep replay pure. It was rejected
because introducing the note layer *later* is a migration on an append-only log: existing cards
would have to be split, their identities re-minted, and their accumulated review history either
orphaned or repaired by a fix-up event invented for the purpose. The reverse direction is free —
this model permits a kind that generates exactly one card, which is a flat model with one
indirection. We take the door that stays open.

**This creates sibling cards, and ADR-0001 §7 already disabled sibling avoidance.** That decision
stands and needs no revisiting: cards drawn from the same note may fall due on the same day, and we
do not spread them, because spreading them depends on state outside a single card's history and
would break replay purity.

### 2. Note kinds are a closed set, defined in the codebase

A note declares a **kind**. The kind determines the note's fields and how many cards it generates.

The set of kinds is closed: a user selects a kind, and cannot author a new one. Adding a kind is a
change to the application.

The kind is stored as a **string identifier**, not an enumeration ordinal, so that adding a kind is
an additive change to every stored and exported document rather than a breaking one. A kind
identifier is permanent: never reused, never repurposed to mean something else.

The starting set:

| Kind id | Fields | Cards |
|---|---|---|
| `basic` | Front, Back | 1 |
| `basic-reverse` | Front, Back | 2, one per direction |
| `vocab` | Term, Meaning, Pronunciation, Example | 2, one per direction |
| `cloze` | Text | one per numbered blank (§5) |

Rejected: **user-authored note types**, in which a user defines arbitrary named fields and writes
card templates against them. Three reasons, in descending weight.

First, a user-editable schema is the part of this data model that provably does not merge. The most
mature application in this space — two decades of production use, a data model close to ours, and
the only prior art operating at real scale — does not attempt to merge a note type whose field or
template count differs between two replicas; it aborts the sync and demands a full resynchronisation
from one side. That is an implementation that has run this exact problem for twenty years declaring
the surface unmergeable. Standing constraint 3 requires that sync not be foreclosed, and shipping a
user-editable schema forecloses it before a single line of sync code is written.

Second, it breaks portability. A deck carrying its own bespoke schema means import must reconcile
two schemas that share a name and disagree about their fields. A closed set means every deck
everywhere speaks the same shapes and import has nothing to reconcile.

Third, a template language is a parser, a renderer and an authoring UI on desktop, web *and*
Android — a sub-project in its own right, purchased for flexibility nobody asked for.

**Accepted cost**: a shape the set does not cover is a change to the application, not something a
user can work around locally. The string identifier keeps that change cheap.

### 3. A field is either asked or shown-only

Not every field is something you are tested on. On a `vocab` note, `Term` and `Meaning` each serve
as the question in one card and the answer in the other; `Pronunciation` is never either. It is
supporting information that belongs next to `Term` wherever `Term` appears.

Every field therefore declares one of two roles:

- **`asked`** — may appear as a card's prompt or as its answer.
- **`shown-with(F)`** — never asked. Renders wherever field `F` renders, on whichever side `F` is
  on for the card being displayed.

The rendering rule is exactly: *for a card whose prompt includes field `F`, every field declared
`shown-with(F)` renders on the prompt; for a card whose answer includes `F`, they render on the
answer.*

This is what produces the behaviour we want without special-casing it. Reviewing German→English,
`Term` is the prompt, so the pronunciation appears with the prompt — where it is useful and gives
nothing away, because it is not the answer. Reviewing English→German, `Term` is the answer, so the
pronunciation appears with the answer, where it teaches the pronunciation at the moment the word is
revealed.

A `shown-with` field may attach only to an `asked` field, and attachment may not chain.

### 4. Layout is data, stored once per kind

A kind definition is **data**, not code branching on a kind identifier:

```
KindDefinition {
  id:     "vocab",                     // permanent string identifier
  fields: [ { name: "Term",          role: asked },
            { name: "Meaning",       role: asked },
            { name: "Pronunciation", role: shown-with("Term") },
            { name: "Example",       role: shown-with("Term") } ],
  cards:  [ { prompt: ["Term"],    answer: ["Meaning"] },      // ordinal 0
            { prompt: ["Meaning"], answer: ["Term"] } ]        // ordinal 1
}
```

These definitions are **read-only data shipped with the application**. Being data does not make them
user-editable — §2 stands. What it buys is that a deck file can carry them, and therefore stands on
its own.

> **Widened by [ADR-0008 §7](0008-the-deck-export-format.md)**: a collection may also hold definitions
> **acquired from an imported file**, for kinds the running build does not ship — which is what lets an
> old build render a deck built by a newer one. They remain read-only, and a **shipped definition always
> wins**: an acquired one can never displace it. §2 is not reopened, because its objection was
> user-editability rather than provenance, and the evolution rules below are what make an acquired
> definition safe to trust.

A note stores only its kind and its values:

```
Note { id, kind: "vocab", fields: { "Term": "der Hund", "Meaning": "the dog", … }, tags: [ … ] }
```

**Definitions are stored once per kind, never copied onto each note.** A 500-note vocabulary deck
carries one `vocab` definition, not 500 identical copies of it; a correction to the layout is one
edit, not 500, and cannot leave a deck internally inconsistent.

**An export carries the definitions for every kind its notes use** (§9, and #13 owns the container).
This is what makes a deck self-contained: a reader that has never heard of `vocab` can still render
the cards correctly, because the file explains itself. Constraint 2 asks for a self-contained export
format, and layout that lives only in our source code would not be one.

#### Rules for evolving a kind definition

Binding on anyone editing these definitions later:

- **The `cards` list may only be appended to.** Never reorder, never remove. A card's ordinal is
  half its identity (§6); reordering the list silently reassigns every accumulated review history
  in the collection to a different card, and the log cannot be edited to repair it. This is the
  single most destructive edit available in this codebase.
- **Fields may be added.** A note that predates the field reads it as empty.
- **Removing or renaming a field is a breaking change** and out of scope here; it needs a migration
  decision, because notes hold values under the old name.
- **A kind identifier is never reused** for a different shape.

### 5. Cloze produces one card per numbered blank

A `cloze` note holds a `Text` field containing **numbered blanks**:

```
The {{1::mitochondria}} is the powerhouse of the {{2::cell}}.
```

Each distinct number generates one card, which hides that number's blanks and shows the rest of the
text. The example yields two cards, scheduled independently, so you can be shaky on *cell* while
solid on *mitochondria*.

Precise rules:

- The syntax is `{{n::text}}`, `n` a positive integer. Chosen on its properties: doubled braces are
  vanishingly rare in ordinary prose, `::` separates unambiguously from the hidden text, and the
  number is written explicitly rather than inferred. None of it collides with the Markdown subset
  in §8, which gives `{` no meaning.
- **The number is the card's identity, not its position in the text.** A blank's number is what it
  was authored as. Inserting a new blank before existing ones does not renumber them.
- **The same number may appear more than once**, hiding every occurrence on the same card — the
  natural way to blank both instances of a repeated word.
- **Numbers may have gaps.** `{{1::…}}` and `{{3::…}}` yield two cards, at ordinals 1 and 3. A gap
  is normal and expected: it is what deleting blank 2 leaves behind.
- Numbering conventionally starts at 1. This differs from fixed-arity kinds, whose ordinals index
  the `cards` list from 0 — the two never occur in the same note, so no ambiguity arises, and a
  false uniformity would be worse than the honest difference.

**The authoring UI must never renumber blanks automatically.** Renumbering is indistinguishable from
retyping the note's history onto the wrong cards: change blank 2 to blank 3 and every review of the
old blank 3 attaches to what used to be blank 2. Editors that "tidy" numbering are actively
dangerous here.

This is the one kind whose card count varies with its **content** rather than its kind. §7 explains
why that costs nothing.

Rejected: **one card per cloze note**, hiding all blanks at once. It preserves the tidy property
that card count is fixed by kind, but it gives up the reason to want cloze at all — independent
tracking per blank — and produces a card that gets easier as any one blank is learned.

### 6. A card's identity is derived from its note and its ordinal, never minted

A card is identified by the pair:

```
CardRef { note: NoteId, ordinal: u16 }
```

For fixed-arity kinds the ordinal is the index into the kind's `cards` list. For `cloze` it is the
authored blank number. **No card is ever assigned an identifier of its own.**

This is forced by the absence of a server, and the failure it avoids is worth stating concretely.
Add a third blank to a cloze note on a laptop; the phone, offline, receives the new text later and
generates its own third card. Had each device minted a random identifier at generation time, the
same blank would now carry two identities that no merge could ever reconcile — nothing distinguishes
them from two genuinely different cards — and the review history for that blank would be split in
half permanently. Cards are **generated by rule, not created by a user action**, so no
"card created" event exists to merge; two devices running the same rule over the same content must
independently arrive at the same answer. Derived identity is the only construction with that
property.

Notes, by contrast, *are* created by a user action, so a minted identifier is correct there:

- **A note's id is a UUIDv4**, minted once at creation, never changed — not by editing, not by
  export, not by import. Import uses it to distinguish "I already have this note" from "this is
  new". Random rather than time-ordered: the collection is a few thousand notes, so the index
  locality a sortable id would buy is worth nothing, while the map's unresolved clock-skew tension
  is a standing reason to prefer identifiers that do not depend on a wall clock at all. *(This is
  the one item in this ADR not put to the human directly; it is an implementation detail the spec
  has to pin, and #12 may revisit the encoding without disturbing anything above.)*

#### Canonical encoding

ADR-0001 §7 seeds interval fuzz from `(card_id, review_count)` and requires every device to compute
the same date. That requires a byte encoding fixed here rather than left to each call site:

> **A `CardRef` encodes as the note UUID's 16 bytes in RFC 9562 order, followed by the ordinal as a
> big-endian `u16`.** Eighteen bytes, no separators, no text form.

This is the encoding used for fuzz seeding and for card references in the event log. #9 may frame it
differently on the wire, but any framing must be a bijection with these 18 bytes.

### 7. Content is mutable and lives apart from the review log

Two stores, with different rules:

- **The review log** is append-only and immutable. Reviews are facts that happened.
- **The note store** is mutable. A later edit supersedes an earlier one.

Content edits are **not** events in the review log.

Rejected: **a single log carrying content edits as events**. It is appealingly uniform and yields a
full edit history, but it does not actually escape the merge problem — replaying two conflicting
edits still needs a rule for which wins — and it pays for that non-escape by growing the immutable
log with every typo correction.

Constraint 2's separation of content from progress becomes structural under this split rather than a
filter someone has to remember to apply: exporting content without progress is exporting one store
and not the other.

**Accepted cost**: editing the same note on two offline devices loses one of the edits when they
meet, silently. For a single-user application this is rare and tolerable. #9 owns the exact rule —
whole-note or per-field last-write-wins — and the recommendation from here is per-field, since it
costs one timestamp and loses strictly less.

#### What falls out: there is no such thing as retiring a card

Sections 5, 6 and 7 combine into a property none of them was designed to produce.

A note's card set is **computed from its current content**, and the log holds review events keyed by
`CardRef`. Replay therefore has an obvious definition:

> For each card the note **currently** generates, replay the events whose `CardRef` matches it.
> Events referencing a `CardRef` the current content does not generate are retained in the log and
> **not projected**.

Delete blank 2 from a cloze note and its card stops existing. Its forty reviews stay in the log,
project onto nothing, and cost nothing. Restore the blank and **the history reattaches by itself** —
because the identity was derived from content in the first place, and the content is the same again.

So the model needs no retirement flag, no tombstone, no card lifecycle, and no deletion event. The
same mechanism absorbs a note changing kind, a kind gaining a card, and a blank being renumbered and
put back.

Two consequences to be honest about:

- **Deleting a blank silently retires a card with history behind it.** The authoring UI must say so
  before the edit is committed, because nothing downstream can.
- **Log growth is unbounded by content deletion** — events for long-gone cards are never collected.
  At a few hundred events a day this is measured in kilobytes a year. Compaction, if it is ever
  wanted, is #9's.

### 8. A field is a plain string of restricted Markdown

Field values are text. The renderer supports **bold, italic, inline code, line breaks and lists**,
and nothing else.

The point is not the formatting; it is that **a field remains a plain string**. Everything
downstream depends on that: the export file is readable and diffable, the last-write-wins merge in
§7 compares two strings, and a broken note can be repaired in a text editor. Rich text makes a field
a structured document and degrades every one of those, in exchange for an editor to be built three
times over.

Explicitly excluded, each for its own reason:

- **Raw HTML passthrough** — a sanitisation surface on every platform, for no gain over the subset.
- **Image and link syntax** — would silently pre-empt §9.
- **Mathematical notation.** A maths renderer on desktop, web and Android is a sub-project. Adding
  it later carries a specific hazard worth recording now: whichever syntax it claims — `$…$` being
  the conventional choice — will begin reinterpreting existing notes that happen to contain that
  character. This is fog on the map, not a decision deferred by oversight.

**The authoring UI renders a live preview** beside the input, so what a card will look like is
visible while writing it. This matters more than it sounds: a markup a user cannot see the effect of
is a markup they will get wrong, and `{{1::…}}` blanks are hard to proofread unrendered.

### 9. No media, but the export container must be able to carry it

Fields hold text. There are no image or audio attachments.

Deferring is safe because of where media falls: §7 put content and reviews in separate stores, and
media is purely a content concern. It cannot touch the review log and it cannot touch card identity,
since a `CardRef` is a note id and an ordinal regardless of what the fields hold. The blast radius is
the note store and the export format — nothing structural, nothing immutable.

**Binding on #13**: the export format is a **container capable of carrying binary files**, even
though it carries none today. A deck is an archive, not a single text document. This costs almost
nothing now, and it converts "add media later" from a format break invalidating every deck ever
exported into filling a slot that was left open.

Two reasons to defer rather than build:

1. The motivating case is already solved as text. `Pronunciation` on the `vocab` kind is a written
   field — searchable, diffable, mergeable, and weightless. Recorded audio is none of those.
2. Media is where sync stops being easy. Text notes are kilobytes; an audio library is hundreds of
   megabytes, and constraint 3 is far easier to honour while the syncable payload stays small.
   Deferring keeps the decision unmade until the ticket that owns sync can weigh it properly.

**Audio remains on the map's fog**, called out specifically. For a language whose pronunciation is
not derivable from spelling, a native speaker's recording is worth more than a phonetic
transcription, and that case will come back.

### 10. Tags live on notes, and they are content

A note carries a set of **tags** — free-form strings, cutting across whatever decks exist.

**On the note, not the card.** A tag describes the material, and every card drawn from a note shares
that material. Tagging one direction of a vocabulary pair `irregular` but not the other is a
distinction with no meaning and twice the upkeep.

**Tags are content.** They live in the note store, and they export with the deck: `chapter-3` and
`irregular` describe the material and should travel with it.

The personal case — *cards that keep catching me out* — is deliberately not served by tags, because
it is already served better by something free. **Scheduling state is derived from the review log**,
so "cards I keep failing" is a query over history: always current, never needing maintenance, and
incapable of going stale the way a hand-applied label does.

Should personal tags ever be wanted, they are an additive change — a second tag set on the progress
side of the §7 split — and nothing here forecloses it.

**Recommendation to #9**: merge tags by **set union** rather than the whole-note last-write-wins
rule. Adding `verbs` on a phone and `irregular` on a laptop, both offline, should not lose one of
them; a set of strings has an obvious commutative merge and no reason to inherit the coarser rule.
Removal under set union needs its own answer, which is #9's to give.

## Requirements this places on downstream tickets

### [#9 — the review event log format](https://github.com/amin-bf/leitner/issues/9)

1. A review event references a card as a **`CardRef`**, per the canonical 18-byte encoding in §6.
   There is no standalone card identifier to record.
2. **Content edits do not appear in this log.** The log is reviews and scheduler configuration
   (ADR-0001 §6), nothing else.
3. **Replay ignores events whose `CardRef` the current content does not generate** (§7). This is not
   an error condition; it is normal operation, and it must not warn or discard.
4. Still open for #9: whole-note versus per-field last-write-wins on the mutable surface
   (per-field recommended); set-union for tags (§10); whether the log is ever compacted.
5. The clock-skew tension the map flags is **untouched** by this ADR and remains #9's to confront.

### [#13 — the deck export format](https://github.com/amin-bf/leitner/issues/13)

1. **A container able to hold binary files**, from the first version (§9).
2. **Carries the kind definitions its notes use** (§4), so the file explains itself.
3. Content and progress are separate stores (§7), so exporting content without progress is a choice
   of what to include, not a filtering pass.
4. Note ids are stable across export and import (§6); import deduplicates on them.

### [#10 — the deck model](https://github.com/amin-bf/leitner/issues/10)

1. **Notes** belong to decks; cards do not belong to decks independently of their note.
2. Tags cut across decks (§10) and are not a deck mechanism.

### [#11 — the review session prototype](https://github.com/amin-bf/leitner/issues/11)

1. The prompt/answer split of a card comes from its kind definition, and `shown-with` fields follow
   their anchor field to whichever side it lands on (§3).
2. Sibling cards from one note may fall due together, and are not spread (§1, ADR-0001 §7).

### Newly required, not previously owned

An **authoring/editing experience** ticket: the Markdown subset with live preview (§8), entering and
proofreading numbered blanks (§5), and the warning before an edit retires a card with history (§7).

## Glossary

**Moved.** These terms are now of record in [`content`](../../crates/core/src/content/CONTEXT.md), per
[ADR-0009 §6](0009-crate-and-workspace-layout.md), which fixed where contexts live. They
were marked provisional here precisely so this could happen: the `CONTEXT.md` is
authoritative, and this ADR keeps the reasoning behind them.

**Dormant card** moved to [`replay`](../../crates/core/src/replay/CONTEXT.md) instead: it names the join between content and the log, not a property of content.

## Consequences

- Review history is attached to content position, not to an object. This is what makes it survive
  offline generation on multiple devices, and equally what makes reordering a kind's `cards` list
  or renumbering cloze blanks silently destructive. Both are named as forbidden in §4 and §5.
- Cards need no creation, deletion or retirement events. The card set is a pure function of content.
- Two offline edits to one note lose one of them. Accepted; per-field merge narrows the window.
- A shape outside the kind set requires an application change. The string kind identifier keeps that
  additive.
- Decks are self-describing on export and lean in storage, because kind definitions are stored once
  and copied only into exports.
- Media and mathematical notation are both deferred with their re-entry costs written down, rather
  than left as unexamined gaps.

## Open items handed onward

| Item | Owner |
|---|---|
| Field-level versus note-level merge; tag set-union; compaction | [#9 — the review event log format](https://github.com/amin-bf/leitner/issues/9) |
| Export container format; what a "share without progress" export contains | [#13 — the deck export format](https://github.com/amin-bf/leitner/issues/13) |
| Whether a note may belong to more than one deck | [#10 — the deck model](https://github.com/amin-bf/leitner/issues/10) |
| Note id encoding in the physical store | [#12 — the local store](https://github.com/amin-bf/leitner/issues/12) |
| Authoring UI: preview, blank entry, destructive-edit warning | Newly surfaced |
| Audio on cards | Map fog |
| Mathematical notation in fields | Map fog |
| Removing or renaming a field on an existing kind | Not yet owned |
