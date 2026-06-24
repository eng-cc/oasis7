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
MAIN_WORKTREE="$TEST_REPO"
CURRENT_WORKTREE="$TMPDIR/current task worktree"
CLEAN_WORKTREE="$TMPDIR/task worktree with spaces"
OPEN_PR_WORKTREE="$TMPDIR/open-pr-task-worktree"
BROKEN_WORKTREE="$TMPDIR/broken-worktree"
PRUNABLE_WORKTREE="$TMPDIR/prunable-worktree"
PRUNABLE_MAIN_WORKTREE="$TMPDIR/prunable-main-worktree"

mkdir -p "$TEST_REPO/.pm/tasks" "$TEST_REPO/.git" "$TMPDIR/bin" "$CURRENT_WORKTREE/scripts" "$CLEAN_WORKTREE/target" "$CLEAN_WORKTREE/crates/oasis7_viewer/node_modules" "$OPEN_PR_WORKTREE" "$BROKEN_WORKTREE"
cp "$ROOT_DIR/scripts/worktree-gc-report.sh" "$CURRENT_WORKTREE/scripts/worktree-gc-report.sh"
cp "$ROOT_DIR/scripts/worktree-harness-lib.sh" "$CURRENT_WORKTREE/scripts/worktree-harness-lib.sh"
chmod +x "$CURRENT_WORKTREE/scripts/worktree-gc-report.sh"
printf 'target-cache' > "$CLEAN_WORKTREE/target/sample.bin"
printf 'node-cache' > "$CLEAN_WORKTREE/crates/oasis7_viewer/node_modules/sample.bin"

cat > "$TEST_REPO/.pm/tasks/task_11111111111111111111111111111111.yaml" <<EOF
task_uid: task_11111111111111111111111111111111
title: cleanup closed worktree
status: done
updated_at: 2026-04-24T11:12:00+08:00
worktree_hint: "$CLEAN_WORKTREE"
EOF

cat > "$TEST_REPO/.pm/tasks/task_22222222222222222222222222222222.yaml" <<EOF
task_uid: task_22222222222222222222222222222222
title: closed task points at main
status: done
updated_at: 2026-04-25T11:12:00+08:00
worktree_hint: "$MAIN_WORKTREE"
EOF

cat > "$TEST_REPO/.pm/tasks/task_33333333333333333333333333333333.yaml" <<EOF
task_uid: task_33333333333333333333333333333333
title: closed task with open pr branch
status: done
updated_at: 2026-04-26T11:12:00+08:00
worktree_hint: "$OPEN_PR_WORKTREE"
EOF

cat > "$TMPDIR/bin/git" <<EOF
#!/usr/bin/env bash
set -euo pipefail

repo_root="$TEST_REPO"
main_worktree="$MAIN_WORKTREE"
current_worktree="$CURRENT_WORKTREE"
clean_worktree="$CLEAN_WORKTREE"
open_pr_worktree="$OPEN_PR_WORKTREE"
broken_worktree="$BROKEN_WORKTREE"
prunable_worktree="$PRUNABLE_WORKTREE"
prunable_main_worktree="$PRUNABLE_MAIN_WORKTREE"

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
  printf 'worktree %s\nHEAD 1111111\nbranch refs/heads/main\n\n' "\$main_worktree"
  printf 'worktree %s\nHEAD 5555555\nbranch refs/heads/task/current\n\n' "\$current_worktree"
  printf 'worktree %s\nHEAD 2222222\nbranch refs/heads/task/review\$(rm)\n\n' "\$clean_worktree"
  printf 'worktree %s\nHEAD 7777777\nbranch refs/heads/task/open-pr\n\n' "\$open_pr_worktree"
  printf 'worktree %s\nHEAD 3333333\nbranch refs/heads/task/broken\n\n' "\$broken_worktree"
  printf 'worktree %s\nHEAD 4444444\nbranch refs/heads/task/prunable\nprunable gitdir file points to non-existent location\n\n' "\$prunable_worktree"
  printf 'worktree %s\nHEAD 6666666\nbranch refs/heads/main\nprunable gitdir file points to non-existent location\n\n' "\$prunable_main_worktree"
  exit 0
fi

if [[ "\${1:-}" == "merge-base" && "\${2:-}" == "--is-ancestor" ]]; then
  case "\${3:-}" in
    2222222 | 3333333 | 4444444 | 6666666)
      exit 0
      ;;
    7777777)
      exit 1
      ;;
    *)
      echo "unexpected merge-base commit: \${3:-}" >&2
      exit 2
      ;;
  esac
fi

if [[ "\${1:-}" == "-C" && "\${3:-}" == "status" && "\${4:-}" == "--short" ]]; then
  case "\$2" in
    "\$main_worktree" | "\$current_worktree" | "\$clean_worktree" | "\$open_pr_worktree")
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

cat > "$TMPDIR/bin/gh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "pr" && "${2:-}" == "list" ]]; then
  printf '[{"headRefName":"task/open-pr"}]\n'
  exit 0
fi

echo "unexpected gh invocation: $*" >&2
exit 1
EOF
chmod +x "$TMPDIR/bin/gh"

REPORT_FILE="$TMPDIR/worktree-gc-report.json"
NO_FOOTPRINT_REPORT_FILE="$TMPDIR/worktree-gc-report-no-footprint.json"
UNKNOWN_PR_STATE_REPORT_FILE="$TMPDIR/worktree-gc-report-unknown-pr-state.json"
TIMEOUT_PR_STATE_REPORT_FILE="$TMPDIR/worktree-gc-report-timeout-pr-state.json"
(cd "$CURRENT_WORKTREE" && PATH="$TMPDIR/bin:$PATH" ./scripts/worktree-gc-report.sh --json --footprint > "$REPORT_FILE")
(cd "$CURRENT_WORKTREE" && PATH="$TMPDIR/bin:$PATH" ./scripts/worktree-gc-report.sh --json > "$NO_FOOTPRINT_REPORT_FILE")

cat > "$TMPDIR/bin/gh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
exit 1
EOF
chmod +x "$TMPDIR/bin/gh"
(cd "$CURRENT_WORKTREE" && PATH="$TMPDIR/bin:$PATH" ./scripts/worktree-gc-report.sh --json > "$UNKNOWN_PR_STATE_REPORT_FILE")

cat > "$TMPDIR/bin/gh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
sleep 2
EOF
chmod +x "$TMPDIR/bin/gh"
(cd "$CURRENT_WORKTREE" && PATH="$TMPDIR/bin:$PATH" WORKTREE_GC_REPORT_GH_TIMEOUT_SECONDS=0.1 ./scripts/worktree-gc-report.sh --json > "$TIMEOUT_PR_STATE_REPORT_FILE")

python3 - "$REPORT_FILE" "$NO_FOOTPRINT_REPORT_FILE" "$UNKNOWN_PR_STATE_REPORT_FILE" "$TEST_REPO" "$CURRENT_WORKTREE" "$CLEAN_WORKTREE" "$OPEN_PR_WORKTREE" "$BROKEN_WORKTREE" "$PRUNABLE_WORKTREE" "$PRUNABLE_MAIN_WORKTREE" <<'PY'
from __future__ import annotations

import json
import shlex
import sys
from pathlib import Path

report_path = Path(sys.argv[1])
no_footprint_report_path = Path(sys.argv[2])
unknown_pr_state_report_path = Path(sys.argv[3])
repo_root = str(Path(sys.argv[4]).resolve())
current_worktree = str(Path(sys.argv[5]).resolve())
clean_worktree = str(Path(sys.argv[6]).resolve())
open_pr_worktree = str(Path(sys.argv[7]).resolve())
broken_worktree = str(Path(sys.argv[8]).resolve())
prunable_worktree = str(Path(sys.argv[9]).resolve())
prunable_main_worktree = str(Path(sys.argv[10]).resolve())

payload = json.loads(report_path.read_text(encoding="utf-8"))
no_footprint_payload = json.loads(no_footprint_report_path.read_text(encoding="utf-8"))
unknown_pr_state_payload = json.loads(unknown_pr_state_report_path.read_text(encoding="utf-8"))
if "footprint_included" in no_footprint_payload["summary"]:
    raise SystemExit(f"non-footprint summary should not gain footprint keys: {no_footprint_payload['summary']}")
if any("footprint" in entry for entry in no_footprint_payload["entries"]):
    raise SystemExit("non-footprint entries should not gain footprint keys")
expected_summary = {
    "total_worktrees": 7,
    "prunable_worktrees": 2,
    "dirty_worktrees": 0,
    "cleanup_candidates": 3,
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

main_entry = entries[repo_root]
if main_entry["cleanup_candidate"]:
    raise SystemExit(f"main worktree must not be a cleanup candidate: {main_entry}")
if main_entry["branch_delete_candidate"]:
    raise SystemExit(f"main branch must not be a delete candidate: {main_entry}")
if main_entry["cleanup_commands"]:
    raise SystemExit(f"main worktree must not have cleanup commands: {main_entry}")
if main_entry["protected_cleanup_reasons"] != ["canonical_repo_root", "main_branch"]:
    raise SystemExit(f"expected main protection reasons: {main_entry}")

current_entry = entries[current_worktree]
if not current_entry["current"] or current_entry["cleanup_candidate"]:
    raise SystemExit(f"current task worktree should stay non-candidate: {current_entry}")

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

open_pr_entry = entries[open_pr_worktree]
if open_pr_entry["cleanup_candidate"]:
    raise SystemExit(f"open PR worktree must not be a cleanup candidate: {open_pr_entry}")
if open_pr_entry["cleanup_commands"]:
    raise SystemExit(f"open PR worktree must not have cleanup commands: {open_pr_entry}")
if open_pr_entry["branch_delete_candidate"]:
    raise SystemExit(f"open PR branch must not be a delete candidate: {open_pr_entry}")
if open_pr_entry["protected_cleanup_reasons"] != ["open_pr", "branch_not_merged_to_main"]:
    raise SystemExit(f"expected open PR protection reason: {open_pr_entry}")
if open_pr_entry["open_pr"] is not True:
    raise SystemExit(f"expected open_pr flag: {open_pr_entry}")
if open_pr_entry["merged_to_main"] is not False:
    raise SystemExit(f"expected open PR branch to be marked unmerged: {open_pr_entry}")
if open_pr_entry["pr_state_known"] is not True:
    raise SystemExit(f"expected PR state to be known: {open_pr_entry}")

unknown_entries = {entry["path"]: entry for entry in unknown_pr_state_payload["entries"]}
unknown_clean_entry = unknown_entries[clean_worktree]
if unknown_pr_state_payload["summary"]["cleanup_candidates"] != 2:
    raise SystemExit(
        "unknown PR state should suppress closed-task cleanup candidates: "
        f"{unknown_pr_state_payload['summary']}"
    )
if unknown_clean_entry["cleanup_candidate"]:
    raise SystemExit(f"unknown PR state must protect clean done worktree: {unknown_clean_entry}")
if unknown_clean_entry["cleanup_commands"]:
    raise SystemExit(f"unknown PR state must not emit cleanup commands: {unknown_clean_entry}")
if unknown_clean_entry["protected_cleanup_reasons"] != ["open_pr_state_unknown"]:
    raise SystemExit(f"expected unknown PR state protection reason: {unknown_clean_entry}")
if unknown_clean_entry["pr_state_known"] is not False:
    raise SystemExit(f"expected PR state to be unknown: {unknown_clean_entry}")

broken_entry = entries[broken_worktree]
if broken_entry["dirty"] is not None or broken_entry["cleanup_candidate"]:
    raise SystemExit(f"expected broken worktree to stay non-candidate with dirty=null: {broken_entry}")

prunable_entry = entries[prunable_worktree]
if prunable_entry["cleanup_reasons"] != ["prunable_worktree"]:
    raise SystemExit(f"expected prunable cleanup reason: {prunable_entry}")
if not prunable_entry["branch_delete_candidate"]:
    raise SystemExit(f"expected branch delete candidate for prunable worktree: {prunable_entry}")

prunable_main_entry = entries[prunable_main_worktree]
if prunable_main_entry["cleanup_reasons"] != ["prunable_worktree"]:
    raise SystemExit(f"expected prunable main cleanup reason: {prunable_main_entry}")
if prunable_main_entry["protected_cleanup_reasons"] != ["main_branch"]:
    raise SystemExit(f"expected main branch protection reason: {prunable_main_entry}")
if not prunable_main_entry["cleanup_candidate"]:
    raise SystemExit(f"expected prunable main worktree to stay visible for cleanup: {prunable_main_entry}")
if prunable_main_entry["branch_delete_candidate"]:
    raise SystemExit(f"prunable main must not delete the main branch: {prunable_main_entry}")
expected_prunable_main_remove = "git -C {} worktree remove -f {}".format(
    shlex.quote(repo_root),
    shlex.quote(prunable_main_worktree),
)
if prunable_main_entry["cleanup_commands"] != [expected_prunable_main_remove]:
    raise SystemExit(f"unexpected prunable main cleanup commands: {prunable_main_entry['cleanup_commands']}")
PY

python3 - "$UNKNOWN_PR_STATE_REPORT_FILE" "$TIMEOUT_PR_STATE_REPORT_FILE" "$CLEAN_WORKTREE" "$OPEN_PR_WORKTREE" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

payloads = [
    json.loads(Path(sys.argv[1]).read_text(encoding="utf-8")),
    json.loads(Path(sys.argv[2]).read_text(encoding="utf-8")),
]
clean_worktree = str(Path(sys.argv[3]).resolve())
open_pr_worktree = str(Path(sys.argv[4]).resolve())

for payload in payloads:
    entries = {entry["path"]: entry for entry in payload["entries"]}
    for path in (clean_worktree, open_pr_worktree):
        entry = entries[path]
        if entry["cleanup_candidate"]:
            raise SystemExit(f"closed task worktree must fail closed when PR state is unknown: {entry}")
        if entry["cleanup_commands"]:
            raise SystemExit(f"closed task worktree must not emit cleanup commands when PR state is unknown: {entry}")
        if entry["branch_delete_candidate"]:
            raise SystemExit(f"closed task branch must not be a delete candidate when PR state is unknown: {entry}")
        if "open_pr_state_unknown" not in entry["protected_cleanup_reasons"]:
            raise SystemExit(f"expected PR-state protection reason: {entry}")
        if entry["pr_state_known"] is not False:
            raise SystemExit(f"expected PR state to be unknown: {entry}")
PY

echo "worktree-gc-report.test: OK"
