#!/usr/bin/env bash
# Cross-platform maintenance: keep this Bash entrypoint and its Python durable-store
# transaction compatible with the repository's supported POSIX and Windows shells.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

die() { echo "post-merge-main-sync: $*" >&2; exit 1; }
usage() {
  echo "Usage: $0 --repo-root <path> --main-ref <branch> --task-uid <uid> --pr-receipt <json> --receipt-output <json> [--patch-equivalence-receipt <json>]"
}

REPO_ROOT="" MAIN_REF="" TASK_UID="" PR_RECEIPT="" RECEIPT_OUTPUT="" PATCH_RECEIPT=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo-root) REPO_ROOT="${2:-}"; shift 2 ;;
    --main-ref) MAIN_REF="${2:-}"; shift 2 ;;
    --task-uid) TASK_UID="${2:-}"; shift 2 ;;
    --pr-receipt) PR_RECEIPT="${2:-}"; shift 2 ;;
    --patch-equivalence-receipt) PATCH_RECEIPT="${2:-}"; shift 2 ;;
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
if [[ -n "$PATCH_RECEIPT" ]]; then
  PATCH_RECEIPT="$(python3 "$SCRIPT_DIR/canonical-receipt-root.py" --default-worktree "$REPO_ROOT" --task-uid "$TASK_UID" --create --path "$PATCH_RECEIPT" --name patch-equivalence-receipt.json)" || die "noncanonical patch-equivalence receipt"
  [[ -f "$PATCH_RECEIPT" ]] || die "patch-equivalence receipt is unavailable"
fi
ACTUAL_BRANCH="$(git -C "$REPO_ROOT" symbolic-ref --quiet --short HEAD)" || die "repo root must be on a named branch"
[[ "$ACTUAL_BRANCH" == "$MAIN_REF" ]] || die "repo root is not checked out on the requested default branch"
MAPPING="$REPO_ROOT/.pm/github-project-sync/tasks.json"
[[ -f "$MAPPING" ]] || die "task mapping is unavailable"
# Validate receipt facts that do not depend on task-cache data before recovery
# can mutate the default mapping. Full task/receipt binding remains below.
python3 - "$PR_RECEIPT" "$MAIN_REF" <<'PY'
import datetime as d,json,sys
try:
 receipt=json.load(open(sys.argv[1],encoding='utf-8'))
 if not isinstance(receipt,dict): raise ValueError('receipt is not an object')
 if receipt.get('receipt_type')!='oasis7_pr_merge' or receipt.get('issuer')!='github_live_query' or receipt.get('evidence_mode')!='production' or receipt.get('state')!='MERGED': raise ValueError('invalid merge receipt')
 for key in ('repository','default_branch','pr_number','pr_url','merged_at','head_oid','base_ref','observed_at'):
  if receipt.get(key) in (None,''): raise ValueError(f'merge receipt is missing {key}')
 if receipt.get('base_ref')!=sys.argv[2] or receipt.get('default_branch')!=sys.argv[2]: raise ValueError('default branch identity mismatch')
 seen=d.datetime.fromisoformat(str(receipt['observed_at']).replace('Z','+00:00'))
 age=(d.datetime.now(d.timezone.utc)-seen).total_seconds()
 if age < -30 or age > 600: raise ValueError('merge receipt is stale')
except (OSError,json.JSONDecodeError,TypeError,ValueError) as exc:
 raise SystemExit(f'post-merge-main-sync: {exc}')
PY
# A terminal Project item may be absent from a freshly generated default cache.
# Recover only from the record retained by its registered canonical worktree;
# the helper validates complete task/receipt identity before atomic import.
python3 "$SCRIPT_DIR/recover-terminal-task-mapping.py" \
  --repo-root "$REPO_ROOT" --mapping "$MAPPING" --task-uid "$TASK_UID" \
  --main-ref "$MAIN_REF" --pr-receipt "$PR_RECEIPT" >/dev/null \
  || die "default task mapping recovery failed"
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
LOCAL_MAIN="$(git -C "$REPO_ROOT" rev-parse "refs/heads/$MAIN_REF")"
if [[ "$LOCAL_MAIN" != "$REMOTE_MAIN" ]]; then
  [[ -z "$(git -C "$REPO_ROOT" status --porcelain --untracked-files=all)" ]] || die "default-branch worktree is dirty; refusing branch update"
  git -C "$REPO_ROOT" merge --ff-only "$REMOTE_MAIN" >/dev/null || die "default branch cannot fast-forward to origin"
  LOCAL_MAIN="$(git -C "$REPO_ROOT" rev-parse "refs/heads/$MAIN_REF")"
fi
[[ "$LOCAL_MAIN" == "$REMOTE_MAIN" ]] || die "default branch did not synchronize exactly"
MERGED_HEAD="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1],encoding="utf-8")).get("head_oid") or "")' "$PR_RECEIPT")"
INTEGRATION_MODE="ancestry"
PATCH_RECEIPT_SHA=""
PATCH_ID=""
PROJECTED_TREE_OID=""
MAIN_TREE_OID=""
PATCH_MAIN_COMMIT=""
PATCH_MAIN_PARENT=""
if git -C "$REPO_ROOT" merge-base --is-ancestor "$MERGED_HEAD" "$LOCAL_MAIN"; then
  [[ -z "$PATCH_RECEIPT" ]] || die "patch-equivalence receipt is not accepted when synchronized main contains merged head"
else
  [[ -n "$PATCH_RECEIPT" ]] || die "synchronized default branch does not contain merged head and no patch-equivalence receipt was supplied"
  PATCH_FIELDS="$(python3 - "$PATCH_RECEIPT" "$MERGED_HEAD" <<'PY'
import json,sys
p=json.load(open(sys.argv[1],encoding='utf-8'))
if p.get('receipt_type')!='oasis7_patch_equivalence' or p.get('issuer')!='oasis7_patch_equivalence_helper':
 raise SystemExit('post-merge-main-sync: invalid patch-equivalence receipt type or issuer')
if p.get('branch_tip')!=sys.argv[2]:
 raise SystemExit('post-merge-main-sync: patch-equivalence receipt identity mismatch')
if p.get('schema_version') != 2 or not p.get('main_parent') or not p.get('patch_id') or not p.get('projected_tree_oid') or not p.get('main_tree_oid'):
 raise SystemExit('post-merge-main-sync: incomplete patch-equivalence receipt')
print(p['main_commit']); print(p['main_parent']); print(p['patch_id']); print(p['projected_tree_oid']); print(p['main_tree_oid'])
PY
)" || die "invalid patch-equivalence receipt"
  PATCH_MAIN_COMMIT="$(printf '%s\n' "$PATCH_FIELDS" | sed -n '1p')"
  PATCH_MAIN_PARENT="$(printf '%s\n' "$PATCH_FIELDS" | sed -n '2p')"
  PATCH_ID="$(printf '%s\n' "$PATCH_FIELDS" | sed -n '3p')"
  PROJECTED_TREE_OID="$(printf '%s\n' "$PATCH_FIELDS" | sed -n '4p')"
  MAIN_TREE_OID="$(printf '%s\n' "$PATCH_FIELDS" | sed -n '5p')"
  git -C "$REPO_ROOT" merge-base --is-ancestor "$PATCH_MAIN_COMMIT" "$LOCAL_MAIN" || die "patch-equivalence integration commit is not contained in synchronized main"
  git -C "$REPO_ROOT" rev-list --first-parent "$PATCH_MAIN_COMMIT" \
    | awk -v base="$PATCH_MAIN_PARENT" '$0==base { found=1 } END { exit !found }' \
    || die "patch-equivalence receipt main parent is not on the integration first-parent chain"
  PATCH_BASE="$(git -C "$REPO_ROOT" merge-base "$MERGED_HEAD" "$PATCH_MAIN_PARENT")"
  BRANCH_PATCH="$(git -C "$REPO_ROOT" diff "$PATCH_BASE..$MERGED_HEAD" | git patch-id --stable | awk '{print $1}')"
  [[ -n "$BRANCH_PATCH" && "$BRANCH_PATCH" == "$PATCH_ID" ]] || die "patch-equivalence branch identity failed recomputation"
  RECOMPUTED_PROJECTED_TREE="$(git -C "$REPO_ROOT" merge-tree --write-tree "$PATCH_MAIN_PARENT" "$MERGED_HEAD")" \
    || die "patch-equivalence branch projection conflicts"
  RECOMPUTED_MAIN_TREE="$(git -C "$REPO_ROOT" rev-parse "$PATCH_MAIN_COMMIT^{tree}")"
  [[ "$RECOMPUTED_PROJECTED_TREE" == "$RECOMPUTED_MAIN_TREE" && "$RECOMPUTED_PROJECTED_TREE" == "$PROJECTED_TREE_OID" && "$RECOMPUTED_MAIN_TREE" == "$MAIN_TREE_OID" ]] \
    || die "patch-equivalence projected tree failed recomputation"
  PATCH_RECEIPT_SHA="$(python3 - "$PATCH_RECEIPT" <<'PY'
import hashlib,pathlib,sys
print(hashlib.sha256(pathlib.Path(sys.argv[1]).read_bytes()).hexdigest())
PY
)"
  INTEGRATION_MODE="patch_equivalence"
fi

mkdir -p "$(dirname "$RECEIPT_OUTPUT")"
TMP_RECEIPT="$(mktemp "$(dirname "$RECEIPT_OUTPUT")/.main-sync.XXXXXX")"
trap 'rm -f "$TMP_RECEIPT"' EXIT
python3 - "$TMP_RECEIPT" "$MAPPING" "$TASK_UID" "$PR_RECEIPT" "$MAIN_REF" "$LOCAL_MAIN" "$INTEGRATION_MODE" "$PATCH_RECEIPT_SHA" "$PATCH_ID" "$PROJECTED_TREE_OID" "$MAIN_TREE_OID" "$PATCH_MAIN_COMMIT" "$PATCH_MAIN_PARENT" <<'PY'
import datetime as d,hashlib,json,pathlib,sys
mapping=json.load(open(sys.argv[2],encoding='utf-8')); record=(mapping.get('tasks') or {}).get(sys.argv[3]) or {}
receipt_path=pathlib.Path(sys.argv[4]); digest=hashlib.sha256(receipt_path.read_bytes()).hexdigest()
out={'receipt_type':'oasis7_main_sync','issuer':'post-merge-main-sync','task_uid':sys.argv[3],
     'repository':record['repository'],'default_branch':sys.argv[5],
     'main_commit':sys.argv[6],'remote_main_commit':sys.argv[6],
     'merge_receipt_sha256':digest,'integration_mode':sys.argv[7],
     'observed_at':d.datetime.now(d.timezone.utc).isoformat()}
if sys.argv[7]=='patch_equivalence':
 out['patch_equivalence_receipt_sha256']=sys.argv[8]; out['patch_id']=sys.argv[9]
 out['projected_tree_oid']=sys.argv[10]; out['main_tree_oid']=sys.argv[11]
 out['integration_commit']=sys.argv[12]; out['integration_parent']=sys.argv[13]
pathlib.Path(sys.argv[1]).write_text(json.dumps(out,indent=2,sort_keys=True)+'\n',encoding='utf-8')
PY
mv "$TMP_RECEIPT" "$RECEIPT_OUTPUT"
trap - EXIT
# {"workflow_phase":"main_sync"}
python3 - "$SCRIPT_DIR/workflow-durable-store.py" "$MAPPING" "$TASK_UID" "$RECEIPT_OUTPUT" "$PR_RECEIPT" <<'PY'
import hashlib,importlib.util,json,pathlib,sys
spec=importlib.util.spec_from_file_location('workflow_durable_store',sys.argv[1]); store=importlib.util.module_from_spec(spec); spec.loader.exec_module(store)
receipt_path=pathlib.Path(sys.argv[4]); receipt=json.loads(receipt_path.read_text())
merge_path=pathlib.Path(sys.argv[5]); merge_receipt=json.loads(merge_path.read_text())
merge_digest=hashlib.sha256(merge_path.read_bytes()).hexdigest()
IMMUTABLE_MERGE_IDENTITY=(
 'receipt_type','issuer','evidence_mode','repository','default_branch','pr_number',
 'pr_url','state','merged_at','head_oid','base_ref',
)
def update(data):
 record=(data.get('tasks') or {}).get(sys.argv[3]) or {}; record['workflow_phase']='main_sync'
 stored_receipt=record.get('merge_receipt'); stored_digest=record.get('merge_receipt_sha256')
 if stored_receipt is None:
  if stored_digest is not None: raise SystemExit('post-merge-main-sync: stored merge receipt conflicts with validated receipt')
 elif (not isinstance(stored_receipt,dict) or
       tuple(stored_receipt.get(key) for key in IMMUTABLE_MERGE_IDENTITY) !=
       tuple(merge_receipt.get(key) for key in IMMUTABLE_MERGE_IDENTITY)):
  raise SystemExit('post-merge-main-sync: stored merge receipt conflicts with validated receipt')
 # A fresh observation of the same immutable merge identity replaces both
 # receipt and digest within this one durable-store transaction.
 record['merge_receipt']=merge_receipt; record['merge_receipt_sha256']=merge_digest
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
