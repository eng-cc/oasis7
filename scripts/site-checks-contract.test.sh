#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
pages_workflow="$repo_root/.github/workflows/pages.yml"
required_workflow="$repo_root/.github/workflows/rust.yml"
ci_tests="$repo_root/scripts/ci-tests.sh"

site_checks=(
  scripts/site-link-check.sh
  scripts/site-homepage-claim-check.sh
  scripts/site-manual-sync-check.sh
  scripts/site-download-check.sh
)

for check in "${site_checks[@]}"; do
  if [[ ! -x "$repo_root/$check" ]]; then
    echo "site quality check is missing or not executable: $check" >&2
    exit 1
  fi
  if ! grep -Fq "./$check" "$pages_workflow"; then
    echo "Pages workflow does not run site quality check: $check" >&2
    exit 1
  fi
  if ! grep -Fq "  run ./$check" "$ci_tests"; then
    echo "ci-tests site selector does not run site quality check: $check" >&2
    exit 1
  fi
done

for path in \
  'site/**' \
  'scripts/site-link-check.sh' \
  'scripts/site-homepage-claim-check.sh' \
  'scripts/site-manual-sync-check.sh' \
  'scripts/site-download-check.sh' \
  '.github/workflows/pages.yml'; do
  if ! grep -Fq "      - \"$path\"" "$pages_workflow"; then
    echo "Pages workflow is missing its site quality path trigger: $path" >&2
    exit 1
  fi
done

if ! grep -Fq 'run_site_contract_tests: ${{ steps.scope.outputs.run_site_contract_tests }}' "$required_workflow"; then
  echo "required-gate does not expose the site quality planner selector" >&2
  exit 1
fi
if ! grep -Fq 'OASIS7_CI_RUN_SITE_CONTRACT_TESTS: ${{ steps.scope.outputs.run_site_contract_tests }}' "$required_workflow"; then
  echo "required-gate does not pass the site quality planner selector to ci-tests" >&2
  exit 1
fi
if ! grep -Fq 'run_required_component "site quality contracts"' "$ci_tests"; then
  echo "ci-tests does not consume the site quality planner selector" >&2
  exit 1
fi

echo "site checks contract: passed"
