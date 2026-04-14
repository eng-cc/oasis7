#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PLATFORM=""
BUNDLE_DIR=""

usage() {
  cat <<'USAGE'
Usage: ./scripts/validate-release-platform-entrypoints.sh [options]

Validate that a prepared release bundle exposes platform-native launch entrypoints.

Options:
  --platform <id>      required: linux-x64 | macos-x64 | windows-x64
  --bundle-dir <path>  required: prepared bundle directory
  -h, --help           show this help
USAGE
}

resolve_abs_path() {
  local input="$1"
  if [[ "$input" == /* ]]; then
    printf '%s\n' "$input"
  else
    printf '%s\n' "$ROOT_DIR/$input"
  fi
}

require_path() {
  local path="$1"
  if [[ ! -e "$path" ]]; then
    echo "error: required release bundle entrypoint missing: $path" >&2
    exit 1
  fi
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --platform)
      PLATFORM="${2:-}"
      shift 2
      ;;
    --bundle-dir)
      BUNDLE_DIR="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown option: $1" >&2
      usage
      exit 1
      ;;
  esac
done

case "$PLATFORM" in
  linux-x64|macos-x64|windows-x64) ;;
  *)
    echo "error: --platform must be one of linux-x64|macos-x64|windows-x64" >&2
    exit 1
    ;;
esac

[[ -n "$BUNDLE_DIR" ]] || { echo "error: --bundle-dir is required" >&2; exit 1; }
BUNDLE_DIR="$(resolve_abs_path "$BUNDLE_DIR")"
[[ -d "$BUNDLE_DIR" ]] || { echo "error: bundle dir does not exist: $BUNDLE_DIR" >&2; exit 1; }

require_path "$BUNDLE_DIR/bin"
require_path "$BUNDLE_DIR/web/index.html"
require_path "$BUNDLE_DIR/web-launcher/index.html"
require_path "$BUNDLE_DIR/README.txt"
require_path "$BUNDLE_DIR/.oasis7-bundle-manifest.json"
require_path "$BUNDLE_DIR/run-client.sh"
require_path "$BUNDLE_DIR/run-web-launcher.sh"
require_path "$BUNDLE_DIR/run-game.sh"
require_path "$BUNDLE_DIR/run-chain-runtime.sh"

case "$PLATFORM" in
  linux-x64)
    require_path "$BUNDLE_DIR/bin/oasis7_client_launcher"
    require_path "$BUNDLE_DIR/bin/oasis7_game_launcher"
    require_path "$BUNDLE_DIR/bin/oasis7_web_launcher"
    require_path "$BUNDLE_DIR/bin/oasis7_viewer_live"
    require_path "$BUNDLE_DIR/bin/oasis7_chain_runtime"
    ;;
  macos-x64)
    require_path "$BUNDLE_DIR/bin/oasis7_client_launcher"
    require_path "$BUNDLE_DIR/bin/oasis7_game_launcher"
    require_path "$BUNDLE_DIR/bin/oasis7_web_launcher"
    require_path "$BUNDLE_DIR/bin/oasis7_viewer_live"
    require_path "$BUNDLE_DIR/bin/oasis7_chain_runtime"
    require_path "$BUNDLE_DIR/oasis7 Client Launcher.app/Contents/Info.plist"
    require_path "$BUNDLE_DIR/oasis7 Client Launcher.app/Contents/MacOS/oasis7-client-launcher"
    ;;
  windows-x64)
    require_path "$BUNDLE_DIR/bin/oasis7_client_launcher.exe"
    require_path "$BUNDLE_DIR/bin/oasis7_game_launcher.exe"
    require_path "$BUNDLE_DIR/bin/oasis7_web_launcher.exe"
    require_path "$BUNDLE_DIR/bin/oasis7_viewer_live.exe"
    require_path "$BUNDLE_DIR/bin/oasis7_chain_runtime.exe"
    require_path "$BUNDLE_DIR/run-client.cmd"
    require_path "$BUNDLE_DIR/run-web-launcher.cmd"
    require_path "$BUNDLE_DIR/run-game.cmd"
    require_path "$BUNDLE_DIR/run-chain-runtime.cmd"
    ;;
esac

echo "Release bundle entrypoints validated: $BUNDLE_DIR ($PLATFORM)"
