#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

run_id=$(date -u +"%Y-%m-%dT%H-%M-%SZ")
out_dir="${PIXEL_WORLD_BEVY_PIXEL_PROBE_OUT_DIR:-$repo_root/output/pixel-world-bevy-pixel-regression/$run_id}"

mkdir -p "$out_dir"

location_out_dir="$out_dir/selected-location"
agent_out_dir="$out_dir/selected-agent"
mkdir -p "$location_out_dir" "$agent_out_dir"

PIXEL_WORLD_BEVY_PIXEL_PROBE_OUT_DIR="$location_out_dir" \
  env -u RUSTC_WRAPPER cargo test -p pixel_world_bridge \
    bevy_pixel_regression_exports_selected_location_ring_with_world_layers \
    --lib -- --nocapture

PIXEL_WORLD_BEVY_PIXEL_PROBE_OUT_DIR="$agent_out_dir" \
  env -u RUSTC_WRAPPER cargo test -p pixel_world_bridge \
    bevy_pixel_regression_exports_selected_agent_corner_cue_with_world_layers \
    --lib -- --nocapture

for case_out_dir in "$location_out_dir" "$agent_out_dir"; do
  test -s "$case_out_dir/pixel-summary.json"
  test -s "$case_out_dir/pixel-regression.png"
  test -s "$case_out_dir/pixel-regression-crop.png"
done
rg -q '"selected_agent_cue_pixels": [1-9]' "$agent_out_dir/pixel-summary.json"
printf 'pixel-world Bevy pixel regression passed: %s\n' "$out_dir"
