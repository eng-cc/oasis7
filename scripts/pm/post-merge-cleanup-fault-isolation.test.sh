#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FIXTURE="$ROOT_DIR/scripts/pm/fixtures/post-merge-cleanup-fault.sh"
TMPDIR="$(mktemp -d)"; trap 'rm -rf "$TMPDIR"' EXIT
ISOLATED="$TMPDIR/isolated"; OUTSIDE="$TMPDIR/outside"
mkdir -p "$ISOLATED" "$OUTSIDE"

# The fixture owns an explicit isolation boundary. It must reject receipt paths
# outside that boundary before executing anything.
if "$FIXTURE" --isolation-root "$ISOLATED" \
  --fault TPM_CLEANUP_FAULT_AFTER_WORKTREE_REMOVE -- \
  /usr/bin/true --terminal-receipt-output "$OUTSIDE/terminal.json" \
  >"$TMPDIR/outside.out" 2>"$TMPDIR/outside.err"; then
  echo "expected external receipt path to be rejected" >&2; exit 1
fi
grep -Eqi 'isolat|outside|boundary' "$TMPDIR/outside.err" || {
  echo "expected isolation-boundary rejection, got:" >&2; cat "$TMPDIR/outside.err" >&2; exit 1;
}

# Even entirely in the isolation root, the wrapper may invoke only the fixed
# production cleanup executable; it is not an arbitrary command runner.
if "$FIXTURE" --isolation-root "$ISOLATED" \
  --fault TPM_CLEANUP_FAULT_AFTER_WORKTREE_REMOVE -- \
  /usr/bin/true --repo-root "$ISOLATED/repo" \
  --worktree "$ISOLATED/worktree" \
  --terminal-receipt-output "$ISOLATED/terminal.json" \
  >"$TMPDIR/command.out" 2>"$TMPDIR/command.err"; then
  echo "expected arbitrary command to be rejected" >&2; exit 1
fi
grep -Eqi 'fixed|cleanup|command|executable' "$TMPDIR/command.err" || {
  echo "expected fixed-command rejection, got:" >&2; cat "$TMPDIR/command.err" >&2; exit 1;
}

# Repository, worktree and receipt must share the isolation root and Git
# common-dir; an external repository is rejected before destructive effects.
git -C "$OUTSIDE" init -q -b main
if "$FIXTURE" --isolation-root "$ISOLATED" \
  --fault TPM_CLEANUP_FAULT_AFTER_WORKTREE_REMOVE -- \
  "$ROOT_DIR/scripts/pm/post-merge-cleanup.sh" \
  --repo-root "$OUTSIDE" --worktree "$ISOLATED/worktree" \
  --terminal-receipt-output "$ISOLATED/terminal.json" \
  >"$TMPDIR/repo.out" 2>"$TMPDIR/repo.err"; then
  echo "expected external repository to be rejected" >&2; exit 1
fi
grep -Eqi 'isolat|outside|common-dir|repository' "$TMPDIR/repo.err" || {
  echo "expected repository-boundary rejection, got:" >&2; cat "$TMPDIR/repo.err" >&2; exit 1;
}

echo "post-merge-cleanup-fault-isolation.test: OK"
