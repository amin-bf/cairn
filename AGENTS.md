# leitner-app

## Writing conventions

**Name the fact, not the product.** Prior art is cited by what it establishes and why, never by a
bare product name standing in for the explanation. "X does it this way" tells a reader nothing
unless they already know X; it makes the document depend on knowledge that isn't in it.

A named application may appear **only alongside the substance** — the mechanism, the reasoning, and
a primary source — so the passage stands on its own and a reader who has never used that
application loses nothing. Research notes in `docs/research/` are where this most often applies:
they exist to carry evidence, so the evidence must be written out, not pointed at.

Everywhere else — ADRs, `CONTEXT.md`, issues, code, commit messages — prefer stating the finding
and its source directly. If a fact only exists as "that app does X", find the underlying source or
argue the trade-off on its own merits.

This applies to every agent working in this repo, on every artifact that persists.

## Start with the context map

**[`CONTEXT-MAP.md`](./CONTEXT-MAP.md) is the entry point to the codebase** — the five crates, the
seven contexts, and an index saying which ADR sections bind which context. `docs/adr/` is over 2,600
lines; the index is what stops "read the ADRs" from meaning all of them.

Read this file first, then the context map, then the `CONTEXT.md` for the area you are touching.
`docs/research/` is the evidence trail for reopening a decision, not reading for implementing one.

## The workspace

Five crates, laid out in [ADR-0009](./docs/adr/0009-crate-and-workspace-layout.md):
`leitner-core` (the domain, pure), `leitner-store` (SQLite and the platform seam), `leitner-export`
(the `.ldeck` container), `leitner-app` (egui, lib + cdylib), `leitner-desktop` (a shim, forced by
`cargo-apk`).

Contexts are **modules, not crates**. Vocabulary lives in a `CONTEXT.md` beside the code; decisions
live system-wide in `docs/adr/`, and context-scoped `docs/adr/` directories are not used here.

### Rules that are easy to break silently

1. **`leitner-core` has no dependencies, and adding one is an ADR-sized decision.** Its empty
   `[dependencies]` is what makes `cargo test -p leitner-core` need no database, no window and no
   handset. `rusqlite` belongs in `leitner-store`; `egui` and `eframe` belong in `leitner-app`.
2. **Time and identity are values, never injected traits.** Replay needs no clock at all — day
   numbers are frozen on the row at write time, and fuzz is seeded from card identity. The two call
   sites that need "now" take it as a parameter. A `SystemTime::now()` inside `leitner-core` breaks
   the property that two devices replaying one log agree.
3. **There is no fake store.** Store tests open a real SQLite database in a temp directory, because
   the design *is* WAL, `BEGIN IMMEDIATE`, `ATTACH` and `INSERT OR IGNORE`.
4. **A new ADR must be added to `CONTEXT-MAP.md`'s index.** One that is not there is invisible to
   the agent it was written for.

## The client stack

**egui / eframe**, chosen in [ADR-0003](./docs/adr/0003-client-stack.md). One binary per platform,
no webview, no IPC. Setup and commands are in [`README.md`](./README.md).

**The targets are desktop and Android.** The web target was ruled out of scope in
[ADR-0007 §1](./docs/adr/0007-the-local-store.md) — a browser cannot be the system of record for an
app whose only copy of the data is local. Rules below that mention the web build are retained
because they are validated findings, not because a web build ships.

### Rules that are easy to break silently

1. **All user-visible text goes through the bidi helper.** egui places text runs left-to-right in
   logical order, so a plain `ui.label("…")` renders Persian and Arabic with the words backwards, and
   Arabic-Indic digits reversed. Build a `LayoutJob` with sections in visual order instead. A
   `ui.label` on card content is a bug, not a style choice.
2. **`TextEdit` needs the same treatment, via `.layouter()`** — it lays out its own text and
   otherwise bypasses the helper. Note that caret and selection are then in visual order while the
   buffer is logical, so RTL editing is imprecise; design around it rather than fighting it.
3. **The storage seam is a compile-time `#[cfg]`.** Keep it that way. Never introduce a runtime
   platform check — the whole stack choice rests on wrong platform code failing the build. It is two
   functions in `leitner-store::platform`, and a **third function appearing there means the seam is
   eroding**. A `#[cfg(target_os)]` anywhere else in the workspace is a defect.
4. **Immediate mode has nowhere to `await`.** Spawn the future, store a handle, read the result on a
   later frame, and call `ctx.request_repaint()` on completion or the result sits unseen until the
   next input event.
5. **The desktop binary lives in its own crate, and that crate stays empty.** `cargo-apk` panics
   after signing when one crate has both a cdylib and a bin — the APK is fine, the exit code is not,
   and CI breaks. So never add a `[[bin]]` to `leitner-app`, and never put logic in
   `leitner-desktop`: code there is never compiled for Android and never runs on the handset.
   It takes `eframe` by re-export from `leitner-app`, not as its own dependency.
6. **`eframe`'s dependency is split per target** — its default `accesskit` feature is rejected
   alongside `android-native-activity`.
7. **Fonts are ours to ship — and must be installed on the first frame, not in `CreationContext`.**
   egui bundles only Hack, Ubuntu-Light and Noto Emoji. Register any added face in **every** family
   you use, including `Monospace`, or text silently renders as boxes. Registering during creation
   breaks the web build: wgpu panics with "Tried to update a texture that has not been allocated
   yet", glow renders everything near-black. Defer it one frame.
8. **Android text input is ASCII-only, and cannot be fixed here.** winit's Android backend handles
   only motion and key events — it has no IME path, so composed text never reaches the app. This is
   not the activity backend: GameActivity was tried and reverted (see
   [`prototypes/egui-slice/android/README.md`](https://github.com/amin-bf/leitner/blob/prototypes/issue-8/prototypes/egui-slice/android/README.md)). Never design a feature that requires typing non-Latin
   text on Android.
9. **Verify Android on the real handset.** The emulator is x86_64; the Pixel 8 Pro is arm64-v8a only.

## The local store

Two SQLite files, chosen in [ADR-0007](./docs/adr/0007-the-local-store.md): `collection.db` is
authoritative, `derived.db` is a disposable cache attached to the same connection.

### Rules that are easy to break silently

1. **`INSERT OR IGNORE` is for merge-ingest only. Our own writes use plain `INSERT`.** On another
   device's rows it is the union merge; on our own it silently drops a review.
2. **Never take the next sequence number from `MAX(seq) WHERE writer = me`.** After a merge that
   continues *another* device's numbering, which is the duplicate-writer failure the design exists to
   prevent. Use `local.seq_highwater` — and a log row above it means someone else is writing as us,
   so mint a new writer id.
3. **Every write transaction is `BEGIN IMMEDIATE`.** Sequence allocation is a read-modify-write; a
   deferred transaction loses updates between two processes.
4. **Only `log.line` is authoritative.** Every other column, and everything in `derived.db`, is
   derived and may be dropped and rebuilt. Derived columns do **not** have to round-trip.
5. **The writer marker lives outside the backup set** — `getNoBackupFilesDir()` on Android,
   `$XDG_STATE_HOME` on desktop. Move it into the data directory and a restored phone becomes a
   duplicate writer.

## Deck export

A `.ldeck` file is a **zip archive** carrying deck content and never review progress, chosen in
[ADR-0008](./docs/adr/0008-the-deck-export-format.md).

### Rules that are easy to break silently

1. **A shipped kind definition always wins; an imported file may never overwrite one.** A file's
   definition is used only for kinds this build does not ship. Break this and an import becomes a
   remote path to reordering a kind's `cards` list, which silently retypes every accumulated review
   onto the wrong card and cannot be repaired from the log.
2. **Never write an ADR-0004 §7 stamp into an export.** A stamp is a counter plus a **writer id**,
   which is a device fingerprint, and its counter is meaningless outside its own collection. Import
   assigns fresh local stamps — and only to values whose content actually differs, or every import
   floods the user's own devices with edits.
3. **Import branches on deck id, and the branches have opposite rules.** Id already held → the file
   wins and may move notes into that deck (ADR-0005 §9). Id new → notes already held are never touched
   or moved (ADR-0005 §2). Applying one rule everywhere lets a stranger's file take notes out of decks
   the user already has.
4. **The importer accepts only the known member names and the `media/` prefix**, rejecting absolute
   paths, `..` segments and symlink entries. Zip path traversal is this container's classic defect.
5. **Export must be byte-for-byte deterministic** — fixed member order, pinned timestamps, fixed
   deflate level, no extra fields. Zip's per-member timestamps otherwise make identical content export
   as different bytes, which leaks build time and breaks "same revision, same file".
6. **The revision advances only when the content digest changes**, not on every export. Incrementing
   per export inflates the counter and makes relaying an unmodified deck emit a phantom revision that
   competes with the original author's next one.
7. **`zip`'s `deflate-flate2` feature does not compile** — it selects no zlib backend. Use
   `--no-default-features --features deflate-flate2-zlib-rs`; the `deflate` umbrella feature builds but
   drags in zopfli for an encoder we do not need.

## Agent skills

### Issue tracker

Issues live as GitHub issues on `amin-bf/leitner`, managed via the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

The five canonical triage roles, using the default label strings. See `docs/agents/triage-labels.md`.

### Domain docs

Multi-context — a root `CONTEXT-MAP.md` pointing at per-context `CONTEXT.md` files. See `docs/agents/domain.md`.
