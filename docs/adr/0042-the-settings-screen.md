# ADR-0042: The Settings screen — groups by where a setting lives, and the reach line binds a closing control

- **Status**: Accepted
- **Date**: 2026-10-29
- **Resolves**: [Design Pass: Take the Settings Slice — What the Screen Is Once the Specimens Leave](https://github.com/amin-bf/cairn/issues/152)
- **Map**: [Map: The Cairn Design Pass](https://github.com/amin-bf/cairn/issues/121)
- **Related**: [ADR-0035 §1](0035-the-vertical-anchor.md) (the reach line, which §1 here amends),
  [ADR-0039 §8](0039-the-list-row.md) and [ADR-0040 §4](0040-the-note-editor.md) (the two earlier
  readings of it, the first of which §1 agrees with and the second of which it replaces),
  [ADR-0011 §5](0011-new-card-rate-and-daily-limits.md) (the rate syncs),
  [ADR-0014](0014-when-parameter-optimisation-runs.md) (optimise),
  [ADR-0015 §3 §7 §10 §12](0015-the-sync-experience.md) (the sync surface and *one screen, and
  everything sync-shaped is on it*), [ADR-0036 §3](0036-the-light-palette.md) (appearance is
  device-local), [ADR-0041 §5](0041-the-file-surface.md) (the file list, which #167 put on this screen)
- **Evidence**: one sitting over a review page of the whole screen, the enrolled sync row and
  enrolment, each drawn in the application's own type and palette; then the running application
  captured at 560×860 and 1280×800 by every storyboard that crosses Settings. The prototype is
  preserved as the tag `prototypes/issue-152`, readme at
  [`docs/design/prototype-152/README.md`](../design/prototype-152/README.md).

## Context

The ticket read Settings as nine blocks: appearance, the new-card rate, optimise, the sync surface,
reset, two development specimens, and the file list and inbound preview. [#167](https://github.com/amin-bf/cairn/issues/167)
settled the last two — the file list stays on Settings as a specified block and the preview is a
screen of its own — so this slice was left with seven blocks, one sub-state (enrolment) and a sync
surface that cannot be drawn enrolled here, because the device flow needs a network this
environment lacks (ADR-0013 §11).

Two things on the screen were wrong rather than undecided. **Enrolment drew under the page's
heading**, so the screen read *Settings* and *Set up sync* one over the other. **The sync block had no
label**: it began with the promise sentence, the only block on the page that did not say what it was.
And the code carried a comment saying *everything below Appearance syncs*, which was false for two of
the three blocks below it — the sync grant and the file list belong to this machine.

The order had one principle, from [#143](https://github.com/amin-bf/cairn/issues/143): Appearance
first, because it is about this device. It had been put there partly so the capture harness could
reach it — a control below wrapping prose sits at a different y at each width — which is a reason
that looks like design once nobody remembers it.

## Decision

### 1. The reach line binds a screen's **closing** control

> **ADR-0035 §1 places the control pressed to finish or go on**: the grades, the editor's *Done*, the
> import gate, the leech entrance. A page of peer settings has no such control, so Settings places
> nothing on the line. Its sub-states do: enrolment's *Back* sits on it now, and the enrolled sync
> screen's control will.

ADR-0040 §4 read §1 as *the last control on the page*, which worked for the editor because *Done*
was both. On Settings the two come apart. The last control is whatever block happens to be drawn
last, and the screen's several blocks are equals — pinning one would rank it above the others for
no reason but its position. A hand returns to the grades dozens of times in a sitting, and that is
what earned them the line; nothing on Settings is pressed that way.

This agrees with [ADR-0039 §8](0039-the-list-row.md), where the note list's *Create note* is on the
line because it is how the screen is left for the work, not because it is last. **Amin was unsure of
this one and took the recommendation**, so it is the section most open to a later sitting.

### 2. The screen is grouped by **where a setting lives**

> Two groups, each a name in the bold face at body size with a caption under it:
> - **Studying** — *Follows your collection to every device.* — the new-card rate and optimise.
> - **This device** — *Stays on this machine.* — sync, the file list and appearance.

#143's principle was the right one and it was stated in a comment nobody could see. The headings
make it visible and make it true: the rate and the scheduler's parameters are what ride the
collection to another device (ADR-0011 §5, ADR-0014), and the sync grant, the files this application
wrote and the theme are this machine's. The order inside the page now follows the groups.

The group heading is the bold face at body size because that is how the import preview names a deck
(ADR-0041): a name over what follows, not a second page heading.

### 3. The development controls stay, **at the end, under one heading, in every build**

> Reset, the fixture bench, the rendering specimen and the hand-off specimen sit under a bold
> **Development** heading after a separator at the very end of Settings. They are drawn in every
> build.

The judgement the ticket asked for — whether a discharged instrument earns a place on a user's
screen — went to keeping them, **against the recommendation**, which was to remove both specimens
(their tickets, #97 and #98, are closed, and a native client needs its own bench) and to draw reset
and the bench only in a build with a bench switch. Amin chose to keep all four and always draw them.
Their reasons have not gone: a script is judged only by a reader on the device, a hand-off seam is
verified only against a real platform, and the fixture bench is the handset's only route to a
fixture. The heading is what keeps them from reading as settings.

### 4. Enrolment is a sub-state with **one** heading and *Back* on the line

> Enrolment replaces the whole screen and draws its own heading only. *Back* is the lone primary,
> placed with `frame::slack_above` on the reach line. When enrolment gains a way forward, that becomes
> the primary on the line and *Back* becomes a text action.

### 5. Enrolled, sync becomes **one row that opens its own screen**

> Decided, not drawn. Once a device is enrolled the sync block on Settings is a single row, and the
> resting state, the account, *Sync now*, the device list, Disconnect and the history cutoff live on
> a sync screen of their own.

This reads ADR-0015 §12's *one screen, and everything sync-shaped is on it* literally, and it is the
shape enrolment already has. It is recorded here and on `sync_block` because nothing in this
environment can enrol, and a faked grant would be a picture of a state the application cannot reach.

### 6. The storyboards reach Appearance **by keyboard focus**

> `light.txt` and the other light storyboards press Tab eight times and Space rather than clicking a
> coordinate.

Moving Appearance off the top of the page made its y depend on the prose above it again. Focus order
does not move with width, so the storyboards Tab to the control instead. The trade is that a block
added above Appearance adds stops, which the storyboard's comment lists.
[`desktop-capture.md`](../environment/desktop-capture.md) records the technique.

## Consequences

- **ADR-0035 §1 carries an amendment** pointing here, and ADR-0040 §4's reading — *the last control
  on the page* — is replaced by *the closing control*. Every existing call site already satisfies
  both.
- **The storyboards that cross Settings were re-measured.** `file-surface.txt` and
  `fixture-bench.txt` still click coordinates and still run at 1280×800 only; their rows moved down by
  the group heading.
- **Two follow-ons go to the map's *Out of scope***: the enrolled sync screen (§5), which waits on a
  live grant, and the export screen, the next caller of `hand_off`. This slice added no capability.
- **Not photographed:** the handset, and the enrolled sync row.
