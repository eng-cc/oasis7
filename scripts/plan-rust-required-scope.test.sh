#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

plan_for_path() {
  "$ROOT_DIR/scripts/plan-rust-required-scope.sh" \
    --event-name pull_request \
    --changed-path "$1"
}

plan_for_paths() {
  local args=(--event-name pull_request)
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

product_doc_output="$(plan_for_path doc/product/world-rules-core-gameplay.prd.md)"
assert_key_equals "$product_doc_output" scope minimal
assert_key_equals "$product_doc_output" run_rust_baseline false
assert_key_equals "$product_doc_output" needs_rust_toolchain false
assert_key_equals "$product_doc_output" needs_node false
assert_key_matches "$product_doc_output" planner_config_sha256 '^sha256:[0-9a-f]{64}$'
assert_reason_contains "$product_doc_output" "governance_doc:doc/product/world-rules-core-gameplay.prd.md"

launcher_output="$(plan_for_path crates/oasis7_client_launcher/src/lib.rs)"
assert_key_equals "$launcher_output" needs_node true
assert_key_equals "$launcher_output" needs_trunk true
assert_key_equals "$launcher_output" needs_rust_toolchain true

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

governance_helper_output="$(plan_for_paths \
  scripts/prepare-task-pr.sh \
  scripts/pm/patch-equivalence-receipt.sh \
  scripts/pm/patch-equivalence-receipt.test.sh \
  scripts/pm/prepare-task-pr-review-risk.test.py \
  scripts/prepare-task-pr.test.sh \
  scripts/plan-rust-required-scope.test.sh)"
assert_key_equals "$governance_helper_output" scope minimal
assert_key_equals "$governance_helper_output" run_rust_baseline false
assert_key_equals "$governance_helper_output" needs_rust_toolchain false
assert_key_equals "$governance_helper_output" needs_node false
assert_reason_contains "$governance_helper_output" "governance_script:scripts/prepare-task-pr.sh"
assert_reason_contains "$governance_helper_output" "governance_script:scripts/pm/patch-equivalence-receipt.sh"
assert_reason_contains "$governance_helper_output" "governance_script:scripts/pm/patch-equivalence-receipt.test.sh"
assert_reason_contains "$governance_helper_output" "governance_script:scripts/prepare-task-pr.test.sh"
assert_reason_contains "$governance_helper_output" "governance_script:scripts/plan-rust-required-scope.test.sh"
assert_reason_absent "$governance_helper_output" "unclassified_or_unresolvable:"

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
  assert_key_equals "$workflow_contract_output" run_operational_contracts true
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
assert_key_equals "$receipt_governance_output" run_operational_contracts true
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
assert_key_equals "$pm_gameplay_union_output" run_operational_contracts true
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
assert_key_equals "$workflow_governance_output" run_operational_contracts true
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
  scripts/viewer-pixel-world-fragment-visual-smoke.sh)"
assert_key_equals "$viewer_web_wrapper_output" scope targeted
assert_key_equals "$viewer_web_wrapper_output" run_viewer_contract_tests true
assert_key_equals "$viewer_web_wrapper_output" run_viewer_wasm_check true
assert_key_equals "$viewer_web_wrapper_output" run_launcher_web_build false
assert_key_equals "$viewer_web_wrapper_output" run_oasis7_required_tests false
assert_reason_contains "$viewer_web_wrapper_output" "viewer_web_wrapper:scripts/build-viewer-software-safe.sh"
assert_reason_contains "$viewer_web_wrapper_output" "viewer_web_wrapper:scripts/viewer-dependency-preflight.sh"
assert_reason_contains "$viewer_web_wrapper_output" "viewer_web_wrapper:scripts/viewer-dependency-preflight.test.sh"
assert_reason_contains "$viewer_web_wrapper_output" "viewer_web_wrapper:scripts/viewer-pixel-world-fragment-visual-smoke.sh"
assert_reason_absent "$viewer_web_wrapper_output" "unclassified_or_unresolvable:"

viewer_launcher_wrapper_output="$(plan_for_paths \
  scripts/run-launcher-stack.sh \
  scripts/run-producer-playtest.sh \
  scripts/worktree-harness.sh \
  scripts/worktree-harness-contract.test.sh)"
assert_key_equals "$viewer_launcher_wrapper_output" scope targeted
assert_key_equals "$viewer_launcher_wrapper_output" run_viewer_contract_tests true
assert_key_equals "$viewer_launcher_wrapper_output" run_viewer_wasm_check true
assert_key_equals "$viewer_launcher_wrapper_output" run_launcher_web_build true
assert_key_equals "$viewer_launcher_wrapper_output" needs_trunk true
assert_reason_contains "$viewer_launcher_wrapper_output" "viewer_launcher_wrapper:scripts/run-launcher-stack.sh"
assert_reason_contains "$viewer_launcher_wrapper_output" "viewer_launcher_wrapper:scripts/run-producer-playtest.sh"
assert_reason_contains "$viewer_launcher_wrapper_output" "viewer_launcher_wrapper:scripts/worktree-harness.sh"
assert_reason_contains "$viewer_launcher_wrapper_output" "viewer_launcher_wrapper:scripts/worktree-harness-contract.test.sh"
assert_reason_absent "$viewer_launcher_wrapper_output" "unclassified_or_unresolvable:"

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

invalid_config="$(mktemp)"
trap 'rm -f "$invalid_config"' EXIT
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

wasm_build_output="$(plan_for_path crates/oasis7_wasm_build/src/lib.rs)"
assert_key_equals "$wasm_build_output" scope targeted
assert_key_equals "$wasm_build_output" run_oasis7_workspace_support_crate_tests true
assert_key_equals "$wasm_build_output" run_launcher_web_build false
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

echo "plan-rust-required-scope.test: OK"
