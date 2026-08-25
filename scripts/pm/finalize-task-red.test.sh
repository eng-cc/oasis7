#!/usr/bin/env bash
# RED contract for the single task-UID/PR-driven terminal orchestrator.
#
# This is intentionally source/help focused: the implementation slice must
# first expose one resumable entrypoint and keep the existing fail-closed
# helpers as the only effectful lifecycle boundaries.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ORCHESTRATOR="$ROOT_DIR/scripts/pm/finalize-task.sh"

[[ -x "$ORCHESTRATOR" ]] || {
  echo "RED finalize-task: missing executable terminal orchestrator" >&2
  exit 1
}

HELP="$($ORCHESTRATOR --help)"
for marker in \
  '--task-uid' '--pr' '--resume' '--repo-root' \
  '--patch-equivalence-receipt' '--json'; do
  grep -F -- "$marker" <<<"$HELP" >/dev/null || {
    echo "RED finalize-task: help is missing $marker" >&2
    exit 1
  }
done

SOURCE="$(<"$ORCHESTRATOR")"
# The facade owns task/PR identity and delegates each existing terminal step.
for marker in \
  'task_uid' 'pr_number' 'pr-merge-receipt.py' 'task-closeout.sh' \
  'refresh-task-cache.sh' 'post-merge-main-sync.sh' \
  'post-merge-cleanup.sh' 'post-merge-finalize.py'; do
  grep -F -- "$marker" <<<"$SOURCE" >/dev/null || {
    echo "RED finalize-task: missing lifecycle delegation marker $marker" >&2
    exit 1
  }
done

# Ordinary merges use ancestry; squash/rebase merges require an explicit,
# repository-generated patch-equivalence receipt before retrying sync/cleanup.
for marker in \
  'patch-equivalence-receipt.sh' '--patch-equivalence-receipt' \
  'merge-base' 'ancestry' 'patch_equivalence'; do
  grep -F -- "$marker" <<<"$SOURCE" >/dev/null || {
    echo "RED finalize-task: missing ordinary/squash marker $marker" >&2
    exit 1
  }
done

# A retry must resume the durable journal and fail closed on identity drift;
# it may not mint a second task/PR terminal identity.
for marker in \
  '--resume' 'already_finalized' 'task-uid' 'pr' 'mismatch' 'fail'; do
  grep -F -- "$marker" <<<"$SOURCE" >/dev/null || {
    echo "RED finalize-task: missing retry/fail-closed marker $marker" >&2
    exit 1
  }
done

echo "finalize-task-red.test: OK"
