# The app inside its page frame

Twenty-one captures of the **shipped app** after [#131](https://github.com/amin-bf/cairn/issues/131)
landed the page frame — the *after* to
[`docs/design/baseline-2026-08-08/`](../baseline-2026-08-08/README.md)'s *before*, taken by the same
harness from the same storyboard and the same seed, so the pair differs in the app and nothing else.

The decision and its reasoning are [ADR-0031](../../adr/0031-the-page-frame.md); the four candidate
frames that were judged to reach it are the tag `prototypes/issue-131`.

## What produced these

```sh
cargo build -p cairn-desktop --bin cairn
scripts/capture-frame-widths.sh docs/design/framed-2026-08-09
```

Nothing appears on the operator's screen and nothing touches their collection — the app runs inside a
nested compositor on a throwaway profile (`docs/environment/desktop-capture.md`).

## Three widths, not the usual two

| | why |
|---|---|
| `1280x800` | the width the design pass judges at |
| `880x800` | **new.** Just below `frame::TWO_COLUMN_MIN_WIDTH`, so the editor's fallback is photographed rather than asserted |
| `560x860` | the app's own default window |

The pair is what makes the map's *one responsive design* claim checkable. The third exists because
ADR-0031 §4 introduced the app's **second** arrangement change, and a threshold with no capture
either side of it is a claim.

## What to look at

**`01`–`03`, Review.** The frame alone: same palette, same type, same controls as the baseline, and
1280px of window no longer buys a 1280px *Good* button. The nav row's buttons start on the same
vertical line the card does.

**`04`, the note list.** A column instead of an edge-to-edge sprawl. The rows are still left-packed
and still spend none of their width — that is the Notes slice's to answer, and the frame deliberately
does not prejudge it.

**`05`, the editor — the screen that changed most.** At 1280 the two panes are genuinely side by
side for the first time: form left with the header travelling with it, cards right. At 880 and 560
it is one column with the `Write | Cards` toggle, exactly as before. It opens an existing note rather
than a fresh draft, because a draft has no cards and the right column would be an empty demonstration
of the thing these captures exist to show.

Note that the nav row **moves** between `04` and `05` at 1280: it follows the editor's wider frame,
which is ADR-0031 §3's deliberate trade.

**`06`–`07`, Settings.** Every element on the screen now shares one right edge, and the sync
paragraph — 150 characters on one line in the baseline — wraps at a measure the eye can track back to.

## Reading them

Look at the images. #122's finding stands: a storyboard cannot tell you it missed, and this run
changed the coordinates every screen is reached by, so the risk of a plausible-looking wrong capture
was higher here than usual. Each of the twenty-one was checked against the screen it claims to be.
