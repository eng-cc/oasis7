#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

usage() {
  cat <<'USAGE'
Usage: ./scripts/capture-viewer-frame.sh [options]

Options:
  --scenario <name>       oasis7_viewer_live scenario (default: llm_bootstrap)
  --addr <host:port>      bind/viewer address (default: 127.0.0.1:5023)
  --display <display>     Xvfb display id for Linux mode (default: :100)
  --width <px>            virtual screen width (default: 1280)
  --height <px>           virtual screen height (default: 800)
  --viewer-wait <sec>     wait before capture (default: 8)
  --capture-max-wait <s>  max wait for internal capture (macOS only)
  --auto-focus-target <target>
                          viewer auto-focus target (e.g. first_fragment, location:frag-1)
  --auto-focus-radius <n> viewer auto-focus radius override
  --auto-focus-keep-2d    keep 2D mode during auto-focus (default behavior)
  --auto-focus-force-3d   force switch to 3D during auto-focus (hold-only 3D inspection)
  --auto-select-target <target>
                          viewer auto-select target (e.g. first_agent, agent:agent-0)
  --automation-steps <s>  viewer automation steps (e.g. mode=2d;select=agent:agent-0)
  --llm                   enable --llm on oasis7_viewer_live
  --no-prewarm            skip prewarm cargo build step
  --keep-tmp              do not clear .tmp at start
  -h, --help              show help

Behavior:
  - Linux: uses Xvfb + xwininfo + ffmpeg
  - macOS: uses Bevy internal screenshot (no system screen-recording permission)
  - native fallback keeps 2D during auto-focus by default; switch to 3D only when
    explicitly inspecting paused/hold visual work
  - default prewarm: builds `oasis7_viewer_live` + `oasis7_viewer` first to reduce
    run-time compile wait and screenshot timeout risk

Output:
  .tmp/screens/
    live_server.log viewer.log xvfb.log
    root.png window.png window_line.txt window_geom.txt
USAGE
}

require_cmd() {
  local cmd=$1
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "missing required command: $cmd" >&2
    exit 1
  fi
}

run() {
  echo "+ $*"
  "$@"
}

viewer_env_value() {
  local suffix=$1
  local primary_key="OASIS7_VIEWER_${suffix}"
  printf '%s' "${!primary_key-}"
}

detect_platform() {
  case "$(uname -s)" in
    Linux)
      echo "linux"
      ;;
    Darwin)
      echo "macos"
      ;;
    *)
      echo "unsupported"
      ;;
  esac
}


VALID_SCENARIOS=(
  minimal
  two_bases
  llm_bootstrap
  power_bootstrap
  resource_bootstrap
  twin_region_bootstrap
  triad_region_bootstrap
  triad_p2p_bootstrap
  asteroid_fragment_bootstrap
  asteroid_fragment_detail_bootstrap
  asteroid_fragment_twin_region_bootstrap
  asteroid_fragment_triad_region_bootstrap
)

normalize_scenario_alias() {
  case "$1" in
    triad)
      echo "triad_region_bootstrap"
      ;;
    triad_p2p)
      echo "triad_p2p_bootstrap"
      ;;
    twin|twin_region)
      echo "twin_region_bootstrap"
      ;;
    asteroid_fragment)
      echo "asteroid_fragment_bootstrap"
      ;;
    asteroid_fragment_detail)
      echo "asteroid_fragment_detail_bootstrap"
      ;;
    asteroid_fragment_twin)
      echo "asteroid_fragment_twin_region_bootstrap"
      ;;
    asteroid_fragment_triad)
      echo "asteroid_fragment_triad_region_bootstrap"
      ;;
    *)
      echo "$1"
      ;;
  esac
}

is_valid_scenario() {
  local target=$1
  local item
  for item in "${VALID_SCENARIOS[@]}"; do
    if [[ "$item" == "$target" ]]; then
      return 0
    fi
  done
  return 1
}

scenario_list_csv() {
  local first=1
  local item
  for item in "${VALID_SCENARIOS[@]}"; do
    if [[ $first -eq 1 ]]; then
      printf "%s" "$item"
      first=0
    else
      printf ", %s" "$item"
    fi
  done
  printf "\n"
}

validate_scenario_or_exit() {
  local raw=$1
  local normalized
  normalized=$(normalize_scenario_alias "$raw")

  if [[ "$normalized" != "$raw" ]]; then
    echo "scenario alias mapped: $raw -> $normalized" >&2
  fi

  if is_valid_scenario "$normalized"; then
    echo "$normalized"
    return 0
  fi

  echo "invalid scenario: $raw" >&2
  echo "supported scenarios: $(scenario_list_csv)" >&2
  echo "common aliases: triad, triad_p2p, twin, asteroid_fragment, asteroid_fragment_detail" >&2
  exit 2
}

wait_linux_window_line() {
  local display=$1
  local line=""
  for _ in $(seq 1 30); do
    line=$(DISPLAY="$display" xwininfo -root -tree 2>/dev/null | grep -E "oasis7 Viewer" | head -n1 || true)
    if [[ -n "$line" ]]; then
      echo "$line"
      return 0
    fi
    sleep 1
  done
  return 1
}

wait_for_file() {
  local path=$1
  local timeout_secs=$2
  local steps=$((timeout_secs * 2))
  if [[ "$steps" -lt 1 ]]; then
    steps=1
  fi
  for _ in $(seq 1 "$steps"); do
    if [[ -s "$path" ]]; then
      return 0
    fi
    sleep 0.5
  done
  return 1
}

wait_for_tcp_port() {
  local host=$1
  local port=$2
  local timeout_secs=$3
  local step
  for step in $(seq 1 "$timeout_secs"); do
    if (echo >"/dev/tcp/$host/$port") >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  return 1
}

parse_addr_host_port() {
  local raw=$1
  if [[ "$raw" != *:* ]]; then
    echo "invalid --addr: $raw (expected host:port)" >&2
    exit 2
  fi

  local host=${raw%:*}
  local port=${raw##*:}
  if [[ -z "$host" || -z "$port" || ! "$port" =~ ^[0-9]+$ ]]; then
    echo "invalid --addr: $raw (expected host:port)" >&2
    exit 2
  fi

  printf '%s\n%s\n' "$host" "$port"
}

parse_truthy_flag() {
  local raw=${1:-}
  local normalized
  normalized=$(echo "$raw" | tr '[:upper:]' '[:lower:]')
  case "$normalized" in
    1|true|yes|on)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

resolve_capture_max_wait() {
  local viewer_wait=$1
  local capture_max_wait_override=$2

  local viewer_wait_int=${viewer_wait%.*}
  if [[ -z "$viewer_wait_int" ]]; then
    viewer_wait_int=8
  fi

  local extra_wait=20
  if parse_truthy_flag "$(viewer_env_value "SHOW_FRAGMENT_ELEMENTS")"; then
    extra_wait=60
  fi

  local capture_max_wait=$((viewer_wait_int + extra_wait))
  if [[ -n "$capture_max_wait_override" ]]; then
    if [[ ! "$capture_max_wait_override" =~ ^[0-9]+$ ]]; then
      echo "invalid --capture-max-wait: $capture_max_wait_override" >&2
      exit 1
    fi
    capture_max_wait=$capture_max_wait_override
  fi

  echo "$capture_max_wait"
}

prewarm_viewer_binaries() {
  run env -u RUSTC_WRAPPER cargo build -p oasis7 --bin oasis7_viewer_live
  run env -u RUSTC_WRAPPER cargo build -p oasis7_viewer
}

capture_linux() {
  local display=$1
  local width=$2
  local height=$3
  local viewer_wait=$4
  local addr=$5
  local viewer_log=$6
  local xvfb_log=$7
  local root_png=$8
  local window_png=$9
  local window_line_txt=${10}
  local window_geom_txt=${11}
  local auto_focus_enabled=${12:-0}
  local auto_focus_target=${13:-}
  local auto_focus_radius=${14:-}
  local auto_focus_force_3d=${15:-1}
  local auto_select_target=${16:-}
  local automation_steps=${17:-}
  local capture_status_txt=${18:-}

  echo "+ Xvfb $display -screen 0 ${width}x${height}x24 > $xvfb_log"
  Xvfb "$display" -screen 0 "${width}x${height}x24" >"$xvfb_log" 2>&1 &
  XVFB_PID=$!

  sleep 2

  if [[ "$auto_focus_enabled" == "1" ]]; then
    echo "+ DISPLAY=$display OASIS7_VIEWER_AUTO_FOCUS=1 OASIS7_VIEWER_AUTO_FOCUS_TARGET=${auto_focus_target:-first_fragment} OASIS7_VIEWER_AUTO_FOCUS_FORCE_3D=$auto_focus_force_3d ${auto_focus_radius:+OASIS7_VIEWER_AUTO_FOCUS_RADIUS=$auto_focus_radius }${auto_select_target:+OASIS7_VIEWER_AUTO_SELECT=1 OASIS7_VIEWER_AUTO_SELECT_TARGET=$auto_select_target }${automation_steps:+OASIS7_VIEWER_AUTOMATION_STEPS=$automation_steps }env -u RUSTC_WRAPPER cargo run -p oasis7_viewer -- $addr > $viewer_log"
    DISPLAY="$display" \
    OASIS7_VIEWER_CAPTURE_STATUS_PATH="$capture_status_txt" \
    OASIS7_VIEWER_AUTO_FOCUS="1" \
    OASIS7_VIEWER_AUTO_FOCUS_TARGET="${auto_focus_target:-first_fragment}" \
    OASIS7_VIEWER_AUTO_FOCUS_FORCE_3D="$auto_focus_force_3d" \
    OASIS7_VIEWER_AUTO_FOCUS_RADIUS="$auto_focus_radius" \
    OASIS7_VIEWER_AUTO_SELECT="${auto_select_target:+1}" \
    OASIS7_VIEWER_AUTO_SELECT_TARGET="$auto_select_target" \
    OASIS7_VIEWER_AUTOMATION_STEPS="$automation_steps" \
    env -u RUSTC_WRAPPER cargo run -p oasis7_viewer -- "$addr" >"$viewer_log" 2>&1 &
  else
    echo "+ DISPLAY=$display ${auto_select_target:+OASIS7_VIEWER_AUTO_SELECT=1 OASIS7_VIEWER_AUTO_SELECT_TARGET=$auto_select_target }${automation_steps:+OASIS7_VIEWER_AUTOMATION_STEPS=$automation_steps }env -u RUSTC_WRAPPER cargo run -p oasis7_viewer -- $addr > $viewer_log"
    DISPLAY="$display" \
    OASIS7_VIEWER_CAPTURE_STATUS_PATH="$capture_status_txt" \
    OASIS7_VIEWER_AUTO_SELECT="${auto_select_target:+1}" \
    OASIS7_VIEWER_AUTO_SELECT_TARGET="$auto_select_target" \
    OASIS7_VIEWER_AUTOMATION_STEPS="$automation_steps" \
    env -u RUSTC_WRAPPER cargo run -p oasis7_viewer -- "$addr" >"$viewer_log" 2>&1 &
  fi
  VIEWER_PID=$!

  local window_line
  if ! window_line=$(wait_linux_window_line "$display"); then
    echo "failed to find window: oasis7 Viewer" >&2
    exit 2
  fi
  echo "$window_line" > "$window_line_txt"

  sleep "$viewer_wait"

  run ffmpeg -y -f x11grab -video_size "${width}x${height}" -i "${display}.0" -frames:v 1 "$root_png"

  local window_geom
  window_geom=$(echo "$window_line" | sed -n 's/.* \([0-9]\+x[0-9]\++[0-9]\++[0-9]\+\).*/\1/p')
  if [[ -n "$window_geom" ]]; then
    echo "$window_geom" > "$window_geom_txt"
    local window_size window_x window_y
    window_size=$(echo "$window_geom" | cut -d+ -f1)
    window_x=$(echo "$window_geom" | cut -d+ -f2)
    window_y=$(echo "$window_geom" | cut -d+ -f3)
    run ffmpeg -y -f x11grab -video_size "$window_size" -i "${display}.0+${window_x},${window_y}" -frames:v 1 "$window_png"
  fi
}

capture_macos() {
  local viewer_wait=$1
  local capture_max_wait_override=$2
  local addr=$3
  local viewer_log=$4
  local xvfb_log=$5
  local root_png=$6
  local window_png=$7
  local window_line_txt=$8
  local window_geom_txt=$9
  local auto_focus_target=${10:-}
  local auto_focus_radius=${11:-}
  local auto_focus_force_3d=${12:-1}
  local auto_focus_enabled=${13:-0}
  local auto_select_target=${14:-}
  local automation_steps=${15:-}
  local capture_status_txt=${16:-}

  local capture_max_wait
  capture_max_wait=$(resolve_capture_max_wait "$viewer_wait" "$capture_max_wait_override")

  echo "macOS mode: Bevy internal screenshot (no Xvfb)" > "$xvfb_log"
  echo "bevy_internal_capture oasis7 Viewer" > "$window_line_txt"
  echo "internal" > "$window_geom_txt"

  local viewer_cmd=(env -u RUSTC_WRAPPER cargo run -p oasis7_viewer -- "$addr")
  if [[ "$auto_focus_enabled" == "1" ]]; then
    echo "+ OASIS7_VIEWER_CAPTURE_PATH=$window_png OASIS7_VIEWER_AUTO_FOCUS=1 OASIS7_VIEWER_AUTO_FOCUS_TARGET=${auto_focus_target:-first_fragment} OASIS7_VIEWER_AUTO_FOCUS_FORCE_3D=$auto_focus_force_3d ${auto_focus_radius:+OASIS7_VIEWER_AUTO_FOCUS_RADIUS=$auto_focus_radius }${auto_select_target:+OASIS7_VIEWER_AUTO_SELECT=1 OASIS7_VIEWER_AUTO_SELECT_TARGET=$auto_select_target }${automation_steps:+OASIS7_VIEWER_AUTOMATION_STEPS=$automation_steps }${viewer_cmd[*]} > $viewer_log"
    OASIS7_VIEWER_CAPTURE_PATH="$window_png" \
    OASIS7_VIEWER_CAPTURE_STATUS_PATH="$capture_status_txt" \
    OASIS7_VIEWER_CAPTURE_DELAY_SECS="$viewer_wait" \
    OASIS7_VIEWER_CAPTURE_MAX_WAIT_SECS="$capture_max_wait" \
    OASIS7_VIEWER_AUTO_FOCUS_FORCE_3D="$auto_focus_force_3d" \
    OASIS7_VIEWER_AUTO_FOCUS="1" \
    OASIS7_VIEWER_AUTO_FOCUS_TARGET="${auto_focus_target:-first_fragment}" \
    OASIS7_VIEWER_AUTO_FOCUS_RADIUS="$auto_focus_radius" \
    OASIS7_VIEWER_AUTO_SELECT="${auto_select_target:+1}" \
    OASIS7_VIEWER_AUTO_SELECT_TARGET="$auto_select_target" \
    OASIS7_VIEWER_AUTOMATION_STEPS="$automation_steps" \
    "${viewer_cmd[@]}" >"$viewer_log" 2>&1 &
  else
    echo "+ OASIS7_VIEWER_CAPTURE_PATH=$window_png ${auto_select_target:+OASIS7_VIEWER_AUTO_SELECT=1 OASIS7_VIEWER_AUTO_SELECT_TARGET=$auto_select_target }${automation_steps:+OASIS7_VIEWER_AUTOMATION_STEPS=$automation_steps }${viewer_cmd[*]} > $viewer_log"
    OASIS7_VIEWER_CAPTURE_PATH="$window_png" \
    OASIS7_VIEWER_CAPTURE_STATUS_PATH="$capture_status_txt" \
    OASIS7_VIEWER_CAPTURE_DELAY_SECS="$viewer_wait" \
    OASIS7_VIEWER_CAPTURE_MAX_WAIT_SECS="$capture_max_wait" \
    OASIS7_VIEWER_AUTO_SELECT="${auto_select_target:+1}" \
    OASIS7_VIEWER_AUTO_SELECT_TARGET="$auto_select_target" \
    OASIS7_VIEWER_AUTOMATION_STEPS="$automation_steps" \
    "${viewer_cmd[@]}" >"$viewer_log" 2>&1 &
  fi
  VIEWER_PID=$!

  if ! wait_for_file "$window_png" "$capture_max_wait"; then
    echo "failed to generate internal viewer capture: $window_png" >&2
    echo "capture timeout: wait=${capture_max_wait}s (viewer-wait=${viewer_wait}s)" >&2
    if [[ -s "$viewer_log" ]]; then
      echo "---- viewer.log tail ----" >&2
      tail -n 60 "$viewer_log" >&2 || true
      echo "-------------------------" >&2
    fi
    exit 3
  fi

  cp "$window_png" "$root_png"

  # viewer should exit automatically after capture; tolerate delayed termination
  wait "$VIEWER_PID" >/dev/null 2>&1 || true
}

scenario="llm_bootstrap"
addr="127.0.0.1:5023"
display=":100"
width="1280"
height="800"
viewer_wait="8"
auto_focus_target=""
auto_focus_radius=""
auto_focus_force_3d="0"
auto_focus_enabled="0"
auto_select_target=""
automation_steps=""
capture_max_wait=""
enable_llm=0
prewarm=1
keep_tmp=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --scenario)
      scenario=${2:-}
      shift 2
      ;;
    --addr)
      addr=${2:-}
      shift 2
      ;;
    --display)
      display=${2:-}
      shift 2
      ;;
    --width)
      width=${2:-}
      shift 2
      ;;
    --height)
      height=${2:-}
      shift 2
      ;;
    --viewer-wait)
      viewer_wait=${2:-}
      shift 2
      ;;
    --capture-max-wait)
      capture_max_wait=${2:-}
      shift 2
      ;;
    --auto-focus-target)
      auto_focus_target=${2:-}
      auto_focus_enabled="1"
      shift 2
      ;;
    --auto-focus-radius)
      auto_focus_radius=${2:-}
      auto_focus_enabled="1"
      shift 2
      ;;
    --auto-focus-keep-2d)
      auto_focus_force_3d="0"
      auto_focus_enabled="1"
      shift
      ;;
    --auto-focus-force-3d)
      auto_focus_force_3d="1"
      auto_focus_enabled="1"
      shift
      ;;
    --auto-select-target)
      auto_select_target=${2:-}
      shift 2
      ;;
    --automation-steps)
      automation_steps=${2:-}
      shift 2
      ;;
    --llm)
      enable_llm=1
      shift
      ;;
    --no-prewarm)
      prewarm=0
      shift
      ;;
    --keep-tmp)
      keep_tmp=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

scenario=$(validate_scenario_or_exit "$scenario")

platform=$(detect_platform)
if [[ "$platform" == "unsupported" ]]; then
  echo "unsupported platform: $(uname -s)" >&2
  exit 1
fi

require_cmd cargo
if [[ "$platform" == "linux" ]]; then
  require_cmd Xvfb
  require_cmd xwininfo
  require_cmd ffmpeg
fi

if [[ $prewarm -eq 1 ]]; then
  prewarm_viewer_binaries
fi

if [[ $keep_tmp -eq 0 ]]; then
  run rm -rf .tmp
fi

out_dir=".tmp/screens"
run mkdir -p "$out_dir"

server_log="$out_dir/live_server.log"
viewer_log="$out_dir/viewer.log"
xvfb_log="$out_dir/xvfb.log"
root_png="$out_dir/root.png"
window_png="$out_dir/window.png"
window_line_txt="$out_dir/window_line.txt"
window_geom_txt="$out_dir/window_geom.txt"
capture_status_txt="$out_dir/capture_status.txt"

cleanup() {
  local pid
  for pid in "${VIEWER_PID:-}" "${SERVER_PID:-}" "${XVFB_PID:-}"; do
    if [[ -n "${pid:-}" ]] && kill -0 "$pid" 2>/dev/null; then
      kill "$pid" >/dev/null 2>&1 || true
      wait "$pid" >/dev/null 2>&1 || true
    fi
  done
}
trap cleanup EXIT

server_cmd=(env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_viewer_live -- "$scenario" --bind "$addr")
if [[ $enable_llm -eq 1 ]]; then
  server_cmd+=(--llm)
fi

echo "+ ${server_cmd[*]} > $server_log"
"${server_cmd[@]}" >"$server_log" 2>&1 &
SERVER_PID=$!

run rm -f "$capture_status_txt"
while IFS= read -r line; do
  if [[ -z "${server_host:-}" ]]; then
    server_host=$line
  else
    server_port=$line
  fi
done < <(parse_addr_host_port "$addr")
echo "+ wait for viewer server $server_host:$server_port"
if ! wait_for_tcp_port "$server_host" "$server_port" 60; then
  echo "viewer server did not come up on $server_host:$server_port" >&2
  if [[ -s "$server_log" ]]; then
    echo "---- live_server.log tail ----" >&2
    tail -n 80 "$server_log" >&2 || true
    echo "------------------------------" >&2
  fi
  exit 4
fi

if [[ "$platform" == "linux" ]]; then
  capture_linux "$display" "$width" "$height" "$viewer_wait" "$addr" "$viewer_log" "$xvfb_log" "$root_png" "$window_png" "$window_line_txt" "$window_geom_txt" "$auto_focus_enabled" "$auto_focus_target" "$auto_focus_radius" "$auto_focus_force_3d" "$auto_select_target" "$automation_steps" "$capture_status_txt"
else
  capture_macos "$viewer_wait" "$capture_max_wait" "$addr" "$viewer_log" "$xvfb_log" "$root_png" "$window_png" "$window_line_txt" "$window_geom_txt" "$auto_focus_target" "$auto_focus_radius" "$auto_focus_force_3d" "$auto_focus_enabled" "$auto_select_target" "$automation_steps" "$capture_status_txt"
fi

echo "capture complete"
echo "  mode:   $platform"
echo "  root:   $root_png"
echo "  window: $window_png"
echo "  status: $capture_status_txt"
echo "  logs:   $server_log, $viewer_log"
