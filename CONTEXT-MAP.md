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

Eight crates. Two of the boundaries are forced by the toolchain; see ADR-0009 §1. `cairn-ops` and
`cairn-ffi` are the seventh and eighth ([ADR-0046](./docs/adr/0046-the-line-between-kotlin-and-rust.md)
§2). The Kotlin client that loads `cairn-ffi` is not a crate: it is the Gradle project in
[`android/`](./android), which builds the Rust core itself
([`docs/environment/android-toolchain.md`](./docs/environment/android-toolchain.md)).

| Crate | Path | What it is |
|---|---|---|
| `cairn-core` | [`crates/core/`](./crates/core) | The domain, entire and pure. **One dependency — `fsrs` — and nothing else** ([ADR-0027](./docs/adr/0027-the-scheduler-dependency.md)). |
| `cairn-store` | [`crates/store/`](./crates/store) | SQLite persistence and the two-directory platform seam. |
| `cairn-export` | [`crates/export/`](./crates/export) | The `.cdeck` and `.ccoll` containers, the import policy, and the user-files platform seam. Holds the zip dependency. |
| `cairn-sync` | [`crates/sync/`](./crates/sync) | Publishing to the remote and reading it back. Holds the network dependencies. |
| `cairn-ops` | [`crates/ops/`](./crates/ops) | The application's operations and the state they need, in plain Rust: the session and the sitting, cards, decks, the editor's settling, the note list with its search folding and the note, deck and kind writes, the leech list with suspend and unsuspend (suspended leeches counted once), import planning with the gate each plan is owed, optimisation, the fixture bench, the restricted Markdown, and Review as facts (the entrance's five states and counts, the sitting, the card shown as styled runs with its direction, box badge and interval previews). Depends on `cairn-core`, `cairn-store` and `cairn-export` (and `unicode-bidi`, for the direction fact both renderers share) and on **no renderer** (`egui`, `eframe`, `winit`) and no binding; time is passed in, never read in the sitting or the editor. The egui app and the binding call it (optimisation's nudge, start and finish among the rest). |
| `cairn-ffi` | [`crates/ffi/`](./crates/ffi) | The Kotlin binding: **the only crate that depends on uniffi** (`=0.32.2`), `cdylib` + `lib`, library `cairn_ffi`, generated Kotlin in `dev.cairn.ffi`. **Thin**: an exported collection handle over `cairn-ops` behind a `Mutex` (Review's state, the leech list and its two writes, a sitting handle, the optimisation nudge and an optimise handle polled for phase, cancel and finish, the fixture bench listed with `fixtures()`, installed by name and saying what it reached, and its deck files with `bench_files()`, an inbound file handle (`open_file`, whose `preview()` re-derives the plan on every call; its `apply()` writes it and says which deck the note list lands on), and `reset()` — all in every build; only the Kotlin *Development* block that reaches them is gated by `CAIRN_DEVELOPMENT=1`), the mirror records and enums, and one error crossing as `CairnException` (no field named `message`). Exports no platform function and calls none ([ADR-0046](./docs/adr/0046-the-line-between-kotlin-and-rust.md) §5). Its `uniffi-bindgen` binary sits behind the host-only `cli` feature, which the Android build leaves off; the release profile never gains `panic = "abort"` (§7). |
| `cairn-app` | [`crates/app/`](./crates/app) | The egui application, **desktop only** since the egui Android build retired ([ADR-0045](./docs/adr/0045-the-native-android-client.md) §8), and the bidi helper — and the sentence helpers (`sentences`) and `sync`, which stay with it ([ADR-0046](./docs/adr/0046-the-line-between-kotlin-and-rust.md) §2, §4). |
| `cairn-desktop` | [`crates/desktop/`](./crates/desktop) | The desktop binaries, `cairn` and `cairn-fixture`: twenty-line shims over `cairn-app` and `cairn-ops`. Keep them empty. |

Two rules about this table that are easy to break:

- **Nothing is added to `cairn-core`'s `[dependencies]` casually.** It holds exactly one entry,
  `fsrs`, admitted by [ADR-0027](./docs/adr/0027-the-scheduler-dependency.md) because ADR-0001 §1
  names it and `scheduling` is a module here. What makes `cargo test -p cairn-core` need no
  database, no window and no handset is ADR-0027 §2's five-part test, not the count — and `rand`,
  `serde`, `rayon` and `ndarray` sitting in the lockfile transitively are **not** available to our
  code.
- **`cairn-app` has no `src/main.rs`, and `cairn-desktop` has no logic.** The split was forced by
  the retired egui Android build's packager and stays because the scripts, captures and docs run
  `-p cairn-desktop`; code put in `desktop` is never reached by a test.

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
| UI | [`crates/app/src/`](./crates/app/src/CONTEXT.md) | The desktop's screens, the session, the bidi helper |

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
- **`ui` holds no platform seam any more.** Its one-function inset seam
  ([ADR-0025 §2](./docs/adr/0025-the-authoring-screen-under-a-soft-keyboard.md)) and `android_main`
  went with the egui Android build ([ADR-0045](./docs/adr/0045-the-native-android-client.md) §8); the
  Kotlin app reads its own insets. On Android the two remaining seams' arms refuse: the Kotlin app
  passes the directories in and moves the files itself.
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
| `store` | [0007](./docs/adr/0007-the-local-store.md) | 0004 §11, 0003 §5, 0013 §9, 0016 §3, 0016 §7, 0019 §6, 0020 §3, 0020 §4, 0028 §5, 0036 §3 |
| `export` | [0008](./docs/adr/0008-the-deck-export-format.md), [0016](./docs/adr/0016-backup-and-restore.md), [0022](./docs/adr/0022-the-import-preview-and-export-report.md), [0023](./docs/adr/0023-sending-a-written-file.md), [0024](./docs/adr/0024-identifying-a-written-file.md), [0041 §2 §5](./docs/adr/0041-the-file-surface.md) | 0005, 0002 §9, 0004 §11, 0011 §7, 0020 §4, 0021 §3, 0028 §3 §3a |
| `sync` | [0013](./docs/adr/0013-the-sync-transport.md) | 0004 §2, 0004 §7, 0004 §10, 0007, 0014 §7, 0015 §2, 0015 §4, 0016 §10, 0019 §4, 0019 §6, 0020 §5, 0020 §6, 0020 §7 |
| `ui` | [0003](./docs/adr/0003-client-stack.md), [0006](./docs/adr/0006-the-review-session-experience.md), [0010](./docs/adr/0010-leeches.md), [0011](./docs/adr/0011-new-card-rate-and-daily-limits.md), [0012](./docs/adr/0012-the-note-authoring-experience.md), [0014](./docs/adr/0014-when-parameter-optimisation-runs.md), [0015](./docs/adr/0015-the-sync-experience.md), [0018](./docs/adr/0018-the-card-pane-ordering.md), [0019](./docs/adr/0019-naming-the-account-at-enrolment.md), [0021](./docs/adr/0021-note-ordering-saving-and-the-note-list.md), [0022](./docs/adr/0022-the-import-preview-and-export-report.md), [0025](./docs/adr/0025-the-authoring-screen-under-a-soft-keyboard.md), [0026](./docs/adr/0026-the-per-tap-keyboard-re-pop.md), [0029](./docs/adr/0029-editing-a-note-from-the-review-screen.md), [0030](./docs/adr/0030-the-first-finish-pass-decisions.md), [0031](./docs/adr/0031-the-page-frame.md), [0032](./docs/adr/0032-the-type-scale-and-the-rhythm.md), [0033](./docs/adr/0033-the-card.md), [0034](./docs/adr/0034-the-controls.md), [0035](./docs/adr/0035-the-vertical-anchor.md), [0036](./docs/adr/0036-the-light-palette.md), [0037](./docs/adr/0037-motion-and-elevation.md), [0038](./docs/adr/0038-the-mark-and-the-icon-rule.md), [0039](./docs/adr/0039-the-list-row.md), [0040](./docs/adr/0040-the-note-editor.md), [0041](./docs/adr/0041-the-file-surface.md), [0042](./docs/adr/0042-the-settings-screen.md), [0043](./docs/adr/0043-the-leech-screen.md), [0044](./docs/adr/0044-the-ambient-hit-target.md), [0049](./docs/adr/0049-text-search-folds-persian-letter-variants.md) | 0002 §4, 0016 §5, 0016 §6, 0016 §11, 0016 §12, 0017 §5, 0017 §6, 0020 §7, 0023 §5, 0023 §6, 0024 §3, 0028 §1 §2 |
| *the Android client* (Kotlin) | [0045](./docs/adr/0045-the-native-android-client.md), [0046](./docs/adr/0046-the-line-between-kotlin-and-rust.md), [0047](./docs/adr/0047-the-file-surface-on-android.md), [0048](./docs/adr/0048-notes-and-the-editor-on-android.md), [0050](./docs/adr/0050-the-leech-screen-on-android.md), [0051](./docs/adr/0051-settings-on-android.md), [0052](./docs/adr/0052-the-file-surface-on-android-is-import-only.md), [0053](./docs/adr/0053-backup-and-restore-on-android.md) | 0007 §6, 0016 §7, 0021 §7, 0022, 0023, 0024, 0031–0044 |
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

**[0025](./docs/adr/0025-the-authoring-screen-under-a-soft-keyboard.md) and
[0026](./docs/adr/0026-the-per-tap-keyboard-re-pop.md) describe the egui Android build, retired by
[0045 §8](./docs/adr/0045-the-native-android-client.md).** 0025 is why the egui build carried an inset
seam and a reserved band: on Android winit reported no soft keyboard, so the covered band had no scroll
range. 0026 is why it carried a vendored `egui-winit`: every tap into a text field dismissed and
reopened the keyboard. Both workarounds went with the build; read them as history, and as the
reason [0012 §5](./docs/adr/0012-the-note-authoring-experience.md)'s destructive-edit warning sits
*above* the fields rather than after the last one.

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
read §4 before touching the **editor**: its two panes genuinely sit side by side, which finally makes
0012 §1's description true. §4's **900px window** threshold is **gone** — [0040 §1](./docs/adr/0040-the-note-editor.md)
folds the panes where a soft keyboard exists and measures no width, which since the egui Android build
retired means the desktop never folds — but §4's trap is still worth
knowing: under a frame `ui.available_width()` is the column, and a 640 column is not `>= 640`, so a
width test there would put every desktop into the phone's toggle with nothing failing.

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

**[0033](./docs/adr/0033-the-card.md) is the card, and it is 0030's rule a fourth time — plus the case
that rule cannot reach**: a card is **one** object with two faces divided by a hairline, drawn as a
**well** (`STONE_0`, a `STONE_4` edge, an 8px corner) with the box badge **inside** it, all of it in a
`surface` module that both review and the editor's card pane call. Read it before drawing note content
anywhere. Four things it decides are easy to undo by accident. The page is now `panel_fill` via
`clear_color`, and **without that override every screen sits on eframe's `#080808`**, below every rung
of the ramp, where a well measures 1.07:1 and inverts into a raised surface — the rule about naming
colours once cannot catch a colour the *renderer* supplies. The card face **steps down** the scale to
fit and **stops at body**, so 0032 §1's display tier is the card's maximum rather than its size. The
badge takes the corner reading does **not** start at, mirrored by the **prompt's** script, because
top-right is a footnote in Latin and the first thing seen in Persian. And §3 **binds
[#134](https://github.com/amin-bf/cairn/issues/134)**: the controls must end up quieter than the card,
since filled buttons outweigh every candidate card and making the card recede without them makes it
worse. It also lands 0030 §4's lower-case `box 3`, which that ADR recorded as shipped and which never
was.

**[0034](./docs/adr/0034-the-controls.md) is what a control is *made of*, and the answer is a role
rather than a treatment**: `faint_bg_color` for an ordinary control (quieter than the card, which
discharges 0033 §3), the old heavier rung kept as `primary` for **the one way forward on a screen
with no card**, and one frameless `text_action` beside a primary. Ask `controls` for a role, never a
`Button` with a fill. Read it before adding any control anywhere. Applying one flat treatment
everywhere satisfies §3 on the review screen and guts the screens with no card, where the only mass
on the page ends up reading as disabled — that is the trap, and it is why there are three weights and
not one. §4 also makes the 10-minute checkpoint **compact and above the card**, implementing 0006 §1
for the first time: it had been an `else if` that replaced the card since it was written, with nothing
failing because reaching that state needs ten real minutes no test waits for.

**[0035](./docs/adr/0035-the-vertical-anchor.md) is where the controls *sit*, and it is the frame's
first vertical number**: the last cluster on a screen ends **165px above the bottom of the page**
whenever there is room, *Edit note* rides directly under the card, and under a **thumb** the four
grades stack instead of taking 0034 §1's segmented row. Read it before placing anything vertically or
touching the grade row. 165 was measured — the cluster was dragged into place by thumb twice, at two
different heights, and its bottom edge landed within 7px both times — so the rule is a *line above the
bottom*, never a gap below the card. Two traps it records: **`Ui::available_height` returns zero
inside a `ScrollArea`** (the content Ui is sized to its content; use `frame::page_room`, which reads
the clip rect), and the card **must not move on reveal**, which is what struck both arrangements the
ticket originally proposed. The touch test is read off `platform::SoftKeyboard` rather than a new seam
or a `#[cfg]`, and the ADR states the rule as *touch* so a native client can implement it without
inheriting egui's way of noticing.

**[0036](./docs/adr/0036-the-light-palette.md) is the second palette, and it supersedes 0030 §2**:
the application offers **System / Light / Dark** on Settings, fills **both** theme slots, and stores
the choice **device-local** so it never syncs. Read it before touching any colour, and before adding
a colour rule of any kind — every one of them now has to hold in two themes, and the tests iterate
both because a rule checked in one says nothing about the other. **A light palette cannot be the
dark one lightened**, and this is the trap: the three weights use *both directions* from a page near
the bottom of the ramp, and above a light page there is only 1.13:1 of range in total, so a `primary`
lighter than the page does not exist at any hue. Light therefore puts all three fills *below* its
page and places them by the **pairwise gaps** dark delivers, not by 0033 §3's page-relative ratios —
which is the finding: those ratios are satisfiable on a light page while the separation they exist to
protect collapses to 1.02:1, with nothing failing. §3's invariant is restated as two claims about
what a screen can show. `theme::card_fill` and friends now take `&Visuals` and read the ambient slot;
returning a constant paints a dark card on a light page and nothing fails. The tightest reading pair
in the application is light's body-on-`primary` at **7.06:1**, pinned by figure.

0028 also carries the one item in that change that cannot be taken back, the Android package id. Its
extension rename **is** discharged: `.cdeck` and `.ccoll` were measured on the handset at API 37 and
reach our filters exactly as `.ldeck` did
([evidence](./docs/research/extension-rename-reachability/README.md)).

**[0037](./docs/adr/0037-motion-and-elevation.md) is motion and elevation, and it is the one ADR on
this map that overturns a rule rather than extending one.** Read it before adding any animation, any
shadow, or anything to the review card. Three things it settles and one it corrects. **Exactly one
surface floats** — only what the renderer already calls a popup, menu or window — and it is a *rise*
(1.12:1 above the page, the mirror of 0033's well), the *separator* rung as its edge, and a shadow at
**alpha 200 dark / 25 light**, which differ by 8× and buy the same 1.16:1; stock's 96/25 buy 1.083
and 1.156, so the value that looked like one gesture is two. **Motion is 240ms and `cubic_out`**, in
a `motion` module holding the duration ambiently in `Style::animation_time` and the easing as a
constant beside it, because the curve is which function you call and cannot be ambient. And **the
reveal is the answer half opening**, which narrows the *never where it is* rule to arrival: nothing
slides, scales, springs or grows **on arrival**, and a card turning over is one object opening. The
correction is 0033's: **the step-down must not fire at the reveal**, because the tier was chosen
against content that changed, so a wrapping cloze prompt was drawn at display before the tap and at
heading after it — visible only at 560, and in none of the pass's captures until a cloze fixture
existed.

**[0038](./docs/adr/0038-the-mark-and-the-icon-rule.md) is the mark and the icon rule. Read it before
drawing any picture.** **An icon is a glyph in a shipped face** — `Cairn Icons`, appended as a
fallback into every family, one code point (`fonts::MARK` at `U+E000`, private use so it shadows
nothing and nothing shadows it), generated from the Android launcher's own monochrome drawable by
`scripts/build-icon-face.py`, whose `--check` is what keeps *the mark is the launcher's four stones*
true. No call site selects a family, so an icon at `BODY` **is** `BODY` — and an icon's size is
therefore a **font** size, named in `typography` like every other. **The mark stands over *All caught
up.* at 104** (75px of stones — the ink is one cap height), in `weak_text_color()`, one construction
in both themes; 104 is not a fifth tier of the scale and is deliberately not installed into
`text_styles`. Two traps. **An icon standing alone is allocated its ink, not its line box** —
`ui.label` allocates the *family's* row height, which at 150 puts 109px of stones in a 172px row and
adds 53px nobody chose before the stated gap; `crate::icon` is the one to call, and `ui.label` stays
right for an icon **inline** with its word. And **the coverage test was an allowlist** while its own
comment described a denylist, so a private-use code point was silently skipped; it is a denylist now,
and the icon face joins `SPECIMENS` as an ordinary row. 0038 also carries two amendments: **0006 §5
holds within a renderer** (the gesture belongs to the platform, which the native clients need), and
**0035 §1 is a page rule** with a second call site — the leech entrance on the caught-up floor, whose
y is now window-dependent and reached by `%BY-183%` rather than a literal.

**[0039](./docs/adr/0039-the-list-row.md) is the list row. Read it before drawing any list.** A row
is a **band** carrying its text with a right-aligned **column** of icon actions, each allocated a
square of `controls::HEIGHT` — `controls::row`, which lives in `controls` and not in a screen
because **every bare `ui.button` left in the crate is a list row**: the note list never received
0034, drawing seventy-five controls at `widgets.inactive` and 19px against the 36px slab six pixels
above them, and the leech screen still does. The pictures **stand alone**, which is #149's icon-rule
exception taking its first real test and passing; the word survives as hover text, because the
exception buys a picture the right to stand alone *on screen*, not to be unnameable. Four things
that will catch a later reader. **The chrome's boundary is a hairline and the gap was already
right** — offered as a knob from `gap(1)` to `gap(8)`, the thumb left it where it opened and turned
the line on. **The text mirrors to the row's own direction and the cluster does not**, which narrows
0033 §5 to *content, not furniture*: a cluster that mirrored per row would destroy the column on the
one screen it was invented for. **A page rule reaches a scrolling surface only by pinning outside
the scroll** — *Create note* is the second thing in the app to live outside the `ScrollArea`, at the
cost of 209px of viewport, because a list has no leftover height for `slack_above` to spend.
And **0038 §1 gains a set clause**: a glyph standing alone keeps advance-equals-ink, a glyph in a
set takes a square advance, or two icon-only controls are two widths and the column is ragged again.
It also makes the filter a three-way so *Unfiled* is expressible, and gives *Delete deck* a warning
that names the count.

**[0040](./docs/adr/0040-the-note-editor.md) is the note editor. Read it before touching the editor,
a text field's fill, or any label above a field.** Seven decisions, most of them amending something
older. **The panes fold where a soft keyboard exists and no width is measured** — two columns dragged
to 118px each still read, so a narrow window is a gradient with no threshold in it; the app now has no
width-driven arrangement change anywhere, and `frame::cap_for` takes no width (0012 §1, 0031 §3 §4,
0025 §5). **A text field sits between the page and the card** — `extreme_bg_color` is the field, the
card holds `STONE_0`, and the two stopped being the same colour (0033 §2). **The Cards pane is a
preview**, so a dormant entry is drawn as a card-shaped outline — *a card that is not there* — and the
warning puts the dormant names at body, not the reassurance. ***Done* is on the reach line**, which
makes 0035 §1 mean *the last control on the page* rather than *the way forward*. **A field's label
follows its own field's direction and nothing else in the editor mirrors** — 0039 §4's *content, not
furniture* applied to a form; mirroring the whole editor flipped the screen on the first letter typed.
And the trap that reached three screens: **an un-wrapped egui widget inherits the ordinary weight** —
`widgets.inactive.bg_fill` is the slot every bare widget reads, so a role riding it is a broadcast,
and `primary_fill` names its rung instead. Every exit from the editor now settles the field you are
in; the idle half of 0021 §7 is still unbuilt and has its own ticket.

**[0041](./docs/adr/0041-the-file-surface.md) is the file surface — the import preview and the file
list. Read it before drawing a screen that puts a stranger's string beside the application's own.**
The rule underneath it: **two statements never share a string.** A deck's name is its own line,
aligned to its own direction, and a name inside a sentence goes through `bidi::isolate`; joined, a
Persian name decided the direction of the application's words around it. `cairn_export::plain` now
**drops bidi formatting characters**, so a file cannot close that isolate early and reverse the
sentence after it. The gate is *Import* and *Cancel* pinned on the reach line, or a lone *Close* for a
refusal and for a file that would change nothing; the nav row steps aside while a file is held, and
the preview opens at its top. And **0041 §4 settles the icon rule** 0039 left open: a picture stands
alone only where the symbol already exists, and elsewhere may only accompany its word — the file
row's picture sits **beside** *deck*, never instead of it.

**[0042](./docs/adr/0042-the-settings-screen.md) is the Settings screen. Read it before placing a
control on the reach line.** It amends 0035 §1: **the line binds a screen's *closing* control** —
the one pressed to finish or go on — not whichever control is drawn last, so a page of peer settings
places nothing there and its sub-states take it. Settings is grouped by **where a setting lives**:
*Studying* follows the collection to every device, *This device* stays on the machine, and the
development controls sit under their own heading at the end, in every build. Enrolled sync is decided
as one row opening its own screen and not yet drawn. The storyboards reach Appearance by **keyboard
focus**, because a coordinate below wrapping prose differs by width.

**[0043](./docs/adr/0043-the-leech-screen.md) is the leech screen.** Each card is a **well** with
*Edit*, *Suspend* and *Delete* worded underneath at the ordinary weight, rows sit 8 inside and 32
apart, and *Back to review* is on the reach line. The cost line reads **`6 bad days · 12 reviews ·
4 minutes`**: the rank key, the reviews and ADR-0010 §6's answer time, never *still failing*, which
the app does not know.

**[0044](./docs/adr/0044-the-ambient-hit-target.md) is the ambient hit target. Read it before drawing
a row of anything.** `controls::install` writes `controls::HEIGHT` into `interact_size.y` in every
theme slot, so a widget drawn as egui ships it — a bare button, a combo box and its entries, a
selectable label — is 36px without asking, and the nav row with it, which the repository owner judged
and kept. **`ui.horizontal` reads the same slot** and opens a row at 36px, so a row of *text* goes
through `spacing::text_row` or comes out double-spaced. A combo box's popup cap grows by the same
factor, since it is a height and not a count.

**[0045](./docs/adr/0045-the-native-android-client.md) is the native Android client.** Android
becomes Kotlin and Compose over the shared Rust core, in this repository, under the same package id.
On Android only, it lifts ADR-0016 §5's picker and filename refusals. Where Android has an
established practice, the app follows it without a decision. **§8 retires the egui Android build**
(31.10.2026): its packaging, the vendored patch, the inset seam and the ASCII-only rule are gone, an
upgrade over an installed egui app needs nothing, and the desktop stays egui.

**[0046](./docs/adr/0046-the-line-between-kotlin-and-rust.md) is the line between Kotlin and Rust.
Read it before moving anything out of `cairn-app` or writing a Kotlin call into Rust.** The binding
carries **operations**, not the domain: ids cross as canonical strings, and Kotlin composes no rule.
Two crates arrive — **`cairn-ops`**, the egui-free logic both renderers call, and **`cairn-ffi`**,
the only crate that touches uniffi — and `cairn-core` stays uniffi-free. Rule-bearing state (the
sitting, its reveal gate) is Rust's; the text being typed is Kotlin's. **Facts cross, words do not**:
a renderer says them in its own words. The platform — directories, the picker, `MediaStore`, the
share sheet — is Kotlin's, so export crosses as bytes, and everything persisted still goes through
the store. Never add `panic = "abort"` to the release profile: a panic would kill the process
instead of reaching Kotlin as an exception.

**[0047](./docs/adr/0047-the-file-surface-on-android.md) is the file surface on Android. Read it
before carrying a desktop file rule into the Kotlin app.** The system picker **replaces** the file
list — there is no list on Android — and asks for any type, so the bytes decide what was opened.
Export and the archive go through the system save dialog, pre-filled with the derived name (an
archive's carries its date), and the report states the name written with no location line; the
hand-off stays beside it. The Kotlin app calls none of `cairn-export::platform`; Kotlin reads the
document and its bytes cross to Rust (ADR-0046 §5).

**[0048](./docs/adr/0048-notes-and-the-editor-on-android.md) is Notes and the editor on Android. Read
it before carrying an editor rule into the Kotlin app.** The egui specification carries; what changes
is where the phone's input meets it. The destinations row stays words at the top (*Review · Notes ·
Settings*, Settings having joined with its slice), because the bottom is the reach line's. The editor gains a visible
*New note* above *Done*; the keyboard's action key is *Next* and closes the keyboard on the last field,
committing nothing; a note's delete is confirmed in its row; Back is *Done*, and an editor opened from
Review returns to the sitting. The panes always fold on a phone.

**[0049](./docs/adr/0049-text-search-folds-persian-letter-variants.md) folds Persian letter variants
in text search, on both renderers.** Query and field are folded alike — Arabic yeh and kaf to their
Persian forms, digits to Latin, the zero-width non-joiner, tatweel and vowel marks removed — and the
stored text is never changed.

**[0050](./docs/adr/0050-the-leech-screen-on-android.md) is the leech screen on Android.** It is a
place on Review's back stack, not a destination; *Back to review* and Back are one exit. Its *Delete*
asks first in the row with the note list's sentence, naming the note. The entrance counts only the
leeches the screen lists, on both renderers, so a suspended leech is named once. The pointer is held
in memory only.

**[0051](./docs/adr/0051-settings-on-android.md) is Settings on Android. Read it before adding a
development control to the Kotlin app.** The *Development* block holds reset and the fixture bench
only, and is compiled in only when the build is made with a build-time environment variable. A
session that installs such a build on the handset ends by upgrading it to one made without the
variable, never by uninstalling. No sync block is drawn until sync is built, and the empty note list
drops *"or set up sync"*. Until the file-surface slice, *This device* holds Appearance alone, and
reset and a fixture's install ask first in the row.

**[0052](./docs/adr/0052-the-file-surface-on-android-is-import-only.md) makes the file surface on
Android import only.** *Open a file…*, *Open with* and share reach the one import preview. Deck
export leaves the native map, because no renderer has an export screen. The save dialog, the report
and the hand-off are built first for the collection archive by the backup slice, and that is where
the share sheet gets checked. Back on the preview is the gate's dismissal.

**[0053](./docs/adr/0053-backup-and-restore-on-android.md) builds backup and restore on Android in
the native map**, though no renderer had it. Backup is a row in Settings' *Files* block, carrying
ADR-0016 §6's nudge, and saves through the save dialog. Restore has no action of its own: an archive
opened by *Open a file…*, *Open with* or share shows the restore preview, and a finished restore
lands on the note list unfiltered. "Last backup" is device-local and never settles.

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
- **Store tests run on desktop.** On Android the Kotlin app passes the store its directories, and
  the store is exercised there through the binding by the Kotlin app's instrumented tests.
- **The highest-value test in the repository** is that any interleaving of two devices' rows replays
  to the same state. It needs no sync implementation and no second device.

## Building

See [`README.md`](./README.md) for prerequisites. In short:

```sh
cargo run -p cairn-desktop            # desktop
cargo test --workspace                  # everything testable without hardware

source scripts/android-env.sh           # required before ANY Android command
cd android && ./gradlew assembleDebug   # the Kotlin app, with cairn-ffi's .so built in
```

Verify UI judgements on the **real handset** — the emulator is x86_64 and the Pixel 8 Pro is
arm64-v8a only.
