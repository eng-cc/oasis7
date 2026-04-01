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
Usage: ./scripts/viewer-texture-inspector.sh [options]

Purpose:
  Preview texture sets on a stable viewer composition and capture screenshots.
  Texture sources can be selected from theme preset entity slots.
  Status: hold-only 3D lookdev tool while PRD-WORLD_SIMULATOR-041 keeps 3D work paused.

Options:
  --preset-file <path>     preset env file (default: industrial_v3_default.env)
  --inspect <list>         entity source list: agent,location,asset,power_plant,all (default: all)
  --variants <list>        default,matte,glossy,all (default: all)
  --scenario <name>        oasis7_viewer_live scenario (default: llm_bootstrap)
  --base-port <port>       start port per capture (default: 6123)
  --viewer-wait <sec>      viewer wait before capture (default: 8)
  --render-profile <name>  debug,balanced,cinematic (default: cinematic)
  --fragment-strategy <s>  readability,fidelity (default: fidelity)
  --base-texture <path>    override source base texture
  --normal-texture <path>  override source normal texture
  --mr-texture <path>      override source metallic_roughness texture
  --emissive-texture <p>   override source emissive texture
  --base-texture-template <p>
                           variant texture template, supports {variant}
  --normal-texture-template <p>
                           variant normal template, supports {variant}
  --mr-texture-template <p>
                           variant metallic_roughness template, supports {variant}
  --emissive-texture-template <p>
                           variant emissive template, supports {variant}
  --use-source-mesh        use inspected entity mesh as location mesh in preview
  --out-dir <dir>          output root (default: output/texture_inspector/<timestamp>)
  --art-capture            enable art-review mode (director ui + source mesh + crop output)
  --automation-steps <s>   override viewer automation steps for all captures
  --closeup-automation-steps <s>
                           override closeup automation steps for all captures
  --composition-profile <id>
                           legacy,art_review_v2 (default: art_review_v2)
  --art-lighting           enable art-review lighting preset
  --no-art-lighting        disable art-review lighting preset
  --lighting-profile <id>  art_review_v1,art_review_v2 (default: art_review_v2)
  --resource-pack-file <p> optional env file for entity/variant mesh/texture/material overrides
  --ui-profile-file <path> optional UI profile env file (default: scripts/viewer-release-ui-profile.env)
  --art-hide-panel         hide right panel in art capture (default: on when art_capture=1)
  --no-art-hide-panel      keep right panel visible in art capture
  --art-selection-highlight
                           keep selection highlight+halo (default: auto off when art_capture=1)
  --no-art-selection-highlight
                           disable selection highlight+halo
  --variant-ssim-threshold <f>
                           power variant validation threshold (default: 0.9995)
  --detail-edge-threshold <f>
                           closeup detail edge threshold (default: 0.35)
  --semantic-gate-mode <m> off,auto,strict (default: strict)
  --crop-window <w:h:x:y>  crop window for viewer_art.png; use 'none' or 'auto'
  --preview-mode <mode>    scene_proxy,lookdev,direct_entity (default: scene_proxy)
  --material-profile <id>  theme_default,art_review_v1 (default: theme_default)
  --no-prewarm             pass --no-prewarm to all capture runs
  -h, --help               show help

Outputs:
  output/texture_inspector/<timestamp>/<entity>/<variant>/
    viewer.png viewer_art.png viewer_closeup.png viewer_art_closeup.png
    live_server.log viewer.log live_server_closeup.log viewer_closeup.log meta.txt
USAGE
}

run() {
  echo "+ $*"
  "$@"
}

# shellcheck source=/dev/null
source "scripts/viewer-texture-inspector-lib.sh"

if ! preset_file=$(resolve_default_preset_file_for_pack "$default_theme_pack"); then
  echo "invalid VIEWER_THEME_DEFAULT_PACK in $theme_defaults_file: $default_theme_pack" >&2
  echo "supported theme packs: industrial_v3,industrial_v2,industrial_v1" >&2
  exit 2
fi
inspect_raw="all"
variants_raw="all"
scenario="llm_bootstrap"
base_port=6123
viewer_wait=8
render_profile="cinematic"
fragment_strategy="fidelity"
out_dir=""
force_no_prewarm=0
use_source_mesh=0
art_capture=0
automation_steps_override=""
closeup_automation_steps_override=""
composition_profile="art_review_v2"
art_lighting_mode="auto"
lighting_profile="art_review_v2"
resource_pack_file=""
ui_profile_file="scripts/viewer-release-ui-profile.env"
art_hide_panel_mode="auto"
art_selection_highlight_mode="auto"
variant_ssim_threshold="0.9995"
detail_edge_threshold="0.35"
semantic_gate_mode="strict"
crop_window_raw=""
preview_mode="scene_proxy"
material_profile="theme_default"

override_base_texture=""
override_normal_texture=""
override_mr_texture=""
override_emissive_texture=""
override_base_texture_template=""
override_normal_texture_template=""
override_mr_texture_template=""
override_emissive_texture_template=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --preset-file)
      preset_file=${2:-}
      shift 2
      ;;
    --inspect)
      inspect_raw=${2:-}
      shift 2
      ;;
    --variants)
      variants_raw=${2:-}
      shift 2
      ;;
    --scenario)
      scenario=${2:-}
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
    --render-profile)
      render_profile=${2:-}
      shift 2
      ;;
    --fragment-strategy)
      fragment_strategy=${2:-}
      shift 2
      ;;
    --base-texture)
      override_base_texture=${2:-}
      shift 2
      ;;
    --normal-texture)
      override_normal_texture=${2:-}
      shift 2
      ;;
    --mr-texture)
      override_mr_texture=${2:-}
      shift 2
      ;;
    --emissive-texture)
      override_emissive_texture=${2:-}
      shift 2
      ;;
    --base-texture-template)
      override_base_texture_template=${2:-}
      shift 2
      ;;
    --normal-texture-template)
      override_normal_texture_template=${2:-}
      shift 2
      ;;
    --mr-texture-template)
      override_mr_texture_template=${2:-}
      shift 2
      ;;
    --emissive-texture-template)
      override_emissive_texture_template=${2:-}
      shift 2
      ;;
    --use-source-mesh)
      use_source_mesh=1
      shift
      ;;
    --out-dir)
      out_dir=${2:-}
      shift 2
      ;;
    --art-capture)
      art_capture=1
      shift
      ;;
    --automation-steps)
      automation_steps_override=${2:-}
      shift 2
      ;;
    --closeup-automation-steps)
      closeup_automation_steps_override=${2:-}
      shift 2
      ;;
    --composition-profile)
      composition_profile=${2:-}
      shift 2
      ;;
    --art-lighting)
      art_lighting_mode="on"
      shift
      ;;
    --no-art-lighting)
      art_lighting_mode="off"
      shift
      ;;
    --lighting-profile)
      lighting_profile=${2:-}
      shift 2
      ;;
    --resource-pack-file)
      resource_pack_file=${2:-}
      shift 2
      ;;
    --ui-profile-file)
      ui_profile_file=${2:-}
      shift 2
      ;;
    --art-hide-panel)
      art_hide_panel_mode="on"
      shift
      ;;
    --no-art-hide-panel)
      art_hide_panel_mode="off"
      shift
      ;;
    --art-selection-highlight)
      art_selection_highlight_mode="on"
      shift
      ;;
    --no-art-selection-highlight)
      art_selection_highlight_mode="off"
      shift
      ;;
    --variant-ssim-threshold)
      variant_ssim_threshold=${2:-}
      shift 2
      ;;
    --detail-edge-threshold)
      detail_edge_threshold=${2:-}
      shift 2
      ;;
    --semantic-gate-mode)
      semantic_gate_mode=${2:-}
      shift 2
      ;;
    --crop-window)
      crop_window_raw=${2:-}
      shift 2
      ;;
    --preview-mode)
      preview_mode=${2:-}
      shift 2
      ;;
    --material-profile)
      material_profile=${2:-}
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

case "$(echo "${ui_profile_file:-}" | tr '[:upper:]' '[:lower:]')" in
  none|off|disable)
    ui_profile_file=""
    ;;
  *)
    ;;
esac

if [[ ! -f "$preset_file" ]]; then
  echo "missing preset file: $preset_file" >&2
  exit 1
fi

if [[ ! "$base_port" =~ ^[0-9]+$ ]]; then
  echo "--base-port must be an integer" >&2
  exit 2
fi

case "$render_profile" in
  debug|balanced|cinematic)
    ;;
  *)
    echo "invalid --render-profile: $render_profile" >&2
    echo "supported render profiles: debug,balanced,cinematic" >&2
    exit 2
    ;;
esac

case "$fragment_strategy" in
  readability|fidelity)
    ;;
  *)
    echo "invalid --fragment-strategy: $fragment_strategy" >&2
    echo "supported fragment strategies: readability,fidelity" >&2
    exit 2
    ;;
esac

case "$preview_mode" in
  scene_proxy|lookdev|direct_entity)
    ;;
  *)
    echo "invalid --preview-mode: $preview_mode" >&2
    echo "supported preview modes: scene_proxy,lookdev,direct_entity" >&2
    exit 2
    ;;
esac

case "$material_profile" in
  theme_default|art_review_v1)
    ;;
  *)
    echo "invalid --material-profile: $material_profile" >&2
    echo "supported material profiles: theme_default,art_review_v1" >&2
    exit 2
    ;;
esac

case "$composition_profile" in
  legacy|art_review_v2)
    ;;
  *)
    echo "invalid --composition-profile: $composition_profile" >&2
    echo "supported composition profiles: legacy,art_review_v2" >&2
    exit 2
    ;;
esac

case "$lighting_profile" in
  art_review_v1|art_review_v2)
    ;;
  *)
    echo "invalid --lighting-profile: $lighting_profile" >&2
    echo "supported lighting profiles: art_review_v1,art_review_v2" >&2
    exit 2
    ;;
esac

case "$art_hide_panel_mode" in
  on|off|auto)
    ;;
  *)
    echo "invalid art panel mode: $art_hide_panel_mode" >&2
    echo "supported modes: on,off,auto" >&2
    exit 2
    ;;
esac

case "$art_selection_highlight_mode" in
  on|off|auto)
    ;;
  *)
    echo "invalid art selection highlight mode: $art_selection_highlight_mode" >&2
    echo "supported modes: on,off,auto" >&2
    exit 2
    ;;
esac

if [[ -n "$resource_pack_file" && ! -f "$resource_pack_file" ]]; then
  echo "missing --resource-pack-file: $resource_pack_file" >&2
  exit 1
fi

if [[ -n "$ui_profile_file" && ! -f "$ui_profile_file" ]]; then
  echo "missing --ui-profile-file: $ui_profile_file" >&2
  exit 1
fi

if [[ -z "$out_dir" ]]; then
  timestamp=$(date '+%Y%m%d_%H%M%S')
  out_dir="output/texture_inspector/$timestamp"
fi

variant_ssim_threshold=$(parse_unit_interval_float_or_exit "$variant_ssim_threshold" "--variant-ssim-threshold")
detail_edge_threshold=$(parse_non_negative_float_or_exit "$detail_edge_threshold" "--detail-edge-threshold")

case "$semantic_gate_mode" in
  off|auto|strict)
    ;;
  *)
    echo "invalid --semantic-gate-mode: $semantic_gate_mode" >&2
    echo "supported semantic gate modes: off,auto,strict" >&2
    exit 2
    ;;
esac

entities=($(resolve_entities "$inspect_raw"))
variants=($(resolve_variants "$variants_raw"))
mkdir -p "$out_dir"

default_automation_steps="mode=3d;focus=first_location;pan=0,2,0;zoom=1.2;orbit=10,-25;select=first_location;wait=0.4"
if [[ "$art_capture" -eq 1 && "$use_source_mesh" -eq 0 ]]; then
  if [[ "$preview_mode" != "direct_entity" ]]; then
    use_source_mesh=1
  fi
fi
art_panel_hidden=0
if [[ "$art_hide_panel_mode" == "on" ]]; then
  art_panel_hidden=1
elif [[ "$art_hide_panel_mode" == "auto" && "$art_capture" -eq 1 ]]; then
  art_panel_hidden=1
fi
art_selection_highlight_enabled=1
if [[ "$art_selection_highlight_mode" == "off" ]]; then
  art_selection_highlight_enabled=0
elif [[ "$art_selection_highlight_mode" == "auto" && "$art_capture" -eq 1 ]]; then
  art_selection_highlight_enabled=0
fi
if [[ -z "$crop_window_raw" ]]; then
  crop_window_raw="auto"
fi
crop_window=$(parse_crop_window "$crop_window_raw")

art_lighting_enabled=0
if [[ "$art_lighting_mode" == "on" ]]; then
  art_lighting_enabled=1
elif [[ "$art_lighting_mode" == "auto" && "$art_capture" -eq 1 ]]; then
  art_lighting_enabled=1
fi

capture_index=0
capture_variant_bundle() {
  local entity=$1
  local variant=$2
  local variant_dir=$3
  local port=$4
  local hero_steps=$5
  local closeup_steps=$6
  local no_prewarm_arg=$7
  local retry_attempt=$8
  local closeup_candidate_index=${9:-0}
  local closeup_candidate_total=${10:-1}
  local closeup_pose_label=${11:-initial}
  local capture_scenario="$scenario"
  local hero_steps_effective="$hero_steps"
  local closeup_steps_effective="$closeup_steps"
  local src_prefix
  src_prefix=$(entity_prefix "$entity")

  mkdir -p "$variant_dir"

  (
    # Load base theme preset first, then pin variant and inspector overrides.
    source "$preset_file"
    if [[ -n "$ui_profile_file" ]]; then
      source "$ui_profile_file"
    fi
    if [[ -n "$resource_pack_file" ]]; then
      source "$resource_pack_file"
    fi
    material_variant_preset_for_run="$variant"
    if [[ "$material_profile" == "art_review_v1" ]]; then
      material_variant_preset_for_run="default"
    fi
    export OASIS7_VIEWER_MATERIAL_VARIANT_PRESET="$material_variant_preset_for_run"
    export OASIS7_VIEWER_RENDER_PROFILE="$render_profile"
    export OASIS7_VIEWER_FRAGMENT_MATERIAL_STRATEGY="$fragment_strategy"
    if [[ "$art_selection_highlight_enabled" -eq 1 ]]; then
      set_or_unset_viewer_env "HIGHLIGHT_SELECTED" ""
    else
      set_or_unset_viewer_env "HIGHLIGHT_SELECTED" "0"
    fi
    apply_variant_material_profile "$material_profile" "$entity" "$variant"
    effective_preview_mode="$preview_mode"
    preview_mode_fallback_reason="none"
    if [[ "$preview_mode" == "direct_entity" && "$entity" == "location" ]]; then
      effective_preview_mode="scene_proxy"
      preview_mode_fallback_reason="location_direct_entity_not_applicable"
    fi
    if [[ "$effective_preview_mode" == "direct_entity" && "$entity" == "power_plant" ]]; then
      effective_preview_mode="lookdev"
      preview_mode_fallback_reason="power_direct_entity_fallback"
    fi
    capture_scenario=$(resolve_capture_scenario_for_entity "$entity" "$scenario" "$effective_preview_mode")
    if [[ -z "$automation_steps_override" ]]; then
      hero_steps_effective=$(default_automation_steps_for_entity "$entity" "$capture_scenario")
    fi
    if [[ -z "$closeup_automation_steps_override" && "$retry_attempt" -eq 0 ]]; then
      closeup_steps_effective=$(default_closeup_automation_steps_for_entity "$entity" "$capture_scenario")
    fi
    if [[ "$effective_preview_mode" != "direct_entity" && "$art_capture" -eq 1 ]]; then
      use_source_mesh=1
    fi
    if [[ "$effective_preview_mode" == "direct_entity" ]]; then
      export OASIS7_VIEWER_SHOW_LOCATIONS=0
    else
      export OASIS7_VIEWER_SHOW_LOCATIONS=1
    fi
    if [[ "$effective_preview_mode" == "direct_entity" && "$entity" == "agent" ]]; then
      export OASIS7_VIEWER_SHOW_AGENTS=1
    else
      export OASIS7_VIEWER_SHOW_AGENTS=0
    fi
    capture_auto_focus_target="first_location"
    if [[ "$effective_preview_mode" == "direct_entity" ]]; then
      capture_auto_focus_target=$(resolve_focus_target_for_entity "$entity" "$capture_scenario")
    fi
    location_interference_disabled=0
    if [[ "$effective_preview_mode" == "lookdev" || "$effective_preview_mode" == "direct_entity" ]]; then
      location_interference_disabled=1
      set_or_unset_viewer_env "LOCATION_SHELL_ENABLED" "0"
      set_or_unset_viewer_env "LOCATION_RADIATION_GLOW" "0"
      set_or_unset_viewer_env "LOCATION_DAMAGE_VISUAL" "0"
    else
      set_or_unset_viewer_env "LOCATION_SHELL_ENABLED" ""
      set_or_unset_viewer_env "LOCATION_RADIATION_GLOW" ""
      set_or_unset_viewer_env "LOCATION_DAMAGE_VISUAL" ""
    fi
    if [[ "$art_capture" -eq 1 ]]; then
      export OASIS7_VIEWER_EXPERIENCE_MODE="director"
      export OASIS7_VIEWER_PANEL_MODE="observe"
      export OASIS7_VIEWER_SHOW_OPS_NAV=0
      if [[ "$art_panel_hidden" -eq 1 ]]; then
        set_or_unset_viewer_env "PANEL_HIDDEN" "1"
      else
        set_or_unset_viewer_env "PANEL_HIDDEN" ""
      fi
    fi
    if [[ "$art_lighting_enabled" -eq 1 ]]; then
      apply_art_lighting_profile "$lighting_profile" "$entity" "$variant"
    fi

    src_mesh_key=$(viewer_env_key "${src_prefix}_MESH_ASSET")
    src_base_key=$(viewer_env_key "${src_prefix}_BASE_TEXTURE_ASSET")
    src_normal_key=$(viewer_env_key "${src_prefix}_NORMAL_TEXTURE_ASSET")
    src_mr_key=$(viewer_env_key "${src_prefix}_METALLIC_ROUGHNESS_TEXTURE_ASSET")
    src_emissive_key=$(viewer_env_key "${src_prefix}_EMISSIVE_TEXTURE_ASSET")

    src_mesh="${!src_mesh_key:-}"
    src_base="${!src_base_key:-}"
    src_normal="${!src_normal_key:-}"
    src_mr="${!src_mr_key:-}"
    src_emissive="${!src_emissive_key:-}"
    pack_mesh_override=$(resource_pack_value "$entity" "$variant" "MESH_ASSET")
    pack_base_texture_override=$(resource_pack_value "$entity" "$variant" "BASE_TEXTURE_ASSET")
    pack_normal_texture_override=$(resource_pack_value "$entity" "$variant" "NORMAL_TEXTURE_ASSET")
    pack_mr_texture_override=$(resource_pack_value "$entity" "$variant" "METALLIC_ROUGHNESS_TEXTURE_ASSET")
    pack_emissive_texture_override=$(resource_pack_value "$entity" "$variant" "EMISSIVE_TEXTURE_ASSET")
    pack_roughness_override=$(resource_pack_value "$entity" "$variant" "ROUGHNESS")
    pack_metallic_override=$(resource_pack_value "$entity" "$variant" "METALLIC")
    pack_emissive_boost_override=$(resource_pack_value "$entity" "$variant" "EMISSIVE_BOOST")
    pack_base_color_override=$(resource_pack_value "$entity" "$variant" "BASE_COLOR")
    pack_emissive_color_override=$(resource_pack_value "$entity" "$variant" "EMISSIVE_COLOR")
    resource_pack_override_hits=0
    for raw_override in \
      "$pack_mesh_override" \
      "$pack_base_texture_override" \
      "$pack_normal_texture_override" \
      "$pack_mr_texture_override" \
      "$pack_emissive_texture_override" \
      "$pack_roughness_override" \
      "$pack_metallic_override" \
      "$pack_emissive_boost_override" \
      "$pack_base_color_override" \
      "$pack_emissive_color_override"; do
      if [[ -n "$raw_override" ]]; then
        resource_pack_override_hits=$((resource_pack_override_hits + 1))
      fi
    done
    variant_base_texture_override=$(resolve_variant_texture_override "$override_base_texture" "$override_base_texture_template" "$variant")
    variant_normal_texture_override=$(resolve_variant_texture_override "$override_normal_texture" "$override_normal_texture_template" "$variant")
    variant_mr_texture_override=$(resolve_variant_texture_override "$override_mr_texture" "$override_mr_texture_template" "$variant")
    variant_emissive_texture_override=$(resolve_variant_texture_override "$override_emissive_texture" "$override_emissive_texture_template" "$variant")
    effective_mesh_asset="${pack_mesh_override:-$src_mesh}"
    effective_base_texture_asset="${variant_base_texture_override:-${pack_base_texture_override:-$src_base}}"
    effective_normal_texture_asset="${variant_normal_texture_override:-${pack_normal_texture_override:-$src_normal}}"
    effective_mr_texture_asset="${variant_mr_texture_override:-${pack_mr_texture_override:-$src_mr}}"
    effective_emissive_texture_asset="${variant_emissive_texture_override:-${pack_emissive_texture_override:-$src_emissive}}"

    if [[ "$material_profile" == "art_review_v1" ]]; then
      case "$entity:$variant" in
        power_plant:matte)
          effective_base_texture_asset=""
          effective_normal_texture_asset=""
          effective_mr_texture_asset=""
          effective_emissive_texture_asset=""
          ;;
        power_plant:glossy)
          effective_base_texture_asset=""
          effective_normal_texture_asset=""
          effective_mr_texture_asset=""
          effective_emissive_texture_asset=""
          ;;
        *)
          ;;
      esac
    fi

    if [[ "$effective_preview_mode" == "direct_entity" ]]; then
      if [[ "$use_source_mesh" -eq 1 ]]; then
        set_or_unset_viewer_env "${src_prefix}_MESH_ASSET" "$effective_mesh_asset"
      fi
      set_or_unset_viewer_env "${src_prefix}_BASE_TEXTURE_ASSET" "$effective_base_texture_asset"
      set_or_unset_viewer_env "${src_prefix}_NORMAL_TEXTURE_ASSET" "$effective_normal_texture_asset"
      set_or_unset_viewer_env "${src_prefix}_METALLIC_ROUGHNESS_TEXTURE_ASSET" "$effective_mr_texture_asset"
      set_or_unset_viewer_env "${src_prefix}_EMISSIVE_TEXTURE_ASSET" "$effective_emissive_texture_asset"
    else
      if [[ "$use_source_mesh" -eq 1 ]]; then
        set_or_unset_viewer_env "LOCATION_MESH_ASSET" "$effective_mesh_asset"
      fi

      set_or_unset_viewer_env "LOCATION_BASE_TEXTURE_ASSET" "$effective_base_texture_asset"
      set_or_unset_viewer_env "LOCATION_NORMAL_TEXTURE_ASSET" "$effective_normal_texture_asset"
      set_or_unset_viewer_env "LOCATION_METALLIC_ROUGHNESS_TEXTURE_ASSET" "$effective_mr_texture_asset"
      set_or_unset_viewer_env "LOCATION_EMISSIVE_TEXTURE_ASSET" "$effective_emissive_texture_asset"
    fi

    if [[ "$entity" == "power_plant" ]]; then
      if [[ -n "$pack_roughness_override" ]]; then
        set_or_unset_viewer_env "MATERIAL_POWER_PLANT_ROUGHNESS" "$pack_roughness_override"
      fi
      if [[ -n "$pack_metallic_override" ]]; then
        set_or_unset_viewer_env "MATERIAL_POWER_PLANT_METALLIC" "$pack_metallic_override"
      fi
      if [[ -n "$pack_emissive_boost_override" ]]; then
        set_or_unset_viewer_env "MATERIAL_POWER_PLANT_EMISSIVE_BOOST" "$pack_emissive_boost_override"
      fi
      if [[ -n "$pack_base_color_override" ]]; then
        set_or_unset_viewer_env "POWER_PLANT_BASE_COLOR" "$pack_base_color_override"
      fi
      if [[ -n "$pack_emissive_color_override" ]]; then
        set_or_unset_viewer_env "POWER_PLANT_EMISSIVE_COLOR" "$pack_emissive_color_override"
      fi
    fi

    direct_entity_mesh_key=$(viewer_env_key "${src_prefix}_MESH_ASSET")
    direct_entity_base_key=$(viewer_env_key "${src_prefix}_BASE_TEXTURE_ASSET")
    direct_entity_normal_key=$(viewer_env_key "${src_prefix}_NORMAL_TEXTURE_ASSET")
    direct_entity_mr_key=$(viewer_env_key "${src_prefix}_METALLIC_ROUGHNESS_TEXTURE_ASSET")
    direct_entity_emissive_key=$(viewer_env_key "${src_prefix}_EMISSIVE_TEXTURE_ASSET")
    direct_entity_mesh_asset="${!direct_entity_mesh_key:-}"
    direct_entity_base_texture_asset="${!direct_entity_base_key:-}"
    direct_entity_normal_texture_asset="${!direct_entity_normal_key:-}"
    direct_entity_metallic_roughness_texture_asset="${!direct_entity_mr_key:-}"
    direct_entity_emissive_texture_asset="${!direct_entity_emissive_key:-}"

    run ./scripts/capture-viewer-frame.sh \
      --scenario "$capture_scenario" \
      --addr "127.0.0.1:$port" \
      --viewer-wait "$viewer_wait" \
      --auto-focus-target "$capture_auto_focus_target" \
      --automation-steps "$hero_steps_effective" \
      --keep-tmp \
      ${no_prewarm_arg:+$no_prewarm_arg}

    capture_status_file=".tmp/screens/capture_status.txt"
    if [[ ! -s "$capture_status_file" ]]; then
      echo "missing capture status file: $capture_status_file (entity=$entity variant=$variant)" >&2
      exit 1
    fi
    capture_connection_status=$(capture_status_value "$capture_status_file" "connection_status")
    capture_snapshot_ready=$(capture_status_value "$capture_status_file" "snapshot_ready")
    capture_last_error=$(capture_status_value "$capture_status_file" "last_error")
    if [[ "$capture_connection_status" != "connected" || "$capture_snapshot_ready" != "1" ]]; then
      echo "texture inspector capture connectivity gate failed: entity=$entity variant=$variant connection_status=${capture_connection_status:-unknown} snapshot_ready=${capture_snapshot_ready:-unknown}" >&2
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
    effective_crop_window=$(resolve_effective_crop_window "$crop_window" "$entity" "$art_capture" "$art_panel_hidden")
    if [[ "$crop_window" == "auto" && "$effective_preview_mode" == "direct_entity" ]]; then
      effective_crop_window="none"
    fi
    viewer_art_capture_status=$(crop_or_copy_image "$variant_dir/viewer.png" "$variant_dir/viewer_art.png" "$effective_crop_window")
    if [[ "$viewer_art_capture_status" == "crop_failed_fallback" ]]; then
      echo "warn: crop failed, fallback to viewer.png (entity=$entity variant=$variant crop_window=$effective_crop_window)" >&2
    fi

    capture_connection_status_closeup="$capture_connection_status"
    capture_snapshot_ready_closeup="$capture_snapshot_ready"
    viewer_art_closeup_capture_status="passthrough"
    viewer_art_closeup_ssim_capture_status="passthrough"
    ssim_metric_crop_window="none"
    selection_kind_closeup=$(capture_status_value "$capture_status_file" "selection_kind")
    selection_id_closeup=$(capture_status_value "$capture_status_file" "selection_id")
    camera_mode_closeup=$(capture_status_value "$capture_status_file" "camera_mode")
    orbit_radius_closeup=$(capture_status_value "$capture_status_file" "orbit_radius")
    scene_power_plant_count_closeup=$(capture_status_value "$capture_status_file" "scene_power_plant_count")
    selection_gate_expected_kind=$(expected_selection_kind_for_entity "$entity" "$effective_preview_mode")
    selection_gate_mode_effective="$semantic_gate_mode"
    selection_gate_enforced=0
    selection_gate_pass=1
    selection_gate_reason="skipped"
    closeup_edge_energy="0"

    if [[ "$art_capture" -eq 1 ]]; then
      run ./scripts/capture-viewer-frame.sh \
        --scenario "$capture_scenario" \
        --addr "127.0.0.1:$port" \
        --viewer-wait "$viewer_wait" \
        --auto-focus-target "$capture_auto_focus_target" \
        --automation-steps "$closeup_steps_effective" \
        --keep-tmp \
        --no-prewarm

      capture_status_file=".tmp/screens/capture_status.txt"
      if [[ ! -s "$capture_status_file" ]]; then
        echo "missing capture status file: $capture_status_file (entity=$entity variant=$variant closeup=1)" >&2
        exit 1
      fi
      capture_connection_status_closeup=$(capture_status_value "$capture_status_file" "connection_status")
      capture_snapshot_ready_closeup=$(capture_status_value "$capture_status_file" "snapshot_ready")
      capture_last_error_closeup=$(capture_status_value "$capture_status_file" "last_error")
      selection_kind_closeup=$(capture_status_value "$capture_status_file" "selection_kind")
      selection_id_closeup=$(capture_status_value "$capture_status_file" "selection_id")
      camera_mode_closeup=$(capture_status_value "$capture_status_file" "camera_mode")
      orbit_radius_closeup=$(capture_status_value "$capture_status_file" "orbit_radius")
      scene_power_plant_count_closeup=$(capture_status_value "$capture_status_file" "scene_power_plant_count")
      if [[ "$capture_connection_status_closeup" != "connected" || "$capture_snapshot_ready_closeup" != "1" ]]; then
        echo "texture inspector closeup capture connectivity gate failed: entity=$entity variant=$variant connection_status=${capture_connection_status_closeup:-unknown} snapshot_ready=${capture_snapshot_ready_closeup:-unknown}" >&2
        if [[ -n "$capture_last_error_closeup" ]]; then
          echo "last_error=$capture_last_error_closeup" >&2
        fi
        cat "$capture_status_file" >&2 || true
        exit 1
      fi

      cp .tmp/screens/window.png "$variant_dir/viewer_closeup.png"
      cp .tmp/screens/live_server.log "$variant_dir/live_server_closeup.log"
      cp .tmp/screens/viewer.log "$variant_dir/viewer_closeup.log"
      cp "$capture_status_file" "$variant_dir/capture_status_closeup.txt"
      viewer_art_closeup_capture_status=$(crop_or_copy_image "$variant_dir/viewer_closeup.png" "$variant_dir/viewer_art_closeup.png" "$effective_crop_window")
      if [[ "$viewer_art_closeup_capture_status" == "crop_failed_fallback" ]]; then
        echo "warn: closeup crop failed, fallback to viewer_closeup.png (entity=$entity variant=$variant crop_window=$effective_crop_window)" >&2
      fi
      closeup_edge_energy=$(image_edge_energy "$variant_dir/viewer_art_closeup.png")
    else
      cp "$variant_dir/viewer.png" "$variant_dir/viewer_closeup.png"
      cp "$variant_dir/live_server.log" "$variant_dir/live_server_closeup.log"
      cp "$variant_dir/viewer.log" "$variant_dir/viewer_closeup.log"
      cp "$variant_dir/capture_status.txt" "$variant_dir/capture_status_closeup.txt"
      cp "$variant_dir/viewer_art.png" "$variant_dir/viewer_art_closeup.png"
      closeup_edge_energy=$(image_edge_energy "$variant_dir/viewer_art_closeup.png")
    fi

    if [[ "$effective_preview_mode" == "direct_entity" && "$entity" == "power_plant" ]]; then
      ssim_metric_crop_window="760:760:220:20"
    fi
    viewer_art_closeup_ssim_capture_status=$(crop_or_copy_image \
      "$variant_dir/viewer_art_closeup.png" \
      "$variant_dir/viewer_art_closeup_ssim.png" \
      "$ssim_metric_crop_window")
    if [[ "$viewer_art_closeup_ssim_capture_status" == "crop_failed_fallback" ]]; then
      echo "warn: ssim metric crop failed, fallback to viewer_art_closeup.png (entity=$entity variant=$variant crop_window=$ssim_metric_crop_window)" >&2
    fi

    if semantic_gate_enforced_for_entity "$entity" "$semantic_gate_mode" "$art_capture" "$effective_preview_mode"; then
      selection_gate_enforced=1
      if [[ -z "$selection_gate_expected_kind" ]]; then
        selection_gate_pass=0
        selection_gate_reason="expected_kind_missing"
      elif [[ "$selection_kind_closeup" != "$selection_gate_expected_kind" ]]; then
        selection_gate_pass=0
        selection_gate_reason="selection_kind_mismatch:${selection_kind_closeup:-none}"
      elif [[ "$camera_mode_closeup" != "3d" ]]; then
        selection_gate_pass=0
        selection_gate_reason="camera_mode_not_3d:${camera_mode_closeup:-unknown}"
      elif [[ "$effective_preview_mode" == "direct_entity" && -n "$orbit_radius_closeup" ]] && float_ge "$orbit_radius_closeup" "40"; then
        selection_gate_pass=0
        selection_gate_reason="orbit_radius_too_large:${orbit_radius_closeup}"
      else
        selection_gate_pass=1
        selection_gate_reason="matched"
      fi
    fi

    cat >"$variant_dir/meta.txt" <<META
preset_file=$preset_file
ui_profile_file=$ui_profile_file
scenario=$capture_scenario
entity=$entity
variant=$variant
port=$port
render_profile=$render_profile
fragment_strategy=$fragment_strategy
art_capture=$art_capture
preview_mode=$preview_mode
preview_mode_effective=$effective_preview_mode
preview_mode_fallback_reason=$preview_mode_fallback_reason
composition_profile=$composition_profile
material_profile=$material_profile
material_variant_preset_for_run=$material_variant_preset_for_run
hero_automation_steps=$hero_steps_effective
closeup_automation_steps=$closeup_steps_effective
closeup_pose_label=$closeup_pose_label
closeup_candidate_index=$closeup_candidate_index
closeup_candidate_total=$closeup_candidate_total
capture_auto_focus_target=$capture_auto_focus_target
crop_window_requested=$crop_window
crop_window_effective=$effective_crop_window
viewer_art_capture_status=$viewer_art_capture_status
viewer_art_closeup_capture_status=$viewer_art_closeup_capture_status
viewer_art_closeup_ssim_capture_status=$viewer_art_closeup_ssim_capture_status
ssim_metric_crop_window=$ssim_metric_crop_window
retry_attempt=$retry_attempt
art_lighting_enabled=$art_lighting_enabled
lighting_profile=$lighting_profile
panel_hidden=$art_panel_hidden
selection_highlight_mode=$art_selection_highlight_mode
selection_highlight_enabled=$art_selection_highlight_enabled
variant_ssim_threshold=$variant_ssim_threshold
detail_edge_threshold=$detail_edge_threshold
semantic_gate_mode=$semantic_gate_mode
use_source_mesh=$use_source_mesh
resource_pack_file=$resource_pack_file
resource_pack_override_hits=$resource_pack_override_hits
resource_pack_mesh_override=$pack_mesh_override
resource_pack_base_texture_override=$pack_base_texture_override
resource_pack_normal_texture_override=$pack_normal_texture_override
resource_pack_mr_texture_override=$pack_mr_texture_override
resource_pack_emissive_texture_override=$pack_emissive_texture_override
resource_pack_roughness_override=$pack_roughness_override
resource_pack_metallic_override=$pack_metallic_override
resource_pack_emissive_boost_override=$pack_emissive_boost_override
resource_pack_base_color_override=$pack_base_color_override
resource_pack_emissive_color_override=$pack_emissive_color_override
lighting_tonemapping=$(viewer_env_value "TONEMAPPING")
lighting_bloom_enabled=$(viewer_env_value "BLOOM_ENABLED")
lighting_bloom_intensity=$(viewer_env_value "BLOOM_INTENSITY")
lighting_color_grading_exposure=$(viewer_env_value "COLOR_GRADING_EXPOSURE")
lighting_ambient_brightness=$(viewer_env_value "AMBIENT_BRIGHTNESS")
lighting_fill_light_ratio=$(viewer_env_value "FILL_LIGHT_RATIO")
lighting_rim_light_ratio=$(viewer_env_value "RIM_LIGHT_RATIO")
lighting_exposure_ev100=$(viewer_env_value "EXPOSURE_EV100")
location_interference_disabled=$location_interference_disabled
lookdev_location_shell_enabled=$(viewer_env_value "LOCATION_SHELL_ENABLED")
lookdev_location_radiation_glow=$(viewer_env_value "LOCATION_RADIATION_GLOW")
lookdev_location_damage_visual=$(viewer_env_value "LOCATION_DAMAGE_VISUAL")
material_agent_roughness_override=$(viewer_env_value "MATERIAL_AGENT_ROUGHNESS")
material_agent_metallic_override=$(viewer_env_value "MATERIAL_AGENT_METALLIC")
material_asset_roughness_override=$(viewer_env_value "MATERIAL_ASSET_ROUGHNESS")
material_asset_metallic_override=$(viewer_env_value "MATERIAL_ASSET_METALLIC")
material_facility_roughness_override=$(viewer_env_value "MATERIAL_FACILITY_ROUGHNESS")
material_facility_metallic_override=$(viewer_env_value "MATERIAL_FACILITY_METALLIC")
material_power_plant_roughness_override=$(viewer_env_value "MATERIAL_POWER_PLANT_ROUGHNESS")
material_power_plant_metallic_override=$(viewer_env_value "MATERIAL_POWER_PLANT_METALLIC")
material_power_plant_emissive_boost_override=$(viewer_env_value "MATERIAL_POWER_PLANT_EMISSIVE_BOOST")
material_power_plant_base_color_override=$(viewer_env_value "POWER_PLANT_BASE_COLOR")
material_power_plant_emissive_color_override=$(viewer_env_value "POWER_PLANT_EMISSIVE_COLOR")
base_texture_template_override=$override_base_texture_template
normal_texture_template_override=$override_normal_texture_template
mr_texture_template_override=$override_mr_texture_template
emissive_texture_template_override=$override_emissive_texture_template
base_texture_effective_override=$variant_base_texture_override
normal_texture_effective_override=$variant_normal_texture_override
mr_texture_effective_override=$variant_mr_texture_override
emissive_texture_effective_override=$variant_emissive_texture_override
effective_mesh_asset=$effective_mesh_asset
effective_base_texture_asset=$effective_base_texture_asset
effective_normal_texture_asset=$effective_normal_texture_asset
effective_mr_texture_asset=$effective_mr_texture_asset
effective_emissive_texture_asset=$effective_emissive_texture_asset
location_mesh_asset=$(viewer_env_value "LOCATION_MESH_ASSET")
location_base_texture_asset=$(viewer_env_value "LOCATION_BASE_TEXTURE_ASSET")
location_normal_texture_asset=$(viewer_env_value "LOCATION_NORMAL_TEXTURE_ASSET")
location_metallic_roughness_texture_asset=$(viewer_env_value "LOCATION_METALLIC_ROUGHNESS_TEXTURE_ASSET")
location_emissive_texture_asset=$(viewer_env_value "LOCATION_EMISSIVE_TEXTURE_ASSET")
direct_entity_mesh_asset=$direct_entity_mesh_asset
direct_entity_base_texture_asset=$direct_entity_base_texture_asset
direct_entity_normal_texture_asset=$direct_entity_normal_texture_asset
direct_entity_metallic_roughness_texture_asset=$direct_entity_metallic_roughness_texture_asset
direct_entity_emissive_texture_asset=$direct_entity_emissive_texture_asset
selection_gate_mode_effective=$selection_gate_mode_effective
selection_gate_enforced=$selection_gate_enforced
selection_gate_expected_kind=$selection_gate_expected_kind
selection_gate_selection_kind_closeup=${selection_kind_closeup:-}
selection_gate_selection_id_closeup=${selection_id_closeup:-}
selection_gate_camera_mode_closeup=${camera_mode_closeup:-}
selection_gate_orbit_radius_closeup=${orbit_radius_closeup:-}
selection_gate_reason=$selection_gate_reason
selection_gate_pass=$selection_gate_pass
closeup_edge_energy=$closeup_edge_energy
scene_power_plant_count_closeup=${scene_power_plant_count_closeup:-}
capture_connection_status=$capture_connection_status
capture_snapshot_ready=$capture_snapshot_ready
capture_connection_status_closeup=$capture_connection_status_closeup
capture_snapshot_ready_closeup=$capture_snapshot_ready_closeup
META
  )
}

for entity in "${entities[@]}"; do
  entity_default_automation_steps=$(default_automation_steps_for_entity "$entity")
  entity_default_closeup_steps=$(default_closeup_automation_steps_for_entity "$entity")

  for variant in "${variants[@]}"; do
    port=$((base_port + capture_index))
    capture_index=$((capture_index + 1))
    variant_dir="$out_dir/$entity/$variant"

    if [[ -n "$automation_steps_override" ]]; then
      hero_steps="$automation_steps_override"
    elif [[ "$art_capture" -eq 1 ]]; then
      hero_steps="$entity_default_automation_steps"
    else
      hero_steps="$default_automation_steps"
    fi

    if [[ -n "$closeup_automation_steps_override" ]]; then
      closeup_steps="$closeup_automation_steps_override"
    elif [[ "$art_capture" -eq 1 ]]; then
      closeup_steps="$entity_default_closeup_steps"
    else
      closeup_steps="$hero_steps"
    fi

    no_prewarm_arg=""
    if [[ "$force_no_prewarm" -eq 1 || "$capture_index" -gt 1 ]]; then
      no_prewarm_arg="--no-prewarm"
    fi

    capture_variant_bundle "$entity" "$variant" "$variant_dir" "$port" "$hero_steps" "$closeup_steps" "$no_prewarm_arg" "0" "0" "1" "initial"
  done

  if [[ "$art_capture" -eq 1 && "$entity" == "power_plant" ]]; then
    if captures_are_all_present "$out_dir" "$entity"; then
      unique_count=$(variant_hash_unique_count "$out_dir" "$entity")
      min_ssim=$(variant_min_pair_ssim "$out_dir" "$entity")
      min_edge_energy=$(variant_min_edge_energy "$out_dir" "$entity")
      semantic_fail_count=$(variant_semantic_fail_count "$out_dir" "$entity")
      unique_count_retry="$unique_count"
      min_ssim_retry="$min_ssim"
      min_edge_energy_retry="$min_edge_energy"
      semantic_fail_count_retry="$semantic_fail_count"
      initial_high_ssim=0
      retry_high_ssim=0
      initial_low_edge=0
      retry_low_edge=0
      validation_retry_reason="none"
      validation_status="passed"
      validation_retry_candidates_attempted=0
      validation_retry_candidate_used="none"
      if float_ge "$min_ssim" "$variant_ssim_threshold"; then
        initial_high_ssim=1
      fi
      if float_lt "$min_edge_energy" "$detail_edge_threshold"; then
        initial_low_edge=1
      fi
      if [[ "$unique_count" -eq 1 || "$initial_high_ssim" -eq 1 || "$initial_low_edge" -eq 1 || "$semantic_fail_count" -gt 0 ]]; then
        validation_capture_scenario=$(resolve_capture_scenario_for_entity "$entity" "$scenario" "direct_entity")
        if [[ "$unique_count" -eq 1 && "$initial_high_ssim" -eq 1 ]]; then
          validation_retry_reason="identical_hash_and_high_ssim"
        elif [[ "$unique_count" -eq 1 ]]; then
          validation_retry_reason="identical_hash"
        elif [[ "$semantic_fail_count" -gt 0 ]]; then
          validation_retry_reason="selection_gate_failed"
        elif [[ "$initial_low_edge" -eq 1 ]]; then
          validation_retry_reason="low_edge_energy"
        else
          validation_retry_reason="high_ssim"
        fi
        validation_status="retrying"
        mapfile -t retry_candidates < <(retry_closeup_candidate_specs_for_entity "$entity" "$validation_capture_scenario")
        retry_candidates_total=${#retry_candidates[@]}
        if [[ "$retry_candidates_total" -eq 0 ]]; then
          retry_candidates=("fallback|$(fallback_closeup_automation_steps_for_entity "$entity" "$validation_capture_scenario")")
          retry_candidates_total=1
        fi
        echo "warn: material variant validation triggered entity=$entity reason=$validation_retry_reason unique_count=$unique_count min_ssim=$min_ssim min_edge=$min_edge_energy semantic_fail_count=$semantic_fail_count threshold=$variant_ssim_threshold edge_threshold=$detail_edge_threshold; retry with closeup candidates=$retry_candidates_total" >&2
        for retry_candidate in "${retry_candidates[@]}"; do
          validation_retry_candidates_attempted=$((validation_retry_candidates_attempted + 1))
          retry_candidate_label=${retry_candidate%%|*}
          retry_candidate_steps=${retry_candidate#*|}
          if [[ -z "$retry_candidate_steps" ]]; then
            continue
          fi

          for retry_variant in default matte glossy; do
            retry_dir="$out_dir/$entity/$retry_variant"
            if [[ ! -d "$retry_dir" ]]; then
              continue
            fi
            retry_port=$((base_port + capture_index))
            capture_index=$((capture_index + 1))
            capture_variant_bundle "$entity" "$retry_variant" "$retry_dir" "$retry_port" "$entity_default_automation_steps" "$retry_candidate_steps" "--no-prewarm" "1" "$validation_retry_candidates_attempted" "$retry_candidates_total" "$retry_candidate_label"
          done

          unique_count_retry=$(variant_hash_unique_count "$out_dir" "$entity")
          min_ssim_retry=$(variant_min_pair_ssim "$out_dir" "$entity")
          min_edge_energy_retry=$(variant_min_edge_energy "$out_dir" "$entity")
          semantic_fail_count_retry=$(variant_semantic_fail_count "$out_dir" "$entity")
          retry_high_ssim=0
          retry_low_edge=0
          if float_ge "$min_ssim_retry" "$variant_ssim_threshold"; then
            retry_high_ssim=1
          fi
          if float_lt "$min_edge_energy_retry" "$detail_edge_threshold"; then
            retry_low_edge=1
          fi

          if [[ "$unique_count_retry" -eq 1 || "$retry_high_ssim" -eq 1 || "$retry_low_edge" -eq 1 || "$semantic_fail_count_retry" -gt 0 ]]; then
            validation_status="retrying"
            echo "warn: material variant candidate failed entity=$entity candidate=$retry_candidate_label unique_count=$unique_count_retry min_ssim=$min_ssim_retry min_edge=$min_edge_energy_retry semantic_fail_count=$semantic_fail_count_retry threshold=$variant_ssim_threshold edge_threshold=$detail_edge_threshold" >&2
          else
            validation_status="passed_after_retry"
            validation_retry_candidate_used="$retry_candidate_label"
            break
          fi
        done

        if [[ "$validation_status" != "passed_after_retry" ]]; then
          validation_status="failed_after_retry"
          validation_retry_candidate_used="exhausted"
          echo "warn: material variant validation still failed after retry entity=$entity unique_count=$unique_count_retry min_ssim=$min_ssim_retry min_edge=$min_edge_energy_retry semantic_fail_count=$semantic_fail_count_retry threshold=$variant_ssim_threshold edge_threshold=$detail_edge_threshold" >&2
        fi
      fi

      cat >"$out_dir/$entity/variant_validation.txt" <<VALIDATION
entity=$entity
status=$validation_status
retry_reason=$validation_retry_reason
unique_count_initial=$unique_count
unique_count_after_retry=$unique_count_retry
ssim_threshold=$variant_ssim_threshold
min_pair_ssim_initial=$min_ssim
min_pair_ssim_after_retry=$min_ssim_retry
detail_edge_threshold=$detail_edge_threshold
min_edge_energy_initial=$min_edge_energy
min_edge_energy_after_retry=$min_edge_energy_retry
semantic_fail_count_initial=$semantic_fail_count
semantic_fail_count_after_retry=$semantic_fail_count_retry
retry_candidates_attempted=$validation_retry_candidates_attempted
retry_candidate_used=$validation_retry_candidate_used
composition_profile=$composition_profile
VALIDATION

      for retry_variant in default matte glossy; do
        retry_meta="$out_dir/$entity/$retry_variant/meta.txt"
        if [[ -f "$retry_meta" ]]; then
          echo "variant_validation=$validation_status" >>"$retry_meta"
          echo "variant_validation_retry_reason=$validation_retry_reason" >>"$retry_meta"
          echo "variant_validation_unique_count_initial=$unique_count" >>"$retry_meta"
          echo "variant_validation_unique_count_after_retry=$unique_count_retry" >>"$retry_meta"
          echo "variant_validation_ssim_threshold=$variant_ssim_threshold" >>"$retry_meta"
          echo "variant_validation_min_pair_ssim_initial=$min_ssim" >>"$retry_meta"
          echo "variant_validation_min_pair_ssim_after_retry=$min_ssim_retry" >>"$retry_meta"
          echo "variant_validation_detail_edge_threshold=$detail_edge_threshold" >>"$retry_meta"
          echo "variant_validation_min_edge_energy_initial=$min_edge_energy" >>"$retry_meta"
          echo "variant_validation_min_edge_energy_after_retry=$min_edge_energy_retry" >>"$retry_meta"
          echo "variant_validation_semantic_fail_count_initial=$semantic_fail_count" >>"$retry_meta"
          echo "variant_validation_semantic_fail_count_after_retry=$semantic_fail_count_retry" >>"$retry_meta"
          echo "variant_validation_retry_candidates_attempted=$validation_retry_candidates_attempted" >>"$retry_meta"
          echo "variant_validation_retry_candidate_used=$validation_retry_candidate_used" >>"$retry_meta"
          echo "variant_validation_composition_profile=$composition_profile" >>"$retry_meta"
        fi
      done
    fi
  fi
done

echo "texture inspector artifacts: $out_dir"
