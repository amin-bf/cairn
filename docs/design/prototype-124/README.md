# Four takes on the Review slice

The primary source for [#124](https://github.com/amin-bf/cairn/issues/124): forty captures of a
throwaway prototype, at both judging widths, of four structurally different Review screens.

**This never merges into `main`.** It is preserved as the tag **`prototypes/issue-124`**, which is
this repo's standing convention for prototypes — `prototypes/issue-8`, `-11`, `-20`, `-28`, `-67`
and `-120` are its predecessors, and every one of them is reachable by tag and contained in no
branch.

The convention is right, and the argument for breaking it was wrong. That argument ran: tickets are
worked in parallel worktree sessions branching from `origin/main`, #124's four children all continue
from variant E, so a prototype those sessions cannot see is one each of them would rebuild
differently. The premise is true and the conclusion does not follow — **a tag is fetched by every
clone**. Any session can read this without merging it:

```sh
git show prototypes/issue-124:docs/design/prototype-124/README.md
git checkout prototypes/issue-124 -- crates/desktop/src/bin/review-prototype.rs
```

So `main` keeps the validated decision, the options that lost stay reachable beside it, and neither
a throwaway binary nor fifty PNGs sit in the tree of every future checkout.

**What did land on `main` is the two harness fixes** — the swallowed storyboard `export` and the
pinned `CAIRN_BIN`. Those are bugs in a tool the repo keeps, not prototype material, and they went
up as their own pull request.

## What produced these

`crates/desktop/src/bin/review-prototype.rs`, driven by `scripts/capture-prototype.sh`:

```sh
cargo build -p cairn-desktop --bin review-prototype
scripts/capture-prototype.sh          # all four variants, both widths
```

It is a **separate binary** rather than a flag inside `cairn-app`, because a prototype has to be
free to break the app's rules — its own type sizes, its own spacing, its own idea of what a card is
— and a variant switch threaded through the real crate would leave that freedom behind in
production code once the question was answered.

Nothing here is wired into the app. No test covers it. It reads no collection.

## What is held constant, and why

**The palette is ADR-0030's, unchanged, in all four.** That is the single most important thing about
this set. Holding colour still is what lets the images be read as statements about *arrangement,
type and rhythm* — and the result is a finding in its own right: all four read markedly better than
the baseline while spending none of ADR-0030's supersession budget.

**Everything else varies per variant, as a complete set.** A hero-card layout and a dense two-column
layout do not want the same type scale, and giving them one is how a scale ends up decided in the
abstract — the cost ADR-0030 itself records, having been judged "from measurements and wireframes,
not a build".

| | display | heading | body | label | small | unit | margin | measure | radius | control |
|---|---|---|---|---|---|---|---|---|---|---|
| **A** framed column | 24 | 19 | 15 | 15 | 12 | 4 | 20 | 640 | 3 | 40 |
| **B** card is the screen | 40 | 20 | 15 | 15 | 12 | 8 | 28 | 560 | 8 | 44 |
| **C** two columns at width | 30 | 19 | 14 | 14 | 11 | 6 | 24 | 1040 | 4 | 42 |
| **D** grades as one row | 36 | 19 | 15 | 14 | 11 | 6 | 24 | 720 | 6 | 48 |

Every gap is **stated**: the prototype zeroes egui's ambient `item_spacing` on the first line it
draws. Without that, stock's 8×3 is added between consecutive widgets on top of every explicit
`add_space`, so a row sized as *n* controls plus *n−1* stated gaps overruns its column by
`(n−1) × 8` — which the first capture run showed as *Edit note* being wider than the grade row above
it. It also means the table above is the truth rather than an intention: a variant that says its
rhythm is 8 draws 8.

Hit targets follow touch at both widths, per the map's Notes — the `control` column never shrinks
because the window grew.

## The five states

| | |
|---|---|
| `1-picker` | The entrance: a fresh deck and the count choice |
| `2-question` | A card shown, answer hidden |
| `3-revealed` | Revealed: both faces, the box badge, four grades, *Edit note* |
| `4-empty` | Caught up — nothing due |
| `5-midsitting` | Revealed, two of five graded |

`5-midsitting` exists because **the dashboard cannot be judged at zero**: an empty progress rule and
an empty row of ticks are both just absence, and three of the four variants say something different
about progress only once there is some. The baseline set has the same pair — `03-review-revealed`
and `12-review-mid-session`.

`4-empty` has **no baseline to compare against**. The capture seed always leaves cards due, so the
caught-up state has never been photographed on `main` at all. That is worth knowing independently of
this ticket.

## Widths

`1280x800` is the width the design pass judges at; `560x860` is the app's own default window. Both
sets carry all twenty images. The pair is what makes the map's *one responsive design* claim
checkable — and what it currently shows is that only variant **C** changes arrangement with width.
A, B and D centre a column and leave roughly half of the 1280 empty, which is a decision to take
rather than a defect to fix.

## Reading them

Look at the images. A storyboard cannot tell you it missed, and this run proved that twice: a
subshell was swallowing the storyboard's `export`, so every shot after the first photographed the
previous screen under the new screen's name (fixed in `capture-desktop-session.sh`), and a nav bar
built with `horizontal_centered` silently claimed the whole page. Both were caught by looking and by
nothing else.
