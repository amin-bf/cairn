# Prototype #163 — two knobs for the note editor

Throwaway code built to answer two of [The Note
Editor](https://github.com/amin-bf/cairn/issues/163)'s questions by looking. **Never merges** —
preserved as the tag `prototypes/issue-163`, contained in no branch, per `AGENTS.md` rule 3.

```sh
git show prototypes/issue-163:docs/design/prototype-163/README.md
git checkout prototypes/issue-163 -- crates/app/src/theme.rs
```

Based on `8ce75a61`, which is the last commit where `frame::TWO_COLUMN_MIN_WIDTH` still existed and
the first where `Fixture::Dormant` does — so both knobs have the state they need under them.

## 1. The width knob: is a narrow two-column editor unreadable?

**The knob is the window edge**, so there is no slider — the variable being judged is the thing the
person is already holding. What was missing was the readout: *window · frame · each pane*, drawn
under *Done*. `TWO_COLUMN_MIN_WIDTH` is set to **320** so the arrangement never switches while the
window is dragged; an earlier pass at **700** produced the still sweep at 720/760/800/840/880/920.

### What it found: nothing, and that is the answer

| window | each pane | verdict |
|---|---|---|
| 880 | 398 | fine |
| 386 | 151 | fine |
| 320 | 118 | fine — the card face wraps to four lines and still reads |

118px per pane is **a third of the narrowest case the ticket had argued about**. Nothing clips,
nothing overflows, nothing becomes unreadable. The repo owner's verdict, in full: *"I don't get what
is the failure."*

So **narrowness produces a gradient, not a failure, and a gradient has no threshold in it.** The
ticket's recorded doubt — *"an 880px window with two 420px columns may simply be too narrow to read,
in which case the threshold is right for a reason it does not currently state"* — has nothing behind
it. `TWO_COLUMN_MIN_WIDTH` was **deleted** rather than moved, and the panes now fold on
`SoftKeyboard::exists`, which is the axis ADR-0025 §4 always said the toggle was about and ADR-0025
§5 had already written down: *"the failure is vertical, and no width rule addresses it."*

**The question was wrong, not the instrument.** *Where does it break* presumed a break. The knob was
right and it was pointed at the wrong thing — worth recording, because a knob that finds nothing
still costs a sitting.

Also corrected here: the ticket's *"two 420px columns"* at 880 is **398**, because the estimate had
not subtracted the 28px gutter. It makes no difference to the answer, which is the point — the answer
was never near the margin of that error.

## 2. The fill knob: may a card and a text field share a fill?

ADR-0033 §2 accepted the sharing partly because *"the two never appear on the same screen"*, which
has been untrue since the card landed — the editor draws the fields in one column and the card faces
in the other. #150 measured **1.000:1**: not similar, identical.

The knob moves the **field** from the card's fill toward the page, 0.0 → 1.0, and **holds the card
still**. That asymmetry is deliberate: the card's value is ADR-0033's decided well and #125 banked a
result on it (1.121:1 still reading as cut into the page on an OLED panel at low brightness), while
the field's fill has no argument behind it at all — it is the rung egui happens to put text edits on.
Moving toward the page also keeps the card the deepest surface, which is what ADR-0033 §3's ordering
wants.

At knob 0 the readout shows exactly what shipped, so the floor of the prototype is the real app
rather than a copy of it. `CAIRN_FIELD_KNOB` seeds it, so a position a thumb chose can be
photographed — a knob whose value cannot be handed to a screenshot is only half an instrument.

### What it found: the same knob, two different answers

Both themes stopped at **0.55**.

| | field | card | field : card |
|---|---|---|---|
| dark | `#15191b` | `#0f1214` | **1.063:1** |
| light | `#d2d6d7` | `#c4c8c9` | **1.152:1** |

**Dark landed on a rung the ramp had numbered and never filled.** `STONE_0` → `STONE_2` is the
ramp's only double step — (11, 12, 13) where its neighbours move (7, 8, 9) and (4, 4, 4) — because
nothing had ever needed a value between the well and the page. A true midpoint is `(20, 24, 26)`; the
thumb stopped at `(21, 25, 27)`, **one unit per channel**. That overturns a recorded cost: #143 wrote
that giving a field its own fill *"spends a rung of a ramp that had none to spare in dark"*, and
nothing is spent.

**Light minted its own rather than reusing the edge rung.** `#d2d6d7` is four of 255 per channel from
`STONE_L_EDGE`, which is #155's ink-knob situation almost exactly — and was refused on **meaning**
rather than distance. #155's two candidates both meant *quiet ink*; `STONE_L_EDGE` means separators,
pressed widgets and a control's edge, so reusing it paints a **resting** field the value of a
**pressed** control — ADR-0033 §2's own category error arriving one rung over.

**The same knob position is not the same separation** — 1.063 against 1.152 — so the decision is
recorded as a **rung** (*the field sits between the page and the card*) rather than as a knob position
or a ratio. That is #143's finding applied: stating ADR-0033 §3 as ratios kept the magnitude and threw
away the structure.

## Running it

```sh
cargo build -p cairn-desktop --bin cairn && cargo build --bin cairn-fixture

# Live, on a scratch profile — never the real collection.
export XDG_DATA_HOME=/tmp/cairn-knob/data XDG_STATE_HOME=/tmp/cairn-knob/state
export XDG_CONFIG_HOME=/tmp/cairn-knob/config XDG_CACHE_HOME=/tmp/cairn-knob/cache
mkdir -p "$XDG_DATA_HOME" "$XDG_STATE_HOME" "$XDG_CONFIG_HOME" "$XDG_CACHE_HOME"
./target/debug/cairn-fixture dormant && ./target/debug/cairn

# Or through the harness, which takes no window and no focus.
CAIRN_FIELD_KNOB=0.55 scripts/capture-desktop.sh scripts/storyboards/dormant.txt 1280 800
```

Open **Notes → row 7**, the pruned cloze note: the largest field in the app beside a card carrying a
full sentence on both faces, with the destructive-edit warning above it. Rows 8 and 9 are the other
two dormant cases. The theme switch is on Settings (ADR-0036 §3).

## What did not need a prototype

The ticket's third question — **what the Cards pane is** — was answered in conversation as *a
preview*, from the shipped app plus the `dormant` fixture's first captures. Building variants for it
would have been doing the redraw's work before its material was decided.

---

# Second round: two sets of variants

Added after the knobs, once the ticket reached its arrangement questions. **These two are preserved as
captures and as descriptions rather than as code**, and that is deliberate: unlike the knobs, each one
was a handful of lines against a base that has since moved several times, so a stale copy that no
longer compiles would be worth less than the pictures plus a paragraph that says exactly what changed.
The knobs above are the code half of this tag; these are the judgement half.

## 3. Where *Done* sits — and whether the reach line has anything to place

Selected with `CAIRN_DONE_AT=a|b|c`, captured at 1280×800 and 560×860.

| | variant | captures |
|---|---|---|
| **A** | *Done* above the heading — what shipped | `done-a-1280.png`, `done-a-560.png` |
| **B** | below the heading: *Edit note* → the way out → the note | `done-b-1280.png`, `done-b-560.png` |
| **C** | at the foot of the page on ADR-0035 §1's reach line | `done-c-1280.png`, `done-c-560.png` |

**C won, and building the three is what collapsed two questions into one.** ADR-0035 §1 places *the
last control on the page*, and while *Done* sat at the top the editor's last control was the **Back
field** — a form whose inputs float at the foot of the page is not an arrangement anyone wants. So §1
had no target on this screen at all until the exit moved, and A and B are the same answer to the reach
line wearing different hats. Three tickets had inherited *apply §1 here* without being able to.

It also forced §1 to say which of two things it means. Every call site before this one places
something the reader is meant to press **next** — a grade cluster, the leech entrance, *Back to
review*. *Done* is what you press when you are **finished**. §1 is therefore read as *the last control
on the page* rather than *the way forward*.

Measured on landing: bottom edge **166px** above the page bottom — §1's 165 plus the stroke — at both
widths.

## 4. The card pane and its warning

Selected with `CAIRN_PANE=1|2|3`, on the `dormant` fixture at 1280×800. Judged in a browser review
surface against 2× crops, one region at a time with the variants swapped **in place**.

| | variant | captures |
|---|---|---|
| **1** | what shipped | `pane-v1-pruned.png`, `pane-v1-nolive.png`, `warn-v1.png`, `pane-v1.png` |
| **2** | the warning's weight uninverted and given a left rule; pane untouched | `warn-v2.png`, `pane-v2-*.png` |
| **3** | 2, plus the dormant entry as a peer of the card and §6's statement demoted | `warn-v3.png`, `pane-v3-*.png` |

The `warn-*` files are the left column at 2× — the warning block alone. The `pane-*` files are the
right column at 2×, where a real card sits directly above the dormant entry.

**3 won.** What it turns on could not be measured and could not be decided by the person who drew it:
whether an outline at the card's own footprint reads as **a card that is not there** or merely as a
box. Put in front of the repo owner beside a real card, it came back as an absence.

It also depends on this ticket's answer to what the pane **is**. In a *preview* — a specimen case — an
entry that is not a specimen has to look like one missing. Had the answer been *listing*, the same
shape would only have been a row and the redraw would have gone the other way. Question 3 decided
question 4's form, which neither was written expecting.

**A fourth was considered and not built**: dropping the history from the pane entry to kill the
duplicated sentence outright. It would overturn ADR-0018 §2, which puts the history there, so it is
recorded as available rather than tried.

## What the second round cost, and it was not the variants

Two process failures worth more than the pictures.

**The first review artifact was three variants side by side at ~370px wide**, and the response was
*"the pics are too small I do not see the details"*. A standing note already said screenshots to judge
go one per screen at full width with variants swapped in place. Having the rule and ignoring it is a
different failure from not having it, and it cost a round.

**Two sessions built position-based detectors and both lied.** Checking *did the editor open* by
sampling the card-pane area for the card fill reports "no" on the note that has no live card — which
is precisely the note under test. A parallel session sampling for *Done* at its old coordinates read
"no" on a frame where it had opened, because this ticket had just moved *Done*. Looking at the image
settled it both times, in both sessions.
