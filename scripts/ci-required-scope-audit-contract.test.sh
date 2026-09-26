#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
planner="$repo_root/scripts/plan-rust-required-scope.sh"
ci_tests="$repo_root/scripts/ci-tests.sh"
versioned_config="$repo_root/scripts/fixtures/ci-required-scope.versioned-test.json"
legacy_config="$repo_root/scripts/fixtures/ci-required-scope.legacy-test.json"
legacy_config_sha256="sha256:d656841b3c9fcf66fcd5ea1c37b43d9628e61d13ea48be8bda54e71511d505b4"
versioned_config_sha256="sha256:2d4228e5d7446c393ea3811084183860b5f8b4bb0e3b673957a3a020ab172dec"

value_for_key() {
  local output="$1"
  local key="$2"
  printf '%s\n' "$output" | awk -F= -v key="$key" '$1 == key {print substr($0, length(key) + 2)}'
}

require_key() {
  local output="$1"
  local key="$2"
  local expected="$3"
  local actual
  actual="$(value_for_key "$output" "$key")"
  if [[ "$actual" != "$expected" ]]; then
    echo "required-gate always-on check is not auditable in planner: expected $key=$expected, got $actual" >&2
    exit 1
  fi
}

require_reason_contains() {
  local output="$1"
  local expected="$2"
  local actual
  actual="$(value_for_key "$output" reason_summary)"
  if [[ "$actual" != *"$expected"* ]]; then
    echo "required-gate always-on check has no auditable planner reason: expected reason_summary to contain $expected, got $actual" >&2
    exit 1
  fi
}

require_ci_tests_line() {
  local expected="$1"
  if ! grep -Fqx -- "$expected" "$ci_tests"; then
    echo "standalone lockfile checks are not planner-gated: missing ci-tests line: $expected" >&2
    exit 1
  fi
}

required_component_impl="$(
  sed -n '/^should_run_ci_required_component() {/,/^}/p' "$ci_tests"
  sed -n '/^run_required_component() {/,/^}/p' "$ci_tests"
)"
if [[ -z "$required_component_impl" ]]; then
  echo "unable to extract run_required_component from ci-tests.sh for the fixture" >&2
  exit 1
fi
eval "$required_component_impl"

fixture_dir="$(mktemp -d)"
trap 'rm -rf "$fixture_dir"' EXIT
fixture_marker="$fixture_dir/standalone-lockfile-check-ran"
run_standalone_tool_lockfiles_checks_fixture() {
  : >"$fixture_marker"
}

OASIS7_CI_RUN_RUST_BASELINE=false
run_required_component \
  "standalone tool lockfiles" \
  "$OASIS7_CI_RUN_RUST_BASELINE" \
  "disabled_by_scope_planner" \
  run_standalone_tool_lockfiles_checks_fixture
if [[ -e "$fixture_marker" ]]; then
  echo "standalone lockfile fixture ran while OASIS7_CI_RUN_RUST_BASELINE=false" >&2
  exit 1
fi

OASIS7_CI_RUN_RUST_BASELINE=true
run_required_component \
  "standalone tool lockfiles" \
  "$OASIS7_CI_RUN_RUST_BASELINE" \
  "disabled_by_scope_planner" \
  run_standalone_tool_lockfiles_checks_fixture
if [[ ! -e "$fixture_marker" ]]; then
  echo "standalone lockfile fixture did not run while OASIS7_CI_RUN_RUST_BASELINE=true" >&2
  exit 1
fi

minimal_plan="$("$planner" --event-name pull_request --changed-path doc/testing/prd.md)"
require_key "$minimal_plan" run_required_gate_baseline true
require_key "$minimal_plan" run_operational_contracts false
require_key "$minimal_plan" selected_capabilities required_gate_baseline
require_reason_contains "$minimal_plan" required_gate_baseline:always_on
effective_execution_contract="$(value_for_key "$minimal_plan" execution_contract)"
effective_config_sha256="$(value_for_key "$minimal_plan" planner_config_sha256)"
if [[ -z "$effective_execution_contract" && "$effective_config_sha256" == "$legacy_config_sha256" ]]; then
  effective_policy_mode=legacy
  cmp -s "$repo_root/scripts/ci-required-scope.v2.json" "$legacy_config" || {
    echo "legacy effective required-scope config differs from its pinned compatibility fixture" >&2
    exit 1
  }
elif [[ "$effective_execution_contract" == required-domain-split/v1 && "$effective_config_sha256" == "$versioned_config_sha256" ]]; then
  effective_policy_mode=versioned
  cmp -s "$repo_root/scripts/ci-required-scope.v2.json" "$versioned_config" || {
    echo "versioned effective required-scope config differs from its pinned fixture" >&2
    exit 1
  }
  require_key "$minimal_plan" run_packaging_contracts false
  require_key "$minimal_plan" run_workflow_governance_contracts false
  require_key "$minimal_plan" run_doc_checker_contracts false
  require_key "$minimal_plan" run_cargo_tooling_contracts false
else
  echo "effective required-scope config has an unsupported contract or digest: contract=${effective_execution_contract:-legacy} digest=$effective_config_sha256" >&2
  exit 1
fi

packaging_plan="$($planner --event-name pull_request \
  --changed-path scripts/package-native-installer.sh \
  --changed-path scripts/validate-release-platform-entrypoints.sh \
  --changed-path scripts/package-viewer-web-delivery.sh \
  --changed-path scripts/packaging-artifact-size-contract.test.sh \
  --changed-path scripts/copy-viewer-web-dist.test.sh \
  --changed-path scripts/native-packaging-contract.test.sh \
  --changed-path scripts/package-workflow-cache-reuse-contract.test.sh)"
require_key "$packaging_plan" scope targeted
require_key "$packaging_plan" selected_capabilities packaging_contracts
if [[ "$effective_policy_mode" == legacy ]]; then
  require_key "$packaging_plan" run_operational_contracts true
else
  require_key "$packaging_plan" execution_contract required-domain-split/v1
  require_key "$packaging_plan" planner_config_sha256 "$versioned_config_sha256"
  require_key "$packaging_plan" run_packaging_contracts true
  require_key "$packaging_plan" run_operational_contracts false
  require_key "$packaging_plan" needs_python true
  require_key "$packaging_plan" needs_markdown true
fi
require_key "$packaging_plan" run_rust_baseline false
require_key "$packaging_plan" needs_rust_toolchain false
require_key "$packaging_plan" needs_node false
require_key "$packaging_plan" needs_system_deps false
for packaging_path in \
  scripts/package-native-installer.sh \
  scripts/validate-release-platform-entrypoints.sh \
  scripts/package-viewer-web-delivery.sh \
  scripts/packaging-artifact-size-contract.test.sh \
  scripts/copy-viewer-web-dist.test.sh \
  scripts/native-packaging-contract.test.sh \
  scripts/package-workflow-cache-reuse-contract.test.sh; do
  require_reason_contains "$packaging_plan" "packaging_contracts:$packaging_path"
done
require_reason_contains "$packaging_plan" "required_gate_baseline:always_on"

# Exercise versioned packaging semantics against the pinned fixture regardless
# of the current effective policy; effective assertions above are selected
# only after validating its contract and config digest.
versioned_packaging_plan="$("$planner" --event-name pull_request --config "$versioned_config" \
  --changed-path scripts/package-native-installer.sh \
  --changed-path scripts/validate-release-platform-entrypoints.sh \
  --changed-path scripts/package-viewer-web-delivery.sh \
  --changed-path scripts/packaging-artifact-size-contract.test.sh \
  --changed-path scripts/copy-viewer-web-dist.test.sh \
  --changed-path scripts/native-packaging-contract.test.sh \
  --changed-path scripts/package-workflow-cache-reuse-contract.test.sh)"
require_key "$versioned_packaging_plan" execution_contract required-domain-split/v1
require_key "$versioned_packaging_plan" planner_config_sha256 sha256:2d4228e5d7446c393ea3811084183860b5f8b4bb0e3b673957a3a020ab172dec
require_key "$versioned_packaging_plan" scope targeted
require_key "$versioned_packaging_plan" selected_capabilities packaging_contracts
require_key "$versioned_packaging_plan" run_packaging_contracts true
require_key "$versioned_packaging_plan" run_operational_contracts false
require_key "$versioned_packaging_plan" run_rust_baseline false
require_key "$versioned_packaging_plan" needs_rust_toolchain false
require_key "$versioned_packaging_plan" needs_python true
require_key "$versioned_packaging_plan" needs_markdown true
require_reason_contains "$versioned_packaging_plan" "packaging_contracts:scripts/package-native-installer.sh"

release_packaging_plan="$($planner --event-name pull_request \
  --changed-path .github/workflows/release-packages.yml \
  --changed-path scripts/build-game-launcher-bundle.sh)"
require_key "$release_packaging_plan" scope full
require_key "$release_packaging_plan" run_rust_baseline true
require_key "$release_packaging_plan" needs_rust_toolchain true

operational_plan="$("$planner" --event-name pull_request --changed-path scripts/p2p-public-testnet-package-rollout.test.sh)"
require_key "$operational_plan" run_required_gate_baseline true
require_key "$operational_plan" run_operational_contracts true
if [[ "$effective_policy_mode" == versioned ]]; then
  require_key "$operational_plan" execution_contract required-domain-split/v1
  require_key "$operational_plan" planner_config_sha256 "$versioned_config_sha256"
  require_key "$operational_plan" run_packaging_contracts false
fi
require_key "$operational_plan" run_rust_baseline false
require_key "$operational_plan" needs_rust_toolchain false
require_key "$operational_plan" selected_capabilities operational_contracts
require_reason_contains "$operational_plan" operational_contracts:scripts/p2p-public-testnet-package-rollout.test.sh

site_plan="$("$planner" --event-name pull_request --changed-path site/index.html)"
require_key "$site_plan" run_required_gate_baseline true
require_key "$site_plan" run_site_contract_tests true
require_key "$site_plan" run_rust_baseline false
require_key "$site_plan" needs_rust_toolchain false
require_key "$site_plan" selected_capabilities site_quality
require_reason_contains "$site_plan" site_quality:site/index.html

workflow_governance_operational_source="$(sed -n '/^run_workflow_governance_operational_contract_tests() {/,/^}/p' "$ci_tests")"
if ! grep -Fqx '  run python3 ./scripts/pm/ci-ready-receipt.test.py' <<<"$workflow_governance_operational_source"; then
  echo "workflow-governance receipt contract is not wired into run_workflow_governance_operational_contract_tests" >&2
  exit 1
fi
if ! grep -Fqx '  run ./scripts/ci-required-scope-audit-contract.test.sh' <<<"$workflow_governance_operational_source"; then
  echo "required scope audit contract is not wired into run_workflow_governance_operational_contract_tests" >&2
  exit 1
fi

if ! grep -Fqx '    run_required_component "site quality contracts" "${OASIS7_CI_RUN_SITE_CONTRACT_TESTS:-}" "disabled_by_scope_planner" run_site_contract_tests' "$ci_tests"; then
  echo "site quality contracts are not wired to the planner selector in ci-tests" >&2
  exit 1
fi

packaging_runner_source="$(sed -n '/^run_packaging_artifact_contract_tests() {/,/^}/p' "$ci_tests")"
if ! grep -Fqx '  run bash ./scripts/native-packaging-contract.test.sh' <<<"$packaging_runner_source"; then
  echo "packaging contract runner is not wired to native packaging fixtures" >&2
  exit 1
fi
if ! grep -Fqx '  run bash ./scripts/packaging-artifact-size-contract.test.sh' <<<"$packaging_runner_source"; then
  echo "packaging contract runner is not wired to artifact-size fixtures" >&2
  exit 1
fi
if ! grep -Fqx '  run bash ./scripts/copy-viewer-web-dist.test.sh' <<<"$packaging_runner_source"; then
  echo "packaging contract runner is not wired to Viewer delivery fixtures" >&2
  exit 1
fi
if ! grep -Fqx '  run bash ./scripts/package-workflow-cache-reuse-contract.test.sh' <<<"$packaging_runner_source"; then
  echo "packaging contract runner is not wired to package workflow cache fixtures" >&2
  exit 1
fi
legacy_mixed_contract_source="$(sed -n '/^run_legacy_mixed_operational_contract_tests() {/,/^}/p' "$ci_tests")"
if ! grep -Fqx '  run_packaging_artifact_contract_tests' <<<"$legacy_mixed_contract_source" || \
   ! grep -Fqx '  run_packaging_artifact_contract_tests' <<<"$(sed -n '/^run_packaging_contract_tests() {/,/^}/p' "$ci_tests")"; then
  echo "legacy mixed contract runner must include the focused packaging artifact runner" >&2
  exit 1
fi

# The focused runner must stay a non-Rust boundary even when its fixture
# scripts inspect Rust-producing workflows as data.
if grep -Eiq '(^|[[:space:];|&()])(cargo|rustup)([[:space:]]|$)' <<<"$packaging_runner_source" || \
   grep -Eiq '(^|[[:space:];|&()])run_cargo([[:space:]]|$)' <<<"$packaging_runner_source"; then
  echo "packaging contract runner must not invoke Cargo or rustup directly" >&2
  exit 1
fi

for package_workflow_path in \
  .github/workflows/testnet-packages.yml \
  .github/workflows/mainnet-packages.yml \
  .github/workflows/release-packages.yml; do
  package_workflow_plan="$($planner --event-name pull_request --changed-path "$package_workflow_path")"
  require_key "$package_workflow_plan" scope full
  require_key "$package_workflow_plan" run_operational_contracts true
  require_key "$package_workflow_plan" run_rust_baseline true
  if [[ "$effective_policy_mode" == versioned ]]; then
    require_key "$package_workflow_plan" run_packaging_contracts true
  fi
  require_reason_contains "$package_workflow_plan" "packaging_workflow:$package_workflow_path"
done

python3 - \
  "$repo_root/scripts/ci-required-scope.v2.json" \
  "$ci_tests" \
  "$repo_root/.github/workflows/rust.yml" \
  "$planner" <<'PY'
import json
import os
import re
import subprocess
import sys
import tempfile
import textwrap
from types import SimpleNamespace
from pathlib import Path

config_path, ci_tests_path, workflow_path, planner_path = map(Path, sys.argv[1:])
repo_root = config_path.parent.parent
config = json.loads(config_path.read_text(encoding="utf-8"))
ownership = config.get("selector_ownership")
if not isinstance(ownership, list):
    raise SystemExit("selector ownership registry is missing")
declared = {}
for item in ownership:
    if not isinstance(item, dict):
        raise SystemExit("selector ownership entries must be objects")
    name = item.get("name")
    if not isinstance(name, str) or name in declared:
        raise SystemExit(f"selector ownership name is missing or duplicated: {name!r}")
    declared[name] = item
inventory = set(re.findall(r"OASIS7_CI_RUN_[A-Z0-9_]+", ci_tests_path.read_text(encoding="utf-8")))
execution_contract = config.get("execution_contract", "")
versioned_selector_names = {
    "OASIS7_CI_RUN_WORKFLOW_GOVERNANCE_CONTRACTS",
    "OASIS7_CI_RUN_PACKAGING_CONTRACTS",
    "OASIS7_CI_RUN_DOC_CHECKER_CONTRACTS",
    "OASIS7_CI_RUN_CARGO_TOOLING_CONTRACTS",
}
active_inventory = inventory
if execution_contract == "":
    active_inventory = inventory - versioned_selector_names
elif execution_contract != "required-domain-split/v1":
    raise SystemExit(f"unsupported effective required-gate execution contract: {execution_contract}")
if active_inventory != set(declared):
    raise SystemExit(
        "selector ownership drift: "
        f"missing={sorted(active_inventory - set(declared))}, "
        f"stale={sorted(set(declared) - active_inventory)}"
    )
for name, item in declared.items():
    mode = item.get("mode")
    if mode == "planner-owned":
        if not isinstance(item.get("planner_field"), str) or not item["planner_field"].startswith("run_"):
            raise SystemExit(f"planner-owned selector lacks planner field: {name}")
        if "owner" in item or "reason" in item:
            raise SystemExit(f"planner-owned selector has manual-only metadata: {name}")
    elif mode == "manual-only":
        if not item.get("owner") or not item.get("reason"):
            raise SystemExit(f"manual-only selector lacks owner/reason: {name}")
        if "planner_field" in item:
            raise SystemExit(f"manual-only selector unexpectedly has planner field: {name}")
    else:
        raise SystemExit(f"selector has invalid ownership mode: {name}")

planner_run = subprocess.run(
    [
        str(planner_path),
        "--event-name",
        "pull_request",
        "--config",
        str(config_path),
        "--changed-path",
        "README.md",
    ],
    check=False,
    capture_output=True,
    text=True,
)
if planner_run.returncode != 0:
    raise SystemExit(
        "required-gate planner failed while auditing selector output path: "
        + planner_run.stderr.strip()
    )
planner_outputs = {
    key: value
    for line in planner_run.stdout.splitlines()
    if "=" in line
    for key, value in [line.split("=", 1)]
}
if execution_contract == "" and any(
    field in planner_outputs
    for field in (
        "execution_contract",
        "run_workflow_governance_contracts",
        "run_packaging_contracts",
        "run_doc_checker_contracts",
        "run_cargo_tooling_contracts",
        "needs_python",
        "needs_markdown",
    )
):
    raise SystemExit("legacy effective planner unexpectedly emitted versioned contract fields")

# Every public-testnet package/rollout/observer/fleet-health implementation
# source must stay paired with an operational fixture that invokes or imports
# it. Audit both sides here: a missing fixture reference is a contract gap,
# while an unmatched source would otherwise silently widen the required gate
# through the planner's fail-closed fallback.
operational_source_fixtures = {
    "scripts/p2p-public-testnet-package-rollout.py": [
        "scripts/p2p-public-testnet-package-rollout.test.sh",
        "scripts/p2p-observer-checkpoint-closure-probe-safety.test.py",
    ],
    "scripts/p2p-public-testnet-package-node-upgrade.sh": [
        "scripts/p2p-public-testnet-package-rollout.test.sh",
        "scripts/p2p-public-testnet-package-node-upgrade.test.sh",
        "scripts/p2p-public-testnet-package-node-upgrade-health.test.sh",
        "scripts/p2p-public-testnet-package-node-upgrade-order.test.sh",
        "scripts/p2p-public-testnet-package-node-upgrade-rollback-contract.test.sh",
    ],
    "scripts/p2p-public-testnet-bootstrap-fresh-validator-host.sh": [
        "scripts/p2p-public-testnet-bootstrap-fresh-validator-host.test.sh",
    ],
    "scripts/p2p-public-testnet-local-observer-sync.sh": [
        "scripts/p2p-public-testnet-local-observer-sync.test.sh",
    ],
    "scripts/p2p-observer-checkpoint-closure-probe.py": [
        "scripts/p2p-observer-checkpoint-closure-probe.test.sh",
        "scripts/p2p-observer-checkpoint-closure-probe-safety.test.py",
    ],
    "scripts/p2p-public-testnet-fleet-health.py": [
        "scripts/p2p-public-testnet-fleet-health.test.py",
    ],
    "scripts/p2p-verify-linux-package-bundle.py": [
        "scripts/p2p-public-testnet-package-rollout.test.sh",
        "scripts/p2p-public-testnet-bootstrap-fresh-validator-host.test.sh",
        "scripts/p2p-public-testnet-package-node-upgrade.test.sh",
        "scripts/testnet-packages-linux-bundle-bootstrap-contract.test.sh",
    ],
    "scripts/p2p-rebuild-linux-bundle-checksums.py": [
        "scripts/p2p-public-testnet-package-rollout.test.sh",
        "scripts/p2p-public-testnet-package-node-upgrade.test.sh",
    ],
}
for source_path, fixture_paths in operational_source_fixtures.items():
    source = repo_root / source_path
    if not source.is_file():
        raise SystemExit(f"operational source is missing: {source_path}")
    source_name = source.name
    source_plan = subprocess.run(
        [
            str(planner_path),
            "--event-name",
            "pull_request",
            "--config",
            str(config_path),
            "--changed-path",
            source_path,
        ],
        check=False,
        capture_output=True,
        text=True,
    )
    if source_plan.returncode != 0:
        raise SystemExit(
            f"planner failed for operational source {source_path}: "
            + source_plan.stderr.strip()
        )
    source_outputs = {
        key: value
        for line in source_plan.stdout.splitlines()
        if "=" in line
        for key, value in [line.split("=", 1)]
    }
    for key, expected in {
        "scope": "targeted",
        "selected_capabilities": "operational_contracts",
        "run_operational_contracts": "true",
        "run_rust_baseline": "false",
        "needs_rust_toolchain": "false",
    }.items():
        if source_outputs.get(key) != expected:
            raise SystemExit(
                f"operational source planner drift for {source_path}: "
                f"expected {key}={expected}, got {source_outputs.get(key)!r}"
            )
    if "unclassified_or_unresolvable:" in source_outputs.get("reason_summary", ""):
        raise SystemExit(
            f"operational source remains unmatched in planner: {source_path}"
        )
    for fixture_path in fixture_paths:
        fixture = repo_root / fixture_path
        if not fixture.is_file():
            raise SystemExit(f"operational fixture is missing: {fixture_path}")
        if source_name not in fixture.read_text(encoding="utf-8"):
            raise SystemExit(
                f"operational fixture does not invoke/import {source_path}: "
                f"{fixture_path}"
            )

workflow_text = workflow_path.read_text(encoding="utf-8")
required_gate_match = re.search(
    r"(?ms)^  required-gate:\n(?P<body>.*?)(?=^  [A-Za-z0-9_-]+:\n|\Z)",
    workflow_text,
)
if not required_gate_match:
    raise SystemExit("required-gate workflow job is missing")
required_gate_body = required_gate_match.group("body")
if '--github-output "${GITHUB_OUTPUT}"' not in required_gate_body:
    raise SystemExit("required-gate planner output is not written to GITHUB_OUTPUT")
run_tier_match = re.search(
    r"(?ms)^      - name: Run required test tier\n(?P<body>.*?)(?=^      - |\Z)",
    required_gate_body,
)
if not run_tier_match:
    raise SystemExit("required-gate test-tier env path is missing")
run_tier_body = run_tier_match.group("body")

canonical_workflow_text = (
    repo_root / "doc/engineering/workflow/source-of-truth.md"
).read_text(encoding="utf-8")
native_wasm_anchor = '<a id="required-gate-native-wasm-smoke"></a>'
native_wasm_anchor_start = canonical_workflow_text.find(native_wasm_anchor)
if native_wasm_anchor_start < 0:
    raise SystemExit("canonical native WASM smoke exception is missing")
native_wasm_anchor_end = canonical_workflow_text.find(
    "<a id=", native_wasm_anchor_start + len(native_wasm_anchor)
)
native_wasm_contract = canonical_workflow_text[
    native_wasm_anchor_start:
    native_wasm_anchor_end if native_wasm_anchor_end >= 0 else None
]
for contract_fragment in (
    "Cargo package-profile planner's `target` field is a selector",
    "`native` selects host-native Cargo tests",
    "every planned item whose `target` is `native`",
    "`OASIS7_WASM_BUILD_STD=0`",
    "independent of package name",
    "copied subprocess environment",
    "non-native items",
    "`OASIS7_WASM_TOOLCHAIN=nightly-2025-12-11`",
    "`OASIS7_WASM_BUILD_STD=1`",
    "`OASIS7_WASM_BUILD_STD_COMPONENTS=std,panic_abort`",
):
    if contract_fragment not in native_wasm_contract:
        raise SystemExit(
            "canonical native WASM smoke exception omits required contract: "
            f"{contract_fragment}"
        )

profile_runner_match = re.search(
    r"(?ms)^[ \t]*python3 -I - \"\$\{OASIS7_CARGO_PROFILE_PLAN\}\" "
    r"\"\$\{OASIS7_CARGO_PROFILE_RESULTS\}\" <<'PY'[ \t]*\n"
    r"(?P<source>.*?)^[ \t]*PY[ \t]*$",
    run_tier_body,
)
if not profile_runner_match:
    raise SystemExit("required-gate package-profile runner heredoc is missing")
profile_runner_source = textwrap.dedent(profile_runner_match.group("source"))
profile_plan = {
    "items": [
        {
            "id": "wasm_build_suite-native",
            "command": ["cargo", "test", "--package", "wasm_build_suite_native"],
            "command_digest": "native-command-digest",
            "package": "wasm_build_suite",
            "profile": "native",
            "target": "native",
            "features": [],
        },
        {
            "id": "wasm_module_observe-native",
            "command": ["cargo", "test", "--package", "wasm_module_observe"],
            "command_digest": "observer-command-digest",
            "package": "wasm_module_observe",
            "profile": "native",
            "target": "native",
            "features": [],
        },
        {
            "id": "unrelated-native-profile",
            "command": ["cargo", "test", "--package", "unrelated_native"],
            "command_digest": "other-command-digest",
            "package": "unrelated_native",
            "profile": "native",
            "target": "native",
            "features": [],
        },
        {
            "id": "wasm_build_suite-wasm",
            "command": ["cargo", "check", "--package", "wasm_build_suite", "--target", "wasm32-unknown-unknown"],
            "command_digest": "wasm-command-digest",
            "package": "wasm_build_suite",
            "profile": "wasm",
            "target": "wasm32-unknown-unknown",
            "features": [],
        },
    ],
    "plan_id": "profile-env-contract-test",
    "trusted_authority": {"toolchain": "1.96.0"},
    "integration_base": "base-commit",
    "source_head": "head-commit",
    "tested_tree": "tested-tree",
}
with tempfile.TemporaryDirectory() as temporary_directory:
    temporary_path = Path(temporary_directory)
    plan_path = temporary_path / "plan.json"
    results_path = temporary_path / "results.json"
    plan_path.write_text(json.dumps(profile_plan), encoding="utf-8")
    prior_argv = sys.argv
    prior_run = subprocess.run
    prior_build_std = os.environ.get("OASIS7_WASM_BUILD_STD")
    observed_profile_envs = []
    observed_global_build_std = []

    def record_profile_run(command, *, env, check):
        observed_profile_envs.append((command, dict(env), check))
        observed_global_build_std.append(os.environ.get("OASIS7_WASM_BUILD_STD"))
        return SimpleNamespace(returncode=0)

    os.environ["OASIS7_WASM_BUILD_STD"] = "1"
    subprocess.run = record_profile_run
    sys.argv = ["required-gate-profile-runner", str(plan_path), str(results_path)]
    try:
        exec(compile(profile_runner_source, "required-gate-profile-runner", "exec"), {})
    finally:
        sys.argv = prior_argv
        subprocess.run = prior_run
        if prior_build_std is None:
            os.environ.pop("OASIS7_WASM_BUILD_STD", None)
        else:
            os.environ["OASIS7_WASM_BUILD_STD"] = prior_build_std

if len(observed_profile_envs) != len(profile_plan["items"]):
    raise SystemExit("required-gate profile runner omitted planned item invocations")
observed_build_std_by_item = {
    item["id"]: environment.get("OASIS7_WASM_BUILD_STD")
    for item, (_, environment, _) in zip(profile_plan["items"], observed_profile_envs)
}
for item_id in (
    "wasm_build_suite-native",
    "wasm_module_observe-native",
    "unrelated-native-profile",
):
    if observed_build_std_by_item.get(item_id) != "0":
        raise SystemExit(
            "required-gate native-target item must disable build-std independent of package name: "
            f"{item_id}={observed_build_std_by_item.get(item_id)!r}"
        )
if observed_build_std_by_item.get("wasm_build_suite-wasm") != "1":
    raise SystemExit(
        "required-gate non-native item must inherit the workflow build-std environment"
    )
if observed_global_build_std != ["1"] * len(profile_plan["items"]):
    raise SystemExit("required-gate native override mutated the workflow-level environment")
if any(check for _, _, check in observed_profile_envs):
    raise SystemExit("required-gate profile runner changed subprocess check semantics")

for workflow_fragment in (
    '  OASIS7_WASM_TOOLCHAIN: nightly-2025-12-11',
    '  OASIS7_WASM_BUILD_STD: "1"',
    "  OASIS7_WASM_BUILD_STD_COMPONENTS: std,panic_abort",
    'rustup toolchain install "${OASIS7_WASM_TOOLCHAIN}" --profile minimal --component rust-src',
):
    if workflow_fragment not in workflow_text:
        raise SystemExit(
            "required-gate native override must preserve pinned nightly determinism contract: "
            f"{workflow_fragment}"
        )

for name, item in declared.items():
    if item.get("mode") == "planner-owned":
        field = item["planner_field"]
        if field not in planner_outputs:
            raise SystemExit(
                f"planner-owned selector is missing planner output: {name} -> {field}"
            )
        expected_env = f"          {name}: ${{{{ steps.scope.outputs.{field} }}}}"
        if expected_env not in run_tier_body:
            raise SystemExit(
                "planner-owned selector is not passed through required-gate env: "
                f"{name} -> {field}"
            )
    else:
        env_prefix = f"          {name}:"
        env_assignment = next(
            (line for line in run_tier_body.splitlines() if line.startswith(env_prefix)),
            "",
        )
        if "steps.scope.outputs." in env_assignment:
            raise SystemExit(
                f"manual-only selector is unexpectedly planner-wired in workflow: {name}"
            )
        expected_default = f"${{{name}:-false}}"
        if expected_default not in ci_tests_path.read_text(encoding="utf-8"):
            raise SystemExit(
                f"manual-only selector does not default disabled in ci-tests: {name}"
            )
PY

# This is intentionally a direct-source guard.  Operational contract fixtures
# may mention Cargo or use fake Cargo binaries in their own test processes, but
# the runner itself must not gain a real Rust toolchain invocation unnoticed.
if grep -Eiq '(^|[[:space:];|&()])(cargo|rustup)([[:space:]]|$)' <<<"$legacy_mixed_contract_source" || \
   grep -Eiq '(^|[[:space:];|&()])run_cargo([[:space:]]|$)' <<<"$legacy_mixed_contract_source"; then
  echo "operational contract runner must not invoke Cargo or rustup directly" >&2
  exit 1
fi

require_ci_tests_line 'run_standalone_tool_lockfiles_checks() {'
require_ci_tests_line '  run bash ./scripts/check-standalone-tool-lockfiles.test.sh'
require_ci_tests_line '  run ./scripts/check-standalone-tool-lockfiles.sh'
require_ci_tests_line '  run_required_component "standalone tool lockfiles" "${OASIS7_CI_RUN_RUST_BASELINE:-}" "disabled_by_scope_planner" run_standalone_tool_lockfiles_checks'

workflow="$repo_root/.github/workflows/rust.yml"
for job in windows-package-rollout-behavior testnet-packages-macos-arm64-contract public-testnet-fleet-health-contract; do
  if ! awk -v job="$job" '
    $0 ~ "^  " job ":" { active=1 }
    active && !($0 ~ "^  " job ":") && /^  [A-Za-z0-9_-]+:/ { active=0 }
    active { body=body $0 "\n" }
    END {
      event_scoped=(body ~ /github.event_name == .pull_request./ && body ~ /workflow_dispatch/)
      planner_scoped=(body ~ /needs.required-gate.outputs.run_operational_contracts == .true./ || body ~ /needs.required-gate.outputs.run_packaging_contracts == .true./)
      exit(event_scoped && planner_scoped ? 0 : 1)
    }
  ' "$workflow"; then
    echo "operational PR job is not planner-scoped: $job" >&2
    exit 1
  fi
done

for step in \
  "Prepare exact manual integration from trusted default workflow" \
  "Verify fleet-health collection contract"; do
  if ! awk -v step="$step" '
    $0 == "  public-testnet-fleet-health-contract:" { in_job=1; next }
    in_job && /^  [A-Za-z0-9_-]+:/ { exit }
    in_job && ($0 == "      - name: " step || $0 == "        name: " step) {
      in_step=1
      next
    }
    in_step && /^      - / { in_step=0 }
    in_step && $0 == "        shell: bash" { has_bash_shell=1 }
    in_step && $0 == "        run: |" { has_run=1 }
    END { exit(has_bash_shell && has_run ? 0 : 1) }
  ' "$workflow"; then
    echo "fleet-health step must explicitly select Bash for its Bash run block: $step" >&2
    exit 1
  fi
done

echo "ci required scope audit contract: passed"
