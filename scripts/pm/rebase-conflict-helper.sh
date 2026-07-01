#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${PM_ROOT_DIR:-$(cd "$SCRIPT_DIR/../.." && pwd)}"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: ./scripts/pm/rebase-conflict-helper.sh [options]

Inspect `.pm/**` conflicts during an active git rebase and classify which ones
need manual handling after the local signal inbox retirement.

Default conventions:
- action: report only
- scope: conflicted `.pm/**` files from `git ls-files -u`
- no `.pm/**` path is automatically repaired

Options:
  --json              Print machine-readable JSON summary only
  -h, --help          Show help

Examples:
  ./scripts/pm/rebase-conflict-helper.sh
  ./scripts/pm/rebase-conflict-helper.sh --json
USAGE
}

die() {
  echo "pm-rebase-conflict-helper: $*" >&2
  exit 1
}

OUTPUT_JSON=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --json)
      OUTPUT_JSON=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "unknown option: $1"
      ;;
  esac
done

REBASE_IN_PROGRESS=0
if [[ -d "$(git rev-parse --git-path rebase-merge)" || -d "$(git rev-parse --git-path rebase-apply)" ]]; then
  REBASE_IN_PROGRESS=1
fi

PM_CONFLICT_RAW="$(git ls-files -u -- .pm || true)"

REPORT_JSON="$(
python3 - "$PM_CONFLICT_RAW" "$REBASE_IN_PROGRESS" <<'PY'
from __future__ import annotations

import json
import sys
from collections import defaultdict

raw = sys.argv[1]
rebase_in_progress = sys.argv[2] == "1"

path_to_stages: dict[str, list[int]] = defaultdict(list)
for line in raw.splitlines():
    if not line.strip():
        continue
    meta, path = line.split("\t", 1)
    stage = int(meta.split()[2])
    path_to_stages[path].append(stage)


def classify(path: str) -> tuple[str, str]:
    if path == ".pm/inbox/signals.jsonl":
        return ("retired_signal_inbox", "remove_retired_file_or_manual_archive")
    if path == ".pm/registry/tasks.yaml" or (
        path.startswith(".pm/roles/") and "/backlog/" in path and path.endswith(".yaml")
    ):
        return ("generated_view", "preserve_main_deletion_then_sync_views")
    if path.startswith(".pm/tasks/") and path.endswith(".execution.md"):
        return ("task_execution_log", "manual_merge")
    if path.startswith(".pm/tasks/") and path.endswith(".yaml"):
        return ("task_yaml", "manual_merge")
    if path.startswith(".pm/stage/") and path.endswith(".yaml"):
        return ("stage_yaml", "manual_merge")
    if (
        path.startswith(".pm/roles/")
        and "/memory/" in path
        and path.endswith(".yaml")
    ) or path.startswith(".pm/shared/memory/"):
        return ("memory_yaml", "manual_merge")
    return ("other_pm", "manual_merge")


conflicts = []
summary = {
    "total_conflicted_paths": 0,
    "retired_signal_conflicts": 0,
    "generated_view_conflicts": 0,
    "manual_conflicts": 0,
}

for path in sorted(path_to_stages):
    category, recommended_action = classify(path)
    conflicts.append(
        {
            "path": path,
            "category": category,
            "stages": sorted(path_to_stages[path]),
            "recommended_action": recommended_action,
        }
    )
    summary["total_conflicted_paths"] += 1
    if category == "retired_signal_inbox":
        summary["retired_signal_conflicts"] += 1
    elif category == "generated_view":
        summary["generated_view_conflicts"] += 1
    else:
        summary["manual_conflicts"] += 1

recommended_commands = []
if any(item["recommended_action"] == "preserve_main_deletion_then_sync_views" for item in conflicts):
    recommended_commands.append("./scripts/pm/sync-views.sh")

payload = {
    "rebase_in_progress": rebase_in_progress,
    "summary": summary,
    "resolved_now": {},
    "conflicts": conflicts,
    "recommended_commands": recommended_commands,
}
print(json.dumps(payload, ensure_ascii=False, indent=2))
PY
)"

if [[ "$OUTPUT_JSON" == "1" ]]; then
  printf '%s\n' "$REPORT_JSON"
  exit 0
fi

python3 - "$REPORT_JSON" <<'PY'
from __future__ import annotations

import json
import sys

payload = json.loads(sys.argv[1])

print("pm rebase conflict helper")
print(f"- rebase_in_progress: {str(payload['rebase_in_progress']).lower()}")
print(f"- total_conflicted_paths: {payload['summary']['total_conflicted_paths']}")
print(f"- retired_signal_conflicts: {payload['summary']['retired_signal_conflicts']}")
print(f"- generated_view_conflicts: {payload['summary']['generated_view_conflicts']}")
print(f"- manual_conflicts: {payload['summary']['manual_conflicts']}")

if not payload["conflicts"]:
    print("- details: none")
    raise SystemExit(0)

print("- details:")
for conflict in payload["conflicts"]:
    print(
        f"  - {conflict['category']} | {conflict['path']} | "
        f"stages={','.join(str(stage) for stage in conflict['stages'])} | "
        f"action={conflict['recommended_action']}"
    )

commands = payload["recommended_commands"]
if commands:
    print("- recommended_commands:")
    for command in commands:
      print(f"  - {command}")
PY
