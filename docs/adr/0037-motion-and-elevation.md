# ADR-0037: Motion and elevation — the reveal is an opening, and one surface floats

- **Status**: Accepted
- **Date**: 2026-08-30
- **Resolves**: [Design Pass: Motion and Elevation — the Reveal, and What Floats](https://github.com/amin-bf/cairn/issues/154)
- **Related**: [ADR-0033 §1 §2](0033-the-card.md) (a card is one object with two faces, cut *into*
  the page as a well — the direction this ADR's one risen surface inverts; **its step-down gains a
  consequence in §5 here**), [ADR-0034 §1](0034-the-controls.md) (the grades, whose cluster the
  reveal must not move), [ADR-0035 §1](0035-the-vertical-anchor.md) (the reach line the card sits
  above), [ADR-0036 §2](0036-the-light-palette.md) (weights placed by the **pairwise gaps** dark
  delivers, never by each theme's page-relative ratio — the method §1 applies), [ADR-0030 §1
  §4](0030-the-first-finish-pass-decisions.md) (the single naming site; the badge as a quiet aside),
  [ADR-0032 §1](0032-the-type-scale-and-the-rhythm.md) (the four pinned sizes §5 protects),
  [ADR-0015 §5](0015-the-sync-experience.md) (the notice channel is **persistent, non-modal**, so it
  casts nothing), [ADR-0006 §4](0006-the-review-session-experience.md) (the answer is not on screen
  before the reveal — the invariant §4's test pins)
- **Evidence**: `docs/design/prototype-154/` — five reveal candidates, five overlay materials and
  four knobs, built as the application itself and judged as a **running sitting** in both themes,
  on a vocabulary collection and then on a cloze one. Preserved as the tag `prototypes/issue-154`.
  Contrast figures are WCAG 2.1, computed by the helpers `crates/app/src/theme.rs`'s tests use;
  layout figures are measured through `run_ui` rather than read off the source.

## Context

[The Craft](https://github.com/amin-bf/cairn/issues/149) decided two rules and deliberately built
neither. *Elevation exists, and it means temporarily on top.* *Motion may change what is on the
screen, never where it is.* It also found that all four families [#123](https://github.com/amin-bf/cairn/issues/123)
had recorded as having no ambient role turned out to have one — elevation in `Visuals::popup_shadow`,
motion's duration in `Style::animation_time`, its application in `Ui::multiply_opacity` — so the
values had somewhere to live before anyone had chosen them.

Three of those values were **stock and unchosen, and had been in every capture this repository
holds**: `window_fill` is assigned `panel_fill` in both themes, so the application's only overlay is
drawn in *exactly* the page colour; `window_stroke` is never assigned at all, so its hairline is
stock grey — 60 in dark, 190 in light, off the ramp in both; and both shadows are `NONE`. The one
thing in the product that floats was separated from the page it floats over by a value nobody chose
and nothing else.

**And #149's motion rule was wrong about its own worked example.** It said the reveal is the case the
rule binds, and that the card *"already cannot"* twitch because `surface::REVIEW_HEIGHT` is a 300px
floor. That is true of the card and false of everything drawn inside it: `surface::card` centres its
content on the card's centre line and the content grows at the reveal, so the prompt is centred in
300px before the tap and in the top half after it. Measured at both judging widths, the prompt moves
**41.9px**. The reveal did not merely *read* as a jump-cut; it **was** one, under the rule written to
forbid it, in as many words.

So this ADR settles values the product ships without having chosen, and it re-opens the rule it was
meant to implement, because building the rule is what showed it was wrong.

## Decision

### 1. One surface floats, and it is made of a rise, a chosen edge and a shadow — in both themes

**A shadow is cast only by something the renderer already calls a popup, a menu or a window**, which
is #149 §1 unchanged and the part of it this ADR only ratifies. Nothing permanent lifts off the page;
the card stays cut into it (ADR-0033 §1), so depth is subtractive everywhere else. The notice channel
is worded persistent and non-modal (ADR-0015 §5) and casts none.

What is new is what such a surface is *made of*, and it is all three:

| | dark | light |
|---|---|---|
| fill — risen off the page | `#22282b`, **1.124:1** | `#eaeff0`, **1.125:1** |
| edge — the separator rung | `STONE_4` `#282e33` | `STONE_L_EDGE` `#ced2d3` |
| shadow — `from_black_alpha` | **200** | **25** |

**The rise is exactly as much as a card sinks.** ADR-0033 cuts a card into the page by 1.121:1 in
dark, so the one surface that is temporary rises by the same amount, in both themes — placed by the
gap dark delivers rather than by each theme's own page-relative ratio, which is ADR-0036 §2's method
and the thing that ADR was written to stop anyone mirroring.

**The edge is the separator rung** rather than a rung of its own. A popup's edge makes the claim a
separator makes — *this is a boundary* — and both themes already own that value.

**The shadow keeps stock's geometry and replaces its darkening.** Offset `[6, 10]`, blur `8`, spread
`0`; nothing in this ticket disputed the shape. Only the alpha was on a knob, and only in both themes
at once.

**The two alphas differ by 8× and agree on what they buy to within 0.003.** Dark's 200 buys
**1.159:1** against the page; light's 25 buys **1.156:1**. Stock's own 96 and 25 buy 1.083 and 1.156
— a 1.88× disagreement in the very thing being chosen, at identical offset and blur. That asymmetry
is real and stock has its direction right: a light ground needs far less alpha. It has the magnitude
wrong by roughly a factor of two.

**This is #143's `weak_text_alpha` shape reached from the other side, and it resolves differently.**
There, one setting produced different weights on the two grounds, and the answer was to name the
value in light and derive it in dark — *different mechanism, same weight*. Here the mechanism was
offered a split too: in dark the shadow at stock's alpha is **quieter than the card's own well**
(1.083:1 against 1.121:1), so the rise would have had to carry dark alone, while in light the risen
direction holds only **1.305:1** in total and this rise spends 1.125 of it, so the shadow could carry
light alone. Judged blurred — ADR-0033 §3's own instrument — **one material won in both themes**.
So the asymmetry lives in the numbers rather than in the rule, which is the smaller place for it.

**Light can afford exactly one risen surface**, and that is now a load-bearing fact rather than an
observation: the 1.305:1 above `#dee2e3` is spent. It is the strongest argument the system has that
elevation stays scoped to overlays, and it should be quoted at whoever next proposes a raised card.

### 2. Motion is 240ms and `cubic_out`, and it lives in a `motion` module

**A fifth per-family module** — `theme`, `frame`, `typography`, `spacing`, `motion` — holding one
constant and an `install(ctx)`, matching `typography::install` and `spacing::install`. There is **no
shared token module**, which all five now answer the same way.

**The duration is ambient and global on purpose.** `Style::animation_time` is read from the global
style, so a screen cannot override it locally, and that is the wanted behaviour: a screen that wants
its own tempo is a screen inventing a value the system already names. `install` writes it into every
theme slot, for the reason `typography::install` does — a slot left unwritten is a silent wrong value
that only appears when someone switches theme.

**The curve is not ambient, because it cannot be.** It is which function a call site passes:
`animate_bool` is linear, `animate_bool_responsive` is the only one that picks a curve, and the
parameter is a bare `fn` pointer with no closures and no cubic-Bézier constructor. `motion` therefore
names the easing as a constant beside the duration, so the choice is stated once even though the
renderer will not carry it ambiently.

**240ms and `cubic_out` were dragged, not chosen from a menu** (ADR-0035's rule — a distance wants a
knob). The two are not independent to the eye and the sitting was told so first: at `cubic_out`, 50%
of the *time* is 87.5% of the *opacity*, so most of the fade is over in the first quarter of its
duration.

**Cost is not a constraint on the duration, and this is the first time that is measured rather than
computed.** #123 computed twelve full layout-and-tessellation passes per 0.2s transition and said
explicitly that nothing was run. Run in release at 1280×800 through `eframe`'s own `cpu_usage`, split
into review-card frames with a transition in flight and without: **twelve frames, 0.51ms each** —
about 6.1ms of CPU spread over the transition, roughly 3% of one core. An animating frame is in fact
*cheaper* than a resting one on that screen, because a resting review frame carries the once-a-second
queue re-derivation and an animating one re-uses the card it has.

### 3. The reveal is the answer half **opening** — so #149 §2 is narrowed, not ignored

*Motion may change what is on the screen, never where it is* was built as candidate C — both faces'
room reserved from the first frame, so nothing is ever re-placed — and judged against candidate E,
where the answer half wipes open and the prompt rides the opening. **E won, in a running sitting, on
a vocabulary collection and again on a cloze one.**

The rule is therefore restated rather than deleted:

> **Nothing slides, scales, springs or grows *on arrival*.** A thing that appears fades in, a thing
> that leaves fades out, a colour that changes crosses over. No control, sheet, screen or notice
> travels.
>
> **A card turning over is one object opening.** The prompt riding that opening is the object moving,
> not the layout jumping — which is the one place in the product where movement is the honest
> description of the event.

The distinction is not a loophole and it is worth being able to state. ADR-0033 §1 already says a
card is **one object with two faces divided by a hairline**. If that is true, the reveal is that
object opening, and an object that opens has an inside that moves. What #149 §2 was actually right
about is everything that *arrives* — a badge, a notice, a control appearing — and that half is
untouched and now has a worked example on the same screen, since the box badge still fades up in
place and gains nothing else.

**The naive fade is why this had to be built rather than argued.** Candidate B — an opacity fade laid
over today's layout — leaves the prompt jumping on frame zero while the answer fades in behind it:
the movement the rule forbids, wearing the motion the rule asks for. It passes every reading of §2
and is the worst candidate on the screen. That is #143's shape exactly — *the rule passes while the
values fail* — arriving in motion instead of in colour.

**An amendment ADR-0006 §5's shape.** This narrows #149 §2 to *arrival*, and scopes the exception to
*within one object at the reveal*. It does not license a screen transition, a sliding panel, or a
control that travels to its position.

### 4. The reveal animates on **one** id, reset when the card changes

Both traps #123 named live in the choice of animation id, and they pull in opposite directions.

**Keyed on the `Ui`'s own id the state is too stable.** It survives the card changing, so grading
leaves `revealed` false with the value still at 1.0 and **the next card's answer is drawn, fading
out, for the whole duration** — a card nobody has turned over showing its answer, which is the one
thing the application exists to withhold. Found by counting frames: a gated run reported 24 animating
frames per reveal where twelve were expected, and the extra transition was running on every *grade*.

**Keyed on the card's `CardRef` the state is never reclaimed.** egui's animation state is an
`IdMap<BoolAnim>` inserted into and never removed from — there is no eviction anywhere in
`animation_manager.rs`, and `Context::clear_animations` drops the whole map or nothing — so a
per-card id retains one entry for every card ever reviewed, for the life of the process. Twelve bytes
and a `u64` key each, so nothing is at risk; but it is growth with no ceiling in the loop the
application runs all day, reached by accident rather than chosen.

**So: one id, reset with an `animation_time` of zero on the frame the card changes.** A zero duration
makes the manager's step divide by zero, which fails its own `is_finite()` check and falls through to
the target — snapping is the documented behaviour of a zero-length animation, reached deliberately.
O(1) memory, a new card that starts unrevealed with no fade, and **both traps discharged rather than
traded against each other**. The reset hangs off `s.shown != Some(offered.card)`, which the review
screen already computes, so it costs the change one bool.

### 5. The reveal must not change the prompt's **type size** — a consequence ADR-0033 did not know it had

ADR-0033's step-down draws a card face at the largest tier it *fits* in, floored at body. The tier is
chosen against the content, and **before this ADR the content changed at the reveal** — so on any
card whose two faces together overflow the 300px budget while the prompt alone does not, the prompt
was drawn at display before the tap and at heading after it.

Measured at 560 on four ordinary cloze notes, on the layout this ADR replaces:

| the card | prompt tier, before → after | the prompt's travel |
|---|---|---|
| `Le chat […] la souris` | 40 → 40 | −41.9 |
| `La Tour Eiffel … de […]` | **40 → 20** | +4.1 |
| `[…], je m'appelle Amin et je travaille…` | 40 → 40 | −64.9 |
| `Le Traité de Versailles … les […]` | **40 → 20** | +27.1 |

**The sentence being read halves in size at the moment the reader is looking hardest at it**, which
is a louder event than the 42px jump this ticket was opened about, and it appears in neither the
ticket nor #149 nor the prototype's first round.

**So the tier is computed once, against the card's full content, and holds across the reveal.** That
falls out of reserving the answer's room, which E does anyway — but it is recorded as its own claim
because it is the part that would break silently if someone later made the reveal cheaper by laying
out only what is visible.

Three things this makes explicit for whoever reads it next:

- **The defect hid at the wide width.** At 1280 none of the four notes step down and every candidate
  behaves identically. It exists only at **560** — the application's own window, and the width
  nearest the handset.
- **"42px" was one card's number.** Across four ordinary cloze cards the travel is −41.9, −64.9, +4.1
  and +27.1, and on two of them the prompt moved *down*. The figure the ticket, #141 and #149 all
  quote is the six-word vocabulary seed's.
- **It costs something, and the cost was judged rather than absorbed.** Before the tap, a wrapping
  card's prompt is now drawn at heading where it used to be drawn at display, so an unrevealed cloze
  card is *quieter* than it was. That was put in front of the sitting explicitly and accepted.

## Consequences

**The application's only overlay stops being stock.** Three values that had never been chosen —
`window_fill`, `window_stroke`, `popup_shadow` — are named in both palettes, and
`both_palettes_name_the_same_slots` now covers them, so a slot filled in one theme and left stock in
the other fails the build rather than waiting to be noticed.

**`Style::animation_time` stops being 0.2 by inheritance and becomes 0.24 by decision.** Every
`animate_bool` in the crate moves with it. That is the intent of an ambient value and the reason the
duration is not passed per call site.

**The box badge's craft change ships, and it is the whole of it.** It fades up with the answer and
gains no picture, fill, colour or count — #149 §7's refusal, with ADR-0001 §3's argument behind it,
unchanged by anything here.

**The reserved layout costs a frame's worth of extra measurement.** The card lays out both faces from
the first frame whether or not the answer is showing, so `content_height` runs over the answer on
every unrevealed frame. At twelve frames per transition and 0.51ms a frame this is not visible, and
it is the price of both §3 and §5.

**Two figures in the prototype's own write-up are superseded by it**, recorded so a reader of the tag
is not misled: the reveal's travel is **41.91px** and not the 56px an intermediate build showed,
because the wipe's closed lead did not pay for the badge row drawn above it (16.8 reserved against
14.0 drawn) and started the card 14px below today's resting position.

**What this ADR does not decide.** The handset's half of #123's cost figure belongs to
[Checkpoint Two](https://github.com/amin-bf/cairn/issues/126); nothing here is sized or placed
differently on a phone, so there is no arrangement question to take there. And the screen the overlay
is judged on — the Notes deck dropdown — remains undesigned and belongs to
[The Notes Slice](https://github.com/amin-bf/cairn/issues/150). This decides the **material**, not
the screen, the way ADR-0033 decided a card's material before any list drew one.
