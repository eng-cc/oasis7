#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/p2p-export-state-sync-bundle.sh
       [--status-url <url> | --status-json <path>]
       --world-dir <dir>
       --out-dir <dir>
       [--execution-records-dir <dir>]
       [--checkpoint-height <height>]
       [--checkpoint-hash <hash>]

Exports a trusted checkpoint manifest, validator-set manifest, and a minimal
state-sync bundle from a healthy node's persisted execution world plus chain
status. The resulting artifacts are designed to be consumed by
scripts/p2p-upgrade-preflight.sh.
USAGE
}

STATUS_URL=""
STATUS_JSON=""
WORLD_DIR=""
OUT_DIR=""
EXECUTION_RECORDS_DIR=""
CHECKPOINT_HEIGHT=""
CHECKPOINT_HASH=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --status-url)
      STATUS_URL="${2:-}"
      shift 2
      ;;
    --status-json)
      STATUS_JSON="${2:-}"
      shift 2
      ;;
    --world-dir)
      WORLD_DIR="${2:-}"
      shift 2
      ;;
    --out-dir)
      OUT_DIR="${2:-}"
      shift 2
      ;;
    --execution-records-dir)
      EXECUTION_RECORDS_DIR="${2:-}"
      shift 2
      ;;
    --checkpoint-height)
      CHECKPOINT_HEIGHT="${2:-}"
      shift 2
      ;;
    --checkpoint-hash)
      CHECKPOINT_HASH="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -z "$WORLD_DIR" || -z "$OUT_DIR" ]]; then
  echo "error: --world-dir and --out-dir are required" >&2
  usage >&2
  exit 2
fi
if [[ -n "$STATUS_URL" && -n "$STATUS_JSON" ]]; then
  echo "error: provide only one of --status-url or --status-json" >&2
  exit 2
fi
if [[ -z "$STATUS_URL" && -z "$STATUS_JSON" ]]; then
  echo "error: one of --status-url or --status-json is required" >&2
  exit 2
fi

SNAPSHOT_PATH="$WORLD_DIR/snapshot.json"
JOURNAL_PATH="$WORLD_DIR/journal.json"
if [[ ! -f "$SNAPSHOT_PATH" ]]; then
  echo "error: snapshot file not found: $SNAPSHOT_PATH" >&2
  exit 2
fi
if [[ ! -f "$JOURNAL_PATH" ]]; then
  echo "error: journal file not found: $JOURNAL_PATH" >&2
  exit 2
fi

if [[ -z "$EXECUTION_RECORDS_DIR" ]]; then
  candidate_execution_records_dir="$(dirname "$WORLD_DIR")/execution-records"
  if [[ -d "$candidate_execution_records_dir" ]]; then
    EXECUTION_RECORDS_DIR="$candidate_execution_records_dir"
  fi
fi
if [[ -n "$EXECUTION_RECORDS_DIR" ]]; then
  if [[ ! -d "$EXECUTION_RECORDS_DIR" ]]; then
    echo "error: execution records dir not found: $EXECUTION_RECORDS_DIR" >&2
    exit 2
  fi
  if [[ ! -f "$EXECUTION_RECORDS_DIR/latest.json" ]]; then
    echo "error: execution records latest.json not found in $EXECUTION_RECORDS_DIR; export must run from a node with materialized execution records" >&2
    exit 2
  fi
fi

state_sync_checkpoint_mismatch() {
  echo "error: state_sync_checkpoint_mismatch: $*" >&2
  exit 2
}

mkdir -p "$OUT_DIR"
STATUS_CAPTURE_PATH="$OUT_DIR/status.json"
if [[ -n "$STATUS_JSON" ]]; then
  [[ -f "$STATUS_JSON" ]] || {
    echo "error: status json not found: $STATUS_JSON" >&2
    exit 2
  }
  cp "$STATUS_JSON" "$STATUS_CAPTURE_PATH"
else
  curl -fsSL "$STATUS_URL" -o "$STATUS_CAPTURE_PATH"
fi

STATUS_CANONICAL_JSON="$(jq -S -c '.' "$STATUS_CAPTURE_PATH")" || {
  echo "error: failed to parse status payload: $STATUS_CAPTURE_PATH" >&2
  exit 2
}

derived_checkpoint_height="$(jq -r '.consensus.network_head.height // .consensus.network_committed_height // empty' <<<"$STATUS_CANONICAL_JSON")"
derived_checkpoint_hash="$(jq -r '.consensus.network_head.block_hash // empty' <<<"$STATUS_CANONICAL_JSON")"
CHECKPOINT_HEIGHT="${CHECKPOINT_HEIGHT:-$derived_checkpoint_height}"
CHECKPOINT_HASH="${CHECKPOINT_HASH:-$derived_checkpoint_hash}"
if [[ -z "$CHECKPOINT_HEIGHT" || -z "$CHECKPOINT_HASH" ]]; then
  echo "error: checkpoint height/hash unavailable from status payload; pass --checkpoint-height and --checkpoint-hash explicitly" >&2
  exit 2
fi
if [[ ! "$CHECKPOINT_HEIGHT" =~ ^[0-9]+$ ]]; then
  echo "error: checkpoint height must be a non-negative integer: $CHECKPOINT_HEIGHT" >&2
  exit 2
fi

# A materialized execution-record directory makes this a recovery-capable
# export. It must therefore prove that the status head, persisted record, and
# persisted world snapshot name the same checkpoint before any recovery
# manifest is written. A best-effort filesystem copy made while the source is
# running cannot establish that identity and must fail closed.
if [[ -n "$EXECUTION_RECORDS_DIR" ]]; then
  latest_record_path="$EXECUTION_RECORDS_DIR/latest.json"
  snapshot_checkpoint_height=$(jq -er '.checkpoint_height // .state.checkpoint_height // empty' "$SNAPSHOT_PATH" 2>/dev/null) \
    || state_sync_checkpoint_mismatch "snapshot missing checkpoint_height"
  record_checkpoint_height=$(jq -er '.height // .last_applied_committed_height // empty' "$latest_record_path" 2>/dev/null) \
    || state_sync_checkpoint_mismatch "execution record missing height"
  record_execution_state_root=$(jq -er '.execution_state_root // .last_execution_state_root // empty' "$latest_record_path" 2>/dev/null) \
    || state_sync_checkpoint_mismatch "execution record missing execution_state_root"
  [[ "$snapshot_checkpoint_height" =~ ^[0-9]+$ ]] \
    || state_sync_checkpoint_mismatch "snapshot checkpoint_height is not numeric: $snapshot_checkpoint_height"
  [[ "$record_checkpoint_height" =~ ^[0-9]+$ ]] \
    || state_sync_checkpoint_mismatch "execution record height is not numeric: $record_checkpoint_height"
  [[ "$snapshot_checkpoint_height" == "$CHECKPOINT_HEIGHT" ]] \
    || state_sync_checkpoint_mismatch "snapshot height=$snapshot_checkpoint_height status height=$CHECKPOINT_HEIGHT"
  [[ "$record_checkpoint_height" == "$CHECKPOINT_HEIGHT" ]] \
    || state_sync_checkpoint_mismatch "execution record height=$record_checkpoint_height status height=$CHECKPOINT_HEIGHT"
  [[ -n "$SNAPSHOT_REF" ]] \
    || state_sync_checkpoint_mismatch "status missing network_head.execution_state_root"
  [[ "$record_execution_state_root" == "$SNAPSHOT_REF" ]] \
    || state_sync_checkpoint_mismatch "execution record state root does not match status head"
fi

VALIDATOR_SET_PATH="$OUT_DIR/validator-set.json"
TRUSTED_CHECKPOINT_PATH="$OUT_DIR/trusted-checkpoint.json"
STATE_SYNC_DIR="$OUT_DIR/state-sync-bundle"
STATE_SYNC_MANIFEST_PATH="$OUT_DIR/state-sync-bundle.json"
mkdir -p "$STATE_SYNC_DIR/snapshots"

SNAPSHOT_BASENAME="checkpoint-${CHECKPOINT_HEIGHT}.json"
SNAPSHOT_EXPORT_PATH="$STATE_SYNC_DIR/snapshots/$SNAPSHOT_BASENAME"
cp "$SNAPSHOT_PATH" "$SNAPSHOT_EXPORT_PATH"

SNAPSHOT_STATE_JSON="$(jq -S -c '.state' "$SNAPSHOT_EXPORT_PATH")" || {
  echo "error: snapshot json missing .state object: $SNAPSHOT_EXPORT_PATH" >&2
  exit 2
}
STATE_ROOT="sha256:$(printf '%s' "$SNAPSHOT_STATE_JSON" | sha256sum | awk '{print $1}')"
SNAPSHOT_SHA256="$(sha256sum "$SNAPSHOT_EXPORT_PATH" | awk '{print $1}')"
SNAPSHOT_REF="$(jq -r '.consensus.network_head.execution_state_root // empty' <<<"$STATUS_CANONICAL_JSON")"

VALIDATOR_SET_JSON="$(jq -e -S -c '
  (.state.governance_finality_signer_registry // empty) as $registry
  | if ($registry | type) != "object" then
      error("snapshot missing state.governance_finality_signer_registry")
    else
      ($registry.signer_bindings // {}) as $bindings
      | ($registry.validator_stakes // {}) as $stakes
      | [($bindings | to_entries[]?) as $entry
          | {
              validator_id: $entry.key,
              public_key_hex: $entry.value,
              stake: (($stakes[$entry.key] // 0) | tonumber)
            }]
      | if length == 0 then
          error("governance_finality_signer_registry has no signer_bindings")
        elif any(.[]; (.validator_id | type) != "string" or (.validator_id | length) == 0) then
          error("validator set contains invalid validator_id")
        elif any(.[]; (.public_key_hex | type) != "string" or (.public_key_hex | length) == 0) then
          error("validator set contains invalid public_key_hex")
        elif any(.[]; .stake <= 0) then
          error("validator set contains non-positive stake")
        else
          sort_by(.validator_id)
        end
    end
' "$SNAPSHOT_EXPORT_PATH")" || exit 2
printf '%s\n' "$VALIDATOR_SET_JSON" | jq '{validators: .}' >"$VALIDATOR_SET_PATH"

VALIDATOR_SET_HASH_JSON="$(jq -S -c '[.[] | {validator_id, stake, public_key_path: null}]' <<<"$VALIDATOR_SET_JSON")"
VALIDATOR_STAKE_JSON="$(jq -S -c '[.[] | {validator_id, stake}]' <<<"$VALIDATOR_SET_JSON")"
VALIDATOR_SET_HASH="sha256:$(printf '%s' "$VALIDATOR_SET_HASH_JSON" | sha256sum | awk '{print $1}')"
VALIDATOR_STAKE_ROOT="sha256:$(printf '%s' "$VALIDATOR_STAKE_JSON" | sha256sum | awk '{print $1}')"
VALIDATOR_STAKES_OBJECT="$(jq -S -c 'map({key: .validator_id, value: .stake}) | from_entries' <<<"$VALIDATOR_SET_JSON")"

jq -n \
  --argjson height "$CHECKPOINT_HEIGHT" \
  --arg block_hash "$CHECKPOINT_HASH" \
  --arg source "unsigned-local-world-export" \
  --arg validator_set_hash "$VALIDATOR_SET_HASH" \
  --arg stake_root "$VALIDATOR_STAKE_ROOT" \
  --argjson validator_stakes "$VALIDATOR_STAKES_OBJECT" \
  '{
    height: $height,
    block_hash: $block_hash,
    source: $source,
    validator_set_hash: $validator_set_hash,
    stake_root: $stake_root,
    min_signatures: 0,
    threshold_bps: 0,
    validator_stakes: $validator_stakes,
    signatures: []
  }' >"$TRUSTED_CHECKPOINT_PATH"

jq -n \
  --argjson checkpoint_height "$CHECKPOINT_HEIGHT" \
  --arg checkpoint_hash "$CHECKPOINT_HASH" \
  --arg snapshot_ref "$SNAPSHOT_REF" \
  --arg state_root "$STATE_ROOT" \
  --arg snapshot_path "snapshots/$SNAPSHOT_BASENAME" \
  --arg snapshot_sha256 "$SNAPSHOT_SHA256" \
  '{
    checkpoint_height: $checkpoint_height,
    checkpoint_hash: $checkpoint_hash,
    snapshot_ref: (if $snapshot_ref == "" then null else $snapshot_ref end),
    state_root: $state_root,
    snapshot_path: $snapshot_path,
    snapshot_sha256: $snapshot_sha256
  }' >"$STATE_SYNC_MANIFEST_PATH"

cat <<EOF
trusted_checkpoint_manifest=$TRUSTED_CHECKPOINT_PATH
validator_set_manifest=$VALIDATOR_SET_PATH
state_sync_bundle_manifest=$STATE_SYNC_MANIFEST_PATH
state_sync_bundle_dir=$STATE_SYNC_DIR
checkpoint_height=$CHECKPOINT_HEIGHT
checkpoint_hash=$CHECKPOINT_HASH
state_root=$STATE_ROOT
EOF
