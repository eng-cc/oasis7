#!/usr/bin/env bash
# This fixture must remain compatible with POSIX and Git Bash with native Windows Python.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
python3 - "$SCRIPT_DIR/claim-ready.sh" "$SCRIPT_DIR/ci-ready-receipt.py" <<'PY'
import importlib.util,pathlib,re,sys

claim_source=pathlib.Path(sys.argv[1]).read_text(encoding='utf-8')
ci_path=pathlib.Path(sys.argv[2])
sys.path.insert(0,str(ci_path.parent))
spec=importlib.util.spec_from_file_location('ci_ready_receipt',ci_path)
module=importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

def exercise_live(draft,allow_ready):
    def fake_gh(*args):
        endpoint=args[1]
        if endpoint.endswith('/pulls/7'):
            return {'draft':draft,'state':'open','merged':False,
                    'body':'Task: task_11111111111111111111111111111111\nRefs #42',
                    'head':{'sha':'a'*40},'base':{'sha':'b'*40}}
        if '/check-runs?' in endpoint:
            return {'check_runs':[{'id':9,'name':'required-gate','status':'completed',
                    'conclusion':'success','completed_at':'2026-01-01T00:00:00Z',
                    'head_sha':'a'*40,
                    'pull_requests':[{'number':7,'base':{'sha':'b'*40},
                                      'head':{'sha':'a'*40}}],
                    'app':{'id':15368}}]}
        raise AssertionError(args)
    original=module.gh
    module.gh=fake_gh
    try:
        return module.live('eng-cc/oasis7','task_11111111111111111111111111111111',42,7,
                           'required-gate','15368',allow_ready)
    finally:
        module.gh=original

assert exercise_live(True,False)[3]=='a'*40
assert exercise_live(False,True)[3]=='a'*40
try:
    exercise_live(False,False)
except SystemExit as exc:
    assert 'not a draft candidate' in str(exc),exc
else:
    raise AssertionError('promoted-ready PR must require explicit allow-ready-pr revalidation')

match=re.search(r'python3 "\$SCRIPT_DIR/ci-ready-receipt\.py"(.*?)>/dev/null',claim_source,re.S)
assert match,'missing claim-ready CI receipt live-revalidation call'
call=match.group(1)
required={
    '--repository': '$RECEIPT_REPOSITORY',
    '--task-uid': '$RECEIPT_TASK_UID',
    '--task-issue-number': '$RECEIPT_ISSUE',
    '--pr-number': '$RECEIPT_PR',
    '--check-name': '$RECEIPT_CHECK',
    '--check-app-id': '$RECEIPT_APP',
    '--planner-digest': '$RECEIPT_PLANNER',
    '--receipt': '$CI_READY_RECEIPT',
}
for flag,value in required.items():
    assert re.search(re.escape(flag)+r'\s+"?'+re.escape(value)+r'"?',call),(flag,call)
assert '--allow-ready-pr' in call,call

ci_source=ci_path.read_text(encoding='utf-8')
assert 'for key,val in live_identity.items()' in ci_source
assert 'old.get(key)!=val' in ci_source
assert 'not 0 <= (dt.datetime.now(dt.timezone.utc)-seen).total_seconds() <= 600' in ci_source
PY

echo "claim-ready-ready-pr.test: OK"
