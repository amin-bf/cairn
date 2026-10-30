# ADR-0043: The leech screen — a well per card, worded controls, and a cost line that states the time

- **Status**: Accepted
- **Date**: 2026-10-29
- **Resolves**: [Design Pass: The Leech Screen — the One List the Pass Walked Past](https://github.com/amin-bf/cairn/issues/156)
- **Map**: [Map: The Cairn Design Pass](https://github.com/amin-bf/cairn/issues/121)
- **Related**: [ADR-0010 §4 §6 §7 §8](0010-leeches.md) (ranked not cut, where the user meets the
  list, the three actions, the suspended section — none of which this reopens),
  [ADR-0033 §1](0033-the-card.md) (the well §1 draws a row in),
  [ADR-0034 §1 §2](0034-the-controls.md) (the ordinary weight and the 36px height §2 repairs),
  [ADR-0035 §1](0035-the-vertical-anchor.md) and [ADR-0038 §5](0038-the-mark-and-the-icon-rule.md)
  (the reach line, a page rule, which §5 brings to a screen it had not reached),
  [ADR-0039](0039-the-list-row.md) and [ADR-0041 §4](0041-the-file-surface.md) (the other two list
  rows, and where the icon rule this screen tested first was settled)
- **Evidence**: two sittings. The first judged a ladder of row shapes, control weights, rhythms and
  captions in the running application, preserved as the tag `prototypes/issue-156`, readme at
  `docs/design/prototype-156/README.md` on that tag. The second judged five caption wordings in the
  running application on the `leeches` fixture once
  [The Constant Answer Time](https://github.com/amin-bf/cairn/issues/173) had made answer times real,
  preserved as the tag `prototypes/issue-156-caption`.

## Context

The leech screen lists the cards that keep failing, worst first (ADR-0010 §4), each with *Edit*,
*Suspend* and *Delete*, and keeps suspended cards in a section of their own (§8). It was **the one
screen the pass walked past**: the frame, the type scale, the rhythm, the card and the controls were
each settled against Review, and none of them had been judged here.

Three things were wrong on sight. A note's own text was a button among *Suspend* and *Delete*, so
the subject of the row wore the weight of its actions. Rows had no rhythm: a caption sat as far from
its own row as from the previous one, so three leeches read as one block of six lines. And the
caption — the rank, which ADR-0010 §4 puts the screen's whole weight on — stated one rank key and
spent its other half on the review count, which orders nothing.

Measuring the screen added two faults that had numbers behind them: every row control was a bare
`ui.button`, which drew the `primary` fill ADR-0034 §2 allows a screen one of, nine times over, at
**19px** where the touch target is 36; and *Back to review* drew hard under the list with the rest
of the page empty, the reach line's third call site, which #155 had not listed.

## Decision

### 1. A row is the card, in a well, with its actions underneath

This is the one screen that lists **cards**, so the row is drawn in ADR-0033 §1's material: the
card's prompt sits in a well, and the well is the only fill on the screen, which is what says *this
is one thing*. The prompt is text, not a control, so **`Edit` is named** as a third control: the
price of the subject no longer being a button, paid visibly. The suspended section uses the same
row with no cost line, because a suspended card is not ranked.

### 2. The controls keep their words, at the ordinary weight and the touch target

Every control on the screen goes through `controls::*`: ordinary weight, 36px. No control here takes
`primary`, *Back to review* included — ADR-0034 §2's primary is for a screen with no card, and this
screen has one per row. The durable entrance on the caught-up floor keeps its primary, because that
screen still has none.

**Icons alone were drawn and lost.** This was the first list with a control on every row, so it was
the first build of #149's exception (*a control that appears on every row may stand as an icon
alone*). *Suspend* has no picture a reader already holds; a pause bar means *paused*, not *not shown
in review*. Repetition can teach where a known symbol sits; it cannot invent the symbol. The rule
that came out of this screen, the note list and the file surface is ADR-0041 §4's, and this screen
is its *words stay* case.

### 3. Eight inside a row, thirty-two between rows

The gap inside a row (well to its controls, prompt to its cost line) is `gap(1)`; between rows it is
`gap(4)`. The fault was the ratio: at 8 against 16 a list read as one block. Judged by dragging the
two distances against each other in the running application.

### 4. The cost line: the rank key, the reviews, and the time

Each active row's line inside the well reads:

> **6 bad days · 12 reviews · 4 minutes**

- **Failure days** first: the primary rank key (ADR-0010 §2, §4).
- **Reviews**: how many answers the card has taken.
- **Answer time**: ADR-0010 §6's concrete cost, the card's summed answer time
  (`CardState::answer_ms`), rounded to the nearest minute; *1 minute* in the singular, and *under a
  minute* below thirty seconds, because *0 minutes* reads as none.

**§6's own sentence was not taken whole.** *"22 reviews, 14 minutes, still failing"* ends on a claim
the app does not check: a card stays on the list for its failure days in the trailing window even
after its latest answer was a pass, and the fixture's leeches are exactly that case. So the line
states the time as a fact and leaves the verdict to the reader.

**The second rank key is not drawn, and that was chosen.** Two rows can carry the same failure days
and be ordered by which failed more recently; a candidate that drew both keys (*"6 bad days, the last
15 days ago · 4 minutes over 12 reviews"*) was in the sitting and lost to this one. The heading's
*worst first* is what states the order; the line states the cost. Neither the review count nor the
time ranks anything (ADR-0010 §3), and the fixture orders both against the rank so a picture cannot
suggest otherwise.

### 5. *Back to review* sits on the reach line

ADR-0038 §5 made ADR-0035 §1 a page rule, and this screen's last control is its third call site:
`frame::slack_above` places *Back to review* on the reach line rather than under the list.

## Consequences

- `leech_row`, `cost_line` and `BETWEEN_ROWS` in `crates/app/src/screens/review.rs` are the screen;
  `a_leech_row_draws_no_primary_and_its_controls_are_the_touch_target` guards §2 on drawn pixels in
  both palettes, and `the_cost_line_adds_the_answer_time_in_whole_minutes` guards §4's wording.
- The answer time is read on this screen only. It is never a rank key, a floor or an input to memory
  state (ADR-0010 §3).
- **The empty state was carried over, not judged.** *"Nothing is costing you a lot right now."* is
  ADR-0010 §4's plain statement and neither sitting looked at it in the new material.
- Answer time is whatever the log recorded, so a card answered with the phone put down mid-card shows
  minutes it was not studied. §6 accepts that noise on the list, and §3 is why it never goes further.
