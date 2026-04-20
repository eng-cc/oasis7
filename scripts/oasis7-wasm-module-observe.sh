#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: ./scripts/oasis7-wasm-module-observe.sh [options]

Build one wasm module from its local observe spec, then run standardized
contract checks and perf sampling.

Options:
  --spec <path>                explicit observe spec path
  --manifest-path <path>       infer spec as <module-dir>/observability/module_observe.json
  --out-dir <path>             output root passed through to the runner
  -h, --help                   show help

Default behavior:
  If no --spec or --manifest-path is provided, infer
  ./observability/module_observe.json from the current directory.
USAGE
}

spec_path=""
manifest_path=""
out_dir=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --spec)
      spec_path=${2:-}
      shift 2
      ;;
    --manifest-path)
      manifest_path=${2:-}
      shift 2
      ;;
    --out-dir)
      out_dir=${2:-}
      shift 2
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

if [[ -n "$spec_path" && -n "$manifest_path" ]]; then
  echo "use either --spec or --manifest-path, not both" >&2
  exit 2
fi

if [[ -n "$manifest_path" ]]; then
  module_dir="$(cd "$(dirname "$manifest_path")" && pwd)"
  spec_path="$module_dir/observability/module_observe.json"
elif [[ -z "$spec_path" ]]; then
  spec_path="$(pwd)/observability/module_observe.json"
fi

if [[ ! -f "$spec_path" ]]; then
  echo "observe spec not found: $spec_path" >&2
  exit 2
fi

cmd=(
  env -u RUSTC_WRAPPER
  cargo run --manifest-path tools/wasm_module_observe/Cargo.toml -- observe
  --spec "$spec_path"
)

if [[ -n "$out_dir" ]]; then
  cmd+=(--out-dir "$out_dir")
fi

"${cmd[@]}"
