#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-bundle-ops-default.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

mkdir -p "$TMP_DIR/web" "$TMP_DIR/web-launcher"
printf '<!doctype html><title>fixture</title>\n' >"$TMP_DIR/web/index.html"
printf '<!doctype html><title>fixture</title>\n' >"$TMP_DIR/web-launcher/index.html"

run_dry_bundle() {
  local output=$1
  shift
  "$ROOT_DIR/scripts/build-game-launcher-bundle.sh" \
    --dry-run \
    --out-dir "$output" \
    --web-dist "$TMP_DIR/web" \
    --web-launcher-dist "$TMP_DIR/web-launcher" \
    "$@"
}

player_output="$TMP_DIR/player-output"
player_log="$TMP_DIR/player.log"
run_dry_bundle "$player_output" >"$player_log"
for tool in oasis7_world_repair_rebuild oasis7_governance_registry_import oasis7_governance_registry_audit; do
  if grep -Fq "$player_output/bin/$tool" "$player_log"; then
    echo "default player bundle dry-run routed operator tool into player output: $tool" >&2
    exit 1
  fi
done

ops_output="$TMP_DIR/ops-output"
ops_log="$TMP_DIR/ops.log"
run_dry_bundle "$player_output-explicit-ops" --ops-out-dir "$ops_output" >"$ops_log"
for tool in oasis7_world_repair_rebuild oasis7_governance_registry_import oasis7_governance_registry_audit; do
  grep -Fq "$ops_output/bin/$tool" "$ops_log" || {
    echo "explicit ops output did not receive $tool" >&2
    exit 1
  }
done

echo "build-game-launcher-bundle-ops-default.test: OK"
