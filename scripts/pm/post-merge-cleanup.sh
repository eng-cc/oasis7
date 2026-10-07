#!/usr/bin/env bash
# This script must remain cross-platform across Windows and Linux/macOS.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DURABLE_STORE="$SCRIPT_DIR/workflow-durable-store.py"
journal_write() { python3 "$DURABLE_STORE" write-journal --path "$1" --json "$2"; }

# The UID-bound v2 path accepts no caller-selected resource identities. Keep
# the existing direct arguments below as the historical-v1 adapter.
for cleanup_arg in "$@"; do
  if [[ "$cleanup_arg" == "--delivery" ]]; then
    exec python3 "$SCRIPT_DIR/resource-cleanup-executor.py" "$@"
  fi
done

record_remote_branch_blocker() {
  local observed_tip="$1" blocker_json
  [[ -f "$INTENT_JOURNAL" ]] || die "cannot persist remote branch blocker without cleanup intent"
  blocker_json="$(python3 - "$INTENT_JOURNAL" "$TASK_UID" "$RECORDED_REPOSITORY" "$WORKTREE" "$BRANCH" "$BRANCH_TIP" "$observed_tip" <<'PY'
import datetime,json,pathlib,sys
path=pathlib.Path(sys.argv[1]); task_uid,repository,worktree,branch,expected_tip,observed_tip=sys.argv[2:]
journal=json.loads(path.read_text(encoding='utf-8'))
identity={'task_uid':task_uid,'repository':repository,'worktree':worktree,'branch':branch}
for key,value in identity.items():
    if journal.get(key)!=value:
        raise SystemExit(f'post-merge-cleanup: cannot persist blocker; cleanup intent {key} mismatch')
if journal.get('branch_tip')!=expected_tip:
    raise SystemExit('post-merge-cleanup: cannot persist blocker; cleanup intent branch tip mismatch')
now=datetime.datetime.now(datetime.timezone.utc).isoformat()
blocker=journal.get('remote_branch_blocker')
history=journal.get('remote_branch_blocker_history') or []
if not isinstance(history,list):
    raise SystemExit('post-merge-cleanup: remote branch blocker history is malformed')
if blocker is not None:
    if not isinstance(blocker,dict) or any(blocker.get(key)!=value for key,value in {
        'schema':'oasis7_cleanup_blocker_v1','kind':'remote_branch_tip_mismatch',
        'branch':branch,'expected_tip':expected_tip,
    }.items()):
        raise SystemExit('post-merge-cleanup: existing remote branch blocker identity mismatch')
    if blocker.get('resolved') is True:
        history.append(blocker)
        blocker=None
if blocker is None:
    blocker={'schema':'oasis7_cleanup_blocker_v1','kind':'remote_branch_tip_mismatch',
        'branch':branch,'expected_tip':expected_tip,'observed_tip':observed_tip,
        'observed_tips':[observed_tip],'resolved':False,'created_at':now,'updated_at':now}
else:
    observed_tips=blocker.get('observed_tips')
    if not isinstance(observed_tips,list):
        raise SystemExit('post-merge-cleanup: remote branch blocker observations are malformed')
    if observed_tip not in observed_tips:
        observed_tips.append(observed_tip)
    blocker.update(observed_tip=observed_tip,observed_tips=observed_tips,updated_at=now)
journal['remote_branch_blocker']=blocker
if history:
    journal['remote_branch_blocker_history']=history
journal['revision']=int(journal.get('revision',0))+1
print(json.dumps(journal))
PY
)" || die "remote branch blocker persistence failed"
  journal_write "$INTENT_JOURNAL" "$blocker_json" || die "remote branch blocker persistence failed"
}

resolve_remote_branch_blocker() {
  local resolution="$1" resolved_tip="$2" resolution_json
  [[ -f "$INTENT_JOURNAL" ]] || return 0
  resolution_json="$(python3 - "$INTENT_JOURNAL" "$TASK_UID" "$RECORDED_REPOSITORY" "$WORKTREE" "$BRANCH" "$BRANCH_TIP" "$resolution" "$resolved_tip" <<'PY'
import datetime,json,pathlib,sys
path=pathlib.Path(sys.argv[1]); task_uid,repository,worktree,branch,expected_tip,resolution,resolved_tip=sys.argv[2:]
journal=json.loads(path.read_text(encoding='utf-8'))
identity={'task_uid':task_uid,'repository':repository,'worktree':worktree,'branch':branch}
for key,value in identity.items():
    if journal.get(key)!=value:
        raise SystemExit(f'post-merge-cleanup: cannot resolve blocker; cleanup intent {key} mismatch')
if journal.get('branch_tip')!=expected_tip:
    raise SystemExit('post-merge-cleanup: cannot resolve blocker; cleanup intent branch tip mismatch')
blocker=journal.get('remote_branch_blocker')
if blocker is None:
    print('')
    raise SystemExit(0)
if not isinstance(blocker,dict):
    raise SystemExit('post-merge-cleanup: cannot resolve malformed remote branch blocker')
if blocker.get('resolved') is True:
    print('')
    raise SystemExit(0)
if any(blocker.get(key)!=value for key,value in {
    'schema':'oasis7_cleanup_blocker_v1','kind':'remote_branch_tip_mismatch',
    'branch':branch,'expected_tip':expected_tip,
}.items()):
    raise SystemExit('post-merge-cleanup: cannot resolve remote branch blocker identity mismatch')
blocker.update(resolved=True,resolution=resolution,resolved_tip=resolved_tip,
    resolved_at=datetime.datetime.now(datetime.timezone.utc).isoformat())
journal['revision']=int(journal.get('revision',0))+1
print(json.dumps(journal))
PY
)" || die "remote branch blocker resolution failed"
  [[ -n "$resolution_json" ]] || return 0
  journal_write "$INTENT_JOURNAL" "$resolution_json" || die "remote branch blocker resolution failed"
}

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

legacy_worktree_operation() {
  local mode="$1" result
  result="$(python3 - "$SCRIPT_DIR" "$REPO_ROOT" "$MAPPING" "$TASK_UID" "$WORKTREE" "$BRANCH" "$BRANCH_TIP" "$mode" <<'PY'
import importlib.util,json,pathlib,sys
script_dir,repo,mapping,uid,path,branch,tip,mode=sys.argv[1:]
sys.path.insert(0,script_dir)
spec=importlib.util.spec_from_file_location('resource_cleanup_executor',pathlib.Path(script_dir)/'resource-cleanup-executor.py')
if spec is None or spec.loader is None: raise SystemExit('resource cleanup safety helper is unavailable')
module=importlib.util.module_from_spec(spec); sys.modules[spec.name]=module; spec.loader.exec_module(module)
mapping_path=pathlib.Path(mapping); repo_path=pathlib.Path(repo).resolve(); target=pathlib.Path(path).resolve()
records=(json.loads(mapping_path.read_text(encoding='utf-8')).get('tasks') or {})
record=records.get(uid) or {}; registration=record.get('worktree_registration')
if not isinstance(registration,dict) or set(registration)!={'common_dir','admin_dir','instance_id'}:
 raise SystemExit('task worktree has no trusted registration-instance binding; retaining legacy worktree')
if pathlib.Path(record.get('canonical_worktree','')).resolve()!=target:
 raise SystemExit('task worktree path changed; retaining legacy worktree')
resource_id={'path':str(target),'common_dir':registration.get('common_dir'),
             'admin_dir':registration.get('admin_dir'),'instance_id':registration.get('instance_id'),
             'expected_head_oid':tip}
row=module._worktree_state(repo_path,mapping_path,uid,resource_id,branch,mutate=(mode=='remove'))
allowed=('removed','already_absent') if mode=='remove' else ('ready','already_absent')
if row.get('state') not in allowed:
 raise SystemExit(f"{row.get('reason','cleanup blocked')}: {row.get('state')}")
print(json.dumps(row,sort_keys=True))
PY
)" || die "legacy worktree safety check failed: $result"
  printf '%s\n' "$result"
}

legacy_local_branch_check() {
  local result
  result="$(python3 - "$SCRIPT_DIR" "$REPO_ROOT" "$MAPPING" "$TASK_UID" "refs/heads/$BRANCH" "$BRANCH_TIP" <<'PY'
import importlib.util,json,pathlib,sys
script_dir,repo,mapping,uid,full_ref,expected=sys.argv[1:]
sys.path.insert(0,script_dir)
spec=importlib.util.spec_from_file_location('resource_cleanup_executor',pathlib.Path(script_dir)/'resource-cleanup-executor.py')
if spec is None or spec.loader is None: raise SystemExit('resource cleanup safety helper is unavailable')
module=importlib.util.module_from_spec(spec); sys.modules[spec.name]=module; spec.loader.exec_module(module)
repo_path=pathlib.Path(repo).resolve(); resource={'full_ref':full_ref,'expected_oid':expected}
row=module._local_branch_state(repo_path,resource,module._worktree_rows(repo_path),mutate=False)
if row.get('state')!='ready': raise SystemExit(f"{row.get('reason','branch cleanup blocked')}: {row.get('state')}")
print(json.dumps(row,sort_keys=True))
PY
  )" || die "legacy local branch safety check failed: $result"
  printf '%s\n' "$result"
}

update_intent_flag() {
  local field="$1" value="$2" update
  [[ -f "$INTENT_JOURNAL" ]] || die "cannot update cleanup intent before it is durable"
  update="$(python3 - "$INTENT_JOURNAL" "$TASK_UID" "$RECORDED_REPOSITORY" "$WORKTREE" "$BRANCH" "$field" "$value" <<'PY'
import json,pathlib,sys
p=pathlib.Path(sys.argv[1]); uid,repository,worktree,branch,field,value=sys.argv[2:]
data=json.loads(p.read_text(encoding='utf-8'))
expected={'receipt_type':'oasis7_cleanup_intent','task_uid':uid,'repository':repository,'worktree':worktree,'branch':branch}
for key,want in expected.items():
 if data.get(key)!=want: raise SystemExit(f'post-merge-cleanup: cleanup intent {key} mismatch')
old=data.get(field)
if old is True and value!='true': raise SystemExit(f'post-merge-cleanup: cleanup intent {field} cannot be reversed')
data[field]=(value=='true')
data['revision']=int(data.get('revision',0))+1
print(json.dumps(data))
PY
)" || die "cleanup intent update failed: $field"
  journal_write "$INTENT_JOURNAL" "$update" || die "cleanup intent update failed: $field"
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
# accept an already missing worktree/branch, or reconcile an exact canonical
# worktree reappearance, only after this matching journal and fresh live
# merge/main-sync proof have been revalidated.
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
ALREADY_TERMINAL="$(python3 - "$MAPPING" "$TASK_UID" <<'PY'
import json,sys
r=(json.load(open(sys.argv[1])).get('tasks') or {}).get(sys.argv[2]) or {}
terminal_phase='post_'+'merge_done'
print('1' if r.get('workflow_phase')==terminal_phase else '0')
PY
)"
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
REPO_COMMON_DIR="$(cd "$REPO_ROOT" && cd "$(git rev-parse --git-common-dir)" && pwd -P)" \
  || die "canonical repository common-dir cannot be resolved"

INTENT_JOURNAL="$(dirname "$TERMINAL_RECEIPT_OUTPUT")/cleanup-intent.json"
INTENT_STATE="0 0 0 0 0 0 0 0"
if [[ -f "$INTENT_JOURNAL" ]]; then
  INTENT_STATE="$(python3 - "$INTENT_JOURNAL" "$TASK_UID" "$RECORDED_REPOSITORY" "$WORKTREE" "$BRANCH" <<'PY'
import json,sys
r=json.load(open(sys.argv[1],encoding='utf-8'))
expected={'receipt_type':'oasis7_cleanup_intent','task_uid':sys.argv[2],'repository':sys.argv[3],'worktree':sys.argv[4],'branch':sys.argv[5]}
for key,value in expected.items():
 if r.get(key)!=value: raise SystemExit(f'post-merge-cleanup: cleanup intent mismatch on retry: {key}')
print(int(bool(r.get('worktree_removed'))),int(bool(r.get('branch_deleted'))),int(bool(r.get('terminal_receipt_committed'))),
      int(bool(r.get('worktree_remove_started'))),int(bool(r.get('local_branch_delete_started'))),
      int(bool(r.get('remote_branch_delete_started'))),int(bool(r.get('remote_branch_released'))),
      int(bool(r.get('remote_branch_absent_observed'))))
PY
)" || die "cleanup intent validation failed"
  JOURNAL_WORKTREE_COMMON_DIR="$(python3 - "$INTENT_JOURNAL" <<'PY'
import json,sys
print(json.load(open(sys.argv[1],encoding='utf-8')).get('worktree_common_dir') or '')
PY
)"
  JOURNAL_BRANCH_TIP="$(python3 - "$INTENT_JOURNAL" <<'PY'
import json,sys
print(json.load(open(sys.argv[1],encoding='utf-8')).get('branch_tip') or '')
PY
)"
else
  JOURNAL_WORKTREE_COMMON_DIR=""
  JOURNAL_BRANCH_TIP=""
fi
WORKTREE_REMOVED="$(printf '%s' "$INTENT_STATE" | awk '{print $1}')"
BRANCH_DELETED="$(printf '%s' "$INTENT_STATE" | awk '{print $2}')"
TERMINAL_COMMITTED="$(printf '%s' "$INTENT_STATE" | awk '{print $3}')"
WORKTREE_REMOVE_STARTED="$(printf '%s' "$INTENT_STATE" | awk '{print $4}')"
LOCAL_BRANCH_DELETE_STARTED="$(printf '%s' "$INTENT_STATE" | awk '{print $5}')"
REMOTE_BRANCH_DELETE_STARTED="$(printf '%s' "$INTENT_STATE" | awk '{print $6}')"
REMOTE_BRANCH_RELEASED="$(printf '%s' "$INTENT_STATE" | awk '{print $7}')"
REMOTE_BRANCH_ABSENT_OBSERVED="$(printf '%s' "$INTENT_STATE" | awk '{print $8}')"
WORKTREE_REAPPEARED=0
WORKTREE_EFFECT_RECOVERED=0
BRANCH_EFFECT_RECOVERED=0
if [[ "$WORKTREE_REMOVED" != 1 && ! -e "$WORKTREE" && -f "$INTENT_JOURNAL" ]]; then
  ! worktree_is_registered "$REPO_ROOT" "$WORKTREE" \
    || die "cleanup intent exists but absent worktree remains registered"
  [[ -n "$JOURNAL_WORKTREE_COMMON_DIR" && -n "$JOURNAL_BRANCH_TIP" ]] \
    || die "cleanup intent lacks identity needed to reconcile an interrupted removal"
  WORKTREE_REMOVED=1
  WORKTREE_EFFECT_RECOVERED=1
fi
if [[ "$WORKTREE_REMOVE_STARTED" == 1 && "$WORKTREE_REMOVED" != 1 && -e "$WORKTREE" ]]; then
  die "path_reused_or_recreated: prior cleanup attempt may have removed this legacy worktree instance"
fi
if [[ "$LOCAL_BRANCH_DELETE_STARTED" == 1 && "$BRANCH_DELETED" != 1 ]] \
    && git -C "$REPO_ROOT" show-ref --verify --quiet "refs/heads/$BRANCH"; then
  die "local_branch_recreated_after_uncertain_delete: prior cleanup intent cannot authorize deleting this ref"
fi
if [[ "$REMOTE_BRANCH_RELEASED" == 1 || "$REMOTE_BRANCH_DELETE_STARTED" == 1 || "$REMOTE_BRANCH_ABSENT_OBSERVED" == 1 ]]; then
  REMOTE_BRANCH_LINE="$(git -C "$REPO_ROOT" ls-remote --heads origin "refs/heads/$BRANCH")" \
    || die "remote task branch readback failed during cleanup resume"
  if [[ -n "$REMOTE_BRANCH_LINE" ]]; then
    die "remote_branch_recreated_after_cleanup_intent: refusing to delete a possibly reused ref"
  elif [[ "$REMOTE_BRANCH_DELETE_STARTED" == 1 && "$REMOTE_BRANCH_RELEASED" != 1 ]]; then
    update_intent_flag remote_branch_released true
    REMOTE_BRANCH_RELEASED=1
  fi
fi
if [[ "$WORKTREE_REMOVED" == 1 && "$BRANCH_DELETED" != 1 ]] \
    && ! git -C "$REPO_ROOT" show-ref --verify --quiet "refs/heads/$BRANCH"; then
  [[ -f "$INTENT_JOURNAL" && -n "$JOURNAL_BRANCH_TIP" ]] \
    || die "task branch is absent without a matching cleanup intent"
  BRANCH_DELETED=1
  BRANCH_EFFECT_RECOVERED=1
fi
if [[ "$WORKTREE_REMOVED" == 1 ]]; then
  if [[ -e "$WORKTREE" ]]; then
    # A v1 intent predates trusted registration-instance identity. Its old
    # removal bit cannot authorize deleting a resource recreated at that path.
    die "path_reused_or_recreated: historical cleanup intent cannot authorize deleting this worktree instance"
  else
    ! worktree_is_registered "$REPO_ROOT" "$WORKTREE" \
      || die "cleanup journal says worktree_removed but git still registers it"
    ACTUAL_BRANCH="$RECORDED_BRANCH"
    if [[ "$BRANCH_DELETED" == 1 ]]; then
      # A prior retry already deleted the branch; the receipt head remains the
      # only available tip identity for the terminal-receipt retry.
      BRANCH_TIP="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("head_oid") or "")' "$PR_RECEIPT")"
    else
      BRANCH_TIP="$(git -C "$REPO_ROOT" rev-parse --verify "refs/heads/$ACTUAL_BRANCH^{commit}")" \
        || die "task branch tip cannot be resolved during cleanup resume"
    fi
  fi
else
  [[ -e "$WORKTREE" ]] || die "worktree path is absent and no matching cleanup intent proves removal"
  git -C "$WORKTREE" rev-parse --is-inside-work-tree >/dev/null 2>&1 || die "worktree path is not a git worktree"
  worktree_is_registered "$REPO_ROOT" "$WORKTREE" || die "task worktree is not registered under repo root"
  WORKTREE_COMMON_DIR="$(cd "$WORKTREE" && cd "$(git rev-parse --git-common-dir)" && pwd -P)" \
    || die "task worktree common-dir cannot be resolved"
  [[ "$WORKTREE_COMMON_DIR" == "$REPO_COMMON_DIR" ]] || die "task worktree common-dir mismatch against canonical repository"
  [[ -z "$(git -C "$WORKTREE" status --porcelain --untracked-files=all)" ]] || die "task worktree is dirty"
  ACTUAL_BRANCH="$(git -C "$WORKTREE" symbolic-ref --quiet --short HEAD)" || die "task worktree must be on a named branch"
  [[ "$BRANCH" == "$ACTUAL_BRANCH" ]] || die "caller branch disagrees with canonical task worktree branch"
  BRANCH_TIP="$(git -C "$REPO_ROOT" rev-parse "refs/heads/$ACTUAL_BRANCH")"
fi
[[ "$ACTUAL_BRANCH" == "$RECORDED_BRANCH" ]] || die "branch identity disagrees with canonical task truth"
if [[ -n "$JOURNAL_WORKTREE_COMMON_DIR" ]]; then
  [[ "$JOURNAL_WORKTREE_COMMON_DIR" == "$REPO_COMMON_DIR" ]] \
    || die "cleanup journal worktree common-dir identity mismatch"
  if [[ -n "${WORKTREE_COMMON_DIR:-}" ]]; then
    [[ "$JOURNAL_WORKTREE_COMMON_DIR" == "$WORKTREE_COMMON_DIR" ]] \
      || die "cleanup journal worktree common-dir identity mismatch"
  fi
fi
if [[ -n "$JOURNAL_BRANCH_TIP" ]]; then
  [[ "$JOURNAL_BRANCH_TIP" == "$BRANCH_TIP" ]] \
    || die "cleanup journal branch tip identity mismatch"
fi
if [[ "$BRANCH_DELETED" == 1 ]]; then
  ! git -C "$REPO_ROOT" show-ref --verify --quiet "refs/heads/$BRANCH" \
    || die "cleanup journal says branch_deleted but the ref name has been reused"
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
if [[ "$WORKTREE_REMOVED" == 1 && "$BRANCH_DELETED" != 1 ]]; then
  BRANCH_TIP="$(git -C "$REPO_ROOT" rev-parse --verify "refs/heads/$ACTUAL_BRANCH^{commit}")" \
    || die "task branch tip cannot be resolved during cleanup resume"
fi
[[ -n "$RECEIPT_HEAD" && "$RECEIPT_HEAD" == "$BRANCH_TIP" ]] || die "fresh PR receipt is not bound to current task branch tip"
MAIN_COMMIT="$(git -C "$REPO_ROOT" rev-parse "refs/heads/$RECEIPT_DEFAULT_BRANCH")" || die "local default branch is unavailable"
SYNC_MAIN_COMMIT="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1],encoding="utf-8")).get("main_commit") or "")' "$MAIN_SYNC_RECEIPT")"
[[ -n "$SYNC_MAIN_COMMIT" && "$MAIN_COMMIT" == "$SYNC_MAIN_COMMIT" ]] || die "local default branch moved after main-sync receipt"
if git -C "$REPO_ROOT" remote get-url origin >/dev/null 2>&1; then
  git -C "$REPO_ROOT" fetch --quiet origin "refs/heads/$RECEIPT_DEFAULT_BRANCH:refs/remotes/origin/$RECEIPT_DEFAULT_BRANCH" || die "failed to refresh origin default branch"
  REMOTE_MAIN="$(git -C "$REPO_ROOT" rev-parse "refs/remotes/origin/$RECEIPT_DEFAULT_BRANCH")"
  [[ "$MAIN_COMMIT" == "$REMOTE_MAIN" ]] || die "local default branch is not synchronized with origin"
fi
PATCH_EQUIVALENCE_PROVEN=0
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
  PATCH_EQUIVALENCE_PROVEN=1
else
  [[ "$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("integration_mode") or "")' "$MAIN_SYNC_RECEIPT")" == "ancestry" ]] || die "ancestry cleanup requires ancestry main-sync receipt"
  [[ -z "$PATCH_RECEIPT" ]] || die "patch-equivalence receipt is not accepted when main contains the task branch tip"
fi

# A v1 terminal receipt can be written before the task finalizer advances
# task truth to post_merge_done. On a crash/resume retry, validate the existing
# bytes against this fresh merge/main-sync chain and preserve them verbatim.
# The intent journal proves the helper reached this receipt-creation stage;
# identity and raw-chain digests bind the bytes to the current task.
EXISTING_TERMINAL_RECEIPT=0
if [[ "$DRY_RUN" == "0" && -f "$TERMINAL_RECEIPT_OUTPUT" ]]; then
  [[ -f "$INTENT_JOURNAL" ]] || die "existing terminal cleanup receipt has no matching cleanup intent"
  python3 - "$TERMINAL_RECEIPT_OUTPUT" "$MAPPING" "$TASK_UID" "$RECORDED_REPOSITORY" \
      "$RECORDED_PR_NUMBER" "$WORKTREE" "$BRANCH" "$PR_RECEIPT" "$MAIN_SYNC_RECEIPT" <<'PY'
import datetime,json,pathlib,sys
path,mapping_path,uid,repository,pr_number,worktree,branch,merge_path,sync_path=sys.argv[1:]
def unique_object(pairs):
    result={}
    for key,value in pairs:
        if key in result:
            raise ValueError(f'duplicate JSON key: {key}')
        result[key]=value
    return result
try:
    receipt=json.loads(pathlib.Path(path).read_text(encoding='utf-8'),object_pairs_hook=unique_object)
    records=json.loads(pathlib.Path(mapping_path).read_text(encoding='utf-8'),object_pairs_hook=unique_object).get('tasks') or {}
    task=records.get(uid) or {}
except (OSError,json.JSONDecodeError) as exc:
    raise SystemExit(f'post-merge-cleanup: existing terminal cleanup receipt is malformed: {exc}')
import hashlib
expected={
    'receipt_type':'oasis7_terminal_cleanup',
    'issuer':'post-merge-cleanup',
    'cleanup_intent_required':True,
    'task_uid':uid,
    'repository':repository,
    'issue_number':task.get('issue_number'),
    'pr_number':int(pr_number),
    'worktree':worktree,
    'branch':branch,
    'merge_receipt_sha256':hashlib.sha256(pathlib.Path(merge_path).read_bytes()).hexdigest(),
    'main_sync_receipt_sha256':hashlib.sha256(pathlib.Path(sync_path).read_bytes()).hexdigest(),
}
if not isinstance(receipt,dict):
    raise SystemExit('post-merge-cleanup: existing terminal cleanup receipt is not an object')
for key,value in expected.items():
    if receipt.get(key)!=value:
        raise SystemExit(f'post-merge-cleanup: existing terminal cleanup receipt {key} disagrees with live task evidence')
observed=receipt.get('observed_at')
if not isinstance(observed,str):
    raise SystemExit('post-merge-cleanup: existing terminal cleanup receipt lacks its original observed_at')
try:
    datetime.datetime.fromisoformat(observed.replace('Z','+00:00'))
except ValueError as exc:
    raise SystemExit(f'post-merge-cleanup: existing terminal cleanup receipt observed_at is malformed: {exc}')
PY
  if [[ "$TERMINAL_COMMITTED" != 1 ]]; then
    update_intent_flag terminal_receipt_committed true
    TERMINAL_COMMITTED=1
  fi
  EXISTING_TERMINAL_RECEIPT=1
elif [[ "$DRY_RUN" == "0" && "$TERMINAL_COMMITTED" == 1 ]]; then
  die "cleanup intent says terminal receipt is committed but its canonical bytes are missing"
fi

printf 'git -C %q worktree remove %q\n' "$REPO_ROOT" "$WORKTREE"
if [[ "$PATCH_EQUIVALENCE_PROVEN" == 1 ]]; then
  printf 'git -C %q update-ref -d %q %q # exact-OID compare-and-delete\n' "$REPO_ROOT" "refs/heads/$BRANCH" "$BRANCH_TIP"
else
  printf 'git -C %q update-ref -d %q %q # exact-OID compare-and-delete\n' "$REPO_ROOT" "refs/heads/$BRANCH" "$BRANCH_TIP"
fi
if [[ "$DRY_RUN" == "0" ]]; then
  if [[ "$WORKTREE_REMOVED" != 1 ]]; then
    legacy_worktree_operation check >/dev/null
  fi
  INTENT_JOURNAL="$(dirname "$TERMINAL_RECEIPT_OUTPUT")/cleanup-intent.json"
  # A removed task path cannot be re-read; its linked-worktree identity is the
  # live canonical repository common-dir already checked above.
  JOURNAL_BACKFILL_COMMON_DIR="${WORKTREE_COMMON_DIR:-$REPO_COMMON_DIR}"
  JOURNAL_JSON="$(python3 - "$INTENT_JOURNAL" "$TASK_UID" "$RECORDED_REPOSITORY" "$WORKTREE" "$BRANCH" "$JOURNAL_BACKFILL_COMMON_DIR" "$BRANCH_TIP" "$WORKTREE_EFFECT_RECOVERED" "$BRANCH_EFFECT_RECOVERED" "$MAPPING" <<'PY'
import json,pathlib,sys
mapping=pathlib.Path(sys.argv[10]); p=pathlib.Path(sys.argv[1]); identity={"receipt_type":"oasis7_cleanup_intent","task_uid":sys.argv[2],
 "repository":sys.argv[3],"worktree":sys.argv[4],"branch":sys.argv[5]}
derived={"worktree_common_dir":sys.argv[6],"branch_tip":sys.argv[7]}
record=(json.loads(mapping.read_text(encoding='utf-8')).get('tasks') or {}).get(sys.argv[2]) or {}
registration=record.get('worktree_registration') or {}
instance_id=registration.get('instance_id') if isinstance(registration,dict) else None
if instance_id: derived['worktree_instance_id']=instance_id
expected=dict(identity)
if p.exists():
 old=json.loads(p.read_text());
 for key,value in identity.items():
  if old.get(key)!=value: raise SystemExit(f'post-merge-cleanup: cleanup intent mismatch on retry: {key}')
 # Preserve forward-compatible durable state such as blocker history while
 # updating the known identity and progress fields below.
 expected.update(old)
 expected.update(identity)
 # Legacy journals intentionally lack the derived identity fields.  Preserve
 # existing values exactly; only after all live receipt/proof checks above may
 # this transaction add a missing field as a one-time backfill.
 for key,value in derived.items():
  if key in old:
   if old[key]!=value: raise SystemExit(f'post-merge-cleanup: cleanup intent mismatch on retry: {key}')
   expected[key]=old[key]
  elif value:
   expected[key]=value
 expected.update({k:bool(old.get(k)) for k in ('worktree_removed','branch_deleted','terminal_receipt_committed')})
 for key in ('worktree_remove_started','local_branch_delete_started','remote_branch_delete_started',
             'remote_branch_released','remote_branch_absent_observed'):
  if key in old and not isinstance(old[key],bool): raise SystemExit(f'post-merge-cleanup: cleanup intent {key} is malformed')
  expected[key]=bool(old.get(key))
else:
 expected.update(derived)
 expected.update(worktree_removed=False,branch_deleted=False,terminal_receipt_committed=False,
                 worktree_remove_started=False,local_branch_delete_started=False,
                 remote_branch_delete_started=False,remote_branch_released=False,
                 remote_branch_absent_observed=False)
 if p.exists():
  for key,value in derived.items():
   if key in old and old[key]!=value: raise SystemExit(f'post-merge-cleanup: cleanup intent mismatch on retry: {key}')
   if key not in old and value: expected[key]=value
expected['worktree_removed']=expected['worktree_removed'] or sys.argv[8]=='1'
expected['branch_deleted']=expected['branch_deleted'] or sys.argv[9]=='1'
expected['revision']=int((json.loads(p.read_text()).get('revision',0) if p.exists() else 0))+1
print(json.dumps(expected))
PY
  )"; journal_write "$INTENT_JOURNAL" "$JOURNAL_JSON"
  if [[ "$WORKTREE_REMOVED" != 1 ]]; then
    update_intent_flag worktree_remove_started true
    legacy_worktree_operation remove >/dev/null
    JOURNAL_JSON="$(python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); d["worktree_removed"]=True; d["revision"]+=1; print(json.dumps(d))' "$INTENT_JOURNAL")"; journal_write "$INTENT_JOURNAL" "$JOURNAL_JSON"
  fi
  if [[ "$BRANCH_DELETED" != 1 ]]; then
    legacy_local_branch_check >/dev/null
    CURRENT_BRANCH_TIP="$(git -C "$REPO_ROOT" rev-parse --verify "refs/heads/$BRANCH^{commit}")" \
      || die "task branch tip cannot be revalidated before deletion"
    [[ "$CURRENT_BRANCH_TIP" == "$BRANCH_TIP" ]] \
      || die "task branch tip changed before deletion"
    update_intent_flag local_branch_delete_started true
    git -C "$REPO_ROOT" update-ref -d "refs/heads/$BRANCH" "$BRANCH_TIP" \
      || die "task branch exact-OID compare-and-delete failed"
    ! git -C "$REPO_ROOT" show-ref --verify --quiet "refs/heads/$BRANCH" \
      || die "task branch deletion readback found a reappeared ref"
    JOURNAL_JSON="$(python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); d["branch_deleted"]=True; d["revision"]+=1; print(json.dumps(d))' "$INTENT_JOURNAL")"; journal_write "$INTENT_JOURNAL" "$JOURNAL_JSON"
  fi
  # The merged PR binds the exact head. Delete a same-head remote task branch
  # idempotently; refuse to delete if the name was reused for another commit.
  if [[ "$RECORDED_REPOSITORY" != fixture/* ]]; then
    REMOTE_BRANCH_LINE="$(git -C "$REPO_ROOT" ls-remote --heads origin "refs/heads/$BRANCH")" \
      || die "remote task branch readback failed"
    if [[ -n "$REMOTE_BRANCH_LINE" ]]; then
      REMOTE_BRANCH_TIP="$(printf '%s\n' "$REMOTE_BRANCH_LINE" | awk 'NR==1 {print $1}')"
      if [[ "$REMOTE_BRANCH_TIP" != "$BRANCH_TIP" ]]; then
        record_remote_branch_blocker "$REMOTE_BRANCH_TIP"
        die "durable cleanup blocker: remote task branch tip disagrees with merged PR head; inspect cleanup-intent.json before resuming"
      fi
      update_intent_flag remote_branch_delete_started true
      if ! git -C "$REPO_ROOT" push --force-with-lease="refs/heads/$BRANCH:$REMOTE_BRANCH_TIP" \
        origin ":refs/heads/$BRANCH" >/dev/null; then
        REMOTE_BRANCH_LINE="$(git -C "$REPO_ROOT" ls-remote --heads origin "refs/heads/$BRANCH")" \
          || die "remote task branch deletion failed and readback is unavailable"
        REMOTE_BRANCH_TIP="$(printf '%s\n' "$REMOTE_BRANCH_LINE" | awk 'NR==1 {print $1}')"
        if [[ -n "$REMOTE_BRANCH_TIP" && "$REMOTE_BRANCH_TIP" != "$BRANCH_TIP" ]]; then
          record_remote_branch_blocker "$REMOTE_BRANCH_TIP"
          die "durable cleanup blocker: remote task branch tip changed during deletion; inspect cleanup-intent.json before resuming"
        fi
        die "remote task branch deletion failed"
      fi
      REMOTE_BRANCH_LINE="$(git -C "$REPO_ROOT" ls-remote --heads origin "refs/heads/$BRANCH")" \
        || die "remote task branch deletion readback failed"
      if [[ -n "$REMOTE_BRANCH_LINE" ]]; then
        REMOTE_BRANCH_TIP="$(printf '%s\n' "$REMOTE_BRANCH_LINE" | awk 'NR==1 {print $1}')"
        if [[ "$REMOTE_BRANCH_TIP" != "$BRANCH_TIP" ]]; then
          record_remote_branch_blocker "$REMOTE_BRANCH_TIP"
          die "durable cleanup blocker: remote task branch tip changed during deletion; inspect cleanup-intent.json before resuming"
        fi
        die "remote task branch deletion readback failed"
      fi
      update_intent_flag remote_branch_released true
      resolve_remote_branch_blocker "matching_tip_deleted" "$BRANCH_TIP"
    else
      update_intent_flag remote_branch_absent_observed true
      resolve_remote_branch_blocker "remote_ref_absent" ""
    fi
  fi
  if [[ "$ALREADY_TERMINAL" == 1 ]]; then
    # Terminal receipt bytes are immutable authority. Reconciliation may
    # remove a recreated checkout/branch, but it must not mint a new digest.
    python3 - "$MAPPING" "$TASK_UID" "$TERMINAL_RECEIPT_OUTPUT" <<'PY'
import hashlib,json,pathlib,sys
r=(json.load(open(sys.argv[1])).get('tasks') or {}).get(sys.argv[2]) or {}
p=pathlib.Path(sys.argv[3]); receipt=json.loads(p.read_text(encoding='utf-8'))
terminal_phase='post_'+'merge_done'
if (r.get('phase_receipts') or {}).get(terminal_phase) != receipt:
 raise SystemExit('post-merge-cleanup: existing terminal receipt disagrees with task truth')
if (r.get('phase_receipt_sha256') or {}).get(terminal_phase) != hashlib.sha256(p.read_bytes()).hexdigest():
 raise SystemExit('post-merge-cleanup: existing terminal receipt digest disagrees with task truth')
PY
    exit 0
  fi
  if [[ "$EXISTING_TERMINAL_RECEIPT" == 1 ]]; then
    printf 'python3 %q --repo-root %q --task-uid %q --terminal-receipt %q\n' \
      "$REPO_ROOT/scripts/pm/post-merge-finalize.py" "$REPO_ROOT" "$TASK_UID" "$TERMINAL_RECEIPT_OUTPUT"
    exit 0
  fi
  mkdir -p "$(dirname "$TERMINAL_RECEIPT_OUTPUT")"
  TMP_TERMINAL="$(mktemp "$(dirname "$TERMINAL_RECEIPT_OUTPUT")/.terminal-cleanup.XXXXXX")"
  python3 - "$TMP_TERMINAL" "$MAPPING" "$TASK_UID" "$PR_RECEIPT" "$MAIN_SYNC_RECEIPT" "$RECORDED_REPOSITORY" "$RECORDED_PR_NUMBER" "$WORKTREE" "$BRANCH" <<'PY'
import datetime as d,hashlib,json,os,pathlib,sys
mapping=json.load(open(sys.argv[2],encoding='utf-8')); record=(mapping.get('tasks') or {}).get(sys.argv[3]) or {}
merge=pathlib.Path(sys.argv[4]); sync=pathlib.Path(sys.argv[5])
out={"receipt_type":"oasis7_terminal_cleanup","issuer":"post-merge-cleanup",
 "cleanup_intent_required":True,
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
