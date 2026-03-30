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
  devlog -> signal -> candidate task/memory -> blocked task -> memory lint -> stage report

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
PY

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

PRODUCER_SIGNAL_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/promote-signal.sh" \
  --source-type devlog \
  --source-ref doc/devlog/2026-03-30.md \
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
  --source-type devlog \
  --source-ref doc/devlog/2026-03-30.md \
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
  --source-type devlog \
  --source-ref doc/devlog/2026-03-30.md \
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

python3 - "$TMPDIR" "$TASK_ID" <<'PY'
from pathlib import Path
import sys

root = Path(sys.argv[1])
task_id = sys.argv[2]

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

RESULT_JSON="$(python3 - "$TMPDIR" "$SIGNAL_JSON" "$MOVE_JSON" "$PRODUCER_MEMORY_JSON" "$SHARED_MEMORY_JSON" "$REJECTED_MEMORY_JSON" "$STAGE_REPORT_JSON" <<'PY'
from __future__ import annotations

import json
import sys

signal_payload = json.loads(sys.argv[2])
move_payload = json.loads(sys.argv[3])
producer_memory = json.loads(sys.argv[4])
shared_memory = json.loads(sys.argv[5])
rejected_memory = json.loads(sys.argv[6])
stage_report = json.loads(sys.argv[7])

print(
    json.dumps(
        {
            "temp_root": sys.argv[1],
            "signal": signal_payload,
            "move": move_payload,
            "producer_memory": producer_memory,
            "shared_memory": shared_memory,
            "rejected_memory": rejected_memory,
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
print(f"- rejected_memory_signal: {payload['rejected_memory']['signal_id']}")
PY
