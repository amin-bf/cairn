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

## The client stack

**egui / eframe**, chosen in [ADR-0003](./docs/adr/0003-client-stack.md). One crate, one binary per
platform, no webview, no IPC. Setup and commands are in [`README.md`](./README.md).

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
   platform check — the whole stack choice rests on wrong platform code failing the build.
4. **Immediate mode has nowhere to `await`.** Spawn the future, store a handle, read the result on a
   later frame, and call `ctx.request_repaint()` on completion or the result sits unseen until the
   next input event.
5. **The desktop binary must live in its own crate.** `cargo-apk` panics after signing when one crate
   has both a cdylib and a bin. The APK is fine; the exit code is not, and CI will break.
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

## Agent skills

### Issue tracker

Issues live as GitHub issues on `amin-bf/leitner`, managed via the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

The five canonical triage roles, using the default label strings. See `docs/agents/triage-labels.md`.

### Domain docs

Multi-context — a root `CONTEXT-MAP.md` pointing at per-context `CONTEXT.md` files. See `docs/agents/domain.md`.
