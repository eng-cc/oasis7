#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

usage() {
  cat <<'USAGE'
Usage: ./scripts/doc-inventory-report.sh

Generates a Markdown inventory report for the current `doc/` corpus, including:
  - total Markdown count under doc/
  - per-module density
  - top hotspot subdirectories
  - doc/devlog backlog
  - non-devlog near-limit files
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ $# -ne 0 ]]; then
  usage
  exit 1
fi

python3 - <<'PY'
from collections import Counter
from datetime import datetime
from pathlib import Path

TOTAL_ALERT = 1500
MODULE_ALERT = 200
SUBDIR_ALERT = 80
DEVLOG_ALERT = 50
DEVLOG_LINE_ALERT = 2000
NEAR_LIMIT = 850

root = Path("doc")
files = sorted(root.rglob("*.md"))
module_counts = Counter()
subdir_counts = Counter()
line_counts = []
devlog_rows = []
non_devlog_near_limit = []

for path in files:
    rel = path.relative_to(root)
    parts = rel.parts
    if len(parts) >= 2:
        module_counts[parts[0]] += 1
    elif parts:
        module_counts["(doc-root)"] += 1
    if len(parts) >= 3:
        parent_parts = rel.parent.parts
        subdir_counts[f"{parent_parts[0]}/{parent_parts[1]}"] += 1
    try:
        line_count = sum(1 for _ in path.open("r", encoding="utf-8"))
    except UnicodeDecodeError:
        line_count = sum(1 for _ in path.open("r", encoding="utf-8", errors="ignore"))
    line_counts.append((line_count, path.as_posix()))
    if path.parts[:2] == ("doc", "devlog") and path.name != "README.md":
        devlog_rows.append((line_count, path.as_posix()))
    elif line_count >= NEAR_LIMIT:
        non_devlog_near_limit.append((line_count, path.as_posix()))

total_docs = len(files)
devlog_count = len(devlog_rows)
largest_doc = max(line_counts, default=(0, "N/A"))
largest_devlog = max(devlog_rows, default=(0, "N/A"))
now = datetime.now().astimezone().isoformat(timespec="seconds")

def status(flag: bool, label_true: str, label_false: str = "normal") -> str:
    return label_true if flag else label_false

print("# Doc Inventory Report")
print()
print(f"- Generated At: {now}")
print(f"- Total Markdown Files: {total_docs} ({status(total_docs >= TOTAL_ALERT, 'action_required')})")
print(f"- doc/devlog Files: {devlog_count} ({status(devlog_count >= DEVLOG_ALERT, 'action_required')})")
print(f"- Largest Markdown File: `{largest_doc[1]}` ({largest_doc[0]} lines)")
print(f"- Largest devlog File: `{largest_devlog[1]}` ({largest_devlog[0]} lines, {status(largest_devlog[0] >= DEVLOG_LINE_ALERT, 'action_required')})")
print()

print("## Module Density")
print("| Module | Files | Status |")
print("| --- | --- | --- |")
for module, count in module_counts.most_common():
    print(f"| `{module}` | {count} | {status(count >= MODULE_ALERT, 'action_required')} |")
print()

print("## Hotspot Subdirectories")
print("| Subdirectory | Files | Status |")
print("| --- | --- | --- |")
for subdir, count in sorted(subdir_counts.items(), key=lambda item: (-item[1], item[0]))[:15]:
    print(f"| `doc/{subdir}` | {count} | {status(count >= SUBDIR_ALERT, 'action_required')} |")
print()

print("## Near-Limit Active Docs")
print("| File | Lines | Status |")
print("| --- | --- | --- |")
near_limit_rows = sorted(non_devlog_near_limit, reverse=True)
if near_limit_rows:
    for line_count, path in near_limit_rows[:15]:
        flag = "split_required" if line_count >= 1000 else "action_required"
        print(f"| `{path}` | {line_count} | {flag} |")
else:
    print("| _none_ | 0 | normal |")
print()

print("## Largest devlog Files")
print("| File | Lines | Status |")
print("| --- | --- | --- |")
for line_count, path in sorted(devlog_rows, reverse=True)[:10]:
    print(f"| `{path}` | {line_count} | {status(line_count >= DEVLOG_LINE_ALERT, 'action_required')} |")
PY
