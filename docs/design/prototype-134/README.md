# What the controls are — fifty-five captures

The primary source for [#134](https://github.com/amin-bf/cairn/issues/134), the fourth and last
slice of the Review vertical on the design pass map
([#121](https://github.com/amin-bf/cairn/issues/121)): captures of a throwaway prototype that varies
**only the controls**, inside the frame [ADR-0031](../../adr/0031-the-page-frame.md), the scale
[ADR-0032](../../adr/0032-the-type-scale-and-the-rhythm.md) and the card
[ADR-0033](../../adr/0033-the-card.md) already fixed.

**This never merges into `main`.** It is preserved as the tag **`prototypes/issue-134`**, contained
in no branch — the repo's standing convention (`AGENTS.md`, *Rules that are easy to break silently*
3), whose predecessors are `prototypes/issue-8`, `-11`, `-20`, `-28`, `-67`, `-120`, `-124`, `-131`
and `-133`. A tag is fetched by every clone, so a later session reads this without merging anything:

```sh
git show prototypes/issue-134:docs/design/prototype-134/README.md
git checkout prototypes/issue-134 -- crates/desktop/src/bin/controls-prototype.rs
```

## What produced these

`crates/desktop/src/bin/controls-prototype.rs`, driven by `scripts/capture-controls.sh`:

```sh
cargo build -p cairn-desktop --bin controls-prototype
PROTO_SCREEN=revealed PROTO_GRADES=row PROTO_WEIGHT=quiet PROTO_PREVIEW=small \
  scripts/capture-controls.sh a3-quiet-row 1280 800
```

Nine axes, all in the environment, so one one-line storyboard serves every combination:

| var | values |
|---|---|
| `PROTO_SCREEN` | `revealed`, `picker`, `caughtup`, `pointer`, `checkpoint`, `live` |
| `PROTO_GRADES` | `stacked`, `row`, `row4`, `rowplus` |
| `PROTO_WEIGHT` | `solid`, `faint`, `quiet` |
| `PROTO_PREVIEW` | `same`, `small`, `none` |
| `PROTO_ENTRANCE` | `counts`, `primary`, `primarylink`, `plain` |
| `PROTO_EMPTY` | `sentence`, `centred`, `bare`, `display` |
| `PROTO_CHECKPOINT` | `replaces`, `over`, `compact` |
| `PROTO_PRIMARY` | `quiet`, `filled` |
| `PROTO_EDIT` | `control`, `tertiary` |

**The last three were added during the judging**, because looking at the first round raised questions
it could not answer. `PROTO_PRIMARY` and `PROTO_EDIT` are findings 4 and 5 below; `compact` is
finding 7's.

## What is held constant

Everything the three ADRs before this one fixed, drawn through the application's own modules —
`frame::column`, `typography`, `spacing::gap`, `theme::cairn_dark`, and **`surface::card` itself**.
The card in these images is the shipped card, not an approximation of it, and that matters more here
than in any prototype before: [ADR-0033 §3](../../adr/0033-the-card.md) states this ticket's
constraint as a *comparison* — the controls must end up quieter **than the card** — so a prototype
drawing its own card would be measuring against something the application does not have.

## Nine findings

### 1. §3's two numbers were measured on two different pages

ADR-0033 §3 records the grade buttons at **1.54:1** against the page and the card at **1.12:1**, and
concludes the controls are the heaviest mass on the screen. Both figures are correct and they are not
comparable: 1.54 is `STONE_5` against eframe's `#080808`, and **§2 of that same ADR abolished
`#080808`**. The card's 1.12 is against `panel_fill`, the page that replaced it.

On the page the application now draws, every figure moves:

| | vs `#080808` (the old page) | vs `panel_fill` (the page today) |
|---|---|---|
| `STONE_5` — a control as it ships | 1.544:1 | **1.293:1** |
| `STONE_3` — `faint_bg_color` | 1.312:1 | **1.099:1** |
| `STONE_0` — the card, as a well | 1.065:1 | **1.121:1** |
| a 1px `STONE_4` edge, no fill | 1.458:1 | 1.222:1 |

So §3's gap between the controls and the card is **0.17, not 0.42**, and fixing the page did most of
§3's work before this ticket opened. The conclusion survives — the controls are still the heavier
mass — but by much less than the ADR that binds this ticket states, and the correction is owed in
the same way ADR-0033 §5 owed one to ADR-0030 §3.

### 2. `faint` already satisfies §3 with a fill intact — *outline or slab* was a false pair

`faint_bg_color` measures **1.099:1** against the page. The card measures **1.121:1**. A control
filled with `STONE_3` is therefore *already quieter than the card*, without giving up being a filled
surface at all. §3 photographed `quiet` — no fill, a 1px edge — and drew the right conclusion from
it, but it had drawn only the two ends.

`a2-faint-row` and `a3-quiet-row` are the pair, and the honest report is that **they are very nearly
the same picture**: the dark end of this palette is compressed enough that a 1.099:1 fill and a
1.000:1 absence read alike. What differs is what each costs elsewhere — see finding 4.

### 3. The arrangement does not fix §3. The material does.

`a1-solid-row` is #124's chosen arrangement — *Forgot* apart, three passes segmented, preview demoted
— in today's material. The card still loses. Freeing vertical space and grouping the passes are
worth having and they do not touch the weight question at all; only `faint` and `quiet` invert it.

`a0-today` is the control: what `main` draws.

### 4. §3 is a **relationship**, and applying it as a **material** guts the screens with no card

This is the finding round one produced rather than tested, and it is the reason `PROTO_PRIMARY`
exists.

The picker and the caught-up screen have no card. There is nothing for §3's comparison to be about,
and giving every control the same outline leaves a page whose only mass is a faint rectangle that
reads as **disabled**. `d2-primary` is that page: *Start — all 6* is the one thing to do on the
screen and it is drawn like a grade.

`d6-primary-filled` is the same screen with the single primary keeping its fill. The rule that falls
out is not a treatment but a sentence:

> **A control is quieter than the card on any screen that has one. On a screen with no card, the one
> control that is the way forward keeps its fill.**

### 5. At the quiet weight, *Edit note* becomes a fifth grade

`a3-quiet-row` puts five identical rectangles on the screen, four of which commit a grading and one
of which does not. The shipped screen hides this: its grades and its *Edit note* are told apart by a
gap and by nothing else, and solid fill plus a gap is just enough.

`h1-edit-tertiary` draws it frameless at the same height — the hit target is unchanged, and the
cluster is visibly four things with an aside under it. `h3-faint-tertiary` is the same at the faint
weight.

### 6. The row survives a fourth pass grade, at both widths

The ticket asks. The answer is yes, and it is not close:

| | at 1280 (measure 640) | at 560 (column 504) |
|---|---|---|
| three passes in a row | 208px each | 163px each |
| four passes in a row | 154px each | **118px each** |

`b4-quiet-rowplus` at 560×860 carries `Trivial` and its `7d` inside 118px with room to spare. So the
segmented row is **not** a constraint on the grade scale, and a future scale change does not have to
re-open this arrangement. `b3-quiet-row4` is the arrangement #124 rejected — all four in one row,
*Forgot* not held apart — drawn so the rejection is visible rather than remembered.

### 7. The shipped 10-minute checkpoint contradicts ADR-0006 §1, and nothing fails

[ADR-0006 §1](../../adr/0006-the-review-session-experience.md) says the checkpoint surfaces
*"without hiding the card underneath — the reviewer can still grade what they're looking at while
deciding"*, and calls the timer *"a courtesy check-in, not an enforcement mechanism"*.
`screens/review.rs` draws it as an `else if` branch that **replaces the card entirely**. This is the
same class of defect as ADR-0030 §4's badge case: the application contradicts its own accepted ADR
in writing, and no test could notice.

`f1-check-replaces` is what ships. `f2-check-over` is the literal fix — and it costs two full-width
slabs above the card and pushes it 140px down the page, which is how an application draws an
enforcement. `f3-check-compact` keeps the same guarantee as one line of small text and two frameless
actions: the card barely moves, and the check-in reads as a check-in.

### 8. #124's entrance, drawn as it actually was, gives a dormant accent its first caller

Variant E's *"or a shorter sitting: 5 10 20"* sets the three numbers in the **link** accent.
[ADR-0030 §5](../../adr/0030-the-first-finish-pass-decisions.md) keeps warn, error and link defined
and unreachable *precisely* so that nobody finds a call site for one because the colour exists — and
a sitting size is not a hyperlink.

`d3-primarylink` is E as drawn; `d2-primary` is the same line in weak text. The trade-off is real
rather than rhetorical: in weak text at the small tier, the second line is very nearly invisible, so
refusing the accent costs the sizes most of their discoverability. `d4-plain` is the third answer —
no size choice at all — which is the ticket's *"is the count picker the right entrance"* made a
picture. `d1-counts` and `d5-counts-solid` are today's.

A smaller defect in passing: the second line offers **5, 10 and 20 against a queue of 6**. The
shipped `count_buttons` caps its options by what is available; whatever lands here has to cap too, or
the line states work that does not exist.

### 9. The caught-up screen has no tier between heading and display

#124's variant E set *"All caught up."* at `display * 0.6` = 24px. ADR-0032 fixed four sizes with
nothing between 20 and 40, so 24 is not a size the application has.

At `HEADING` (`e2-empty-centred`) the screen's entire content is the same size as the word *Review*
three lines above it. At `DISPLAY` (`e4-empty-display`) it owns the screen — but ADR-0032 §1 calls
the display tier *"the text actually being read"*, and this is a status line, not a card face. It is
the scale meeting a case it was not chosen against, which is the same shape as the card's step-down
in ADR-0033 §4 and it is not obviously the same answer.

`e3-empty-bare` is the screen **without** the durable leech entrance — what three of #124's five
variants drew, with nothing failing. On a caught-up Review that entrance is the only control on the
screen (ADR-0010 §6, §8), so this is a picture of the state losing its last affordance.

## One more, on the pointer

`g1-pointer-quiet` and `g2-pointer-solid` draw the end-of-session pointer as it ships: two
full-width controls, *Show me* and *Not now*, at identical weight. ADR-0010 §6 requires the pointer
be *"never a decision point itself"*, and two equal slabs is exactly how an application draws a
decision point. Nothing here proposes a fix; it is photographed so the next reader sees it.

## The index

At `1280x800` unless noted; `560x860` is the application's own window.

| | |
|---|---|
| `a0-today` | The control — what `main` draws |
| `a1-solid-row` | #124's arrangement in today's material. **Finding 3.** |
| `a2-faint-row` / `a3-quiet-row` | The two candidate materials. **Findings 1, 2.** |
| `b1-quiet-stacked` | The quiet weight on today's stacked arrangement |
| `b3-quiet-row4` / `b4-quiet-rowplus` | Four in a row; a fourth pass grade. **Finding 6.** |
| `c1-preview-same` / `c3-preview-none` | The interval preview at button size, and gone |
| `d1-counts` / `d5-counts-solid` | Today's entrance |
| `d2-primary` / `d3-primarylink` | #124's entrance, in weak text and in the link accent. **Finding 8.** |
| `d4-plain` / `d7-plain-filled` | No size choice at all |
| `d6-primary-filled` | The primary keeping its fill. **Finding 4.** |
| `e1`–`e4` | The caught-up screen: today, centred, without its entrance, at display. **Finding 9.** |
| `f1`–`f3` | The checkpoint: what ships, the literal fix, the compact one. **Finding 7.** |
| `g1`, `g2` | The end-of-session pointer |
| `h1-edit-tertiary` / `h3-faint-tertiary` | *Edit note* as an aside. **Finding 5.** |
| `h2-row-48` (560 only) | The row at #124's 48px control height |

## Reading them

Look at the images. This set proved the rule again on its first run: with no `mousemove` in the
storyboard the cursor rests at the centre of the output, which at 560×860 is **on the first grade
control** — so every 560 image in round one photographed *Forgot* with its hover stroke lit, under
names claiming to be about the resting state. Nothing failed; the count was right; the images were
wrong. The storyboard now parks the pointer at `4 4` before it shoots.
