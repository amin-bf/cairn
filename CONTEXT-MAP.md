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
| `leitner-core` | [`crates/core/`](./crates/core) | The domain, entire and pure. **Zero dependencies**, permanently. |
| `leitner-store` | [`crates/store/`](./crates/store) | SQLite persistence and the whole platform seam. |
| `leitner-export` | [`crates/export/`](./crates/export) | The `.ldeck` and `.lcoll` containers, the import policy, and the user-files platform seam. Holds the zip dependency. |
| `leitner-sync` | [`crates/sync/`](./crates/sync) | Publishing to the remote and reading it back. Holds the network dependencies. |
| `leitner-app` | [`crates/app/`](./crates/app) | The egui application, the bidi helper, the Android entry point. |
| `leitner-desktop` | [`crates/desktop/`](./crates/desktop) | A twenty-line shim. Forced by `cargo-apk`; keep it empty. |

Two rules about this table that are easy to break:

- **Nothing is added to `leitner-core`'s `[dependencies]` casually.** Its emptiness is what makes
  `cargo test -p leitner-core` need no database, no window and no handset. Adding to it is an ADR-
  sized decision.
- **`leitner-app` has no `src/main.rs`, and `leitner-desktop` has no logic.** `cargo-apk` panics
  after signing when one crate has both a cdylib and a bin, so the split is load-bearing, and code
  put in `desktop` is never compiled for Android and never runs on the handset.

## Contexts

| Context | `CONTEXT.md` | What it owns |
|---|---|---|
| Content | [`crates/core/src/content/`](./crates/core/src/content/CONTEXT.md) | Notes, cards, kinds, fields, decks, tags |
| Log | [`crates/core/src/log/`](./crates/core/src/log/CONTEXT.md) | Rows, writer ids, sequences, day scale, stamps, interchange |
| Scheduling | [`crates/core/src/scheduling/`](./crates/core/src/scheduling/CONTEXT.md) | FSRS arithmetic, grades, memory state, boxes |
| Replay | [`crates/core/src/replay/`](./crates/core/src/replay/CONTEXT.md) | The join: what exists, what state it is in, what is due |
| Store | [`crates/store/src/`](./crates/store/src/CONTEXT.md) | The two databases, device identity, the platform seam |
| Export | [`crates/export/src/`](./crates/export/src/CONTEXT.md) | Deck files, collection archives, profiles, revisions, import policy |
| Sync | [`crates/sync/src/`](./crates/sync/src/CONTEXT.md) | The remote, namespaces, objects, roll-up, enrolment |
| UI | [`crates/app/src/`](./crates/app/src/CONTEXT.md) | Screens, the session, the bidi helper |

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
  `leitner-core` cannot. It is also **the second crate with a platform seam**: ADR-0016 §5 puts
  put/get/list for user-visible files here, three `#[cfg]` arms with a `compile_error!` third, so
  that `leitner-store::platform` stays at exactly two functions. **The seam rule is per crate.**
- **`sync` is a crate for the reason this file predicted**: it needs HTTP, TLS and OAuth, which
  `leitner-core` cannot hold. [ADR-0013](./docs/adr/0013-the-sync-transport.md) realised the
  anticipated sixth crate rather than overturning anything. It depends on `log` and knows nothing
  about cards — the remote is a **rendezvous point, not a system of record**, so deleting it costs
  one republish and no data.

## Which ADRs bind which context

Read the ADR sections in your row. Read the whole ADR only if you are changing the decision.

| Context | Binding ADRs | Also bound by |
|---|---|---|
| `content` | [0002](./docs/adr/0002-the-card-model.md), [0005](./docs/adr/0005-the-deck-model.md), [0017](./docs/adr/0017-card-slots.md) | 0011 §7, 0012 §3, 0018 §3 |
| `log` | [0004](./docs/adr/0004-the-review-event-log.md) | 0002 §7, 0001 §6, 0010 §5, 0011 §5, 0013 §12, 0014 §6 |
| `scheduling` | [0001](./docs/adr/0001-scheduling-algorithm-and-grade-scale.md) | 0004 §4, 0004 §5, 0014 §6 |
| `replay` | *none of its own* | 0001 §7, 0002 §7, 0004 §9, 0007 §2, 0010 §2, 0011 §8, 0012 §5, 0017 §1, 0017 §5, 0018 §2 |
| `store` | [0007](./docs/adr/0007-the-local-store.md) | 0004 §11, 0003 §5, 0013 §9, 0016 §3, 0016 §7, 0019 §6, 0020 §3, 0020 §4 |
| `export` | [0008](./docs/adr/0008-the-deck-export-format.md), [0016](./docs/adr/0016-backup-and-restore.md) | 0005, 0002 §9, 0004 §11, 0011 §7, 0020 §4 |
| `sync` | [0013](./docs/adr/0013-the-sync-transport.md) | 0004 §2, 0004 §7, 0004 §10, 0007, 0014 §7, 0015 §2, 0015 §4, 0016 §10, 0019 §4, 0019 §6, 0020 §5, 0020 §6, 0020 §7 |
| `ui` | [0003](./docs/adr/0003-client-stack.md), [0006](./docs/adr/0006-the-review-session-experience.md), [0010](./docs/adr/0010-leeches.md), [0011](./docs/adr/0011-new-card-rate-and-daily-limits.md), [0012](./docs/adr/0012-the-note-authoring-experience.md), [0014](./docs/adr/0014-when-parameter-optimisation-runs.md), [0015](./docs/adr/0015-the-sync-experience.md), [0018](./docs/adr/0018-the-card-pane-ordering.md), [0019](./docs/adr/0019-naming-the-account-at-enrolment.md) | 0002 §4, 0016 §6, 0016 §11, 0016 §12, 0017 §5, 0017 §6, 0020 §7 |
| *the workspace itself* | [0009](./docs/adr/0009-crate-and-workspace-layout.md) | 0013 §11, 0013 §12, 0015 §15, 0016 §5 |

**`replay` having no ADR of its own is why it is a context.** Its rules were each written for another
purpose and sit scattered across four documents; its `CONTEXT.md` is the only place they appear as
one mechanism. If you are touching replay, read that file before the ADRs.

**[0020](./docs/adr/0020-protection-at-rest.md) binds four contexts and owns none**, which is why it
appears only in the right-hand column. It decides what is protected at rest across every artifact at
once — the store, the archive, the credential and the published log — and answering for one of them
alone is what three earlier ADRs each declined to do.

**If you write a new ADR, add it to this table.** An ADR that is not in this index is invisible to
the agent it was written for.

## Testing

- **`cargo test -p leitner-core`** needs no database, no window and no handset. Most of the
  specification is verifiable here, and that is deliberate.
- **Time and identity are values, never injected traits.** Replay needs no clock at all — day
  numbers are frozen on the row at write time and fuzz is seeded from card identity. The two places
  that need "now" take it as a parameter.
- **There is no fake store.** Store tests open a real SQLite database in a temp directory, because
  the design *is* WAL, `BEGIN IMMEDIATE`, `ATTACH` and `INSERT OR IGNORE`.
- **Store tests run on desktop.** On Android the store depends on the activity existing, so
  `leitner-store` is not independently runnable there.
- **The highest-value test in the repository** is that any interleaving of two devices' rows replays
  to the same state. It needs no sync implementation and no second device.

## Building

See [`README.md`](./README.md) for prerequisites. In short:

```sh
cargo run -p leitner-desktop            # desktop
cargo test --workspace                  # everything testable without hardware

source scripts/android-env.sh           # required before ANY Android command
cd crates/app && cargo apk build        # APK: a manifest and one .so
```

Verify UI judgements on the **real handset** — the emulator is x86_64 and the Pixel 8 Pro is
arm64-v8a only.
