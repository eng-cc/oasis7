#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${PM_ROOT_DIR:-$(cd "$SCRIPT_DIR/../.." && pwd)}"
cd "$ROOT_DIR"
SOURCE_ROOT="$ROOT_DIR"

PM_LINT_TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-pm-lint.XXXXXX")"
cleanup() {
  rm -rf "$PM_LINT_TMP_DIR"
}
trap cleanup EXIT
PM_LINT_ROOT="$PM_LINT_TMP_DIR/root"
mkdir -p "$PM_LINT_ROOT"
copy_attempts="${PM_LINT_COPY_MAX_ATTEMPTS:-3}"
if [[ ! "$copy_attempts" =~ ^[1-9][0-9]*$ || "$copy_attempts" -gt 10 ]]; then
  echo "pm-lint: FAIL: PM_LINT_COPY_MAX_ATTEMPTS must be between 1 and 10" >&2
  exit 1
fi
copy_consistent=0
for ((copy_attempt = 1; copy_attempt <= copy_attempts; copy_attempt++)); do
  source_before="$($SCRIPT_DIR/tree-manifest.py --root "$SOURCE_ROOT/.pm" --reject-symlinks)"
  if [[ -n "${PM_LINT_COPY_READY_FILE:-}" ]]; then
    printf '%s\n' "$copy_attempt" >"$PM_LINT_COPY_READY_FILE"
    if [[ -n "${PM_LINT_COPY_CONTINUE_FILE:-}" ]]; then
      for _ in {1..500}; do
        [[ -f "$PM_LINT_COPY_CONTINUE_FILE" ]] && break
        sleep 0.01
      done
      if [[ ! -f "$PM_LINT_COPY_CONTINUE_FILE" ]]; then
        echo "pm-lint: FAIL: timed out waiting for PM_LINT_COPY_CONTINUE_FILE" >&2
        exit 1
      fi
    fi
  fi
  rm -rf "$PM_LINT_ROOT/.pm"
  cp -R "$SOURCE_ROOT/.pm" "$PM_LINT_ROOT/.pm"
  source_after="$($SCRIPT_DIR/tree-manifest.py --root "$SOURCE_ROOT/.pm")"
  snapshot_hash="$($SCRIPT_DIR/tree-manifest.py --root "$PM_LINT_ROOT/.pm")"
  if [[ "$source_before" == "$source_after" && "$source_after" == "$snapshot_hash" ]]; then
    copy_consistent=1
    break
  fi
  echo "pm-lint: source .pm changed during snapshot attempt $copy_attempt/$copy_attempts" >&2
done
if [[ "$copy_consistent" != "1" ]]; then
  echo "pm-lint: FAIL: could not capture a coherent .pm snapshot" >&2
  exit 1
fi
shopt -s dotglob nullglob
for path in "$ROOT_DIR"/*; do
  name="$(basename "$path")"
  [[ "$name" == ".pm" || "$name" == ".git" ]] && continue
  ln -s "$path" "$PM_LINT_ROOT/$name"
done
shopt -u dotglob nullglob

if [[ -n "${PM_LINT_SNAPSHOT_READY_FILE:-}" ]]; then
  : >"$PM_LINT_SNAPSHOT_READY_FILE"
  if [[ -n "${PM_LINT_CONTINUE_FILE:-}" ]]; then
    for _ in {1..500}; do
      [[ -f "$PM_LINT_CONTINUE_FILE" ]] && break
      sleep 0.01
    done
    if [[ ! -f "$PM_LINT_CONTINUE_FILE" ]]; then
      echo "pm-lint: FAIL: timed out waiting for PM_LINT_CONTINUE_FILE" >&2
      exit 1
    fi
  fi
fi

# All PM validation below reads one coherent snapshot epoch. Repository files
# outside .pm are read through snapshot-root symlinks and are never written.
ROOT_DIR="$PM_LINT_ROOT"
export PM_ROOT_DIR="$PM_LINT_ROOT"
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
require_dir ".pm/github-project-sync"
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
require_file "scripts/pm/audit-pr-watch-issues.py"
require_file "scripts/pm/audit-pr-watch-issues.sh"
require_file "scripts/pm/audit-pr-watch-issues.test.sh"
require_file "scripts/pm/append-execution-log.sh"
require_file "scripts/pm/claim-ready.sh"
require_file "scripts/pm/fallback-evidence.sh"
require_file "scripts/pm/lint.sh"
require_file "scripts/pm/lint.test.sh"
require_file "scripts/pm/tree-manifest.py"
require_file "scripts/pm/move-task.sh"
require_file "scripts/pm/new-task.sh"
require_file "scripts/pm/task-closeout.sh"
require_file "scripts/pm/workflow-report.sh"

if [[ "${PM_ALLOW_RETIRED_TASK_FILES:-0}" != "1" ]]; then
  while IFS= read -r path; do
    fail "retired .pm task file present after GitHub Project Step 3: $path"
  done < <(find .pm/tasks -type f \( -name 'task_*.yaml' -o -name '*.execution.md' \) 2>/dev/null | sort)
fi

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
  if [[ "$path" == .pm/roles/*/backlog/*.yaml ]]; then
    continue
  fi
  [[ -f "$path" ]] || fail "registry path missing: $path"
done < <(sed -n 's/^    [a-z_]*_path: //p; s/^  active_path: //p; s/^  superseded_path: //p' .pm/registry/roles.yaml)

python3 - <<'PY' || failures=$((failures + 1))
import json
import pathlib
import sys

mapping_path = pathlib.Path(".pm/github-project-sync/tasks.json")
archive_path = pathlib.Path(".pm/github-project-sync/task-archive.jsonl")
archive_records = [json.loads(line) for line in archive_path.read_text(encoding="utf-8").splitlines() if line.strip()]
if mapping_path.is_file():
    mapping = json.loads(mapping_path.read_text(encoding="utf-8"))
    tasks = mapping.get("tasks") or {}
    missing = [
        uid
        for uid, record in tasks.items()
        if not record.get("issue_url") or not record.get("issue_number") or not record.get("project_item_id")
    ]
    if missing:
        print(f"pm-lint: FAIL: {len(missing)} mapping records missing issue/project handles")
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
PYTHONPYCACHEPREFIX="$PM_LINT_TMP_DIR/pycache" python3 -m py_compile \
  "$SCRIPT_DIR/github-project-task.py" \
  "$SCRIPT_DIR/github-project-sync.py" \
  "$SCRIPT_DIR/github-project-workflow.py" \
  "$SCRIPT_DIR/github-project-retire-tasks.py" \
  "$SCRIPT_DIR/audit-pr-watch-issues.py"

echo "pm-lint: OK"
