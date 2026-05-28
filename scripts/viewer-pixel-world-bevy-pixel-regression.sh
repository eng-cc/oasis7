#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

run_id=$(date -u +"%Y-%m-%dT%H-%M-%SZ")
out_dir="${PIXEL_WORLD_BEVY_PIXEL_PROBE_OUT_DIR:-$repo_root/output/pixel-world-bevy-pixel-regression/$run_id}"

mkdir -p "$out_dir"

PIXEL_WORLD_BEVY_PIXEL_PROBE_OUT_DIR="$out_dir" \
  env -u RUSTC_WRAPPER cargo test -p pixel_world_bridge \
    bevy_pixel_regression_rasterizes_fragment_location_agent_hierarchy \
    --lib -- --nocapture

test -s "$out_dir/pixel-summary.json"
test -s "$out_dir/pixel-regression.png"
test -s "$out_dir/pixel-regression-crop.png"
printf 'pixel-world Bevy pixel regression passed: %s\n' "$out_dir/pixel-summary.json"
printf 'pixel-world Bevy pixel image: %s\n' "$out_dir/pixel-regression.png"
printf 'pixel-world Bevy pixel crop: %s\n' "$out_dir/pixel-regression-crop.png"
