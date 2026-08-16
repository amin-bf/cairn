# ADR-0036: The light palette — the ink construction, what §3's invariant actually is, and the return of theme-following

- **Status**: Accepted
- **Date**: 2026-08-16
- **Resolves**: [Design Pass: The Light Palette — Re-derive the Weights, and Whether the OS Chooses](https://github.com/amin-bf/cairn/issues/143)
- **Related**: [ADR-0030 §2 §3 §5](0030-the-first-finish-pass-decisions.md) (pinned dark, dropped
  OS-following, set the 7:1 floor and the single naming site — **§2 is superseded here**),
  [ADR-0033 §2 §3](0033-the-card.md) (a card is a well; the controls must end up quieter than it),
  [ADR-0034 §1 §2](0034-the-controls.md) (the three weights, and the primary that exists for a
  card-less screen), [ADR-0032 §1](0032-the-type-scale-and-the-rhythm.md) (the 12px small tier that
  removed §3's premise), [ADR-0012 §8](0012-the-note-authoring-experience.md) (*bold is a face,
  never a colour* — the argument ADR-0030 §2 required re-checking against a light body),
  [ADR-0011 §3 §5](0011-decks-and-settings.md) (settings sync; this one must not)
- **Evidence**: `docs/design/prototype-143/` — three constructions and a knob, judged as a running
  page with ADR-0033 §3's own blur test, on Review and on the two card-less screens. Preserved as
  the tag `prototypes/issue-143`. Contrast figures throughout are WCAG 2.1, computed by the same
  helpers `crates/app/src/theme.rs`'s tests use.

## Context

[ADR-0030 §2](0030-the-first-finish-pass-decisions.md) pinned the application to dark and dropped
OS-theme following, for exactly one reason and it said so: only a dark palette was drawn, and
*"following the OS with one palette drawn is worse than either branch"* because a light-preferring
user would silently get **stock egui** — the 5.12:1 body §3 exists to leave behind — reached by
omission rather than by anyone choosing it. It recorded a light palette as deferred finish work,
reopenable without permission.

The design project carried a light scope the whole time. It had never been measured, and this ticket
found it was worse than unmeasured: `tokens/colors.css` gave `--surface-card:#fbfaf8` **lighter**
than its `--surface-panel:#f2f1ed`, so the card was not a well at all and ADR-0033 §3's ordering was
broken outright (ordinary 1.096:1, card 1.083:1); `guidelines/colors-light.card.html` gave the same
token as `#e2e1db`, which tied exactly with `--surface-primary`; the whole scope was **warm** where
the palette is cool slate, a re-hue nobody decided; and its `--text-weak` sat at 3.92:1 against
dark's 5.59:1.

Those are symptoms. The cause is the thing this ADR exists to record.

## Decision

### 1. The light palette is **re-derived**, never re-hued — and a transplant is impossible

The three control weights are **1.099:1** (ordinary), **1.121:1** (the card) and **1.293:1**
(`primary`). Every one was measured against a page near the **bottom** of the ramp, and they use
**both directions**: the card is a well *below* the page and the two controls rise *above* it.

A light page sits near the top of the ramp, which inverts which direction is scarce.

| page | total range **above** it | total range **below** it |
|---|---|---|
| `#1a1e21`, the dark page | 16.78:1 | 1.25:1 |
| `#f2f1ed`, the placeholder light page | **1.13:1** | 18.58:1 |
| `#dee2e3`, the page chosen here | **1.31:1** | 16.09:1 |

The dark page puts **one** weight in its scarce direction and it just fits — the card needs 1.121 of
the 1.252 available, which is what ADR-0033 §2 meant by *"the dark end of the stone ramp is
compressed enough that `STONE_0`-on-`panel_fill` is only 1.12:1"*. A mirrored light page does the
reverse and puts **two** weights in the scarce direction.

**How badly that fails depends on where the light page sits, and the honest numbers are worth
stating rather than rounding into a slogan.** Above the placeholder's paper-white `#f2f1ed` there is
**1.130:1** in total, so the primary's 1.293:1 is unreachable and even the ordinary control's 1.099:1
would sit a fraction off pure white: on that page, a `primary` lighter than the page **does not exist
at any hue**. Above the `#dee2e3` this ADR chose there is **1.305:1**, which does clear 1.293 — by
**0.012**. So the mirrored construction is impossible on a paper page and merely unbuildable on this
one, and the difference matters only to a reader checking the arithmetic.

**It was not rejected on that arithmetic.** All three constructions were drawn at the chosen page and
judged **blurred**, which is ADR-0033 §3's own instrument; ink won by eye, on Review and on the two
card-less screens. The arithmetic is why a mirror had to be considered at all, not why it lost.

> **Every light value is placed at the ratio the dark palette gives the same role against the same
> reference, moving *away from the page* — which on a light page is downward for all three fills.
> Lightening the dark values is not the move, on any light page worth having.**

`the_light_ramp_is_re_derived_not_re_hued` recomputes the whole light ramp from the dark constants
and fails if a hand has been laid on any of it. The tint is interpolated off the shipped ramp so the
neutrals stay cool slate; the accents keep their hue and change only their value.

### 2. The construction is **ink**: all three fills below the page, placed by the **gaps**

Three constructions were drawn and judged. Two of them are recorded here because the losers carry
the finding.

| | page kept | card ↔ ordinary | card ↔ primary |
|---|---|---|---|
| *dark, for reference* | — | 1.231:1 | 1.449:1 |
| **opposite sides** — mirror the structure | only from `#dee2e2` down | 1.230:1 | 1.454:1 |
| **ink, keeping the stated ratios** | any | **1.020:1** | 1.152:1 |
| **ink, keeping the gaps** ← chosen | any | 1.232:1 | 1.457:1 |

> **The page is `#dee2e3`. An ordinary control is `#d9dddd` (1.049:1), a card `#c4c8c9` (1.292:1),
> and the `primary` `#a2a6a7` (1.883:1) — all three *below* the page, placed so the pairwise gaps
> the dark palette delivers are preserved rather than its page-relative ratios.**

**The middle row is the finding, and it is the one a careful reader would have shipped.** ADR-0033 §3
and ADR-0034 §1 §2 record the invariant as three ratios **against the page**. Page-relative ratios
keep magnitude and throw away **direction** — and in dark the card and the controls sit on *opposite
sides* of the page, so the stated 1.099 and 1.121 quietly buy **1.231:1** between them. Implement
those same three numbers on a light page, where all three must go the same way, and §3's ordering
**holds at every page position** while delivering **83%** of the separation: a card↔ordinary gap of
1.02:1, which is two colours that are the same colour. The card and the buttons become one material
— [#133](https://github.com/amin-bf/cairn/issues/133)'s original complaint, *"the thing being
studied is made of the same material as the buttons under it"*, arrived at from the other direction,
with nothing failing.

**The page is a judgement and not a derivation.** `#dee2e3` is a light grey rather than paper, and
the ink construction delivers full separation at *every* page position, so nothing forced it — it
was chosen by looking, at the end of a knob whose readout was in these units. It is recorded as a
decision with no arithmetic behind it, which is the honest state.

**The primary is chained off the card, and the alternative was rejected in the picture.** Placing it
by its gap from the *ordinary* control instead gives `#c8cccd`, a healthier 10.71:1 for its label and
an ordinary↔primary of 1.182:1 that matches dark's 1.177:1 almost exactly — against the chosen
value's 1.794:1. It was judged blurred on the two card-less screens and lost: the darker slab is what
reads as the way forward. Worth recording because the *measurements* favour the loser.

**The price is one tight pair.** Light's body-on-`primary` is **7.06:1** — over ADR-0030 §3's floor by
0.06, where dark's equivalent is 10.32:1 and no other pair in either theme is under 10. It is the
tightest reading pair the application has, it clears the floor, and
`the_light_primary_is_the_tightest_reading_pair` pins the **figure** rather than the inequality so
the margin has to be re-argued rather than quietly spent. This is the ink construction's cost: a
primary 1.883:1 *below* the page eats into the body's headroom in a way one 1.293:1 *above* it never
could.

### 3. Theme-following returns — as a **choice**, not as obedience

ADR-0030 §2's reason is discharged: a light palette exists, so following no longer risks handing
anyone stock egui.

> **The application offers three appearances — System, Light and Dark, defaulting to System — on
> Settings. The preference is device-local and never syncs. Both theme slots are filled, always.**

- **What §2's refusal becomes.** §2 forbade a *reachable slot left on stock*, and honoured that by
  making no slot reachable. Now that light is drawn, the same refusal is honoured by leaving no slot
  **unfilled** — which is what makes `System` safe to offer. `install` writes each palette into its
  own slot with `set_visuals_of`, never the untargeted `set_visuals`, which writes to whichever slot
  is active and would fill one and leave the other stock, depending on what the OS happened to
  prefer at construction. `install_fills_both_slots_and_leaves_neither_stock` pins it.
- **Why a choice and not just the OS.** The case the OS cannot serve is the one a reading
  application most needs: a dark room and a desktop set to light. Following alone leaves that user
  with no move.
- **Why device-local.** Every other preference rides the mutable surface and *syncs between a user's
  own devices* ([ADR-0011 §5](0011-decks-and-settings.md)), which is right for a new-card rate and
  wrong for a theme — a desktop under a lamp and a handset in bed want opposite answers, and a
  synced theme would have each clobber the other on every write. It goes to the `local` table, which
  no sync path reads. **That table has until now held only sync machinery** — the sequence
  highwater, the lamport counter, the writer and collection ids — so this is the first row in it that
  a person chose.
- **The store keeps the string uninterpreted.** `Collection::theme_preference` returns it as
  written; `theme::ThemeChoice::parse` is the only place that decides what it means. This is
  [ADR-0011 §3](0011-decks-and-settings.md)'s division with one change: a theme is *presentation*, so
  the interpretation belongs to `cairn-app` rather than to `cairn-core`. A domain crate has no
  business knowing what "light" is. An unrecognised value — an older build's, after a downgrade —
  degrades to `System` rather than refusing to start.
- **The decision is the three options, not `ThemePreference`.** A native client that never links
  egui honours this section by offering the same three against its own platform setting. Stated this
  way for the reason [ADR-0035](0035-the-vertical-anchor.md) stated its rule as *touch* rather than
  as Android: the client-stack migration is out of the design pass's scope and must inherit the
  decision without inheriting egui's way of carrying it.

### 4. The role functions read the **ambient** visuals, and §3's invariant is restated

`theme::card_fill`, `control_fill`, `primary_fill` and `link` returned dark constants. All four roles
already ride an ambient slot — the card on `extreme_bg_color`, an ordinary control on
`faint_bg_color`, the primary on `widgets.inactive.bg_fill`, the link on `hyperlink_color` — so they
now **read the slot**, which is both closer to [ADR-0030 §1](0030-the-first-finish-pass-decisions.md)'s
*"every screen keeps reading the ambient visuals"* and the only version that draws the right colour
in two themes. Returning a constant would paint a dark card on a light page with nothing failing.

`card_stroke` and `card_divider` are the exception: egui has no "card edge" slot, and the light
palette had to pull the card's edge apart from the separator (one rung in dark, two in light, because
a well's edge follows the well). They branch on the theme — a narrow case, and the same shape as
[#121](https://github.com/amin-bf/cairn/issues/121)'s open question about values the renderer offers
no slot for.

> **§3's invariant is two claims about what a screen can show, not three ratios against a page:**
>
> 1. **On a screen with a card**, every control on it is quieter than the card — dark 1.099 < 1.121,
>    light 1.049 < 1.292.
> 2. **On a screen with no card**, the primary is louder than an ordinary control.
>
> Both hold in both themes. The three-ratios form was always an artifact of dark's geometry.

Every contrast test in `theme.rs` and the weight tests in `controls.rs` now run against **both**
palettes. A rule checked in one theme says nothing about the other, which is precisely how the
design project's light placeholders came to break §3 without anything failing.

### 5. What this ADR does *not* settle

- **The 7:1 floor's number.** ADR-0030 §3 argued it from a 9px small tier;
  [ADR-0032 §1](0032-the-type-scale-and-the-rhythm.md) raised small to 12px and
  [#125](https://github.com/amin-bf/cairn/issues/125) then judged that tier legible at arm's length
  on a handset at low brightness. The floor kept its number and lost its argument. It is kept
  because no evidence pushes it either way, not because the original reasoning survives; reopening
  the number needs no permission.
- **Whether the light palette survives an OLED panel in a dark room.** That is
  [#126](https://github.com/amin-bf/cairn/issues/126)'s, which this ticket blocks precisely so light
  is judged in the same sitting.
- **The three dormant accents.** `CLAY_L` and `ROSE_L` are drawn and have no call site, exactly as
  their dark counterparts do. [ADR-0030 §5](0030-the-first-finish-pass-decisions.md)'s rule is
  unchanged: defined-and-dormant is the correct state, and a second theme's worth of them is not an
  invitation to find callers.

## Amendments to accepted ADRs

| ADR | What changes | Why |
|---|---|---|
| [ADR-0030 §2](0030-the-first-finish-pass-decisions.md) | **Superseded.** Dark is no longer pinned and theme-following is no longer dropped; the application offers System / Light / Dark and fills both slots. §2's *refusal* survives, re-expressed: no reachable slot may be left on stock egui. | §3 above. §2's stated reason — that only one palette was drawn — is discharged. |
| [ADR-0030 §3](0030-the-first-finish-pass-decisions.md) | The floor now binds **both** themes, and its stated premise (*"the small text style is 9px"*) is recorded as false rather than left standing. The number is unchanged. | §4, §5 above. |
| [ADR-0030 §6](0030-the-first-finish-pass-decisions.md) | Its two open items — *a light palette* and *the restoration of system-following* — are **discharged**. | §1–§3. |
| [ADR-0033 §3](0033-the-card.md) | Its invariant is restated as two claims about what a screen can show. Its three page-relative ratios are kept as **dark's measurements**, not as the rule. | §4 above: the ratio form is satisfiable on a light page while the separation it protects disappears. |
| [ADR-0034 §2](0034-the-controls.md) | Its weight table gains a light column: ordinary 1.049:1, card 1.292:1, primary 1.883:1. *"The card sits between the first two"* is dark's arithmetic; in light the card is still between them, by a different construction. | §1, §2 above. |
| [ADR-0012 §8](0012-the-note-authoring-experience.md) | **Unchanged, and re-checked.** ADR-0030 §2 required *bold is a face, never a colour* be re-argued against a light body. `markdown_job` sets `font_id` for bold and never touches `color`, so the decision transfers with no change; the rejected alternative — brightening — would have inverted its sense, which is why §2 flagged it. | §1. Recorded because the check was owed and passing it silently would look like it was skipped. |

## Glossary

New terms are added to [`ui`'s `CONTEXT.md`](../../crates/app/src/CONTEXT.md): **Ink construction**,
**Appearance**. The **Palette** entry is revised to carry two themes rather than one, and the
**Contrast floor** entry to record that it binds both.

## Consequences

- **The application has two palettes, and every colour rule now has to be checked twice.** That is
  the standing cost of this ADR, and the tests carry it: contrast, the weight ordering and the
  slot-coverage check all iterate both. `both_palettes_name_the_same_slots` exists because a field
  set in one palette and left stock in the other is invisible until someone switches theme.
- **A user on a light desktop no longer sees a dark application by default.** This reverses a real
  behaviour change ADR-0030 §2 made deliberately, and it is the second time this behaviour has moved;
  it is recorded rather than silent, both times.
- **The badge's weight is now named in light and derived in dark.** `weak_text_alpha` is 0.6, and 60%
  of a near-black over a light ground lands much closer to the ground (~4.2:1) than 60% of a
  near-white over a dark one (~5.6:1). Left alone, light's badge would be quieter than dark's by an
  accident of compositing rather than by ADR-0030 §4's decision, so the light visuals set
  `weak_text_color` explicitly. The two themes differ in *mechanism* and agree on the *weight*.
- **`local` now holds a preference.** A table that was sync machinery has acquired a user-facing row.
  It is one row and it is named, but the next thing to want device-local storage will find a
  precedent rather than a seam, and that is worth noticing before there are three.
- **ADR-0033 §2's `clear_color` trap did not reacquire, and the reason is reusable.**
  `CairnApp::clear_color` takes the **active** `&Visuals` and `page_color` returns its `panel_fill`,
  so the page follows whichever slot is live and the light theme inherits the fix rather than needing
  its own. §2's defence was written against the visuals rather than against a constant, and that is
  what made it transfer. The light theme's exposure was always a *different* one — an unfilled light
  slot means stock egui, not eframe's unnamed `#080808`.
- **ADR-0033 §2's card/text-field carve-out is false on `main`, and has been since the card landed.**
  §2 accepts that a card shares `extreme_bg_color` with a text field, on the grounds that the two are
  *"told apart by an 8px corner against the widget's 2px, and by never appearing on the same
  screen"*. The **editor puts them side by side** — the Front and Back fields in the left column, the
  card faces in the Cards pane on the right — in **both** themes, and the captures show it plainly.
  Nothing is wrong with the colours and nothing fails; the *justification* is what is untrue. This
  ADR does not fix it, because whether the two diverge or the editor changes is a card question and
  not a palette one; it is recorded here because drawing the palette twice is what made anyone look,
  and handed to [#121](https://github.com/amin-bf/cairn/issues/121)'s fog.
- **[#132](https://github.com/amin-bf/cairn/issues/132)'s bequest held.** `typography::install` and
  `spacing::install` write through `all_styles_mut` and are pinned for both slots, so the light
  palette inherited the scale and the rhythm rather than getting stock egui's 13px body and 8×3
  spacing. It is the one thing about this ticket that needed no work, and it needed none because
  #132 spent a decision on it.

## Open items handed onward

| Item | Owner |
|---|---|
| Whether 7:1 is still the right floor, now that its premise is gone (§5) | Whoever reopens the palette; needs no permission |
| Judging the light palette on an OLED panel in a dark room | [#126](https://github.com/amin-bf/cairn/issues/126), which this ticket blocks |
| Whether the `#dee2e3` page is right in daily use — it is a judgement with no arithmetic behind it (§2) | Post-implementation, like [ADR-0010](0010-leeches.md)'s thresholds |
| A second device-local preference, if one is ever wanted, and whether `local` is the right seam | Whoever wants it |
