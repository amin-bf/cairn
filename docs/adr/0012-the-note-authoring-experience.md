# ADR-0012: The note authoring and editing experience

- **Status**: Accepted
- **Date**: 2026-07-31
- **Resolves**: [Prototype: the note authoring and editing experience](https://github.com/amin-bf/leitner/issues/28)
- **Map**: [Map: local-first Leitner app spec](https://github.com/amin-bf/leitner/issues/1)
- **Evidence**: four egui/eframe variants at tag
  [`prototypes/issue-28`](https://github.com/amin-bf/leitner/tree/prototypes/issue-28)
  (`prototypes/note-authoring-28/`), judged live by the repo owner on desktop across two rounds.
- **Related**: [ADR-0002](0002-the-card-model.md) (notes, cards, kinds, blanks — this ADR **amends
  it**), [ADR-0003](0003-client-stack.md) (egui/eframe, the bidi helper, the font rules),
  [ADR-0006](0006-the-review-session-experience.md) (the review surface these previews imitate)

## Context

ADR-0002 settled what a note *is* and deliberately left how you write one unowned. The ticket asked
six questions: what a live Markdown preview looks like where there is no room for two panes; how a
user enters and proofreads numbered blanks; what the UI offers *instead* of tidying blank numbers,
given §5 forbids renumbering; what the destructive-edit warning says; whether a note's kind can be
changed later; and whether the editor makes `shown-with` fields visibly never-asked.

Prose could not settle it, so it went through `/prototype` in two rounds.

**Round 1** built three structurally incompatible editors, each disagreeing on every axis rather
than on styling: **A** put a rendered-Markdown preview in a second pane, folded into a
`Write | Preview` toggle on a phone, and warned in a modal at save; **B** made the preview *be the
cards the note generates*, in one scrolling column, with a dormant card left greyed in the stack;
**C** had no preview pane at all, rendering each field beneath its own input with the warning
firing at the keystroke.

The repo owner did not pick one. As in ADR-0006, the verdict was a **graft** — A's split view and
A's kind dropdown with B's card visuals — built as variant **D** and judged in round 2. What
follows is D plus the corrections made while driving it.

## Decision

### 1. Two panes: a form, and the cards the note generates

The editor is a form on one side and, on the other, **the cards this note currently generates**,
drawn the way review draws them (ADR-0006) — prompt, separator, answer, with a history badge.

> **Amended by [ADR-0018 §2](0018-the-card-pane-ordering.md): that is true of *live* entries only.**
> A dormant entry is a **single line** — its name, the word *dormant*, its history — never a card and
> never a greyed card, because a dormant card is the *absence* of a generated card and so usually has
> nothing left to draw. The pane therefore holds three entry shapes rather than one: a card, a dormant
> line, and ADR-0018 §6's statement for a note that currently generates none.

The pane is not a rendering of the *fields*. That distinction is the whole result of round 1:
a field-rendering preview answers "did I get the markup right", which matters only while typing
markup, whereas the card preview answers "what will I be asked", which is the question a note
exists to settle. It is also the only layout in which ADR-0002 §3's `shown-with` rule explains
itself — the pronunciation is *seen* moving from prompt to answer between card 0 and card 1, with
no sentence written to explain it.

**On a phone the two panes become a `Write | Cards` toggle**, which is ADR-0002 §8's "preview
beside the input" honoured by serialising rather than dropping it. §8's requirement is satisfied by
the card pane: a card is rendered Markdown.

### 2. The kind is a dropdown, chosen at creation and changeable afterwards

A closed set of four (§2) is small enough to show as chips and was prototyped both ways; the
dropdown won on the plain ground that the kind is chosen once and then stops being interesting,
while chips spend a permanent row on it.

**Changing kind later is permitted, and is not a special mechanism** — see §6 for the hazard it
carries, which is the sharpest finding in this ADR.

> **Extended by [ADR-0021 §9](0021-note-ordering-saving-and-the-note-list.md): there is a second
> dropdown beside it, for the note's *deck*, with *create a new deck* available from it.** This ADR
> specified the kind dropdown and was silent on deck, and no other ADR said where a note's `deck`
> reference is set — so between them nothing did. Creation belongs on the dropdown because the moment
> you need a deck that does not exist is while filing the note that wants it, and
> [ADR-0005 §8](0005-the-deck-model.md) forbids ever auto-creating one. Declining costs nothing:
> ADR-0005 §7 makes an absent reference legal, and such a note is unfiled and still reviewable.

> **Amended by [ADR-0017 §6](0017-card-slots.md): the dropdown lists the shipped kinds, plus the
> note's own current kind when that kind was *acquired*** ([ADR-0008 §7](0008-the-deck-export-format.md)).
> A note of an imported kind therefore shows its own kind, can be switched away from it, and can be
> switched back — so reversibility survives — while **no note can ever be switched *into* a kind whose
> slot namespace this codebase did not mint**. That is what keeps a stranger's kind definition unable
> to collide with ours, and is why the importer needs no slot validation.
>
> §6's hazard is **closed** by the same ADR; a kind change is now an ordinary edit that makes cards
> dormant, and the sentence above is true without qualification.

### 3. Blanks: created from a selection, numbered by rule, never tidied

A blank is made by selecting text and invoking *Blank it*, which wraps the selection as
`{{n::…}}`. The number is **one above the highest ever used in that field**, never the lowest free
one.

This is not a convenience choice, it is §5's renumbering prohibition applied one step earlier.
Filling a gap left by a deleted blank hands the new blank the *deleted card's identity*, so its
reviews reattach to different content — precisely the damage auto-renumbering does, arriving one
edit later instead. **Gaps are therefore normal and are shown as normal**; the editor says so
rather than closing them.

**A half-typed `{{1::` stays literal.** Live preview means the parser sees every keystroke, and an
inferred number is an invented identity.

Blanks are proofread from a **list, one line per blank**: the number, the text it hides, how many
places it hides, and its review history in words. The raw syntax is unreadable at a glance and the
card pane shows one card at a time, so neither answers "check the set". A compact chip row was
tried and rejected as unreadable: `blanks 1 · 5r 2` parses as a *count*, and its inner separator
carried the same weight as the gap between entries.

### 4. `shown-with` is stated where it is typed

A passenger field's label carries `· shown with Term, never asked`. The card pane then demonstrates
it. Label plus demonstration was better than either alone: the label answers "why did filling this
in not make a third card", and the pane answers "then where does it go".

### 5. The destructive-edit warning is ambient and continuous, never modal

Dormancy is **recomputed from the draft every frame**, so the warning is a property of the content
rather than a check at save time — which is what §7 actually describes. A modal at save was
variant A's position and lost: by the time you press Save the edit is already made, and a dialog
asks for a decision about work you have stopped thinking about.

Three rules:

- **The dormant card stays in the card pane, in ordinal position** — card 1, dormant card 2,
  card 4 — not appended after the live cards. Round 1 proved position matters: in B the dormant
  card was last in a scrolling column and fell below the fold, leaving a counter in the header to
  do all the warning.

> **Widened by [ADR-0017 §5](0017-card-slots.md): this warning now absorbs the §6 kind change too,
> needing nothing added to it.** A kind change makes cards dormant and can no longer do anything
> worse, so `vocab` → `cloze` simply shows two dormant cards instead of one, and the copy below is
> true unchanged — switching back genuinely restores them.
>
> **One layout question is handed to the visual design pass**: under ADR-0017 §3's partition a
> converted note's dormant cards sit at slots 2–3 while its live blank sits at 32769, so "in ordinal
> position" sorts **both dormant cards above the live one**. For a pane whose job §1 defines as *"what
> will I be asked"*, leading with two dormant cards is arguably wrong. Recorded so the rule does not
> quietly produce it unnoticed.
>
> **Settled by [ADR-0018](0018-the-card-pane-ordering.md), which kept this rule.** Ordinal position
> stands, on the **raw slot number** — no masking, no grouping by dormancy. The defect was never
> cloze's (a `basic` note switched to `vocab` leads with a dormant card too, with no cloze in sight),
> and it was never an ordering defect: it assumed a full-size dormant card, which §2 above never
> specified. As a **line** (ADR-0018 §2), leading costs two lines rather than two screens. What kept
> ordinal position is a case neither ADR looked at — a deleted cloze blank shows its dormant entry
> **in its gap**, which is §3's "gaps are shown as normal" delivered by the ordering rule itself, and
> every partition rule destroys it.
- **The warning also appears in the form pane**, because on a phone the card pane is behind a
  toggle and an ambient warning that lives only there is invisible exactly when it matters. On
  desktop it therefore appears twice, deliberately.

> **Reason corrected by [ADR-0018 §4](0018-the-card-pane-ordering.md); the rule is unchanged.** The
> form-pane warning is **primary on both platforms**, and not as redundancy for the phone's benefit:
> **ordinal position cannot guarantee the card-pane entry is on screen at all.** A twenty-blank cloze
> note losing blank 18 puts that entry eighteenth — below the fold on desktop too. So the constraint
> this ADR set, *position is the mechanism*, is not satisfiable by any position rule; visibility is a
> property of the edit. The card-pane entry **demonstrates**, the form-pane warning **warns**.
>
> This also re-reads round 1. Its finding was recorded as *position is the mechanism*; what it showed
> is that **a count is not a warning** — variant B carried two defects at once, a counter in the header
> *and* the card below the fold, and the repair was credited to position alone.
- **It offers Undo**, and the copy says what is true: nothing is deleted, the reviews stay in the
  log, and they reattach by themselves if the content returns.

**The word is *dormant*.** `replay/CONTEXT.md` rules out retired, deleted, orphaned and tombstoned
because each implies a stored lifecycle that deliberately does not exist. The prototype's own copy
broke this twice before it was caught, which is how the wrong word reaches a spec.

### 6. Changing a note's kind can reattach history to an unrelated card — and this ADR does not fix it

> **Closed by [ADR-0017](0017-card-slots.md), which took neither of the two options below.** Both
> treat the ordinal's *meaning* as given and argue about the identity's *encoding*; the defect was in
> the rule that **assigns** ordinals. A card's ordinal is now a **slot declared in the kind
> definition**, drawn from one collection-wide namespace, with cloze partitioned into `0x8000 | n` —
> so `vocab`'s cards and a cloze blank can never share a number, and those reviews go correctly
> dormant. **`CardRef` keeps its 18-byte encoding and gains no discriminator**, because option 2 turns
> out to be actively wrong rather than merely expensive: making identity kind-scoped orphans the
> history of a `basic` note gaining its reverse direction — the most likely kind change there is, and
> one where reattachment is *correct* — under the same silence. The interim warning this section
> mandates is **retired** into §5's ambient dormancy warning.

**This is the finding that outranks everything else here, and it is a gap in ADR-0002 rather than
in the editor.**

§6 identifies a card as `CardRef { note, ordinal }`, where the ordinal is an index into the kind's
`cards` list for fixed-arity kinds and the **authored blank number** for cloze. The kind is not
part of the identity. So switching a `vocab` note to `cloze` puts blank 1 at ordinal 1 — the slot
the `Meaning → Term` card occupied — and §7's replay rule projects that card's reviews onto a blank
the user has never seen. Nothing marks it dormant, because ordinal 1 *is* still generated; it is
simply generated by a different question. The scheduler then believes a new blank carries five
reviews of history and a stability to match.

§7 offers *"the same mechanism absorbs a note changing kind"* as a prize. This is its other face:
absorbing is the wrong behaviour when the two cards ask different things.

It was found by driving the editor — reviews appeared on a blank that could not have earned them —
and is pinned by
`model::tests::changing_kind_reattaches_history_to_a_semantically_different_card` at the evidence
tag.

**It cannot be fixed in the authoring UI**, which is why it is recorded rather than solved: content
edits are deliberately not log events (§7), so replay has no way to learn what kind a note used to
be. Two options exist and both are ADR-0002's to take:

1. **Accept it and warn at the kind change** — the editor already knows the before and after, even
   though replay never will. Cheap, and consistent with §5's position that the authoring UI is the
   only place such warnings can happen.
2. **Put a kind discriminator into `CardRef`** — correct, and expensive: it reaches §6's 18-byte
   canonical encoding, ADR-0001 §7's fuzz seed, every log row and the export format.

**Until that is decided, this ADR requires option 1's warning**: changing kind must state which
ordinals already carry history, because that is the set at risk.

### 7. Editing rules the prototype had to discover

These are not preferences. Each was a defect found by driving the editor, and each binds the real
implementation.

- **Enter in a single-line field does nothing**, and the field keeps focus. egui treats Enter in a
  `TextEdit::singleline` as a submit and surrenders focus, which leaves the caret nowhere.
  Advancing to the next field was tried and rejected: a note editor is not a wizard.

> **Widened by [ADR-0021 §8](0021-note-ordering-saving-and-the-note-list.md) to the *last* field,
> which §9 below left unowned — the rule holds without exception.** Under ADR-0021 §7's autosave there
> is nothing for Enter to commit, so the only meaning left would be *"and now give me a fresh note"* —
> a navigation act, and it must not be bound to Enter for two reasons. **"The last field" is a
> property of the kind definition, which is *data***, so a kind gaining a field would silently change
> what a key does, with no code change and nothing failing — and
> [ADR-0008 §7](0008-the-deck-export-format.md) lets a note carry an **acquired** kind, putting that
> in a stranger's hands. And the rule could not be uniform anyway: `cloze`'s field is multiline, where
> Enter must insert a newline, so the carve-out would be invisible to a user pressing Enter in the
> last field of two different notes. The rhythm instead gets a **New note** action carrying kind and
> deck forward, with one modifier-chord accelerator that can never collide with a field's own Enter.
- **Single-line fields must be `singleline`**, or Enter inserts a newline into a `Term` and long
  values wrap inside a one-row box.
- **Each line aligns by its own first strong character** — the `dir="auto"` rule. A Persian term
  aligns right; a Latin pronunciation beneath it aligns left. Giving a card one shared edge taken
  from the asked field was tried and rejected: it holds the block together but pushes Latin text
  rightwards, where it reads as misplaced.
- **RTL alignment must not be done with `LayoutJob::halign`.** `halign = Max` produces a galley
  spanning negative x, so a widget allocating from it draws the text off its own left edge — which
  clipped the last character in every RTL input. Measure the galley and pad in front of it.

### 8. The shipped font set is part of the spec, not a packaging detail

ADR-0002 §8 puts `**bold**` in the Markdown subset and §9 defers audio on the grounds that the
motivating case *"is already solved as text"* by a written `Pronunciation` field. Both claims
depend on fonts the app does not currently ship.

- **The bundled faces cannot draw IPA.** egui ships Hack and Ubuntu-Light; neither covers the IPA
  extensions, so `deːɐ̯ hʊnt` renders as `de□ □ h□nt`. A `Pronunciation` field the app cannot draw
  does not solve anything.
- **Bold must be a face, not a colour.** egui bundles no bold face, epaint has no synthetic
  emboldening, and `RichText::strong` answers emphasis by brightening — invisible against a body
  colour that is already near-white. A real bold face in its own font family measured 26% wider
  than the body face; a colour shift measured nothing.

**So the app owes, per writing system, a face with IPA coverage and a bold cut of it**, registered
in every family it uses (ADR-0003 §4). A new named font family is **not referenceable on the frame
that registers it** — `set_fonts` applies at the start of the next pass, and drawing into an
unbound family aborts — so the first frame must render nothing, which extends ADR-0003 §7's
one-frame deferral to a second reason.

### 9. What this ADR does *not* settle

- **Visual design.** The palette, spacing and typography are scaffolding inherited from ADR-0006
  §10, which already deferred a look-and-feel pass. Nothing here revisits it.
- **The phone layout under a soft keyboard.** The prototype's phone-width toggle fakes the width
  but not a keyboard taking half the screen, which is what decides whether a live preview survives
  while typing. Judged on desktop only; see Consequences.
- **Saving semantics** — autosave versus explicit save, and what Enter means on the last field.
- **Editing a note mid-review**, and where authoring is entered from.

> **The last two are closed by [ADR-0021](0021-note-ordering-saving-and-the-note-list.md).**
>
> **Saving is automatic, per field** (§7) — and the decisive argument is *this ADR's own §5*. Making
> the destructive-edit warning ambient *"rather than a check at save time"* already spent the only
> decision a save could have carried, leaving a control that commits bytes and asks nothing. Two
> further grounds: this ADR's *Consequences* note that *"the editor holds a draft the store has not
> seen"* describes the one piece of state in the whole design a kill can lose, against
> [ADR-0006 §2](0006-the-review-session-experience.md)'s proof that nothing else does; and on Android
> an app is **frozen, not slowed**, so under explicit save putting the phone down mid-note is the
> standard way to lose work, silently. It also makes §5's Undo copy **literally** true rather than
> approximately: undo becomes an ordinary edit writing the old value back with a fresh stamp.
>
> **Where authoring is entered from** (§5, §6) turned out to need a screen that did not exist —
> **no browse surface is specified anywhere in twenty ADRs**, which is why
> [ADR-0010 §7](0010-leeches.md) could say *"edit … already exists"* about a door nobody had built.
> ADR-0021 specifies a **note list** and makes this one editor with four entrances. **A note is
> editable mid-review**, and **entering the editor counts as a reveal** — without which §4 of
> ADR-0006 is quietly false, since the editor shows the back.

## Amendments to accepted ADRs

- **ADR-0002 §7** — its claim that the replay mechanism "absorbs a note changing kind" is amended
  by §6 above: it absorbs it, and in doing so can reattach history to a semantically different
  card. §7's list of consequences to be honest about gains this one. *(Since **discharged by
  [ADR-0017](0017-card-slots.md)** — with disjoint slots the claim is true as originally written.)*
- **ADR-0002 §8** — the restricted Markdown subset is amended by §8 above: `**bold**` obliges the
  app to ship a bold face, since it cannot be rendered by any other means.
- **ADR-0002 §9** — its argument for deferring audio is amended by §8 above: it rests on a text
  `Pronunciation` field, which requires a face with IPA coverage.

*(§1 and §5 are in turn amended by [ADR-0018](0018-the-card-pane-ordering.md) — a dormant entry is a
line rather than a card, and the form-pane warning is primary on both platforms. Both amendments are
recorded inline above.)*

*(§2, §7 and §9 are amended by [ADR-0021](0021-note-ordering-saving-and-the-note-list.md) — the
editor gains a deck dropdown, the Enter rule widens to the last field, and two of §9's four unsettled
items close. All three are recorded inline above.)*

## Consequences

- **The authoring surface is the only place two ADR-0002 hazards can be caught.** §5's blank
  deletion and §6's kind change both cause silent history reattachment that nothing downstream can
  detect. This ADR makes both loud, and neither can be moved elsewhere later.
- **The card pane needs card generation available while editing an uncommitted draft**, not only
  for stored notes. `leitner-core` generates cards from content by rule with no dependencies
  (ADR-0009 §1), so this costs nothing — but it does mean the editor holds a draft the store has
  not seen.
- **Font bytes grow the binary.** Two faces per writing system, and ADR-0003 already records 19 MB
  for CJK as the practical bar. This is now a spec requirement rather than a choice.
- **Non-Latin authoring is desktop-only**, unchanged from the map: winit's Android backend has no
  IME path, so the phone can author Latin text and review anything.

## Open items handed onward

- ~~**`CardRef` and the kind discriminator** (§6)~~ — **discharged by
  [ADR-0017](0017-card-slots.md)**: no discriminator, and the ordinal becomes an assigned slot instead.
- **A soft-keyboard pass on the handset** (§9) — the one question desktop cannot answer. Now owned by
  [Prototype: the authoring screen under a soft keyboard](https://github.com/amin-bf/leitner/issues/67).
- ~~**Saving semantics** (§9)~~ — **discharged by
  [ADR-0021 §7 and §8](0021-note-ordering-saving-and-the-note-list.md)**: autosave, per field, on blur
  or a short idle, with a new note committed on its first non-empty field; and Enter stays inert in
  every single-line field, the last one included.
- ~~**Editing a note mid-review, and where authoring is entered from** (§9)~~ — **discharged by
  [ADR-0021 §5 and §6](0021-note-ordering-saving-and-the-note-list.md)**: one editor with four
  entrances, one of them the review screen, where opening it counts as a reveal.
  *This row was missing from the table until 2026-08-01*, along with saving semantics: §9 named four
  things this ADR does not settle and the table carried two, so the map's fog triage — which sweeps
  these tables — never saw them. Recorded rather than quietly added, because the gap is the reason a
  session's worth of decisions sat unowned for a month.
- ~~**Whether notes are user-reorderable, and how `position` is surfaced while authoring**~~ — handed
  *here* by [ADR-0011](0011-new-card-rate-and-daily-limits.md) and never answered; **discharged by
  [ADR-0021 §3 and §4](0021-note-ordering-saving-and-the-note-list.md)**, which found that ADR-0011
  §7's *"need not be dense"* permission was one its own assignment rule never let anyone use, and made
  `position` an order key with infill so a move is one write. Surfaced on the **note list**, not here.
- **Visual design** — **out of scope** for the map as of 2026-07-31, as *the visual design pass* that
  [ADR-0006 §10](0006-the-review-session-experience.md) opened.
  Narrowed by [ADR-0018](0018-the-card-pane-ordering.md) for this pane: what a dormant line *says*,
  where it *sits* and when it appears are settled; only how it *looks* is still that pass's.
