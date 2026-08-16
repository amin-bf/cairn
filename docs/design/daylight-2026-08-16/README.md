# The app in daylight

Fourteen captures of the **shipped app** in the light appearance, after
[#143](https://github.com/amin-bf/cairn/issues/143) drew a second palette and
[ADR-0036](../../adr/0036-the-light-palette.md) recorded it.

There is no *before* to pair these with, because before them the light theme did not exist — the
application pinned dark ([ADR-0030 §2](../../adr/0030-the-first-finish-pass-decisions.md)). The thing
to hold them against is [`docs/design/controlled-2026-08-12/`](../controlled-2026-08-12/README.md),
which is the same seven screens in dark from the same harness and the same seed, so the pair differs
in the palette and nothing else.

The three constructions that lost are the tag `prototypes/issue-143`, with their readme at
`git show prototypes/issue-143:docs/design/prototype-143/README.md`.

## What produced these

```sh
cargo build -p cairn-desktop --bin cairn
scripts/capture-desktop.sh scripts/storyboards/light.txt 1280 800
scripts/capture-desktop.sh scripts/storyboards/light.txt  560 860
```

Nothing appears on the operator's screen and nothing touches their collection — the app runs inside a
nested compositor on a throwaway profile (`docs/environment/desktop-capture.md`).

**`light.txt` switches the theme through the real control on Settings**, not through a test hook, so
these are pictures of a state the application can actually be put into — and the storyboard fails
loudly if `theme_control` stops working.

## What to look at

| | |
|---|---|
| `01-review-start` | the **primary** on a screen with no card. `#a2a6a7`, 1.883:1 below the page — the one weight the ink construction places furthest, and the one the sitting picked by blurring this screen |
| `03-review-revealed` | ADR-0033 §3, in the theme it was not written for. The card is `#c4c8c9` at 1.292:1 and the grades `#d9dddd` at 1.049:1, so the card is the heaviest mass on the page and the controls are quiet under it |
| `05-notes-editor` | two columns above a 900px window, and the one screen that draws a text field and a card face **side by side** — see below |
| `07-settings-scrolled` | Persian and Arabic in all three families on a light ground. **Bold is visibly a face, not a colour**, which is the re-check ADR-0030 §2 owed [ADR-0012 §8](../../adr/0012-the-note-authoring-experience.md) |
| `06-settings-top` | the Appearance control itself — System / Light / Dark, marked with `selectable_label` the same way the nav marks the current destination |

## Two things these captures caught

**ADR-0033 §2's card/text-field carve-out is false, and was already false in dark.** §2 accepts that
a card shares its fill with a text field because the two are *"told apart by an 8px corner against
the widget's 2px, and by never appearing on the same screen"*. Compare `05-notes-editor` here with
[the dark one](../controlled-2026-08-12/README.md): the Front and Back fields sit in the left column
while the card faces sit in the Cards pane on the right, in **both** themes. Nothing is wrong with
the colours; the justification is untrue. Recorded in ADR-0036's consequences and handed to
[#121](https://github.com/amin-bf/cairn/issues/121)'s fog, because whether the two diverge or the
editor changes is a card question, not a palette one.

**The first run of this storyboard produced seven perfectly valid captures of the wrong theme.** The
Appearance control originally sat below the new-card-rate prose, which wraps to two lines at 560 and
one at 1280 — so a single y hit the control at one width and empty page at the other, and the 560 run
photographed the app in dark under filenames that say light. Nothing failed. The control now sits
directly under the heading, where nothing above it can wrap, and its y is identical at both widths.
This is [#122](https://github.com/amin-bf/cairn/issues/122)'s finding — *a storyboard that misses its
target fails silently* — arriving for the second time, and the reason every image in this folder was
looked at rather than counted.
