# Coding Standards

Rust standards for this crate. The reviewer agent loads this file during code
review via `@.sandcastle/CODING_STANDARDS.md`, so these are enforced at review
time without costing tokens during implementation.

Architectural decisions live in `docs/adr/` and the per-context `CONTEXT.md`
files, not here. Where an ADR and this file disagree, the ADR wins — say so
rather than silently following one.

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

- The domain core — scheduling, the event log, replay — stays free of I/O, of
  the clock, and of any UI or platform dependency. It should be testable with
  nothing but `cargo test`.
- Platform-specific code (storage backends, Android specifics, wasm) sits behind
  a trait at the edge, with the core depending on the trait rather than the
  implementation.
- Keep modules focused on one responsibility, and prefer composition and traits
  over deep generic gymnastics. If a signature needs three lines of `where`
  clauses, reconsider the design.
- Make illegal states unrepresentable where the type system can do it cheaply —
  an enum over a `bool` pair, a newtype over a bare `u64` id.
- No new dependency without justification in the commit message. Prefer the
  standard library, and prefer one well-maintained crate over three small ones.
