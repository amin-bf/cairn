# ADR-0030: The first finish-pass decisions — palette, dark mode, the contrast floor, and the box badge's case

- **Status**: Accepted
- **Date**: 2026-08-08
- **Resolves**: [Finish Pass: Decide the Palette, Light Mode and the Box Badge's Case](https://github.com/amin-bf/cairn/issues/114)
- **Related**: [ADR-0006 §6 §10](0006-the-review-session-experience.md) (opened the finish pass from a
  blank slate, and — as corrected in #96 — handed **this** pass the box badge's case and face),
  [ADR-0001 §3](0001-scheduling-algorithm-and-grade-scale.md) (a box means durability, never a
  queue position — the reading the badge must not acquire),
  [ADR-0003 §4](0003-client-stack.md) and [ADR-0012 §8](0012-the-note-authoring-experience.md)
  (*bold is a face, never a colour*, an argument that rests on the body being a near-white — which
  this palette formalises), [ADR-0021](0021-note-ordering-saving-and-the-note-list.md) (its Context
  rests on the app being *"answerable without knowing a single colour"*),
  [ADR-0015 §5](0015-the-sync-experience.md) (the notice channel that does not exist yet, one of the
  three call sites the unreachable accents wait on)
- **Evidence**: the drafted palette — cool slate neutrals with four desaturated accents — measured
  against the stock egui theme the binary ships today. The contrast figures are inline in §3. This
  is a **finish-pass** decision judged from the measurements and the wireframes, not through
  `/prototype`; see *Consequences*.

## Context

[ADR-0006 §10](0006-the-review-session-experience.md) opened the finish pass and said it starts from
a blank slate: the dark palette, spacing and typography carried through both review-session prototype
rounds were *"scaffolding carried over for convenience, never a considered decision"*. A palette has
since been designed and measured against that stock theme. The measurements are in hand; what was
missing is the decision record, and without it the palette cannot be implemented (issue #115) without
re-litigating four questions each screen would otherwise answer for itself, one screen at a time.

This ADR answers those four and records a fifth fact. It does **not** open the rest of the finish
pass: typography beyond the one face the badge needs, spacing, and weight stay blank-slate.

## Decision

### 1. Colour enters `cairn-app`, at exactly one naming site

Three documents currently rest on the app naming no colour and setting no theme:
[ADR-0006 §10](0006-the-review-session-experience.md) (the blank slate),
[ADR-0021](0021-note-ordering-saving-and-the-note-list.md)'s Context (*"what an entry says, where it
sits and when it appears were answerable without knowing a single colour"*), and
[ADR-0012 §8](0012-the-note-authoring-experience.md)'s *"bold is a face, never a colour"*, whose
argument rests on the body colour being an ambient near-white it never names. That stops being true
here: the app now owns a palette.

> **The palette is named in exactly one place — a `theme` module whose single function produces an
> `egui::Visuals` — and it is installed once on the context at construction. Every screen keeps
> reading the *ambient* visuals (`ui.visuals().text_color()`, `weak_text_color()`,
> `hyperlink_color`, and the rest) exactly as it does today. A colour literal anywhere outside
> `theme` is the defect.**

This mirrors `fonts`: one enumeration, many readers, and the readers ask for a role rather than a
value. The single site is what keeps the palette a palette rather than a drift of per-screen tweaks
that each render fine and none of which fails. It is *not* deferred to the first frame the way
`fonts` is — visuals allocate no texture, so ADR-0012 §8's `CreationContext` hazard does not reach
them, and setting them at construction avoids a frame of the wrong theme.

The screens already read the ambient visuals — `lib.rs` and `screens/settings.rs` call
`ui.visuals().text_color()` and `weak_text_color()` throughout, and `fonts.rs` names white only in
its coverage probe, which draws nothing a user sees. So this decision changes *where the values come
from*, not the call sites: no screen is rewritten to adopt the palette.

### 2. Dark is pinned; system-following is dropped, deliberately

The app follows the OS theme today — egui's default, never overridden, because nothing here ever
called `set_visuals`. The drafted palette is dark only.

> **The app pins dark. It no longer follows the OS theme. This is a deliberate removal of a
> behaviour the product currently has, recorded here so it is not dropped in silence.**

Pinning is two acts, not one, and doing only the first fails silently (see the rule added to
`AGENTS.md`): install the dark palette as the visuals, **and** disable theme-following, so that an OS
theme change does not clobber the palette back to stock egui.

A light palette is **not** drawn now, and following the OS with only a dark palette drawn is the one
outcome refused. The reasoning:

- **A light palette is a second finish pass, not a fork of this one.** It needs its own colours, its
  own contrast measurement against a light surface, and a re-check of every argument that currently
  rests on a near-white body — [ADR-0012 §8](0012-the-note-authoring-experience.md)'s *bold is a
  face* (a heavier face reads on any surface, but the rejected alternative, brightening, inverts
  its sense on a light one) and the answer-emphasis renderer that followed it. None of that is done.
- **Following the OS with one palette drawn is worse than either branch.** On a light-preferring OS
  the user would silently get **stock egui** — the 5.12:1 body §3 exists to fix — reached by
  omission rather than by anyone choosing it. That is precisely the silent drop the issue forbids,
  wearing the costume of *"we kept system-following"*.

So dark is pinned, and a light palette is deferred finish work, reopenable without permission
(*Open items*).

### 3. The contrast floor: 7:1, binding text against its surface

The reason to adopt the palette at all is measured: body text against the panel improves from
**5.12:1 to 13.34:1**. It matters because the body style is **12.5px** and the small style **9px**,
and 9px is exactly where stock egui's 5.12:1 hurts.

> **The floor is 7:1, and it binds every pair where a palette colour carries *text* against a
> surface the palette owns.** Chosen over WCAG AA's 4.5:1 because at 9px, 4.5:1 is already the
> marginal case this pass exists to leave behind; 7:1 is WCAG AAA for body text.

- **Bound (text):** body-on-panel — measured at **13.34:1**, clearing the floor with margin — and by
  the same rule any accent that carries text. Implementation (#115) confirms each such pair against
  7:1 (body-on-panel 13.34:1, body-on-card 10.32:1, body over the selection fill 8.14:1); the
  measured exemplar is the headline, not the whole set.
  - **Weak text is the exception, and #115 reclassifies it.** This section listed weak-text-on-panel
    among the bound pairs, but the derived `weak_text_color()` lands near `#8b979b`, ~5.6:1 on the
    panel — below the floor. It is **not** lifted: §4 draws the box badge in the weak colour as a
    *quiet footnote*, and a 7:1 weak text is a loud one, so the two decisions pull against each other
    and §4's quiet-footnote requirement governs the colour. It is treated as a **pre-existing
    weakness** (stock's weak text is 5.12:1, so the palette *improves* it and never regresses it),
    of the same kind as the non-text pairs below, and #115's test pins it against stock rather than
    against 7:1. Recorded here so its absence from the enforced floor is a decision, not an oversight.
- **Not bound (non-text):** widget fills against the panel, and decorative strokes against their
  fills. These mostly fail even 3:1 in stock egui **and** in the new palette, so they are a
  **pre-existing weakness rather than a regression**, and lifting them is a separate, larger job than
  a palette swap. Out of scope for this pass, stated so their absence from the floor is a decision
  rather than an oversight.
  - **One exception was called out because it *regresses*, and #115 lifted it.** The hover stroke
    against its own fill crossed **3.19:1 → 2.49:1**. This section originally recorded it as accepted,
    not fixed, and deferred it to a separate contrast pass. Implementation (#115) reversed that: it is
    a lone regression inside an across-the-board improvement — exactly the thing a later reader
    assumes was missed — and the fix is one rung of the stone ramp (the stroke moves up to `#8b979b`,
    the light stone the derived weak text also lands near, clearing 3:1 with margin), not the larger
    job of binding decorative strokes to a floor. So it is lifted and tested (`hover_stroke_clears_
    three_to_one`) rather than deferred; the *other* non-text pairs stay out of scope.

### 4. The box badge: lower-case, in the small-text face

[ADR-0006 §6](0006-the-review-session-experience.md), as corrected in #96, hands this pass the
badge's **case and face** by name, records that the current `Box 3` rendering is *conformant but
undecided*, and keeps one clause with teeth: **the badge reads `new` for a card with no review
history, never a box number**. That clause is [ADR-0001 §3](0001-scheduling-algorithm-and-grade-scale.md)'s,
it is unchanged, and it is restated here so settling the two open properties does not read as
reopening the settled one.

> **The badge is drawn in the ordinary small-text proportional face, in the weak text colour, and in
> lower case: `box 3`, and `new`.**

- **Face — not monospace.** Monospace was the prototype's scaffolding, disclaimed by ADR-0006 §10
  along with the rest. It reads as *data* — a field value, a code span — which the box is not, and a
  face nothing else on the screen uses makes the badge **louder**, the opposite of the *"small,
  non-interactive"* footnote §6 intends. The small-text face in the weak colour is what every other
  quiet aside on the screen already uses.
- **Case — lower.** Capitalising `Box` makes it a label and a proper noun; the badge is a footnote,
  not a heading. `box 3` and `new` share one case so neither reads as the more important of the two —
  which matters because they are the same field in two states, and a card crossing from `new` to
  `box 1` should change its *content*, not its *register*. This settles the case against today's
  `Box 3`; #115 renders `box 3`.

### 5. Recorded, not fixed: three of the four accents are currently unreachable

The palette advertises **four** desaturated accents. Only **one** has a call site today — the accent
egui already draws for selection and the active destination. The other three — **warn, error,
link** — have none: the notice channel ([ADR-0015 §5](0015-the-sync-experience.md)) and hyperlinks
do not exist yet, and the app reads only the body and weak text colours besides that one accent.

> **The warn, error and link accents land set-and-unused. This is accepted, not overlooked.**

It is written down because a palette naming four accents that can currently express one invites a
later reader to *find* call sites for the other three — to colour something warn-coloured because the
colour exists, which is how a "quiet" surface acquires speakers ADR-0015 §5 spent a rule forbidding.
Defined-and-dormant is the correct state: a colour with no caller, like a slot with no card, waiting
for the surface that will read it rather than for someone to invent one.

### 6. What this ADR does *not* settle

- **The rest of the finish pass** — typography beyond the badge's one face, spacing, weight
  ([ADR-0006 §10](0006-the-review-session-experience.md)).
- **A light palette**, and with it the restoration of system-following (§2).
- **The decorative, non-text contrast pairs** (§3) — *except* the lone hover-stroke regression, which
  #115 lifted back over 3:1 (§3); the rest stay deferred.

## Amendments to accepted ADRs

| ADR | What changes | Why |
|---|---|---|
| [ADR-0006 §6](0006-the-review-session-experience.md) | The badge's **case and face are settled**: lower-case, in the small-text proportional face and weak colour. #96's handoff is **discharged**. The `new`-not-a-number clause is unchanged. | §4 above. §6 as corrected named these two as this pass's to settle and marked today's `Box 3` conformant-but-undecided. |
| [ADR-0006 §10](0006-the-review-session-experience.md) | The finish pass is **no longer entirely unopened**: palette, dark-vs-light, the contrast floor and the badge's case and face are decided here. The remainder — typography, spacing, weight — stays blank-slate. | §1–§5. §10 opened the pass; this ADR takes its first decisions and says which parts are still open. |

## Glossary

New terms are added to [`ui`'s `CONTEXT.md`](../../crates/app/src/CONTEXT.md), which owns the
screens: **Palette**, **Contrast floor**. The **Box badge** entry is revised to carry its now-settled
case and face, and the **Finish pass** entry to record that its first decisions are taken here.

## Consequences

- **Colour is now something `cairn-app` owns**, and the single-site rule is the whole of what keeps
  it one palette rather than a screen-by-screen drift. The three documents that rested on the app
  naming no colour are amended or superseded in the same move (§1).
- **Following the OS theme is gone.** A user on a light-preferring OS now sees the dark palette. This
  is a real behaviour change, recorded rather than silent, and reopenable when a light palette is
  drawn.
- **Body text reads at 13.34:1**, up from 5.12:1, which is the point. Non-text contrast is otherwise
  unchanged from stock — no better — except the one hover-stroke pair, which #115 lifted back over
  3:1 rather than shipping the regression this ADR first recorded (§3). Weak text stays below the
  floor as a pre-existing weakness, deliberately, so the box badge reads as the quiet footnote §4
  asks for (§3).
- **Three accents are defined and have no caller.** That is the expected resting state until the
  notice channel and links exist, not drift, and not an invitation to give them one.
- **This was judged from measurements and wireframes, not a build.** Like
  [ADR-0029](0029-editing-a-note-from-the-review-screen.md), and unlike
  [ADR-0006](0006-the-review-session-experience.md) and
  [ADR-0012](0012-the-note-authoring-experience.md), which went through `/prototype`. The badge's
  case is the item most likely to be revisited once someone is reviewing daily; §4's argument rests
  on register rather than on use, so reopening it needs no permission.

## Open items handed onward

| Item | Owner |
|---|---|
| A light palette, and restoring system-following behind it | The **finish pass**, reopenable without permission (§2) |
| Typography beyond the badge's face, spacing, weight | The **finish pass** ([ADR-0006 §10](0006-the-review-session-experience.md)) |
| The decorative non-text contrast pairs (the hover-stroke regression is **done** — #115 lifted it, §3) | A separate contrast pass (§3) |
| Lifting weak text to the 7:1 floor, if §4's quiet footnote is ever reconsidered | The **finish pass** (§3) |
| Whether the badge's lower-case register reads right in daily use | Post-implementation, like [ADR-0010](0010-leeches.md)'s thresholds |
