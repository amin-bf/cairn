# ADR-0018: The card pane's ordering, and what a dormant entry is

- **Status**: Accepted
- **Date**: 2026-07-31
- **Resolves**: [Decide: card-pane ordering when dormant cards outrank live ones](https://github.com/amin-bf/leitner/issues/57)
- **Map**: [Map: local-first Leitner app spec](https://github.com/amin-bf/leitner/issues/1)
- **Related**: [ADR-0002 §4 §7](0002-the-card-model.md) (kind definitions, replay and dormancy),
  [ADR-0006](0006-the-review-session-experience.md) (how review draws a card),
  [ADR-0012 §1 §3 §5](0012-the-note-authoring-experience.md) (the card pane, blank numbering, the
  ambient dormancy warning), [ADR-0017 §1 §3 §4 §5](0017-card-slots.md) (slots, the high-bit
  partition, the golden slot list, the handoff this ADR discharges)

## Context

[ADR-0012 §5](0012-the-note-authoring-experience.md) holds a dormant card **in ordinal position** in
the authoring pane — *card 1, dormant card 2, card 4* — rather than appended after the live cards.
Round 1 of that prototype was read as proving position is the mechanism: appended last, the dormant
card fell below the fold and a counter in the header did all the warning.

[ADR-0017 §3](0017-card-slots.md) then partitioned cloze blanks above the high bit (`0x8000 | n`) and
recorded the consequence rather than settling it: a `vocab` note switched to `cloze` has dormant cards
at slots 2 and 3 and its live blank at 32769, so "in ordinal position" sorts **both dormant cards
above the live one**. For a pane whose job [ADR-0012 §1](0012-the-note-authoring-experience.md) defines
as *"what will I be asked"*, leading with two dormant cards is arguably wrong. Both ADRs handed it to
the visual design pass.

It is settled here instead, because it is not a styling question. It is a behavioural rule about what a
specified pane shows first, answerable without knowing a single colour, and §5's rule as written
produces the bad ordering silently.

**Two things were wrong with the question before it could be answered.**

**The defect is not cloze's doing.** Working the shipped registry's transitions out gives a second case
with no cloze in it: a `basic` note (slot 0) switched to `vocab` (slots 2 and 3) has its dormant card at
slot 0 and both live cards above it. The general shape is *any kind change where the old kind's slots
sort below the new kind's*, and the high-bit partition merely makes cloze the extreme instance. A fix
aimed at cloze would have left the defect standing.

**And the complaint assumed a rendering that was never specified.** "Leading with two dormant cards" is
only a problem if a dormant card costs a screen. `replay/CONTEXT.md` defines a dormant card as **the
absence of a generated card** — a `CardRef` with events in the log that the current content no longer
generates — so in the general case there is nothing to draw: a deleted cloze blank has no text, an
emptied field has no answer. What is always available is the **slot**.

## Decision

### 1. Ordinal position stands, sorting on the raw slot number

The card pane orders every entry, live and dormant alike, by its raw `u16` slot. No masking, no
grouping by dormancy, no partition of any kind. ADR-0012 §5's rule is confirmed rather than replaced.

What decided it is a case neither source ADR looked at — a `cloze` note whose blank 3 was deleted:

| note | pane, in raw slot order |
|---|---|
| `vocab` → `cloze` | *line* Term→Meaning · *line* Meaning→Term · **card** blank 1 |
| `basic` → `vocab` | *line* Front→Back · **card** Term→Meaning · **card** Meaning→Term |
| `cloze`, blank 3 deleted | **card** 1 · **card** 2 · *line* 3 · **card** 4 |
| `basic` → `basic-reverse` | **card** Front→Back · **card** Back→Front (nothing dormant) |

Row three puts the dormant entry **in its gap**, between blanks 2 and 4. That is
[ADR-0012 §3](0012-the-note-authoring-experience.md)'s *"gaps are therefore normal and are shown as
normal"* delivered by the ordering rule itself, with no sentence written to explain it — the same shape
as §1's `shown-with` demonstration, where the layout does the explaining. **Every partition rule
destroys it**, because grouping by dormancy moves the entry away from the gap that is the whole point
of showing it.

Rejected: **live cards first, dormant appended.** This is round 1's failure, and §2's compact rendering
does not rescue it — it puts the entry below the fold on a phone, where the pane is already behind a
`Write | Cards` toggle, and it breaks row three.

Rejected: **sorting on the masked value (`ordinal & 0x7FFF`)**, so blank 1 sorts as 1. It fixes row one
and is wrong twice over. It would interleave cloze blanks among fixed-arity slots — blank 1 sorting
*between* slot 0 and slot 2 — asserting an adjacency between two namespaces
[ADR-0017 §3](0017-card-slots.md) partitioned **precisely because they are not comparable**. And it only
ever fixes cloze, leaving `basic` → `vocab` exactly as it was.

**The mask is a name, never a sort key.** §3 uses it for the first and this section forbids it for the
second, and that split is the whole of this ADR's relationship with the high bit.

### 2. A dormant entry is a line, not a card

A live entry is a card, drawn the way review draws it
([ADR-0012 §1](0012-the-note-authoring-experience.md), [ADR-0006](0006-the-review-session-experience.md))
— prompt, separator, answer, history badge. A **dormant entry is a single line**: its name, the word
*dormant*, and its history. It is never a full card rendering, and never a greyed one.

Three grounds, in descending weight.

**Full fidelity is not available in general.** A dormant card is the absence of a generated card, so
the content that would be drawn is usually the content whose removal caused the dormancy. Rendering it
"when we can" would give the pane two entry heights whose selection depends on *which kind* of
dormancy the user caused — a card for a kind change that retained its fields, a line for a deleted
blank. The fold behaviour would then vary by edit.

**It does not weaken round 1's finding, because fidelity was never the variable round 1 tested.**
Variant B's dormant card was full-fidelity and still failed; it was below the fold. A line in position
is emphatically not the header counter that failed either, because it names the question and its
history where a counter reports a quantity.

**It dissolves the complaint rather than resolving it.** Two dormant lines above a live card do not
cost the pane its job. The live card is still the first card you see, and the ticket's objection —
that the pane leads with what you will *not* be asked — stops applying at the point where leading
costs two lines.

**Accepted cost, stated because it is a real loss**: where the content *does* survive a kind change,
you can no longer read what the dormant card used to ask. That is the right thing to lose. §1 defines
the pane's job as "what will I be asked", and a dormant card is precisely what you will not be asked;
its identity and its history are what the warning needs, and both are on the line.

**The word is *dormant*.** `replay/CONTEXT.md` rules out retired, deleted, orphaned and tombstoned,
each implying a stored lifecycle that deliberately does not exist. ADR-0012 §5 records that the
prototype's own copy broke this twice.

### 3. Naming a dormant entry makes the golden slot list a runtime artifact

The note is `cloze`; the dormant entries are slots 2 and 3, which `cloze` does not declare. So the
pane cannot get their names from the note's own kind. It looks them up across **every kind definition
the collection holds** — shipped, and acquired via [ADR-0008 §7](0008-the-deck-export-format.md).

That works, and it works only because of [ADR-0017 §1](0017-card-slots.md): one namespace, a slot
never reused for a different question, so whichever definition declares slot 2 gives the same answer
as any other. **A lookup keyed on a number that meant different things in different kinds would be
unanswerable**, which is worth stating because it is the first consumer of §1's global uniqueness
outside the transition safety it was argued for.

ADR-0017 §4 specified the `slot → (prompt, answer)` list as a **checked-in golden fixture** guarding
slot immutability. It is now also a **runtime lookup**. The same table serves both, which is the
reason this costs nothing.

Three cases, in order of precedence:

1. **Slot declared in a held definition** — name it by its **field roles**: *"Term → Meaning · dormant
   · 23 reviews kept"*. Roles rather than content, because the content is exactly what may be gone.
2. **High bit set** — a cloze blank, in no definition at all, since blank numbers are authored and
   unbounded ([ADR-0002 §5](0002-the-card-model.md)). Name it by the masked number: *"blank 3 ·
   dormant · 6 reviews kept"*.
3. **Neither** — name it by the bare slot: *"card 7 · dormant · 12 reviews kept"*. Reachable when a row
   was written by a build shipping a kind this one does not, because a log row carries a `CardRef` and
   no kind ([ADR-0004 §5](0004-the-review-event-log.md)).

**Case 3 is shown, never hidden**, and this is the load-bearing part of the section. The tempting
alternative is to omit what cannot be named, and omission is the header-counter failure taken to its
limit: an unnameable dormant card is still history attached to this note, and the entry exists to say
so. A bare slot number is honest about exactly what the build knows.

The word is **kept**, not lost. ADR-0012 §5's copy rule is that nothing is deleted, the reviews stay
in the log, and they reattach by themselves if the content returns.

### 4. The form-pane warning is the warning, on both platforms

Ordinal position cannot guarantee the entry is on screen. A `cloze` note with twenty blanks where
blank 18 is deleted puts that line eighteenth — below the fold on desktop as well as on a phone. This
is round 1's exact failure mode, and §1's rule does not prevent it; it prevented it only for slots
that happened to sort early.

So the constraint this decision inherited — *position is the mechanism, and any replacement must keep
the warning visible* — **is not satisfiable by any position rule**. Visibility under an ordering rule
is a property of the edit, not of the rule.

[ADR-0012 §5](0012-the-note-authoring-experience.md)'s second bullet already specifies the warning
that does the job, and already puts it on both platforms. Only its reason changes. §5 says the warning
appears in the form pane *because on a phone the card pane is behind a toggle*, and that "on desktop
it therefore appears twice, deliberately". The real reason is stronger and platform-independent: **the
card pane cannot guarantee its own entry is on screen**, so the form-pane warning is not redundancy
for the phone's benefit — it is the only warning that is always visible. Nothing is added. A rule that
was right for a partial reason is given its whole one.

**This re-reads round 1.** Its finding was recorded as *position is the mechanism*; what it showed is
that **a count is not a warning**. Variant B carried two defects at once — a counter in the header and
the card below the fold — and the ADR credited the repair to position alone.

The card-pane entry keeps the job §1 gives it, which is **demonstration**: it shows *which* card and
*how much* history, in the place the pane already puts that card.

Rejected: **a pinned indicator in the card pane's header.** It is the counter that failed, and it would
be a third thing speaking about one edit.

Rejected: **auto-scrolling the pane to a newly-dormant entry.** It needs "newly". §5 recomputes
dormancy from the draft every frame and holds no before-state, so there is no frame in which *just
became dormant* is a fact the pane possesses.

### 5. The layout jump is accepted, unreserved

Dormancy is recomputed every frame, so an entry that is a line at one keystroke is a card at the next:
filling an empty `Back` field grows a line into a card and pushes everything below it down. Under a
full-size greyed rendering this was a colour change in place; under §2 it is a resize. No height is
reserved and nothing is animated.

It happens only at **discrete boundaries** — a field crossing empty↔non-empty, a `{{n::…}}` being
closed, the kind dropdown changing — never continuously, because
[ADR-0012 §3](0012-the-note-authoring-experience.md)'s *"a half-typed `{{1::` stays literal"* already
removes the one case that would churn per keystroke. It is also **desktop-only in practice**: on a
phone you are in `Write` while typing, so the flip happens behind the toggle and the user meets the
result rather than the movement.

And the growth is the demonstration §1 asks for. Reserving card height for a dormant entry would
return exactly what §2 bought.

### 6. A pane with nothing live in it is its own state

Switch a reviewed `vocab` note to `cloze` and type nothing, and every entry is dormant. The pane's
answer to "what will I be asked" is *nothing*, and it says so: the dormant entries in slot order, plus
a plain statement that this note currently generates no cards.

It does **not** fall back to the empty-note state. A new note generates nothing *yet*; this note
generates nothing *and has history*, and the second is the one worth seeing. The distinction costs a
sentence and is reachable by an ordinary edit.

## Amendments to accepted ADRs

- **ADR-0012 §1** — the pane draws **live** entries the way review draws them. A dormant entry is a
  line (§2 above), so "drawn the way review draws them" is no longer true of every entry in the pane.
- **ADR-0012 §5** — its first bullet's "in ordinal position" is **confirmed and made precise**: the
  sort key is the raw slot number, and the dormant entry is a line rather than a card held in the
  stack. Its second bullet's **reason is corrected** — the form-pane warning is primary on both
  platforms because position cannot guarantee visibility, not because a phone hides the pane (§4). Its
  third bullet, the Undo copy, is unchanged.
- **ADR-0017 §4** — the golden `slot → (prompt, answer)` list is **also a runtime lookup**, not only a
  test fixture. Its consumer set grows by one, and §3 above is why.
- **ADR-0017 §5** — its handoff, *"handed to the visual design pass, rather than settled here"*, is
  **discharged**. The ordering was not a layout question after all; it was a rendering question wearing
  an ordering question's clothes.

## Consequences

- **No persisted artifact changes and no new state.** The log, `CardRef`, the fuzz seed and the
  `.ldeck` format are untouched, and nothing is stored to support any rule here — dormancy stays
  recomputed per frame, and §4 explicitly declines the one rule that would have needed a before-state.
- **The visual design pass loses an item and keeps a smaller one.** What a line *looks like* — its
  type, weight and spacing — is still that pass's. What it *says*, where it *sits* and when it appears
  are settled here.
- **The slot registry gains a second consumer, and that raises the price of an unnameable slot.** Under
  ADR-0017 §4 a slot missing from the table broke a test; it now also degrades a user-visible string to
  §3's case 3. Both point the same way — do not let the table fall behind the definitions — but the
  second failure is silent where the first is a red build.
- **A note with several dormant cards stays readable**, which was not true before: four dormant lines
  cost four lines, where four greyed cards cost a screen. This is what makes §1's decision to leave
  them interleaved affordable.
- **The pane now has three entry shapes** — card, dormant line, and §6's no-cards statement — where
  ADR-0012 §1 described one. That is the cost of the pane answering two questions at once, and §4
  keeps them from competing by moving the warning out.

## Open items handed onward

- **The visual design pass** ([ADR-0006 §10](0006-the-review-session-experience.md)) — the typography,
  weight and spacing of a dormant line, and how *dormant* reads against a live card without becoming a
  second warning.
