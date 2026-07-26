#!/usr/bin/env bash
#
# Emits the open, work-ready issues as a JSON array, with GitHub's *native*
# issue-dependency edges resolved for each one.
#
# `gh issue list --json` has no dependencies field, so the blocked-by edges have
# to be fetched per issue from the REST API. Each issue in the output gains:
#
#   blocked_by       — every issue blocking it, with number/state/title
#   blocked_by_open  — just the numbers of the *open* blockers (the live gate)
#
# Runs inside the sandbox: sandcastle expands the !`...` block in plan-prompt.md
# via sandbox.exec(). Two constraints follow from that, and both shape this
# script:
#
#   1. Prompt expansion is capped at 30s total. The per-issue dependency calls
#      therefore run in parallel (PARALLELISM), not in a sequential loop —
#      at --limit 100 a serial version would not reliably finish in time.
#   2. A non-zero exit fails the entire sandcastle run. That is the behaviour we
#      want for an auth or network failure — a token missing Issues access must
#      NOT look like "no work to do" — so the issue listing is checked
#      explicitly. Only the per-issue dependency calls degrade quietly to an
#      empty list, so that a repo without the dependencies API still plans.
#
# Requires a GitHub token with Issues: read on this repo.
#
# The label filter is `Sandcastle`, deliberately NOT the `ready-for-agent`
# triage label from docs/agents/triage-labels.md. The two mean different things:
#
#   ready-for-agent  — a triage state: fully specified, no human evaluation
#                      needed. /to-spec and /to-tickets apply it on publish.
#   Sandcastle       — a human gate: cleared to run unsupervised in this loop.
#                      Applied by hand, one `gh issue edit` per issue.
#
# Keeping them separate means specification-completeness never implies consent
# to run AFK. It also keeps containers out of the queue for free: /to-spec
# labels the spec issue `ready-for-agent`, so a single vocabulary would offer
# the planner the whole PRD as one unit of work. It cannot see it here.
#
# Usage: bash .sandcastle/issues-with-deps.sh
# Testing overrides: LABEL=... LIMIT=... PARALLELISM=...

set -uo pipefail

LABEL="${LABEL:-Sandcastle}"
LIMIT="${LIMIT:-100}"
PARALLELISM="${PARALLELISM:-8}"

# Re-entrant worker: fetches one issue's blockers. Invoked by xargs below.
# Kept as a mode of this same script so the parallel fan-out needs no nested
# quoting of a bash -c payload.
if [ "${1:-}" = "--fetch-deps" ]; then
  number="$2"
  blockers=$(gh api "repos/{owner}/{repo}/issues/${number}/dependencies/blocked_by" \
    --jq '[.[] | {number, state, title}]' 2>/dev/null) || blockers='[]'
  [ -z "$blockers" ] && blockers='[]'
  jq -nc --argjson b "$blockers" --arg n "$number" \
    '{number: ($n | tonumber), blocked_by: $b}'
  exit 0
fi

issues=$(gh issue list \
  --state open \
  --label "$LABEL" \
  --limit "$LIMIT" \
  --json number,title,body,labels,comments \
  --jq '[.[] | {number, title, body, labels: [.labels[].name], comments: [.comments[].body]}]')
rc=$?

# Distinguish a real failure from a legitimately empty backlog: a successful
# listing with no matches still prints "[]", so empty output means the command
# itself failed. `gh issue list` exits non-zero on auth/network errors, and we
# want the run to die loudly there rather than plan an empty iteration and
# report "nothing to do".
if [ "$rc" -ne 0 ] || [ -z "$issues" ]; then
  echo "issues-with-deps: 'gh issue list' failed — check the token has Issues: read on this repo" >&2
  exit 1
fi

numbers=$(printf '%s' "$issues" | jq -r '.[].number')

if [ -z "$numbers" ]; then
  echo "[]"
  exit 0
fi

deps=$(printf '%s\n' "$numbers" \
  | xargs -P "$PARALLELISM" -I {} bash "$0" --fetch-deps {} \
  | jq -sc '.')

[ -z "$deps" ] && deps='[]'

# Join the dependency edges onto each issue, and derive the open-blocker gate.
printf '%s' "$issues" | jq --argjson deps "$deps" '
  ($deps | map({key: (.number | tostring), value: .blocked_by}) | from_entries) as $byNumber
  | [ .[]
      | (($byNumber[.number | tostring]) // []) as $b
      | . + {
          blocked_by: $b,
          blocked_by_open: [$b[] | select(.state == "open") | .number]
        }
    ]
'
