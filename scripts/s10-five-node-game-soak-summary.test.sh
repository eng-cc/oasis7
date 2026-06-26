#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

find_bash4() {
  local candidate
  for candidate in "${BASH4_BIN:-}" /opt/homebrew/bin/bash /usr/local/bin/bash bash; do
    [[ -n "$candidate" ]] || continue
    if command -v "$candidate" >/dev/null 2>&1 && "$candidate" -c '(( BASH_VERSINFO[0] >= 4 ))' >/dev/null 2>&1; then
      command -v "$candidate"
      return 0
    fi
  done
  return 1
}

bash4=$(find_bash4) || {
  echo "skip: Bash 4+ not available for s10-five-node-game-soak dry-run smoke"
  exit 0
}

tmp_root=$(mktemp -d "${TMPDIR:-/tmp}/oasis7-s10-summary-test.XXXXXX")
trap 'rm -rf "$tmp_root"' EXIT

"$bash4" ./scripts/s10-five-node-game-soak.sh \
  --dry-run \
  --no-prewarm \
  --duration-secs 1 \
  --out-dir "$tmp_root" >/dev/null

summary_json=$(find "$tmp_root" -type f -name summary.json | sort | tail -n 1)
[[ -n "$summary_json" && -f "$summary_json" ]] || { echo "missing summary.json" >&2; exit 1; }

jq -e '
  .overall_status == "dry_run"
  and .api_viewer_projection.status == "not_collected"
  and .api_viewer_projection.same_window_required == true
  and .api_viewer_projection.chain_status_endpoint == "/v1/chain/status"
  and (.api_viewer_projection.chain_status_samples_ref | endswith("timeline.csv"))
  and .api_viewer_projection.api_projection_ref == null
  and .api_viewer_projection.viewer_projection_ref == null
  and .api_viewer_projection.world_state_projection_match == null
  and (.api_viewer_projection.does_not_claim | index("API/viewer projection verified")) != null
  and (.api_viewer_projection.does_not_claim | index("release_full")) != null
  and (.api_viewer_projection.does_not_claim | index("public_testnet ready")) != null
' "$summary_json" >/dev/null

summary_md=$(jq -r '.artifacts.summary_md' "$summary_json")
grep -Fq 'API / Viewer Projection Contract' "$summary_md"
grep -Fq 'world_state_projection_match: `null`' "$summary_md"

echo "s10 five-node summary projection contract smoke checks passed"
