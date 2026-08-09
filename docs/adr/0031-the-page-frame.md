# ADR-0031: The page frame — the margin, the measure, and what the leftover width does

- **Status**: Accepted
- **Date**: 2026-08-09
- **Resolves**: [Design Pass: The Frame — Page Margin, Measure, and What Width Does](https://github.com/amin-bf/cairn/issues/131)
- **Related**: [ADR-0030 §1](0030-the-first-finish-pass-decisions.md) (colour named at exactly one
  site — the rule this ADR repeats for layout, and the reason it is a module rather than a
  convention), [ADR-0012 §1](0012-the-note-authoring-experience.md) (the editor's two panes, which
  this ADR finally makes side by side), [ADR-0025 §5](0025-the-authoring-screen-under-a-soft-keyboard.md)
  (*"where both fit they show together"* — superseded in §4 by *where both fit they sit side by
  side*), [ADR-0021 §1](0021-note-ordering-saving-and-the-note-list.md) (the pinned nav row, whose
  alignment §3 decides), [ADR-0003 §4](0003-client-stack.md) (one binary per platform, so a frame
  decided here is the frame everywhere)
- **Evidence**: thirty-two captures of four candidate frames across four screens at two widths,
  preserved as the tag **`prototypes/issue-131`** with a written-up
  [README](https://github.com/amin-bf/cairn/blob/prototypes/issue-131/docs/design/prototype-131/README.md);
  then twenty-one captures of the implemented frame at three widths in
  [`docs/design/framed-2026-08-09/`](../design/framed-2026-08-09/README.md). Everything but the frame
  was held at the Review slice's chosen direction while the four were judged.

## Context

The app had **no frame at all**. Content ran edge to edge at every width, so 1280px of window bought
1280px of button, a card with one word centred in it, and a Settings paragraph drawn as a single
150-character line. At 560 the same absence put text against both window edges and let full-width
buttons bleed off the frame.

Nobody had listed this as a foundation. It surfaced sideways: [#124](https://github.com/amin-bf/cairn/issues/124)
was judging Review arrangements and found that a margin and a capped measure accounted for most of
the visible distance between the baseline and *every* variant it tried — a finding about the thing it
was holding constant rather than the thing it was varying. #124 settled the frame for **Review**: one
arrangement, centred, no second breakpoint, the window simply gets emptier. It left open whether that
holds for the note list, the editor and Settings, which are rows and forms rather than a single card.

Deciding it late was not an option. Every other question in the Review slice is judged *inside* the
frame, so the type scale, the card and the controls would each have been decided against a container
that then moved.

## Decision

### 1. The two numbers live in one module, and screens ask for a frame rather than a number

`cairn_app::frame` names the page margin and the measure, and exposes `column` to apply them. This is
[ADR-0030 §1](0030-the-first-finish-pass-decisions.md)'s rule for colour, repeated one layer down and
for the same reason: a literal `28.0` of padding or a hand-rolled `min(available, 640.0)` on some
screen **renders fine to the author and drifts the layout one screen at a time, with nothing
failing**. [#123](https://github.com/amin-bf/cairn/issues/123) already found the app paying exactly
that cost for spacing, at around sixty literal call sites.

A `Color32::from_rgb` outside `theme` is a defect; a second place that computes a column width is the
same defect in different units.

### 2. The margin is 28 and the measure is 640, at every width, on every destination

**One arrangement, centred, everywhere.** #124's answer for Review is the answer for the note list
and Settings too: at 1280 roughly half the window is empty *by design*. The app has **one** frame,
not a frame per destination.

The alternatives were built and photographed, and each lost on a specific screen rather than on
taste. Giving rows and forms a wider column than prose leaves every Settings section ragged between
its full-width button and its capped paragraph. Letting them spend the whole window puts 1100px
between a note's title and its *Delete*, which breaks the association a row depends on.

**640 is reused, not invented.** The prototype proposed 620; the difference is twenty pixels, and the
app gains nothing from carrying two neighbouring numbers that mean roughly the same thing. 640 was
already in the tree as the editor's threshold — see §4 for what happened to that, and note that the
reuse survives the two ceasing to mean the same thing, because the argument for it is *one number
instead of two*, not *these two things are the same thing*.

**The margin is the half of this that the handset feels.** At 1280 it is nearly invisible next to the
measure; at 560 it is the whole difference, costing 10% of the width and buying the screen back. That
is the reverse of how a margin and a measure are usually argued about, and it is worth stating because
the desktop captures make the margin look like the minor decision.

**A frame is a decision, so changing one of these numbers fails a test** (`frame::tests`) rather than
passing review as a diff nobody notices.

### 3. The nav row aligns to the column beneath it, and both sides read one function

The pinned row of [ADR-0021 §1](0021-note-ordering-saving-and-the-note-list.md) draws its buttons
inside the frame, so the row and the content share one left edge. This was the single most visible
difference between the frames that were judged: a nav left at the window edge while the card sits
centred reads as two unrelated pages.

The row therefore **moves** when the editor takes its wider frame (§4). That is a deliberate trade and
the smaller of two — the alternative buys a still row by breaking the alignment on the one screen
where the eye has two columns to line up against rather than one.

`frame::cap_for` is asked by the nav row *and* by the screen, so the two cannot disagree. Had each
named its own number, the nav would drift out of step the first time one of them changed.

### 4. The editor's two panes sit side by side, and the threshold measures the window

[ADR-0012 §1](0012-the-note-authoring-experience.md) has always described the wide editor as two
panes that *"cannot sit side by side"* below a threshold. The implementation stacked them vertically
with a rule between them — so its `640` was a **width** test gating a decision about **vertical**
room, and the comment beside it half-admitted this. Nothing failed, because nothing measured the gap
between what the code said and what it did.

A page frame made that latent oddity load-bearing, in the way this map keeps finding: `ui.available_width()`
was the *window's* width only because the app had no frame. The moment one exists it becomes *the
column's* width, and a 640 column is not `>= 640` once the margin is off it — so the desktop would
have shown the phone's `Write | Cards` toggle at every window size, silently.

So both halves are fixed at once:

- **Two panes now mean two columns.** Form left — which keeps
  [ADR-0025 §4](0025-the-authoring-screen-under-a-soft-keyboard.md)'s destructive-edit warning at the
  top of the first thing read — cards right, split by the page margin. The editor's header travels
  with the form rather than stretching across both, so the left column reads as *the note* and the
  right as *its cards*.
- **The threshold is `frame::TWO_COLUMN_MIN_WIDTH = 900`, measured against the window**
  (`viewport_rect`), never against a column. 900 is #124's number for the same shape of question,
  reused rather than invented; it leaves each pane around 430px inside a 1280 window.
- **The editor is the only screen with a second frame**, capped at 1120 — two full measures plus a
  gutter would want 1308px and does not fit the 1280 this pass judges at. `frame::wide_column` exists
  for this one caller, and a second caller reaching for it is the frame eroding into a per-screen
  preference.

This **supersedes** [ADR-0025 §5](0025-the-authoring-screen-under-a-soft-keyboard.md)'s *"where both
fit they show together"*: where both fit, they now sit **side by side**. Nothing else in ADR-0025
moves — the soft-keyboard band, the three guards and the seam are untouched, and below the threshold
the `Write | Cards` toggle behaves exactly as it did.

**What is not decided here**: whether a *width* test is the right test at all. The toggle exists
because a soft keyboard eats vertical room, which is a phone problem, and the keyboard seam of
[ADR-0026 §5](0026-the-per-tap-keyboard-re-pop.md) already distinguishes *no soft keyboard on this
platform* from *keyboard down*. Re-basing the test on that is the Notes slice's to take.

### 5. Full width is a target on a phone and a distance on a desktop

`compact_button` is `full_width_button` with the stretching taken off and **the same 36px height**.
The map holds hit targets and density to touch, never to the pointer, so this is not a desktop
control — it is the same control not spanning a width the eye has to cross. The editor's *Done* is
the first customer: at 1120 it drew a button wider than the two columns of content beneath it.

## Consequences

**Storyboards changed with the frame, and the harness grew a token for it.** A literal x was correct
only while content ran to the window edge; the column is centred, so the same nav button is at 320+n
at 1280 and 28+n at 560. `scripts/capture-desktop-session.sh` now expands **`%LX%`** and `%LX+n%` —
the column's left edge — and `docs/environment/desktop-capture.md` records it beside `%CX%`. The
harness duplicates the two numbers rather than importing them from the app, deliberately: a capture
harness that cannot start without the app being correct cannot photograph a broken app, which is most
of what it is for.

**The baseline storyboard now visits the editor last.** Because the nav follows the editor's wider
frame, any `%LX+n%` nav click after that point would land on empty page, and leaving by *Done* is no
better — that button is compact at 1280 and full-width at 560, so no single coordinate reaches it at
both widths.

**Three judging widths now, not two.** 1280×800 and 560×860 remain the pair that makes *one
responsive design* checkable; 880×800 was added because §4 introduced the app's second arrangement
change and a threshold with no capture either side of it is a claim rather than a fact.

**The frames that lost are reachable and were not merged.** `prototypes/issue-131`, contained in no
branch, per the convention in `AGENTS.md`.

**What this ADR deliberately does not touch.** The type scale, the card, and the controls are
[#132](https://github.com/amin-bf/cairn/issues/132), [#133](https://github.com/amin-bf/cairn/issues/133)
and [#134](https://github.com/amin-bf/cairn/issues/134), and every one of them is now judged inside a
settled container rather than against one that is about to move. The note list's rows are still
left-packed and still spend none of their width; that is the Notes slice's, and the frame does not
prejudge it.
