# ADR-0046: The line between Kotlin and Rust — operations cross, the platform and the words stay in Kotlin

- **Status**: Accepted
- **Date**: 2026-10-29
- **Resolves**: [Native Android: Where the Line Between Kotlin and Rust Sits](https://github.com/amin-bf/cairn/issues/195)
- **Map**: [Map: Cairn as a Native Android App in Kotlin](https://github.com/amin-bf/cairn/issues/193)
- **Decided by**: the agent resolving the ticket, under [ADR-0045 §7](0045-the-native-android-client.md)
  and the repository owner's instruction for this grilling (29.10.2026): *go with the obvious, and
  come back only where it really needs me*. Every section below follows from an accepted ADR, the
  binding research, or Android's and Rust's established practice; none was a fork that needed the
  owner, so none was put to him.
- **Evidence**: [Binding the Rust core to Kotlin](../research/kotlin-binding/README.md) (#194)
- **Related**: [ADR-0009](0009-crate-and-workspace-layout.md) (crates, the platform seam),
  [ADR-0027 §2](0027-the-scheduler-dependency.md) (what `cairn-core` may depend on),
  [ADR-0038 §6](0038-the-mark-and-the-icon-rule.md) (the system names the operation, the gesture
  belongs to the platform), [ADR-0021 §7](0021-note-ordering-saving-and-the-note-list.md) (settling),
  [ADR-0036 §3](0036-the-light-palette.md) (the device-local preference)

## Context

ADR-0045 §2 makes the Kotlin app a UI over the shared Rust core, so that scheduling, replay, the log,
card identity and the import rules have one implementation. It left open where the line falls. The
research found three facts that shape the answer: uniffi's derives cannot go on `cairn-core`, whose
dependency test an FFI runtime fails; the `ndk_context` arms of both platform seams panic in a
library loaded by a Kotlin activity; and the application logic both renderers need — the session,
the editor's settling, the note list, import planning, optimisation, the fixtures — sits in
`cairn-app` beside eframe, winit and android-activity, which a binding cannot ship.

The ticket offered two grains: a coarse facade a screen calls, or the domain types bound one for one
and composed in Kotlin.

## Decision

1. **The binding carries operations, not the domain.** What crosses is what a screen asks the
   application to do — *open the collection*, *compose today's session*, *reveal*, *grade this card*,
   *commit this field*, *plan this import*, *apply it*, *export this deck* — and the records a screen
   draws from. Domain types cross only as identifiers (`CardRef`, `NoteId`, `DeckId` as their
   canonical strings) and inside those records; Kotlin never sees a log row, a memory state or a
   replay result, and composes no rule from them. Binding the domain one for one would put the
   session's composition, the sitting's counting and the import policy's sequencing in Kotlin — a
   second implementation of exactly what ADR-0045 §2 refuses — and uniffi's limits (no `usize`, no
   arrays or tuples, `&self` only) force a layer of mirror types anyway, so the coarse grain costs
   nothing the fine one avoids.

2. **Two new crates, and `cairn-core` stays as it is.**
   - **`cairn-ops`** (`crates/ops/`, lib) holds the application's operations and the state they
     need, in plain Rust with no binding and no renderer: the egui-free modules of `cairn-app` move
     here (`session`, `cards`, `deck`, `editor`, `notes`, `listing`, the import planning in
     `inbound`, `optimise`, `fixtures`, `file_bench`), with the restricted-Markdown grammar
     (`markdown`) and the sitting and editor state now in `cairn-app`'s `lib.rs`. It depends on
     `cairn-core`, `cairn-store` and `cairn-export`. **The egui app and the binding both call it**, so
     the desktop and Android run one implementation of every operation. A crate, not a module behind
     a feature flag, for ADR-0009 §2's reason: a crate boundary is the only way to state *this code
     cannot reach a renderer* such that the compiler enforces it.
   - **`cairn-ffi`** (`crates/ffi/`, `cdylib` + `lib`, library name `cairn_ffi`) is the only crate
     that depends on uniffi. It is thin: an exported handle over `cairn-ops`, the mirror records and
     enums uniffi can carry, and one error type. Its generated Kotlin lives in the package
     `dev.cairn.ffi`. The `uniffi-bindgen` binary needs uniffi's `cli` feature, which the NDK build
     must not carry (research §6), so it sits behind a host-only feature the Gradle build leaves off.
   - **`cairn-app` keeps what draws**: the screens, `theme`, `typography`, `spacing`, `frame`,
     `controls`, `surface`, `motion`, `fonts`, `bidi`, `keyboard`, the window's inset seam and the
     egui Android entry point. Sync's half (`sync.rs`) stays too: sync is outside the map, and
     moves when its own effort decides its Android half.

3. **State that carries a rule lives in Rust; state that is presentation lives in Kotlin.** The
   sitting — the count chosen, the gradings made, the card shown, whether it is revealed, when the
   sitting began, the leeches at its start — is a `cairn-ops` object, so ADR-0006 §4's *no grade
   before the answer is seen* and ADR-0029's *edit only when revealed* are refused by the operation,
   not by each renderer remembering to. **The text being typed is Kotlin's**: the field's buffer and
   its composition belong to the platform's text input, which is the whole reason Android left egui.
   Kotlin hands Rust a field's value when it commits or settles, and Rust decides whether it differs
   from the store (ADR-0021 §7). Navigation, scroll, focus, animation and which screen is showing are
   the Kotlin ViewModel's. **Time is a value** here as everywhere: where an operation needs *now*, the
   caller passes it.

4. **Facts cross; words do not.** An operation returns counts, enums and identifiers, and the
   renderer says them in its own words — Kotlin from its string resources, as the egui screens say
   them from theirs today. *Which* statement applies is a fact and is decided in Rust (the nudge is
   `Standard` or `Fitted`, an import is refused for a stated reason); *how it is worded* is the
   renderer's. The few sentence helpers that now sit in the logic modules (`nudge_text`, the card
   history, the dormant name) move to `cairn-app` with the egui screens rather than into `cairn-ops`.
   The ADRs that fix a sentence are the specification of that sentence for both renderers.

5. **The platform is Kotlin's.** Kotlin passes `filesDir` and `noBackupFilesDir` to the open
   operation, which `cairn_store::Collection::open` already takes. Every user file is held by Kotlin —
   the system picker and `ContentResolver` (ADR-0045 §6), `MediaStore`, the share sheet, the *Open
   with* intent — and **export crosses as bytes**: Rust reads a file's bytes to summarise, gate and
   plan it, and writes an export into bytes Kotlin saves. That holds whatever
   [the file-surface ticket](https://github.com/amin-bf/cairn/issues/196) decides the surface shows.
   `cairn-ffi` exports no platform function and calls none. **The Android arms of
   `cairn_store::platform` and `cairn_export::platform` stay** while the egui Android build exists,
   which needs them, and are deleted with it (ADR-0045 §5); until then they are dead code in the
   bound library, linked but never called.

6. **Everything Cairn persists goes through the store.** The Kotlin app keeps no preferences file or
   database of its own: the device-local appearance choice (ADR-0036 §3) and anything like it is read
   and written through the same operations the egui app uses, so the upgrade over the egui build
   (ADR-0045 §4) finds every choice where it was left. Only the platform's transient instance state
   is Kotlin's.

7. **One collection per process, called off the main thread.** The exported handle wraps
   `Collection` in a `Mutex` (research §5), and Kotlin holds exactly one, in an application-scoped
   owner, making every call from a coroutine on `Dispatchers.IO` — Android's own guidance for
   database work, under ADR-0045 §7. Long work keeps the shape `cairn-ops` gives it, with an explicit
   cancel, since uniffi has no built-in cancellation. **Errors** cross as one `CairnException`, its
   fields never named `message` (research §2); **a panic** reaches Kotlin as an exception only while
   the release profile unwinds, so `panic = "abort"` must never be added to it.

## Considered options

- **Bind the domain one for one and compose in Kotlin.** Refused by §1: it is the Kotlin rewrite of
  ADR-0045 §2 entered by the side door.
- **Put the uniffi derives on `cairn-ops` and skip the mirror crate.** One crate fewer and no mirror
  types, but the desktop would then build a binding runtime it never calls, and the shared logic
  would be shaped by the FFI's limits — `&self` everywhere, ids as strings, `u64` counts — for a
  renderer that has none of them.
- **Gate egui out of `cairn-app` behind a feature instead of a new crate.** Cargo unifies features
  across a workspace build, so `cargo test --workspace` would build the binding with egui in, and
  nothing would state the boundary the compiler checks.
- **Move the words into Rust as well**, for one copy of every sentence. Refused by §4: the egui
  screens already hold almost all of the app's words, the handful in the logic modules are the
  exception, and the rule *a renderer says it* is the one the existing code already follows.

## Consequences

- `cairn-ops` is a seventh crate and `cairn-ffi` an eighth; `CONTEXT-MAP.md`'s crate table gains
  them when they are built, not before.
- The glossary entries that say the session *exists only* in the UI context stay true in meaning —
  the session is still not a domain object — but its home moves to `cairn-ops`.
- **Handed to the spec round** ([Native Android: Spec Round — the Kotlin Skeleton and the Review
  Slice](https://github.com/amin-bf/cairn/issues/197)), because each is build detail with an
  established answer rather than a line to draw: the exact uniffi pin (0.32.2 at the time of the
  research), the NDK pin and the toolchain document's move to AGP 9.4 with built-in Kotlin, R8's
  keep rules for JNA and a measured minified size, and whether to move to uniffi's JNI generator once
  it is released.
