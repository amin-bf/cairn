# PROTOTYPE — round 3, throwaway. Answers #67 only.

**Question:** does the authoring screen's split view survive a soft keyboard on the handset — and if
it does not, what gives way?
[#67](https://github.com/amin-bf/leitner/issues/67), handed on by
[ADR-0012 §9](../../docs/adr/0012-the-note-authoring-experience.md) and
[ADR-0021 §10](../../docs/adr/0021-note-ordering-saving-and-the-note-list.md).

Round 2 (`PROTOTYPE.md`, tag `prototypes/issue-28`) judged the whole editor on a desktop with a faked
phone **width**. Its own note said the width preset "cannot fake a soft keyboard taking half the
screen, which is the part of that question only the Pixel can answer." This is that pass.

**Not yet judged.** Everything below is measurement and the artefact to react to. The decision is
the repo owner's, live on the handset — the two questions waiting for that are at the bottom.

## Run it

```
cd prototypes/note-authoring-28
source ../../scripts/android-env.sh
cargo apk build --release            # panics "Bin is not compatible with Cdylib" *after* signing
adb install -r target/release/apk/note-authoring-28.apk
```

The panic is the known cargo-apk defect ([ADR-0003 §5](../../docs/adr/0003-client-stack.md)) — it
fires after the APK is signed, so the artefact is good and the exit code is not.

On the handset the app opens on **variant D at phone width** — round 2 had to remember to press a
toggle, and this pass does not have to fake anything. The prototype's own controls are collapsed
behind `▸` at the **top**; at the bottom they are the first thing the keyboard eats, and then the
switch being judged is unreachable in exactly the state it exists to fix.

The one control that matters is **`insets: ON | OFF`**, with the live readout beside it.

## What was measured, before building anything

On the Pixel 8 Pro, API 37, with round 2's APK unchanged:

| | keyboard down | keyboard up |
|---|---|---|
| `mInputShown` | false | **true** |
| app window frame | `[0,0][1344,2992]` | **`[0,0][1344,2992]`** |
| `mImeHeight` | 0 | **1145** |

**The app is never told.** The window does not resize, and no inset reaches it. The window carries
`sim={adjust=resize}` — which since API 35 is inert, because the same window is
`EDGE_TO_EDGE_ENFORCED` and an edge-to-edge window is expected to read the IME inset itself rather
than be resized under it.

This is the unrecorded other half of [`AGENTS.md`](../../AGENTS.md) client-stack rule 8. That rule
says winit's Android backend handles only motion and key events and has no IME path, and records the
consequence for **composed text**. The same gap costs the **insets**: `set_ime_allowed` reaches
`show_soft_input`, so the keyboard goes up, and nothing comes back.

### The consequence is worse than occlusion

egui is handed a screen rect 1145px taller than the one the user can see. The content fits inside
it, so the `ScrollArea` has **no scroll range at all** — verified by swiping, which produced a
byte-identical frame. The covered band is **unreachable, not scrolled off**.

Measured cost of typing, in dp at this density:

| | height |
|---|---|
| display | 997 |
| usable, keyboard down (bars 151px top, 72px bottom) | 923 |
| **usable, keyboard up** | **565** |

**39% of the screen, with no notification, no reflow and nothing to scroll.**

## What was built

Two things, so the question is judged against the spec as it stands today rather than as it stood in
July.

### 1. `src/insets.rs` — the app asks the platform itself

`WindowInsets.getInsets(Type.ime() | Type.systemBars())` over JNI, re-read every frame, applied as a
reserved band via `Panel::bottom(...).exact_size(...)`. `insets: ON` is what the app would do if it
read them; `OFF` is what it does today. Both are on the handset at once, on one tap, so the
difference is judged rather than described.

With it on: the readout tracks the keyboard live (`kbd 1217px`), the status and gesture bars stop
being drawn under, `CentralPanel` gets a viewport matching what the user can see, and **the
`ScrollArea` gains a real scroll range** — the form below the fold becomes reachable by scrolling.

Two traps, both worth carrying into the real client:

- **`ndk_context`'s context is the `Application`, not the `Activity`.** `getWindow()` is an
  `Activity` method, so the first build threw `NoSuchMethodError` and **aborted the process**. The
  prototype's existing JNI (`android.rs`) gave no warning, because `getFilesDir()` is a `Context`
  method that `Application` has too. The activity handle is `AndroidApp::activity_as_ptr()`, stashed
  by `android_main`.
- **A failed JNI lookup leaves a Java exception pending, and the *next* call aborts.** So the crash
  presents as `SIGABRT` inside an unrelated later frame. `?` on a JNI result is only safe if
  something clears up behind it.

### 2. Variant D brought up to ADR-0021

Round 2 predates it, and three of its changes land on the surface being judged:

- **§7 — the Save button is gone.** Saving is automatic, per field. That control used to sit at the
  very bottom of the editor, so keeping it would have meant judging a keyboard against a layout the
  spec no longer describes.
- **§8 — *New note* replaces it**, carrying kind and deck forward. Its accelerator is a desktop
  modifier chord, so on a phone **this button is the only way to take the action** — and it is at
  the bottom, competing with the keyboard.
- **§9 — a deck dropdown beside the kind dropdown.** A second picker opening into the space the
  keyboard wants.

## Findings

1. **The split view survives horizontally, and that was not the risk.** At 448pt the two panes fit
   and read fine — the desktop preset is legible on the handset unprompted. What fails is
   **vertical**, and no width rule addresses it.
2. **What gives way is whatever the form pane puts last** — and today that is exactly what
   [ADR-0012 §5](../../docs/adr/0012-the-note-authoring-experience.md) and
   [ADR-0018 §4](../../docs/adr/0018-the-card-pane-ordering.md) made **primary on both platforms**:
   the ambient destructive-edit warning, which renders after the last field. §3's numbered blank
   list has the same fate for a cloze note, sitting under the field it describes.
3. **The sharpest one: with the keyboard up, what remains on screen is a counter.** Staged live —
   `basic + reverse` → `basic`, one card goes dormant, focus the front field. Visible: `· 1 dormant`
   on the toggle row. Not visible: the warning block, its copy, and Undo.

   ADR-0018 §4 re-read round 1 and concluded **a count is not a warning** — that variant B carried
   two defects at once and the repair had been credited to position alone. The handset reproduces
   that exact failure, from the opposite direction: not because a counter was chosen over a warning,
   but because the keyboard leaves only the counter standing.

   **This holds with `insets: ON` too.** Reserving the band makes the warning *reachable*; it does
   not make it *visible at the moment of the edit*. So insets are necessary and not sufficient.
4. **ADR-0018 §4's own argument already predicted this, and stopped one step short.** It rejected
   position as the mechanism because "ordinal position cannot guarantee the card-pane entry is on
   screen — blank 18 of 20 lands below the fold on desktop too", and moved the warning to the form
   pane on the ground that the form pane is the one always on screen. On a handset with the keyboard
   up, **the form pane is not always on screen either** — only its first ~565dp is.
5. **The app draws under the status bar and the gesture bar today**, keyboard or no keyboard. Round
   2 never saw it because it ran on a desktop. Independent of #67 and cheap to fix with the same
   inset read.

## What is waiting on a live judgement

Both are behavioural, so neither is the visual design pass's:

- **Does the client read IME insets itself?** It is ~90 lines of JNI plus a reserved band, it fixes
  the status/gesture bars for free, and it is the only thing that makes the bottom of the editor
  reachable at all while typing. It is also a new platform seam in a codebase that keeps
  `leitner-store::platform` to exactly two functions
  ([ADR-0016 §6](../../docs/adr/0016-backup-and-restore.md)) — so where it lives is a real question,
  not a detail.
- **Given finding 3, does §5's warning stay at the bottom of the form pane?** Insets alone leave the
  moment of the edit showing a counter. The options visible from here — pin the warning to the field
  it concerns, or hoist it above the fields, or accept the counter and say so — are a decision this
  prototype exists to inform, not to take. Note that "pin it" is close to the pinned header
  indicator ADR-0018 §4 forbids, and the difference needs stating rather than assuming.

## Capture

Throwaway. Lands on a branch and is tagged, never merged to `main` — same convention as
`prototypes/issue-8`, `issue-11`, `issue-20` and `issue-28`. Only the validated decision goes to
`main`, as an ADR amendment.
