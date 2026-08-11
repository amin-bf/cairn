# What a card is — twenty-five captures

The primary source for [#133](https://github.com/amin-bf/cairn/issues/133), the third slice of the
design pass ([#121](https://github.com/amin-bf/cairn/issues/121)): twenty-five captures of a
throwaway prototype that varies **only the card**, inside the foundations
[ADR-0031](../../adr/0031-the-page-frame.md) and [ADR-0032](../../adr/0032-the-type-scale-and-the-rhythm.md)
already fixed.

**This never merges into `main`.** It is preserved as the tag **`prototypes/issue-133`**, contained
in no branch — the repo's standing convention (`AGENTS.md`, *Rules that are easy to break silently*
3), whose predecessors are `prototypes/issue-8`, `-11`, `-20`, `-28`, `-67`, `-120`, `-124` and
`-131`. A tag is fetched by every clone, so a later session reads this without merging anything:

```sh
git show prototypes/issue-133:docs/design/prototype-133/README.md
git checkout prototypes/issue-133 -- crates/desktop/src/bin/card-prototype.rs
```

## What produced these

`crates/desktop/src/bin/card-prototype.rs`, driven by `scripts/capture-card.sh`:

```sh
cargo build -p cairn-desktop --bin card-prototype
PROTO_PAGE=panel PROTO_CARD=well PROTO_BADGE=corner PROTO_CONTENT=fa-sentence \
PROTO_HEIGHT=grow PROTO_SCREEN=revealed scripts/capture-card.sh my-name 1280 800
```

Six axes, all in the environment, so one one-line storyboard serves every combination:

| var | values |
|---|---|
| `PROTO_PAGE` | `shipped`, `panel` |
| `PROTO_CARD` | `today`, `well`, `raised`, `outline`, `two` |
| `PROTO_BADGE` | `corner`, `below` |
| `PROTO_CONTENT` | `word`, `sentence`, `long`, `fa-word`, `fa-sentence`, `markdown` |
| `PROTO_HEIGHT` | `grow`, `fixed` |
| `PROTO_SCREEN` | `question`, `revealed`, `live` |

`PROTO_SCREEN=live` runs a sitting with a hand on the mouse, which is the only way to see the one
thing a still cannot show: whether the prompt **moves** underneath you at the moment of reveal.

## What is held constant, and why that inverts #124's method

#124's prototype gave every variant a **complete, coherent token set**, because the foundations were
open and a hero layout and a dense layout do not want the same scale. They are open no longer. So
this prototype goes the other way and draws through the application's own modules — `frame::column`,
`typography::display`, `spacing::gap`, `theme::cairn_dark`, `bidi::markdown_job`, the real font set.
Every candidate is the *same* screen with the *same* foundations, differing only in what the card is
made of.

The question is no longer "which of five worlds", it is "what is a card, inside the world already
decided" — and holding everything else still is what makes an image a statement about the card.

## Four findings

### 1. The page is a colour nobody chose, and it makes a well undrawable

**`STONE_0` measures 1.07:1 against the page the application actually draws.** That is invisible,
and it means variant E's card — *"a hole you read into rather than a button sitting on top of it"* —
is on the shipped page a **raised** surface, one rung up.

The cause is not in the palette. Cairn implements `eframe::App::ui`, whose contract states that the
`Ui` it hands you *"has no margin or background color"*. Cairn overrides neither `clear_color` nor
wraps its content in a `CentralPanel`, so the page on **every screen** is eframe's default —
a hard-coded `rgba(12, 12, 12, 180)`, compositing to `#080808`. `panel_fill` reaches only the nav
strip and the inset bands, which is why the nav strip is visibly *lighter* than the page beneath it.

`#080808` sits below every rung of the stone ramp. This is precisely
[ADR-0030 §1](../../adr/0030-the-first-finish-pass-decisions.md)'s defect — a colour drifting in with
nothing failing — arriving from **outside** the crate, where a rule about naming colours in one
module cannot reach it. It also means §3's recorded measurements describe a surface the app does not
draw: body-on-panel is quoted at 13.34:1 and is really 15.92:1, and weak text is quoted at 5.59:1 and
is really **6.67:1** — so the box badge is *louder* than §4's quiet-footnote requirement was reasoned
against, on top of the 9→12px rise #132 already recorded.

`a2-well-shipped` and `a3-well-panel` are the pair. Nothing else about them differs.

### 2. `outline` is the one candidate that does not depend on that being fixed

A card with **no fill at all** — the page showing through, a 1px stroke drawing the edge — reads the
same on either page, because it takes its interior *from* the page rather than from a rung of the
ramp. `a6-outline-shipped` and `a7-outline-panel` are near-identical; `a2` and `a3` are not.

That is a robustness argument, not an aesthetic one, and it is worth separating from the question of
whether an outline card is the one that looks right.

### 3. The display tier is a size for a word, not for a paragraph

Every capture in `docs/design/prototype-124/` used `chien`/`dog`. A cloze note's `Text` is a
paragraph and nothing stops it, and at 40px a paragraph card is the whole window:

- `e2-well-long-560` — at the app's own default window the card fills the screen, *Edit note* is off
  the bottom, and the grade row is only just in. The page scrolls, so nothing is unreachable; what
  is gone is the ability to see the answer and the choice about it at once.
- `e4-well-long-fix-560` — the same content with the face stepping down a tier until it fits. The
  whole screen survives, at the cost of the card face being drawn at body size, which is the size of
  a button label.

Neither is obviously right, which is why both are photographed. What is *not* in question is that
one unconditional 40px does not survive the content the app can be handed.

A smaller observation from the same set: a wrapped face ends up **left-aligned within a centred
block**, which happens to be correct — a centred paragraph is hard to read — but it happens by
accident of the layout rather than by decision, and it means "the card centres its face" is only
true of a face that fits on one line.

### 4. The corner is not a quiet place in both directions

Persian renders correctly inside the card, mixed-direction cards work, and the −455px overhang #132
found is gone (`c3-well-fa-word`, `c4-well-fa-sentence`).

But **top-right is where a right-to-left reader's eye starts.** The badge rides there because that is
the quiet corner of a left-to-right card; on a Persian card it is the most prominent position on the
surface. [ADR-0030 §4](../../adr/0030-the-first-finish-pass-decisions.md) requires the badge stay a
*"small, non-interactive footnote … quiet aside"*, and a placement whose quietness depends on the
content's direction cannot hold that on its own. The choices are to mirror the badge with the face's
direction — which makes it move card to card — or to put it back on the page, where the page's own
direction governs and one placement serves both.

`b1-well-panel-below` is the page placement for comparison.

## The two-object question, as photographed

`a8-two-panel` is the same material as `a3-well-panel` with the single object split in two. Two
things are visible there that no argument produced:

- the two slabs read as two unrelated surfaces stacked, rather than as one card with two faces; and
- **the badge has to pick one of them.** It belongs to the *card*, and with two boxes on screen it
  can only sit on one, where it reads as belonging to the answer.

The second is the sharper point, because it is not a matter of taste: the badge's referent is
ambiguous in the two-object arrangement and unambiguous in the one-object one.

## The index

At `1280x800` unless noted.

| | |
|---|---|
| `a1-today-shipped` | The control — what `main` draws today, inside the landed frame |
| `a2-well-shipped` / `a3-well-panel` | The well, on each page. **Finding 1.** |
| `a4-raised-panel` / `a5-raised-shipped` | One object, today's material |
| `a6-outline-shipped` / `a7-outline-panel` | One object, no material. **Finding 2.** |
| `a8-two-panel` / `a9-two-shipped` | Two objects, the well's material |
| `b1-well-panel-below` / `b2-outline-ship-below` | The badge on the page instead of the corner |
| `c1`–`c5` | The well against sentence, paragraph, Persian word, Persian sentence, Markdown |
| `c6`, `c7` | The outline against paragraph and Persian sentence |
| `c8-two-long` | Two objects against a paragraph |
| `d1`, `d2` | The step-down height policy at 1280 |
| `e1`–`e4` (560×860) | The app's own window: word, paragraph, Persian sentence, and the step-down |

## Reading them

Look at the images. #124 recorded that a storyboard cannot tell you it missed and proved it twice;
#132 then found a card face drawn 455px off the window with nothing failing and no capture that
would ever have shown it. Every image here was looked at.
