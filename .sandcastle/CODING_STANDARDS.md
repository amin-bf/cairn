# Coding Standards

Rust standards for this crate. The reviewer agent loads this file during code
review via `@.sandcastle/CODING_STANDARDS.md`, so these are enforced at review
time without costing tokens during implementation.

Architectural decisions live in `docs/adr/` and the per-context `CONTEXT.md`
files, not here. Where an ADR and this file disagree, the ADR wins — say so
rather than silently following one. `CONTEXT-MAP.md` at the repository root
indexes which ADR sections bind which context; `docs/adr/` is past 2,600 lines,
and that index is what stops "read the ADRs" from meaning all of them.

## Style

- Formatting is `cargo fmt --all`. Never hand-format, never argue with it.
- `cargo clippy --all-targets --all-features -- -D warnings` must be clean.
  Suppress a lint only with `#[allow(...)]` plus a comment saying why; never
  crate-wide.
- Naming follows the Rust API Guidelines: `snake_case` items, `CamelCase` types,
  `SCREAMING_SNAKE_CASE` consts. Conversions are named for their cost —
  `as_`/`to_`/`into_` mean borrow/clone/consume respectively.
- Prefer borrowing in function signatures (`&str` over `String`, `&[T]` over
  `Vec<T>`) unless ownership is genuinely needed.
- Public items get a `///` doc comment stating what it does and what it
  guarantees. Skip the ones that only restate the signature.
- Comments explain *why*, never *what*. Delete a comment that paraphrases the
  line under it.
- Use the domain glossary's vocabulary from the relevant `CONTEXT.md`. If a term
  is missing there, that is a signal — either the name is wrong or the glossary
  has a gap worth noting.

## Errors and panics

- Library code returns `Result`; it does not panic. `unwrap()` and `expect()`
  belong in tests, and in non-test code only where the invariant is local and a
  comment or `expect()` message explains why it cannot fail.
- No `as` casts between integer widths on values from input, storage, or the
  clock. Use `try_into()` and handle the failure, or `From` where it is
  lossless. Truncated timestamps and intervals are silent data corruption.
- Errors carry context. Don't flatten a specific error into a `String` or a bare
  enum variant that loses what went wrong.
- Never discard a `Result` with `let _ =` or `.ok()` without a comment saying
  why the failure is genuinely ignorable.

## Testing

- Unit tests sit in a `#[cfg(test)] mod tests` beside the code; integration
  tests live in `tests/`.
- Test names describe the behaviour asserted, not the function called:
  `fails_a_card_back_to_the_first_box`, not `test_review`.
- Test observable behaviour through the smallest public surface that exposes it.
  Reaching into private internals makes refactoring look like breakage.
- **Time is a parameter, never ambient.** Anything scheduling-related takes the
  current instant as an argument so tests can pass an arbitrary one. A function
  that reads the wall clock internally is untestable, and this crate's whole
  domain is time arithmetic.
- Derived state must be reproducible: replaying the same event log twice
  produces identical state. Prefer a test that asserts that property over one
  that asserts a hardcoded snapshot.
- Property-based tests are welcome for merge, replay, and interval arithmetic —
  the places where hand-picked examples reliably miss the interesting cases.

## Architecture

- The domain core — content, the event log, scheduling, replay — stays free of
  I/O, of the clock, and of any UI or platform dependency. `leitner-core` is a
  crate boundary rather than a convention: `cargo test -p leitner-core` needs no
  database, no window and no handset (ADR-0009 §2).
- **The platform seam is a compile-time `#[cfg]`, never a trait and never a
  runtime check** (ADR-0003 §5, ADR-0009 §4). Three arms, the third a
  `compile_error!`, so an unrecognised target fails the build instead of
  silently taking the desktop path. The rule is **per crate**: `leitner-store`
  stays at exactly two functions and a third appearing there means the seam is
  eroding; a crate needing the platform for an unrelated reason gets its own
  module under the same discipline (ADR-0016 §5).
- **A trait-based store seam was considered and rejected** (ADR-0009 §2). There
  is one adapter and `rusqlite` opens `:memory:`, so the abstraction would exist
  only to serve tests that do not need it — which is why there is **no fake
  store**: store tests open a real database in a temp directory, because the
  design *is* WAL, `BEGIN IMMEDIATE`, `ATTACH` and `INSERT OR IGNORE`.
- **Time and identity are values, never injected traits** (ADR-0009 §8). Replay
  needs no clock at all — day numbers are frozen on the row at write time and
  fuzz is seeded from card identity — so a `Clock` trait would be a hypothetical
  seam. The two call sites that need "now" take it as a parameter.
- Keep modules focused on one responsibility, and prefer composition and traits
  over deep generic gymnastics. If a signature needs three lines of `where`
  clauses, reconsider the design.
- Make illegal states unrepresentable where the type system can do it cheaply —
  an enum over a `bool` pair, a newtype over a bare `u64` id.
- No new dependency without justification in the commit message. Prefer the
  standard library, and prefer one well-maintained crate over three small ones.
