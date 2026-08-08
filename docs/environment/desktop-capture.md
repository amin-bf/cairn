# Capturing the desktop app's screens

The design pass ([#121](https://github.com/amin-bf/cairn/issues/121)) judges by looking, so it needs
screens as images, cheaply and repeatably. This is how they are made. The procedure is one command —
`scripts/capture-desktop.sh` — so this document is the reasoning behind it and the facts it rests on,
not a sequence to retype.

```
scripts/capture-desktop.sh scripts/storyboards/baseline.txt 1280 800
```

Images land in `target/capture/<width>x<height>/`, which is ignored by git. A set worth keeping is
copied out deliberately; the first such set is `docs/design/baseline-2026-08-08/`.

## What it rests on

Measured on the machine this was established on, 8 August 2026:

| | |
|---|---|
| Session | Wayland, KDE Plasma, `kwin_wayland` 6.7.4, with XWayland available |
| Keyboard | one layout, `us`; no Persian layout configured |
| Input method | **none installed and none running** — no ibus, no fcitx5 |
| Cold build | `cargo build -p cairn-desktop` from a clean worktree: **21.2 s** wall, 3 m 31 s CPU across 16 cores, rustc 1.97.0 |
| Binary | `target/debug/cairn` — the crate is `cairn-desktop`, the binary is not |
| Default window | 560×860, set in `crates/desktop/src/main.rs` |

## Nothing lands on the operator's screen

The app runs inside a **nested compositor rendering to a virtual framebuffer**
(`kwin_wayland --virtual`), on a throwaway XDG profile. Two properties follow, and both are the
point rather than a nicety:

- **A capture costs the operator nothing.** No window appears, no focus is taken, no screen is
  grabbed. A screen that costs someone their display to look at gets looked at less, and a design
  pass that is judged by looking cannot afford that.
- **A capture cannot touch the real collection.** `cairn-store` locates its databases through
  `XDG_DATA_HOME` and `XDG_STATE_HOME` (`crates/store/src/platform/desktop.rs`), so redirecting both
  into a temporary directory is sufficient and needs no flag in the app. The directory is wiped
  before each run, which means every run starts from a **first launch** — so the seed in
  `CairnApp::open_store` is the fixture, and one run's grades cannot colour the next.

## The output is exactly the size asked for

A window rule written into the scratch config fullscreens whatever opens and drops its border, so
the client area *is* the output: `1280 800` produces 1280×800 of application pixels, with no title
bar, no drop shadow and no desktop behind it. Two runs at one size produce images that can be diffed.

The alternative — shooting the window and cropping — was tried and does not reproduce. The
client-side decorations carry a soft shadow whose alpha falls off gradually, so `-trim` lands on a
different rectangle from one run to the next (measured: a 560×860 window shot as 690×958, trimming
to 672×940, neither of which is the client area).

## Driving it

Past the first screen, something has to click. The app is launched as an **X11 client on the nested
XWayland** so `xdotool` can drive it; nothing else in the session drops to X11, and in particular
`spectacle` keeps its Wayland socket, because the image comes from KWin's ScreenShot2 interface and
an X11 spectacle silently produces no file at all.

**KWin asks before letting an X11 client inject input** — *"xdotool is asking to control input
devices"* — and an unattended run has nobody to answer it: the prompt sits there and every shot after
the first is a picture of the prompt. `XwaylandEisNoPrompt=true` in the scratch `kwinrc` is the
switch that question is asked from. It is set **only in the scratch config**, so the operator's own
session keeps its prompt; the grant covers one throwaway compositor with one client in it.

### Storyboards

A storyboard is a line-per-step file under `scripts/storyboards/`:

| line | effect |
|---|---|
| `shot <name>` | screenshot into `$CAIRN_SHOTS/<name>.png` |
| `sleep <n>` | wait |
| `restart` | kill and relaunch the app on the same collection |
| `sh <command>` | run a shell command — used for `xdotool type`, which needs its own quoting |
| anything else | passed to `xdotool` verbatim, e.g. `mousemove 640 131 click 1` |

**The first click of any storyboard is spent giving the window keyboard focus and never reaches a
widget**, so aim it at empty space. This is not a timing problem and more settle time does not fix
it — but it *is* fixable another way, and the distinction matters when a storyboard has no empty
space to spare. Sending the move and the press as **separate lines** with a settle between them
(`mousemove x y`, `sleep 0.5`, `click 1`) delivers the motion as its own event and the first click
then lands. Measured while proving the #131 prototype's click-through mode: as one
`mousemove x y click 1`, none of three nav clicks reached a widget; split, all three did.

**Write `%CX%` and `%CY%` rather than a literal centre.** They expand to the centre of the output, so
one storyboard runs at any width. Almost every control the app draws is full-width.

**And write `%LX+n%` rather than a literal left-edge x.** It expands to `n` px inside the **page
frame's** column (`%LX%` alone is the edge itself). A literal x used to be correct for the nav row and
anything else packed against the start of a line, because content ran to the window edge. Since
[ADR-0031](../adr/0031-the-page-frame.md) the column is centred, so the same nav button sits at
`320+n` at 1280 and `28+n` at 560 — and a storyboard written with literals clicks empty page at one
width and the wrong control at the other. That is the failure `%CX%` exists to kill, arriving from
the other side.

The margin and measure are **duplicated** in `capture-desktop-session.sh` rather than read out of the
binary, and overridable with `CAIRN_PAGE_MARGIN` / `CAIRN_MEASURE`. A harness that imported them
could not start unless the app was already correct, and photographing a *broken* app is most of what
this is for.

**Two of the app's arrangements move the nav row, so mind the order of a storyboard.** Above
`frame::TWO_COLUMN_MIN_WIDTH` the editor takes a wider frame and the nav follows it (ADR-0031 §3), so
a `%LX+n%` nav click *after* the editor is open lands on empty page. Leaving by *Done* does not help:
that button is compact at 1280 and full-width at 560, so no single coordinate reaches it at both.
`storyboards/baseline.txt` visits the editor **last** for exactly this reason, and says so.

This is worth more than convenience, because **the failure it prevents is silent**. A click aimed at
a full-width control with a hard-coded `640` simply misses at 560 — nothing errors, the screen never
changes, and the next `shot` photographs the *previous* screen under the new screen's name. The
first narrow run here produced three such images and they looked entirely plausible.

**Coordinates are still the brittle part, and knowingly so.** They are fixed pixel positions, and
this map exists to move the things they point at — so a storyboard is expected to need editing
whenever the layout it drives changes. That cost was accepted rather than overlooked: the alternative
is a capture-mode entry point *inside* the app, which is app code shaped by the harness's needs, and
that is a decision the design pass should make deliberately if the editing ever becomes the expensive
part. Keep storyboards short, keep coordinates commented with what they aim at, and **look at the
images** — a storyboard cannot tell you it missed.

## Persian: what the harness proves, and what it does not

`scripts/storyboards/persian.txt` types Persian into the note editor and captures the result. It
**does** demonstrate, end to end:

- Persian characters reach the app as ordinary key events and arrive in the field;
- the text is shaped and joined correctly, and right-aligned, with the full stop at the far left;
- a note carrying Persian saves, and its card preview renders the same text;
- Latin in a second field on the same note is unaffected.

`xdotool type` remaps keysyms to reach characters the layout does not carry, so what it exercises is
the **keymap** path — the same events an `ir(pes)` layout produces. Persian needs no more than that:
it is an alphabetic script typed one character per key, with no composition step.

**It does not exercise a composing input method.** `zwp_text_input_v3`, and with it the preedit
handling in the vendored `egui-winit` adapter, is untouched by this — and cannot be exercised on this
machine at all, because no input-method framework is installed. That is a gap in evidence, not a
known failure: nothing here suggests the composing path is broken, only that it is unproven. Proving
it needs an input method installed (`fcitx5` plus a Persian engine) and a person at the keyboard,
and it is only worth doing if the product ever needs input that composes.

Android is the opposite case and is settled: it has **no** IME path at all, so composed text never
reaches the app there (`AGENTS.md` client-stack rule 8).
