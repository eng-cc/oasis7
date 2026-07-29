# Network Tier Signer Truth Binding (2026-06-05)

## Scope
- Bind both `mainnet` and `public_testnet` repo-owned tier/genesis truth to concrete public-only signer material.
- Keep private keys and operator-local directory structure out of the repository.

## Bound Truth
- Mainnet governance signer truth:
  - `doc/testing/evidence/mainnet-governance-public-signers-2026-06-05.json`
- Mainnet liveops signer truth:
  - `doc/testing/evidence/mainnet-liveops-public-signers-2026-06-05.json`
- Public testnet governance signer truth:
  - `doc/testing/evidence/public-testnet-governance-public-signers-2026-06-05.json`
- Public testnet liveops signer truth:
  - `doc/testing/evidence/public-testnet-liveops-public-signers-2026-06-05.json`

## Source Batches
- Mainnet governance: `oasis7-governance-batch-20260323-01`
- Mainnet liveops: `oasis7-liveops-batch-20260330-01`
- Public testnet governance: `oasis7-governance-batch-20260605-01`
- Public testnet liveops: `oasis7-liveops-batch-20260605-01`

## Notes
- These repo-owned JSON mirrors are public-only signer truth. They intentionally omit private keys and operator-local custody paths.
- Mainnet controller thresholds and bucket/controller bindings remain governed by `doc/p2p/token/mainchain-token-genesis-freeze-sheet.md`.
- Public testnet remains a resettable test surface; binding these signer truths does not upgrade its claim boundary to `mainnet_live`.
