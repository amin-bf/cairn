# The light palette — three constructions and a knob

> **Outcome.** The page is **`#dee2e3`** and the construction is **ink, placed by the gaps** — all
> three fills below the page, the ordinary control at 1.049:1, the card at 1.292:1 and the `primary`
> at 1.883:1, chained off the card. Recorded as
> [ADR-0036](../../adr/0036-the-light-palette.md), landed in #143, drawn in the design project.
>
> **The question was never which light *hues* to pick.** It was which **construction** the three
> weights take on a light page — and the one ADR-0033 §3 and ADR-0034 §1 §2 literally *describe* is
> the one that fails. Carried over as written, their three page-relative ratios hold §3's ordering at
> every page position while the card↔ordinary separation collapses from 1.231:1 to **1.02:1**: the
> card and the buttons become one material, which is #133's original complaint arrived at from the
> other side, with nothing failing. That is the finding the rest of the map inherits, and it is why
> §3's invariant is now stated as two claims about what a screen can show rather than as three ratios.
>
> **The blur decided it, not the arithmetic.** Two constructions were live at the chosen page and the
> measurements favoured the loser: chaining the `primary` off the *ordinary* control instead gives an
> ordinary↔primary of 1.182:1 against dark's 1.177:1, and a comfortable 10.71:1 for its label, where
> the chosen value gives 1.794:1 and **7.06:1**. Blurred on the two card-less screens, the darker slab
> is what reads as the way forward. The 7.06:1 is the tightest reading pair the application has and is
> pinned by figure.
>
> **The page is a judgement with no arithmetic behind it.** Ink delivers full separation at *every*
> knob position, so nothing forced `#dee2e3`; it was chosen by looking, at the end of a knob whose
> readout was in the units the ADR would name. ADR-0036 §2 records it as such.
>
> **Two rounds, and the second existed because the first asked the wrong question.** Round one
> offered three constructions and a page knob. It was only while writing up the winner that
> ADR-0034 §2's *"there is deliberately no call site for a primary beside a card"* turned out to mean
> the **card↔primary gap describes a pair that never appears on a screen** — so the rung the primary
> had been chained off was a measurement of dark, not a requirement. Round two added the alternative
> chaining as a fourth switch and re-ran the blur test at the decided page.

The primary source for [#143](https://github.com/amin-bf/cairn/issues/143) on the design pass map
([#121](https://github.com/amin-bf/cairn/issues/121)).

**This never merges into `main`.** It is preserved as the tag **`prototypes/issue-143`**, contained
in no branch — `AGENTS.md`, *Rules that are easy to break silently* 3.

```sh
git show prototypes/issue-143:docs/design/prototype-143/README.md
git checkout prototypes/issue-143 -- docs/design/prototype-143/light-palette.html
```

## What to open

```sh
xdg-open docs/design/prototype-143/light-palette.html
```

One page, no build, no server. It draws **three screens twice** — the light candidate beside the
shipped dark, unchanged, as the reference:

| screen | why it is here |
|---|---|
| **Review, revealed** | the only screen with a card, so the only one ADR-0033 §3 can be checked on |
| **Caught up** | ADR-0034 §2's `primary` exists for *the one control on a screen with no card* |
| **Settings** | a second card-less screen, and the one with several control clusters |

Controls: a **knob** for where the page sits on the ramp, a **construction** switch, and a **blur**
toggle. The blur is the instrument, not the numbers — ADR-0033 §3 settled the card-versus-controls
question by blurring until nothing is legible and seeing what the eye reaches for first, and that is
the test this has to pass.

## Why a transplant cannot work

The three weights — **1.099:1** ordinary, **1.121:1** the card, **1.293:1** `primary` — were every
one of them measured against a page near the **bottom** of the ramp, and they use both directions:
the card is a well **below** the page, the two controls rise **above** it.

A light page sits near the **top** of the ramp, which inverts which direction is scarce.

| page | total range **above** it | total range **below** it |
|---|---|---|
| `#1a1e21`, the shipped dark page | 16.78:1 | 1.25:1 |
| `#f2f1ed`, the placeholder light page | **1.13:1** | 18.58:1 |

The dark page puts **one** weight in its scarce direction and it just fits — the card needs 1.121 of
the 1.252 available, which is why ADR-0033 §2 could say *"the dark end of the stone ramp is
compressed enough that `STONE_0`-on-`panel_fill` is only 1.12:1"*. A mirrored light page does the
reverse and puts **two** weights in the scarce direction, where the entire budget is 1.130:1 — less
than the *ordinary* control's 1.099:1, never mind the primary's 1.293:1. **On a near-white page a
`primary` lighter than the page does not exist**, at any hue.

## The three constructions

Every one of them re-derives from the shipped dark palette rather than picking values. Body text and
weak text are placed at dark's own **13.34:1** and **5.59:1** away from whatever page the knob
chooses, so they are re-derived too; the stone tint is interpolated off the shipped ramp so the hue
is carried rather than re-picked. The ticket's instruction is *re-derive the weights, do not re-hue
them*, and a hardcoded light body colour would be exactly that re-hue.

### 1. Opposite sides — the dark structure, mirrored

Card **down**, both controls **up**. Reproduces dark's separations exactly — but only once the page
has come far enough down the ramp to afford them.

| page | §3 ordering | card ↔ ordinary | card ↔ primary |
|---|---|---|---|
| `#f7fbfb` near-white | **broken** | 1.163 (94%) | 1.163 (80%) — both controls **clamped to white** |
| `#edf1f1` | holds | 1.229 (100%) | 1.263 (87%) — primary still clamped |
| `#e3e7e7` | holds | 1.227 (100%) | 1.388 (96%) — primary still clamped |
| **`#dee2e2`** | holds | **1.230 (100%)** | **1.454 (100%)** — nothing clamped |
| `#c0c4c5` mid grey | holds | 1.235 (100%) | 1.459 (101%) |

**The cost is the page.** This construction works from about `#dee2e2` down, and `#dee2e2` is a light
*grey*, not paper.

### 2. Ink, keeping the ratios — the literal reading of the ADRs, and it fails

All three **down**, at the page-relative ratios ADR-0033 §2 and ADR-0034 §1 §2 state. This is what a
careful reader implementing those ADRs against a light page would write.

**§3's ordering holds at every page position. It is satisfied everywhere, and it delivers 83% of the
separation at every one of them.**

| page | §3 ordering | card ↔ ordinary | card ↔ primary |
|---|---|---|---|
| `#f7fbfb` | holds | 1.018 (**83%**) | 1.149 (79%) |
| `#dee2e2` | holds | 1.020 (**83%**) | 1.152 (79%) |
| `#c0c4c5` | holds | 1.022 (**83%**) | 1.144 (79%) |

A card↔ordinary gap of **1.02:1** is two colours that are the same colour. The card and the control
beneath it become one material — which is
[#133](https://github.com/amin-bf/cairn/issues/133)'s original complaint, *"the thing being studied
is made of the same material as the buttons under it"*, arrived at from the other direction.

**This is the finding the rest of the map inherits.** §3's invariant is recorded as three ratios
**against the page**, and page-relative ratios record magnitude while throwing away **direction**. In
dark the card and the controls sit on opposite sides of the page, so the stated 1.099 and 1.121 buy
**1.231:1** between them — the numbers understate what the eye gets, and the understatement is
load-bearing. Carried to a light page as written, the invariant passes while the thing it protects
disappears, with nothing failing.

### 3. Ink, keeping the gaps — all three down, and it works anywhere

All three **down**, but placed by the **pairwise gaps** dark actually delivers rather than by its
stated page-relative ratios: the card sits 1.231:1 below the ordinary control, the primary 1.449:1
below the card.

| page | §3 ordering | card ↔ ordinary | card ↔ primary |
|---|---|---|---|
| `#f7fbfb` near-white | holds | 1.224 (99%) | 1.457 (101%) |
| `#dee2e2` | holds | 1.232 (100%) | 1.457 (101%) |
| `#c0c4c5` | holds | 1.221 (99%) | 1.460 (101%) |

**It keeps the near-white page and delivers both separations in full, at every knob position.** What
it gives up is the *structure*: the card is no longer on the opposite side of the page from the
controls, and its page-relative weights come out at **1.05 / 1.29 / 1.88**, which bear no
resemblance to ADR-0033's and ADR-0034's numbers. A `primary` at 1.88:1 below a near-white page is a
dark filled slab — ordinary enough in a light interface, and nothing like its dark counterpart.

## What the sitting decided

1. **Construction 3**, ink placed by the gaps, with the `primary` chained off the card. Blurred, the
   card still takes the eye before the grade row on Review, and the `primary` still reads as the way
   forward on Caught up and Settings — which was the pair that mattered, since construction 3's
   ordinary control is only 1.049:1 off the page and
   [#134](https://github.com/amin-bf/cairn/issues/134) had already found once that *frameless is not
   obviously clickable*.
2. **The page at `#dee2e3`**, by eye.
3. **Theme-following returns as a three-way choice** — System / Light / Dark on Settings,
   device-local, defaulting to System. Grilled separately, not judged here; ADR-0036 §3.

## What the arithmetic said, and where it was overruled

Worth keeping, because the numbers point the other way in two places.

**A mirrored construction is not quite impossible at the chosen page.** Above a paper-white
`#f2f1ed` there is 1.130:1 in total, so a `primary` lighter than the page genuinely does not exist at
any hue. Above `#dee2e3` there is **1.305:1**, which clears the primary's 1.293:1 — by 0.012, with the
fill a hair off pure white. Impossible on a paper page and merely unbuildable on this one. The mirror
lost to the blur, not to the budget.

**And the chosen primary is the one the measurements disliked.** See the outcome above: 1.794:1
against the ordinary control where dark manages 1.177:1, and a label at 7.06:1 where the alternative
offered 10.71:1. Both were drawn at `#dee2e3` and both were looked at.

## What was found on the way, and does not need the sitting

- **The design project's light scope is not merely unmeasured, it is inconsistent and it re-hues.**
  `tokens/colors.css` gives `--surface-card:#fbfaf8` — *lighter* than its `--surface-panel:#f2f1ed`,
  so the card is not a well at all and §3's ordering is broken outright (ordinary 1.096, card 1.083).
  `guidelines/colors-light.card.html` gives the same token as `#e2e1db`, which ties exactly with
  `--surface-primary`. And the whole light scope is **warm** (`#f2f1ed`) where the palette is cool
  slate — a re-hue nobody decided. Its `--text-weak` is **3.92:1**, against dark's 5.59:1.
  That card's subtitle also still reads *"egui follows the system theme"*, which ADR-0030 §2 made
  false.
- **ADR-0033 §2's `clear_color` trap does not reacquire, and the reason is worth keeping.** The
  ticket flags that the light theme has its own clear colour and could independently rediscover the
  defect of drawing on a colour nobody chose. It cannot: `CairnApp::clear_color` takes the **active**
  `&Visuals` and `page_color` returns its `panel_fill`, so the page follows whichever slot is live.
  §2's fix was written against the visuals rather than against a constant, and that is what makes it
  transfer. The light theme's exposure is a *different* one — an unfilled light slot means **stock
  egui**, which is precisely what ADR-0030 §2 refused, not eframe's unnamed `#080808`.
- **[#132](https://github.com/amin-bf/cairn/issues/132)'s bequest holds.** `typography::install` and
  `spacing::install` both write through `all_styles_mut`, and both pin it for `Theme::Dark` and
  `Theme::Light`. A light palette inherits the scale and the rhythm rather than getting stock egui's.
