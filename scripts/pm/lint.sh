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

if (( failures > 0 )); then
  exit 1
fi

echo "pm-lint: OK"
