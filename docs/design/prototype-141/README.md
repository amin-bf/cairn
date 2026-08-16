# What the leftover height does — six arrangements and two thumbs

> **Outcome.** The last control cluster on a screen ends **165px above the bottom of the page** when
> there is room, and follows the card when there is not. ***Edit note* rides directly under the
> card.** And under a **thumb** the four grades **stack** rather than taking ADR-0034 §1's segmented
> row, because a thumb travels up and down freely and sideways badly. Recorded as
> [ADR-0035](../../adr/0035-the-vertical-anchor.md).
>
> **165 is measured, and how it was measured is the finding.** The first round produced six
> arrangements to choose between and the answer came back *"closer, but the very bottom is still a
> stretch"* — which is not a choice between candidates, it is a **distance** nobody had a number
> for. So the second round stopped generating variants and made the cluster **draggable**, with a
> live readout in the units the ADR would name. It was then placed by thumb twice, a round apart,
> with clusters of two different heights, and the bottom edge landed within 7px both times.
>
> **The ticket's own two candidates were both struck**, on a rule the sitting produced while
> judging: *the card must not move when it is revealed*. Both of them move it.

The primary source for [#141](https://github.com/amin-bf/cairn/issues/141), the fifth slice of the
Review vertical on the design pass map ([#121](https://github.com/amin-bf/cairn/issues/121)).

**This never merges into `main`.** It is preserved as the tag **`prototypes/issue-141`**, contained
in no branch — the repo's standing convention (`AGENTS.md`, *Rules that are easy to break silently*
3), whose predecessors are `prototypes/issue-8`, `-11`, `-20`, `-28`, `-67`, `-120`, `-124`, `-131`,
`-133` and `-134`. A tag is fetched by every clone, so a later session reads this without merging
anything:

```sh
git show prototypes/issue-141:docs/design/prototype-141/README.md
git checkout prototypes/issue-141 -- crates/app/src/proto.rs
```

## Why this one is not a desktop binary

Every prototype in this map so far has been a `cairn-desktop` bin driven by a capture script,
because every question so far could be settled from a pair of stills at 560×860 and 1280×800. This
one could not, for two reasons. The question **only exists where there is leftover height** — at the
860px window every capture in the map was taken at, there is none — and its answer is a judgement
about **reach**, which a mouse cannot make.

So the prototype is the application itself, varied behind `crates/app/src/proto.rs`, built for the
handset and cycled in the hand. `cargo apk` packages `cairn-app`'s cdylib, so there is no second
Android binary to put it in. **The variant switcher lives on Settings**, never on Review: a control
added to the review screen changes the very arrangement being judged.

Everything the four Review ADRs fixed is held constant and drawn through the shipped modules —
the frame ([ADR-0031](../../adr/0031-the-page-frame.md)), the scale
([ADR-0032](../../adr/0032-the-type-scale-and-the-rhythm.md)), the card
([ADR-0033](../../adr/0033-the-card.md)) and the controls
([ADR-0034](../../adr/0034-the-controls.md)). **Nothing here resizes anything**: #125 found no target
undersized, so every variant draws the same 36px controls and the same 300px card and varies only
where on the page they sit.

## Round one — six arrangements, thirteen captures

`448x997/` — the handset's own dp geometry, drawn in the nested-compositor harness so a capture
costs the operator no window and no focus.

| | *Edit note* | the slack falls | the thumb zone holds |
|---|---|---|---|
| **A** today | below the grades | below everything | nothing |
| **B** bottom | below the grades | above the card | *Edit note* |
| **C** reach | above the card | between *Edit note* and the card | the grades |
| **D** split | below the grades | between the card and the grades | *Edit note* |
| **E** split, edit up | above the card | between the card and the grades | the grades |
| **F** stacked | under the card | between the card and the grades | the grades |

**B is the ticket's *reading order held*** — the existing stack, unchanged, pushed to the bottom.
**C is its *frequency maps to reach*** — the control pressed on every card gets the zone the thumb
owns. D and E were added because the two candidates conflate two questions: *where the slack goes*
and *where **Edit note** goes* are independent, and separating them is what produced an arrangement
neither candidate proposes — the card where reading wants it, the controls placed on their own.

F arrived in round three and is the one that shipped.

## Round two — the knob

`handset/01-round-two-segmented-placed.png`. Variant D, on the physical Pixel 8 Pro.

> **The black band at the top of both handset captures is a redaction, not a rendering defect.**
> These come off real hardware rather than an emulator, so the status bar carries system chrome that
> belongs to the device's owner and not to the application. It is painted over **in place** at the
> exact inset height — 151px, read off `dumpsys` — rather than cropped, so every coordinate measured
> below is still true of the image as committed. See `AGENTS.md`, *Landing work*.

Round one's verdict was *"card on top is good, but the grades all the way at the bottom still puts a
lot of strain on the thumb"*, plus a rule: **the card should not shift on reveal**. That rule strikes
B and C outright — both anchor the card, so it rises on reveal to make room for the grades — and it
forced *Edit note*-above-the-card to **reserve an empty row** before the reveal rather than appear on
it, since a control appearing above the card pushes the card down at the exact moment the eye is
going to the answer.

The rest of the verdict is a distance, so the prototype grew one: the empty space became a **drag
surface** and the block rode wherever the thumb put it, with `drag to place — lift N of M` painted
into the gap. Three details that make it work, all of which cost something to find:

- **The drag lives on the empty space, never on the controls**, or positioning the grades fires them.
- **Both the gap above the block and the space below it take the drag.** Lifting the block eats the
  gap it is dragged by, so at the top of the range the handle would shrink to nothing exactly where
  the answer lives; the space below grows by the same amount, so the two together are always the
  whole page.
- **The starting value is seeded from the previous placement**, so a rebuild does not throw away a
  position that took a sitting to find.

Placed: **lift 134**, cluster 148 tall, **bottom edge 162 above the page bottom**.

## Round three — the stack

`handset/02-round-three-stacked-placed.png`. Variant F.

Round two's verdict was *"E is good now; let's have E, but edit below the card and make the grades
stacked — it is easy to move the thumb up and down but not easy to move side to side"*.

That last clause is the second finding and it is a fact about hands, not about this screen. It
**dissolves the ticket's second axis rather than answering it**: *Barely* and *Easy* flipping between
comfortable and a stretch depending on which hand holds the phone is a property of horizontal
extremes, and a stack has none. The prototype's other candidate — narrowing the row to 72% and
centring it — treats the symptom and keeps the axis, and was never needed.

It also moves *Edit note* from **above** the card to **under** it, which reaches the same aim more
cheaply: both keep the rarest control out of the thumb's zone, and under the card nothing above it
shifts when it appears, so the reserved row round two needed is not needed at all.

Placed: **lift 141**, cluster 184 tall, **bottom edge 169 above the page bottom**.

## The convergence, which is the whole result

| round | cluster | height | bottom edge above the page bottom |
|---|---|---|---|
| two | *Forgot* over a segmented row | 148 | **162** |
| three | four grades stacked | 184 | **169** |

Two placements, a round apart, with different contents and a 36px difference in height, landing
within 7px of each other. So what a thumb picks is **a line above the bottom of the page** — not a
gap below the card, and not a fraction of the page. The cluster grows *upward* from that line and the
slack absorbs the difference, which is why ADR-0035 names one number instead of a gap plus a height.

It is an absolute distance rather than a proportion because the band it clears is where the hand
**grips**, and a grip is physical: a fraction would put the line too high on a tall screen and too
low on a short one.

## Four things this cost to find

### 1. `Ui::available_height` returns zero inside a `ScrollArea`

The first build of variant B rendered **pixel-identically to variant A**, with nothing failing. The
cause is that inside a `ScrollArea` the content `Ui` is sized to its *content* — that is what
scrolling means — so "available" is what the widgets already drawn have claimed, never what the
screen has left. The **clip rect** is the viewport, and the viewport is what a thumb can reach.

This is now `frame::page_room` on `main`, with the finding written where the next caller will look.

### 2. An anchored block needs a bottom margin — and the reach line subsumes it

Anchored flush, the grade row sits against the gesture bar and reads as content that overran rather
than content that was placed. The prototype gave it the page margin, reused rather than invented. The
shipped rule needs no such constant: 165 is measured from the bottom of the page and is six times the
margin, so the gutter is a consequence rather than a second number.

### 3. An uninstall is not a first launch on Android

The prototype seeds 40 notes and lifts the new-card rate to match, because reach is judged by
repetition and the shipped six-note seed is a sitting over in under a minute. On the device it
offered **five**. `adb uninstall` had been followed by `adb install`, and the collection came back:
`cairn_store::platform::data_dir` is `getFilesDir()`, which
[ADR-0007 §6](../../adr/0007-the-local-store.md) deliberately puts **in the Auto Backup set** so a
collection survives onto a replacement phone. So the seed's `is_empty` guard was correctly false and
nothing ran.

`adb shell pm clear dev.cairn.app` is what actually gives a first launch. Any handset checkpoint that
assumes a reinstall does is wrong, and it fails quietly — the app runs perfectly, on the wrong
fixture.

### 4. Judging a distance wants a knob, not a menu

Round one generated six arrangements and the answer was that all six were wrong in the same
direction. Fixed candidates can only ever land on a value someone guessed in advance; a drag with a
readout turns a taste judgement into a measurement, and it took one build instead of three.

## What is on the tag and not on `main`

`crates/app/src/proto.rs` (the six anchors, the drag, the readout and the Settings switcher), the
review-screen and settings-screen edits that call it, the 40-note seed, `scripts/storyboards/proto-141.txt`,
and every capture in this directory. `main` keeps the decision: ADR-0035, `frame::REACH_LINE`,
`frame::page_room`, `frame::slack_above`, and the review screen drawing one arrangement rather than
six.
