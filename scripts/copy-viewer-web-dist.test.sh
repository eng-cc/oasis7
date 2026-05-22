#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
TMPDIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

viewer_root="$TMPDIR/viewer"
dist_dir="$TMPDIR/dist"
mkdir -p "$viewer_root/pixel-world-bridge"

printf '<!doctype html><script type="module" src="./viewer.js"></script>\n' > "$viewer_root/software_safe.html"
printf 'console.log("canonical bundle");\n' > "$viewer_root/viewer.js"
printf '// Generated compat alias; canonical bundle truth lives in ./viewer.js.\nimport "./viewer.js";\n' > "$viewer_root/software_safe.js"
printf '<!doctype html>claim evidence\n' > "$viewer_root/software_safe_first_agent_claim_evidence.html"
printf 'icon\n' > "$viewer_root/favicon.ico"
printf 'export const bridge = true;\n' > "$viewer_root/pixel-world-bridge/pixel_world_bridge.js"

"$ROOT_DIR/scripts/copy-viewer-web-dist.sh" --viewer-root "$viewer_root" --dist-dir "$dist_dir"

cmp "$viewer_root/viewer.js" "$dist_dir/viewer.js"
cmp "$viewer_root/software_safe.js" "$dist_dir/software_safe.js"
cmp "$viewer_root/software_safe.html" "$dist_dir/index.html"
cmp "$viewer_root/software_safe.html" "$dist_dir/viewer.html"
cmp "$viewer_root/software_safe.html" "$dist_dir/software_safe.html"
cmp "$viewer_root/pixel-world-bridge/pixel_world_bridge.js" "$dist_dir/pixel-world-bridge/pixel_world_bridge.js"

echo "copy-viewer-web-dist.test: OK"
