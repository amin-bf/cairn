# ADR-0035: The vertical anchor — the reach line, and the axis a thumb has

- **Status**: Accepted
- **Date**: 2026-08-16
- **Resolves**: [Design Pass: The Vertical Anchor — What the Leftover Height Does](https://github.com/amin-bf/cairn/issues/141)
- **Related**: [ADR-0031](0031-the-page-frame.md) (the page frame, whose two horizontal numbers this
  joins with a vertical one it left unasked), [ADR-0034 §1](0034-the-controls.md) (the segmented
  grade row, which §3 supersedes under a thumb and keeps under a pointer),
  [ADR-0033 §1 §4](0033-the-card.md) (the card is one object at a fixed height, which §1 relies on
  and §2 protects), [ADR-0029 §1](0029-editing-a-note-from-the-review-screen.md) (*when* the edit
  entrance is offered, which §2 moves the *where* of and does not reopen),
  [ADR-0026 §5](0026-the-per-tap-keyboard-re-pop.md) (the soft-keyboard state type §3 reads),
  [ADR-0025 §1](0025-the-authoring-screen-under-a-soft-keyboard.md) (the reserved inset bands the
  reach line is measured from the top of)
- **Evidence**: two sittings on a physical Pixel 8 Pro, in the hand, with the control cluster
  **dragged into position by thumb** and the resulting distance read off a live on-screen readout.
  Thirteen desktop captures of six candidate arrangements at 448×997 precede them, preserved with
  the prototype as the tag `prototypes/issue-141`, readme at
  [`docs/design/prototype-141/README.md`](../design/prototype-141/README.md).

## Context

[ADR-0031](0031-the-page-frame.md) decided what the leftover **width** does — nothing — and never
asked what the leftover **height** does, because at the 860px window every capture in the design pass
was taken at, there isn't any. On a handset there is a great deal of it.

[#125](https://github.com/amin-bf/cairn/issues/125)'s sitting on the physical device found where it
goes and what that costs. Measured there: the control cluster ended at **y=1880 of 2992**, so the
card, the grades and *Edit note* occupied the top 63% of the page and the bottom ~1100px — the part a
thumb owns — was empty. The screen was judged to *look* calm and to *be* a stretch one-handed, and
those are the same fact: it looks calm because everything is up top.

**No target was undersized.** The centre segment of the grade row was comfortable in either hand and
nothing was mis-hit. This is placement alone — which is the first time the design pass's own rule,
*hit targets and density follow touch, not the pointer*, had met an actual thumb. The rule was
honoured in **sizing** — a 36px control is still 36px — and the arrangement was laid out for a
pointer regardless, which is a gap the rule as written does not close.

Two constraints came with the ticket. **The card must not absorb the slack**: `surface::REVIEW_HEIGHT`
is a constant 300 logical px and ADR-0033 §1 makes the card a well cut into the page, so a well
stretched to a 997dp page reads as a container that failed to fill. And **horizontal reach is a
second axis**: *Barely* and *Easy* sit at the two extremes of the segmented row and flip between
comfortable and a stretch depending on which hand holds the phone.

## Decision

### 1. The last control on a screen sits on a **reach line**, 165px above the bottom of the page

> **`frame::REACH_LINE` is 165.** When the page has the room, the bottom edge of the final control
> cluster lands there and the leftover height falls **between the card and the controls**. When it
> does not, the controls follow the card at the ordinary stated gap and nothing is placed.

> **Extended by [ADR-0039 §8](0039-the-list-row.md): on a *scrolling* surface, a control on the
> reach line lives outside the scroll.** `slack_above` spends **leftover** height, and a list has
> none — twenty-five rows are longer than any page — so on the note list this section reached nothing
> at all while the screen's one primary action sat at the very top, the furthest point from a thumb.
> Applying it verbatim *inside* the scroll was drawn and refused: the control then scrolls away the
> moment the list outgrows the page, which is the same defect with a better first screenshot. So the
> rule gains a clause rather than an exception, and `frame::pinned_band` is the arithmetic — at the
> cost of the reserved band, which a 1280×800 window pays 209px of list viewport for.

> **Confirmed as a page rule by [ADR-0038 §5](0038-the-mark-and-the-icon-rule.md), and it now has a
> second call site.** *A screen* was written here deliberately and drawn once: `frame::slack_above`
> had exactly one caller, the grade cluster, so until
> [The Fixture Bench](https://github.com/amin-bf/cairn/issues/153) made the caught-up-with-a-leech
> state photographable, nothing had cause to apply this section or ignore it anywhere else. That
> screen was drawing the durable leech entrance `gap(3)` under the statement, at y=305 of 800 — this
> section standing as written while the application did otherwise.
>
> **Narrowing it to Review was the live alternative, was drawn, and lost by looking.** On a page with
> 500px of nothing under it, a control tucked against the statement reads as attached to the sentence
> rather than as the way onward. The entrance now lands on the reach line like any other last
> control, and a third screen ending in a control inherits this without a decision.

**165 is measured, not chosen, and the measurement is the finding.** The prototype made the cluster
*draggable* rather than offering fixed candidates, because the round before it came back as "closer,
but the very bottom is still a stretch" — which is an answer about a distance nobody had a number
for. It was then placed by thumb twice, a round apart, with different contents:

| round | cluster | height | bottom edge above the page bottom |
|---|---|---|---|
| two | *Forgot* over a segmented row | 148 | **162** |
| three | four grades stacked | 184 | **169** |

Two placements, two shapes, converging within 7px. So what a thumb picks is **a line above the
bottom of the page**, not a gap below the card: the cluster grows upward from that line and the slack
absorbs the difference. That is why this ADR names one number rather than a gap plus a cluster
height, and why the arithmetic subtracts the cluster's own height rather than adding a margin.

**It is an absolute distance, not a fraction of the page.** The band it clears is where the hand
*grips*, and a grip is physical. A proportion would put the line too high on a tall screen and too
low on a short one.

**And it is one rule, not a breakpoint.** The same expression covers the handset, the desktop window
and everything between: the gap absorbs whatever is left over and stops at the stated gap when there
is nothing left. The 860px window the design pass judges at reaches that arm by arithmetic, without
anything having to ask what it is running on — which is what keeps
[#124](https://github.com/amin-bf/cairn/issues/124)'s *one arrangement, centred, at every width*
intact here.

### 2. ***Edit note* rides directly under the card**

> The reading order is unchanged — prompt, answer, then the controls — and the **distance** is not:
> the slack falls between *Edit note* and the grades, so the rarest control on the screen keeps the
> worst reach and the ones pressed on every card take the zone the thumb owns.

The ticket proposed two candidates and **the sitting struck both**, on a rule it produced while
being judged: **the card must not move when it is revealed.** *Reading order held* bottom-anchors the
whole stack, so the card rises on reveal to make room for the grades; *frequency maps to reach* puts
*Edit note* above the card, so the card is pushed down when that control appears. Both move the card
at the exact moment the eye is going to the answer, which is the one moment on this screen when
nothing should move.

Under the card, the same aim costs nothing: the control appears below everything already drawn, so
nothing above it shifts. Above the card the position is only tenable with an **empty reserved row**
before the reveal, which was drawn, judged, and beaten by this.

This moves *where* the entrance is drawn and does not touch **when**: ADR-0029 §1 still offers it
only on a revealed card, and ADR-0006 §4's guarantee still holds by there being no route into the
editor before the reveal.

### 3. Under a thumb the grades **stack**; under a pointer they stay a row

> Four full-width controls, *Forgot* held apart by three units and the three passes a unit apart, on
> a touch-operated platform. On a pointer platform, ADR-0034 §1's arrangement is unchanged.

**This supersedes ADR-0034 §1 for touch**, and the reason is one sentence from the sitting: *a thumb
travels up and down freely and sideways badly*. That is why the horizontal axis the ticket carried is
**dissolved rather than answered** — a stack has no extremes, so there is nothing left at one for a
hand to fail to reach. Narrowing and centring the row was the alternative and it was drawn; it treats
the symptom and keeps the axis.

§1 is not wrong about what it argued. It chose the row on an argument about **the scale** — four
stacked controls read as four rungs of one ladder, which puts the failure grade on a scale it is not
on — and it measured its widths at 208px and 163px, which are comfortable. What it never had was a
thumb, and the axis it chose is the axis a thumb is worst on. **The half of §1 that survives is
*Forgot* held apart**, now carried by a three-unit gap rather than by a change of shape, because in a
stack the gap is doing that work alone.

**The rule is *touch*, and the soft keyboard is how this client reads it.** A platform that raises a
keyboard on the screen is a platform with no pointer, and `platform::SoftKeyboard` already
distinguishes *absent* from *down* because ADR-0026 §5 forced it to — so this needs no new seam
function, no capability constant and no `#[cfg]` at a call site. The proxy is exact for the two
targets that exist. **A native client asks its own platform directly**: this section states the
decision in terms of touch precisely so that a Kotlin or Swift client implements the same rule
without inheriting egui's way of noticing.

## Consequences

- **The frame gains its first vertical number.** `frame` now owns `REACH_LINE`, `page_room` and
  `slack_above` beside `PAGE_MARGIN` and `MEASURE` — one naming site for the page's geometry in both
  axes, on the same argument ADR-0031 made for the horizontal pair.
- **`Ui::available_height` is unusable inside a `ScrollArea` and this is written down in `frame`.**
  It returns **zero**: the content `Ui` is sized to its content — that is what scrolling means — so
  "available" is what the widgets already drawn have claimed, never what the screen has left. The
  clip rect is the viewport. Found by a prototype variant rendering pixel-identically to the thing it
  was varying, with nothing failing and no test that could have caught it.
- **Two tests carry the rule**, both of which fail on a change that renders perfectly. One pins that
  the cluster's bottom edge lands on the line *whatever the cluster holds*; the other asserts the
  grades' **drawn widths** under each platform, because dropping the touch argument and always
  drawing the row compiles, passes everything else, and silently returns the handset to the state
  #141 was opened for.
- **The Review screen now has three vertical regions rather than one stack**: the card and its
  entrance at the top, the slack, the grades on the line. Every other screen is untouched — none of
  them has a control cluster the hand returns to dozens of times in a sitting, which is the property
  that earns the placement. Whether the note list and Settings want it is the Notes and Settings
  slices' question, not this one.
- **An uninstall is not a first launch on Android.** Found while deploying the prototype: `data_dir`
  is `getFilesDir()`, which ADR-0007 §6 deliberately puts *in* the Auto Backup set, so a reinstall
  restored the previous collection and the prototype's seed silently did not run. Any handset
  checkpoint that assumes a fresh install gives a fresh collection is wrong; `adb shell pm clear`
  is the thing that does.
- **The design pass's *targets follow touch* rule now covers arrangement too.** It was honoured in
  sizing and silent about placement, which is how a screen sized for a thumb ended up arranged for a
  pointer with nothing failing. §1 and §3 are the two halves of closing that: where the controls sit,
  and which axis they are spread along.
