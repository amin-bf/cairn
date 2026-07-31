# UI

The egui application: every screen the user sees, the text-layout helper every one of them goes
through, and both platform entry points.

**Bound by** [ADR-0003](../../../docs/adr/0003-client-stack.md),
[ADR-0006](../../../docs/adr/0006-the-review-session-experience.md),
[ADR-0010](../../../docs/adr/0010-leeches.md) and
[ADR-0011](../../../docs/adr/0011-new-card-rate-and-daily-limits.md), the last of which **amends
ADR-0006 §1 and §2** — read those amendments before touching the session; also by
[ADR-0002 §4](../../../docs/adr/0002-the-card-model.md) (layout is data, stored once per kind).

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
  path. Never design a feature that requires typing non-Latin text on Android.
- **Verify on the real handset.** The emulator is x86_64; the Pixel 8 Pro is arm64-v8a only.

## Why this crate has no `main.rs`

`cargo-apk` panics after signing when one crate has both a cdylib and a bin. The desktop binary is
`leitner-desktop`. Adding a `[[bin]]` here breaks the Android release build (ADR-0003 §5).
