#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/ci-required-dispatch-fixture.lib.sh"
trap ci_fixture_cleanup EXIT
ci_fixture_init

inventory="$ci_fixture_root/scripts/ci-required-capability-test-inventory.tsv"
[[ -s "$inventory" ]] || { echo "moved-call inventory is missing" >&2; exit 1; }
awk -F '\t' 'NR == 1 { if (NF != 5) exit 1; next } NF != 5 { exit 1 }' "$inventory"
for capability in doc_checker_contracts cargo_tooling_contracts workflow_governance packaging_contracts operational_contracts; do
  awk -F '\t' -v capability="$capability" 'NR > 1 && $3 == capability { found = 1 } END { exit !found }' "$inventory" || {
    echo "moved-call inventory is missing capability $capability" >&2
    exit 1
  }
done

ci_fixture_run required strict OASIS7_CI_RUN_DOC_CHECKER_CONTRACTS=true
ci_fixture_assert_has 'product-doc-governance-check.test.py'
ci_fixture_assert_lacks 'pm/ci-ready-receipt.test.py'
ci_fixture_assert_lacks 'native-packaging-contract.test.sh'
ci_fixture_assert_lacks 'p2p-public-testnet-identity-v2-signing-tool.test.py'
ci_fixture_assert_inventory_selection_dispatched doc_checker_contracts

ci_fixture_run required strict \
  OASIS7_CI_RUN_CARGO_TOOLING_CONTRACTS=true \
  OASIS7_CI_NEEDS_RUST_TOOLCHAIN=true \
  OASIS7_CI_RUN_RUST_BASELINE=true
ci_fixture_assert_has 'cargo-dev-windows-toolchain.test.sh'
ci_fixture_assert_has 'check-standalone-tool-lockfiles.test.sh'
ci_fixture_assert_lacks 'product-doc-governance-check.test.py'
ci_fixture_assert_lacks 'pm/ci-ready-receipt.test.py'
ci_fixture_assert_lacks 'native-packaging-contract.test.sh'
ci_fixture_assert_lacks 'p2p-public-testnet-identity-v2-signing-tool.test.py'
ci_fixture_assert_lacks 'TOOL:rustup:'
ci_fixture_assert_has 'TOOL:cargo:'
ci_fixture_assert_inventory_selection_dispatched cargo_tooling_contracts

ci_fixture_run required strict OASIS7_CI_RUN_WORKFLOW_GOVERNANCE_CONTRACTS=true
ci_fixture_assert_has 'pm/ci-ready-receipt.test.py'
ci_fixture_assert_has 'ci-tests-argument-contract.test.sh'
ci_fixture_assert_lacks 'product-doc-governance-check.test.py'
ci_fixture_assert_lacks 'native-packaging-contract.test.sh'
ci_fixture_assert_lacks 'p2p-public-testnet-identity-v2-signing-tool.test.py'
ci_fixture_assert_inventory_selection_dispatched workflow_governance

ci_fixture_run required strict OASIS7_CI_RUN_PACKAGING_CONTRACTS=true
ci_fixture_assert_has 'native-packaging-contract.test.sh'
ci_fixture_assert_has 'testnet-packages-macos-arm64-contract.test.sh'
ci_fixture_assert_lacks 'pm/ci-ready-receipt.test.py'
ci_fixture_assert_lacks 'p2p-public-testnet-package-node-upgrade-health.test.sh'
ci_fixture_assert_lacks 'p2p-public-testnet-identity-v2-signing-tool.test.py'
ci_fixture_assert_inventory_selection_dispatched packaging_contracts

ci_fixture_run required strict OASIS7_CI_RUN_OPERATIONAL_CONTRACTS=true
ci_fixture_assert_has 'p2p-public-testnet-full-network-clean-room.test.py'
ci_fixture_assert_has 'p2p-public-testnet-identity-v2-signing-tool.test.py'
ci_fixture_assert_has 'p2p-public-testnet-package-node-upgrade-health.test.sh'
ci_fixture_assert_lacks 'pm/ci-ready-receipt.test.py'
ci_fixture_assert_lacks 'native-packaging-contract.test.sh'
ci_fixture_assert_lacks 'testnet-packages-macos-arm64-contract.test.sh'
ci_fixture_assert_lacks 'TOOL:cargo:'
ci_fixture_assert_lacks 'TOOL:rustup:'
ci_fixture_assert_lacks 'TOOL:curl:'
ci_fixture_assert_lacks 'TOOL:wget:'
ci_fixture_assert_inventory_selection_dispatched operational_contracts

ci_fixture_run required strict \
  OASIS7_CI_RUN_DOC_CHECKER_CONTRACTS=true \
  OASIS7_CI_RUN_CARGO_TOOLING_CONTRACTS=true \
  OASIS7_CI_RUN_WORKFLOW_GOVERNANCE_CONTRACTS=true \
  OASIS7_CI_RUN_PACKAGING_CONTRACTS=true \
  OASIS7_CI_RUN_OPERATIONAL_CONTRACTS=true \
  OASIS7_CI_NEEDS_RUST_TOOLCHAIN=true \
  OASIS7_CI_RUN_RUST_BASELINE=true
ci_fixture_assert_has 'product-doc-governance-check.test.py'
ci_fixture_assert_has 'cargo-dev-windows-toolchain.test.sh'
ci_fixture_assert_has 'pm/ci-ready-receipt.test.py'
ci_fixture_assert_has 'native-packaging-contract.test.sh'
ci_fixture_assert_has 'p2p-public-testnet-identity-v2-signing-tool.test.py'

for tier in full full-core full-support; do
  ci_fixture_run "$tier" full
  ci_fixture_assert_has 'product-doc-governance-check.test.py'
  ci_fixture_assert_has 'cargo-dev-windows-toolchain.test.sh'
  ci_fixture_assert_has 'pm/ci-ready-receipt.test.py'
  ci_fixture_assert_has 'native-packaging-contract.test.sh'
  ci_fixture_assert_has 'testnet-packages-macos-arm64-contract.test.sh'
  ci_fixture_assert_has 'p2p-public-testnet-identity-v2-signing-tool.test.py'
  ci_fixture_assert_lacks 'provider-bridge-live-gate.sh'
  ci_fixture_assert_lacks 'hosted-account-staging-smoke.sh'
  for capability in doc_checker_contracts cargo_tooling_contracts workflow_governance packaging_contracts operational_contracts; do
    ci_fixture_assert_inventory_selection_dispatched "$capability"
  done
done

echo "ci-required-domain-isolation.test: passed"
