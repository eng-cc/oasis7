#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT

bin_dir="$tmp_root/bin"
mkdir -p "$bin_dir"

cat >"$bin_dir/systemctl" <<'SH'
#!/usr/bin/env bash
if [[ "${1:-}" == "is-active" ]]; then
  echo active
  exit 0
fi
exit 1
SH

cat >"$bin_dir/ssh" <<'SH'
#!/usr/bin/env bash
echo "forced ssh failure for fallback test" >&2
exit 255
SH

cat >"$bin_dir/sshpass" <<'SH'
#!/usr/bin/env bash
if [[ "${1:-}" == "-e" ]]; then
  shift
fi
exec "$@"
SH

cat >"$bin_dir/curl" <<'SH'
#!/usr/bin/env bash
if [[ " $* " != *" --max-time "* ]]; then
  echo "curl missing --max-time: $*" >&2
  exit 2
fi
url="${@: -1}"
case "$url" in
  *local/healthz|*sequencer/healthz|*storage/healthz)
    printf '{"ok":true}\n'
    ;;
  *local/status)
    cat <<'JSON'
{"node_id":"local","world_id":"fallback-test","role":"sequencer","running":true,"liveness":{"status":"ok","running":true},"readiness":{"status":"ready","ready":true,"policy":{"tier":"public_testnet","role":"sequencer","quorum_mode":"count","relay_policy":"public_direct_or_relay","slashing_policy":"disabled_for_non_mainnet","slashing_enforced":false}},"sync":{"status":"ok","network_head_source":"local","network_height_lag":0,"fresh_peer_count":1,"stale_peer_count":0,"conflicting_peer_count":0},"observability":{"status":"ok","ready":true,"network_head_available":true,"transport_stable":true,"reachability_policy_ok":true},"consensus":{"committed_height":10,"network_committed_height":10,"known_peer_heads":1,"network_head":{"source":"local","decision":"accepted","required_peer_count":1,"quorum_mode":"count","fresh_peer_count":1,"stale_peer_count":0,"conflicting_peer_count":0,"observed_stake":100,"required_stake":100,"total_stake":300,"stake_quorum_met":true}}}
JSON
    ;;
  *sequencer/status)
    cat <<'JSON'
{"node_id":"sequencer","world_id":"fallback-test","role":"sequencer","running":true,"liveness":{"status":"ok","running":true},"readiness":{"status":"ready","ready":true,"policy":{"tier":"public_testnet","role":"sequencer","quorum_mode":"count","relay_policy":"public_direct_or_relay","slashing_policy":"disabled_for_non_mainnet","slashing_enforced":false}},"sync":{"status":"ok","network_head_source":"peer","network_height_lag":0,"fresh_peer_count":1,"stale_peer_count":0,"conflicting_peer_count":0},"observability":{"status":"ok","ready":true,"network_head_available":true,"transport_stable":true,"reachability_policy_ok":true},"consensus":{"committed_height":20,"network_committed_height":20,"known_peer_heads":1,"network_head":{"source":"peer","decision":"accepted","required_peer_count":1,"quorum_mode":"count","fresh_peer_count":1,"stale_peer_count":0,"conflicting_peer_count":0,"observed_stake":100,"required_stake":100,"total_stake":300,"stake_quorum_met":true}}}
JSON
    ;;
  *storage/status)
    cat <<'JSON'
{"node_id":"storage","world_id":"fallback-test","role":"sequencer","running":true,"liveness":{"status":"ok","running":true},"readiness":{"status":"ready","ready":true,"policy":{"tier":"public_testnet","role":"sequencer","quorum_mode":"count","relay_policy":"public_direct_or_relay","slashing_policy":"disabled_for_non_mainnet","slashing_enforced":false}},"sync":{"status":"ok","network_head_source":"peer","network_height_lag":0,"fresh_peer_count":1,"stale_peer_count":0,"conflicting_peer_count":0},"observability":{"status":"ok","ready":true,"network_head_available":true,"transport_stable":true,"reachability_policy_ok":true},"consensus":{"committed_height":30,"network_committed_height":30,"known_peer_heads":1,"network_head":{"source":"peer","decision":"accepted","required_peer_count":1,"quorum_mode":"count","fresh_peer_count":1,"stale_peer_count":0,"conflicting_peer_count":0,"observed_stake":100,"required_stake":100,"total_stake":300,"stake_quorum_met":true}}}
JSON
    ;;
  *)
    echo "unexpected curl url: $url" >&2
    exit 22
    ;;
esac
SH

chmod +x "$bin_dir/systemctl" "$bin_dir/ssh" "$bin_dir/sshpass" "$bin_dir/curl"

PATH="$bin_dir:$PATH" ./scripts/p2p-real-env-triad-snapshot.sh \
  --samples 1 \
  --interval-secs 1 \
  --ssh-timeout-secs 1 \
  --out-dir "$tmp_root/out" \
  --world-id fallback-test \
  --local-status-url http://local/status \
  --local-health-url http://local/healthz \
  --local-env-file "$tmp_root/missing-local.env" \
  --sequencer-status-url http://127.0.0.1:1/status \
  --sequencer-health-url http://127.0.0.1:1/healthz \
  --sequencer-public-status-url http://sequencer/status \
  --sequencer-public-health-url http://sequencer/healthz \
  --storage-status-url http://127.0.0.1:2/status \
  --storage-health-url http://127.0.0.1:2/healthz \
  --storage-public-status-url http://storage/status \
  --storage-public-health-url http://storage/healthz

run_dir="$(find "$tmp_root/out" -mindepth 1 -maxdepth 1 -type d | sort | tail -n 1)"
summary_json="$run_dir/summary.json"

jq -e '
  .nodes.sequencer_ecs.status_fetch_all_ok == true
  and .nodes.storage_ecs.status_fetch_all_ok == true
  and .nodes.sequencer_ecs.heights.max_committed_height == 20
  and .nodes.storage_ecs.heights.max_committed_height == 30
  and .nodes.sequencer_ecs.node_ids == ["sequencer"]
  and .nodes.storage_ecs.node_ids == ["storage"]
' "$summary_json" >/dev/null

test -f "$run_dir/nodes/sequencer_ecs/samples/sample-001/status.fallback.txt"
test -f "$run_dir/nodes/storage_ecs/samples/sample-001/status.fallback.txt"
