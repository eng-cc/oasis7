#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
REPO="$TMP/repo"
mkdir -p "$REPO/scripts"
cp "$ROOT/scripts/"{new-task-worktree.sh,worktree-harness-lib.sh,cargo-dev.sh,find-python-with-module.sh} "$REPO/scripts/"
git -C "$REPO" init -q
git -C "$REPO" config user.email test@example.com
git -C "$REPO" config user.name test
printf 'tracked\n' >"$REPO/file"
git -C "$REPO" add .
git -C "$REPO" commit -qm initial
"$REPO/scripts/new-task-worktree.sh" docs isolated --path "$TMP/new" --json >"$TMP/result"
python3 - "$TMP/result" "$TMP/new" <<'CHECK'
import json,os,sys
p=json.load(open(sys.argv[1]))
assert p['branch']=='codex/docs-isolated'
assert os.path.realpath(p['worktree_path'])==os.path.realpath(sys.argv[2])
assert 'pm_task' not in p
assert os.path.islink(sys.argv[2]+'/target')
CHECK
printf 'user work\n' >"$TMP/new/file"
if "$REPO/scripts/new-task-worktree.sh" docs isolated --path "$TMP/new" --json >/dev/null 2>&1; then
  echo 'duplicate path unexpectedly accepted' >&2; exit 1
fi
test "$(cat "$TMP/new/file")" = 'user work'
printf 'dirty\n' >"$REPO/file"
if "$REPO/scripts/new-task-worktree.sh" docs dirty --path "$TMP/dirty" >/dev/null 2>&1; then
  echo 'dirty source unexpectedly accepted' >&2; exit 1
fi
test ! -e "$TMP/dirty"
echo 'new-task-worktree.test: OK'
