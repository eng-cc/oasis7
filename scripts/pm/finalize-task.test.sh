#!/usr/bin/env bash
set -euo pipefail

SOURCE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
REPO="$TMP/repo"
TASK="$TMP/task"
UID_VALUE="task_11111111111111111111111111111111"
mkdir -p "$REPO/scripts/pm" "$REPO/.pm/github-project-sync" "$TASK"
cp "$SOURCE_ROOT/scripts/pm/finalize-task.sh" "$REPO/scripts/pm/finalize-task.sh"
chmod +x "$REPO/scripts/pm/finalize-task.sh"

git -C "$REPO" init -q -b main
git -C "$REPO" config user.email test@example.invalid
git -C "$REPO" config user.name Test
printf 'fixture\n' >"$REPO/README"
git -C "$REPO" add README
git -C "$REPO" commit -qm fixture
INITIAL_MAIN_OID="$(git -C "$REPO" rev-parse HEAD)"
ORIGIN="$TMP/origin.git"
git init --bare -q "$ORIGIN"
git -C "$REPO" remote add origin https://github.com/fixture/repo.git
git -C "$REPO" config url."$ORIGIN".insteadOf https://github.com/fixture/repo.git
git -C "$REPO" push -q origin main

# The canonical task checkout is a registered worktree so preflight can prove
# repository/common-dir and branch identity before terminal effects.
git -C "$REPO" worktree add -q -b task/finalize "$TASK" HEAD
mkdir -p "$TASK/scripts/pm"
cp "$SOURCE_ROOT/scripts/pm/finalize-task.sh" "$TASK/scripts/pm/finalize-task.sh"
chmod +x "$TASK/scripts/pm/finalize-task.sh"

# The remote-tracking ref is deliberately stale while the real origin/main
# already contains the ordinary fast-forward integration.  The orchestrator
# must refresh before selecting the ordinary lane; otherwise it misclassifies
# this as squash/rebase and invokes the patch-equivalence helper.
printf 'ordinary change\n' >>"$REPO/README"
git -C "$REPO" add README
git -C "$REPO" commit -qm ordinary-integration
HEAD_OID="$(git -C "$REPO" rev-parse HEAD)"
git -C "$REPO" push -q origin main
git -C "$REPO" update-ref refs/remotes/origin/main "$INITIAL_MAIN_OID"

cat >"$REPO/.pm/github-project-sync/tasks.json" <<EOF
{"version":1,"tasks":{"$UID_VALUE":{"task_uid":"$UID_VALUE","repository":"fixture/repo","issue_number":3379,"issue_url":"https://github.com/fixture/repo/issues/3379","pr_url":"https://github.com/fixture/repo/pull/7","canonical_worktree":"$TASK","task_branch":"task/finalize","default_branch":"main","owner_role":"repository_health_engineer","pr_number":7}}}
EOF

make_mock() {
  local name=$1 body=$2
  cat >"$REPO/scripts/pm/$name" <<EOF
#!/usr/bin/env bash
set -euo pipefail
$body
EOF
  chmod +x "$REPO/scripts/pm/$name"
}
make_mock refresh-task-cache.sh "echo refresh >>\"\$TEST_SEQUENCE\"; printf \"{}\\n\""
cat >"$REPO/scripts/pm/canonical-receipt-root.py" <<'PY'
#!/usr/bin/env python3
import os, pathlib, sys
root = pathlib.Path(os.environ["TEST_REPO"]) / ".git/receipts"
if "--create" in sys.argv[1:]:
    root.mkdir(parents=True, exist_ok=True)
print(root)
PY
chmod +x "$REPO/scripts/pm/canonical-receipt-root.py"
cat >"$REPO/scripts/pm/pr-merge-receipt.py" <<'PY'
#!/usr/bin/env python3
import json, os
with open(os.environ["TEST_SEQUENCE"], "a") as handle: handle.write("merge-receipt\n")
print(json.dumps({"head_oid": os.environ["TEST_HEAD"]}))
PY
chmod +x "$REPO/scripts/pm/pr-merge-receipt.py"
make_mock task-closeout.sh "echo task-closeout >>\"\$TEST_SEQUENCE\""
make_mock post-merge-main-sync.sh "echo main-sync >>\"\$TEST_SEQUENCE\"; exit 91"
cat >"$REPO/scripts/pm/post-merge-cleanup.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo cleanup >>"$TEST_SEQUENCE"
printf '%s\n' '{"status":"cleaned","cleanup_state":"complete","cleanup":{},"cleanup_blockers":[]}'
EOF
chmod +x "$REPO/scripts/pm/post-merge-cleanup.sh"
cat >"$REPO/scripts/pm/post-merge-finalize.py" <<'PY'
#!/usr/bin/env python3
import hashlib, json, os, pathlib, sys
with open(os.environ["TEST_SEQUENCE"], "a") as handle: handle.write("finalize\n")
root=pathlib.Path(os.environ["TEST_REPO"]); receipts=root/".git/receipts"
receipts.mkdir(parents=True,exist_ok=True)
mapping_path=root/".pm/github-project-sync/tasks.json"
mapping=json.loads(mapping_path.read_text()); uid=next(iter(mapping["tasks"]))
record=mapping["tasks"][uid]
selected=(record.get("phase_receipt_type") or {}).get("post_merge_done")=="oasis7_terminal_delivery"
ledger=receipts/"finalizer-ledger.json"
receipt_path=receipts/"terminal-delivery-receipt.json"
if selected and not ledger.exists():
    print("selected v2 delivery has no finalizer ledger",file=sys.stderr); raise SystemExit(97)
if not selected:
    receipt={"receipt_type":"oasis7_terminal_delivery","schema_version":2,"task_uid":uid,
             "repository":record.get("repository"),"completion_semantics":"delivery_only"}
    raw=json.dumps(receipt,sort_keys=True,separators=(",",":" )).encode()+b"\n"
    digest=hashlib.sha256(raw).hexdigest(); receipt_path.write_bytes(raw)
    ledger.write_text(json.dumps({"schema":"oasis7_finalizer_ledger_v1","task_uid":uid,"operations":{}},sort_keys=True)+"\n")
    record["workflow_phase"]="post_merge_done"
    record.setdefault("phase_receipts",{})["post_merge_done"]=receipt
    record.setdefault("phase_receipt_type",{})["post_merge_done"]="oasis7_terminal_delivery"
    record.setdefault("phase_receipt_sha256",{})["post_merge_done"]=digest
    record.setdefault("phase_receipt_comment_id",{})["post_merge_done"]=900
    record.setdefault("phase_receipt_comment_sha256",{})["post_merge_done"]="f"*64
    mapping["tasks"][uid]=record; mapping_path.write_text(json.dumps(mapping,sort_keys=True)+"\n")
if os.environ.get("FINALIZE_FAIL_ONCE") and not (receipts/"finalizer-crashed").exists():
    (receipts/"finalizer-crashed").touch()
    raise SystemExit(97)
print(json.dumps({"status":"already_finalized" if selected else "finalized",
                  "protocol_version":2,"delivery":{"state":"complete","protocol_version":2}}))
PY
chmod +x "$REPO/scripts/pm/post-merge-finalize.py"
make_mock patch-equivalence-receipt.sh 'echo unexpected-patch >&2; exit 91'

SEQUENCE="$TMP/sequence"
PREFLIGHT_SEQUENCE="$TMP/preflight-sequence"
: >"$PREFLIGHT_SEQUENCE"
# Before merge, the default worktree may still carry a helper that predates
# --preflight.  The reviewed task-worktree helper must remain usable while
# retaining the default worktree as --repo-root.
cp "$REPO/scripts/pm/finalize-task.sh" "$TMP/current-finalize-task.sh"
cat >"$REPO/scripts/pm/finalize-task.sh" <<'EOF'
#!/usr/bin/env bash
echo 'legacy default helper does not support --preflight' >&2
exit 2
EOF
chmod +x "$REPO/scripts/pm/finalize-task.sh"
TEST_SEQUENCE="$PREFLIGHT_SEQUENCE" TEST_REPO="$REPO" TEST_HEAD="$HEAD_OID" \
  "$TASK/scripts/pm/finalize-task.sh" --repo-root "$REPO" --task-uid "$UID_VALUE" --pr 7 --preflight --json >"$TMP/compat-preflight.json" 2>"$TMP/compat-preflight.err" || {
    cat "$TMP/compat-preflight.err" >&2
    cat "$TMP/compat-preflight.json" >&2
    exit 1
  }
python3 - "$TMP/compat-preflight.json" <<'PY'
import json, sys
result = json.load(open(sys.argv[1], encoding="utf-8"))
assert result["status"] == "ready", result
assert result["repo_root"].endswith("/repo"), result
PY
cp "$TMP/current-finalize-task.sh" "$REPO/scripts/pm/finalize-task.sh"

TEST_SEQUENCE="$PREFLIGHT_SEQUENCE" TEST_REPO="$REPO" TEST_HEAD="$HEAD_OID" \
  "$REPO/scripts/pm/finalize-task.sh" --repo-root "$REPO" --task-uid "$UID_VALUE" --pr 7 --preflight --json >"$TMP/preflight.json" 2>"$TMP/preflight.err" || {
    cat "$TMP/preflight.err" >&2
    cat "$TMP/preflight.json" >&2
    exit 1
  }
test ! -s "$PREFLIGHT_SEQUENCE"
test ! -e "$REPO/.git/receipts"
mkdir -p "$REPO/.git/receipts"
python3 - "$TMP/preflight.json" <<'PY'
import json, sys
result = json.load(open(sys.argv[1], encoding="utf-8"))
assert result["status"] == "ready", result
assert result["blockers"] == [], result
assert result["task_uid"] == "task_11111111111111111111111111111111", result
assert result["pr_number"] == 7, result
assert result["canonical_worktree"].endswith("/task"), result
assert result["task_branch"] == "task/finalize", result
assert result["next_command"] == [
    "./scripts/pm/finalize-task.sh", "--repo-root", result["repo_root"],
    "--task-uid", result["task_uid"], "--pr", "7", "--resume", "--json"
], result
PY

preflight_blocker() {
  local label="$1"
  local pr="${2:-7}"
  local expected="${3:-$label}"
  set +e
  TEST_SEQUENCE="$PREFLIGHT_SEQUENCE" TEST_REPO="$REPO" TEST_HEAD="$HEAD_OID" \
    "$REPO/scripts/pm/finalize-task.sh" --repo-root "$REPO" --task-uid "$UID_VALUE" --pr "$pr" --preflight --json \
    >"$TMP/preflight-$label.json" 2>"$TMP/preflight-$label.err"
  local status=$?
  set -e
  [[ "$status" != 0 ]]
  python3 - "$TMP/preflight-$label.json" "$expected" <<'PY'
import json, sys
result = json.load(open(sys.argv[1], encoding="utf-8"))
assert result["status"] == "blocked", result
assert result["next_command"] == [], result
assert any(sys.argv[2] in blocker for blocker in result["blockers"]), result
PY
}

set_task_field() {
  python3 - "$REPO/.pm/github-project-sync/tasks.json" "$UID_VALUE" "$1" "$2" <<'PY'
import json, sys
path, uid, key, value = sys.argv[1:]
data = json.load(open(path, encoding="utf-8"))
data["tasks"][uid][key] = value
json.dump(data, open(path, "w", encoding="utf-8"))
PY
}

git -C "$REPO" config --unset remote.origin.url
preflight_blocker missing-origin 7 origin
git -C "$REPO" config remote.origin.url https://github.com/fixture/repo.git

set_task_field issue_url https://example.invalid/issues/3379
preflight_blocker malformed-issue-url 7 "Issue URL"
set_task_field issue_url https://github.com/fixture/repo/issues/3379

set_task_field pr_url https://example.invalid/pull/7
preflight_blocker unsupported-pr-url 7 "PR URL"
set_task_field pr_url https://github.com/fixture/repo/pull/7

git -C "$REPO" checkout --detach -q HEAD
preflight_blocker detached-default 7 detached
git -C "$REPO" switch -q main

preflight_blocker task-pr 8 "task/PR"
set_task_field task_uid task_22222222222222222222222222222222
preflight_blocker task-uid 7 "task UID"
set_task_field task_uid "$UID_VALUE"
set_task_field canonical_worktree /tmp/not-the-task-worktree
preflight_blocker worktree
set_task_field canonical_worktree "$TASK"
set_task_field task_branch task/wrong-branch
preflight_blocker branch
set_task_field task_branch task/finalize
set_task_field repository invalid-repository
preflight_blocker repository
set_task_field repository fixture/repo
set_task_field pr_url https://github.com/other/repo/pull/7
preflight_blocker foreign-pr 7 repository
set_task_field pr_url https://github.com/fixture/repo/pull/7
set_task_field repository other/repo
set_task_field pr_url https://github.com/other/repo/pull/7
preflight_blocker foreign-repo 7 repository
set_task_field repository fixture/repo
set_task_field pr_url https://github.com/fixture/repo/pull/7

TEST_SEQUENCE="$SEQUENCE" TEST_REPO="$REPO" TEST_HEAD="$HEAD_OID" \
  "$REPO/scripts/pm/finalize-task.sh" --repo-root "$REPO" --task-uid "$UID_VALUE" --pr 7 --resume --json >"$TMP/result.json"
printf '%s\n' merge-receipt task-closeout refresh finalize cleanup >"$TMP/expected"
cmp "$TMP/expected" "$SEQUENCE"
python3 - "$TMP/result.json" <<'PY'
import json,sys
r=json.load(open(sys.argv[1])); assert r["status"]=="finalized" and r["pr_number"]==7 and r["resume"] is True,r
PY

: >"$SEQUENCE"
if TEST_SEQUENCE="$SEQUENCE" TEST_REPO="$REPO" TEST_HEAD="$HEAD_OID" \
  "$REPO/scripts/pm/finalize-task.sh" --repo-root "$REPO" --task-uid "$UID_VALUE" --pr 8 --json >"$TMP/bad.out" 2>"$TMP/bad.err"; then
  echo "finalize-task accepted task/PR identity mismatch" >&2
  exit 1
fi
python3 - "$TMP/bad.out" <<'PY'
import json,sys
result=json.load(open(sys.argv[1])); assert result["status"]=="blocked",result
assert any("task/PR mismatch" in item for item in result["blockers"]),result
PY
test ! -s "$SEQUENCE"

# Reset this mocked task to its task_done boundary, then crash after delivery
# receipt/ledger and selector are durable but before cleanup is invoked.
python3 - "$REPO/.pm/github-project-sync/tasks.json" "$UID_VALUE" <<'PY'
import json,sys
path,uid=sys.argv[1:]; data=json.load(open(path)); task=data['tasks'][uid]
task['workflow_phase']='task_done'
for key in ('phase_receipts','phase_receipt_type','phase_receipt_sha256',
            'phase_receipt_comment_id','phase_receipt_comment_sha256','evidence_comments'):
    task.pop(key,None)
json.dump(data,open(path,'w'),sort_keys=True)
PY
rm -f "$REPO/.git/receipts/terminal-delivery-receipt.json" \
  "$REPO/.git/receipts/terminal-tombstone.json" \
  "$REPO/.git/receipts/finalizer-ledger.json" "$REPO/.git/receipts/finalizer-crashed" \
  "$REPO/.git/receipts/merge-receipt.json" "$REPO/.git/receipts/main-sync-receipt.json"
: >"$SEQUENCE"
if FINALIZE_FAIL_ONCE=1 TEST_SEQUENCE="$SEQUENCE" TEST_REPO="$REPO" TEST_HEAD="$HEAD_OID" \
  "$REPO/scripts/pm/finalize-task.sh" --repo-root "$REPO" --task-uid "$UID_VALUE" --pr 7 --resume --json >"$TMP/crash.out" 2>"$TMP/crash.err"; then
  echo "finalize-task unexpectedly hid the injected finalizer crash" >&2
  exit 1
fi
printf '%s\n' merge-receipt task-closeout refresh finalize >"$TMP/crash-expected"
cmp "$TMP/crash-expected" "$SEQUENCE"
test -f "$REPO/.git/receipts/terminal-delivery-receipt.json"
test -f "$REPO/.git/receipts/finalizer-ledger.json"
: >"$SEQUENCE"
TEST_SEQUENCE="$SEQUENCE" TEST_REPO="$REPO" TEST_HEAD="$HEAD_OID" \
  "$REPO/scripts/pm/finalize-task.sh" --repo-root "$REPO" --task-uid "$UID_VALUE" --pr 7 --resume --json >"$TMP/retry.json"
python3 - "$TMP/retry.json" <<'PY'
import json,sys
r=json.load(open(sys.argv[1])); assert r["status"]=="finalized" and r["pr_number"]==7,r
PY
# Producer readback must remain ahead of every cleanup retry.
printf '%s\n' finalize cleanup >"$TMP/retry-expected"
cmp "$TMP/retry-expected" "$SEQUENCE"

# A selected delivery without its finalizer ledger is not sufficient authority
# for cleanup or successful resume.
cp "$REPO/.git/receipts/finalizer-ledger.json" "$TMP/finalizer-ledger.saved.json"
rm -f "$REPO/.git/receipts/finalizer-ledger.json"
rm -rf "$TASK"
# The fixture models a fully cleaned terminal checkout; remove the worktree
# registration and local branch so the retry need not re-run cleanup.
git -C "$REPO" worktree prune
git -C "$REPO" update-ref -d refs/heads/task/finalize
: >"$SEQUENCE"
if TEST_SEQUENCE="$SEQUENCE" TEST_REPO="$REPO" TEST_HEAD="$HEAD_OID" \
  "$REPO/scripts/pm/finalize-task.sh" --repo-root "$REPO" --task-uid "$UID_VALUE" --pr 7 --resume --json >"$TMP/no-ledger-retry.json" 2>"$TMP/no-ledger-retry.err"; then
  echo "finalize-task accepted v2 delivery without its ledger" >&2
  exit 1
fi
printf '%s\n' finalize >"$TMP/no-ledger-expected"
cmp "$TMP/no-ledger-expected" "$SEQUENCE"
cp "$TMP/finalizer-ledger.saved.json" "$REPO/.git/receipts/finalizer-ledger.json"

# Once the ledger exists and no local residue is present, another resume must
# preserve the delivery receipt byte-for-byte and read back before cleanup.
before_digest="$(shasum -a 256 "$REPO/.git/receipts/terminal-delivery-receipt.json" | awk '{print $1}')"
: >"$SEQUENCE"
TEST_SEQUENCE="$SEQUENCE" TEST_REPO="$REPO" TEST_HEAD="$HEAD_OID" \
  "$REPO/scripts/pm/finalize-task.sh" --repo-root "$REPO" --task-uid "$UID_VALUE" --pr 7 --resume --json >"$TMP/residue-free-retry.json"
printf '%s\n' finalize cleanup >"$TMP/residue-free-expected"
cmp "$TMP/residue-free-expected" "$SEQUENCE"
after_digest="$(shasum -a 256 "$REPO/.git/receipts/terminal-delivery-receipt.json" | awk '{print $1}')"
[[ "$before_digest" == "$after_digest" ]]
mkdir -p "$TASK"

# Build a squash/rebase-shaped history. Delivery is still proved from the
# merged PR and live default-branch readback; it does not invoke main-sync or
# synthesize a patch-equivalence cleanup prerequisite.
git -C "$REPO" switch -q -c task/finalize-squash
printf 'squash change\n' >"$REPO/squash.txt"
git -C "$REPO" add squash.txt
git -C "$REPO" commit -qm task-squash
SQUASH_TASK_HEAD="$(git -C "$REPO" rev-parse HEAD)"
git -C "$REPO" switch -q main
printf 'squash change\n' >"$REPO/squash.txt"
git -C "$REPO" add squash.txt
git -C "$REPO" commit -qm squash-integration
SQUASH_MAIN_COMMIT="$(git -C "$REPO" rev-parse HEAD)"
git -C "$REPO" push -q origin main
git -C "$REPO" worktree add -q "$TASK" task/finalize-squash
python3 - "$REPO/.pm/github-project-sync/tasks.json" "$UID_VALUE" <<'PY'
import json,sys
path,uid=sys.argv[1:]
data=json.load(open(path,encoding='utf-8'))
r=data['tasks'][uid]
r['workflow_phase']='task_done'
for key in ('phase_receipts','phase_receipt_type','phase_receipt_sha256',
            'phase_receipt_comment_id','phase_receipt_comment_sha256','evidence_comments'):
    r.pop(key,None)
r.update(task_branch='task/finalize-squash',pr_number=9,pr_url='https://github.com/fixture/repo/pull/9')
json.dump(data,open(path,'w',encoding='utf-8'))
PY
rm -f "$REPO/.git/receipts/terminal-delivery-receipt.json" \
  "$REPO/.git/receipts/terminal-tombstone.json" \
  "$REPO/.git/receipts/finalizer-ledger.json" "$REPO/.git/receipts/finalizer-crashed" \
  "$REPO/.git/receipts/merge-receipt.json" "$REPO/.git/receipts/main-sync-receipt.json" \
  "$REPO/.git/receipts/patch-equivalence-receipt.json"
: >"$SEQUENCE"
TEST_SEQUENCE="$SEQUENCE" TEST_REPO="$REPO" TEST_HEAD="$SQUASH_TASK_HEAD" \
  "$REPO/scripts/pm/finalize-task.sh" --repo-root "$REPO" --task-uid "$UID_VALUE" --pr 9 --resume --json >"$TMP/squash.json"
printf '%s\n' merge-receipt task-closeout refresh finalize cleanup >"$TMP/squash-expected"
cmp "$TMP/squash-expected" "$SEQUENCE"
python3 - "$TMP/squash.json" "$REPO/.git/receipts" <<'PY'
import json,sys
result=json.load(open(sys.argv[1])); assert result['status']=='finalized' and result['pr_number']==9,result
receipts=sys.argv[2]
assert not __import__('pathlib').Path(receipts,'main-sync-receipt.json').exists()
assert not __import__('pathlib').Path(receipts,'patch-equivalence-receipt.json').exists()
PY

echo "finalize-task.test: OK"
