# The file surface, with the half that was never drawn

Taken for [A File Bench](https://github.com/amin-bf/cairn/issues/166) on 14 September 2026, at
**1280×800, dark**, by `scripts/storyboards/file-surface.txt` against `fixture decks` and
`files imports`. The same two temporary Settings blocks as
[`file-surface-before-2026-09-05/`](../file-surface-before-2026-09-05/README.md), with one difference
that matters: **the files now name what the collection holds**, so the preview takes the update path.

| | |
|---|---|
| `01-list.png` | The four files the bench wrote through the seam, each described from its own bytes. |
| `02-preview-update.png` | **The update.** Header, *updating a deck you already have*, *6 new, 7 already yours*, a move from an unfiled note and a move from a held deck, *3 of your notes will be deleted*, the rename, and the drained deck *left empty* — every line ADR-0022 §3 exists for, on one screen, for the first time. |
| `03-preview-three-decks.png` | ADR-0008 §8's multi-deck file with no header: the Persian deck updated quietly, two new decks, and *1 already yours* under a deck the user has never had (a held id on the create path, skipped — ADR-0005 §2). |
| `04-refusal-archive.png` | A collection archive: *"This is a collection archive, not a deck file."* |
| `05-refusal-unreadable.png` | Thirty bytes under a `.cdeck` name: *"This file could not be read as a deck."* |

These are the before for [Draw the File Surface](https://github.com/amin-bf/cairn/issues/167). It
should judge its screen on `02`, not on a new-deck plan.

## Two things the pictures say that nobody had seen

**A Persian name decides the direction of the whole line it is joined into.** The list row in `01` is
built as `"{name} — {description}"`. With `فارسی and 2 more.cdeck` as the name, the first strong
character is right-to-left, so the whole row is laid out as one right-to-left paragraph: it draws as
*"and 2 more.cdeck — deck فارسی"*, with the row's own description in the middle of the file's name.
The deck line in `03` has the same fault the other way round: *"updating a deck you already have —
فارسی"*, where the app's text has been placed ahead of the deck it describes. Both are correct
bidirectional ordering of the wrong string. The file's name and the app's words are separate
statements, and joining them into one paragraph lets a stranger's text decide how the app's own
text is ordered. That is a layout question for #167 rather than a defect in `bidi`, and until the
three-deck file's Persian filename existed there was nothing to show it.

**The numbers are small on purpose.** ADR-0022's worked example reads *"38 new notes, 1,202 already
yours"*. This set's counts are single digits, because every note is a real word a later import leaves
in the list. A four-digit count, and whatever thousands separator it needs, is still unphotographed.

## Reproducing

```sh
cargo build -p cairn-desktop
scripts/capture-desktop.sh scripts/storyboards/file-surface.txt 1280 800
```

1280×800 only; see the storyboard's header for why.
