#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

usage() {
  cat <<'USAGE'
Usage: ./scripts/p2p-real-env-observability-monitor.sh [options]

Run the complete triad observability stack:
  1. same-window chain-status/health snapshot
  2. host/process resource monitor
  3. traffic window monitor
  4. per-node wasm window summaries
  5. merged triad observability summary

Options:
  --samples <n>                    snapshot/host sample count (default: 4)
  --interval-secs <n>              snapshot/host interval seconds (default: 5)
  --traffic-samples <n>            traffic sample count (default: 3)
  --traffic-interval-secs <n>      traffic interval seconds (default: 20)
  --window-minutes <n>             traffic window minutes (default: 10)
  --ssh-timeout-secs <n>           SSH connect timeout (default: 8)
  --out-dir <path>                 output root (default: .tmp/p2p_real_env_observability)

  The remaining node/service/status arguments are passed through to the
  underlying snapshot / host / traffic scripts when applicable.
USAGE
}

samples=4
interval_secs=5
traffic_samples=3
traffic_interval_secs=20
window_minutes=10
ssh_timeout_secs=8
out_root=".tmp/p2p_real_env_observability"

pass_through_args=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --samples)
      samples=${2:-}
      shift 2
      ;;
    --interval-secs)
      interval_secs=${2:-}
      shift 2
      ;;
    --traffic-samples)
      traffic_samples=${2:-}
      shift 2
      ;;
    --traffic-interval-secs)
      traffic_interval_secs=${2:-}
      shift 2
      ;;
    --window-minutes)
      window_minutes=${2:-}
      shift 2
      ;;
    --ssh-timeout-secs)
      ssh_timeout_secs=${2:-}
      shift 2
      ;;
    --out-dir)
      out_root=${2:-}
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      pass_through_args+=("$1")
      if [[ $# -ge 2 && "$2" != --* ]]; then
        pass_through_args+=("$2")
        shift 2
      else
        shift
      fi
      ;;
  esac
done

run_id=$(date +"%Y%m%d-%H%M%S")
run_dir="$out_root/$run_id"
snapshot_root="$run_dir/snapshot"
host_root="$run_dir/host"
traffic_root="$run_dir/traffic"
wasm_root="$run_dir/wasm"
report_root="$run_dir/report"
latest_summary_json="$out_root/latest_summary.json"
latest_summary_md="$out_root/latest_summary.md"

mkdir -p "$snapshot_root" "$host_root" "$traffic_root" "$wasm_root" "$report_root"

./scripts/p2p-real-env-triad-snapshot.sh \
  --samples "$samples" \
  --interval-secs "$interval_secs" \
  --ssh-timeout-secs "$ssh_timeout_secs" \
  --out-dir "$snapshot_root" \
  "${pass_through_args[@]}"

snapshot_run_dir="$(find "$snapshot_root" -mindepth 1 -maxdepth 1 -type d | sort | tail -n 1)"
[[ -n "$snapshot_run_dir" ]] || {
  echo "failed to locate snapshot run dir under $snapshot_root" >&2
  exit 1
}
snapshot_summary="$snapshot_run_dir/summary.json"

./scripts/p2p-real-env-host-monitor.sh \
  --samples "$samples" \
  --interval-secs "$interval_secs" \
  --ssh-timeout-secs "$ssh_timeout_secs" \
  --out-dir "$host_root" \
  "${pass_through_args[@]}"

host_run_dir="$(find "$host_root" -mindepth 1 -maxdepth 1 -type d | sort | tail -n 1)"
[[ -n "$host_run_dir" ]] || {
  echo "failed to locate host run dir under $host_root" >&2
  exit 1
}
host_summary="$host_run_dir/summary.json"

./scripts/p2p-real-env-traffic-monitor.sh \
  --samples "$traffic_samples" \
  --interval-secs "$traffic_interval_secs" \
  --window-minutes "$window_minutes" \
  --ssh-timeout-secs "$ssh_timeout_secs" \
  --out-dir "$traffic_root" \
  "${pass_through_args[@]}"

traffic_summary="$traffic_root/latest_summary.json"

for label in observer_local sequencer_ecs storage_ecs; do
  sample_source_dir="$snapshot_run_dir/nodes/$label/samples"
  wasm_input_dir="$wasm_root/input/$label"
  wasm_out_dir="$wasm_root/$label"
  mkdir -p "$wasm_input_dir" "$wasm_out_dir"
  index=1
  for sample_status in "$sample_source_dir"/sample-*/status.json; do
    [[ -f "$sample_status" ]] || continue
    cp "$sample_status" "$wasm_input_dir/$(printf '%03d' "$index").json"
    index=$(( index + 1 ))
  done
  bash ./scripts/oasis7-node-wasm-metrics-monitor.sh \
    --status-sample-dir "$wasm_input_dir" \
    --node-label "$label" \
    --out-dir "$wasm_out_dir"
done

python3 ./scripts/p2p-real-env-observability-summary.py \
  --snapshot-summary "$snapshot_summary" \
  --host-summary "$host_summary" \
  --traffic-summary "$traffic_summary" \
  --observer-wasm-summary "$wasm_root/observer_local/latest_summary.json" \
  --sequencer-wasm-summary "$wasm_root/sequencer_ecs/latest_summary.json" \
  --storage-wasm-summary "$wasm_root/storage_ecs/latest_summary.json" \
  --summary-json "$report_root/latest_summary.json" \
  --summary-md "$report_root/latest_summary.md" \
  --run-id "$run_id" \
  --run-dir "$run_dir"

cp "$report_root/latest_summary.json" "$latest_summary_json"
cp "$report_root/latest_summary.md" "$latest_summary_md"

echo "p2p real-env observability monitor complete"
echo "  run_dir: $run_dir"
echo "  snapshot_summary: $snapshot_summary"
echo "  host_summary: $host_summary"
echo "  traffic_summary: $traffic_summary"
echo "  report_summary: $report_root/latest_summary.json"
