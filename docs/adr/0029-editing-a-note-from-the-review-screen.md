# ADR-0029: Editing a note from the review screen happens after the reveal

- **Status**: Accepted
- **Date**: 2026-08-08
- **Related**: [ADR-0021 §5 §6](0021-note-ordering-saving-and-the-note-list.md) (the four entrances,
  and the *"at any point in the card's life"* this ADR **narrows**),
  [ADR-0006 §3 §4](0006-the-review-session-experience.md) (tap-the-card, and the guarantee that
  grading cannot precede seeing the answer — this ADR **retires** §3's second cause),
  [ADR-0010 §7 §9](0010-leeches.md) (edit is the primary response to a leech; nothing may prompt for
  a judgement in the moment), [ADR-0012 §1](0012-the-note-authoring-experience.md) (the editor shows
  the back, which is the whole mechanism at issue)
- **Evidence**: the **layout pass** — twenty-two wireframes, judged live, at tag
  [`prototypes/issue-120`](https://github.com/amin-bf/cairn/tree/prototypes/issue-120/prototypes/layout-pass-120)
  ([#120](https://github.com/amin-bf/cairn/issues/120)), following the convention of #11, #28 and
  #67. **Wireframes rather than a build**, which is the first time on this map — see *Consequences*
  for what that does and does not buy. What the pass *decided* is recorded in
  [`ui`'s `CONTEXT.md`](../../crates/app/src/CONTEXT.md); the tag is where to reopen a decision, not
  where to read one.

## Context

[ADR-0021 §6](0021-note-ordering-saving-and-the-note-list.md) put *edit this note* on the review
screen **"at any point in the card's life"**, and had to add a rule to make it safe: **entering the
editor counts as a reveal**, because the editor shows the back, so without it
[ADR-0006 §4](0006-the-review-session-experience.md)'s guarantee that *"self-grading can't happen
before the answer is seen"* is quietly false.

The question this ADR answers was never asked: **does the action need to be there before the
reveal at all?** ADR-0021 §6 argued for the action's existence and never separately for its
availability in the two card states — the phrase *"at any point in the card's life"* settles both
in one clause, and only the first half was argued.

It surfaced in the layout pass, from the other end. A full-width control sits directly beneath the
card, which is itself the reveal target ([ADR-0006 §3](0006-the-review-session-experience.md)), so an
accidental press is an ordinary and expected event — and under ADR-0021 §6 an accidental press
**spends the reveal** on a card the user never chose to look at. Moving the control was tried and
rejected on use: the full-width position below the card is the one that reads, and the cost is not
the control's placement.

## Decision

### 1. *Edit this note* is offered after the reveal, and not before

> **The review screen offers *edit this note* once the card is revealed. Before the reveal there is
> no edit action on the review screen.**

Three arguments, in ascending strength.

**The action's own justification is post-reveal in every case it names.**
[ADR-0010 §7](0010-leeches.md) supplies the reason edit exists here at all: *"the honest diagnosis of
most leeches is a defective card: ambiguous, too large, or testing two facts at once."* All three are
judgements about the **pair**. A card is not too large until you see how much answer it wants; it is
not testing two facts until both are visible; and ambiguity is a relation between prompt and answer.
ADR-0021 §6's own framing — *"the moment a defective card can be diagnosed is the moment it is in
front of you"* — is unchanged by this ADR, because a revealed card is still in front of you, in the
same session, one tap earlier than the end-of-session pointer it was written against.

**It removes an accepted cost rather than trading one for another.** ADR-0021 §6 accepted that
entering the editor burns the reveal. That is exactly right when the user *chose* to edit, and it is
pure loss when they did not. After the reveal the cost does not exist: the answer is already on
screen, so an accidental press opens the editor on a card that has nothing left to give away, and
ADR-0006 §4's guarantee is untouched. **The hazard is a property of the pre-reveal state alone**, so
the state is where it is removed.

**And it deletes a rule rather than adding one — which is the decisive argument.** ADR-0021 §6's
*"entering the editor counts as a reveal"* has exactly one customer: the pre-reveal edit. After the
reveal the card is already revealed and the clause is a no-op. So this ADR does not add a condition
to the review screen; it removes a control from one state and retires the rule that existed to make
that state safe. The specification gets smaller.

### 2. What is given up, and why it is worth giving up

**One class of defect is judgeable only before the reveal, and this ADR accepts losing it.** *"Is
this prompt answerable?"* — ambiguous, malformed markup, a prompt that gives away its own answer — is
a question you **cannot** ask once you know the answer. After the reveal you are contaminated: you
can no longer tell whether the prompt was findable or whether you now simply know. That is the same
epistemic shape as ADR-0006 §4's argument about grading, one step earlier, and it is a real loss
rather than a nominal one.

It is accepted on three grounds.

- **The judgement survives the reveal even though the test does not.** A reviewer who could not
  answer a prompt knows that *at the moment of reveal*, and the reveal is one tap away. What is lost
  is the ability to re-run the test, not the finding.
- **The prompt-defect case has a second surface and the pair-defect case does not.** A note is
  reachable from the note list with text search ([ADR-0021 §2](0021-note-ordering-saving-and-the-note-list.md)),
  and a card the user keeps failing reaches the leech screen by itself
  ([ADR-0010 §4](0010-leeches.md)) — where **edit is the primary action**. Both routes stay open.
- **Nothing in the design can act on the distinction anyway.** Replay records a grading, not a
  diagnosis; there is no "the prompt was bad" event and
  [ADR-0004 §1](0004-the-review-event-log.md)'s membership test would refuse one.

**Rejected: keeping the action before the reveal and de-emphasising it.** Smaller, greyer, further
down — all of it is finish rather than arrangement, none of it removes the hazard, and it makes the
one control on the screen that costs something the hardest one to see. The layout pass tried the
strongest version of this (a small affordance in the header) and it was rejected on use.

**Rejected: a confirmation before entering the editor.** It re-introduces a dialog this
specification has refused three times — [ADR-0012 §5](0012-the-note-authoring-experience.md),
[ADR-0016 §3](0016-backup-and-restore.md), and
[ADR-0022 §1](0022-the-import-preview-and-export-report.md)'s own reasoning about which prompts are
defensible — to protect against a mis-tap, which is the weakest case for one there is.

### 3. This is not ADR-0010 §9's inline prompting, and the distinction is unchanged

[ADR-0010 §9](0010-leeches.md) refused *inline prompting* during review because it *"demands a
considered judgement at the moment the user is most frustrated"*, and
[ADR-0021 §6](0021-note-ordering-saving-and-the-note-list.md) drew the line: that is **the app
interrupting to ask a question**, where edit is **the user choosing to fix a typo**. That line is
untouched here. Nothing prompts; a control is available in one card state instead of two.

### 4. What this ADR does *not* settle

- **Where the control sits.** Full-width beneath the card, judged in the layout pass. That is
  arrangement, and it is recorded in [`ui`'s `CONTEXT.md`](../../crates/app/src/CONTEXT.md) rather
  than here.
- **The other three entrances.** [ADR-0021 §5](0021-note-ordering-saving-and-the-note-list.md)'s note
  list, leech screen and create paths are untouched — none of them has a reveal to spend.
- **Finish**, unchanged from [ADR-0006 §10](0006-the-review-session-experience.md).

## Amendments to accepted ADRs

| ADR | What changes | Why |
|---|---|---|
| [ADR-0021 §6](0021-note-ordering-saving-and-the-note-list.md) | *"at any point in the card's life"* is **narrowed to the revealed state**. The section's argument for the action **existing** is unchanged and is not reopened. | §1 above: the phrase settled two questions in one clause and only the first was argued. |
| [ADR-0021 §6](0021-note-ordering-saving-and-the-note-list.md) | *"Entering the editor counts as a reveal"* is **retired**, not merely unused. Its sole customer was the pre-reveal edit. | §1 above. A rule kept past its last caller is one a later reader will restore a caller for. |
| [ADR-0006 §3](0006-the-review-session-experience.md) | **Reveal has one cause again: tap-the-card.** ADR-0021 §6's second cause is withdrawn. | Same. §3's original text stands as written before that amendment. |
| [ADR-0006 §4](0006-the-review-session-experience.md) | **Confirmed rather than amended**, and strengthened: *"self-grading can't happen before the answer is seen"* no longer depends on a rule about the editor, because no route into the editor exists before the reveal. | §1 above. Worth recording, because §4's guarantee is now a property of the screen rather than of a clause someone must remember. |

## Glossary

**Reveal** is revised in [`ui`](../../crates/app/src/CONTEXT.md), which owns the session — it has one
cause again. No new term is minted.

## Consequences

- **The specification is one rule smaller.** *"Entering the editor counts as a reveal"* is gone
  along with the state that needed it, which is the rarer kind of amendment: a removal rather than a
  substitution.
- **ADR-0006 §4's guarantee stops being conditional.** It held only because a rule about a
  side-effect held. It now holds because there is no route.
- **A prompt-only defect is diagnosed one tap later**, and the reviewer who wants to record *"I could
  not answer this"* still has no way to — which was already true and is not made worse.
- **This was judged against wireframes, not a build**, unlike [ADR-0006](0006-the-review-session-experience.md)
  and [ADR-0012](0012-the-note-authoring-experience.md), which went through `/prototype`, and unlike
  [ADR-0025](0025-the-authoring-screen-under-a-soft-keyboard.md), which was measured on the handset.
  The decision rests on the structure of the reveal rule rather than on use, and §2's accepted loss
  is the part most likely to look different once someone is reviewing daily. Recorded so that
  reopening it needs no permission.

## Open items handed onward

| Item | Owner |
|---|---|
| Whether §2's accepted loss — a prompt you could not answer, diagnosable only pre-reveal — is felt in real use | Post-implementation, like [ADR-0010](0010-leeches.md)'s thresholds and [ADR-0011 §4](0011-new-card-rate-and-daily-limits.md)'s default |
| Where the control sits, and its finish | The **layout pass** and the **finish pass** respectively (`ui`'s `CONTEXT.md`); the first is taken, the second is a blank slate by [ADR-0006 §10](0006-the-review-session-experience.md) |
