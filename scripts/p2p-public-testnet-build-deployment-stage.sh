#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/p2p-public-testnet-build-deployment-stage.sh \
    --runtime-build-ref <path> \
    --bootstrap-peers-file <path> \
    --sequencer-finality-public-key <hex> \
    --storage-finality-public-key <hex> \
    [--extra-validator <node_id:public_key[:stake]>]... \
    [--validator-47-identity-dir <already-staged identity dir>] \
    --out-dir <path> \
    [--validator-pair-provenance-ref <path>] \
    [--track public_testnet_rehearsal|staging|canary]

  ./scripts/p2p-public-testnet-build-deployment-stage.sh \
    --runtime-build-ref <path> \
    --bootstrap-peers-file <path> \
    --sequencer-public-key <hex> \
    --storage-public-key <hex> \
    [--extra-validator <node_id:public_key[:stake]>]... \
    --out-dir <path> \
    [--validator-pair-provenance-ref <path>] \
    [--track public_testnet_rehearsal|staging|canary]

Description:
  Build a deployment-only public_testnet stage from the current validator
  signer truth. Public key flags must be consensus/finality signer keys, not
  libp2p/node identity keys. This stage preserves the frozen public testnet
  chain/world identity but regenerates:
    - validator registry
    - genesis with deployment-only validator-registry ref
    - governed bootstrap world
    - release-candidate bundle pinned to the provided runtime build

  The output directory contains:
    <out-dir>/config/
    <out-dir>/identity/ (validator-47 only; imported key and public receipt)
    <out-dir>/generated-world/world/ (canonical bootstrap execution world)
    <out-dir>/generated-world/generated-scenario-world/ (world sidecar)
    <out-dir>/generated-world/world-generation-provenance.json
    <out-dir>/deployment-truth.md

  Pass <out-dir>/generated-world directly as --world-dir to
  p2p-public-testnet-bootstrap-fresh-validator-host.sh.  Bootstrap consumes
  this exact nested stage layout and flattens the canonical world into the
  host's staged-world directory; do not hand-copy or rewrite the paths.
EOF
}

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

require_command() {
  local name=$1
  command -v "$name" >/dev/null 2>&1 || die "missing command: $name"
}

require_file() {
  local path=$1
  [[ -f "$path" ]] || die "missing file: $path"
}

require_non_empty() {
  local flag=$1
  local value=$2
  [[ -n "$value" ]] || die "missing required option: $flag"
}

sha256_file() {
  shasum -a 256 "$1" | awk '{print $1}'
}

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

# Validator-47's generated environment is consumed directly by
# p2p-triad-node-start.sh, which runs with `set -u`.  Keep the staged paths
# and explicit runtime knobs here so a fresh host does not depend on an
# inherited pair environment.  PoS timing is sourced from the repo-owned
# defaults file, the same file embedded by the runtime.
readonly TRIAD_CHAIN_POS_DEFAULTS_PATH="$repo_root/config/chain-pos-defaults.env"
[[ -f "$TRIAD_CHAIN_POS_DEFAULTS_PATH" ]] || die "missing chain PoS defaults: $TRIAD_CHAIN_POS_DEFAULTS_PATH"
# shellcheck source=/dev/null
source "$TRIAD_CHAIN_POS_DEFAULTS_PATH"
readonly VALIDATOR_47_POS_SLOT_DURATION_MS="${POS_SLOT_DURATION_MS:?missing POS_SLOT_DURATION_MS in $TRIAD_CHAIN_POS_DEFAULTS_PATH}"
readonly VALIDATOR_47_POS_TICKS_PER_SLOT="${POS_TICKS_PER_SLOT:?missing POS_TICKS_PER_SLOT in $TRIAD_CHAIN_POS_DEFAULTS_PATH}"
readonly VALIDATOR_47_POS_PROPOSAL_TICK_PHASE="${POS_PROPOSAL_TICK_PHASE:?missing POS_PROPOSAL_TICK_PHASE in $TRIAD_CHAIN_POS_DEFAULTS_PATH}"
readonly VALIDATOR_47_POS_MAX_PAST_SLOT_LAG="${POS_MAX_PAST_SLOT_LAG:?missing POS_MAX_PAST_SLOT_LAG in $TRIAD_CHAIN_POS_DEFAULTS_PATH}"

# This inventory is the operator authority for the managed public-testnet
# triad.  The staged copy and its digest travel with the deployment truth so
# a host cannot silently reinterpret a pair stage as validator-47.
readonly TRIAD_INVENTORY_RELATIVE="scripts/public-testnet-validator-triad-inventory.v1.json"
readonly TRIAD_INVENTORY_PATH="$repo_root/$TRIAD_INVENTORY_RELATIVE"
readonly TRIAD_INVENTORY_FILE="public-testnet-validator-triad-inventory.v1.json"
# Keep stage generation pinned to the same immutable inventory that bootstrap,
# readback, and fleet health consume.  The source registry is an input to the
# generated file; it is not the runtime registry digest.
readonly TRIAD_INVENTORY_SHA256="3313a899630e3013d623adfee252556a124c25d059406bcf98a541ae2fcdacd5"
readonly TRIAD_SOURCE_REGISTRY_RELATIVE="doc/testing/evidence/public-testnet-governed-bootstrap-validator-triad-registry-2026-09-15.json"
readonly TRIAD_SOURCE_REGISTRY_PATH="$repo_root/$TRIAD_SOURCE_REGISTRY_RELATIVE"
readonly TRIAD_SOURCE_REGISTRY_FILE="public-testnet-governed-bootstrap-validator-triad-registry-2026-09-15.json"
readonly VALIDATOR_47_NODE_ID="triad-testnet-validator-47"
# Inventory role is validator; the runtime consumes the supported storage role
# and the independent full-storage P2P provider role below.
readonly VALIDATOR_47_INVENTORY_ROLE="validator" # NODE_ROLE=validator (inventory authority)
readonly VALIDATOR_47_RUNTIME_NODE_ROLE="storage"
readonly VALIDATOR_47_P2P_NODE_ROLE="full_storage"
readonly VALIDATOR_47_SERVICE="oasis7-triad-validator-47.service"
readonly VALIDATOR_47_STATUS_BIND="0.0.0.0:6634"
readonly VALIDATOR_47_GOSSIP_BIND="0.0.0.0:6834"
readonly VALIDATOR_47_WORLD_ID="oasis7-public-testnet-governed-20260606"
readonly VALIDATOR_47_MANIFEST_PATH="config/public-testnet-governed-bootstrap-manifest-2026-06-06.json"
readonly VALIDATOR_47_REGISTRY_PATH="config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json"
readonly VALIDATOR_47_CONFIG_PATH="config/node-keypair.toml"
readonly VALIDATOR_47_EXECUTION_WORLD_DIR="staged-world"
readonly VALIDATOR_47_EXECUTION_RECORDS_DIR="data/execution-records"
readonly VALIDATOR_47_STORAGE_ROOT="data/storage"
readonly VALIDATOR_47_STORAGE_PROFILE="release_default"
# These explicit values match the public-testnet long-run runtime profile.
readonly VALIDATOR_47_NODE_TICK_MS="200"
readonly VALIDATOR_47_REWARD_RUNTIME_ENABLE="1"
readonly VALIDATOR_47_REWARD_RUNTIME_EPOCH_DURATION_SECS="60"
readonly VALIDATOR_47_REWARD_POINTS_PER_CREDIT="100"
readonly VALIDATOR_47_REWARD_RUNTIME_AUTO_REDEEM="0"
readonly VALIDATOR_47_IDENTITY_KEY_FILE="node-keypair.toml"
readonly VALIDATOR_47_IDENTITY_RECEIPT_FILE="identity-receipt.json"

runtime_build_ref=""
bootstrap_peers_file=""
sequencer_public_key=""
storage_public_key=""
extra_validators=()
out_dir=""
validator_pair_provenance_ref=""
validator_47_identity_dir=""

base_genesis="doc/testing/evidence/public-testnet-governed-bootstrap-genesis-2026-06-06.json"
base_manifest="doc/testing/evidence/public-testnet-governed-bootstrap-manifest-2026-06-06.json"
candidate_id="public-testnet-governed-bootstrap-20260606"
track="public_testnet_rehearsal"
sequencer_node_id="triad-testnet-sequencer"
storage_node_id="triad-testnet-storage"
stake="100"
quorum_numerator="2"
quorum_denominator="3"
governance_signer_count="3"
governance_threshold="2"
governance_threshold_bps="6667"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --runtime-build-ref)
      runtime_build_ref=${2:-}
      shift 2
      ;;
    --bootstrap-peers-file)
      bootstrap_peers_file=${2:-}
      shift 2
      ;;
    --sequencer-node-keypair)
      die "--sequencer-node-keypair is not supported for deployment signer truth; pass --sequencer-finality-public-key"
      ;;
    --storage-node-keypair)
      die "--storage-node-keypair is not supported for deployment signer truth; pass --storage-finality-public-key"
      ;;
    --sequencer-public-key|--sequencer-finality-public-key|--sequencer-consensus-signer-public-key)
      sequencer_public_key=${2:-}
      shift 2
      ;;
    --storage-public-key|--storage-finality-public-key|--storage-consensus-signer-public-key)
      storage_public_key=${2:-}
      shift 2
      ;;
    --extra-validator)
      extra_validators+=("${2:-}")
      shift 2
      ;;
    --validator-47-identity-dir)
      validator_47_identity_dir=${2:-}
      shift 2
      ;;
    --out-dir)
      out_dir=${2:-}
      shift 2
      ;;
    --validator-pair-provenance-ref)
      validator_pair_provenance_ref=${2:-}
      shift 2
      ;;
    --base-genesis)
      base_genesis=${2:-}
      shift 2
      ;;
    --base-manifest)
      base_manifest=${2:-}
      shift 2
      ;;
    --candidate-id)
      candidate_id=${2:-}
      shift 2
      ;;
    --track)
      track=${2:-}
      shift 2
      ;;
    --sequencer-node-id)
      sequencer_node_id=${2:-}
      shift 2
      ;;
    --storage-node-id)
      storage_node_id=${2:-}
      shift 2
      ;;
    --stake)
      stake=${2:-}
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "unknown option: $1"
      ;;
  esac
done

has_validator_47=0
if [[ "$sequencer_node_id" == "$VALIDATOR_47_NODE_ID" || "$storage_node_id" == "$VALIDATOR_47_NODE_ID" ]]; then
  has_validator_47=1
fi
if ((${#extra_validators[@]} > 0)); then
  for extra_validator in "${extra_validators[@]}"; do
    if [[ "$extra_validator" == "$VALIDATOR_47_NODE_ID:"* ]]; then
      has_validator_47=1
    fi
  done
fi

require_command jq
require_command python3
require_command shasum

require_non_empty "--runtime-build-ref" "$runtime_build_ref"
require_non_empty "--bootstrap-peers-file" "$bootstrap_peers_file"
require_non_empty "--out-dir" "$out_dir"

require_file "$runtime_build_ref"
require_file "$bootstrap_peers_file"
require_file "$base_genesis"
require_file "$base_manifest"
triad_bootstrap_peer_sha256=$(sha256_file "$bootstrap_peers_file")
if [[ $has_validator_47 -eq 1 ]]; then
  require_file "$TRIAD_INVENTORY_PATH"
  [[ ! -L "$TRIAD_INVENTORY_PATH" ]] || die "triad inventory authority must not be a symlink"
  triad_inventory_sha256=$(sha256_file "$TRIAD_INVENTORY_PATH")
  [[ "$triad_inventory_sha256" == "$TRIAD_INVENTORY_SHA256" ]] \
    || die "triad inventory is not the canonical governed authority"
  python3 - "$TRIAD_INVENTORY_PATH" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
value = json.loads(path.read_text(encoding="utf-8"))
if value.get("schema_version") != "oasis7.public_testnet_validator_triad_inventory.v1":
    raise SystemExit("triad inventory schema mismatch")
if value.get("network_tier") != "public_testnet" or value.get("topology") != "three_equal_validator":
    raise SystemExit("triad inventory network/topology mismatch")
nodes = value.get("nodes")
if not isinstance(nodes, dict):
    raise SystemExit("triad inventory nodes missing")
validator = nodes.get("validator-47")
if not isinstance(validator, dict):
    raise SystemExit("triad inventory validator-47 binding missing")
expected = {
    "node_id": "triad-testnet-validator-47",
    "service": "oasis7-triad-validator-47.service",
    "roles": ["validator", "checkpoint_provider", "full_storage_provider"],
    "ports": ["6634", "6834"],
}
for key, expected_value in expected.items():
    if validator.get(key) != expected_value:
        raise SystemExit(f"triad inventory validator-47 {key} mismatch")
PY
fi
if [[ -n "$validator_pair_provenance_ref" ]]; then
  require_file "$validator_pair_provenance_ref"
fi

if [[ -n "$validator_47_identity_dir" ]]; then
  validator_47_identity_dir=$(python3 -c 'import os,sys; print(os.path.abspath(os.path.expanduser(sys.argv[1])))' "$validator_47_identity_dir")
  python3 - "$validator_47_identity_dir" <<'PY'
import os
import stat
import sys
from pathlib import Path

root = Path(sys.argv[1])
current = Path(root.anchor or "/")
for component in root.parts[1:]:
    current /= component
    if current.is_symlink():
        # macOS exposes /var and /tmp as stable aliases below /private; these
        # platform-owned aliases are safe, but caller-controlled redirects are
        # not accepted.
        resolved = os.path.realpath(current)
        if str(current) not in {"/var", "/tmp"} or not resolved.startswith("/private/"):
            raise SystemExit(f"validator-47 identity source path contains a symlink: {current}")
if root.is_symlink() or not root.is_dir():
    raise SystemExit("validator-47 identity source must be a regular directory")
root_stat = os.stat(root)
if stat.S_IMODE(root_stat.st_mode) != 0o700:
    raise SystemExit("validator-47 identity source directory must have mode 0700")
for name in ("node-keypair.toml", "identity-receipt.json"):
    path = root / name
    metadata = os.lstat(path)
    if not stat.S_ISREG(metadata.st_mode) or stat.S_IMODE(metadata.st_mode) != 0o600:
        raise SystemExit(f"validator-47 identity source {name} must be a regular file with mode 0600")
    if (metadata.st_uid, metadata.st_gid) != (root_stat.st_uid, root_stat.st_gid):
        raise SystemExit(f"validator-47 identity source {name} ownership differs from its directory")
PY
fi

is_hex_32() {
  [[ "$1" =~ ^[0-9a-fA-F]{64}$ ]]
}

require_non_empty "--sequencer-finality-public-key" "$sequencer_public_key"
require_non_empty "--storage-finality-public-key" "$storage_public_key"
is_hex_32 "$sequencer_public_key" || die "sequencer finality public key must be 32-byte hex"
is_hex_32 "$storage_public_key" || die "storage finality public key must be 32-byte hex"

validator_specs=(
  "$sequencer_node_id:$sequencer_public_key:$stake"
  "$storage_node_id:$storage_public_key:$stake"
)
if ((${#extra_validators[@]} > 0)); then
  for extra_validator in "${extra_validators[@]}"; do
    require_non_empty "--extra-validator" "$extra_validator"
    validator_specs+=("$extra_validator")
  done
fi

if [[ $has_validator_47 -eq 1 && -z "$validator_47_identity_dir" ]]; then
  die "validator-47 deployment stage requires --validator-47-identity-dir"
fi
if [[ $has_validator_47 -eq 0 && -n "$validator_47_identity_dir" ]]; then
  die "--validator-47-identity-dir requires the validator-47 extra validator"
fi

# The inventory, source registry, and bootstrap-peer evidence are one
# authority chain. Validate the caller's complete triad against that chain
# before creating or deleting anything below --out-dir. In particular, a
# caller must not substitute a wrong identity or a self-consistent peer file
# and obtain a locally coherent but non-governed stage.
if [[ $has_validator_47 -eq 1 ]]; then
python3 - "$repo_root" "$TRIAD_INVENTORY_PATH" "$bootstrap_peers_file" "${validator_specs[@]}" <<'PY'
import hashlib
import json
import re
import sys
from pathlib import Path

repo_root = Path(sys.argv[1]).resolve()
inventory_path = Path(sys.argv[2])
bootstrap_input = Path(sys.argv[3])
specs = sys.argv[4:]

def regular(path: Path, label: str) -> bytes:
    if path.is_symlink() or not path.is_file():
        raise SystemExit(f"{label} must be a regular non-symlink file: {path}")
    try:
        return path.read_bytes()
    except OSError as error:
        raise SystemExit(f"{label} is unreadable: {error.__class__.__name__}")

inventory_bytes = regular(inventory_path, "triad inventory authority")
try:
    inventory = json.loads(inventory_bytes.decode("utf-8"))
except (UnicodeError, ValueError) as error:
    raise SystemExit(f"triad inventory is malformed: {error.__class__.__name__}")
if not isinstance(inventory, dict):
    raise SystemExit("triad inventory must be a JSON object")
if (
    inventory.get("schema_version") != "oasis7.public_testnet_validator_triad_inventory.v1"
    or inventory.get("repository") != "eng-cc/oasis7"
    or inventory.get("network_tier") != "public_testnet"
    or inventory.get("topology") != "three_equal_validator"
):
    raise SystemExit("triad inventory identity/role contract mismatch")
authority = inventory.get("authority")
nodes = inventory.get("nodes")
if not isinstance(authority, dict) or not isinstance(nodes, dict):
    raise SystemExit("triad inventory authority or nodes missing")
source_ref = authority.get("source_registry_ref")
source_sha = authority.get("source_registry_sha256")
generated_sha = authority.get("generated_registry_sha256")
generated_semantic_sha = authority.get("generated_registry_semantic_sha256")
peer_ref = authority.get("bootstrap_peer_ref")
peer_sha = authority.get("bootstrap_peer_sha256")
if not all(isinstance(value, str) and value.strip() for value in (source_ref, source_sha, generated_sha, generated_semantic_sha, peer_ref, peer_sha)):
    raise SystemExit("triad inventory source registry/bootstrap peer binding incomplete")

def repo_ref(raw: str, label: str) -> Path:
    candidate = Path(raw)
    if candidate.is_absolute():
        raise SystemExit(f"{label} must be repository-relative")
    resolved = (repo_root / candidate).resolve()
    try:
        resolved.relative_to(repo_root)
    except ValueError:
        raise SystemExit(f"{label} escapes repository root")
    return resolved

source_path = repo_ref(source_ref, "source registry ref")
peer_path = repo_ref(peer_ref, "bootstrap peer ref")
source_bytes = regular(source_path, "source validator registry")
peer_bytes = regular(peer_path, "governed bootstrap peer evidence")
if hashlib.sha256(source_bytes).hexdigest() != source_sha.lower():
    raise SystemExit("source validator registry digest does not match inventory authority")
if hashlib.sha256(peer_bytes).hexdigest() != peer_sha.lower():
    raise SystemExit("governed bootstrap peer evidence digest does not match inventory authority")
if bootstrap_input.is_symlink() or not bootstrap_input.is_file():
    raise SystemExit("bootstrap peer input must be a regular non-symlink file")
if bootstrap_input.read_bytes() != peer_bytes:
    raise SystemExit("bootstrap peer input does not exactly match governed inventory evidence")

try:
    source_registry = json.loads(source_bytes.decode("utf-8"))
except (UnicodeError, ValueError) as error:
    raise SystemExit(f"source validator registry is malformed: {error.__class__.__name__}")
source_validators = source_registry.get("validators") if isinstance(source_registry, dict) else None
if not isinstance(source_validators, list) or len(source_validators) != 3:
    raise SystemExit("source validator registry must contain exactly three validators")
source_by_id = {item.get("node_id"): item for item in source_validators if isinstance(item, dict)}
if len(source_by_id) != 3:
    raise SystemExit("source validator registry has duplicate or malformed node identities")

expected_validators = {}
for name, node in nodes.items():
    if not isinstance(node, dict) or not isinstance(node.get("node_id"), str):
        raise SystemExit(f"triad inventory node is malformed: {name}")
    node_id = node["node_id"]
    expected_validators[node_id] = node
    source = source_by_id.get(node_id)
    if not isinstance(source, dict):
        raise SystemExit(f"source validator registry is missing inventory node: {node_id}")
    for field in ("stake", "finality_signer_public_key"):
        if str(source.get(field, "")).lower() != str(node.get(field, "")).lower():
            raise SystemExit(f"inventory/source registry mismatch for {node_id}: {field}")
    if node_id == "triad-testnet-validator-47":
        for field in ("root_public_key", "finality_public_key", "libp2p_peer_id"):
            if str(source.get(field, "")) != str(node.get(field, "")):
                raise SystemExit(f"inventory/source registry mismatch for validator-47: {field}")

parsed_specs = {}
for spec in specs:
    parts = spec.split(":")
    if len(parts) not in (2, 3):
        raise SystemExit(f"invalid validator spec: {spec}")
    node_id, signer = parts[0].strip(), parts[1].strip().lower()
    try:
        stake = int(parts[2]) if len(parts) == 3 and parts[2].strip() else 100
    except ValueError:
        raise SystemExit(f"invalid validator stake: {spec}")
    if node_id in parsed_specs:
        raise SystemExit(f"duplicate validator spec: {node_id}")
    parsed_specs[node_id] = (signer, stake)
if set(parsed_specs) != set(expected_validators):
    raise SystemExit("validator specs do not exactly match the governed triad inventory")
for node_id, (signer, stake) in parsed_specs.items():
    expected = expected_validators[node_id]
    if signer != str(expected.get("finality_signer_public_key", "")).lower() or stake != expected.get("stake"):
        raise SystemExit(f"validator spec is not the governed identity for {node_id}")

peer_lines = [line.strip() for line in peer_bytes.decode("utf-8").splitlines() if line.strip()]
if len(peer_lines) != 3:
    raise SystemExit("governed bootstrap peer evidence must contain exactly three peers")
for name, node in nodes.items():
    host = str(node.get("host", "")).removeprefix("root@")
    ports = node.get("ports")
    if not host or not isinstance(ports, list) or len(ports) < 2:
        raise SystemExit(f"inventory bootstrap endpoint is incomplete: {name}")
    expected_prefix = f"/ip4/{host}/tcp/{ports[1]}/p2p/"
    matches = [line for line in peer_lines if line.startswith(expected_prefix)]
    if len(matches) != 1:
        raise SystemExit(f"governed bootstrap peer evidence endpoint mismatch: {name}")
    expected_peer = node.get("libp2p_peer_id")
    if expected_peer and not matches[0].endswith(f"/p2p/{expected_peer}"):
        raise SystemExit(f"governed bootstrap peer evidence identity mismatch: {name}")
if not re.fullmatch(r"[0-9a-f]{64}", source_sha.lower()) or not re.fullmatch(r"[0-9a-f]{64}", peer_sha.lower()):
    raise SystemExit("authority digest must be lowercase SHA-256")
if not re.fullmatch(r"[0-9a-f]{64}", generated_sha.lower()):
    raise SystemExit("generated registry digest must be lowercase SHA-256")
if not re.fullmatch(r"[0-9a-f]{64}", generated_semantic_sha.lower()):
    raise SystemExit("generated registry semantic digest must be lowercase SHA-256")
PY
fi

# Validate the already-staged public identity before touching the requested
# output directory.  The generated registry is checked again after it is
# rendered below; this preflight binds the receipt to the exact validator-47
# signer supplied for this stage, so a mismatch cannot leave partial stage
# materialization behind.
validator_47_expected_finality_public_key=""
if [[ $has_validator_47 -eq 1 ]]; then
  for validator_spec in "${validator_specs[@]}"; do
    if [[ "$validator_spec" == "$VALIDATOR_47_NODE_ID:"* ]]; then
      validator_47_spec_tail=${validator_spec#"$VALIDATOR_47_NODE_ID:"}
      validator_47_expected_finality_public_key=${validator_47_spec_tail%%:*}
      break
    fi
  done
  [[ -x "$runtime_build_ref" ]] || die "validator-47 identity readback runtime is not executable"
  validator_47_preflight_identity_receipt=$("$runtime_build_ref" identity-receipt \
    --config-dir "$validator_47_identity_dir" --node-id "$VALIDATOR_47_NODE_ID") \
    || die "validator-47 staged identity readback failed before stage materialization"
  python3 - "$validator_47_identity_dir" "$validator_47_expected_finality_public_key" \
    "$validator_47_preflight_identity_receipt" "$TRIAD_INVENTORY_PATH" <<'PY'
import hashlib
import json
import sys
from pathlib import Path

identity_dir = Path(sys.argv[1])
expected_finality_public_key = sys.argv[2]
runtime_receipt = json.loads(sys.argv[3])
inventory = json.loads(Path(sys.argv[4]).read_text(encoding="utf-8"))
expected_identity = inventory["nodes"]["validator-47"]
receipt = json.loads((identity_dir / "identity-receipt.json").read_text(encoding="utf-8"))
if receipt.get("schema_version") != "oasis7.identity_provision.v1":
    raise SystemExit("validator-47 staged identity receipt schema mismatch")
if receipt.get("node_id") != "triad-testnet-validator-47":
    raise SystemExit("validator-47 staged identity receipt node mismatch")
for field in ("root_public_key", "finality_public_key", "libp2p_peer_id"):
    if not isinstance(receipt.get(field), str) or not receipt[field].strip():
        raise SystemExit(f"validator-47 staged identity receipt missing {field}")
if receipt["finality_public_key"].lower() != expected_finality_public_key.lower():
    raise SystemExit("validator-47 staged public identity does not match supplied governed signer")
for field in ("root_public_key", "finality_public_key", "libp2p_peer_id"):
    if receipt[field] != expected_identity[field]:
        raise SystemExit(f"validator-47 staged public identity does not match governed inventory: {field}")
if "private_key" in json.dumps(receipt, ensure_ascii=False).lower():
    raise SystemExit("validator-47 public identity receipt must not contain private key material")
if runtime_receipt.get("schema_version") != "oasis7.identity_receipt.v1":
    raise SystemExit("validator-47 runtime identity readback schema mismatch")
if runtime_receipt.get("node_id") != receipt["node_id"]:
    raise SystemExit("validator-47 runtime identity readback node mismatch")
key_path = identity_dir / "node-keypair.toml"
if runtime_receipt.get("key_sha256") != hashlib.sha256(key_path.read_bytes()).hexdigest():
    raise SystemExit("validator-47 runtime identity key digest mismatch")
if runtime_receipt.get("peer_id") != receipt["libp2p_peer_id"]:
    raise SystemExit("validator-47 runtime identity peer id mismatch")
PY
fi

out_dir=$(python3 - "$out_dir" <<'PY'
import pathlib
import sys

requested = pathlib.Path(sys.argv[1]).expanduser()
absolute = requested if requested.is_absolute() else pathlib.Path.cwd() / requested
if absolute.is_symlink():
    raise SystemExit(f"refusing symlinked output path: {requested}")
resolved = absolute.resolve()
cwd = pathlib.Path.cwd().resolve()
if resolved == pathlib.Path(resolved.anchor) or resolved == cwd or resolved in cwd.parents:
    raise SystemExit(f"refusing unsafe output path: {requested}")
print(resolved)
PY
)
rm -rf "$out_dir"
mkdir -p "$out_dir/config/doc/testing/evidence" "$out_dir/generated-world"

registry_path="$out_dir/config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json"
public_signers_path="$out_dir/config/public-testnet-governance-public-signers-deployment-2026-06-06.json"
genesis_path="$out_dir/config/public-testnet-governed-bootstrap-genesis-2026-06-06.json"
manifest_path="$out_dir/config/public-testnet-governed-bootstrap-manifest-2026-06-06.json"
bundle_path="$out_dir/config/public-testnet-governed-bootstrap-bundle-2026-06-06.json"
bootstrap_out="$out_dir/config/public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.txt"
inventory_out="$out_dir/config/$TRIAD_INVENTORY_FILE"
node_env_path="$out_dir/config/node.env"
deployment_truth_md="$out_dir/deployment-truth.md"
temp_genesis="$out_dir/.tmp-genesis.json"

stage_inventory_ref=""
stage_inventory_sha256=""

cp "$bootstrap_peers_file" "$bootstrap_out"
cp "$bootstrap_out" "$out_dir/config/doc/testing/evidence/"
if [[ $has_validator_47 -eq 1 ]]; then
  # The fixed inventory and its source registry are authority artifacts only
  # for the exact governed triad validated above. Pair-only and arbitrary
  # extra-validator stages must not carry them as misleading authority.
  cp "$TRIAD_INVENTORY_PATH" "$inventory_out"
  cp "$TRIAD_INVENTORY_PATH" "$out_dir/config/doc/testing/evidence/"
  cp "$TRIAD_SOURCE_REGISTRY_PATH" "$out_dir/config/doc/testing/evidence/"
  stage_inventory_ref="$TRIAD_INVENTORY_RELATIVE"
  stage_inventory_sha256="$triad_inventory_sha256"
fi

python3 - "$registry_path" "$quorum_numerator" "$quorum_denominator" \
  "$governance_signer_count" "$governance_threshold" "$governance_threshold_bps" \
  "${validator_specs[@]}" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
quorum_numerator = int(sys.argv[2])
quorum_denominator = int(sys.argv[3])
governance_signer_count = int(sys.argv[4])
governance_threshold = int(sys.argv[5])
governance_threshold_bps = int(sys.argv[6])
specs = sys.argv[7:]
if len(specs) < 2:
    raise SystemExit("at least two validators are required")

validators = []
seen_node_ids = set()
for spec in specs:
    parts = spec.split(":")
    if len(parts) not in (2, 3):
        raise SystemExit(f"invalid validator spec `{spec}`; expected node_id:finality_public_key[:stake]")
    node_id, public_key = parts[0].strip(), parts[1].strip()
    stake = int(parts[2]) if len(parts) == 3 and parts[2].strip() else 100
    if not node_id:
        raise SystemExit("validator node_id cannot be empty")
    if node_id in seen_node_ids:
        raise SystemExit(f"duplicate validator node_id `{node_id}`")
    if len(public_key) != 64 or any(ch not in "0123456789abcdefABCDEF" for ch in public_key):
        raise SystemExit(f"validator finality public key must be 32-byte hex for node_id={node_id}")
    if stake <= 0:
        raise SystemExit(f"validator stake must be > 0 for node_id={node_id}")
    seen_node_ids.add(node_id)
    validators.append({
        "node_id": node_id,
        "scheme": "ed25519",
        "finality_signer_public_key": public_key,
        "stake": stake,
    })

threshold = max(2, (len(validators) * 2 + 2) // 3)
stake_total = sum(validator["stake"] for validator in validators)
required_stake = (stake_total * quorum_numerator + quorum_denominator - 1) // quorum_denominator
payload = {
    "slot_id": "governance.finality.v1",
    "threshold": threshold,
    "threshold_bps": 0,
    "quorum": {
        "numerator": quorum_numerator,
        "denominator": quorum_denominator,
        "total_stake": stake_total,
        "required_stake": required_stake,
    },
    "governance": {
        "signer_count": governance_signer_count,
        "threshold": governance_threshold,
        "threshold_bps": governance_threshold_bps,
    },
    "validators": validators,
}
path.write_text(json.dumps(payload, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")
PY
cp "$registry_path" "$out_dir/config/doc/testing/evidence/"

if [[ $has_validator_47 -eq 1 ]]; then
  python3 - "$TRIAD_INVENTORY_PATH" "$registry_path" <<'PY'
import json
import pathlib
import sys

inventory = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
registry_path = pathlib.Path(sys.argv[2])
registry = json.loads(registry_path.read_text(encoding="utf-8"))
target = inventory["nodes"]["validator-47"]
matches = [item for item in registry["validators"] if item.get("node_id") == target["node_id"]]
if len(matches) != 1:
    raise SystemExit("generated registry is missing validator-47")
for field in ("root_public_key", "finality_public_key", "libp2p_peer_id"):
    matches[0][field] = target[field]
registry_path.write_text(json.dumps(registry, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")
PY
  cp "$registry_path" "$out_dir/config/doc/testing/evidence/"
fi

triad_generated_registry_sha256=""
if [[ $has_validator_47 -eq 1 ]]; then
  triad_generated_registry_sha256=$(sha256_file "$registry_path")
  triad_expected_generated_registry_sha256=$(jq -r '.authority.generated_registry_sha256 // empty' "$TRIAD_INVENTORY_PATH")
  [[ "$triad_expected_generated_registry_sha256" =~ ^[0-9a-f]{64}$ ]] \
    || die "triad inventory generated registry digest is missing or malformed"
  [[ "$triad_generated_registry_sha256" == "$triad_expected_generated_registry_sha256" ]] \
    || die "generated validator registry digest does not match triad inventory authority"
  triad_generated_registry_semantic_sha256=$(python3 - "$registry_path" <<'PY'
import hashlib
import json
import sys
from pathlib import Path

registry = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
validators = registry.get("validators")
if not isinstance(validators, list):
    raise SystemExit("generated validator registry validators are missing")
canonical = {
    "signer_bindings": {
        f"governance.finality.v1.{item['node_id']}": str(item["finality_signer_public_key"]).lower()
        for item in validators
    },
    "slot_id": registry.get("slot_id"),
    "threshold": registry.get("threshold"),
    "threshold_bps": registry.get("threshold_bps"),
    "validator_stakes": {
        f"governance.finality.v1.{item['node_id']}": item["stake"]
        for item in validators
    },
}
print(hashlib.sha256(json.dumps(canonical, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()).hexdigest())
PY
  )
  triad_expected_generated_registry_semantic_sha256=$(jq -r '.authority.generated_registry_semantic_sha256 // empty' "$TRIAD_INVENTORY_PATH")
  [[ "$triad_expected_generated_registry_semantic_sha256" =~ ^[0-9a-f]{64}$ ]] \
    || die "triad inventory generated registry semantic digest is missing or malformed"
  [[ "$triad_generated_registry_semantic_sha256" == "$triad_expected_generated_registry_semantic_sha256" ]] \
    || die "generated validator registry semantic digest does not match triad inventory authority"
fi

identity_out_dir="$out_dir/identity"
identity_key_out="$identity_out_dir/$VALIDATOR_47_IDENTITY_KEY_FILE"
identity_receipt_out="$identity_out_dir/$VALIDATOR_47_IDENTITY_RECEIPT_FILE"
identity_key_sha256=""
identity_receipt_sha256=""

if [[ $has_validator_47 -eq 1 ]]; then
  identity_key_source="$validator_47_identity_dir/$VALIDATOR_47_IDENTITY_KEY_FILE"
  identity_receipt_source="$validator_47_identity_dir/$VALIDATOR_47_IDENTITY_RECEIPT_FILE"
  # Read the existing key only through the runtime's read-only identity
  # receipt command.  This validates the keypair and derives its public peer
  # identity without creating or rewriting any identity material.
  [[ -x "$runtime_build_ref" ]] || die "validator-47 identity readback runtime is not executable"
  runtime_identity_receipt=$("$runtime_build_ref" identity-receipt \
    --config-dir "$validator_47_identity_dir" --node-id "$VALIDATOR_47_NODE_ID") \
    || die "validator-47 staged identity readback failed"
  python3 - "$identity_receipt_source" "$identity_key_source" "$registry_path" \
    "$runtime_identity_receipt" <<'PY'
import hashlib
import json
import os
import stat
import sys
from pathlib import Path

receipt_path = Path(sys.argv[1])
key_path = Path(sys.argv[2])
registry_path = Path(sys.argv[3])
runtime_receipt = json.loads(sys.argv[4])

def secure_regular(path: Path, label: str) -> os.stat_result:
    metadata = os.lstat(path)
    if not stat.S_ISREG(metadata.st_mode):
        raise SystemExit(f"validator-47 {label} must be a regular file")
    if stat.S_IMODE(metadata.st_mode) != 0o600:
        raise SystemExit(f"validator-47 {label} must have mode 0600")
    return metadata

key_metadata = secure_regular(key_path, "identity source key")
receipt_metadata = secure_regular(receipt_path, "identity source receipt")
if (key_metadata.st_uid, key_metadata.st_gid) != (receipt_metadata.st_uid, receipt_metadata.st_gid):
    raise SystemExit("validator-47 identity source key/receipt ownership mismatch")

receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
if receipt.get("schema_version") != "oasis7.identity_provision.v1":
    raise SystemExit("validator-47 staged identity receipt schema mismatch")
if receipt.get("node_id") != "triad-testnet-validator-47":
    raise SystemExit("validator-47 staged identity receipt node mismatch")
for field in ("root_public_key", "finality_public_key", "libp2p_peer_id"):
    if not isinstance(receipt.get(field), str) or not receipt[field].strip():
        raise SystemExit(f"validator-47 staged identity receipt missing {field}")
if len(receipt["finality_public_key"]) != 64 or any(
    char not in "0123456789abcdefABCDEF" for char in receipt["finality_public_key"]
):
    raise SystemExit("validator-47 staged identity finality public key is not 32-byte hex")
if runtime_receipt.get("schema_version") != "oasis7.identity_receipt.v1":
    raise SystemExit("validator-47 runtime identity readback schema mismatch")
expected_key_sha = hashlib.sha256(key_path.read_bytes()).hexdigest()
if runtime_receipt.get("key_sha256") != expected_key_sha:
    raise SystemExit("validator-47 runtime identity key digest mismatch")
if runtime_receipt.get("peer_id") != receipt["libp2p_peer_id"]:
    raise SystemExit("validator-47 runtime identity peer id mismatch")
registry = json.loads(registry_path.read_text(encoding="utf-8"))
matches = [item for item in registry.get("validators", []) if item.get("node_id") == receipt["node_id"]]
if len(matches) != 1 or matches[0].get("finality_signer_public_key", "").lower() != receipt["finality_public_key"].lower():
    raise SystemExit("validator-47 staged public identity does not match governed registry")
PY
  mkdir -p "$identity_out_dir"
  chmod 0700 "$identity_out_dir"
  cp -p "$identity_key_source" "$identity_key_out"
  cp -p "$identity_receipt_source" "$identity_receipt_out"
  chmod 0600 "$identity_key_out" "$identity_receipt_out"
  python3 - "$validator_47_identity_dir" "$identity_out_dir" <<'PY'
import os
import stat
import sys
from pathlib import Path

source_dir = Path(sys.argv[1])
output_dir = Path(sys.argv[2])
for name in ("node-keypair.toml", "identity-receipt.json"):
    source = source_dir / name
    output = output_dir / name
    source_metadata = os.lstat(source)
    output_metadata = os.lstat(output)
    if not stat.S_ISREG(output_metadata.st_mode) or stat.S_IMODE(output_metadata.st_mode) != 0o600:
        raise SystemExit(f"validator-47 staged identity output {name} is not regular/0600")
    if (source_metadata.st_uid, source_metadata.st_gid) != (output_metadata.st_uid, output_metadata.st_gid):
        raise SystemExit(f"validator-47 staged identity output {name} ownership changed")
    if source.read_bytes() != output.read_bytes():
        raise SystemExit(f"validator-47 staged identity output {name} bytes changed")
PY
  identity_key_sha256=$(sha256_file "$identity_key_out")
  identity_receipt_sha256=$(sha256_file "$identity_receipt_out")
fi

python3 - "$base_genesis" "$public_signers_path" "$registry_path" <<'PY'
import json
import pathlib
import sys

base_genesis = pathlib.Path(sys.argv[1]).resolve()
out_path = pathlib.Path(sys.argv[2]).resolve()
registry_path = pathlib.Path(sys.argv[3]).resolve()

with base_genesis.open("r", encoding="utf-8") as fh:
    genesis = json.load(fh)

refs = genesis.get("governance_bootstrap_refs")
if not isinstance(refs, dict):
    raise SystemExit("base genesis missing governance_bootstrap_refs")
raw_manifest_ref = refs.get("governance_public_manifest_ref")
if not isinstance(raw_manifest_ref, str) or not raw_manifest_ref.strip():
    raise SystemExit("base genesis missing governance_public_manifest_ref")

def resolve_ref(raw: str) -> pathlib.Path:
    candidate = pathlib.Path(raw).expanduser()
    if candidate.is_absolute():
        return candidate.resolve()
    for base in (base_genesis.parent, *base_genesis.parents):
        resolved = (base / candidate).resolve()
        if resolved.exists():
            return resolved
    return (base_genesis.parent / candidate).resolve()

base_manifest_path = resolve_ref(raw_manifest_ref)
base_manifest = json.loads(base_manifest_path.read_text(encoding="utf-8"))
entries = base_manifest.get("entries")
if not isinstance(entries, list):
    raise SystemExit(f"base governance public manifest has no entries: {base_manifest_path}")

registry = json.loads(registry_path.read_text(encoding="utf-8"))
validators = registry.get("validators")
if not isinstance(validators, list) or not validators:
    raise SystemExit(f"validator registry has no validators: {registry_path}")

next_entries = [
    entry
    for entry in entries
    if not (
        isinstance(entry, dict)
        and entry.get("slot_id") == "governance.finality.v1"
    )
]
for validator in validators:
    node_id = validator.get("node_id")
    public_key = validator.get("finality_signer_public_key")
    if not node_id or not public_key:
        raise SystemExit("validator entry missing node_id/finality_signer_public_key")
    next_entries.append(
        {
            "slot_id": "governance.finality.v1",
            "signer_id": node_id,
            "scheme": validator.get("scheme") or "ed25519",
            "public_key_hex": public_key,
        }
    )

base_manifest["entries"] = next_entries
base_manifest["truth_kind"] = "deployment_public_signers"
base_manifest["source_public_manifest_ref"] = str(base_manifest_path)
base_manifest["deployment_validator_registry_ref"] = str(registry_path)
out_path.write_text(
    json.dumps(base_manifest, ensure_ascii=True, indent=2) + "\n",
    encoding="utf-8",
)
PY
cp "$public_signers_path" "$out_dir/config/doc/testing/evidence/"

python3 - "$base_genesis" "$genesis_path" "$registry_path" "$public_signers_path" <<'PY'
import json
import pathlib
import shutil
import sys

base_path = pathlib.Path(sys.argv[1]).resolve()
out_path = pathlib.Path(sys.argv[2]).resolve()
registry_path = pathlib.Path(sys.argv[3]).resolve()
public_signers_path = pathlib.Path(sys.argv[4]).resolve()

with base_path.open("r", encoding="utf-8") as fh:
    payload = json.load(fh)

payload.setdefault("governance_bootstrap_refs", {})
refs = payload["governance_bootstrap_refs"]

def resolve_ref(raw: str) -> str:
    candidate = pathlib.Path(raw).expanduser()
    if candidate.is_absolute():
        return str(candidate.resolve())
    for base in (base_path.parent, *base_path.parents):
        resolved = (base / candidate).resolve()
        if resolved.exists():
            return str(resolved)
    return str((base_path.parent / candidate).resolve())

for key, value in list(refs.items()):
    if isinstance(value, str) and value.strip():
        refs[key] = resolve_ref(value)

refs["genesis_validator_registry_ref"] = str(registry_path)
refs["governance_public_manifest_ref"] = str(public_signers_path)

out_path.write_text(json.dumps(payload, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")

evidence_dir = out_path.parent / "doc" / "testing" / "evidence"
evidence_dir.mkdir(parents=True, exist_ok=True)
for value in refs.values():
    if not isinstance(value, str) or not value.strip():
        continue
    source = pathlib.Path(value)
    if source.is_file():
        target = evidence_dir / source.name
        if source.resolve() != target.resolve():
            shutil.copy2(source, target)
PY
cp "$genesis_path" "$out_dir/config/doc/testing/evidence/"

cp "$base_manifest" "$manifest_path"
cp "$manifest_path" "$out_dir/config/doc/testing/evidence/"

./scripts/p2p-build-governed-bootstrap-world.sh create \
  --genesis "$genesis_path" \
  --out-dir "$out_dir/generated-world" \
  --world-scenario asteroid_fragment_bootstrap \
  --allow-overwrite >/dev/null

python3 - "$deployment_truth_md" "$runtime_build_ref" "$bootstrap_out" "$stage_inventory_ref" "$stage_inventory_sha256" "$triad_bootstrap_peer_sha256" "${validator_specs[@]}" <<'PY'
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
runtime_build_ref = sys.argv[2]
bootstrap_peers_file = pathlib.Path(sys.argv[3])
inventory_ref = sys.argv[4]
inventory_sha256 = sys.argv[5]
bootstrap_peer_sha256 = sys.argv[6]
specs = sys.argv[7:]
validator_lines = []
for spec in specs:
    node_id, public_key, *rest = spec.split(":")
    stake = rest[0] if rest else "100"
    validator_lines.append(f"  - `{node_id}` -> `{public_key}` (stake `{stake}`)")

authority_lines = ""
if inventory_ref:
    authority_lines = (
        f"- Deployment inventory authority: `{inventory_ref}`\n"
        f"- Deployment inventory sha256: `{inventory_sha256}`\n"
    )
content = f"""# Deployment Truth

- Runtime build: `{runtime_build_ref}`
- Bootstrap peers file: `{bootstrap_peers_file}`
- Bootstrap peers sha256: `{bootstrap_peer_sha256}`
{authority_lines}- Canonical bootstrap world: `generated-world/world`
- Fresh-host bootstrap world input: `generated-world`
- Generated map sidecar: `generated-world/generated-scenario-world`
- Generated map provenance: `generated-world/world-generation-provenance.json`
- Validator signer truth:
{chr(10).join(validator_lines)}
"""
path.write_text(content, encoding="utf-8")
PY

if [[ $has_validator_47 -eq 1 ]]; then
  python3 - "$deployment_truth_md" "$VALIDATOR_47_IDENTITY_KEY_FILE" "$identity_key_sha256" \
    "$VALIDATOR_47_IDENTITY_RECEIPT_FILE" "$identity_receipt_sha256" <<'PY'
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
key_file, key_sha256, receipt_file, receipt_sha256 = sys.argv[2:]
with path.open("a", encoding="utf-8") as handle:
    handle.write(
        f"- Validator-47 identity import: `identity/{key_file}` "
        "(bytes copied from the already-staged source; no key regeneration)\n"
        f"- Validator-47 identity key sha256: `{key_sha256}`\n"
        f"- Validator-47 identity public receipt: `identity/{receipt_file}`\n"
        f"- Validator-47 identity receipt sha256: `{receipt_sha256}`\n"
    )
PY
fi

if [[ -n "$validator_pair_provenance_ref" ]]; then
  provenance_copy="$out_dir/config/doc/testing/evidence/$(basename "$validator_pair_provenance_ref")"
  if [[ "$(cd "$(dirname "$validator_pair_provenance_ref")" && pwd)/$(basename "$validator_pair_provenance_ref")" != "$provenance_copy" ]]; then
    cp "$validator_pair_provenance_ref" "$provenance_copy"
  fi
  python3 - "$validator_pair_provenance_ref" "$(dirname "$provenance_copy")" "$provenance_copy" <<'PY'
import json
import hashlib
import os
import shutil
import sys
from pathlib import Path

receipt_path = Path(sys.argv[1]).resolve()
evidence_dir = Path(sys.argv[2]).resolve()
provenance_copy = Path(sys.argv[3]).resolve()
receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
refs = []
signature = receipt.get("signature")
if isinstance(signature, dict):
    refs.extend(signature.get(field) for field in ("signature_ref", "public_key_ref"))
governed = receipt.get("governed")
if isinstance(governed, dict):
    refs.extend(value.get("path") for value in governed.values() if isinstance(value, dict))
closure_map = {}
def content_digest(source: Path) -> str:
    digest = hashlib.sha256()
    if source.is_file():
        digest.update(source.read_bytes())
    elif source.is_dir():
        for child in sorted(source.rglob("*"), key=lambda item: item.relative_to(source).as_posix()):
            relative = child.relative_to(source).as_posix()
            digest.update(relative.encode())
            digest.update(b"\0")
            if child.is_symlink():
                digest.update(b"L\0" + os.readlink(child).encode())
            elif child.is_file():
                digest.update(b"F\0" + child.read_bytes())
            else:
                digest.update(b"D\0")
    return digest.hexdigest()
for raw in refs:
    if not isinstance(raw, str) or not raw.strip():
        continue
    source = Path(raw)
    if not source.is_absolute():
        source = receipt_path.parent / source
    if source.is_symlink() or not source.exists():
        raise SystemExit(f"provenance closure reference missing or symlinked: {source}")
    source = source.resolve()
    destination = evidence_dir / source.name
    if destination.exists() or destination.is_symlink():
        if destination.resolve() == source:
            closure_map[raw] = destination.name
            continue
        if destination.is_dir() and source.is_dir():
            shutil.copytree(source, destination, dirs_exist_ok=True)
        elif destination.is_file() and source.is_file() and destination.read_bytes() == source.read_bytes():
            closure_map[raw] = destination.name
            continue
        else:
            digest = content_digest(source)[:16]
            destination = evidence_dir / f"{digest}-{source.name}"
            if destination.exists() and destination.is_file() and source.is_file() and destination.read_bytes() != source.read_bytes():
                raise SystemExit(f"provenance closure deterministic collision: {destination}")
    if not destination.exists() and not destination.is_symlink():
        if source.is_dir():
            shutil.copytree(source, destination)
        else:
            shutil.copy2(source, destination)
    closure_map[raw] = destination.name
closure_path = Path(str(provenance_copy) + ".closure.json")
closure_payload = {
    "schema_version": "oasis7.validator_pair_provenance_closure.v1",
    "receipt": provenance_copy.name,
    "mapping": {key: closure_map[key] for key in sorted(closure_map)},
}
closure_path.write_text(json.dumps(closure_payload, ensure_ascii=True, sort_keys=True, indent=2) + "\n", encoding="utf-8")
PY
  printf '%s\n' "- Validator pair provenance: \`$provenance_copy\`" >>"$deployment_truth_md"
fi

release_candidate_args=(
  create
  --bundle "$bundle_path"
  --candidate-id "$candidate_id"
  --track "$track"
  --runtime-build-ref "$runtime_build_ref"
  --world-snapshot-ref "$out_dir/generated-world/world"
  --generated-world-sidecar-ref "$out_dir/generated-world/generated-scenario-world"
  --world-generation-provenance-ref "$out_dir/generated-world/world-generation-provenance.json"
  --governance-manifest-ref "$registry_path"
  --evidence-ref "$deployment_truth_md"
  --note "Deployment-only bundle derived from current validator signer truth."
  --allow-dirty-worktree
)
if [[ -n "$validator_pair_provenance_ref" ]]; then
  release_candidate_args+=(--validator-pair-provenance-ref "$provenance_copy")
fi

./scripts/release-candidate-bundle.sh "${release_candidate_args[@]}" >/dev/null
cp "$bundle_path" "$out_dir/config/doc/testing/evidence/"

./scripts/release-candidate-bundle.sh validate --bundle "$bundle_path" >/dev/null
./scripts/p2p-build-governed-bootstrap-world.sh validate \
  --world-dir "$out_dir/generated-world/world" \
  --merged-public-manifest "$out_dir/generated-world/merged-public-manifest-entries.json" >/dev/null

python3 - "$base_manifest" "$manifest_path" "$bundle_path" "$genesis_path" "$bootstrap_out" "$registry_path" "$stage_inventory_ref" "$stage_inventory_sha256" "$triad_bootstrap_peer_sha256" "$has_validator_47" <<'PY'
import hashlib
import json
import pathlib
import sys

base_manifest = pathlib.Path(sys.argv[1])
manifest_path = pathlib.Path(sys.argv[2])
bundle_path = pathlib.Path(sys.argv[3])
genesis_path = pathlib.Path(sys.argv[4])
bootstrap_path = pathlib.Path(sys.argv[5])
registry_path = pathlib.Path(sys.argv[6])
inventory_ref = sys.argv[7]
inventory_sha256 = sys.argv[8]
bootstrap_peer_sha256 = sys.argv[9]
is_triad = sys.argv[10] == "1"
generated_registry_sha256 = hashlib.sha256(registry_path.read_bytes()).hexdigest()

payload = json.loads(base_manifest.read_text(encoding="utf-8"))
registry = json.loads(registry_path.read_text(encoding="utf-8"))
canonical_registry = {
    "signer_bindings": {
        f"governance.finality.v1.{item['node_id']}": str(item["finality_signer_public_key"]).lower()
        for item in registry.get("validators", [])
    },
    "slot_id": registry.get("slot_id"),
    "threshold": registry.get("threshold"),
    "threshold_bps": registry.get("threshold_bps"),
    "validator_stakes": {
        f"governance.finality.v1.{item['node_id']}": item["stake"]
        for item in registry.get("validators", [])
    },
}
generated_registry_semantic_sha256 = hashlib.sha256(
    json.dumps(canonical_registry, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()
).hexdigest()
payload.setdefault("runtime_refs", {})
payload["runtime_refs"]["release_candidate_bundle_ref"] = bundle_path.name
payload["runtime_refs"]["genesis_ref"] = genesis_path.name
payload["runtime_refs"]["bootstrap_peer_ref"] = bootstrap_path.name
payload["runtime_refs"]["generated_world_sidecar_ref"] = "generated-world/generated-scenario-world"
payload["runtime_refs"]["world_generation_provenance_ref"] = "generated-world/world-generation-provenance.json"
payload.setdefault("validator_policy", {})
payload["validator_policy"]["target_validator_count"] = len(registry.get("validators", []))
payload["bootstrap_peer_authority"] = {
    "ref": bootstrap_path.name,
    "sha256": bootstrap_peer_sha256,
}
payload["deployment_validator_registry"] = {
    "ref": f"config/{registry_path.name}",
    "sha256": generated_registry_sha256,
    "semantic_sha256": generated_registry_semantic_sha256,
}
if is_triad:
    payload["deployment_inventory"] = {
        "ref": inventory_ref,
        "sha256": inventory_sha256,
    }
manifest_path.write_text(json.dumps(payload, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")
PY
cp "$manifest_path" "$out_dir/config/doc/testing/evidence/"

# A triad stage carries the complete validator-47 host contract.  Pair-only
# stages intentionally keep their historical shape; they do not get a
# validator-47 node.env that could be mistaken for a third validator.
if [[ $has_validator_47 -eq 1 ]]; then
  python3 - "$TRIAD_INVENTORY_PATH" "$registry_path" "$manifest_path" "$node_env_path" "$triad_inventory_sha256" "$identity_key_sha256" "$identity_receipt_sha256" \
    "$VALIDATOR_47_CONFIG_PATH" "$VALIDATOR_47_EXECUTION_RECORDS_DIR" "$VALIDATOR_47_STORAGE_ROOT" \
    "$VALIDATOR_47_STORAGE_PROFILE" "$VALIDATOR_47_NODE_TICK_MS" \
    "$VALIDATOR_47_POS_SLOT_DURATION_MS" "$VALIDATOR_47_POS_TICKS_PER_SLOT" \
    "$VALIDATOR_47_POS_PROPOSAL_TICK_PHASE" "$VALIDATOR_47_POS_MAX_PAST_SLOT_LAG" \
    "$VALIDATOR_47_REWARD_RUNTIME_ENABLE" "$VALIDATOR_47_REWARD_RUNTIME_EPOCH_DURATION_SECS" \
    "$VALIDATOR_47_REWARD_POINTS_PER_CREDIT" "$VALIDATOR_47_REWARD_RUNTIME_AUTO_REDEEM" <<'PY'
import hashlib
import json
import pathlib
import sys

inventory_path = pathlib.Path(sys.argv[1])
registry_path = pathlib.Path(sys.argv[2])
manifest_path = pathlib.Path(sys.argv[3])
node_env_path = pathlib.Path(sys.argv[4])
expected_inventory_sha256 = sys.argv[5]
identity_key_sha256 = sys.argv[6]
identity_receipt_sha256 = sys.argv[7]
config_path = sys.argv[8]
execution_records_dir = sys.argv[9]
storage_root = sys.argv[10]
storage_profile = sys.argv[11]
node_tick_ms = sys.argv[12]
pos_slot_duration_ms = sys.argv[13]
pos_ticks_per_slot = sys.argv[14]
pos_proposal_tick_phase = sys.argv[15]
pos_max_past_slot_lag = sys.argv[16]
reward_runtime_enable = sys.argv[17]
reward_runtime_epoch_duration_secs = sys.argv[18]
reward_points_per_credit = sys.argv[19]
reward_runtime_auto_redeem = sys.argv[20]
actual_inventory_sha256 = hashlib.sha256(inventory_path.read_bytes()).hexdigest()
if actual_inventory_sha256 != expected_inventory_sha256:
    raise SystemExit("triad inventory digest mismatch before node.env materialization")
actual_registry_sha256 = hashlib.sha256(registry_path.read_bytes()).hexdigest()
inventory = json.loads(inventory_path.read_text(encoding="utf-8"))
registry = json.loads(registry_path.read_text(encoding="utf-8"))
manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
canonical_registry = {
    "signer_bindings": {
        f"governance.finality.v1.{item['node_id']}": str(item["finality_signer_public_key"]).lower()
        for item in registry.get("validators", [])
    },
    "slot_id": registry.get("slot_id"),
    "threshold": registry.get("threshold"),
    "threshold_bps": registry.get("threshold_bps"),
    "validator_stakes": {
        f"governance.finality.v1.{item['node_id']}": item["stake"]
        for item in registry.get("validators", [])
    },
}
actual_registry_semantic_sha256 = hashlib.sha256(
    json.dumps(canonical_registry, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()
).hexdigest()
authority = inventory.get("authority", {})
if actual_registry_sha256 != authority.get("generated_registry_sha256"):
    raise SystemExit("validator-47 node.env generated registry digest mismatch")
if actual_registry_semantic_sha256 != authority.get("generated_registry_semantic_sha256"):
    raise SystemExit("validator-47 node.env generated registry semantic digest mismatch")
if manifest.get("deployment_validator_registry") != {
    "ref": "config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json",
    "sha256": actual_registry_sha256,
    "semantic_sha256": actual_registry_semantic_sha256,
}:
    raise SystemExit("validator-47 node.env manifest registry authority mismatch")
target = inventory["nodes"]["validator-47"]
if target["node_id"] != "triad-testnet-validator-47":
    raise SystemExit("validator-47 node.env NODE_ID mismatch")
if target["service"] != "oasis7-triad-validator-47.service":
    raise SystemExit("validator-47 node.env service mismatch")
if target["roles"] != ["validator", "checkpoint_provider", "full_storage_provider"]:
    raise SystemExit("validator-47 node.env provider role mismatch")
if target["ports"] != ["6634", "6834"]:
    raise SystemExit("validator-47 node.env port binding mismatch")
if manifest.get("network_id") != "oasis7-public-testnet-governed-20260606":
    raise SystemExit("validator-47 node.env network identity mismatch")
if manifest.get("chain_id") != "oasis7-public-testnet-governed-20260606":
    raise SystemExit("validator-47 node.env chain identity mismatch")
if manifest.get("tier") != "public_testnet":
    raise SystemExit("validator-47 node.env network tier mismatch")
validators = registry.get("validators", [])
if not any(item.get("node_id") == target["node_id"] and item.get("stake") == 100 for item in validators):
    raise SystemExit("validator-47 node.env registry binding mismatch")
lines = [
    "# Governed public-testnet triad validator-47 staging contract",
    "NODE_ID=triad-testnet-validator-47",
    "# Inventory role is validator; runtime role is storage with P2P full_storage.",
    "NODE_ROLE=storage",
    "P2P_NODE_ROLE=full_storage",
    "STATUS_BIND=0.0.0.0:6634",
    "NODE_GOSSIP_BIND=0.0.0.0:6834",
    "REPLICATION_NETWORK_LISTEN_ADDRS_CSV=/ip4/0.0.0.0/tcp/6834",
    "CHECKPOINT_PROVIDER=1",
    "FULL_STORAGE_PROVIDER=1",
    "WORLD_ID=oasis7-public-testnet-governed-20260606",
    "NETWORK_TIER_MANIFEST_PATH=config/public-testnet-governed-bootstrap-manifest-2026-06-06.json",
    "GENESIS_VALIDATOR_REGISTRY_PATH=config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json",
    f"GENESIS_VALIDATOR_REGISTRY_SHA256={actual_registry_sha256}",
    f"GENESIS_VALIDATOR_REGISTRY_SEMANTIC_SHA256={actual_registry_semantic_sha256}",
    "EXECUTION_WORLD_DIR=staged-world",
    f"CONFIG_PATH={config_path}",
    f"EXECUTION_RECORDS_DIR={execution_records_dir}",
    f"STORAGE_ROOT={storage_root}",
    f"STORAGE_PROFILE={storage_profile}",
    f"NODE_TICK_MS={node_tick_ms}",
    f"POS_SLOT_DURATION_MS={pos_slot_duration_ms}",
    f"POS_TICKS_PER_SLOT={pos_ticks_per_slot}",
    f"POS_PROPOSAL_TICK_PHASE={pos_proposal_tick_phase}",
    f"POS_MAX_PAST_SLOT_LAG={pos_max_past_slot_lag}",
    f"POS_ADAPTIVE_TICK_SCHEDULER=1",
    f"REWARD_RUNTIME_ENABLE={reward_runtime_enable}",
    f"REWARD_RUNTIME_EPOCH_DURATION_SECS={reward_runtime_epoch_duration_secs}",
    f"REWARD_POINTS_PER_CREDIT={reward_points_per_credit}",
    f"REWARD_RUNTIME_AUTO_REDEEM={reward_runtime_auto_redeem}",
    "DEPLOYMENT_INVENTORY_PATH=config/public-testnet-validator-triad-inventory.v1.json",
    f"DEPLOYMENT_INVENTORY_SHA256={actual_inventory_sha256}",
    "BOOTSTRAP_PEER_PATH=config/public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.txt",
    f"BOOTSTRAP_PEER_SHA256={hashlib.sha256((manifest_path.parent / 'public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.txt').read_bytes()).hexdigest()}",
    "IDENTITY_KEY_PATH=config/node-keypair.toml",
    "IDENTITY_RECEIPT_PATH=config/identity-receipt.json",
    f"IDENTITY_KEY_SHA256={identity_key_sha256}",
    f"IDENTITY_RECEIPT_SHA256={identity_receipt_sha256}",
]
node_env_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
PY
  cp "$node_env_path" "$out_dir/config/doc/testing/evidence/"
fi

printf '%s\n' "$out_dir"
