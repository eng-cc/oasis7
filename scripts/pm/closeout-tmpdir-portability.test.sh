#!/usr/bin/env bash
# This fixture must remain compatible with POSIX and Git Bash with native Windows Python.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
mkdir -p "$ROOT_DIR/.pm/scratch"
TEST_ROOT="$(mktemp -d "$ROOT_DIR/.pm/scratch/closeout-tmpdir-test.XXXXXX")"
trap 'rm -rf "$TEST_ROOT"' EXIT

TMPDIR_ASSERT='case "$(uname -s)" in MINGW*|MSYS*|CYGWIN*) [[ "${TMPDIR:-}" == [A-Za-z]:/* ]] ;; *) [[ -z "${TMPDIR:-}" ]] ;; esac'
env -u TMPDIR OASIS7_ALLOW_FIXTURE_VERIFICATION_PROFILE=1 \
  "$ROOT_DIR/scripts/pm/claim-ready.sh" \
    --claim-type tests_passed \
    --verify-command "$TMPDIR_ASSERT" \
    --json >"$TEST_ROOT/claim-ready.json"
python3 - "$TEST_ROOT/claim-ready.json" <<'PY'
import json,sys
result=json.load(open(sys.argv[1],encoding='utf-8'))
assert result['status']=='verified' and result['allowed_to_claim'] is True,result
PY

FIXTURE="$TEST_ROOT/task-closeout"
mkdir -p "$FIXTURE/scripts/pm" "$FIXTURE/.pm/github-project-sync"
cp "$ROOT_DIR/scripts/pm/task-closeout.sh" "$FIXTURE/scripts/pm/task-closeout.sh"
cat >"$FIXTURE/scripts/pm/claim-ready.sh" <<EOF
#!/usr/bin/env bash
$TMPDIR_ASSERT
printf '{"claim_type":"task_complete","status":"verified","allowed_to_claim":true,"verification_exit_code":0}'
EOF
cat >"$FIXTURE/scripts/pm/github-project-workflow.sh" <<'EOF'
#!/usr/bin/env bash
printf '{"status":"ok","errors":[],"warnings":[],"selected_task":{"task_uid":"task_11111111111111111111111111111111","target":"done","workflow_phase":"task_done"}}'
EOF
cat >"$FIXTURE/scripts/pm/github-project-task.py" <<'PY'
#!/usr/bin/env python3
import json
print(json.dumps({'task_uid':'task_11111111111111111111111111111111','status':'done','issue_url':'https://example.invalid/1'}))
PY
chmod +x "$FIXTURE/scripts/pm/"*
cat >"$FIXTURE/.pm/github-project-sync/tasks.json" <<'EOF'
{"version":1,"tasks":{"task_11111111111111111111111111111111":{"completion_mode":"non_pr_task","non_pr_completion_evidence":"persisted fixture truth"}}}
EOF
env -u TMPDIR PM_ROOT_DIR="$FIXTURE" \
  "$FIXTURE/scripts/pm/task-closeout.sh" --role tpm \
    --task-uid task_11111111111111111111111111111111 \
    --to-status done --claim-type task_complete \
    --verification-profile codex_subagent_role_fit \
    --json >"$TEST_ROOT/task-closeout.json"
python3 - "$TEST_ROOT/task-closeout.json" <<'PY'
import json,sys
result=json.load(open(sys.argv[1],encoding='utf-8'))
assert result['target_status']=='done',result
PY

python3 - "$ROOT_DIR/scripts/pm/claim-ready.sh" "$ROOT_DIR/scripts/pm/task-closeout.sh" <<'PY'
import pathlib,re,sys
for name in sys.argv[1:]:
    source=pathlib.Path(name).read_text(encoding='utf-8')
    preamble=source.split('SCRIPT_DIR=',1)[0]
    assert 'MSYS*|MINGW*|CYGWIN*' in preamble,preamble
    assert 'export TMPDIR' in preamble,preamble
    assert not re.search(r'^\s*\*\)',preamble,re.M),preamble
PY

echo "closeout-tmpdir-portability.test: OK"
