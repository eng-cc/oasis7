#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${PM_ROOT_DIR:-$(cd "$SCRIPT_DIR/../.." && pwd)}"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: ./scripts/pm/rebase-conflict-helper.sh [options]

Inspect `.pm/**` conflicts during an active git rebase, classify which ones are
safe to repair automatically, and optionally resolve `.pm/inbox/signals.jsonl`
signal-id collisions by preserving upstream ids and renumbering branch-local
entries.

Default conventions:
- action: report only
- scope: conflicted `.pm/**` files from `git ls-files -u`
- safe auto-fix boundary: `.pm/inbox/signals.jsonl` during rebase only

Options:
  --resolve-signals   Auto-resolve `.pm/inbox/signals.jsonl` rebase conflicts
  --json              Print machine-readable JSON summary only
  -h, --help          Show help

Examples:
  ./scripts/pm/rebase-conflict-helper.sh
  ./scripts/pm/rebase-conflict-helper.sh --json
  ./scripts/pm/rebase-conflict-helper.sh --resolve-signals --json
USAGE
}

die() {
  echo "pm-rebase-conflict-helper: $*" >&2
  exit 1
}

OUTPUT_JSON=0
RESOLVE_SIGNALS=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --resolve-signals)
      RESOLVE_SIGNALS=1
      shift
      ;;
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
SIGNAL_RENUMBERINGS_JSON="[]"

resolve_signals_conflict() {
  local path=".pm/inbox/signals.jsonl"
  local tmp_dir=""
  local ours_file=""
  local theirs_file=""
  local resolved_file=""
  local renumberings_file=""

  [[ "$REBASE_IN_PROGRESS" == "1" ]] || die "--resolve-signals requires an active git rebase"
  git ls-files -u -- "$path" | grep -q . || die "no conflicted $path entry found"

  tmp_dir="$(mktemp -d)"
  ours_file="$tmp_dir/ours.jsonl"
  theirs_file="$tmp_dir/theirs.jsonl"
  resolved_file="$tmp_dir/resolved.jsonl"
  renumberings_file="$tmp_dir/renumberings.json"

  if ! git show ":2:$path" > "$ours_file"; then
    rm -rf "$tmp_dir"
    die "failed to read upstream stage for $path"
  fi
  if ! git show ":3:$path" > "$theirs_file"; then
    rm -rf "$tmp_dir"
    die "failed to read branch stage for $path"
  fi

  if ! python3 - "$ours_file" "$theirs_file" "$resolved_file" "$renumberings_file" <<'PY'
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ours_path = Path(sys.argv[1])
theirs_path = Path(sys.argv[2])
resolved_path = Path(sys.argv[3])
renumberings_path = Path(sys.argv[4])

signal_pattern = re.compile(r"^SIG-PM-(\d+)$")


def load_jsonl(path: Path) -> list[dict[str, object]]:
    entries: list[dict[str, object]] = []
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line:
            continue
        entries.append(json.loads(line))
    return entries


def canonical(entry: dict[str, object]) -> str:
    return json.dumps(entry, ensure_ascii=False, sort_keys=True)


ours_entries = load_jsonl(ours_path)
theirs_entries = load_jsonl(theirs_path)
resolved_entries = list(ours_entries)
renumberings: list[dict[str, object]] = []

existing_ids = {
    entry.get("signal_id")
    for entry in resolved_entries
    if isinstance(entry.get("signal_id"), str)
}
existing_canonicals = {canonical(entry) for entry in resolved_entries}

max_signal_num = 0
for entry in ours_entries + theirs_entries:
    signal_id = entry.get("signal_id")
    if not isinstance(signal_id, str):
        continue
    match = signal_pattern.match(signal_id)
    if not match:
        continue
    max_signal_num = max(max_signal_num, int(match.group(1)))


def next_signal_id() -> str:
    global max_signal_num
    while True:
        max_signal_num += 1
        candidate = f"SIG-PM-{max_signal_num:04d}"
        if candidate not in existing_ids:
            return candidate


for entry in theirs_entries:
    entry_copy = dict(entry)
    current_canonical = canonical(entry_copy)
    if current_canonical in existing_canonicals:
        continue

    signal_id = entry_copy.get("signal_id")
    if isinstance(signal_id, str) and signal_id in existing_ids:
        new_signal_id = next_signal_id()
        entry_copy["signal_id"] = new_signal_id
        renumberings.append(
            {
                "old_signal_id": signal_id,
                "new_signal_id": new_signal_id,
                "summary": entry_copy.get("summary"),
                "source_ref": entry_copy.get("source_ref"),
            }
        )

    current_canonical = canonical(entry_copy)
    if current_canonical in existing_canonicals:
        continue

    resolved_entries.append(entry_copy)
    existing_canonicals.add(current_canonical)
    signal_id = entry_copy.get("signal_id")
    if isinstance(signal_id, str):
        existing_ids.add(signal_id)

resolved_path.write_text(
    "".join(json.dumps(entry, ensure_ascii=False) + "\n" for entry in resolved_entries),
    encoding="utf-8",
)
renumberings_path.write_text(
    json.dumps(renumberings, ensure_ascii=False),
    encoding="utf-8",
)
PY
  then
    rm -rf "$tmp_dir"
    die "failed to merge signal records for $path"
  fi

  if ! mv "$resolved_file" "$path"; then
    rm -rf "$tmp_dir"
    die "failed to write resolved $path"
  fi
  if ! git add "$path"; then
    rm -rf "$tmp_dir"
    die "failed to stage resolved $path"
  fi
  if ! SIGNAL_RENUMBERINGS_JSON="$(cat "$renumberings_file")"; then
    rm -rf "$tmp_dir"
    die "failed to read renumberings for $path"
  fi
  rm -rf "$tmp_dir"
}

if [[ "$RESOLVE_SIGNALS" == "1" ]]; then
  resolve_signals_conflict
  PM_CONFLICT_RAW="$(git ls-files -u -- .pm || true)"
fi

REPORT_JSON="$(
python3 - "$PM_CONFLICT_RAW" "$REBASE_IN_PROGRESS" "$SIGNAL_RENUMBERINGS_JSON" <<'PY'
from __future__ import annotations

import json
import sys
from collections import defaultdict

raw = sys.argv[1]
rebase_in_progress = sys.argv[2] == "1"
renumberings = json.loads(sys.argv[3])

path_to_stages: dict[str, list[int]] = defaultdict(list)
for line in raw.splitlines():
    if not line.strip():
        continue
    meta, path = line.split("\t", 1)
    stage = int(meta.split()[2])
    path_to_stages[path].append(stage)


def classify(path: str) -> tuple[str, str]:
    if path == ".pm/inbox/signals.jsonl":
        if rebase_in_progress:
            return ("signals_jsonl", "resolve_signals")
        return ("signals_jsonl", "manual_merge")
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
    "signals_conflicts": 0,
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
    if category == "signals_jsonl":
        summary["signals_conflicts"] += 1
    elif category == "generated_view":
        summary["generated_view_conflicts"] += 1
    else:
        summary["manual_conflicts"] += 1

recommended_commands = []
if any(item["recommended_action"] == "resolve_signals" for item in conflicts):
    recommended_commands.append("./scripts/pm/rebase-conflict-helper.sh --resolve-signals")
if any(item["recommended_action"] == "preserve_main_deletion_then_sync_views" for item in conflicts):
    recommended_commands.append("./scripts/pm/sync-views.sh")

payload = {
    "rebase_in_progress": rebase_in_progress,
    "summary": summary,
    "resolved_now": {
        "renumbered_signals": renumberings,
    },
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
print(f"- signals_conflicts: {payload['summary']['signals_conflicts']}")
print(f"- generated_view_conflicts: {payload['summary']['generated_view_conflicts']}")
print(f"- manual_conflicts: {payload['summary']['manual_conflicts']}")

renumbered = payload["resolved_now"]["renumbered_signals"]
if renumbered:
    print(f"- resolved_signals_now: {len(renumbered)}")

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

if renumbered:
    print("- renumbered_signals:")
    for item in renumbered:
        print(
            f"  - {item['old_signal_id']} -> {item['new_signal_id']} | "
            f"{item.get('summary') or '(no summary)'}"
        )

commands = payload["recommended_commands"]
if commands:
    print("- recommended_commands:")
    for command in commands:
      print(f"  - {command}")
PY
