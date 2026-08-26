#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

usage() {
  cat <<'USAGE'
Usage: ./scripts/ci-compile-metrics.sh --package <cargo-package> --out-dir <dir> [options]

Measure isolated cargo compile metrics for the current checkout and optionally
compare them with a baseline git ref on the same runner family.

All Cargo invocations are lockfile-pinned so dependency resolution cannot
silently change the compile surface being measured.

The timed compile commands run offline after the host-target pre-fetch, so a
registry or network miss cannot contaminate the wall-clock measurements.

Every Cargo invocation fixes CARGO_INCREMENTAL=0, CARGO_PROFILE_DEV_DEBUG=0,
and CARGO_PROFILE_TEST_DEBUG=0, and unsets RUSTC_WRAPPER, regardless of caller
environment.

Each checkout performs one dependency-tree query; closure counting and
dependency-presence checks are derived from that shared result.

Cargo registry/source downloads use the caller's CARGO_HOME (or the default
$HOME/.cargo) and are shared by current and baseline measurements. The
pre-fetch is limited to the runner's host target because the measured commands
compile only that surface. Each checkout still receives an isolated target
directory so compile timings remain cold and comparable.

Required:
  --package <name>          Cargo package to measure.
  --out-dir <dir>           Output directory for JSON/Markdown/log artifacts.

Options:
  --binary <name>           Release binary name to size-check. Defaults to package.
  --check-only              Measure package closure and cargo check only (for libraries).
  --no-default-features     Measure the package with Cargo default features disabled.
  --baseline-ref <ref>      Optional git ref/SHA to compare against.
  -h, --help                Show this help.

Outputs:
  <out-dir>/current.metrics.json
  <out-dir>/baseline.metrics.json          (when --baseline-ref is provided)
  <out-dir>/comparison.json
  <out-dir>/summary.md
  <out-dir>/logs/*.log

Each metrics JSON and the summary records the resolved commit OID for the
checkout it measured, so a movable baseline ref cannot make the evidence
ambiguous after the run.  comparison.json also records the package, binary,
and feature-mode identity used for both measurements and rejects mismatches.
USAGE
}

package_name=""
binary_name=""
baseline_ref=""
out_dir=""
check_only=false
no_default_features=false

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
    --check-only)
      check_only=true
      shift
      ;;
    --no-default-features)
      no_default_features=true
      shift
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

if [[ -z "$binary_name" && "$check_only" != true ]]; then
  binary_name="$package_name"
fi

out_dir=$(python3 - "$out_dir" <<'PY'
from pathlib import Path
import sys
print(Path(sys.argv[1]).resolve())
PY
)

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

cargo_home="${CARGO_HOME:-$HOME/.cargo}"
cargo_home=$(python3 - "$cargo_home" <<'PY'
from pathlib import Path
import sys

print(Path(sys.argv[1]).expanduser().resolve())
PY
)

host_target=$(rustc -vV | sed -n 's/^host: //p')
if [[ -z "$host_target" ]]; then
  echo "error: unable to resolve the Rust host target" >&2
  exit 1
fi

# Keep every Cargo query and timed compile on the same measurement contract,
# regardless of caller-provided development settings.  The workflow sets these
# values too, but the harness is also a documented standalone entrypoint.
compile_metrics_cargo_env=(
  env
  -u RUSTC_WRAPPER
  CARGO_INCREMENTAL=0
  CARGO_PROFILE_DEV_DEBUG=0
  CARGO_PROFILE_TEST_DEBUG=0
)

measure_command_seconds() {
  local checkout_path="$1"
  local target_dir="$2"
  local cargo_home="$3"
  local log_path="$4"
  shift 4

  # Keep both timing endpoints in one process. This preserves monotonic elapsed
  # time without comparing clock values produced by separate Python processes.
  python3 - "$checkout_path" "$target_dir" "$cargo_home" "$log_path" "$@" <<'PY'
import os
from pathlib import Path
import subprocess
import sys
import time

checkout_path, target_dir, cargo_home, log_path, *command = sys.argv[1:]
env = os.environ.copy()
env["CARGO_TARGET_DIR"] = target_dir
env["CARGO_HOME"] = cargo_home
start_ns = time.monotonic_ns()
with Path(log_path).open("wb") as log:
    completed = subprocess.run(
        command,
        cwd=checkout_path,
        env=env,
        stdout=log,
        stderr=subprocess.STDOUT,
        check=False,
    )
end_ns = time.monotonic_ns()
if completed.returncode != 0:
    raise SystemExit(completed.returncode)
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

  local commit_oid
  commit_oid=$(git -C "$checkout_path" rev-parse --verify 'HEAD^{commit}')

  local check_target="$tmp_root/${label}-check-target"
  local release_target="$tmp_root/${label}-release-target"
  mkdir -p "$check_target" "$release_target"

  local check_seconds
  local cargo_check_args=("${compile_metrics_cargo_env[@]}" cargo check --offline --locked -p "$package_name")
  if [[ "$no_default_features" == true ]]; then
    cargo_check_args+=(--no-default-features)
  fi
  (
    cd "$checkout_path"
    export CARGO_HOME="$cargo_home"
    "${compile_metrics_cargo_env[@]}" cargo fetch --locked --target "$host_target" \
      >/dev/null 2>"$out_dir/logs/${label}-cargo-fetch.log"
  )

  local dependency_tree_path="$tmp_root/${label}-dependency-tree.txt"
  local tree_args=("${compile_metrics_cargo_env[@]}" cargo tree --offline --locked -p "$package_name")
  if [[ "$no_default_features" == true ]]; then
    tree_args+=(--no-default-features)
  fi
  # Fetch the host-target dependency metadata before the tree query and keep
  # the query offline.  This makes closure evidence deterministic instead of
  # allowing an unbounded registry/index lookup before the timed commands.
  # One forward dependency-tree query is enough for both closure counting and
  # the presence checks below.  The prior inverse queries repeated Cargo's
  # dependency resolution for the same checkout, adding compile-metrics
  # overhead without changing the measured package closure.
  (
    cd "$checkout_path"
    export CARGO_HOME="$cargo_home"
    "${tree_args[@]}" --prefix none
  ) >"$dependency_tree_path"

  local package_count
  # Cargo marks a de-duplicated dependency edge with a trailing `(*)`.  That
  # marker is presentation metadata, not part of the package identity; strip
  # it before counting so shared dependencies do not inflate the closure
  # metric and create false compile-surface regressions.
  package_count=$(sed -E 's/[[:space:]]+\(\*\)$//' "$dependency_tree_path" | sort -u | wc -l | tr -d '[:space:]')

  local wasmtime_present="false"
  if grep -Eq '(^|[[:space:]])wasmtime([[:space:]]|$)' "$dependency_tree_path"; then
    wasmtime_present="true"
  fi

  local wasm_executor_present="false"
  if grep -Eq '(^|[[:space:]])oasis7_wasm_executor([[:space:]]|$)' "$dependency_tree_path"; then
    wasm_executor_present="true"
  fi

  check_seconds=$(
    measure_command_seconds \
      "$checkout_path" \
      "$check_target" \
      "$cargo_home" \
      "$out_dir/logs/${label}-cargo-check.log" \
      "${cargo_check_args[@]}"
  )

  local release_seconds=""
  local binary_bytes=""
  if [[ "$check_only" != true ]]; then
    release_seconds=$(
      measure_command_seconds \
        "$checkout_path" \
        "$release_target" \
        "$cargo_home" \
        "$out_dir/logs/${label}-cargo-build-release.log" \
        "${compile_metrics_cargo_env[@]}" cargo build --offline --locked -p "$package_name" --release --bin "$binary_name"
    )

    local binary_path
    binary_path=$(resolve_binary_path "$release_target/release" "$binary_name")
    binary_bytes=$(python3 - "$binary_path" <<'PY'
from pathlib import Path
import sys
print(Path(sys.argv[1]).stat().st_size)
PY
    )
  fi

  python3 - "$result_path" <<'PY' \
    "$label" \
    "$checkout_path" \
    "$commit_oid" \
    "$package_name" \
    "$binary_name" \
    "$package_count" \
    "$wasmtime_present" \
    "$wasm_executor_present" \
    "$check_seconds" \
    "$release_seconds" \
    "$binary_bytes" \
    "$check_only" \
    "$no_default_features"
import json
import sys

out_path = sys.argv[1]
payload = {
    "label": sys.argv[2],
    "checkout_path": sys.argv[3],
    "commit_oid": sys.argv[4],
    "package": sys.argv[5],
    "binary": sys.argv[6] or None,
    "package_count": int(sys.argv[7]),
    "wasmtime_present": sys.argv[8] == "true",
    "wasm_executor_present": sys.argv[9] == "true",
    "cargo_check_seconds": float(sys.argv[10]),
    "cargo_build_release_seconds": float(sys.argv[11]) if sys.argv[11] else None,
    "release_binary_bytes": int(sys.argv[12]) if sys.argv[12] else None,
    "check_only": sys.argv[13] == "true",
    "no_default_features": sys.argv[14] == "true",
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


# These fields define whether current and baseline compile measurements are
# comparable, independent of their commit provenance.
IDENTITY_FIELDS = (
    "package",
    "binary",
    "check_only",
    "no_default_features",
)


def measurement_identity(metrics: dict, label: str) -> dict:
    if not isinstance(metrics, dict):
        raise SystemExit(f"{label} metrics must be a JSON object")
    missing = [field for field in IDENTITY_FIELDS if field not in metrics]
    if missing:
        raise SystemExit(
            f"{label} metrics missing measurement identity fields: {', '.join(missing)}"
        )

    package = metrics["package"]
    if type(package) is not str or not package:
        raise SystemExit(
            f"{label} measurement identity package must be a non-empty string"
        )

    binary = metrics["binary"]
    if binary is not None and (type(binary) is not str or not binary):
        raise SystemExit(
            f"{label} measurement identity binary must be null or a non-empty string"
        )

    for field in ("check_only", "no_default_features"):
        if type(metrics[field]) is not bool:
            raise SystemExit(f"{label} measurement identity {field} must be a boolean")

    if metrics["check_only"] and binary is not None:
        raise SystemExit(
            f"{label} measurement identity check_only requires binary to be null"
        )
    if not metrics["check_only"] and binary is None:
        raise SystemExit(
            f"{label} measurement identity release-build requires binary to be a non-empty string"
        )

    return {field: metrics[field] for field in IDENTITY_FIELDS}


current = load_json(sys.argv[1])
baseline = load_json(sys.argv[2])
comparison_path = Path(sys.argv[3])
summary_path = Path(sys.argv[4])
baseline_ref = sys.argv[5]

if current is None:
    raise SystemExit("current metrics JSON is missing")

current_identity = measurement_identity(current, "current")
baseline_identity = (
    measurement_identity(baseline, "baseline") if baseline is not None else None
)
if baseline_identity is not None and current_identity != baseline_identity:
    mismatches = [
        f"{field}: current={current_identity[field]!r}, baseline={baseline_identity[field]!r}"
        for field in IDENTITY_FIELDS
        if current_identity[field] != baseline_identity[field]
    ]
    raise SystemExit(
        "current/baseline measurement identity mismatch: " + "; ".join(mismatches)
    )

comparison = {
    "package": current["package"],
    "binary": current["binary"],
    "measurement_identity": current_identity,
    "current_commit_oid": current["commit_oid"],
    "current": current,
    "baseline_ref": baseline_ref or None,
    "baseline_commit_oid": baseline["commit_oid"] if baseline is not None else None,
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
        if current_value is None or baseline_value is None:
            continue
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
lines.append(f"- Current commit OID: `{current['commit_oid']}`")
if current["binary"] is None:
    lines.append("- Binary: `not measured (check-only package)`")
else:
    lines.append(f"- Binary: `{current['binary']}`")
lines.append(f"- Current package closure count: `{current['package_count']}`")
lines.append(f"- Current `wasmtime` present: `{str(current['wasmtime_present']).lower()}`")
lines.append(f"- Current `oasis7_wasm_executor` present: `{str(current['wasm_executor_present']).lower()}`")
lines.append(f"- Current cold `cargo check` seconds: `{fmt_num(current['cargo_check_seconds'])}`")
if current["cargo_build_release_seconds"] is not None:
    lines.append(f"- Current cold `cargo build --release` seconds: `{fmt_num(current['cargo_build_release_seconds'])}`")
if current["release_binary_bytes"] is not None:
    lines.append(f"- Current release binary bytes: `{fmt_int(current['release_binary_bytes'])}`")

if baseline is None:
    lines.append("")
    lines.append("No baseline ref was provided, so this report contains current-run metrics only.")
else:
    lines.append("")
    lines.append(f"Compared against baseline ref `{baseline_ref}` at commit `{baseline['commit_oid']}` measured on the same runner family with isolated cargo target directories.")
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
