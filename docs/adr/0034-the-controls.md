# ADR-0034: The controls — three weights by role, and the two states nobody had drawn

- **Status**: Accepted
- **Date**: 2026-08-12
- **Resolves**: [Design Pass: The Controls — Grades, the Entrance, and the Empty State](https://github.com/amin-bf/cairn/issues/134)
- **Related**: [ADR-0033 §3](0033-the-card.md) (the constraint this ADR discharges, and whose figures
  §5 corrects), [ADR-0030 §1 §5](0030-the-first-finish-pass-decisions.md) (colour named at exactly
  one site; the dormant accents, one of which §2 wakes),
  [ADR-0032 §1](0032-the-type-scale-and-the-rhythm.md) (the display tier, which §3 applies outside a
  card face), [ADR-0006 §1 §4](0006-the-review-session-experience.md) (the checkpoint §4 finally
  implements, and the interval preview §1 keeps),
  [ADR-0010 §6 §8](0010-leeches.md) (the durable leech entrance and the end-of-session pointer),
  [ADR-0001 §3](0001-scheduling-algorithm-and-grade-scale.md) (grades are durability, never a queue)
- **Evidence**: fifty-nine captures of a throwaway prototype that varies **only** the controls, at
  both judging widths, preserved as the tag `prototypes/issue-134` with its readme at
  [`docs/design/prototype-134/README.md`](../design/prototype-134/README.md). Judged as pictures, in
  a review session, over three rounds — the third existing because the second produced an objection
  no measurement would have raised.

## Context

[ADR-0033 §3](0033-the-card.md) closed the card ticket with a constraint rather than a decision:

> **A card outweighs the controls beneath it. This binds #134.**

It reached that by blurring a capture until nothing was legible and asking what still stood out, and
the answer on the shipped Review screen was the grade buttons. It recorded the grade buttons at
**1.54:1** against the page and the card at **1.12:1**, photographed one alternative — the same
controls with their fill removed and a 1px edge kept — and left the treatment here.

Three things then turned up that were not on the ticket, and two of them change what §3 means.

**§3's two figures were measured on two different pages.** The 1.54:1 is `STONE_5` against eframe's
`#080808` — the page **§2 of that same ADR abolished**. The card's 1.12:1 is against `panel_fill`,
the page that replaced it. Measured on one page, a control is **1.293:1** and the card **1.121:1**:
§3's conclusion survives and its gap is **0.17 rather than 0.42**. Fixing the page had already done
most of §3's work before this ticket opened.

**Outline-or-slab was a false pair.** §3 photographed the two ends of the ramp and never the rung
between them. `faint_bg_color` — `STONE_3`, which no control in the application had ever taken —
measures **1.099:1**, which is *quieter than the card itself*. A control can satisfy §3 without
giving up being a surface.

**And §3 is a relationship, not a material.** The picker and the caught-up screen have no card, so
there is nothing for the comparison to be about; drawn at one flat quiet treatment, the one control
that is the way forward becomes a faint rectangle on an empty page that reads as **disabled**. A
treatment applied everywhere satisfies §3 on one screen and damages every other.

Two further things were found by drawing states no capture in this repository holds. The **10-minute
checkpoint contradicts [ADR-0006 §1](0006-the-review-session-experience.md) in writing** — §1 says it
surfaces *"without hiding the card underneath — the reviewer can still grade what they're looking at
while deciding"* and `screens/review.rs` drew it as an `else if` branch replacing the card. And the
**entrance offered sittings longer than the queue**, because the shipped `count_buttons` capped its
options and the replacement second line did not.

## Decision

### 1. A control is `faint_bg_color`, and the grades are ***Forgot* apart, three passes segmented**

> **An ordinary control is `STONE_3` with a `STONE_4` edge — 1.099:1 against the page, quieter than
> the card's 1.121:1.**

This discharges ADR-0033 §3 by measurement rather than by assertion, and it keeps a control looking
like a control, which turned out to be the property the judging cared about most (§4 below).

The arrangement is [#124](https://github.com/amin-bf/cairn/issues/124)'s and this ADR confirms it
against pictures rather than re-deciding it. **The shape is an argument about the scale, not about
space**: *Forgot* is a different kind of answer — *I did not know this* — and the three passes are
degrees of one answer. Four stacked full-width controls say those are four rungs of one ladder, which
places the failure grade at the bottom of a scale it is not on, and that reading is the one
[ADR-0001 §3](0001-scheduling-algorithm-and-grade-scale.md) spends a section forbidding elsewhere.
The vertical budget the row frees is a consequence, not the reason.

**The arrangement does not discharge §3 and the material does.** Drawn in the shipped material, the
row leaves the card losing exactly as before; only the fill inverts it. Worth stating because the two
changes landed together and the credit is otherwise unassignable.

**The row survives a fourth pass grade**, measured rather than hoped: three segments are 208px at the
judging width and **163px** at the application's own; four are 154px and **118px**, and a label with
its interval fits inside 118px with room. A later change to ADR-0001's scale therefore does not have
to reopen this arrangement.

**The interval preview is demoted to the small tier and dimmed.** Both were the same size and colour,
which made two grades that happen to share `1d` read as *the same button twice* rather than as two
answers to one card. ADR-0006 §4 records the preview as wanted information confirmed live, so the fix
could not be to remove it — `PROTO_PREVIEW=none` was drawn as the control that shows what it is worth
and is not a candidate.

### 2. The weight follows the **role**, and a card-less screen keeps its primary

> **A control is quieter than the card on any screen that has one. On a screen with no card, the one
> control that is the way forward keeps its fill.**

| | fill | vs the page | where |
|---|---|---|---|
| ordinary | `STONE_3` | 1.099:1 | grades, *Edit note*, settings, notes — everything else |
| primary | `STONE_5` | 1.293:1 | *Start*, the leech entrance, the end-of-session pointer |
| text action | none | — | a set of alternatives **beside** a primary |

The card sits at **1.121:1**, between the first two, which is §3 restated as arithmetic. A primary is
louder than a card *by design* and there is deliberately no call site for one beside a card.

**The entrance becomes one primary way in.** `Start — all 5` names what pressing it commits to, with
the shorter sittings as a second line. The picker was four equal controls with no way in that was
*the* way in, asking for a decision before anything had happened; the sitting size is a decision most
days do not want to make.

**The second line is the link accent's first caller, and that is a decision taken rather than a use
made.** [ADR-0030 §5](0030-the-first-finish-pass-decisions.md) records warn, error and link as
*"defined-and-dormant"* explicitly so that a later reader does not colour something because the
colour exists. Drawn at weak-text weight the line was very nearly invisible — technically present,
practically gone — and these are text actions with no surface of their own, which is the one shape
the accent fits. **§5's rule is unchanged and warn and error stay dormant.**

**The second line offers only sittings strictly shorter than the queue.** An option equal to it is the
primary said twice in a quieter voice, and options larger than it state work that does not exist. A
consequence worth naming: at the default new-card rate of five, a first-run user sees **no second
line at all**, because none of 5/10/20 is shorter than a queue of five. That is correct — there is no
shorter sitting to offer — and it means the accent's only call site is invisible until a collection
has some history.

### 3. The caught-up screen takes the **display tier**, and keeps the one control it has

> **`All caught up.` centred at the display tier, with the durable leech entrance below it.**

This is the only Review state with no card and no work in it, and it was one body sentence tucked
under the heading — a state given no more room than a form label. Centred and given the screen it
reads as an answer rather than as an absence, which is what it is: nothing is due because the work is
done.

**This applies the display tier to something that is not a card face**, narrowing
[ADR-0032 §1](0032-the-type-scale-and-the-rhythm.md)'s *"the text actually being read"* the same way
[ADR-0033 §4](0033-the-card.md) narrowed it in the other direction. The scale has four sizes and
nothing between 20 and 40; at 20 the whole content of this screen is set at the size of the word
*Review* three lines above it. #124's variant E reached for 24 and the scale does not have it — which
is the second time in three tickets that a case has arrived between two rungs, and is recorded as a
pattern rather than as a complaint.

**The leech entrance stays, and takes the primary weight.** On a caught-up Review it is the *only
control on the screen* ([ADR-0010 §6, §8](0010-leeches.md)) — and three of #124's five variants
dropped it entirely with nothing failing, which is how little there is to notice its absence.

### 4. The 10-minute checkpoint sits **above** the card, compact, and never replaces it

> **ADR-0006 §1 is implemented for the first time.** The checkpoint is the sentence and two controls
> on one line, above a card that stays on screen and stays gradeable.

§1 has said since it was accepted that the checkpoint surfaces *"without hiding the card underneath
— the reviewer can still grade what they're looking at while deciding"*. The application drew it as
an `else if` arm that replaced the card. **Nothing failed, and nothing could have**: `checkpoint_due`
needs ten real minutes to elapse, which no capture run waits for and no test had ever forced, so the
one state that breaks the guarantee was the one state nobody had ever looked at. This is the same
class of defect as ADR-0030 §4's badge case, found the same way — by a ticket that had to draw the
state for another reason.

**Compact, because §1 also calls the timer *"a courtesy check-in, not an enforcement mechanism"***.
The literal fix — two full-width controls above the card — obeys §1 and pushes the card 140px down
the page to ask a question the reviewer did not raise, which is how an application draws an
enforcement. `the_ten_minute_checkpoint_never_hides_the_card` winds the clock past the checkpoint and
pins the card, the prompt and a grade all being on screen together.

### 5. Nothing is frameless except a text action, and that was judged rather than assumed

> **A secondary control gives up its *width*, never its border.**

Two drafts removed a border to stop a secondary action reading as a primary one — *Edit note* under
the grades, and the checkpoint's pair. Both were rejected on sight, on one ground: **it is not
obvious that they are clickable.** No contrast figure would have raised that, and it is the reason
the third round of judging exists.

`controls::text_action` is the single exception and earns it by being a *set of alternatives sitting
beside a primary*, where the primary has already established that this row is a place where things
are pressed. Used alone it is the same defect.

*Edit note* is therefore **full width**, exactly like a grade. The compact form was drawn and lost:
between a secondary action that is unmistakably a control and one that is unmistakably not a grade,
the first matters more. What tells it apart from the four is its position and its separation, which
is what the shipped screen already relied on.

## Consequences

### What this changes

| Document | Change | Why |
|---|---|---|
| [ADR-0033 §3](0033-the-card.md) | **Discharged, and its figures corrected.** Its 1.54:1 was measured against the page §2 of the same ADR abolished; on one page a control is 1.293:1 and the card 1.121:1, so the gap it describes is 0.17. Its conclusion stands. | §3 compared two pages, exactly as ADR-0033 §5 found ADR-0030 §3 had. |
| [ADR-0030 §5](0030-the-first-finish-pass-decisions.md) | **Amended: the link accent is no longer dormant.** §2 above gives it its one call site. Warn and error stay dormant and §5's rule is unchanged. | §5 asks that waking an accent be a decision; this is that decision, taken against a picture. |
| [ADR-0032 §1](0032-the-type-scale-and-the-rhythm.md) | **Widened.** The display tier is not only a card face — §3 above gives it to the caught-up statement. | Second time in three tickets a real case has landed between two rungs of the scale. |
| [ADR-0006 §1](0006-the-review-session-experience.md) | **Implemented.** The checkpoint no longer hides the card. §1 is unchanged; the application now does what it says. | It had never been true, and no test could have noticed. |
| [ADR-0006 §4](0006-the-review-session-experience.md) | **Narrowed.** Four full-width stacked controls become *Forgot* apart and three segmented; the interval preview stays and is demoted. | §4's arrangement was round two's presentation choice, and §10 disclaims the prototype's visual decisions. |
| [ADR-0010 §6](0010-leeches.md) | **Unchanged, and now weighted.** The durable entrance and the pointer take the primary weight, because they sit on screens with no card. | §6 requires the entrance be reachable; it says nothing about what stops it disappearing into the page. |

### What this costs

- **A third weight is a thing to get wrong.** `primary` beside a card breaks §3 and renders
  perfectly. `an_ordinary_control_is_quieter_than_a_card_and_the_primary_is_not` pins the ordering of
  the three fills, which catches the palette drifting but **not** a call site picking the wrong one.
  That is a real gap and it is where the next defect of this class will come from.
- **The link accent's only call site is invisible on a fresh collection** (§2). Nobody sees the
  decision until a queue exceeds five.
- **`Edit note` still reads as a fifth rectangle** — §5 chose that knowingly, preferring an
  unmistakable control to an unmistakable non-grade.
- **The pointer's two controls remain equal in weight**, one of which is a dismissal. ADR-0010 §6
  requires the pointer never be a decision point, and two equal slabs is how an application draws
  one. Photographed, not fixed.

### What this does not settle

| Question | Whose |
|---|---|
| Whether the end-of-session pointer's pair should be weighted against each other | Not yet a question — it needs the leech screen designed first |
| Whether the count picker should exist at all | Beyond this ticket: dropping the size choice changes ADR-0006 §1's session shape, which is behaviour |
| The leech screen itself, which this ADR sends the user to and does not draw | [#121](https://github.com/amin-bf/cairn/issues/121)'s fog |
| A rule for controls whose only home is one screen | [#121](https://github.com/amin-bf/cairn/issues/121)'s fog — §3 met the specific case and did not generalise |
| Light mode, where these three fills must re-derive against a light page | [#121](https://github.com/amin-bf/cairn/issues/121)'s fog |
