#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 || $# -gt 3 ]]; then
  echo "usage: $0 <service> <storage-path> [runtime-match]" >&2
  exit 2
fi

service="$1"
storage_path="$2"
runtime_match="${3:-oasis7_chain_runtime}"

emit_assignment() {
  local key=$1
  local value=${2-}
  printf '%s=%q\n' "$key" "$value"
}

parse_ps_metrics() {
  local pid=$1
  if [[ -z "$pid" ]] || [[ ! "$pid" =~ ^[0-9]+$ ]] || (( pid <= 0 )); then
    echo ""
    return 0
  fi
  ps -p "$pid" -o pcpu= -o pmem= -o etimes= -o nlwp= -o psr= 2>/dev/null | awk 'NR==1 {print $1" "$2" "$3" "$4" "$5}'
}

hostname_value="$(hostname 2>/dev/null || echo unknown)"
cpu_cores="$(nproc 2>/dev/null || echo 0)"

loadavg_raw="$(cat /proc/loadavg 2>/dev/null || echo "0.00 0.00 0.00 0/0 0")"
read -r load1 load5 load15 _ <<<"$loadavg_raw"

mem_total_kb="$(awk '/^MemTotal:/ {print $2}' /proc/meminfo 2>/dev/null || echo 0)"
mem_available_kb="$(awk '/^MemAvailable:/ {print $2}' /proc/meminfo 2>/dev/null || echo 0)"
mem_total_bytes=$(( ${mem_total_kb:-0} * 1024 ))
mem_available_bytes=$(( ${mem_available_kb:-0} * 1024 ))

storage_total_bytes=""
storage_used_bytes=""
storage_available_bytes=""
storage_used_percent=""
if df_output="$(df -B1 "$storage_path" 2>/dev/null | awk 'NR==2 {print $2" "$3" "$4" "$5}')"; then
  read -r storage_total_bytes storage_used_bytes storage_available_bytes storage_used_percent <<<"$df_output"
fi
storage_used_percent="${storage_used_percent%%%}"

service_show="$(systemctl show "$service" -p ActiveState -p SubState -p MainPID --no-pager 2>/dev/null || true)"
service_active_state="$(printf '%s\n' "$service_show" | awk -F= '/^ActiveState=/ {print $2}')"
service_sub_state="$(printf '%s\n' "$service_show" | awk -F= '/^SubState=/ {print $2}')"
service_main_pid="$(printf '%s\n' "$service_show" | awk -F= '/^MainPID=/ {print $2}')"

wrapper_pcpu=""
wrapper_pmem=""
wrapper_elapsed_secs=""
wrapper_nlwp=""
wrapper_psr=""
runtime_pid=""
runtime_pcpu=""
runtime_pmem=""
runtime_elapsed_secs=""
runtime_nlwp=""
runtime_psr=""

wrapper_metrics="$(parse_ps_metrics "$service_main_pid")"
if [[ -n "$wrapper_metrics" ]]; then
  read -r wrapper_pcpu wrapper_pmem wrapper_elapsed_secs wrapper_nlwp wrapper_psr <<<"$wrapper_metrics"
fi

if [[ -n "$service_main_pid" ]] && [[ "$service_main_pid" =~ ^[0-9]+$ ]] && (( service_main_pid > 0 )); then
  runtime_pid="$(pgrep -P "$service_main_pid" -f "$runtime_match" 2>/dev/null | head -n 1 || true)"
  if [[ -z "$runtime_pid" ]]; then
    main_args="$(ps -p "$service_main_pid" -o args= 2>/dev/null || true)"
    if [[ "$main_args" == *"$runtime_match"* ]]; then
      runtime_pid="$service_main_pid"
    fi
  fi
fi

runtime_metrics="$(parse_ps_metrics "$runtime_pid")"
if [[ -n "$runtime_metrics" ]]; then
  read -r runtime_pcpu runtime_pmem runtime_elapsed_secs runtime_nlwp runtime_psr <<<"$runtime_metrics"
fi

emit_assignment hostname "$hostname_value"
emit_assignment cpu_cores "$cpu_cores"
emit_assignment load1 "${load1:-0}"
emit_assignment load5 "${load5:-0}"
emit_assignment load15 "${load15:-0}"
emit_assignment mem_total_bytes "${mem_total_bytes:-0}"
emit_assignment mem_available_bytes "${mem_available_bytes:-0}"
emit_assignment storage_path "$storage_path"
emit_assignment storage_total_bytes "${storage_total_bytes:-}"
emit_assignment storage_used_bytes "${storage_used_bytes:-}"
emit_assignment storage_available_bytes "${storage_available_bytes:-}"
emit_assignment storage_used_percent "${storage_used_percent:-}"
emit_assignment service_active_state "${service_active_state:-unknown}"
emit_assignment service_sub_state "${service_sub_state:-unknown}"
emit_assignment service_main_pid "${service_main_pid:-0}"
emit_assignment wrapper_pcpu "${wrapper_pcpu:-}"
emit_assignment wrapper_pmem "${wrapper_pmem:-}"
emit_assignment wrapper_elapsed_secs "${wrapper_elapsed_secs:-}"
emit_assignment wrapper_nlwp "${wrapper_nlwp:-}"
emit_assignment wrapper_psr "${wrapper_psr:-}"
emit_assignment runtime_pid "${runtime_pid:-}"
emit_assignment runtime_pcpu "${runtime_pcpu:-}"
emit_assignment runtime_pmem "${runtime_pmem:-}"
emit_assignment runtime_elapsed_secs "${runtime_elapsed_secs:-}"
emit_assignment runtime_nlwp "${runtime_nlwp:-}"
emit_assignment runtime_psr "${runtime_psr:-}"
