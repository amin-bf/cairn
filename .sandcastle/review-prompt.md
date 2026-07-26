# TASK

Review the code changes on branch `{{BRANCH}}` and improve code clarity, consistency, and maintainability while preserving exact functionality.

# CONTEXT

## Branch diff

!`git diff {{TARGET_BRANCH}}...{{BRANCH}}`

## Commits on this branch

!`git log {{TARGET_BRANCH}}..{{BRANCH}} --oneline`

# REVIEW PROCESS

1. **Understand the change**: Read the diff and commits above to understand the intent.

2. **Analyze for improvements**: Look for opportunities to:
   - Reduce unnecessary complexity and nesting
   - Eliminate redundant code and abstractions
   - Improve readability through clear variable and function names
   - Consolidate related logic
   - Remove unnecessary comments that describe obvious code
   - Prefer `match` over deeply nested `if let` / `else if` chains
   - Replace `if let Some(x) = ... else { default }` with the combinator that says it directly (`map_or`, `unwrap_or_else`, `ok_or`)
   - Choose clarity over brevity - explicit code is often better than overly compact code

3. **Check correctness**:
   - Does the implementation match the intent? Are edge cases handled?
   - Are new/changed behaviours covered by tests?
   - **Panics in library code**: `unwrap()`, `expect()`, indexing (`v[i]`), and integer division on values derived from input or storage. Each is a crash path — should it be a `Result`? `expect()` is acceptable only where the invariant is genuinely local and the message explains why it cannot fail.
   - **Silent numeric truncation**: `as` casts between integer widths or to/from float. Prefer `try_into()` with explicit handling, or `From` where it is lossless. This matters most for timestamps and interval arithmetic.
   - **`unsafe`**: any new `unsafe` block needs a comment justifying why it is sound. If it can be avoided, avoid it.
   - **Error handling**: is `?` propagating an error type that carries enough context, or is information being flattened away? Are errors swallowed with `let _ =` or `.ok()`?
   - **Time and clocks**: is wall-clock time read where a passed-in timestamp belongs? Untestable time is a design flaw, not a style nit.
   - Does the change introduce injection vulnerabilities, credential leaks, or other security issues? For SQL, confirm parameter binding rather than string interpolation.

4. **Maintain balance**: Avoid over-simplification that could:
   - Reduce code clarity or maintainability
   - Create overly clever solutions that are hard to understand
   - Combine too many concerns into single functions or components
   - Remove helpful abstractions that improve code organization
   - Make the code harder to debug or extend

5. **Apply project standards**: Follow the coding standards defined in @.sandcastle/CODING_STANDARDS.md

6. **Preserve functionality**: Never change what the code does - only how it does it. All original features, outputs, and behaviors must remain intact.

# EXECUTION

If you find improvements to make:

1. Make the changes directly on this branch
2. Verify with `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all-features`
3. Commit describing the refinements

If the code is already clean and well-structured, do nothing.

Once complete, output <promise>COMPLETE</promise>.
