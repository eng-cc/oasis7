#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
FIXTURE_TASK_UID="task_3eb31966906e5ae7b8b8676d756c5510"

OUTPUT_JSON=0
KEEP_TEMP=0

usage() {
  cat <<'USAGE'
Usage: ./scripts/pm/memory-regression-smoke.sh [--json] [--keep-temp]

Run isolated full-tier memory regression checks:
  - needs_review / superseded report output
  - report role filtering
  - role/workflow report backlog + memory aggregation
  - active topic conflict rejection
  - superseded chain rejection
  - new role expansion via registry + scaffold

Options:
  --json       Print machine-readable JSON summary
  --keep-temp  Keep the temporary PM root for inspection
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
      echo "memory-regression-smoke: unknown argument: $1" >&2
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
mkdir -p "$TMPDIR/.pm/evidence" "$TMPDIR/.pm/shared/memory" "$TMPDIR/.pm/stage"

python3 - "$TMPDIR" "$ROOT_DIR" <<'PY'
from pathlib import Path
import json
import shutil
import sys

import yaml

root = Path(sys.argv[1])
source_root = Path(sys.argv[2])


def mirror_source_ref(source_ref: str) -> None:
    path = str(source_ref).split("#", 1)[0].strip()
    if not path:
        return
    if path.startswith(("http://", "https://")):
        return
    resolved = Path(path).expanduser()
    if resolved.is_absolute():
        return
    target = root / resolved
    if target.exists():
        return
    source = source_root / resolved
    if not source.exists():
        return
    target.parent.mkdir(parents=True, exist_ok=True)
    if source.is_dir():
        shutil.copytree(source, target)
    else:
        shutil.copy2(source, target)


for task_path in (root / ".pm/tasks").glob("*.yaml"):
    payload = yaml.safe_load(task_path.read_text(encoding="utf-8")) or {}
    for source_ref in payload.get("source_refs") or []:
        mirror_source_ref(str(source_ref))
    execution_log = payload.get("execution_log_path")
    if execution_log:
        mirror_source_ref(str(execution_log))

for memory_path in list((root / ".pm/roles").glob("*/memory/*.yaml")) + list((root / ".pm/shared/memory").glob("*.yaml")):
    payload = yaml.safe_load(memory_path.read_text(encoding="utf-8")) or {}
    for record in payload.get("records") or []:
        for source_ref in record.get("source_refs") or []:
            mirror_source_ref(str(source_ref))

for working_memory_path in (root / ".pm/working_memory").glob("*.yaml"):
    payload = yaml.safe_load(working_memory_path.read_text(encoding="utf-8")) or {}
    for entry in payload.get("entries") or []:
        for source_ref in entry.get("source_refs") or []:
            mirror_source_ref(str(source_ref))

for stage_path in (root / ".pm/stage").glob("*.yaml"):
    payload = yaml.safe_load(stage_path.read_text(encoding="utf-8")) or {}
    for source_ref in payload.get("updated_from") or []:
        mirror_source_ref(str(source_ref))

signals_path = root / ".pm/inbox/signals.jsonl"
for raw_line in signals_path.read_text(encoding="utf-8").splitlines():
    raw_line = raw_line.strip()
    if not raw_line:
        continue
    mirror_source_ref(str(json.loads(raw_line).get("source_ref") or ""))
PY

python3 - "$TMPDIR" <<'PY'
from datetime import datetime
from pathlib import Path
import sys

root = Path(sys.argv[1])
fresh_ts = datetime.now().astimezone().isoformat(timespec="seconds")

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

(root / ".pm/evidence/bootstrap.md").write_text("# bootstrap evidence\n", encoding="utf-8")

(root / ".pm/roles/qa_engineer/memory/active.yaml").write_text(
    """version: 1
role: qa_engineer
kind: memory_active
records:
  - id: MEM-QA-0002
    role: qa_engineer
    topic: viewer.startup.blocker
    summary: "viewer startup blocker still needs fresh review"
    source_refs:
      - .pm/tasks/__FIXTURE_TASK_UID__.execution.md
    tags:
      - failure_signature
      - gate
    effective_at: 2026-03-15T09:00:00+08:00
    last_reviewed_at: 2026-03-15T09:00:00+08:00
    status: active
    confidence: confirmed
    promotion_reason: failure_signature
""",
    encoding="utf-8",
)
(root / ".pm/roles/qa_engineer/memory/superseded.yaml").write_text(
    """version: 1
role: qa_engineer
kind: memory_superseded
records:
  - id: MEM-QA-0001
    role: qa_engineer
    topic: viewer.startup.blocker
    summary: "older viewer startup blocker signature"
    source_refs:
      - .pm/tasks/__FIXTURE_TASK_UID__.execution.md
    tags:
      - failure_signature
    effective_at: 2026-03-10T09:00:00+08:00
    last_reviewed_at: 2026-03-15T09:00:00+08:00
    status: superseded
    confidence: confirmed
    promotion_reason: failure_signature
    superseded_by: MEM-QA-0002
    superseded_at: 2026-03-15T09:00:00+08:00
    supersede_reason: signature_refined
""",
    encoding="utf-8",
)
(root / ".pm/roles/producer_system_designer/memory/active.yaml").write_text(
    f"""version: 1
role: producer_system_designer
kind: memory_active
records:
  - id: MEM-PRODUCER-0008
    role: producer_system_designer
    topic: stage.current
    summary: "current stage remains internal_playable_alpha_late"
    source_refs:
      - .pm/tasks/__FIXTURE_TASK_UID__.execution.md
    tags:
      - stage
    effective_at: {fresh_ts}
    last_reviewed_at: {fresh_ts}
    status: active
    confidence: confirmed
    promotion_reason: stage_decision
""",
    encoding="utf-8",
)
(root / ".pm/shared/memory/active.yaml").write_text(
    f"""version: 1
scope: shared
kind: memory_active
records:
  - id: MEM-SHARED-0001
    role: shared
    topic: gate.claim_envelope
    summary: "claim envelope remains internal_only"
    source_refs:
      - .pm/tasks/__FIXTURE_TASK_UID__.execution.md
    tags:
      - claim_envelope
    effective_at: {fresh_ts}
    last_reviewed_at: {fresh_ts}
    status: active
    confidence: confirmed
    promotion_reason: stage_decision
""",
    encoding="utf-8",
)
PY

python3 - "$TMPDIR" "$FIXTURE_TASK_UID" <<'PY'
from pathlib import Path
import sys

root = Path(sys.argv[1])
fixture_task_uid = sys.argv[2]
for path in (
    root / ".pm/roles/qa_engineer/memory/active.yaml",
    root / ".pm/roles/qa_engineer/memory/superseded.yaml",
    root / ".pm/roles/producer_system_designer/memory/active.yaml",
    root / ".pm/shared/memory/active.yaml",
):
    path.write_text(
        path.read_text(encoding="utf-8").replace("__FIXTURE_TASK_UID__", fixture_task_uid),
        encoding="utf-8",
    )
PY

cat > "$TMPDIR/.agents/roles/report_smoke_engineer.md" <<'EOF'
# Role: report_smoke_engineer

## Mission
Smoke-only role for temporary PM registry expansion tests.
EOF

python3 - "$TMPDIR" <<'PY'
from pathlib import Path
import sys

root = Path(sys.argv[1])
registry_path = root / ".pm/registry/roles.yaml"
text = registry_path.read_text(encoding="utf-8")
entry = """  - role_name: report_smoke_engineer
    is_active: true
    introduced_at: 2026-03-31
    memory_active_path: .pm/roles/report_smoke_engineer/memory/active.yaml
    memory_superseded_path: .pm/roles/report_smoke_engineer/memory/superseded.yaml
    candidate_path: .pm/roles/report_smoke_engineer/backlog/candidate.yaml
    committed_path: .pm/roles/report_smoke_engineer/backlog/committed.yaml
    blocked_path: .pm/roles/report_smoke_engineer/backlog/blocked.yaml
    done_path: .pm/roles/report_smoke_engineer/backlog/done.yaml
"""
if "shared_memory:\n" not in text:
    raise SystemExit("memory-regression-smoke: shared_memory block missing in roles registry")
registry_path.write_text(text.replace("shared_memory:\n", entry + "shared_memory:\n", 1), encoding="utf-8")
PY

PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/scaffold.sh" report_smoke_engineer >/dev/null
TASK_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/new-task.sh" \
  --owner-role qa_engineer \
  --title "investigate stale viewer blocker" \
  --priority P1 \
  --source-ref .pm/evidence/bootstrap.md \
  --json)"
TASK_UID="$(python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["task_uid"])' <<<"$TASK_JSON")"
TASK_LOG_PATH="$(python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["execution_log_path"])' <<<"$TASK_JSON")"
cat > "$TMPDIR/$TASK_LOG_PATH" <<EOF
# $TASK_UID Execution Log

- task_uid: $TASK_UID
- title: investigate stale viewer blocker
- owner_role: qa_engineer
- worktree_hint: null

## 2026-03-31 10:00:00 CST / qa_engineer
- 完成内容: memory regression smoke fixture.
- 遗留事项: stale blocker still needs review.
EOF
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/move-task.sh" --task-uid "$TASK_UID" --to-status committed >/dev/null
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/workflow-report.sh" --role qa_engineer --phase start --task-uid "$TASK_UID" --json >/dev/null
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/move-task.sh" --task-uid "$TASK_UID" --to-status blocked >/dev/null
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/set-stage.sh" \
  --current-stage internal_playable_alpha_late \
  --claim-envelope internal_only \
  --decision-date 2026-03-31 \
  --gate-id GATE-SMOKE-001 \
  --gate-status blocked \
  --lane-status qa=blocked \
  --blocking-task "$TASK_UID" \
  --source-ref "$TASK_LOG_PATH" >/dev/null
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/memory-lint.sh" >/dev/null
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/lint.sh" >/dev/null

REPORT_JSON_PATH="$TMPDIR/report.json"
QA_REPORT_JSON_PATH="$TMPDIR/qa-report.json"
ROLE_REPORT_JSON_PATH="$TMPDIR/role-report.json"
QA_ROLE_REPORT_JSON_PATH="$TMPDIR/qa-role-report.json"
EXPANDED_WORKFLOW_JSON_PATH="$TMPDIR/expanded-workflow.json"

PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/memory-report.sh" --json > "$REPORT_JSON_PATH"
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/memory-report.sh" --role qa_engineer --no-shared --json > "$QA_REPORT_JSON_PATH"
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/role-report.sh" --json > "$ROLE_REPORT_JSON_PATH"
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/role-report.sh" --role qa_engineer --json > "$QA_ROLE_REPORT_JSON_PATH"
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/workflow-report.sh" --role report_smoke_engineer --phase start --json > "$EXPANDED_WORKFLOW_JSON_PATH"

python3 - "$REPORT_JSON_PATH" "$QA_REPORT_JSON_PATH" "$ROLE_REPORT_JSON_PATH" "$QA_ROLE_REPORT_JSON_PATH" "$EXPANDED_WORKFLOW_JSON_PATH" "$TASK_UID" <<'PY'
from __future__ import annotations

import json
from pathlib import Path
import sys

report = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
qa_report = json.loads(Path(sys.argv[2]).read_text(encoding="utf-8"))
role_report = json.loads(Path(sys.argv[3]).read_text(encoding="utf-8"))
qa_role_report = json.loads(Path(sys.argv[4]).read_text(encoding="utf-8"))
expanded_workflow = json.loads(Path(sys.argv[5]).read_text(encoding="utf-8"))
task_uid = sys.argv[6]

if report["counts"] != {"active": 3, "needs_review": 1, "superseded": 1}:
    raise SystemExit(f"unexpected report counts: {report['counts']}")
if report["roles"]["report_smoke_engineer"] != {"active": 0, "needs_review": 0, "superseded": 0}:
    raise SystemExit("report missing zero-count expanded role")
if len(report["needs_review"]) != 1 or report["needs_review"][0]["id"] != "MEM-QA-0002":
    raise SystemExit("needs_review output missing expected QA record")
if qa_report["counts"] != {"active": 1, "needs_review": 1, "superseded": 1}:
    raise SystemExit(f"unexpected qa_report counts: {qa_report['counts']}")
if list(qa_report["roles"].keys()) != ["qa_engineer"]:
    raise SystemExit("qa_report should only contain qa_engineer role summary")
if role_report["roles"]["report_smoke_engineer"]["backlog_counts"] != {"candidate": 0, "committed": 0, "blocked": 0, "done": 0, "deferred": 0}:
    raise SystemExit("role_report missing zero-count expanded role backlog summary")
if expanded_workflow["role_report"]["backlog_counts"] != {"candidate": 0, "committed": 0, "blocked": 0, "done": 0, "deferred": 0}:
    raise SystemExit("workflow_report missing zero-count expanded role backlog summary")
if expanded_workflow["signal_summary"]["pending_count"] != 0:
    raise SystemExit("workflow_report pending signal count mismatch for expanded role")
if role_report["roles"]["qa_engineer"]["backlog_counts"]["blocked"] != 1:
    raise SystemExit("role_report missing blocked QA task count")
if role_report["roles"]["qa_engineer"]["tasks"]["blocked"][0]["task_uid"] != task_uid:
    raise SystemExit("role_report missing expected blocked QA task")
if qa_role_report["role_filter"] != "qa_engineer":
    raise SystemExit("qa_role_report filter mismatch")
if qa_role_report["roles"]["qa_engineer"]["memory_counts"] != {"active": 1, "needs_review": 1, "superseded": 1}:
    raise SystemExit("qa_role_report memory summary mismatch")
PY

python3 - "$TMPDIR" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1]) / ".pm/roles/qa_engineer/memory/active.yaml"
text = path.read_text(encoding="utf-8")
text += """  - id: MEM-QA-0003
    role: qa_engineer
    topic: viewer.startup.blocker
    summary: "duplicate blocker topic"
    source_refs:
      - .pm/tasks/__FIXTURE_TASK_UID__.execution.md
    tags:
      - failure_signature
    effective_at: 2026-03-31T11:00:00+08:00
    last_reviewed_at: 2026-03-31T11:00:00+08:00
    status: active
    confidence: confirmed
    promotion_reason: failure_signature
"""
path.write_text(text, encoding="utf-8")
PY

CONFLICT_OUTPUT="$(
  set +e
  PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/memory-lint.sh" 2>&1
  echo "exit:$?"
)"
if [[ "$CONFLICT_OUTPUT" != *"active memory topic conflict"* ]] || [[ "$CONFLICT_OUTPUT" != *"exit:1"* ]]; then
  echo "memory-regression-smoke: expected active conflict failure" >&2
  exit 1
fi

python3 - "$TMPDIR" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1]) / ".pm/roles/qa_engineer/memory/active.yaml"
path.write_text(
    """version: 1
role: qa_engineer
kind: memory_active
records:
  - id: MEM-QA-0002
    role: qa_engineer
    topic: viewer.startup.blocker
    summary: "viewer startup blocker still needs fresh review"
    source_refs:
      - .pm/tasks/__FIXTURE_TASK_UID__.execution.md
    tags:
      - failure_signature
      - gate
    effective_at: 2026-03-15T09:00:00+08:00
    last_reviewed_at: 2026-03-15T09:00:00+08:00
    status: active
    confidence: confirmed
    promotion_reason: failure_signature
""",
    encoding="utf-8",
)
superseded_path = Path(sys.argv[1]) / ".pm/roles/qa_engineer/memory/superseded.yaml"
superseded_path.write_text(
    """version: 1
role: qa_engineer
kind: memory_superseded
records:
  - id: MEM-QA-0001
    role: qa_engineer
    topic: viewer.startup.blocker
    summary: "older viewer startup blocker signature"
    source_refs:
      - .pm/tasks/__FIXTURE_TASK_UID__.execution.md
    tags:
      - failure_signature
    effective_at: 2026-03-10T09:00:00+08:00
    last_reviewed_at: 2026-03-15T09:00:00+08:00
    status: superseded
    confidence: confirmed
    promotion_reason: failure_signature
    superseded_by: MEM-QA-9999
    superseded_at: 2026-03-15T09:00:00+08:00
    supersede_reason: signature_refined
""",
    encoding="utf-8",
)
PY

python3 - "$TMPDIR" "$FIXTURE_TASK_UID" <<'PY'
from pathlib import Path
import sys

root = Path(sys.argv[1])
fixture_task_uid = sys.argv[2]
for path in (
    root / ".pm/roles/qa_engineer/memory/active.yaml",
    root / ".pm/roles/qa_engineer/memory/superseded.yaml",
):
    path.write_text(
        path.read_text(encoding="utf-8").replace("__FIXTURE_TASK_UID__", fixture_task_uid),
        encoding="utf-8",
    )
PY

CHAIN_OUTPUT="$(
  set +e
  PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/memory-lint.sh" 2>&1
  echo "exit:$?"
)"
if [[ "$CHAIN_OUTPUT" != *"superseded_by missing target"* ]] || [[ "$CHAIN_OUTPUT" != *"exit:1"* ]]; then
  echo "memory-regression-smoke: expected superseded chain failure" >&2
  exit 1
fi

RESULT_JSON="$(python3 - "$TMPDIR" "$REPORT_JSON_PATH" "$QA_REPORT_JSON_PATH" "$ROLE_REPORT_JSON_PATH" "$QA_ROLE_REPORT_JSON_PATH" "$TASK_UID" <<'PY'
from __future__ import annotations

import json
from pathlib import Path
import sys

print(
    json.dumps(
        {
            "temp_root": sys.argv[1],
            "report": json.loads(Path(sys.argv[2]).read_text(encoding="utf-8")),
            "qa_report": json.loads(Path(sys.argv[3]).read_text(encoding="utf-8")),
            "role_report": json.loads(Path(sys.argv[4]).read_text(encoding="utf-8")),
            "qa_role_report": json.loads(Path(sys.argv[5]).read_text(encoding="utf-8")),
            "blocked_task_uid": sys.argv[6],
            "conflict_failure": "active memory topic conflict",
            "chain_failure": "superseded_by missing target",
            "expanded_role": "report_smoke_engineer",
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

python3 - <<'PY' "$RESULT_JSON"
from __future__ import annotations

import json
import sys

payload = json.loads(sys.argv[1])
print("memory regression smoke: OK")
print(f"- temp_root: {payload['temp_root']}")
print(f"- active_count: {payload['report']['counts']['active']}")
print(f"- needs_review_count: {payload['report']['counts']['needs_review']}")
print(f"- superseded_count: {payload['report']['counts']['superseded']}")
print(f"- expanded_role: {payload['expanded_role']}")
print(f"- qa_blocked_task_uid: {payload['blocked_task_uid']}")
print(f"- conflict_failure: {payload['conflict_failure']}")
print(f"- chain_failure: {payload['chain_failure']}")
PY
