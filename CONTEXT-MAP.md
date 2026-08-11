# Context Map

Start here. This file says what the codebase is made of, which vocabulary applies where, and — most
importantly — **which parts of `docs/adr/` bind the code you are about to touch**, so you do not
have to read 2,600 lines to find the 300 that matter.

Laid out by [ADR-0009](./docs/adr/0009-crate-and-workspace-layout.md).

## Read in this order

Ordered by what breaks silently if you skip it.

1. **[`AGENTS.md`](./AGENTS.md)** — the rules that fail *without an error message*. About forty
   lines, and every one of them is a bug that neither the compiler nor the tests will catch. Read it
   even if you are making a one-line change.
2. **This file** — the crates, the contexts, and the ADR index below.
3. **The `CONTEXT.md` for the area you are touching** — vocabulary, so your code and your commit
   message use the words the ADRs use.
4. **Only the ADR sections the index says bind your context.**

**`docs/research/` is not in this list.** It is the evidence trail for *reopening* a decision, not
reading for implementing one — its findings are already distilled into the ADRs that cite it. Start
there and you will spend your budget before writing a line.

## Crates

Six crates. Two of the boundaries are forced by the toolchain; see ADR-0009 §1.

| Crate | Path | What it is |
|---|---|---|
| `cairn-core` | [`crates/core/`](./crates/core) | The domain, entire and pure. **One dependency — `fsrs` — and nothing else** ([ADR-0027](./docs/adr/0027-the-scheduler-dependency.md)). |
| `cairn-store` | [`crates/store/`](./crates/store) | SQLite persistence and the two-directory platform seam. |
| `cairn-export` | [`crates/export/`](./crates/export) | The `.cdeck` and `.ccoll` containers, the import policy, and the user-files platform seam. Holds the zip dependency. |
| `cairn-sync` | [`crates/sync/`](./crates/sync) | Publishing to the remote and reading it back. Holds the network dependencies. |
| `cairn-app` | [`crates/app/`](./crates/app) | The egui application, the bidi helper, the window's inset seam, the Android entry point. |
| `cairn-desktop` | [`crates/desktop/`](./crates/desktop) | A twenty-line shim. Forced by `cargo-apk`; keep it empty. |

**And one directory that is not a crate.** [`vendor/egui-winit`](./vendor/PATCH.md) is third-party
source the repository carries: a verbatim copy of the published `egui-winit` 0.35.0 with **one block**
behind `#[cfg(not(target_os = "android"))]`, reached only through `[patch.crates-io]`. It is excluded
from the workspace, it is not our code, and it is **outside client-stack rule 3** — the
`#[cfg(target_os)]` a reader will find there is the patch, not a defect
([ADR-0026](./docs/adr/0026-the-per-tap-keyboard-re-pop.md)). Bumping the egui family is therefore no
longer only a version change: `scripts/verify-vendor.sh` runs the recursive diff and the block-shape
check, and a release that restructures the block is **re-judged, not re-applied**.

Two rules about this table that are easy to break:

- **Nothing is added to `cairn-core`'s `[dependencies]` casually.** It holds exactly one entry,
  `fsrs`, admitted by [ADR-0027](./docs/adr/0027-the-scheduler-dependency.md) because ADR-0001 §1
  names it and `scheduling` is a module here. What makes `cargo test -p cairn-core` need no
  database, no window and no handset is ADR-0027 §2's five-part test, not the count — and `rand`,
  `serde`, `rayon` and `ndarray` sitting in the lockfile transitively are **not** available to our
  code.
- **`cairn-app` has no `src/main.rs`, and `cairn-desktop` has no logic.** `cargo-apk` panics
  after signing when one crate has both a cdylib and a bin, so the split is load-bearing, and code
  put in `desktop` is never compiled for Android and never runs on the handset.

## Contexts

| Context | `CONTEXT.md` | What it owns |
|---|---|---|
| Content | [`crates/core/src/content/`](./crates/core/src/content/CONTEXT.md) | Notes, cards, kinds, fields, decks, tags |
| Log | [`crates/core/src/log/`](./crates/core/src/log/CONTEXT.md) | Rows, writer ids, sequences, day scale, stamps, interchange |
| Scheduling | [`crates/core/src/scheduling/`](./crates/core/src/scheduling/CONTEXT.md) | FSRS arithmetic, grades, memory state, boxes |
| Replay | [`crates/core/src/replay/`](./crates/core/src/replay/CONTEXT.md) | The join: what exists, what state it is in, what is due |
| Store | [`crates/store/src/`](./crates/store/src/CONTEXT.md) | The two databases, device identity, its platform seam |
| Export | [`crates/export/src/`](./crates/export/src/CONTEXT.md) | Deck files, collection archives, profiles, revisions, import policy |
| Sync | [`crates/sync/src/`](./crates/sync/src/CONTEXT.md) | The remote, namespaces, objects, roll-up, enrolment |
| UI | [`crates/app/src/`](./crates/app/src/CONTEXT.md) | Screens, the session, the bidi helper, the inset seam and the reserved band |

### How they relate

```
content ──┬──> scheduling ──┐
          │                 ├──> replay ──> store, ui
          └──> log ─────────┤
                            ├──> export ──> ui
                            └──> sync ────> ui
```

- **`content` is the base.** A log row carries a `CardRef`, and scheduler fuzz is seeded from
  `CardRef`'s 18-byte encoding — so content depends on nothing, and everything else may depend on it.
- **`scheduling` does not depend on `log`.** It takes grades and day numbers as values, which is what
  lets FSRS arithmetic be tested against a hand-written list with no rows and no merge.
- **`replay` is the join, and the deep module of the system.** Behind a small interface — what is
  due, what box is this card in, record this grade — sit the log, the content, the scheduler and the
  cache.
- **`export` is a crate, not a module, for the same reason `replay` is a context**: it spans content
  and — now that [ADR-0016](./docs/adr/0016-backup-and-restore.md) has specified the `collection`
  profile — the log, so it belongs inside neither, and it holds the zip dependency that
  `cairn-core` cannot. It is also **the second crate with a platform seam**: ADR-0016 §5 puts
  put/get/list for user-visible files here — widened to **put/get/list/hand_off** by
  [ADR-0023 §1](./docs/adr/0023-sending-a-written-file.md) — three `#[cfg]` arms with a
  `compile_error!` third, so that `cairn-store::platform` stays at exactly two functions. **The
  seam rule is per crate**, and the *number* of operations is not the invariant.
- **`ui` holds the third platform seam, and it is one function.** An inset is a fact about the window
  this crate is drawing into, so routing it through the store would make the store answer a question
  about layout ([ADR-0025 §2](./docs/adr/0025-the-authoring-screen-under-a-soft-keyboard.md)). Its
  return type distinguishes *no soft keyboard on this platform* from *the keyboard is down* — zero
  for both, as §2 first wrote it, makes every gate on "is it down" permanently true off Android
  ([ADR-0026 §5](./docs/adr/0026-the-per-tap-keyboard-re-pop.md)). `android_main` lives in that arm
  because the activity handle originates there and `ndk_context` holds the `Application`, not the
  `Activity`; keeping them together is what leaves the seam one function wide.
- **`sync` is a crate for the reason this file predicted**: it needs HTTP, TLS and OAuth, which
  `cairn-core` cannot hold. [ADR-0013](./docs/adr/0013-the-sync-transport.md) realised the
  anticipated sixth crate rather than overturning anything. It depends on `log` and knows nothing
  about cards — the remote is a **rendezvous point, not a system of record**, so deleting it costs
  one republish and no data.

## Which ADRs bind which context

Read the ADR sections in your row. Read the whole ADR only if you are changing the decision.

| Context | Binding ADRs | Also bound by |
|---|---|---|
| `content` | [0002](./docs/adr/0002-the-card-model.md), [0005](./docs/adr/0005-the-deck-model.md), [0017](./docs/adr/0017-card-slots.md) | 0011 §7, 0012 §3, 0018 §3, 0021 §3 |
| `log` | [0004](./docs/adr/0004-the-review-event-log.md) | 0002 §7, 0001 §6, 0010 §5, 0011 §5, 0013 §12, 0014 §6 |
| `scheduling` | [0001](./docs/adr/0001-scheduling-algorithm-and-grade-scale.md), [0027](./docs/adr/0027-the-scheduler-dependency.md) | 0004 §4, 0004 §5, 0014 §6 |
| `replay` | *none of its own* | 0001 §7, 0002 §7, 0004 §9, 0007 §2, 0010 §2, 0011 §8, 0012 §5, 0017 §1, 0017 §5, 0018 §2 |
| `store` | [0007](./docs/adr/0007-the-local-store.md) | 0004 §11, 0003 §5, 0013 §9, 0016 §3, 0016 §7, 0019 §6, 0020 §3, 0020 §4, 0028 §5 |
| `export` | [0008](./docs/adr/0008-the-deck-export-format.md), [0016](./docs/adr/0016-backup-and-restore.md), [0022](./docs/adr/0022-the-import-preview-and-export-report.md), [0023](./docs/adr/0023-sending-a-written-file.md), [0024](./docs/adr/0024-identifying-a-written-file.md) | 0005, 0002 §9, 0004 §11, 0011 §7, 0020 §4, 0021 §3, 0028 §3 §3a |
| `sync` | [0013](./docs/adr/0013-the-sync-transport.md) | 0004 §2, 0004 §7, 0004 §10, 0007, 0014 §7, 0015 §2, 0015 §4, 0016 §10, 0019 §4, 0019 §6, 0020 §5, 0020 §6, 0020 §7 |
| `ui` | [0003](./docs/adr/0003-client-stack.md), [0006](./docs/adr/0006-the-review-session-experience.md), [0010](./docs/adr/0010-leeches.md), [0011](./docs/adr/0011-new-card-rate-and-daily-limits.md), [0012](./docs/adr/0012-the-note-authoring-experience.md), [0014](./docs/adr/0014-when-parameter-optimisation-runs.md), [0015](./docs/adr/0015-the-sync-experience.md), [0018](./docs/adr/0018-the-card-pane-ordering.md), [0019](./docs/adr/0019-naming-the-account-at-enrolment.md), [0021](./docs/adr/0021-note-ordering-saving-and-the-note-list.md), [0022](./docs/adr/0022-the-import-preview-and-export-report.md), [0025](./docs/adr/0025-the-authoring-screen-under-a-soft-keyboard.md), [0026](./docs/adr/0026-the-per-tap-keyboard-re-pop.md), [0029](./docs/adr/0029-editing-a-note-from-the-review-screen.md), [0030](./docs/adr/0030-the-first-finish-pass-decisions.md), [0031](./docs/adr/0031-the-page-frame.md), [0032](./docs/adr/0032-the-type-scale-and-the-rhythm.md) | 0002 §4, 0016 §5, 0016 §6, 0016 §11, 0016 §12, 0017 §5, 0017 §6, 0020 §7, 0023 §5, 0023 §6, 0024 §3, 0028 §1 §2 |
| *the workspace itself* | [0009](./docs/adr/0009-crate-and-workspace-layout.md), [0027](./docs/adr/0027-the-scheduler-dependency.md), [0028](./docs/adr/0028-the-application-is-named-cairn.md) | 0013 §11, 0013 §12, 0015 §15, 0016 §5 |

**`replay` having no ADR of its own is why it is a context.** Its rules were each written for another
purpose and sit scattered across four documents; its `CONTEXT.md` is the only place they appear as
one mechanism. If you are touching replay, read that file before the ADRs.

**[0020](./docs/adr/0020-protection-at-rest.md) binds four contexts and owns none**, which is why it
appears only in the right-hand column. It decides what is protected at rest across every artifact at
once — the store, the archive, the credential and the published log — and answering for one of them
alone is what three earlier ADRs each declined to do.

**[0022](./docs/adr/0022-the-import-preview-and-export-report.md) is the only ADR binding two
contexts equally**, and the split is worth knowing before you read it. Its `export` half is
mechanism — the **gate/describe** two-stage read, and the rule that an import plan is *derived on
every read, never cached*. Its `ui` half is what four surfaces say: the import preview, the export
screen, the export report and the file list. Change either half without the other and the surfaces
state numbers the mechanism cannot produce.

**[0025](./docs/adr/0025-the-authoring-screen-under-a-soft-keyboard.md) is the only ADR about a
surface the app cannot see.** On Android nothing below the app reports the soft keyboard — the window
is enforced edge-to-edge, so the old resize mode is inert, and winit reports no insets — so the UI
layer is handed a viewport taller than the visible one. Read it before touching any screen with a text
field: the cost is not occlusion but **unreachability**, since the scroll area sizes itself to the
viewport it was given and the covered band has no scroll range. It also carries the guards without
which the keyboard oscillates, and it is why [0012 §5](./docs/adr/0012-the-note-authoring-experience.md)'s
destructive-edit warning sits *above* the fields rather than after the last one. **Read
[0026](./docs/adr/0026-the-per-tap-keyboard-re-pop.md) with it, never instead of it** — 0026 amends its
seam return type and turns its two guards into three.

**[0026](./docs/adr/0026-the-per-tap-keyboard-re-pop.md) is the only ADR about a dependency we do not
take as published**, and it is why a bump of the client stack is not just a version change. As shipped,
every tap into a text field on Android dismisses and reopens the soft keyboard, because the UI toolkit
interrupts IME composition on any pointer interaction and the layer below implements that as
hide-then-show — on a platform whose backend has no IME path, so there was never a composition to
interrupt. No fix exists above the dependency: the interrupt flag has a public setter and no public
clearer. Read it before bumping `egui`, before touching the shared text-field wrapper, and before
assuming the inset seam reports zero off Android.

**[0024](./docs/adr/0024-identifying-a-written-file.md) is an Android-only correction, and it amends
four accepted ADRs rather than deciding much of its own.** Read it before touching anything that
identifies a file by its name: on that platform the extension reaches no intent filter and the media
type is discarded, so the `mimetype` member inside the archive is the sole authority for what a file
is. It also narrows [0016 §5](./docs/adr/0016-backup-and-restore.md)'s file list to *files this
application wrote* — the single fact most likely to be assumed away by someone implementing the
import surfaces.

**[0027](./docs/adr/0027-the-scheduler-dependency.md) is the first ADR written after the map closed,
and it exists because two accepted ADRs contradicted each other.** ADR-0001 §1 requires the scheduler
to be FSRS-6 *via the `fsrs` crate*; ADR-0009 §3 puts `scheduling` inside `cairn-core`; ADR-0009 §2
said that crate's dependency list was empty *"permanently"*. Read it before adding **anything** to
`cairn-core`, and before reading a crate's presence in `Cargo.lock` as permission to use it — its
§3 is entirely about what arrives transitively and stays off-limits.

**[0028](./docs/adr/0028-the-application-is-named-cairn.md) renamed the application, and every ADR
before it still says the old name in its prose.** That is deliberate — an ADR records a decision as it
was made, and `docs/research/` least of all may be rewritten, since those files record what was
measured under the names it was measured under. **So read 0028 before concluding that a document
saying `leitner` or `.ldeck` is stale.** Its §4 draws the line the rest of this repository follows:
what is frozen is the **claim a sentence makes** — a measured filename, a decided extension — never
the **address it cites**, which is why every issue URL was re-pointed to `amin-bf/cairn` and no
document depends on the rename redirect.

**[0029](./docs/adr/0029-editing-a-note-from-the-review-screen.md) is the only ADR that makes the
specification smaller**, and that is the shape to read it for. It narrows ADR-0021 §6's *edit this
note* to the **revealed** state and, in doing so, **retires** ADR-0021 §6's *"entering the editor
counts as a reveal"* — a rule whose sole customer was the state being removed. Read it before
touching the review screen, and note what it does **not** do: ADR-0006 §4's guarantee that grading
cannot precede seeing the answer is not weakened but *strengthened*, because it now holds by there
being no route into the editor rather than by a clause about what the editor's side-effect must be.
It is also the first UI decision on this map judged against **wireframes rather than a build**, which
its own Consequences record.

**[0030](./docs/adr/0030-the-first-finish-pass-decisions.md) is where colour enters the app**, and it
is the first of the finish pass [0006 §10](./docs/adr/0006-the-review-session-experience.md) opened.
It answers four questions — the palette lives at **one naming site** (a `theme` module; a colour
literal anywhere else is the defect, and every screen keeps reading the ambient visuals), **dark is
pinned** and system-following is dropped *deliberately* (install the palette **and** disable
following, or an OS theme change silently restores stock egui), a **7:1 contrast floor** binds text
against its surface and not the decorative non-text pairs, and §6's box badge is **lower-case in the
small-text face**. Read it before implementing the palette (#115) or adding any colour to a screen.
It also records, as accepted rather than overlooked, that three of the palette's four accents — warn,
error, link — have no call site until the notice channel and links exist.

**[0031](./docs/adr/0031-the-page-frame.md) is the page frame, and it is 0030's rule one layer down**:
the margin (28) and the measure (640) live in a `frame` module and screens ask for a frame rather
than a number, because a hand-rolled `min(available, 640.0)` on some screen drifts the layout exactly
the way a stray `Color32::from_rgb` drifts the palette. **One arrangement, centred, on every
destination** — at 1280 half the window is empty by design. Read it before laying out any screen, and
read §4 before touching the **editor**: its two panes now genuinely sit side by side above a
**900px window** threshold, which supersedes 0025 §5's *"where both fit they show together"* and
finally makes 0012 §1's description true. The threshold measures the **window**, never a column —
under a frame `ui.available_width()` is the column, and a 640 column is not `>= 640`, so the old test
would have put every desktop into the phone's toggle with nothing failing.

**[0032](./docs/adr/0032-the-type-scale-and-the-rhythm.md) is the type scale and the rhythm, and it is
0030's rule a third time**: four sizes — display **40**, heading **20**, body **15**, small **12** —
in a `typography` module, with control text an **alias** of body rather than a fifth constant, and
`display` reaching exactly one caller, the card face. The rhythm is the half that breaks the pattern
on purpose: `item_spacing` is **zeroed** and every gap is a whole unit of **8** through
`spacing::gap`, because an ambient gap cannot express a gap smaller than itself and would make every
site name a number that is not the gap it wants. So **the number in the source is the number on the
screen** — which is what made 0031's editor stop overrunning its own frame by 8px. Read it before
setting any text size or writing any `add_space`, and note two traps it records: a `TextStyle::Name`
that was never installed **panics** rather than falling back, and both modules install into *every*
theme slot, since `Style` is per-theme exactly like `Visuals`. §3 exempts the page margin from the
grid deliberately; §5 keeps the 7:1 contrast floor while retiring the 9px argument 0030 §3 gave for it.

0028 also carries the one item in that change that cannot be taken back, the Android package id. Its
extension rename **is** discharged: `.cdeck` and `.ccoll` were measured on the handset at API 37 and
reach our filters exactly as `.ldeck` did
([evidence](./docs/research/extension-rename-reachability/README.md)).

**If you write a new ADR, add it to this table.** An ADR that is not in this index is invisible to
the agent it was written for.

## Testing

- **`cargo test -p cairn-core`** needs no database, no window and no handset. Most of the
  specification is verifiable here, and that is deliberate.
- **Time and identity are values, never injected traits.** Replay needs no clock at all — day
  numbers are frozen on the row at write time and fuzz is seeded from card identity. The two places
  that need "now" take it as a parameter.
- **There is no fake store.** Store tests open a real SQLite database in a temp directory, because
  the design *is* WAL, `BEGIN IMMEDIATE`, `ATTACH` and `INSERT OR IGNORE`.
- **Store tests run on desktop.** On Android the store depends on the activity existing, so
  `cairn-store` is not independently runnable there.
- **The highest-value test in the repository** is that any interleaving of two devices' rows replays
  to the same state. It needs no sync implementation and no second device.

## Building

See [`README.md`](./README.md) for prerequisites. In short:

```sh
cargo run -p cairn-desktop            # desktop
cargo test --workspace                  # everything testable without hardware
scripts/verify-vendor.sh                # vendor/egui-winit: verbatim plus exactly one change

source scripts/android-env.sh           # required before ANY Android command
cd crates/app && cargo apk build        # APK: a manifest, one .so and res/ (the icon)
```

Verify UI judgements on the **real handset** — the emulator is x86_64 and the Pixel 8 Pro is
arm64-v8a only.
