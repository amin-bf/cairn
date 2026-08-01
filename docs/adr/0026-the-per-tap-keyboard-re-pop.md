# ADR-0026: The per-tap keyboard re-pop, and the first dependency we patch

- **Status**: Accepted
- **Date**: 2026-08-01
- **Resolves**: [Decide: whether we carry a patched `egui-winit` so Android text entry stops re-popping the keyboard](https://github.com/amin-bf/leitner/issues/75)
- **Map**: [Map: local-first Leitner app spec](https://github.com/amin-bf/leitner/issues/1)
- **Related**: [ADR-0003 §3 §5 §6](0003-client-stack.md) (the client stack; the precedent that a
  rendering defect is patched in our own code rather than upstream; what winit's Android backend does
  not do), [ADR-0025 §1 §2 §3 §6](0025-the-authoring-screen-under-a-soft-keyboard.md)
  (the inset seam, the guards that come with it, and the handoff this ADR discharges),
  [ADR-0016 §5](0016-backup-and-restore.md) (the per-crate platform-seam rule),
  [ADR-0001 §1](0001-scheduling-algorithm-and-grade-scale.md) (the exact-pinning precedent)

## Context

[ADR-0025 §6](0025-the-authoring-screen-under-a-soft-keyboard.md) settled the authoring screen under a
soft keyboard and explicitly declined one defect found on the way, because it is not a layout question
and reproduces with inset handling switched off:

**As shipped, text entry on Android dismisses and reopens the soft keyboard on every tap into a text
field — including a tap on the field that already has focus.**

The mechanism runs through three layers, and each link was read in the source rather than inferred.
The UI toolkit's `Memory::request_focus` interrupts IME composition unconditionally, and its
`TextEdit` calls that on **every** pointer interaction — `did_interact || response.clicked()` — with no
check for whether the widget is already focused. The toolkit's windowing adapter implements the
interruption as `set_ime_allowed(false)` immediately followed by `set_ime_allowed(true)`, a mechanism
its own source marks as provisional with a `TODO` asking for a proper interrupt if the windowing layer
ever provides one. The windowing layer's Android backend maps those two calls onto the platform's
hide-soft-input and show-soft-input. So an ordinary tap is a visible dismiss-and-reopen.

**It buys nothing on this platform.** That same backend handles only motion and key events and has no
IME path at all — the gap [ADR-0003 §6](0003-client-stack.md) and client-stack rule 8 already record
for composed text — so there is never a composition to interrupt.

Measured on the Pixel 8 Pro with the platform's own IME request tracker, tapping the already-focused
field three times: **6 hide and 17 show requests** as shipped, **0 and 0** with the block dropped on
Android. Evidence and two rejected fixes are on branch
[`prototypes/issue-67`](https://github.com/amin-bf/leitner/tree/prototypes/issue-67).

**There is no fix available in our own code, and this was established before any option was weighed.**
The interrupt flag is a private field on the toolkit's `Memory`; `interrupt_ime()` is a public
*setter* onto it, the reader is crate-private, and the flag is cleared only in `Memory::begin_pass`.
`Context::end_pass` stamps it onto the frame's platform output *after* all application code has run,
and the application layer offers no hook on that output before the adapter consumes it. An application
can raise the interrupt and can never lower it. The adapter's `State::set_allow_ime` is public, but the
application framework owns the `State` and never hands it over. Short of writing our own text widget,
the fix cannot live above the dependency.

**That test was run first because [ADR-0003 §3](0003-client-stack.md) sets the standard.** The bidi
defect that nearly cost us the stack was fixed in about sixty lines of our own code with **no fork**, by
exploiting a contract the renderer already published — and that ADR names the absence of a fork as the
reason the defect was survivable. This is the same question asked again and answered the other way, on
a private field and a missing hook rather than on preference.

## Decision

### 1. Accepting the behaviour is not available, because it defeats ADR-0025 §1

The obvious reading is that this is cosmetic — an editor that flickers — and cosmetics belong to the
visual design pass, which the map ruled out of scope. That reading was tested and it fails, because
[ADR-0025 §1](0025-the-authoring-screen-under-a-soft-keyboard.md) landed in between.

§1 reads the IME inset **so the covered band becomes reachable at all**: before it, the UI layer was
handed a viewport taller than the visible one, the content fitted inside it, and the covered region was
unreachable rather than scrolled off. The re-pop collapses and restores that inset on every tap, which
restores and re-shrinks the viewport, which is what the prototype observed as the scroll position
resetting. So the user scrolls to bring a covered field into view, taps it, and the scroll just
established is discarded — at the precise moment §1 exists to serve.

That is a reachability guarantee being functionally defeated, not an editor that looks bad.
[§4](0025-the-authoring-screen-under-a-soft-keyboard.md)'s warning-above-the-fields is in the same
blast radius, since a scroll reset is exactly what decides which screen of the form pane is on show.

### 2. The guard goes in the windowing adapter, not in the toolkit's core

Three layers could hold it, and the choice is not the one it first appears to be.

**The toolkit's core looks like the honest bug report.** `TextEdit` calling `request_focus` on a widget
that already has focus, and `request_focus` interrupting composition unconditionally, is a defensible
thing to call wrong on any platform. A `has_focus` guard there would kill the measured case and the
drag case with it, since `did_interact` is true while dragging inside a focused field.

**But it does not fix our problem.** It leaves genuine field-to-field focus changes still interrupting,
and on Android that is still a hide-then-show with no composition to interrupt. Moving from `Front` to
`Back` is the most ordinary act there is on the authoring screen, so the core guard reproduces §1's
inset collapse on a slightly rarer trigger. It narrows the defect; it does not remove it.

**The windowing layer** — giving its Android backend a real IME path — is the fix that would also close
client-stack rule 8. [ADR-0003](0003-client-stack.md) looked straight at that and declined it, and
nothing since has changed the estimate. Not reopened here.

**So the guard sits in the adapter**, suppressing the interrupt on Android. Its justification is a fact
this repository has already validated and written down — the backend has no IME path, so there is no
composition — rather than a premise this ADR invents.

### 3. We vendor the published crate, because forking the repository forks the whole family

The adapter crate **as published** declares an ordinary registry dependency on the toolkit,
`egui = { version = "0.35.0" }`. The same crate **inside its own repository** declares
`egui = { workspace = true }`, which resolves to a path dependency on that repository's copy. So a
fork-and-branch — attractive because re-applying a patch to a newer release is then a rebase, with
conflicts landing exactly on the changed block — would pull a second copy of the toolkit into the
dependency graph beside the registry copy the application framework and our own crates use, whose types
would not unify. Making it work means patching the entire family to the fork, so we would be building
all of the toolkit from a source revision in order to change one block. The vendored tree preserves both
forms of the manifest side by side, which is where this is visible.

**So: a verbatim copy of the published crate, wired in with `[patch.crates-io]`.** Nine files, about
214 KB, carrying exactly one change.

A build-time patch tool was rejected on a softer ground: it adds a third-party step to a stack
[ADR-0003](0003-client-stack.md) chose partly because the build is plain cargo and the APK is a
manifest plus a shared object. A patcher fails at build time on a fresh machine, which is the worst
place to fail.

Three disciplines come with it:

- **Verbatim plus exactly one change**, verifiable by recursive diff against a freshly fetched pristine
  copy of the same version.
- **The version is pinned exactly**, adapter and toolkit together, following
  [ADR-0001 §2](0001-scheduling-algorithm-and-grade-scale.md)'s precedent for the scheduler crate.
- **The vendored tree is outside client-stack rule 3.** That rule makes a `#[cfg(target_os)]` anywhere
  in the workspace a defect signal, and the patch is one. It is not a workspace member — a patched path
  dependency never is — but a reader grepping the tree will find it, so the rule says so explicitly
  rather than letting a correct instance blunt a signal the repository relies on.

### 4. The recovery half is ours, and it lives in the shared text-field wrapper

**Dropping the block alone breaks recovery, and this is the part most likely to be lost in
translation.** The adapter debounces its allow-IME flag against its own previous value. After the user
dismisses the keyboard with the IME's own chevron, the toolkit's state has not changed — only the
platform's has — so nothing ever re-asserts it and tapping a field does nothing. The interrupt block was
the only thing re-asserting show, so removing the defect removed recovery with it.

Re-asserting *show* inside the adapter instead was tried and is worse: `request_focus` fires while
**dragging** as well as on a tap, so a single scroll gesture issued **72 show requests**. Both wrong
versions made the same mistake — hanging behaviour off a per-frame flag when the thing being modelled is
a discrete event.

**So the application raises the keyboard itself, from a discrete pointer press.** It goes through
`ViewportCommand::IMEAllowed(true)`, which the adapter maps straight onto the window without touching
its own debounced flag — public API, and no state to desync.

**It lives in the crate's one shared text-field wrapper**, the one client-stack rule 2 already forces
every field through for the bidi layouter, keyed on the field's own click. Not on a frame-level "some
widget is focused and the pointer went down", which is the prototype's shape: that fires for a focused
button as readily as a focused field, and it is per-frame where the event is discrete. One wrapper is
also the only place that can promise every field behaves the same way.

**This is a third guard**, alongside [ADR-0025 §3](0025-the-authoring-screen-under-a-soft-keyboard.md)'s
two, and it differs from them in origin: those two are consequences of *reading insets*, this one is a
consequence of *carrying the patch*. An implementation that takes the patch without it has a keyboard
the user cannot get back after dismissing it by hand.

### 5. The seam's return type learns to say "no soft keyboard here"

The raise is gated on the platform reporting a keyboard that is currently **down**, because without the
gate every click on a field sends a redundant show request and the validated zero-hides-zero-shows
result becomes zero hides and three inert shows. Those shows are invisible — the harmful half was always
the hide — but the measured shape is the one this repository prefers to ship.

**The gate as the prototype wrote it is a live defect off Android.**
[ADR-0025 §2](0025-the-authoring-screen-under-a-soft-keyboard.md) specifies the UI crate's seam as one
function returning the insets, *with a non-Android implementation that returns zero*. Zero is also what
a down keyboard reports, so on desktop "the keyboard is down" is permanently true, the gate never fires,
and every pointer press with any widget focused re-enables IME behind the adapter's back — including
when the adapter has deliberately disabled it because the focused widget is not a text field.

**So the seam's return type distinguishes "this platform has no soft keyboard" from "the keyboard is
down".** This is not a widening: it stays **one function** under
[ADR-0016 §5](0016-backup-and-restore.md)'s per-crate rule, and the type it returns becomes honest.
Collapsing the two states was the error.

It is deliberately **not** expressed as a second compile-time capability constant. The existing one —
[ADR-0015 §9](0015-the-sync-experience.md)'s non-Latin-input constant, client-stack rule 3's one
sanctioned exception — exists *to make a limitation visible, never to vary behaviour*, and this gate
varies behaviour. A seam that already answers questions about the window is the right owner.

### 6. What re-applying the patch requires, and what invalidates it

**The patch is bound to a block shape, not to a line number.** It guards
`if !is_toggling_ime && ime.should_interrupt_composition { set_ime_allowed(false); set_ime_allowed(true) }`.
If a future release restructures that block, the instruction is **re-judge, not re-apply** — because a
guard mechanically applied to a block that no longer means the same thing looks perfectly healthy in a
diff. This is the silent failure and it is the reason the shape is written out here.

**Routine bumps are gated proportionately**: recursive diff against pristine, plus the block-shape
check. The handset measurement is required only when either of those is unhappy. Demanding a handset run
for every patch bump buys nothing when the block is byte-identical and the guard applied cleanly, and a
discipline that heavy is one that gets skipped — which is worse than a lighter one that is followed.

**The invalidating event is the windowing layer gaining an Android IME path.** Then a real composition
exists, and suppressing its interruption becomes a bug rather than a fix. That same event invalidates
**client-stack rule 8**, which this repository already watches — so rule 8 is the tripwire, stated here
rather than left as an inference for whoever bumps the dependency.

### 7. We report the finding upstream; we do not commit to landing it

**The guard is half a fix, and that is why this is not a pull request.** Suppressing the interrupt
without §4's recovery leaves any other application on this stack with a keyboard that never comes back
after a manual dismiss. An honest contribution has to carry the recovery half *inside* the adapter,
which is a design task — and the 72-request scroll gesture shows how easily it goes wrong there. Its
latency is unbounded and the work sits past a destination that contains decisions.

**The measurement is the expensive part and it is already paid for.** Publishing the mechanism and the
request counts as an upstream *issue* costs almost nothing, lands where upstream's own `TODO` already
concedes the mechanism is provisional, and is the only realistic route by which the vendored tree is ever
retired. Filing it is the repository owner's, on the same terms as issues here.

### 8. What this ADR does *not* settle

- **Non-Latin authoring on Android**, which client-stack rule 8 forecloses. The same missing IME path
  causes both, and fixing that path is the windowing-layer work §2 declines. Nothing here reopens it.
- **Whether the guard is ever accepted upstream.** §7 reports; it does not predict.
- **Any other dependency.** This is the first patched dependency the repository carries, and §3's
  disciplines are written for this one. A second would be an erosion signal worth arguing on its own.

## Amendments to accepted ADRs

| ADR | What changes | Why |
|---|---|---|
| [0025 §2](0025-the-authoring-screen-under-a-soft-keyboard.md) | The seam's **return type** distinguishes *no soft keyboard on this platform* from *keyboard down*, replacing "a non-Android implementation that returns zero". Still one function. | §5 above. Collapsing both to zero makes the off-Android gate permanently true and re-enables IME behind the adapter's back. |
| [0025 §3](0025-the-authoring-screen-under-a-soft-keyboard.md) | Its **two** guards become **three**. The third — raise the keyboard from a discrete press on a text field — has a different origin: it is a consequence of the patch, not of reading insets. | §4 above. §3's list reads as complete, and an implementer taking it as complete ships a keyboard that cannot be recovered after a manual dismiss. |
| [0003 §5](0003-client-stack.md) | The toolchain constraints gain one: **the windowing adapter is vendored and patched**, so a version bump of the stack is no longer only a version change — §6's diff, block-shape check and re-judge rule run with it. | §3 and §6 above. The stack choice is unchanged; what changes is that one crate of it is no longer taken as published. |

## Consequences

- **The repository carries third-party source for the first time**, and the delta is invisible by
  inspection — a reader sees a whole crate, not a one-block change. §3's recursive-diff discipline is
  what makes the delta recoverable, and it is load-bearing rather than tidy.
- **Client-stack rule 3's defect signal now has a stated exclusion.** A correct `#[cfg(target_os)]`
  living in a tree the rule does not govern is exactly how a signal quietly stops meaning anything.
- **The keyboard's correctness now spans three places that must agree** — the vendored guard, the
  raise in the text-field wrapper, and the seam's honest return type. Any one of them alone is a
  regression: the guard alone loses recovery, the raise alone is redundant, the seam alone changes
  nothing.
- **Every screen with a text field inherits this**, on the same terms as
  [ADR-0025](0025-the-authoring-screen-under-a-soft-keyboard.md)'s two guards, because the raise lives
  in the wrapper every field already goes through.
- **The vendored tree has a stated exit.** §7's report is the only route to retiring it, and until
  something lands upstream the patch is permanent by default rather than by oversight.

## Open items handed onward

- **Reporting the finding upstream** (§7) — the repository owner's, not this map's.
- **Nothing else.** The defect is fully characterised, the remedy is chosen, and the residual is a
  watched tripwire (§6) rather than an open question.
