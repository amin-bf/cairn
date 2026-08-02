# The one patched dependency

`vendor/egui-winit` is a **verbatim copy of the published `egui-winit` 0.35.0** carrying exactly one
change. It is not our code and it is not a workspace member — it is reached only through
`[patch.crates-io]` in the root `Cargo.toml`.

Decided in [ADR-0026](../docs/adr/0026-the-per-tap-keyboard-re-pop.md); `AGENTS.md` client-stack
rule 12 is the short form.

## What the change is

One attribute, in `src/lib.rs`, on the block that interrupts IME composition:

```rust
#[cfg(not(target_os = "android"))]
if !is_toggling_ime && ime.should_interrupt_composition {
    window.set_ime_allowed(false);
    window.set_ime_allowed(true);
}
```

## Why

As published, **every tap into a text field on Android dismisses and reopens the soft keyboard** —
including a tap on the field that already has focus. Three layers, each read in the source rather
than inferred:

- `egui`'s `TextEdit` calls `request_focus` on **every** pointer interaction
  (`did_interact || response.clicked()`), with no check for whether the widget is already focused.
- `egui`'s `Memory::request_focus` interrupts IME composition unconditionally.
- This crate implements that interruption as `set_ime_allowed(false)` immediately followed by
  `set_ime_allowed(true)` — a mechanism its own source marks provisional with a `TODO` asking for a
  real interrupt if winit ever provides one — and winit's Android backend maps those two calls onto
  the platform's `hide_soft_input` and `show_soft_input`.

Measured on the Pixel 8 Pro with Android's own `ImeTracker`, tapping the already-focused field three
times: **6 hide and 17 show requests** as published, **0 and 0** with the block guarded.

**It buys nothing on that platform.** winit's Android backend handles only motion and key events and
has no IME path at all, so there is never a composition to interrupt (`AGENTS.md` client-stack
rule 8). And it costs more than flicker: the hide-then-show collapses and restores the IME inset,
which throws away the scroll position [ADR-0025 §1](../docs/adr/0025-the-authoring-screen-under-a-soft-keyboard.md)
exists to make meaningful — at the precise moment it exists to serve.

## Why it is vendored rather than fixed above, or forked

**Above:** there is nothing to fix from. The interrupt flag is a private field on `egui`'s `Memory`;
`interrupt_ime()` is a public *setter* onto it and the reader is crate-private; it is cleared only in
`Memory::begin_pass`; and `Context::end_pass` stamps it onto the frame's platform output after all
application code has run, with no hook on that output before this crate consumes it. An application
can raise the interrupt and can never lower it.

**Forked:** the crate *as published* declares an ordinary registry dependency on `egui`. The same
crate *inside its own repository* declares `egui = { workspace = true }`, a path dependency on that
repository's copy — so a fork pulls a second `egui` into the graph beside the registry one `eframe`
and our crates use, and the two sets of types do not unify. Both manifests are preserved side by
side here (`Cargo.toml` and `Cargo.toml.orig`), which is where this is visible.

## Half a fix — the other half is ours

Dropping the block alone **breaks recovery**, and this is the part most easily lost. This crate
debounces its allow-IME flag against its own previous value. After the user dismisses the keyboard
with the IME's own chevron, `egui`'s state has not changed — only the platform's has — so nothing
re-asserts show and tapping a field does nothing. The interrupt block was the only thing re-asserting
it.

So `leitner-app` raises the keyboard itself, from a **discrete click** on a text field in the shared
text-field wrapper, through `ViewportCommand::IMEAllowed(true)` — public API that reaches the window
without touching this crate's debounced flag. Re-asserting show *inside* this crate was tried and is
worse: `request_focus` fires while **dragging** too, so a single scroll gesture issued **72 show
requests**.

This is also why the finding is reported upstream as an issue rather than as a pull request
(ADR-0026 §7): the guard on its own would leave any other application on this stack with a keyboard
that never comes back.

## Bumping it

Run:

```
scripts/verify-vendor.sh
```

It does the two checks ADR-0026 §6 requires, and they answer different questions:

1. **Recursive diff against a pristine copy** — nothing else moved.
2. **The block-shape check** — the guard is still on the block it was justified against.

**The patch is bound to that block's shape, not to its line number.** If a release restructures it,
the instruction is **re-judge, not re-apply**: a guard mechanically applied to a block that no longer
means the same thing looks perfectly healthy in a diff. That is the silent failure, and it is why the
shape is written out rather than a location.

Routine bumps — both checks clean — need nothing else. The handset measurement
(`AGENTS.md` client-stack rule 9) is required only when either check is unhappy.

## What retires it

**winit's Android backend gaining an IME path.** Then a real composition exists and suppressing its
interruption becomes a bug rather than a fix. That same event retires `AGENTS.md` client-stack
rule 8, which this repository already watches — so rule 8 is the tripwire for this directory too.

## Note for anyone grepping the tree

Client-stack rule 3 makes a `#[cfg(target_os = ...)]` anywhere in the workspace a defect signal. The
one in this directory is correct and this tree is **outside that rule** — it is not a workspace
member and it is not our code. Said explicitly because a correct instance of a construct used as a
defect signal is how the signal quietly stops meaning anything.
