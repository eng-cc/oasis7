#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
SYNC_SCRIPT="$ROOT_DIR/scripts/p2p-public-testnet-local-observer-sync.sh"
START_SCRIPT="$ROOT_DIR/scripts/p2p-triad-node-start.sh"
BASE_MANIFEST="$ROOT_DIR/doc/testing/evidence/public-testnet-governed-bootstrap-manifest-2026-06-06.json"
BASE_GENESIS="$ROOT_DIR/doc/testing/evidence/public-testnet-governed-bootstrap-genesis-2026-06-06.json"
BASE_BOOTSTRAP="$ROOT_DIR/doc/testing/evidence/public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.txt"

tmp_dir=$(mktemp -d "${TMPDIR:-/tmp}/oasis7-local-observer-runtime-preflight.XXXXXX")
cleanup() {
  if [[ -n "${observer_pid:-}" ]]; then
    kill "$observer_pid" 2>/dev/null || true
    wait "$observer_pid" 2>/dev/null || true
  fi
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

stack_root="$tmp_dir/observer-stack"
source_config="$tmp_dir/source-config"
mkdir -p "$source_config/generated-world" "$source_config/scripts" "$source_config/config"
read -r status_port gossip_port < <(python3 - <<'PY'
import socket

ports = []
for _ in range(2):
    sock = socket.socket()
    sock.bind(("127.0.0.1", 0))
    ports.append(str(sock.getsockname()[1]))
    sock.close()
print(*ports)
PY
)

cat >"$tmp_dir/local.env" <<EOF
HOST_LABEL=isolated-local-observer
SERVICE_NAME=oasis7-triad-local-observer.service
STACK_ROOT=$stack_root
NODE_ID=triad-testnet-local
WORLD_ID=oasis7-public-testnet-governed-20260606
NODE_ROLE=observer
STATUS_BIND=127.0.0.1:$status_port
NODE_GOSSIP_BIND=127.0.0.1:$gossip_port
NODE_AUTO_ATTEST_FLAG=--node-no-auto-attest-all
CONFIG_PATH=\$STACK_ROOT/config.toml
EXECUTION_WORLD_DIR=\$STACK_ROOT/data/execution-world
EXECUTION_RECORDS_DIR=\$STACK_ROOT/data/execution-records
STORAGE_ROOT=\$STACK_ROOT/data/storage
RUNTIME_ROOT=\$STACK_ROOT/data/runtime-root
REPLICATION_ROOT=\$STACK_ROOT/data/replication-root
REPLICATION_NETWORK_LISTEN_ADDRS_CSV=/ip4/127.0.0.1/tcp/0
TRAFFIC_PROFILE=triad_low_traffic
TRAFFIC_MONITOR_ENABLE=0
TRAFFIC_MONITOR_INTERVAL_SECS=30
TRAFFIC_MONITOR_WINDOW_MINUTES=10
TRAFFIC_MONITOR_TOP_N=5
TRAFFIC_MONITOR_OUTPUT_DIR=\$STACK_ROOT/output/traffic-monitor
P2P_NODE_ROLE=observer_light
EOF

cat >"$tmp_dir/sequencer.env" <<'EOF'
WORLD_ID=oasis7-public-testnet-governed-20260606
NODE_ROLE=sequencer
STORAGE_PROFILE=dev_local
NODE_GOSSIP_PEERS_CSV=127.0.0.1:1
REPLICATION_NETWORK_BOOTSTRAP_PEERS_CSV=/ip4/127.0.0.1/tcp/1
REPLICATION_REMOTE_WRITERS_CSV=e01e5c34dee2da3087653bc4cec02be01632f56250a800994c96ea44ae6f3690
POS_SLOT_CLOCK_GENESIS_UNIX_MS=1779068751846
POS_ADAPTIVE_TICK_SCHEDULER=0
NODE_TICK_MS=200
POS_SLOT_DURATION_MS=1200
POS_TICKS_PER_SLOT=6
POS_PROPOSAL_TICK_PHASE=0
POS_MAX_PAST_SLOT_LAG=12
REWARD_RUNTIME_ENABLE=0
REWARD_RUNTIME_EPOCH_DURATION_SECS=60
REWARD_POINTS_PER_CREDIT=100
REWARD_RUNTIME_AUTO_REDEEM=0
NODE_VALIDATORS_CSV=triad-testnet-sequencer:100,triad-testnet-storage:50
NODE_VALIDATOR_SIGNERS_CSV=triad-testnet-sequencer:e01e5c34dee2da3087653bc4cec02be01632f56250a800994c96ea44ae6f3690,triad-testnet-storage:1f530cae002d7adb9a6c3dd8f4bc861226f112f88fdd252b28b6494019e21c33
EOF

cat >"$tmp_dir/storage.env" <<'EOF'
WORLD_ID=oasis7-public-testnet-governed-20260606
NODE_ROLE=storage
STORAGE_PROFILE=dev_local
NODE_GOSSIP_PEERS_CSV=127.0.0.1:2
REPLICATION_NETWORK_BOOTSTRAP_PEERS_CSV=/ip4/127.0.0.1/tcp/2
REPLICATION_REMOTE_WRITERS_CSV=1f530cae002d7adb9a6c3dd8f4bc861226f112f88fdd252b28b6494019e21c33
POS_SLOT_CLOCK_GENESIS_UNIX_MS=1779068751846
POS_ADAPTIVE_TICK_SCHEDULER=0
NODE_TICK_MS=200
POS_SLOT_DURATION_MS=1200
POS_TICKS_PER_SLOT=6
POS_PROPOSAL_TICK_PHASE=0
POS_MAX_PAST_SLOT_LAG=12
REWARD_RUNTIME_ENABLE=0
REWARD_RUNTIME_EPOCH_DURATION_SECS=60
REWARD_POINTS_PER_CREDIT=100
REWARD_RUNTIME_AUTO_REDEEM=0
NODE_VALIDATORS_CSV=triad-testnet-sequencer:100,triad-testnet-storage:50
NODE_VALIDATOR_SIGNERS_CSV=triad-testnet-sequencer:e01e5c34dee2da3087653bc4cec02be01632f56250a800994c96ea44ae6f3690,triad-testnet-storage:1f530cae002d7adb9a6c3dd8f4bc861226f112f88fdd252b28b6494019e21c33
EOF

cp "$BASE_GENESIS" "$source_config/genesis.json"
cp "$BASE_BOOTSTRAP" "$source_config/peers.txt"
python3 - "$source_config/config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json" <<'PY'
import json
import pathlib
import sys

validators = [
    {
        "node_id": "triad-testnet-sequencer",
        "scheme": "ed25519",
        "finality_signer_public_key": "e01e5c34dee2da3087653bc4cec02be01632f56250a800994c96ea44ae6f3690",
        "stake": 100,
    },
    {
        "node_id": "triad-testnet-storage",
        "scheme": "ed25519",
        "finality_signer_public_key": "1f530cae002d7adb9a6c3dd8f4bc861226f112f88fdd252b28b6494019e21c33",
        "stake": 50,
    },
]
pathlib.Path(sys.argv[1]).write_text(
    json.dumps(
        {
            "slot_id": "governance.finality.v1",
            "threshold": 2,
            "threshold_bps": 0,
            "validators": validators,
        },
        ensure_ascii=True,
        indent=2,
        sort_keys=True,
    )
    + "\n",
    encoding="utf-8",
)
PY
printf '{"world":"isolated-runtime-preflight"}\n' >"$source_config/generated-world/snapshot.json"
printf '[]\n' >"$source_config/generated-world/journal.json"
printf '{"generated":true}\n' >"$source_config/world-generation-provenance.json"
cat >"$source_config/bundle.json" <<'EOF'
{
  "schema_version": "oasis7.release_candidate_bundle.v1",
  "runtime_build": {"sha256": "0000000000000000000000000000000000000000000000000000000000000000"},
  "generated_world_sidecar": {
    "kind": "directory",
    "ref": "generated-world",
    "resolved_path": "generated-world"
  },
  "world_generation_provenance": {
    "kind": "file",
    "ref": "world-generation-provenance.json",
    "resolved_path": "world-generation-provenance.json"
  }
}
EOF
python3 - "$source_config/bundle.json" "$source_config/generated-world" "$source_config/world-generation-provenance.json" <<'PY'
import hashlib
import json
import pathlib
import sys

bundle_path, sidecar_path, provenance_path = map(pathlib.Path, sys.argv[1:])
combined = hashlib.sha256()
file_count = 0
total_bytes = 0
for child in sorted(path for path in sidecar_path.rglob("*") if path.is_file()):
    relative = child.relative_to(sidecar_path).as_posix()
    digest = hashlib.sha256(child.read_bytes()).hexdigest()
    size = child.stat().st_size
    combined.update(relative.encode())
    combined.update(b"\0")
    combined.update(digest.encode())
    combined.update(b"\0")
    combined.update(str(size).encode())
    combined.update(b"\n")
    file_count += 1
    total_bytes += size
bundle = json.loads(bundle_path.read_text(encoding="utf-8"))
bundle["generated_world_sidecar"].update(
    sha256_tree=combined.hexdigest(), file_count=file_count, total_bytes=total_bytes
)
bundle["world_generation_provenance"].update(
    sha256=hashlib.sha256(provenance_path.read_bytes()).hexdigest()
)
bundle_path.write_text(json.dumps(bundle, indent=2) + "\n", encoding="utf-8")
PY

registry_sha256=$(shasum -a 256 "$source_config/config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json" | awk '{print $1}')
registry_semantic_sha256=$(python3 - "$source_config/config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json" <<'PY'
import hashlib
import json
import pathlib
import sys

registry = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
canonical = {
    "signer_bindings": {
        f"governance.finality.v1.{item['node_id']}": str(item["finality_signer_public_key"]).lower()
        for item in registry["validators"]
    },
    "slot_id": registry["slot_id"],
    "threshold": registry["threshold"],
    "threshold_bps": registry["threshold_bps"],
    "validator_stakes": {
        f"governance.finality.v1.{item['node_id']}": item["stake"]
        for item in registry["validators"]
    },
}
print(hashlib.sha256(json.dumps(canonical, sort_keys=True, separators=(",", ":")).encode()).hexdigest())
PY
)
python3 - "$BASE_MANIFEST" "$source_config/manifest.json" "$source_config/bundle.json" "$registry_sha256" "$registry_semantic_sha256" <<'PY'
import json
import pathlib
import sys

base_path, output_path, bundle_path, registry_sha, registry_semantic_sha = sys.argv[1:]
manifest = json.loads(pathlib.Path(base_path).read_text(encoding="utf-8"))
manifest["network_id"] = "oasis7-public-testnet-governed-20260606"
manifest["chain_id"] = "oasis7-public-testnet-governed-20260606"
manifest["validator_policy"]["target_validator_count"] = 2
manifest["validator_policy"]["allow_observer_nodes"] = True
source_dir = pathlib.Path(bundle_path).resolve().parent
manifest["runtime_refs"] = {
    "release_candidate_bundle_ref": str(pathlib.Path(bundle_path).resolve()),
    "genesis_ref": str((source_dir / "genesis.json").resolve()),
    "bootstrap_peer_ref": str((source_dir / "peers.txt").resolve()),
    "generated_world_sidecar_ref": str((source_dir / "generated-world").resolve()),
    "world_generation_provenance_ref": str(
        (source_dir / "world-generation-provenance.json").resolve()
    ),
}
manifest["deployment_validator_registry"] = {
    "ref": "config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json",
    "sha256": registry_sha,
    "semantic_sha256": registry_semantic_sha,
}
pathlib.Path(output_path).write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
PY

# apply is the contract under test: the env consumed below is the exact env
# materialized by the local-observer synchronizer, including localized authority.
DEPLOYMENT_INVENTORY_PATH="$tmp_dir/inherited-inventory-that-must-not-win" \
GENESIS_VALIDATOR_REGISTRY_PATH="$tmp_dir/inherited-registry-that-must-not-win" \
bash "$SYNC_SCRIPT" apply \
  --local-env "$tmp_dir/local.env" \
  --sequencer-env "$tmp_dir/sequencer.env" \
  --storage-env "$tmp_dir/storage.env" \
  --manifest-path "$stack_root/config/public-testnet-governed-bootstrap-manifest-2026-06-06.json" \
  --manifest-source "$source_config/manifest.json" \
  --manifest-dest "$stack_root/config/public-testnet-governed-bootstrap-manifest-2026-06-06.json" \
  --start-script-source "$START_SCRIPT" \
  --start-script-dest "$stack_root/bin/start-node.sh" \
  --backup-dir "$tmp_dir/apply-backup" \
  >"$tmp_dir/apply.stdout"

generated_env="$tmp_dir/local.env"
manifest_path=$(awk -F= '$1 == "NETWORK_TIER_MANIFEST_PATH" {print substr($0, index($0, "=") + 1)}' "$generated_env")
registry_path=$(awk -F= '$1 == "GENESIS_VALIDATOR_REGISTRY_PATH" {print substr($0, index($0, "=") + 1)}' "$generated_env")
[[ "$manifest_path" == "$stack_root/config/public-testnet-governed-bootstrap-manifest-2026-06-06.json" ]] \
  || { echo "generated observer env drifted manifest path: $manifest_path" >&2; exit 1; }
if grep -q '^DEPLOYMENT_INVENTORY_PATH=' "$generated_env" || grep -q '^DEPLOYMENT_INVENTORY_SHA256=' "$generated_env"; then
  echo "registry-only observer env unexpectedly contains deployment inventory authority" >&2
  exit 1
fi
bound_registry_ref=$(jq -r '.deployment_validator_registry.ref // empty' "$manifest_path")
expected_registry_path="$stack_root/config/$(basename "$bound_registry_ref")"
[[ -n "$bound_registry_ref" && "$registry_path" == "$expected_registry_path" ]] \
  || { echo "generated observer env drifted registry binding: ref=$bound_registry_ref path=$registry_path" >&2; exit 1; }
for authority_file in "$manifest_path" "$registry_path"; do
  [[ -f "$authority_file" && ! -L "$authority_file" ]] \
    || { echo "generated observer authority is not a regular file: $authority_file" >&2; exit 1; }
done
grep -q "^GENESIS_VALIDATOR_REGISTRY_SHA256=$registry_sha256$" "$generated_env"
grep -q "^GENESIS_VALIDATOR_REGISTRY_SEMANTIC_SHA256=$registry_semantic_sha256$" "$generated_env"
shasum -a 256 "$registry_path" | awk -v expected="$registry_sha256" '$1 == expected {ok=1} END {exit !ok}'
jq -e --arg registry "$registry_sha256" --arg semantic "$registry_semantic_sha256" \
  --arg registry_ref "$bound_registry_ref" \
  '.deployment_validator_registry.ref == $registry_ref and .deployment_validator_registry.sha256 == $registry and .deployment_validator_registry.semantic_sha256 == $semantic and (.deployment_inventory == null)' \
  "$manifest_path" >/dev/null

runtime_bin="${OASIS7_CHAIN_RUNTIME_BIN:-}"
if [[ -z "$runtime_bin" ]]; then
  target_dir=$(cargo metadata --no-deps --format-version 1 | jq -r '.target_directory')
  runtime_bin="$target_dir/debug/oasis7_chain_runtime"
  env -u RUSTC_WRAPPER cargo build -p oasis7 --bin oasis7_chain_runtime --quiet
fi
[[ -x "$runtime_bin" ]] || { echo "missing isolated runtime binary: $runtime_bin" >&2; exit 1; }
runtime_sha=$(shasum -a 256 "$runtime_bin" | awk '{print $1}')
python3 - "$stack_root/config/bundle.json" "$runtime_sha" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
data = json.loads(path.read_text(encoding="utf-8"))
data["runtime_build"]["sha256"] = sys.argv[2]
path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
PY

# A ready marker is emitted after runtime_authority::load_runtime_authority_binding,
# governance bootstrap, runtime.start, and status-server startup.  This is an
# actual isolated runtime admission, not a launcher argv/dry-run assertion.
runtime_stdout="$tmp_dir/runtime.stdout"
env -u DEPLOYMENT_INVENTORY_PATH -u DEPLOYMENT_INVENTORY_SHA256 \
  APP_ROOT="$stack_root" ENV_FILE="$generated_env" BIN="$runtime_bin" \
  "$stack_root/bin/start-node.sh" >"$runtime_stdout" 2>&1 &
observer_pid=$!
ready=0
for _ in $(seq 1 200); do
  if grep -q '^oasis7_chain_runtime ready\.' "$stack_root/logs/chain-runtime.log" 2>/dev/null; then
    ready=1
    break
  fi
  if ! kill -0 "$observer_pid" 2>/dev/null; then
    break
  fi
  sleep 0.1
done
if [[ "$ready" -ne 1 ]]; then
  wait "$observer_pid" 2>/dev/null || true
  cat "$runtime_stdout" >&2 || true
  cat "$stack_root/logs/chain-runtime.log" >&2 || true
  echo "isolated generated-env observer runtime did not reach ready after authority preflight" >&2
  exit 1
fi
kill "$observer_pid" 2>/dev/null || true
wait "$observer_pid" 2>/dev/null || true
observer_pid=""
grep -q '^oasis7_chain_runtime ready\.' "$stack_root/logs/chain-runtime.log"

# Registry bytes, ref, and semantic digest are all independently authenticated
# by the manifest-bound observer authority. Exercise each drift shape against a
# copy containing the already-materialized world and require rejection without
# changing that world.
world_digest() {
  local world_dir=$1
  if [[ -d "$world_dir" ]]; then
    find "$world_dir" -type f -exec shasum -a 256 {} \; | sort
  fi
}

for drift_case in registry_bytes registry_ref registry_semantic_sha256; do
  drift_stack="$tmp_dir/observer-drift-$drift_case-stack"
  drift_env="$tmp_dir/observer-drift-$drift_case.env"
  drift_manifest="$drift_stack/config/$(basename "$manifest_path")"
  drift_registry="$drift_stack/config/$(basename "$registry_path")"
  drift_stderr="$tmp_dir/observer-drift-$drift_case.stderr"
  cp -R "$stack_root" "$drift_stack"
  # The positive admission leaves a ready marker in its log. Start every
  # copied scenario with fresh logs and no inherited launcher process state so
  # rejection is attributable to the current drift case only.
  rm -rf "$drift_stack/logs"
  mkdir -p "$drift_stack/logs"
  sed "s#${stack_root}#${drift_stack}#g" "$generated_env" >"$drift_env"
  case "$drift_case" in
    registry_bytes)
      printf '\nregistry-byte-drift\n' >>"$drift_registry"
      ;;
    registry_ref|registry_semantic_sha256)
      python3 - "$drift_manifest" "$drift_case" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
case = sys.argv[2]
data = json.loads(path.read_text(encoding="utf-8"))
binding = data["deployment_validator_registry"]
if case == "registry_ref":
    binding["ref"] = "config/missing-observer-registry.json"
else:
    binding["semantic_sha256"] = "0" * 64
path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
PY
      ;;
  esac
  world_before=$(world_digest "$drift_stack/data/execution-world")
  if env -u DEPLOYMENT_INVENTORY_PATH -u DEPLOYMENT_INVENTORY_SHA256 \
    APP_ROOT="$drift_stack" ENV_FILE="$drift_env" BIN="$runtime_bin" \
    "$drift_stack/bin/start-node.sh" >"$tmp_dir/observer-drift-$drift_case.stdout" 2>"$drift_stderr"; then
    echo "observer registry drift unexpectedly admitted: $drift_case" >&2
    exit 1
  fi
  if grep -q '^oasis7_chain_runtime ready\.' "$drift_stack/logs/chain-runtime.log" 2>/dev/null; then
    echo "observer registry drift reached runtime ready: $drift_case" >&2
    exit 1
  fi
  world_after=$(world_digest "$drift_stack/data/execution-world")
  [[ "$world_before" == "$world_after" ]] \
    || { echo "observer registry drift mutated execution world: $drift_case" >&2; exit 1; }
done

# Managed triad identities cannot downgrade to registry-only observer authority,
# even when the generated env spoofs NodeRole=observer. Each must fail before a
# runtime command is materialized.
for managed_node_id in triad-testnet-sequencer triad-testnet-storage triad-testnet-validator-47; do
  managed_stack="$tmp_dir/managed-$managed_node_id-stack"
  managed_env="$tmp_dir/managed-$managed_node_id.env"
  managed_stderr="$tmp_dir/managed-$managed_node_id.stderr"
  sed -e 's/^NODE_ID=triad-testnet-local$/NODE_ID='"$managed_node_id"'/' \
    "$generated_env" >"$managed_env"
  if env -u DEPLOYMENT_INVENTORY_PATH -u DEPLOYMENT_INVENTORY_SHA256 \
    APP_ROOT="$managed_stack" ENV_FILE="$managed_env" BIN="$runtime_bin" \
    "$stack_root/bin/start-node.sh" >"$tmp_dir/managed-$managed_node_id.stdout" 2>"$managed_stderr"; then
    echo "managed triad unexpectedly accepted registry-only observer authority: $managed_node_id" >&2
    exit 1
  fi
  grep -q 'managed triad startup requires DEPLOYMENT_INVENTORY_PATH' "$managed_stderr"
  [[ ! -e "$managed_stack/logs/last-command.sh" ]] \
    || { echo "managed triad partial authority materialized a runtime command: $managed_node_id" >&2; exit 1; }
done

echo "ok: generated registry-only observer env admits isolated runtime authority and managed triad remains fail-closed"
