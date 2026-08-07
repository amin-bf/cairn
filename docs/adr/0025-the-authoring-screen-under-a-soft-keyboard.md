# ADR-0025: The authoring screen under a soft keyboard

- **Status**: Accepted
- **Date**: 2026-08-01
- **Resolves**: [Prototype: the authoring screen under a soft keyboard](https://github.com/amin-bf/cairn/issues/67)
- **Map**: [Map: local-first Leitner app spec](https://github.com/amin-bf/cairn/issues/1)
- **Related**: [ADR-0003 §6](0003-client-stack.md) (the client stack, and what winit's Android
  backend does not do), [ADR-0012 §1 §5 §9](0012-the-note-authoring-experience.md) (the two panes, the
  ambient destructive-edit warning, the handoff this ADR discharges),
  [ADR-0018 §2 §4](0018-the-card-pane-ordering.md) (a dormant entry is a line; the form-pane warning is
  primary on both platforms), [ADR-0021 §7 §8 §9](0021-note-ordering-saving-and-the-note-list.md)
  (autosave, *New note*, the deck dropdown — all of which land on this surface),
  [ADR-0016 §5](0016-backup-and-restore.md) (the per-crate platform-seam rule this ADR adds a third seam under)

## Context

[ADR-0012 §9](0012-the-note-authoring-experience.md) recorded that its prototype *"fakes the width but
not a keyboard taking half the screen, which is what decides whether a live preview survives while
typing"*, and handed the question on. [ADR-0021 §10](0021-note-ordering-saving-and-the-note-list.md)
handed it on again untouched. Everything about the authoring screen had therefore been judged on a
desktop.

Two things specified in the meantime were at stake and are re-judged here rather than assumed:
§5's ambient destructive-edit warning, which [ADR-0018 §4](0018-the-card-pane-ordering.md) made
**primary on both platforms** rather than a concession to the phone; and §3's numbered blank list,
which is read while typing into the field above it.

Answered on the Pixel 8 Pro, per [`AGENTS.md`](../../AGENTS.md) client-stack rule 9. Evidence:
tag `prototypes/issue-67`.

## Decision

### 1. The client reads the platform's IME insets, because nothing below it does

**The app is never told the soft keyboard exists.** Measured with the keyboard up: the window frame
stays at the full display, and the platform reports the keyboard's height only to something that asks.
The window carries the resize soft-input mode, which is inert — the same window is enforced
edge-to-edge, and an edge-to-edge window is expected to read the inset and lay itself out rather than
be resized under it.

[ADR-0003 §6](0003-client-stack.md) and client-stack rule 8 already record that winit's Android
backend handles only motion and key events and has no IME path, and draw the consequence for **composed
text**. This is the same gap's other half, and it had not been written down: the backend reports no
**insets** either. Raising the keyboard works — the "allow IME" call reaches the platform's
show-soft-input — and nothing comes back.

**So the client asks the platform directly**, reading the IME and system-bar insets from the activity's
window each frame.

**The consequence of not asking is worse than occlusion, which is why this is a decision and not a
polish item.** The UI layer is handed a viewport taller than the one the user can see; the content fits
inside it; so the scroll area has **no scroll range at all** and the covered region is **unreachable
rather than scrolled off**. Measured at this device's density, typing costs **923dp of usable height
down to 565dp — 39% of the screen — with no notification, no reflow, and nothing to scroll.**

Reading the insets also fixes, for free, a defect this pass found on the way: the app currently draws
its first line of text under the status bar and its last under the gesture bar, keyboard or no
keyboard.

### 2. Where the seam lives, and why it is not the store's

[ADR-0016 §5](0016-backup-and-restore.md) settled that the platform-seam rule is **per crate** rather
than per workspace, keeping `leitner-store::platform` at exactly two functions and giving
`leitner-export` its own. This is a **third** such module, in the **UI crate**, and it
belongs there: an inset is a fact about the window the UI is drawing into, and routing it through the
store would make the store answer a question about layout.

It is bound by the same discipline: **one function, returning the insets** — not a family of window
queries that grows a helper per platform question. The compile-time storage seam of client-stack rule 3
is unaffected.

> **Amended by [ADR-0026 §5](0026-the-per-tap-keyboard-re-pop.md).** This section originally specified
> *"a non-Android implementation that returns zero"*. Zero is also what a **down** keyboard reports, so
> off Android the two states are indistinguishable and any gate on "the keyboard is down" is permanently
> true. The return type therefore distinguishes **no soft keyboard on this platform** from **keyboard
> down**. Still one function; the type gets honest, the seam does not widen.

### 3. Two guards, without which the keyboard oscillates

Both were found by driving the prototype, both are behavioural rather than incidental, and an
implementation missing either is visibly broken. They are specified here because "read the insets and
shrink the viewport" is *not* a sufficient instruction.

- **The focused field must be kept inside the shrunken viewport, in the same frame the viewport
  shrinks.** The text widget publishes its IME output only while its rect is visible, and the layer
  below turns the *absence* of that output into hide-the-keyboard. So reserving the band clips the
  focused field, which hides the keyboard, which collapses the inset, which restores the viewport,
  which shows the field, which raises the keyboard — a closed loop, and on the handset a continuously
  flickering keyboard. Requesting a scroll is not enough: it lands a frame later, and one frame without
  the output is one hide.
- **A focused field the user scrolls *completely* out of view surrenders focus.** The same loop runs
  from the other end when the field leaves the viewport by scrolling. Dragging it back is wrong,
  because scrolling away is deliberate; surrendering focus makes the state consistent — no focus, no
  IME output, no keyboard, nothing to oscillate between. **Completely**, not merely clipped: a field
  half off the edge is still being typed into.

> **Amended by [ADR-0026 §4](0026-the-per-tap-keyboard-re-pop.md): there are three guards, not two.**
> The third is **raise the keyboard from a discrete press on a text field**, and it has a different
> origin — these two follow from *reading insets*, that one follows from *carrying the patch*. Once the
> per-tap interrupt is suppressed, nothing re-asserts show after the user dismisses the keyboard with
> the IME's own chevron, because the layer below debounces its allow-IME flag against a state that never
> changed. An implementation that takes this section as complete ships a keyboard the user cannot get
> back.

### 4. The destructive-edit warning moves above the fields

[ADR-0012 §5](0012-the-note-authoring-experience.md) puts the ambient warning in the form pane and
[ADR-0018 §4](0018-the-card-pane-ordering.md) makes it **primary on both platforms**. Its position —
after the last field — does not survive a soft keyboard, and the failure is exactly the one ADR-0018 §4
diagnosed.

**With the keyboard up, the only thing on screen at the moment of a destructive edit is a counter.**
Staged on the handset: a `basic + reverse` note switched to `basic`, one card dormant, front field
focused. Visible: the `· 1 dormant` marker on the pane toggle. Not visible: the warning, its copy, and
Undo.

ADR-0018 §4 re-read round 1 of the authoring prototype and concluded that **a count is not a warning**
— that the earlier variant carried two defects at once and the repair had been wrongly credited to
position alone. The handset reproduces that failure from the opposite direction: not because a counter
was chosen over a warning, but because the keyboard leaves only the counter standing.

**Reading the insets does not fix this**, and the distinction matters. Reserving the band makes the
warning *reachable* by scrolling; it does not make it *visible at the moment of the edit*. Insets are
necessary and not sufficient.

**And ADR-0018 §4's own argument already predicted it.** It rejected position as the mechanism because
*"ordinal position cannot guarantee the card-pane entry is on screen — blank 18 of 20 lands below the
fold on desktop too"*, and moved the warning to the form pane on the ground that the form pane is **the
one always on screen**. With a keyboard up, the form pane is not always on screen either; only its
first ~565dp is. The argument survives intact and the *position* it chose does not.

**So the warning sits above the fields**, where the form pane's first screen is the part that is
always visible. §4's reasoning is unchanged and its conclusion — the form pane warns, the card pane
demonstrates — is unchanged; only where in the form pane is corrected.

**This is not the pinned header indicator ADR-0018 §4 forbids**, and the difference is stated rather
than assumed. What that section rules out is a **counter** in a fixed header, on the ground that a
count does not warn. This is the warning itself — the sentence naming the card, its history and Undo —
placed where it can be read. A third speaker is still forbidden, and moving the warning adds none:
there are still exactly two, the form pane's warning and the card pane's entry.

### 5. The split view survives, and width was never the risk

At this handset's width both panes fit and read without prompting, so
[ADR-0012 §1](0012-the-note-authoring-experience.md)'s `Write | Cards` toggle stands on its own merits
rather than on necessity, and [ADR-0002 §8](0002-the-card-model.md)'s *"preview beside the input"* is
serialised rather than dropped exactly as §1 claimed. **The failure is vertical, and no width rule
addresses it** — which is why the toggle was never the thing to re-judge and the form pane's ordering
was.

### 6. What this ADR does *not* settle

- **The per-tap keyboard re-pop.** As shipped, text entry on this platform dismisses and reopens the
  soft keyboard on **every tap** into a text field, including a tap on the field that already has
  focus, because the UI layer interrupts IME composition on every pointer interaction and the layer
  below implements that interruption as hide-then-show. It is independent of everything above — it
  reproduces with inset handling switched off, and it is not a layout question. **Since settled by
  [ADR-0026](0026-the-per-tap-keyboard-re-pop.md)**: the guard is carried as a patch to the vendored
  windowing adapter, with the recovery half in this crate's shared text-field wrapper.
- **Visual design**, unchanged from [ADR-0012 §9](0012-the-note-authoring-experience.md) and out of
  scope for the map. Where the warning sits is behaviour; what it looks like is not.
- **Non-Latin authoring on the handset**, which client-stack rule 8 forecloses and this ADR does not
  reopen. The pass was judged with Latin content, as that rule requires.

## Amendments to accepted ADRs

| ADR | What changes | Why |
|---|---|---|
| [0012 §5](0012-the-note-authoring-experience.md) | The form-pane warning sits **above the fields**, not after the last one. | §4 above: after the last field it is off-screen under a keyboard, and what remains is a counter. |
| [0018 §4](0018-the-card-pane-ordering.md) | Its ground — *the form pane is the one always on screen* — is **narrowed**: on a handset with the keyboard up, only the form pane's first screen is. Its conclusion is unchanged. | §4 above. The section's own argument against position applies to the form pane too, one level up. |
| [0003 §6](0003-client-stack.md) | The record of what winit's Android backend does not do gains its second half: **no IME insets**, not only no composed text. | §1 above. The first half was recorded; the second cost 39% of the screen silently. |
| [0016 §5](0016-backup-and-restore.md) | The per-crate platform-seam rule gains a **third** module under it, in the UI crate, held to one function. | §2 above. Recorded so the rule is seen to be applied rather than bypassed, and so [ADR-0009 §4](0009-crate-and-workspace-layout.md)'s erosion signal keeps meaning what it says. |

## Consequences

- **The editor is the first surface that must know about insets, and it will not be the last.** Every
  screen with a text field inherits §3's two guards. They are written as rules rather than as editor
  details for that reason.
- **A viewport that matches what the user can see is now load-bearing for reachability, not comfort.**
  Without it there is no scroll range over the covered band, so "it is below the fold" and "it does not
  exist" are the same thing.
- **The form pane's first screen is now a specified resource.** Two things claim it — the warning, and
  the field being typed into — and anything added to the top of that pane later is competing with a
  warning that was moved there because nowhere else works.
- **The handset can now be used to judge a layout, which it could not before.** The prototype's inset
  read and guards are what make the surface stable enough to react to; the two keyboard defects had to
  be diagnosed before the layout question could be answered at all.

## Open items handed onward

- ~~**The per-tap keyboard re-pop** (§6)~~ — **settled by
  [ADR-0026](0026-the-per-tap-keyboard-re-pop.md)**, which also amends §2's seam return type and adds a
  third guard to §3. Read it before implementing either.
- **What the moved warning looks like** — the visual design pass's, on the same terms
  [ADR-0018](0018-the-card-pane-ordering.md) left for a dormant line: what it says and where it sits are
  settled here, only how it looks is not.
