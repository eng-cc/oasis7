#!/usr/bin/env bash
# This fixture must remain compatible with POSIX and Git Bash with native Windows Python.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
python3 - "$SCRIPT_DIR/workflow-behavior-eval.sh" <<'PY'
import pathlib,sys

source=pathlib.Path(sys.argv[1]).read_text(encoding='utf-8')
execution,results=source.split('RESULT_JSON=',1)
required_commands=(
    './scripts/pm/closeout-tmpdir-portability.test.sh',
    './scripts/pm/claim-ready-ready-pr.test.sh',
)
for command in required_commands:
    invocation=f'"$ROOT_DIR/{command[2:]}"'
    assert invocation in execution,f'workflow behavior eval does not execute {command}'
assert '"id": "completion_claim_gate"' in results
assert './scripts/pm/claim-ready.test.sh && ./scripts/pm/claim-ready-ready-pr.test.sh' in results
assert '"ready_pr_revalidation": "passed"' in results
assert '"id": "closeout_tmpdir_portability"' in results
assert '"command": "./scripts/pm/closeout-tmpdir-portability.test.sh"' in results
assert '"windows_native_tmpdir": "passed"' in results
assert '"posix_tmpdir_preserved": "passed"' in results
PY

echo "workflow-behavior-portability-contract.test: OK"
