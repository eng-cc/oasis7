#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/bundle-freshness-lib.sh"

tmp_repo="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_repo"
}
trap cleanup EXIT

mkdir -p \
  "$tmp_repo/bundle" \
  "$tmp_repo/crates/oasis7_proto/src" \
  "$tmp_repo/crates/oasis7_viewer" \
  "$tmp_repo/scripts"

printf '[workspace]\nmembers = []\n' > "$tmp_repo/Cargo.toml"
printf '# lock\n' > "$tmp_repo/Cargo.lock"
printf 'pub const VIEWER_PROTOCOL_VERSION: u32 = 7;\n' > "$tmp_repo/crates/oasis7_proto/src/viewer.rs"
printf '<!doctype html>software safe\n' > "$tmp_repo/crates/oasis7_viewer/software_safe.html"
printf 'console.log("viewer");\n' > "$tmp_repo/crates/oasis7_viewer/viewer.js"
printf 'import "./viewer.js";\n' > "$tmp_repo/crates/oasis7_viewer/software_safe.js"
printf '#!/usr/bin/env bash\nset -euo pipefail\n' > "$tmp_repo/scripts/copy-viewer-web-dist.sh"
printf '#!/usr/bin/env bash\nset -euo pipefail\n' > "$tmp_repo/scripts/viewer-web-dist-contract.sh"

required_scope_entries=(
  "crates/oasis7_viewer/software_safe.js"
  "scripts/copy-viewer-web-dist.sh"
  "scripts/viewer-web-dist-contract.sh"
)

source_scope="$(bundle_source_metadata_json "$tmp_repo" | python3 -c 'import json,sys; print("\n".join(json.load(sys.stdin)["sourceScope"]))')"
for entry in "${required_scope_entries[@]}"; do
  if ! grep -Fxq "$entry" <<<"$source_scope"; then
    echo "expected bundle source scope to include $entry" >&2
    exit 1
  fi
done

for entry in "${required_scope_entries[@]}"; do
  rm -f "$(bundle_manifest_path "$tmp_repo/bundle")"
  bundle_write_manifest "$tmp_repo" "$tmp_repo/bundle"

  printf '\n# freshness drift for %s\n' "$entry" >> "$tmp_repo/$entry"

  if freshness_output="$(bundle_check_freshness "$tmp_repo" "$tmp_repo/bundle" 2>&1)"; then
    echo "expected bundle freshness to fail after mutating $entry" >&2
    exit 1
  fi
  if ! grep -Fq "source fingerprint drift" <<<"$freshness_output"; then
    echo "expected source fingerprint drift after mutating $entry, got: $freshness_output" >&2
    exit 1
  fi
done

echo "bundle freshness lib tests passed"
