# The note list, after

Twenty-nine captures of the note list as [ADR-0039](../../adr/0039-the-list-row.md) leaves it,
resolving [#162](https://github.com/amin-bf/cairn/issues/162). The *before* is
[`../notes-before-2026-09-05/`](../notes-before-2026-09-05/), and the throwaway prototype the
decisions were judged against is the tag `prototypes/issue-162`:

```sh
git show prototypes/issue-162:docs/design/prototype-162/README.md
```

## What produced these

```sh
cargo build -p cairn-desktop && cargo build --bin cairn-fixture
for wh in "1280 800" "560 860"; do
  scripts/capture-desktop.sh scripts/storyboards/notes.txt        $wh
  scripts/capture-desktop.sh scripts/storyboards/notes-decks.txt  $wh
  scripts/capture-desktop.sh scripts/storyboards/notes-light.txt  $wh
done
scripts/capture-desktop.sh scripts/storyboards/notes-persian.txt 1280 800
```

**Every capture had its page colour read off the file** — dark `#1a1e21`, light `#dee2e3` — rather
than counted, which is #143's finding and #122's before it. And every one was **looked at**, which
this set needed more than most: see *The trap* below.

`notes-decks.txt` is new. `notes.txt` runs against `backlog`, which holds no decks — so there every
note is unfiled by definition, no row is captioned (ADR-0039 §3), and the filter offers only the two
values that exist when no deck does. The deck surface lives in the other file.

## What to look at

### `01-list` — the row is a band, and the actions are a column

Compare against `../notes-before-2026-09-05/1280x800/01-list.png`. Three things changed and only one
of them was a design question.

**The row is 36px on `control_fill`**, where it was 19px on `widgets.inactive`. That is ADR-0034
reaching this screen for the first time — seventy-five controls that had never had it — and it is a
repair rather than a choice. It costs density: twenty-five rows go from 667px to 1092px, or 1167px
carrying a deck.

**The two actions land on the same x on every row**, at 880→916 and 924→960 at 1280, with the second
ending on the page frame. Before, they landed somewhere new on all twenty-five.

**They are pictures with no words**, which is #149's icon-rule exception taking its first real test.
The word survives as hover text.

### `decks-*/01-list` — a row says which deck it is in, and `02-deck-open` — *Unfiled* exists

Under *All decks* each row captions itself, so `le colporteur` reads **Unfiled** among twenty-two
filed ones. Under a named deck (`03-deck-filtered`) the caption is gone: it would repeat the name the
filter already states, once per row.

The filter now holds **five entries and a value that had never been expressible**. `notes::Filter`'s
deck was an `Option` whose `None` meant *narrow nothing*, so *unfiled only* could not be asked for —
ADR-0005 §8 says such a note *"appears in an unfiled view"* and no such view existed.

### `04-deck-delete-warning` — the control names what it destroys

*Delete deck* asks first, and the question carries the count: *"Delete Expressions idiomatiques et
proverbes? Its 3 notes are deleted with it, and cannot be undeleted."* Deleting a deck derives every
note in it deleted (ADR-0005 §7) and there is no undelete (ADR-0021 §2); before this it happened on
one tap, from a control wearing the same weight as the *New deck* directly above it.

**The weight is unchanged**, and that was decided rather than defaulted — see ADR-0039 §6.

### `03-placement` — the state inverted

The notes are rows and the targets are quiet: *Place here* draws at **131 of 255**, dragged to a stop
on a live knob. The hit area is 36px at every ink, which is what let it go this far down.

The note being moved is **held** — drawn as the row it is, on `window_fill`, the one material that
means *temporarily on top* (ADR-0037 §2) and the first call site where it describes its own contents
rather than a popup's.

Before, this was twenty-six identical full-width slabs with the notes as plain body text between
them.

### `persian-1280x800/07-list-persian` — the row has an end now

A shrink-to-fit button had no spare width, so which end its text sat at could not be asked. A
full-width band can be, and the answer is the note's own direction (ADR-0039 §4) — the same rule
ADR-0033 §5 already applies to the box badge.

**The action column does not mirror with it**, and that is a ruling rather than an oversight: the
column exists because the actions land on one x, so a cluster that mirrored per row would destroy it
on any collection holding both scripts. §5 governs content, not furniture.

### `Create note`, in every shot — pinned below the scroll

Its bottom edge lands 165px above the page, which is ADR-0035 §1's reach line, and it does not
scroll. It was at the very top of the page — the furthest point from a thumb — because a list has no
leftover height for the page rule to spend *inside* the scroll. Measured at 1280×800: the button runs
y=599→635 of 800.

The band is opaque and carries a unit of page above the control. Both are repairs to the first run:
a transparent frame let rows draw through the button, and without the unit the list's last clipped
row met it as one block of the same colour.

## The trap in taking these

**Every coordinate below the heading moved, and the page-colour check cannot see it.**

*Create note* leaving the top of the page raised the whole chrome by that control plus its gap, so
the deck filter sits at y=109 where it sat at 145. A run against the old numbers does not fail: it
misses the dropdown, lands on a row, and photographs the **editor** under the list's name — and the
editor is dark, so the check that caught the theme miss in #143 and #150 passes it.

That is #122's silent miss from a further side, and the second one caused by a *ticket's own
decision* rather than wrong when written (#155 was the first). The instrument that catches it is the
one the harness's own documentation names and nothing automates: **look at the images**.
