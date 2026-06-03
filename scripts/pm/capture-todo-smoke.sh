#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

TMPDIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

mkdir -p "$TMPDIR/scripts"
cp -R "$ROOT_DIR/.pm" "$TMPDIR/.pm"
cp -R "$ROOT_DIR/.agents" "$TMPDIR/.agents"
cp -R "$ROOT_DIR/scripts/pm" "$TMPDIR/scripts/pm"
mkdir -p "$TMPDIR/.pm/evidence"
printf 'discovered pre-task todo source\n' > "$TMPDIR/.pm/evidence/discovery.md"

SIGNAL_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/capture-todo.sh" \
  --signal-id SIG-PM-9001 \
  --source-ref .pm/evidence/discovery.md \
  --summary "capture a pre-task discovery without creating a task" \
  --json)"

python3 - "$TMPDIR" "$SIGNAL_JSON" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
payload = json.loads(sys.argv[2])
if payload["signal_id"] != "SIG-PM-9001":
    raise SystemExit("expected deterministic signal id")
if payload["promotion_state"] != "triaged":
    raise SystemExit("expected signal-only capture to remain triaged")
if payload["task"] is not None:
    raise SystemExit("default capture must not create a task")

signals = [
    json.loads(line)
    for line in (root / ".pm/inbox/signals.jsonl").read_text(encoding="utf-8").splitlines()
    if line.strip()
]
entry = signals[-1]
if entry["signal_id"] != "SIG-PM-9001":
    raise SystemExit("expected capture to append the requested signal")
if entry["source_type"] != "reflection":
    raise SystemExit("expected pre-task TODOs to use reflection source_type")
if entry["role_hint"] != "tpm":
    raise SystemExit("expected default role hint to be tpm")
if entry["severity"] != "low":
    raise SystemExit("expected default severity to be low")
PY

TASK_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/capture-todo.sh" \
  --signal-id SIG-PM-9002 \
  --source-ref .pm/evidence/discovery.md \
  --text "promote this discovery into a candidate task" \
  --role-hint agent_engineer \
  --severity medium \
  --create-task \
  --title "Promote pre-task discovery" \
  --owner-role qa_engineer \
  --priority P2 \
  --acceptance "candidate task is created only when requested" \
  --json)"

python3 - "$TASK_JSON" <<'PY'
from __future__ import annotations

import json
import sys

payload = json.loads(sys.argv[1])
if payload["signal_id"] != "SIG-PM-9002":
    raise SystemExit("expected second deterministic signal id")
if payload["promotion_state"] != "promoted_candidate_task":
    raise SystemExit("expected promoted candidate task state")
task = payload["task"]
if not task:
    raise SystemExit("expected created task payload")
if task["source_signal"] != "SIG-PM-9002":
    raise SystemExit("expected task to reference source signal")
if task["owner_role"] != "qa_engineer":
    raise SystemExit("expected explicit owner role passthrough")
if task["priority"] != "P2":
    raise SystemExit("expected explicit priority passthrough")
PY

echo "capture-todo-smoke: OK"
