# PROTOTYPE — throwaway. Answers #11 only. Lands as ADR-0006.

**Question:** what does a review session feel like? Judged live by the repo owner, on the Pixel 8
Pro and on desktop, across two rounds of variants. Verdict below; see ADR-0006 for the full
write-up.

## Round 1 — three incompatible session shapes

Explored the ticket's open axes independently: session shape (bounded batch vs. open queue vs.
timed), reveal mechanic (button vs. tap-the-card), box display (walled off vs. inline badge vs.
absent), interval preview (on vs. off). Built as variants A/B/C, switchable from a floating bar,
against four scenarios (Normal / Empty / New deck / Backlog).

Judged live: on the Pixel 8 Pro (real touch, real force-stop/relaunch to prove session position is
log-derived, not stored) and on desktop. The switcher bar itself turned out to have a real
touch-dispatch quirk under `adb`-synthesized taps — real-finger touch on the device worked
correctly throughout; only the synthetic-input debugging path was affected, so it wasn't
investigated further.

**Converged live on a graft, not a single variant**: pick a card count up front (round 1's
variant A) + a 10-minute timer with backlog-aware framing (variant C) + tap-the-card-to-reveal
with a box badge and per-grade interval preview (variant B) — but softened: reaching the timer
should offer a choice ("finish here" / "keep going"), not force a stop.

## Round 2 — three presentations of the converged design

With the interaction model fixed (see `core.rs` — shared by all three, presentation-only
differences from here), rebuilt variants A/B/C as three layout treatments of the *same* mechanics:

- **A — Card-first.** Chrome shrinks to a thin corner readout (progress, timer); the card fills
  the screen; box badge and interval preview are quiet monospace footnotes.
- **B — Dashboard header.** A persistent header strip (progress bar + timer + backlog note)
  always visible above the card; interval preview originally a 4-column grid, revised live to
  full-width grade buttons stacked vertically (each showing its interval underneath).
- **C — Checkpoint-forward.** Design attention on the newest, least-tested piece — the timer text
  warms from gray to amber in the final 60s instead of jumping straight to a banner, and the
  finish-or-continue checkpoint slides in as a footer under the card rather than replacing it.

## Verdict

**B — Dashboard header, with vertically-stacked grade buttons** (not the original 4-column grid).
Chosen live after trying all three on desktop. Explicitly **not** a verdict on visual styling —
the dark palette/spacing throughout is scaffolding inherited from the `prototypes/egui-slice`
client-stack prototype for convenience, not a considered design decision; a real look-and-feel
pass is separate, later work.

## Shared mechanics (`core.rs`, fixed across all variants from round 2 on)

- **Session shape**: pick a count (10/20/40, capped by what's due) — the timer starts at that
  moment. Reaching the count ends normally. Reaching the 10-minute mark surfaces a checkpoint
  ("finish here" / "keep going") instead of forcing a stop; "keep going" dismisses it and the
  session continues untimed.
- **Session position is never stored** — only derived. The due queue is `scenario's due cards
  minus cards with a log entry`, recomputed every frame. The chosen count and the timer start are
  *not* persisted, only grades are — proven for real via a Pixel 8 Pro force-stop/relaunch and via
  a desktop "Simulate kill & restart" control: both return to the count-picker with the right
  cards already excluded.
- **Reveal** is tap-the-card, not a separate button — verified by real touch on the Pixel 8 Pro
  and by mouse click on desktop.
- **Box display** resolves constraint 4 for the review screen: a small, non-interactive,
  monospace badge (`"box 3"` / `"new"`) appears only *after* reveal — never before, never sorted,
  never counted, never presented as a queue.
- **Interval preview** is on, per grade button, illustrative (`"~9d"`-style) — confirmed wanted,
  not noise.
- **Empty/new-deck/backlog states** are explicit worded states, not blank screens.
- **Offline**: no networking code anywhere in this crate — structural, not runtime-checked.
- **Log persistence** is a flat JSON-lines file (desktop: next to the binary; Android: the app's
  private files dir via JNI, `android.rs`, copied verbatim from `prototypes/egui-slice`) — a
  throwaway shim, not ADR-0004's real event log, that exists only to make restart-resume concrete.

## Run it

```
cd prototypes/review-session-11
cargo run
```

`←`/`→` (or the switcher) cycle A/B/C; scenario buttons pick the data condition.

## Capture

Landed as ADR-0006. This prototype (all three round-1 variants plus all three round-2 variants)
is throwaway evidence — tagged `prototypes/issue-11`, not merged to `main`, same convention as
`prototypes/issue-8`.
