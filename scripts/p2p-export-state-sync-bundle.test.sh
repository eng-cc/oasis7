#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-state-sync-export.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

WORLD_DIR="$TMP_DIR/world"
OUT_DIR="$TMP_DIR/out"
mkdir -p "$WORLD_DIR"

cat >"$WORLD_DIR/snapshot.json" <<'JSON'
{
  "state": {
    "governance_finality_signer_registry": {
      "slot_id": "governance.finality.v1",
      "threshold": 2,
      "threshold_bps": 6700,
      "signer_bindings": {
        "triad-testnet-sequencer": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "triad-testnet-storage": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
      },
      "validator_stakes": {
        "triad-testnet-sequencer": 60,
        "triad-testnet-storage": 40
      }
    },
    "accounts": {
      "alice": 10
    }
  },
  "tick_consensus_records": []
}
JSON

cat >"$WORLD_DIR/journal.json" <<'JSON'
[]
JSON

cat >"$TMP_DIR/status.json" <<'JSON'
{
  "node_id": "triad-testnet-sequencer",
  "running": true,
  "consensus": {
    "network_committed_height": 3085,
    "network_head": {
      "height": 3085,
      "block_hash": "trusted-block-hash",
      "execution_state_root": "peer-state-root"
    },
    "state_sync_fallback_required": true,
    "state_sync_snapshot_available": false,
    "state_sync_trusted_checkpoint_required_height": 3085
  },
  "sync": {
    "network_height_lag": 3084
  },
  "readiness": {
    "policy": {
      "max_network_height_lag": 1
    },
    "failed_gates": [
      "state_sync_fallback_required"
    ],
    "status": "not_ready"
  }
}
JSON

"$ROOT_DIR/scripts/p2p-export-state-sync-bundle.sh" \
  --status-json "$TMP_DIR/status.json" \
  --world-dir "$WORLD_DIR" \
  --out-dir "$OUT_DIR" \
  >"$TMP_DIR/export.out"

test -f "$OUT_DIR/trusted-checkpoint.json"
test -f "$OUT_DIR/validator-set.json"
test -f "$OUT_DIR/state-sync-bundle.json"
test -f "$OUT_DIR/state-sync-bundle/snapshots/checkpoint-3085.json"

jq -e '
  .height == 3085
  and .block_hash == "trusted-block-hash"
  and .source == "unsigned-local-world-export"
  and .min_signatures == 0
  and .threshold_bps == 0
  and .validator_stakes["triad-testnet-sequencer"] == 60
  and .validator_stakes["triad-testnet-storage"] == 40
' "$OUT_DIR/trusted-checkpoint.json" >/dev/null

jq -e '
  (.validators | length) == 2
  and .validators[0].validator_id == "triad-testnet-sequencer"
  and .validators[0].stake == 60
  and .validators[1].validator_id == "triad-testnet-storage"
  and .validators[1].stake == 40
' "$OUT_DIR/validator-set.json" >/dev/null

"$ROOT_DIR/scripts/p2p-upgrade-preflight.sh" \
  --status-json "$TMP_DIR/status.json" \
  --trusted-checkpoint-manifest "$OUT_DIR/trusted-checkpoint.json" \
  --validator-set-manifest "$OUT_DIR/validator-set.json" \
  --state-sync-bundle-manifest "$OUT_DIR/state-sync-bundle.json" \
  --state-sync-bundle-dir "$OUT_DIR/state-sync-bundle" \
  --require-state-sync-bundle \
  --verify-state-sync-bundle-semantics \
  >"$TMP_DIR/preflight.out"

grep -q '^PASS ' "$TMP_DIR/preflight.out"
grep -q 'trusted_checkpoint_state_sync_fallback_required' "$TMP_DIR/preflight.out"

mkdir -p "$TMP_DIR/node/world" "$TMP_DIR/node/execution-records"
cp "$WORLD_DIR/snapshot.json" "$TMP_DIR/node/world/snapshot.json"
cp "$WORLD_DIR/journal.json" "$TMP_DIR/node/world/journal.json"

if "$ROOT_DIR/scripts/p2p-export-state-sync-bundle.sh" \
  --status-json "$TMP_DIR/status.json" \
  --world-dir "$TMP_DIR/node/world" \
  --out-dir "$TMP_DIR/node-out" \
  >"$TMP_DIR/node-export.out" 2>"$TMP_DIR/node-export.err"; then
  echo "expected export script to reject missing execution bridge latest record" >&2
  exit 1
fi

grep -q 'materialized execution records' "$TMP_DIR/node-export.err"

# RED regression: the export must not declare a 8435 status checkpoint while
# packaging an execution-world snapshot from 8399. Current production code
# accepts the mixed input because it only checks that latest.json exists.
MIXED_WORLD_DIR="$TMP_DIR/mixed-world"
MIXED_RECORDS_DIR="$TMP_DIR/mixed-execution-records"
MIXED_OUT_DIR="$TMP_DIR/mixed-out"
mkdir -p "$MIXED_WORLD_DIR" "$MIXED_RECORDS_DIR"
cat >"$MIXED_WORLD_DIR/snapshot.json" <<'JSON'
{
  "checkpoint_height": 8399,
  "state": {
    "governance_finality_signer_registry": {
      "slot_id": "governance.finality.v1",
      "threshold": 2,
      "threshold_bps": 6700,
      "signer_bindings": {
        "triad-testnet-sequencer": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "triad-testnet-storage": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
      },
      "validator_stakes": {
        "triad-testnet-sequencer": 60,
        "triad-testnet-storage": 40
      }
    }
  },
  "tick_consensus_records": []
}
JSON
printf '[]\n' >"$MIXED_WORLD_DIR/journal.json"
printf '{"height":8435,"execution_state_root":"state-h8435"}\n' >"$MIXED_RECORDS_DIR/latest.json"
cat >"$TMP_DIR/mixed-status.json" <<'JSON'
{
  "consensus": {
    "network_head": {
      "height": 8435,
      "block_hash": "block-h8435",
      "execution_state_root": "state-h8435"
    }
  }
}
JSON

if "$ROOT_DIR/scripts/p2p-export-state-sync-bundle.sh" \
  --status-json "$TMP_DIR/mixed-status.json" \
  --world-dir "$MIXED_WORLD_DIR" \
  --execution-records-dir "$MIXED_RECORDS_DIR" \
  --out-dir "$MIXED_OUT_DIR" \
  >"$TMP_DIR/mixed-export.out" 2>"$TMP_DIR/mixed-export.err"; then
  jq -e '.checkpoint_height == 8435' "$MIXED_OUT_DIR/state-sync-bundle.json" >/dev/null
  jq -e '.checkpoint_height == 8399' \
    "$MIXED_OUT_DIR/state-sync-bundle/snapshots/checkpoint-8435.json" >/dev/null
  echo "expected export to reject mixed state-sync checkpoint identity (8399 snapshot / 8435 status and execution record)" >&2
  exit 1
fi

grep -q 'state_sync_checkpoint_mismatch' "$TMP_DIR/mixed-export.err"
