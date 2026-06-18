#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

out_dir="output/rust-governance"

usage() {
  cat <<'USAGE'
Usage: ./scripts/ci-rust-governance-report.sh [--out-dir <dir>]

Produce report-only Rust governance artifacts:
- cargo-deny full policy report
- duplicate dependency report
- unsafe usage distribution

The script returns success even when report-only checks find issues. The
required advisory gate lives in ./scripts/ci-tests.sh.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out-dir)
      out_dir="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

[[ -n "$out_dir" ]] || { echo "error: --out-dir cannot be empty" >&2; exit 2; }
mkdir -p "$out_dir"

run_report() {
  local label="$1"
  local log_path="$2"
  shift 2

  local status=0
  "$@" >"$log_path" 2>&1 || status=$?
  printf '%s\n' "$status" >"${log_path}.rc"
}

if python3 - "$out_dir/cargo-deny-install.log" <<'PY'
from __future__ import annotations

from pathlib import Path
import subprocess
import sys

log_path = Path(sys.argv[1])
with log_path.open("w", encoding="utf-8") as log:
    try:
        result = subprocess.run(
            ["./scripts/ensure-cargo-deny.sh"],
            stdout=log,
            stderr=subprocess.STDOUT,
            timeout=120,
            check=False,
        )
    except subprocess.TimeoutExpired:
        log.write("error: cargo-deny install timed out after 120 seconds\n")
        raise SystemExit(124)
raise SystemExit(result.returncode)
PY
then
  run_report "cargo deny" "$out_dir/cargo-deny.log" cargo deny check
else
  {
    echo "cargo-deny unavailable; see cargo-deny-install.log"
    echo "report-only cargo deny check skipped"
  } >"$out_dir/cargo-deny.log"
  printf '%s\n' 127 >"$out_dir/cargo-deny.log.rc"
fi
run_report "duplicate dependencies" "$out_dir/cargo-tree-duplicates.log" cargo tree -d
run_report "unsafe usage" "$out_dir/unsafe-usage.log" rg -n --glob '*.rs' '\bunsafe\b' .

python3 - "$out_dir" <<'PY'
from __future__ import annotations

from collections import Counter
from pathlib import Path
import json
import sys

out_dir = Path(sys.argv[1])

def read_rc(name: str) -> int:
    path = out_dir / f"{name}.rc"
    try:
        return int(path.read_text(encoding="utf-8").strip())
    except FileNotFoundError:
        return 127

unsafe_counts: Counter[str] = Counter()
unsafe_log = out_dir / "unsafe-usage.log"
if unsafe_log.exists():
    for line in unsafe_log.read_text(encoding="utf-8", errors="replace").splitlines():
        path = line.split(":", 1)[0]
        if path:
            parts = Path(path).parts
            bucket = "/".join(parts[:3]) if len(parts) >= 3 else path
            unsafe_counts[bucket] += 1

summary = {
    "cargo_deny_rc": read_rc("cargo-deny.log"),
    "duplicate_dependencies_rc": read_rc("cargo-tree-duplicates.log"),
    "unsafe_usage_rc": read_rc("unsafe-usage.log"),
    "unsafe_usage_total": sum(unsafe_counts.values()),
    "unsafe_usage_top_buckets": unsafe_counts.most_common(20),
}

(out_dir / "summary.json").write_text(
    json.dumps(summary, ensure_ascii=False, indent=2) + "\n",
    encoding="utf-8",
)

lines = [
    "# Rust Governance Report",
    "",
    "Report-only checks; non-zero statuses below are findings, not CI failures for this job.",
    "",
    "| Check | Status | Artifact |",
    "| --- | ---: | --- |",
    f"| cargo deny check | {summary['cargo_deny_rc']} | `cargo-deny.log` |",
    f"| cargo tree -d | {summary['duplicate_dependencies_rc']} | `cargo-tree-duplicates.log` |",
    f"| unsafe usage scan | {summary['unsafe_usage_rc']} | `unsafe-usage.log` |",
    "",
    f"- Unsafe usage matches: `{summary['unsafe_usage_total']}`",
]

if summary["unsafe_usage_top_buckets"]:
    lines.append("")
    lines.append("## Top Unsafe Usage Buckets")
    lines.append("")
    for bucket, count in summary["unsafe_usage_top_buckets"]:
        lines.append(f"- `{bucket}`: {count}")

(out_dir / "summary.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
PY

cat "$out_dir/summary.md"
