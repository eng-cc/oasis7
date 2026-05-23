#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

OUTPUT_JSON=0
KEEP_TEMP=0

usage() {
  cat <<'USAGE'
Usage: ./scripts/pm/task-compaction-smoke.sh [--json] [--keep-temp]

Create a temporary PM root, seed one survivor task plus two completed micro-tasks,
assert compaction refuses while tracked docs still reference the dropped task UIDs,
then rewrite the doc trace, compact the task group, and verify only the survivor remains.

Options:
  --json       Print machine-readable JSON summary
  --keep-temp  Keep the temporary directory for inspection
  -h, --help   Show help
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --json)
      OUTPUT_JSON=1
      shift
      ;;
    --keep-temp)
      KEEP_TEMP=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "task-compaction-smoke: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

TMPDIR="$(mktemp -d)"
cleanup() {
  if [[ "$KEEP_TEMP" != "1" ]]; then
    rm -rf "$TMPDIR"
  fi
}
trap cleanup EXIT

mkdir -p "$TMPDIR/scripts"
cp -R "$ROOT_DIR/.pm" "$TMPDIR/.pm"
cp -R "$ROOT_DIR/.agents" "$TMPDIR/.agents"
cp -R "$ROOT_DIR/scripts/pm" "$TMPDIR/scripts/pm"
mkdir -p "$TMPDIR/doc/engineering/self-evolution"

python3 - "$TMPDIR" <<'PY'
from pathlib import Path
import sys

root = Path(sys.argv[1])

for active_path in (root / ".pm/roles").glob("*/memory/active.yaml"):
    role = active_path.parts[-3]
    active_path.write_text(
        f"version: 1\nrole: {role}\nkind: memory_active\nrecords: []\n",
        encoding="utf-8",
    )

for superseded_path in (root / ".pm/roles").glob("*/memory/superseded.yaml"):
    role = superseded_path.parts[-3]
    superseded_path.write_text(
        f"version: 1\nrole: {role}\nkind: memory_superseded\nrecords: []\n",
        encoding="utf-8",
    )

(root / ".pm/shared/memory/active.yaml").write_text(
    "version: 1\nscope: shared\nkind: memory_active\nrecords: []\n",
    encoding="utf-8",
)
(root / ".pm/shared/memory/superseded.yaml").write_text(
    "version: 1\nscope: shared\nkind: memory_superseded\nrecords: []\n",
    encoding="utf-8",
)
(root / ".pm/inbox/signals.jsonl").write_text("", encoding="utf-8")
(root / ".pm/stage/current.yaml").write_text(
    "version: 1\ncurrent_stage: null\ncandidate_stage: null\nclaim_envelope: null\ndecision_date: null\nupdated_from: []\nblocking_tasks: []\n",
    encoding="utf-8",
)
(root / ".pm/stage/gate.yaml").write_text(
    "version: 1\ngate_id: null\nstatus: draft\nlane_status: []\nblocking_tasks: []\nupdated_from: []\n",
    encoding="utf-8",
)

for path in (root / ".pm/tasks").glob("*.yaml"):
    path.unlink()
for path in (root / ".pm/tasks").glob("*.execution.md"):
    path.unlink()
for path in (root / ".pm/working_memory").glob("*.yaml"):
    path.unlink()

(root / ".pm/registry/tasks.yaml").write_text(
    'version: 2\nidentity_key: task_uid\ngenerated_from: ".pm/tasks/*.yaml"\ntasks: []\n',
    encoding="utf-8",
)
(root / ".pm/registry/codex-sessions.yaml").write_text("version: 1\nsessions: []\n", encoding="utf-8")

for backlog_path in (root / ".pm/roles").glob("*/backlog/*.yaml"):
    role = backlog_path.parts[-3]
    status = backlog_path.stem
    backlog_path.write_text(
        f"version: 1\nrole: {role}\nstatus: {status}\ntasks: []\n",
        encoding="utf-8",
    )
PY

cat > "$TMPDIR/doc/engineering/project.md" <<'EOF'
# temporary project

- survivor Trace: .pm/tasks/__SURVIVOR__.yaml
- micro Trace A: .pm/tasks/__DROP_A__.yaml
- micro Trace B: .pm/tasks/__DROP_B__.yaml
EOF

create_task() {
  local title="$1"
  local acceptance="$2"
  PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/new-task.sh" \
    --owner-role producer_system_designer \
    --title "$title" \
    --source-ref doc/engineering/project.md \
    --acceptance "$acceptance" \
    --json
}

SURVIVOR_JSON="$(create_task "aggregate workflow task" "survivor task remains canonical")"
DROP_A_JSON="$(create_task "doc truth refresh micro task" "drop task metadata gets merged")"
DROP_B_JSON="$(create_task "conflict table refresh micro task" "drop task is archived away")"

SURVIVOR_UID="$(python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["task_uid"])' <<<"$SURVIVOR_JSON")"
DROP_A_UID="$(python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["task_uid"])' <<<"$DROP_A_JSON")"
DROP_B_UID="$(python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["task_uid"])' <<<"$DROP_B_JSON")"

python3 - "$TMPDIR/doc/engineering/project.md" "$SURVIVOR_UID" "$DROP_A_UID" "$DROP_B_UID" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text(encoding="utf-8")
text = text.replace("__SURVIVOR__", sys.argv[2])
text = text.replace("__DROP_A__", sys.argv[3])
text = text.replace("__DROP_B__", sys.argv[4])
path.write_text(text, encoding="utf-8")
PY

append_log_entry() {
  local task_uid="$1"
  local title="$2"
  cat > "$TMPDIR/.pm/tasks/${task_uid}.execution.md" <<EOF
# ${task_uid} Execution Log

- task_uid: ${task_uid}
- title: ${title}
- owner_role: producer_system_designer
- worktree_hint: null

## 2026-05-22 18:00:00 CST / producer_system_designer
- 完成内容: close ${title}.
- 遗留事项: none.
EOF
}

append_log_entry "$SURVIVOR_UID" "aggregate workflow task"
append_log_entry "$DROP_A_UID" "doc truth refresh micro task"
append_log_entry "$DROP_B_UID" "conflict table refresh micro task"

PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/move-task.sh" --task-uid "$SURVIVOR_UID" --to-status done >/dev/null
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/move-task.sh" --task-uid "$DROP_A_UID" --to-status done >/dev/null
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/move-task.sh" --task-uid "$DROP_B_UID" --to-status deferred >/dev/null

REFUSAL_OUTPUT="$(
  PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/compact-task-group.sh" \
    --survivor-task-uid "$SURVIVOR_UID" \
    --drop-task-uid "$DROP_A_UID" \
    --drop-task-uid "$DROP_B_UID" \
    2>&1 || true
)"

if [[ "$REFUSAL_OUTPUT" != *"compact-task-group blocked by remaining tracked references"* ]]; then
  echo "task-compaction-smoke: expected reference refusal, got: $REFUSAL_OUTPUT" >&2
  exit 1
fi

cat > "$TMPDIR/doc/engineering/project.md" <<EOF
# temporary project

- survivor Trace: .pm/tasks/${SURVIVOR_UID}.yaml
EOF

SUMMARY_JSON="$(
  PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/compact-task-group.sh" \
    --survivor-task-uid "$SURVIVOR_UID" \
    --drop-task-uid "$DROP_A_UID" \
    --drop-task-uid "$DROP_B_UID" \
    --json
)"

RESULT_JSON="$(
  python3 - "$TMPDIR" "$SURVIVOR_UID" "$DROP_A_UID" "$DROP_B_UID" "$SUMMARY_JSON" <<'PY'
from pathlib import Path
import json
import sys

root = Path(sys.argv[1])
survivor_uid = sys.argv[2]
drop_a_uid = sys.argv[3]
drop_b_uid = sys.argv[4]
summary = json.loads(sys.argv[5])

survivor_path = root / ".pm/tasks" / f"{survivor_uid}.yaml"
if not survivor_path.is_file():
    raise SystemExit("survivor task file missing after compaction")
survivor_text = survivor_path.read_text(encoding="utf-8")
for expected in [
    "aggregate workflow task",
    "drop task metadata gets merged",
    "drop task is archived away",
]:
    if expected not in survivor_text:
        raise SystemExit(f"survivor task missing merged metadata: {expected}")

survivor_log = (root / ".pm/tasks" / f"{survivor_uid}.execution.md").read_text(encoding="utf-8")
if "将 2 个已关闭微任务并档回当前聚合 task" not in survivor_log:
    raise SystemExit("survivor execution log missing compaction entry")

for task_uid in (drop_a_uid, drop_b_uid):
    if (root / ".pm/tasks" / f"{task_uid}.yaml").exists():
        raise SystemExit(f"dropped task yaml still exists: {task_uid}")
    if (root / ".pm/tasks" / f"{task_uid}.execution.md").exists():
        raise SystemExit(f"dropped task execution log still exists: {task_uid}")

project_text = (root / "doc/engineering/project.md").read_text(encoding="utf-8")
for task_uid in (drop_a_uid, drop_b_uid):
    if task_uid in project_text:
        raise SystemExit(f"project doc still references dropped task uid: {task_uid}")

registry_text = (root / ".pm/registry/tasks.yaml").read_text(encoding="utf-8")
if registry_text.count("task_uid:") != 1:
    raise SystemExit("registry task count mismatch after compaction")

print(
    json.dumps(
        {
            "survivor_task_uid": survivor_uid,
            "dropped_task_uids": [drop_a_uid, drop_b_uid],
            "deleted_paths": summary["deleted_paths"],
            "reference_refusal_verified": True,
            "temp_root": str(root),
        },
        ensure_ascii=False,
    )
)
PY
)"

if [[ "$OUTPUT_JSON" == "1" ]]; then
  printf '%s\n' "$RESULT_JSON"
  exit 0
fi

cat <<INFO
task-compaction smoke passed
- temp root: $TMPDIR
- survivor task: $SURVIVOR_UID
- dropped tasks: $DROP_A_UID, $DROP_B_UID
- summary: $RESULT_JSON
INFO
