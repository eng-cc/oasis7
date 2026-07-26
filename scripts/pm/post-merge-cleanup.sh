#!/usr/bin/env bash
# This script must remain cross-platform across Windows and Linux/macOS.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DURABLE_STORE="$SCRIPT_DIR/workflow-durable-store.py"
journal_write() { python3 "$DURABLE_STORE" write-journal --path "$1" --json "$2"; }

# Git for Windows porcelain uses forward slashes while native Python resolves
# Windows paths with backslashes. Normalize before comparing worktree identity.
normalize_path_identity() {
  python3 -c 'import os,sys; path=os.path.normcase(os.path.realpath(sys.argv[1])); print(path.replace("\\", "/") if os.name == "nt" else path)' "$1"
}

worktree_is_registered() {
  local repo_root="$1" worktree="$2" entry
  while IFS= read -r entry; do
    [[ "$entry" == "worktree "* ]] || continue
    [[ "$(normalize_path_identity "${entry#worktree }")" == "$worktree" ]] && return 0
  done < <(git -C "$repo_root" worktree list --porcelain)
  return 1
}

die() { echo "post-merge-cleanup: $*" >&2; exit 1; }
usage() {
  echo "Usage: $0 --repo-root <path> --worktree <path> --branch <name> --main-ref <ref> --task-uid <uid> --pr-receipt <json> --main-sync-receipt <json> --terminal-receipt-output <json> [--patch-equivalence-receipt <json>] [--dry-run]"
}

REPO_ROOT="" WORKTREE="" BRANCH="" MAIN_REF="" TASK_UID="" PR_RECEIPT="" MAIN_SYNC_RECEIPT="" TERMINAL_RECEIPT_OUTPUT="" PATCH_RECEIPT="" DRY_RUN=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo-root) REPO_ROOT="${2:-}"; shift 2 ;;
    --worktree) WORKTREE="${2:-}"; shift 2 ;;
    --branch) BRANCH="${2:-}"; shift 2 ;;
    --main-ref) MAIN_REF="${2:-}"; shift 2 ;;
    --task-uid) TASK_UID="${2:-}"; shift 2 ;;
    --pr-receipt) PR_RECEIPT="${2:-}"; shift 2 ;;
    --main-sync-receipt) MAIN_SYNC_RECEIPT="${2:-}"; shift 2 ;;
    --terminal-receipt-output) TERMINAL_RECEIPT_OUTPUT="${2:-}"; shift 2 ;;
    --patch-equivalence-receipt) PATCH_RECEIPT="${2:-}"; shift 2 ;;
    --dry-run) DRY_RUN=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) die "unknown argument: $1" ;;
  esac
done
for name in REPO_ROOT WORKTREE BRANCH MAIN_REF TASK_UID PR_RECEIPT MAIN_SYNC_RECEIPT; do
  [[ -n "${!name}" ]] || die "missing --$(printf '%s' "$name" | tr '[:upper:]_' '[:lower:]-')"
done
[[ "$DRY_RUN" == "1" || -n "$TERMINAL_RECEIPT_OUTPUT" ]] || die "missing --terminal-receipt-output"
# Crash journal schema: oasis7_cleanup_intent with monotonic states
# worktree_removed, branch_deleted, terminal_receipt_committed. A retry may
# accept an already missing worktree/branch only after this matching journal
# and fresh live merge/main-sync proof have been revalidated.
# Read the complete cached identity before any network call, fetch, worktree
# inspection, or cleanup effect.  Partial terminal authority is never repaired
# from caller flags.
# Durable terminal authority always lives in the canonical default worktree.
# The task worktree is an input to validate/remove, never a post-effect sink.
MAPPING="$REPO_ROOT/.pm/github-project-sync/tasks.json"
[[ -f "$MAPPING" ]] || die "default-worktree task mapping is unavailable; run refresh-task-cache.sh from --repo-root before cleanup"
# Canonical receipt-root validation precedes every external or destructive effect.
PR_RECEIPT="$(python3 "$SCRIPT_DIR/canonical-receipt-root.py" --default-worktree "$REPO_ROOT" --task-uid "$TASK_UID" --create --path "$PR_RECEIPT" --name merge-receipt.json)" || die "noncanonical merge receipt"
MAIN_SYNC_RECEIPT="$(python3 "$SCRIPT_DIR/canonical-receipt-root.py" --default-worktree "$REPO_ROOT" --task-uid "$TASK_UID" --create --path "$MAIN_SYNC_RECEIPT" --name main-sync-receipt.json)" || die "noncanonical main-sync receipt"
if [[ -n "$PATCH_RECEIPT" ]]; then
  PATCH_RECEIPT="$(python3 "$SCRIPT_DIR/canonical-receipt-root.py" --default-worktree "$REPO_ROOT" --task-uid "$TASK_UID" --create --path "$PATCH_RECEIPT" --name patch-equivalence-receipt.json)" || die "noncanonical patch-equivalence receipt"
  [[ -f "$PATCH_RECEIPT" ]] || die "patch-equivalence receipt is unavailable"
fi
if [[ "$DRY_RUN" == "0" ]]; then
  TERMINAL_RECEIPT_OUTPUT="$(python3 "$SCRIPT_DIR/canonical-receipt-root.py" --default-worktree "$REPO_ROOT" --task-uid "$TASK_UID" --create --path "$TERMINAL_RECEIPT_OUTPUT" --name terminal-cleanup-receipt.json)" || die "noncanonical terminal cleanup receipt"
fi
python3 - "$MAPPING" "$TASK_UID" <<'PY'
import json,sys
r=(json.load(open(sys.argv[1],encoding='utf-8')).get('tasks') or {}).get(sys.argv[2]) or {}
for key in ('repository','canonical_worktree','task_branch','default_branch'):
    if not str(r.get(key) or '').strip():
        raise SystemExit(f'post-merge-cleanup: task truth is missing required {key}; branch tip/head_oid/patch equivalence validation cannot start')
PY
TASK_FIELDS="$(python3 - "$MAPPING" "$TASK_UID" <<'PY'
import json,sys
r=(json.load(open(sys.argv[1],encoding='utf-8')).get('tasks') or {}).get(sys.argv[2]) or {}
print(r.get('pr_number') or '')
print(r.get('canonical_worktree') or '')
print(r.get('task_branch') or '')
print(r.get('repository') or '')
print(r.get('default_branch') or '')
PY
)"
RECORDED_PR_NUMBER="$(printf '%s\n' "$TASK_FIELDS" | sed -n '1p')"
RECORDED_WORKTREE="$(printf '%s\n' "$TASK_FIELDS" | sed -n '2p')"
RECORDED_BRANCH="$(printf '%s\n' "$TASK_FIELDS" | sed -n '3p')"
RECORDED_REPOSITORY="$(printf '%s\n' "$TASK_FIELDS" | sed -n '4p')"
RECORDED_DEFAULT_BRANCH="$(printf '%s\n' "$TASK_FIELDS" | sed -n '5p')"
[[ -n "$RECORDED_PR_NUMBER" ]] || die "task truth has no recorded PR"
if [[ "$DRY_RUN" == "0" ]]; then
  # TERMINAL_RECEIPT_OUTPUT passes Path.is_absolute and resolved
  # relative_to(canonical_worktree) rejection before intent journal access.
  TERMINAL_RECEIPT_OUTPUT="$(python3 "$SCRIPT_DIR/validate-durable-terminal-path.py" \
    --mapping "$MAPPING" --task-uid "$TASK_UID" --path "$TERMINAL_RECEIPT_OUTPUT" \
    --label "terminal cleanup receipt output")" || die "invalid durable terminal receipt path"
fi
RECORDED_WORKTREE="$(normalize_path_identity "$RECORDED_WORKTREE")"
WORKTREE="$(normalize_path_identity "$WORKTREE")"
[[ "$WORKTREE" == "$RECORDED_WORKTREE" ]] || die "caller worktree disagrees with canonical task truth"
[[ "$MAIN_REF" == "$RECORDED_DEFAULT_BRANCH" ]] || die "caller main ref disagrees with canonical task truth"
git -C "$REPO_ROOT" rev-parse --is-inside-work-tree >/dev/null || die "invalid repo root"
REPO_ROOT="$(git -C "$REPO_ROOT" rev-parse --show-toplevel)"

INTENT_JOURNAL="$(dirname "$TERMINAL_RECEIPT_OUTPUT")/cleanup-intent.json"
INTENT_STATE="0 0 0"
if [[ -f "$INTENT_JOURNAL" ]]; then
  INTENT_STATE="$(python3 - "$INTENT_JOURNAL" "$TASK_UID" "$RECORDED_REPOSITORY" "$WORKTREE" "$BRANCH" <<'PY'
import json,sys
r=json.load(open(sys.argv[1],encoding='utf-8'))
expected={'receipt_type':'oasis7_cleanup_intent','task_uid':sys.argv[2],'repository':sys.argv[3],'worktree':sys.argv[4],'branch':sys.argv[5]}
for key,value in expected.items():
 if r.get(key)!=value: raise SystemExit(f'post-merge-cleanup: cleanup intent mismatch on retry: {key}')
print(int(bool(r.get('worktree_removed'))),int(bool(r.get('branch_deleted'))),int(bool(r.get('terminal_receipt_committed'))))
PY
)" || die "cleanup intent validation failed"
fi
WORKTREE_REMOVED="$(printf '%s' "$INTENT_STATE" | awk '{print $1}')"
BRANCH_DELETED="$(printf '%s' "$INTENT_STATE" | awk '{print $2}')"
TERMINAL_COMMITTED="$(printf '%s' "$INTENT_STATE" | awk '{print $3}')"
if [[ "$WORKTREE_REMOVED" == 1 ]]; then
  [[ ! -e "$WORKTREE" ]] || die "cleanup journal says worktree_removed but path still exists"
  ! worktree_is_registered "$REPO_ROOT" "$WORKTREE" \
    || die "cleanup journal says worktree_removed but git still registers it"
  ACTUAL_BRANCH="$RECORDED_BRANCH"
  BRANCH_TIP="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("head_oid") or "")' "$PR_RECEIPT")"
else
  [[ -e "$WORKTREE" ]] || die "worktree path is absent and no matching cleanup intent proves removal"
  git -C "$WORKTREE" rev-parse --is-inside-work-tree >/dev/null 2>&1 || die "worktree path is not a git worktree"
  worktree_is_registered "$REPO_ROOT" "$WORKTREE" || die "task worktree is not registered under repo root"
  WORKTREE_COMMON_DIR="$(cd "$WORKTREE" && cd "$(git rev-parse --git-common-dir)" && pwd -P)" \
    || die "task worktree common-dir cannot be resolved"
  REPO_COMMON_DIR="$(cd "$REPO_ROOT" && cd "$(git rev-parse --git-common-dir)" && pwd -P)" \
    || die "canonical repository common-dir cannot be resolved"
  [[ "$WORKTREE_COMMON_DIR" == "$REPO_COMMON_DIR" ]] || die "task worktree common-dir mismatch against canonical repository"
  [[ -z "$(git -C "$WORKTREE" status --porcelain --untracked-files=all)" ]] || die "task worktree is dirty"
  ACTUAL_BRANCH="$(git -C "$WORKTREE" symbolic-ref --quiet --short HEAD)" || die "task worktree must be on a named branch"
  [[ "$BRANCH" == "$ACTUAL_BRANCH" ]] || die "caller branch disagrees with canonical task worktree branch"
  BRANCH_TIP="$(git -C "$REPO_ROOT" rev-parse "refs/heads/$ACTUAL_BRANCH")"
fi
[[ "$ACTUAL_BRANCH" == "$RECORDED_BRANCH" ]] || die "branch identity disagrees with canonical task truth"
if [[ "$BRANCH_DELETED" == 1 ]]; then
  ! git -C "$REPO_ROOT" show-ref --verify --quiet "refs/heads/$BRANCH" \
    || die "cleanup journal says branch_deleted but branch still exists"
elif [[ "$WORKTREE_REMOVED" == 1 ]]; then
  git -C "$REPO_ROOT" show-ref --verify --quiet "refs/heads/$BRANCH" \
    || die "cleanup journal does not prove branch deletion and branch is already missing"
fi
LIVE_RECEIPT="$(mktemp)"
trap 'rm -f "$LIVE_RECEIPT"' EXIT
(cd "$REPO_ROOT" && python3 "$SCRIPT_DIR/pr-merge-receipt.py" "$RECORDED_PR_NUMBER" --json) >"$LIVE_RECEIPT" || die "fresh live merged PR query failed"
python3 - "$PR_RECEIPT" "$LIVE_RECEIPT" "$RECORDED_REPOSITORY" <<'PY'
import json,sys
a=json.load(open(sys.argv[1],encoding='utf-8')); b=json.load(open(sys.argv[2],encoding='utf-8'))
for key in ('receipt_type','issuer','pr_number','pr_url','state','head_oid','merged_at'):
 if a.get(key)!=b.get(key): raise SystemExit(f'post-merge-cleanup: supplied receipt disagrees with fresh live query: {key}')
if b.get('evidence_mode') == 'production':
 if a.get('evidence_mode') != 'production': raise SystemExit('post-merge-cleanup: supplied receipt is not production evidence')
 for key in ('repository','default_branch','base_ref'):
  if a.get(key)!=b.get(key): raise SystemExit(f'post-merge-cleanup: supplied receipt disagrees with fresh live query: {key}')
 if sys.argv[3] and b.get('repository') != sys.argv[3]: raise SystemExit('post-merge-cleanup: merge receipt repository disagrees with task truth')
elif sys.argv[3]:
 raise SystemExit('post-merge-cleanup: untrusted fixture receipt is forbidden for repository-bound task truth')
PY
RECEIPT_DEFAULT_BRANCH="$(python3 - "$LIVE_RECEIPT" <<'PY'
import json,sys
r=json.load(open(sys.argv[1],encoding='utf-8'))
print(r.get('default_branch') or '')
PY
)"
[[ -n "$RECEIPT_DEFAULT_BRANCH" && "$MAIN_REF" == "$RECEIPT_DEFAULT_BRANCH" ]] || die "caller main ref disagrees with repository default branch"
[[ "$RECEIPT_DEFAULT_BRANCH" == "$RECORDED_DEFAULT_BRANCH" ]] || die "merge receipt default branch disagrees with task truth"
RECEIPT_BASE_REF="$(python3 - "$LIVE_RECEIPT" <<'PY'
import json,sys
print(json.load(open(sys.argv[1],encoding='utf-8')).get('base_ref') or '')
PY
)"
[[ "$RECEIPT_BASE_REF" == "$RECEIPT_DEFAULT_BRANCH" ]] || die "merged PR did not target the repository default branch"
python3 - "$MAIN_SYNC_RECEIPT" "$PR_RECEIPT" "$TASK_UID" "$RECORDED_REPOSITORY" "$MAIN_REF" <<'PY'
import datetime as d,hashlib,json,os,pathlib,sys
path=pathlib.Path(sys.argv[1])
if not path.is_file(): raise SystemExit('post-merge-cleanup: main-sync receipt is unavailable')
r=json.loads(path.read_text(encoding='utf-8'))
if r.get('receipt_type')!='oasis7_main_sync' or r.get('issuer')!='post-merge-main-sync': raise SystemExit('post-merge-cleanup: invalid main-sync receipt')
for key,expected in (('task_uid',sys.argv[3]),('repository',sys.argv[4]),('default_branch',sys.argv[5])):
 if r.get(key)!=expected: raise SystemExit(f'post-merge-cleanup: main-sync receipt {key} mismatch')
if r.get('merge_receipt_sha256')!=hashlib.sha256(pathlib.Path(sys.argv[2]).read_bytes()).hexdigest(): raise SystemExit('post-merge-cleanup: main-sync receipt is not bound to merge receipt')
if r.get('integration_mode') not in ('ancestry','patch_equivalence'): raise SystemExit('post-merge-cleanup: main-sync receipt integration mode is invalid')
seen=d.datetime.fromisoformat(str(r.get('observed_at')).replace('Z','+00:00'))
age=(d.datetime.now(d.timezone.utc)-seen).total_seconds()
if age < -30 or age > 600: raise SystemExit('post-merge-cleanup: main-sync receipt is stale')
PY
RECEIPT_HEAD="$(python3 - "$MAPPING" "$TASK_UID" "$PR_RECEIPT" <<'PY'
import datetime as d,json, sys
from pathlib import Path
p = Path(sys.argv[1])
if not p.is_file():
    raise SystemExit("post-merge-cleanup: task mapping is unavailable")
record = (json.loads(p.read_text(encoding="utf-8")).get("tasks") or {}).get(sys.argv[2]) or {}
if record.get("status") != "done":
    raise SystemExit("post-merge-cleanup: task status must be done")
receipt=json.load(open(sys.argv[3],encoding='utf-8'))
if receipt.get('receipt_type')!='oasis7_pr_merge' or receipt.get('issuer')!='github_live_query' or receipt.get('state')!='MERGED': raise SystemExit('post-merge-cleanup: invalid fresh PR receipt')
if str(receipt.get('pr_number'))!=str(record.get('pr_number')) or receipt.get('pr_url')!=record.get('pr_url'): raise SystemExit('post-merge-cleanup: PR receipt does not match task truth')
seen=d.datetime.fromisoformat(str(receipt.get('observed_at')).replace('Z','+00:00'))
if (d.datetime.now(d.timezone.utc)-seen).total_seconds()>600: raise SystemExit('post-merge-cleanup: PR receipt is stale')
print(receipt.get('head_oid') or '')
PY
)"
[[ -n "$RECEIPT_HEAD" && "$RECEIPT_HEAD" == "$BRANCH_TIP" ]] || die "fresh PR receipt is not bound to current task branch tip"
MAIN_COMMIT="$(git -C "$REPO_ROOT" rev-parse "refs/heads/$RECEIPT_DEFAULT_BRANCH")" || die "local default branch is unavailable"
SYNC_MAIN_COMMIT="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1],encoding="utf-8")).get("main_commit") or "")' "$MAIN_SYNC_RECEIPT")"
[[ -n "$SYNC_MAIN_COMMIT" && "$MAIN_COMMIT" == "$SYNC_MAIN_COMMIT" ]] || die "local default branch moved after main-sync receipt"
if git -C "$REPO_ROOT" remote get-url origin >/dev/null 2>&1; then
  git -C "$REPO_ROOT" fetch --quiet origin "refs/heads/$RECEIPT_DEFAULT_BRANCH:refs/remotes/origin/$RECEIPT_DEFAULT_BRANCH" || die "failed to refresh origin default branch"
  REMOTE_MAIN="$(git -C "$REPO_ROOT" rev-parse "refs/remotes/origin/$RECEIPT_DEFAULT_BRANCH")"
  [[ "$MAIN_COMMIT" == "$REMOTE_MAIN" ]] || die "local default branch is not synchronized with origin"
fi
if ! git -C "$REPO_ROOT" merge-base --is-ancestor "$BRANCH_TIP" "$MAIN_COMMIT"; then
  [[ -n "$PATCH_RECEIPT" && -f "$PATCH_RECEIPT" ]] || die "$MAIN_REF does not contain task branch tip and no patch-equivalence receipt was supplied"
  SYNC_PATCH_FIELDS="$(python3 - "$MAIN_SYNC_RECEIPT" "$PATCH_RECEIPT" <<'PY'
import hashlib,json,pathlib,sys
r=json.load(open(sys.argv[1],encoding='utf-8'))
if r.get('integration_mode')!='patch_equivalence': raise SystemExit('post-merge-cleanup: main-sync receipt did not authorize patch equivalence')
digest=hashlib.sha256(pathlib.Path(sys.argv[2]).read_bytes()).hexdigest()
if r.get('patch_equivalence_receipt_sha256')!=digest: raise SystemExit('post-merge-cleanup: patch-equivalence receipt digest mismatch')
print(r.get('patch_id') or ''); print(r.get('projected_tree_oid') or ''); print(r.get('main_tree_oid') or '')
print(r.get('integration_commit') or ''); print(r.get('integration_parent') or '')
PY
)" || die "main-sync patch-equivalence binding failed"
  SYNC_PATCH_ID="$(printf '%s\n' "$SYNC_PATCH_FIELDS" | sed -n '1p')"
  SYNC_PROJECTED_TREE="$(printf '%s\n' "$SYNC_PATCH_FIELDS" | sed -n '2p')"
  SYNC_MAIN_TREE="$(printf '%s\n' "$SYNC_PATCH_FIELDS" | sed -n '3p')"
  SYNC_INTEGRATION_COMMIT="$(printf '%s\n' "$SYNC_PATCH_FIELDS" | sed -n '4p')"
  SYNC_INTEGRATION_PARENT="$(printf '%s\n' "$SYNC_PATCH_FIELDS" | sed -n '5p')"
  PATCH_FIELDS="$(python3 - "$PATCH_RECEIPT" "$BRANCH_TIP" "$SYNC_INTEGRATION_COMMIT" <<'PY'
import json,sys
p=json.load(open(sys.argv[1],encoding='utf-8'))
if p.get('receipt_type')!='oasis7_patch_equivalence' or p.get('schema_version')!=2 or p.get('issuer')!='oasis7_patch_equivalence_helper' or p.get('branch_tip')!=sys.argv[2] or p.get('main_commit')!=sys.argv[3] or not p.get('patch_id') or not p.get('projected_tree_oid') or not p.get('main_tree_oid'):
 raise SystemExit('post-merge-cleanup: invalid patch-equivalence receipt')
print(p.get('main_parent',''))
print(p['patch_id'])
print(p['projected_tree_oid'])
print(p['main_tree_oid'])
PY
)"
  MAIN_PARENT="$(printf '%s\n' "$PATCH_FIELDS" | sed -n '1p')"
  EXPECTED_PATCH="$(printf '%s\n' "$PATCH_FIELDS" | sed -n '2p')"
  EXPECTED_PROJECTED_TREE="$(printf '%s\n' "$PATCH_FIELDS" | sed -n '3p')"
  EXPECTED_MAIN_TREE="$(printf '%s\n' "$PATCH_FIELDS" | sed -n '4p')"
  [[ "$EXPECTED_PATCH" == "$SYNC_PATCH_ID" && "$EXPECTED_PROJECTED_TREE" == "$SYNC_PROJECTED_TREE" && "$EXPECTED_MAIN_TREE" == "$SYNC_MAIN_TREE" ]] || die "main-sync patch proof disagrees with patch-equivalence receipt"
  [[ "$MAIN_PARENT" == "$SYNC_INTEGRATION_PARENT" ]] || die "main-sync integration parent disagrees with patch-equivalence receipt"
  git -C "$REPO_ROOT" merge-base --is-ancestor "$SYNC_INTEGRATION_COMMIT" "$MAIN_COMMIT" || die "patch-equivalence integration commit is not contained in synchronized main"
  git -C "$REPO_ROOT" rev-list --first-parent "$SYNC_INTEGRATION_COMMIT" \
    | awk -v base="$MAIN_PARENT" '$0==base { found=1 } END { exit !found }' \
    || die "patch-equivalence receipt main parent is not on the integration first-parent chain"
  BASE="$(git -C "$REPO_ROOT" merge-base "$BRANCH_TIP" "$MAIN_PARENT")"
  BRANCH_PATCH="$(git -C "$REPO_ROOT" diff "$BASE..$BRANCH_TIP" | git patch-id --stable | awk '{print $1}')"
  [[ -n "$BRANCH_PATCH" && "$BRANCH_PATCH" == "$EXPECTED_PATCH" ]] || die "patch-equivalence branch identity failed recomputation"
  RECOMPUTED_PROJECTED_TREE="$(git -C "$REPO_ROOT" merge-tree --write-tree "$MAIN_PARENT" "$BRANCH_TIP")" \
    || die "patch-equivalence branch projection conflicts"
  RECOMPUTED_MAIN_TREE="$(git -C "$REPO_ROOT" rev-parse "$SYNC_INTEGRATION_COMMIT^{tree}")"
  [[ "$RECOMPUTED_PROJECTED_TREE" == "$RECOMPUTED_MAIN_TREE" && "$RECOMPUTED_PROJECTED_TREE" == "$EXPECTED_PROJECTED_TREE" && "$RECOMPUTED_MAIN_TREE" == "$EXPECTED_MAIN_TREE" ]] \
    || die "patch-equivalence projected tree failed recomputation"
else
  [[ "$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("integration_mode") or "")' "$MAIN_SYNC_RECEIPT")" == "ancestry" ]] || die "ancestry cleanup requires ancestry main-sync receipt"
  [[ -z "$PATCH_RECEIPT" ]] || die "patch-equivalence receipt is not accepted when main contains the task branch tip"
fi

printf 'git -C %q worktree remove %q\n' "$REPO_ROOT" "$WORKTREE"
printf 'git -C %q branch -d %q\n' "$REPO_ROOT" "$BRANCH"
if [[ "$DRY_RUN" == "0" ]]; then
  INTENT_JOURNAL="$(dirname "$TERMINAL_RECEIPT_OUTPUT")/cleanup-intent.json"
  JOURNAL_JSON="$(python3 - "$INTENT_JOURNAL" "$TASK_UID" "$RECORDED_REPOSITORY" "$WORKTREE" "$BRANCH" <<'PY'
import json,pathlib,sys
p=pathlib.Path(sys.argv[1]); expected={"receipt_type":"oasis7_cleanup_intent","task_uid":sys.argv[2],
 "repository":sys.argv[3],"worktree":sys.argv[4],"branch":sys.argv[5]}
if p.exists():
 old=json.loads(p.read_text());
 if any(old.get(k)!=v for k,v in expected.items()): raise SystemExit('post-merge-cleanup: cleanup intent mismatch on retry')
 expected.update({k:bool(old.get(k)) for k in ('worktree_removed','branch_deleted','terminal_receipt_committed')})
else: expected.update(worktree_removed=False,branch_deleted=False,terminal_receipt_committed=False)
expected['revision']=int((json.loads(p.read_text()).get('revision',0) if p.exists() else 0))+1
print(json.dumps(expected))
PY
)"; journal_write "$INTENT_JOURNAL" "$JOURNAL_JSON"
  if [[ "$WORKTREE_REMOVED" != 1 ]]; then
    git -C "$REPO_ROOT" worktree remove "$WORKTREE"
    JOURNAL_JSON="$(python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); d["worktree_removed"]=True; d["revision"]+=1; print(json.dumps(d))' "$INTENT_JOURNAL")"; journal_write "$INTENT_JOURNAL" "$JOURNAL_JSON"
  fi
  if [[ "$BRANCH_DELETED" != 1 ]]; then
    git -C "$REPO_ROOT" branch -d "$BRANCH"
    JOURNAL_JSON="$(python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); d["branch_deleted"]=True; d["revision"]+=1; print(json.dumps(d))' "$INTENT_JOURNAL")"; journal_write "$INTENT_JOURNAL" "$JOURNAL_JSON"
  fi
  mkdir -p "$(dirname "$TERMINAL_RECEIPT_OUTPUT")"
  TMP_TERMINAL="$(mktemp "$(dirname "$TERMINAL_RECEIPT_OUTPUT")/.terminal-cleanup.XXXXXX")"
  python3 - "$TMP_TERMINAL" "$MAPPING" "$TASK_UID" "$PR_RECEIPT" "$MAIN_SYNC_RECEIPT" "$RECORDED_REPOSITORY" "$RECORDED_PR_NUMBER" "$WORKTREE" "$BRANCH" <<'PY'
import datetime as d,hashlib,json,os,pathlib,sys
mapping=json.load(open(sys.argv[2],encoding='utf-8')); record=(mapping.get('tasks') or {}).get(sys.argv[3]) or {}
merge=pathlib.Path(sys.argv[4]); sync=pathlib.Path(sys.argv[5])
out={"receipt_type":"oasis7_terminal_cleanup","issuer":"post-merge-cleanup",
 "task_uid":sys.argv[3],"repository":sys.argv[6],"issue_number":record.get("issue_number"),
 "pr_number":int(sys.argv[7]),"worktree":sys.argv[8],"branch":sys.argv[9],
 "merge_receipt_sha256":hashlib.sha256(merge.read_bytes()).hexdigest(),
 "main_sync_receipt_sha256":hashlib.sha256(sync.read_bytes()).hexdigest(),
 "observed_at":d.datetime.now(d.timezone.utc).isoformat()}
path=pathlib.Path(sys.argv[1])
with path.open('w',encoding='utf-8') as stream:
 json.dump(out,stream,indent=2,sort_keys=True); stream.write('\n'); stream.flush(); os.fsync(stream.fileno())
PY
  # The helper performs flush/fsync, atomic replace, then parent-directory
  # fsync; only after that durable sequence may the journal become committed.
  python3 "$DURABLE_STORE" replace-json-file --path "$TERMINAL_RECEIPT_OUTPUT" --json-file "$TMP_TERMINAL"
  rm -f "$TMP_TERMINAL"
  JOURNAL_JSON="$(python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); d["terminal_receipt_committed"]=True; d["revision"]+=1; print(json.dumps(d))' "$INTENT_JOURNAL")"; journal_write "$INTENT_JOURNAL" "$JOURNAL_JSON"
  printf 'python3 %q --repo-root %q --task-uid %q --terminal-receipt %q\n' \
    "$REPO_ROOT/scripts/pm/post-merge-finalize.py" "$REPO_ROOT" "$TASK_UID" "$TERMINAL_RECEIPT_OUTPUT"
fi
