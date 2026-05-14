#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

usage() {
  cat <<'USAGE'
Usage:
  ./scripts/network-tier-exit-review.sh --manifest <path>

Purpose:
  Print a minimal exit-review summary for a validated formal network tier manifest.
  Current focus:
  - public_testnet -> exit review input
  - mainnet -> gate completeness review
USAGE
}

manifest_path=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --manifest)
      manifest_path="${2:-}"
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

if [[ -z "$manifest_path" ]]; then
  echo "error: --manifest is required" >&2
  exit 2
fi

./scripts/network-tier-manifest.sh validate --manifest "$manifest_path" >/dev/null

python3 - "$manifest_path" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1]).resolve()
data = json.loads(path.read_text(encoding="utf-8"))
tier = data["tier"]
gates = list(data["promotion_policy"]["required_gates"])
allowed_claims = list(data["claims_policy"]["allowed_claims"])
denied_claims = list(data["claims_policy"]["denied_claims"])

summary = {
    "manifest_path": str(path),
    "tier": tier,
    "status": data["status"],
    "network_id": data["network_id"],
    "chain_id": data["chain_id"],
    "required_gates": gates,
    "allowed_claims": allowed_claims,
    "denied_claims": denied_claims,
}

if tier == "public_testnet":
    required = {
        "shared_devnet_pass",
        "public_rpc_ready",
        "faucet_guard_ready",
        "reset_policy_announced",
    }
    summary["exit_review_readiness"] = "ready_for_rehearsal_review" if required.issubset(set(gates)) else "missing_required_public_testnet_gates"
elif tier == "mainnet":
    required = {"MAINNET-1", "MAINNET-2", "MAINNET-3", "MAINNET-4"}
    summary["exit_review_readiness"] = "ready_for_mainnet_gate_review" if required.issubset(set(gates)) else "missing_required_mainnet_gates"
else:
    summary["exit_review_readiness"] = "tier_has_no_formal_exit_review_flow"

print(json.dumps(summary, ensure_ascii=True, indent=2) + "\n")
PY
