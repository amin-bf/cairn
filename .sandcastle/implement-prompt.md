# TASK

Fix issue {{TASK_ID}}: {{ISSUE_TITLE}}

Pull in the issue using `gh issue view <ID>`. If it has a parent PRD, pull that in too.

Only work on the issue specified.

Work on branch {{BRANCH}}. Make commits and run tests.

**The branch may already carry work.** A previous run on this issue can be interrupted after it
commits, and you will then start on a branch where some or all of the issue is already done. Check
`git log HEAD --not $(git merge-base HEAD @{u} 2>/dev/null || echo HEAD~0)` — or simply read the
branch's commits — before writing anything. Finish only what is left. If the issue is already fully
implemented and the feedback loops below are clean, **do not manufacture a commit to show for it**:
say so on the issue and output the completion promise. An empty run on a finished branch is a
correct outcome, and the harness reads the branch itself rather than your commit count.

# CONTEXT

Here are the last 10 commits:

<recent-commits>

!`git log -n 10 --format="%H%n%ad%n%B---" --date=short`

</recent-commits>

# EXPLORATION

Explore the repo and fill your context window with relevant information that will allow you to complete the task.

Pay extra attention to test files that touch the relevant parts of the code.

# EXECUTION

If applicable, use RGR to complete the task.

1. RED: write one test
2. GREEN: write the implementation to pass that test
3. REPEAT until done
4. REFACTOR the code

# FEEDBACK LOOPS

Before committing, run all four and make sure they are clean:

1. `cargo fmt --all` — formats; never hand-format instead
2. `cargo clippy --all-targets --all-features -- -D warnings` — lints, warnings are errors
3. `cargo test --all-features` — the test suite
4. `cargo build --release` **only** if the change could plausibly affect release-only behaviour
5. `cargo build --target aarch64-linux-android` **only** if the change touches a `#[cfg(target_os = "android")]` arm, a `platform` module, or `leitner-app`

This sandbox can **compile and link** for the handset but cannot package or run: there is no JDK, no SDK and no device, so `cargo apk build` is unavailable and nothing here can verify on-device behaviour. Issues needing that carry a separate "Verify on the handset" ticket — do not attempt to discharge one, and do not treat its absence as a reason to skip step 5.

`cargo clippy` type-checks as it lints, so there is no separate type-check step — a clean clippy run means the crate compiles.

If a command fails for a reason unrelated to your change (a pre-existing failure on the base branch), say so explicitly in the commit message rather than working around it or disabling the check.

# COMMIT

Make a git commit. The commit message must:

1. Start with `RALPH:` prefix
2. Include task completed + PRD reference
3. Key decisions made
4. Files changed
5. Blockers or notes for next iteration

Keep it concise.

# THE ISSUE

If the task is not complete, leave a comment on the issue with what was done.

Do not close the issue - this will be done later.

Once complete, output <promise>COMPLETE</promise>.

# FINAL RULES

ONLY WORK ON A SINGLE TASK.
