#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT

manifest_path="$tmpdir/public-testnet-smoke.json"

./scripts/network-tier-manifest.sh create \
  --manifest "$manifest_path" \
  --tier public_testnet \
  --status specified_skeleton_only \
  --network-id oasis7-public-testnet-smoke \
  --chain-id oasis7-public-testnet-smoke \
  --release-candidate-bundle-ref output/release-candidates/public-testnet-smoke.json \
  --genesis-ref doc/testing/templates/public-testnet-genesis.example.json \
  --bootstrap-peer-ref doc/testing/templates/public-testnet-bootstrap.example.txt \
  --rpc-ref https://public-testnet.example.invalid/rpc \
  --explorer-ref https://public-testnet.example.invalid/explorer \
  --faucet-ref https://public-testnet.example.invalid/faucet \
  --governance-mode shared_ops \
  --validator-admission allowlist_or_governed_candidate \
  --target-validator-count 4 \
  --allow-observer-nodes true \
  --token-symbol OC \
  --faucet-mode guarded_testnet_faucet \
  --reset-policy resettable \
  --value-semantics testnet \
  --promote-from shared_devnet \
  --require-gate shared_devnet_pass \
  --require-gate public_rpc_ready \
  --require-gate faucet_guard_ready \
  --require-gate reset_policy_announced \
  --allowed-claim public_testnet \
  --denied-claim mainnet_live \
  --denied-claim production_oc_settlement \
  --evidence-ref doc/testing/evidence/public-testnet-skeleton-example.md >/dev/null

./scripts/network-tier-manifest.sh validate --manifest "$manifest_path" >/dev/null
./scripts/network-tier-manifest.sh validate --manifest doc/testing/templates/network-tier-shared-devnet.example.json >/dev/null
./scripts/network-tier-manifest.sh validate --manifest doc/testing/templates/network-tier-public-testnet.example.json >/dev/null
./scripts/network-tier-manifest.sh validate --manifest doc/testing/templates/network-tier-mainnet.example.json >/dev/null
./scripts/network-tier-exit-review.sh --manifest doc/testing/templates/network-tier-public-testnet.example.json >/dev/null
./scripts/network-tier-exit-review.sh --manifest doc/testing/templates/network-tier-mainnet.example.json >/dev/null

echo "network-tier-manifest smoke passed"
