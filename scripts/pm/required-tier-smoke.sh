#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

OUTPUT_JSON=0
KEEP_TEMP=0

usage() {
  cat <<'USAGE'
Usage: ./scripts/pm/required-tier-smoke.sh [--json] [--keep-temp]

Run an isolated required-tier validation chain for the file-based PM runtime:
  seed evidence -> task execution log -> signal -> task/memory -> blocked task -> workflow/role/stage report

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
      echo "required-tier-smoke: unknown argument: $1" >&2
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

for backlog_path in (root / ".pm/roles").glob("*/backlog/*.yaml"):
    role = backlog_path.parts[-3]
    status = backlog_path.stem
    backlog_path.write_text(
        f"version: 1\nrole: {role}\nstatus: {status}\ntasks: []\n",
        encoding="utf-8",
    )
PY

cat > "$TMPDIR/.pm/evidence/bootstrap.md" <<'EOF'
# bootstrap evidence

- issue: viewer smoke blocked on startup bridge init
EOF

SIGNAL_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/promote-signal.sh" \
  --source-type bootstrap_evidence \
  --source-ref .pm/evidence/bootstrap.md \
  --role-hint qa_engineer \
  --severity high \
  --summary "viewer smoke blocked on startup bridge init" \
  --create-task \
  --related-prd doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.prd.md \
  --acceptance "blocked task exists in qa backlog" \
  --json)"

TASK_UID="$(python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["task"]["task_uid"])' <<<"$SIGNAL_JSON")"
TASK_LOG_PATH="$(python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["task"]["execution_log_path"])' <<<"$SIGNAL_JSON")"
cat > "$TMPDIR/$TASK_LOG_PATH" <<EOF
# $TASK_UID Execution Log

- task_uid: $TASK_UID
- title: viewer smoke blocked on startup bridge init
- owner_role: qa_engineer
- worktree_hint: null

## 2026-03-30 22:30:00 CST / qa_engineer
- 完成内容: viewer smoke blocked on startup bridge init.
- 遗留事项: needs escalation into candidate task and stage gate.
EOF

MOVE_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/move-task.sh" \
  --task-uid "$TASK_UID" \
  --to-status blocked \
  --json)"

QA_MEMORY_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/promote-memory.sh" \
  --signal-id "$(python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["signal_id"])' <<<"$SIGNAL_JSON")" \
  --role qa_engineer \
  --topic viewer.startup.blocker \
  --tag failure_signature \
  --tag gate \
  --promotion-reason failure_signature \
  --effective-at 2026-03-20T10:00:00+08:00 \
  --json)"

PRODUCER_SIGNAL_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/promote-signal.sh" \
  --source-type task_execution_log \
  --source-ref "$TASK_LOG_PATH" \
  --role-hint producer_system_designer \
  --severity medium \
  --summary "current stage remains internal_playable_alpha_late" \
  --json)"

PRODUCER_SIGNAL_ID="$(python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["signal_id"])' <<<"$PRODUCER_SIGNAL_JSON")"

PRODUCER_MEMORY_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/promote-memory.sh" \
  --signal-id "$PRODUCER_SIGNAL_ID" \
  --role producer_system_designer \
  --topic stage.current \
  --tag stage \
  --tag claim_envelope \
  --promotion-reason stage_decision \
  --json)"

SHARED_SIGNAL_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/promote-signal.sh" \
  --source-type task_execution_log \
  --source-ref "$TASK_LOG_PATH" \
  --role-hint producer_system_designer \
  --severity medium \
  --summary "claim envelope remains internal_only" \
  --json)"

SHARED_SIGNAL_ID="$(python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["signal_id"])' <<<"$SHARED_SIGNAL_JSON")"

SHARED_MEMORY_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/promote-memory.sh" \
  --signal-id "$SHARED_SIGNAL_ID" \
  --scope shared \
  --role producer_system_designer \
  --topic gate.claim_envelope \
  --tag claim_envelope \
  --promotion-reason stage_decision \
  --json)"

NOISE_SIGNAL_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/promote-signal.sh" \
  --source-type task_execution_log \
  --source-ref "$TASK_LOG_PATH" \
  --role-hint qa_engineer \
  --severity low \
  --summary "reran smoke once after cache clear" \
  --json)"

NOISE_SIGNAL_ID="$(python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["signal_id"])' <<<"$NOISE_SIGNAL_JSON")"

REJECTED_MEMORY_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/promote-memory.sh" \
  --signal-id "$NOISE_SIGNAL_ID" \
  --role qa_engineer \
  --reject-reason one_off_operation \
  --json)"

LIVEOPS_SIGNAL_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/promote-signal.sh" \
  --source-type incident \
  --source-ref "$TASK_LOG_PATH" \
  --role-hint liveops_community \
  --severity high \
  --summary "community escalation still needs owner follow-up" \
  --json)"

cat > "$TMPDIR/.pm/roles/producer_system_designer/memory/superseded.yaml" <<EOF
version: 1
role: producer_system_designer
kind: memory_superseded
records:
  - id: MEM-PRODUCER-0000
    role: producer_system_designer
    topic: stage.current
    summary: "current stage remained internal_playable_alpha_mid"
    source_refs:
      - $TASK_LOG_PATH
    tags:
      - stage
    effective_at: 2026-03-15T10:00:00+08:00
    last_reviewed_at: 2026-03-20T10:00:00+08:00
    status: superseded
    confidence: confirmed
    promotion_reason: stage_decision
    superseded_by: MEM-PRODUCER-0001
    superseded_at: 2026-03-30T10:00:00+08:00
    supersede_reason: stage_upgraded
EOF

if PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/stage-lint.sh" >/dev/null 2>&1; then
  echo "required-tier-smoke: expected stage-lint to fail before canonical stage files are updated" >&2
  exit 1
fi

SET_STAGE_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/set-stage.sh" \
  --current-stage internal_playable_alpha_late \
  --candidate-stage limited_preview_readiness \
  --claim-envelope internal_only \
  --decision-date 2026-03-30 \
  --gate-id GATE-ALPHA-001 \
  --gate-status blocked \
  --lane-status qa=blocked \
  --lane-status liveops=monitor \
  --blocking-task "$TASK_UID" \
  --source-ref "$TASK_LOG_PATH" \
  --json)"

FAILED_SET_STAGE_STDERR="$TMPDIR/set-stage-fail.stderr"
if PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/set-stage.sh" \
  --clear-blocking-tasks \
  --source-ref "$TASK_LOG_PATH" \
  --json > /dev/null 2>"$FAILED_SET_STAGE_STDERR"; then
  echo "required-tier-smoke: expected set-stage to fail when clearing a still-blocked task from blocking_tasks" >&2
  exit 1
fi
python3 - "$TMPDIR" "$TASK_UID" <<'PY'
from pathlib import Path
import sys

root = Path(sys.argv[1])
task_uid = sys.argv[2]

for path_str in (".pm/stage/current.yaml", ".pm/stage/gate.yaml"):
    text = (root / path_str).read_text(encoding="utf-8")
    if f"blocking_tasks:\n  - {task_uid}\n" not in text:
        raise SystemExit(f"failed set-stage should not persist cleared blocker in {path_str}")
PY

python3 - "$TMPDIR" "$TASK_UID" <<'PY'
from pathlib import Path
import sys

root = Path(sys.argv[1])
task_uid = sys.argv[2]

for path_str in (".pm/stage/current.yaml", ".pm/stage/gate.yaml"):
    path = root / path_str
    text = path.read_text(encoding="utf-8")
    needle = f"blocking_tasks:\n  - {task_uid}\n"
    if needle not in text:
        raise SystemExit(f"expected blocking task entry not found in {path}")
    path.write_text(text.replace(needle, "blocking_tasks: []\n"), encoding="utf-8")
PY

STAGE_DRIFT_STDERR="$TMPDIR/stage-drift.stderr"
if PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/stage-lint.sh" > /dev/null 2>"$STAGE_DRIFT_STDERR"; then
  echo "required-tier-smoke: expected stage-lint to fail when blocked task drifts out of stage/gate blocking_tasks" >&2
  exit 1
fi
if ! grep -q "blocked task missing from stage/gate blocking_tasks: $TASK_UID" "$STAGE_DRIFT_STDERR"; then
  echo "required-tier-smoke: stage drift failure did not mention missing blocked task $TASK_UID" >&2
  exit 1
fi

SET_STAGE_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/set-stage.sh" \
  --current-stage internal_playable_alpha_late \
  --candidate-stage limited_preview_readiness \
  --claim-envelope internal_only \
  --decision-date 2026-03-30 \
  --gate-id GATE-ALPHA-001 \
  --gate-status blocked \
  --lane-status qa=blocked \
  --lane-status liveops=monitor \
  --blocking-task "$TASK_UID" \
  --source-ref "$TASK_LOG_PATH" \
  --json)"

BROKEN_BACKLOG="$TMPDIR/.pm/roles/qa_engineer/backlog/blocked.yaml"
cp "$BROKEN_BACKLOG" "$BROKEN_BACKLOG.bak"
printf 'this is not a valid backlog doc\n' > "$BROKEN_BACKLOG"
WORKFLOW_FAIL_STDERR="$TMPDIR/workflow-report-fail.stderr"
if PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/workflow-report.sh" --role qa_engineer --phase start --task-uid "$TASK_UID" --json > /dev/null 2>"$WORKFLOW_FAIL_STDERR"; then
  echo "required-tier-smoke: expected workflow-report to fail when backlog input is malformed" >&2
  exit 1
fi
python3 - "$TMPDIR" "$TASK_UID" <<'PY'
from pathlib import Path
import sys
import yaml

root = Path(sys.argv[1])
task_uid = sys.argv[2]
task_path = root / f".pm/tasks/{task_uid}.yaml"
payload = yaml.safe_load(task_path.read_text(encoding="utf-8"))
if payload.get("last_started_at") not in (None, ""):
    raise SystemExit(f"workflow-report failure should not write last_started_at for {task_uid}")
if payload.get("last_closed_at") not in (None, ""):
    raise SystemExit(f"workflow-report failure should not write last_closed_at for {task_uid}")
PY
mv "$BROKEN_BACKLOG.bak" "$BROKEN_BACKLOG"

python3 - "$TMPDIR" <<'PY'
from pathlib import Path
import sys

root = Path(sys.argv[1])

registry_path = root / ".pm/registry/tasks.yaml"
if registry_path.exists():
    registry_path.unlink()

for backlog_path in (root / ".pm/roles").glob("*/backlog/*.yaml"):
    backlog_path.unlink()
PY

REGEN_ROLE_REPORT_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/role-report.sh" --role qa_engineer --json)"

python3 - "$TMPDIR" "$TASK_UID" "$REGEN_ROLE_REPORT_JSON" <<'PY'
from __future__ import annotations

import json
from pathlib import Path
import sys

root = Path(sys.argv[1])
task_uid = sys.argv[2]
report = json.loads(sys.argv[3])

registry_path = root / ".pm/registry/tasks.yaml"
if not registry_path.exists():
    raise SystemExit("role-report should regenerate .pm/registry/tasks.yaml when it is missing")

for role_dir in sorted((root / ".pm/roles").glob("*")):
    if not role_dir.is_dir():
        continue
    for lane in ("candidate", "committed", "blocked", "done"):
        backlog_path = role_dir / "backlog" / f"{lane}.yaml"
        if not backlog_path.exists():
            raise SystemExit(f"role-report should regenerate missing backlog view: {backlog_path}")

qa_payload = report["roles"]["qa_engineer"]
if qa_payload["backlog_counts"]["blocked"] != 1:
    raise SystemExit("role-report should still report one blocked qa task after regenerating views")
blocked_tasks = qa_payload["tasks"]["blocked"]
if len(blocked_tasks) != 1 or blocked_tasks[0]["task_uid"] != task_uid:
    raise SystemExit("role-report regenerated views but lost the blocked qa task entry")
PY

WORKFLOW_START_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/workflow-report.sh" --role qa_engineer --phase start --task-uid "$TASK_UID" --json)"
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/memory-lint.sh" >/dev/null
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/lint.sh" >/dev/null
MEMORY_REPORT_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/memory-report.sh" --json)"
ROLE_REPORT_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/role-report.sh" --role qa_engineer --json)"
WORKFLOW_CLOSE_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/workflow-report.sh" --role qa_engineer --phase close --task-uid "$TASK_UID" --json)"
mkdir -p "$TMPDIR/.pm/working_memory"
cat > "$TMPDIR/.pm/working_memory/$TASK_UID.yaml" <<EOF
version: 1
task_uid: $TASK_UID
role: qa_engineer
worktree_hint: null
entries:
  - entry_id: WM-0001
    entry_kind: decision
    summary: "viewer startup blocker should be reflected into follow-up review"
    source_refs:
      - $TASK_LOG_PATH
    captured_at: 2026-03-30T22:40:00+08:00
    expires_at: 2026-04-01T22:40:00+08:00
    promoted_to: []
EOF
WORKFLOW_CLOSE_WITH_WM_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/workflow-report.sh" --role qa_engineer --phase close --task-uid "$TASK_UID" --json)"
WORKFLOW_REVIEW_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/workflow-report.sh" --role producer_system_designer --phase review --json)"
STAGE_REPORT_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/stage-report.sh" --json)"

RESULT_JSON="$(python3 - "$TMPDIR" "$SIGNAL_JSON" "$MOVE_JSON" "$QA_MEMORY_JSON" "$PRODUCER_MEMORY_JSON" "$SHARED_MEMORY_JSON" "$REJECTED_MEMORY_JSON" "$LIVEOPS_SIGNAL_JSON" "$SET_STAGE_JSON" "$MEMORY_REPORT_JSON" "$ROLE_REPORT_JSON" "$REGEN_ROLE_REPORT_JSON" "$WORKFLOW_START_JSON" "$WORKFLOW_CLOSE_JSON" "$WORKFLOW_CLOSE_WITH_WM_JSON" "$WORKFLOW_REVIEW_JSON" "$STAGE_REPORT_JSON" <<'PY'
from __future__ import annotations

import json
import sys

signal_payload = json.loads(sys.argv[2])
move_payload = json.loads(sys.argv[3])
qa_memory = json.loads(sys.argv[4])
producer_memory = json.loads(sys.argv[5])
shared_memory = json.loads(sys.argv[6])
rejected_memory = json.loads(sys.argv[7])
liveops_signal = json.loads(sys.argv[8])
set_stage = json.loads(sys.argv[9])
memory_report = json.loads(sys.argv[10])
role_report = json.loads(sys.argv[11])
regen_role_report = json.loads(sys.argv[12])
workflow_start = json.loads(sys.argv[13])
workflow_close = json.loads(sys.argv[14])
workflow_close_with_wm = json.loads(sys.argv[15])
workflow_review = json.loads(sys.argv[16])
stage_report = json.loads(sys.argv[17])

if workflow_start["signal_summary"]["pending_count"] != 0:
    raise SystemExit("qa workflow start should not treat rejected signal as pending")
if workflow_start["task_context"]["task_uid"] != move_payload["task_uid"]:
    raise SystemExit("workflow start should bind explicit task_uid")
if not workflow_start["task_context"]["last_started_at"]:
    raise SystemExit("workflow start should record last_started_at")
if not workflow_close["task_context"]["last_closed_at"]:
    raise SystemExit("workflow close should record last_closed_at")
if workflow_close["working_memory_summary"]["entry_count"] != 0:
    raise SystemExit("workflow close should use task-scoped working_memory counts for explicit task_uid")
if workflow_review["signal_summary"]["pending_count"] != 1:
    raise SystemExit("producer workflow review should see one cross-role pending signal")
pending = workflow_review["signal_summary"]["pending_signals"]
if len(pending) != 1 or pending[0]["signal_id"] != liveops_signal["signal_id"]:
    raise SystemExit("producer workflow review missing expected liveops pending signal")
if pending[0]["role_hint"] != "liveops_community":
    raise SystemExit("producer workflow review pending signal role mismatch")
if any(item.get("id") == "review-signals" and "command" in item for item in workflow_review["checklist"]):
    raise SystemExit("workflow review checklist should not suggest promote-signal for pending signal handling")
if any(item.get("id") == "triage-signals" and "command" in item for item in workflow_start["checklist"]):
    raise SystemExit("workflow start checklist should not suggest promote-signal for pending signal handling")
if any(item.get("id") == "codex-review" for item in workflow_close["checklist"]):
    raise SystemExit("workflow close checklist should no longer require local codex review")
if not any(item.get("id") == "prepare-pr-review" for item in workflow_close["checklist"]):
    raise SystemExit("workflow close checklist should point to GitHub PR review as the default review boundary")
prepare_items = [item for item in workflow_close["checklist"] if item.get("id") == "prepare-pr-review"]
if prepare_items[0].get("command") != "./scripts/prepare-task-pr.sh":
    raise SystemExit("workflow close PR review checklist should point to prepare-task-pr.sh")
if not any(item.get("id") == "bootstrap-working-memory" for item in workflow_close["checklist"]):
    raise SystemExit("workflow close checklist should suggest bootstrapping working_memory when the current task has no entries")
bootstrap_items = [item for item in workflow_close["checklist"] if item.get("id") == "bootstrap-working-memory"]
if bootstrap_items[0].get("command") != f"./scripts/pm/codex-working-memory.sh --task-uid {move_payload['task_uid']} --role qa_engineer --session-id <session_id>":
    raise SystemExit("workflow close bootstrap command should require an explicit session_id by default")
if any(item.get("id") == "review-working-memory" for item in workflow_close["checklist"]):
    raise SystemExit("workflow close checklist should not suggest reviewing working_memory when the current task has no entries")
if any(item.get("id") == "autoflow-working-memory" for item in workflow_close["checklist"]):
    raise SystemExit("workflow close checklist should not suggest autoflow before the current task has working_memory entries")
if workflow_close_with_wm["working_memory_summary"]["entry_count"] != 1:
    raise SystemExit("workflow close with seeded working_memory should report one task-scoped entry")
if any(item.get("id") == "bootstrap-working-memory" for item in workflow_close_with_wm["checklist"]):
    raise SystemExit("workflow close with seeded working_memory should not suggest bootstrap")
if not any(item.get("id") == "review-working-memory" for item in workflow_close_with_wm["checklist"]):
    raise SystemExit("workflow close with seeded working_memory should suggest reviewing task-scoped working_memory")
if not any(item.get("id") == "autoflow-working-memory" for item in workflow_close_with_wm["checklist"]):
    raise SystemExit("workflow close with seeded working_memory should suggest autoflow for task-scoped working_memory")
if regen_role_report["roles"]["qa_engineer"]["backlog_counts"]["blocked"] != 1:
    raise SystemExit("regenerated role report should keep qa blocked count")

print(
    json.dumps(
        {
            "temp_root": sys.argv[1],
            "signal": signal_payload,
            "move": move_payload,
            "qa_memory": qa_memory,
            "producer_memory": producer_memory,
            "shared_memory": shared_memory,
            "rejected_memory": rejected_memory,
            "liveops_signal": liveops_signal,
            "set_stage": set_stage,
            "memory_report": memory_report,
            "role_report": role_report,
            "regen_role_report": regen_role_report,
            "workflow_start": workflow_start,
            "workflow_close": workflow_close,
            "workflow_close_with_wm": workflow_close_with_wm,
            "workflow_review": workflow_review,
            "stage_report": stage_report,
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

python3 - <<'PY' "$RESULT_JSON" "$TMPDIR"
from __future__ import annotations

import json
import sys

payload = json.loads(sys.argv[1])
temp_root = sys.argv[2]
stage = payload["stage_report"]
signal_id = payload["signal"]["signal_id"]
task_uid = payload["move"]["task_uid"]

print("required-tier smoke: OK")
print(f"- temp_root: {temp_root}")
print(f"- signal_id: {signal_id}")
print(f"- task_uid: {task_uid}")
print(f"- current_stage: {stage['current_stage']}")
print(f"- gate_status: {stage['gate']['status']}")
print(f"- blocked_tasks: {len(stage['blocking_tasks'])}")
print(f"- producer_active_memory: {len(stage['memory_inputs']['producer_active'])}")
print(f"- shared_active_memory: {len(stage['memory_inputs']['shared_active'])}")
print(f"- needs_review_memory: {payload['memory_report']['counts']['needs_review']}")
print(f"- superseded_memory: {payload['memory_report']['counts']['superseded']}")
print(f"- qa_blocked_tasks: {payload['role_report']['roles']['qa_engineer']['backlog_counts']['blocked']}")
print(f"- qa_pending_signals: {payload['workflow_start']['signal_summary']['pending_count']}")
print(f"- qa_close_actions: {len(payload['workflow_close']['checklist'])}")
print(f"- producer_pending_signals: {payload['workflow_review']['signal_summary']['pending_count']}")
print(f"- producer_review_actions: {len(payload['workflow_review']['checklist'])}")
print(f"- rejected_memory_signal: {payload['rejected_memory']['signal_id']}")
PY
