#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VIEWER_DIR="$ROOT_DIR/crates/oasis7_viewer"
DIST_DIR="$VIEWER_DIR/dist"
ADDRESS="127.0.0.1"
PORT="4173"

usage() {
  cat <<'USAGE'
Usage: ./scripts/run-viewer-web.sh [options]

Build and serve the viewer static site.

Options:
  --address <host>  HTTP bind address (default: 127.0.0.1)
  --port <port>     HTTP bind port (default: 4173)
  -h, --help        Show this help
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --address)
      ADDRESS="${2:-}"
      shift 2
      ;;
    --port)
      PORT="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

[[ -n "$ADDRESS" ]] || { echo "error: --address cannot be empty" >&2; exit 2; }
[[ "$PORT" =~ ^[0-9]+$ ]] || { echo "error: --port must be a positive integer" >&2; exit 2; }

if ! command -v npm >/dev/null 2>&1; then
  echo "error: npm is not installed" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "error: python3 is not installed" >&2
  exit 1
fi

(
  cd "$ROOT_DIR"
  npm --prefix crates/oasis7_viewer run build:software-safe >/dev/null
)
mkdir -p "$DIST_DIR"
cp "$VIEWER_DIR/software_safe.html" "$DIST_DIR/index.html"
cp "$VIEWER_DIR/software_safe.html" "$DIST_DIR/viewer.html"
cp "$VIEWER_DIR/software_safe.html" "$DIST_DIR/software_safe.html"
cp "$VIEWER_DIR/software_safe.js" "$DIST_DIR/viewer.js"
cp "$VIEWER_DIR/software_safe.js" "$DIST_DIR/software_safe.js"
cp "$VIEWER_DIR/software_safe_first_agent_claim_evidence.html" \
  "$DIST_DIR/software_safe_first_agent_claim_evidence.html"
cp "$VIEWER_DIR/favicon.ico" "$DIST_DIR/favicon.ico"
if [[ -d "$VIEWER_DIR/pixel-world-bridge" ]]; then
  rm -rf "$DIST_DIR/pixel-world-bridge"
  cp -R "$VIEWER_DIR/pixel-world-bridge" "$DIST_DIR/pixel-world-bridge"
fi

exec python3 -m http.server "$PORT" --bind "$ADDRESS" --directory "$DIST_DIR"
