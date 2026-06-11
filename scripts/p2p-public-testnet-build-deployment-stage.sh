#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/p2p-public-testnet-build-deployment-stage.sh \
    --runtime-build-ref <path> \
    --bootstrap-peers-file <path> \
    --sequencer-node-keypair <path> \
    --storage-node-keypair <path> \
    --out-dir <path>

  ./scripts/p2p-public-testnet-build-deployment-stage.sh \
    --runtime-build-ref <path> \
    --bootstrap-peers-file <path> \
    --sequencer-public-key <hex> \
    --storage-public-key <hex> \
    --out-dir <path>

Description:
  Build a deployment-only public_testnet stage from the current validator
  signer truth. This stage preserves the frozen public testnet chain/world
  identity but regenerates:
    - validator registry
    - genesis with deployment-only validator-registry ref
    - governed bootstrap world
    - release-candidate bundle pinned to the provided runtime build

  The output directory contains:
    <out-dir>/config/
    <out-dir>/generated-world/
    <out-dir>/deployment-truth.md
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

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

runtime_build_ref=""
bootstrap_peers_file=""
sequencer_node_keypair=""
storage_node_keypair=""
sequencer_public_key=""
storage_public_key=""
out_dir=""

base_genesis="doc/testing/evidence/public-testnet-governed-bootstrap-genesis-2026-06-06.json"
base_manifest="doc/testing/evidence/public-testnet-governed-bootstrap-manifest-2026-06-06.json"
candidate_id="public-testnet-governed-bootstrap-20260606"
track="public_testnet"
sequencer_node_id="triad-testnet-sequencer"
storage_node_id="triad-testnet-storage"
stake="100"

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
      sequencer_node_keypair=${2:-}
      shift 2
      ;;
    --storage-node-keypair)
      storage_node_keypair=${2:-}
      shift 2
      ;;
    --sequencer-public-key)
      sequencer_public_key=${2:-}
      shift 2
      ;;
    --storage-public-key)
      storage_public_key=${2:-}
      shift 2
      ;;
    --out-dir)
      out_dir=${2:-}
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

require_command jq
require_command python3

require_non_empty "--runtime-build-ref" "$runtime_build_ref"
require_non_empty "--bootstrap-peers-file" "$bootstrap_peers_file"
require_non_empty "--out-dir" "$out_dir"

require_file "$runtime_build_ref"
require_file "$bootstrap_peers_file"
require_file "$base_genesis"
require_file "$base_manifest"

extract_public_key_from_toml() {
  local path=$1
  python3 - "$path" <<'PY'
import sys
import tomllib
from pathlib import Path

path = Path(sys.argv[1])
with path.open("rb") as fh:
    payload = tomllib.load(fh)
node = payload.get("node", {})
value = (node.get("public_key") or "").strip()
if not value:
    raise SystemExit(f"missing node.public_key in {path}")
print(value)
PY
}

is_hex_32() {
  [[ "$1" =~ ^[0-9a-fA-F]{64}$ ]]
}

if [[ -n "$sequencer_node_keypair" ]]; then
  require_file "$sequencer_node_keypair"
  sequencer_public_key=$(extract_public_key_from_toml "$sequencer_node_keypair")
fi
if [[ -n "$storage_node_keypair" ]]; then
  require_file "$storage_node_keypair"
  storage_public_key=$(extract_public_key_from_toml "$storage_node_keypair")
fi

require_non_empty "--sequencer-node-keypair or --sequencer-public-key" "$sequencer_public_key"
require_non_empty "--storage-node-keypair or --storage-public-key" "$storage_public_key"
is_hex_32 "$sequencer_public_key" || die "sequencer public key must be 32-byte hex"
is_hex_32 "$storage_public_key" || die "storage public key must be 32-byte hex"

out_dir=$(python3 -c 'import pathlib,sys; print(pathlib.Path(sys.argv[1]).expanduser().resolve())' "$out_dir")
rm -rf "$out_dir"
mkdir -p "$out_dir/config/doc/testing/evidence" "$out_dir/generated-world"

registry_path="$out_dir/config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json"
genesis_path="$out_dir/config/public-testnet-governed-bootstrap-genesis-2026-06-06.json"
manifest_path="$out_dir/config/public-testnet-governed-bootstrap-manifest-2026-06-06.json"
bundle_path="$out_dir/config/public-testnet-governed-bootstrap-bundle-2026-06-06.json"
bootstrap_out="$out_dir/config/public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.txt"
deployment_truth_md="$out_dir/deployment-truth.md"
temp_genesis="$out_dir/.tmp-genesis.json"

cp "$bootstrap_peers_file" "$bootstrap_out"
cp "$bootstrap_out" "$out_dir/config/doc/testing/evidence/"

python3 - "$registry_path" "$sequencer_node_id" "$sequencer_public_key" "$storage_node_id" "$storage_public_key" "$stake" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
sequencer_node_id = sys.argv[2]
sequencer_public_key = sys.argv[3]
storage_node_id = sys.argv[4]
storage_public_key = sys.argv[5]
stake = int(sys.argv[6])

payload = {
    "slot_id": "governance.finality.v1",
    "threshold": 2,
    "threshold_bps": 0,
    "validators": [
        {
            "node_id": sequencer_node_id,
            "scheme": "ed25519",
            "finality_signer_public_key": sequencer_public_key,
            "stake": stake,
        },
        {
            "node_id": storage_node_id,
            "scheme": "ed25519",
            "finality_signer_public_key": storage_public_key,
            "stake": stake,
        },
    ],
}
path.write_text(json.dumps(payload, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")
PY
cp "$registry_path" "$out_dir/config/doc/testing/evidence/"

python3 - "$base_genesis" "$genesis_path" "$registry_path" <<'PY'
import json
import pathlib
import sys

base_path = pathlib.Path(sys.argv[1]).resolve()
out_path = pathlib.Path(sys.argv[2]).resolve()
registry_path = pathlib.Path(sys.argv[3]).resolve()

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

out_path.write_text(json.dumps(payload, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")
PY
cp "$genesis_path" "$out_dir/config/doc/testing/evidence/"

cp "$base_manifest" "$manifest_path"
cp "$manifest_path" "$out_dir/config/doc/testing/evidence/"

./scripts/p2p-build-governed-bootstrap-world.sh create \
  --genesis "$genesis_path" \
  --out-dir "$out_dir/generated-world" \
  --allow-overwrite >/dev/null

python3 - "$deployment_truth_md" "$runtime_build_ref" "$bootstrap_out" "$sequencer_node_id" "$sequencer_public_key" "$storage_node_id" "$storage_public_key" <<'PY'
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
runtime_build_ref = sys.argv[2]
bootstrap_peers_file = pathlib.Path(sys.argv[3])
sequencer_node_id = sys.argv[4]
sequencer_public_key = sys.argv[5]
storage_node_id = sys.argv[6]
storage_public_key = sys.argv[7]

content = f"""# Deployment Truth

- Runtime build: `{runtime_build_ref}`
- Bootstrap peers file: `{bootstrap_peers_file}`
- Validator signer truth:
  - `{sequencer_node_id}` -> `{sequencer_public_key}`
  - `{storage_node_id}` -> `{storage_public_key}`
"""
path.write_text(content, encoding="utf-8")
PY

./scripts/release-candidate-bundle.sh create \
  --bundle "$bundle_path" \
  --candidate-id "$candidate_id" \
  --track "$track" \
  --runtime-build-ref "$runtime_build_ref" \
  --world-snapshot-ref "$out_dir/generated-world/world" \
  --governance-manifest-ref "$registry_path" \
  --evidence-ref "$deployment_truth_md" \
  --note "Deployment-only bundle derived from current validator signer truth." \
  --allow-dirty-worktree >/dev/null
cp "$bundle_path" "$out_dir/config/doc/testing/evidence/"

./scripts/release-candidate-bundle.sh validate --bundle "$bundle_path" >/dev/null
./scripts/p2p-build-governed-bootstrap-world.sh validate \
  --world-dir "$out_dir/generated-world/world" \
  --merged-public-manifest "$out_dir/generated-world/merged-public-manifest-entries.json" >/dev/null

python3 - "$base_manifest" "$manifest_path" "$bundle_path" "$genesis_path" "$bootstrap_out" <<'PY'
import json
import pathlib
import sys

base_manifest = pathlib.Path(sys.argv[1])
manifest_path = pathlib.Path(sys.argv[2])
bundle_path = pathlib.Path(sys.argv[3])
genesis_path = pathlib.Path(sys.argv[4])
bootstrap_path = pathlib.Path(sys.argv[5])

payload = json.loads(base_manifest.read_text(encoding="utf-8"))
payload.setdefault("runtime_refs", {})
payload["runtime_refs"]["release_candidate_bundle_ref"] = bundle_path.name
payload["runtime_refs"]["genesis_ref"] = genesis_path.name
payload["runtime_refs"]["bootstrap_peer_ref"] = bootstrap_path.name
manifest_path.write_text(json.dumps(payload, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")
PY
cp "$manifest_path" "$out_dir/config/doc/testing/evidence/"

printf '%s\n' "$out_dir"
