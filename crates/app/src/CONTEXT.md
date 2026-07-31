# UI

The egui application: every screen the user sees, the text-layout helper every one of them goes
through, and both platform entry points.

**Bound by** [ADR-0003](../../../docs/adr/0003-client-stack.md),
[ADR-0006](../../../docs/adr/0006-the-review-session-experience.md),
[ADR-0010](../../../docs/adr/0010-leeches.md) and
[ADR-0011](../../../docs/adr/0011-new-card-rate-and-daily-limits.md), the last of which **amends
ADR-0006 §1 and §2** — read those amendments before touching the session; also by
[ADR-0002 §4](../../../docs/adr/0002-the-card-model.md) (layout is data, stored once per kind) and
[ADR-0015](../../../docs/adr/0015-the-sync-experience.md) (everything the user sees about sync — the
`sync` crate holds the mechanism and none of the surface).

## Language

**Session**:
One sitting of review: a chosen card count, with a 10-minute timer running from the same moment.
**Not a domain object** (ADR-0005 §6) — it exists only here, and its position is never stored, only
derived from the log.
_Avoid_: Study session, cram session, queue.

**Session count**:
The size the user picks at the start of a session, and **the only bound on review work there is** —
no daily review limit exists, and a user may start as many sessions in a day as they like
(ADR-0011 §1). It counts **gradings, not distinct cards** (ADR-0011 §9), so a lapse re-show advances
it and the progress bar always moves when the user acts.
_Avoid_: Daily limit, quota, target.

**Checkpoint**:
What the timer surfaces when it expires: finish here, or keep going. A courtesy check-in, never an
enforcement mechanism — reaching the chosen count is what ends a session normally.

**Reveal**:
Tapping the card to show its back. Verified identical by touch and by mouse; the two platforms do
not diverge here.

**Box badge**:
The small, non-interactive indicator shown **only after reveal**. Reports durability. Never sorted,
never counted, never presented as a queue — see `scheduling`'s rules, which bind everything in this
file.

**Interval preview**:
The illustrative next-interval shown on each grade button. Confirmed wanted rather than noise once
seen rather than described.

**Backlog**:
More cards due than the user will get through. Always *framed* ("pick a comfortable size, the rest
will keep"), never reported as a bare number.

**Last caught up**:
The only resting statement the app makes about sync — *when* it last completed one, a fact.
**Never "in sync"**: after a sync the app knows every writer's highest *published* sequence, and
never whether another device has reviewed since. Claiming agreement between devices is unknowable
(ADR-0015 §4), the same shape as the box badge claiming something about the queue.
_Avoid_: In sync, up to date, synced, a status icon or checkmark anywhere in the chrome.

**Set up sync**:
Granting this device access, once, via the device flow. Ends with the user naming **this** device
(ADR-0015 §8) and with the app stating what it found — *"the first device here"* or the devices it
met — because a wrong-account enrolment is otherwise undetectable (ADR-0015 §7).
_Avoid_: Login, sign-in, pairing, connecting an account.

**The notice channel**:
The persistent, non-modal line for the **only two things permitted to speak about sync**: a dead
grant, and ADR-0004 §8's clock-skew warning. A network failure never speaks — offline is normal
(ADR-0015 §5).

## Rules that are easy to break silently

- **All user-visible text goes through `bidi`.** egui places text runs left-to-right in logical
  order, so a plain `ui.label("…")` renders Persian with the words backwards and Arabic-Indic digits
  reversed. This is the single most likely way to break the app without any test failing.
- **`TextEdit` needs the same treatment, via `.layouter()`** — it lays out its own text and
  otherwise bypasses the helper. Caret and selection are then in visual order while the buffer is
  logical, so RTL editing is imprecise; design around it rather than fighting it.
- **Immediate mode has nowhere to `await`.** Spawn the future, store a handle, read the result on a
  later frame, and call `ctx.request_repaint()` on completion or the result sits unseen until the
  next input event.
- **A backgrounded Android app is frozen, not slowed**, so long work starts from the foreground, by
  a user action, with nothing persisted until it completes — then a frozen or killed run leaves no
  partial state to repair. Never schedule it, and never tell the user a started job is still
  progressing (ADR-0014 §3).
- **Fonts are installed on the first frame, never in `CreationContext`**, and every added face must
  be registered in **every** family including `Monospace`, or text silently renders as boxes.
- **Android text input is ASCII-only and cannot be fixed here.** winit's Android backend has no IME
  path. Never design a feature that requires typing non-Latin text on Android. Because we receive no
  events, the failure is *silence* — so the editor states it in advance, off a compile-time
  capability constant (ADR-0015 §9). That constant is the one sanctioned exception to the
  no-`#[cfg(target_os)]` rule, and it exists to make a limitation visible, never to vary behaviour.
- **Never start a sync while the review screen is up**, and let one already in flight finish. This
  is not a lock on review — the app never blocks reviewing (ADR-0015 §1) — it is what stops a merge
  recomputing every `(S, D)` mid-session, which ADR-0014 called locally unfixable. It works only
  because there is no background sync, so treat that absence as load-bearing (ADR-0015 §6).
- **Only two things may speak about sync**, and every future feature will have a reason to want a
  third. A badge, a toast on success, a "syncing…" indicator in the chrome — each is a defect
  against ADR-0015 §4, not a UX improvement.
- **Verify on the real handset.** The emulator is x86_64; the Pixel 8 Pro is arm64-v8a only.

## Why this crate has no `main.rs`

`cargo-apk` panics after signing when one crate has both a cdylib and a bin. The desktop binary is
`leitner-desktop`. Adding a `[[bin]]` here breaks the Android release build (ADR-0003 §5).
