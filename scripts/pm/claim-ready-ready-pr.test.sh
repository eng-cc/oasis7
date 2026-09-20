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

# A high-risk v2 projection must reject an ordinary source-bound receipt at the
# direct claim-ready entrypoint. This uses a small isolated worktree and stubs
# only the live receipt/bootstrap readers so the claim path reaches the real
# v2 identity and projection classifier.
TEST_ROOT="$(mktemp -d)"
trap 'rm -rf -- "$TEST_ROOT"' EXIT
FIXTURE="$TEST_ROOT/fixture"
UID_VALUE="task_11111111111111111111111111111111"
mkdir -p "$FIXTURE/scripts/pm" "$FIXTURE/.pm/scratch/$UID_VALUE/review-plans" "$FIXTURE/.pm/github-project-sync" "$TEST_ROOT/bin"
cp "$SCRIPT_DIR/claim-ready.sh" "$SCRIPT_DIR/ci_ready_receipt_identity.py" "$SCRIPT_DIR/repo-state-fingerprint.py" "$FIXTURE/scripts/pm/"
python3 - "$FIXTURE/scripts/pm/ci-ready-receipt.py" "$FIXTURE/scripts/pm/bootstrap-task-snapshot.py" <<'PY'
from pathlib import Path
import sys

for raw in sys.argv[1:]:
    Path(raw).write_text("#!/usr/bin/env python3\nraise SystemExit(0)\n", encoding="utf-8")
PY
chmod +x "$FIXTURE/scripts/pm/claim-ready.sh"
python3 - "$TEST_ROOT/bin/gh" <<'PY'
from pathlib import Path
import sys

Path(sys.argv[1]).write_text(
    "#!/usr/bin/env bash\n"
    "if [[ \"$1 $2\" == \"pr view\" ]]; then\n"
    "  printf '%s\\n' \"${TARGET_OID:?}\"\n"
    "  exit 0\n"
    "fi\n"
    "echo \"unexpected gh invocation: $*\" >&2\n"
    "exit 9\n",
    encoding="utf-8",
)
PY
chmod +x "$TEST_ROOT/bin/gh"
git -C "$FIXTURE" init -q -b main
git -C "$FIXTURE" config user.email test@example.com
git -C "$FIXTURE" config user.name Test
printf '/scripts/pm/\n/.pm/\n/receipt.json\n' >"$FIXTURE/.gitignore"
printf 'contract\n' >"$FIXTURE/contract.txt"
git -C "$FIXTURE" add .gitignore contract.txt
git -C "$FIXTURE" commit -qm scope
FIXTURE_SCOPE="$(git -C "$FIXTURE" rev-parse HEAD)"
printf 'feature\n' >"$FIXTURE/feature.txt"
git -C "$FIXTURE" add feature.txt
git -C "$FIXTURE" commit -qm source
FIXTURE_HEAD="$(git -C "$FIXTURE" rev-parse HEAD)"
git -C "$FIXTURE" branch target-related "$FIXTURE_SCOPE"
git -C "$FIXTURE" checkout -q target-related
printf 'related target change\n' >>"$FIXTURE/contract.txt"
git -C "$FIXTURE" add contract.txt
git -C "$FIXTURE" commit -qm related-target
RELATED_TARGET="$(git -C "$FIXTURE" rev-parse HEAD)"
git -C "$FIXTURE" checkout -q main
git -C "$FIXTURE" branch target-unrelated "$FIXTURE_SCOPE"
git -C "$FIXTURE" checkout -q target-unrelated
printf 'unrelated target change\n' >"$FIXTURE/unrelated.txt"
git -C "$FIXTURE" add unrelated.txt
git -C "$FIXTURE" commit -qm unrelated-target
UNRELATED_TARGET="$(git -C "$FIXTURE" rev-parse HEAD)"
git -C "$FIXTURE" checkout -q main
python3 - "$FIXTURE" "$FIXTURE_HEAD" "$UID_VALUE" "$FIXTURE_SCOPE" <<'PY'
import hashlib
import importlib.util
import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
head = sys.argv[2]
task_uid = sys.argv[3]
scope = sys.argv[4]
changed_paths = ["feature.txt"]
changed_paths_digest = "sha256:" + hashlib.sha256(
    json.dumps(changed_paths, sort_keys=True, separators=(",", ":")).encode()
).hexdigest()
spec = importlib.util.spec_from_file_location(
    "claim_ready_identity_fixture", root / "scripts/pm/ci_ready_receipt_identity.py"
)
identity = importlib.util.module_from_spec(spec)
spec.loader.exec_module(identity)
source = identity.source_review_identity(
    task_uid=task_uid,
    bootstrap_epoch=1,
    repository="eng-cc/oasis7",
    pr_number=7,
    source_head_oid=head,
    source_scope_oid=scope,
    changed_paths_digest=changed_paths_digest.removeprefix("sha256:"),
    ordered_role_ids=["repository_health_engineer"],
    role_contract_digest="b" * 64,
    review_policy_digest="c" * 64,
    input_contract_digest="d" * 64,
)
projection = {
    "schema": "oasis7-workflow-impact-projection/v2",
    "task_uid": task_uid,
    "source_head_oid": head,
    "scope_base_oid": scope,
    "changed_paths": changed_paths,
    "changed_paths_digest": changed_paths_digest,
    "closure_status": {"status": "complete"},
    "review_escalated": False,
    "verification_affected": False,
    "change_class": "mixed",
    "review_reasons": [],
    "public_semantics": [],
}
projection["projection_digest"] = "sha256:" + hashlib.sha256(
    json.dumps(
        {key: value for key, value in projection.items() if key != "projection_digest"},
        sort_keys=True,
        separators=(",", ":"),
    ).encode()
).hexdigest()
applicability = identity.review_applicability_identity(source)
plan = {
    "schema": "oasis7-review-plan/v2",
    "task_uid": task_uid,
    "source_review_identity": source,
    "source_review_digest": identity.source_review_digest(source),
    "impact_projection": projection,
    "impact_projection_schema": "oasis7-workflow-impact-projection/v2",
    "impact_projection_digest": projection["projection_digest"],
    "impact_projection_planner_digest": "sha256:" + "e" * 64,
    "effective_mode": {"effective_policy": "manual"},
    "professional_review_applicability": {
        "identity": applicability,
        "identity_digest": identity.review_applicability_digest(applicability),
        "verified": True,
    },
}
task_root = root / ".pm" / "scratch" / task_uid
(task_root / "review-plans" / "1.json").write_text(json.dumps(plan), encoding="utf-8")
(task_root / "bootstrap-task-snapshot.json").write_text(
    json.dumps({"request": {"identity": "fixture"}, "task": {"bootstrap_epoch": 1}}),
    encoding="utf-8",
)
(root / ".pm/github-project-sync/tasks.json").write_text(
    json.dumps({"project": {"repo": "eng-cc/oasis7"}, "tasks": {task_uid: {"status": "ready", "issue_number": 0}}}),
    encoding="utf-8",
)
receipt = {
    "receipt_type": "oasis7_ci_ready_receipt",
    "issuer": "github_live_query",
    "repository": "eng-cc/oasis7",
    "task_uid": task_uid,
    "task_issue_number": 0,
    "pr_number": 7,
    "base_oid": scope,
    "head_oid": head,
    "check_name": "required-gate",
    "check_app_id": "15368",
    "check_run_id": 1,
    "planner_digest": "f" * 64,
    "conclusion": "success",
    "ci_validation_mode": "ordinary_pr",
    "base_ref": "main",
    "live_validation": "ci-ready-receipt-live",
    "impact_projection_schema": "oasis7-workflow-impact-projection/v2",
    "impact_projection_digest": projection["projection_digest"],
    "impact_projection_planner_digest": "sha256:" + "e" * 64,
}
(root / "receipt.json").write_text(json.dumps(receipt), encoding="utf-8")
PY
set +e
TARGET_OID="$FIXTURE_HEAD" PATH="$TEST_ROOT/bin:$PATH" PM_ROOT_DIR="$FIXTURE" \
  "$FIXTURE/scripts/pm/claim-ready.sh" \
  --claim-type ready_for_pr \
  --verification-profile repository_required \
  --task-uid "$UID_VALUE" \
  --ci-ready-receipt "$FIXTURE/receipt.json" \
  --json >"$TEST_ROOT/claim.json" 2>"$TEST_ROOT/claim.err"
CLAIM_STATUS=$?
set -e
if [[ "$CLAIM_STATUS" == "0" ]]; then
  echo "claim-ready accepted ordinary CI for a high-risk v2 projection" >&2
  exit 1
fi
grep -F "high-risk projection requires trusted integration CI" "$TEST_ROOT/claim.err" >/dev/null

# The same direct path must reject a mapped related target-only change while
# allowing an unrelated target advance once the live target object is bound.
python3 - "$FIXTURE" "$UID_VALUE" <<'PY'
import hashlib
import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
task_uid = sys.argv[2]
task_root = root / ".pm" / "scratch" / task_uid
plan_path = task_root / "review-plans" / "1.json"
plan = json.loads(plan_path.read_text(encoding="utf-8"))
projection = plan["impact_projection"]
projection.update(
    {
        "change_class": "code",
        "affected_consumers": [{"path": "contract.txt"}],
        "review_escalated": False,
        "verification_affected": False,
        "review_reasons": [],
        "public_semantics": [],
    }
)
projection["projection_digest"] = "sha256:" + hashlib.sha256(
    json.dumps(
        {key: value for key, value in projection.items() if key != "projection_digest"},
        sort_keys=True,
        separators=(",", ":"),
    ).encode()
).hexdigest()
plan["impact_projection_digest"] = projection["projection_digest"]
plan_path.write_text(json.dumps(plan), encoding="utf-8")
receipt_path = root / "receipt.json"
receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
receipt["impact_projection_digest"] = projection["projection_digest"]
receipt_path.write_text(json.dumps(receipt), encoding="utf-8")
PY
set +e
TARGET_OID="$RELATED_TARGET" PATH="$TEST_ROOT/bin:$PATH" PM_ROOT_DIR="$FIXTURE" \
  "$FIXTURE/scripts/pm/claim-ready.sh" \
  --claim-type ready_for_pr \
  --verification-profile repository_required \
  --task-uid "$UID_VALUE" \
  --ci-ready-receipt "$FIXTURE/receipt.json" \
  --json >"$TEST_ROOT/related.json" 2>"$TEST_ROOT/related.err"
RELATED_STATUS=$?
set -e
if [[ "$RELATED_STATUS" == "0" ]]; then
  echo "claim-ready accepted a related target-only change" >&2
  exit 1
fi
grep -F "v2 source-review reuse is not proven" "$TEST_ROOT/related.err" >/dev/null

TARGET_OID="$UNRELATED_TARGET" PATH="$TEST_ROOT/bin:$PATH" PM_ROOT_DIR="$FIXTURE" \
  "$FIXTURE/scripts/pm/claim-ready.sh" \
  --claim-type ready_for_pr \
  --verification-profile repository_required \
  --task-uid "$UID_VALUE" \
  --ci-ready-receipt "$FIXTURE/receipt.json" \
  --json >"$TEST_ROOT/unrelated.json"
python3 - "$TEST_ROOT/unrelated.json" <<'PY'
import json
import sys

payload = json.loads(open(sys.argv[1], encoding="utf-8").read())
if payload.get("status") != "verified" or payload.get("allowed_to_claim") is not True:
    raise SystemExit(f"unrelated target advance should remain reusable: {payload}")
PY

echo "claim-ready-ready-pr.test: OK"
