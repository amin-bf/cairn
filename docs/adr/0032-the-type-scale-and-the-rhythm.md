# ADR-0032: The type scale and the rhythm — four sizes, one unit, and what a stated gap costs

- **Status**: Accepted
- **Date**: 2026-08-11
- **Resolves**: [Design Pass: The Type Scale and the Rhythm, Against Review](https://github.com/amin-bf/cairn/issues/132)
- **Related**: [ADR-0030 §1](0030-the-first-finish-pass-decisions.md) (colour named at exactly one
  site — the rule this ADR repeats for type, and §3's contrast floor, whose stated argument §5 has to
  restate), [ADR-0031 §1 §2](0031-the-page-frame.md) (the page frame; the same rule for layout, and
  the margin §3 exempts from the rhythm), [ADR-0002 §8](0002-the-card-model.md) (the restricted
  Markdown a card face renders, which is why `Monospace` is body-sized), [ADR-0012 §7
  §8](0012-the-note-authoring-experience.md) (`dir="auto"`, and the `bold` family the scale is drawn
  in), [ADR-0006 §10](0006-the-review-session-experience.md) (the finish pass this continues to
  discharge)
- **Evidence**: twenty-five captures of the implemented scale at the three judging widths in
  [`docs/design/typed-2026-08-11/`](../design/typed-2026-08-11/README.md), including the first
  right-to-left card face this repository has ever photographed. The direction was chosen in
  [#124](https://github.com/amin-bf/cairn/issues/124) as **variant E**, judged as a running sitting
  and preserved as the tag `prototypes/issue-124`.

## Context

The application named **no** type size and **no** spacing value. Every tier was stock egui — body and
button 13, small 9, heading 18 — and `item_spacing` was stock's `vec2(8.0, 3.0)`. Nothing chose any
of it.

Two consequences had been living in the tree unnoticed.

**The card face was drawn at button size.** `card_face` shared the `text` helper with every button in
the application, so the one surface whose whole job is to be read was set at 13px, the size of a
label.

**Seventy stated gaps were all wrong, by a constant nobody could see.** egui adds `item_spacing`
*before* any gap a caller states — its own documentation says a stated gap is *"in addition to the
`item_spacing` that is always added"*. So the source read as a tidy 4 / 8 / 12 grid and drew **7, 11
and 15**, which is a grid of nothing, while every horizontal row overran its column by 8 per gap.
That was measured rather than inferred: the two-column editor [#131](https://github.com/amin-bf/cairn/issues/131)
had just landed sat **8px wider than its own page frame**, off-centre with 80px of margin on the left
and 72 on the right, and nothing failed.

[#123](https://github.com/amin-bf/cairn/issues/123) had already found the shape of this — type is an
**ambient** role a screen resolves implicitly, spacing is ambient in principle but named at around
sixty literal call sites — and left open where a value lives when the renderer offers no slot for it.

## Decision

### 1. Four sizes, pinned as integers, and control text is an alias

| tier | logical px | what it is |
|---|---|---|
| `display` | **40** | the card face — the text actually being read |
| `heading` | **20** | screen and section titles |
| `body` | **15** | sentences, **and the text inside every control** |
| `small` | **12** | the footnote tier: the box badge, the interval preview, a field caption |

**They are four chosen numbers, not a ratio.** The scale *accelerates* — ×1.25 from small to body,
×1.333 to heading, ×2.0 to display — tight where the tiers must coexist in a sentence and dramatic at
the top where the card face has one job. That shape was chosen against the alternative rather than
fallen into: variant **A** was the principled one, *"a conservative 1.25 scale off a 15px body"*, and
A lost. A uniform 1.25 ladder cannot reach a 40px display tier without four rungs of headroom nothing
uses. `the_scale_accelerates_rather_than_holding_one_ratio` pins the shape so a later tidy-up into
one ratio fails a test.

**Control text is an *alias* of body, never a fifth constant.** Two identical constants are strictly
worse than one: they drift apart with nothing failing, and no test can tell a deliberate divergence
from a typo. As an alias, *"a control's label is prose-sized"* is a claim
`control_text_is_body_sized` holds, and a ticket that wants them different has to break it on
purpose. `Monospace` is body-sized for a different reason — it is the `` `code` `` face *inside* a
sentence ([ADR-0002 §8](0002-the-card-model.md)), so a size disagreeing with the prose around it
would break the line it sits in.

**`display` has exactly one caller, `card_face`.** A second means it has stopped meaning *the text
being read*.

### 2. The rhythm is **stated**: `item_spacing` is zero and every gap is a whole unit of 8

`cairn_app::spacing` names the unit and `gap(n)` applies it. The ambient is zeroed, so **the number in
the source is the number on the screen**, and width arithmetic like `(available - gutter) / 2` becomes
correct rather than approximately correct — which is what removes #131's 8px editor overrun as a
consequence rather than as a patch.

**This deliberately does not follow the ambient-role pattern** of ADR-0030 §1 and ADR-0031 §1, and the
reason belongs here because it otherwise reads as an inconsistency. Naming a value once only helps
when call sites can then stop naming values, and an ambient gap cannot do that for a rhythm of many
different gaps: with an ambient of 8, the sites wanting 16 must write `8` to reach it — every site
naming a number that is not the gap it wants — and a gap *smaller* than the ambient is not expressible
at all, because there is no negative space. Colour and the page frame could be made ambient. A rhythm
could not.

**The unit is 8 and `gap` takes a whole number, so a half-step will not compile.** A unit that permits
halves is a four-unit wearing an eight label, which is the same untruth as the invisible 3 this
decision removes. Eight rather than four because a unit earns its keep by *refusing* things, and at
the range of gaps this application draws a four-grid refuses almost nothing — variant A used four and
lost.

**What the zero costs, paid in one place each.** egui's own composites lose their internals' spacing,
so `spacing::row` and `row_wrapped` restore it for a row of controls, and `composite_spacing` is the
single named value a combo-box or scroll-area is given. And **every widget pair that was leaning on
the ambient 3px fuses.** That was not a small set — the three pass grades became one slab, the note
list became one block, the editor's two card previews became a single tall face, and the import
preview's statements ran together. Each now states its gap. They are listed because *"nothing chose
this"* was true of them too: a 3px separation nobody decided is not a design being preserved.

### 3. The page frame is a different value family and is exempt

[ADR-0031 §2](0031-the-page-frame.md)'s margin is **28**, which is not a multiple of the unit, and
that is correct rather than an oversight. A margin is the distance from content to the *window edge*,
judged against captures at three widths; a gap is the distance between two things *inside* the
column. The rhythm has no more claim on the margin than on the 640 measure or the 36px control
height, and bending 28 to fit a grid it was never judged against would re-open a decided ADR to buy
tidiness. `the_page_margin_is_deliberately_off_the_grid` pins the mismatch so it reads as this
decision rather than as a defect to clean up.

### 4. Both live in their own module and are installed into **every** theme slot

`cairn_app::typography` and `cairn_app::spacing`, one module per value family — because the
single-naming-site rule is only enforceable per family. *"A `Color32::from_rgb` outside `theme` is a
defect"* is a statement you can grep for; folded into one `tokens` module it becomes "no literals
outside tokens", which is true and no longer tells you what you are looking at.

**egui's `Style` is per-theme**, so `text_styles` and `spacing` sit in exactly the trap
[ADR-0030 §2](0030-the-first-finish-pass-decisions.md) records for `Visuals`: an untargeted
`set_style` writes to whichever slot is active at construction. Colour genuinely differs between the
two and was right to target the dark slot; **type and rhythm do not**, so both are written to *all*
slots through `all_styles_mut`. That makes the trap inapplicable rather than merely avoided, and the
light mode still sitting in [#121](https://github.com/amin-bf/cairn/issues/121)'s fog inherits both
instead of silently getting stock.

**The numbers are logical pixels — Android's `dp`, iOS's `pt`.** Nothing in §1 or §2 is expressed in
a unit egui owns, so the scale and the rhythm carry to a native client unchanged and only `install` is
rewritten. This matters concretely rather than in principle: the Android client is already committed
to leaving egui ([#121](https://github.com/amin-bf/cairn/issues/121), *Out of scope*), and it inherits
these values rather than re-deciding them.

**A named tier that was never installed panics — it does not fall back.** Unlike the built-in
variants, `TextStyle::Name` has nothing to resolve to, and that loudness is kept on purpose:
resolving defensively at the call site would draw the 40px card face at stock's 13 on any path that
skipped `install`, with nothing failing.

### 5. The 7:1 contrast floor keeps its number and loses its argument

[ADR-0030 §3](0030-the-first-finish-pass-decisions.md) argued the floor **from the type scale**: 7:1
*"because the small text style is 9px, where WCAG AA's 4.5:1 is already the marginal case this palette
exists to leave behind."* §1 raises small to 12, so that premise is gone.

**The floor stays at 7:1**, now as a floor the palette *chooses* rather than one a 9px tier forces.
Nothing about the palette changes and no measurement moves.

Two consequences are recorded rather than fixed. The **box badge gets louder** — it is `small` in
`weak_text_color()`, held below the floor at ~5.6:1 precisely so it reads as ADR-0030 §4's *"quiet
aside"*, and a third more type size works against that in the one place the palette went out of its
way to be quiet. And ADR-0030 §3's *"the small text style is 9px"* is now simply false as written.
Both belong to whatever re-judges the palette; [#133](https://github.com/amin-bf/cairn/issues/133)
owns where the badge sits and meets the first of them.

## Consequences

**A right-to-left card face was being drawn outside its own card, and the display tier is the only
reason anyone found out.** `bidi` sets `halign = RIGHT` as a *direction marker* and states that every
caller must reset it, because an RTL galley left that way spans **negative x**. `card_face` was the
one caller that never did, so the button centred a rect beginning left of zero. At 13px the overhang
was around 118px inside a 500px card and merely looked off-centre; at 40px it measured **−455px** and
ran clean off the window. The face now resets `halign` and sets a wrap width — a `LayoutJob` wraps at
`f32::INFINITY` by default, so a long face had never been asked to fit anything.

Nothing failed and nothing would have: **the seed collection is French, so no capture in this
repository had ever put right-to-left text on a card face.** The Persian storyboard grew a shot of the
card pane for exactly this reason, and `a_right_to_left_card_face_is_drawn_inside_its_card` pins it.

**The Persian storyboard had been missing its target since #131 merged, silently.** It clicked a
**literal** x=86 for the *Notes* tab; the page frame moved the nav 28px right, and the wider nav
labels of §1 finished the job, so the click landed on *Review* and the file photographed the Review
screen three times under three editor names. `baseline.txt` was migrated to `%LX+n%` when the frame
landed and this one was not. Both are now on the same token. This is [#122](https://github.com/amin-bf/cairn/issues/122)'s
finding for the third time — a storyboard cannot tell you it missed — and it is the reason every
coordinate in this pass was re-measured off a capture rather than adjusted by eye.

**Every stated gap in the application moved.** 4 → one unit, 8 → two, 12 → three, and the single stray
`10` with them. The result is close to the gap distribution variant E was judged with, where two and
three units dominate. It is also, net, less of an inflation than it looks: the ambient 3px that
disappeared everywhere was being paid between *every* pair of widgets, not only the seventy that
stated a gap.

**A capture of a screen this pass cannot reach still needed fixing.** The import preview is unreachable
by the harness — it needs a file *dropped* on the window, which synthetic input cannot produce — so its
fused statements were found by reading rather than by looking. Anything else in that position is in the
same danger.

**What this ADR deliberately does not touch.** The card ([#133](https://github.com/amin-bf/cairn/issues/133))
and the controls ([#134](https://github.com/amin-bf/cairn/issues/134)). The card face is still a 96px
box, which at 40px holds two lines and not three; whether that is the right height, and whether the
three pass grades become a segmented row rather than three buttons a unit apart, are theirs. The
palette is untouched, as it has been by every ticket in this slice.
