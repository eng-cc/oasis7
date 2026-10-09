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

if grep -Fq '  push:' "$pages_workflow"; then
  echo "Pages workflow must not deploy automatically from a push" >&2
  exit 1
fi
if ! grep -Fq '  workflow_dispatch:' "$pages_workflow"; then
  echo "Pages workflow must remain manually dispatchable" >&2
  exit 1
fi

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

python3 - "$ci_tests" "$required_workflow" "$repo_root/scripts/ci-required-scope.json" <<'PYTHON'
import json
from pathlib import Path
import re
import sys

driver, workflow = (Path(path).read_text() for path in sys.argv[1:3])
config = json.loads(Path(sys.argv[3]).read_text())
if not re.search(r'^\s*site_quality\)\s+run_site_contract_tests\s*;;', driver, re.M):
    raise SystemExit("site quality group does not execute its product checks")
jobs = dict(re.findall(r'^  ([a-z][a-z0-9-]*):\n(.*?)(?=^  [a-z][a-z0-9-]*:|\Z)',
                       workflow.split('\njobs:\n', 1)[1], re.M | re.S))
job = jobs.get('site-quality', '')
for expected in ('needs: select', "if: needs.select.outputs.run_site_quality == 'true'",
                 'fromJSON(needs.select.outputs.matrix_site_quality)', '--group "${{ matrix.group }}"'):
    if expected not in job:
        raise SystemExit("site quality job is missing selected group execution: " + expected)
gate = jobs.get('required-gate', '')
needs = re.search(r'    needs: \[(.*)\]', gate)
if not needs or 'site-quality' not in needs.group(1).split(', '):
    raise SystemExit("final required-gate omits site quality results")
if 'site_quality' not in config['groups'] or not any(
        'site_quality' in rule.get('groups', []) and 'site/**' in rule['match']
        for rule in config['rules']):
    raise SystemExit("site directory changes do not select the site quality group")
PYTHON

echo "site checks contract: passed"
