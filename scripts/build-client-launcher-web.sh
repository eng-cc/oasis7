#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"

usage() {
  cat <<'USAGE'
Usage: ./scripts/build-client-launcher-web.sh --dist-dir <path> [--release]

Build the `oasis7_client_launcher` trunk web dist using the repo-pinned
`wasm-bindgen` CLI instead of allowing trunk to fetch it on demand.

Options:
  --dist-dir <path>  Output directory for the trunk dist.
  --release          Build in release mode.
  -h, --help         Show this help.
USAGE
}

DIST_DIR=""
RELEASE_FLAG=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dist-dir)
      DIST_DIR="${2:-}"
      shift 2
      ;;
    --release)
      RELEASE_FLAG=1
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

if [[ -z "$DIST_DIR" ]]; then
  echo "error: --dist-dir is required" >&2
  usage >&2
  exit 2
fi

launcher_wasm_bindgen_bin="$("$ROOT_DIR/scripts/ensure-wasm-bindgen-cli.sh" --print-bin)"
launcher_wasm_bindgen_dir="$(dirname "$launcher_wasm_bindgen_bin")"

mkdir -p "$DIST_DIR"

trunk_args=(build --dist "$DIST_DIR")
if [[ "$RELEASE_FLAG" == "1" ]]; then
  trunk_args=(build --release --dist "$DIST_DIR")
fi

echo "+ PATH=$launcher_wasm_bindgen_dir:\$PATH WASM_BINDGEN_BIN=$launcher_wasm_bindgen_bin trunk ${trunk_args[*]}"
(
  cd "$ROOT_DIR/crates/oasis7_client_launcher"
  PATH="$launcher_wasm_bindgen_dir:$PATH" \
  WASM_BINDGEN_BIN="$launcher_wasm_bindgen_bin" \
  env -u NO_COLOR trunk "${trunk_args[@]}"
)
