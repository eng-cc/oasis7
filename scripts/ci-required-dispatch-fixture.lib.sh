#!/usr/bin/env bash
set -euo pipefail

ci_fixture_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
ci_fixture_work=""
ci_fixture_repo=""
ci_fixture_bin=""
ci_fixture_log=""

ci_fixture_old_selectors=(
  OASIS7_CI_RUN_OASIS7_REQUIRED_TESTS
  OASIS7_CI_RUN_CONSENSUS_TESTS
  OASIS7_CI_RUN_DISTFS_TESTS
  OASIS7_CI_RUN_OASIS7_NODE_TESTS
  OASIS7_CI_RUN_OASIS7_NET_TESTS
  OASIS7_CI_RUN_OASIS7_NET_LIBP2P_TESTS
  OASIS7_CI_RUN_VIEWER_CONTRACT_TESTS
  OASIS7_CI_RUN_VIEWER_WASM_CHECK
  OASIS7_CI_RUN_VIEWER_PERF_SMOKE
  OASIS7_CI_RUN_PIXEL_WORLD_BRIDGE_LIB_TESTS
  OASIS7_CI_RUN_PIXEL_WORLD_BRIDGE_WASM_CHECK
  OASIS7_CI_RUN_LAUNCHER_WEB_BUILD
  OASIS7_CI_RUN_WORKSPACE_SUPPORT_CRATE_TESTS
  OASIS7_CI_RUN_SCENARIO_REGRESSION
  OASIS7_CI_RUN_OPERATIONAL_CONTRACTS
  OASIS7_CI_RUN_SITE_CONTRACT_TESTS
  OASIS7_CI_RUN_CODEX_AGENT_CONFIG_VALIDATION
  OASIS7_CI_RUN_COMPILE_METRICS_CONTRACT_TESTS
  OASIS7_CI_RUN_RUST_BASELINE
)
ci_fixture_new_selectors=(
  OASIS7_CI_RUN_WORKFLOW_GOVERNANCE_CONTRACTS
  OASIS7_CI_RUN_PACKAGING_CONTRACTS
  OASIS7_CI_RUN_DOC_CHECKER_CONTRACTS
  OASIS7_CI_RUN_CARGO_TOOLING_CONTRACTS
)
ci_fixture_resources=(
  OASIS7_CI_NEEDS_PYTHON
  OASIS7_CI_NEEDS_MARKDOWN
  OASIS7_CI_NEEDS_RUST_TOOLCHAIN
  OASIS7_CI_NEEDS_NODE
  OASIS7_CI_NEEDS_SYSTEM_DEPS
  OASIS7_CI_NEEDS_TRUNK
  OASIS7_CI_NEEDS_WASM_TARGET
)

ci_fixture_init() {
  ci_fixture_work=$(mktemp -d "${TMPDIR:-/tmp}/oasis7-ci-dispatch.XXXXXX")
  ci_fixture_repo="$ci_fixture_work/repo"
  ci_fixture_bin="$ci_fixture_work/bin"
  ci_fixture_log="$ci_fixture_work/commands.log"
  mkdir -p "$ci_fixture_repo/scripts" "$ci_fixture_bin"

  while IFS= read -r script; do
    [[ -n "$script" ]] || continue
    local target="$ci_fixture_repo/$script"
    mkdir -p "$(dirname "$target")"
    cat >"$target" <<'STUB'
#!/usr/bin/env bash
printf 'SCRIPT:%s\n' "$0" >>"${CAPTURE_LOG:?}"
if [[ "${CI_FIXTURE_FAIL_SCRIPT:-}" == "${0##*/}" ]]; then
  printf 'INJECTED_FAILURE:%s\n' "${0##*/}" >>"${CAPTURE_LOG:?}"
  exit 37
fi
exit 0
STUB
    chmod +x "$target"
  done < <(grep -oE '\./scripts/[[:alnum:]_./-]+\.sh' "$ci_fixture_root/scripts/ci-tests.sh" | sort -u)

  for command_name in bash python3 cargo rustup npm node trunk gh curl wget ssh scp sudo apt-get; do
    cat >"$ci_fixture_bin/$command_name" <<'STUB'
#!/bin/sh
printf 'TOOL:%s:%s\n' "${0##*/}" "$*" >>"${CAPTURE_LOG:?}"
if [ "${CI_FIXTURE_FAIL_SCRIPT:-}" = "${1##*/}" ]; then
  printf 'INJECTED_FAILURE:%s\n' "${1##*/}" >>"${CAPTURE_LOG:?}"
  exit 37
fi
exit 0
STUB
    chmod +x "$ci_fixture_bin/$command_name"
  done

  mkdir -p "$ci_fixture_repo/crates/oasis7_client_launcher"
  local viewer="$ci_fixture_repo/crates/oasis7_viewer"
  mkdir -p "$viewer/node_modules/.bin"
  touch "$viewer/package.json" "$viewer/package-lock.json" "$viewer/node_modules/.package-lock.json"
  touch "$viewer/node_modules/.bin/vite" "$viewer/node_modules/.bin/vitest"
  chmod +x "$viewer/node_modules/.bin/vite" "$viewer/node_modules/.bin/vitest"
}

ci_fixture_run() {
  local tier="$1"
  local contract_mode="$2"
  shift 2
  local -a env_args=("PATH=$ci_fixture_bin:$PATH" "CAPTURE_LOG=$ci_fixture_log")
  local variable override override_name override_value index replaced

  for variable in "${ci_fixture_old_selectors[@]}"; do
    env_args+=("$variable=false")
  done
  env_args+=(OASIS7_CI_RUN_HOSTED_ACCOUNT_SMOKE=false OASIS7_CI_RUN_PROVIDER_LIVE_GATE=false)

  case "$contract_mode" in
    strict|strict-incomplete|full)
      env_args+=(OASIS7_CI_EXECUTION_CONTRACT=required-domain-split/v1)
      for variable in "${ci_fixture_new_selectors[@]}"; do
        if [[ "$contract_mode" == strict-incomplete && "$variable" == OASIS7_CI_RUN_CARGO_TOOLING_CONTRACTS ]]; then
          continue
        fi
        env_args+=("$variable=false")
      done
      for variable in "${ci_fixture_resources[@]}"; do
        if [[ "$variable" == OASIS7_CI_NEEDS_PYTHON || "$variable" == OASIS7_CI_NEEDS_MARKDOWN ]]; then
          env_args+=("$variable=true")
        else
          env_args+=("$variable=false")
        fi
      done
      ;;
    legacy|legacy-mixed)
      ;;
    *)
      echo "invalid fixture mode: $contract_mode" >&2
      return 2
      ;;
  esac

  for override in "$@"; do
    override_name="${override%%=*}"
    override_value="${override#*=}"
    [[ "$override_name" != "$override" ]] || {
      echo "fixture override must be NAME=value: $override" >&2
      return 2
    }
    replaced=false
    for index in "${!env_args[@]}"; do
      if [[ "${env_args[$index]}" == "$override_name="* ]]; then
        env_args[$index]="$override_name=$override_value"
        replaced=true
        break
      fi
    done
    if [[ "$replaced" != true ]]; then
      env_args+=("$override_name=$override_value")
    fi
  done
  : >"$ci_fixture_log"
  env -i "${env_args[@]}" /bin/bash "$ci_fixture_root/scripts/ci-tests.sh" "$tier" --repo-root "$ci_fixture_repo" >"$ci_fixture_work/output.log" 2>&1
}

ci_fixture_assert_has() {
  local expected="$1"
  if ! grep -Fq -- "$expected" "$ci_fixture_log"; then
    echo "dispatcher capture missing: $expected" >&2
    cat "$ci_fixture_work/output.log" >&2
    cat "$ci_fixture_log" >&2
    return 1
  fi
}

ci_fixture_assert_lacks() {
  local unexpected="$1"
  if grep -Fq -- "$unexpected" "$ci_fixture_log"; then
    echo "dispatcher capture unexpectedly ran: $unexpected" >&2
    cat "$ci_fixture_log" >&2
    return 1
  fi
}

ci_fixture_assert_inventory_selection_dispatched() {
  local selection="$1"
  local paths path
  local found_selection=false
  local -a path_items
  while IFS=$'\t' read -r _ paths selected _ _; do
    [[ "$selected" == "$selection" ]] || continue
    found_selection=true
    IFS=',' read -r -a path_items <<<"$paths"
    for path in "${path_items[@]}"; do
      if [[ "$path" == run_checker_stage_changed_path_contract_tests ]]; then
        if ! grep -Fq 'DISPATCH:run_checker_stage_changed_path_contract_tests' "$ci_fixture_work/output.log"; then
          echo "inventory helper was not dispatched for $selection: $path" >&2
          return 1
        fi
      elif [[ "$path" == scripts/* ]]; then
        if ! grep -Fq "$path" "$ci_fixture_work/output.log" && ! grep -Fq "$path" "$ci_fixture_log"; then
          echo "inventory test path was not dispatched for $selection: $path" >&2
          return 1
        fi
      elif [[ "$path" == run_* ]]; then
        echo "inventory contains an unhandled pseudo-path for $selection: $path" >&2
        return 1
      fi
    done
  done <"$ci_fixture_root/scripts/ci-required-capability-test-inventory.tsv"
  if [[ "$found_selection" != true ]]; then
    echo "inventory has no test-call row for selection $selection" >&2
    return 1
  fi
}

ci_fixture_cleanup() {
  [[ -z "$ci_fixture_work" ]] || rm -rf "$ci_fixture_work"
}
