#!/usr/bin/env bash
set -euo pipefail

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)

usage() {
  cat <<'EOF'
Usage:
  ./scripts/p2p-public-testnet-local-observer-sync.sh render \
    --local-env <path> \
    --sequencer-env <path> \
    --storage-env <path> \
    --manifest-path <path> \
    [--out <path>]

  ./scripts/p2p-public-testnet-local-observer-sync.sh apply \
    --local-env <path> \
    --sequencer-env <path> \
    --storage-env <path> \
    --manifest-path <path> \
    [--manifest-source <path>] \
    [--manifest-dest <path>] \
    [--start-script-source <path>] \
    [--start-script-dest <path>] \
    [--backup-dir <path>]

Description:
  Derive the local public_testnet observer env contract from the current
  two-validator ECS env files. The rendered env preserves local-only binds,
  storage paths, and player-entry settings, while replacing validator,
  signer, bootstrap-peer, and manifest settings with the live ECS contract.
  When apply mode installs a manifest source from the repo, it also localizes
  runtime_refs files into the target config directory and rewrites the manifest
  to point at those local copies.
EOF
}

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

require_file() {
  local path=$1
  [[ -f "$path" ]] || die "missing file: $path"
}

raw_value() {
  local path=$1
  local key=$2
  awk -F= -v key="$key" '
    $1 == key {
      sub(/^[^=]*=/, "");
      print;
      found = 1;
      exit 0;
    }
    END {
      if (!found) {
        exit 1;
      }
    }
  ' "$path"
}

required_value() {
  local path=$1
  local key=$2
  local value
  value=$(raw_value "$path" "$key") || die "missing $key in $path"
  printf '%s' "$value"
}

optional_value() {
  local path=$1
  local key=$2
  raw_value "$path" "$key" 2>/dev/null || true
}

join_unique_csv() {
  local first_csv=$1
  local second_csv=$2
  local -a first_items=()
  local -a second_items=()
  local -a output=()
  local item
  local seen_items=""

  IFS=, read -r -a first_items <<< "$first_csv"
  IFS=, read -r -a second_items <<< "$second_csv"

  for item in "${first_items[@]}" "${second_items[@]}"; do
    [[ -n "$item" ]] || continue
    case ",$seen_items," in
      *,"$item",*) ;;
      *)
        output+=("$item")
        if [[ -n "$seen_items" ]]; then
          seen_items+=","
        fi
        seen_items+="$item"
        ;;
    esac
  done

  local joined=""
  if (( ${#output[@]} > 0 )); then
    local old_ifs=$IFS
    IFS=,
    joined="${output[*]}"
    IFS=$old_ifs
  fi
  printf '%s' "$joined"
}

append_if_present() {
  local key=$1
  local value=$2
  if [[ -n "$value" ]]; then
    printf '%s=%s\n' "$key" "$value"
  fi
}

repo_root=$(cd "$script_dir/.." && pwd)

resolve_repo_ref() {
  local ref=$1
  if [[ "$ref" = /* ]]; then
    printf '%s' "$ref"
  else
    printf '%s/%s' "$repo_root" "$ref"
  fi
}

render_env() {
  local local_env=$1
  local sequencer_env=$2
  local storage_env=$3
  local manifest_path=$4

  local seq_world_id storage_world_id
  seq_world_id=$(required_value "$sequencer_env" WORLD_ID)
  storage_world_id=$(required_value "$storage_env" WORLD_ID)
  [[ "$seq_world_id" == "$storage_world_id" ]] || die "WORLD_ID mismatch between ECS env files"

  local seq_validators storage_validators
  seq_validators=$(required_value "$sequencer_env" NODE_VALIDATORS_CSV)
  storage_validators=$(required_value "$storage_env" NODE_VALIDATORS_CSV)
  [[ "$seq_validators" == "$storage_validators" ]] || die "NODE_VALIDATORS_CSV mismatch between ECS env files"

  local seq_signers storage_signers
  seq_signers=$(required_value "$sequencer_env" NODE_VALIDATOR_SIGNERS_CSV)
  storage_signers=$(required_value "$storage_env" NODE_VALIDATOR_SIGNERS_CSV)
  [[ "$seq_signers" == "$storage_signers" ]] || die "NODE_VALIDATOR_SIGNERS_CSV mismatch between ECS env files"

  local seq_role storage_role
  seq_role=$(required_value "$sequencer_env" NODE_ROLE)
  storage_role=$(required_value "$storage_env" NODE_ROLE)
  [[ "$seq_role" == "$storage_role" ]] || die "NODE_ROLE mismatch between ECS env files"

  local gossip_peers replication_peers replication_remote_writers
  gossip_peers=$(join_unique_csv \
    "$(required_value "$storage_env" NODE_GOSSIP_PEERS_CSV)" \
    "$(required_value "$sequencer_env" NODE_GOSSIP_PEERS_CSV)")
  replication_peers=$(join_unique_csv \
    "$(required_value "$storage_env" REPLICATION_NETWORK_BOOTSTRAP_PEERS_CSV)" \
    "$(required_value "$sequencer_env" REPLICATION_NETWORK_BOOTSTRAP_PEERS_CSV)")
  replication_remote_writers=$(join_unique_csv \
    "$(required_value "$storage_env" REPLICATION_REMOTE_WRITERS_CSV)" \
    "$(required_value "$sequencer_env" REPLICATION_REMOTE_WRITERS_CSV)")

  local pos_slot_clock_genesis
  pos_slot_clock_genesis=$(required_value "$sequencer_env" POS_SLOT_CLOCK_GENESIS_UNIX_MS)

  local player_entry_enable player_entry_http_bind player_entry_http_port
  local player_entry_web_bind player_entry_viewer_bind player_entry_deployment_mode
  local player_entry_llm_mode
  player_entry_enable=$(optional_value "$local_env" PLAYER_ENTRY_ENABLE)
  player_entry_http_bind=$(optional_value "$local_env" PLAYER_ENTRY_HTTP_BIND)
  player_entry_http_port=$(optional_value "$local_env" PLAYER_ENTRY_HTTP_PORT)
  player_entry_web_bind=$(optional_value "$local_env" PLAYER_ENTRY_WEB_BIND)
  player_entry_viewer_bind=$(optional_value "$local_env" PLAYER_ENTRY_VIEWER_BIND)
  player_entry_deployment_mode=$(optional_value "$local_env" PLAYER_ENTRY_DEPLOYMENT_MODE)
  player_entry_llm_mode=$(optional_value "$local_env" PLAYER_ENTRY_LLM_MODE)

  cat <<EOF
HOST_LABEL=$(required_value "$local_env" HOST_LABEL)
SERVICE_NAME=$(required_value "$local_env" SERVICE_NAME)
STACK_ROOT=$(required_value "$local_env" STACK_ROOT)
NODE_ID=$(required_value "$local_env" NODE_ID)
WORLD_ID=$seq_world_id
NODE_ROLE=$(required_value "$local_env" NODE_ROLE)
STORAGE_PROFILE=$(required_value "$sequencer_env" STORAGE_PROFILE)
STATUS_BIND=$(required_value "$local_env" STATUS_BIND)
NODE_GOSSIP_BIND=$(required_value "$local_env" NODE_GOSSIP_BIND)
NODE_GOSSIP_PEERS_CSV=$gossip_peers
NODE_VALIDATORS_CSV=$seq_validators
NODE_VALIDATOR_SIGNERS_CSV=$seq_signers
NODE_TICK_MS=$(required_value "$sequencer_env" NODE_TICK_MS)
POS_SLOT_DURATION_MS=$(required_value "$sequencer_env" POS_SLOT_DURATION_MS)
POS_TICKS_PER_SLOT=$(required_value "$sequencer_env" POS_TICKS_PER_SLOT)
POS_PROPOSAL_TICK_PHASE=$(required_value "$sequencer_env" POS_PROPOSAL_TICK_PHASE)
POS_MAX_PAST_SLOT_LAG=$(required_value "$sequencer_env" POS_MAX_PAST_SLOT_LAG)
POS_ADAPTIVE_TICK_SCHEDULER=$(required_value "$sequencer_env" POS_ADAPTIVE_TICK_SCHEDULER)
REWARD_RUNTIME_ENABLE=$(required_value "$sequencer_env" REWARD_RUNTIME_ENABLE)
REWARD_RUNTIME_EPOCH_DURATION_SECS=$(required_value "$sequencer_env" REWARD_RUNTIME_EPOCH_DURATION_SECS)
REWARD_POINTS_PER_CREDIT=$(required_value "$sequencer_env" REWARD_POINTS_PER_CREDIT)
REWARD_RUNTIME_AUTO_REDEEM=$(required_value "$sequencer_env" REWARD_RUNTIME_AUTO_REDEEM)
NODE_AUTO_ATTEST_FLAG=$(required_value "$local_env" NODE_AUTO_ATTEST_FLAG)
CONFIG_PATH=$(required_value "$local_env" CONFIG_PATH)
EXECUTION_WORLD_DIR=$(required_value "$local_env" EXECUTION_WORLD_DIR)
EXECUTION_RECORDS_DIR=$(required_value "$local_env" EXECUTION_RECORDS_DIR)
STORAGE_ROOT=$(required_value "$local_env" STORAGE_ROOT)
REPLICATION_NETWORK_LISTEN_ADDRS_CSV=$(required_value "$local_env" REPLICATION_NETWORK_LISTEN_ADDRS_CSV)
REPLICATION_NETWORK_BOOTSTRAP_PEERS_CSV=$replication_peers
REPLICATION_REMOTE_WRITERS_CSV=$replication_remote_writers
TRAFFIC_MONITOR_ENABLE=$(required_value "$local_env" TRAFFIC_MONITOR_ENABLE)
TRAFFIC_MONITOR_INTERVAL_SECS=$(required_value "$local_env" TRAFFIC_MONITOR_INTERVAL_SECS)
TRAFFIC_MONITOR_WINDOW_MINUTES=$(required_value "$local_env" TRAFFIC_MONITOR_WINDOW_MINUTES)
TRAFFIC_MONITOR_TOP_N=$(required_value "$local_env" TRAFFIC_MONITOR_TOP_N)
TRAFFIC_MONITOR_OUTPUT_DIR=$(required_value "$local_env" TRAFFIC_MONITOR_OUTPUT_DIR)
TRAFFIC_PROFILE=$(required_value "$local_env" TRAFFIC_PROFILE)
POS_SLOT_CLOCK_GENESIS_UNIX_MS=$pos_slot_clock_genesis
EOF
  append_if_present PLAYER_ENTRY_ENABLE "$player_entry_enable"
  append_if_present PLAYER_ENTRY_HTTP_BIND "$player_entry_http_bind"
  append_if_present PLAYER_ENTRY_HTTP_PORT "$player_entry_http_port"
  append_if_present PLAYER_ENTRY_WEB_BIND "$player_entry_web_bind"
  append_if_present PLAYER_ENTRY_VIEWER_BIND "$player_entry_viewer_bind"
  append_if_present PLAYER_ENTRY_DEPLOYMENT_MODE "$player_entry_deployment_mode"
  append_if_present PLAYER_ENTRY_LLM_MODE "$player_entry_llm_mode"
  printf 'NETWORK_TIER_MANIFEST_PATH=%s\n' "$manifest_path"
}

write_rendered_env() {
  local rendered=$1
  local out_path=$2

  if [[ "$out_path" == "-" ]]; then
    printf '%s' "$rendered"
    return 0
  fi

  mkdir -p "$(dirname "$out_path")"
  printf '%s' "$rendered" > "$out_path"
}

localize_manifest_runtime_refs() {
  local manifest_source=$1
  local manifest_dest=$2

  python3 - "$manifest_source" "$manifest_dest" "$repo_root" <<'PY'
import json
import os
import shutil
import sys

manifest_source, manifest_dest, repo_root = sys.argv[1:4]
manifest_dest = os.path.abspath(manifest_dest)
manifest_dir = os.path.dirname(manifest_dest)

with open(manifest_source, "r", encoding="utf-8") as fh:
    data = json.load(fh)

runtime_refs = data.get("runtime_refs", {})
for key in ("release_candidate_bundle_ref", "genesis_ref", "bootstrap_peer_ref"):
    ref = runtime_refs.get(key)
    if not ref:
        continue
    source = ref if os.path.isabs(ref) else os.path.join(repo_root, ref)
    if not os.path.isfile(source):
        raise SystemExit(f"missing manifest runtime ref source: {source}")
    target_name = os.path.basename(ref)
    target = os.path.join(manifest_dir, target_name)
    os.makedirs(os.path.dirname(target), exist_ok=True)
    shutil.copy2(source, target)
    runtime_refs[key] = target_name

with open(manifest_dest, "w", encoding="utf-8") as fh:
    json.dump(data, fh, ensure_ascii=True, indent=2)
    fh.write("\n")
PY
}

mode=${1:-}
[[ -n "$mode" ]] || {
  usage
  exit 1
}
shift || true

local_env=""
sequencer_env=""
storage_env=""
manifest_path=""
out_path="-"
manifest_source=""
manifest_dest=""
start_script_source="$script_dir/p2p-triad-node-start.sh"
start_script_dest=""
backup_dir=""

while (( $# > 0 )); do
  case "$1" in
    --local-env)
      local_env=${2:-}
      shift 2
      ;;
    --sequencer-env)
      sequencer_env=${2:-}
      shift 2
      ;;
    --storage-env)
      storage_env=${2:-}
      shift 2
      ;;
    --manifest-path)
      manifest_path=${2:-}
      shift 2
      ;;
    --out)
      out_path=${2:-}
      shift 2
      ;;
    --manifest-source)
      manifest_source=${2:-}
      shift 2
      ;;
    --manifest-dest)
      manifest_dest=${2:-}
      shift 2
      ;;
    --start-script-source)
      start_script_source=${2:-}
      shift 2
      ;;
    --start-script-dest)
      start_script_dest=${2:-}
      shift 2
      ;;
    --backup-dir)
      backup_dir=${2:-}
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "unknown argument: $1"
      ;;
  esac
done

[[ "$mode" == "render" || "$mode" == "apply" ]] || die "unknown mode: $mode"
[[ -n "$local_env" ]] || die "--local-env is required"
[[ -n "$sequencer_env" ]] || die "--sequencer-env is required"
[[ -n "$storage_env" ]] || die "--storage-env is required"
[[ -n "$manifest_path" ]] || die "--manifest-path is required"

require_file "$local_env"
require_file "$sequencer_env"
require_file "$storage_env"

rendered_env=$(render_env "$local_env" "$sequencer_env" "$storage_env" "$manifest_path")

if [[ "$mode" == "render" ]]; then
  write_rendered_env "$rendered_env" "$out_path"
  exit 0
fi

require_file "$start_script_source"

local_stack_root=$(required_value "$local_env" STACK_ROOT)
if [[ -z "$manifest_dest" && -n "$manifest_source" ]]; then
  manifest_dest=$manifest_path
fi
if [[ -z "$start_script_dest" ]]; then
  start_script_dest="$local_stack_root/bin/start-node.sh"
fi
if [[ -z "$backup_dir" ]]; then
  backup_dir="$local_stack_root/backups/local-observer-contract-sync-$(date +%Y%m%d-%H%M%S)"
fi

mkdir -p "$backup_dir"

cp "$local_env" "$backup_dir/node.env.before"
tmp_env="$backup_dir/node.env.rendered"
printf '%s' "$rendered_env" > "$tmp_env"
cp "$tmp_env" "$local_env"

if [[ -n "$manifest_source" ]]; then
  require_file "$manifest_source"
  [[ -n "$manifest_dest" ]] || die "--manifest-dest is required when --manifest-source is set"
  mkdir -p "$(dirname "$manifest_dest")"
  if [[ -f "$manifest_dest" ]]; then
    cp "$manifest_dest" "$backup_dir/$(basename "$manifest_dest").before"
  fi
  localize_manifest_runtime_refs "$manifest_source" "$manifest_dest"
fi

if [[ -n "$start_script_dest" ]]; then
  mkdir -p "$(dirname "$start_script_dest")"
  if [[ -f "$start_script_dest" ]]; then
    cp "$start_script_dest" "$backup_dir/$(basename "$start_script_dest").before"
  fi
  install -m 0755 "$start_script_source" "$start_script_dest"
fi

printf 'updated %s\n' "$local_env"
if [[ -n "$manifest_source" ]]; then
  printf 'installed manifest to %s\n' "$manifest_dest"
fi
if [[ -n "$start_script_dest" ]]; then
  printf 'installed start script to %s\n' "$start_script_dest"
fi
printf 'backup_dir=%s\n' "$backup_dir"
