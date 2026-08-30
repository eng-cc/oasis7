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
{"version":1,"tasks":{"$UID_VALUE":{"task_uid":"$UID_VALUE","repository":"fixture/repo","issue_number":3379,"issue_url":"https://github.com/fixture/repo/issues/3379","pr_url":"https://example.invalid/pull/7","canonical_worktree":"$TASK","task_branch":"task/finalize","default_branch":"main","owner_role":"repository_health_engineer","pr_number":7}}}
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
make_mock post-merge-main-sync.sh "echo main-sync >>\"\$TEST_SEQUENCE\""
make_mock post-merge-cleanup.sh "echo cleanup >>\"\$TEST_SEQUENCE\"; : >\"\$TEST_REPO/.git/receipts/terminal-cleanup-receipt.json\""
cat >"$REPO/scripts/pm/post-merge-finalize.py" <<'PY'
#!/usr/bin/env python3
import os, pathlib
with open(os.environ["TEST_SEQUENCE"], "a") as handle: handle.write("finalize\n")
pathlib.Path(os.environ["TEST_REPO"] + "/.git/receipts/finalizer-ledger.json").touch()
if os.environ.get("FINALIZE_FAIL_ONCE") and not pathlib.Path(os.environ["TEST_REPO"] + "/.git/receipts/finalizer-crashed").exists():
    pathlib.Path(os.environ["TEST_REPO"] + "/.git/receipts/finalizer-crashed").touch()
    raise SystemExit(97)
PY
chmod +x "$REPO/scripts/pm/post-merge-finalize.py"
make_mock patch-equivalence-receipt.sh 'echo unexpected-patch >&2; exit 91'

SEQUENCE="$TMP/sequence"
PREFLIGHT_SEQUENCE="$TMP/preflight-sequence"
: >"$PREFLIGHT_SEQUENCE"
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
set_task_field pr_url https://example.invalid/pull/7
set_task_field repository other/repo
set_task_field pr_url https://github.com/other/repo/pull/7
preflight_blocker foreign-repo 7 repository
set_task_field repository fixture/repo
set_task_field pr_url https://example.invalid/pull/7

TEST_SEQUENCE="$SEQUENCE" TEST_REPO="$REPO" TEST_HEAD="$HEAD_OID" \
  "$REPO/scripts/pm/finalize-task.sh" --repo-root "$REPO" --task-uid "$UID_VALUE" --pr 7 --resume --json >"$TMP/result.json"
printf '%s\n' merge-receipt task-closeout refresh main-sync cleanup finalize >"$TMP/expected"
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
grep -F 'task/PR mismatch' "$TMP/bad.err" >/dev/null
test ! -s "$SEQUENCE"

# A crash after the terminal receipt and finalizer ledger are durable must
# resume through the finalizer readback only, without replaying earlier steps.
rm -f "$REPO/.git/receipts/terminal-cleanup-receipt.json" \
  "$REPO/.git/receipts/finalizer-ledger.json" "$REPO/.git/receipts/finalizer-crashed" \
  "$REPO/.git/receipts/merge-receipt.json" "$REPO/.git/receipts/main-sync-receipt.json"
: >"$SEQUENCE"
if FINALIZE_FAIL_ONCE=1 TEST_SEQUENCE="$SEQUENCE" TEST_REPO="$REPO" TEST_HEAD="$HEAD_OID" \
  "$REPO/scripts/pm/finalize-task.sh" --repo-root "$REPO" --task-uid "$UID_VALUE" --pr 7 --resume --json >"$TMP/crash.out" 2>"$TMP/crash.err"; then
  echo "finalize-task unexpectedly hid the injected finalizer crash" >&2
  exit 1
fi
printf '%s\n' merge-receipt task-closeout refresh main-sync cleanup finalize >"$TMP/crash-expected"
cmp "$TMP/crash-expected" "$SEQUENCE"
test -f "$REPO/.git/receipts/terminal-cleanup-receipt.json"
test -f "$REPO/.git/receipts/finalizer-ledger.json"
: >"$SEQUENCE"
TEST_SEQUENCE="$SEQUENCE" TEST_REPO="$REPO" TEST_HEAD="$HEAD_OID" \
  "$REPO/scripts/pm/finalize-task.sh" --repo-root "$REPO" --task-uid "$UID_VALUE" --pr 7 --resume --json >"$TMP/retry.json"
python3 - "$TMP/retry.json" <<'PY'
import json,sys
r=json.load(open(sys.argv[1])); assert r["status"]=="already_finalized" and r["pr_number"]==7,r
PY
# The fixture leaves the canonical checkout directory present after its mocked
# cleanup, modeling a post-terminal checkout resurrection. Resume must run the
# receipt-bound cleanup again before finalizer readback.
printf '%s\n' cleanup finalize >"$TMP/retry-expected"
cmp "$TMP/retry-expected" "$SEQUENCE"

# A published terminal receipt with no ledger is the precise crash window
# between cleanup and finalizer entry. Resume must invoke only the receipt-
# bound finalizer; the now-absent checkout is not required.
rm -f "$REPO/.git/receipts/finalizer-ledger.json"
rm -rf "$TASK"
# The fixture models a fully cleaned terminal checkout; remove the worktree
# registration and local branch so the retry need not re-run cleanup.
git -C "$REPO" worktree prune
git -C "$REPO" update-ref -d refs/heads/task/finalize
: >"$SEQUENCE"
TEST_SEQUENCE="$SEQUENCE" TEST_REPO="$REPO" TEST_HEAD="$HEAD_OID" \
  "$REPO/scripts/pm/finalize-task.sh" --repo-root "$REPO" --task-uid "$UID_VALUE" --pr 7 --resume --json >"$TMP/no-ledger-retry.json"
printf '%s\n' finalize >"$TMP/no-ledger-expected"
cmp "$TMP/no-ledger-expected" "$SEQUENCE"
python3 - "$TMP/no-ledger-retry.json" <<'PY'
import json,sys
r=json.load(open(sys.argv[1])); assert r["status"]=="finalized",r
PY

# Once the ledger exists and no local residue is present, another resume must
# preserve the terminal receipt byte-for-byte and perform finalizer readback.
before_digest="$(shasum -a 256 "$REPO/.git/receipts/terminal-cleanup-receipt.json" | awk '{print $1}')"
: >"$SEQUENCE"
TEST_SEQUENCE="$SEQUENCE" TEST_REPO="$REPO" TEST_HEAD="$HEAD_OID" \
  "$REPO/scripts/pm/finalize-task.sh" --repo-root "$REPO" --task-uid "$UID_VALUE" --pr 7 --resume --json >"$TMP/residue-free-retry.json"
printf '%s\n' finalize >"$TMP/residue-free-expected"
cmp "$TMP/residue-free-expected" "$SEQUENCE"
after_digest="$(shasum -a 256 "$REPO/.git/receipts/terminal-cleanup-receipt.json" | awk '{print $1}')"
[[ "$before_digest" == "$after_digest" ]]
mkdir -p "$TASK"

# Build a real squash/rebase-shaped history.  The task head is not an
# ancestor of origin/main, so the orchestrator must derive and bind a
# repository-generated patch-equivalence receipt before sync/cleanup.
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
r.update(task_branch='task/finalize-squash',pr_number=9,pr_url='https://example.invalid/pull/9')
json.dump(data,open(path,'w',encoding='utf-8'))
PY
rm -f "$REPO/.git/receipts/terminal-cleanup-receipt.json" \
  "$REPO/.git/receipts/finalizer-ledger.json" "$REPO/.git/receipts/finalizer-crashed" \
  "$REPO/.git/receipts/merge-receipt.json" "$REPO/.git/receipts/main-sync-receipt.json" \
  "$REPO/.git/receipts/patch-equivalence-receipt.json"
PATCH_HELPER="$SOURCE_ROOT/scripts/pm/patch-equivalence-receipt.sh"
export PATCH_HELPER
cat >"$REPO/scripts/pm/patch-equivalence-receipt.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo patch-equivalence >>"$TEST_SEQUENCE"
exec "$PATCH_HELPER" "$@"
EOF
chmod +x "$REPO/scripts/pm/patch-equivalence-receipt.sh"
cat >"$REPO/scripts/pm/post-merge-main-sync.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo main-sync >>"$TEST_SEQUENCE"
patch=""
while [[ $# -gt 0 ]]; do
  if [[ "$1" == "--patch-equivalence-receipt" ]]; then patch="${2:-}"; shift 2
  else shift
  fi
done
[[ -n "$patch" && -s "$patch" ]] || { echo 'automatic squash proof was not forwarded' >&2; exit 91; }
EOF
chmod +x "$REPO/scripts/pm/post-merge-main-sync.sh"
cat >"$REPO/scripts/pm/post-merge-cleanup.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo cleanup >>"$TEST_SEQUENCE"
patch=""
while [[ $# -gt 0 ]]; do
  if [[ "$1" == "--patch-equivalence-receipt" ]]; then patch="${2:-}"; shift 2
  else shift
  fi
done
[[ -n "$patch" && -s "$patch" ]] || { echo 'automatic squash proof was not forwarded to cleanup' >&2; exit 92; }
: >"$TEST_REPO/.git/receipts/terminal-cleanup-receipt.json"
EOF
chmod +x "$REPO/scripts/pm/post-merge-cleanup.sh"
: >"$SEQUENCE"
TEST_SEQUENCE="$SEQUENCE" TEST_REPO="$REPO" TEST_HEAD="$SQUASH_TASK_HEAD" \
  "$REPO/scripts/pm/finalize-task.sh" --repo-root "$REPO" --task-uid "$UID_VALUE" --pr 9 --resume --json >"$TMP/squash.json"
printf '%s\n' merge-receipt task-closeout refresh patch-equivalence main-sync cleanup finalize >"$TMP/squash-expected"
cmp "$TMP/squash-expected" "$SEQUENCE"
python3 - "$TMP/squash.json" "$REPO/.git/receipts/patch-equivalence-receipt.json" "$SQUASH_TASK_HEAD" "$SQUASH_MAIN_COMMIT" <<'PY'
import json,sys
result=json.load(open(sys.argv[1])); assert result['status']=='finalized' and result['pr_number']==9,result
proof=json.load(open(sys.argv[2])); assert proof['receipt_type']=='oasis7_patch_equivalence',proof
assert proof['branch_tip']==sys.argv[3],proof
assert proof['main_commit']==sys.argv[4],proof
assert proof['projected_tree_oid']==proof['main_tree_oid'],proof
PY

echo "finalize-task.test: OK"
