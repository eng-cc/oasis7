#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

usage() {
  cat <<'USAGE'
Usage: ./scripts/p2p-real-env-host-monitor.sh [options]

Collect triad host/process monitoring samples from:
  - local triad node
  - remote ECS sequencer node
  - remote ECS storage node

Options:
  --samples <n>                    number of samples per node (default: 4)
  --interval-secs <n>              sleep interval between samples (default: 5)
  --ssh-timeout-secs <n>           SSH connect timeout in seconds (default: 8)
  --out-dir <path>                 output root (default: .tmp/p2p_real_env_host_monitor)

  --local-service <name>           local node systemd unit
                                   (default: oasis7-triad-observer.service)
  --local-storage-path <path>      local node storage path
                                   (default: /opt/oasis7/p2p-triad-local/data/storage)
  --observer-*                     deprecated aliases for the local-* options above

  --sequencer-target <user@host>   remote sequencer SSH target
                                   (default: root@39.104.204.172)
  --sequencer-service <name>       remote sequencer systemd unit
                                   (default: oasis7-triad-sequencer.service)
  --sequencer-storage-path <path>  remote sequencer storage path
                                   (default: /opt/oasis7/p2p-triad/data/storage)

  --storage-target <user@host>     remote storage SSH target
                                   (default: root@39.104.205.67)
  --storage-service <name>         remote storage systemd unit
                                   (default: oasis7-triad-storage.service)
  --storage-storage-path <path>    remote storage storage path
                                   (default: /opt/oasis7/p2p-triad/data/storage)

Environment:
  P2PARCH6_SEQ_SSH_PASSWORD        optional sequencer SSH password for sshpass
  P2PARCH6_STORAGE_SSH_PASSWORD    optional storage SSH password for sshpass

Artifacts:
  <out-dir>/<timestamp>/samples.ndjson
  <out-dir>/<timestamp>/summary.json
  <out-dir>/<timestamp>/summary.md
  <out-dir>/latest_summary.json
  <out-dir>/latest_summary.md
USAGE
}

ensure_positive_int() {
  local flag=$1
  local value=$2
  if [[ ! "$value" =~ ^[0-9]+$ ]] || (( value <= 0 )); then
    echo "invalid $flag: $value" >&2
    exit 2
  fi
}

run_ssh() {
  local target=$1
  local password=${2:-}
  shift 2
  local cmd=(
    ssh
    -o StrictHostKeyChecking=no
    -o ConnectTimeout="$ssh_timeout_secs"
    -o ServerAliveInterval=5
    "$target"
    "$@"
  )
  if [[ -n "$password" ]]; then
    SSHPASS="$password" sshpass -e "${cmd[@]}"
  else
    "${cmd[@]}"
  fi
}

samples=4
interval_secs=5
ssh_timeout_secs=8
out_root=".tmp/p2p_real_env_host_monitor"

local_service="oasis7-triad-observer.service"
local_storage_path="/opt/oasis7/p2p-triad-local/data/storage"

sequencer_target="root@39.104.204.172"
sequencer_service="oasis7-triad-sequencer.service"
sequencer_storage_path="/opt/oasis7/p2p-triad/data/storage"

storage_target="root@39.104.205.67"
storage_service="oasis7-triad-storage.service"
storage_storage_path="/opt/oasis7/p2p-triad/data/storage"

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
    --ssh-timeout-secs)
      ssh_timeout_secs=${2:-}
      shift 2
      ;;
    --out-dir)
      out_root=${2:-}
      shift 2
      ;;
    --local-service|--observer-service)
      local_service=${2:-}
      shift 2
      ;;
    --local-storage-path|--observer-storage-path)
      local_storage_path=${2:-}
      shift 2
      ;;
    --sequencer-target)
      sequencer_target=${2:-}
      shift 2
      ;;
    --sequencer-service)
      sequencer_service=${2:-}
      shift 2
      ;;
    --sequencer-storage-path)
      sequencer_storage_path=${2:-}
      shift 2
      ;;
    --storage-target)
      storage_target=${2:-}
      shift 2
      ;;
    --storage-service)
      storage_service=${2:-}
      shift 2
      ;;
    --storage-storage-path)
      storage_storage_path=${2:-}
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

ensure_positive_int --samples "$samples"
ensure_positive_int --interval-secs "$interval_secs"
ensure_positive_int --ssh-timeout-secs "$ssh_timeout_secs"

run_id=$(date +"%Y%m%d-%H%M%S")
run_dir="$out_root/$run_id"
nodes_root="$run_dir/nodes"
samples_ndjson="$run_dir/samples.ndjson"
summary_json="$run_dir/summary.json"
summary_md="$run_dir/summary.md"
latest_summary_json="$out_root/latest_summary.json"
latest_summary_md="$out_root/latest_summary.md"

mkdir -p \
  "$nodes_root/local_node" \
  "$nodes_root/sequencer_ecs" \
  "$nodes_root/storage_ecs"
: > "$samples_ndjson"

seq_password=${P2PARCH6_SEQ_SSH_PASSWORD:-}
storage_password=${P2PARCH6_STORAGE_SSH_PASSWORD:-}
helper_script="$repo_root/scripts/p2p-real-env-node-host-sample.sh"

capture_sample() {
  local label=$1
  local mode=$2
  local service=$3
  local storage_path=$4
  local target=${5:-}
  local password=${6:-}
  local sample_dir="$nodes_root/$label/sample-$(printf '%03d' "$sample_index")"
  mkdir -p "$sample_dir"
  local sample_env="$sample_dir/sample.env"

  if [[ "$mode" == "local" ]]; then
    bash "$helper_script" "$service" "$storage_path" >"$sample_env"
  else
    run_ssh "$target" "$password" "bash -s -- '$service' '$storage_path'" <"$helper_script" >"$sample_env"
  fi

  # shellcheck disable=SC1090
  source "$sample_env"

  jq -c -n \
    --arg captured_at "$captured_at" \
    --arg captured_at_unix_ms "$captured_at_unix_ms" \
    --arg label "$label" \
    --arg service "$service" \
    --arg hostname "$hostname" \
    --arg cpu_cores "$cpu_cores" \
    --arg load1 "$load1" \
    --arg load5 "$load5" \
    --arg load15 "$load15" \
    --arg mem_total_bytes "$mem_total_bytes" \
    --arg mem_available_bytes "$mem_available_bytes" \
    --arg storage_path "$storage_path" \
    --arg storage_total_bytes "${storage_total_bytes:-}" \
    --arg storage_used_bytes "${storage_used_bytes:-}" \
    --arg storage_available_bytes "${storage_available_bytes:-}" \
    --arg storage_used_percent "${storage_used_percent:-}" \
    --arg service_active_state "${service_active_state:-unknown}" \
    --arg service_sub_state "${service_sub_state:-unknown}" \
    --arg service_main_pid "${service_main_pid:-0}" \
    --arg wrapper_pcpu "${wrapper_pcpu:-}" \
    --arg wrapper_pmem "${wrapper_pmem:-}" \
    --arg wrapper_elapsed_secs "${wrapper_elapsed_secs:-}" \
    --arg wrapper_nlwp "${wrapper_nlwp:-}" \
    --arg wrapper_psr "${wrapper_psr:-}" \
    --arg runtime_pid "${runtime_pid:-}" \
    --arg runtime_pcpu "${runtime_pcpu:-}" \
    --arg runtime_pmem "${runtime_pmem:-}" \
    --arg runtime_elapsed_secs "${runtime_elapsed_secs:-}" \
    --arg runtime_nlwp "${runtime_nlwp:-}" \
    --arg runtime_psr "${runtime_psr:-}" \
    '{
      captured_at: $captured_at,
      captured_at_unix_ms: ($captured_at_unix_ms | tonumber),
      label: $label,
      service_name: $service,
      host: {
        hostname: $hostname,
        cpu_cores: ($cpu_cores | tonumber),
        loadavg_1m: ($load1 | tonumber),
        loadavg_5m: ($load5 | tonumber),
        loadavg_15m: ($load15 | tonumber),
        mem_total_bytes: ($mem_total_bytes | tonumber),
        mem_available_bytes: ($mem_available_bytes | tonumber)
      },
      storage: {
        path: $storage_path,
        total_bytes: (if ($storage_total_bytes | length) == 0 then null else ($storage_total_bytes | tonumber) end),
        used_bytes: (if ($storage_used_bytes | length) == 0 then null else ($storage_used_bytes | tonumber) end),
        available_bytes: (if ($storage_available_bytes | length) == 0 then null else ($storage_available_bytes | tonumber) end),
        used_percent: (if ($storage_used_percent | length) == 0 then null else ($storage_used_percent | tonumber) end)
      },
      service: {
        active_state: $service_active_state,
        sub_state: $service_sub_state,
        main_pid: (if ($service_main_pid | length) == 0 then null else ($service_main_pid | tonumber) end)
      },
      wrapper: {
        present: (($service_main_pid | length) > 0 and $service_main_pid != "0"),
        pid: (if ($service_main_pid | length) == 0 then null else ($service_main_pid | tonumber) end),
        pcpu: (if ($wrapper_pcpu | length) == 0 then null else ($wrapper_pcpu | tonumber) end),
        pmem: (if ($wrapper_pmem | length) == 0 then null else ($wrapper_pmem | tonumber) end),
        elapsed_secs: (if ($wrapper_elapsed_secs | length) == 0 then null else ($wrapper_elapsed_secs | tonumber) end),
        nlwp: (if ($wrapper_nlwp | length) == 0 then null else ($wrapper_nlwp | tonumber) end),
        psr: (if ($wrapper_psr | length) == 0 then null else ($wrapper_psr | tonumber) end)
      },
      runtime: {
        present: (($runtime_pid | length) > 0),
        pid: (if ($runtime_pid | length) == 0 then null else ($runtime_pid | tonumber) end),
        pcpu: (if ($runtime_pcpu | length) == 0 then null else ($runtime_pcpu | tonumber) end),
        pmem: (if ($runtime_pmem | length) == 0 then null else ($runtime_pmem | tonumber) end),
        elapsed_secs: (if ($runtime_elapsed_secs | length) == 0 then null else ($runtime_elapsed_secs | tonumber) end),
        nlwp: (if ($runtime_nlwp | length) == 0 then null else ($runtime_nlwp | tonumber) end),
        psr: (if ($runtime_psr | length) == 0 then null else ($runtime_psr | tonumber) end)
      }
    }' >>"$samples_ndjson"
}

started_at=$(date -Iseconds)
for ((sample_index = 1; sample_index <= samples; sample_index++)); do
  captured_at=$(date -Iseconds)
  captured_at_unix_ms=$(( $(date +%s) * 1000 ))
  echo "host sample $sample_index/$samples @ $captured_at"

  capture_sample local_node local "$local_service" "$local_storage_path"
  capture_sample sequencer_ecs remote "$sequencer_service" "$sequencer_storage_path" "$sequencer_target" "$seq_password"
  capture_sample storage_ecs remote "$storage_service" "$storage_storage_path" "$storage_target" "$storage_password"

  if (( sample_index < samples )); then
    sleep "$interval_secs"
  fi
done
ended_at=$(date -Iseconds)

python3 "$repo_root/scripts/p2p-real-env-host-summary.py" \
  --history-path "$samples_ndjson" \
  --summary-json "$summary_json" \
  --summary-md "$summary_md" \
  --run-id "$run_id" \
  --run-dir "$run_dir"

cp "$summary_json" "$latest_summary_json"
cp "$summary_md" "$latest_summary_md"

echo "p2p real-env host monitor complete"
echo "  run_dir: $run_dir"
echo "  started_at: $started_at"
echo "  ended_at: $ended_at"
echo "  summary_json: $summary_json"
echo "  summary_md: $summary_md"
