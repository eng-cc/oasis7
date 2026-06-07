#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

usage() {
  cat <<'USAGE'
Usage: ./scripts/ci-compile-metrics.sh --package <cargo-package> --out-dir <dir> [options]

Measure isolated cargo compile metrics for the current checkout and optionally
compare them with a baseline git ref on the same runner family.

Required:
  --package <name>          Cargo package to measure.
  --out-dir <dir>           Output directory for JSON/Markdown/log artifacts.

Options:
  --binary <name>           Release binary name to size-check. Defaults to package.
  --baseline-ref <ref>      Optional git ref/SHA to compare against.
  -h, --help                Show this help.

Outputs:
  <out-dir>/current.metrics.json
  <out-dir>/baseline.metrics.json          (when --baseline-ref is provided)
  <out-dir>/comparison.json
  <out-dir>/summary.md
  <out-dir>/logs/*.log
USAGE
}

package_name=""
binary_name=""
baseline_ref=""
out_dir=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --package)
      package_name="${2:-}"
      shift 2
      ;;
    --binary)
      binary_name="${2:-}"
      shift 2
      ;;
    --baseline-ref)
      baseline_ref="${2:-}"
      shift 2
      ;;
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

if [[ -z "$package_name" || -z "$out_dir" ]]; then
  echo "error: --package and --out-dir are required" >&2
  usage >&2
  exit 2
fi

if [[ -z "$binary_name" ]]; then
  binary_name="$package_name"
fi

mkdir -p "$out_dir/logs"

tmp_root=$(mktemp -d "${TMPDIR:-/tmp}/oasis7-compile-metrics-XXXXXX")
cleanup_paths=("$tmp_root")
cleanup() {
  local path
  for path in "${cleanup_paths[@]}"; do
    rm -rf "$path"
  done
}
trap cleanup EXIT

measure_command_seconds() {
  local checkout_path="$1"
  local target_dir="$2"
  local log_path="$3"
  shift 3

  local start_ns
  local end_ns
  start_ns=$(python3 - <<'PY'
import time
print(time.monotonic_ns())
PY
)
  (
    cd "$checkout_path"
    export CARGO_TARGET_DIR="$target_dir"
    "$@"
  ) >"$log_path" 2>&1
  end_ns=$(python3 - <<'PY'
import time
print(time.monotonic_ns())
PY
)
  python3 - "$start_ns" "$end_ns" <<'PY'
import sys
start_ns = int(sys.argv[1])
end_ns = int(sys.argv[2])
print(f"{(end_ns - start_ns) / 1_000_000_000:.3f}")
PY
}

resolve_binary_path() {
  local release_dir="$1"
  local bin_name="$2"
  python3 - "$release_dir" "$bin_name" <<'PY'
from pathlib import Path
import os
import sys

release_dir = Path(sys.argv[1])
bin_name = sys.argv[2]

candidates = []
if os.name == "nt":
    candidates.extend(
        [
            release_dir / f"{bin_name}.exe",
            release_dir / bin_name,
        ]
    )
else:
    candidates.extend(
        [
            release_dir / bin_name,
            release_dir / f"{bin_name}.exe",
        ]
    )

for candidate in candidates:
    if candidate.is_file():
        print(candidate)
        raise SystemExit(0)

raise SystemExit(
    f"error: unable to find built binary for {bin_name} under {release_dir}"
)
PY
}

measure_checkout() {
  local label="$1"
  local checkout_path="$2"
  local result_path="$3"

  local check_target="$tmp_root/${label}-check-target"
  local release_target="$tmp_root/${label}-release-target"
  mkdir -p "$check_target" "$release_target"

  local package_count
  package_count=$(
    cd "$checkout_path"
    cargo tree -p "$package_name" --prefix none | sort -u | wc -l | tr -d '[:space:]'
  )

  local wasmtime_present="false"
  if (
    cd "$checkout_path"
    cargo tree -p "$package_name" -i wasmtime >/dev/null 2>&1
  ); then
    wasmtime_present="true"
  fi

  local wasm_executor_present="false"
  if (
    cd "$checkout_path"
    cargo tree -p "$package_name" -i oasis7_wasm_executor >/dev/null 2>&1
  ); then
    wasm_executor_present="true"
  fi

  local check_seconds
  check_seconds=$(
    measure_command_seconds \
      "$checkout_path" \
      "$check_target" \
      "$out_dir/logs/${label}-cargo-check.log" \
      env -u RUSTC_WRAPPER cargo check -p "$package_name"
  )

  local release_seconds
  release_seconds=$(
    measure_command_seconds \
      "$checkout_path" \
      "$release_target" \
      "$out_dir/logs/${label}-cargo-build-release.log" \
      env -u RUSTC_WRAPPER cargo build -p "$package_name" --release --bin "$binary_name"
  )

  local binary_path
  binary_path=$(resolve_binary_path "$release_target/release" "$binary_name")
  local binary_bytes
  binary_bytes=$(python3 - "$binary_path" <<'PY'
from pathlib import Path
import sys
print(Path(sys.argv[1]).stat().st_size)
PY
)

  python3 - "$result_path" <<'PY' \
    "$label" \
    "$checkout_path" \
    "$package_name" \
    "$binary_name" \
    "$package_count" \
    "$wasmtime_present" \
    "$wasm_executor_present" \
    "$check_seconds" \
    "$release_seconds" \
    "$binary_bytes"
import json
import sys

out_path = sys.argv[1]
payload = {
    "label": sys.argv[2],
    "checkout_path": sys.argv[3],
    "package": sys.argv[4],
    "binary": sys.argv[5],
    "package_count": int(sys.argv[6]),
    "wasmtime_present": sys.argv[7] == "true",
    "wasm_executor_present": sys.argv[8] == "true",
    "cargo_check_seconds": float(sys.argv[9]),
    "cargo_build_release_seconds": float(sys.argv[10]),
    "release_binary_bytes": int(sys.argv[11]),
}
with open(out_path, "w", encoding="utf-8") as handle:
    json.dump(payload, handle, indent=2, sort_keys=True)
    handle.write("\n")
PY
}

current_metrics_json="$out_dir/current.metrics.json"
baseline_metrics_json="$out_dir/baseline.metrics.json"
comparison_json="$out_dir/comparison.json"
summary_md="$out_dir/summary.md"

measure_checkout "current" "$repo_root" "$current_metrics_json"

baseline_checkout_path=""
if [[ -n "$baseline_ref" ]]; then
  baseline_checkout_path="$tmp_root/baseline-worktree"
  git worktree add --detach "$baseline_checkout_path" "$baseline_ref" >/dev/null
  cleanup_paths+=("$baseline_checkout_path")
  measure_checkout "baseline" "$baseline_checkout_path" "$baseline_metrics_json"
fi

python3 - "$current_metrics_json" "$baseline_metrics_json" "$comparison_json" "$summary_md" "$baseline_ref" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path


def load_json(path: str) -> dict | None:
    file_path = Path(path)
    if not file_path.exists():
        return None
    with file_path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def delta(current: float, baseline: float) -> float:
    return current - baseline


def pct(current: float, baseline: float) -> float | None:
    if baseline == 0:
        return None
    return ((current - baseline) / baseline) * 100.0


def fmt_num(value: float) -> str:
    return f"{value:.3f}"


def fmt_int(value: int) -> str:
    return f"{value:,}"


def fmt_pct(value: float | None) -> str:
    if value is None:
        return "n/a"
    return f"{value:+.2f}%"


current = load_json(sys.argv[1])
baseline = load_json(sys.argv[2])
comparison_path = Path(sys.argv[3])
summary_path = Path(sys.argv[4])
baseline_ref = sys.argv[5]

if current is None:
    raise SystemExit("current metrics JSON is missing")

comparison = {
    "package": current["package"],
    "binary": current["binary"],
    "current": current,
    "baseline_ref": baseline_ref or None,
    "baseline": baseline,
}

metric_rows: list[dict] = []
if baseline is not None:
    for key in (
        "package_count",
        "cargo_check_seconds",
        "cargo_build_release_seconds",
        "release_binary_bytes",
    ):
        current_value = current[key]
        baseline_value = baseline[key]
        metric_rows.append(
            {
                "metric": key,
                "baseline": baseline_value,
                "current": current_value,
                "delta": delta(current_value, baseline_value),
                "percent": pct(current_value, baseline_value),
            }
        )
comparison["metric_rows"] = metric_rows

with comparison_path.open("w", encoding="utf-8") as handle:
    json.dump(comparison, handle, indent=2, sort_keys=True)
    handle.write("\n")

lines: list[str] = []
lines.append(f"# Compile Metrics: {current['package']}")
lines.append("")
lines.append(f"- Binary: `{current['binary']}`")
lines.append(f"- Current package closure count: `{current['package_count']}`")
lines.append(f"- Current `wasmtime` present: `{str(current['wasmtime_present']).lower()}`")
lines.append(f"- Current `oasis7_wasm_executor` present: `{str(current['wasm_executor_present']).lower()}`")
lines.append(f"- Current cold `cargo check` seconds: `{fmt_num(current['cargo_check_seconds'])}`")
lines.append(f"- Current cold `cargo build --release` seconds: `{fmt_num(current['cargo_build_release_seconds'])}`")
lines.append(f"- Current release binary bytes: `{fmt_int(current['release_binary_bytes'])}`")

if baseline is None:
    lines.append("")
    lines.append("No baseline ref was provided, so this report contains current-run metrics only.")
else:
    lines.append("")
    lines.append(f"Compared against baseline ref `{baseline_ref}` measured on the same runner family with isolated cargo target directories.")
    lines.append("")
    lines.append("| Metric | Baseline | Current | Delta | Delta % |")
    lines.append("| --- | ---: | ---: | ---: | ---: |")
    for row in metric_rows:
        metric = row["metric"]
        baseline_value = row["baseline"]
        current_value = row["current"]
        delta_value = row["delta"]
        percent_value = row["percent"]
        if metric.endswith("_seconds"):
            baseline_fmt = fmt_num(baseline_value)
            current_fmt = fmt_num(current_value)
            delta_fmt = f"{delta_value:+.3f}"
        else:
            baseline_fmt = fmt_int(int(baseline_value))
            current_fmt = fmt_int(int(current_value))
            delta_fmt = f"{int(delta_value):+,}"
        lines.append(
            f"| `{metric}` | `{baseline_fmt}` | `{current_fmt}` | `{delta_fmt}` | `{fmt_pct(percent_value)}` |"
        )
    lines.append("")
    lines.append(
        "Timing numbers are cold-build wall-clock measurements from this workflow run and are most meaningful when compared against the baseline in the same report, not across unrelated runs."
    )

with summary_path.open("w", encoding="utf-8") as handle:
    handle.write("\n".join(lines))
    handle.write("\n")
PY

echo "wrote compile metrics to $out_dir"
