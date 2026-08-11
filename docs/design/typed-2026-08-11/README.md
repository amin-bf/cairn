# The app in its own type and its own rhythm

Twenty-five captures of the **shipped app** after [#132](https://github.com/amin-bf/cairn/issues/132)
gave it the first type sizes and the first spacing unit it has ever named — the *after* to
[`docs/design/framed-2026-08-09/`](../framed-2026-08-09/README.md), taken by the same harness from the
same storyboards and the same seed, so the pair differs in the app and nothing else.

The decision and its reasoning are
[ADR-0032](../../adr/0032-the-type-scale-and-the-rhythm.md). The direction is **variant E**, chosen in
[#124](https://github.com/amin-bf/cairn/issues/124) as a running sitting and preserved as the tag
`prototypes/issue-124`.

## What produced these

```sh
cargo build -p cairn-desktop --bin cairn
scripts/capture-desktop.sh scripts/storyboards/baseline.txt 1280 800
scripts/capture-desktop.sh scripts/storyboards/baseline.txt  880 800
scripts/capture-desktop.sh scripts/storyboards/baseline.txt  560 860
scripts/capture-desktop.sh scripts/storyboards/persian.txt   560 860
```

Nothing appears on the operator's screen and nothing touches their collection — the app runs inside a
nested compositor on a throwaway profile (`docs/environment/desktop-capture.md`).

## What changed

| tier | before (stock egui) | after |
|---|---|---|
| display | *did not exist* — the card face drew at `Button`, **13** | **40** |
| heading | 18 | **20** |
| body / control text | 13 | **15** |
| small | 9 | **12** |
| the gap between two things | stated 4 / 8 / 12, **drawn 7 / 11 / 15** | stated and drawn **8 / 16 / 24** |

The right-hand column of that last row is the whole of §2: egui added `item_spacing` before every
stated gap, so the old numbers were each wrong by a constant nobody could see.

## Three widths, unchanged from #131's reasoning

| | why |
|---|---|
| `1280x800` | the width the design pass judges at |
| `880x800` | just below `frame::TWO_COLUMN_MIN_WIDTH`, so the editor's fallback is photographed rather than asserted |
| `560x860` | the app's own default window |

`03-review-revealed` is the shot the slice is about, and it is the one to compare against
`framed-2026-08-09` first.

## The four Persian captures, and why one of them is new

`08` and `09` are the existing pair — Persian typed into the editor, then Latin beside it — and they
show the **keymap** path, which is the whole of what Persian input means (#122). `10` is the note list
carrying it.

**`11-cards-persian-display` is new, and it earned its place by failing.** It is the only shot in this
repository that has ever put right-to-left text on a **card face**, and the first run of it showed the
text starting 455px left of the card and running off the window. `bidi` sets `halign = RIGHT` as a
direction marker and requires every caller to reset it; `card_face` never did, and at 13px the
overhang was small enough to read as slightly-off-centre rather than as a bug. The 40px tier is what
made it undeniable.

The reason it survived this long is worth stating plainly: **the seed collection is French.** No
capture here had ever drawn a right-to-left card, so nothing was wrong in any image anyone had looked
at.

## Reading them

Look at the images. Two silent failures were caught in this pass by looking and by nothing else — the
one above, and the Persian storyboard clicking a **literal** x that #131's page frame had moved out
from under it, so it photographed the Review screen three times under three editor names. Every
coordinate in both storyboards was re-measured off a capture rather than adjusted by eye.
