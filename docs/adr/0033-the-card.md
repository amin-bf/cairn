# ADR-0033: The card — one object, cut into the page, and what its content does to it

- **Status**: Accepted
- **Date**: 2026-08-11
- **Resolves**: [Design Pass: The Card — One Object or Two, and What a Card Is](https://github.com/amin-bf/cairn/issues/133)
- **Related**: [ADR-0030 §1 §3 §4](0030-the-first-finish-pass-decisions.md) (colour named at exactly
  one site — the rule §2 finds a case outside; the contrast floor, whose figures §5 corrects; and the
  box badge's *quiet aside*, which §3 is an argument about and §5 finally lands),
  [ADR-0031 §1](0031-the-page-frame.md) (the page frame the card is drawn inside),
  [ADR-0032 §1](0032-the-type-scale-and-the-rhythm.md) (the display tier, which §4 stops applying
  unconditionally), [ADR-0006 §3 §6](0006-the-review-session-experience.md) (the whole card is the
  reveal target; `new` is never a box number), [ADR-0001 §3](0001-scheduling-algorithm-and-grade-scale.md)
  (the badge may never read as a queue position), [ADR-0012 §1](0012-the-note-authoring-experience.md)
  (the editor's card pane draws cards *the way review draws them* — which §1 makes literally true),
  [ADR-0002 §8](0002-the-card-model.md) (the restricted Markdown a card face renders)
- **Evidence**: twenty-eight captures of a throwaway prototype that varies **only** the card, at both
  judging widths, preserved as the tag `prototypes/issue-133` with its readme at
  [`docs/design/prototype-133/README.md`](../design/prototype-133/README.md). Judged as pictures, in
  a review session, by the person who reads both scripts the application serves. The direction came
  from [#124](https://github.com/amin-bf/cairn/issues/124)'s **variant E** (`prototypes/issue-124`),
  and this ADR keeps its structure, replaces its material, and overturns one of its details.

## Context

A card was **two** slabs, each 96px tall, drawn on `widgets.inactive.bg_fill` with the box badge on
the page underneath them. Not one of those four properties was ever chosen: the height and the fill
are `card_face`'s literals from the walking skeleton, and the badge's position is where it landed
when the review screen was first assembled.

#124 photographed five Review variants and picked E, which drew **one** card with two halves divided
by a hairline, on a fill darker than the page, with the badge riding the card's own top-right corner.
It left the argument for that unrecorded, which is what this ticket was opened to settle.

Three things then turned up that were not on the ticket.

**The page is a colour nobody chose.** `eframe::App::ui`'s contract says the `Ui` it hands you *"has
no margin or background color"*. Cairn implements `ui`, overrides neither `clear_color` nor wraps its
content in a `CentralPanel`, and so drew **every screen** on eframe's own default —
`rgba(12, 12, 12, 180)`, compositing to `#080808`. `panel_fill` reached only the nav strip and the
inset bands, which is why the strip was visibly *lighter* than the page below it, on every capture
this repository holds.

That is not a cosmetic detail here: `#080808` sits **below every rung of the stone ramp**, so
`STONE_0` measures **1.07:1** against it. E's well, drawn on the page the application actually had,
is not a hole at all — it is a *raised* surface, one rung up, and invisible besides. **The card
question could not be answered without settling the page first.**

**The card had only ever been drawn with one short Latin word on it.** Every capture in
`docs/design/prototype-124/` uses `chien`/`dog`. A cloze note's `Text` is a paragraph and nothing
stops it, so the fixed height and the centred layout were both untested against the content the
application can actually be handed — as was the display tier, which at 40px turns a paragraph card
into the entire 560×860 window with *Edit note* off the bottom.

**ADR-0030 §4's lower-case badge never landed.** §4 decided the badge reads `box 3` and `new`, and
states that *"#115 renders `box 3`"*. `box_badge_wording` had returned `format!("Box {box_}")` since
the day it was written, with a test pinning the capital. The application had contradicted its own
accepted ADR in writing, and nothing failed, for as long as the function existed.

## Decision

### 1. A card is **one object with two faces**, divided by a hairline

The prompt and the answer are two halves of one card, separated by a centred rule a quarter of the
card's width — never two cards, and never a full-width rule, which reads as a division between two
stacked objects rather than a fold in one.

> **A card is one surface. The reveal adds a face to it; it does not add a card.**

The argument #124 left unrecorded is *two faces of one thing, not two things*, and it is true but
weak — it restates the conclusion. The captures produced a stronger one that is simply visible:

**With two objects, the badge has to pick one of them.** The badge reports the durability of *the
card*. Draw two slabs and it can only sit on one, where it reads as belonging to the answer. Its
referent is ambiguous in the two-object arrangement and unambiguous in the one-object one. That is
not a matter of taste, and it decides the question without appeal to how either looks.

This also makes [ADR-0012 §1](0012-the-note-authoring-experience.md)'s *"drawn the way review draws
it"* **literally** true rather than approximately: the editor's card pane and the review screen now
call one function, so the material cannot drift between the two screens the way two independent
`card_face` calls could.

### 2. A card is a **well**: darker than the page — and the page is the palette's

> **The page is `panel_fill`. A card is `STONE_0` with a `STONE_4` edge and an 8px corner.**

Two halves, and the first is a precondition for the second. `CairnApp::clear_color` now returns
`visuals.panel_fill`, so the page is the palette's on every screen and the nav strip stops being
lighter than the page it sits on.

**The defence is to take the default over, not to name a new value.** ADR-0030 §1 put colour behind a
single naming site because a literal elsewhere *"renders fine to the author and drifts the palette one
screen at a time, with nothing failing"*. This is that defect in a form the rule does not reach: the
value was supplied by the **renderer**, so nothing in this crate was wrong and the screen was still a
colour nobody picked. Returning `panel_fill` keeps the page coming from `theme` and creates no second
literal to drift. The general question — what else the app inherits by default and has never looked
at — is #121's, not this ADR's.

The card's own colours are named in `theme` (`card_fill`, `card_stroke`, `card_divider`) and its
shape in `surface`, because `theme` remains the only module in this crate that may name a colour and
a card is not an exception to it.

**The card shares a fill with a text field, and that is accepted rather than overlooked.** `STONE_0`
is `extreme_bg_color`. A well therefore means *content*, not *editable*; the two are told apart by an
8px corner against the widget's 2px, and by never appearing on the same screen. If they ever must
diverge, `theme::card_fill` is the one line that changes.

**What was rejected, and why it is close.** An **outline** card — no fill, the page showing through —
is very nearly the same picture, because the dark end of the stone ramp is compressed enough that
`STONE_0`-on-`panel_fill` is only 1.12:1. It was rejected for what it costs later rather than for how
it looks: a card with no material at all, under controls that have one, leaves §3 nowhere to go.

### 3. The controls must end up **quieter than the card**

> **A card outweighs the controls beneath it. This binds
> [#134](https://github.com/amin-bf/cairn/issues/134).**

Blurred until nothing on the screen is legible — a crude test, and the only one that measures what
the eye reaches for first — today's filled grade buttons are the heaviest mass on the Review screen
and *every* candidate card is lighter than they are. The buttons sit at 1.54:1 against the page; the
card, as a well, at 1.12:1. Making the card recede therefore makes the problem **worse** unless the
controls come down with it, and no choice of card fill fixes that on its own.

The prototype drew the pair (`PROTO_CONTROLS=quiet`: the same controls, same size, same hit targets,
their fill removed and a 1px edge kept), and with it the card becomes the dominant shape and the two
brightest points on the screen are the words being studied. That treatment is **not decided here** —
the controls are #134's, and this ADR deliberately does not spend that ticket's room. What is decided
here is the **constraint** it inherits, because §2 is a bet on it: a Review screen whose controls
outweigh its card has failed this ADR, whatever the controls end up looking like.

### 4. A card face steps **down** the scale to fit, and stops at body

> **The face is drawn at the largest tier it fits in — display, then heading, then body — and never
> smaller than body. A card that will not fit at body grows instead, and the page scrolls.**

[ADR-0032 §1](0032-the-type-scale-and-the-rhythm.md) gave the card face the display tier and named it
*"the text actually being read"*. That holds for a word and fails for a paragraph: at 40px in the
application's own window, a paragraph card is the whole screen and the grade buttons are below the
fold. The display tier is the size of **a word being studied**, not of everything a card can hold.

Both halves of the rule carry weight. Without the step-down the screen reflows around its content.
Without the floor, a long enough card shrinks until nothing is readable — trading a visible failure
for a silent one — and a card face set smaller than ordinary prose has stopped serving the reader it
was sized for. Body is therefore the floor, and past it the card takes the room instead.

A consequence worth stating because it falls out of the layout rather than being asked for: **a face
that fits on one line is centred, and a face that wraps is left-aligned within the centred block.**
That is the right outcome — a centred paragraph is hard to read — but it is an emergent property, and
the next person to touch the layout should know it was noticed rather than intended.

### 5. The badge rides the card's **quiet corner**, and which corner that is follows the script

> **The badge sits inside the card, in the corner opposite the one reading begins at: top-right for
> left-to-right, top-left for right-to-left. The *prompt's* direction governs, for the card's whole
> life on screen.**

It belongs to the card and moves inside it, because a footnote on the page below could as easily
belong to the screen, and the badge is a fact about one card.

**A fixed corner cannot hold [ADR-0030 §4](0030-the-first-finish-pass-decisions.md).** §4 requires the
badge stay a *"small, non-interactive footnote … quiet aside"*. Top-right is the quiet corner of a
Latin card and the corner a Persian reader's eye **starts** from — so the same placement is a
footnote in one script and the first thing seen in the other, in an application built to be used in
both. Mirroring keeps the *rule* fixed (the corner reading does not begin at) and lets only the
geometry follow the text, which is the same move `dir="auto"` already makes for the text itself
([ADR-0012 §7](0012-the-note-authoring-experience.md)).

**The prompt governs, never the answer and never "whichever face is showing".** A Persian prompt with
an English answer is an ordinary vocabulary card; deciding the corner per visible face would make the
badge jump sides at the reveal — the one moment the reader is looking hardest at something else.

**And the badge is finally lower case.** ADR-0030 §4 decided `box 3` / `new` and recorded that #115
had shipped it. It had not. This lands it, and §4's sentence about #115 is false as written and
should be read as the intent it states.

## Consequences

### What this changes

| Document | Change | Why |
|---|---|---|
| [ADR-0030 §1](0030-the-first-finish-pass-decisions.md) | **Extended, not overturned.** The single-naming-site rule governs colours this crate *names*; §2 above adds the case where the **renderer** supplies one by default, and answers it by taking the default over rather than by naming a value. | The page was a colour nobody chose, on every screen, and §1 as written could not have caught it. |
| [ADR-0030 §3](0030-the-first-finish-pass-decisions.md) | **Its figures are corrected.** Every ratio in §3 is computed against `panel_fill`, which the application did not draw. Measured on the real page, body was **15.92:1** where §3 records 13.34:1, and weak text **6.67:1** where it records 5.59:1. §2 above makes §3's figures true rather than aspirational, by making the app draw the surface they were computed on. | The floor was never breached — the real page was *higher* contrast — but §3 described a surface that did not exist. |
| [ADR-0030 §4](0030-the-first-finish-pass-decisions.md) | **Landed, and the placement decided.** The lower-case badge ships. §4's *"#115 renders `box 3`"* was never true. §5 above settles *where* the badge sits, which §4 left open. | §4 decided case and face and was never implemented; the badge is louder than §4 assumed (12px since #132, and on a page two rungs darker than the figures allowed for), so its placement now carries more of the quietness than its colour does. |
| [ADR-0032 §1](0032-the-type-scale-and-the-rhythm.md) | **Narrowed.** The display tier is the card face's *maximum*, not its size. §4 above steps down to heading and then body when the content does not fit. | The tier was chosen against `chien`/`dog`, the only card content ever photographed. |
| [ADR-0012 §1](0012-the-note-authoring-experience.md) | **Made literal.** "Drawn the way review draws it" is now one function serving both screens, not two similar call sites. | The material can no longer drift between the editor and review. |
| [ADR-0006 §3](0006-the-review-session-experience.md) | **Unchanged, and now better served.** The whole card is the reveal target, taken over the surface's rect rather than by making it a button. | A card is a surface, not a control that happens to be large. |

### What this costs

- **A card and a text field share a fill.** Told apart by corner radius and by context (§2). If that
  proves insufficient, `theme::card_fill` is one line — and the unused `STONE_1` rung in the design
  project's ramp, which this crate has never taken, is the obvious next value.
- **#134 inherits a constraint it did not choose** (§3). That is the point, but it is a real
  narrowing: the controls ticket can no longer decide its weight independently of the card.
- **The card face has three possible sizes**, so "the card face is 40px" is no longer a true sentence
  about the application. `surface` is the one place that decides which, and the floor is pinned by
  test.
- **The lower-case badge is a visible change to shipped copy** that no ticket asked for. It discharges
  ADR-0030 §4 rather than deciding anything new.

### What this does not settle

| Question | Whose |
|---|---|
| What the grade controls actually look like, given §3's constraint | [#134](https://github.com/amin-bf/cairn/issues/134) |
| Whether the card's fill should leave `extreme_bg_color` for a rung of its own | The palette, if §2's corner-radius distinction proves too thin |
| Every other value inherited from an egui or eframe default and never looked at | [#121](https://github.com/amin-bf/cairn/issues/121)'s fog — §2 found one and did not go looking for the rest |
| Whether a card should ever scroll internally rather than growing | Not yet a question; §4's floor makes it reachable only for very long content |
| Light mode, where a "well" must invert to stay a well | [#121](https://github.com/amin-bf/cairn/issues/121)'s fog |
