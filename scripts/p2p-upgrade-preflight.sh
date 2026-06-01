#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/p2p-upgrade-preflight.sh --status-url <url> [--status-url <url> ...]
       [--status-json <path> [--status-json <path> ...]]
       [--trusted-checkpoint-height <height> --trusted-checkpoint-hash <hash>]
       [--trusted-checkpoint-manifest <path>]
       [--verify-trusted-checkpoint-signatures]
       [--validator-set-manifest <path>]
       [--state-sync-bundle-manifest <path>]
       [--state-sync-bundle-dir <dir>]
       [--require-state-sync-bundle]
       [--verify-state-sync-bundle-semantics]
       [--generate-restore-command-plan --node-service-name <name>
       --node-data-dir <dir> --restore-backup-dir <dir>
       --restore-allow-dir <dir> ...]
       [--restore-script-dir <dir>]
       [--execute-restore-scripts]
       [--auto-rollback-on-restore-failure]
       [--recovery-plan-dir <dir>]

Fails before a rolling runtime upgrade when a node is already outside the
smooth-upgrade envelope: not running, replication gap-sync blocked without an
explicit trusted checkpoint fallback, no fresh peer heads while network height
is ahead, or height lag exceeds policy.

When --recovery-plan-dir is provided, the script writes one dry-run recovery
plan JSON per checked node. Plans are operator guidance only; they do not modify
local ledger or execution state.
USAGE
}

declare -a STATUS_URLS=()
declare -a STATUS_JSONS=()
TRUSTED_CHECKPOINT_HEIGHT=""
TRUSTED_CHECKPOINT_HASH=""
TRUSTED_CHECKPOINT_MANIFEST=""
TRUSTED_CHECKPOINT_SOURCE=""
TRUSTED_CHECKPOINT_SIGNATURE_COUNT=""
TRUSTED_CHECKPOINT_UNIQUE_SIGNER_COUNT=""
TRUSTED_CHECKPOINT_MIN_SIGNATURES=""
TRUSTED_CHECKPOINT_VALIDATOR_SET_HASH=""
TRUSTED_CHECKPOINT_STAKE_ROOT=""
TRUSTED_CHECKPOINT_PAYLOAD_SHA256=""
TRUSTED_CHECKPOINT_SIGNATURES_VERIFIED=false
TRUSTED_CHECKPOINT_THRESHOLD_BPS=""
TRUSTED_CHECKPOINT_APPROVED_STAKE=""
TRUSTED_CHECKPOINT_REQUIRED_STAKE=""
TRUSTED_CHECKPOINT_TOTAL_STAKE=""
VALIDATOR_SET_MANIFEST=""
VALIDATOR_SET_HASH=""
VALIDATOR_SET_STAKE_ROOT=""
VALIDATOR_SET_PROOF_VERIFIED=false
STATE_SYNC_BUNDLE_MANIFEST=""
STATE_SYNC_BUNDLE_HEIGHT=""
STATE_SYNC_BUNDLE_CHECKPOINT_HASH=""
STATE_SYNC_BUNDLE_HASH=""
STATE_SYNC_BUNDLE_SNAPSHOT_REF=""
STATE_SYNC_BUNDLE_STATE_ROOT=""
STATE_SYNC_BUNDLE_DIR=""
STATE_SYNC_BUNDLE_SNAPSHOT_PATH=""
STATE_SYNC_BUNDLE_JOURNAL_PATH=""
STATE_SYNC_BUNDLE_SNAPSHOT_SHA256=""
STATE_SYNC_BUNDLE_JOURNAL_SHA256=""
STATE_SYNC_BUNDLE_CHUNK_COUNT=""
STATE_SYNC_BUNDLE_CHUNKS_ROOT=""
STATE_SYNC_BUNDLE_CHUNK_MANIFEST_JSON="[]"
REQUIRE_STATE_SYNC_BUNDLE=false
STATE_SYNC_BUNDLE_SEMANTICS_VERIFIED=false
GENERATE_RESTORE_COMMAND_PLAN=false
NODE_SERVICE_NAME=""
NODE_DATA_DIR=""
RESTORE_BACKUP_DIR=""
declare -a RESTORE_ALLOW_DIRS=()
RESTORE_SCRIPT_DIR=""
EXECUTE_RESTORE_SCRIPTS=false
AUTO_ROLLBACK_ON_RESTORE_FAILURE=false
RECOVERY_PLAN_DIR=""

validate_restore_shell_safe_path() {
  local label="$1"
  local value="$2"
  local require_absolute="${3:-false}"
  if [[ -z "$value" ]]; then
    return 0
  fi
  if [[ "$require_absolute" == "true" && "$value" != /* ]]; then
    echo "error: $label must be an absolute path for restore command generation" >&2
    exit 2
  fi
  if [[ "$value" == *".."* ]]; then
    echo "error: $label contains unsupported parent traversal for restore command generation" >&2
    exit 2
  fi
  if [[ ! "$value" =~ ^[A-Za-z0-9._~:/+=,@%-]+$ ]]; then
    echo "error: $label contains shell-unsafe characters for restore command generation" >&2
    exit 2
  fi
}

validate_restore_shell_safe_token() {
  local label="$1"
  local value="$2"
  if [[ ! "$value" =~ ^[A-Za-z0-9@_.:-]+$ ]]; then
    echo "error: $label contains shell-unsafe characters for restore command generation" >&2
    exit 2
  fi
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --status-url)
      STATUS_URLS+=("${2:-}")
      shift 2
      ;;
    --status-json)
      STATUS_JSONS+=("${2:-}")
      shift 2
      ;;
    --trusted-checkpoint-height)
      TRUSTED_CHECKPOINT_HEIGHT="${2:-}"
      shift 2
      ;;
    --trusted-checkpoint-hash)
      TRUSTED_CHECKPOINT_HASH="${2:-}"
      shift 2
      ;;
    --trusted-checkpoint-manifest)
      TRUSTED_CHECKPOINT_MANIFEST="${2:-}"
      shift 2
      ;;
    --verify-trusted-checkpoint-signatures)
      TRUSTED_CHECKPOINT_SIGNATURES_VERIFIED=true
      shift
      ;;
    --validator-set-manifest)
      VALIDATOR_SET_MANIFEST="${2:-}"
      shift 2
      ;;
    --state-sync-bundle-manifest)
      STATE_SYNC_BUNDLE_MANIFEST="${2:-}"
      shift 2
      ;;
    --state-sync-bundle-dir)
      STATE_SYNC_BUNDLE_DIR="${2:-}"
      shift 2
      ;;
    --require-state-sync-bundle)
      REQUIRE_STATE_SYNC_BUNDLE=true
      shift
      ;;
    --verify-state-sync-bundle-semantics)
      STATE_SYNC_BUNDLE_SEMANTICS_VERIFIED=true
      shift
      ;;
    --generate-restore-command-plan)
      GENERATE_RESTORE_COMMAND_PLAN=true
      shift
      ;;
    --node-service-name)
      NODE_SERVICE_NAME="${2:-}"
      shift 2
      ;;
    --node-data-dir)
      NODE_DATA_DIR="${2:-}"
      shift 2
      ;;
    --restore-backup-dir)
      RESTORE_BACKUP_DIR="${2:-}"
      shift 2
      ;;
    --restore-allow-dir)
      RESTORE_ALLOW_DIRS+=("${2:-}")
      shift 2
      ;;
    --restore-script-dir)
      RESTORE_SCRIPT_DIR="${2:-}"
      shift 2
      ;;
    --execute-restore-scripts)
      EXECUTE_RESTORE_SCRIPTS=true
      shift
      ;;
    --auto-rollback-on-restore-failure)
      AUTO_ROLLBACK_ON_RESTORE_FAILURE=true
      shift
      ;;
    --recovery-plan-dir)
      RECOVERY_PLAN_DIR="${2:-}"
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

if [[ ${#STATUS_URLS[@]} -eq 0 && ${#STATUS_JSONS[@]} -eq 0 ]]; then
  echo "error: at least one --status-url or --status-json is required" >&2
  usage >&2
  exit 2
fi
if [[ -n "$VALIDATOR_SET_MANIFEST" ]]; then
  if [[ ! -f "$VALIDATOR_SET_MANIFEST" ]]; then
    echo "error: --validator-set-manifest file not found: $VALIDATOR_SET_MANIFEST" >&2
    exit 2
  fi
  validator_set_json="$(jq -e -S -c '
    [(.validators // .validator_set // [])[]
      | {
          validator_id: (.validator_id // .node_id // .signer // empty),
          stake: ((.stake // .weight // 0) | tonumber),
          public_key_path: (.public_key_path // .validator_public_key_path // null)
        }]
    | if length == 0 then
        error("validator set manifest missing validators")
      elif any(.[]; (.validator_id | type) != "string" or (.validator_id | length) == 0) then
        error("validator set manifest contains missing validator id")
      elif any(.[]; .stake <= 0) then
        error("validator set manifest contains non-positive stake")
      else
        sort_by(.validator_id)
      end
  ' "$VALIDATOR_SET_MANIFEST")" || exit 2
  validator_set_count="$(jq -r 'length' <<<"$validator_set_json")"
  validator_set_unique_count="$(jq -r '[.[].validator_id] | unique | length' <<<"$validator_set_json")"
  if (( validator_set_count != validator_set_unique_count )); then
    echo "error: validator set manifest contains duplicate validator ids" >&2
    exit 2
  fi
  validator_stake_json="$(jq -S -c '[.[] | {validator_id, stake}]' <<<"$validator_set_json")"
  VALIDATOR_SET_HASH="sha256:$(printf '%s' "$validator_set_json" | sha256sum | awk '{print $1}')"
  VALIDATOR_SET_STAKE_ROOT="sha256:$(printf '%s' "$validator_stake_json" | sha256sum | awk '{print $1}')"
fi
if [[ -n "$TRUSTED_CHECKPOINT_MANIFEST" ]]; then
  if [[ ! -f "$TRUSTED_CHECKPOINT_MANIFEST" ]]; then
    echo "error: --trusted-checkpoint-manifest file not found: $TRUSTED_CHECKPOINT_MANIFEST" >&2
    exit 2
  fi
  manifest_summary="$(jq -e -r '
    def required_string($name; $value):
      if (($value // "") | type) != "string" or (($value // "") | length) == 0 then
        error("trusted checkpoint manifest missing " + $name)
      else
        $value
      end;
    def required_height:
      (.height // .checkpoint_height) as $height
      | if $height == null then
          error("trusted checkpoint manifest missing height")
        elif (($height | tostring) | test("^[0-9]+$") | not) then
          error("trusted checkpoint manifest height must be a non-negative integer")
        else
          ($height | tonumber)
        end;
    {
      height: required_height,
      hash: required_string("block_hash/checkpoint_hash"; (.block_hash // .checkpoint_hash)),
      source: ((.source // .governance_source // "manifest") | tostring),
      validator_set_hash: (.validator_set_hash // null),
      stake_root: (.stake_root // .validator_stake_root // null),
      min_signatures: ((.min_signatures // .min_validator_signatures // 0) | tonumber),
      threshold_bps: ((.threshold_bps // .stake_threshold_bps // 0) | tonumber),
      signature_count: ((.signatures // .validator_signatures // []) | length),
      unique_signer_count: (
        [(.signatures // .validator_signatures // [])[]
          | (.validator_id // .signer // .public_key // .node_id // empty)]
        | unique
        | length
      ),
      signed_stake: (
        [(.signatures // .validator_signatures // [])[]
          | (.validator_id // .signer // .public_key // .node_id // empty)] as $signers
        | [(.validator_stakes // .stakes // {}) | to_entries[]
          | .key as $validator_id
          | select($signers | index($validator_id))
          | (.value | tonumber)]
        | add // 0
      ),
      total_stake: (
        [(.validator_stakes // .stakes // {}) | to_entries[] | (.value | tonumber)]
        | add // 0
      )
    }
    | .required_stake = (
        if .threshold_bps > 0 and .total_stake > 0 then
          ((.total_stake * .threshold_bps + 9999) / 10000 | floor)
        else
          0
        end
      )
    | @base64
  ' "$TRUSTED_CHECKPOINT_MANIFEST")" || exit 2
  manifest_json="$(printf '%s' "$manifest_summary" | base64 -d)"
  manifest_height="$(jq -r '.height' <<<"$manifest_json")"
  manifest_hash="$(jq -r '.hash' <<<"$manifest_json")"
  manifest_signature_count="$(jq -r '.signature_count' <<<"$manifest_json")"
  manifest_unique_signer_count="$(jq -r '.unique_signer_count' <<<"$manifest_json")"
  manifest_min_signatures="$(jq -r '.min_signatures' <<<"$manifest_json")"
  manifest_threshold_bps="$(jq -r '.threshold_bps' <<<"$manifest_json")"
  manifest_signed_stake="$(jq -r '.signed_stake' <<<"$manifest_json")"
  manifest_required_stake="$(jq -r '.required_stake' <<<"$manifest_json")"
  manifest_total_stake="$(jq -r '.total_stake' <<<"$manifest_json")"
  if (( manifest_signature_count != manifest_unique_signer_count )); then
    echo "error: trusted checkpoint manifest contains duplicate or missing validator signer ids" >&2
    exit 2
  fi
  if (( manifest_min_signatures > 0 && manifest_unique_signer_count < manifest_min_signatures )); then
    echo "error: trusted checkpoint manifest unique signer count $manifest_unique_signer_count is below min_signatures $manifest_min_signatures" >&2
    exit 2
  fi
  if (( manifest_threshold_bps > 0 )); then
    if (( manifest_total_stake <= 0 )); then
      echo "error: trusted checkpoint manifest threshold_bps requires validator_stakes" >&2
      exit 2
    fi
    if (( manifest_signed_stake < manifest_required_stake )); then
      echo "error: trusted checkpoint manifest signed stake $manifest_signed_stake is below required stake $manifest_required_stake" >&2
      exit 2
    fi
  fi
  if [[ -n "$TRUSTED_CHECKPOINT_HEIGHT" && "$TRUSTED_CHECKPOINT_HEIGHT" != "$manifest_height" ]]; then
    echo "error: --trusted-checkpoint-height does not match manifest height" >&2
    exit 2
  fi
  if [[ -n "$TRUSTED_CHECKPOINT_HASH" && "$TRUSTED_CHECKPOINT_HASH" != "$manifest_hash" ]]; then
    echo "error: --trusted-checkpoint-hash does not match manifest hash" >&2
    exit 2
  fi
  TRUSTED_CHECKPOINT_HEIGHT="$manifest_height"
  TRUSTED_CHECKPOINT_HASH="$manifest_hash"
  TRUSTED_CHECKPOINT_SOURCE="$(jq -r '.source' <<<"$manifest_json")"
  TRUSTED_CHECKPOINT_SIGNATURE_COUNT="$manifest_signature_count"
  TRUSTED_CHECKPOINT_UNIQUE_SIGNER_COUNT="$manifest_unique_signer_count"
  TRUSTED_CHECKPOINT_MIN_SIGNATURES="$manifest_min_signatures"
  TRUSTED_CHECKPOINT_VALIDATOR_SET_HASH="$(jq -r '.validator_set_hash // empty' <<<"$manifest_json")"
  TRUSTED_CHECKPOINT_STAKE_ROOT="$(jq -r '.stake_root // empty' <<<"$manifest_json")"
  trusted_checkpoint_payload_json="$(jq -S -c '{
    height,
    hash,
    validator_set_hash,
    stake_root
  }' <<<"$manifest_json")"
  TRUSTED_CHECKPOINT_PAYLOAD_SHA256="$(printf '%s' "$trusted_checkpoint_payload_json" | sha256sum | awk '{print $1}')"
  manifest_payload_sha256="$(jq -r '.checkpoint_payload_sha256 // .payload_sha256 // empty' "$TRUSTED_CHECKPOINT_MANIFEST")"
  if [[ -n "$manifest_payload_sha256" && "$manifest_payload_sha256" != "$TRUSTED_CHECKPOINT_PAYLOAD_SHA256" ]]; then
    echo "error: trusted checkpoint manifest payload sha256 does not match canonical checkpoint payload" >&2
    exit 2
  fi
  signature_payload_binding_count="$(jq -r '
    [(.signatures // .validator_signatures // [])[]
      | (.checkpoint_payload_sha256 // .payload_sha256 // .signed_payload_sha256 // empty)]
    | length
  ' "$TRUSTED_CHECKPOINT_MANIFEST")"
  signature_payload_mismatch_count="$(jq -r --arg payload_sha "$TRUSTED_CHECKPOINT_PAYLOAD_SHA256" '
    [(.signatures // .validator_signatures // [])[]
      | (.checkpoint_payload_sha256 // .payload_sha256 // .signed_payload_sha256 // empty)
      | select(. != $payload_sha)]
    | length
  ' "$TRUSTED_CHECKPOINT_MANIFEST")"
  if (( manifest_signature_count > 0 && signature_payload_binding_count != manifest_signature_count )); then
    echo "error: trusted checkpoint signatures must bind checkpoint_payload_sha256" >&2
    exit 2
  fi
  if (( signature_payload_mismatch_count > 0 )); then
    echo "error: trusted checkpoint signature payload sha256 mismatch" >&2
    exit 2
  fi
  if [[ -n "$VALIDATOR_SET_MANIFEST" ]]; then
    if [[ "$TRUSTED_CHECKPOINT_VALIDATOR_SET_HASH" != "$VALIDATOR_SET_HASH" ]]; then
      echo "error: trusted checkpoint validator_set_hash does not match independently computed validator set hash" >&2
      exit 2
    fi
    if [[ "$TRUSTED_CHECKPOINT_STAKE_ROOT" != "$VALIDATOR_SET_STAKE_ROOT" ]]; then
      echo "error: trusted checkpoint stake_root does not match independently computed validator stake root" >&2
      exit 2
    fi
    unknown_checkpoint_signer_count="$(jq -r --argjson validators "$validator_set_json" '
      [(.signatures // .validator_signatures // [])[]
        | (.validator_id // .signer // .public_key // .node_id // empty) as $signer
        | select([$validators[].validator_id] | index($signer) | not)]
      | length
    ' "$TRUSTED_CHECKPOINT_MANIFEST")"
    if (( unknown_checkpoint_signer_count > 0 )); then
      echo "error: trusted checkpoint manifest contains signer outside independently verified validator set" >&2
      exit 2
    fi
    checkpoint_stake_mismatch_count="$(jq -r --argjson validators "$validator_set_json" '
      (.validator_stakes // .stakes // {}) as $stakes
      | [$validators[]
          | .validator_id as $validator_id
          | select(($stakes[$validator_id] // null) != null)
          | select((($stakes[$validator_id] | tonumber) != .stake))]
      | length
    ' "$TRUSTED_CHECKPOINT_MANIFEST")"
    if (( checkpoint_stake_mismatch_count > 0 )); then
      echo "error: trusted checkpoint manifest stake does not match independently verified validator set" >&2
      exit 2
    fi
    VALIDATOR_SET_PROOF_VERIFIED=true
  fi
  if [[ "$TRUSTED_CHECKPOINT_SIGNATURES_VERIFIED" == "true" ]]; then
    if (( manifest_signature_count == 0 )); then
      echo "error: --verify-trusted-checkpoint-signatures requires signatures" >&2
      exit 2
    fi
    command -v openssl >/dev/null 2>&1 || {
      echo "error: --verify-trusted-checkpoint-signatures requires openssl" >&2
      exit 2
    }
    command -v xxd >/dev/null 2>&1 || {
      echo "error: --verify-trusted-checkpoint-signatures requires xxd" >&2
      exit 2
    }
    checkpoint_manifest_dir="$(cd "$(dirname "$TRUSTED_CHECKPOINT_MANIFEST")" && pwd)"
    checkpoint_payload_file="$(mktemp)"
    printf '%s' "$trusted_checkpoint_payload_json" >"$checkpoint_payload_file"
    while IFS= read -r signature_row; do
      signature_json="$(printf '%s' "$signature_row" | base64 -d)"
      signature_validator_id="$(jq -r '.validator_id' <<<"$signature_json")"
      signature_public_key_path="$(jq -r '.public_key_path' <<<"$signature_json")"
      signature_hex="$(jq -r '.signature_hex' <<<"$signature_json")"
      if [[ -z "$signature_public_key_path" || "$signature_public_key_path" == "null" ]]; then
        echo "error: trusted checkpoint signature for $signature_validator_id missing public_key_path" >&2
        rm -f "$checkpoint_payload_file"
        exit 2
      fi
      if [[ -z "$signature_hex" || "$signature_hex" == "null" || ! "$signature_hex" =~ ^[0-9A-Fa-f]+$ ]]; then
        echo "error: trusted checkpoint signature for $signature_validator_id missing hex signature" >&2
        rm -f "$checkpoint_payload_file"
        exit 2
      fi
      signature_public_key_file="$signature_public_key_path"
      if [[ "$signature_public_key_file" != /* ]]; then
        signature_public_key_file="$checkpoint_manifest_dir/$signature_public_key_file"
      fi
      if [[ "$VALIDATOR_SET_PROOF_VERIFIED" == "true" ]]; then
        validator_public_key_path="$(jq -r --arg validator_id "$signature_validator_id" '
          .[]
          | select(.validator_id == $validator_id)
          | .public_key_path // empty
        ' <<<"$validator_set_json")"
        if [[ -z "$validator_public_key_path" ]]; then
          echo "error: independently verified validator set missing public key path for $signature_validator_id" >&2
          rm -f "$checkpoint_payload_file"
          exit 2
        fi
        validator_set_manifest_dir="$(cd "$(dirname "$VALIDATOR_SET_MANIFEST")" && pwd)"
        validator_public_key_file="$validator_public_key_path"
        if [[ "$validator_public_key_file" != /* ]]; then
          validator_public_key_file="$validator_set_manifest_dir/$validator_public_key_file"
        fi
        if [[ "$(realpath -m "$signature_public_key_file")" != "$(realpath -m "$validator_public_key_file")" ]]; then
          echo "error: trusted checkpoint signature public key path does not match independently verified validator set for $signature_validator_id" >&2
          rm -f "$checkpoint_payload_file"
          exit 2
        fi
      fi
      if [[ ! -f "$signature_public_key_file" ]]; then
        echo "error: trusted checkpoint signature public key not found for $signature_validator_id: $signature_public_key_file" >&2
        rm -f "$checkpoint_payload_file"
        exit 2
      fi
      signature_file="$(mktemp)"
      if ! printf '%s' "$signature_hex" | xxd -r -p >"$signature_file"; then
        echo "error: trusted checkpoint signature for $signature_validator_id is not valid hex" >&2
        rm -f "$checkpoint_payload_file" "$signature_file"
        exit 2
      fi
      if ! openssl pkeyutl -verify -rawin -pubin -inkey "$signature_public_key_file" -sigfile "$signature_file" -in "$checkpoint_payload_file" >/dev/null 2>&1; then
        echo "error: trusted checkpoint signature verification failed for $signature_validator_id" >&2
        rm -f "$checkpoint_payload_file" "$signature_file"
        exit 2
      fi
      rm -f "$signature_file"
    done < <(jq -r '
      (.signatures // .validator_signatures // [])[]
      | {
          validator_id: (.validator_id // .signer // .public_key // .node_id // "unknown"),
          public_key_path: (.public_key_path // .validator_public_key_path // empty),
          signature_hex: (.signature_hex // .signature_ed25519_hex // empty)
        }
      | @base64
    ' "$TRUSTED_CHECKPOINT_MANIFEST")
    rm -f "$checkpoint_payload_file"
  fi
  TRUSTED_CHECKPOINT_THRESHOLD_BPS="$manifest_threshold_bps"
  TRUSTED_CHECKPOINT_APPROVED_STAKE="$manifest_signed_stake"
  TRUSTED_CHECKPOINT_REQUIRED_STAKE="$manifest_required_stake"
  TRUSTED_CHECKPOINT_TOTAL_STAKE="$manifest_total_stake"
fi
if [[ "$TRUSTED_CHECKPOINT_SIGNATURES_VERIFIED" == "true" && -z "$TRUSTED_CHECKPOINT_MANIFEST" ]]; then
  echo "error: --verify-trusted-checkpoint-signatures requires --trusted-checkpoint-manifest" >&2
  exit 2
fi
if [[ -n "$VALIDATOR_SET_MANIFEST" && -z "$TRUSTED_CHECKPOINT_MANIFEST" ]]; then
  echo "error: --validator-set-manifest requires --trusted-checkpoint-manifest" >&2
  exit 2
fi
if [[ -n "$TRUSTED_CHECKPOINT_HEIGHT" && ! "$TRUSTED_CHECKPOINT_HEIGHT" =~ ^[0-9]+$ ]]; then
  echo "error: --trusted-checkpoint-height must be a non-negative integer" >&2
  exit 2
fi
if [[ -n "$TRUSTED_CHECKPOINT_HASH" && -z "$TRUSTED_CHECKPOINT_HEIGHT" ]]; then
  echo "error: --trusted-checkpoint-hash requires --trusted-checkpoint-height" >&2
  exit 2
fi
if [[ -n "$TRUSTED_CHECKPOINT_HEIGHT" && -z "$TRUSTED_CHECKPOINT_HASH" ]]; then
  echo "error: --trusted-checkpoint-height requires --trusted-checkpoint-hash" >&2
  exit 2
fi
if [[ -n "$STATE_SYNC_BUNDLE_MANIFEST" ]]; then
  if [[ ! -f "$STATE_SYNC_BUNDLE_MANIFEST" ]]; then
    echo "error: --state-sync-bundle-manifest file not found: $STATE_SYNC_BUNDLE_MANIFEST" >&2
    exit 2
  fi
  bundle_summary="$(jq -e -r '
    def required_string($name; $value):
      if (($value // "") | type) != "string" or (($value // "") | length) == 0 then
        error("state-sync bundle manifest missing " + $name)
      else
        $value
      end;
    def required_height:
      (.checkpoint_height // .height) as $height
      | if $height == null then
          error("state-sync bundle manifest missing checkpoint_height")
        elif (($height | tostring) | test("^[0-9]+$") | not) then
          error("state-sync bundle manifest checkpoint_height must be a non-negative integer")
        else
          ($height | tonumber)
        end;
    {
      checkpoint_height: required_height,
      checkpoint_hash: required_string("checkpoint_hash/block_hash"; (.checkpoint_hash // .block_hash)),
      bundle_hash: (.bundle_hash // .manifest_hash // null),
      snapshot_ref: (.snapshot_ref // .state_snapshot_ref // null),
      state_root: (.state_root // .execution_state_root // null),
      snapshot_path: (.snapshot_path // .snapshot_file // null),
      journal_path: (.journal_path // .journal_file // null),
      snapshot_sha256: (.snapshot_sha256 // .snapshot_hash // null),
      journal_sha256: (.journal_sha256 // .journal_hash // null),
      chunk_count: ((.chunks // .snapshot_chunks // []) | length),
      chunks_root: (.chunks_root // .chunk_root // null)
    }
    | @base64
  ' "$STATE_SYNC_BUNDLE_MANIFEST")" || exit 2
  bundle_json="$(printf '%s' "$bundle_summary" | base64 -d)"
  STATE_SYNC_BUNDLE_HEIGHT="$(jq -r '.checkpoint_height' <<<"$bundle_json")"
  STATE_SYNC_BUNDLE_CHECKPOINT_HASH="$(jq -r '.checkpoint_hash' <<<"$bundle_json")"
  STATE_SYNC_BUNDLE_HASH="$(jq -r '.bundle_hash // empty' <<<"$bundle_json")"
  STATE_SYNC_BUNDLE_SNAPSHOT_REF="$(jq -r '.snapshot_ref // empty' <<<"$bundle_json")"
  STATE_SYNC_BUNDLE_STATE_ROOT="$(jq -r '.state_root // empty' <<<"$bundle_json")"
  STATE_SYNC_BUNDLE_SNAPSHOT_PATH="$(jq -r '.snapshot_path // empty' <<<"$bundle_json")"
  STATE_SYNC_BUNDLE_JOURNAL_PATH="$(jq -r '.journal_path // empty' <<<"$bundle_json")"
  STATE_SYNC_BUNDLE_SNAPSHOT_SHA256="$(jq -r '.snapshot_sha256 // empty' <<<"$bundle_json")"
  STATE_SYNC_BUNDLE_JOURNAL_SHA256="$(jq -r '.journal_sha256 // empty' <<<"$bundle_json")"
  STATE_SYNC_BUNDLE_CHUNK_COUNT="$(jq -r '.chunk_count' <<<"$bundle_json")"
  STATE_SYNC_BUNDLE_CHUNKS_ROOT="$(jq -r '.chunks_root // empty' <<<"$bundle_json")"
  if [[ -z "$TRUSTED_CHECKPOINT_HEIGHT" || -z "$TRUSTED_CHECKPOINT_HASH" ]]; then
    echo "error: --state-sync-bundle-manifest requires trusted checkpoint height/hash or manifest" >&2
    exit 2
  fi
  if [[ "$STATE_SYNC_BUNDLE_HEIGHT" != "$TRUSTED_CHECKPOINT_HEIGHT" ]]; then
    echo "error: state-sync bundle checkpoint height does not match trusted checkpoint height" >&2
    exit 2
  fi
  if [[ "$STATE_SYNC_BUNDLE_CHECKPOINT_HASH" != "$TRUSTED_CHECKPOINT_HASH" ]]; then
    echo "error: state-sync bundle checkpoint hash does not match trusted checkpoint hash" >&2
    exit 2
  fi
fi
if [[ -n "$STATE_SYNC_BUNDLE_DIR" ]]; then
  if [[ ! -d "$STATE_SYNC_BUNDLE_DIR" ]]; then
    echo "error: --state-sync-bundle-dir not found: $STATE_SYNC_BUNDLE_DIR" >&2
    exit 2
  fi
  if [[ -z "$STATE_SYNC_BUNDLE_MANIFEST" ]]; then
    echo "error: --state-sync-bundle-dir requires --state-sync-bundle-manifest" >&2
    exit 2
  fi
  if [[ -z "$STATE_SYNC_BUNDLE_SNAPSHOT_PATH" ]]; then
    echo "error: state-sync bundle manifest missing snapshot_path for bundle dir validation" >&2
    exit 2
  fi
  bundle_snapshot_file="$STATE_SYNC_BUNDLE_DIR/$STATE_SYNC_BUNDLE_SNAPSHOT_PATH"
  if [[ ! -f "$bundle_snapshot_file" ]]; then
    echo "error: state-sync bundle snapshot file not found: $bundle_snapshot_file" >&2
    exit 2
  fi
  if [[ -n "$STATE_SYNC_BUNDLE_JOURNAL_PATH" ]]; then
    bundle_journal_file="$STATE_SYNC_BUNDLE_DIR/$STATE_SYNC_BUNDLE_JOURNAL_PATH"
    if [[ ! -f "$bundle_journal_file" ]]; then
      echo "error: state-sync bundle journal file not found: $bundle_journal_file" >&2
      exit 2
    fi
  fi
  if [[ -n "$STATE_SYNC_BUNDLE_SNAPSHOT_SHA256" ]]; then
    bundle_snapshot_hash="$(sha256sum "$bundle_snapshot_file" | awk '{print $1}')"
    if [[ "$bundle_snapshot_hash" != "$STATE_SYNC_BUNDLE_SNAPSHOT_SHA256" ]]; then
      echo "error: state-sync bundle snapshot hash mismatch" >&2
      exit 2
    fi
  fi
  if [[ -n "$STATE_SYNC_BUNDLE_JOURNAL_PATH" && -n "$STATE_SYNC_BUNDLE_JOURNAL_SHA256" ]]; then
    bundle_journal_hash="$(sha256sum "$bundle_journal_file" | awk '{print $1}')"
    if [[ "$bundle_journal_hash" != "$STATE_SYNC_BUNDLE_JOURNAL_SHA256" ]]; then
      echo "error: state-sync bundle journal hash mismatch" >&2
      exit 2
    fi
  fi
  if (( ${STATE_SYNC_BUNDLE_CHUNK_COUNT:-0} > 0 )); then
    chunk_manifest_json="$(jq -e -S -c '
      [(.chunks // .snapshot_chunks // [])[]
        | {
            path: (.path // .chunk_path // empty),
            sha256: (.sha256 // .hash // empty)
          }]
      | if any(.[]; (.path | length) == 0 or (.sha256 | length) == 0) then
          error("state-sync bundle chunk entries require path and sha256")
        else
          sort_by(.path)
        end
    ' "$STATE_SYNC_BUNDLE_MANIFEST")" || exit 2
    while IFS= read -r chunk_row; do
      chunk_json="$(printf '%s' "$chunk_row" | base64 -d)"
      chunk_path="$(jq -r '.path' <<<"$chunk_json")"
      chunk_sha256="$(jq -r '.sha256' <<<"$chunk_json")"
      chunk_file="$STATE_SYNC_BUNDLE_DIR/$chunk_path"
      if [[ ! -f "$chunk_file" ]]; then
        echo "error: state-sync bundle chunk file not found: $chunk_file" >&2
        exit 2
      fi
      actual_chunk_sha256="$(sha256sum "$chunk_file" | awk '{print $1}')"
      if [[ "$actual_chunk_sha256" != "$chunk_sha256" ]]; then
        echo "error: state-sync bundle chunk hash mismatch: $chunk_path" >&2
        exit 2
      fi
    done < <(jq -r '.[] | @base64' <<<"$chunk_manifest_json")
    computed_chunks_root="sha256:$(printf '%s' "$chunk_manifest_json" | sha256sum | awk '{print $1}')"
    if [[ -n "$STATE_SYNC_BUNDLE_CHUNKS_ROOT" && "$computed_chunks_root" != "$STATE_SYNC_BUNDLE_CHUNKS_ROOT" ]]; then
      echo "error: state-sync bundle chunks root mismatch" >&2
      exit 2
    fi
    STATE_SYNC_BUNDLE_CHUNKS_ROOT="$computed_chunks_root"
    STATE_SYNC_BUNDLE_CHUNK_MANIFEST_JSON="$chunk_manifest_json"
  fi
  if [[ "$STATE_SYNC_BUNDLE_SEMANTICS_VERIFIED" == "true" ]]; then
    snapshot_state_json="$(jq -e -S -c '.state' "$bundle_snapshot_file")" || {
      echo "error: state-sync bundle snapshot semantic validation requires JSON object with state" >&2
      exit 2
    }
    snapshot_state_root="sha256:$(printf '%s' "$snapshot_state_json" | sha256sum | awk '{print $1}')"
    if [[ "$snapshot_state_root" != "$STATE_SYNC_BUNDLE_STATE_ROOT" ]]; then
      echo "error: state-sync bundle snapshot state root mismatch" >&2
      exit 2
    fi
    if [[ -n "$STATE_SYNC_BUNDLE_JOURNAL_PATH" ]]; then
      journal_checkpoint_height="$(jq -e -r '.checkpoint_height' "$bundle_journal_file")" || {
        echo "error: state-sync bundle journal semantic validation requires checkpoint_height" >&2
        exit 2
      }
      journal_checkpoint_hash="$(jq -e -r '.checkpoint_hash' "$bundle_journal_file")" || {
        echo "error: state-sync bundle journal semantic validation requires checkpoint_hash" >&2
        exit 2
      }
      journal_state_root="$(jq -e -r '.state_root' "$bundle_journal_file")" || {
        echo "error: state-sync bundle journal semantic validation requires state_root" >&2
        exit 2
      }
      if [[ "$journal_checkpoint_height" != "$STATE_SYNC_BUNDLE_HEIGHT" ]]; then
        echo "error: state-sync bundle journal checkpoint height mismatch" >&2
        exit 2
      fi
      if [[ "$journal_checkpoint_hash" != "$STATE_SYNC_BUNDLE_CHECKPOINT_HASH" ]]; then
        echo "error: state-sync bundle journal checkpoint hash mismatch" >&2
        exit 2
      fi
      if [[ "$journal_state_root" != "$STATE_SYNC_BUNDLE_STATE_ROOT" ]]; then
        echo "error: state-sync bundle journal state root mismatch" >&2
        exit 2
      fi
    fi
  fi
fi
if [[ "$STATE_SYNC_BUNDLE_SEMANTICS_VERIFIED" == "true" && -z "$STATE_SYNC_BUNDLE_DIR" ]]; then
  echo "error: --verify-state-sync-bundle-semantics requires --state-sync-bundle-dir" >&2
  exit 2
fi
if [[ "$GENERATE_RESTORE_COMMAND_PLAN" == "true" ]]; then
  if [[ -z "$RECOVERY_PLAN_DIR" ]]; then
    echo "error: --generate-restore-command-plan requires --recovery-plan-dir" >&2
    exit 2
  fi
  if [[ -z "$NODE_SERVICE_NAME" || -z "$NODE_DATA_DIR" || -z "$RESTORE_BACKUP_DIR" ]]; then
    echo "error: --generate-restore-command-plan requires --node-service-name, --node-data-dir, and --restore-backup-dir" >&2
    exit 2
  fi
  if [[ -z "$STATE_SYNC_BUNDLE_DIR" || -z "$STATE_SYNC_BUNDLE_SNAPSHOT_PATH" ]]; then
    echo "error: --generate-restore-command-plan requires verified state-sync bundle dir and snapshot path" >&2
    exit 2
  fi
  if [[ ${#RESTORE_ALLOW_DIRS[@]} -eq 0 ]]; then
    echo "error: --generate-restore-command-plan requires at least one --restore-allow-dir" >&2
    exit 2
  fi
  node_data_realpath="$(realpath -m "$NODE_DATA_DIR")"
  restore_backup_realpath="$(realpath -m "$RESTORE_BACKUP_DIR")"
  state_sync_bundle_realpath="$(realpath -m "$STATE_SYNC_BUNDLE_DIR")"
  validate_restore_shell_safe_token "--node-service-name" "$NODE_SERVICE_NAME"
  validate_restore_shell_safe_path "--node-data-dir" "$node_data_realpath" true
  validate_restore_shell_safe_path "--restore-backup-dir" "$restore_backup_realpath" true
  validate_restore_shell_safe_path "--state-sync-bundle-dir" "$state_sync_bundle_realpath" true
  validate_restore_shell_safe_path "state-sync bundle snapshot path" "$STATE_SYNC_BUNDLE_SNAPSHOT_PATH" false
  validate_restore_shell_safe_path "state-sync bundle journal path" "$STATE_SYNC_BUNDLE_JOURNAL_PATH" false
  while IFS= read -r chunk_path; do
    validate_restore_shell_safe_path "state-sync bundle chunk path" "$chunk_path" false
  done < <(jq -r '.[].path' <<<"$STATE_SYNC_BUNDLE_CHUNK_MANIFEST_JSON")
  node_data_path_allowed=false
  restore_backup_path_allowed=false
  for restore_allow_dir in "${RESTORE_ALLOW_DIRS[@]}"; do
    restore_allow_realpath="$(realpath -m "$restore_allow_dir")"
    if [[ "$node_data_realpath" == "$restore_allow_realpath" || "$node_data_realpath" == "$restore_allow_realpath"/* ]]; then
      node_data_path_allowed=true
    fi
    if [[ "$restore_backup_realpath" == "$restore_allow_realpath" || "$restore_backup_realpath" == "$restore_allow_realpath"/* ]]; then
      restore_backup_path_allowed=true
    fi
  done
  if [[ "$node_data_path_allowed" != "true" || "$restore_backup_path_allowed" != "true" ]]; then
    echo "error: restore command plan paths are outside --restore-allow-dir roots" >&2
    exit 2
  fi
  RESTORE_ALLOW_DIRS_JSON="$(printf '%s\n' "${RESTORE_ALLOW_DIRS[@]}" | jq -R -s -c 'split("\n") | map(select(length > 0))')"
  NODE_DATA_DIR="$node_data_realpath"
  RESTORE_BACKUP_DIR="$restore_backup_realpath"
  STATE_SYNC_BUNDLE_DIR="$state_sync_bundle_realpath"
else
  RESTORE_ALLOW_DIRS_JSON="[]"
fi
if [[ -n "$RESTORE_SCRIPT_DIR" && "$GENERATE_RESTORE_COMMAND_PLAN" != "true" ]]; then
  echo "error: --restore-script-dir requires --generate-restore-command-plan" >&2
  exit 2
fi
if [[ "$EXECUTE_RESTORE_SCRIPTS" == "true" ]]; then
  if [[ -z "$RESTORE_SCRIPT_DIR" ]]; then
    echo "error: --execute-restore-scripts requires --restore-script-dir" >&2
    exit 2
  fi
  if [[ "${OASIS7_ALLOW_RESTORE_EXECUTION:-}" != "I_UNDERSTAND_THIS_CAN_REPLACE_NODE_STATE" ]]; then
    echo "error: --execute-restore-scripts requires OASIS7_ALLOW_RESTORE_EXECUTION=I_UNDERSTAND_THIS_CAN_REPLACE_NODE_STATE" >&2
    exit 2
  fi
fi
if [[ "$AUTO_ROLLBACK_ON_RESTORE_FAILURE" == "true" && "$EXECUTE_RESTORE_SCRIPTS" != "true" ]]; then
  echo "error: --auto-rollback-on-restore-failure requires --execute-restore-scripts" >&2
  exit 2
fi
if [[ -n "$RECOVERY_PLAN_DIR" ]]; then
  mkdir -p "$RECOVERY_PLAN_DIR"
fi
if [[ -n "$RESTORE_SCRIPT_DIR" ]]; then
  mkdir -p "$RESTORE_SCRIPT_DIR"
fi

overall=0
if [[ ${#STATUS_JSONS[@]} -gt 0 && ${#STATUS_URLS[@]} -gt 0 && ${#STATUS_JSONS[@]} -ne ${#STATUS_URLS[@]} ]]; then
  echo "error: --status-url and --status-json counts must match when both are provided" >&2
  exit 2
fi
status_count=${#STATUS_URLS[@]}
if [[ ${#STATUS_JSONS[@]} -gt 0 && ${#STATUS_URLS[@]} -eq 0 ]]; then
  status_count=${#STATUS_JSONS[@]}
fi
for ((status_index = 0; status_index < status_count; status_index++)); do
  url=""
  status_json_path=""
  if [[ ${#STATUS_URLS[@]} -gt 0 ]]; then
    url="${STATUS_URLS[$status_index]}"
  fi
  if [[ ${#STATUS_JSONS[@]} -gt 0 ]]; then
    status_json_path="${STATUS_JSONS[$status_index]}"
    if [[ ! -f "$status_json_path" ]]; then
      echo "FAIL $status_json_path status_json_missing"
      overall=1
      continue
    fi
    status_json="$(cat "$status_json_path")"
  else
    status_json="$(curl -fsS "$url")" || {
      echo "FAIL $url status_fetch_failed"
      overall=1
      continue
    }
  fi
  status_label="${url:-$status_json_path}"
  summary="$(jq -r \
    --argjson trusted_checkpoint_height "${TRUSTED_CHECKPOINT_HEIGHT:-null}" \
    --arg trusted_checkpoint_hash "$TRUSTED_CHECKPOINT_HASH" \
    --arg trusted_checkpoint_manifest "$TRUSTED_CHECKPOINT_MANIFEST" \
    --arg trusted_checkpoint_source "$TRUSTED_CHECKPOINT_SOURCE" \
    --argjson trusted_checkpoint_signature_count "${TRUSTED_CHECKPOINT_SIGNATURE_COUNT:-null}" \
    --argjson trusted_checkpoint_unique_signer_count "${TRUSTED_CHECKPOINT_UNIQUE_SIGNER_COUNT:-null}" \
    --argjson trusted_checkpoint_min_signatures "${TRUSTED_CHECKPOINT_MIN_SIGNATURES:-null}" \
    --arg trusted_checkpoint_validator_set_hash "$TRUSTED_CHECKPOINT_VALIDATOR_SET_HASH" \
    --arg trusted_checkpoint_stake_root "$TRUSTED_CHECKPOINT_STAKE_ROOT" \
    --arg trusted_checkpoint_payload_sha256 "$TRUSTED_CHECKPOINT_PAYLOAD_SHA256" \
    --argjson trusted_checkpoint_signatures_verified "$TRUSTED_CHECKPOINT_SIGNATURES_VERIFIED" \
    --argjson trusted_checkpoint_threshold_bps "${TRUSTED_CHECKPOINT_THRESHOLD_BPS:-null}" \
    --argjson trusted_checkpoint_approved_stake "${TRUSTED_CHECKPOINT_APPROVED_STAKE:-null}" \
    --argjson trusted_checkpoint_required_stake "${TRUSTED_CHECKPOINT_REQUIRED_STAKE:-null}" \
    --argjson trusted_checkpoint_total_stake "${TRUSTED_CHECKPOINT_TOTAL_STAKE:-null}" \
    --arg validator_set_manifest "$VALIDATOR_SET_MANIFEST" \
    --arg validator_set_hash "$VALIDATOR_SET_HASH" \
    --arg validator_set_stake_root "$VALIDATOR_SET_STAKE_ROOT" \
    --argjson validator_set_proof_verified "$VALIDATOR_SET_PROOF_VERIFIED" \
    --arg state_sync_bundle_manifest "$STATE_SYNC_BUNDLE_MANIFEST" \
    --argjson state_sync_bundle_height "${STATE_SYNC_BUNDLE_HEIGHT:-null}" \
    --arg state_sync_bundle_checkpoint_hash "$STATE_SYNC_BUNDLE_CHECKPOINT_HASH" \
    --arg state_sync_bundle_hash "$STATE_SYNC_BUNDLE_HASH" \
    --arg state_sync_bundle_snapshot_ref "$STATE_SYNC_BUNDLE_SNAPSHOT_REF" \
    --arg state_sync_bundle_state_root "$STATE_SYNC_BUNDLE_STATE_ROOT" \
    --arg state_sync_bundle_dir "$STATE_SYNC_BUNDLE_DIR" \
    --arg state_sync_bundle_snapshot_path "$STATE_SYNC_BUNDLE_SNAPSHOT_PATH" \
    --arg state_sync_bundle_journal_path "$STATE_SYNC_BUNDLE_JOURNAL_PATH" \
    --arg state_sync_bundle_snapshot_sha256 "$STATE_SYNC_BUNDLE_SNAPSHOT_SHA256" \
    --arg state_sync_bundle_journal_sha256 "$STATE_SYNC_BUNDLE_JOURNAL_SHA256" \
    --argjson state_sync_bundle_chunk_count "${STATE_SYNC_BUNDLE_CHUNK_COUNT:-null}" \
    --arg state_sync_bundle_chunks_root "$STATE_SYNC_BUNDLE_CHUNKS_ROOT" \
    --argjson state_sync_bundle_chunk_manifest "$STATE_SYNC_BUNDLE_CHUNK_MANIFEST_JSON" \
    --argjson require_state_sync_bundle "$REQUIRE_STATE_SYNC_BUNDLE" \
    --argjson state_sync_bundle_semantics_verified "$STATE_SYNC_BUNDLE_SEMANTICS_VERIFIED" \
    --argjson generate_restore_command_plan "$GENERATE_RESTORE_COMMAND_PLAN" \
    --arg node_service_name "$NODE_SERVICE_NAME" \
    --arg node_data_dir "$NODE_DATA_DIR" \
    --arg restore_backup_dir "$RESTORE_BACKUP_DIR" \
    --argjson restore_allow_dirs "$RESTORE_ALLOW_DIRS_JSON" '
    def n($v): ($v // 0 | tonumber);
    . as $root
    | {
        node_id: (.node_id // "unknown"),
        running: (.running // false),
        readiness: (.readiness.status // "unknown"),
        failed_gates: (.readiness.failed_gates // []),
        committed_height: n(.consensus.committed_height),
        network_committed_height: n(.consensus.network_committed_height),
        replication_persisted_height: n(.consensus.replication_persisted_height),
        gap_blocked_height: (.consensus.replication_gap_sync_blocked_height // null),
        gap_blocked_reason: (.consensus.replication_gap_sync_blocked_reason // null),
        repair_attempt: (.consensus.replication_gap_sync_repair_attempt_summary // null),
        fallback_required: (.consensus.state_sync_fallback_required // false),
        fallback_snapshot_available: (.consensus.state_sync_snapshot_available // false),
        fallback_required_height: (.consensus.state_sync_trusted_checkpoint_required_height // null),
        fallback_reason: (.consensus.state_sync_fallback_reason // null),
        trusted_checkpoint_height: $trusted_checkpoint_height,
        trusted_checkpoint_hash: (if $trusted_checkpoint_hash == "" then null else $trusted_checkpoint_hash end),
        trusted_checkpoint_manifest: (if $trusted_checkpoint_manifest == "" then null else $trusted_checkpoint_manifest end),
        trusted_checkpoint_source: (if $trusted_checkpoint_source == "" then null else $trusted_checkpoint_source end),
        trusted_checkpoint_signature_count: $trusted_checkpoint_signature_count,
        trusted_checkpoint_unique_signer_count: $trusted_checkpoint_unique_signer_count,
        trusted_checkpoint_min_signatures: $trusted_checkpoint_min_signatures,
        trusted_checkpoint_validator_set_hash: (if $trusted_checkpoint_validator_set_hash == "" then null else $trusted_checkpoint_validator_set_hash end),
        trusted_checkpoint_stake_root: (if $trusted_checkpoint_stake_root == "" then null else $trusted_checkpoint_stake_root end),
        trusted_checkpoint_payload_sha256: (if $trusted_checkpoint_payload_sha256 == "" then null else $trusted_checkpoint_payload_sha256 end),
        trusted_checkpoint_signatures_verified: $trusted_checkpoint_signatures_verified,
        trusted_checkpoint_threshold_bps: $trusted_checkpoint_threshold_bps,
        trusted_checkpoint_approved_stake: $trusted_checkpoint_approved_stake,
        trusted_checkpoint_required_stake: $trusted_checkpoint_required_stake,
        trusted_checkpoint_total_stake: $trusted_checkpoint_total_stake,
        validator_set_manifest: (if $validator_set_manifest == "" then null else $validator_set_manifest end),
        validator_set_hash: (if $validator_set_hash == "" then null else $validator_set_hash end),
        validator_set_stake_root: (if $validator_set_stake_root == "" then null else $validator_set_stake_root end),
        validator_set_proof_verified: $validator_set_proof_verified,
        state_sync_bundle_manifest: (if $state_sync_bundle_manifest == "" then null else $state_sync_bundle_manifest end),
        state_sync_bundle_height: $state_sync_bundle_height,
        state_sync_bundle_checkpoint_hash: (if $state_sync_bundle_checkpoint_hash == "" then null else $state_sync_bundle_checkpoint_hash end),
        state_sync_bundle_hash: (if $state_sync_bundle_hash == "" then null else $state_sync_bundle_hash end),
        state_sync_bundle_snapshot_ref: (if $state_sync_bundle_snapshot_ref == "" then null else $state_sync_bundle_snapshot_ref end),
        state_sync_bundle_state_root: (if $state_sync_bundle_state_root == "" then null else $state_sync_bundle_state_root end),
        state_sync_bundle_dir: (if $state_sync_bundle_dir == "" then null else $state_sync_bundle_dir end),
        state_sync_bundle_snapshot_path: (if $state_sync_bundle_snapshot_path == "" then null else $state_sync_bundle_snapshot_path end),
        state_sync_bundle_journal_path: (if $state_sync_bundle_journal_path == "" then null else $state_sync_bundle_journal_path end),
        state_sync_bundle_snapshot_sha256: (if $state_sync_bundle_snapshot_sha256 == "" then null else $state_sync_bundle_snapshot_sha256 end),
        state_sync_bundle_journal_sha256: (if $state_sync_bundle_journal_sha256 == "" then null else $state_sync_bundle_journal_sha256 end),
        state_sync_bundle_chunk_count: $state_sync_bundle_chunk_count,
        state_sync_bundle_chunks_root: (if $state_sync_bundle_chunks_root == "" then null else $state_sync_bundle_chunks_root end),
        state_sync_bundle_chunk_manifest: $state_sync_bundle_chunk_manifest,
        require_state_sync_bundle: $require_state_sync_bundle,
        state_sync_bundle_semantics_verified: $state_sync_bundle_semantics_verified,
        generate_restore_command_plan: $generate_restore_command_plan,
        node_service_name: (if $node_service_name == "" then null else $node_service_name end),
        node_data_dir: (if $node_data_dir == "" then null else $node_data_dir end),
        restore_backup_dir: (if $restore_backup_dir == "" then null else $restore_backup_dir end),
        restore_allow_dirs: $restore_allow_dirs,
        known_peer_heads: n(.consensus.known_peer_heads),
        fresh_peer_count: n(.consensus.network_head.fresh_peer_count),
        allowed_lag: n(.readiness.policy.max_network_height_lag),
        lag: n(.sync.network_height_lag)
      }
    | .checkpoint_required_height = (
        if .fallback_required_height != null then
          .fallback_required_height
        elif .gap_blocked_height != null then
          ([.gap_blocked_height, .network_committed_height] | max)
        else
          null
        end
      )
    | .trusted_checkpoint_usable = (
        .trusted_checkpoint_height != null
        and .trusted_checkpoint_hash != null
        and .checkpoint_required_height != null
        and .trusted_checkpoint_height >= .checkpoint_required_height
        and (
          .network_committed_height == 0
          or .trusted_checkpoint_height <= .network_committed_height
        )
      )
    | .state_sync_bundle_ready = (
        .state_sync_bundle_manifest != null
        and .state_sync_bundle_dir != null
        and .state_sync_bundle_snapshot_path != null
        and .state_sync_bundle_snapshot_sha256 != null
        and .state_sync_bundle_state_root != null
        and (
          .state_sync_bundle_journal_path == null
          or .state_sync_bundle_journal_sha256 != null
        )
      )
    | .failures = (
        []
        + (if .running then [] else ["not_running"] end)
        + (if (.gap_blocked_height != null and (.trusted_checkpoint_usable | not)) then ["replication_gap_sync_blocked"] else [] end)
        + (if (.fallback_required and (.trusted_checkpoint_usable | not)) then ["state_sync_fallback_checkpoint_unavailable"] else [] end)
        + (if (.fallback_required and .trusted_checkpoint_usable and .require_state_sync_bundle and (.state_sync_bundle_ready | not)) then ["state_sync_bundle_required"] else [] end)
        + (if (.network_committed_height > .replication_persisted_height and .known_peer_heads == 0 and (.trusted_checkpoint_usable | not)) then ["peer_head_unavailable_for_repair"] else [] end)
        + (if (.lag > .allowed_lag and (.trusted_checkpoint_usable | not)) then ["network_height_lag_exceeds_policy"] else [] end)
      )
    | .warnings = (
        []
        + (if (.trusted_checkpoint_usable and ((.require_state_sync_bundle | not) or .state_sync_bundle_ready)) then ["trusted_checkpoint_state_sync_fallback_required"] else [] end)
      )
    | .recovery_plan = {
        dry_run_only: true,
        mode: (
          if (.trusted_checkpoint_usable and .require_state_sync_bundle and (.state_sync_bundle_ready | not)) then "blocked_missing_state_sync_bundle"
          elif .trusted_checkpoint_usable then "trusted_checkpoint_state_sync"
          elif .checkpoint_required_height != null then "blocked_missing_trusted_checkpoint"
          else "not_required"
          end
        ),
        required_height: .checkpoint_required_height,
        trusted_checkpoint_height: .trusted_checkpoint_height,
        trusted_checkpoint_hash: .trusted_checkpoint_hash,
        trusted_checkpoint_manifest: .trusted_checkpoint_manifest,
        trusted_checkpoint_source: .trusted_checkpoint_source,
        trusted_checkpoint_signature_count: .trusted_checkpoint_signature_count,
        trusted_checkpoint_unique_signer_count: .trusted_checkpoint_unique_signer_count,
        trusted_checkpoint_min_signatures: .trusted_checkpoint_min_signatures,
        trusted_checkpoint_validator_set_hash: .trusted_checkpoint_validator_set_hash,
        trusted_checkpoint_stake_root: .trusted_checkpoint_stake_root,
        trusted_checkpoint_payload_sha256: .trusted_checkpoint_payload_sha256,
        trusted_checkpoint_signatures_verified: .trusted_checkpoint_signatures_verified,
        trusted_checkpoint_threshold_bps: .trusted_checkpoint_threshold_bps,
        trusted_checkpoint_approved_stake: .trusted_checkpoint_approved_stake,
        trusted_checkpoint_required_stake: .trusted_checkpoint_required_stake,
        trusted_checkpoint_total_stake: .trusted_checkpoint_total_stake,
        validator_set_manifest: .validator_set_manifest,
        validator_set_hash: .validator_set_hash,
        validator_set_stake_root: .validator_set_stake_root,
        validator_set_proof_verified: .validator_set_proof_verified,
        state_sync_bundle_manifest: .state_sync_bundle_manifest,
        state_sync_bundle_height: .state_sync_bundle_height,
        state_sync_bundle_checkpoint_hash: .state_sync_bundle_checkpoint_hash,
        state_sync_bundle_hash: .state_sync_bundle_hash,
        state_sync_bundle_snapshot_ref: .state_sync_bundle_snapshot_ref,
        state_sync_bundle_state_root: .state_sync_bundle_state_root,
        state_sync_bundle_dir: .state_sync_bundle_dir,
        state_sync_bundle_snapshot_path: .state_sync_bundle_snapshot_path,
        state_sync_bundle_journal_path: .state_sync_bundle_journal_path,
        state_sync_bundle_snapshot_sha256: .state_sync_bundle_snapshot_sha256,
        state_sync_bundle_journal_sha256: .state_sync_bundle_journal_sha256,
        state_sync_bundle_chunk_count: .state_sync_bundle_chunk_count,
        state_sync_bundle_chunks_root: .state_sync_bundle_chunks_root,
        state_sync_bundle_chunk_manifest: .state_sync_bundle_chunk_manifest,
        state_sync_bundle_ready: .state_sync_bundle_ready,
        require_state_sync_bundle: .require_state_sync_bundle,
        state_sync_bundle_semantics_verified: .state_sync_bundle_semantics_verified,
        restore_command_plan_enabled: .generate_restore_command_plan,
        node_service_name: .node_service_name,
        node_data_dir: .node_data_dir,
        restore_backup_dir: .restore_backup_dir,
        restore_allow_dirs: .restore_allow_dirs,
        restore_command_plan: (
          if (.generate_restore_command_plan and .trusted_checkpoint_usable and .state_sync_bundle_ready) then
            ("/" + (.node_id | gsub("[^A-Za-z0-9_.-]"; "_"))) as $node_suffix
            | [
                {
                  step: "verify_restore_toolchain",
                  command: "for tool in systemctl rsync sha256sum cmp find sort cut; do command -v \"$tool\" >/dev/null; done"
                },
                {
                  step: "verify_state_sync_snapshot_sha256",
                  command: ("printf \"%s  %s\\n\" " + .state_sync_bundle_snapshot_sha256 + " " + .state_sync_bundle_dir + "/" + .state_sync_bundle_snapshot_path + " | sha256sum -c -")
                }
              ]
              + (if .state_sync_bundle_journal_path != null then [
                  {
                    step: "verify_state_sync_journal_sha256",
                    command: ("printf \"%s  %s\\n\" " + .state_sync_bundle_journal_sha256 + " " + .state_sync_bundle_dir + "/" + .state_sync_bundle_journal_path + " | sha256sum -c -")
                  }
                ] else [] end)
              + (if (.state_sync_bundle_chunk_manifest | length) > 0 then
                  .state_sync_bundle_dir as $bundle_dir
                  | .state_sync_bundle_chunks_root as $chunks_root
                  | .state_sync_bundle_chunk_manifest as $chunk_manifest
                  | ($chunk_manifest
                    | map({
                        step: "verify_state_sync_chunk_sha256",
                        command: ("printf \"%s  %s\\n\" " + .sha256 + " " + $bundle_dir + "/" + .path + " | sha256sum -c -")
                      }))
                  + [
                    {
                      step: "verify_state_sync_chunks_root",
                      command: ("computed_chunks_root=\"sha256:$(printf %s " + ($chunk_manifest | @json | @sh) + " | sha256sum | cut -d \" \" -f 1)\"; test \"$computed_chunks_root\" = " + ($chunks_root | @sh))
                    }
                  ]
                else [] end)
              + [
                {
                  step: "write_service_show_snapshot",
                  command: ("mkdir -p " + .restore_backup_dir + $node_suffix + " && systemctl show " + .node_service_name + " > " + .restore_backup_dir + $node_suffix + "/service-before-state-sync.systemctl-show.txt")
                },
                {
                  step: "write_service_status_snapshot",
                  command: ("mkdir -p " + .restore_backup_dir + $node_suffix + " && systemctl status " + .node_service_name + " > " + .restore_backup_dir + $node_suffix + "/service-before-state-sync.systemctl-status.txt || true")
                },
                {
                  step: "stop_service",
                  command: ("systemctl stop " + .node_service_name)
                },
                {
                  step: "write_source_sha256_manifest",
                  command: ("mkdir -p " + .restore_backup_dir + $node_suffix + " && cd " + .node_data_dir + " && find . -type f -print | sort | while read -r f; do sha256sum \"$f\"; done > " + .restore_backup_dir + $node_suffix + "/data-before-state-sync.source.sha256")
                },
                {
                  step: "write_source_metadata_manifest",
                  command: ("mkdir -p " + .restore_backup_dir + $node_suffix + " && cd " + .node_data_dir + " && find . -mindepth 1 -printf \"%P\\t%y\\t%s\\t%m\\t%U\\t%G\\n\" | sort > " + .restore_backup_dir + $node_suffix + "/data-before-state-sync.source.metadata.tsv")
                },
                {
                  step: "backup_data_dir",
                  command: ("rsync -a --delete " + .node_data_dir + "/ " + .restore_backup_dir + $node_suffix + "/data-before-state-sync/")
                },
                {
                  step: "write_backup_sha256_manifest",
                  command: ("cd " + .restore_backup_dir + $node_suffix + "/data-before-state-sync && find . -type f -print | sort | while read -r f; do sha256sum \"$f\"; done > ../data-before-state-sync.sha256")
                },
                {
                  step: "write_backup_metadata_manifest",
                  command: ("cd " + .restore_backup_dir + $node_suffix + "/data-before-state-sync && find . -mindepth 1 -printf \"%P\\t%y\\t%s\\t%m\\t%U\\t%G\\n\" | sort > ../data-before-state-sync.metadata.tsv")
                },
                {
                  step: "verify_backup_sha256_manifest",
                  command: ("cmp " + .restore_backup_dir + $node_suffix + "/data-before-state-sync.source.sha256 " + .restore_backup_dir + $node_suffix + "/data-before-state-sync.sha256")
                },
                {
                  step: "verify_backup_metadata_manifest",
                  command: ("cmp " + .restore_backup_dir + $node_suffix + "/data-before-state-sync.source.metadata.tsv " + .restore_backup_dir + $node_suffix + "/data-before-state-sync.metadata.tsv")
                },
                {
                  step: "restore_snapshot",
                  command: ("mkdir -p " + .node_data_dir + "/state-sync && rsync -a " + .state_sync_bundle_dir + "/" + .state_sync_bundle_snapshot_path + " " + .node_data_dir + "/state-sync/snapshot")
                }
              ]
              + (if .state_sync_bundle_journal_path != null then [
                  {
                    step: "restore_journal",
                    command: ("rsync -a " + .state_sync_bundle_dir + "/" + .state_sync_bundle_journal_path + " " + .node_data_dir + "/state-sync/journal")
                  }
                ] else [] end)
              + [
                {
                  step: "start_service",
                  command: ("systemctl start " + .node_service_name)
                },
                {
                  step: "post_restore_preflight",
                  command: "rerun scripts/p2p-upgrade-preflight.sh without checkpoint or restore-plan overrides and require PASS before resuming rollout"
                }
              ]
          else
            []
          end
        ),
        rollback_command_plan: (
          if (.generate_restore_command_plan and .trusted_checkpoint_usable and .state_sync_bundle_ready) then
            ("/" + (.node_id | gsub("[^A-Za-z0-9_.-]"; "_"))) as $node_suffix
            | [
                {
                  step: "stop_service",
                  command: ("systemctl stop " + .node_service_name)
                },
                {
                  step: "restore_data_backup",
                  command: ("rsync -a --delete " + .restore_backup_dir + $node_suffix + "/data-before-state-sync/ " + .node_data_dir + "/")
                },
                {
                  step: "start_service",
                  command: ("systemctl start " + .node_service_name)
                },
                {
                  step: "post_rollback_preflight",
                  command: "rerun scripts/p2p-upgrade-preflight.sh and keep rollout paused until status is understood"
                }
              ]
          else
            []
          end
        ),
        snapshot_available: .fallback_snapshot_available,
        blocked_reasons: .failures,
        operator_steps: (
          if (.trusted_checkpoint_usable and .require_state_sync_bundle and (.state_sync_bundle_ready | not)) then [
            "obtain verified state-sync bundle manifest bound to the trusted checkpoint",
            "stage local state-sync bundle directory with snapshot and optional journal files",
            "rerun p2p-upgrade-preflight with --state-sync-bundle-manifest and --state-sync-bundle-dir"
          ]
          elif .trusted_checkpoint_usable then [
            "stop node process after taking service manager state snapshot",
            "backup execution world directory and replication data directory",
            "verify trusted checkpoint hash and height against governance-approved source",
            "restore verified snapshot/state-sync bundle for checkpoint height",
            "restart node with local consensus participation still held until status is healthy",
            "rerun p2p-upgrade-preflight without checkpoint override before resuming rollout"
          ]
          elif .checkpoint_required_height != null then [
            "obtain governance-approved trusted checkpoint height and hash",
            "ensure verified snapshot/state-sync bundle covers required_height",
            "rerun p2p-upgrade-preflight with --trusted-checkpoint-height and --trusted-checkpoint-hash"
          ]
          else []
          end
        )
      }
    | @json
  ' <<<"$status_json")"
  if [[ -n "$RECOVERY_PLAN_DIR" ]]; then
    plan_name="$(jq -r '.node_id | gsub("[^A-Za-z0-9_.-]"; "_")' <<<"$summary")"
    jq -S --arg status_url "$status_label" '.recovery_plan + {
      node_id,
      status_url: $status_url,
      committed_height,
      network_committed_height,
      replication_persisted_height,
      gap_blocked_height,
      fallback_reason,
      warnings
    }' <<<"$summary" >"$RECOVERY_PLAN_DIR/$plan_name.recovery-plan.json"
    if [[ -n "$RESTORE_SCRIPT_DIR" ]]; then
      restore_script="$RESTORE_SCRIPT_DIR/$plan_name.restore.sh"
      rollback_script="$RESTORE_SCRIPT_DIR/$plan_name.rollback.sh"
      {
        printf '#!/usr/bin/env bash\n'
        printf 'set -euo pipefail\n'
        printf ': "${OASIS7_EXECUTE_RESTORE:?set OASIS7_EXECUTE_RESTORE=%s to execute this audited restore script}"\n' "$plan_name"
        printf 'if [[ "$OASIS7_EXECUTE_RESTORE" != %q ]]; then echo "restore confirmation mismatch" >&2; exit 2; fi\n' "$plan_name"
        jq -r '.recovery_plan.restore_command_plan[]? | .command | if startswith("rerun ") then "echo " + (. | @sh) else . end' <<<"$summary"
      } >"$restore_script"
      {
        printf '#!/usr/bin/env bash\n'
        printf 'set -euo pipefail\n'
        printf ': "${OASIS7_EXECUTE_ROLLBACK:?set OASIS7_EXECUTE_ROLLBACK=%s to execute this audited rollback script}"\n' "$plan_name"
        printf 'if [[ "$OASIS7_EXECUTE_ROLLBACK" != %q ]]; then echo "rollback confirmation mismatch" >&2; exit 2; fi\n' "$plan_name"
        jq -r '.recovery_plan.rollback_command_plan[]? | .command | if startswith("rerun ") then "echo " + (. | @sh) else . end' <<<"$summary"
      } >"$rollback_script"
      chmod 700 "$restore_script" "$rollback_script"
      restore_script_sha256="$(sha256sum "$restore_script" | awk '{print $1}')"
      rollback_script_sha256="$(sha256sum "$rollback_script" | awk '{print $1}')"
      jq -S \
        --arg restore_script "$restore_script" \
        --arg rollback_script "$rollback_script" \
        --arg restore_script_sha256 "$restore_script_sha256" \
        --arg rollback_script_sha256 "$rollback_script_sha256" \
        '. + {
          restore_script: $restore_script,
          rollback_script: $rollback_script,
          restore_script_sha256: $restore_script_sha256,
          rollback_script_sha256: $rollback_script_sha256
        }' "$RECOVERY_PLAN_DIR/$plan_name.recovery-plan.json" >"$RECOVERY_PLAN_DIR/$plan_name.recovery-plan.json.tmp"
      mv "$RECOVERY_PLAN_DIR/$plan_name.recovery-plan.json.tmp" "$RECOVERY_PLAN_DIR/$plan_name.recovery-plan.json"
      if [[ "$EXECUTE_RESTORE_SCRIPTS" == "true" ]]; then
        restore_execution_log="$RESTORE_SCRIPT_DIR/$plan_name.restore.log"
        restore_execution_state_file="$RESTORE_SCRIPT_DIR/$plan_name.restore.state.json"
        jq -n -S \
          --arg node_id "$plan_name" \
          --arg restore_script "$restore_script" \
          --arg restore_script_sha256 "$restore_script_sha256" \
          --arg restore_execution_log "$restore_execution_log" \
          '{
            node_id: $node_id,
            phase: "restore",
            status: "running",
            restore_script: $restore_script,
            restore_script_sha256: $restore_script_sha256,
            restore_execution_log: $restore_execution_log
          }' >"$restore_execution_state_file"
        restore_execution_status="passed"
        set +e
        OASIS7_EXECUTE_RESTORE="$plan_name" bash "$restore_script" >"$restore_execution_log" 2>&1
        restore_execution_exit_code=$?
        set -e
        if (( restore_execution_exit_code != 0 )); then
          restore_execution_status="failed"
          overall=1
        fi
        jq -n -S \
          --arg node_id "$plan_name" \
          --arg restore_script "$restore_script" \
          --arg restore_script_sha256 "$restore_script_sha256" \
          --arg restore_execution_log "$restore_execution_log" \
          --arg restore_execution_status "$restore_execution_status" \
          --argjson restore_execution_exit_code "$restore_execution_exit_code" \
          '{
            node_id: $node_id,
            phase: "restore",
            status: $restore_execution_status,
            exit_code: $restore_execution_exit_code,
            restore_script: $restore_script,
            restore_script_sha256: $restore_script_sha256,
            restore_execution_log: $restore_execution_log
          }' >"$restore_execution_state_file"
        jq -S \
          --arg restore_execution_log "$restore_execution_log" \
          --arg restore_execution_state_file "$restore_execution_state_file" \
          --arg restore_execution_status "$restore_execution_status" \
          --argjson restore_execution_exit_code "$restore_execution_exit_code" \
          '. + {
            restore_execution_log: $restore_execution_log,
            restore_execution_state_file: $restore_execution_state_file,
            restore_execution_status: $restore_execution_status,
            restore_execution_exit_code: $restore_execution_exit_code
          }' "$RECOVERY_PLAN_DIR/$plan_name.recovery-plan.json" >"$RECOVERY_PLAN_DIR/$plan_name.recovery-plan.json.tmp"
        mv "$RECOVERY_PLAN_DIR/$plan_name.recovery-plan.json.tmp" "$RECOVERY_PLAN_DIR/$plan_name.recovery-plan.json"
        if [[ "$restore_execution_status" == "failed" && "$AUTO_ROLLBACK_ON_RESTORE_FAILURE" == "true" ]]; then
          rollback_execution_log="$RESTORE_SCRIPT_DIR/$plan_name.rollback.log"
          rollback_execution_state_file="$RESTORE_SCRIPT_DIR/$plan_name.rollback.state.json"
          jq -n -S \
            --arg node_id "$plan_name" \
            --arg rollback_script "$rollback_script" \
            --arg rollback_script_sha256 "$rollback_script_sha256" \
            --arg rollback_execution_log "$rollback_execution_log" \
            '{
              node_id: $node_id,
              phase: "rollback",
              status: "running",
              rollback_script: $rollback_script,
              rollback_script_sha256: $rollback_script_sha256,
              rollback_execution_log: $rollback_execution_log
            }' >"$rollback_execution_state_file"
          rollback_execution_status="passed"
          set +e
          OASIS7_EXECUTE_ROLLBACK="$plan_name" bash "$rollback_script" >"$rollback_execution_log" 2>&1
          rollback_execution_exit_code=$?
          set -e
          if (( rollback_execution_exit_code != 0 )); then
            rollback_execution_status="failed"
          fi
          jq -n -S \
            --arg node_id "$plan_name" \
            --arg rollback_script "$rollback_script" \
            --arg rollback_script_sha256 "$rollback_script_sha256" \
            --arg rollback_execution_log "$rollback_execution_log" \
            --arg rollback_execution_status "$rollback_execution_status" \
            --argjson rollback_execution_exit_code "$rollback_execution_exit_code" \
            '{
              node_id: $node_id,
              phase: "rollback",
              status: $rollback_execution_status,
              exit_code: $rollback_execution_exit_code,
              rollback_script: $rollback_script,
              rollback_script_sha256: $rollback_script_sha256,
              rollback_execution_log: $rollback_execution_log
            }' >"$rollback_execution_state_file"
          jq -S \
            --arg rollback_execution_log "$rollback_execution_log" \
            --arg rollback_execution_state_file "$rollback_execution_state_file" \
            --arg rollback_execution_status "$rollback_execution_status" \
            --argjson rollback_execution_exit_code "$rollback_execution_exit_code" \
            '. + {
              rollback_execution_log: $rollback_execution_log,
              rollback_execution_state_file: $rollback_execution_state_file,
              rollback_execution_status: $rollback_execution_status,
              rollback_execution_exit_code: $rollback_execution_exit_code
            }' "$RECOVERY_PLAN_DIR/$plan_name.recovery-plan.json" >"$RECOVERY_PLAN_DIR/$plan_name.recovery-plan.json.tmp"
          mv "$RECOVERY_PLAN_DIR/$plan_name.recovery-plan.json.tmp" "$RECOVERY_PLAN_DIR/$plan_name.recovery-plan.json"
        fi
      fi
    fi
  fi
  failures_len="$(jq -r '.failures | length' <<<"$summary")"
  if [[ "$failures_len" == "0" ]]; then
    echo "PASS $status_label $summary"
  else
    echo "FAIL $status_label $summary"
    overall=1
  fi
done

exit "$overall"
