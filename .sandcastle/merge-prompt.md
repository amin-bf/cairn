# TASK

Merge the following branches into the current branch:

{{BRANCHES}}

For each branch:

1. Run `git merge <branch> --no-edit`
2. If there are merge conflicts, resolve them intelligently by reading both sides and choosing the correct resolution
3. After resolving conflicts, verify the merge with `cargo clippy --all-targets --all-features -- -D warnings` and `cargo test --all-features`
4. If either fails, fix the issues before proceeding to the next branch

A merge that is textually clean can still break the build — two branches adding different variants to the same enum, or changing a shared function's signature and its callers independently, both merge without conflict markers and then fail to compile. Run the checks after **every** branch, not just at the end, so a failure is attributable to one merge.

If a conflict is in `Cargo.toml` dependencies, keep both dependencies and take the higher version requirement rather than picking one side. Then run `cargo update --workspace` and confirm `Cargo.lock` is coherent.

After all branches are merged, make a single commit summarizing the merge.

# CLOSE ISSUES

For each branch that was merged, close its issue using the following command:

`gh issue close <ID> --comment "Completed by Sandcastle"`

Here are all the issues:

{{ISSUES}}

Once you've merged everything you can, output <promise>COMPLETE</promise>.
