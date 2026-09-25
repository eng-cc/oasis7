#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSIONED_TEST_CONFIG="$ROOT_DIR/scripts/fixtures/ci-required-scope.versioned-test.json"

plan_for_path() {
  "$ROOT_DIR/scripts/plan-rust-required-scope.sh" \
    --event-name pull_request \
    --config "$VERSIONED_TEST_CONFIG" \
    --changed-path "$1"
}

plan_for_paths() {
  local args=(--event-name pull_request --config "$VERSIONED_TEST_CONFIG")
  local path
  for path in "$@"; do
    args+=(--changed-path "$path")
  done
  "$ROOT_DIR/scripts/plan-rust-required-scope.sh" "${args[@]}"
}

value_for_key() {
  local output="$1"
  local key="$2"
  printf '%s\n' "$output" | awk -F= -v key="$key" '$1 == key {print substr($0, length(key) + 2)}'
}

assert_key_equals() {
  local output="$1"
  local key="$2"
  local expected="$3"
  local actual
  actual="$(value_for_key "$output" "$key")"
  if [[ "$actual" != "$expected" ]]; then
    echo "expected $key=$expected, got $actual" >&2
    printf '%s\n' "$output" >&2
    exit 1
  fi
}

assert_reason_contains() {
  local output="$1"
  local expected="$2"
  local actual
  actual="$(value_for_key "$output" reason_summary)"
  if [[ "$actual" != *"$expected"* ]]; then
    echo "expected reason_summary to contain $expected, got $actual" >&2
    printf '%s\n' "$output" >&2
    exit 1
  fi
}

assert_reason_absent() {
  local output="$1"
  local unexpected="$2"
  local actual
  actual="$(value_for_key "$output" reason_summary)"
  if [[ "$actual" == *"$unexpected"* ]]; then
    echo "expected reason_summary not to contain $unexpected, got $actual" >&2
    printf '%s\n' "$output" >&2
    exit 1
  fi
}

assert_key_matches() {
  local output="$1"
  local key="$2"
  local pattern="$3"
  local actual
  actual="$(value_for_key "$output" "$key")"
  if [[ ! "$actual" =~ $pattern ]]; then
    echo "expected $key to match $pattern, got $actual" >&2
    printf '%s\n' "$output" >&2
    exit 1
  fi
}

# The checked-in W policy is the policy used for production planning, so keep
# its domain-split contract live instead of testing only the versioned fixture.
if ! cmp -s "$ROOT_DIR/scripts/ci-required-scope.v2.json" "$VERSIONED_TEST_CONFIG"; then
  echo "trusted W policy drifted from the versioned contract fixture" >&2
  exit 1
fi
trusted_policy_output="$($ROOT_DIR/scripts/plan-rust-required-scope.sh \
  --event-name pull_request --changed-path doc/product/example.prd.md)"
assert_key_equals "$trusted_policy_output" execution_contract required-domain-split/v1
assert_key_equals "$trusted_policy_output" run_required_gate_baseline true
assert_key_equals "$trusted_policy_output" run_rust_baseline false
assert_key_equals "$trusted_policy_output" needs_python true
assert_key_equals "$trusted_policy_output" needs_markdown true

trusted_doc_checker_output="$($ROOT_DIR/scripts/plan-rust-required-scope.sh \
  --event-name pull_request --changed-path scripts/product-doc-governance-check.test.py)"
assert_key_equals "$trusted_doc_checker_output" run_doc_checker_contracts true
assert_key_equals "$trusted_doc_checker_output" run_cargo_tooling_contracts false
assert_key_equals "$trusted_doc_checker_output" needs_rust_toolchain false

trusted_cargo_tooling_output="$($ROOT_DIR/scripts/plan-rust-required-scope.sh \
  --event-name pull_request --changed-path scripts/cargo-dev-lib.test.sh)"
assert_key_equals "$trusted_cargo_tooling_output" run_cargo_tooling_contracts true
assert_key_equals "$trusted_cargo_tooling_output" run_doc_checker_contracts false
assert_key_equals "$trusted_cargo_tooling_output" needs_rust_toolchain true

product_doc_output="$(plan_for_path doc/product/world-rules-core-gameplay.prd.md)"
assert_key_equals "$product_doc_output" scope minimal
assert_key_equals "$product_doc_output" run_rust_baseline false
assert_key_equals "$product_doc_output" needs_rust_toolchain false
assert_key_equals "$product_doc_output" needs_node false
assert_key_equals "$product_doc_output" needs_python true
assert_key_equals "$product_doc_output" needs_markdown true
assert_key_equals "$product_doc_output" execution_contract required-domain-split/v1
assert_key_equals "$product_doc_output" required_test_units required_gate_baseline
assert_key_matches "$product_doc_output" planner_config_sha256 '^sha256:[0-9a-f]{64}$'
assert_reason_contains "$product_doc_output" "governance_doc:doc/product/world-rules-core-gameplay.prd.md"

if ! "$ROOT_DIR/scripts/plan-rust-required-scope.sh" --help | grep -Fq -- '--impact-projection'; then
  echo "required-scope planner must expose the digest-bound impact projection adapter" >&2
  exit 1
fi

site_output="$(plan_for_path site/index.html)"
assert_key_equals "$site_output" scope targeted
assert_key_equals "$site_output" selected_capabilities site_quality
assert_key_equals "$site_output" run_site_contract_tests true
assert_key_equals "$site_output" required_test_units 'required_gate_baseline;site_quality'
assert_key_equals "$site_output" run_rust_baseline false
assert_key_equals "$site_output" needs_rust_toolchain false
assert_key_equals "$site_output" needs_node false
assert_key_equals "$site_output" needs_system_deps false
assert_reason_contains "$site_output" "site_quality:site/index.html"
assert_reason_absent "$site_output" "unclassified_or_unresolvable:"

pages_workflow_output="$(plan_for_path .github/workflows/pages.yml)"
assert_key_equals "$pages_workflow_output" scope full
assert_key_equals "$pages_workflow_output" run_rust_baseline true
assert_key_equals "$pages_workflow_output" needs_rust_toolchain true
assert_reason_contains "$pages_workflow_output" "shared_required_gate:.github/workflows/pages.yml"

site_readme_output="$(plan_for_path README.md)"
assert_key_equals "$site_readme_output" scope targeted
assert_key_equals "$site_readme_output" selected_capabilities site_quality
assert_key_equals "$site_readme_output" run_site_contract_tests true
assert_key_equals "$site_readme_output" run_rust_baseline false
assert_key_equals "$site_readme_output" needs_rust_toolchain false

site_manual_output="$(plan_for_path doc/world-simulator/viewer/viewer-manual.manual.md)"
assert_key_equals "$site_manual_output" scope targeted
assert_key_equals "$site_manual_output" selected_capabilities site_quality
assert_key_equals "$site_manual_output" run_site_contract_tests true
assert_key_equals "$site_manual_output" run_rust_baseline false
assert_key_equals "$site_manual_output" needs_rust_toolchain false

launcher_output="$(plan_for_path crates/oasis7_client_launcher/src/lib.rs)"
assert_key_equals "$launcher_output" needs_node true
assert_key_equals "$launcher_output" needs_trunk true
assert_key_equals "$launcher_output" needs_rust_toolchain true
assert_key_equals "$launcher_output" needs_wasm_target true
assert_key_equals "$launcher_output" needs_system_deps true

overlap_output="$(plan_for_paths crates/oasis7_node/src/network_bridge.rs crates/oasis7_net/src/lib.rs)"
assert_key_equals "$overlap_output" scope targeted
assert_key_equals "$overlap_output" run_oasis7_node_tests true
assert_key_equals "$overlap_output" run_oasis7_net_tests true
assert_key_equals "$overlap_output" run_oasis7_net_libp2p_tests true

rename_delete_output="$(plan_for_paths crates/oasis7_node/src/old_bridge.rs crates/oasis7_net/src/new_bridge.rs)"
assert_key_equals "$rename_delete_output" scope targeted
assert_key_equals "$rename_delete_output" run_oasis7_node_tests true
assert_key_equals "$rename_delete_output" run_oasis7_net_tests true

config_output="$(plan_for_path scripts/ci-required-scope.v2.json)"
assert_key_equals "$config_output" scope full
assert_key_equals "$config_output" run_rust_baseline true
assert_key_equals "$config_output" needs_rust_toolchain true
assert_key_equals "$config_output" needs_node true

static_governance_output="$(plan_for_paths \
  scripts/check-script-executable-bits.sh \
  scripts/check-windows-paths.sh \
  scripts/doc-governance-check.sh \
  scripts/doc-governance-check.test.sh \
  scripts/lint-skills.sh \
  scripts/product-doc-governance-check.py \
  scripts/product-doc-governance-check.test.py \
  scripts/testing-manual-active-contract.test.sh \
  scripts/unified-world-code-terminology-scan.test.sh)"
assert_key_equals "$static_governance_output" scope targeted
assert_key_equals "$static_governance_output" selected_capabilities doc_checker_contracts
assert_key_equals "$static_governance_output" run_doc_checker_contracts true
assert_key_equals "$static_governance_output" run_rust_baseline false
assert_key_equals "$static_governance_output" needs_rust_toolchain false
assert_key_equals "$static_governance_output" needs_node false
for static_governance_path in \
  scripts/check-script-executable-bits.sh \
  scripts/check-windows-paths.sh \
  scripts/doc-governance-check.sh \
  scripts/doc-governance-check.test.sh \
  scripts/lint-skills.sh \
  scripts/product-doc-governance-check.py \
  scripts/product-doc-governance-check.test.py \
  scripts/testing-manual-active-contract.test.sh \
  scripts/unified-world-code-terminology-scan.test.sh; do
  assert_reason_contains "$static_governance_output" "governance_script:$static_governance_path"
done
assert_reason_absent "$static_governance_output" "unclassified_or_unresolvable:"

rust_gate_helper_output="$(plan_for_paths \
  scripts/cargo-dev.sh \
  scripts/cargo-dev-lib.sh \
  scripts/cargo-dev-lib.test.sh \
  scripts/cargo-dev-windows-toolchain.test.sh \
  scripts/check-standalone-tool-lockfiles.sh \
  scripts/check-standalone-tool-lockfiles.test.sh \
  scripts/check-rust-file-size.sh \
  scripts/check-rust-file-size.test.sh \
  scripts/check-rustsec-ignore-baseline.sh \
  scripts/ensure-cargo-deny.sh)"
assert_key_equals "$rust_gate_helper_output" scope full
assert_key_equals "$rust_gate_helper_output" run_rust_baseline true
assert_key_equals "$rust_gate_helper_output" needs_rust_toolchain true
for rust_gate_helper_path in \
  scripts/check-rust-file-size.sh \
  scripts/check-rust-file-size.test.sh \
  scripts/check-rustsec-ignore-baseline.sh \
  scripts/ensure-cargo-deny.sh; do
  assert_reason_contains "$rust_gate_helper_output" "shared_required_gate:$rust_gate_helper_path"
done
for cargo_tooling_path in \
  scripts/cargo-dev.sh \
  scripts/cargo-dev-lib.sh \
  scripts/cargo-dev-lib.test.sh \
  scripts/cargo-dev-windows-toolchain.test.sh \
  scripts/check-standalone-tool-lockfiles.test.sh; do
  assert_reason_contains "$rust_gate_helper_output" \
    "cargo_tooling_contracts:$cargo_tooling_path"
done
assert_reason_contains "$rust_gate_helper_output" \
  "shared_required_gate:scripts/check-standalone-tool-lockfiles.sh"
assert_reason_absent "$rust_gate_helper_output" "unclassified_or_unresolvable:"

launcher_dependency_output="$(plan_for_paths \
  scripts/check-launcher-p2p-dependency-surface.sh \
  scripts/check-launcher-p2p-dependency-surface.test.sh)"
assert_key_equals "$launcher_dependency_output" scope full
assert_key_equals "$launcher_dependency_output" run_rust_baseline true
assert_key_equals "$launcher_dependency_output" needs_rust_toolchain true
assert_reason_contains "$launcher_dependency_output" \
  "shared_required_gate:scripts/check-launcher-p2p-dependency-surface.sh"
assert_reason_contains "$launcher_dependency_output" \
  "shared_required_gate:scripts/check-launcher-p2p-dependency-surface.test.sh"
assert_reason_absent "$launcher_dependency_output" "unclassified_or_unresolvable:"

release_contract_output="$(plan_for_path scripts/release-packages-trunk-cache-contract.test.sh)"
assert_key_equals "$release_contract_output" scope minimal
assert_key_equals "$release_contract_output" selected_capabilities required_gate_baseline
assert_key_equals "$release_contract_output" run_rust_baseline false
assert_key_equals "$release_contract_output" needs_rust_toolchain false
assert_reason_contains "$release_contract_output" \
  "governance_script:scripts/release-packages-trunk-cache-contract.test.sh"
assert_reason_absent "$release_contract_output" "unclassified_or_unresolvable:"

bundle_portability_output="$(plan_for_paths \
  scripts/build-game-launcher-bundle.sh \
  scripts/build-game-launcher-bundle-macos-bash3.test.sh)"
assert_key_equals "$bundle_portability_output" scope targeted
assert_key_equals "$bundle_portability_output" run_operational_contracts true
assert_key_equals "$bundle_portability_output" run_rust_baseline true
assert_key_equals "$bundle_portability_output" needs_rust_toolchain true
assert_key_equals "$bundle_portability_output" needs_node false
assert_key_equals "$bundle_portability_output" selected_capabilities operational_contracts
assert_reason_contains "$bundle_portability_output" \
  "operational_contracts:scripts/build-game-launcher-bundle.sh"
assert_reason_contains "$bundle_portability_output" \
  "operational_contracts:scripts/build-game-launcher-bundle-macos-bash3.test.sh"

operational_contract_output="$(plan_for_path scripts/p2p-public-testnet-package-rollout.test.sh)"
assert_key_equals "$operational_contract_output" scope targeted
assert_key_equals "$operational_contract_output" selected_capabilities operational_contracts
assert_key_equals "$operational_contract_output" run_operational_contracts true
assert_key_equals "$operational_contract_output" run_rust_baseline false
assert_key_equals "$operational_contract_output" needs_rust_toolchain false
assert_key_equals "$operational_contract_output" needs_system_deps false
assert_reason_contains "$operational_contract_output" \
  "operational_contracts:scripts/p2p-public-testnet-package-rollout.test.sh"
assert_reason_absent "$operational_contract_output" "unclassified_or_unresolvable:"

service_readback_output="$(plan_for_path scripts/p2p-public-testnet-service-readback.test.sh)"
assert_key_equals "$service_readback_output" scope targeted
assert_key_equals "$service_readback_output" selected_capabilities operational_contracts
assert_key_equals "$service_readback_output" run_operational_contracts true
assert_key_equals "$service_readback_output" run_rust_baseline false
assert_key_equals "$service_readback_output" needs_rust_toolchain false
assert_key_equals "$service_readback_output" needs_node false
assert_reason_contains "$service_readback_output" \
  "operational_contracts:scripts/p2p-public-testnet-service-readback.test.sh"
assert_reason_absent "$service_readback_output" "unclassified_or_unresolvable:"

# Keep the implementation sources paired with their fixture-backed operational
# contracts. Source-only changes must select the same non-Rust lane rather than
# widening to the full required gate through the unmatched-path fallback.
operational_source_paths=(
  scripts/p2p-public-testnet-package-rollout.py
  scripts/p2p-public-testnet-package-node-upgrade.sh
  scripts/p2p-public-testnet-bootstrap-fresh-validator-host.sh
  scripts/p2p-public-testnet-local-observer-sync.sh
  scripts/p2p-observer-checkpoint-closure-probe.py
  scripts/p2p-public-testnet-fleet-health.py
  scripts/p2p-verify-linux-package-bundle.py
  scripts/p2p-rebuild-linux-bundle-checksums.py
)
for operational_source_path in "${operational_source_paths[@]}"; do
  operational_source_output="$(plan_for_path "$operational_source_path")"
  assert_key_equals "$operational_source_output" scope targeted
  assert_key_equals "$operational_source_output" selected_capabilities operational_contracts
  assert_key_equals "$operational_source_output" run_operational_contracts true
  assert_key_equals "$operational_source_output" run_rust_baseline false
  assert_key_equals "$operational_source_output" needs_rust_toolchain false
  assert_key_equals "$operational_source_output" needs_system_deps false
  assert_reason_contains "$operational_source_output" \
    "operational_contracts:$operational_source_path"
  assert_reason_absent "$operational_source_output" "unclassified_or_unresolvable:"
done

# Native installer and split Viewer delivery helpers are pure packaging
# contracts. They must run their focused non-Rust fixtures without inheriting
# the Rust baseline or the viewer JS build capability.
packaging_contract_output="$(plan_for_paths \
  scripts/package-native-installer.sh \
  scripts/validate-release-platform-entrypoints.sh \
  scripts/package-viewer-web-delivery.sh \
  scripts/packaging-artifact-size-contract.test.sh \
  scripts/copy-viewer-web-dist.test.sh \
  scripts/native-packaging-contract.test.sh \
  scripts/package-workflow-cache-reuse-contract.test.sh \
  scripts/testnet-packages-macos-arm64-contract.test.sh)"
assert_key_equals "$packaging_contract_output" scope targeted
assert_key_equals "$packaging_contract_output" selected_capabilities packaging_contracts
assert_key_equals "$packaging_contract_output" run_packaging_contracts true
assert_key_equals "$packaging_contract_output" run_operational_contracts false
assert_key_equals "$packaging_contract_output" run_rust_baseline false
assert_key_equals "$packaging_contract_output" needs_rust_toolchain false
assert_key_equals "$packaging_contract_output" needs_node false
assert_key_equals "$packaging_contract_output" needs_system_deps false
for packaging_path in \
  scripts/package-native-installer.sh \
  scripts/validate-release-platform-entrypoints.sh \
  scripts/package-viewer-web-delivery.sh \
  scripts/packaging-artifact-size-contract.test.sh \
  scripts/copy-viewer-web-dist.test.sh \
  scripts/native-packaging-contract.test.sh \
  scripts/package-workflow-cache-reuse-contract.test.sh \
  scripts/testnet-packages-macos-arm64-contract.test.sh; do
  assert_reason_contains "$packaging_contract_output" \
    "packaging_contracts:$packaging_path"
done
assert_reason_absent "$packaging_contract_output" "unclassified_or_unresolvable:"

for package_workflow_path in \
  .github/workflows/testnet-packages.yml \
  .github/workflows/mainnet-packages.yml \
  .github/workflows/release-packages.yml; do
  package_workflow_output="$(plan_for_path "$package_workflow_path")"
  assert_key_equals "$package_workflow_output" scope full
  assert_key_equals "$package_workflow_output" run_operational_contracts true
  assert_key_equals "$package_workflow_output" run_rust_baseline true
  assert_key_equals "$package_workflow_output" needs_rust_toolchain true
  assert_reason_contains "$package_workflow_output" \
    "packaging_workflow:$package_workflow_path"
  assert_reason_absent "$package_workflow_output" "unclassified_or_unresolvable:"
done

# Release workflows and Rust-producing bundle boundaries remain full even
# though their packaging consumers have a focused non-Rust capability.
release_packaging_output="$(plan_for_paths \
  .github/workflows/release-packages.yml \
  scripts/build-game-launcher-bundle.sh)"
assert_key_equals "$release_packaging_output" scope full
assert_key_equals "$release_packaging_output" run_rust_baseline true
assert_key_equals "$release_packaging_output" needs_rust_toolchain true

governance_helper_output="$(plan_for_paths \
  scripts/prepare-task-pr.sh \
  scripts/pm/patch-equivalence-receipt.sh \
  scripts/pm/patch-equivalence-receipt.test.sh \
  scripts/pm/prepare-task-pr-review-risk.test.py \
  scripts/prepare-task-pr.test.sh \
  scripts/plan-rust-required-scope.test.sh)"
assert_key_equals "$governance_helper_output" scope targeted
assert_key_equals "$governance_helper_output" selected_capabilities workflow_governance
assert_key_equals "$governance_helper_output" run_workflow_governance_contracts true
assert_key_equals "$governance_helper_output" run_operational_contracts false
assert_key_equals "$governance_helper_output" run_rust_baseline false
assert_key_equals "$governance_helper_output" needs_rust_toolchain false
assert_key_equals "$governance_helper_output" needs_node false
assert_reason_contains "$governance_helper_output" "governance_script:scripts/prepare-task-pr.sh"
assert_reason_contains "$governance_helper_output" "governance_script:scripts/pm/patch-equivalence-receipt.sh"
assert_reason_contains "$governance_helper_output" "governance_script:scripts/pm/patch-equivalence-receipt.test.sh"
assert_reason_contains "$governance_helper_output" "governance_script:scripts/prepare-task-pr.test.sh"
assert_reason_contains "$governance_helper_output" "governance_script:scripts/plan-rust-required-scope.test.sh"
assert_reason_absent "$governance_helper_output" "unclassified_or_unresolvable:"

# Traceability checkers and their caller contract are required-gate governance
# fixtures.  Keep them on the minimal lane so adding these checks does not
# widen an otherwise documentation-only change through the unmatched fallback.
traceability_governance_output="$(plan_for_paths \
  scripts/system-design-traceability-check.py \
  scripts/system-design-traceability-check.test.py \
  scripts/product-doc-content-check.py \
  scripts/product-doc-content-check.test.py \
  scripts/product-doc-content-callers.test.sh)"
assert_key_equals "$traceability_governance_output" scope targeted
assert_key_equals "$traceability_governance_output" selected_capabilities doc_checker_contracts
assert_key_equals "$traceability_governance_output" run_doc_checker_contracts true
assert_key_equals "$traceability_governance_output" run_rust_baseline false
assert_key_equals "$traceability_governance_output" needs_rust_toolchain false
assert_key_equals "$traceability_governance_output" needs_node false
for traceability_governance_path in \
  scripts/system-design-traceability-check.py \
  scripts/system-design-traceability-check.test.py \
  scripts/product-doc-content-check.py \
  scripts/product-doc-content-check.test.py \
  scripts/product-doc-content-callers.test.sh; do
  assert_reason_contains "$traceability_governance_output" \
    "governance_script:$traceability_governance_path"
done
assert_reason_absent "$traceability_governance_output" "unclassified_or_unresolvable:"

# Document inventory helpers form a document-only static-validation closure:
# the two inventory checkers inspect JSON/Markdown snapshots (the corpus
# checker delegates only to the evidence checker), and the report uses Python
# stdlib to count Markdown files. Keep this inventory separate from the
# Markdown parser and checker implementation dependency contract.
document_governance_paths=(
  scripts/doc-evidence-inventory-check.py
  scripts/doc-evidence-inventory-check.test.py
  scripts/document-corpus-inventory-check.py
  scripts/document-corpus-inventory-check.test.py
  scripts/doc-inventory-report.sh
)
document_governance_output="$(plan_for_paths "${document_governance_paths[@]}")"
assert_key_equals "$document_governance_output" scope minimal
assert_key_equals "$document_governance_output" selected_capabilities required_gate_baseline
assert_key_equals "$document_governance_output" run_rust_baseline false
assert_key_equals "$document_governance_output" needs_rust_toolchain false
assert_key_equals "$document_governance_output" needs_node false
assert_key_equals "$document_governance_output" needs_system_deps false
for document_governance_path in "${document_governance_paths[@]}"; do
  assert_reason_contains "$document_governance_output" \
    "governance_script:$document_governance_path"
done
assert_reason_absent "$document_governance_output" "unclassified_or_unresolvable:"

# The legacy hook is explicitly a silent compatibility no-op.  Its focused
# fixture supplies forbidden-command shims and proves that the hook emits no
# output or validation call; keep only the hook and its fixture test minimal.
precommit_noop_output="$(plan_for_paths \
  scripts/pre-commit.sh \
  scripts/pre-commit.test.sh)"
assert_key_equals "$precommit_noop_output" scope minimal
assert_key_equals "$precommit_noop_output" selected_capabilities required_gate_baseline
assert_key_equals "$precommit_noop_output" run_rust_baseline false
assert_key_equals "$precommit_noop_output" needs_rust_toolchain false
assert_key_equals "$precommit_noop_output" needs_node false
assert_key_equals "$precommit_noop_output" needs_system_deps false
assert_reason_contains "$precommit_noop_output" \
  "governance_script:scripts/pre-commit.sh"
assert_reason_contains "$precommit_noop_output" \
  "governance_script:scripts/pre-commit.test.sh"
assert_reason_absent "$precommit_noop_output" "unclassified_or_unresolvable:"

# Explicit repair is a different contract: it runs Cargo formatting, mutates
# the index with git add -u, and invokes the commit-tier ci-tests entrypoint.
# Keep it on the shared full required-gate rule so an isolated change cannot
# under-plan those Rust/staging/commit effects.
precommit_repair_output="$(plan_for_path scripts/fix-precommit.sh)"
assert_key_equals "$precommit_repair_output" scope full
assert_key_equals "$precommit_repair_output" run_rust_baseline true
assert_key_equals "$precommit_repair_output" needs_rust_toolchain true
assert_key_equals "$precommit_repair_output" needs_node true
assert_key_equals "$precommit_repair_output" needs_system_deps true
assert_reason_contains "$precommit_repair_output" \
  "shared_required_gate:scripts/fix-precommit.sh"
assert_reason_absent "$precommit_repair_output" "unclassified_or_unresolvable:"

# These seven shell fixtures validate required-gate workflow wiring.  Keep
# every path exact so a missing mapping cannot silently widen this plan to the
# full Rust baseline/toolchain.
workflow_contract_paths=(
  scripts/ci-tests-argument-contract.test.sh
  scripts/ci-tests-full-superset-contract.test.sh
  scripts/ci-tests-pixel-world-required-contract.test.sh
  scripts/rust-required-gate-apt-contract.test.sh
  scripts/rust-required-gate-compile-command-contract.test.sh
  scripts/rust-full-tier-trunk-prerequisite-contract.test.sh
  scripts/ci-required-scope-audit-contract.test.sh
)
for workflow_contract_path in "${workflow_contract_paths[@]}"; do
  workflow_contract_output="$(plan_for_path "$workflow_contract_path")"
  assert_key_equals "$workflow_contract_output" scope targeted
  assert_key_equals "$workflow_contract_output" selected_capabilities workflow_governance
  assert_key_equals "$workflow_contract_output" run_workflow_governance_contracts true
  assert_key_equals "$workflow_contract_output" run_operational_contracts false
  assert_key_equals "$workflow_contract_output" run_rust_baseline false
  assert_key_equals "$workflow_contract_output" needs_rust_toolchain false
  assert_reason_contains "$workflow_contract_output" \
    "workflow_governance:$workflow_contract_path"
  assert_reason_absent "$workflow_contract_output" "unclassified_or_unresolvable:"
done

# Pure PM implementation and test helpers are workflow-governance checks, not
# Rust workspace changes.  Keep representative Python and test/sh paths in a
# single union so one unclassified path cannot silently widen the whole plan.
pm_workflow_output="$(plan_for_paths \
  scripts/pm/terminal-task-audit.py \
  scripts/pm/terminal-task-audit-project-semantics.test.py \
  scripts/pm/github-project-workflow.py \
  scripts/pm/github-project-workflow.test.sh)"
assert_key_equals "$pm_workflow_output" scope targeted
assert_key_equals "$pm_workflow_output" selected_capabilities workflow_governance
assert_key_equals "$pm_workflow_output" run_rust_baseline false
assert_key_equals "$pm_workflow_output" needs_rust_toolchain false
assert_reason_contains "$pm_workflow_output" \
  "workflow_governance:scripts/pm/terminal-task-audit.py"
assert_reason_contains "$pm_workflow_output" \
  "workflow_governance:scripts/pm/terminal-task-audit-project-semantics.test.py"
assert_reason_contains "$pm_workflow_output" \
  "workflow_governance:scripts/pm/github-project-workflow.py"
assert_reason_contains "$pm_workflow_output" \
  "workflow_governance:scripts/pm/github-project-workflow.test.sh"
assert_reason_absent "$pm_workflow_output" "unclassified_or_unresolvable:"

receipt_governance_output="$(plan_for_path scripts/pm/ci-ready-receipt.py)"
assert_key_equals "$receipt_governance_output" scope targeted
assert_key_equals "$receipt_governance_output" selected_capabilities workflow_governance
assert_key_equals "$receipt_governance_output" run_workflow_governance_contracts true
assert_key_equals "$receipt_governance_output" run_operational_contracts false
assert_key_equals "$receipt_governance_output" run_rust_baseline false
assert_key_equals "$receipt_governance_output" needs_rust_toolchain false
assert_reason_contains "$receipt_governance_output" \
  "workflow_governance:scripts/pm/ci-ready-receipt.py"
assert_reason_absent "$receipt_governance_output" "unclassified_or_unresolvable:"

# An explicit gameplay/high-risk verification rule must union with the broad
# PM workflow rule.  The specific rule retains its Rust capabilities while the
# generic rule adds governance coverage.
pm_gameplay_union_output="$(plan_for_path scripts/pm/verify-gameplay-high-risk-hardening.sh)"
assert_key_equals "$pm_gameplay_union_output" scope targeted
assert_key_equals "$pm_gameplay_union_output" selected_capabilities \
  'oasis7_required;viewer_js_required;workflow_governance'
assert_key_equals "$pm_gameplay_union_output" run_oasis7_required_tests true
assert_key_equals "$pm_gameplay_union_output" run_viewer_contract_tests true
assert_key_equals "$pm_gameplay_union_output" run_operational_contracts false
assert_key_equals "$pm_gameplay_union_output" run_workflow_governance_contracts true
assert_key_equals "$pm_gameplay_union_output" run_rust_baseline true
assert_key_equals "$pm_gameplay_union_output" needs_rust_toolchain true
assert_reason_contains "$pm_gameplay_union_output" \
  "workflow_governance:scripts/pm/verify-gameplay-high-risk-hardening.sh"
assert_reason_contains "$pm_gameplay_union_output" \
  "viewer_gameplay_verification:scripts/pm/verify-gameplay-high-risk-hardening.sh"
assert_reason_absent "$pm_gameplay_union_output" "unclassified_or_unresolvable:"

# Workflow/PM changes should select the workflow and operational contract
# checks without inheriting unrelated Rust crates.  This fixture is RED until
# the planner has an explicit non-Rust workflow-governance capability.
workflow_governance_output="$(plan_for_paths \
  scripts/new-task-worktree.sh \
  scripts/pm/finalize-task.sh \
  scripts/pm/finalize-task.test.sh \
  scripts/pm/review-closeout.sh \
  scripts/pm/review-closeout-facade.test.sh \
  scripts/pm/new-task-worktree-acceptance-pre-mutation.test.sh \
  scripts/pm/new-task-worktree-partial-bootstrap.test.sh \
  scripts/pm/workflow-behavior-eval.sh \
  scripts/launcher-help-contract.sh \
  scripts/launcher-help-contract.test.sh \
  scripts/prepare-task-pr.sh \
  scripts/pm/prepare-task-pr-review-risk.test.py \
  .agents/skills/requesting-repo-owned-review/SKILL.md \
  doc/engineering/workflow/source-of-truth.md)"
assert_key_equals "$workflow_governance_output" scope targeted
assert_key_equals "$workflow_governance_output" run_workflow_governance_contracts true
assert_key_equals "$workflow_governance_output" run_operational_contracts false
assert_key_equals "$workflow_governance_output" run_rust_baseline false
assert_key_equals "$workflow_governance_output" needs_rust_toolchain false
assert_key_equals "$workflow_governance_output" run_oasis7_required_tests false
assert_key_equals "$workflow_governance_output" run_launcher_web_build false
assert_key_equals "$workflow_governance_output" selected_capabilities workflow_governance
assert_reason_contains "$workflow_governance_output" \
  "workflow_governance:scripts/new-task-worktree.sh"
assert_reason_absent "$workflow_governance_output" "unclassified_or_unresolvable:"

planner_semantic_output="$(plan_for_path scripts/plan-rust-required-scope.py)"
assert_key_equals "$planner_semantic_output" scope full
assert_key_equals "$planner_semantic_output" run_rust_baseline true
assert_reason_contains "$planner_semantic_output" \
  "shared_required_gate:scripts/plan-rust-required-scope.py"

peer_registry_contract_failures=0
for registry_path in \
  scripts/p2p-public-testnet-peer-registry.py \
  scripts/p2p-public-testnet-peer-registry.test.py; do
  registry_scope_output="$(plan_for_path "$registry_path")"
  if ! (
    assert_key_equals "$registry_scope_output" scope targeted
    assert_key_equals "$registry_scope_output" run_operational_contracts true
    assert_key_equals "$registry_scope_output" run_rust_baseline false
    assert_key_equals "$registry_scope_output" selected_capabilities operational_contracts
    assert_reason_contains "$registry_scope_output" "operational_contracts:$registry_path"
    assert_reason_absent "$registry_scope_output" "unclassified_or_unresolvable:"
  ); then
    echo "managed peer registry scope contract failed: $registry_path" >&2
    peer_registry_contract_failures=$((peer_registry_contract_failures + 1))
  fi
done
if ! sed -n '/^run_operational_identity_contract_tests() {$/,/^}$/p' \
  "$ROOT_DIR/scripts/ci-tests.sh" \
  | grep -Fxq \
    '  run python3 ./scripts/p2p-public-testnet-peer-registry.test.py'; then
  echo "expected required gate to execute the managed peer registry admission suite" >&2
  peer_registry_contract_failures=$((peer_registry_contract_failures + 1))
fi
if [[ "$peer_registry_contract_failures" -ne 0 ]]; then
  echo "managed peer registry contract failures: $peer_registry_contract_failures" >&2
  exit 1
fi

clean_room_scope_output="$(plan_for_paths \
  scripts/p2p-public-testnet-full-network-clean-room.py \
  scripts/p2p-public-testnet-full-network-clean-room.test.py \
  scripts/p2p-public-testnet-full-network-clean-room-adapter.py \
  scripts/p2p-public-testnet-full-network-clean-room-adapter.test.py \
  scripts/p2p-public-testnet-identity-v2-evidence-aggregate.py \
  scripts/p2p-public-testnet-identity-v2-evidence-aggregate.test.py \
  scripts/fixtures/oasis7-governance-root.v1.json)"
assert_key_equals "$clean_room_scope_output" scope targeted
assert_key_equals "$clean_room_scope_output" run_operational_contracts true
assert_key_equals "$clean_room_scope_output" run_rust_baseline false
assert_key_equals "$clean_room_scope_output" selected_capabilities operational_contracts
assert_reason_contains "$clean_room_scope_output" \
  "operational_contracts:scripts/p2p-public-testnet-full-network-clean-room.py"
assert_reason_contains "$clean_room_scope_output" \
  "operational_contracts:scripts/p2p-public-testnet-identity-v2-evidence-aggregate.py"
assert_reason_contains "$clean_room_scope_output" \
  "operational_contracts:scripts/p2p-public-testnet-identity-v2-evidence-aggregate.test.py"
assert_reason_contains "$clean_room_scope_output" \
  "operational_contracts:scripts/fixtures/oasis7-governance-root.v1.json"
assert_reason_absent "$clean_room_scope_output" "unclassified_or_unresolvable:"

if ! sed -n '/^run_operational_identity_contract_tests() {$/,/^}$/p' \
  "$ROOT_DIR/scripts/ci-tests.sh" \
  | grep -Fq \
    'run python3 ./scripts/p2p-public-testnet-identity-v2-evidence-aggregate.test.py'; then
  echo "expected required gate to execute the aggregate evidence contract suite" >&2
  exit 1
fi

compile_metrics_output="$(plan_for_paths \
  scripts/ci-compile-metrics.sh \
  scripts/ci-compile-metrics-gate.py \
  scripts/ci-compile-metrics-contract.test.sh)"
assert_key_equals "$compile_metrics_output" scope targeted
assert_key_equals "$compile_metrics_output" run_compile_metrics_contract_tests true
assert_key_equals "$compile_metrics_output" run_rust_baseline false
assert_key_equals "$compile_metrics_output" needs_rust_toolchain false
assert_key_equals "$compile_metrics_output" needs_node false
assert_key_equals "$compile_metrics_output" needs_system_deps false
assert_key_equals "$compile_metrics_output" selected_capabilities compile_metrics
assert_reason_contains "$compile_metrics_output" "compile_metrics:scripts/ci-compile-metrics.sh"
assert_reason_absent "$compile_metrics_output" "unclassified_or_unresolvable:"

compile_metrics_workflow_output="$(plan_for_path .github/workflows/compile-metrics.yml)"
assert_key_equals "$compile_metrics_workflow_output" scope full
assert_key_equals "$compile_metrics_workflow_output" run_compile_metrics_contract_tests true
assert_key_equals "$compile_metrics_workflow_output" run_rust_baseline true
assert_reason_contains "$compile_metrics_workflow_output" "compile_metrics_workflow:.github/workflows/compile-metrics.yml"

viewer_web_wrapper_output="$(plan_for_paths \
  scripts/build-viewer-software-safe.sh \
  scripts/viewer-dependency-preflight.sh \
  scripts/viewer-dependency-preflight.test.sh \
  scripts/viewer-pixel-world-fragment-visual-smoke.sh \
  scripts/viewer-prompt-control-regression.sh \
  scripts/viewer-prompt-control-regression.test.sh \
  scripts/agent-browser-lib.sh \
  scripts/agent-browser-viewer-dist-freshness-test.sh \
  scripts/bundle-freshness-lib.sh \
  scripts/bundle-freshness-lib.test.sh \
  scripts/copy-viewer-web-dist.sh \
  scripts/viewer-web-dist-contract.sh)"
assert_key_equals "$viewer_web_wrapper_output" scope targeted
assert_key_equals "$viewer_web_wrapper_output" run_viewer_contract_tests true
assert_key_equals "$viewer_web_wrapper_output" run_viewer_wasm_check true
assert_key_equals "$viewer_web_wrapper_output" run_launcher_web_build false
assert_key_equals "$viewer_web_wrapper_output" run_oasis7_required_tests false
assert_key_equals "$viewer_web_wrapper_output" needs_rust_toolchain true
assert_key_equals "$viewer_web_wrapper_output" needs_node true
assert_key_equals "$viewer_web_wrapper_output" needs_system_deps true
assert_key_equals "$viewer_web_wrapper_output" needs_wasm_target true
assert_key_equals "$viewer_web_wrapper_output" needs_trunk false
assert_reason_contains "$viewer_web_wrapper_output" "viewer_web_wrapper:scripts/build-viewer-software-safe.sh"
assert_reason_contains "$viewer_web_wrapper_output" "viewer_web_wrapper:scripts/viewer-dependency-preflight.sh"
assert_reason_contains "$viewer_web_wrapper_output" "viewer_web_wrapper:scripts/viewer-dependency-preflight.test.sh"
assert_reason_contains "$viewer_web_wrapper_output" "viewer_web_wrapper:scripts/viewer-pixel-world-fragment-visual-smoke.sh"
assert_reason_contains "$viewer_web_wrapper_output" "viewer_web_wrapper:scripts/viewer-prompt-control-regression.sh"
assert_reason_contains "$viewer_web_wrapper_output" "viewer_web_wrapper:scripts/viewer-prompt-control-regression.test.sh"
assert_reason_contains "$viewer_web_wrapper_output" "viewer_web_wrapper:scripts/agent-browser-lib.sh"
assert_reason_contains "$viewer_web_wrapper_output" "viewer_web_wrapper:scripts/agent-browser-viewer-dist-freshness-test.sh"
assert_reason_contains "$viewer_web_wrapper_output" "viewer_web_wrapper:scripts/bundle-freshness-lib.sh"
assert_reason_contains "$viewer_web_wrapper_output" "viewer_web_wrapper:scripts/bundle-freshness-lib.test.sh"
assert_reason_contains "$viewer_web_wrapper_output" "viewer_web_wrapper:scripts/copy-viewer-web-dist.sh"
assert_reason_contains "$viewer_web_wrapper_output" "viewer_web_wrapper:scripts/viewer-web-dist-contract.sh"
assert_reason_absent "$viewer_web_wrapper_output" "unclassified_or_unresolvable:"

for viewer_shared_helper in \
  scripts/bundle-freshness-lib.sh \
  scripts/copy-viewer-web-dist.sh \
  scripts/viewer-web-dist-contract.sh; do
  viewer_shared_helper_output="$(plan_for_path "$viewer_shared_helper")"
  assert_key_equals "$viewer_shared_helper_output" scope targeted
  assert_key_equals "$viewer_shared_helper_output" \
    selected_capabilities 'operational_contracts;viewer_js_required'
  assert_key_equals "$viewer_shared_helper_output" run_viewer_contract_tests true
  assert_key_equals "$viewer_shared_helper_output" run_operational_contracts true
  assert_key_equals "$viewer_shared_helper_output" run_rust_baseline true
  assert_reason_contains "$viewer_shared_helper_output" \
    "operational_contracts:$viewer_shared_helper"
  assert_reason_contains "$viewer_shared_helper_output" \
    "viewer_web_wrapper:$viewer_shared_helper"
  assert_reason_absent "$viewer_shared_helper_output" "unclassified_or_unresolvable:"
done

viewer_launcher_wrapper_output="$(plan_for_paths \
  scripts/run-launcher-stack.sh \
  scripts/run-producer-playtest.sh \
  scripts/worktree-harness.sh \
  scripts/worktree-harness-lib.sh \
  scripts/worktree-harness-contract.test.sh \
  scripts/worktree-harness-lifecycle.test.sh \
  scripts/worktree-harness-lifecycle-races.test.sh \
  scripts/run-launcher-stack-local-mock-lane.test.sh)"
assert_key_equals "$viewer_launcher_wrapper_output" scope targeted
assert_key_equals "$viewer_launcher_wrapper_output" run_viewer_contract_tests true
assert_key_equals "$viewer_launcher_wrapper_output" run_viewer_wasm_check true
assert_key_equals "$viewer_launcher_wrapper_output" run_launcher_web_build true
assert_key_equals "$viewer_launcher_wrapper_output" needs_trunk true
assert_reason_contains "$viewer_launcher_wrapper_output" "viewer_launcher_wrapper:scripts/run-launcher-stack.sh"
assert_reason_contains "$viewer_launcher_wrapper_output" "viewer_launcher_wrapper:scripts/run-producer-playtest.sh"
assert_reason_contains "$viewer_launcher_wrapper_output" "viewer_launcher_wrapper:scripts/worktree-harness.sh"
assert_reason_contains "$viewer_launcher_wrapper_output" "viewer_launcher_wrapper:scripts/worktree-harness-lib.sh"
assert_reason_contains "$viewer_launcher_wrapper_output" "viewer_launcher_wrapper:scripts/worktree-harness-contract.test.sh"
assert_reason_contains "$viewer_launcher_wrapper_output" "viewer_launcher_wrapper:scripts/worktree-harness-lifecycle.test.sh"
assert_reason_contains "$viewer_launcher_wrapper_output" "viewer_launcher_wrapper:scripts/worktree-harness-lifecycle-races.test.sh"
assert_reason_contains "$viewer_launcher_wrapper_output" "viewer_launcher_wrapper:scripts/run-launcher-stack-local-mock-lane.test.sh"
assert_reason_absent "$viewer_launcher_wrapper_output" "unclassified_or_unresolvable:"

# The harness library is shared by launcher code and workflow-governance
# entrypoints (new-task-worktree, PR preparation, and review closeout).  Its
# isolated plan must retain both classifications so the governance contracts
# run without dropping the launcher checks.
worktree_harness_lib_output="$(plan_for_path scripts/worktree-harness-lib.sh)"
assert_key_equals "$worktree_harness_lib_output" scope targeted
assert_key_equals "$worktree_harness_lib_output" selected_capabilities \
  'launcher_web;viewer_js_required;workflow_governance'
assert_key_equals "$worktree_harness_lib_output" run_workflow_governance_contracts true
assert_key_equals "$worktree_harness_lib_output" run_operational_contracts false
assert_reason_contains "$worktree_harness_lib_output" \
  "viewer_launcher_wrapper:scripts/worktree-harness-lib.sh"
assert_reason_contains "$worktree_harness_lib_output" \
  "workflow_governance:scripts/worktree-harness-lib.sh"
assert_reason_absent "$worktree_harness_lib_output" "unclassified_or_unresolvable:"

# The local launcher mock lane remains launcher-only; it must not inherit the
# governance overlap added for the shared harness library.
launcher_mock_lane_output="$(plan_for_path \
  scripts/run-launcher-stack-local-mock-lane.test.sh)"
assert_key_equals "$launcher_mock_lane_output" scope targeted
assert_key_equals "$launcher_mock_lane_output" \
  selected_capabilities 'launcher_web;viewer_js_required'
assert_key_equals "$launcher_mock_lane_output" run_operational_contracts false
assert_reason_contains "$launcher_mock_lane_output" \
  "viewer_launcher_wrapper:scripts/run-launcher-stack-local-mock-lane.test.sh"
assert_reason_absent "$launcher_mock_lane_output" \
  "workflow_governance:scripts/run-launcher-stack-local-mock-lane.test.sh"
assert_reason_absent "$launcher_mock_lane_output" "unclassified_or_unresolvable:"

# The standalone viewer server is retained as a compatibility/debug entrypoint
# outside the current launcher caller graph.  Keep its ambiguous/deprecated
# path on the fail-closed full fallback until its ownership and active caller
# contract are made unambiguous.
legacy_viewer_web_output="$(plan_for_path scripts/run-viewer-web.sh)"
assert_key_equals "$legacy_viewer_web_output" scope full
assert_key_equals "$legacy_viewer_web_output" run_rust_baseline true
assert_reason_contains "$legacy_viewer_web_output" \
  "unclassified_or_unresolvable:scripts/run-viewer-web.sh"

# The baseline LLM fixture is intentionally full: it runs several test_tier_full
# Rust cases and is only invoked by the full/full-support tiers.  Keep it
# unmatched so the fail-closed full fallback remains explicit until a dedicated
# capability can prove an equivalent focused lane.
llm_baseline_fixture_output="$(plan_for_path scripts/llm-baseline-fixture-smoke.sh)"
assert_key_equals "$llm_baseline_fixture_output" scope full
assert_key_equals "$llm_baseline_fixture_output" run_rust_baseline true
assert_key_equals "$llm_baseline_fixture_output" needs_rust_toolchain true
assert_reason_contains "$llm_baseline_fixture_output" \
  "unclassified_or_unresolvable:scripts/llm-baseline-fixture-smoke.sh"

viewer_gameplay_hardening_output="$(plan_for_path scripts/pm/verify-gameplay-high-risk-hardening.sh)"
assert_key_equals "$viewer_gameplay_hardening_output" scope targeted
assert_key_equals "$viewer_gameplay_hardening_output" run_oasis7_required_tests true
assert_key_equals "$viewer_gameplay_hardening_output" run_viewer_contract_tests true
assert_key_equals "$viewer_gameplay_hardening_output" run_viewer_wasm_check true
assert_key_equals "$viewer_gameplay_hardening_output" run_pixel_world_bridge_lib_tests false
assert_reason_contains "$viewer_gameplay_hardening_output" "viewer_gameplay_verification:scripts/pm/verify-gameplay-high-risk-hardening.sh"
assert_reason_absent "$viewer_gameplay_hardening_output" "unclassified_or_unresolvable:"

viewer_attraction_verification_output="$(plan_for_path scripts/verify-gameplay-attraction-automation.sh)"
assert_key_equals "$viewer_attraction_verification_output" scope targeted
assert_key_equals "$viewer_attraction_verification_output" run_oasis7_required_tests true
assert_key_equals "$viewer_attraction_verification_output" run_viewer_contract_tests true
assert_key_equals "$viewer_attraction_verification_output" run_viewer_wasm_check true
assert_key_equals "$viewer_attraction_verification_output" run_pixel_world_bridge_lib_tests true
assert_key_equals "$viewer_attraction_verification_output" run_pixel_world_bridge_wasm_check true
assert_reason_contains "$viewer_attraction_verification_output" "viewer_attraction_verification:scripts/verify-gameplay-attraction-automation.sh"
assert_reason_absent "$viewer_attraction_verification_output" "unclassified_or_unresolvable:"

unknown_output="$(plan_for_path unknown-unclassified-input.txt)"
assert_key_equals "$unknown_output" scope full
assert_reason_contains "$unknown_output" "unclassified_or_unresolvable:unknown-unclassified-input.txt"

codex_agent_config_output="$(plan_for_paths \
  scripts/pm/validate-codex-agent-config.py \
  scripts/pm/validate-codex-agent-config.test.sh \
  scripts/pm/verify-codex-subagent-role-fit.sh \
  scripts/pm/codex-role-fit-task-binding.test.sh)"
assert_key_equals "$codex_agent_config_output" scope targeted
assert_key_equals "$codex_agent_config_output" run_codex_agent_config_validation true
assert_key_equals "$codex_agent_config_output" run_oasis7_required_tests false
assert_key_equals "$codex_agent_config_output" run_consensus_tests false
assert_key_equals "$codex_agent_config_output" run_viewer_contract_tests false
assert_key_equals "$codex_agent_config_output" run_launcher_web_build false
assert_key_equals "$codex_agent_config_output" run_rust_baseline false
assert_key_equals "$codex_agent_config_output" needs_rust_toolchain false
assert_key_equals "$codex_agent_config_output" needs_node false
assert_reason_contains "$codex_agent_config_output" "codex_agent_config_validation:scripts/pm/validate-codex-agent-config.py"
assert_reason_contains "$codex_agent_config_output" "codex_agent_config_validation:scripts/pm/validate-codex-agent-config.test.sh"
assert_reason_contains "$codex_agent_config_output" "codex_agent_config_validation:scripts/pm/verify-codex-subagent-role-fit.sh"
assert_reason_contains "$codex_agent_config_output" "codex_agent_config_validation:scripts/pm/codex-role-fit-task-binding.test.sh"

codex_config_output="$(plan_for_paths \
  .codex/config.toml \
  .codex/agents/repository_health_engineer.toml \
  scripts/ci-tests-codex-agent-config-required-contract.test.sh)"
assert_key_equals "$codex_config_output" scope targeted
assert_key_equals "$codex_config_output" selected_capabilities codex_agent_config_validation
assert_key_equals "$codex_config_output" run_codex_agent_config_validation true
assert_key_equals "$codex_config_output" run_operational_contracts false
assert_key_equals "$codex_config_output" run_rust_baseline false
assert_key_equals "$codex_config_output" needs_rust_toolchain false
assert_key_equals "$codex_config_output" needs_node false
assert_reason_contains "$codex_config_output" "codex_agent_config:.codex/config.toml"
assert_reason_contains "$codex_config_output" \
  "codex_agent_config:.codex/agents/repository_health_engineer.toml"
assert_reason_contains "$codex_config_output" \
  "codex_agent_config_validation:scripts/ci-tests-codex-agent-config-required-contract.test.sh"
assert_reason_absent "$codex_config_output" "unclassified_or_unresolvable:"

canonical_role_cards=(
  agent_engineer
  blockchain_ops_engineer
  game_visual_interaction_designer
  gameplay_designer
  liveops_community
  producer_system_designer
  qa_engineer
  repository_health_engineer
  runtime_engineer
  viewer_engineer
  wasm_platform_engineer
)
for role in "${canonical_role_cards[@]}"; do
  role_card_path=".agents/roles/${role}.md"
  codex_role_card_output="$(plan_for_path "$role_card_path")"
  assert_key_equals "$codex_role_card_output" scope targeted
  assert_key_equals "$codex_role_card_output" selected_capabilities codex_agent_config_validation
  assert_key_equals "$codex_role_card_output" run_codex_agent_config_validation true
  assert_key_equals "$codex_role_card_output" run_operational_contracts false
  assert_key_equals "$codex_role_card_output" run_rust_baseline false
  assert_key_equals "$codex_role_card_output" needs_rust_toolchain false
  assert_reason_contains "$codex_role_card_output" "codex_role_card:${role_card_path}"
  assert_reason_absent "$codex_role_card_output" "unclassified_or_unresolvable:"
done

role_template_output="$(plan_for_path .agents/roles/templates/subagent-slice-card.md)"
assert_key_equals "$role_template_output" scope minimal
assert_key_equals "$role_template_output" selected_capabilities required_gate_baseline
assert_key_equals "$role_template_output" run_codex_agent_config_validation false
assert_reason_contains "$role_template_output" \
  "governance_doc:.agents/roles/templates/subagent-slice-card.md"

review_skill_output="$(plan_for_path .agents/skills/requesting-repo-owned-review/SKILL.md)"
assert_key_equals "$review_skill_output" scope targeted
assert_key_equals "$review_skill_output" selected_capabilities workflow_governance
assert_key_equals "$review_skill_output" run_workflow_governance_contracts true
assert_key_equals "$review_skill_output" run_operational_contracts false
assert_key_equals "$review_skill_output" run_rust_baseline false
assert_reason_contains "$review_skill_output" \
  "workflow_skill_contract:.agents/skills/requesting-repo-owned-review/SKILL.md"

doc_checker_output="$(plan_for_paths \
  scripts/product-doc-governance-check.py \
  scripts/product-doc-governance-check.test.py \
  scripts/product-doc-content-check.py \
  scripts/product-doc-content-check.test.py \
  scripts/system-design-traceability-check.py \
  scripts/system-design-traceability-check.test.py \
  scripts/product-doc-content-callers.test.sh \
  scripts/doc-governance-check.test.sh \
  scripts/product_doc_markdown.py \
  scripts/doc-governance-requirements.txt)"
assert_key_equals "$doc_checker_output" scope targeted
assert_key_equals "$doc_checker_output" selected_capabilities doc_checker_contracts
assert_key_equals "$doc_checker_output" run_doc_checker_contracts true
assert_key_equals "$doc_checker_output" run_cargo_tooling_contracts false
assert_key_equals "$doc_checker_output" run_rust_baseline false
assert_key_equals "$doc_checker_output" needs_rust_toolchain false
assert_key_equals "$doc_checker_output" needs_python true
assert_key_equals "$doc_checker_output" needs_markdown true

cargo_tooling_output="$(plan_for_paths \
  scripts/cargo-dev.sh \
  scripts/cargo-dev-lib.sh \
  scripts/cargo-dev-lib.test.sh \
  scripts/cargo-dev-windows-toolchain.test.sh \
  scripts/cargo-dev-worktree-isolation.test.sh \
  scripts/pm/new-task-worktree-cargo-cache-migration.test.sh)"
assert_key_equals "$cargo_tooling_output" scope targeted
assert_key_equals "$cargo_tooling_output" selected_capabilities \
  'cargo_tooling_contracts;workflow_governance'
assert_key_equals "$cargo_tooling_output" run_cargo_tooling_contracts true
assert_key_equals "$cargo_tooling_output" run_workflow_governance_contracts true
assert_key_equals "$cargo_tooling_output" run_doc_checker_contracts false
assert_key_equals "$cargo_tooling_output" run_rust_baseline true
assert_key_equals "$cargo_tooling_output" needs_rust_toolchain true
assert_key_equals "$cargo_tooling_output" needs_node false
assert_key_equals "$cargo_tooling_output" needs_system_deps false
assert_key_equals "$cargo_tooling_output" needs_trunk false
assert_key_equals "$cargo_tooling_output" needs_wasm_target false
assert_key_equals "$cargo_tooling_output" needs_python true
assert_key_equals "$cargo_tooling_output" needs_markdown true

legacy_config="$(mktemp)"
python3 - "$ROOT_DIR/scripts/ci-required-scope.v2.json" "$legacy_config" <<'PY'
import json
import sys

source, destination = sys.argv[1:]
config = json.load(open(source, encoding="utf-8"))
config.pop("execution_contract", None)
config.pop("resource_requirements", None)
config.pop("baseline_resources", None)
config["capabilities"] = [
    item for item in config["capabilities"]
    if item not in {"doc_checker_contracts", "cargo_tooling_contracts"}
]
config["selector_ownership"] = [
    item for item in config["selector_ownership"]
    if item.get("planner_field") not in {
        "run_doc_checker_contracts", "run_cargo_tooling_contracts",
        "run_packaging_contracts", "run_workflow_governance_contracts",
    }
]
for item in config["selector_ownership"]:
    if item.get("planner_field") in {
        "run_packaging_contracts", "run_workflow_governance_contracts",
    }:
        item["planner_field"] = "run_operational_contracts"
config["rules"] = [
    rule for rule in config["rules"]
    if not set(rule.get("capabilities", [])) & {
        "doc_checker_contracts", "cargo_tooling_contracts",
    }
]
json.dump(config, open(destination, "w", encoding="utf-8"))
PY
legacy_packaging_output="$("$ROOT_DIR/scripts/plan-rust-required-scope.sh" \
  --event-name pull_request --config "$legacy_config" \
  --changed-path scripts/testnet-packages-macos-arm64-contract.test.sh)"
assert_key_equals "$legacy_packaging_output" execution_contract ""
assert_key_equals "$legacy_packaging_output" run_operational_contracts true
if [[ -n "$(value_for_key "$legacy_packaging_output" run_packaging_contracts)" ]]; then
  echo "legacy planner must not synthesize versioned selector fields" >&2
  exit 1
fi
rm -f "$legacy_config"

invalid_config="$(mktemp)"
trap 'rm -f "$invalid_config"; rm -rf "${missing_selector_source_dir:-}"' EXIT
printf '{not json}\n' >"$invalid_config"
if "$ROOT_DIR/scripts/plan-rust-required-scope.sh" --event-name pull_request --config "$invalid_config" --changed-path README.md >"$invalid_config.out" 2>"$invalid_config.err"; then
  echo "expected invalid planner configuration to fail closed" >&2
  exit 1
fi
if ! grep -qi "config" "$invalid_config.err"; then
  echo "expected config validation failure, got:" >&2
  cat "$invalid_config.err" >&2
  exit 1
fi

assert_invalid_selector_type() {
  local selector="$1"
  local typed_invalid_config
  typed_invalid_config="$(mktemp)"
  python3 - "$ROOT_DIR/scripts/ci-required-scope.v2.json" "$selector" "$typed_invalid_config" <<'PY'
import json
import sys

source, selector, destination = sys.argv[1:]
with open(source, encoding="utf-8") as handle:
    config = json.load(handle)
config["rules"][0][selector] = "false"
with open(destination, "w", encoding="utf-8") as handle:
    json.dump(config, handle)
    handle.write("\n")
PY
  if "$ROOT_DIR/scripts/plan-rust-required-scope.sh" \
    --event-name pull_request \
    --config "$typed_invalid_config" \
    --changed-path README.md \
    >"$typed_invalid_config.out" 2>"$typed_invalid_config.err"; then
    echo "expected non-boolean $selector selector to fail closed, got:" >&2
    cat "$typed_invalid_config.out" >&2
    cat "$typed_invalid_config.err" >&2
    exit 1
  fi
  if ! grep -qi "config" "$typed_invalid_config.err"; then
    echo "expected $selector selector type validation failure, got:" >&2
    cat "$typed_invalid_config.err" >&2
    exit 1
  fi
  rm -f "$typed_invalid_config" "$typed_invalid_config.out" "$typed_invalid_config.err"
}

assert_invalid_selector_type full
assert_invalid_selector_type minimal

invalid_selector_ownership_config="$(mktemp)"
python3 - "$ROOT_DIR/scripts/ci-required-scope.v2.json" "$invalid_selector_ownership_config" <<'PY'
import json
import sys

source, destination = sys.argv[1:]
with open(source, encoding="utf-8") as handle:
    config = json.load(handle)
config["selector_ownership"] = config["selector_ownership"][:-1]
with open(destination, "w", encoding="utf-8") as handle:
    json.dump(config, handle)
    handle.write("\n")
PY
if "$ROOT_DIR/scripts/plan-rust-required-scope.sh" \
  --event-name pull_request \
  --config "$invalid_selector_ownership_config" \
  --changed-path README.md \
  >"$invalid_selector_ownership_config.out" 2>"$invalid_selector_ownership_config.err"; then
  echo "expected selector ownership drift to fail closed" >&2
  exit 1
fi
if ! grep -qi "selector ownership" "$invalid_selector_ownership_config.err"; then
  echo "expected selector ownership validation failure, got:" >&2
  cat "$invalid_selector_ownership_config.err" >&2
  exit 1
fi
rm -f \
  "$invalid_selector_ownership_config" \
  "$invalid_selector_ownership_config.out" \
  "$invalid_selector_ownership_config.err"

missing_selector_source_dir="$(mktemp -d)"
missing_selector_source_planner="$missing_selector_source_dir/plan-rust-required-scope.py"
cp "$ROOT_DIR/scripts/plan-rust-required-scope.py" "$missing_selector_source_planner"
if python3 "$missing_selector_source_planner" \
  --event-name pull_request \
  --config "$ROOT_DIR/scripts/ci-required-scope.v2.json" \
  --changed-path README.md \
  >"$missing_selector_source_dir/out" 2>"$missing_selector_source_dir/err"; then
  echo "expected selector parity to fail closed when ci-tests.sh is absent" >&2
  cat "$missing_selector_source_dir/out" >&2
  cat "$missing_selector_source_dir/err" >&2
  exit 1
fi
if ! grep -qi "selector source" "$missing_selector_source_dir/err"; then
  echo "expected missing selector source validation failure, got:" >&2
  cat "$missing_selector_source_dir/err" >&2
  exit 1
fi
rm -rf "$missing_selector_source_dir"

wasm_build_output="$(plan_for_path crates/oasis7_wasm_build/src/lib.rs)"
assert_key_equals "$wasm_build_output" scope targeted
assert_key_equals "$wasm_build_output" run_oasis7_workspace_support_crate_tests true
assert_key_equals "$wasm_build_output" run_launcher_web_build false
assert_key_equals "$wasm_build_output" needs_rust_toolchain true
assert_key_equals "$wasm_build_output" needs_system_deps true
assert_key_equals "$wasm_build_output" needs_wasm_target false
assert_key_equals "$wasm_build_output" needs_node false
assert_key_equals "$wasm_build_output" needs_trunk false
assert_reason_contains "$wasm_build_output" "wasm_support:crates/oasis7_wasm_build/src/lib.rs"

wasm_store_output="$(plan_for_path crates/oasis7_wasm_store/src/lib.rs)"
assert_key_equals "$wasm_store_output" scope targeted
assert_key_equals "$wasm_store_output" run_oasis7_required_tests true
assert_key_equals "$wasm_store_output" run_oasis7_workspace_support_crate_tests true
assert_key_equals "$wasm_store_output" run_launcher_web_build false
assert_key_equals "$wasm_store_output" needs_system_deps true
assert_reason_contains "$wasm_store_output" "runtime_wasm_support:crates/oasis7_wasm_store/src/lib.rs"
assert_reason_contains "$wasm_store_output" "wasm_support:crates/oasis7_wasm_store/src/lib.rs"

wasm_router_output="$(plan_for_path crates/oasis7_wasm_router/src/lib.rs)"
assert_key_equals "$wasm_router_output" scope targeted
assert_key_equals "$wasm_router_output" run_oasis7_required_tests true
assert_key_equals "$wasm_router_output" run_oasis7_workspace_support_crate_tests true
assert_reason_contains "$wasm_router_output" "runtime_wasm_support:crates/oasis7_wasm_router/src/lib.rs"
assert_reason_contains "$wasm_router_output" "wasm_support:crates/oasis7_wasm_router/src/lib.rs"

wasm_executor_output="$(plan_for_path crates/oasis7_wasm_executor/src/lib.rs)"
assert_key_equals "$wasm_executor_output" scope targeted
assert_key_equals "$wasm_executor_output" run_oasis7_required_tests true
assert_key_equals "$wasm_executor_output" run_oasis7_workspace_support_crate_tests true
assert_reason_contains "$wasm_executor_output" "runtime_wasm_support:crates/oasis7_wasm_executor/src/lib.rs"
assert_reason_contains "$wasm_executor_output" "wasm_support:crates/oasis7_wasm_executor/src/lib.rs"

builtin_wasm_output="$(plan_for_path crates/oasis7_builtin_wasm_modules/src/lib.rs)"
assert_key_equals "$builtin_wasm_output" scope targeted
assert_key_equals "$builtin_wasm_output" run_oasis7_workspace_support_crate_tests true
assert_reason_contains "$builtin_wasm_output" "wasm_support:crates/oasis7_builtin_wasm_modules/src/lib.rs"

wasm_abi_output="$(plan_for_path crates/oasis7_wasm_abi/src/lib.rs)"
assert_key_equals "$wasm_abi_output" scope targeted
assert_key_equals "$wasm_abi_output" run_oasis7_workspace_support_crate_tests true
assert_key_equals "$wasm_abi_output" run_launcher_web_build true
assert_key_equals "$wasm_abi_output" needs_system_deps true
assert_key_equals "$wasm_abi_output" needs_wasm_target true
assert_reason_contains "$wasm_abi_output" "wasm_abi_support:crates/oasis7_wasm_abi/src/lib.rs"
assert_reason_contains "$wasm_abi_output" "launcher_wasm_abi:crates/oasis7_wasm_abi/src/lib.rs"

proto_output="$(plan_for_path crates/oasis7_proto/src/lib.rs)"
assert_key_equals "$proto_output" scope targeted
assert_key_equals "$proto_output" run_launcher_web_build true
assert_key_equals "$proto_output" needs_trunk true
assert_reason_contains "$proto_output" "launcher_proto:crates/oasis7_proto/src/lib.rs"

viewer_output="$(plan_for_path crates/oasis7_viewer/src/lib.rs)"
assert_key_equals "$viewer_output" scope targeted
assert_key_equals "$viewer_output" run_viewer_contract_tests true
assert_key_equals "$viewer_output" run_viewer_wasm_check true
assert_key_equals "$viewer_output" run_viewer_perf_smoke false
assert_key_equals "$viewer_output" selected_capabilities viewer_js_required
assert_key_equals "$viewer_output" needs_system_deps true
assert_reason_contains "$viewer_output" "viewer_js_required:crates/oasis7_viewer/src/lib.rs"

viewer_perf_output="$(plan_for_path crates/oasis7_viewer/software_safe_src/performance_metrics.js)"
assert_key_equals "$viewer_perf_output" scope targeted
assert_key_equals "$viewer_perf_output" run_viewer_contract_tests true
assert_key_equals "$viewer_perf_output" run_viewer_perf_smoke true
assert_key_equals "$viewer_perf_output" run_rust_baseline true
assert_key_equals "$viewer_perf_output" needs_rust_toolchain true
assert_key_equals "$viewer_perf_output" selected_capabilities 'viewer_js_required;viewer_performance_report'
assert_reason_contains "$viewer_perf_output" "viewer_js_required:crates/oasis7_viewer/software_safe_src/performance_metrics.js"
assert_reason_contains "$viewer_perf_output" "viewer_performance_report:crates/oasis7_viewer/software_safe_src/performance_metrics.js"

viewer_perf_probe_output="$(plan_for_path scripts/viewer-performance-probe.sh)"
assert_key_equals "$viewer_perf_probe_output" scope targeted
assert_key_equals "$viewer_perf_probe_output" run_viewer_contract_tests false
assert_key_equals "$viewer_perf_probe_output" run_viewer_perf_smoke true
assert_key_equals "$viewer_perf_probe_output" needs_node true
assert_key_equals "$viewer_perf_probe_output" selected_capabilities viewer_performance_report
assert_reason_contains "$viewer_perf_probe_output" "viewer_performance_report:scripts/viewer-performance-probe.sh"

viewer_perf_report_output="$(plan_for_paths \
  scripts/viewer-performance-report-only.sh \
  scripts/viewer-performance-report-only-contract.test.sh)"
assert_key_equals "$viewer_perf_report_output" scope targeted
assert_key_equals "$viewer_perf_report_output" run_viewer_perf_smoke true
assert_key_equals "$viewer_perf_report_output" selected_capabilities viewer_performance_report
assert_key_equals "$viewer_perf_report_output" run_rust_baseline false
assert_key_equals "$viewer_perf_report_output" needs_rust_toolchain false
assert_key_equals "$viewer_perf_report_output" needs_node true
assert_reason_contains "$viewer_perf_report_output" \
  "viewer_performance_report:scripts/viewer-performance-report-only.sh"
assert_reason_contains "$viewer_perf_report_output" \
  "viewer_performance_report:scripts/viewer-performance-report-only-contract.test.sh"

pixel_world_bridge_output="$(plan_for_path crates/pixel_world_bridge/src/render.rs)"
assert_key_equals "$pixel_world_bridge_output" scope targeted
assert_key_equals "$pixel_world_bridge_output" run_pixel_world_bridge_lib_tests true
assert_key_equals "$pixel_world_bridge_output" run_pixel_world_bridge_wasm_check true
assert_key_equals "$pixel_world_bridge_output" run_oasis7_workspace_support_crate_tests false
assert_key_equals "$pixel_world_bridge_output" needs_rust_toolchain true
assert_key_equals "$pixel_world_bridge_output" needs_system_deps true
assert_key_equals "$pixel_world_bridge_output" needs_wasm_target true
assert_key_equals "$pixel_world_bridge_output" selected_capabilities pixel_world_bridge
assert_reason_contains "$pixel_world_bridge_output" "pixel_world_bridge:crates/pixel_world_bridge/src/render.rs"

scenario_output="$(plan_for_path crates/oasis7/src/simulator/scenario.rs)"
assert_key_equals "$scenario_output" scope targeted
assert_key_equals "$scenario_output" run_scenario_regression true
assert_key_equals "$scenario_output" selected_capabilities 'launcher_web;oasis7_required;scenario_regression'
assert_reason_contains "$scenario_output" "scenario_regression:crates/oasis7/src/simulator/scenario.rs"

always_on_output="$(plan_for_path doc/testing/prd.md)"
assert_key_equals "$always_on_output" scope minimal
assert_key_equals "$always_on_output" run_required_gate_baseline true
assert_key_equals "$always_on_output" selected_capabilities required_gate_baseline
assert_reason_contains "$always_on_output" "required_gate_baseline:always_on"

shared_required_output="$(plan_for_path .github/workflows/rust.yml)"
assert_key_equals "$shared_required_output" scope full
assert_key_equals "$shared_required_output" run_oasis7_required_tests true
assert_key_equals "$shared_required_output" run_launcher_web_build true
assert_key_equals "$shared_required_output" needs_system_deps true
assert_reason_contains "$shared_required_output" "shared_required_gate:.github/workflows/rust.yml"

node_output="$(plan_for_path crates/oasis7_node/src/network_bridge.rs)"
assert_key_equals "$node_output" scope targeted
assert_key_equals "$node_output" run_oasis7_node_tests true
assert_key_equals "$node_output" run_oasis7_net_tests false
assert_key_equals "$node_output" run_oasis7_net_libp2p_tests false
assert_reason_contains "$node_output" "node:crates/oasis7_node/src/network_bridge.rs"

net_output="$(plan_for_path crates/oasis7_net/src/lib.rs)"
assert_key_equals "$net_output" scope targeted
assert_key_equals "$net_output" run_oasis7_node_tests false
assert_key_equals "$net_output" run_oasis7_net_tests true
assert_key_equals "$net_output" run_oasis7_net_libp2p_tests true
assert_reason_contains "$net_output" "net:crates/oasis7_net/src/lib.rs"

# Trusted integration revalidation executes against the current target plus
# the unchanged source.  A target-only required-gate path can therefore widen
# execution to full while the immutable source projection remains targeted.
integration_tmp_dir="$(mktemp -d)"
integration_input="$integration_tmp_dir/input.json"
integration_projection="$integration_tmp_dir/projection.json"
integration_tampered_projection="$integration_tmp_dir/tampered-projection.json"
integration_head="$(git rev-parse HEAD)"
integration_base="$(git rev-parse HEAD^)"
python3 - "$ROOT_DIR" "$integration_head" "$integration_base" "$integration_input" <<'PY'
import hashlib
import json
import pathlib
import sys

root, head, base, output = map(pathlib.Path, sys.argv[1:])
stable_contract = root / "Cargo.toml"
evidence = {
    "path": "Cargo.toml",
    "sha256": "sha256:" + hashlib.sha256(stable_contract.read_bytes()).hexdigest(),
}
payload = {
    "task_uid": "task_" + "2" * 32,
    "source_head_oid": str(head),
    "scope_base_oid": str(base),
    "changed_paths": ["scripts/pm/workflow-next.py"],
    "change_class": "unknown",
    "manual_roles": ["repository_health_engineer", "qa_engineer"],
    "domain_role": None,
    "test_profile": "required",
    "declared_tests": ["required_gate_baseline"],
    "consumed_contracts": [{"id": "engineering-workflow-terminal-runbook", "revision": "current-main"}],
    "public_semantics": ["terminal task cold-start recovery"],
    "affected_consumers": ["scripts/pm/workflow-next.py"],
    "closure_status": {
        "status": "complete",
        "reason": "focused integration scope regression",
        "evidence": [evidence],
    },
    "verification_affected": False,
}
output.write_text(json.dumps(payload), encoding="utf-8")
PY
python3 "$ROOT_DIR/scripts/pm/workflow-impact-projection.py" \
  --root "$ROOT_DIR" \
  --input "$integration_input" \
  --out "$integration_projection" >/dev/null
integration_projection_digest="$(python3 - "$integration_projection" <<'PY'
import json
import sys
print(json.load(open(sys.argv[1], encoding="utf-8"))["projection_digest"])
PY
)"
integration_source_scope="$(python3 - "$integration_projection" <<'PY'
import json
import sys
print(json.load(open(sys.argv[1], encoding="utf-8"))["ci_scope"])
PY
)"
if [[ "$integration_source_scope" != targeted ]]; then
  echo "expected immutable source projection to remain targeted, got $integration_source_scope" >&2
  exit 1
fi
integration_targeted_output="$("$ROOT_DIR/scripts/plan-rust-required-scope.sh" \
  --event-name workflow_dispatch \
  --run-mode integration_revalidation \
  --base-ref "$integration_base" \
  --head-ref "$integration_head" \
  --task-uid task_22222222222222222222222222222222 \
  --scope-base-oid "$integration_base" \
  --impact-projection "$integration_projection" \
  --changed-path scripts/pm/workflow-next.py)"
assert_key_equals "$integration_targeted_output" scope targeted
assert_key_equals "$integration_targeted_output" selected_capabilities workflow_governance
assert_key_equals "$integration_targeted_output" impact_projection_status verified
assert_key_equals "$integration_targeted_output" impact_projection_digest "$integration_projection_digest"
integration_output="$("$ROOT_DIR/scripts/plan-rust-required-scope.sh" \
  --event-name workflow_dispatch \
  --run-mode integration_revalidation \
  --base-ref "$integration_base" \
  --head-ref "$integration_head" \
  --task-uid task_22222222222222222222222222222222 \
  --scope-base-oid "$integration_base" \
  --impact-projection "$integration_projection" \
  --changed-path .github/workflows/rust.yml)"
assert_key_equals "$integration_output" scope full
assert_key_equals "$integration_output" impact_projection_status verified
assert_key_equals "$integration_output" impact_projection_digest "$integration_projection_digest"
effective_shared_required_output="$("$ROOT_DIR/scripts/plan-rust-required-scope.sh" \
  --event-name pull_request --changed-path .github/workflows/rust.yml)"
assert_key_equals "$integration_output" selected_capabilities \
  "$(value_for_key "$effective_shared_required_output" selected_capabilities)"

python3 - "$integration_projection" "$integration_tampered_projection" <<'PY'
import json
import sys

source, destination = sys.argv[1:]
projection = json.load(open(source, encoding="utf-8"))
projection["public_semantics"].append("tampered")
json.dump(projection, open(destination, "w", encoding="utf-8"))
PY
if "$ROOT_DIR/scripts/plan-rust-required-scope.sh" \
  --event-name workflow_dispatch \
  --run-mode integration_revalidation \
  --base-ref "$integration_base" \
  --head-ref "$integration_head" \
  --task-uid task_22222222222222222222222222222222 \
  --scope-base-oid "$integration_base" \
  --impact-projection "$integration_tampered_projection" \
  --changed-path .github/workflows/rust.yml \
  >"$integration_tmp_dir/tampered.out" 2>"$integration_tmp_dir/tampered.err"; then
  echo "expected integration planner to reject a tampered projection digest" >&2
  exit 1
fi
if ! grep -Fq "impact projection digest mismatch" "$integration_tmp_dir/tampered.err"; then
  echo "expected projection digest failure, got:" >&2
  cat "$integration_tmp_dir/tampered.err" >&2
  exit 1
fi
rm -rf "$integration_tmp_dir"

echo "plan-rust-required-scope.test: OK"
