# ISSUES

Here are the open issues in the repo:

<issues-json>

!`bash .sandcastle/issues-with-deps.sh`

</issues-json>

The list above has already been filtered to issues ready for work — open, and carrying the `Sandcastle` label, which a human applies by hand to clear an issue for this unsupervised loop.

Each issue carries GitHub's **native dependency edges**, resolved from the issue tracker:

- `blocked_by` — every issue the tracker records as blocking this one, with its `state` and `title`. Closed blockers stay in this list; they no longer gate anything.
- `blocked_by_open` — the numbers of the **open** blockers only. This is the live gate.

# TASK

Analyze the open issues and build a dependency graph. For each issue, determine whether it **blocks** or **is blocked by** any other open issue.

Dependencies come from two sources, and they are not equally authoritative.

## 1. Declared dependencies (authoritative)

**An issue with a non-empty `blocked_by_open` is blocked. This is not a judgement call.** A human recorded that edge on the tracker deliberately; do not second-guess it, do not reason your way past it, and do not select the issue because it looks independent to you. An empty `blocked_by_open` means nothing is *declared* against it — it does not by itself mean the issue is workable, because of the next section.

Note that a blocker may be absent from the issue list entirely — it might be closed, or unlabelled, or otherwise filtered out. Only `blocked_by_open` decides; an issue you cannot see is not an issue you can ignore.

## 2. Inferred dependencies (your judgement)

The tracker does not know about file-level collisions, so infer these yourself on top of the declared edges. Issue B is blocked by issue A if:

- B requires code or infrastructure that A introduces
- B and A modify overlapping files or modules, making concurrent work likely to produce merge conflicts
- B's requirements depend on a decision or API shape that A will establish

An issue is **unblocked** only when `blocked_by_open` is empty **and** you infer no blocking dependency on another open issue.

For each unblocked issue, assign a branch name using the exact format `sandcastle/issue-{id}` (no slug or other suffix). This must be deterministic so that re-planning the same issue always produces the same branch name and accumulated progress is preserved.

# OUTPUT

Output your plan as a JSON object wrapped in `<plan>` tags:

<plan>
{"issues": [{"id": "42", "title": "Fix auth bug", "branch": "sandcastle/issue-42"}]}
</plan>

Include only unblocked issues.

If every issue has a non-empty `blocked_by_open`, output an empty plan. **Never select an issue with a declared open blocker**, even as a last resort — the tracker's edges are a hard constraint, and working a declared-blocked issue produces exactly the conflict the edge exists to prevent.

If every issue is blocked only by *inferred* dependencies — none has a declared open blocker — then include the single highest-priority candidate (the one with the fewest or weakest inferred dependencies), since inference can be wrong and stalling forever is worse.

Always emit the `<plan>` tags, even when there is nothing to do. If there are no issues to work on at all, output `<plan>{"issues": []}</plan>` so the run can exit cleanly.
