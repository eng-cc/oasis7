#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

run_id=$(date -u +"%Y-%m-%dT%H-%M-%SZ")
out_dir="${PIXEL_WORLD_BEVY_RENDER_PROBE_OUT_DIR:-$repo_root/output/pixel-world-bevy-render-probe/$run_id}"

mkdir -p "$out_dir"

PIXEL_WORLD_BEVY_RENDER_PROBE_OUT_DIR="$out_dir" \
  env -u RUSTC_WRAPPER cargo test -p pixel_world_bridge \
    bevy_render_probe_contract_captures_visual_hierarchy \
    --lib -- --nocapture

test -s "$out_dir/summary.json"
printf 'pixel-world Bevy render probe passed: %s\n' "$out_dir/summary.json"
