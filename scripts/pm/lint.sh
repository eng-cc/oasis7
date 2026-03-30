#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

failures=0

fail() {
  echo "pm-lint: FAIL: $*"
  failures=$((failures + 1))
}

require_file() {
  local path="$1"
  [[ -f "$path" ]] || fail "missing file: $path"
}

require_dir() {
  local path="$1"
  [[ -d "$path" ]] || fail "missing directory: $path"
}

require_dir ".pm"
require_file ".pm/README.md"
require_file ".pm/registry/roles.yaml"
require_file ".pm/registry/tasks.yaml"
require_dir ".pm/inbox"
require_file ".pm/inbox/signals.jsonl"
require_dir ".pm/tasks"
require_file ".pm/stage/current.yaml"
require_file ".pm/stage/gate.yaml"
require_file ".pm/shared/memory/active.yaml"
require_file ".pm/shared/memory/superseded.yaml"
require_file ".pm/templates/role-memory-active.yaml"
require_file ".pm/templates/role-memory-superseded.yaml"
require_file ".pm/templates/role-backlog.yaml"
require_file ".pm/templates/role.yaml"
require_file ".pm/templates/task.yaml"
require_file ".pm/templates/signal.json"
require_file ".pm/templates/stage-current.yaml"
require_file ".pm/templates/stage-gate.yaml"
require_file "scripts/pm/scaffold.sh"
require_file "scripts/pm/new-task.sh"
require_file "scripts/pm/promote-signal.sh"
require_file "scripts/pm/lint.sh"
require_file "scripts/pm/stage-report.sh"
require_file "scripts/pm/role-report.sh"

mapfile -t CANONICAL_ROLES < <(find .agents/roles -mindepth 1 -maxdepth 1 -type f -name '*.md' -printf '%f\n' | sed 's/\.md$//' | sort)
mapfile -t REGISTRY_ROLES < <(sed -n 's/^  - role_name: //p' .pm/registry/roles.yaml | sort)

if [[ "${#CANONICAL_ROLES[@]}" -ne "${#REGISTRY_ROLES[@]}" ]]; then
  fail "role count mismatch: canonical=${#CANONICAL_ROLES[@]} registry=${#REGISTRY_ROLES[@]}"
fi

for role in "${CANONICAL_ROLES[@]}"; do
  if ! printf '%s\n' "${REGISTRY_ROLES[@]}" | grep -Fxq "$role"; then
    fail "registry missing canonical role: $role"
  fi
done

while IFS= read -r path; do
  [[ -f "$path" ]] || fail "registry path missing: $path"
done < <(sed -n 's/^    [a-z_]*_path: //p; s/^  active_path: //p; s/^  superseded_path: //p' .pm/registry/roles.yaml)

if ! grep -Eq '^next_sequence: [0-9]+$' .pm/registry/tasks.yaml; then
  fail "tasks registry missing numeric next_sequence"
fi

python3 - "$ROOT_DIR" <<'PY'
from __future__ import annotations

import json
import pathlib
import re
import sys

root = pathlib.Path(sys.argv[1])
failures: list[str] = []

roles = set()
for line in (root / ".pm/registry/roles.yaml").read_text(encoding="utf-8").splitlines():
    if line.startswith("  - role_name: "):
        roles.add(line.split(": ", 1)[1].strip())

def fail(message: str) -> None:
    failures.append(message)

def parse_registry_tasks(path: pathlib.Path) -> tuple[int | None, list[dict[str, str | None]]]:
    text = path.read_text(encoding="utf-8")
    next_sequence_match = re.search(r"^next_sequence: (\d+)$", text, re.MULTILINE)
    next_sequence = int(next_sequence_match.group(1)) if next_sequence_match else None
    entries: list[dict[str, str | None]] = []
    current: dict[str, str | None] | None = None
    for raw_line in text.splitlines():
        if raw_line.startswith("  - task_id: "):
            if current is not None:
                entries.append(current)
            current = {"task_id": raw_line.split(": ", 1)[1].strip()}
            continue
        if current is not None and raw_line.startswith("    "):
            key, _, value = raw_line.strip().partition(": ")
            current[key] = None if value == "null" else value
            continue
        if current is not None and raw_line.strip():
            entries.append(current)
            current = None
    if current is not None:
        entries.append(current)
    return next_sequence, entries

def parse_task_file(path: pathlib.Path) -> dict[str, str | None]:
    fields: dict[str, str | None] = {}
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        if raw_line.startswith("  - "):
            continue
        if not raw_line or raw_line.startswith(" "):
            continue
        key, _, value = raw_line.partition(": ")
        if not _:
            continue
        fields[key] = None if value == "null" else value
    return fields

def parse_candidate_backlog(path: pathlib.Path) -> set[str]:
    task_ids: set[str] = set()
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        if raw_line.startswith("  - task_id: "):
            task_ids.add(raw_line.split(": ", 1)[1].strip())
    return task_ids

signals_path = root / ".pm/inbox/signals.jsonl"
signal_ids: set[str] = set()
promoted_signal_ids: set[str] = set()
allowed_signal_states = {"new", "triaged", "promoted_candidate_task", "discarded", "deferred"}

for line_no, raw_line in enumerate(signals_path.read_text(encoding="utf-8").splitlines(), start=1):
    line = raw_line.strip()
    if not line:
        continue
    try:
        payload = json.loads(line)
    except json.JSONDecodeError as exc:
        fail(f"invalid JSON in .pm/inbox/signals.jsonl:{line_no}: {exc}")
        continue
    required_keys = {
        "signal_id",
        "source_type",
        "source_ref",
        "role_hint",
        "severity",
        "summary",
        "promotion_state",
    }
    missing = sorted(required_keys - payload.keys())
    if missing:
        fail(f"signal missing keys at .pm/inbox/signals.jsonl:{line_no}: {', '.join(missing)}")
        continue
    signal_id = payload["signal_id"]
    if signal_id in signal_ids:
        fail(f"duplicate signal_id: {signal_id}")
    signal_ids.add(signal_id)
    if payload["role_hint"] not in roles:
        fail(f"signal role_hint not registered: {signal_id} -> {payload['role_hint']}")
    if payload["promotion_state"] not in allowed_signal_states:
        fail(f"signal promotion_state invalid: {signal_id} -> {payload['promotion_state']}")
    if payload["promotion_state"] == "promoted_candidate_task":
        promoted_signal_ids.add(signal_id)

next_sequence, registry_entries = parse_registry_tasks(root / ".pm/registry/tasks.yaml")
registry_by_task_id: dict[str, dict[str, str | None]] = {}

for entry in registry_entries:
    task_id = entry.get("task_id")
    if not task_id:
        fail("registry task missing task_id")
        continue
    if task_id in registry_by_task_id:
        fail(f"duplicate registry task_id: {task_id}")
        continue
    registry_by_task_id[task_id] = entry
    for key in ("owner_role", "task_path", "status", "priority", "updated_at"):
        if not entry.get(key):
            fail(f"registry task missing {key}: {task_id}")
    owner_role = entry.get("owner_role")
    if owner_role and owner_role not in roles:
        fail(f"registry task owner_role not registered: {task_id} -> {owner_role}")
    task_path = entry.get("task_path")
    if task_path and not (root / task_path).is_file():
        fail(f"registry task path missing: {task_id} -> {task_path}")
    source_signal = entry.get("source_signal")
    if source_signal and source_signal not in signal_ids:
        fail(f"registry task source_signal missing from inbox: {task_id} -> {source_signal}")

task_files = sorted(path for path in (root / ".pm/tasks").glob("TASK-PM-*.yaml") if path.is_file())
if len(task_files) != len(registry_entries):
    fail(
        f"task file count mismatch: files={len(task_files)} registry={len(registry_entries)}"
    )

task_source_signals: set[str] = set()
for task_path in task_files:
    fields = parse_task_file(task_path)
    task_id = fields.get("task_id")
    if not task_id:
        fail(f"task file missing task_id: {task_path.relative_to(root)}")
        continue
    if task_path.name != f"{task_id}.yaml":
        fail(f"task file name mismatch: {task_path.relative_to(root)} -> {task_id}")
    owner_role = fields.get("owner_role")
    if owner_role not in roles:
        fail(f"task file owner_role not registered: {task_id} -> {owner_role}")
    registry_entry = registry_by_task_id.get(task_id)
    if registry_entry is None:
        fail(f"task file missing from registry: {task_id}")
    else:
        expected_path = registry_entry.get("task_path")
        actual_path = f".pm/tasks/{task_path.name}"
        if expected_path != actual_path:
            fail(f"registry task_path mismatch: {task_id} -> {expected_path} != {actual_path}")
        if registry_entry.get("owner_role") != owner_role:
            fail(f"registry owner_role mismatch: {task_id}")
        if registry_entry.get("status") != fields.get("status"):
            fail(f"registry status mismatch: {task_id}")
        if registry_entry.get("priority") != fields.get("priority"):
            fail(f"registry priority mismatch: {task_id}")
    source_signal = fields.get("source_signal")
    if source_signal:
        task_source_signals.add(source_signal)
        if source_signal not in signal_ids:
            fail(f"task source_signal missing from inbox: {task_id} -> {source_signal}")

for signal_id in promoted_signal_ids:
    if signal_id not in task_source_signals:
        fail(f"promoted signal has no task file: {signal_id}")

candidate_backlog_map = {
    role: parse_candidate_backlog(root / f".pm/roles/{role}/backlog/candidate.yaml")
    for role in roles
}

for task_id, entry in registry_by_task_id.items():
    if entry.get("status") != "candidate":
        continue
    owner_role = entry.get("owner_role")
    if owner_role and task_id not in candidate_backlog_map.get(owner_role, set()):
        fail(f"candidate task missing from owner backlog: {task_id} -> {owner_role}")

for role, task_ids in candidate_backlog_map.items():
    for task_id in task_ids:
        if task_id not in registry_by_task_id:
            fail(f"candidate backlog task missing from registry: {role} -> {task_id}")

if next_sequence is not None and registry_by_task_id:
    max_sequence = max(int(task_id.rsplit("-", 1)[1]) for task_id in registry_by_task_id)
    if next_sequence <= max_sequence:
        fail(
            f"next_sequence not ahead of existing tasks: next_sequence={next_sequence} max_task={max_sequence}"
        )

if failures:
    for failure in failures:
        print(f"pm-lint: FAIL: {failure}")
    raise SystemExit(1)
PY

if (( failures > 0 )); then
  exit 1
fi

echo "pm-lint: OK"
