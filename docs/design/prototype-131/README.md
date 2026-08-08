# Four takes on the page frame

The primary source for [#131](https://github.com/amin-bf/cairn/issues/131): thirty-two captures of a
throwaway prototype, at both judging widths, of four page frames applied to four screens.

**This never merges into `main`.** It is preserved as the tag **`prototypes/issue-131`**, which is
this repo's standing convention for prototypes — `prototypes/issue-8`, `-11`, `-20`, `-28`, `-67`,
`-120` and `-124` are its predecessors, and every one of them is reachable by tag and contained in
no branch. A tag is fetched by every clone, so a later worktree reads this without merging anything:

```sh
git show prototypes/issue-131:docs/design/prototype-131/README.md
git checkout prototypes/issue-131 -- crates/desktop/src/bin/frame-prototype.rs
```

## What produced these

`crates/desktop/src/bin/frame-prototype.rs`, driven by `scripts/capture-frame.sh`:

```sh
cargo build -p cairn-desktop --bin frame-prototype
scripts/capture-frame.sh              # all four frames, both widths
PROTO_FRAME=f2 PROTO_SCREEN=live \
  cargo run -p cairn-desktop --bin frame-prototype   # click through the destinations
```

Nothing here is wired into the app. No test covers it. It reads no collection.

## What is held constant, and why

**Everything except the frame, and it is all variant E's.** Palette, type scale, rhythm unit, corner
radius, control height, card treatment and grade row are E, unchanged, in all four frames. That is
the same discipline #124 used when it held the palette still, applied one level down: an image here
is a statement about the frame and nothing else, and the type and the card are #132's and #133's to
move.

It is also why **F0 exists**. The `docs/design/baseline-2026-08-08/` captures are not the control for
this question — they differ in type as well as frame, so a difference between them and F1 cannot be
attributed. F0 is today's frame (no margin, no measure) drawn with E's type, so F0 → F1 isolates the
frame exactly.

## The four frames

Three numbers and one alignment rule.

| | page margin | read measure | work measure | nav aligns to |
|---|---|---|---|---|
| **F0** no frame | 0 | — | — | window edge |
| **F1** one column | 28 | 620 | 620 | the one column |
| **F2** two measures | 28 | 620 | 960 | the work column |
| **F3** rooms | 28 | 620 | window − margin | the page margin |

**`read` versus `work` is the prototype's hypothesis.** A measure exists because a long line of prose
is hard to track back to — a fact about *reading*. A list of rows and a column of form fields are not
reading. Whether that distinction earns two numbers, or whether one number everywhere is simpler and
good enough, is what the images are for.

**The nav aligns to the widest column its frame allows**, so it is one fixed vertical line however
the destination beneath it is drawn. The alternative — aligning the nav to whatever measure the
current destination uses — was rejected before it was drawn: the row is pinned (ADR-0021 §1), and a
pinned row that slides sideways when you change destination is worse than one that is merely offset
from the card. The F2 and F3 Review images are the cost of the choice that was kept.

**960 is not picked for looks.** It is the smallest working column that leaves the editor's 640
threshold clear rather than sitting on it, with room for the margin either side inside a 1280 window.

## The four screens

| | |
|---|---|
| `1-review` | Revealed. The settled screen — here to show the frame leaves E alone, not to be re-judged. |
| `2-notes` | The note list: rows, the deck filter, search. |
| `3-editor` | **The screen that decides the ticket.** |
| `4-settings` | Prose and full-width controls stacked in one scroll. |

The note list's actions are drawn **right-aligned** rather than packed against the title as the app
draws them today. That is not a row redesign smuggled in — it is what makes the comparison honest.
Left-packed, every frame looks identical because the row never uses its width at all, and the
question "what does the leftover width do" cannot be photographed.

## What the images show

**The editor is the whole decision.** `TWO_PANE_MIN_WIDTH = 640` in
`crates/app/src/screens/notes.rs` is a test on `ui.available_width()` — which under a frame is the
*column's* width, not the window's. So F1 puts a 1280px desktop window into the phone's
`Write | Cards` toggle (`1280x800/f1-3-editor.png`), and F2 does not (`f2-3-editor.png`). Nothing
fails; the desktop simply becomes a phone.

And the threshold is a worse fit than it looks. **The editor's two panes are stacked vertically with
a rule between them, not side by side** — `notes.rs` has no `horizontal` there. So 640 is a *width*
test standing in for a question about *vertical* room, and the code comment beside it already half
admits this: "the soft-keyboard failure ADR-0025 addresses is vertical, so the toggle stands on its
own merits rather than necessity". A frame is what makes that latent oddity load-bearing.

**Settings is the one screen where one column beats two measures.** In F1 every element on the screen
shares one right edge (`f1-4-settings.png`). In F2 the paragraphs stop at 620 while the buttons above
them run to 960, so each section is ragged in a way that reads as an accident rather than a decision
(`f2-4-settings.png`).

**F3 fails on the note list.** At 1280 the row is 1224px wide, and `chien` and its `Move`/`Delete`
sit 1100px apart with nothing between them (`f3-2-notes.png`). The association between a row and its
actions is what the width breaks.

**F1, F2 and F3 are pixel-identical at 560×860, on every screen.** Verified by cropping the caption
strip and diffing: `magick compare -metric AE` reports 0 differing pixels for every F1/F2 and F1/F3
pair. At 560 the column is `560 − 56 = 504` in all three, which is below every cap any of them names.
So **the choice between them is a desktop-width decision that costs the handset nothing** — and the
handset checkpoint (#125) cannot tell them apart, which is worth knowing before it is spent.

**The margin is unarguable at 560, not at 1280.** F0 at the narrow width has text touching both edges
and buttons bleeding off the frame (`560x860/f0-4-settings.png`); the 28px gutter costs 10% of the
width and buys the screen back. At 1280 the margin is nearly invisible next to the measure, which is
the reverse of how the two numbers are usually argued about.

## Widths

`1280x800` is the width the design pass judges at; `560x860` is the app's own default window. Both
sets carry all sixteen images, because the map holds **one responsive design** and the pair is what
makes that claim checkable.

## A harness finding, and a candidate for `main`

`scripts/storyboards/frame-prototype-live.txt` is a smoke test for the click-through mode, written
because the README above tells a reader `PROTO_SCREEN=live` gives them one, and a claim about a mode
nobody has run is exactly what #122 found fails silently. Running it turned up something about the
harness rather than the prototype.

**Sent as one line — `mousemove 430 24 click 1` — none of three nav clicks reached a widget.** Split
into `mousemove 430 24`, a settle, then `click 1`, **all three landed**, first click included. Same
binary, same coordinates, same run length; the only variable was whether the motion was delivered as
its own event before the press.

That refines `docs/environment/desktop-capture.md`, which currently says *"the first click of any
storyboard is spent giving the window keyboard focus and never reaches a widget … this is not a
timing problem and more settle time does not fix it."* The first half holds for the combined form.
The conclusion does not generalise: it is not settle time, but it *is* fixable, by making the motion
its own event. The app's own storyboards get away with the combined form by following every click
with `sleep 1`, which hides the distinction rather than resolving it.

Scope of the evidence: two runs, one variable, three clicks each, F1 at 1280 only. Enough to write
down, not enough to rewrite the section without a wider check.

**This part belongs on `main`** — it is a fact about a tool the repo keeps, not prototype material —
and per `AGENTS.md` it should be cherry-picked onto its own branch and land as its own pull request,
the way #124's two harness fixes did.

## Reading them

Look at the images. Each one carries a caption strip along its bottom edge naming its frame, its
screen and the window width it was drawn at — thirty-two images of four frames are
indistinguishable once they leave their directory, and #122's finding was that a storyboard cannot
tell you it missed.
