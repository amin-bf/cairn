# The file surface, before it moves

The *before* for [Draw the File Surface](https://github.com/amin-bf/cairn/issues/167), taken while
charting [The File Surface](https://github.com/amin-bf/cairn/issues/151) on 5 September 2026. Six
captures at **1280×800, dark**, of the two temporary blocks at the bottom of Settings —
`file_list_specimen` and `inbound_specimen` — drawing real files through the real seam.

**This is a partial before, deliberately, and the missing half is the finding.** See *What is not
here* below before reading it as the whole surface.

| | |
|---|---|
| `01-settings-file-blocks-untouched.png` | Both blocks as a user reaches them: the list not yet pressed, nothing arrived. |
| `02-file-list-populated.png` | Four rows, each described **from its own bytes** — a deck, a multi-deck file, a collection archive, and one we wrote and can no longer parse, listed and marked *unreadable* rather than hidden (ADR-0022 §11). |
| `03-preview-one-deck.png` | A deck file selected: the file's own header (author, description, licence) and the effect lines, in one weight, with no separation and no gate. |
| `04-preview-three-decks.png` | ADR-0008 §8's multi-deck file — three decks, each grouped per ADR-0022 §3. |
| `05-refusal-wrong-profile.png` | A collection archive offered to deck import: *"This is a collection archive, not a deck file."* |
| `06-refusal-unreadable.png` | The unparseable row previewed: *"not a recognised container"*, then *"This file could not be read as a deck."* |

## The surface is reachable by the harness, and the ticket that needed it thought it was not

#151 inherited *"neither state this draws is reachable by the capture harness today"*, and that the
import preview *"needs a file **dropped** on the window, which synthetic input cannot produce."*

**That is false**, and [#108](https://github.com/amin-bf/cairn/issues/108)'s own `Arrival::Listed` is
why. Selecting a row re-reads the bytes through `platform::get` and hands them to the **identical**
`inbound::read` a drop reaches — ADR-0022 §5's *one mechanism, not two*, which the specimen was built
to honour and which turns out to also be the way in for a harness that cannot drag. No drop is needed.

The whole cost was **one environment variable**. `cairn_export::platform::desktop::files_dir` honours
`XDG_DOCUMENTS_DIR` when it is set and absolute, so:

```sh
cargo build -p cairn-desktop --bin cairn
rustc --edition 2024 storyboards/seed-files.rs …   # see below; writes four files into $DOCS
XDG_DOCUMENTS_DIR="$DOCS" scripts/capture-desktop.sh storyboards/list.txt 1280 800
```

No fixture, no app change, no drag, no handset.

**This also exposes a defect in the harness that is not this ticket's.** `XDG_DOCUMENTS_DIR` is the
one XDG base `capture-desktop.sh` does **not** redirect into the scratch profile, so a capture run
that writes a file writes it into the operator's real `~/Documents` — which the script's own header
promises does not happen (*"nothing touches their collection"*). The hand-off specimen's *Write a deck
file* button does exactly that today. Fixing it belongs to
[A File Bench](https://github.com/amin-bf/cairn/issues/166).

## What is not here, and why it matters more than what is

**Every plan above says *new deck*.** The lines ADR-0022 exists for are all absent:

- *French A1 — updating a deck you already have*
- *38 new notes, 1,202 already yours*
- *12 notes moving in from German*
- *3 of your notes will be deleted*
- *Renaming your "My French" to "French A1"*
- *German will be left empty*

Every one requires the collection to already hold the file's **deck ids** (ADR-0008 §11 — authority
follows deck id). It cannot: `create_deck` appears **nowhere** in `crates/app/src/fixtures.rs`, and
`CairnApp::open_store`'s six seeded notes are **unfiled**. So no collection state this repository can
reach holds a deck at all, and the entire update path — the destructive half, the half the gate exists
for — has never been drawn by anything.

**Nor is there a gate.** `[ Import ] [ Cancel ]` is ADR-0022 §1's opening decision and there is
nothing behind *Import*: no apply path exists in the app or the store. That deferral is not new — it
is written into [#89](https://github.com/amin-bf/cairn/issues/89)'s closing comment, and nothing
carried it afterwards. It is now [#165](https://github.com/amin-bf/cairn/issues/165).

So these six images are the *before* of **the reachable half**. The other half needs
[#161](https://github.com/amin-bf/cairn/issues/161)'s decked fixture and
[#166](https://github.com/amin-bf/cairn/issues/166)'s file bench, and the drawing ticket should
**re-take this set once both land** rather than judging the screen on a new-deck plan — which is the
easy state and the one that misleads.

## Reproducing

`storyboards/` holds what produced these, as files rather than as prose, so the next session does not
re-derive fifty-six wheel clicks.

- **`seed-files.rs`** — a throwaway that writes the four files into a directory given as `argv[1]`:
  a one-deck `.cdeck` with metadata and tombstones, a three-deck one, a `.ccoll` archive, and thirty
  bytes of garbage under a `.cdeck` name. It depends on `cairn-export` and `cairn-core` by path. It is
  kept here as the **recipe**, not as a build target — #166 is where this becomes a fixture arm.
- **`list.txt`** — the list, pressed and populated (captures 01 and 02).
- **`deck.txt`**, **`multi.txt`**, **`collection.txt`**, **`unreadable.txt`** — one preview each
  (03–06).

**They are measured at 1280×800 and are not width-independent.** Both blocks sit below several
paragraphs of prose that wrap at one width and not another, so the scroll count and every y are wrong
at 560×860 — the exact shape of the miss ADR-0036's ticket hit, and the reason these are not in
`scripts/storyboards/`. They also take `XDG_DOCUMENTS_DIR` from the caller, which a storyboard cannot
set. **Promote them once #166 makes both facts untrue.**

Each is self-evidencing in the way #153 asked for: a missed click leaves the file list reading
*"Not listed yet — press the button."* or *"No files this application has written yet."* under the
name of a populated one.
