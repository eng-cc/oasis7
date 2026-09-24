#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/ci-required-dispatch-fixture.lib.sh"
trap ci_fixture_cleanup EXIT
ci_fixture_init

ci_fixture_run required strict
ci_fixture_assert_has 'doc-governance-check.sh'
ci_fixture_assert_lacks 'product-doc-governance-check.test.py'
ci_fixture_assert_lacks 'system-design-traceability-check.test.py'
ci_fixture_assert_lacks 'pm/ci-ready-receipt.test.py'
ci_fixture_assert_lacks 'native-packaging-contract.test.sh'
ci_fixture_assert_lacks 'p2p-public-testnet-identity-v2-signing-tool.test.py'

ci_fixture_run required strict OASIS7_CI_RUN_DOC_CHECKER_CONTRACTS=true
ci_fixture_assert_has 'doc-governance-check.sh'
ci_fixture_assert_has 'product-doc-governance-check.test.py'
ci_fixture_assert_has 'system-design-traceability-check.test.py'
ci_fixture_assert_has 'product-doc-content-check.test.py'
ci_fixture_assert_lacks 'p2p-public-testnet-full-network-clean-room.test.py'
ci_fixture_assert_lacks 'pm/ci-ready-receipt.test.py'
ci_fixture_assert_lacks 'native-packaging-contract.test.sh'
ci_fixture_assert_lacks 'TOOL:cargo:'
ci_fixture_assert_lacks 'TOOL:rustup:'
ci_fixture_assert_lacks 'TOOL:curl:'
ci_fixture_assert_lacks 'TOOL:wget:'

ci_fixture_run required legacy
ci_fixture_assert_has 'product-doc-governance-check.test.py'
ci_fixture_assert_has 'p2p-public-testnet-full-network-clean-room.test.py'
ci_fixture_assert_has 'cargo-dev-windows-toolchain.test.sh'
ci_fixture_assert_lacks 'pm/ci-ready-receipt.test.py'

ci_fixture_run required legacy-mixed OASIS7_CI_RUN_OPERATIONAL_CONTRACTS=true
ci_fixture_assert_has 'pm/ci-ready-receipt.test.py'
ci_fixture_assert_has 'native-packaging-contract.test.sh'
ci_fixture_assert_has 'p2p-public-testnet-package-node-upgrade-health.test.sh'

if ci_fixture_run required strict-incomplete; then
  echo "strict version must reject an omitted planner selector" >&2
  exit 1
fi
ci_fixture_assert_lacks 'SCRIPT:'
grep -Fq 'OASIS7_CI_RUN_CARGO_TOOLING_CONTRACTS must be explicitly true or false' "$ci_fixture_work/output.log"

if ci_fixture_run required strict OASIS7_CI_RUN_PACKAGING_CONTRACTS=1; then
  echo "strict version must reject malformed selector booleans" >&2
  exit 1
fi
ci_fixture_assert_lacks 'TOOL:'
grep -Fq 'OASIS7_CI_RUN_PACKAGING_CONTRACTS must be explicitly true or false' "$ci_fixture_work/output.log"

if ci_fixture_run required strict OASIS7_CI_EXECUTION_CONTRACT=unknown/v9; then
  echo "strict version must reject unknown contracts" >&2
  exit 1
fi
ci_fixture_assert_lacks 'TOOL:'
grep -Fq 'unsupported required-gate execution contract: unknown/v9' "$ci_fixture_work/output.log"

if ci_fixture_run required strict OASIS7_CI_NEEDS_MARKDOWN=false; then
  echo "strict version must reject baseline resource contradictions" >&2
  exit 1
fi
ci_fixture_assert_lacks 'TOOL:'
grep -Fq 'baseline document checks require planned Python and Markdown resources' "$ci_fixture_work/output.log"

if ci_fixture_run required strict OASIS7_CI_RUN_CARGO_TOOLING_CONTRACTS=true; then
  echo "strict version must reject selected Cargo tooling without its Rust resource" >&2
  exit 1
fi
ci_fixture_assert_lacks 'TOOL:'
grep -Fq 'Cargo tooling contracts require the planned Rust toolchain resource' "$ci_fixture_work/output.log"

if ci_fixture_run required strict \
  OASIS7_CI_RUN_PIXEL_WORLD_BRIDGE_LIB_TESTS=true \
  OASIS7_CI_RUN_PIXEL_WORLD_BRIDGE_WASM_CHECK=true; then
  echo "selected pixel-world WASM bridge must reject missing Rust and WASM resources" >&2
  exit 1
fi
ci_fixture_assert_lacks 'TOOL:cargo:'
grep -Fq 'OASIS7_CI_RUN_PIXEL_WORLD_BRIDGE_LIB_TESTS requires planned resource OASIS7_CI_NEEDS_RUST_TOOLCHAIN' "$ci_fixture_work/output.log"

if ci_fixture_run required strict \
  OASIS7_CI_RUN_PIXEL_WORLD_BRIDGE_LIB_TESTS=true \
  OASIS7_CI_RUN_PIXEL_WORLD_BRIDGE_WASM_CHECK=true \
  OASIS7_CI_RUN_RUST_BASELINE=true \
  OASIS7_CI_NEEDS_RUST_TOOLCHAIN=true \
  OASIS7_CI_NEEDS_SYSTEM_DEPS=true; then
  echo "selected pixel-world WASM bridge must reject a missing WASM target" >&2
  exit 1
fi
ci_fixture_assert_lacks 'TOOL:cargo:'
grep -Fq 'OASIS7_CI_RUN_PIXEL_WORLD_BRIDGE_LIB_TESTS requires planned resource OASIS7_CI_NEEDS_WASM_TARGET' "$ci_fixture_work/output.log"

if ci_fixture_run required strict \
  OASIS7_CI_RUN_VIEWER_CONTRACT_TESTS=true \
  OASIS7_CI_RUN_VIEWER_WASM_CHECK=true \
  OASIS7_CI_RUN_RUST_BASELINE=true \
  OASIS7_CI_NEEDS_RUST_TOOLCHAIN=true \
  OASIS7_CI_NEEDS_NODE=true \
  OASIS7_CI_NEEDS_SYSTEM_DEPS=true; then
  echo "selected viewer build must reject a missing WASM target" >&2
  exit 1
fi
ci_fixture_assert_lacks 'SCRIPT:'
grep -Fq 'OASIS7_CI_RUN_VIEWER_CONTRACT_TESTS requires planned resource OASIS7_CI_NEEDS_WASM_TARGET' "$ci_fixture_work/output.log"

if ci_fixture_run required strict \
  OASIS7_CI_RUN_LAUNCHER_WEB_BUILD=true \
  OASIS7_CI_RUN_RUST_BASELINE=true \
  OASIS7_CI_NEEDS_RUST_TOOLCHAIN=true \
  OASIS7_CI_NEEDS_NODE=true \
  OASIS7_CI_NEEDS_SYSTEM_DEPS=true \
  OASIS7_CI_NEEDS_WASM_TARGET=true; then
  echo "selected launcher build must reject a missing Trunk resource" >&2
  exit 1
fi
ci_fixture_assert_lacks 'SCRIPT:'
grep -Fq 'OASIS7_CI_RUN_LAUNCHER_WEB_BUILD requires planned resource OASIS7_CI_NEEDS_TRUNK' "$ci_fixture_work/output.log"

if ci_fixture_run required strict \
  OASIS7_CI_RUN_DOC_CHECKER_CONTRACTS=true \
  CI_FIXTURE_FAIL_SCRIPT=product-doc-governance-check.test.py; then
  echo "a selected failing contract must fail the required dispatcher" >&2
  exit 1
fi
grep -Fq 'INJECTED_FAILURE:product-doc-governance-check.test.py' "$ci_fixture_log"
grep -Fq 'TOOL:python3:./scripts/product-doc-governance-check.test.py' "$ci_fixture_log"

if ci_fixture_run required legacy OASIS7_CI_RUN_PACKAGING_CONTRACTS=false; then
  echo "unversioned legacy mode must reject mixed new-selector fields" >&2
  exit 1
fi
ci_fixture_assert_lacks 'SCRIPT:'
grep -Fq 'new required-gate selector/resource values require OASIS7_CI_EXECUTION_CONTRACT' "$ci_fixture_work/output.log"

if ci_fixture_run required legacy OASIS7_CI_NEEDS_PYTHON=true; then
  echo "unversioned legacy mode must reject mixed new-resource fields" >&2
  exit 1
fi
ci_fixture_assert_lacks 'TOOL:'
grep -Fq 'new required-gate selector/resource values require OASIS7_CI_EXECUTION_CONTRACT' "$ci_fixture_work/output.log"

echo "ci-required-baseline-routing.test: passed"
