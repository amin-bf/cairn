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
- **The warning also appears in the form pane**, because on a phone the card pane is behind a
  toggle and an ambient warning that lives only there is invisible exactly when it matters. On
  desktop it therefore appears twice, deliberately.
- **It offers Undo**, and the copy says what is true: nothing is deleted, the reviews stay in the
  log, and they reattach by themselves if the content returns.

**The word is *dormant*.** `replay/CONTEXT.md` rules out retired, deleted, orphaned and tombstoned
because each implies a stored lifecycle that deliberately does not exist. The prototype's own copy
broke this twice before it was caught, which is how the wrong word reaches a spec.

### 6. Changing a note's kind can reattach history to an unrelated card — and this ADR does not fix it

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

## Amendments to accepted ADRs

- **ADR-0002 §7** — its claim that the replay mechanism "absorbs a note changing kind" is amended
  by §6 above: it absorbs it, and in doing so can reattach history to a semantically different
  card. §7's list of consequences to be honest about gains this one.
- **ADR-0002 §8** — the restricted Markdown subset is amended by §8 above: `**bold**` obliges the
  app to ship a bold face, since it cannot be rendered by any other means.
- **ADR-0002 §9** — its argument for deferring audio is amended by §8 above: it rests on a text
  `Pronunciation` field, which requires a face with IPA coverage.

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

- **`CardRef` and the kind discriminator** (§6) — ADR-0002's decision to take. Until then the
  warning stands in for it.
- **A soft-keyboard pass on the handset** (§9) — the one question desktop cannot answer.
- **Saving semantics** (§9) — autosave versus explicit save.
- **Visual design** — still ADR-0006 §10's open item, now with a second screen waiting on it.
