# PR Evidence

## Unified Persistent World Terminology Migration

Task UID: `task_cb987cd0fdfb4ecc98a6ddde7d96204c`

Branch: `task/core-unified-world-terminology-upgrade-plan`

Summary:
- Make the product/default model a unified persistent world across active docs and site copy.
- Migrate active `shared_devnet` / `shared-network` manifest and script compatibility toward public-testnet rehearsal / network-rehearsal terminology.
- Keep old script names as compatibility wrappers only, with warnings that point to canonical entrypoints.
- Add a global follow-up task for code-layer work that is outside this terminology/compatibility slice.

Verification:
- `./scripts/network-tier-manifest-smoke.sh`
- `./scripts/shared-network-track-gate-smoke.sh`
- `./scripts/shared-devnet-rehearsal-smoke.sh`
- `./scripts/shared-devnet-blocker-packet-smoke.sh`
- `./scripts/release-candidate-bundle-smoke.sh`
- `./scripts/check-script-executable-bits.sh`
- `bash -n scripts/network-tier-manifest.sh scripts/network-rehearsal-track-gate.sh scripts/public-testnet-rehearsal.sh scripts/public-testnet-rehearsal-blocker-packet.sh scripts/shared-network-track-gate.sh scripts/shared-devnet-rehearsal.sh scripts/shared-devnet-blocker-packet.sh scripts/shared-network-track-gate-smoke.sh scripts/shared-devnet-rehearsal-smoke.sh scripts/shared-devnet-blocker-packet-smoke.sh scripts/network-tier-manifest-smoke.sh scripts/release-gate.sh`
- `./scripts/cargo-dev.sh test -p oasis7 --lib network_tier_manifest -- --nocapture`
- `git diff --check`

Known Deferred Boundaries:
- `./scripts/release-gate-smoke.sh` is not fully passing because downstream longrun scripts still use Bash 4-only features under macOS Bash 3.2.
- `./scripts/cargo-dev.sh test -p oasis7 --bin oasis7_chain_runtime network_tier -- --list` is not passing because the chain-runtime status/observability surface references API drift outside this terminology migration.

Follow-up:
- Global candidate task `task_acb7e3599b4242628a7ac99a62628d55` tracks the code-layer follow-up for chain-runtime API drift, release-gate longrun shell compatibility, legacy wrapper retirement, and code-level old-term regression scans.
