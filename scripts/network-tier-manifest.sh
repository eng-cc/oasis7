#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

usage() {
  cat <<'USAGE'
Usage:
  ./scripts/network-tier-manifest.sh create [options]
  ./scripts/network-tier-manifest.sh validate --manifest <path>

Purpose:
  Freeze and validate one machine-readable formal network tier manifest for:
  - local_devnet
  - shared_devnet
  - public_testnet
  - mainnet

Create options:
  --manifest <path>                    Output manifest json path (required)
  --tier <name>                        local_devnet|shared_devnet|public_testnet|mainnet
  --status <name>                      planned|specified_skeleton_only|rehearsal|live
  --network-id <id>                    Stable network id
  --chain-id <id>                      Stable chain id
  --release-candidate-bundle-ref <ref> Release candidate bundle ref
  --genesis-ref <ref>                  Genesis config/world ref
  --bootstrap-peer-ref <ref>           Bootstrap peer / peer-set ref
  --rpc-ref <ref>                      Public or operator RPC ref
  --explorer-ref <ref>                 Explorer ref
  --faucet-ref <ref>                   Faucet ref (use '-' when absent)
  --governance-mode <name>             bootstrap_local|shared_ops|governance_registry
  --validator-admission <name>         local_only|shared_allowlist|allowlist_or_governed_candidate|governance_registry_only
  --target-validator-count <n>         Target validator count
  --allow-observer-nodes <bool>        true|false
  --token-symbol <symbol>              Usually OC
  --faucet-mode <name>                 none|operator_grant|guarded_testnet_faucet
  --reset-policy <name>                ephemeral|resettable|frozen
  --value-semantics <name>             preview|testnet|production
  --promote-from <tier>                Repeatable source tier
  --require-gate <gate>                Repeatable gate requirement
  --allowed-claim <text>               Repeatable allowed claim
  --denied-claim <text>                Repeatable denied claim
  --evidence-ref <ref>                 Repeatable evidence ref

Validate options:
  --manifest <path>                    Manifest json path (required)

Examples:
  ./scripts/network-tier-manifest.sh validate \
    --manifest doc/testing/templates/network-tier-public-testnet.example.json
USAGE
}

mode=${1:-}
if [[ -z "$mode" ]]; then
  usage >&2
  exit 2
fi
shift || true

manifest_path=""
tier=""
status=""
network_id=""
chain_id=""
release_candidate_bundle_ref=""
genesis_ref=""
bootstrap_peer_ref=""
rpc_ref=""
explorer_ref=""
faucet_ref=""
governance_mode=""
validator_admission=""
target_validator_count=""
allow_observer_nodes=""
token_symbol="OC"
faucet_mode=""
reset_policy=""
value_semantics=""
declare -a promote_from=()
declare -a require_gates=()
declare -a allowed_claims=()
declare -a denied_claims=()
declare -a evidence_refs=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --manifest)
      manifest_path=${2:-}
      shift 2
      ;;
    --tier)
      tier=${2:-}
      shift 2
      ;;
    --status)
      status=${2:-}
      shift 2
      ;;
    --network-id)
      network_id=${2:-}
      shift 2
      ;;
    --chain-id)
      chain_id=${2:-}
      shift 2
      ;;
    --release-candidate-bundle-ref)
      release_candidate_bundle_ref=${2:-}
      shift 2
      ;;
    --genesis-ref)
      genesis_ref=${2:-}
      shift 2
      ;;
    --bootstrap-peer-ref)
      bootstrap_peer_ref=${2:-}
      shift 2
      ;;
    --rpc-ref)
      rpc_ref=${2:-}
      shift 2
      ;;
    --explorer-ref)
      explorer_ref=${2:-}
      shift 2
      ;;
    --faucet-ref)
      faucet_ref=${2:-}
      shift 2
      ;;
    --governance-mode)
      governance_mode=${2:-}
      shift 2
      ;;
    --validator-admission)
      validator_admission=${2:-}
      shift 2
      ;;
    --target-validator-count)
      target_validator_count=${2:-}
      shift 2
      ;;
    --allow-observer-nodes)
      allow_observer_nodes=${2:-}
      shift 2
      ;;
    --token-symbol)
      token_symbol=${2:-}
      shift 2
      ;;
    --faucet-mode)
      faucet_mode=${2:-}
      shift 2
      ;;
    --reset-policy)
      reset_policy=${2:-}
      shift 2
      ;;
    --value-semantics)
      value_semantics=${2:-}
      shift 2
      ;;
    --promote-from)
      promote_from+=("${2:-}")
      shift 2
      ;;
    --require-gate)
      require_gates+=("${2:-}")
      shift 2
      ;;
    --allowed-claim)
      allowed_claims+=("${2:-}")
      shift 2
      ;;
    --denied-claim)
      denied_claims+=("${2:-}")
      shift 2
      ;;
    --evidence-ref)
      evidence_refs+=("${2:-}")
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

require_non_empty() {
  local flag=$1
  local value=$2
  if [[ -z "$value" ]]; then
    echo "error: missing required option: $flag" >&2
    exit 2
  fi
}

case "$mode" in
  create)
    require_non_empty "--manifest" "$manifest_path"
    require_non_empty "--tier" "$tier"
    require_non_empty "--status" "$status"
    require_non_empty "--network-id" "$network_id"
    require_non_empty "--chain-id" "$chain_id"
    require_non_empty "--release-candidate-bundle-ref" "$release_candidate_bundle_ref"
    require_non_empty "--genesis-ref" "$genesis_ref"
    require_non_empty "--bootstrap-peer-ref" "$bootstrap_peer_ref"
    require_non_empty "--rpc-ref" "$rpc_ref"
    require_non_empty "--explorer-ref" "$explorer_ref"
    require_non_empty "--governance-mode" "$governance_mode"
    require_non_empty "--validator-admission" "$validator_admission"
    require_non_empty "--target-validator-count" "$target_validator_count"
    require_non_empty "--allow-observer-nodes" "$allow_observer_nodes"
    require_non_empty "--token-symbol" "$token_symbol"
    require_non_empty "--faucet-mode" "$faucet_mode"
    require_non_empty "--reset-policy" "$reset_policy"
    require_non_empty "--value-semantics" "$value_semantics"
    mkdir -p "$(dirname "$manifest_path")"
    NETWORK_TIER_MANIFEST="$manifest_path" python3 - "$tier" "$status" "$network_id" "$chain_id" "$release_candidate_bundle_ref" "$genesis_ref" "$bootstrap_peer_ref" "$rpc_ref" "$explorer_ref" "$faucet_ref" "$governance_mode" "$validator_admission" "$target_validator_count" "$allow_observer_nodes" "$token_symbol" "$faucet_mode" "$reset_policy" "$value_semantics" "${promote_from[*]}" "${require_gates[*]}" "${allowed_claims[*]}" "${denied_claims[*]}" "${evidence_refs[*]}" <<'PY'
import json
import os
import pathlib
import sys

manifest_path = pathlib.Path(os.environ["NETWORK_TIER_MANIFEST"]).resolve()
(
    tier,
    status,
    network_id,
    chain_id,
    release_candidate_bundle_ref,
    genesis_ref,
    bootstrap_peer_ref,
    rpc_ref,
    explorer_ref,
    faucet_ref,
    governance_mode,
    validator_admission,
    target_validator_count,
    allow_observer_nodes,
    token_symbol,
    faucet_mode,
    reset_policy,
    value_semantics,
    promote_from_raw,
    require_gates_raw,
    allowed_claims_raw,
    denied_claims_raw,
    evidence_refs_raw,
) = sys.argv[1:]

def split_items(raw: str) -> list[str]:
    if not raw:
        return []
    return [item for item in raw.split(" ") if item]

manifest = {
    "schema_version": "oasis7.network_tier_manifest.v1",
    "tier": tier,
    "status": status,
    "network_id": network_id,
    "chain_id": chain_id,
    "runtime_refs": {
        "release_candidate_bundle_ref": release_candidate_bundle_ref,
        "genesis_ref": genesis_ref,
        "bootstrap_peer_ref": bootstrap_peer_ref,
    },
    "endpoint_policy": {
        "rpc_ref": rpc_ref,
        "explorer_ref": explorer_ref,
        "faucet_ref": None if faucet_ref in ("", "-") else faucet_ref,
    },
    "validator_policy": {
        "governance_mode": governance_mode,
        "validator_admission": validator_admission,
        "target_validator_count": int(target_validator_count),
        "allow_observer_nodes": allow_observer_nodes.lower() == "true",
    },
    "token_policy": {
        "symbol": token_symbol,
        "faucet_mode": faucet_mode,
        "reset_policy": reset_policy,
        "value_semantics": value_semantics,
    },
    "claims_policy": {
        "allowed_claims": split_items(allowed_claims_raw),
        "denied_claims": split_items(denied_claims_raw),
    },
    "promotion_policy": {
        "promote_from": split_items(promote_from_raw),
        "required_gates": split_items(require_gates_raw),
    },
    "evidence_refs": split_items(evidence_refs_raw),
}

manifest_path.write_text(json.dumps(manifest, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")
PY
    "$0" validate --manifest "$manifest_path" >/dev/null
    echo "$manifest_path"
    ;;
  validate)
    require_non_empty "--manifest" "$manifest_path"
    if [[ ! -f "$manifest_path" ]]; then
      echo "error: manifest not found: $manifest_path" >&2
      exit 2
    fi
    python3 - "$manifest_path" <<'PY'
import json
import pathlib
import sys

manifest_path = pathlib.Path(sys.argv[1]).resolve()
with manifest_path.open("r", encoding="utf-8") as fh:
    data = json.load(fh)

required_top = [
    "schema_version",
    "tier",
    "status",
    "network_id",
    "chain_id",
    "runtime_refs",
    "endpoint_policy",
    "validator_policy",
    "token_policy",
    "claims_policy",
    "promotion_policy",
    "evidence_refs",
]
for field in required_top:
    if field not in data:
        raise SystemExit(f"missing top-level field: {field}")

if data["schema_version"] != "oasis7.network_tier_manifest.v1":
    raise SystemExit("unsupported schema_version")

tiers = {"local_devnet", "shared_devnet", "public_testnet", "mainnet"}
statuses = {"planned", "specified_skeleton_only", "rehearsal", "live"}
governance_modes = {"bootstrap_local", "shared_ops", "governance_registry"}
validator_admissions = {
    "local_only",
    "shared_allowlist",
    "allowlist_or_governed_candidate",
    "governance_registry_only",
}
faucet_modes = {"none", "operator_grant", "guarded_testnet_faucet"}
reset_policies = {"ephemeral", "resettable", "frozen"}
value_semantics = {"preview", "testnet", "production"}

def resolve_ref(raw: str) -> pathlib.Path:
    candidate = pathlib.Path(raw)
    if candidate.is_absolute():
        return candidate
    manifest_relative = manifest_path.parent / candidate
    if manifest_relative.exists():
        return manifest_relative.resolve()
    return (pathlib.Path.cwd() / candidate).resolve()

def require_enum(name: str, value: str, allowed: set[str]) -> None:
    if value not in allowed:
        raise SystemExit(f"invalid {name}: {value}")

require_enum("tier", data["tier"], tiers)
require_enum("status", data["status"], statuses)
require_enum("governance_mode", data["validator_policy"]["governance_mode"], governance_modes)
require_enum("validator_admission", data["validator_policy"]["validator_admission"], validator_admissions)
require_enum("faucet_mode", data["token_policy"]["faucet_mode"], faucet_modes)
require_enum("reset_policy", data["token_policy"]["reset_policy"], reset_policies)
require_enum("value_semantics", data["token_policy"]["value_semantics"], value_semantics)

for field in ("network_id", "chain_id"):
    if not isinstance(data[field], str) or not data[field].strip():
        raise SystemExit(f"invalid {field}")

for field in ("release_candidate_bundle_ref", "genesis_ref", "bootstrap_peer_ref"):
    value = data["runtime_refs"].get(field)
    if not isinstance(value, str) or not value.strip():
        raise SystemExit(f"invalid runtime_refs.{field}")

genesis_ref_path = resolve_ref(data["runtime_refs"]["genesis_ref"])
bootstrap_ref_path = resolve_ref(data["runtime_refs"]["bootstrap_peer_ref"])
if not genesis_ref_path.is_file():
    raise SystemExit(f"missing runtime_refs.genesis_ref file: {genesis_ref_path}")
if not bootstrap_ref_path.is_file():
    raise SystemExit(f"missing runtime_refs.bootstrap_peer_ref file: {bootstrap_ref_path}")

for field in ("rpc_ref", "explorer_ref"):
    value = data["endpoint_policy"].get(field)
    if not isinstance(value, str) or not value.strip():
        raise SystemExit(f"invalid endpoint_policy.{field}")

target_validator_count = data["validator_policy"].get("target_validator_count")
allow_observer_nodes = data["validator_policy"].get("allow_observer_nodes")
if not isinstance(target_validator_count, int) or target_validator_count <= 0:
    raise SystemExit("invalid validator_policy.target_validator_count")
if not isinstance(allow_observer_nodes, bool):
    raise SystemExit("invalid validator_policy.allow_observer_nodes")

for field in ("allowed_claims", "denied_claims"):
    value = data["claims_policy"].get(field)
    if not isinstance(value, list):
        raise SystemExit(f"invalid claims_policy.{field}")
    if any(not isinstance(item, str) or not item.strip() for item in value):
        raise SystemExit(f"invalid claims_policy.{field} entry")

for field in ("promote_from", "required_gates"):
    value = data["promotion_policy"].get(field)
    if not isinstance(value, list):
        raise SystemExit(f"invalid promotion_policy.{field}")
    if any(not isinstance(item, str) or not item.strip() for item in value):
        raise SystemExit(f"invalid promotion_policy.{field} entry")

if not isinstance(data["evidence_refs"], list):
    raise SystemExit("invalid evidence_refs")

tier = data["tier"]
token_policy = data["token_policy"]
validator_policy = data["validator_policy"]
endpoint_policy = data["endpoint_policy"]
claims_policy = data["claims_policy"]

if tier == "local_devnet":
    if token_policy["value_semantics"] != "preview":
        raise SystemExit("local_devnet must use value_semantics=preview")
    if token_policy["reset_policy"] != "ephemeral":
        raise SystemExit("local_devnet must use reset_policy=ephemeral")
    if validator_policy["validator_admission"] != "local_only":
        raise SystemExit("local_devnet must use validator_admission=local_only")

if tier == "shared_devnet":
    if token_policy["value_semantics"] != "preview":
        raise SystemExit("shared_devnet must use value_semantics=preview")
    if token_policy["reset_policy"] not in {"ephemeral", "resettable"}:
        raise SystemExit("shared_devnet must use resettable-style policy")

if tier == "public_testnet":
    if token_policy["value_semantics"] != "testnet":
        raise SystemExit("public_testnet must use value_semantics=testnet")
    if token_policy["reset_policy"] != "resettable":
        raise SystemExit("public_testnet must use reset_policy=resettable")
    if token_policy["faucet_mode"] != "guarded_testnet_faucet":
        raise SystemExit("public_testnet must use faucet_mode=guarded_testnet_faucet")
    if validator_policy["validator_admission"] not in {"allowlist_or_governed_candidate", "shared_allowlist"}:
        raise SystemExit("public_testnet validator admission is too weak or too strong")
    if not endpoint_policy.get("faucet_ref"):
        raise SystemExit("public_testnet requires endpoint_policy.faucet_ref")

if tier == "mainnet":
    if token_policy["value_semantics"] != "production":
        raise SystemExit("mainnet must use value_semantics=production")
    if token_policy["reset_policy"] != "frozen":
        raise SystemExit("mainnet must use reset_policy=frozen")
    if token_policy["faucet_mode"] != "none":
        raise SystemExit("mainnet must use faucet_mode=none")
    if validator_policy["governance_mode"] != "governance_registry":
        raise SystemExit("mainnet must use governance_mode=governance_registry")
    if validator_policy["validator_admission"] != "governance_registry_only":
        raise SystemExit("mainnet must use validator_admission=governance_registry_only")
    if endpoint_policy.get("faucet_ref") not in (None, ""):
        raise SystemExit("mainnet must not define faucet_ref")
    required_gates = set(data["promotion_policy"]["required_gates"])
    for gate in ("MAINNET-1", "MAINNET-2", "MAINNET-3", "MAINNET-4"):
        if gate not in required_gates:
            raise SystemExit(f"mainnet missing required gate: {gate}")

joined_denied = " ".join(claims_policy["denied_claims"]).lower()
if tier != "mainnet" and "mainnet" not in joined_denied:
    raise SystemExit("non-mainnet tiers must explicitly deny mainnet claims")

joined_allowed = " ".join(claims_policy["allowed_claims"]).lower()
if tier == "public_testnet" and "public_testnet" not in joined_allowed:
    raise SystemExit("public_testnet must explicitly allow public_testnet claims")
if tier == "public_testnet" and "production_oc_settlement" not in joined_denied:
    raise SystemExit("public_testnet must explicitly deny production_oc_settlement claims")
if tier == "shared_devnet" and "public_testnet" in joined_allowed:
    raise SystemExit("shared_devnet must not allow public_testnet claims")
if tier == "mainnet" and "faucet" in joined_allowed:
    raise SystemExit("mainnet must not allow faucet claims")

print(json.dumps(
    {
        "schema_version": data["schema_version"],
        "manifest_path": str(manifest_path),
        "genesis_ref_path": str(genesis_ref_path),
        "bootstrap_peer_ref_path": str(bootstrap_ref_path),
        "tier": tier,
        "status": data["status"],
        "network_id": data["network_id"],
        "chain_id": data["chain_id"],
        "validator_count": target_validator_count,
        "validate_result": "pass",
    },
    ensure_ascii=True,
    indent=2,
) + "\n")
PY
    ;;
  *)
    echo "error: unsupported mode: $mode" >&2
    usage >&2
    exit 2
    ;;
esac
