#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

usage() {
  cat <<'USAGE'
Usage: ./scripts/release-gate-web-strict.sh [options]

Run the release web strict gate in two phases:
1. Verify the public/default Web entry contract (`/` and `render_mode=auto`)
2. Verify the `software_safe` realtime interaction contract

Options:
  --out-dir <path>   Artifact root (default: .tmp/release_gate_web_strict)
  --headed           Run browser checks in headed mode
  --headless         Run browser checks in headless mode (default)
  -h, --help         Show help
USAGE
}

OUT_DIR=".tmp/release_gate_web_strict"
HEADED=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out-dir)
      OUT_DIR=${2:-}
      shift 2
      ;;
    --headed)
      HEADED=1
      shift
      ;;
    --headless)
      HEADED=0
      shift
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

mkdir -p "$OUT_DIR"

browser_mode=(--headless)
if [[ "$HEADED" -eq 1 ]]; then
  browser_mode=(--headed)
fi
step_browser_mode=(--headless)
echo "+ ./scripts/viewer-primary-web-entry-regression.sh --out-dir $OUT_DIR/primary-entry ${browser_mode[*]}"
./scripts/viewer-primary-web-entry-regression.sh \
  --out-dir "$OUT_DIR/primary-entry" \
  "${browser_mode[@]}"

echo "+ ./scripts/viewer-software-safe-step-regression.sh --skip-llm-provider-preflight --out-dir $OUT_DIR/software-safe-step ${step_browser_mode[*]}"
./scripts/viewer-software-safe-step-regression.sh \
  --release-web-fixture \
  --skip-llm-provider-preflight \
  --live-bind 127.0.0.1:5033 \
  --web-bind 127.0.0.1:5021 \
  --viewer-port 4273 \
  --out-dir "$OUT_DIR/software-safe-step" \
  "${step_browser_mode[@]}"
