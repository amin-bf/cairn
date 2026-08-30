# Motion and elevation — the prototype for #154

Round one for [#154](https://github.com/amin-bf/cairn/issues/154), preserved as the tag
`prototypes/issue-154` and contained in no branch (`AGENTS.md`, *Rules that are easy to break
silently* 3).

```sh
git show prototypes/issue-154:docs/design/prototype-154/README.md
git checkout prototypes/issue-154 -- crates/app/src/proto.rs
```

**Nothing here is decided.** The reveal cannot be settled from a still, so what this produces is a
thing to sit in front of, a set of pictures for the half that *can* be photographed, and four
measurements the ticket asked for and one it did not.

---

## What the build gives you

The shipped application, varied behind `crates/app/src/proto.rs`, switched from the top of Settings —
the shape [#141](https://github.com/amin-bf/cairn/issues/141) used, for the same reason: the question
only exists in time, and a `cairn-desktop` bin driven by a capture script cannot ask it.

```sh
cargo run --release -p cairn-desktop
```

The switcher sits **directly under the Settings heading**, above Appearance. That position is forced
rather than preferred, and the reason is the third finding below.

| axis | control | opens at |
|---|---|---|
| the reveal | five candidates, A–E | **C** |
| duration | a drag knob, 0–600ms, live readout | **200ms** — `Style::animation_time`'s stock 0.2 |
| curve | six of `emath::easing`'s twenty-two | **cubic_out** — the only curve egui itself picks |
| hold | pin the transition at 0 / 25 / 50 / 75 / 100% | running |
| the overlay | five materials, 1–5 | **5**, the full proposal |
| shadow | a drag knob per theme, 0–255 alpha | **96 dark / 25 light** — stock's own |

Appearance still switches the theme, so every axis is judged in both.

---

## Five findings, in the order they arrived

### 1. The prompt already moves 42px when the card is revealed

The ticket says the card "already cannot" twitch, because `surface::REVIEW_HEIGHT` is a 300px floor.
That is true of the **card** and false of everything drawn inside it.

`surface::card` centres its content on the card's centre line, and the content grows at the reveal —
by the answer face, two 24px face gaps and the hairline, less the badge line arriving with them. So
the prompt is centred in 300px before the tap and in the top half after it. Measured through
`run_ui`, at both judging widths:

| | prompt's y | |
|---|---|---|
| before the tap | 128.0 | |
| after the tap | 86.1 | **−41.9** |
| the answer's arrival alone | | −47.5 |
| the badge's arrival alone | | +5.6 |

So the reveal does not merely *read* as a jump-cut. It **is** one, under
[#149](https://github.com/amin-bf/cairn/issues/149)'s rule in as many words: *motion may change what
is on the screen, it may never change where it is.* The rule was written to bind this moment and the
moment already broke it.

**It is content-dependent, which is why nobody has seen it.** A card whose content already overflows
the 300px budget has no centring space to redistribute, so a paragraph card moves **+4.1px** — the
badge line alone. The jump belongs to *short* cards, and the seed collection is six French words:
every capture in this repository is of the fixture that shows it worst, and a still cannot show a
jump at all. This is #124's *"a storyboard that misses fails silently"* one layer down — the harness
was aimed correctly and photographed a defect it structurally could not record.

**It decides what a fade has to be**, which is why it is finding one rather than a footnote. An
opacity fade laid over today's layout leaves the prompt jumping on frame zero while the answer fades
in behind it: the movement the rule forbids, wearing the motion the rule asks for. That is
[#143](https://github.com/amin-bf/cairn/issues/143)'s shape exactly — *the rule passes while the
values fail* — and it is candidate **B**, drawn so it can be seen rather than argued.

### 2. Twelve frames per transition is right, and it was hiding a defect

[#123](https://github.com/amin-bf/cairn/issues/123) computed that tessellation is not cached between
frames, so a 0.2s transition is roughly twelve full layout-and-tessellation passes over the whole
viewport, and said explicitly that nothing was run. It has now been run, in **release**, at
1280×800, through `eframe`'s own `cpu_usage` — which is `App::ui` plus rendering, excluding the vsync
wait — split into review-card frames with a transition in flight and review-card frames without.

| | frames | mean CPU |
|---|---|---|
| a review frame with a transition in flight | 12 per reveal | **0.51 ms** |
| a review frame at rest | | **0.74 ms** |

**#123's arithmetic is confirmed**: twelve frames, so a reveal costs about **6.1 ms of CPU** spread
over 200 ms — roughly 3% of one core. Nothing about the duration is constrained by this.

Two things about that table are worth keeping. **The split has to be same-screen or it measures
nothing.** The first run counted every frame and put Settings — which draws the rendering specimen,
every script in three families — in the *still* bucket and Review in the *animating* one, and duly
reported that animating frames were twice as cheap as still ones. **And an animating frame really is
cheaper than a resting one on this screen**: a resting review frame includes the once-a-second
repaint that re-derives the queue, and an animating frame re-uses the card it already has. The
transition's marginal cost is tessellation, and tessellation is not the expensive part.

**The measurement found the defect.** The first gated run counted **24** animating frames per reveal
where twelve were expected, which is one transition too many — and the extra one was running on every
*grade*. Keyed on the `Ui`'s own id, the animation state is **too** stable: it survives the card
changing, so grading leaves `revealed` false with the value still at 1.0, and the **next card's
answer is drawn, fading out, for the whole duration**. A card nobody had turned over was showing its
answer.

This is #123's first trap from the other side. #123 named it as *an id that changes between frames
snaps, so there is no motion at all, with nothing failing*; this is an id that does not change when
it should, so there is motion that shows the reviewer something the whole application is built to
withhold, also with nothing failing.

**And #123's second trap decides how to fix it.** Keying on the card's own `CardRef` works — a new
card is a new id and egui snaps it to 0 on first sight — but egui's animation state is an
`IdMap<BoolAnim>` that is **inserted into and never removed from**: there is no eviction anywhere in
`animation_manager.rs`, and `Context::clear_animations` drops the whole map or nothing. So a per-card
id retains one entry for every card ever reviewed, for the life of the process. It is twelve bytes
and a `u64` key, so a thousand-card day is tens of kilobytes and nothing is at risk — but it is
growth with no ceiling in a loop the application runs all day, reached by accident rather than
chosen.

**So the reveal is keyed on one id and reset when the card changes**, with an `animation_time` of
zero. That works because the manager's step divides by the duration: at zero the result is infinite,
fails its own `is_finite()` check, and falls through to the target. One id, O(1) memory, and a new
card that starts unrevealed with no fade — both traps discharged rather than traded against each
other. The frame count halved to twelve either way, which is how the fix was confirmed.

**Whatever ships owes four tests, and they are already written** (`proto.rs`, `mod tests`). They
assert on `egui::Context` directly rather than on this module, because what they pin is *what the
renderer does* — the thing a later egui release could change underneath a decision recorded here: a
first-sight id snaps; a stable id animates; state is retained indefinitely; a zero duration snaps a
stable id. The fifth test belongs to the application rather than to the renderer, and it is not a
test about motion at all — it is a test that the answer is not on screen before the reveal
(ADR-0006 §4).

### 3. Everything the storyboard clicks has to sit above Appearance's sentence

Appearance's explanatory line wraps to two lines at 560 and one at 1280, so **everything below it
sits 17px lower at the narrow width** — measured, not feared. Anything a storyboard must click at
both judging widths therefore has to sit *above* that sentence, and only the one-word heading can be
above this switcher.

It is #143's trap from the other side: there, the control that moved *was* Appearance, and the run
produced seven perfectly valid captures of the wrong theme. Here the control that moves is everything
else. The general shape is worth naming, because it has now cost this map twice: **a control whose y
a harness depends on must have nothing above it that can reflow**, and prose above a control is
exactly that. The frame-cost readout, whose counters grow, is drawn last for the same reason —
nothing is below it to move.

### 4. The overlay's three materials, measured off the shipped pixels

Confirmed against the captures rather than the source. In `overlay-1280x800/`, sampling inside the
open dropdown and on the page beside it:

| | fill | the page | |
|---|---|---|---|
| dark, today | `#1a1e21` | `#1a1e21` | **identical** |
| light, today | `#dee2e3` | `#dee2e3` | **identical** |

So the ticket's claim is exact: the one thing in the application that floats is drawn in *exactly*
the page colour, separated from it by an unchosen hairline and nothing else.

**The risen candidate.** ADR-0033 cuts a card *into* the page, so depth is subtractive everywhere
permanent, and the one surface #149 calls temporary is the one surface that goes the other way — by
the same amount. Dark delivers **1.121:1** between page and card, so the popup is 1.12:1 *above* the
page in both themes, placed by the gap dark delivers rather than by each theme's own page-relative
ratio, which is ADR-0036 §2's method. That is `#22282b` in dark (1.124:1) and `#eaeff0` in light
(1.125:1), and both are in the captures.

Two facts the arithmetic hands over:

- **In dark the risen direction is fully occupied.** `STONE_3` is the ordinary control at 1.099,
  `STONE_4` the separator at 1.222, `STONE_5` the primary at 1.293 — every rung above the page
  already means something, so the popup lands *between* two rungs rather than on one.
- **In light the risen direction is empty and nearly exhausted.** There is **1.305:1** in total
  between the page and pure white, no role occupies any of it, and this rise spends 1.125 of it. So
  light can afford exactly one risen surface. That is the strongest argument on the table that only
  a popup gets one — and it is the same razor ADR-0036 §2 found for the primary, which cleared its
  target by 0.012.

### 5. Stock's shadow asymmetry is in the right direction and out by 2×

Stock egui uses `from_black_alpha(96)` in dark and `from_black_alpha(25)` in light, at identical
offset and blur — a 3.84× difference nobody in this repository has decided. The ticket predicted a
darkening is not one gesture. It is not, and the correction overshoots:

| | page | shadowed | what the shadow buys |
|---|---|---|---|
| dark, alpha 96 | `#1a1e21` | `#131618` | **1.083:1** |
| light, alpha 25 | `#dee2e3` | `#cfd3d4` | **1.156:1** |

Measured at the same pixel offset outside the popup's right edge, in the shipped captures. Light's
shadow is **1.88× the weight** of dark's as a contrast ratio, and 22× as an absolute luminance
difference. Stock is right that light needs less alpha and wrong about how much less.

Two consequences for the sitting.

**A dark shadow at stock's own alpha is quieter than the card's own well** — 1.083:1 against
ADR-0033's 1.121:1 — so in dark the shadow cannot be the thing that says *this floats*, and the rise
has to carry it. In light, where the rise is nearly unaffordable, the shadow can. **Elevation
therefore differs in mechanism between the two themes and should agree on weight**, which is exactly
the shape #143 found for `weak_text_alpha`: derived in dark, named in light, differing in mechanism
and agreeing on the thing that was actually decided.

**And if the two are to agree by ratio, dark's alpha is about 200, not 96.** Back-solved from the
measured blur profile at that pixel: alpha 200 buys dark 1.159:1, which is light's 1.156:1. That is a
starting position for the knob, not a proposal.

---

## The candidates

### The reveal — `reveal-1280x800/`, `reveal-560x860/`

Fifteen shots: hold 0%, 50% and 100% × A–E, at both judging widths. At 0% the card is drawn as it
looks *before* the tap with `revealed` already true, which is how a reserved candidate's empty lower
half becomes photographable at all.

| | the prompt at the reveal | the answer | the hairline |
|---|---|---|---|
| **A** today | jumps 42px | appears | appears |
| **B** naive fade | jumps 42px | fades up | fades up |
| **C** reserved fade | **holds still** | fades up | fades up |
| **D** reserved fade, standing hairline | **holds still** | fades up | always drawn |
| **E** wipe | **moves smoothly** | wipes open | travels |

**C is the rule implemented honestly.** The card lays out as if both faces are present from the first
frame, so the prompt is placed once and never re-placed. Its cost is visible in
`reveal-1280x800/12-C-hold000.png`: an unrevealed short card now has an empty lower half. That is
either the reveal invitation #124 wanted *inside* the card, drawn as silence, or a hole — and it is
the judgement C exists to collect.

**D asks whether the hairline belongs to the card or to the answer.** ADR-0033 §1 says a card is *one
object with two faces divided by a hairline*; if that is true from the first frame, the divider is
not something the reveal delivers. D draws it standing, which also gives the empty half an edge.

**E is the rule's opponent and the ticket named it.** The answer half opens and the prompt slides the
whole 42px rather than jumping it. If it wins, #149 §2 is superseded rather than quietly ignored.

**One thing the held stills teach on their own**: at cubic_out, holding at 50% of the *time* is 87.5%
of the *opacity* — `1 − (1−0.5)³`. Most of a cubic_out fade is over in the first quarter of its
duration, which is worth knowing before the duration knob is dragged, because it means the curve and
the duration are not independent knobs to the eye.

### The overlay — `overlay-1280x800/`, `overlay-blurred/`

| | fill | edge | shadow |
|---|---|---|---|
| **1** today | the page | stock grey (60 / 190) | none |
| **2** edge only | the page | the separator rung | none |
| **3** shadow only | the page | the separator rung | chosen |
| **4** rise | 1.12:1 off the page, upward | the separator rung | none |
| **5** rise and shadow | 1.12:1 off the page | the separator rung | chosen |

`overlay-blurred/` is the same set cropped to the dropdown and blurred past legibility — ADR-0033
§3's own instrument, and what actually decided #143. Blurred, candidate 1 dissolves into the page and
candidate 5 stays a distinct pale slab with a darkened surround. **The difference survives the
blur**, which is the whole argument for making it at all.

---

## What the sitting has to produce

Four numbers and three verdicts.

1. **A duration**, in milliseconds, read off the knob — not chosen from three photographs (#141).
2. **A curve**, from the six.
3. **A shadow alpha for dark** and **one for light**, read off the two knobs.
4. **Which reveal candidate**, and if it is E, whether #149 §2 is superseded.
5. **Which overlay material**, and whether the rise, the shadow or both carry it in each theme.
6. **Whether C's empty lower half is an invitation or a hole**, which is the one thing here that no
   measurement can reach.

The build is `cargo run --release -p cairn-desktop`. Everything is on Settings, at the top.

---

## What is deliberately not here

**No light captures of the reveal.** An opacity fade is theme-independent in kind, and the theme-
sensitive half of this ticket is the shadow, which is captured in both. The sitting switches
Appearance and judges the reveal in both anyway; the shipping change owes captures in both themes at
both widths, and those are the *landed app's*, not the prototype's.

**No handset run.** #123's cost figure has a handset half and it belongs to
[#126](https://github.com/amin-bf/cairn/issues/126), which is Checkpoint Two. Nothing here is sized
or placed differently on a phone, so there is no arrangement question to take there.

**No decision about `Style::animation_time` being global.** It is, it is read from the global style
so a screen cannot override it locally, and the ticket already recorded that as the wanted behaviour.
Nothing in this round argued with it.
