#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${PM_ROOT_DIR:-$(cd "$SCRIPT_DIR/../.." && pwd)}"
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
require_file ".pm/registry/codex-sessions.yaml"
require_dir ".pm/inbox"
require_file ".pm/inbox/signals.jsonl"
require_dir ".pm/github-project-sync"
require_file ".pm/github-project-sync/tasks.json"
require_file ".pm/github-project-sync/task-archive.jsonl"
require_dir ".pm/working_memory"
require_file ".pm/stage/current.yaml"
require_file ".pm/stage/gate.yaml"
require_file ".pm/shared/memory/active.yaml"
require_file ".pm/shared/memory/superseded.yaml"

require_file "scripts/pm/github-project-task.py"
require_file "scripts/pm/github-project-task.test.sh"
require_file "scripts/pm/github-project-sync.py"
require_file "scripts/pm/github-project-sync.sh"
require_file "scripts/pm/github-project-sync.test.sh"
require_file "scripts/pm/github-project-workflow.py"
require_file "scripts/pm/github-project-workflow.sh"
require_file "scripts/pm/github-project-workflow.test.sh"
require_file "scripts/pm/github-project-retire-tasks.py"
require_file "scripts/pm/github-project-retire-tasks.sh"
require_file "scripts/pm/github-project-retire-tasks.test.sh"
require_file "scripts/pm/append-execution-log.sh"
require_file "scripts/pm/claim-ready.sh"
require_file "scripts/pm/lint.sh"
require_file "scripts/pm/move-task.sh"
require_file "scripts/pm/new-task.sh"
require_file "scripts/pm/task-closeout.sh"
require_file "scripts/pm/workflow-report.sh"

require_file "scripts/pm/memory-lint.sh"
require_file "scripts/pm/memory-report.sh"
require_file "scripts/pm/promote-memory.sh"
require_file "scripts/pm/promote-signal.sh"
require_file "scripts/pm/reflection-report.sh"
require_file "scripts/pm/stage-lint.sh"
require_file "scripts/pm/stage-report.sh"
require_file "scripts/pm/working-memory-lint.sh"
require_file "scripts/pm/working-memory-report.sh"

CANONICAL_ROLES=()
while IFS= read -r role; do
  CANONICAL_ROLES+=("$role")
done < <(find .agents/roles -mindepth 1 -maxdepth 1 -type f -name '*.md' | sed 's#^.*/##; s/\.md$//' | sort)

REGISTRY_ROLES=()
while IFS= read -r role; do
  REGISTRY_ROLES+=("$role")
done < <(sed -n 's/^  - role_name: //p' .pm/registry/roles.yaml | sort)

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

python3 - <<'PY' || failures=$((failures + 1))
import json
import pathlib
import sys

mapping_path = pathlib.Path(".pm/github-project-sync/tasks.json")
archive_path = pathlib.Path(".pm/github-project-sync/task-archive.jsonl")
mapping = json.loads(mapping_path.read_text(encoding="utf-8"))
tasks = mapping.get("tasks") or {}
missing = [uid for uid, record in tasks.items() if not record.get("issue_url") or not record.get("issue_number") or not record.get("project_item_id")]
archive_records = [json.loads(line) for line in archive_path.read_text(encoding="utf-8").splitlines() if line.strip()]
archive_uids = {record.get("task_uid") for record in archive_records}
if missing:
    print(f"pm-lint: FAIL: {len(missing)} mapping records missing issue/project handles")
    sys.exit(1)
if archive_records and not archive_uids.issubset(set(tasks)):
    print("pm-lint: FAIL: archive contains task_uid not present in mapping")
    sys.exit(1)
PY

if (( failures > 0 )); then
  exit 1
fi

./scripts/pm/memory-lint.sh >/dev/null
./scripts/pm/working-memory-lint.sh >/dev/null
./scripts/pm/stage-lint.sh >/dev/null
./scripts/pm/memory-report.sh --json >/dev/null
./scripts/pm/working-memory-report.sh --json >/dev/null
./scripts/pm/reflection-report.sh --json >/dev/null
python3 -m py_compile \
  "$SCRIPT_DIR/github-project-task.py" \
  "$SCRIPT_DIR/github-project-sync.py" \
  "$SCRIPT_DIR/github-project-workflow.py" \
  "$SCRIPT_DIR/github-project-retire-tasks.py"

echo "pm-lint: OK"
