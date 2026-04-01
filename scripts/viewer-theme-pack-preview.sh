#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

theme_defaults_file="scripts/viewer-theme-defaults.env"
if [[ -f "$theme_defaults_file" ]]; then
  # shellcheck source=/dev/null
  source "$theme_defaults_file"
fi
default_theme_pack="${VIEWER_THEME_DEFAULT_PACK:-industrial_v3}"

usage() {
  cat <<'USAGE'
Usage: ./scripts/viewer-theme-pack-preview.sh [options]

Status:
  hold-only 3D theme preview helper while PRD-WORLD_SIMULATOR-041 keeps 3D work paused.

Options:
  --scenario <name>        oasis7_viewer_live scenario (default: llm_bootstrap)
  --theme-pack <name>      theme pack: industrial_v3,industrial_v2,industrial_v1 (default: industrial_v3)
  --base-port <port>       starting port for per-variant capture (default: 5423)
  --viewer-wait <sec>      viewer wait before capture (default: 10)
  --variants <list>        comma-separated variants: default,matte,glossy,all (default: all)
  --out-dir <dir>          output root (default: output/theme_preview/<timestamp>)
  --ui-profile-file <path> optional UI profile env file (default: scripts/viewer-release-ui-profile.env)
  --no-prewarm             pass --no-prewarm to all capture runs
  -h, --help               show help

Outputs:
  output/theme_preview/<timestamp>/<variant>/
    viewer.png live_server.log viewer.log meta.txt
USAGE
}

run() {
  echo "+ $*"
  "$@"
}

capture_status_value() {
  local status_file=$1
  local key=$2
  grep -E "^${key}=" "$status_file" | tail -n 1 | cut -d'=' -f2-
}

resolve_variants() {
  local raw=$1
  local normalized
  normalized=$(echo "$raw" | tr '[:upper:]' '[:lower:]')
  if [[ "$normalized" == "all" || -z "$normalized" ]]; then
    echo "default matte glossy"
    return 0
  fi

  local parsed=()
  IFS=',' read -r -a parsed <<<"$normalized"
  local item
  for item in "${parsed[@]}"; do
    case "$item" in
      default|matte|glossy)
        ;;
      *)
        echo "invalid variant: $item" >&2
        echo "supported variants: default,matte,glossy,all" >&2
        exit 2
        ;;
    esac
  done
  echo "${parsed[*]}"
}

scenario="llm_bootstrap"
theme_pack="$default_theme_pack"
base_port=5423
viewer_wait=10
variants_raw="all"
out_dir=""
force_no_prewarm=0
ui_profile_file="scripts/viewer-release-ui-profile.env"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --scenario)
      scenario=${2:-}
      shift 2
      ;;
    --theme-pack)
      theme_pack=${2:-}
      shift 2
      ;;
    --base-port)
      base_port=${2:-}
      shift 2
      ;;
    --viewer-wait)
      viewer_wait=${2:-}
      shift 2
      ;;
    --variants)
      variants_raw=${2:-}
      shift 2
      ;;
    --out-dir)
      out_dir=${2:-}
      shift 2
      ;;
    --ui-profile-file)
      ui_profile_file=${2:-}
      shift 2
      ;;
    --no-prewarm)
      force_no_prewarm=1
      shift
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

if [[ -n "$ui_profile_file" && ! -f "$ui_profile_file" ]]; then
  echo "missing --ui-profile-file: $ui_profile_file" >&2
  exit 1
fi

if [[ -z "$out_dir" ]]; then
  timestamp=$(date '+%Y%m%d_%H%M%S')
  out_dir="output/theme_preview/$timestamp"
fi

if [[ ! "$base_port" =~ ^[0-9]+$ ]]; then
  echo "--base-port must be an integer" >&2
  exit 2
fi

variants=($(resolve_variants "$variants_raw"))
mkdir -p "$out_dir"

case "$theme_pack" in
  industrial_v3)
    preset_dir="crates/oasis7_viewer/assets/themes/industrial_v3/presets"
    preset_prefix="industrial_v3"
    ;;
  industrial_v2)
    preset_dir="crates/oasis7_viewer/assets/themes/industrial_v2/presets"
    preset_prefix="industrial_v2"
    ;;
  industrial_v1)
    preset_dir="crates/oasis7_viewer/assets/themes/industrial_v1/presets"
    preset_prefix="industrial"
    ;;
  *)
    echo "invalid --theme-pack: $theme_pack" >&2
    echo "supported theme packs: industrial_v3,industrial_v2,industrial_v1" >&2
    exit 2
    ;;
esac

# Keep a deterministic composition that consistently includes visible location geometry.
automation_steps="mode=3d;focus=first_location;pan=0,2,0;zoom=1.2;orbit=10,-25;select=first_location;wait=0.4"

index=0
for variant in "${variants[@]}"; do
  port=$((base_port + index))
  variant_dir="$out_dir/$variant"
  mkdir -p "$variant_dir"
  preset_file="$preset_dir/${preset_prefix}_${variant}.env"
  if [[ ! -f "$preset_file" ]]; then
    echo "missing preset: $preset_file" >&2
    exit 1
  fi

  no_prewarm_arg=""
  if [[ "$force_no_prewarm" -eq 1 || "$index" -gt 0 ]]; then
    no_prewarm_arg="--no-prewarm"
  fi

  (
    source "$preset_file"
    if [[ -n "$ui_profile_file" ]]; then
      source "$ui_profile_file"
    fi
    run ./scripts/capture-viewer-frame.sh \
      --scenario "$scenario" \
      --addr "127.0.0.1:$port" \
      --viewer-wait "$viewer_wait" \
      --auto-focus-target first_location \
      --automation-steps "$automation_steps" \
      --keep-tmp \
      ${no_prewarm_arg:+$no_prewarm_arg}
  )

  capture_status_file=".tmp/screens/capture_status.txt"
  if [[ ! -s "$capture_status_file" ]]; then
    echo "missing capture status file: $capture_status_file (variant=$variant)" >&2
    exit 1
  fi
  capture_connection_status=$(capture_status_value "$capture_status_file" "connection_status")
  capture_snapshot_ready=$(capture_status_value "$capture_status_file" "snapshot_ready")
  capture_last_error=$(capture_status_value "$capture_status_file" "last_error")
  if [[ "$capture_connection_status" != "connected" || "$capture_snapshot_ready" != "1" ]]; then
    echo "theme preview capture connectivity gate failed: variant=$variant connection_status=${capture_connection_status:-unknown} snapshot_ready=${capture_snapshot_ready:-unknown}" >&2
    if [[ -n "$capture_last_error" ]]; then
      echo "last_error=$capture_last_error" >&2
    fi
    cat "$capture_status_file" >&2 || true
    exit 1
  fi

  cp .tmp/screens/window.png "$variant_dir/viewer.png"
  cp .tmp/screens/live_server.log "$variant_dir/live_server.log"
  cp .tmp/screens/viewer.log "$variant_dir/viewer.log"
  cp "$capture_status_file" "$variant_dir/capture_status.txt"
  cat >"$variant_dir/meta.txt" <<META
scenario=$scenario
variant=$variant
port=$port
viewer_wait=$viewer_wait
theme_pack=$theme_pack
preset_file=$preset_file
ui_profile_file=$ui_profile_file
capture_connection_status=$capture_connection_status
capture_snapshot_ready=$capture_snapshot_ready
META

  index=$((index + 1))
done

echo "theme preview artifacts: $out_dir"
