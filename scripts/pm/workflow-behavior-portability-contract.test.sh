#!/usr/bin/env bash
# This fixture must remain compatible with POSIX and Git Bash with native Windows Python.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
test "$(git -C "$ROOT_DIR" ls-files -s -- scripts/pm/post-merge-main-sync-default-cache-recovery.test.sh | awk '{print $1}')" = "100755"
python3 - "$SCRIPT_DIR/workflow-behavior-eval.sh" <<'PY'
import pathlib
import subprocess
import sys
import tempfile

source=pathlib.Path(sys.argv[1]).read_text(encoding='utf-8')
execution,results=source.split('RESULT_JSON=',1)
closeout=pathlib.Path(sys.argv[1]).with_name('closeout-tmpdir-portability.test.sh').read_text(encoding='utf-8')
assert 'OASIS7_WORKFLOW_EVAL_SCRATCH="$TMP_DIR/pm-scratch"' in execution, \
    'workflow behavior eval must allocate its scratch outside the repository projection'
assert 'export OASIS7_WORKFLOW_EVAL_SCRATCH' in execution, \
    'workflow behavior eval must export its isolated scratch root to child fixtures'
assert 'OASIS7_WORKFLOW_EVAL_SCRATCH:-$ROOT_DIR/.pm/scratch' in closeout, \
    'closeout portability fixture must consume the eval-owned scratch root'
guard = pathlib.Path(sys.argv[1]).with_name('guard-tracked-files.py')
with tempfile.TemporaryDirectory(prefix='oasis7-workflow-projection-') as tmp:
    repo = pathlib.Path(tmp) / 'repo'
    state = pathlib.Path(tmp) / 'state'
    eval_scratch = pathlib.Path(tmp) / 'eval-scratch'
    repo.mkdir()
    subprocess.run(['git', 'init', '-q', str(repo)], check=True)
    subprocess.run([
        sys.executable, str(guard), 'snapshot', '--root', str(repo),
        '--state', str(state), '--pathspec', '.pm',
    ], check=True)
    eval_scratch.mkdir()
    isolated = subprocess.run([
        sys.executable, str(guard), 'check', '--root', str(repo),
        '--state', str(state), '--pathspec', '.pm',
    ], text=True, capture_output=True, check=False)
    assert isolated.returncode == 0, isolated.stderr
    (repo / '.pm' / 'scratch').mkdir(parents=True)
    leaked = subprocess.run([
        sys.executable, str(guard), 'check', '--root', str(repo),
        '--state', str(state), '--pathspec', '.pm',
    ], text=True, capture_output=True, check=False)
    assert leaked.returncode != 0, leaked.stdout
    assert 'new filesystem projection path: .pm/scratch' in leaked.stderr, leaked.stderr
required_commands=(
    './scripts/pm/closeout-tmpdir-portability.test.sh',
    './scripts/pm/claim-ready-ready-pr.test.sh',
    './scripts/pm/post-merge-main-sync-default-cache-recovery.test.sh',
    './scripts/pm/recover-terminal-task-mapping.test.py',
)
for command in required_commands:
    invocation=(f'python3 "$ROOT_DIR/{command[2:]}"' if command.endswith('.py')
                else f'"$ROOT_DIR/{command[2:]}"')
    assert invocation in execution,f'workflow behavior eval does not execute {command}'
assert '"id": "completion_claim_gate"' in results
assert './scripts/pm/claim-ready.test.sh && ./scripts/pm/claim-ready-ready-pr.test.sh' in results
assert '"ready_pr_revalidation": "passed"' in results
assert '"id": "closeout_tmpdir_portability"' in results
assert '"command": "./scripts/pm/closeout-tmpdir-portability.test.sh"' in results
assert '"windows_native_tmpdir": "passed"' in results
assert '"posix_tmpdir_preserved": "passed"' in results
assert '"id": "terminal_default_cache_recovery"' in results
assert './scripts/pm/post-merge-main-sync-default-cache-recovery.test.sh && python3 ./scripts/pm/recover-terminal-task-mapping.test.py' in results
assert '"registered_canonical_worktree_import": "passed"' in results
assert '"atomic_default_mapping_update": "passed"' in results
assert '"conflict_and_identity_rejection": "passed"' in results
PY

echo "workflow-behavior-portability-contract.test: OK"
