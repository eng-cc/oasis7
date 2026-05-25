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

  ./scripts/p2p-public-testnet-local-observer-sync.sh reset-state \
    --local-env <path> \
    [--backup-dir <path>]

  SSHPASS=<remote-password> \
  ./scripts/p2p-public-testnet-local-observer-sync.sh seed-from-remote \
    --local-env <path> \
    --remote-host <user@host> \
    [--remote-env <path>] \
    [--backup-dir <path>]

Description:
  Derive the local public_testnet observer env contract from the current
  two-validator ECS env files. The rendered env preserves local-only binds,
  storage paths, and player-entry settings, while replacing validator,
  signer, bootstrap-peer, and manifest settings with the live ECS contract.
  When apply mode installs a manifest source from the repo, it also localizes
  runtime_refs files into the target config directory and rewrites the manifest
  to point at those local copies. reset-state backs up and clears the local
  observer's replicated execution state so a drifted pre-sync history can be
  rebuilt from the current two-validator network contract. seed-from-remote
  bypasses replay-from-genesis by copying the healthy peer's current execution
  world, simulator mirror, and bridge state into the local observer tree so
  the observer can resume from the healthy peer's current execution head. It
  intentionally seeds only the current head, not a full historical recovery
  archive, and requires sshpass-compatible credentials via SSHPASS.
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

resolved_env_value() {
  local env_file=$1
  local key=$2
  local value

  value=$(ENV_FILE="$env_file" KEY_NAME="$key" bash -lc '
    source "$ENV_FILE"
    value="${!KEY_NAME:-}"
    [[ -n "$value" ]] || exit 1
    printf "%s" "$value"
  ') || die "missing or unresolved $key in $env_file"

  printf '%s' "$value"
}

require_command() {
  local command_name=$1
  command -v "$command_name" >/dev/null 2>&1 || die "missing command: $command_name"
}

sshpass_ssh() {
  require_command ssh
  require_command sshpass
  [[ -n "${SSHPASS:-}" ]] || die "SSHPASS is required for remote access"

  local remote_host=$1
  shift
  sshpass -e ssh \
    -o StrictHostKeyChecking=no \
    -o UserKnownHostsFile=/dev/null \
    "$remote_host" \
    "$@"
}

sshpass_scp_from_remote() {
  require_command scp
  require_command sshpass
  [[ -n "${SSHPASS:-}" ]] || die "SSHPASS is required for remote access"

  local remote_spec=$1
  local local_path=$2
  shift 2

  mkdir -p "$(dirname "$local_path")"
  sshpass -e scp \
    -q \
    -o StrictHostKeyChecking=no \
    -o UserKnownHostsFile=/dev/null \
    "$@" \
    "$remote_spec" \
    "$local_path" \
    < /dev/null
}

stream_remote_tar_dir_to_local() {
  require_command tar

  local remote_host=$1
  local remote_dir=$2
  local local_parent_dir=$3
  local remote_cmd

  mkdir -p "$local_parent_dir"
  printf -v remote_cmd \
    'tar -C %q -cf - %q' \
    "$(dirname "$remote_dir")" \
    "$(basename "$remote_dir")"
  sshpass_ssh "$remote_host" "$remote_cmd" | tar -C "$local_parent_dir" -xf -
}

stream_remote_tar_contents_to_local() {
  require_command tar

  local remote_host=$1
  local remote_dir=$2
  local local_dir=$3
  local remote_cmd

  mkdir -p "$local_dir"
  printf -v remote_cmd \
    'tar -C %q -cf - .' \
    "$remote_dir"
  sshpass_ssh "$remote_host" "$remote_cmd" | tar -C "$local_dir" -xf -
}

stage_remote_seed_tree() {
  local remote_host=$1
  local remote_stack_root=$2
  local remote_execution_world_dir=$3
  local remote_execution_records_dir=$4
  local remote_storage_root=$5
  local remote_replication_root=$6
  local remote_simulator_dir=$7
  local remote_execution_bridge_state_path=$8
  local remote_script remote_cmd

  remote_script=$(cat <<'PY'
import json
import os
import shutil
import sys
import tempfile
import time

stack_root = os.environ["REMOTE_STACK_ROOT"]
execution_world_dir = os.environ["REMOTE_EXECUTION_WORLD_DIR"]
execution_records_dir = os.environ["REMOTE_EXECUTION_RECORDS_DIR"]
storage_root = os.environ["REMOTE_STORAGE_ROOT"]
replication_root = os.environ["REMOTE_REPLICATION_ROOT"]
simulator_dir = os.environ["REMOTE_SIMULATOR_DIR"]
bridge_state_path = os.environ["REMOTE_EXECUTION_BRIDGE_STATE_PATH"]
stage_root = os.path.join(stack_root, "tmp")
os.makedirs(stage_root, exist_ok=True)

def link_or_copy(src: str, dst: str) -> None:
    os.makedirs(os.path.dirname(dst), exist_ok=True)
    try:
        os.link(src, dst)
    except OSError:
        shutil.copy2(src, dst)

def copy_file(src: str, dst: str) -> None:
    os.makedirs(os.path.dirname(dst), exist_ok=True)
    shutil.copy2(src, dst)

for attempt in range(1, 6):
    stage_dir = tempfile.mkdtemp(prefix="local-observer-seed.", dir=stage_root)
    try:
        execution_world_stage = os.path.join(stage_dir, "execution-world")
        execution_records_stage = os.path.join(stage_dir, "execution-records")
        storage_stage = os.path.join(stage_dir, "storage")
        replication_stage = os.path.join(stage_dir, "replication-root")
        simulator_stage = os.path.join(stage_dir, "execution-world-simulator-mirror")
        bridge_stage_dir = os.path.join(stage_dir, "chain-runtime")
        os.makedirs(execution_world_stage, exist_ok=True)
        os.makedirs(execution_records_stage, exist_ok=True)
        os.makedirs(storage_stage, exist_ok=True)
        os.makedirs(replication_stage, exist_ok=True)
        os.makedirs(simulator_stage, exist_ok=True)
        os.makedirs(bridge_stage_dir, exist_ok=True)

        copy_file(
            os.path.join(execution_world_dir, "snapshot.json"),
            os.path.join(execution_world_stage, "snapshot.json"),
        )
        copy_file(
            os.path.join(execution_world_dir, "journal.json"),
            os.path.join(execution_world_stage, "journal.json"),
        )

        module_registry_path = os.path.join(execution_world_dir, "module_registry.json")
        if os.path.isfile(module_registry_path):
            copy_file(
                module_registry_path,
                os.path.join(execution_world_stage, "module_registry.json"),
            )

        archive_index_path = os.path.join(execution_world_dir, "tick-consensus.archive.index.json")
        if os.path.isfile(archive_index_path):
            copy_file(
                archive_index_path,
                os.path.join(execution_world_stage, "tick-consensus.archive.index.json"),
            )
            with open(
                os.path.join(execution_world_stage, "snapshot.json"),
                "r",
                encoding="utf-8",
            ) as fh:
                snapshot_payload = json.load(fh)
            with open(
                os.path.join(execution_world_stage, "tick-consensus.archive.index.json"),
                "r",
                encoding="utf-8",
            ) as fh:
                archive_index = json.load(fh)
            if snapshot_payload.get("tick_consensus_hot_from_tick") != archive_index.get(
                "hot_from_tick"
            ) or snapshot_payload.get("tick_consensus_hot_to_tick") != archive_index.get(
                "hot_to_tick"
            ):
                raise RuntimeError("tick consensus archive hot range drift during remote staging")
            indexed_record_count = sum(
                int(segment.get("record_count", 0))
                for segment in archive_index.get("archived_segments", [])
            )
            if snapshot_payload.get("tick_consensus_archived_record_count") != indexed_record_count:
                raise RuntimeError(
                    "tick consensus archive record count drift during remote staging"
                )
            for segment in archive_index.get("archived_segments", []):
                relative_path = segment.get("relative_path")
                if not relative_path:
                    continue
                source_path = os.path.join(execution_world_dir, relative_path)
                if not os.path.isfile(source_path):
                    raise FileNotFoundError(source_path)
                copy_file(
                    source_path,
                    os.path.join(execution_world_stage, relative_path),
                )

        archive_path = os.path.join(execution_world_dir, "tick-consensus.archive.json")
        if os.path.isfile(archive_path):
            copy_file(
                archive_path,
                os.path.join(execution_world_stage, "tick-consensus.archive.json"),
            )

        modules_dir = os.path.join(execution_world_dir, "modules")
        if os.path.isdir(modules_dir):
            shutil.copytree(
                modules_dir,
                os.path.join(execution_world_stage, "modules"),
                copy_function=shutil.copy2,
            )

        shutil.copytree(
            execution_records_dir,
            execution_records_stage,
            dirs_exist_ok=True,
            copy_function=shutil.copy2,
        )
        shutil.copytree(
            storage_root,
            storage_stage,
            dirs_exist_ok=True,
            copy_function=link_or_copy,
        )
        shutil.copytree(
            replication_root,
            replication_stage,
            dirs_exist_ok=True,
            copy_function=link_or_copy,
        )

        copy_file(
            os.path.join(simulator_dir, "snapshot.json"),
            os.path.join(simulator_stage, "snapshot.json"),
        )
        copy_file(
            os.path.join(simulator_dir, "journal.json"),
            os.path.join(simulator_stage, "journal.json"),
        )
        copy_file(
            bridge_state_path,
            os.path.join(bridge_stage_dir, os.path.basename(bridge_state_path)),
        )

        print(stage_dir, end="")
        sys.exit(0)
    except (FileNotFoundError, RuntimeError):
        shutil.rmtree(stage_dir, ignore_errors=True)
        if attempt == 5:
            raise
        time.sleep(0.2)
    except Exception:
        shutil.rmtree(stage_dir, ignore_errors=True)
        raise
PY
)

  remote_cmd=$(cat <<EOF
env REMOTE_STACK_ROOT=$(printf '%q' "$remote_stack_root") REMOTE_EXECUTION_WORLD_DIR=$(printf '%q' "$remote_execution_world_dir") REMOTE_EXECUTION_RECORDS_DIR=$(printf '%q' "$remote_execution_records_dir") REMOTE_STORAGE_ROOT=$(printf '%q' "$remote_storage_root") REMOTE_REPLICATION_ROOT=$(printf '%q' "$remote_replication_root") REMOTE_SIMULATOR_DIR=$(printf '%q' "$remote_simulator_dir") REMOTE_EXECUTION_BRIDGE_STATE_PATH=$(printf '%q' "$remote_execution_bridge_state_path") python3 - <<'PY'
$remote_script
PY
EOF
)

  sshpass_ssh "$remote_host" "$remote_cmd"
}

cleanup_remote_seed_tree() {
  local remote_host=$1
  local remote_stage_dir=$2
  local remote_cmd

  [[ -n "$remote_stage_dir" ]] || return 0

  remote_cmd=$(cat <<EOF
env REMOTE_STAGE_DIR=$(printf '%q' "$remote_stage_dir") python3 - <<'PY'
import os
import shutil

stage_dir = os.environ["REMOTE_STAGE_DIR"]
if stage_dir:
    shutil.rmtree(stage_dir, ignore_errors=True)
PY
EOF
)

  sshpass_ssh "$remote_host" "$remote_cmd" >/dev/null 2>&1 || true
}

remote_resolved_env_value() {
  local remote_host=$1
  local remote_env=$2
  local key=$3
  local remote_cmd
  printf -v remote_cmd \
    'env REMOTE_ENV=%q KEY_NAME=%q bash -lc %q' \
    "$remote_env" \
    "$key" \
    'source "$REMOTE_ENV" && value="${!KEY_NAME:-}" && [[ -n "$value" ]] && printf "%s" "$value"'
  sshpass_ssh "$remote_host" "$remote_cmd" \
    || die "missing or unresolved remote $key in $remote_env via $remote_host"
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

backup_and_remove_path() {
  local path=$1
  local backup_target=$2

  if [[ ! -e "$path" ]]; then
    printf 'skipped missing %s\n' "$path"
    return 0
  fi

  mkdir -p "$(dirname "$backup_target")"
  mv "$path" "$backup_target"
  printf 'backed up %s -> %s\n' "$path" "$backup_target"
}

reset_local_state() {
  local local_env=$1
  local backup_dir=$2

  local local_stack_root node_id execution_world_dir execution_records_dir
  local replication_root execution_bridge_state_path
  local storage_root

  local_stack_root=$(resolved_env_value "$local_env" STACK_ROOT)
  node_id=$(resolved_env_value "$local_env" NODE_ID)
  execution_world_dir=$(resolved_env_value "$local_env" EXECUTION_WORLD_DIR)
  execution_records_dir=$(resolved_env_value "$local_env" EXECUTION_RECORDS_DIR)
  storage_root=$(resolved_env_value "$local_env" STORAGE_ROOT)
  replication_root="$local_stack_root/output/node-distfs/$node_id"
  execution_bridge_state_path="$local_stack_root/output/chain-runtime/$node_id/reward-runtime-execution-bridge-state.json"

  if [[ -z "$backup_dir" ]]; then
    backup_dir="$local_stack_root/backups/local-observer-state-reset-$(date +%Y%m%d-%H%M%S)"
  fi

  mkdir -p "$backup_dir"

  backup_and_remove_path "$execution_world_dir" "$backup_dir/execution-world"
  backup_and_remove_path "$execution_records_dir" "$backup_dir/execution-records"
  backup_and_remove_path "$replication_root" "$backup_dir/node-distfs/$node_id"
  backup_and_remove_path \
    "$execution_bridge_state_path" \
    "$backup_dir/chain-runtime/$node_id/$(basename "$execution_bridge_state_path")"

  printf 'preserved storage_root=%s\n' "$storage_root"
  printf 'backup_dir=%s\n' "$backup_dir"
}

seed_local_state_from_remote() {
  local local_env=$1
  local remote_host=$2
  local remote_env=$3
  local backup_dir=$4

  local local_stack_root local_node_id local_execution_world_dir local_execution_records_dir
  local local_storage_root local_replication_root local_execution_bridge_state_path
  local local_simulator_dir
  local remote_stack_root remote_node_id remote_execution_world_dir remote_execution_records_dir
  local remote_storage_root remote_replication_root remote_execution_bridge_state_path
  local remote_simulator_dir
  local remote_stage_dir

  local_stack_root=$(resolved_env_value "$local_env" STACK_ROOT)
  local_node_id=$(resolved_env_value "$local_env" NODE_ID)
  local_execution_world_dir=$(resolved_env_value "$local_env" EXECUTION_WORLD_DIR)
  local_execution_records_dir=$(resolved_env_value "$local_env" EXECUTION_RECORDS_DIR)
  local_storage_root=$(resolved_env_value "$local_env" STORAGE_ROOT)
  local_replication_root="$local_stack_root/output/node-distfs/$local_node_id"
  local_execution_bridge_state_path="$local_stack_root/output/chain-runtime/$local_node_id/reward-runtime-execution-bridge-state.json"
  local_simulator_dir="${local_execution_world_dir}-simulator-mirror"

  remote_stack_root=$(remote_resolved_env_value "$remote_host" "$remote_env" STACK_ROOT)
  remote_node_id=$(remote_resolved_env_value "$remote_host" "$remote_env" NODE_ID)
  remote_execution_world_dir=$(remote_resolved_env_value "$remote_host" "$remote_env" EXECUTION_WORLD_DIR)
  remote_execution_records_dir=$(remote_resolved_env_value "$remote_host" "$remote_env" EXECUTION_RECORDS_DIR)
  remote_storage_root=$(remote_resolved_env_value "$remote_host" "$remote_env" STORAGE_ROOT)
  remote_replication_root="$remote_stack_root/output/node-distfs/$remote_node_id"
  remote_execution_bridge_state_path="$remote_stack_root/output/chain-runtime/$remote_node_id/reward-runtime-execution-bridge-state.json"
  remote_simulator_dir="${remote_execution_world_dir}-simulator-mirror"

  [[ "$remote_execution_world_dir" == "$remote_stack_root"/data/* ]] \
    || die "remote execution world dir must live under remote stack root data/: $remote_execution_world_dir"

  if [[ -z "$backup_dir" ]]; then
    backup_dir="$local_stack_root/backups/local-observer-remote-seed-$(date +%Y%m%d-%H%M%S)"
  fi

  mkdir -p "$backup_dir"

  backup_and_remove_path "$local_execution_world_dir" "$backup_dir/execution-world"
  backup_and_remove_path "$local_simulator_dir" "$backup_dir/execution-world-simulator-mirror"
  backup_and_remove_path "$local_execution_records_dir" "$backup_dir/execution-records"
  backup_and_remove_path "$local_storage_root" "$backup_dir/storage"
  backup_and_remove_path "$local_replication_root" "$backup_dir/node-distfs/$local_node_id"
  backup_and_remove_path \
    "$local_execution_bridge_state_path" \
    "$backup_dir/chain-runtime/$local_node_id/$(basename "$local_execution_bridge_state_path")"

  mkdir -p "$(dirname "$local_execution_bridge_state_path")"
  mkdir -p "$local_execution_records_dir" "$local_storage_root" "$local_replication_root"
  mkdir -p "$local_execution_world_dir" "$local_simulator_dir"

  remote_stage_dir=$(stage_remote_seed_tree \
    "$remote_host" \
    "$remote_stack_root" \
    "$remote_execution_world_dir" \
    "$remote_execution_records_dir" \
    "$remote_storage_root" \
    "$remote_replication_root" \
    "$remote_simulator_dir" \
    "$remote_execution_bridge_state_path")

  sshpass_scp_from_remote \
    "$remote_host:$remote_stage_dir/execution-world/snapshot.json" \
    "$local_execution_world_dir/snapshot.json"
  sshpass_scp_from_remote \
    "$remote_host:$remote_stage_dir/execution-world/journal.json" \
    "$local_execution_world_dir/journal.json"
  if sshpass_ssh "$remote_host" test -d "$remote_stage_dir/execution-records"; then
    stream_remote_tar_dir_to_local \
      "$remote_host" \
      "$remote_stage_dir/execution-records" \
      "$(dirname "$local_execution_records_dir")"
  fi
  if sshpass_ssh "$remote_host" test -d "$remote_stage_dir/storage"; then
    stream_remote_tar_dir_to_local \
      "$remote_host" \
      "$remote_stage_dir/storage" \
      "$(dirname "$local_storage_root")"
  fi
  if sshpass_ssh "$remote_host" test -d "$remote_stage_dir/replication-root"; then
    stream_remote_tar_contents_to_local \
      "$remote_host" \
      "$remote_stage_dir/replication-root" \
      "$local_replication_root"
  fi

  if sshpass_ssh "$remote_host" test -f "$remote_stage_dir/execution-world/module_registry.json"; then
    sshpass_scp_from_remote \
      "$remote_host:$remote_stage_dir/execution-world/module_registry.json" \
      "$local_execution_world_dir/module_registry.json"
  fi
  if sshpass_ssh "$remote_host" test -f "$remote_stage_dir/execution-world/tick-consensus.archive.index.json"; then
    sshpass_scp_from_remote \
      "$remote_host:$remote_stage_dir/execution-world/tick-consensus.archive.index.json" \
      "$local_execution_world_dir/tick-consensus.archive.index.json"
  fi
  if sshpass_ssh "$remote_host" test -d "$remote_stage_dir/execution-world/tick-consensus.archive.segments"; then
    sshpass_scp_from_remote \
      "$remote_host:$remote_stage_dir/execution-world/tick-consensus.archive.segments" \
      "$local_execution_world_dir/" \
      -r
  fi
  if sshpass_ssh "$remote_host" test -f "$remote_stage_dir/execution-world/tick-consensus.archive.json"; then
    sshpass_scp_from_remote \
      "$remote_host:$remote_stage_dir/execution-world/tick-consensus.archive.json" \
      "$local_execution_world_dir/tick-consensus.archive.json"
  fi
  if sshpass_ssh "$remote_host" test -d "$remote_stage_dir/execution-world/modules"; then
    sshpass_scp_from_remote \
      "$remote_host:$remote_stage_dir/execution-world/modules" \
      "$local_execution_world_dir/" \
      -r
  fi

  sshpass_scp_from_remote \
    "$remote_host:$remote_stage_dir/execution-world-simulator-mirror/snapshot.json" \
    "$local_simulator_dir/snapshot.json"
  sshpass_scp_from_remote \
    "$remote_host:$remote_stage_dir/execution-world-simulator-mirror/journal.json" \
    "$local_simulator_dir/journal.json"
  sshpass_scp_from_remote \
    "$remote_host:$remote_stage_dir/chain-runtime/$(basename "$local_execution_bridge_state_path")" \
    "$local_execution_bridge_state_path"

  cleanup_remote_seed_tree "$remote_host" "$remote_stage_dir"

  printf 'seeded local observer state from %s\n' "$remote_host"
  printf 'remote_env=%s\n' "$remote_env"
  printf 'remote_node_id=%s\n' "$remote_node_id"
  printf 'backup_dir=%s\n' "$backup_dir"
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
remote_host=""
remote_env=""

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
    --remote-host)
      remote_host=${2:-}
      shift 2
      ;;
    --remote-env)
      remote_env=${2:-}
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

case "$mode" in
  render|apply)
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
    ;;
  reset-state)
    [[ -n "$local_env" ]] || die "--local-env is required"
    require_file "$local_env"
    reset_local_state "$local_env" "$backup_dir"
    ;;
  seed-from-remote)
    [[ -n "$local_env" ]] || die "--local-env is required"
    [[ -n "$remote_host" ]] || die "--remote-host is required"
    require_file "$local_env"
    if [[ -z "$remote_env" ]]; then
      remote_env="/opt/oasis7/p2p-testnet/config/node.env"
    fi
    seed_local_state_from_remote "$local_env" "$remote_host" "$remote_env" "$backup_dir"
    ;;
  *)
    die "unknown mode: $mode"
    ;;
esac
