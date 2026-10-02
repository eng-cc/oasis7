#!/usr/bin/env bash
set -euo pipefail

# Exercise the delivery-first wrapper with an exact local worktree/ref and a
# remote branch whose tip was replaced after merge. Producer readback is a
# deterministic local fixture; cleanup mutations and OID checks use real Git.
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$ROOT_DIR/scripts/pm/test-fixtures/resource-cleanup-process-probe.sh"
TMPDIR="$(mktemp -d)"
REAL_GIT="$(command -v git)"
REPO="$TMPDIR/repo"
TASK="$TMPDIR/task"
ORIGIN="$TMPDIR/origin.git"
UID_VALUE="task_11111111111111111111111111111111"
BRANCH="task/finalize-remote-mismatch"
RECEIPT_ROOT=""
cleanup() {
  "$REAL_GIT" -C "$REPO" worktree remove --force "$TASK" >/dev/null 2>&1 || true
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

mkdir -p "$REPO/scripts/pm" "$REPO/.pm/github-project-sync" "$TMPDIR/bin"
git init --bare -q "$ORIGIN"
git -C "$ORIGIN" symbolic-ref HEAD refs/heads/main
git -C "$REPO" init -q -b main
git -C "$REPO" config user.email test@example.invalid
git -C "$REPO" config user.name Test
git -C "$REPO" remote add origin https://github.com/eng-cc/oasis7.git
printf 'base\n' >"$REPO/file"
git -C "$REPO" add file
git -C "$REPO" commit -qm base
git -C "$REPO" -c url."$ORIGIN".insteadOf=https://github.com/eng-cc/oasis7.git push -q origin main
git -C "$REPO" worktree add -qb "$BRANCH" "$TASK"
TASK="$(cd "$TASK" && pwd -P)"
printf 'task change\n' >>"$TASK/file"
git -C "$TASK" add file
git -C "$TASK" commit -qm task
REVIEWED_HEAD="$(git -C "$TASK" rev-parse HEAD)"
git -C "$REPO" merge --ff-only "$BRANCH" >/dev/null
MAIN_TIP="$(git -C "$REPO" rev-parse HEAD)"
git -C "$REPO" -c url."$ORIGIN".insteadOf=https://github.com/eng-cc/oasis7.git push -q origin main "$BRANCH"

# Replace the remote branch tip while the reviewed local branch and task
# checkout remain at the exact delivered head.
printf 'remote-only\n' >>"$TASK/file"
git -C "$TASK" commit -qam remote-only
REMOTE_TIP="$(git -C "$TASK" rev-parse HEAD)"
git -C "$TASK" reset --hard "$REVIEWED_HEAD" >/dev/null
git -C "$REPO" -c url."$ORIGIN".insteadOf=https://github.com/eng-cc/oasis7.git push -q --force origin "$REMOTE_TIP:refs/heads/$BRANCH"

for helper in finalize-task.sh post-merge-cleanup.sh canonical-receipt-root.py \
  resource-cleanup-executor.py portable_file_lock.py worktree_registration.py \
  workflow-durable-store.py; do
  cp "$ROOT_DIR/scripts/pm/$helper" "$REPO/scripts/pm/$helper"
done
chmod +x "$REPO/scripts/pm/"*.sh "$REPO/scripts/pm/"*.py

# Mint the trusted worktree instance binding in this new disposable checkout.
mkdir -p "$TASK/.pm/github-project-sync"
cat >"$TASK/.pm/github-project-sync/tasks.json" <<EOF
{"tasks":{"$UID_VALUE":{"task_uid":"$UID_VALUE","repository":"eng-cc/oasis7","canonical_worktree":"$TASK","task_branch":"$BRANCH","default_branch":"main"}}}
EOF
REGISTRATION="$(python3 "$ROOT_DIR/scripts/pm/worktree_registration.py" --repo-root "$TASK" --task-uid "$UID_VALUE")"
rm -rf "$TASK/.pm"
RECEIPT_ROOT="$(python3 "$REPO/scripts/pm/canonical-receipt-root.py" --default-worktree "$REPO" --task-uid "$UID_VALUE" --create)"
cat >"$RECEIPT_ROOT/merge-receipt.json" <<EOF
{"receipt_type":"oasis7_pr_merge","issuer":"github_live_query","evidence_mode":"production","repository":"eng-cc/oasis7","default_branch":"main","pr_number":1,"pr_url":"https://github.com/eng-cc/oasis7/pull/1","state":"MERGED","merged_at":"2026-10-01T00:00:00Z","head_oid":"$REVIEWED_HEAD","merge_commit_oid":"$MAIN_TIP","base_ref":"main","observed_at":"2026-10-01T00:00:00Z"}
EOF

python3 - "$RECEIPT_ROOT/terminal-delivery-receipt.json" "$UID_VALUE" "$REVIEWED_HEAD" \
  "$MAIN_TIP" "$TASK" "$BRANCH" "$REPO/.pm/github-project-sync/tasks.json" \
  "$REGISTRATION" <<'PY'
import hashlib,json,pathlib,sys
path=pathlib.Path(sys.argv[1])
uid,head,merge_oid,worktree,branch,mapping_path=sys.argv[2:8]
registration=json.loads(sys.argv[8])
merge_path=path.with_name("merge-receipt.json")
merge_digest=hashlib.sha256(merge_path.read_bytes()).hexdigest()
receipt={
 "receipt_type":"oasis7_terminal_delivery","schema_version":2,
 "issuer":"post-merge-finalize","evidence_mode":"production",
 "task_uid":uid,"repository":"eng-cc/oasis7","issue_number":10,
 "pr_number":1,"pr_url":"https://github.com/eng-cc/oasis7/pull/1",
 "head_oid":head,"merge_commit_oid":merge_oid,"default_branch":"main",
 "observed_target_oid":merge_oid,"merge_receipt_sha256":merge_digest,
 "task_complete_claim_sha256":"sha256:"+"a"*64,"worktree":worktree,
 "branch":branch,"completion_semantics":"delivery_only",
 "observed_at":"2026-10-01T00:00:00Z",
}
raw=json.dumps(receipt,ensure_ascii=False,sort_keys=True,separators=(",",":" )).encode()+b"\n"
path.write_bytes(raw)
record={
 "task_uid":uid,"status":"done","workflow_phase":"post_merge_done",
 "repository":"eng-cc/oasis7","issue_number":10,
 "issue_url":"https://github.com/eng-cc/oasis7/issues/10",
 "pr_number":1,"pr_url":"https://github.com/eng-cc/oasis7/pull/1",
 "canonical_worktree":worktree,"task_branch":branch,"default_branch":"main",
 "owner_role":"repository_health_engineer","worktree_registration":registration,
 "merge_receipt":json.loads(merge_path.read_text()),
 "phase_receipts":{"post_merge_done":receipt},
 "phase_receipt_type":{"post_merge_done":"oasis7_terminal_delivery"},
 "phase_receipt_sha256":{"post_merge_done":hashlib.sha256(raw).hexdigest()},
}
mapping=pathlib.Path(mapping_path)
mapping.write_text(json.dumps({"version":1,"tasks":{uid:record}},sort_keys=True)+"\n",encoding="utf-8")
PY

# The production producer boundary is a fixture because this test has no live
# GitHub authority. It still checks the mapped protocol selector and exact raw
# receipt digest on every producer and cleanup preflight call.
cat >"$REPO/scripts/pm/post-merge-finalize.py" <<'PY'
#!/usr/bin/env python3
import hashlib,json,pathlib,sys
args=sys.argv[1:]
def value(flag): return args[args.index(flag)+1]
root=pathlib.Path(value("--repo-root")).resolve(); uid=value("--task-uid")
mapping=json.loads((root/".pm/github-project-sync/tasks.json").read_text())
record=mapping["tasks"][uid]
receipt_root=root/".git/oasis7-workflow-receipts"/uid
receipt_path=receipt_root/"terminal-delivery-receipt.json"
raw=receipt_path.read_bytes(); receipt=json.loads(raw)
assert record["phase_receipt_type"]["post_merge_done"]=="oasis7_terminal_delivery"
assert record["phase_receipt_sha256"]["post_merge_done"]==hashlib.sha256(raw).hexdigest()
assert record["phase_receipts"]["post_merge_done"]==receipt
assert receipt["task_uid"]==uid and receipt["repository"]==record["repository"]
log=pathlib.Path(__import__("os").environ["V2_CALL_LOG"])
with log.open("a",encoding="utf-8") as stream: stream.write(" ".join(args)+"\n")
print(json.dumps({"delivery":{"state":"complete","protocol_version":2,"task_uid":uid}}))
PY
chmod +x "$REPO/scripts/pm/post-merge-finalize.py"

# Preserve the canonical GitHub origin string for repo-identity checks, while
# routing only isolated remote reads/deletes to the local bare test remote.
cat >"$TMPDIR/bin/git" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
real_git="${OASIS7_REAL_GIT:?}"
fixture_origin="${OASIS7_FIXTURE_ORIGIN:?}"
operation=""
for arg in "$@"; do
  if [[ "$arg" == ls-remote || "$arg" == push ]]; then operation="$arg"; break; fi
done
if [[ -n "$operation" ]]; then
  args=()
  for arg in "$@"; do
    if [[ "$arg" == origin ]]; then args+=("$fixture_origin"); else args+=("$arg"); fi
  done
  exec "$real_git" "${args[@]}"
fi
exec "$real_git" "$@"
SH
chmod +x "$TMPDIR/bin/git"
export V2_CALL_LOG="$TMPDIR/producer-calls.log"
export OASIS7_REAL_GIT="$REAL_GIT" OASIS7_FIXTURE_ORIGIN="$ORIGIN"

# A mismatched caller PR fails before producer or cleanup; it cannot select a
# different task delivery or mutate any real Git resource.
set +e
PATH="$TMPDIR/bin:$PATH" bash "$REPO/scripts/pm/finalize-task.sh" --repo-root "$REPO" --task-uid "$UID_VALUE" \
  --pr 2 --resume --json >"$TMPDIR/wrong-pr.out" 2>"$TMPDIR/wrong-pr.err"
wrong_status=$?
set -e
[[ "$wrong_status" != 0 ]]
grep -F "task/PR mismatch" "$TMPDIR/wrong-pr.out" "$TMPDIR/wrong-pr.err" >/dev/null
[[ ! -e "$V2_CALL_LOG" ]]
[[ -d "$TASK" ]]
[[ "$(git -C "$REPO" rev-parse "refs/heads/$BRANCH")" == "$REVIEWED_HEAD" ]]
[[ "$(git -C "$ORIGIN" rev-parse "refs/heads/$BRANCH")" == "$REMOTE_TIP" ]]

# The next two calls exercise intended cleanup paths. Keep delivery/cleanup and
# all Git checks live while giving the process guard a complete readback.
oasis7_install_complete_process_probe "$TMPDIR/bin"

MERGE_BEFORE="$(shasum -a 256 "$RECEIPT_ROOT/merge-receipt.json" | awk '{print $1}')"
DELIVERY_BEFORE="$(shasum -a 256 "$RECEIPT_ROOT/terminal-delivery-receipt.json" | awk '{print $1}')"
MAPPING_BEFORE="$(shasum -a 256 "$REPO/.pm/github-project-sync/tasks.json" | awk '{print $1}')"
PATH="$TMPDIR/bin:$PATH" bash "$REPO/scripts/pm/finalize-task.sh" --repo-root "$REPO" --task-uid "$UID_VALUE" \
  --pr 1 --resume --json >"$TMPDIR/finalize.out"

python3 - "$TMPDIR/finalize.out" <<'PY'
import json,sys
result=json.loads(open(sys.argv[1],encoding="utf-8").read())
assert result["status"]=="finalized",result
assert result["delivery"]=={"state":"complete","protocol_version":2},result
assert result["cleanup_state"]=="cleanup_deferred",result
assert result.get("cleanup",{}).get("remote_branch",{}).get("state")=="retained",result
PY
[[ ! -e "$TASK" ]]
! git -C "$REPO" show-ref --verify --quiet "refs/heads/$BRANCH"
[[ "$(git -C "$ORIGIN" rev-parse "refs/heads/$BRANCH")" == "$REMOTE_TIP" ]]

# The wrapper must read back delivery before invoking the cleanup executor's
# producer preflight. The retained remote ref does not revoke delivery.
python3 - "$V2_CALL_LOG" <<'PY'
import sys
rows=open(sys.argv[1],encoding="utf-8").read().splitlines()
assert len(rows)>=2,rows
assert "--delivery --json" in rows[0],rows
assert "--delivery --preflight --json" in rows[1],rows
PY
[[ "$(shasum -a 256 "$RECEIPT_ROOT/merge-receipt.json" | awk '{print $1}')" == "$MERGE_BEFORE" ]]
[[ "$(shasum -a 256 "$RECEIPT_ROOT/terminal-delivery-receipt.json" | awk '{print $1}')" == "$DELIVERY_BEFORE" ]]
[[ "$(shasum -a 256 "$REPO/.pm/github-project-sync/tasks.json" | awk '{print $1}')" == "$MAPPING_BEFORE" ]]

# A cleanup-only retry rechecks delivery, retains the replaced remote OID,
# and cannot change the already accepted delivery selector or receipt bytes.
set +e
PATH="$TMPDIR/bin:$PATH" bash "$REPO/scripts/pm/finalize-task.sh" --repo-root "$REPO" --task-uid "$UID_VALUE" \
  --pr 1 --resume --cleanup-only --json >"$TMPDIR/cleanup-only.out" 2>"$TMPDIR/cleanup-only.err"
cleanup_status=$?
set -e
[[ "$cleanup_status" != 0 ]]
python3 - "$TMPDIR/cleanup-only.out" <<'PY'
import json,sys
result=json.loads(open(sys.argv[1],encoding="utf-8").read())
assert result["delivery"]=={"state":"complete","protocol_version":2},result
assert result["cleanup"]["remote_branch"]["state"]=="retained",result
PY
[[ "$(git -C "$ORIGIN" rev-parse "refs/heads/$BRANCH")" == "$REMOTE_TIP" ]]
[[ "$(shasum -a 256 "$RECEIPT_ROOT/merge-receipt.json" | awk '{print $1}')" == "$MERGE_BEFORE" ]]
[[ "$(shasum -a 256 "$RECEIPT_ROOT/terminal-delivery-receipt.json" | awk '{print $1}')" == "$DELIVERY_BEFORE" ]]
[[ "$(shasum -a 256 "$REPO/.pm/github-project-sync/tasks.json" | awk '{print $1}')" == "$MAPPING_BEFORE" ]]
echo "finalize-task-remote-branch-mismatch.test: OK"
