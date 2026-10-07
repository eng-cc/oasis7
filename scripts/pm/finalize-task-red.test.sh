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
  '--preflight' '--cleanup=defer' '--cleanup-only' '--json'; do
  grep -F -- "$marker" <<<"$HELP" >/dev/null || {
    echo "RED finalize-task: help is missing $marker" >&2
    exit 1
  }
done

SOURCE="$(<"$ORCHESTRATOR")"
# The facade owns task/PR identity and delegates task completion plus the
# versioned producer and independent cleanup boundary.
for marker in \
  'task_uid' 'pr_number' 'pr-merge-receipt.py' 'task-closeout.sh' \
  'refresh-task-cache.sh' \
  'post-merge-cleanup.sh' 'post-merge-finalize.py'; do
  grep -F -- "$marker" <<<"$SOURCE" >/dev/null || {
    echo "RED finalize-task: missing lifecycle delegation marker $marker" >&2
    exit 1
  }
done
if grep -F -- 'post-merge-main-sync.sh' <<<"$SOURCE" >/dev/null; then
  echo "finalize-task: v2 delivery must not require main-sync" >&2
  exit 1
fi

# Preserve the immutable v1 terminal-reader adapter without routing new v2
# delivery through the historical main-sync/cleanup receipt chain.
for marker in 'protocol_selector' '== v1' '--terminal-receipt' 'legacy_v1_unchanged'; do
  grep -F -- "$marker" <<<"$SOURCE" >/dev/null || {
    echo "finalize-task: missing v1 compatibility marker $marker" >&2
    exit 1
  }
done

# Selector conflicts and cleanup-only requests fail closed; a retry may not
# mint a second task/PR terminal identity.
for marker in \
  '--resume' 'already_finalized' 'terminal protocol selector is malformed' \
  '--cleanup-only requires a mapped delivery receipt' 'task/PR mismatch' 'fail'; do
  grep -F -- "$marker" <<<"$SOURCE" >/dev/null || {
    echo "RED finalize-task: missing retry/fail-closed marker $marker" >&2
    exit 1
  }
done

# A fresh v2 producer readback precedes independent cleanup. Cleanup failure
# can be reported without revoking completed delivery.
for marker in \
  '--delivery --json' '--delivery --preflight --json' 'producer did not return a complete delivery proof' \
  'cleanup_deferred' 'cleanup_blockers'; do
  grep -F -- "$marker" <<<"$SOURCE" >/dev/null || {
    echo "finalize-task: missing delivery/cleanup boundary marker $marker" >&2
    exit 1
  }
done

producer_line="$(grep -nF 'delivery_json="$(python3 "$SCRIPT_DIR/post-merge-finalize.py"' <<<"$SOURCE" | cut -d: -f1)"
cleanup_line="$(grep -nF 'cleanup_json="$(run_cleanup)' <<<"$SOURCE" | tail -n 1 | cut -d: -f1)"
[[ -n "$producer_line" && -n "$cleanup_line" && "$producer_line" -lt "$cleanup_line" ]] || {
  echo "finalize-task: delivery readback must precede normal cleanup" >&2
  exit 1
}

echo "finalize-task-red.test: OK"
