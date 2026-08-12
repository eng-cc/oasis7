#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
ci_tests="$repo_root/scripts/ci-tests.sh"

require_line() {
  local expected="$1"
  if ! grep -Fqx "$expected" "$ci_tests"; then
    echo "missing ci-tests Codex agent-config required contract: $expected" >&2
    exit 1
  fi
}

require_line '  run ./scripts/pm/validate-codex-agent-config.test.sh'
require_line '  run ./scripts/pm/codex-role-fit-task-binding.test.sh'
require_line '    run_required_component "Codex agent-config validation" "${OASIS7_CI_RUN_CODEX_AGENT_CONFIG_VALIDATION:-}" "disabled_by_scope_planner" run_codex_agent_config_validation'

echo "ci-tests Codex agent-config required contract: passed"
