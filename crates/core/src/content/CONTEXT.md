# Content

The authored, mutable half of a collection: notes, the cards generated from them, and the decks that
ship them. Everything here is editable and settles by ADR-0004 §7's rule; nothing here is a log row.

This is the base context — `log`, `scheduling` and `replay` all depend on it, and it depends on none
of them.

**Bound by** [ADR-0002](../../../../docs/adr/0002-the-card-model.md) and
[ADR-0005](../../../../docs/adr/0005-the-deck-model.md), whose glossaries this file supersedes; also
amended by [ADR-0008](../../../../docs/adr/0008-the-deck-export-format.md), which widened §2 to admit
acquired kind definitions and §4's per-deck slot to hold authoring values, and by
[ADR-0011 §7](../../../../docs/adr/0011-new-card-rate-and-daily-limits.md), which adds **`position`**
to the note so authored order survives publication. Also bound by
[ADR-0017](../../../../docs/adr/0017-card-slots.md), which makes a card's ordinal a **slot** the kind
definition declares rather than its index in a list.

## Language

### Notes and cards

**Note**:
The unit of authored content: a kind, a set of named field values, and tags. Never scheduled, never
shown as a question.
_Avoid_: Fact, entry, item.

**Card**:
A question generated from a note by rule, and the unit that carries a schedule. Identified by
`CardRef`, never by an identifier of its own.

**CardRef**:
The pair `(note id, ordinal)` identifying a card. Encodes as exactly 18 bytes (ADR-0002 §6), which
`scheduling` uses as its fuzz seed — so the encoding is load-bearing beyond this context.

**Ordinal**:
The second half of a `CardRef` — the **slot** the card occupies. *Not* an index into anything
(ADR-0017 §1): reading it as a position in the kind's `cards` list is the defect the slot rule exists
to prevent.
_Avoid_: Index, card number.

**Slot**:
The number a kind definition assigns to one of its cards, drawn from **one namespace shared by every
kind**. `basic` and `basic-reverse` both declare slot 0 for Front→Back — deliberately, because it is
the same card, which is what lets a note gain its reverse direction without orphaning its history. A
slot is a card's identity: **never changed, never reused for a different question, and list order
carries nothing.** Cloze is partitioned off by the high bit — blank `n` is `0x8000 | n`, fixed-arity
slots are `0x0000–0x7FFF` — so the two numbering schemes cannot collide even though a note may move
between them.
_Avoid_: Ordinal position, card index, template index.

**Sibling**:
Another card generated from the same note. **At most one card per note is introduced per day**
(ADR-0011 §8) — siblings shown in one session measure ninety-second recall rather than the separate
skills they exist to schedule separately.

**Position**:
The integer fixing a note's place in authored order. From a local high-water counter on creation,
from the `notes.jsonl` line index on import, ties broken by note id. **Not dense and not unique** —
it only has to sort. Two things read it: new cards are introduced in `(position, ordinal)` order, and
`export` emits notes in it (ADR-0011 §7).
_Avoid_: Index, sequence number (which means `log`'s per-writer counter), sort key.

### Kinds and fields

**Kind**:
The closed-set identifier declaring a note's fields and how its cards are generated. A permanent
string, never reused. Defined in code, never authored by a user.
_Avoid_: Note type, template, model.

**Kind definition**:
The read-only data describing a kind's fields and cards. Shipped with the application and carried in
exports, so a deck file explains itself.

**Acquired kind definition**:
A kind definition the collection holds because it arrived in an imported file, for a kind this build
does not ship. Read-only, and never displaces a shipped definition. A widening of the closed set's
*provenance*, not of its editability — see [`export`](../../../export/src/CONTEXT.md), which owns the
import side of it.

**Field**:
A named text value on a note, either **asked** or **shown-with**. A plain string of restricted
Markdown — no media, no mathematical notation (ADR-0002 §8, §9).

**Asked field**:
A field that may serve as a card's prompt or answer.

**Shown-with field**:
A field never asked, rendering wherever its anchor field renders. This is what makes a pronunciation
follow its term to whichever side of the card the term lands on.

**Blank**:
A numbered `{{n::text}}` region in a `cloze` note's text. Each distinct number generates one card.
Auto-renumbering blanks is forbidden — it silently retypes review history onto the wrong card.

### Decks

**Deck**:
The unit of ownership and export: `{ id, name }`, plus the notes whose `deck` reference names it.
Never a filing structure — personal organisation is tags' job, and `::` in a name carries no
structural meaning.
_Avoid_: Folder, category, collection (which means the whole body of a user's data).

**Deck id**:
A UUIDv4, minted once at creation and preserved through export and import. What lets an import be
recognised as an update to the same deck rather than a new one.

**Unfiled**:
The state of a note whose `deck` reference names no deck the collection currently holds. Fully
reviewable, never dropped.

**Personal deck preference**:
A per-deck setting on the mutable surface, keyed by deck id, that never exports and never appears in
the review log. Distinguished from deck content by one test: does it travel with the deck?
**The slot is deliberately still empty**: ADR-0011 §6 declined a per-deck new-card rate, because
with one collection-wide queue the real daily obligation becomes a sum shown on no screen.

### The two halves

**Note store**:
The mutable half of a collection — content, tags, kind, deck references. Last edit wins, by ADR-0004
§7's settling rule. Contrast **review log**, which is `log`'s term for the append-only half.

## Rules that are easy to break silently

- **Never change a slot number, and never reuse one for a different question** — and **never
  auto-renumber cloze blanks**. Both silently retype existing review history onto the wrong card, and
  nothing downstream can detect it (ADR-0017 §4, ADR-0002 §5). Reordering a kind's `cards` list is now
  *harmless*, because the slot travels with the entry; **reading the list index instead of the slot
  field** is what replaced it as the defect.
- **Slot uniqueness and slot immutability are tests, not conventions** (ADR-0017 §4) — a uniqueness
  check across the shipped definitions, and a golden `slot → (prompt, answer)` list. They are the only
  thing making the rule above enforceable, and they need no database, no window and no handset.
- **An acquired kind definition's slots are never validated**, and that is safe only because a note
  can never be switched *into* an acquired kind: a card is `(note, slot)`, so a stranger's slot 0 and
  ours are different cards on different notes. Adding acquired kinds to the authoring dropdown makes
  the importer owe a check it does not have (ADR-0017 §6).
- **A card is never minted, only derived.** Two offline devices must reach the same `CardRef`
  without communicating; a minted id splits one blank's history across two irreconcilable
  identities.
- **Card retirement does not exist.** The card set is computed from current content; events with no
  matching card are retained and simply not projected. See **dormant card** in `replay`.
