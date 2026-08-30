# The states the harness could not reach

Twenty captures of the **shipped app** in four decided Review states that no capture in this
repository had ever held, taken with the fixture bench
[#153](https://github.com/amin-bf/cairn/issues/153) built.

Every earlier set here is a picture of a **first launch**. The capture harness wipes the app's whole
data directory per run, so the seed in `CairnApp::open_store` — six fresh cards, all due — was the
only collection anything was ever photographed against. The caught-up floor, the leech screen, the
end-of-session pointer and the 10-minute checkpoint are simply not in that collection, and
[#134](https://github.com/amin-bf/cairn/issues/134) had to photograph three of them inside a
**prototype**. So until now the application shipped decided states whose only pictures were of
something that is not the application. These are the first pictures of them that are.

**Nothing about the app changed to make them reachable.** A fixture is a **pre-made collection**
dropped into the scratch directory the harness already owns, so `open_store` and the shipping seed
are untouched and [`docs/design/baseline-2026-08-08/`](../baseline-2026-08-08/README.md) stays
comparable to everything after it. The one exception is `09-checkpoint`, which is not a collection
state at all — see below.

## What produced these

```sh
cargo build -p cairn-desktop
for s in caught-up leeches crossing backlog checkpoint; do
  scripts/capture-desktop.sh "scripts/storyboards/$s.txt" 1280 800
  scripts/capture-desktop.sh "scripts/storyboards/$s.txt"  560 860
done
scripts/capture-desktop.sh scripts/storyboards/fixture-bench.txt 1280 800
```

Each storyboard **names its own fixture** on a `fixture <name>` line, so it cannot be run without
one; `cairn-fixture` verifies the collection landed where the fixture says and the run is abandoned
if it did not. Nothing appears on the operator's screen and nothing touches their collection
(`docs/environment/desktop-capture.md`).

`10-` and `11-` come from `fixture-bench.txt` and exist at 1280×800 only — see *The bench itself*.

## What to look at

| | |
|---|---|
| `01-caught-up` | **ADR-0034 §3's floor**, from the application rather than a prototype: the statement centred at the display tier, given the screen, with nothing else on it |
| `02-caught-up-with-leeches` | the same floor **with a control under it**. This is the second call site ADR-0035 §1 has ever had, and the app does **not** put this control on the reach line — it sits directly under the statement at y=252 of 800. [#155](https://github.com/amin-bf/cairn/issues/155) owes §1 an amendment and this is the picture to judge it from |
| `03-leech-screen` | ADR-0010 §6's screen, drawn for the first time. It is visibly **pre-design-pass**: three rows of stock-weight buttons with a card preview crammed inline, no rhythm between rows, and *Suspend*/*Delete* wearing the same weight as the note itself |
| `07-session-pointer` | the end-of-session pointer, reachable only because the fixture leaves a card **one failure day short** of the floor and the sitting is what pushes it over |
| `08-backlog` | the **link accent's only call site**, lit. `or a shorter sitting: 5 10 20` is invisible on a fresh collection, because a first-run queue is five cards and none of 5/10/20 is shorter than five |
| `09-checkpoint` | ADR-0006 §1 and ADR-0034 §4, photographed: the check-in on one line **above** a card that is still visible and still gradeable. The app contradicted §1 here for ten months and nothing failed, because reaching this state needed ten real minutes |
| `06-crossing-revealed` | the reach line holding at both heights — the grade cluster's bottom edge lands ~163px above the page at 800 and at 860, which is what `%BY-n%` was added to keep true |

## The bench itself

`10-installed-from-settings` and `11-bench-verdict` are not design captures. They drive the
**temporary block on Settings** — the fixture bench's other way in — and exist because that block is
the only route on a handset: `getFilesDir()` is not writable from outside the app, and
[#141](https://github.com/amin-bf/cairn/issues/141) found that an uninstall is not a first launch
there either, since ADR-0007 §6 deliberately puts it in the Auto Backup set. So
[Android Checkpoint Two](https://github.com/amin-bf/cairn/issues/126) reaches every sub-state by
thumb through that block, and this pair is what keeps the handset from being where it is first found
broken.

Installing a fixture lands you on Review, because looking at the state is the point;
`11-bench-verdict` is what the bench had to say, kept on Settings for the return trip, because a
handset has no console to print it to.

## What is not here

**Nothing in light.** These are dark only. The same storyboards can be re-run under the light
appearance, but doing so means switching the theme through the real control on Settings first (the
argument `storyboards/light.txt` records), which is a different storyboard rather than a flag — and
no live ticket needs it yet.

**No Persian card.** [#132](https://github.com/amin-bf/cairn/issues/132) found that the French seed
hides right-to-left defects — the card face was drawing Persian 455px off the window with nothing
failing — and every fixture here is French for continuity with the seed, so it hides them too. A
right-to-left fixture is the obvious next one and was left out because Persian on a card is *already*
reachable, by typing a note in the editor (`storyboards/persian.txt`), and this ticket's brief was
the states that are not.
