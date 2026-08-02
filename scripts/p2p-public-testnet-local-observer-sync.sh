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

Description:
  Derive the local public_testnet observer env contract from the current
  two-validator ECS env files. The rendered env preserves local-only binds,
  storage paths, and player-entry settings, while replacing bootstrap-peer,
  manifest, and genesis validator registry settings with the live ECS contract.
  apply mode writes GENESIS_VALIDATOR_REGISTRY_PATH and generates that genesis
  registry as a one-time migration adapter from legacy ECS NODE_VALIDATORS_CSV
  / NODE_VALIDATOR_SIGNERS_CSV. Runtime canonical validator truth remains the
  genesis/world-state registry, not the legacy CSV env.
  When apply mode installs a manifest source from the repo, it also localizes
  runtime_refs files into the target config directory and rewrites the manifest
  to point at those local copies. reset-state backs up and clears the local
  observer's replicated execution state, storage root, simulator mirror, and
  bridge state so a drifted pre-sync history can be rebuilt from the current
  two-validator network contract. Recovery must then start the observer and use
  signed replication checkpoint sync. The former seed-from-remote mode is
  disabled because a live filesystem copy cannot bind the execution world,
  records, storage, replication, simulator mirror, and bridge state to one
  committed checkpoint.
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

optional_resolved_env_value() {
  local env_file=$1
  local key=$2
  local value

  if ! raw_value "$env_file" "$key" >/dev/null 2>&1; then
    return 0
  fi

  value=$(ENV_FILE="$env_file" KEY_NAME="$key" bash -lc '
    source "$ENV_FILE"
    value="${!KEY_NAME:-}"
    [[ -n "$value" ]] || exit 1
    printf "%s" "$value"
  ' 2>/dev/null) || return 0

  printf '%s' "$value"
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


execution_bridge_state_path_for_root() {
  local stack_root=$1
  local node_id=$2
  local runtime_root=${3:-}

  if [[ -n "$runtime_root" ]]; then
    printf '%s/reward-runtime-execution-bridge-state.json' "$runtime_root"
  else
    printf '%s/output/chain-runtime/%s/reward-runtime-execution-bridge-state.json' "$stack_root" "$node_id"
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
  local emit_genesis_registry_path=${5:-0}

  local seq_world_id storage_world_id
  seq_world_id=$(required_value "$sequencer_env" WORLD_ID)
  storage_world_id=$(required_value "$storage_env" WORLD_ID)
  [[ "$seq_world_id" == "$storage_world_id" ]] || die "WORLD_ID mismatch between ECS env files"

  local seq_role storage_role local_role
  seq_role=$(required_value "$sequencer_env" NODE_ROLE)
  storage_role=$(required_value "$storage_env" NODE_ROLE)
  local_role=$(required_value "$local_env" NODE_ROLE)
  [[ "$local_role" == "observer" ]] || die "local observer env must declare NODE_ROLE=observer"
  [[ "$seq_role" == "sequencer" ]] || die "sequencer env must declare NODE_ROLE=sequencer"
  [[ "$storage_role" == "storage" ]] || die "storage env must declare NODE_ROLE=storage"

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
  local local_stack_root local_runtime_root local_replication_root genesis_validator_registry_path
  local_stack_root=$(required_value "$local_env" STACK_ROOT)
  local_runtime_root=$(optional_resolved_env_value "$local_env" RUNTIME_ROOT)
  local_replication_root=$(optional_resolved_env_value "$local_env" REPLICATION_ROOT)
  genesis_validator_registry_path="$local_stack_root/config/genesis-validator-registry.json"

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
STACK_ROOT=$local_stack_root
NODE_ID=$(required_value "$local_env" NODE_ID)
WORLD_ID=$seq_world_id
NODE_ROLE=$(required_value "$local_env" NODE_ROLE)
STORAGE_PROFILE=$(required_value "$sequencer_env" STORAGE_PROFILE)
STATUS_BIND=$(required_value "$local_env" STATUS_BIND)
NODE_GOSSIP_BIND=$(required_value "$local_env" NODE_GOSSIP_BIND)
NODE_GOSSIP_PEERS_CSV=$gossip_peers
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
$(append_if_present "RUNTIME_ROOT" "$local_runtime_root")
$(append_if_present "REPLICATION_ROOT" "$local_replication_root")
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
  if [[ "$emit_genesis_registry_path" == "1" ]]; then
    printf 'GENESIS_VALIDATOR_REGISTRY_PATH=%s\n' "$genesis_validator_registry_path"
  fi
  append_if_present PLAYER_ENTRY_ENABLE "$player_entry_enable"
  append_if_present PLAYER_ENTRY_HTTP_BIND "$player_entry_http_bind"
  append_if_present PLAYER_ENTRY_HTTP_PORT "$player_entry_http_port"
  append_if_present PLAYER_ENTRY_WEB_BIND "$player_entry_web_bind"
  append_if_present PLAYER_ENTRY_VIEWER_BIND "$player_entry_viewer_bind"
  append_if_present PLAYER_ENTRY_DEPLOYMENT_MODE "$player_entry_deployment_mode"
  append_if_present PLAYER_ENTRY_LLM_MODE "$player_entry_llm_mode"
  printf 'NETWORK_TIER_MANIFEST_PATH=%s\n' "$manifest_path"
}

write_genesis_validator_registry() {
  local validators_csv=$1
  local signers_csv=$2
  local output_path=$3

  mkdir -p "$(dirname "$output_path")"
  env VALIDATORS_CSV="$validators_csv" SIGNERS_CSV="$signers_csv" python3 - <<'PY' > "$output_path"
import json
import os
import sys

validators = {}
for raw in os.environ["VALIDATORS_CSV"].split(","):
    raw = raw.strip()
    if not raw:
        continue
    if ":" not in raw:
        sys.exit(f"validator entry requires id:stake: {raw}")
    node_id, stake_raw = raw.split(":", 1)
    node_id = node_id.strip()
    stake_raw = stake_raw.strip()
    if not node_id or not stake_raw.isdigit() or int(stake_raw) <= 0:
        sys.exit(f"invalid validator entry: {raw}")
    validators[node_id] = int(stake_raw)

signers = {}
for raw in os.environ["SIGNERS_CSV"].split(","):
    raw = raw.strip()
    if not raw:
        continue
    if ":" not in raw:
        sys.exit(f"signer entry requires id:public_key_hex: {raw}")
    node_id, key_hex = raw.split(":", 1)
    node_id = node_id.strip()
    key_hex = key_hex.strip()
    if not node_id or len(key_hex) != 64 or any(ch not in "0123456789abcdefABCDEF" for ch in key_hex):
        sys.exit(f"invalid signer entry: {raw}")
    signers[node_id] = key_hex

missing = sorted(set(validators) - set(signers))
extra = sorted(set(signers) - set(validators))
if missing:
    sys.exit(f"missing signer binding for validators: {','.join(missing)}")
if extra:
    sys.exit(f"signer binding references unknown validators: {','.join(extra)}")
if not validators:
    sys.exit("genesis validator registry requires at least one validator")

threshold = min(len(validators), (2 * len(validators)) // 3 + 1)
doc = {
    "slot_id": "governance.finality.v1",
    "threshold": threshold,
    "threshold_bps": 0,
    "validators": [
        {
            "node_id": node_id,
            "scheme": "ed25519",
            "finality_signer_public_key": signers[node_id],
            "stake": validators[node_id],
        }
        for node_id in sorted(validators)
    ],
}
print(json.dumps(doc, indent=2, sort_keys=True))
PY
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

  if [[ ! -e "$path" && ! -L "$path" ]]; then
    printf 'skipped missing %s\n' "$path"
    return 0
  fi

  if [[ -e "$backup_target" || -L "$backup_target" ]]; then
    die "refusing to overwrite existing backup $backup_target while live path exists: $path"
  fi

  mkdir -p "$(dirname "$backup_target")"
  mv "$path" "$backup_target"
  printf 'backed up %s -> %s\n' "$path" "$backup_target"
}

restore_governed_bootstrap_artifacts() {
  local mode=$1
  local manifest_path=$2
  local execution_world_dir=$3
  local source_execution_world_dir=$4

  python3 - "$mode" "$manifest_path" "$execution_world_dir" "$source_execution_world_dir" <<'PY'
import hashlib
import json
import os
import pathlib
import shutil
import sys
import tempfile

mode, manifest_path, execution_world_dir, source_execution_world_dir = sys.argv[1:5]
manifest_path = os.path.abspath(manifest_path)
manifest_dir = os.path.dirname(manifest_path)
execution_world_dir = os.path.abspath(execution_world_dir)
source_execution_world_dir = os.path.abspath(source_execution_world_dir)

def fail(message):
    raise SystemExit(message)

def load_json(path, label):
    if not os.path.isfile(path) or os.path.islink(path):
        fail(f"{label} must be a regular file: {path}")
    with open(path, "r", encoding="utf-8") as fh:
        return json.load(fh)

def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()

def sha256_dir_tree(path):
    root = pathlib.Path(path)
    combined = hashlib.sha256()
    file_count = 0
    total_bytes = 0
    for child in sorted(candidate for candidate in root.rglob("*") if candidate.is_file()):
        relative = child.relative_to(root).as_posix()
        child_digest = sha256_file(child)
        size = child.stat().st_size
        combined.update(relative.encode("utf-8"))
        combined.update(b"\0")
        combined.update(child_digest.encode("ascii"))
        combined.update(b"\0")
        combined.update(str(size).encode("ascii"))
        combined.update(b"\n")
        file_count += 1
        total_bytes += size
    return combined.hexdigest(), file_count, total_bytes

def require_sha256(value, label):
    if (
        not isinstance(value, str)
        or len(value) != 64
        or any(ch not in "0123456789abcdef" for ch in value)
    ):
        fail(f"invalid {label}")
    return value

def resolve_ref(ref, base_dir, label):
    if not isinstance(ref, str) or not ref:
        fail(f"missing {label}")
    return os.path.abspath(ref if os.path.isabs(ref) else os.path.join(base_dir, ref))

def is_within(path, root):
    try:
        return os.path.commonpath((path, root)) == root and path != root
    except ValueError:
        return False

def reject_symlink_components(path, root, label):
    relative = os.path.relpath(path, root)
    current = root
    if os.path.islink(current):
        fail(f"unsafe symlink in {label}: {current}")
    for component in relative.split(os.sep):
        current = os.path.join(current, component)
        if os.path.islink(current):
            fail(f"unsafe symlink in {label}: {current}")

manifest = load_json(manifest_path, "network tier manifest")
runtime_refs = manifest.get("runtime_refs")
if not isinstance(runtime_refs, dict):
    fail("network tier manifest runtime_refs must be an object")

ref_keys = (
    "release_candidate_bundle_ref",
    "generated_world_sidecar_ref",
    "world_generation_provenance_ref",
)
optional_ref_keys = ref_keys[1:]
configured_optional = [key for key in optional_ref_keys if runtime_refs.get(key)]
if not configured_optional:
    sys.exit(0)
if len(configured_optional) != len(optional_ref_keys) or not runtime_refs.get(ref_keys[0]):
    fail("governed bootstrap artifact refs must be configured together")

bundle_ref = runtime_refs["release_candidate_bundle_ref"]
if os.path.isabs(bundle_ref):
    fail(f"unsafe absolute release candidate bundle ref: {bundle_ref}")
bundle_path = resolve_ref(bundle_ref, manifest_dir, "release_candidate_bundle_ref")
if os.path.commonpath((bundle_path, manifest_dir)) != manifest_dir:
    fail(f"unsafe release candidate bundle ref outside manifest directory: {bundle_ref}")

sidecar_ref = runtime_refs["generated_world_sidecar_ref"]
provenance_ref = runtime_refs["world_generation_provenance_ref"]
sidecar_target = resolve_ref(sidecar_ref, manifest_dir, "generated_world_sidecar_ref")
provenance_target = resolve_ref(
    provenance_ref, manifest_dir, "world_generation_provenance_ref"
)
target_common = os.path.commonpath((provenance_target, sidecar_target))
if target_common in (sidecar_target, provenance_target):
    fail("world generation provenance path must not overlap generated world sidecar")

bundle = load_json(bundle_path, "release candidate bundle")
governed = (
    (
        "generated_world_sidecar",
        "directory",
        sidecar_target,
        ("snapshot.json", "journal.json"),
        bundle.get("generated_world_sidecar"),
    ),
    (
        "world_generation_provenance",
        "file",
        provenance_target,
        (),
        bundle.get("world_generation_provenance"),
    ),
)
for key, expected_kind, expected_path, _, entry in governed:
    if not isinstance(entry, dict):
        fail(f"release candidate bundle missing {key}")
    if entry.get("kind") != expected_kind:
        fail(f"release candidate bundle {key} kind must be {expected_kind}")
    bundle_ref_path = resolve_ref(entry.get("ref"), manifest_dir, f"bundle {key} ref")
    bundle_resolved_path = resolve_ref(
        entry.get("resolved_path"), manifest_dir, f"bundle {key} resolved_path"
    )
    if bundle_ref_path != expected_path or bundle_resolved_path != expected_path:
        fail(f"release candidate bundle {key} does not resolve to governed path")

def validate_artifact(key, expected_kind, source, source_root, required_files, entry):
    reject_symlink_components(source, source_root, f"{key} backup path")
    if expected_kind == "directory":
        if not os.path.isdir(source) or os.path.islink(source):
            fail(f"missing governed {key} backup: {source}")
        for child in pathlib.Path(source).rglob("*"):
            if child.is_symlink():
                fail(f"unsafe symlink in {key} backup path: {child}")
        for filename in required_files:
            required_source = os.path.join(source, filename)
            if not os.path.isfile(required_source) or os.path.islink(required_source):
                fail(f"missing governed {key} artifact: {required_source}")
        expected_digest = require_sha256(entry.get("sha256_tree"), f"{key}.sha256_tree")
        actual_digest, actual_file_count, actual_total_bytes = sha256_dir_tree(source)
        if actual_digest != expected_digest:
            fail(
                f"{key} sha256_tree drift: bundle={expected_digest} current={actual_digest}"
            )
        expected_file_count = entry.get("file_count")
        if expected_file_count is not None and expected_file_count != actual_file_count:
            fail(
                f"{key} file_count drift: bundle={expected_file_count} current={actual_file_count}"
            )
        expected_total_bytes = entry.get("total_bytes")
        if expected_total_bytes is not None and expected_total_bytes != actual_total_bytes:
            fail(
                f"{key} total_bytes drift: bundle={expected_total_bytes} current={actual_total_bytes}"
            )
    else:
        if not os.path.isfile(source) or os.path.islink(source):
            fail(f"missing governed {key} backup: {source}")
        expected_digest = require_sha256(entry.get("sha256"), f"{key}.sha256")
        actual_digest = sha256_file(source)
        if actual_digest != expected_digest:
            fail(f"{key} sha256 drift: bundle={expected_digest} current={actual_digest}")

owned = []
for key, expected_kind, target, required_files, entry in governed:
    if not is_within(target, execution_world_dir):
        continue
    relative = os.path.relpath(target, execution_world_dir)
    source = os.path.join(source_execution_world_dir, relative)
    validate_artifact(
        key,
        expected_kind,
        source,
        source_execution_world_dir,
        required_files,
        entry,
    )
    owned.append((key, expected_kind, target, source, required_files, entry))

if mode == "validate":
    sys.exit(0)
if mode != "restore":
    fail(f"unknown governed bootstrap restore mode: {mode}")
if not owned:
    sys.exit(0)
if os.path.lexists(execution_world_dir):
    fail(f"governed bootstrap restore target already exists: {execution_world_dir}")

execution_world_parent = os.path.dirname(execution_world_dir)
os.makedirs(execution_world_parent, exist_ok=True)
stage_dir = tempfile.mkdtemp(
    prefix=f".{os.path.basename(execution_world_dir)}.governed-restore.",
    dir=execution_world_parent,
)
try:
    for _, expected_kind, target, source, _, _ in owned:
        stage_target = os.path.join(stage_dir, os.path.relpath(target, execution_world_dir))
        if expected_kind == "directory":
            os.makedirs(os.path.dirname(stage_target), exist_ok=True)
            shutil.copytree(source, stage_target, copy_function=shutil.copy2)
        else:
            os.makedirs(os.path.dirname(stage_target), exist_ok=True)
            shutil.copy2(source, stage_target)
    for key, expected_kind, target, _, required_files, entry in owned:
        stage_target = os.path.join(stage_dir, os.path.relpath(target, execution_world_dir))
        validate_artifact(
            key,
            expected_kind,
            stage_target,
            stage_dir,
            required_files,
            entry,
        )
    os.replace(stage_dir, execution_world_dir)
    stage_dir = None
finally:
    if stage_dir is not None:
        shutil.rmtree(stage_dir, ignore_errors=True)
PY
}

reset_local_state() {
  local local_env=$1
  local backup_dir=$2

  local local_stack_root node_id execution_world_dir execution_records_dir simulator_dir
  local replication_root runtime_root execution_bridge_state_path
  local storage_root manifest_path governed_restore_source

  local_stack_root=$(resolved_env_value "$local_env" STACK_ROOT)
  node_id=$(resolved_env_value "$local_env" NODE_ID)
  execution_world_dir=$(resolved_env_value "$local_env" EXECUTION_WORLD_DIR)
  simulator_dir="${execution_world_dir}-simulator-mirror"
  execution_records_dir=$(resolved_env_value "$local_env" EXECUTION_RECORDS_DIR)
  storage_root=$(resolved_env_value "$local_env" STORAGE_ROOT)
  replication_root=$(optional_resolved_env_value "$local_env" REPLICATION_ROOT)
  if [[ -z "$replication_root" ]]; then
    replication_root="$local_stack_root/output/node-distfs/$node_id"
  fi
  runtime_root=$(optional_resolved_env_value "$local_env" RUNTIME_ROOT)
  execution_bridge_state_path=$(execution_bridge_state_path_for_root "$local_stack_root" "$node_id" "$runtime_root")
  if raw_value "$local_env" NETWORK_TIER_MANIFEST_PATH >/dev/null 2>&1; then
    manifest_path=$(resolved_env_value "$local_env" NETWORK_TIER_MANIFEST_PATH)
    require_file "$manifest_path"
  else
    manifest_path=""
  fi

  if [[ -z "$backup_dir" ]]; then
    backup_dir="$local_stack_root/backups/local-observer-state-reset-$(date +%Y%m%d-%H%M%S)"
  fi

  mkdir -p "$backup_dir"

  if [[ -n "$manifest_path" ]]; then
    governed_restore_source="$execution_world_dir"
    if [[ -e "$backup_dir/execution-world" || -L "$backup_dir/execution-world" ]]; then
      governed_restore_source="$backup_dir/execution-world"
    fi
    restore_governed_bootstrap_artifacts \
      validate \
      "$manifest_path" \
      "$execution_world_dir" \
      "$governed_restore_source"
  fi

  backup_and_remove_path "$execution_world_dir" "$backup_dir/execution-world"
  backup_and_remove_path "$simulator_dir" "$backup_dir/execution-world-simulator-mirror"
  backup_and_remove_path "$execution_records_dir" "$backup_dir/execution-records"
  backup_and_remove_path "$storage_root" "$backup_dir/storage"
  backup_and_remove_path "$replication_root" "$backup_dir/replication-root"
  if [[ -n "$runtime_root" ]]; then
    backup_and_remove_path "$runtime_root" "$backup_dir/runtime-root"
  fi
  backup_and_remove_path \
    "$execution_bridge_state_path" \
    "$backup_dir/chain-runtime/$node_id/$(basename "$execution_bridge_state_path")"
  if [[ -n "$manifest_path" ]]; then
    restore_governed_bootstrap_artifacts \
      restore \
      "$manifest_path" \
      "$execution_world_dir" \
      "$backup_dir/execution-world"
  fi
  printf 'backup_dir=%s\n' "$backup_dir"
}

localize_manifest_runtime_refs() {
  local manifest_source=$1
  local manifest_dest=$2
  local governance_stage=${3:-}
  local preflight_only=${4:-0}

  python3 - "$manifest_source" "$manifest_dest" "$repo_root" "$governance_stage" "$preflight_only" <<'PY'
import json
import os
import shutil
import sys

manifest_source, manifest_dest, repo_root, governance_stage, preflight_only = sys.argv[1:6]
manifest_dest = os.path.abspath(manifest_dest)
manifest_dir = os.path.dirname(manifest_dest)
manifest_source = os.path.abspath(manifest_source)
manifest_source_dir = os.path.dirname(manifest_source)

def resolve_ref(raw_ref, context_dir=manifest_source_dir):
    if os.path.isabs(raw_ref):
        return raw_ref
    candidates = [
        os.path.join(context_dir, raw_ref),
        os.path.join(os.path.dirname(context_dir), raw_ref),
        os.path.join(repo_root, raw_ref),
    ]
    for candidate in candidates:
        if os.path.exists(candidate):
            return candidate
    return candidates[0]

with open(manifest_source, "r", encoding="utf-8") as fh:
    data = json.load(fh)

runtime_refs = data.get("runtime_refs", {})

def confined_manifest_target_ref(raw_ref, key):
    if not isinstance(raw_ref, str) or not raw_ref:
        raise SystemExit(f"manifest runtime ref escapes localization root for {key}: {raw_ref}")
    if os.path.isabs(raw_ref):
        target_ref = os.path.basename(os.path.normpath(raw_ref))
    else:
        target_ref = os.path.normpath(raw_ref.replace("\\", os.path.sep))
    target = os.path.abspath(os.path.join(manifest_dir, target_ref))
    try:
        contained = os.path.commonpath((manifest_dir, target)) == manifest_dir
    except ValueError:
        contained = False
    if not target_ref or target_ref in (".", "..") or not contained:
        raise SystemExit(f"manifest runtime ref escapes localization root for {key}: {raw_ref}")
    return target_ref, target

generated_runtime_ref_preflight = {}
for key in ("generated_world_sidecar_ref", "world_generation_provenance_ref"):
    ref = runtime_refs.get(key)
    if not ref:
        continue
    source = os.path.abspath(resolve_ref(ref))
    target_ref, target = confined_manifest_target_ref(ref, key)
    if key == "generated_world_sidecar_ref":
        source_exists = os.path.isdir(source)
    else:
        source_exists = os.path.isfile(source)
    if not source_exists:
        raise SystemExit(f"missing manifest runtime ref source: {source}")
    if source == target:
        raise SystemExit(f"ref source and localized target must differ: {source}")
    generated_runtime_ref_preflight[key] = (source, target_ref, target)

if preflight_only == "1":
    raise SystemExit(0)

localized_sources = {}
for key in ("release_candidate_bundle_ref", "genesis_ref", "bootstrap_peer_ref"):
    ref = runtime_refs.get(key)
    if not ref:
        continue
    source = resolve_ref(ref)
    if not os.path.isfile(source):
        raise SystemExit(f"missing manifest runtime ref source: {source}")
    source = os.path.abspath(source)
    target_name = os.path.basename(ref)
    target = os.path.join(manifest_dir, target_name)
    os.makedirs(os.path.dirname(target), exist_ok=True)
    if source != os.path.abspath(target):
        shutil.copy2(source, target)
    localized_sources[key] = source
    runtime_refs[key] = target_name

genesis_ref = runtime_refs.get("genesis_ref")
genesis_source = localized_sources.get("genesis_ref")
if genesis_ref and genesis_source:
    genesis_path = os.path.join(manifest_dir, genesis_ref)
    with open(genesis_path, "r", encoding="utf-8") as fh:
        genesis = json.load(fh)
    bootstrap_refs = genesis.get("governance_bootstrap_refs")
    if isinstance(bootstrap_refs, dict):
        evidence_dir = os.path.join(manifest_dir, "doc", "testing", "evidence")
        target_sources = {}
        for key, ref in bootstrap_refs.items():
            if not isinstance(ref, str) or not ref:
                continue
            source = os.path.abspath(resolve_ref(ref, os.path.dirname(genesis_source)))
            if not os.path.isfile(source):
                raise SystemExit(f"missing genesis governance ref source for {key}: {source}")
            target_name = os.path.basename(os.path.normpath(ref))
            if not target_name or target_name in (".", os.path.sep):
                raise SystemExit(f"invalid genesis governance ref for {key}: {ref}")
            target = os.path.abspath(os.path.join(evidence_dir, target_name))
            previous_source = target_sources.get(target)
            if previous_source and previous_source != source:
                raise SystemExit(
                    f"genesis governance refs collide at localized target {target}: "
                    f"{previous_source} and {source}"
                )
            target_sources[target] = source
            os.makedirs(os.path.dirname(target), exist_ok=True)
            staged_source = os.path.join(governance_stage, target_name) if governance_stage else source
            if not os.path.isfile(staged_source):
                raise SystemExit(f"staged genesis governance ref source missing for {key}: {staged_source}")
            if os.path.abspath(staged_source) != target:
                shutil.copy2(staged_source, target)
            if not os.path.isfile(target):
                raise SystemExit(f"localized genesis governance ref target missing for {key}: {target}")
            bootstrap_refs[key] = target
        with open(genesis_path, "w", encoding="utf-8") as fh:
            json.dump(genesis, fh, ensure_ascii=True, indent=2)
            fh.write("\n")

for key in ("generated_world_sidecar_ref", "world_generation_provenance_ref"):
    preflight = generated_runtime_ref_preflight.get(key)
    if preflight is None:
        continue
    source, target_ref, target = preflight
    os.makedirs(os.path.dirname(target), exist_ok=True)
    if key == "generated_world_sidecar_ref":
        if os.path.exists(target):
            shutil.rmtree(target)
        shutil.copytree(source, target)
    else:
        shutil.copy2(source, target)
    runtime_refs[key] = target_ref

bundle_ref = runtime_refs.get("release_candidate_bundle_ref")
if bundle_ref:
    bundle_path = os.path.join(manifest_dir, bundle_ref)
    with open(bundle_path, "r", encoding="utf-8") as fh:
        bundle = json.load(fh)
    sidecar_ref = runtime_refs.get("generated_world_sidecar_ref")
    if sidecar_ref and isinstance(bundle.get("generated_world_sidecar"), dict):
        bundle["generated_world_sidecar"]["ref"] = sidecar_ref
        bundle["generated_world_sidecar"]["resolved_path"] = os.path.join(manifest_dir, sidecar_ref)
    provenance_ref = runtime_refs.get("world_generation_provenance_ref")
    if provenance_ref and isinstance(bundle.get("world_generation_provenance"), dict):
        bundle["world_generation_provenance"]["ref"] = provenance_ref
        bundle["world_generation_provenance"]["resolved_path"] = os.path.join(
            manifest_dir, provenance_ref
        )
    with open(bundle_path, "w", encoding="utf-8") as fh:
        json.dump(bundle, fh, ensure_ascii=True, indent=2)
        fh.write("\n")

with open(manifest_dest, "w", encoding="utf-8") as fh:
    json.dump(data, fh, ensure_ascii=True, indent=2)
    fh.write("\n")
PY
}

preflight_manifest_governance_refs() {
  local manifest_source=$1
  local stage_dir=$2

  python3 - "$manifest_source" "$repo_root" "$stage_dir" <<'PY'
import hashlib
import json
import os
import shutil
import sys

manifest_source, repo_root, stage_dir = map(os.path.abspath, sys.argv[1:4])
manifest_dir = os.path.dirname(manifest_source)

def resolve_ref(raw_ref, context_dir):
    if os.path.isabs(raw_ref):
        return os.path.abspath(raw_ref)
    candidates = (
        os.path.join(context_dir, raw_ref),
        os.path.join(os.path.dirname(context_dir), raw_ref),
        os.path.join(repo_root, raw_ref),
    )
    for candidate in candidates:
        if os.path.exists(candidate):
            return os.path.abspath(candidate)
    return os.path.abspath(candidates[0])

with open(manifest_source, "r", encoding="utf-8") as fh:
    manifest = json.load(fh)
genesis_ref = manifest.get("runtime_refs", {}).get("genesis_ref")
if not genesis_ref:
    raise SystemExit("manifest missing runtime_refs.genesis_ref")
genesis_source = resolve_ref(str(genesis_ref), manifest_dir)
if not os.path.isfile(genesis_source):
    raise SystemExit(f"missing manifest runtime ref source: {genesis_source}")
with open(genesis_source, "r", encoding="utf-8") as fh:
    genesis = json.load(fh)
refs = genesis.get("governance_bootstrap_refs", {})
if not isinstance(refs, dict):
    raise SystemExit("genesis governance_bootstrap_refs must be an object")
targets = {}
for key, raw_ref in refs.items():
    if not isinstance(raw_ref, str) or not raw_ref:
        continue
    source = resolve_ref(raw_ref, os.path.dirname(genesis_source))
    if not os.path.isfile(source):
        raise SystemExit(f"missing genesis governance ref source for {key}: {source}")
    target_name = os.path.basename(os.path.normpath(raw_ref))
    target = os.path.join(stage_dir, target_name)
    previous = targets.get(target_name)
    if previous and previous != source:
        raise SystemExit(
            f"genesis governance refs collide at localized target {target_name}: {previous} and {source}"
        )
    targets[target_name] = source
    shutil.copy2(source, target)
    if hashlib.sha256(open(source, "rb").read()).digest() != hashlib.sha256(open(target, "rb").read()).digest():
        raise SystemExit(f"staged genesis governance ref integrity mismatch for {key}: {source}")
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

    if [[ "$mode" == "render" ]]; then
      rendered_env=$(render_env "$local_env" "$sequencer_env" "$storage_env" "$manifest_path" 0)
      write_rendered_env "$rendered_env" "$out_path"
      exit 0
    fi

    require_file "$start_script_source"

    local_stack_root=$(required_value "$local_env" STACK_ROOT)
    seq_validators=$(required_value "$sequencer_env" NODE_VALIDATORS_CSV)
    storage_validators=$(required_value "$storage_env" NODE_VALIDATORS_CSV)
    [[ "$seq_validators" == "$storage_validators" ]] || die "legacy NODE_VALIDATORS_CSV mismatch between ECS env files"
    seq_signers=$(required_value "$sequencer_env" NODE_VALIDATOR_SIGNERS_CSV)
    storage_signers=$(required_value "$storage_env" NODE_VALIDATOR_SIGNERS_CSV)
    [[ "$seq_signers" == "$storage_signers" ]] || die "legacy NODE_VALIDATOR_SIGNERS_CSV mismatch between ECS env files"
    genesis_validator_registry_path="$local_stack_root/config/genesis-validator-registry.json"
    rendered_env=$(render_env "$local_env" "$sequencer_env" "$storage_env" "$manifest_path" 1)
    if [[ -z "$manifest_dest" && -n "$manifest_source" ]]; then
      manifest_dest=$manifest_path
    fi
    if [[ -z "$start_script_dest" ]]; then
      start_script_dest="$local_stack_root/bin/start-node.sh"
    fi
    if [[ -z "$backup_dir" ]]; then
      backup_dir="$local_stack_root/backups/local-observer-contract-sync-$(date +%Y%m%d-%H%M%S)"
    fi

    governance_stage=""
    if [[ -n "$manifest_source" ]]; then
      require_file "$manifest_source"
      [[ -n "$manifest_dest" ]] || die "--manifest-dest is required when --manifest-source is set"
      localize_manifest_runtime_refs "$manifest_source" "$manifest_dest" "" 1
      governance_stage=$(mktemp -d "${TMPDIR:-/tmp}/oasis7-observer-governance-stage.XXXXXX")
      trap 'rm -rf "${governance_stage:-}"' EXIT
      preflight_manifest_governance_refs "$manifest_source" "$governance_stage"
    fi

    mkdir -p "$backup_dir"

    cp "$local_env" "$backup_dir/node.env.before"
    tmp_env="$backup_dir/node.env.rendered"
    printf '%s' "$rendered_env" > "$tmp_env"
    cp "$tmp_env" "$local_env"
    write_genesis_validator_registry \
      "$seq_validators" \
      "$seq_signers" \
      "$genesis_validator_registry_path"

    if [[ -n "$manifest_source" ]]; then
      mkdir -p "$(dirname "$manifest_dest")"
      if [[ -f "$manifest_dest" ]]; then
        cp "$manifest_dest" "$backup_dir/$(basename "$manifest_dest").before"
      fi
      localize_manifest_runtime_refs "$manifest_source" "$manifest_dest" "$governance_stage"
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
    printf 'installed genesis validator registry to %s\n' "$genesis_validator_registry_path"
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
    # Deprecated parser tombstone: retain argument parsing for existing callers,
    # but reject before resolving local paths or mutating local state.
    die "state_sync_checkpoint_drift: unsafe-live-seed-disabled; run reset-state, then start the observer for signed protocol checkpoint sync"
    ;;
  *)
    die "unknown mode: $mode"
    ;;
esac
