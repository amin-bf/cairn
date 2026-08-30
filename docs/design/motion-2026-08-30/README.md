# Motion and elevation, as landed — 30 August 2026

Twenty captures of the change [ADR-0037](../../adr/0037-motion-and-elevation.md) records, from
[#154](https://github.com/amin-bf/cairn/issues/154): **both themes at both judging widths**, on the
`cloze` fixture rather than the shipping seed.

```sh
scripts/capture-desktop.sh scripts/storyboards/motion.txt       1280 800
scripts/capture-desktop.sh scripts/storyboards/motion-light.txt 1280 800
scripts/capture-desktop.sh scripts/storyboards/motion.txt        560 860
scripts/capture-desktop.sh scripts/storyboards/motion-light.txt  560 860
```

## Why cloze, and why that is the whole point

A vocabulary card grows by a **word** at the reveal. A cloze card grows by a whole **face**, because
its two faces are the same sentence differing by one masked word — and only that shape reaches
ADR-0033 §4's step-down. Until ADR-0037 §5 the step-down was evaluated against content that *changed
at the reveal*, so a wrapping prompt was drawn at **display before the tap and heading after it**.

No capture in this repository could have shown that. Every one of them is of six French words, and
no card in that seed is long enough to step down at all — which is why the fifth fixture exists.

## What each pair is for

| | |
|---|---|
| `01-reveal-shut` → `02-reveal-open` | The reveal on a card that fits. Shut, the prompt is on the card's own centre line: no badge, no hairline, no empty half. Open, both faces and the badge, with the card's own rect unchanged — what moved is the boundary inside it. |
| `03-stepdown-shut` → `04-stepdown-open` | **The evidence for §5.** Same tier, same wrap, both frames. At 560 the card is stepped down in both; at 1280 nothing steps down and the pair is one line each. |
| `05-overlay` | The one thing in the application that floats (§1) — a rise, a chosen edge and a shadow. It was drawn in *exactly* the page colour, inside stock egui's unassigned grey hairline, with no shadow, in every capture this repository held before this set. |

Read `01-reveal-shut` against the same shot in the other theme, not against its own `02`: elevation
and the reveal are both things a single theme can be wrong about while looking right.

## Two things worth keeping, neither of them a picture

**The grade row cannot be reached by a literal y, and this set was captured wrong before it was
captured right.** ADR-0035 §1 anchors the last control cluster to a line above the **bottom of the
page**, so its y is a function of the window height: *Good* is at 677 in an 860-tall window and 617
in an 800-tall one. The first run used the literal 677, which reaches it at 560×860 and lands on
empty page at 1280×800 — and it still produced five perfectly valid images, **of the previous card,
under the next card's names**. That is #122's and #143's silent miss arriving from the third axis,
and `%BY-n%` is what exists to kill it. It is now `%CX% %BY-183%`, and 183 is the same at both
widths because the reach line is.

**The fixture's self-check corrected the person writing it.** `Fixture::Cloze` was written expecting
five cards offered and reached four: ADR-0011 §7 introduces at most **one card per note per day**, so
the two-blank note's second card waits until tomorrow. The bench refused to install rather than
quietly photographing a state nobody had named — which is the half of #153 that earns its keep.
