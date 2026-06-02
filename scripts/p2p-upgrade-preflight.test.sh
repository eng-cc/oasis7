#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRIPT="$ROOT_DIR/scripts/p2p-upgrade-preflight.sh"
TMP_DIR="$(mktemp -d)"
cleanup() {
  if [[ -n "${SERVER_PID:-}" ]]; then
    kill "$SERVER_PID" >/dev/null 2>&1 || true
  fi
  if [[ -n "${KEEP_TMP_DIR:-}" ]]; then
    echo "KEEP_TMP_DIR=$TMP_DIR" >&2
  else
    rm -rf "$TMP_DIR"
  fi
}
trap cleanup EXIT

cat >"$TMP_DIR/pass.json" <<'JSON'
{
  "node_id": "node-pass",
  "running": true,
  "readiness": {
    "status": "ready",
    "failed_gates": [],
    "policy": {"max_network_height_lag": 1}
  },
  "sync": {"network_height_lag": 0},
  "consensus": {
    "committed_height": 10,
    "network_committed_height": 10,
    "replication_persisted_height": 10,
    "replication_gap_sync_blocked_height": null,
    "replication_gap_sync_blocked_reason": null,
    "replication_gap_sync_repair_attempt_summary": null,
    "known_peer_heads": 1,
    "network_head": {"fresh_peer_count": 1}
  }
}
JSON

cat >"$TMP_DIR/fail.json" <<'JSON'
{
  "node_id": "node-gap",
  "running": true,
  "readiness": {
    "status": "not_ready",
    "failed_gates": ["replication_gap_sync_blocked"],
    "policy": {"max_network_height_lag": 1}
  },
  "sync": {"network_height_lag": 42},
  "consensus": {
    "committed_height": 4,
    "network_committed_height": 30,
    "replication_persisted_height": 4,
    "replication_gap_sync_blocked_height": 5,
    "replication_gap_sync_blocked_reason": "replication gap sync blocked",
    "replication_gap_sync_repair_attempt_summary": "generic:found=false;peer:p1:found=false",
    "state_sync_fallback_required": true,
    "state_sync_snapshot_available": false,
    "state_sync_trusted_checkpoint_required_height": 30,
    "state_sync_fallback_reason": "trusted checkpoint and verified snapshot/state-sync required at height >= 30",
    "known_peer_heads": 0,
    "network_head": {"fresh_peer_count": 0}
  }
}
JSON

cat >"$TMP_DIR/fallback.json" <<'JSON'
{
  "node_id": "node-gap-with-snapshot",
  "running": true,
  "readiness": {
    "status": "not_ready",
    "failed_gates": ["replication_gap_sync_blocked"],
    "policy": {"max_network_height_lag": 100}
  },
  "sync": {"network_height_lag": 26},
  "consensus": {
    "committed_height": 4,
    "network_committed_height": 30,
    "replication_persisted_height": 4,
    "replication_gap_sync_blocked_height": 5,
    "replication_gap_sync_blocked_reason": "replication gap sync blocked",
    "replication_gap_sync_repair_attempt_summary": "generic:found=false;peer:p1:found=false",
    "state_sync_fallback_required": true,
    "state_sync_snapshot_available": true,
    "state_sync_trusted_checkpoint_required_height": 30,
    "state_sync_fallback_reason": "trusted checkpoint and verified snapshot/state-sync required at height >= 30",
    "known_peer_heads": 0,
    "network_head": {"fresh_peer_count": 0}
  }
}
JSON

cat >"$TMP_DIR/gap-without-fallback-flag.json" <<'JSON'
{
  "node_id": "node-gap-without-fallback-flag",
  "running": true,
  "readiness": {
    "status": "not_ready",
    "failed_gates": ["replication_gap_sync_blocked"],
    "policy": {"max_network_height_lag": 100}
  },
  "sync": {"network_height_lag": 0},
  "consensus": {
    "committed_height": 100,
    "network_committed_height": 120,
    "replication_persisted_height": 100,
    "replication_gap_sync_blocked_height": 101,
    "replication_gap_sync_blocked_reason": "replication gap sync blocked",
    "known_peer_heads": 1,
    "network_head": {"fresh_peer_count": 1}
  }
}
JSON

cat >"$TMP_DIR/trusted-checkpoint.json" <<'JSON'
{
  "height": 30,
  "block_hash": "trusted-block-hash",
  "source": "governance-signed-checkpoint",
  "validator_set_hash": "validator-set-root-1",
  "stake_root": "stake-root-1",
  "min_signatures": 2,
  "threshold_bps": 6700,
  "validator_stakes": {
    "v1": 40,
    "v2": 30,
    "v3": 30
  },
  "signatures": [
    {"validator_id": "v1", "signature": "sig1", "checkpoint_payload_sha256": "__CHECKPOINT_PAYLOAD_SHA256__"},
    {"validator_id": "v2", "signature": "sig2", "checkpoint_payload_sha256": "__CHECKPOINT_PAYLOAD_SHA256__"}
  ]
}
JSON

cat >"$TMP_DIR/insufficient-checkpoint.json" <<'JSON'
{
  "height": 30,
  "block_hash": "trusted-block-hash",
  "source": "governance-signed-checkpoint",
  "min_signatures": 2,
  "signatures": [
    {"validator_id": "v1", "signature": "sig1", "checkpoint_payload_sha256": "__CHECKPOINT_PAYLOAD_SHA256__"}
  ]
}
JSON

cat >"$TMP_DIR/duplicate-signer-checkpoint.json" <<'JSON'
{
  "height": 30,
  "block_hash": "trusted-block-hash",
  "source": "governance-signed-checkpoint",
  "min_signatures": 2,
  "signatures": [
    {"validator_id": "v1", "signature": "sig1", "checkpoint_payload_sha256": "__CHECKPOINT_PAYLOAD_SHA256__"},
    {"validator_id": "v1", "signature": "sig1b", "checkpoint_payload_sha256": "__CHECKPOINT_PAYLOAD_SHA256__"}
  ]
}
JSON

cat >"$TMP_DIR/insufficient-stake-checkpoint.json" <<'JSON'
{
  "height": 30,
  "block_hash": "trusted-block-hash",
  "source": "governance-signed-checkpoint",
  "min_signatures": 1,
  "threshold_bps": 6700,
  "validator_stakes": {
    "v1": 40,
    "v2": 30,
    "v3": 30
  },
  "signatures": [
    {"validator_id": "v1", "signature": "sig1", "checkpoint_payload_sha256": "__CHECKPOINT_PAYLOAD_SHA256__"}
  ]
}
JSON

cat >"$TMP_DIR/mismatched-payload-checkpoint.json" <<'JSON'
{
  "height": 30,
  "block_hash": "trusted-block-hash",
  "source": "governance-signed-checkpoint",
  "min_signatures": 1,
  "signatures": [
    {"validator_id": "v1", "signature": "sig1", "checkpoint_payload_sha256": "deadbeef"}
  ]
}
JSON

cat >"$TMP_DIR/verified-checkpoint.json" <<'JSON'
{
  "height": 30,
  "block_hash": "trusted-block-hash",
  "source": "governance-signed-checkpoint",
  "validator_set_hash": "__VALIDATOR_SET_HASH__",
  "stake_root": "__VALIDATOR_SET_STAKE_ROOT__",
  "min_signatures": 1,
  "signatures": [
    {
      "validator_id": "v1",
      "signature_hex": "__CHECKPOINT_SIGNATURE_HEX__",
      "public_key_path": "trusted-v1.pub.pem",
      "checkpoint_payload_sha256": "__CHECKPOINT_PAYLOAD_SHA256__"
    }
  ]
}
JSON

cat >"$TMP_DIR/tampered-signature-checkpoint.json" <<'JSON'
{
  "height": 30,
  "block_hash": "trusted-block-hash",
  "source": "governance-signed-checkpoint",
  "validator_set_hash": "__VALIDATOR_SET_HASH__",
  "stake_root": "__VALIDATOR_SET_STAKE_ROOT__",
  "min_signatures": 1,
  "signatures": [
    {
      "validator_id": "v1",
      "signature_hex": "__CHECKPOINT_SIGNATURE_HEX__",
      "public_key_path": "trusted-v1.pub.pem",
      "checkpoint_payload_sha256": "__CHECKPOINT_PAYLOAD_SHA256__"
    }
  ]
}
JSON

cat >"$TMP_DIR/mismatched-validator-set-checkpoint.json" <<'JSON'
{
  "height": 30,
  "block_hash": "trusted-block-hash",
  "source": "governance-signed-checkpoint",
  "validator_set_hash": "sha256:deadbeef",
  "stake_root": "__VALIDATOR_SET_STAKE_ROOT__",
  "min_signatures": 1,
  "signatures": [
    {
      "validator_id": "v1",
      "signature_hex": "deadbeef",
      "public_key_path": "trusted-v1.pub.pem",
      "checkpoint_payload_sha256": "__CHECKPOINT_PAYLOAD_SHA256__"
    }
  ]
}
JSON

cat >"$TMP_DIR/unknown-validator-checkpoint.json" <<'JSON'
{
  "height": 30,
  "block_hash": "trusted-block-hash",
  "source": "governance-signed-checkpoint",
  "validator_set_hash": "__VALIDATOR_SET_HASH__",
  "stake_root": "__VALIDATOR_SET_STAKE_ROOT__",
  "min_signatures": 1,
  "signatures": [
    {
      "validator_id": "v9",
      "signature_hex": "deadbeef",
      "public_key_path": "trusted-v1.pub.pem",
      "checkpoint_payload_sha256": "__CHECKPOINT_PAYLOAD_SHA256__"
    }
  ]
}
JSON

cat >"$TMP_DIR/validator-set.json" <<'JSON'
{
  "validators": [
    {"validator_id": "v1", "stake": 40, "public_key_path": "trusted-v1.pub.pem"},
    {"validator_id": "v2", "stake": 30, "public_key_path": "trusted-v2.pub.pem"},
    {"validator_id": "v3", "stake": 30, "public_key_path": "trusted-v3.pub.pem"}
  ]
}
JSON

cat >"$TMP_DIR/state-sync-bundle.json" <<'JSON'
{
  "checkpoint_height": 30,
  "checkpoint_hash": "trusted-block-hash",
  "bundle_hash": "bundle-hash-1",
  "snapshot_ref": "snapshot-ref-1",
  "state_root": "__STATE_ROOT__",
  "snapshot_path": "snapshots/checkpoint-30.cbor",
  "journal_path": "journals/checkpoint-30.cbor",
  "snapshot_sha256": "__SNAPSHOT_SHA256__",
  "journal_sha256": "__JOURNAL_SHA256__",
  "chunks": [
    {"path": "chunks/checkpoint-30.part0", "sha256": "__CHUNK0_SHA256__"},
    {"path": "chunks/checkpoint-30.part1", "sha256": "__CHUNK1_SHA256__"}
  ]
}
JSON

cat >"$TMP_DIR/mismatched-state-sync-bundle.json" <<'JSON'
{
  "checkpoint_height": 29,
  "checkpoint_hash": "trusted-block-hash",
  "bundle_hash": "bundle-hash-2",
  "snapshot_ref": "snapshot-ref-2",
  "state_root": "state-root-2"
}
JSON

mkdir -p "$TMP_DIR/state-sync-bundle/snapshots" "$TMP_DIR/state-sync-bundle/journals" "$TMP_DIR/state-sync-bundle/chunks"
cat >"$TMP_DIR/state-sync-bundle/snapshots/checkpoint-30.cbor" <<'JSON'
{
  "state": {
    "accounts": {"alice": 10, "bob": 20},
    "height": 30
  }
}
JSON
STATE_JSON="$(jq -S -c '.state' "$TMP_DIR/state-sync-bundle/snapshots/checkpoint-30.cbor")"
STATE_ROOT="sha256:$(printf '%s' "$STATE_JSON" | sha256sum | awk '{print $1}')"
cat >"$TMP_DIR/state-sync-bundle/journals/checkpoint-30.cbor" <<JSON
{
  "checkpoint_height": 30,
  "checkpoint_hash": "trusted-block-hash",
  "state_root": "$STATE_ROOT",
  "entries": []
}
JSON
printf 'chunk zero\n' >"$TMP_DIR/state-sync-bundle/chunks/checkpoint-30.part0"
printf 'chunk one\n' >"$TMP_DIR/state-sync-bundle/chunks/checkpoint-30.part1"
SNAPSHOT_SHA256="$(sha256sum "$TMP_DIR/state-sync-bundle/snapshots/checkpoint-30.cbor" | awk '{print $1}')"
JOURNAL_SHA256="$(sha256sum "$TMP_DIR/state-sync-bundle/journals/checkpoint-30.cbor" | awk '{print $1}')"
CHUNK0_SHA256="$(sha256sum "$TMP_DIR/state-sync-bundle/chunks/checkpoint-30.part0" | awk '{print $1}')"
CHUNK1_SHA256="$(sha256sum "$TMP_DIR/state-sync-bundle/chunks/checkpoint-30.part1" | awk '{print $1}')"
CHUNKS_JSON="$(jq -n -S -c \
  --arg chunk0 "$CHUNK0_SHA256" \
  --arg chunk1 "$CHUNK1_SHA256" \
  '[
    {"path": "chunks/checkpoint-30.part0", "sha256": $chunk0},
    {"path": "chunks/checkpoint-30.part1", "sha256": $chunk1}
  ] | sort_by(.path)')"
CHUNKS_ROOT="sha256:$(printf '%s' "$CHUNKS_JSON" | sha256sum | awk '{print $1}')"
sed -i "s/__STATE_ROOT__/$STATE_ROOT/g" "$TMP_DIR/state-sync-bundle.json"
sed -i "s/__SNAPSHOT_SHA256__/$SNAPSHOT_SHA256/g" "$TMP_DIR/state-sync-bundle.json"
sed -i "s/__JOURNAL_SHA256__/$JOURNAL_SHA256/g" "$TMP_DIR/state-sync-bundle.json"
sed -i "s/__CHUNK0_SHA256__/$CHUNK0_SHA256/g" "$TMP_DIR/state-sync-bundle.json"
sed -i "s/__CHUNK1_SHA256__/$CHUNK1_SHA256/g" "$TMP_DIR/state-sync-bundle.json"

VALIDATOR_SET_JSON="$(jq -S -c '
  [.validators[]
    | {
        validator_id: (.validator_id // .node_id // .signer),
        stake: ((.stake // .weight) | tonumber),
        public_key_path: (.public_key_path // .validator_public_key_path // null)
      }]
  | sort_by(.validator_id)
' "$TMP_DIR/validator-set.json")"
VALIDATOR_STAKE_JSON="$(jq -S -c '[.[] | {validator_id, stake}]' <<<"$VALIDATOR_SET_JSON")"
VALIDATOR_SET_HASH="sha256:$(printf '%s' "$VALIDATOR_SET_JSON" | sha256sum | awk '{print $1}')"
VALIDATOR_SET_STAKE_ROOT="sha256:$(printf '%s' "$VALIDATOR_STAKE_JSON" | sha256sum | awk '{print $1}')"
for checkpoint_manifest in \
  "$TMP_DIR/verified-checkpoint.json" \
  "$TMP_DIR/tampered-signature-checkpoint.json" \
  "$TMP_DIR/mismatched-validator-set-checkpoint.json" \
  "$TMP_DIR/unknown-validator-checkpoint.json"; do
  sed -i "s/__VALIDATOR_SET_HASH__/$VALIDATOR_SET_HASH/g" "$checkpoint_manifest"
  sed -i "s/__VALIDATOR_SET_STAKE_ROOT__/$VALIDATOR_SET_STAKE_ROOT/g" "$checkpoint_manifest"
done

for checkpoint_manifest in \
  "$TMP_DIR/trusted-checkpoint.json" \
  "$TMP_DIR/insufficient-checkpoint.json" \
  "$TMP_DIR/duplicate-signer-checkpoint.json" \
  "$TMP_DIR/insufficient-stake-checkpoint.json" \
  "$TMP_DIR/verified-checkpoint.json" \
  "$TMP_DIR/tampered-signature-checkpoint.json" \
  "$TMP_DIR/mismatched-validator-set-checkpoint.json" \
  "$TMP_DIR/unknown-validator-checkpoint.json"; do
  CHECKPOINT_PAYLOAD_JSON="$(jq -S -c '{
    height: (.height // .checkpoint_height),
    hash: (.block_hash // .checkpoint_hash),
    validator_set_hash: (.validator_set_hash // null),
    stake_root: (.stake_root // .validator_stake_root // null)
  }' "$checkpoint_manifest")"
  CHECKPOINT_PAYLOAD_SHA256="$(printf '%s' "$CHECKPOINT_PAYLOAD_JSON" | sha256sum | awk '{print $1}')"
  sed -i "s/__CHECKPOINT_PAYLOAD_SHA256__/$CHECKPOINT_PAYLOAD_SHA256/g" "$checkpoint_manifest"
done

openssl genpkey -algorithm Ed25519 -out "$TMP_DIR/trusted-v1.key.pem" >/dev/null 2>&1
openssl pkey -in "$TMP_DIR/trusted-v1.key.pem" -pubout -out "$TMP_DIR/trusted-v1.pub.pem" >/dev/null 2>&1
VERIFIED_CHECKPOINT_PAYLOAD_JSON="$(jq -S -c '{
  height: (.height // .checkpoint_height),
  hash: (.block_hash // .checkpoint_hash),
  validator_set_hash: (.validator_set_hash // null),
  stake_root: (.stake_root // .validator_stake_root // null)
}' "$TMP_DIR/verified-checkpoint.json")"
printf '%s' "$VERIFIED_CHECKPOINT_PAYLOAD_JSON" >"$TMP_DIR/verified-checkpoint.payload"
openssl pkeyutl -sign -rawin -inkey "$TMP_DIR/trusted-v1.key.pem" -in "$TMP_DIR/verified-checkpoint.payload" -out "$TMP_DIR/verified-checkpoint.sig" >/dev/null 2>&1
CHECKPOINT_SIGNATURE_HEX="$(xxd -p -c 256 "$TMP_DIR/verified-checkpoint.sig")"
sed -i "s/__CHECKPOINT_SIGNATURE_HEX__/$CHECKPOINT_SIGNATURE_HEX/g" "$TMP_DIR/verified-checkpoint.json"
sed -i "s/__CHECKPOINT_SIGNATURE_HEX__/deadbeef/g" "$TMP_DIR/tampered-signature-checkpoint.json"

PORT="$(python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
)"

python3 -m http.server "$PORT" --bind 127.0.0.1 --directory "$TMP_DIR" >"$TMP_DIR/server.log" 2>&1 &
SERVER_PID=$!

for _ in $(seq 1 50); do
  if curl -fsS "http://127.0.0.1:$PORT/pass.json" >/dev/null 2>&1; then
    break
  fi
  sleep 0.1
done

if ! curl -fsS "http://127.0.0.1:$PORT/pass.json" >/dev/null 2>&1; then
  echo "server did not start" >&2
  cat "$TMP_DIR/server.log" >&2 || true
  exit 1
fi

"$SCRIPT" --status-url "http://127.0.0.1:$PORT/pass.json" >"$TMP_DIR/pass.out"
grep -q '^PASS ' "$TMP_DIR/pass.out"

"$SCRIPT" --status-json "$TMP_DIR/pass.json" >"$TMP_DIR/pass-status-json.out"
grep -q '^PASS ' "$TMP_DIR/pass-status-json.out"

FAKE_CURL_DIR="$TMP_DIR/fake-curl-bin"
mkdir -p "$FAKE_CURL_DIR"
cat >"$FAKE_CURL_DIR/curl" <<SH
#!/usr/bin/env bash
if [[ " \$* " != *" --max-time "* ]]; then
  echo "curl missing --max-time: \$*" >&2
  exit 2
fi
cat "$TMP_DIR/pass.json"
SH
chmod +x "$FAKE_CURL_DIR/curl"
PATH="$FAKE_CURL_DIR:$PATH" "$SCRIPT" --status-url "http://fake/pass.json" >"$TMP_DIR/pass-status-url-timeout.out"
grep -q '^PASS ' "$TMP_DIR/pass-status-url-timeout.out"

if "$SCRIPT" --status-url "http://127.0.0.1:$PORT/fail.json" >"$TMP_DIR/fail.out"; then
  echo "expected failing preflight to exit non-zero" >&2
  exit 1
fi
grep -q 'replication_gap_sync_blocked' "$TMP_DIR/fail.out"
grep -q 'state_sync_fallback_checkpoint_unavailable' "$TMP_DIR/fail.out"
grep -q 'peer_head_unavailable_for_repair' "$TMP_DIR/fail.out"
grep -q 'network_height_lag_exceeds_policy' "$TMP_DIR/fail.out"

if "$SCRIPT" \
  --status-url "http://127.0.0.1:$PORT/gap-without-fallback-flag.json" \
  --recovery-plan-dir "$TMP_DIR/gap-without-fallback-plans" \
  >"$TMP_DIR/gap-without-fallback.out"; then
  echo "expected gap without trusted checkpoint to exit non-zero" >&2
  exit 1
fi
grep -q 'replication_gap_sync_blocked' "$TMP_DIR/gap-without-fallback.out"
test -f "$TMP_DIR/gap-without-fallback-plans/node-gap-without-fallback-flag.recovery-plan.json"
jq -e '
  .dry_run_only == true
  and .mode == "blocked_missing_trusted_checkpoint"
  and .required_height == 120
  and (.blocked_reasons | index("replication_gap_sync_blocked"))
  and (.operator_steps | length) == 3
' "$TMP_DIR/gap-without-fallback-plans/node-gap-without-fallback-flag.recovery-plan.json" >/dev/null

if "$SCRIPT" \
  --status-url "http://127.0.0.1:$PORT/gap-without-fallback-flag.json" \
  --trusted-checkpoint-height 121 \
  --trusted-checkpoint-hash future-checkpoint \
  --recovery-plan-dir "$TMP_DIR/future-checkpoint-plans" \
  >"$TMP_DIR/future-checkpoint.out"; then
  echo "expected future trusted checkpoint to exit non-zero" >&2
  exit 1
fi
grep -q 'replication_gap_sync_blocked' "$TMP_DIR/future-checkpoint.out"
test -f "$TMP_DIR/future-checkpoint-plans/node-gap-without-fallback-flag.recovery-plan.json"
jq -e '
  .dry_run_only == true
  and .mode == "blocked_missing_trusted_checkpoint"
  and .required_height == 120
  and .trusted_checkpoint_height == 121
  and (.blocked_reasons | index("replication_gap_sync_blocked"))
' "$TMP_DIR/future-checkpoint-plans/node-gap-without-fallback-flag.recovery-plan.json" >/dev/null

"$SCRIPT" \
  --status-url "http://127.0.0.1:$PORT/fallback.json" \
  --trusted-checkpoint-manifest "$TMP_DIR/trusted-checkpoint.json" \
  --state-sync-bundle-manifest "$TMP_DIR/state-sync-bundle.json" \
  --state-sync-bundle-dir "$TMP_DIR/state-sync-bundle" \
  --require-state-sync-bundle \
  --verify-state-sync-bundle-semantics \
  --generate-restore-command-plan \
  --node-service-name oasis7-node@testnet-a \
  --node-data-dir /var/lib/oasis7/testnet-a \
  --restore-backup-dir /var/backups/oasis7-state-sync \
  --restore-allow-dir /var/lib/oasis7 \
  --restore-allow-dir /var/backups/oasis7-state-sync \
  --restore-script-dir "$TMP_DIR/recovery-scripts" \
  --recovery-plan-dir "$TMP_DIR/recovery-plans" \
  >"$TMP_DIR/fallback.out"
grep -q '^PASS ' "$TMP_DIR/fallback.out"
grep -q 'trusted_checkpoint_state_sync_fallback_required' "$TMP_DIR/fallback.out"
test -f "$TMP_DIR/recovery-plans/node-gap-with-snapshot.recovery-plan.json"
jq -e --arg snapshot_sha "$SNAPSHOT_SHA256" --arg journal_sha "$JOURNAL_SHA256" --arg state_root "$STATE_ROOT" --arg chunks_root "$CHUNKS_ROOT" '
  .dry_run_only == true
  and .mode == "trusted_checkpoint_state_sync"
  and .required_height == 30
  and .trusted_checkpoint_height == 30
  and .trusted_checkpoint_hash == "trusted-block-hash"
  and .trusted_checkpoint_manifest != null
  and .trusted_checkpoint_source == "governance-signed-checkpoint"
  and .trusted_checkpoint_signature_count == 2
  and .trusted_checkpoint_unique_signer_count == 2
  and .trusted_checkpoint_min_signatures == 2
  and .trusted_checkpoint_validator_set_hash == "validator-set-root-1"
  and .trusted_checkpoint_stake_root == "stake-root-1"
  and .trusted_checkpoint_payload_sha256 != null
  and .trusted_checkpoint_signatures_verified == false
  and .trusted_checkpoint_threshold_bps == 6700
  and .trusted_checkpoint_approved_stake == 70
  and .trusted_checkpoint_required_stake == 67
  and .trusted_checkpoint_total_stake == 100
  and .state_sync_bundle_manifest != null
  and .state_sync_bundle_height == 30
  and .state_sync_bundle_checkpoint_hash == "trusted-block-hash"
  and .state_sync_bundle_hash == "bundle-hash-1"
  and .state_sync_bundle_snapshot_ref == "snapshot-ref-1"
  and .state_sync_bundle_state_root == $state_root
  and .state_sync_bundle_dir != null
  and .state_sync_bundle_snapshot_path == "snapshots/checkpoint-30.cbor"
  and .state_sync_bundle_journal_path == "journals/checkpoint-30.cbor"
  and .state_sync_bundle_snapshot_sha256 == $snapshot_sha
  and .state_sync_bundle_journal_sha256 == $journal_sha
  and .state_sync_bundle_chunk_count == 2
  and .state_sync_bundle_chunks_root == $chunks_root
  and .state_sync_bundle_ready == true
  and .require_state_sync_bundle == true
  and .state_sync_bundle_semantics_verified == true
  and .restore_command_plan_enabled == true
  and .node_service_name == "oasis7-node@testnet-a"
  and .node_data_dir == "/var/lib/oasis7/testnet-a"
  and .restore_backup_dir == "/var/backups/oasis7-state-sync"
  and (.restore_allow_dirs | length) == 2
  and (.restore_command_plan | length) == 20
  and (.rollback_command_plan | length) == 4
  and (.state_sync_bundle_chunk_manifest | length) == 2
  and (.restore_command_plan[] | select(.step == "verify_restore_toolchain") | .command | contains("command -v"))
  and (.restore_command_plan[] | select(.step == "verify_restore_toolchain") | .command | contains(" cut;"))
  and (.restore_command_plan[] | select(.step == "verify_state_sync_snapshot_sha256") | .command | contains("sha256sum -c -"))
  and (.restore_command_plan[] | select(.step == "verify_state_sync_journal_sha256") | .command | contains("sha256sum -c -"))
  and ([.restore_command_plan[] | select(.step == "verify_state_sync_chunk_sha256")] | length) == 2
  and (.restore_command_plan[] | select(.step == "verify_state_sync_chunks_root") | .command | contains("computed_chunks_root"))
  and (.restore_command_plan[] | select(.step == "write_service_show_snapshot") | .command | contains("service-before-state-sync.systemctl-show.txt"))
  and (.restore_command_plan[] | select(.step == "write_service_status_snapshot") | .command | contains("service-before-state-sync.systemctl-status.txt"))
  and (.restore_command_plan[] | select(.step == "stop_service") | .command == "systemctl stop oasis7-node@testnet-a")
  and (.restore_command_plan[] | select(.step == "backup_data_dir") | .command | contains("/var/backups/oasis7-state-sync/node-gap-with-snapshot/data-before-state-sync/"))
  and (.restore_command_plan[] | select(.step == "write_source_sha256_manifest") | .command | contains("data-before-state-sync.source.sha256"))
  and (.restore_command_plan[] | select(.step == "write_source_metadata_manifest") | .command | contains("data-before-state-sync.source.metadata.tsv"))
  and (.restore_command_plan[] | select(.step == "write_backup_sha256_manifest") | .command | contains("data-before-state-sync.sha256"))
  and (.restore_command_plan[] | select(.step == "write_backup_metadata_manifest") | .command | contains("data-before-state-sync.metadata.tsv"))
  and (.restore_command_plan[] | select(.step == "verify_backup_sha256_manifest") | .command | contains("cmp "))
  and (.restore_command_plan[] | select(.step == "verify_backup_metadata_manifest") | .command | contains("cmp "))
  and (.restore_command_plan[] | select(.step == "restore_snapshot") | .command | contains("snapshots/checkpoint-30.cbor"))
  and (.restore_command_plan[] | select(.step == "restore_journal") | .command | contains("journals/checkpoint-30.cbor"))
  and (.rollback_command_plan[] | select(.step == "restore_data_backup") | .command | contains("/var/backups/oasis7-state-sync/node-gap-with-snapshot/data-before-state-sync/"))
  and .restore_script != null
  and .rollback_script != null
  and .restore_script_sha256 != null
  and .rollback_script_sha256 != null
  and .snapshot_available == true
  and (.operator_steps | length) >= 6
' "$TMP_DIR/recovery-plans/node-gap-with-snapshot.recovery-plan.json" >/dev/null
CHUNKS_ROOT_COMMAND="$(
  jq -r '.restore_command_plan[] | select(.step == "verify_state_sync_chunks_root") | .command' \
    "$TMP_DIR/recovery-plans/node-gap-with-snapshot.recovery-plan.json"
)"
[[ "$CHUNKS_ROOT_COMMAND" == *"$CHUNKS_ROOT"* ]]
eval "$CHUNKS_ROOT_COMMAND"

"$SCRIPT" \
  --status-url "http://127.0.0.1:$PORT/fallback.json" \
  --trusted-checkpoint-height 30 \
  --trusted-checkpoint-hash trusted-block-hash \
  --trusted-checkpoint-manifest "$TMP_DIR/trusted-checkpoint.json" \
  --state-sync-bundle-manifest "$TMP_DIR/state-sync-bundle.json" \
  --state-sync-bundle-dir "$TMP_DIR/state-sync-bundle" \
  --require-state-sync-bundle \
  --verify-state-sync-bundle-semantics \
  --generate-restore-command-plan \
  --node-service-name oasis7-node@testnet-a \
  --node-data-dir /var/lib/oasis7/testnet-a \
  --restore-backup-dir /var/backups/oasis7-state-sync \
  --restore-allow-dir /var/lib/oasis7 \
  --restore-allow-dir /var/backups/oasis7-state-sync \
  --restore-script-dir "$TMP_DIR/recovery-scripts-lag" \
  --recovery-plan-dir "$TMP_DIR/recovery-plans-lag" \
  >"$TMP_DIR/recovery-plans-lag.out"
grep -q '^PASS ' "$TMP_DIR/recovery-plans-lag.out"
test -f "$TMP_DIR/recovery-plans-lag/node-gap-with-snapshot.recovery-plan.json"
jq -e '
  .dry_run_only == true
  and .mode == "trusted_checkpoint_state_sync"
  and .required_height == 30
  and .trusted_checkpoint_height == 30
  and (.blocked_reasons | index("network_height_lag_exceeds_policy") | not)
  and ((.warnings // []) | index("trusted_checkpoint_state_sync_fallback_required"))
' "$TMP_DIR/recovery-plans-lag/node-gap-with-snapshot.recovery-plan.json" >/dev/null
test -x "$TMP_DIR/recovery-scripts-lag/node-gap-with-snapshot.restore.sh"
test -x "$TMP_DIR/recovery-scripts/node-gap-with-snapshot.restore.sh"
test -x "$TMP_DIR/recovery-scripts/node-gap-with-snapshot.rollback.sh"
grep -q 'OASIS7_EXECUTE_RESTORE' "$TMP_DIR/recovery-scripts/node-gap-with-snapshot.restore.sh"
grep -q 'systemctl stop oasis7-node@testnet-a' "$TMP_DIR/recovery-scripts/node-gap-with-snapshot.restore.sh"
grep -q 'OASIS7_EXECUTE_ROLLBACK' "$TMP_DIR/recovery-scripts/node-gap-with-snapshot.rollback.sh"
grep -q 'restore confirmation mismatch' "$TMP_DIR/recovery-scripts/node-gap-with-snapshot.restore.sh"

if "$SCRIPT" \
  --status-url "http://127.0.0.1:$PORT/fallback.json" \
  --trusted-checkpoint-manifest "$TMP_DIR/trusted-checkpoint.json" \
  --state-sync-bundle-manifest "$TMP_DIR/state-sync-bundle.json" \
  --state-sync-bundle-dir "$TMP_DIR/state-sync-bundle" \
  --restore-script-dir "$TMP_DIR/restore-script-without-plan" \
  --recovery-plan-dir "$TMP_DIR/restore-script-without-plan-recovery" \
  >"$TMP_DIR/restore-script-without-plan.out" 2>"$TMP_DIR/restore-script-without-plan.err"; then
  echo "expected restore script dir without restore plan to exit non-zero" >&2
  exit 1
fi
grep -q 'restore-script-dir requires --generate-restore-command-plan' "$TMP_DIR/restore-script-without-plan.err"

if "$SCRIPT" \
  --status-url "http://127.0.0.1:$PORT/fallback.json" \
  --trusted-checkpoint-manifest "$TMP_DIR/trusted-checkpoint.json" \
  --state-sync-bundle-manifest "$TMP_DIR/state-sync-bundle.json" \
  --state-sync-bundle-dir "$TMP_DIR/state-sync-bundle" \
  --generate-restore-command-plan \
  --node-service-name oasis7-node@testnet-a \
  --node-data-dir "$TMP_DIR/unsafe node data" \
  --restore-backup-dir "$TMP_DIR/exec-backups" \
  --restore-allow-dir "$TMP_DIR" \
  --recovery-plan-dir "$TMP_DIR/unsafe-node-path-plans" \
  >"$TMP_DIR/unsafe-node-path.out" 2>"$TMP_DIR/unsafe-node-path.err"; then
  echo "expected shell-unsafe node data path to exit non-zero" >&2
  exit 1
fi
grep -q 'node-data-dir.*shell-unsafe' "$TMP_DIR/unsafe-node-path.err"

if "$SCRIPT" \
  --status-url "http://127.0.0.1:$PORT/fallback.json" \
  --trusted-checkpoint-manifest "$TMP_DIR/trusted-checkpoint.json" \
  --state-sync-bundle-manifest "$TMP_DIR/state-sync-bundle.json" \
  --state-sync-bundle-dir "$TMP_DIR/state-sync-bundle" \
  --generate-restore-command-plan \
  --node-service-name 'oasis7-node@testnet-a;reboot' \
  --node-data-dir "$TMP_DIR/exec-node-data" \
  --restore-backup-dir "$TMP_DIR/exec-backups" \
  --restore-allow-dir "$TMP_DIR" \
  --recovery-plan-dir "$TMP_DIR/unsafe-service-plans" \
  >"$TMP_DIR/unsafe-service.out" 2>"$TMP_DIR/unsafe-service.err"; then
  echo "expected shell-unsafe service name to exit non-zero" >&2
  exit 1
fi
grep -q 'node-service-name.*shell-unsafe' "$TMP_DIR/unsafe-service.err"

mkdir -p "$TMP_DIR/fake-bin" "$TMP_DIR/exec-node-data" "$TMP_DIR/exec-backups"
printf 'pre-restore ledger bytes\n' >"$TMP_DIR/exec-node-data/ledger.dat"
cat >"$TMP_DIR/fake-bin/systemctl" <<'SH'
#!/usr/bin/env bash
printf 'systemctl %s\n' "$*" >>"$FAKE_RESTORE_LOG"
if [[ "${1:-}" == "show" ]]; then
  printf 'Id=%s\nActiveState=active\nSubState=running\n' "${2:-unknown.service}"
elif [[ "${1:-}" == "status" ]]; then
  printf '%s - fake status\n   Active: active (running)\n' "${2:-unknown.service}"
fi
SH
cat >"$TMP_DIR/fake-bin/rsync" <<'SH'
#!/usr/bin/env bash
printf 'rsync %s\n' "$*" >>"$FAKE_RESTORE_LOG"
args=("$@")
src="${args[$((${#args[@]} - 2))]}"
dest="${args[$((${#args[@]} - 1))]}"
if [[ "$src" == */ ]]; then
  mkdir -p "$dest"
  cp -a "${src%/}/." "$dest/"
else
  if [[ -n "${FAIL_RESTORE_SNAPSHOT:-}" && "$dest" == */state-sync/snapshot ]]; then
    exit 23
  fi
  mkdir -p "$(dirname "$dest")"
  cp -a "$src" "$dest"
fi
SH
chmod +x "$TMP_DIR/fake-bin/systemctl" "$TMP_DIR/fake-bin/rsync"

FAKE_RESTORE_LOG="$TMP_DIR/fake-restore.log" \
PATH="$TMP_DIR/fake-bin:$PATH" \
OASIS7_ALLOW_RESTORE_EXECUTION=I_UNDERSTAND_THIS_CAN_REPLACE_NODE_STATE \
"$SCRIPT" \
  --status-url "http://127.0.0.1:$PORT/fallback.json" \
  --trusted-checkpoint-manifest "$TMP_DIR/trusted-checkpoint.json" \
  --state-sync-bundle-manifest "$TMP_DIR/state-sync-bundle.json" \
  --state-sync-bundle-dir "$TMP_DIR/state-sync-bundle" \
  --generate-restore-command-plan \
  --node-service-name oasis7-node@testnet-exec \
  --node-data-dir "$TMP_DIR/exec-node-data" \
  --restore-backup-dir "$TMP_DIR/exec-backups" \
  --restore-allow-dir "$TMP_DIR" \
  --restore-script-dir "$TMP_DIR/exec-scripts" \
  --execute-restore-scripts \
  --recovery-plan-dir "$TMP_DIR/exec-recovery-plans" \
  >"$TMP_DIR/execute-restore.out"
grep -q '^PASS ' "$TMP_DIR/execute-restore.out"
grep -q 'systemctl stop oasis7-node@testnet-exec' "$TMP_DIR/fake-restore.log"
grep -q 'systemctl start oasis7-node@testnet-exec' "$TMP_DIR/fake-restore.log"
test -f "$TMP_DIR/exec-backups/node-gap-with-snapshot/service-before-state-sync.systemctl-show.txt"
test -f "$TMP_DIR/exec-backups/node-gap-with-snapshot/service-before-state-sync.systemctl-status.txt"
grep -q 'ActiveState=active' "$TMP_DIR/exec-backups/node-gap-with-snapshot/service-before-state-sync.systemctl-show.txt"
test -f "$TMP_DIR/exec-backups/node-gap-with-snapshot/data-before-state-sync.sha256"
test -f "$TMP_DIR/exec-backups/node-gap-with-snapshot/data-before-state-sync.source.sha256"
test -f "$TMP_DIR/exec-backups/node-gap-with-snapshot/data-before-state-sync.metadata.tsv"
test -f "$TMP_DIR/exec-backups/node-gap-with-snapshot/data-before-state-sync.source.metadata.tsv"
cmp "$TMP_DIR/exec-backups/node-gap-with-snapshot/data-before-state-sync.source.sha256" "$TMP_DIR/exec-backups/node-gap-with-snapshot/data-before-state-sync.sha256"
cmp "$TMP_DIR/exec-backups/node-gap-with-snapshot/data-before-state-sync.source.metadata.tsv" "$TMP_DIR/exec-backups/node-gap-with-snapshot/data-before-state-sync.metadata.tsv"
grep -q 'ledger.dat' "$TMP_DIR/exec-backups/node-gap-with-snapshot/data-before-state-sync.sha256"
grep -q $'ledger.dat\tf\t25\t' "$TMP_DIR/exec-backups/node-gap-with-snapshot/data-before-state-sync.metadata.tsv"
jq -e '
  .restore_execution_status == "passed"
  and .restore_execution_exit_code == 0
  and .restore_execution_log != null
  and .restore_execution_state_file != null
' "$TMP_DIR/exec-recovery-plans/node-gap-with-snapshot.recovery-plan.json" >/dev/null
jq -e '
  .phase == "restore"
  and .status == "passed"
  and .exit_code == 0
  and .restore_execution_log != null
' "$TMP_DIR/exec-scripts/node-gap-with-snapshot.restore.state.json" >/dev/null

mkdir -p "$TMP_DIR/tampered-node-data" "$TMP_DIR/tampered-backups" "$TMP_DIR/tampered-bundle/snapshots" "$TMP_DIR/tampered-bundle/journals" "$TMP_DIR/tampered-bundle/chunks"
cp "$TMP_DIR/state-sync-bundle.json" "$TMP_DIR/tampered-state-sync-bundle.json"
cp "$TMP_DIR/state-sync-bundle/snapshots/checkpoint-30.cbor" "$TMP_DIR/tampered-bundle/snapshots/checkpoint-30.cbor"
cp "$TMP_DIR/state-sync-bundle/journals/checkpoint-30.cbor" "$TMP_DIR/tampered-bundle/journals/checkpoint-30.cbor"
cp "$TMP_DIR/state-sync-bundle/chunks/checkpoint-30.part0" "$TMP_DIR/tampered-bundle/chunks/checkpoint-30.part0"
cp "$TMP_DIR/state-sync-bundle/chunks/checkpoint-30.part1" "$TMP_DIR/tampered-bundle/chunks/checkpoint-30.part1"
printf 'pre-restore tampered bytes\n' >"$TMP_DIR/tampered-node-data/ledger.dat"
FAKE_RESTORE_LOG="$TMP_DIR/fake-tampered-generate.log" \
  PATH="$TMP_DIR/fake-bin:$PATH" \
  OASIS7_ALLOW_RESTORE_EXECUTION=I_UNDERSTAND_THIS_CAN_REPLACE_NODE_STATE \
  "$SCRIPT" \
    --status-url "http://127.0.0.1:$PORT/fallback.json" \
    --trusted-checkpoint-manifest "$TMP_DIR/trusted-checkpoint.json" \
    --state-sync-bundle-manifest "$TMP_DIR/tampered-state-sync-bundle.json" \
    --state-sync-bundle-dir "$TMP_DIR/tampered-bundle" \
    --generate-restore-command-plan \
    --node-service-name oasis7-node@testnet-tampered \
    --node-data-dir "$TMP_DIR/tampered-node-data" \
    --restore-backup-dir "$TMP_DIR/tampered-backups" \
    --restore-allow-dir "$TMP_DIR" \
    --restore-script-dir "$TMP_DIR/tampered-scripts" \
    --recovery-plan-dir "$TMP_DIR/tampered-recovery-plans" \
    >"$TMP_DIR/tampered-generate.out"
printf 'tampered after manifest validation\n' >"$TMP_DIR/tampered-bundle/snapshots/checkpoint-30.cbor"
set +e
FAKE_RESTORE_LOG="$TMP_DIR/fake-tampered.log" \
  PATH="$TMP_DIR/fake-bin:$PATH" \
  OASIS7_EXECUTE_RESTORE=node-gap-with-snapshot \
  bash "$TMP_DIR/tampered-scripts/node-gap-with-snapshot.restore.sh" \
  >"$TMP_DIR/tampered-scripts/node-gap-with-snapshot.restore.log" 2>&1
tampered_restore_exit=$?
set -e
if [[ "$tampered_restore_exit" -eq 0 ]]; then
  echo "expected restore script to fail when snapshot changes after manifest validation" >&2
  exit 1
fi
grep -q 'FAILED' "$TMP_DIR/tampered-scripts/node-gap-with-snapshot.restore.log"
if [[ -f "$TMP_DIR/fake-tampered.log" ]] && grep -q 'systemctl stop oasis7-node@testnet-tampered' "$TMP_DIR/fake-tampered.log"; then
  echo "expected tampered bundle execution to fail before stopping service" >&2
  exit 1
fi

mkdir -p "$TMP_DIR/tampered-chunk-node-data" "$TMP_DIR/tampered-chunk-backups" "$TMP_DIR/tampered-chunk-bundle/snapshots" "$TMP_DIR/tampered-chunk-bundle/journals" "$TMP_DIR/tampered-chunk-bundle/chunks"
cp "$TMP_DIR/state-sync-bundle.json" "$TMP_DIR/tampered-chunk-state-sync-bundle.json"
cp "$TMP_DIR/state-sync-bundle/snapshots/checkpoint-30.cbor" "$TMP_DIR/tampered-chunk-bundle/snapshots/checkpoint-30.cbor"
cp "$TMP_DIR/state-sync-bundle/journals/checkpoint-30.cbor" "$TMP_DIR/tampered-chunk-bundle/journals/checkpoint-30.cbor"
cp "$TMP_DIR/state-sync-bundle/chunks/checkpoint-30.part0" "$TMP_DIR/tampered-chunk-bundle/chunks/checkpoint-30.part0"
cp "$TMP_DIR/state-sync-bundle/chunks/checkpoint-30.part1" "$TMP_DIR/tampered-chunk-bundle/chunks/checkpoint-30.part1"
printf 'pre-restore tampered chunk bytes\n' >"$TMP_DIR/tampered-chunk-node-data/ledger.dat"
FAKE_RESTORE_LOG="$TMP_DIR/fake-tampered-chunk-generate.log" \
  PATH="$TMP_DIR/fake-bin:$PATH" \
  OASIS7_ALLOW_RESTORE_EXECUTION=I_UNDERSTAND_THIS_CAN_REPLACE_NODE_STATE \
  "$SCRIPT" \
    --status-url "http://127.0.0.1:$PORT/fallback.json" \
    --trusted-checkpoint-manifest "$TMP_DIR/trusted-checkpoint.json" \
    --state-sync-bundle-manifest "$TMP_DIR/tampered-chunk-state-sync-bundle.json" \
    --state-sync-bundle-dir "$TMP_DIR/tampered-chunk-bundle" \
    --generate-restore-command-plan \
    --node-service-name oasis7-node@testnet-tampered-chunk \
    --node-data-dir "$TMP_DIR/tampered-chunk-node-data" \
    --restore-backup-dir "$TMP_DIR/tampered-chunk-backups" \
    --restore-allow-dir "$TMP_DIR" \
    --restore-script-dir "$TMP_DIR/tampered-chunk-scripts" \
    --recovery-plan-dir "$TMP_DIR/tampered-chunk-recovery-plans" \
    >"$TMP_DIR/tampered-chunk-generate.out"
printf 'tampered chunk after manifest validation\n' >"$TMP_DIR/tampered-chunk-bundle/chunks/checkpoint-30.part0"
set +e
FAKE_RESTORE_LOG="$TMP_DIR/fake-tampered-chunk.log" \
  PATH="$TMP_DIR/fake-bin:$PATH" \
  OASIS7_EXECUTE_RESTORE=node-gap-with-snapshot \
  bash "$TMP_DIR/tampered-chunk-scripts/node-gap-with-snapshot.restore.sh" \
  >"$TMP_DIR/tampered-chunk-scripts/node-gap-with-snapshot.restore.log" 2>&1
tampered_chunk_restore_exit=$?
set -e
if [[ "$tampered_chunk_restore_exit" -eq 0 ]]; then
  echo "expected restore script to fail when chunk changes after manifest validation" >&2
  exit 1
fi
grep -q 'FAILED' "$TMP_DIR/tampered-chunk-scripts/node-gap-with-snapshot.restore.log"
if [[ -f "$TMP_DIR/fake-tampered-chunk.log" ]] && grep -q 'systemctl stop oasis7-node@testnet-tampered-chunk' "$TMP_DIR/fake-tampered-chunk.log"; then
  echo "expected tampered chunk execution to fail before stopping service" >&2
  exit 1
fi

mkdir -p "$TMP_DIR/fail-node-data" "$TMP_DIR/fail-backups"
printf 'pre-restore rollback bytes\n' >"$TMP_DIR/fail-node-data/ledger.dat"
if FAKE_RESTORE_LOG="$TMP_DIR/fake-rollback.log" \
  FAIL_RESTORE_SNAPSHOT=1 \
  PATH="$TMP_DIR/fake-bin:$PATH" \
  OASIS7_ALLOW_RESTORE_EXECUTION=I_UNDERSTAND_THIS_CAN_REPLACE_NODE_STATE \
  "$SCRIPT" \
    --status-url "http://127.0.0.1:$PORT/fallback.json" \
    --trusted-checkpoint-manifest "$TMP_DIR/trusted-checkpoint.json" \
    --state-sync-bundle-manifest "$TMP_DIR/state-sync-bundle.json" \
    --state-sync-bundle-dir "$TMP_DIR/state-sync-bundle" \
    --generate-restore-command-plan \
    --node-service-name oasis7-node@testnet-rollback \
    --node-data-dir "$TMP_DIR/fail-node-data" \
    --restore-backup-dir "$TMP_DIR/fail-backups" \
    --restore-allow-dir "$TMP_DIR" \
    --restore-script-dir "$TMP_DIR/rollback-scripts" \
    --execute-restore-scripts \
    --auto-rollback-on-restore-failure \
    --recovery-plan-dir "$TMP_DIR/rollback-recovery-plans" \
    >"$TMP_DIR/auto-rollback.out"; then
  echo "expected restore failure with auto rollback to exit non-zero" >&2
  exit 1
fi
jq -e '
  .restore_execution_status == "failed"
  and .restore_execution_exit_code == 23
  and .rollback_execution_status == "passed"
  and .rollback_execution_exit_code == 0
  and .rollback_execution_log != null
  and .rollback_execution_state_file != null
' "$TMP_DIR/rollback-recovery-plans/node-gap-with-snapshot.recovery-plan.json" >/dev/null
jq -e '
  .phase == "rollback"
  and .status == "passed"
  and .exit_code == 0
' "$TMP_DIR/rollback-scripts/node-gap-with-snapshot.rollback.state.json" >/dev/null

if "$SCRIPT" \
  --status-url "http://127.0.0.1:$PORT/fallback.json" \
  --trusted-checkpoint-manifest "$TMP_DIR/trusted-checkpoint.json" \
  --state-sync-bundle-manifest "$TMP_DIR/state-sync-bundle.json" \
  --state-sync-bundle-dir "$TMP_DIR/state-sync-bundle" \
  --generate-restore-command-plan \
  --node-service-name oasis7-node@testnet-a \
  --node-data-dir "$TMP_DIR/exec-node-data" \
  --restore-backup-dir "$TMP_DIR/exec-backups" \
  --restore-allow-dir "$TMP_DIR" \
  --restore-script-dir "$TMP_DIR/execute-without-env-scripts" \
  --execute-restore-scripts \
  --recovery-plan-dir "$TMP_DIR/execute-without-env-plans" \
  >"$TMP_DIR/execute-without-env.out" 2>"$TMP_DIR/execute-without-env.err"; then
  echo "expected execute restore without dangerous env confirmation to exit non-zero" >&2
  exit 1
fi
grep -q 'requires OASIS7_ALLOW_RESTORE_EXECUTION' "$TMP_DIR/execute-without-env.err"

if "$SCRIPT" \
  --status-url "http://127.0.0.1:$PORT/fallback.json" \
  --trusted-checkpoint-manifest "$TMP_DIR/trusted-checkpoint.json" \
  --state-sync-bundle-manifest "$TMP_DIR/state-sync-bundle.json" \
  --state-sync-bundle-dir "$TMP_DIR/state-sync-bundle" \
  --generate-restore-command-plan \
  --node-service-name oasis7-node@testnet-a \
  --node-data-dir /var/lib/oasis7/testnet-a \
  --recovery-plan-dir "$TMP_DIR/restore-plan-missing-backup-plans" \
  >"$TMP_DIR/restore-plan-missing-backup.out" 2>"$TMP_DIR/restore-plan-missing-backup.err"; then
  echo "expected restore command plan missing backup dir to exit non-zero" >&2
  exit 1
fi
grep -q 'requires --node-service-name, --node-data-dir, and --restore-backup-dir' "$TMP_DIR/restore-plan-missing-backup.err"

if "$SCRIPT" \
  --status-url "http://127.0.0.1:$PORT/fallback.json" \
  --trusted-checkpoint-manifest "$TMP_DIR/trusted-checkpoint.json" \
  --state-sync-bundle-manifest "$TMP_DIR/state-sync-bundle.json" \
  --state-sync-bundle-dir "$TMP_DIR/state-sync-bundle" \
  --generate-restore-command-plan \
  --node-service-name oasis7-node@testnet-a \
  --node-data-dir /var/lib/oasis7/testnet-a \
  --restore-backup-dir /var/backups/oasis7-state-sync \
  --recovery-plan-dir "$TMP_DIR/restore-plan-missing-allow-plans" \
  >"$TMP_DIR/restore-plan-missing-allow.out" 2>"$TMP_DIR/restore-plan-missing-allow.err"; then
  echo "expected restore command plan missing allow dir to exit non-zero" >&2
  exit 1
fi
grep -q 'requires at least one --restore-allow-dir' "$TMP_DIR/restore-plan-missing-allow.err"

if "$SCRIPT" \
  --status-url "http://127.0.0.1:$PORT/fallback.json" \
  --trusted-checkpoint-manifest "$TMP_DIR/trusted-checkpoint.json" \
  --state-sync-bundle-manifest "$TMP_DIR/state-sync-bundle.json" \
  --state-sync-bundle-dir "$TMP_DIR/state-sync-bundle" \
  --generate-restore-command-plan \
  --node-service-name oasis7-node@testnet-a \
  --node-data-dir /var/lib/oasis7/testnet-a \
  --restore-backup-dir /var/backups/oasis7-state-sync \
  --restore-allow-dir /tmp/not-oasis7 \
  --recovery-plan-dir "$TMP_DIR/restore-plan-outside-allow-plans" \
  >"$TMP_DIR/restore-plan-outside-allow.out" 2>"$TMP_DIR/restore-plan-outside-allow.err"; then
  echo "expected restore command plan outside allow dir to exit non-zero" >&2
  exit 1
fi
grep -q 'outside --restore-allow-dir roots' "$TMP_DIR/restore-plan-outside-allow.err"

if "$SCRIPT" \
  --status-url "http://127.0.0.1:$PORT/fallback.json" \
  --trusted-checkpoint-manifest "$TMP_DIR/trusted-checkpoint.json" \
  --require-state-sync-bundle \
  --recovery-plan-dir "$TMP_DIR/missing-bundle-recovery-plans" \
  >"$TMP_DIR/missing-bundle.out"; then
  echo "expected missing required state-sync bundle to exit non-zero" >&2
  exit 1
fi
grep -q 'state_sync_bundle_required' "$TMP_DIR/missing-bundle.out"
test -f "$TMP_DIR/missing-bundle-recovery-plans/node-gap-with-snapshot.recovery-plan.json"
jq -e '
  .dry_run_only == true
  and .mode == "blocked_missing_state_sync_bundle"
  and .state_sync_bundle_ready == false
  and .require_state_sync_bundle == true
  and (.blocked_reasons | index("state_sync_bundle_required"))
' "$TMP_DIR/missing-bundle-recovery-plans/node-gap-with-snapshot.recovery-plan.json" >/dev/null

if "$SCRIPT" \
  --status-url "http://127.0.0.1:$PORT/fallback.json" \
  --trusted-checkpoint-manifest "$TMP_DIR/trusted-checkpoint.json" \
  --state-sync-bundle-manifest "$TMP_DIR/state-sync-bundle.json" \
  --state-sync-bundle-dir "$TMP_DIR/missing-state-sync-bundle" \
  >"$TMP_DIR/bundle-dir-missing.out" 2>"$TMP_DIR/bundle-dir-missing.err"; then
  echo "expected missing state-sync bundle dir to exit non-zero" >&2
  exit 1
fi
grep -q 'state-sync-bundle-dir not found' "$TMP_DIR/bundle-dir-missing.err"

cat >"$TMP_DIR/bundle-hash-mismatch.json" <<'JSON'
{
  "checkpoint_height": 30,
  "checkpoint_hash": "trusted-block-hash",
  "bundle_hash": "bundle-hash-1",
  "snapshot_ref": "snapshot-ref-1",
  "state_root": "state-root-1",
  "snapshot_path": "snapshots/checkpoint-30.cbor",
  "journal_path": "journals/checkpoint-30.cbor",
  "snapshot_sha256": "deadbeef",
  "journal_sha256": "deadbeef"
}
JSON

if "$SCRIPT" \
  --status-url "http://127.0.0.1:$PORT/fallback.json" \
  --trusted-checkpoint-manifest "$TMP_DIR/trusted-checkpoint.json" \
  --state-sync-bundle-manifest "$TMP_DIR/bundle-hash-mismatch.json" \
  --state-sync-bundle-dir "$TMP_DIR/state-sync-bundle" \
  >"$TMP_DIR/bundle-hash-mismatch.out" 2>"$TMP_DIR/bundle-hash-mismatch.err"; then
  echo "expected state-sync bundle hash mismatch to exit non-zero" >&2
  exit 1
fi
grep -q 'snapshot hash mismatch' "$TMP_DIR/bundle-hash-mismatch.err"

cat >"$TMP_DIR/bundle-state-root-mismatch.json" <<JSON
{
  "checkpoint_height": 30,
  "checkpoint_hash": "trusted-block-hash",
  "bundle_hash": "bundle-hash-1",
  "snapshot_ref": "snapshot-ref-1",
  "state_root": "sha256:deadbeef",
  "snapshot_path": "snapshots/checkpoint-30.cbor",
  "journal_path": "journals/checkpoint-30.cbor",
  "snapshot_sha256": "$SNAPSHOT_SHA256",
  "journal_sha256": "$JOURNAL_SHA256"
}
JSON

if "$SCRIPT" \
  --status-url "http://127.0.0.1:$PORT/fallback.json" \
  --trusted-checkpoint-manifest "$TMP_DIR/trusted-checkpoint.json" \
  --state-sync-bundle-manifest "$TMP_DIR/bundle-state-root-mismatch.json" \
  --state-sync-bundle-dir "$TMP_DIR/state-sync-bundle" \
  --verify-state-sync-bundle-semantics \
  >"$TMP_DIR/bundle-state-root-mismatch.out" 2>"$TMP_DIR/bundle-state-root-mismatch.err"; then
  echo "expected state-sync bundle state root mismatch to exit non-zero" >&2
  exit 1
fi
grep -q 'snapshot state root mismatch' "$TMP_DIR/bundle-state-root-mismatch.err"

cat >"$TMP_DIR/bundle-chunk-hash-mismatch.json" <<JSON
{
  "checkpoint_height": 30,
  "checkpoint_hash": "trusted-block-hash",
  "bundle_hash": "bundle-hash-1",
  "snapshot_ref": "snapshot-ref-1",
  "state_root": "$STATE_ROOT",
  "snapshot_path": "snapshots/checkpoint-30.cbor",
  "journal_path": "journals/checkpoint-30.cbor",
  "snapshot_sha256": "$SNAPSHOT_SHA256",
  "journal_sha256": "$JOURNAL_SHA256",
  "chunks": [
    {"path": "chunks/checkpoint-30.part0", "sha256": "deadbeef"},
    {"path": "chunks/checkpoint-30.part1", "sha256": "$CHUNK1_SHA256"}
  ]
}
JSON

if "$SCRIPT" \
  --status-url "http://127.0.0.1:$PORT/fallback.json" \
  --trusted-checkpoint-manifest "$TMP_DIR/trusted-checkpoint.json" \
  --state-sync-bundle-manifest "$TMP_DIR/bundle-chunk-hash-mismatch.json" \
  --state-sync-bundle-dir "$TMP_DIR/state-sync-bundle" \
  >"$TMP_DIR/bundle-chunk-hash-mismatch.out" 2>"$TMP_DIR/bundle-chunk-hash-mismatch.err"; then
  echo "expected state-sync bundle chunk hash mismatch to exit non-zero" >&2
  exit 1
fi
grep -q 'chunk hash mismatch' "$TMP_DIR/bundle-chunk-hash-mismatch.err"

if "$SCRIPT" \
  --status-url "http://127.0.0.1:$PORT/fallback.json" \
  --trusted-checkpoint-manifest "$TMP_DIR/trusted-checkpoint.json" \
  --state-sync-bundle-manifest "$TMP_DIR/mismatched-state-sync-bundle.json" \
  >"$TMP_DIR/bundle-mismatch.out" 2>"$TMP_DIR/bundle-mismatch.err"; then
  echo "expected state-sync bundle mismatch to exit non-zero" >&2
  exit 1
fi
grep -q 'checkpoint height does not match' "$TMP_DIR/bundle-mismatch.err"

if "$SCRIPT" \
  --status-url "http://127.0.0.1:$PORT/fallback.json" \
  --trusted-checkpoint-manifest "$TMP_DIR/insufficient-checkpoint.json" \
  >"$TMP_DIR/insufficient.out" 2>"$TMP_DIR/insufficient.err"; then
  echo "expected insufficient checkpoint signatures to exit non-zero" >&2
  exit 1
fi
grep -q 'below min_signatures' "$TMP_DIR/insufficient.err"

if "$SCRIPT" \
  --status-url "http://127.0.0.1:$PORT/fallback.json" \
  --trusted-checkpoint-manifest "$TMP_DIR/duplicate-signer-checkpoint.json" \
  >"$TMP_DIR/duplicate.out" 2>"$TMP_DIR/duplicate.err"; then
  echo "expected duplicate signer checkpoint to exit non-zero" >&2
  exit 1
fi
grep -q 'duplicate or missing validator signer ids' "$TMP_DIR/duplicate.err"

if "$SCRIPT" \
  --status-url "http://127.0.0.1:$PORT/fallback.json" \
  --trusted-checkpoint-manifest "$TMP_DIR/insufficient-stake-checkpoint.json" \
  >"$TMP_DIR/insufficient-stake.out" 2>"$TMP_DIR/insufficient-stake.err"; then
  echo "expected insufficient checkpoint stake to exit non-zero" >&2
  exit 1
fi
grep -q 'signed stake 40 is below required stake 67' "$TMP_DIR/insufficient-stake.err"

if "$SCRIPT" \
  --status-url "http://127.0.0.1:$PORT/fallback.json" \
  --trusted-checkpoint-manifest "$TMP_DIR/mismatched-payload-checkpoint.json" \
  >"$TMP_DIR/payload-mismatch.out" 2>"$TMP_DIR/payload-mismatch.err"; then
  echo "expected checkpoint payload sha mismatch to exit non-zero" >&2
  exit 1
fi
grep -q 'signature payload sha256 mismatch' "$TMP_DIR/payload-mismatch.err"

"$SCRIPT" \
  --status-url "http://127.0.0.1:$PORT/fallback.json" \
  --trusted-checkpoint-manifest "$TMP_DIR/verified-checkpoint.json" \
  --verify-trusted-checkpoint-signatures \
  --validator-set-manifest "$TMP_DIR/validator-set.json" \
  --recovery-plan-dir "$TMP_DIR/verified-signature-recovery-plans" \
  >"$TMP_DIR/verified-signature.out"
grep -q '^PASS ' "$TMP_DIR/verified-signature.out"
jq -e '
  .trusted_checkpoint_signatures_verified == true
  and .trusted_checkpoint_payload_sha256 != null
  and .validator_set_manifest != null
  and .validator_set_hash != null
  and .validator_set_stake_root != null
  and .validator_set_proof_verified == true
' "$TMP_DIR/verified-signature-recovery-plans/node-gap-with-snapshot.recovery-plan.json" >/dev/null

"$SCRIPT" \
  --status-json "$TMP_DIR/fallback.json" \
  --trusted-checkpoint-manifest "$TMP_DIR/verified-checkpoint.json" \
  --verify-trusted-checkpoint-signatures \
  --validator-set-manifest "$TMP_DIR/validator-set.json" \
  --recovery-plan-dir "$TMP_DIR/verified-signature-status-json-plans" \
  >"$TMP_DIR/verified-signature-status-json.out"
grep -q '^PASS ' "$TMP_DIR/verified-signature-status-json.out"
jq -e '
  .status_url != null
  and .trusted_checkpoint_signatures_verified == true
  and .validator_set_proof_verified == true
' "$TMP_DIR/verified-signature-status-json-plans/node-gap-with-snapshot.recovery-plan.json" >/dev/null

if "$SCRIPT" \
  --status-url "http://127.0.0.1:$PORT/fallback.json" \
  --trusted-checkpoint-manifest "$TMP_DIR/tampered-signature-checkpoint.json" \
  --verify-trusted-checkpoint-signatures \
  >"$TMP_DIR/tampered-signature.out" 2>"$TMP_DIR/tampered-signature.err"; then
  echo "expected tampered checkpoint signature to exit non-zero" >&2
  exit 1
fi
grep -q 'signature verification failed' "$TMP_DIR/tampered-signature.err"

if "$SCRIPT" \
  --status-url "http://127.0.0.1:$PORT/fallback.json" \
  --trusted-checkpoint-manifest "$TMP_DIR/mismatched-validator-set-checkpoint.json" \
  --validator-set-manifest "$TMP_DIR/validator-set.json" \
  >"$TMP_DIR/mismatched-validator-set.out" 2>"$TMP_DIR/mismatched-validator-set.err"; then
  echo "expected validator set hash mismatch to exit non-zero" >&2
  exit 1
fi
grep -q 'validator_set_hash does not match' "$TMP_DIR/mismatched-validator-set.err"

if "$SCRIPT" \
  --status-url "http://127.0.0.1:$PORT/fallback.json" \
  --trusted-checkpoint-manifest "$TMP_DIR/unknown-validator-checkpoint.json" \
  --validator-set-manifest "$TMP_DIR/validator-set.json" \
  >"$TMP_DIR/unknown-validator.out" 2>"$TMP_DIR/unknown-validator.err"; then
  echo "expected signer outside validator set to exit non-zero" >&2
  exit 1
fi
grep -q 'signer outside independently verified validator set' "$TMP_DIR/unknown-validator.err"

if "$SCRIPT" \
  --status-url "http://127.0.0.1:$PORT/fallback.json" \
  --trusted-checkpoint-manifest "$TMP_DIR/trusted-checkpoint.json" \
  --trusted-checkpoint-height 31 \
  >"$TMP_DIR/manifest-mismatch.out" 2>"$TMP_DIR/manifest-mismatch.err"; then
  echo "expected manifest/height mismatch to exit non-zero" >&2
  exit 1
fi
grep -q 'does not match manifest height' "$TMP_DIR/manifest-mismatch.err"

if "$SCRIPT" \
  --status-url "http://127.0.0.1:$PORT/fallback.json" \
  --trusted-checkpoint-height 29 \
  --trusted-checkpoint-hash stale-block-hash \
  --recovery-plan-dir "$TMP_DIR/stale-recovery-plans" \
  >"$TMP_DIR/stale-fallback.out"; then
  echo "expected stale trusted checkpoint to exit non-zero" >&2
  exit 1
fi
grep -q 'state_sync_fallback_checkpoint_unavailable' "$TMP_DIR/stale-fallback.out"
test -f "$TMP_DIR/stale-recovery-plans/node-gap-with-snapshot.recovery-plan.json"
jq -e '
  .dry_run_only == true
  and .mode == "blocked_missing_trusted_checkpoint"
  and (.blocked_reasons | index("state_sync_fallback_checkpoint_unavailable"))
' "$TMP_DIR/stale-recovery-plans/node-gap-with-snapshot.recovery-plan.json" >/dev/null
