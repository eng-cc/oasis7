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
  devlog -> signal -> candidate task -> blocked task -> memory lint -> stage report

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

mkdir -p "$TMPDIR/scripts" "$TMPDIR/doc/devlog"
cp -R "$ROOT_DIR/.pm" "$TMPDIR/.pm"
cp -R "$ROOT_DIR/scripts/pm" "$TMPDIR/scripts/pm"

cat > "$TMPDIR/doc/devlog/2026-03-30.md" <<'EOF'
# 2026-03-30

## 22:30:00 CST / qa_engineer
- 完成内容: viewer smoke blocked on startup bridge init.
- 遗留事项: needs escalation into candidate task and stage gate.
EOF

SIGNAL_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/promote-signal.sh" \
  --source-type devlog \
  --source-ref doc/devlog/2026-03-30.md \
  --role-hint qa_engineer \
  --severity high \
  --summary "viewer smoke blocked on startup bridge init" \
  --create-task \
  --related-prd doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.prd.md \
  --acceptance "blocked task exists in qa backlog" \
  --json)"

TASK_ID="$(python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["task"]["task_id"])' <<<"$SIGNAL_JSON")"

MOVE_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/move-task.sh" \
  --task-id "$TASK_ID" \
  --to-status blocked \
  --json)"

python3 - "$TMPDIR" "$TASK_ID" <<'PY'
from pathlib import Path
import sys

root = Path(sys.argv[1])
task_id = sys.argv[2]

(root / ".pm/roles/producer_system_designer/memory/active.yaml").write_text(
    """version: 1
role: producer_system_designer
kind: memory_active
records:
  - id: MEM-PRODUCER-0001
    role: producer_system_designer
    topic: stage.current
    summary: "current stage remains internal_playable_alpha_late"
    source_refs:
      - doc/devlog/2026-03-30.md
    effective_at: 2026-03-30T22:30:00+08:00
    last_reviewed_at: 2026-03-30T22:30:00+08:00
    status: active
    confidence: confirmed
    promotion_reason: stage_decision
""",
    encoding="utf-8",
)

(root / ".pm/shared/memory/active.yaml").write_text(
    """version: 1
scope: shared
kind: memory_active
records:
  - id: MEM-SHARED-0001
    role: shared
    topic: gate.claim_envelope
    summary: "claim envelope remains internal_only"
    source_refs:
      - doc/devlog/2026-03-30.md
    effective_at: 2026-03-30T22:30:00+08:00
    last_reviewed_at: 2026-03-30T22:30:00+08:00
    status: active
    confidence: confirmed
    promotion_reason: stage_decision
""",
    encoding="utf-8",
)

(root / ".pm/stage/current.yaml").write_text(
    f"""version: 1
current_stage: internal_playable_alpha_late
candidate_stage: limited_preview_readiness
claim_envelope: internal_only
decision_date: 2026-03-30
updated_from:
  - doc/devlog/2026-03-30.md
blocking_tasks:
  - {task_id}
""",
    encoding="utf-8",
)

(root / ".pm/stage/gate.yaml").write_text(
    f"""version: 1
gate_id: GATE-ALPHA-001
status: blocked
lane_status:
  - qa=blocked
  - liveops=monitor
blocking_tasks:
  - {task_id}
updated_from:
  - doc/devlog/2026-03-30.md
""",
    encoding="utf-8",
)
PY

PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/memory-lint.sh" >/dev/null
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/lint.sh" >/dev/null
STAGE_REPORT_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/stage-report.sh" --json)"

RESULT_JSON="$(python3 - "$TMPDIR" "$SIGNAL_JSON" "$MOVE_JSON" "$STAGE_REPORT_JSON" <<'PY'
from __future__ import annotations

import json
import sys

signal_payload = json.loads(sys.argv[2])
move_payload = json.loads(sys.argv[3])
stage_report = json.loads(sys.argv[4])

print(
    json.dumps(
        {
            "temp_root": sys.argv[1],
            "signal": signal_payload,
            "move": move_payload,
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
task_id = payload["move"]["task_id"]

print("required-tier smoke: OK")
print(f"- temp_root: {temp_root}")
print(f"- signal_id: {signal_id}")
print(f"- task_id: {task_id}")
print(f"- current_stage: {stage['current_stage']}")
print(f"- gate_status: {stage['gate']['status']}")
print(f"- blocked_tasks: {len(stage['blocking_tasks'])}")
print(f"- producer_active_memory: {len(stage['memory_inputs']['producer_active'])}")
print(f"- shared_active_memory: {len(stage['memory_inputs']['shared_active'])}")
PY
