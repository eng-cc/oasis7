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

PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/capture-todo.sh" \
  --source-ref .pm/evidence/discovery.md \
  --summary "capture a pre-task discovery with no passthrough args" \
  >"$TMPDIR/minimal.out"
grep -q "promote-signal: wrote SIG-PM-" "$TMPDIR/minimal.out" || {
  echo "capture-todo-smoke: minimal capture did not write a signal" >&2
  cat "$TMPDIR/minimal.out" >&2
  exit 1
}

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

set +e
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/capture-todo.sh" \
  --signal-id SIG-PM-9002 \
  --source-ref .pm/evidence/discovery.md \
  --summary "duplicate signal id should be rejected" \
  --json >"$TMPDIR/duplicate.out" 2>"$TMPDIR/duplicate.err"
DUPLICATE_STATUS=$?
set -e
if [[ "$DUPLICATE_STATUS" == "0" ]]; then
  echo "capture-todo-smoke: expected duplicate signal id to fail" >&2
  exit 1
fi
grep -q "duplicate signal_id: SIG-PM-9002" "$TMPDIR/duplicate.err" || {
  echo "capture-todo-smoke: duplicate signal id failure did not explain the conflict" >&2
  cat "$TMPDIR/duplicate.err" >&2
  exit 1
}

for index in 1 2 3 4 5 6 7 8; do
  PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/capture-todo.sh" \
    --source-ref .pm/evidence/discovery.md \
    --summary "concurrent capture smoke $index" \
    --role-hint repository_health_engineer \
    --json >"$TMPDIR/concurrent-$index.json" &
done
wait

python3 - "$TMPDIR" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
signals = [
    json.loads(line)
    for line in (root / ".pm/inbox/signals.jsonl").read_text(encoding="utf-8").splitlines()
    if line.strip()
]
signal_ids = [entry["signal_id"] for entry in signals]
if len(signal_ids) != len(set(signal_ids)):
    raise SystemExit(f"concurrent capture produced duplicate ids: {signal_ids}")
concurrent_payloads = [
    json.loads((root / f"concurrent-{index}.json").read_text(encoding="utf-8"))
    for index in range(1, 9)
]
concurrent_ids = [payload["signal_id"] for payload in concurrent_payloads]
if len(concurrent_ids) != 8 or len(concurrent_ids) != len(set(concurrent_ids)):
    raise SystemExit(f"concurrent capture outputs are not unique: {concurrent_ids}")
if not all(signal_id.startswith("SIG-PM-") for signal_id in concurrent_ids):
    raise SystemExit(f"unexpected concurrent signal ids: {concurrent_ids}")
PY

echo "capture-todo-smoke: OK"
