#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

die() { echo "post-merge-main-sync: $*" >&2; exit 1; }
usage() {
  echo "Usage: $0 --repo-root <path> --main-ref <branch> --task-uid <uid> --pr-receipt <json> --receipt-output <json>"
}

REPO_ROOT="" MAIN_REF="" TASK_UID="" PR_RECEIPT="" RECEIPT_OUTPUT=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo-root) REPO_ROOT="${2:-}"; shift 2 ;;
    --main-ref) MAIN_REF="${2:-}"; shift 2 ;;
    --task-uid) TASK_UID="${2:-}"; shift 2 ;;
    --pr-receipt) PR_RECEIPT="${2:-}"; shift 2 ;;
    --receipt-output) RECEIPT_OUTPUT="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) die "unknown argument: $1" ;;
  esac
done
for name in REPO_ROOT MAIN_REF TASK_UID PR_RECEIPT RECEIPT_OUTPUT; do
  [[ -n "${!name}" ]] || die "missing --$(printf '%s' "$name" | tr '[:upper:]_' '[:lower:]-')"
done
[[ -f "$PR_RECEIPT" ]] || die "merge receipt is unavailable"
git -C "$REPO_ROOT" rev-parse --is-inside-work-tree >/dev/null || die "invalid repo root"
REPO_ROOT="$(git -C "$REPO_ROOT" rev-parse --show-toplevel)"
# Canonical validation precedes fetch or any other repository effect.
PR_RECEIPT="$(python3 "$SCRIPT_DIR/canonical-receipt-root.py" --default-worktree "$REPO_ROOT" --task-uid "$TASK_UID" --create --path "$PR_RECEIPT" --name merge-receipt.json)" || die "noncanonical merge receipt"
RECEIPT_OUTPUT="$(python3 "$SCRIPT_DIR/canonical-receipt-root.py" --default-worktree "$REPO_ROOT" --task-uid "$TASK_UID" --create --path "$RECEIPT_OUTPUT" --name main-sync-receipt.json)" || die "noncanonical main-sync receipt output"
ACTUAL_BRANCH="$(git -C "$REPO_ROOT" symbolic-ref --quiet --short HEAD)" || die "repo root must be on a named branch"
[[ "$ACTUAL_BRANCH" == "$MAIN_REF" ]] || die "repo root is not checked out on the requested default branch"
[[ -z "$(git -C "$REPO_ROOT" status --porcelain --untracked-files=all)" ]] || die "default-branch worktree is dirty"
MAPPING="$REPO_ROOT/.pm/github-project-sync/tasks.json"
[[ -f "$MAPPING" ]] || die "task mapping is unavailable"
# RECEIPT_OUTPUT is accepted only after Path.is_absolute and resolved
# relative_to(canonical_worktree) rejection in the shared validator; the
# canonical task worktree is never a durable receipt sink.
RECEIPT_OUTPUT="$(python3 "$SCRIPT_DIR/validate-durable-terminal-path.py" \
  --mapping "$MAPPING" --task-uid "$TASK_UID" --path "$RECEIPT_OUTPUT" \
  --label "main-sync receipt output")" || die "invalid durable receipt output path"

python3 - "$MAPPING" "$TASK_UID" "$PR_RECEIPT" "$MAIN_REF" <<'PY'
import datetime as d,json,sys
record=(json.load(open(sys.argv[1],encoding='utf-8')).get('tasks') or {}).get(sys.argv[2]) or {}
receipt=json.load(open(sys.argv[3],encoding='utf-8'))
for key in ('repository','default_branch','pr_number','pr_url'):
    if not record.get(key): raise SystemExit(f'post-merge-main-sync: task truth is missing {key}')
if record.get('status')!='done': raise SystemExit('post-merge-main-sync: task must be done')
if receipt.get('receipt_type')!='oasis7_pr_merge' or receipt.get('issuer')!='github_live_query' or receipt.get('evidence_mode')!='production' or receipt.get('state')!='MERGED': raise SystemExit('post-merge-main-sync: invalid merge receipt')
for key in ('repository','default_branch','pr_number','pr_url','merged_at','head_oid','base_ref','observed_at'):
    if not receipt.get(key): raise SystemExit(f'post-merge-main-sync: merge receipt is missing {key}')
for receipt_key, record_key in (('repository','repository'),('default_branch','default_branch'),('pr_number','pr_number'),('pr_url','pr_url')):
    if str(receipt.get(receipt_key))!=str(record.get(record_key)): raise SystemExit(f'post-merge-main-sync: receipt {receipt_key} disagrees with task truth')
if receipt.get('base_ref')!=sys.argv[4] or record.get('default_branch')!=sys.argv[4]: raise SystemExit('post-merge-main-sync: default branch identity mismatch')
seen=d.datetime.fromisoformat(str(receipt.get('observed_at')).replace('Z','+00:00'))
age=(d.datetime.now(d.timezone.utc)-seen).total_seconds()
if age < -30 or age > 600: raise SystemExit('post-merge-main-sync: merge receipt is stale')
PY

git -C "$REPO_ROOT" remote get-url origin >/dev/null 2>&1 || die "origin remote is required"
git -C "$REPO_ROOT" fetch --quiet origin "refs/heads/$MAIN_REF:refs/remotes/origin/$MAIN_REF" || die "failed to fetch origin default branch"
REMOTE_MAIN="$(git -C "$REPO_ROOT" rev-parse "refs/remotes/origin/$MAIN_REF")"
git -C "$REPO_ROOT" merge --ff-only "$REMOTE_MAIN" >/dev/null || die "default branch cannot fast-forward to origin"
LOCAL_MAIN="$(git -C "$REPO_ROOT" rev-parse "refs/heads/$MAIN_REF")"
[[ "$LOCAL_MAIN" == "$REMOTE_MAIN" ]] || die "default branch did not synchronize exactly"
MERGED_HEAD="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1],encoding="utf-8")).get("head_oid") or "")' "$PR_RECEIPT")"
git -C "$REPO_ROOT" merge-base --is-ancestor "$MERGED_HEAD" "$LOCAL_MAIN" || die "synchronized default branch does not contain merged head"

mkdir -p "$(dirname "$RECEIPT_OUTPUT")"
TMP_RECEIPT="$(mktemp "$(dirname "$RECEIPT_OUTPUT")/.main-sync.XXXXXX")"
trap 'rm -f "$TMP_RECEIPT"' EXIT
python3 - "$TMP_RECEIPT" "$MAPPING" "$TASK_UID" "$PR_RECEIPT" "$MAIN_REF" "$LOCAL_MAIN" <<'PY'
import datetime as d,hashlib,json,pathlib,sys
mapping=json.load(open(sys.argv[2],encoding='utf-8')); record=(mapping.get('tasks') or {}).get(sys.argv[3]) or {}
receipt_path=pathlib.Path(sys.argv[4]); digest=hashlib.sha256(receipt_path.read_bytes()).hexdigest()
out={'receipt_type':'oasis7_main_sync','issuer':'post-merge-main-sync','task_uid':sys.argv[3],
     'repository':record['repository'],'default_branch':sys.argv[5],
     'main_commit':sys.argv[6],'remote_main_commit':sys.argv[6],
     'merge_receipt_sha256':digest,'observed_at':d.datetime.now(d.timezone.utc).isoformat()}
pathlib.Path(sys.argv[1]).write_text(json.dumps(out,indent=2,sort_keys=True)+'\n',encoding='utf-8')
PY
mv "$TMP_RECEIPT" "$RECEIPT_OUTPUT"
trap - EXIT
# {"workflow_phase":"main_sync"}
python3 - "$SCRIPT_DIR/workflow-durable-store.py" "$MAPPING" "$TASK_UID" "$RECEIPT_OUTPUT" <<'PY'
import hashlib,importlib.util,json,pathlib,sys
spec=importlib.util.spec_from_file_location('workflow_durable_store',sys.argv[1]); store=importlib.util.module_from_spec(spec); spec.loader.exec_module(store)
receipt_path=pathlib.Path(sys.argv[4]); receipt=json.loads(receipt_path.read_text())
def update(data):
 record=(data.get('tasks') or {}).get(sys.argv[3]) or {}; record['workflow_phase']='main_sync'
 record.setdefault('phase_receipts',{})['main_sync']=receipt
 record.setdefault('phase_receipt_sha256',{})['main_sync']=hashlib.sha256(receipt_path.read_bytes()).hexdigest()
 data.setdefault('tasks',{})[sys.argv[3]]=record
store.transact_json(pathlib.Path(sys.argv[2]),update,{'version':1,'tasks':{}})
PY
if python3 -c 'import json,sys; r=(json.load(open(sys.argv[1])).get("tasks") or {}).get(sys.argv[2]) or {}; raise SystemExit(0 if r.get("issue_number") else 1)' "$MAPPING" "$TASK_UID"; then
  python3 "$SCRIPT_DIR/github-project-task.py" set-phase "$REPO_ROOT" \
    --task-uid "$TASK_UID" --phase main_sync --receipt-json "$RECEIPT_OUTPUT" --json >/dev/null \
    || die "failed to persist main_sync workflow phase"
fi
printf '%s\n' "$RECEIPT_OUTPUT"
