# ADR-0009: Crate and workspace layout

- **Status**: Accepted
- **Date**: 2026-07-30
- **Resolves**: [Decide: crate and workspace layout](https://github.com/amin-bf/leitner/issues/14)
- **Map**: [Map: local-first Leitner app spec](https://github.com/amin-bf/leitner/issues/1)
- **Related**: every prior ADR. This one gives the other seven a place to live.

## Context

This is the last decision before the spec is handed off. Seven ADRs settled *what* the application
is; none of them said where any of it goes, and six of them
([ADR-0001](0001-scheduling-algorithm-and-grade-scale.md),
[ADR-0002](0002-the-card-model.md), [ADR-0004](0004-the-review-event-log.md),
[ADR-0005](0005-the-deck-model.md), [ADR-0007](0007-the-local-store.md) and
[ADR-0008](0008-the-deck-export-format.md)) deferred their glossaries to this ticket with the same
sentence: *"they move into a context's `CONTEXT.md` once #14 fixes where contexts live."* Roughly 52
settled terms have been waiting on this file.

The map fixes **agents implement this**, which changes what a good layout is. The usual prizes —
compile times, publishability, a pleasant import graph — matter less than two properties an agent
cannot intuit its way around:

1. **Mistakes must fail the build**, not the app on a handset.
2. **The right thing must be findable** without reading everything. `docs/adr/` is 2,591 lines.

Two constraints arrive already fixed. [ADR-0003 §5](0003-client-stack.md) records that `cargo-apk`
panics after signing (`Bin is not compatible with Cdylib`) when one crate has both a cdylib and a
bin — the APK is correct, the exit code is not — so **the desktop binary must live in its own
crate**. And [ADR-0007](0007-the-local-store.md) narrowed the platform surface to *two directory
lookups plus the Android JNI shim*, which is far smaller than ADR-0003's "the storage backend"
framing implied: `rusqlite` with `bundled` compiles unchanged for both targets.

## Decision

### 1. Five crates

```
leitner/
├── Cargo.toml              workspace root, resolver 3
├── CONTEXT-MAP.md          the reading order and the ADR index
├── docs/adr/               every ADR, one sequence
└── crates/
    ├── core/     leitner-core     lib          the domain, pure, zero dependencies
    ├── store/    leitner-store    lib          rusqlite, the two files, the platform seam
    ├── export/   leitner-export   lib          the .ldeck container and the import policy
    ├── app/      leitner-app      lib+cdylib   egui, the bidi helper, the Android entry point
    └── desktop/  leitner-desktop  bin          a shim, forced by cargo-apk
```

Two of these lines are forced by the toolchain. `desktop` is separate because of the `cargo-apk`
panic above. `app` is separate from `store` because only one of them is a cdylib and only one of
them splits `eframe` per target.

The rest are chosen, and §2, §4 and §11 say why.

> **Extended by [ADR-0013 §11](0013-the-sync-transport.md)**: a sixth crate, `crates/sync/`
> (`leitner-sync`, lib), holds HTTP, TLS and OAuth. **This is not an overturning** — `CONTEXT-MAP.md`
> recorded the prediction as this ADR landed (*"a `sync` context is anticipated, not created… expect
> a sixth crate rather than a fifth module"*), on §2's ground that a network dependency cannot live
> in `leitner-core`. The count in this heading is left as written, because it records what was
> decided here; the live list is `CONTEXT-MAP.md`'s.

### 2. `leitner-core` has no dependencies, and that is its interface

Its `[dependencies]` section is empty, deliberately and permanently. No `rusqlite`, no `egui`, no
clock, no random number generator, no serialisation crate.

This is affordable only because [ADR-0007 §2](0007-the-local-store.md) made the stored `line` column
authoritative and every other column derived: the domain deals in interchange lines, so it never
needs to know a database exists. The property bought is that **`cargo test -p leitner-core` verifies
most of the specification with no database, no window and no handset** — and it is build-checked
rather than merely intended.

A trait-based store seam was considered and rejected. There is exactly one store adapter, and
`rusqlite` opens `:memory:` databases, so an abstraction would exist only to serve tests that do not
need it. A crate boundary is not a swappable-adapter seam; it is the only way to state *this code
cannot reach a database* such that the compiler enforces it.

### 3. Contexts are modules, not crates

`content`, `log`, `scheduling` and `replay` are modules inside `leitner-core`, each with a
`CONTEXT.md` beside it.

Crates were the obvious alternative and would enforce privacy between contexts. They were rejected
because **nothing varies across a context line** — one implementation each — and because the
dependency graph makes the cost concrete rather than theoretical. A log row carries a `CardRef`
([ADR-0004 §5](0004-the-review-event-log.md)) and scheduler fuzz is seeded from `CardRef`'s 18-byte
encoding ([ADR-0001 §7](0001-scheduling-algorithm-and-grade-scale.md)), so a crate-per-context split
needs a fifth shared-types crate on day one. That crate would be the tell that the line was drawn in
the wrong place.

The graph is acyclic with `content` at the base:

```
content ──┬──> log ───────┬──> replay
          └──> scheduling ┘
```

`scheduling` deliberately does **not** depend on `log`: it takes grades and day numbers as values,
so FSRS arithmetic is testable against a hand-written list with no rows, no writers and no merge.

### 4. The platform seam is three arms, and the third is `compile_error!`

All of it lives in `leitner-store::platform`, and it is two functions wide — `data_dir()` and
`state_dir()`.

```rust
#[cfg(target_os = "android")]                    #[path = "android.rs"]  mod imp;
#[cfg(any(linux, macos, windows))]               #[path = "desktop.rs"]  mod imp;
#[cfg(not(any(android, linux, macos, windows)))] compile_error!("unsupported target: …");
```

The third arm is the decision. A binary `android` / `not(android)` partition is tidier and can never
fail to compile — which is precisely its defect: a new target silently takes the desktop arm and
fails at runtime, on a device, which is the failure mode ADR-0003 spent four prototypes buying its
way out of. **Verified**: `cargo check -p leitner-store --target wasm32-unknown-unknown` fails with
the message above rather than compiling.

**A third function appearing in this module means the seam is eroding.** That is the signal to stop,
not to add it.

> **Contradiction recorded by [ADR-0013 §12](0013-the-sync-transport.md).** That sentence and this
> ADR's handoff entry for *Any ticket that adds a platform capability* — "It goes through
> `leitner-store::platform` or it does not exist" — cannot both be followed by a ticket needing a
> platform capability that is **not storage**. #39 was the first to reach for one: the handoff sends
> `open_url()` into a storage crate, and this section forbids it arriving there.
>
> **ADR-0013 does not resolve it**; its §8 chose an enrolment flow needing no platform capability at
> all, so the collision was routed around rather than settled. It is written here because the next
> ticket to need one will meet it with no equivalent escape. **The shape of the fix, when it is
> needed**: the rule becomes per crate rather than per workspace — `leitner-store` keeps exactly two
> functions, and a crate that must touch the platform for an unrelated reason gets its own module
> under the same three-arm discipline, `compile_error!` included. What must *not* happen is a third
> function landing here because the handoff table said so.

> **Amended by [ADR-0015 §15](0015-the-sync-experience.md) — the prohibition is on behaviour, not on
> capability.** #40 is the next ticket the note above predicted, and it met the rule from an angle
> that note did not anticipate: not a *function*, and not in this crate. The Android editor must
> state that non-Latin text cannot be typed there — winit has no IME path, so the failure reaching
> the user is **silence**, and it can only be said in advance.
>
> **What is permitted**: a compile-time constant naming a platform *capability*, so an interface can
> state a limitation. **What is not**: platform-conditional behaviour, and a growing function seam —
> both unchanged, and the sentence above still governs them. The distinction is the one
> [ADR-0003](0003-client-stack.md) won the stack decision with, *a `#[cfg]` the compiler checks beats
> a runtime `if` nobody checks*: this rule exists to stop divergence becoming **invisible**, and a
> capability constant exists to make a limitation **visible**.
>
> **This does not discharge the contradiction above**, which is about a platform capability *function*
> for a non-storage crate. That is still open, and its recorded fix still stands.

The two arms are not symmetric in one respect worth recording: `store`'s Android arm reads the JVM
handle from `ndk_context`, which `android-activity` populates inside `leitner-app`. So the store
cannot be opened before the activity exists, **`leitner-store` is not independently runnable on
Android**, and store tests run on desktop.

### 5. `leitner-desktop` is a shim, and a re-export is what keeps it one

Its `main.rs` is twenty lines and its only dependency is `leitner-app`; `eframe` arrives by
re-export rather than by a dependency of its own.

That is not tidiness. A direct `eframe` dependency here could resolve a different feature set from
the one `leitner-app` was built with, and it would give this crate a route to grow real code.
Anything written here is **never compiled by the Android build and never exercised on the handset** —
a silent desktop-only path, which is the same class of defect as a runtime platform check wearing a
different hat.

`leitner-app` correspondingly has **no `src/main.rs`**, and adding a `[[bin]]` to it breaks the
Android release build.

One manifest detail that resisted the obvious form: `eframe` cannot be a workspace dependency,
because its Android arm needs `default-features = false` (its default `accesskit` is refused
alongside `android-native-activity`, per ADR-0003 §5) and Cargo forbids overriding a workspace
entry's `default-features`. Both arms are therefore declared literally in `crates/app/Cargo.toml`,
side by side in one file where they cannot drift.

### 6. Seven contexts

| Context | Lives in | Supersedes the glossary of |
|---|---|---|
| `content` | `crates/core/src/content/` | ADR-0002, ADR-0005 |
| `log` | `crates/core/src/log/` | ADR-0004 |
| `scheduling` | `crates/core/src/scheduling/` | ADR-0001 |
| `replay` | `crates/core/src/replay/` | the cache terms of ADR-0004 and ADR-0007 |
| `store` | `crates/store/src/` | ADR-0007 |
| `export` | `crates/export/src/` | ADR-0008 |
| `ui` | `crates/app/src/` | ADR-0006 |

Decks fold into `content` rather than taking a context of their own: ADR-0005 §5 settles a deck as
`{ id, name }` and nothing else, and §8 puts membership on the note. Four terms and a two-field
struct do not survive the deletion test. **Deck *files* are a different matter — see §11.**

**`replay` is a context, and it is the one worth arguing about.** It produces scheduling state, so
filing it under `scheduling` is tempting. It is separate because
[ADR-0002 §7](0002-the-card-model.md)'s prize — the card set is computed from current content,
unmatched events are retained and simply not projected, history reattaches by itself — is precisely
the *join* between content and log. Filing it under `scheduling` would make `scheduling` depend on
`content` and forfeit §3's testability property.

It is also **the only context with no ADR of its own**, and that is the strongest argument for
giving it one. Its rules were each written for another purpose and sit scattered across ADR-0001 §7,
ADR-0002 §7, ADR-0004 §9 and ADR-0007 §2. Gathered into one `CONTEXT.md` they are a single coherent
mechanism; left scattered they get reimplemented wrongly.

The name was chosen over `collection`, which has textual support elsewhere in these ADRs but also
names a *file* (`collection.db`) in a different crate. A context sharing a name with a database file
is an ambiguity an agent will resolve wrongly at some point.

**Naming hazard, recorded because it is invisible until it bites:** the `log` module would shadow
the `log` crate. §2's zero-dependency rule makes that collision unreachable rather than merely
unlikely — logging belongs at the edges, in `store` and `app`.

### 7. ADRs are system-wide; `CONTEXT.md` files are local

**Every ADR lives in `docs/adr/` under one sequence. Context-scoped `docs/adr/` directories are not
used in this repository** — `docs/agents/domain.md` permits them, and we decline.

**Every `CONTEXT.md` sits next to the code it describes**, with `CONTEXT-MAP.md` at the root.

The principle: **a glossary describes code you are looking at; a decision constrains code you are
not.** Vocabulary wants locality. Decisions want one discoverable sequence, because their whole
value is reaching the agent who did not know to look for them.

The evidence is one-sided. Every ADR here is cross-cutting — ADR-0004 places requirements on five
separate tickets, ADR-0002 on four — so filing one under a single context would mean choosing a home
for a decision that deliberately does not have one, and an agent working in `scheduling` would never
find the row-format constraint that binds it.

"Both are permitted" was rejected as worse than either: it reliably produces a split-brain where half
the decisions are somewhere else, and it forces an agent to know which contexts exist before it can
find the decisions.

When a decision really is local to one context, it still goes in `docs/adr/`. Numbering is free;
discoverability is not.

### 8. Time and identity are values, not injected traits

`leitner-core` has no `Clock` trait, no RNG trait, and no store trait.

The ADRs make this unusually cheap. [ADR-0004 §4](0004-the-review-event-log.md) stamps the day number
at write time and freezes it, and ADR-0001 §7 disables load balancing, calendar shaping and sibling
avoidance to buy replay purity, seeding fuzz from card identity. **Replay therefore needs no clock
and no randomness at all** — it is a pure function from ordered rows and configuration to memory
state. Only two call sites need "now": stamping a new row, and computing "due today" against the
device-local day. Both are at the edge, in `store` and `app`.

A `Clock` trait would be a hypothetical seam by the two-adapter rule — one clock, one RNG, abstracted
solely for tests that a plain parameter serves better. The payoff is concrete: ADR-0004 §8's
clock-skew guard is tested by passing a `now` two years in the past and asserting the guard fires
against the log's own contents.

**Two testing rules follow.**

- **No fake store.** Store tests open a real SQLite database in a temp directory. ADR-0007's design
  *is* WAL, `BEGIN IMMEDIATE`, `ATTACH` and `INSERT OR IGNORE` on `(writer, seq)`; a fake would pass
  while testing none of it, and its existence would make the real adapter's tests look redundant.
  This is the one place slow tests are preferable to a second adapter.
- **Merge commutativity is a property test.** ADR-0004 §2 makes merging set union with duplicates
  dropped, so the claim the entire sync design rests on is that *any interleaving of two devices'
  rows replays to the same state*. That is verifiable today, with no sync implementation and no
  second device, and it is the highest-value test in the repository.

### 9. What an implementing agent reads, in order

Defined at the top of `CONTEXT-MAP.md`, ordered by what breaks silently if skipped:

1. **`AGENTS.md`** — the rules that fail without an error message. ~40 lines, and every one of them
   is a bug the compiler and the tests will not catch.
2. **`CONTEXT-MAP.md`** — four crates, six contexts, and which `CONTEXT.md` covers your area.
3. **The one or two relevant `CONTEXT.md` files** — vocabulary, so new code uses the ADRs' words.
4. **Only the ADR sections that bind your context**, via the index in `CONTEXT-MAP.md`.

Step 4 is the part that does real work: without an index, "read the ADRs" means 2,591 lines, and an
agent that skims will miss the rule that breaks silently.

`docs/research/` is deliberately **not** in the reading order. It is the evidence trail for
reopening a decision, not reading for implementing one, and its findings are already distilled into
the ADRs that cite it. An agent that starts there spends its budget before writing a line.

### 10. What was verified, not merely asserted

The layout was built and compiled rather than described, because ADR-0003 §5's constraints are the
kind that read fine in prose and fail on a handset.

| Check | Result |
|---|---|
| `cargo check --workspace --all-targets` | clean |
| `cargo test --workspace` | 9 bidi tests pass |
| `cargo apk build` (aarch64-linux-android) | **exit 0** — no `Bin is not compatible with Cdylib` |
| APK contents | `AndroidManifest.xml` + `lib/arm64-v8a/libleitner_app.so`, no `classes.dex`, no `res/` |
| `rusqlite` + `bundled` cross-compile | builds for `aarch64-linux-android` |
| `cargo check -p leitner-store --target wasm32-unknown-unknown` | fails with §4's `compile_error!` |

Release signing is deliberately **not** configured: the only working keystore is a developer's local
debug key, an absolute path to one machine, and deployment is out of scope for this map.
`cargo apk build --release` compiles fully and stops at signing.

### 11. Export is a fifth crate, decided after ADR-0008 landed

[ADR-0008](0008-the-deck-export-format.md) merged while this ticket was being resolved, and it
changes the answer this ADR would otherwise have given. Its "Requirements this places on downstream
tickets" section for #13 originally read *export belongs in `leitner-core`, and if a container format
needs a binary framing library then §2's zero-dependency rule is consciously spent*. ADR-0008 §2
chose a **zip archive**, so that rule would have been spent on the first line of the first
implementation. It is not, and export gets its own crate instead.

Two independent reasons, either of which would be enough:

- **The dependency.** ADR-0008 §2 verified `zip` 8.6 with
  `--no-default-features --features deflate-flate2-zlib-rs` at nine transitive crates with no `-sys`
  crate. Nine crates is cheap, and it still has no business in a domain crate whose emptiness is the
  property §2 exists to protect.
- **The shape.** Export spans contexts. It reads `content` today, and ADR-0008 §1 reserves a
  **progress** profile carrying ADR-0004 §11 interchange lines, which
  [#37](https://github.com/amin-bf/leitner/issues/37) will specify — so it will read `log` too. A
  thing that spans content and log is a peer of `replay`, not a module inside either.

**`leitner-app` depends on `leitner-export`**, so the UI can offer import and export without
reaching around the domain. Nothing in `leitner-core` depends on it, and nothing should: the
container is a serialisation concern, and the domain must stay ignorant of it exactly as it stays
ignorant of SQLite.

This is the one decision in this ADR that was not put to the human first — ADR-0008 landed after the
layout had been settled, and leaving it unplaced would have left six terms homeless and the
zero-dependency rule ambiguous on day one. It is cheap to reverse: folding `export` back into
`content` is a directory move plus a dependency line, for as long as no code has landed in it.

## Requirements this places on downstream tickets

### [#37 — backup and restore](https://github.com/amin-bf/leitner/issues/37)

1. The **progress** profile lands in `leitner-export`, not a new crate — ADR-0008 §1 rejected a
   second container, and §11 above gives the first one a home.
2. It is the point at which `leitner-export` gains a dependency on `log`. That is expected, not a
   violation.

### [#39 / #40 — sync transport and the sync experience](https://github.com/amin-bf/leitner/issues/39)

1. A `sync` context is **anticipated but not created**. It will depend on `log` (the version summary
   is already `log`'s term) and will need a network dependency, which cannot go in `core` under §2 —
   so expect a fifth crate, `leitner-sync`, rather than a fifth module.
2. The version summary and the ahead/behind test are `log`'s, not sync's. Do not reimplement them.

### [#42 — when parameter optimisation runs](https://github.com/amin-bf/leitner/issues/42)

1. The optimiser is a `scheduling` concern, but *when it runs* is a `ui` one — Android freezes a
   backgrounded app, so scheduling the work is a foreground-lifecycle question.

### Any ticket that adds a platform capability

1. It goes through `leitner-store::platform` or it does not exist. A second `#[cfg(target_os)]`
   elsewhere in the workspace is a defect, not a shortcut.
2. **Read §4's recorded contradiction first.** Rule 1 holds for anything storage-shaped and collides
   with §4's "a third function means the seam is eroding" for anything that is not.
   [ADR-0013 §12](0013-the-sync-transport.md) documents the collision and the shape of the fix; it
   deliberately did not apply it, because #39 turned out not to need a platform capability.

## Where the deferred glossaries went

The five "provisional" glossary sections are now superseded. Each ADR's glossary block has been
replaced by a pointer to its context:

| ADR | Terms | Now of record in |
|---|---|---|
| ADR-0001 | 10 | `crates/core/src/scheduling/CONTEXT.md` |
| ADR-0002 | 14 | `crates/core/src/content/CONTEXT.md` (dormant card → `replay`) |
| ADR-0004 | 12 | `crates/core/src/log/CONTEXT.md` (cache → `replay`) |
| ADR-0005 | 4 | `crates/core/src/content/CONTEXT.md` |
| ADR-0007 | 6 | `crates/store/src/CONTEXT.md` (derivation version, cache high-water → `replay`) |
| ADR-0008 | 6 | `crates/export/src/CONTEXT.md` (acquired kind definition also noted in `content`) |

## Consequences

- **`leitner-core`'s empty dependency list is now a decision with a guard.** Adding anything to it
  is a deliberate act that should be argued in an ADR, not a convenience.
- **Most of the specification is verifiable without hardware**, which is what makes an agent fleet
  workable: the expensive checks (handset, APK, real SQLite) are a small minority.
- **The `#[cfg]` seam is two functions and one file.** It is small enough to review at a glance,
  which is the only reason a rule against eroding it can be enforced.
- **The reading order is now a maintained artefact.** A new ADR that binds a context must be added
  to `CONTEXT-MAP.md`'s index, or it becomes invisible to the agent it was written for.
- **Six `CONTEXT.md` files are the vocabulary of record.** Drift between them and the ADRs is now
  possible in a way it was not when the ADRs held the glossaries; the `CONTEXT.md` wins, and the ADR
  keeps the reasoning.
- **A sixth crate is expected for sync**, for the same reason export got the fifth: it needs a
  network dependency that `leitner-core` cannot hold. The crate count is not a target; the rule is
  that contexts are modules unless a dependency or a cross-context span forces otherwise.

## Open items handed onward

| Item | Owner |
|---|---|
| A `sync` crate, once the transport is chosen | [#39](https://github.com/amin-bf/leitner/issues/39) |
| Whether `export` should have been a module in `content` after all | Reversible while the crate is empty |
| Release signing configuration for the APK | Out of scope: deployment and CI |
| Build sequencing for the implementing fleet — what lands first | The fleet, not this map |
