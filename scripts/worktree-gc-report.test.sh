#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

TMPDIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

TEST_REPO="$TMPDIR/repo with space"
CURRENT_WORKTREE="$TEST_REPO"
CLEAN_WORKTREE="$TMPDIR/task worktree with spaces"
BROKEN_WORKTREE="$TMPDIR/broken-worktree"
PRUNABLE_WORKTREE="$TMPDIR/prunable-worktree"

mkdir -p "$TEST_REPO/scripts" "$TEST_REPO/.pm/tasks" "$TEST_REPO/.git" "$TMPDIR/bin" "$CLEAN_WORKTREE/target" "$CLEAN_WORKTREE/crates/oasis7_viewer/node_modules" "$BROKEN_WORKTREE"
cp "$ROOT_DIR/scripts/worktree-gc-report.sh" "$TEST_REPO/scripts/worktree-gc-report.sh"
cp "$ROOT_DIR/scripts/worktree-harness-lib.sh" "$TEST_REPO/scripts/worktree-harness-lib.sh"
chmod +x "$TEST_REPO/scripts/worktree-gc-report.sh"
printf 'target-cache' > "$CLEAN_WORKTREE/target/sample.bin"
printf 'node-cache' > "$CLEAN_WORKTREE/crates/oasis7_viewer/node_modules/sample.bin"

cat > "$TEST_REPO/.pm/tasks/task_11111111111111111111111111111111.yaml" <<EOF
task_uid: task_11111111111111111111111111111111
title: cleanup closed worktree
status: done
updated_at: 2026-04-24T11:12:00+08:00
worktree_hint: "$CLEAN_WORKTREE"
EOF

cat > "$TMPDIR/bin/git" <<EOF
#!/usr/bin/env bash
set -euo pipefail

repo_root="$TEST_REPO"
current_worktree="$CURRENT_WORKTREE"
clean_worktree="$CLEAN_WORKTREE"
broken_worktree="$BROKEN_WORKTREE"
prunable_worktree="$PRUNABLE_WORKTREE"

if [[ "\${1:-}" == "--git-dir="* ]]; then
  shift
fi

if [[ "\${1:-}" == "rev-parse" && "\${2:-}" == "--is-inside-work-tree" ]]; then
  printf 'true\n'
  exit 0
fi

if [[ "\${1:-}" == "rev-parse" && "\${2:-}" == "--git-common-dir" ]]; then
  printf '%s/.git\n' "\$repo_root"
  exit 0
fi

if [[ "\${1:-}" == "worktree" && "\${2:-}" == "list" && "\${3:-}" == "--porcelain" ]]; then
  printf 'worktree %s\nHEAD 1111111\nbranch refs/heads/main\n\n' "\$current_worktree"
  printf 'worktree %s\nHEAD 2222222\nbranch refs/heads/task/review\$(rm)\n\n' "\$clean_worktree"
  printf 'worktree %s\nHEAD 3333333\nbranch refs/heads/task/broken\n\n' "\$broken_worktree"
  printf 'worktree %s\nHEAD 4444444\nbranch refs/heads/task/prunable\nprunable gitdir file points to non-existent location\n\n' "\$prunable_worktree"
  exit 0
fi

if [[ "\${1:-}" == "-C" && "\${3:-}" == "status" && "\${4:-}" == "--short" ]]; then
  case "\$2" in
    "\$current_worktree" | "\$clean_worktree")
      exit 0
      ;;
    "\$broken_worktree")
      exit 1
      ;;
    *)
      echo "unexpected status path: \$2" >&2
      exit 1
      ;;
  esac
fi

echo "unexpected git invocation: \$*" >&2
exit 1
EOF
chmod +x "$TMPDIR/bin/git"

REPORT_FILE="$TMPDIR/worktree-gc-report.json"
NO_FOOTPRINT_REPORT_FILE="$TMPDIR/worktree-gc-report-no-footprint.json"
(cd "$TEST_REPO" && PATH="$TMPDIR/bin:$PATH" ./scripts/worktree-gc-report.sh --json --footprint > "$REPORT_FILE")
(cd "$TEST_REPO" && PATH="$TMPDIR/bin:$PATH" ./scripts/worktree-gc-report.sh --json > "$NO_FOOTPRINT_REPORT_FILE")

python3 - "$REPORT_FILE" "$NO_FOOTPRINT_REPORT_FILE" "$TEST_REPO" "$CLEAN_WORKTREE" "$BROKEN_WORKTREE" "$PRUNABLE_WORKTREE" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

report_path = Path(sys.argv[1])
no_footprint_report_path = Path(sys.argv[2])
repo_root = str(Path(sys.argv[3]).resolve())
clean_worktree = str(Path(sys.argv[4]).resolve())
broken_worktree = str(Path(sys.argv[5]).resolve())
prunable_worktree = str(Path(sys.argv[6]).resolve())

payload = json.loads(report_path.read_text(encoding="utf-8"))
no_footprint_payload = json.loads(no_footprint_report_path.read_text(encoding="utf-8"))
if "footprint_included" in no_footprint_payload["summary"]:
    raise SystemExit(f"non-footprint summary should not gain footprint keys: {no_footprint_payload['summary']}")
if any("footprint" in entry for entry in no_footprint_payload["entries"]):
    raise SystemExit("non-footprint entries should not gain footprint keys")
expected_summary = {
    "total_worktrees": 4,
    "prunable_worktrees": 1,
    "dirty_worktrees": 0,
    "cleanup_candidates": 2,
    "footprint_included": True,
}
for key, value in expected_summary.items():
    if payload["summary"].get(key) != value:
        raise SystemExit(f"unexpected summary {key}: {payload['summary']}")
if not isinstance(payload["summary"].get("known_target_bytes"), int):
    raise SystemExit(f"expected target footprint summary: {payload['summary']}")
if not isinstance(payload["summary"].get("known_viewer_node_modules_bytes"), int):
    raise SystemExit(f"unexpected summary: {payload['summary']}")

entries = {entry["path"]: entry for entry in payload["entries"]}

clean_entry = entries[clean_worktree]
if clean_entry["pm_task_status"] != "done":
    raise SystemExit(f"expected done task for clean worktree: {clean_entry}")
if not clean_entry["footprint"] or clean_entry["footprint"]["target_bytes"] <= 0:
    raise SystemExit(f"expected clean worktree target footprint: {clean_entry}")
if clean_entry["footprint"]["viewer_node_modules_bytes"] <= 0:
    raise SystemExit(f"expected clean worktree node_modules footprint: {clean_entry}")
expected_remove = f"git -C '{repo_root}' worktree remove -f '{clean_worktree}'"
expected_branch = "git -C '{}' branch -d 'task/review$(rm)'".format(repo_root)
if clean_entry["cleanup_commands"] != [expected_remove, expected_branch]:
    raise SystemExit(f"unexpected quoted cleanup commands: {clean_entry['cleanup_commands']}")

broken_entry = entries[broken_worktree]
if broken_entry["dirty"] is not None or broken_entry["cleanup_candidate"]:
    raise SystemExit(f"expected broken worktree to stay non-candidate with dirty=null: {broken_entry}")

prunable_entry = entries[prunable_worktree]
if prunable_entry["cleanup_reasons"] != ["prunable_worktree"]:
    raise SystemExit(f"expected prunable cleanup reason: {prunable_entry}")
if not prunable_entry["branch_delete_candidate"]:
    raise SystemExit(f"expected branch delete candidate for prunable worktree: {prunable_entry}")
PY

echo "worktree-gc-report.test: OK"
