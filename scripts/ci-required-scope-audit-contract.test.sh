#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
planner="$repo_root/scripts/plan-rust-required-scope.sh"
ci_tests="$repo_root/scripts/ci-tests.sh"

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

packaging_plan="$($planner --event-name pull_request \
  --changed-path scripts/package-native-installer.sh \
  --changed-path scripts/validate-release-platform-entrypoints.sh \
  --changed-path scripts/package-viewer-web-delivery.sh \
  --changed-path scripts/packaging-artifact-size-contract.test.sh \
  --changed-path scripts/copy-viewer-web-dist.test.sh \
  --changed-path scripts/native-packaging-contract.test.sh)"
require_key "$packaging_plan" scope targeted
require_key "$packaging_plan" selected_capabilities packaging_contracts
require_key "$packaging_plan" run_operational_contracts true
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
  scripts/native-packaging-contract.test.sh; do
  require_reason_contains "$packaging_plan" "packaging_contracts:$packaging_path"
done
require_reason_contains "$packaging_plan" "required_gate_baseline:always_on"

release_packaging_plan="$($planner --event-name pull_request \
  --changed-path .github/workflows/release-packages.yml \
  --changed-path scripts/build-game-launcher-bundle.sh)"
require_key "$release_packaging_plan" scope full
require_key "$release_packaging_plan" run_rust_baseline true
require_key "$release_packaging_plan" needs_rust_toolchain true

operational_plan="$("$planner" --event-name pull_request --changed-path scripts/p2p-public-testnet-package-rollout.test.sh)"
require_key "$operational_plan" run_required_gate_baseline true
require_key "$operational_plan" run_operational_contracts true
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

operational_contracts_source="$(sed -n '/^run_operational_contract_tests() {/,/^}/p' "$ci_tests")"
if ! grep -Fqx '  run python3 ./scripts/pm/ci-ready-receipt.test.py' <<<"$operational_contracts_source"; then
  echo "workflow-governance receipt contract is not wired into run_operational_contract_tests" >&2
  exit 1
fi
if ! grep -Fqx '  run ./scripts/ci-required-scope-audit-contract.test.sh' <<<"$operational_contracts_source"; then
  echo "required scope audit contract is not wired into run_operational_contract_tests" >&2
  exit 1
fi

if ! grep -Fqx '    run_required_component "site quality contracts" "${OASIS7_CI_RUN_SITE_CONTRACT_TESTS:-}" "disabled_by_scope_planner" run_site_contract_tests' "$ci_tests"; then
  echo "site quality contracts are not wired to the planner selector in ci-tests" >&2
  exit 1
fi

packaging_runner_source="$(sed -n '/^run_packaging_contract_tests() {/,/^}/p' "$ci_tests")"
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
if ! grep -Fqx '  run_packaging_contract_tests' <<<"$(sed -n '/^run_operational_contract_tests() {/,/^}/p' "$ci_tests")"; then
  echo "operational contract runner must include the focused packaging runner" >&2
  exit 1
fi

# The focused runner must stay a non-Rust boundary even when its fixture
# scripts inspect Rust-producing workflows as data.
if grep -Eiq '(^|[[:space:];|&()])(cargo|rustup)([[:space:]]|$)' <<<"$packaging_runner_source" || \
   grep -Eiq '(^|[[:space:];|&()])run_cargo([[:space:]]|$)' <<<"$packaging_runner_source"; then
  echo "packaging contract runner must not invoke Cargo or rustup directly" >&2
  exit 1
fi

python3 - \
  "$repo_root/scripts/ci-required-scope.v2.json" \
  "$ci_tests" \
  "$repo_root/.github/workflows/rust.yml" \
  "$planner" <<'PY'
import json
import re
import subprocess
import sys
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
if inventory != set(declared):
    raise SystemExit(
        "selector ownership drift: "
        f"missing={sorted(inventory - set(declared))}, "
        f"stale={sorted(set(declared) - inventory)}"
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
        if name in workflow_text:
            raise SystemExit(
                f"manual-only selector is unexpectedly auto-wired in workflow: {name}"
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
if grep -Eiq '(^|[[:space:];|&()])(cargo|rustup)([[:space:]]|$)' <<<"$operational_contracts_source" || \
   grep -Eiq '(^|[[:space:];|&()])run_cargo([[:space:]]|$)' <<<"$operational_contracts_source"; then
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
    $0 ~ "^  " job ":" { active=1; next }
    active && /^  [A-Za-z0-9_-]+:/ { exit }
    active && /if: github.event_name == .pull_request. && needs.required-gate.outputs.run_operational_contracts == .true./ { found=1 }
    END { exit(found ? 0 : 1) }
  ' "$workflow"; then
    echo "operational PR job is not planner-scoped: $job" >&2
    exit 1
  fi
done

echo "ci required scope audit contract: passed"
