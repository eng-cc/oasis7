# Network Tier Signer Truth Binding (2026-06-05)

## Scope
- Record the historical `mainnet` and `public_testnet` public-only signer provenance, subject to the availability limits below.
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
- The two public-testnet JSON mirrors listed above are present and public-only. The two mainnet JSON paths are historical intended locators and are absent from the reviewed tree; do not treat them as available repo-owned mirrors or invent replacement signer material. Mainnet provenance remains the operator-local source batches recorded above and the public keys/thresholds in [the genesis freeze sheet](../../p2p/token/mainchain-token-genesis-freeze-sheet.md). The [historical governance drill](shared-network-shared-devnet-governance-live-drill-2026-05-23.md) explicitly identifies an operator-local public manifest. This note does not verify current external custody, recreate missing files or authorize key access.
- Mainnet controller thresholds and bucket/controller bindings remain governed by `doc/p2p/token/mainchain-token-genesis-freeze-sheet.md`.
- Public testnet remains a resettable test surface; binding these signer truths does not upgrade its claim boundary to `mainnet_live`.
