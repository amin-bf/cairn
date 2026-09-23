# ADR-0041: The file surface — the preview, its gate, the file list, and a picture beside a word

- **Status**: Accepted
- **Date**: 2026-09-23
- **Resolves**: [Design Pass: Draw the File Surface — the List and the Import Preview](https://github.com/amin-bf/cairn/issues/167)
- **Map**: [Map: The Cairn Design Pass](https://github.com/amin-bf/cairn/issues/121)
- **Related**: [ADR-0022](0022-the-import-preview-and-export-report.md) (**what** the preview and the
  list state and when — this ADR decides only how they are drawn, which §3 there handed onward),
  [ADR-0039](0039-the-list-row.md) (the row this list uses, and **the icon rule §4 here settles**,
  which 0039 recorded as belonging to *"whichever next has cause to amend it with two builds behind
  it"*), [ADR-0038 §1](0038-the-mark-and-the-icon-rule.md) (icons are glyphs; the set gains three),
  [ADR-0034 §2 §5](0034-the-controls.md) (the primary and the text action, which the gate is),
  [ADR-0035 §1](0035-the-vertical-anchor.md) and [ADR-0039 §8](0039-the-list-row.md) (the reach
  line, reached by pinning outside the scroll), [ADR-0033 §5](0033-the-card.md),
  [ADR-0039 §4](0039-the-list-row.md) and [ADR-0040 §6](0040-the-note-editor.md) (*content, not
  furniture* — the direction rule §2 applies to a preview), [ADR-0024 §1 §3](0024-identifying-a-written-file.md)
  (a file is identified by its bytes, and the list is only the files this application wrote)

## Context

ADR-0022 decided in full what an import preview states and what the file list says, and handed the
drawing to the visual design pass. [#88](https://github.com/amin-bf/cairn/issues/88) landed the
capability and deferred the screen; what stood in for it was two blocks at the bottom of Settings,
each documented in-source as *"Temporary, and not a specified feature"*.
[The File Surface](https://github.com/amin-bf/cairn/issues/151) settled where the surface lives — the
list on Settings, the preview with no destination at all — and
[A File Bench](https://github.com/amin-bf/cairn/issues/166) made the update path photographable, so
every destructive line ADR-0022 exists for could be judged on one screen for the first time.

The bench also found the defect this ADR is organised around. The specimen joined a deck's name and
the application's words into one string — `"{name} — {path}"`, and on the list `"{name} —
{description}"` — so a file named `فارسی and 2 more.cdeck` drew as *"and 2 more.cdeck — deck فارسی"*,
and its deck line as *"updating a deck you already have — فارسی"*. That is correct bidirectional
ordering of a string that should never have been one paragraph: **the file's claims and the
application's statements about them are two statements**, and ADR-0022 §7 had already separated the
header from the effects for a related reason.

The questions were put as wireframes at the application's real metrics (`AGENTS.md`, *Showing a design
question*), answered by the repository owner, and then built and photographed, because two of the
answers — how Persian orders inside egui, and whether the pictures read at their size — are ones only
the renderer can give.

## Decision

### 1. The file's own claims sit above a hairline, in weak text

> The heading is the file's name as it arrived. Under it, the header — *by ⟨author⟩ · ⟨licence⟩*, then
> the description — in `weak_text_color`, then `frame::rule` with `gap(2)` either side.

The hairline is already the application's boundary between chrome and content (ADR-0039 §2), so the
line ADR-0022 §7 draws the whole screen along costs the system no new value. **Weak text makes a
stranger's claims about their own file quieter than the application's statements about yours**,
which is the ranking the screen exists to express.

**Rejected: the header in a sunken well.** It reads as a quotation, which is appealing — but the well's
fill is the card's, and ADR-0033 keeps that surface for cards. A second meaning for the one fill that
says *this is a card* is how a palette stops meaning anything.

A file with no header draws **no rule**: a hairline under nothing divides nothing, and §7's *absent
fields are absent* would otherwise leave a line standing where the header was not. A share that
carried no name headed as *A deck file* — ADR-0024 §1 identifies by the bytes, never the name.

### 2. A deck's name is its own line, in its own direction; the effects keep the interface's edge

> Each deck is its **name** in the bold face, the **path** it takes — *updating a deck you already
> have*, *new deck*, *a deck you already have* — as a small weak caption under it, and its **effects**
> indented one `gap(2)` beneath. The name and its caption align to **the name's** direction. The
> effects do not.

The name is content and the caption is its footnote, which is the list row's rule (ADR-0039 §4) and
the editor's label rule (ADR-0040 §6): *content, not furniture*. The effects are the application's own
sentences about your collection, so a Persian deck's *3 new notes, 5 already yours* stays on the left
under a name on the right. This is the first screen where both edges are in use at once, and it was
chosen over keeping every name on the left precisely because the left-edge variant would be the first
place in the application where a right-to-left name does not sit on its own edge.

**A name that has to sit inside a sentence is isolated.** *4 notes moving in from ⟨X⟩*, *Renaming
your ⟨X⟩ to ⟨Y⟩*, *⟨X⟩ will be left empty* — each ⟨X⟩ goes in through `bidi::isolate`, which wraps it in
FIRST STRONG ISOLATE and POP DIRECTIONAL ISOLATE so it orders as a run of its own and cannot absorb the
neutrals and digits beside it. `bidi::job` orders with the marks and then drops them, because no face
draws them.

**And a stranger may not bring marks of their own.** `cairn_export::plain` now drops every bidi
formatting character — the marks, the embeddings and overrides, the isolates — from a string arriving
in a file. They are format characters rather than controls, so `is_control` let them through, and
they are the one kind of *styling* plain text can still carry: a name ending in a pop-isolate would
close the preview's isolate early, and an override after it would draw *"3 of your notes will be
deleted"* backwards — the application's words, reversed by a file, on the one screen that renders a
stranger's strings before the user has agreed to anything. The joiners stay; Persian spells with them.

### 3. The gate: *Import* and *Cancel*, or a lone *Close* — pinned, and the only way out

> The gate lives in a band pinned outside the scroll with its control on the reach line
> (ADR-0039 §8). A plan that changes something offers ***Import*** as the primary and ***Cancel*** as
> a text action beside it. A refusal, and a plan that changes nothing, offer a lone ***Close*** as the
> primary.

The preview is a screen with no card, so its way forward is a primary (ADR-0034 §2), and *Cancel* is
the frameless alternative that is only legible as a control because a primary stands beside it
(§5).

**The no-op gets one dismissal, not the pair.** ADR-0022 §4 says re-opening an unchanged file *"costs
one dismissal"*; an *Import* that writes nothing is a second button whose only effect is to leave the
screen, which *Close* already does. Refusals take the same single control for the same reason.

**The nav row steps aside while a file is held.** The preview *takes the screen* (ADR-0022 §6): its
only exits are the gate's, so a file can never be left half-decided behind a destination the person
wandered to, and the one screen that renders a stranger's strings is the one screen with nothing else
on it. **It opens at its top** (`Band::scroll_to_top`) however far down Settings the row was, and so
does the note list it hands to — every screen shares one scroll area, and without this a row opened
halfway down Settings opened its preview with the file's name scrolled away.

**After *Import*, nothing is said** (ADR-0022 §5). The application lands on the note list filtered to
the deck the file carried, or unfiltered for several, with an open editor settled first so the field
being typed in is not lost (ADR-0021 §7). Only a store failure speaks, on the preview, where pressing
*Import* again is the recovery.

### 4. The icon rule's third test: a picture beside its word

> A file row carries a picture of its kind — **deck**, **archive**, **unreadable** — at the row's
> leading edge, **beside** the words, never instead of them.

[#149](https://github.com/amin-bf/cairn/issues/149)'s rule was decided with no build behind it, and
the two screens it named answered differently: the note row's glyphs stood alone (ADR-0039 §1), the
leech row kept its word. ADR-0039 reconciled them — *repetition pays for the learning where the symbol
already exists; where it would have to be invented, the word stays* — and recorded that neither ticket
could declare that alone. This is the third build, and it settles the rule with one addition:

> **A picture stands alone only where the symbol already exists. Where it would have to be invented,
> the word stays — and the picture may accompany it.**

A bin and a double-headed arrow are conventions a reader arrives holding. Nobody arrives knowing a
picture for *a deck file*, so it cannot carry that meaning; beside the word it does a different job,
letting the eye sort the list into kinds before reading it. So the three rows read: note row (a
conventional symbol) **glyph alone, the word as hover text**; leech row (*Suspend*, no symbol) **the
word**; file row (no symbol, and a *state* rather than an action) **the picture with the word**.

The picture is drawn at the **heading** size, not body: it stands beside two lines, the name and its
caption, and at body it was eight pixels of ink against thirty-two pixels of text — a speck, not a
picture. It takes the set's square advance (ADR-0039 §9), so every row's name starts at the same x.
`deck` is the design project's own drawing; `archive` and `unreadable` are drawn here, because the
set's sixteen had none — the same reason `move` was.

### 5. The file list: a block that reads itself and says what it is

> Label **Files**. Line: *The files this app wrote. Open one to see what importing it would do.*
> Empty: *This app hasn't written any files yet.* Each row is the file's name, and a caption from its
> **manifest**: *deck · 18 notes · 3 retractions*, *deck · 3 decks, 19 notes*,
> *collection archive · 1 September 2026 · 25 notes, 50 reviews*, *unreadable*.

The words say what the list **is**, never what is missing (ADR-0024 §3), and the empty state cannot
point at an export screen because there is none (the map's *Out of scope*). The list **reads itself**
each time Settings opens; a list that has to be asked for is a development control.

**The caption is ADR-0022 §11's, which the specimen had cut down to the profile alone.** It comes from
`cairn_export::summarise`, which reads the `mimetype` member and `manifest.json` and inflates no
payload — the property §11 says is where the container's zip shape pays for itself. These are the
file's own counts, not its effects: a file whose notes you mostly hold still says *1,240 notes* here,
which is right for a list that answers *which file is this?* and would be wrong on a preview. Counts
are grouped (*1,202*) everywhere on this surface.

## Consequences

- The import preview and the file list are specified screens; both development specimens are gone.
  The hand-off specimen stays, because the export half is out of scope.
- **Restore has no surface anywhere**, and that is recorded rather than absorbed. `restore_preview` and
  the store's `ingest` exist, the list shows archives, and ADR-0022 §11 even gives them a date so three
  can be told apart — but nothing in the application reads one, so opening an archive reaches the deck
  import's *"This is a collection archive, not a deck file."* The application also never *writes* an
  archive; only the bench does. It leaves the map as a named follow-on beside the export screen.
- **A four-digit count has still not been photographed.** Grouping is pinned by
  `counts_are_grouped_by_thousands`, but the bench's counts are single digits, so how a long count
  wraps on a handset line is unseen.
- The icon face carries six glyphs. `scripts/build-icon-face.py` learned to close a subpath, which the
  design project's `deck` needed and nothing before it had.
