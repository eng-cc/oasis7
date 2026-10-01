#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REAL_GIT="$(command -v git)"
TMPDIR="$(mktemp -d)"; trap 'rm -rf "$TMPDIR"' EXIT

register_fixture_worktree() {
  local worktree="$1" branch="$2" uid="$3" repository="$4"
  local fixture_map="$worktree/.pm/github-project-sync/tasks.json" registration
  mkdir -p "$(dirname "$fixture_map")"
  cat >"$fixture_map" <<EOF
{"version":1,"tasks":{"$uid":{"task_uid":"$uid","repository":"$repository","canonical_worktree":"$worktree","task_branch":"$branch","default_branch":"main"}}}
EOF
  registration="$(python3 "$ROOT_DIR/scripts/pm/worktree_registration.py" --repo-root "$worktree" --task-uid "$uid")" || return 1
  python3 - "$fixture_map" "$uid" "$registration" <<'PY'
import json,sys
record=json.load(open(sys.argv[1],encoding="utf-8"))["tasks"][sys.argv[2]]
returned=json.loads(sys.argv[3])
assert record["worktree_registration"] == returned, (record, returned)
PY
  rm -rf "$worktree/.pm"
  printf '%s' "$registration"
}

assert_worktree_registration() {
  local repo_root="$1" uid="$2" worktree="$3"
  python3 - "$ROOT_DIR/scripts/pm" "$repo_root" "$uid" "$worktree" <<'PY'
import json,pathlib,sys
sys.path.insert(0,sys.argv[1])
from worktree_registration import validate_worktree_registration
repo=pathlib.Path(sys.argv[2]); uid=sys.argv[3]; expected=pathlib.Path(sys.argv[4]).resolve()
record=json.loads((repo/".pm/github-project-sync/tasks.json").read_text(encoding="utf-8"))["tasks"][uid]
assert pathlib.Path(record["canonical_worktree"]).resolve() == expected, record
assert validate_worktree_registration(expected, record) == record["worktree_registration"], record
PY
}

run_faulted_cleanup() {
  local isolation_root="$1" repo="$2" worktree="$3" branch="$4" intent="$5" fault="$6" exit_code="$7"
  shift 7
  local fault_bin="$isolation_root/fault-bin" status
  [[ "${1:-}" == "$ROOT_DIR/scripts/pm/post-merge-cleanup.sh" ]] || return 2
  python3 - "$isolation_root" "$repo" "$worktree" "$intent" <<'PY'
import pathlib,subprocess,sys,tempfile
root=pathlib.Path(sys.argv[1]).resolve(); temp=pathlib.Path(tempfile.gettempdir()).resolve()
try: root.relative_to(temp)
except ValueError: raise SystemExit("cleanup fault injection root is outside the system temporary boundary")
repo,worktree,intent=(pathlib.Path(value).resolve() for value in sys.argv[2:])
for path in (repo,worktree,intent):
    try: path.relative_to(root)
    except ValueError: raise SystemExit(f"cleanup fault injection path is outside its fixture root: {path}")
if not repo.is_dir() or not worktree.is_dir():
    raise SystemExit("cleanup fault injection requires an isolated live repository and worktree")
def common(path):
    raw=subprocess.check_output(["git","-C",str(path),"rev-parse","--git-common-dir"],text=True).strip()
    value=pathlib.Path(raw)
    return (value if value.is_absolute() else path/value).resolve()
if common(repo)!=common(worktree):
    raise SystemExit("cleanup fault injection repository/worktree common-dir mismatch")
PY
  mkdir -p "$fault_bin"
  cat >"$fault_bin/git" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
"$CLEANUP_REAL_GIT" "$@"
matched=0
case "$CLEANUP_TEST_FAULT:$*" in
  TPM_CLEANUP_FAULT_AFTER_WORKTREE_REMOVE:"-C $CLEANUP_TEST_REPO worktree remove $CLEANUP_TEST_WORKTREE") matched=1 ;;
  TPM_CLEANUP_FAULT_AFTER_BRANCH_DELETE:"-C $CLEANUP_TEST_REPO update-ref -d refs/heads/$CLEANUP_TEST_BRANCH "*) matched=1 ;;
esac
if [[ "$matched" == 1 ]]; then
  kill -TERM "$CLEANUP_TEST_SCRIPT_PID"
fi
SH
  chmod +x "$fault_bin/git"
  if env PATH="$fault_bin:$isolation_root/bin:$PATH" \
    CLEANUP_REAL_GIT="$REAL_GIT" CLEANUP_TEST_FAULT="$fault" \
    CLEANUP_TEST_EXIT_CODE="$exit_code" CLEANUP_TEST_SCRIPT_PID="" CLEANUP_TEST_REPO="$repo" \
    CLEANUP_TEST_WORKTREE="$worktree" CLEANUP_TEST_BRANCH="$branch" \
    bash -c 'export CLEANUP_TEST_SCRIPT_PID=$$; exec "$@"' bash "$@"; then
    status=0
  else
    status=$?
  fi
  [[ "$status" == 143 ]] || return "$status"
  return "$exit_code"
}

run_case() {
  local name="$1" fault="$2" expected_exit="$3" registration_mode="${4:-registered}"
  local repo="$TMPDIR/$name/repo" worktree="$TMPDIR/$name/task" branch="task/$name"
  local uid="task_11111111111111111111111111111111" receipts registration registration_field=""
  mkdir -p "$repo" "$TMPDIR/$name/bin"
  repo="$(cd "$repo" && pwd -P)"
  git -C "$repo" init -q -b main; git -C "$repo" config user.email test@example.invalid; git -C "$repo" config user.name Test
  receipts="$(python3 "$ROOT_DIR/scripts/pm/canonical-receipt-root.py" --default-worktree "$repo" --task-uid "$uid" --create)"
  printf 'base\n' >"$repo/file"; git -C "$repo" add file; git -C "$repo" commit -qm base
  git -C "$repo" worktree add -qb "$branch" "$worktree"
  worktree="$(cd "$worktree" && pwd -P)"
  if [[ "$registration_mode" == registered ]]; then
    registration="$(register_fixture_worktree "$worktree" "$branch" "$uid" "eng-cc/oasis7")"
    registration_field=",\"worktree_registration\":$registration"
  elif [[ "$registration_mode" != unregistered ]]; then
    echo "unknown cleanup test registration mode: $registration_mode" >&2
    return 2
  fi
  printf 'merged\n' >>"$worktree/file"
  git -C "$worktree" commit -qam merged; local head; head="$(git -C "$worktree" rev-parse HEAD)"
  git -C "$repo" merge --ff-only "$branch" >/dev/null
  local origin="$TMPDIR/$name/origin.git"
  git init --bare -q "$origin"
  git -C "$repo" remote add origin "$origin"
  git -C "$repo" push -q origin main "$branch"
  mkdir -p "$repo/.pm/github-project-sync"
  cat >"$repo/.pm/github-project-sync/tasks.json" <<EOF
{"tasks":{"$uid":{"task_uid":"$uid","status":"done","workflow_phase":"main_sync","repository":"eng-cc/oasis7","issue_number":11,"pr_number":1,"pr_url":"https://example.invalid/pull/1","canonical_worktree":"$worktree","task_branch":"$branch","default_branch":"main"$registration_field}}}
EOF
  if [[ "$registration_mode" == registered ]]; then
    assert_worktree_registration "$repo" "$uid" "$worktree"
  fi
  local now main; now="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"; main="$(git -C "$repo" rev-parse main)"
  cat >"$receipts/merge-receipt.json" <<EOF
{"receipt_type":"oasis7_pr_merge","issuer":"github_live_query","evidence_mode":"production","repository":"eng-cc/oasis7","default_branch":"main","pr_number":1,"pr_url":"https://example.invalid/pull/1","state":"MERGED","merged_at":"$now","head_oid":"$head","base_ref":"main","observed_at":"$now"}
EOF
  python3 - "$receipts/merge-receipt.json" "$receipts/main-sync-receipt.json" "$uid" "$main" "$now" <<'PY'
import hashlib,json,pathlib,sys
m=pathlib.Path(sys.argv[1]); pathlib.Path(sys.argv[2]).write_text(json.dumps({
"receipt_type":"oasis7_main_sync","issuer":"post-merge-main-sync","integration_mode":"ancestry","task_uid":sys.argv[3],"repository":"eng-cc/oasis7",
"default_branch":"main","main_commit":sys.argv[4],"remote_main_commit":sys.argv[4],
"merge_receipt_sha256":hashlib.sha256(m.read_bytes()).hexdigest(),"observed_at":sys.argv[5]})+'\n')
PY
  cat >"$TMPDIR/$name/bin/gh" <<EOF
#!/usr/bin/env bash
if [[ "\$*" == "repo view --json nameWithOwner,defaultBranchRef" ]]; then
 printf '%s\n' '{"nameWithOwner":"eng-cc/oasis7","defaultBranchRef":{"name":"main"}}'
else
 printf '%s\n' '{"number":1,"url":"https://example.invalid/pull/1","state":"MERGED","mergedAt":"$now","headRefOid":"$head","baseRefName":"main"}'
fi
EOF
  chmod +x "$TMPDIR/$name/bin/gh"
  if [[ "$registration_mode" == unregistered ]]; then
    local merge_before sync_before local_before remote_before status
    merge_before="$TMPDIR/$name/merge.before"
    sync_before="$TMPDIR/$name/main-sync.before"
    cp "$receipts/merge-receipt.json" "$merge_before"
    cp "$receipts/main-sync-receipt.json" "$sync_before"
    local_before="$(git -C "$repo" rev-parse "refs/heads/$branch")"
    remote_before="$(git -C "$repo" ls-remote --heads origin "refs/heads/$branch" | awk 'NR==1 {print $1}')"
    set +e
    env PATH="$TMPDIR/$name/bin:$PATH" \
      "$ROOT_DIR/scripts/pm/post-merge-cleanup.sh" --repo-root "$repo" --worktree "$worktree" --branch "$branch" \
      --main-ref main --task-uid "$uid" --pr-receipt "$receipts/merge-receipt.json" \
      --main-sync-receipt "$receipts/main-sync-receipt.json" --terminal-receipt-output "$receipts/terminal-cleanup-receipt.json" \
      >"$receipts/unregistered.out" 2>"$receipts/unregistered.err"
    status=$?
    set -e
    [[ "$status" != 0 ]]
    grep -F "task worktree has no trusted registration-instance binding" "$receipts/unregistered.err" >/dev/null
    [[ -d "$worktree" ]]
    [[ "$(git -C "$repo" rev-parse "refs/heads/$branch")" == "$local_before" ]]
    [[ "$(git -C "$repo" ls-remote --heads origin "refs/heads/$branch" | awk 'NR==1 {print $1}')" == "$remote_before" ]]
    cmp -s "$merge_before" "$receipts/merge-receipt.json"
    cmp -s "$sync_before" "$receipts/main-sync-receipt.json"
    [[ ! -e "$receipts/cleanup-intent.json" ]]
    [[ ! -e "$receipts/terminal-cleanup-receipt.json" ]]
    return 0
  fi
  set +e
  local isolation_root; isolation_root="$(python3 -c 'import os,sys; print(os.path.realpath(sys.argv[1]))' "$TMPDIR/$name")"
  run_faulted_cleanup "$TMPDIR/$name" "$repo" "$worktree" "$branch" "$receipts/cleanup-intent.json" "$fault" "$expected_exit" \
    "$ROOT_DIR/scripts/pm/post-merge-cleanup.sh" --repo-root "$repo" --worktree "$worktree" --branch "$branch" \
    --main-ref main --task-uid "$uid" --pr-receipt "$receipts/merge-receipt.json" \
    --main-sync-receipt "$receipts/main-sync-receipt.json" --terminal-receipt-output "$receipts/terminal-cleanup-receipt.json" \
    >"$receipts/first.out" 2>"$receipts/first.err"
  local status=$?; set -e
  if [[ "$status" != "$expected_exit" ]]; then cat "$receipts/first.err" >&2; return 1; fi
  [[ -f "$receipts/cleanup-intent.json" ]]
  python3 - "$receipts/cleanup-intent.json" "$fault" <<'PY'
import json,sys
j=json.load(open(sys.argv[1])); assert j['receipt_type']=='oasis7_cleanup_intent',j
if sys.argv[2].endswith('BRANCH_DELETE'):
  assert j['worktree_removed'] is True and j['local_branch_delete_started'] is True,j
  assert j['branch_deleted'] is False,j
else:
  assert j['worktree_remove_started'] is True,j
  assert j['worktree_removed'] is False,j
assert j['branch_deleted'] is False,j
assert j['terminal_receipt_committed'] is False,j
PY
  env PATH="$TMPDIR/$name/bin:$PATH" \
    "$ROOT_DIR/scripts/pm/post-merge-cleanup.sh" --repo-root "$repo" --worktree "$worktree" --branch "$branch" \
    --main-ref main --task-uid "$uid" --pr-receipt "$receipts/merge-receipt.json" \
    --main-sync-receipt "$receipts/main-sync-receipt.json" --terminal-receipt-output "$receipts/terminal-cleanup-receipt.json" \
    >"$receipts/retry.out" 2>"$receipts/retry.err"
  [[ -f "$receipts/terminal-cleanup-receipt.json" ]]
  python3 - "$receipts/cleanup-intent.json" <<'PY'
import json,sys
j=json.load(open(sys.argv[1])); assert all(j[k] for k in ('worktree_removed','branch_deleted','terminal_receipt_committed')),j
PY
  [[ ! -e "$worktree" ]]; ! git -C "$repo" show-ref --verify --quiet "refs/heads/$branch"
  [[ -z "$(git -C "$repo" ls-remote --heads origin "refs/heads/$branch")" ]]
}

run_case after_worktree TPM_CLEANUP_FAULT_AFTER_WORKTREE_REMOVE 86
run_case after_branch TPM_CLEANUP_FAULT_AFTER_BRANCH_DELETE 87
run_case unregistered none 0 unregistered
echo "post-merge-cleanup-crash.test: OK"
