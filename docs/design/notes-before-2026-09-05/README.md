# The Notes destination, before it moves

Twenty-six captures of the **shipped app** at `531e449`, taken to size the Notes slice
([#150](https://github.com/amin-bf/cairn/issues/150)) and to give its two children a *before* they
can be judged against.

This is the second destination the design pass takes, and unlike Review it inherits settled
foundations: the frame ([ADR-0031](../../adr/0031-the-page-frame.md)), the type scale and rhythm
(0032), the card (0033), the controls (0034), the vertical anchor (0035), the palette (0036), motion
and elevation (0037), and the mark and the icon rule (0038). Nothing here is a picture of an
undecided foundation. Everything here is **arrangement**.

## What produced these

```sh
cargo build -p cairn-desktop --bin cairn && cargo build --bin cairn-fixture
for wh in "1280 800" "560 860" "880 800" "920 800"; do
  scripts/capture-desktop.sh scripts/storyboards/notes.txt $wh
done
scripts/capture-desktop.sh scripts/storyboards/notes-light.txt 1280 800
scripts/capture-desktop.sh scripts/storyboards/notes-light.txt  560 860
scripts/capture-desktop.sh scripts/storyboards/notes-persian.txt 1280 800
```

All three storyboards name the `backlog` fixture, which is twenty-five notes — the largest list the
bench can produce, and the first time the bench has been used for a **list** state rather than a
queue state. Nothing appears on the operator's screen and nothing touches their collection
(`docs/environment/desktop-capture.md`).

**Every capture here was checked for its page colour** before being committed, not counted. #143
produced seven perfectly valid captures of the wrong theme and #122 named the failure before that;
the light set is `#dee2e3` and the dark set `#1a1e21` in all twenty-six.

## Why four widths

880 and 920 exist for one reason: `frame::TWO_COLUMN_MIN_WIDTH` is **900**, and the editor's whole
arrangement flips across it. `04-editor` in those two directories is the same screen 40 pixels
apart, and it is the evidence for #150's second question.

## What to look at

### `01-list` — the row spends none of its column

Every row is `preview · Move · Delete`, each control sized to its own text, so *Move* and *Delete*
land at a different x on all twenty-five rows. The longest row spends 208px of the 640px measure.
**The row is pixel-identical at 1280 and at 560**, so the frame's leftover width does nothing here —
which is the state [#131](https://github.com/amin-bf/cairn/issues/131) left deliberately, having
settled the container and refused to prejudge the contents.

Above the rows, three groups with no boundary between them: *Create note* as a full-width slab at
the very top, the deck block, and *Search*. They are separated by one gap unit — the same unit that
separates one row from the next.

### `02-list-deck-open` — the app's only overlay, holding the only entry a fixture can produce

Since [ADR-0037](../../adr/0037-motion-and-elevation.md) it rises, takes a chosen edge and casts a
shadow, which is [#149](https://github.com/amin-bf/cairn/issues/149)'s *"a colour nobody chose"*
fixed. What it does not have is anything to show: **no fixture in the bench creates a deck**, so
*All decks* is the only entry, and the filter, *Unfiled*, *Delete deck* and the question of whether
a row carries its deck have never been photographed at all.
[#161](https://github.com/amin-bf/cairn/issues/161) is the fixture that fixes this.

### `03-placement` — the targets outweigh the content

After *Move*: twenty-six identical full-width *Place here* slabs, with the notes they are placed
between set as plain body text in the gaps. The behaviour is bound (ADR-0021 §4, ADR-0006 §5 — two
taps, no drag, no long-press, identical under touch and mouse); the drawing is
[#162](https://github.com/amin-bf/cairn/issues/162)'s.

### `04-editor` — a card and a text field are the same colour

Compare `880x800/04-editor.png` with `920x800/04-editor.png`. At 880 the editor is in the **phone's**
`Write | Cards` toggle with the Cards pane hidden and **450px of empty page below the last field**;
at 920 it is two columns. The toggle exists because a soft keyboard eats *vertical* room, and at 880
there is no soft keyboard and the vertical room is going spare.

Then compare the two-column shot against `light-1280x800/04-editor.png`. ADR-0033 §2 accepts a card
and a text field sharing `extreme_bg_color` on two grounds — an 8px corner against the widget's 2px,
*and* that the two never appear on the same screen. The second has been untrue since the card
landed, which [#143](https://github.com/amin-bf/cairn/issues/143) found. Measured off these two
captures:

| | page | fill (both) | fill : page | **field : card** |
|---|---|---|---|---|
| dark | `#1a1e21` | `#0f1214` | 1.121:1 | **1.000:1** |
| light | `#dee2e3` | `#c4c8c9` | 1.292:1 | **1.000:1** |

They are not *similar*. They are the same colour, in both themes, and the corner is carrying the
whole distinction alone.

### `06-editor-persian` and `07-list-persian` — one row breaks the frame

In the editor, Persian behaves: fields right-align, the card face reads right-to-left, and the box
badge mirrors to the prompt's direction as ADR-0033 §5 says. The `field_label`s do not mirror, and
neither do the two columns, so an RTL note reads its form on the wrong side of the page.

On the list, a Persian row is drawn **outside its own button and outside the page margin**. Measured
off `07-list-persian.png` at y=252, against a Latin row at y=279:

```
latin   row  button 320→414   Move 423→468   Delete 477→528
persian row  glyphs 274→292   button 318→378 (empty)   Move 387→431   Delete 440→492
```

The frame's column starts at 320. The button is allocated 61px and the word needs more, so 44px of it
is drawn outside the button and 46px outside ADR-0031's left margin, and the button itself renders
empty. Nothing failed, and it is the note-list relative of
[#132](https://github.com/amin-bf/cairn/issues/132)'s card face drawing Persian 455px off the
window.

**This one is fixed** — `fc4e129c` on `main`, ahead of
[#162](https://github.com/amin-bf/cairn/issues/162) rather than inside it, because the cause was not
the row. `bidi::job` set `halign = Align::RIGHT` on a right-to-left paragraph, and epaint aligns a
galley's rows against the **origin** rather than the wrap width, so the row laid out into negative
x — to the left of wherever the widget drew it. It was documented as a *direction marker* that every
caller must undo, and two of eleven did: `surface::face`, after #132 found this on the card face,
and `bidi_layouter`, because a `TextEdit` clips at a fixed origin and visibly ate the last character
of every Persian line. The note that told the other nine not to bother was wrong — it said a label
is not clipped because it allocates from `galley.size()` and reserves the space the text hangs into,
and it reserves the *width* while the ink hangs to the **left** of it.

So `07-list-persian.png` is the one capture in this set that is a picture of a **fixed** defect
rather than a live one. It is kept as taken: the *before* is the point of the set, and every other
screen here is pixel-identical across that fix — the change touches right-to-left text and nothing
else, which is the first thing it was checked for.

## What the reach line does here

Nothing, and that is the finding. [ADR-0035](../../adr/0035-the-vertical-anchor.md) §1 is a **page
rule** after [#155](https://github.com/amin-bf/cairn/issues/155) — *the last control on a screen sits
on a reach line* — and `frame::slack_above` still has exactly two call sites, both in
`screens/review.rs`. Both Notes surfaces end their content high and leave the bottom third empty,
which is [#125](https://github.com/amin-bf/cairn/issues/125)'s *arranged for a pointer while sized
for a thumb* on two more screens. #150's body asked this slice to **amend** §1; #155 got there
first, so both children inherit **apply** it.

## The trap in taking these

**The editor's frame has no harness token.** #131 gave it `frame::cap_for` → 1120, so above 900 its
column starts at x=80 where the list's starts at 320. The harness has `%LX+n%` for the *list's*
frame and nothing for the editor's, and `%CX%` — 640 — is fourteen pixels past a field's right edge
at 1280 while reaching it at 560.

`scripts/storyboards/persian.txt` aims at the editor with `%CX%` and has been missing there since
#131; the first run of `notes-persian.txt` did the same, typed nothing, and photographed the French
note under the Persian name. That is #122's silent miss arriving from a **fifth** side, and the
first one caused by a *frame* rather than by a coordinate — the four before it were a literal x
(#132), a literal y under wrapping prose (#143), a missing fixture (#153), and a y made
height-dependent by the ticket's own decision (#155). `notes-persian.txt` is pinned to one width and
says so; [#163](https://github.com/amin-bf/cairn/issues/163) owns the choice between that and adding
the token.
