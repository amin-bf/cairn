# The app with a card in it

Eighteen captures of the **shipped app** after [#133](https://github.com/amin-bf/cairn/issues/133)
made a card one object cut into the page — the *after* to
[`docs/design/typed-2026-08-11/`](../typed-2026-08-11/README.md), taken by the same harness from the
same storyboards and the same seed, so the pair differs in the app and nothing else.

The decision and its reasoning are [ADR-0033](../../adr/0033-the-card.md). The candidates that lost
are the tag `prototypes/issue-133`, with their readme at
`git show prototypes/issue-133:docs/design/prototype-133/README.md`.

## What produced these

```sh
cargo build -p cairn-desktop --bin cairn
scripts/capture-desktop.sh scripts/storyboards/baseline.txt 1280 800
scripts/capture-desktop.sh scripts/storyboards/baseline.txt  560 860
scripts/capture-desktop.sh scripts/storyboards/persian.txt   560 860
```

Nothing appears on the operator's screen and nothing touches their collection — the app runs inside a
nested compositor on a throwaway profile (`docs/environment/desktop-capture.md`).

## What changed

| | before | after |
|---|---|---|
| the page | eframe's default `rgba(12,12,12,180)` → **`#080808`**, on every screen | **`panel_fill`** — `#1a1e21`, the palette's |
| a card | **two** slabs on `widgets.inactive` (`#2c3237`), *lighter* than the page | **one** card on `#0f1214`, *darker* than the page, halves divided by a hairline |
| corner | 2px, the widget radius | **8px** |
| card height | 96px per slab, fixed | **300px floor**, growing when the content needs it |
| the face | always 40px | **40 → 20 → 15**, stepping down to fit, floored at body |
| the box badge | on the page below the card, always left | **inside the card**, in the corner reading does not begin at |
| badge case | `Box 3` | **`box 3`** — ADR-0030 §4, which had never landed |

## What to look at

`03-review-revealed` is the decision. `01`/`02` are the same card before the reveal.

**`11-cards-persian-display` is the one that could not have been checked before this ticket**: a
Persian card in the editor's pane, with the badge on the **left** — the corner a right-to-left
reader's eye does not start from. A fixed top-right corner is a footnote in Latin and the first thing
seen in Persian, which is the whole argument of ADR-0033 §5, and no capture in this repository could
have shown it until #132 put Persian on a card face at all.

The nav strip is no longer lighter than the page beneath it. That was true of **every capture this
repository holds** before this set, and nobody had noticed.

## What is deliberately unchanged

The grade buttons are still four stacked full-width controls. ADR-0033 §3 requires only that they end
up **quieter than the card**, and *what they look like* is [#134](https://github.com/amin-bf/cairn/issues/134)'s
— so this set shows a card that is correct and controls that still outweigh it. That is the expected
intermediate state, and it is photographed rather than described so the next ticket starts from the
picture it has to fix.
