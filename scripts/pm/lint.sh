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

./scripts/pm/sync-views.sh >/dev/null

require_dir ".pm"
require_file ".pm/README.md"
require_file ".pm/registry/roles.yaml"
require_file ".pm/registry/tasks.yaml"
require_file ".pm/registry/codex-sessions.yaml"
require_dir ".pm/inbox"
require_file ".pm/inbox/signals.jsonl"
require_dir ".pm/tasks"
require_dir ".pm/working_memory"
require_file ".pm/stage/current.yaml"
require_file ".pm/stage/gate.yaml"
require_file ".pm/shared/memory/active.yaml"
require_file ".pm/shared/memory/superseded.yaml"
require_file ".pm/templates/role-memory-active.yaml"
require_file ".pm/templates/role-memory-superseded.yaml"
require_file ".pm/templates/role-backlog.yaml"
require_file ".pm/templates/role.yaml"
require_file ".pm/templates/task.yaml"
require_file ".pm/templates/task-execution-log.md"
require_file ".pm/templates/working-memory.yaml"
require_file ".pm/templates/signal.json"
require_file ".pm/templates/stage-current.yaml"
require_file ".pm/templates/stage-gate.yaml"
require_file "scripts/pm/codex-transcript-report.sh"
require_file "scripts/pm/codex-working-memory.sh"
require_file "scripts/pm/codex-working-memory-smoke.sh"
require_file "scripts/pm/lint.sh"
require_file "scripts/pm/memory-lint.sh"
require_file "scripts/pm/memory-report.sh"
require_file "scripts/pm/migrate-task-identity.sh"
require_file "scripts/pm/move-task.sh"
require_file "scripts/pm/new-task.sh"
require_file "scripts/pm/pm_store.py"
require_file "scripts/pm/promote-memory.sh"
require_file "scripts/pm/promote-signal.sh"
require_file "scripts/pm/required-tier-smoke.sh"
require_file "scripts/pm/reflection-report.sh"
require_file "scripts/pm/role-report.sh"
require_file "scripts/pm/scaffold.sh"
require_file "scripts/pm/set-stage.sh"
require_file "scripts/pm/stage-lint.sh"
require_file "scripts/pm/stage-report.sh"
require_file "scripts/pm/supersede-memory.sh"
require_file "scripts/pm/sync-views.sh"
require_file "scripts/pm/task-execution-log-lint.sh"
require_file "scripts/pm/working-memory-lint.sh"
require_file "scripts/pm/working-memory-report.sh"
require_file "scripts/pm/working-memory-autoflow.sh"
require_file "scripts/pm/working-memory-to-signal.sh"
require_file "scripts/pm/workflow-report.sh"
require_file "scripts/pm/schemas/codex-working-memory.schema.json"

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

if (( failures > 0 )); then
  exit 1
fi

./scripts/pm/memory-lint.sh >/dev/null
./scripts/pm/working-memory-lint.sh >/dev/null
./scripts/pm/stage-lint.sh >/dev/null
./scripts/pm/memory-report.sh --json >/dev/null
./scripts/pm/working-memory-report.sh --json >/dev/null
./scripts/pm/reflection-report.sh --json >/dev/null
./scripts/pm/role-report.sh --json >/dev/null
./scripts/pm/workflow-report.sh --role producer_system_designer --phase review --json >/dev/null
python3 "$SCRIPT_DIR/pm_store.py" task-lint "$ROOT_DIR"
./scripts/pm/task-execution-log-lint.sh >/dev/null

echo "pm-lint: OK"
