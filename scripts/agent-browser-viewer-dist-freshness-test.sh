#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/agent-browser-lib.sh"

tmp_repo="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_repo"
}
trap cleanup EXIT

mkdir -p \
  "$tmp_repo/bin" \
  "$tmp_repo/crates/oasis7_viewer/dist" \
  "$tmp_repo/crates/oasis7_proto/src"

printf '<!doctype html>old dist\n' > "$tmp_repo/crates/oasis7_viewer/dist/index.html"
printf 'console.log("safe mode changed");\n' > "$tmp_repo/crates/oasis7_viewer/software_safe.js"
printf '<!doctype html>software safe\n' > "$tmp_repo/crates/oasis7_viewer/software_safe.html"
printf '<!doctype html>claim evidence\n' > "$tmp_repo/crates/oasis7_viewer/software_safe_first_agent_claim_evidence.html"
printf '{"name":"oasis7-viewer-software-safe-ui","scripts":{"build:software-safe":"echo ok"}}\n' > "$tmp_repo/crates/oasis7_viewer/package.json"
printf '{"lockfileVersion":3}\n' > "$tmp_repo/crates/oasis7_viewer/package-lock.json"
printf 'export default {};\n' > "$tmp_repo/crates/oasis7_viewer/vite.software-safe.config.mjs"
mkdir -p \
  "$tmp_repo/crates/oasis7_viewer/scripts" \
  "$tmp_repo/crates/oasis7_viewer/software_safe_src" \
  "$tmp_repo/crates/oasis7_viewer/pixel-world-bridge" \
  "$tmp_repo/crates/pixel_world_bridge/src"
printf 'console.log("finalize");\n' > "$tmp_repo/crates/oasis7_viewer/scripts/finalize-software-safe-build.mjs"
printf 'console.log("src");\n' > "$tmp_repo/crates/oasis7_viewer/software_safe_src/main.jsx"
printf 'export function createPixelWorldBridge() {}\n' > "$tmp_repo/crates/oasis7_viewer/pixel-world-bridge/pixel_world_bridge.js"
printf 'icon\n' > "$tmp_repo/crates/oasis7_viewer/favicon.ico"
printf '[package]\nname = "pixel_world_bridge"\nversion = "0.0.0"\n' > "$tmp_repo/crates/pixel_world_bridge/Cargo.toml"
printf 'pub fn ping() {}\n' > "$tmp_repo/crates/pixel_world_bridge/src/lib.rs"
printf '[package]\nname = "oasis7_proto"\nversion = "0.0.0"\n' > "$tmp_repo/crates/oasis7_proto/Cargo.toml"
printf 'pub const VIEWER_PROTOCOL_VERSION: u32 = 1;\n' > "$tmp_repo/crates/oasis7_proto/src/viewer.rs"
printf '# lock\n' > "$tmp_repo/Cargo.lock"
printf '[workspace]\nmembers = []\n' > "$tmp_repo/Cargo.toml"

touch -d '2026-03-16 00:00:00' "$tmp_repo/crates/oasis7_viewer/dist/index.html"
touch -d '2026-03-17 00:00:00' "$tmp_repo/crates/oasis7_viewer/software_safe.js"

cat > "$tmp_repo/bin/npm" <<'NPM'
#!/usr/bin/env bash
set -euo pipefail
if [[ "$1" != "--prefix" || "$3" != "run" || "$4" != "build:software-safe" ]]; then
  echo "unexpected npm args: $*" >&2
  exit 1
fi
printf 'software safe rebuild\n'
NPM
chmod +x "$tmp_repo/bin/npm"

resolved_dir="$({ PATH="$tmp_repo/bin:$PATH" resolve_viewer_static_dir_for_web_closure "$tmp_repo" web "$tmp_repo/output/check"; } 2>"$tmp_repo/stderr.log")"
expected_dir="$tmp_repo/output/check/web-dist"

if [[ "$resolved_dir" != "$expected_dir" ]]; then
  echo "expected rebuilt dir '$expected_dir', got '$resolved_dir'" >&2
  exit 1
fi

if [[ ! -f "$expected_dir/index.html" ]]; then
  echo "expected rebuilt dist index at $expected_dir/index.html" >&2
  exit 1
fi

if [[ ! -f "$expected_dir/pixel-world-bridge/pixel_world_bridge.js" ]]; then
  echo "expected rebuilt dist pixel world runtime at $expected_dir/pixel-world-bridge/pixel_world_bridge.js" >&2
  exit 1
fi

if ! grep -Fq 'npm --prefix' "$tmp_repo/stderr.log"; then
  echo "expected freshness helper to trigger software_safe rebuild" >&2
  cat "$tmp_repo/stderr.log" >&2
  exit 1
fi

echo "agent-browser viewer dist freshness tests passed"
