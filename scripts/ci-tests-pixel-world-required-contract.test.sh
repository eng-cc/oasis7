#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
ci_tests="$repo_root/scripts/ci-tests.sh"

require_line() {
  local expected="$1"
  if ! grep -Fqx "$expected" "$ci_tests"; then
    echo "missing ci-tests pixel-world required contract: $expected" >&2
    exit 1
  fi
}

require_line '  run_cargo test -p pixel_world_bridge --lib'
require_line '  run_cargo check -p pixel_world_bridge --target wasm32-unknown-unknown'
require_line '    run_required_component "pixel world bridge lib tests" "${OASIS7_CI_RUN_PIXEL_WORLD_BRIDGE_LIB_TESTS:-}" "disabled_by_scope_planner" run_pixel_world_bridge_lib_tests'
require_line '    run_required_component "pixel world bridge wasm check" "${OASIS7_CI_RUN_PIXEL_WORLD_BRIDGE_WASM_CHECK:-}" "disabled_by_scope_planner" run_pixel_world_bridge_wasm_check'

echo "ci-tests pixel-world required contract: passed"
