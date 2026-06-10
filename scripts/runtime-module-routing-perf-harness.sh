#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

usage() {
  cat <<'USAGE'
Usage: ./scripts/runtime-module-routing-perf-harness.sh [options]

Purpose:
  Run the lightweight deterministic runtime module-routing perf probe and
  write a small machine-readable summary for local regression tracking.

Outputs:
  <out-dir>/<timestamp>/
    perf.log
    summary.json
    summary.md

Options:
  --out-dir <path>   Output root (default: .tmp/runtime_module_routing_perf)
  --profile <name>   Cargo profile: release | dev (default: release)
  -h, --help         Show help
USAGE
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "error: required command not found: $1" >&2
    exit 2
  }
}

out_dir=".tmp/runtime_module_routing_perf"
profile="release"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out-dir)
      out_dir=${2:-}
      shift 2
      ;;
    --profile)
      profile=${2:-}
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

need_cmd python3

case "$profile" in
  release|dev) ;;
  *)
    echo "error: --profile must be one of: release, dev" >&2
    exit 2
    ;;
esac

timestamp=$(date '+%Y%m%d-%H%M%S')
run_dir="$out_dir/$timestamp"
mkdir -p "$run_dir"

log_path="$run_dir/perf.log"
summary_json="$run_dir/summary.json"
summary_md="$run_dir/summary.md"

declare -a cargo_args=(
  run
  -p oasis7
  --bin oasis7_runtime_module_routing_perf
)
if [[ "$profile" == "release" ]]; then
  cargo_args+=(--release)
fi

env -u RUSTC_WRAPPER cargo "${cargo_args[@]}" 2>&1 | tee "$log_path"

python3 - "$log_path" "$summary_json" "$summary_md" "$profile" <<'PY'
import json
import pathlib
import re
import sys

log_path = pathlib.Path(sys.argv[1])
summary_json = pathlib.Path(sys.argv[2])
summary_md = pathlib.Path(sys.argv[3])
profile = sys.argv[4]

text = log_path.read_text(encoding="utf-8")
pattern = re.compile(
    r"perf_probe_runtime_module_routing_with_many_active_manifests: "
    r"modules=(?P<modules>\d+) "
    r"iterations=(?P<iterations>\d+) "
    r"event_total_ms=(?P<event_total_ms>[0-9.]+) "
    r"event_avg_ms=(?P<event_avg_ms>[0-9.]+) "
    r"action_total_ms=(?P<action_total_ms>[0-9.]+) "
    r"action_avg_ms=(?P<action_avg_ms>[0-9.]+) "
    r"event_invoked=(?P<event_invoked>\d+) "
    r"action_invoked=(?P<action_invoked>\d+)"
)
match = pattern.search(text)
if not match:
    raise SystemExit("error: perf probe output not found in log")

data = {
    "probe": "perf_probe_runtime_module_routing_with_many_active_manifests",
    "surface": "runtime.module_routing",
    "profile": profile,
    "modules": int(match.group("modules")),
    "iterations": int(match.group("iterations")),
    "event_total_ms": float(match.group("event_total_ms")),
    "event_avg_ms": float(match.group("event_avg_ms")),
    "action_total_ms": float(match.group("action_total_ms")),
    "action_avg_ms": float(match.group("action_avg_ms")),
    "event_invoked": int(match.group("event_invoked")),
    "action_invoked": int(match.group("action_invoked")),
    "log_path": str(log_path.resolve()),
}

summary_json.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
summary_md.write_text(
    "\n".join(
        [
            "# Runtime Module Routing Perf Harness",
            "",
            f"- probe: `{data['probe']}`",
            f"- surface: `{data['surface']}`",
            f"- profile: `{data['profile']}`",
            f"- modules: `{data['modules']}`",
            f"- iterations: `{data['iterations']}`",
            f"- event_total_ms: `{data['event_total_ms']:.3f}`",
            f"- event_avg_ms: `{data['event_avg_ms']:.3f}`",
            f"- action_total_ms: `{data['action_total_ms']:.3f}`",
            f"- action_avg_ms: `{data['action_avg_ms']:.3f}`",
            f"- event_invoked: `{data['event_invoked']}`",
            f"- action_invoked: `{data['action_invoked']}`",
            f"- log_path: `{data['log_path']}`",
        ]
    )
    + "\n",
    encoding="utf-8",
)
PY

echo "runtime module routing perf summary: $summary_md"
echo "runtime module routing perf summary json: $summary_json"
