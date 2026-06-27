#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${PM_ROOT_DIR:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

TMPDIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

mkdir -p "$TMPDIR/.agents/roles" "$TMPDIR/.pm/registry" "$TMPDIR/.pm/tasks" "$TMPDIR/scripts"
cp -R "$ROOT_DIR/scripts/pm" "$TMPDIR/scripts/pm"
printf '# Role: tpm\n' > "$TMPDIR/.agents/roles/tpm.md"
cat > "$TMPDIR/.pm/registry/roles.yaml" <<'YAML'
version: 1
roles:
  - role_name: tpm
YAML

cat > "$TMPDIR/.pm/tasks/task_11111111111111111111111111111111.yaml" <<'YAML'
task_uid: task_11111111111111111111111111111111
title: "first"
owner_role: tpm
module: engineering
worktree_hint: /tmp/first
execution_log_path: .pm/tasks/task_11111111111111111111111111111111.execution.md
status: committed
priority: P2
source_signal: null
source_refs: []
doc_refs: []
related_prd: []
acceptance: []
handoff_to: []
updated_at: 2026-06-27T00:00:00+08:00
YAML

cat > "$TMPDIR/.pm/tasks/task_22222222222222222222222222222222.yaml" <<'YAML'
task_uid: task_22222222222222222222222222222222
title: "second"
owner_role: tpm
module: engineering
worktree_hint: /tmp/second
execution_log_path: .pm/tasks/task_22222222222222222222222222222222.execution.md
status: done
priority: P3
source_signal: null
source_refs: []
doc_refs: []
related_prd: []
acceptance: []
handoff_to: []
updated_at: 2026-06-27T00:00:01+08:00
YAML

cat > "$TMPDIR/.pm/tasks/task_missing_uid.yaml" <<'YAML'
title: "missing uid"
owner_role: tpm
module: engineering
status: committed
priority: P3
YAML

python3 - "$TMPDIR" <<'PY'
from __future__ import annotations

import pathlib
import sys

root = pathlib.Path(sys.argv[1])
sys.path.insert(0, str(root / "scripts/pm"))

import pm_store  # type: ignore

original_load_mapping_document = pm_store.load_mapping_document
task_yaml_reads = 0


def counting_load_mapping_document(path):
    global task_yaml_reads
    resolved = pathlib.Path(path)
    if resolved.parent == root / ".pm/tasks" and resolved.suffix == ".yaml":
        task_yaml_reads += 1
    return original_load_mapping_document(path)


pm_store.load_mapping_document = counting_load_mapping_document
result = pm_store.sync_task_views(root)
_, registry_tasks = pm_store.load_list_document(root / ".pm/registry/tasks.yaml", "tasks")

if result["task_count"] != 3:
    raise SystemExit(f"expected task_count=3, got {result['task_count']}")
if result["role_count"] != 1:
    raise SystemExit(f"expected role_count=1, got {result['role_count']}")
if len(registry_tasks) != 2:
    raise SystemExit(f"expected registry to contain 2 valid tasks, got {len(registry_tasks)}")
if task_yaml_reads != 3:
    raise SystemExit(f"expected exactly one task yaml scan, got {task_yaml_reads} task yaml reads")

print("sync-views.test: OK")
PY
