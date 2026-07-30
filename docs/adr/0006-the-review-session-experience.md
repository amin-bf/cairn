# ADR-0006: The review session experience

- **Status**: Accepted
- **Date**: 2026-07-29
- **Resolves**: [Prototype: the review session experience](https://github.com/amin-bf/leitner/issues/11)
- **Map**: [Map: local-first Leitner app spec](https://github.com/amin-bf/leitner/issues/1)
- **Evidence**: two rounds of egui/eframe variants at tag
  [`prototypes/issue-11`](https://github.com/amin-bf/leitner/tree/prototypes/issue-11)
  (`prototypes/review-session-11/`), judged live by the repo owner on a Pixel 8 Pro and on
  desktop.
- **Related**: [ADR-0001](0001-scheduling-algorithm-and-grade-scale.md) (grades, box, FSRS-6),
  [ADR-0003](0003-client-stack.md) (egui/eframe, the bidi helper), [ADR-0004](0004-the-review-event-log.md)
  (the event log this session's position is derived from)

## Context

ADR-0003 fixed the client stack but left the review session itself open — the ticket asked seven
concrete questions: session shape (bounded or open-ended, pausable, kill-safe?), the grading
interaction, whether touch and desktop diverge, how the answer is revealed, where constraint 4's
box shows up, empty/new/backlog states, and offline affordances. Prose would not settle this — the
ticket called for building it and reacting, so it went through `/prototype` in two rounds.

**Round 1** built three structurally incompatible sessions — bounded batch, open queue, and timed
with backlog framing — each taking a different position on session shape, reveal mechanic, box
display and interval preview, switchable live against four data scenarios (Normal / Empty / New
deck / Backlog). Judged on a Pixel 8 Pro (real touch, and a real `am force-stop` + relaunch to
prove session position survives a kill) and on desktop.

The repo owner did not pick one variant — they converged on a graft: the count-picker from the
bounded-batch variant, the timer and backlog framing from the timed variant (softened: time-up
should be a choice, not a wall), and the tap-to-reveal card with box badge and interval preview
from the open-queue variant.

**Round 2** rebuilt three *presentation-only* variants around that now-fixed interaction model —
a minimal card-first layout, a persistent dashboard header, and a checkpoint-forward layout that
put design care into the time's-up moment specifically. The repo owner picked the dashboard-header
layout, revised live from a 4-column grade grid to vertically-stacked full-width buttons.

## Decision

### 1. Session shape: a chosen count, with a timer that asks rather than tells

A session starts with an explicit choice — how many cards, from 10 / 20 / 40, capped by what's
actually due. A quiet 10-minute timer starts at that same moment. Reaching the chosen count ends
the session normally. Reaching the timer does **not** force a stop: it surfaces a checkpoint
("finish here, or keep going?") without hiding the card underneath — the reviewer can still grade
what they're looking at while deciding. "Finish here" ends early; "keep going" dismisses the
checkpoint and the session continues, untimed, from there.

This was a live design change from round 1's timed variant, which ended outright at zero. The
repo owner's framing — "time's up leaves for the user to decide to finish or continue" — is the
operative rule: the timer is a courtesy check-in, not an enforcement mechanism.

### 2. Session position is never stored — only derived, and this was proven, not assumed

There is no session-progress entity anywhere. The due queue for a session is always *this
scenario's due cards minus cards with a log entry*, recomputed on every read. The chosen count and
the timer's start instant are ordinary in-memory state, not persisted.

Proven for real, not just argued: force-stopping and relaunching the app on the Pixel 8 Pro — and
separately, a "simulate kill & restart" control on desktop that reloads only the on-disk log —
both land back on the count-picker screen with every already-graded card correctly excluded from
the due count. Nothing about "where was I in the batch" survives a kill, and nothing needs to: the
log already answers the only question that matters, which cards are done.

This is a direct consequence of ADR-0004's design (replay from an append-only log) rather than a
new mechanism — the session UI just had to avoid inventing a second source of truth alongside it.

### 3. Reveal is tap-the-card

The card itself is the tap target for revealing the answer — not a separate "Show answer" button.
Verified by real touch on the Pixel 8 Pro and by mouse click on desktop; no divergence between the
two was needed or found.

### 4. Grading: four buttons, full-width, stacked vertically, shown only after reveal

No swipes, no dedicated keyboard-only path (the prototype's variant switcher used arrow keys, but
that is dev harness, not the reviewed design). Buttons appear only once the card is revealed, so
self-grading can't happen before the answer is seen.

Each grade button also shows a projected next interval underneath its label (illustrative
`"~9d"`-style text) — confirmed live as wanted information, not Anki-style noise, once seen next
to the actual button rather than described in prose.

### 5. Touch and desktop do not diverge

The same layout, the same button sizes, worked identically for real touch on the Pixel 8 Pro and
mouse clicks on desktop. No platform-specific interaction path — larger touch targets, different
gestures — was needed. (Keyboard-only grading was not requested or explored; if it's wanted later,
it's additive, not a redesign.)

### 6. Constraint 4's box display: a quiet badge, only after reveal

[ADR-0001 §3](0001-scheduling-algorithm-and-grade-scale.md) fixed *what* a box means (durability,
not urgency) and forbade three things: boxes sorted, boxes counted, or boxes implying review
frequency. This ADR fixes *where* the box appears on the review screen: a small, non-interactive,
monospace footnote — `"box 3"`, or `"new"` for a card with no review history — visible only after
the card is revealed, never before, never as part of the queue itself. "Due today" / remaining
count is a separate figure in the header and is never itself expressed as a box.

Round 1's open-queue variant put this exact badge in front of the repo owner specifically to test
whether an inert, post-reveal badge starts to read as a claim about the queue. It didn't — the
badge survived into the final design unchanged.

### 7. The header is a persistent dashboard, and backlog is framed, not just reported

Progress (`done / total` against the chosen count, with a progress bar) and the timer sit together
above the card at all times, so the session state never has to be inferred. When the deck's
overall due count is large, a backlog-aware note appears — `"N due — pick a comfortable size, the
rest will keep"` at the picker, `"N still waiting, that's fine"` at session end — rather than a
bare number that reads as falling behind.

> **Amended by [ADR-0010 §8](0010-leeches.md): the due count excludes suspended cards.** A suspended
> card is due but never offered, so leaving it in the count would stop the number ever reaching zero
> and make §8's `"nothing due right now"` state unreachable — a header permanently reporting work
> the user is structurally unable to do. This does not touch §6's box badge: the box goes on meaning
> durability and makes no claim about the queue, which is exactly the case constraint 4 was written
> to survive.

### 8. Empty, new-deck and backlog states are explicit, worded states

"Nothing due right now" and "fresh deck, first look" (zero review history) are both rendered
states with their own copy, not blank screens or a session that silently has zero cards in it.

### 9. Offline affordances: confirmed structurally

The prototype crate contains no networking code at all — there is nothing to check at runtime
because there is nothing that could reach a network in the first place.

### 10. What this ADR does *not* settle

**Visual design is out of scope here.** The dark palette, spacing and typography throughout both
prototype rounds are scaffolding carried over from the `prototypes/egui-slice` client-stack
prototype for convenience — never a considered decision, confirmed explicitly live when asked.
A real look-and-feel pass is separate, later work and starts from a blank slate, not from this
prototype's colors.

**The exact numbers are illustrative, not load-bearing.** 10/20/40 as count choices and 10 minutes
as the timer default were picked to have *something* concrete to react to, not measured against
real usage. Nothing in this design depends on those specific values — they're free to change
before or after implementation without revisiting the shape decided here.

## Consequences

- No new persistence concept is introduced. The event log (ADR-0004) remains the single source of
  truth for session state; the client computes "what's left in this session" the same way it
  computes anything else — by replay, not by storing a session object.
- The box becomes a UI-visible fact on the review screen itself, not only in a deck overview.
  Its cost is one `u8` per card, already available from ADR-0001's `box = f(stability)`
  computation — no new query shape is required.
- A session's real wall-clock duration is now unbounded from the app's point of view: "keep going"
  means nothing in the client enforces the 10-minute figure. Nothing downstream currently assumes
  sessions are time-boxed, so this has no other consequence yet, but it's worth remembering if
  anything ever wants to reason about "how long was this session."

## Open items handed onward

- **Visual design system** (§10): colors, typography, spacing — deliberately not started here.
- **Keyboard-only grading**: not requested; additive if wanted.
- **Count/timer defaults**: 10/20/40 and 10 minutes are illustrative; revisit against real usage
  once there's usage to look at.
