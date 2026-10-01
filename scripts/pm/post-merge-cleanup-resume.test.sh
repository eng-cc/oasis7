#!/usr/bin/env bash
set -euo pipefail

# Regression coverage for terminal cleanup resume identities. Each case uses
# the production helper with test-local parent-process fault injection. The legacy case
# intentionally projects the durable journal to its historical schema so the
# test can prove one-time migration without granting production callers a
# journal-editing channel.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
REAL_GIT="$(command -v git)"
TMPDIR="$(mktemp -d)"
cleanup() {
  for repo in "$TMPDIR"/*/repo; do
    [[ -d "$repo" ]] || continue
    worktree="$(dirname "$repo")/task-worktree"
    "$REAL_GIT" -C "$repo" worktree remove --force "$worktree" >/dev/null 2>&1 || true
  done
  if [[ "${KEEP_CLEANUP_TEST_TMPDIR:-0}" == 1 ]]; then
    printf 'post-merge-cleanup-resume.test: preserved fixture at %s\n' "$TMPDIR" >&2
  else
    rm -rf "$TMPDIR"
  fi
}
trap cleanup EXIT

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

run_worktree_crash() {
  local isolation_root="$1" repo="$2" worktree="$3" intent="$4"
  shift 4
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
if [[ "$*" == "-C $CLEANUP_TEST_REPO worktree remove $CLEANUP_TEST_WORKTREE" ]]; then
  kill -TERM "$CLEANUP_TEST_SCRIPT_PID"
fi
SH
  chmod +x "$fault_bin/git"
  if env PATH="$fault_bin:$isolation_root/bin:$PATH" \
      CLEANUP_REAL_GIT="$REAL_GIT" CLEANUP_TEST_SCRIPT_PID="" \
      CLEANUP_TEST_REPO="$repo" CLEANUP_TEST_WORKTREE="$worktree" \
      bash -c 'export CLEANUP_TEST_SCRIPT_PID=$$; exec "$@"' bash "$@"; then
    status=0
  else
    status=$?
  fi
  [[ "$status" == 143 ]] || return "$status"
  return 86
}

write_common_receipts() {
  local repo="$1" receipts="$2" uid="$3" branch_tip="$4" main_commit="$5" observed_at="$6" mode="$7" patch_receipt="$8"
  cat >"$receipts/merge-receipt.json" <<EOF
{"receipt_type":"oasis7_pr_merge","issuer":"github_live_query","evidence_mode":"production","repository":"fixture/repo","default_branch":"main","pr_number":1,"pr_url":"https://example.invalid/pull/1","state":"MERGED","merged_at":"$observed_at","head_oid":"$branch_tip","base_ref":"main","observed_at":"$observed_at"}
EOF
  python3 - "$receipts/merge-receipt.json" "$receipts/main-sync-receipt.json" "$uid" "$main_commit" "$observed_at" "$mode" "$patch_receipt" <<'PY'
import hashlib
import json
import pathlib
import sys

merge = pathlib.Path(sys.argv[1])
output = pathlib.Path(sys.argv[2])
uid, main_commit, observed_at, mode, patch_path = sys.argv[3:]
out = {
    "receipt_type": "oasis7_main_sync",
    "issuer": "post-merge-main-sync",
    "integration_mode": mode,
    "task_uid": uid,
    "repository": "fixture/repo",
    "default_branch": "main",
    "main_commit": main_commit,
    "remote_main_commit": main_commit,
    "merge_receipt_sha256": hashlib.sha256(merge.read_bytes()).hexdigest(),
    "observed_at": observed_at,
}
if mode == "patch_equivalence":
    patch = pathlib.Path(patch_path)
    proof = json.loads(patch.read_text(encoding="utf-8"))
    out.update(
        patch_equivalence_receipt_sha256=hashlib.sha256(patch.read_bytes()).hexdigest(),
        patch_id=proof["patch_id"],
        projected_tree_oid=proof["projected_tree_oid"],
        main_tree_oid=proof["main_tree_oid"],
        integration_commit=proof["main_commit"],
        integration_parent=proof["main_parent"],
    )
output.write_text(json.dumps(out) + "\n", encoding="utf-8")
PY
}

write_gh_fixture() {
  local root="$1" observed_at="$2"
  mkdir -p "$root/bin"
  cat >"$root/bin/gh" <<EOF
#!/usr/bin/env bash
if [[ "\$*" == "repo view --json nameWithOwner,defaultBranchRef" ]]; then
  printf '%s\n' '{"nameWithOwner":"fixture/repo","defaultBranchRef":{"name":"main"}}'
else
  printf '%s\n' '{"number":1,"url":"https://example.invalid/pull/1","state":"MERGED","mergedAt":"$observed_at","headRefOid":"'"\${TEST_HEAD_OID:?}"'","baseRefName":"main"}'
fi
EOF
  chmod +x "$root/bin/gh"
}

run_case() {
  local name="$1" mode="$2" reappear_before_retry="$3" reappear_after_terminal="$4" legacy="$5"
  local root="$TMPDIR/$name"
  local repo="$root/repo" worktree="$root/task-worktree"
  local branch="task/cleanup-$name" uid="task_11111111111111111111111111111111"
  local observed_at base branch_tip main_commit receipts patch_receipt registration
  mkdir -p "$repo"
  repo="$(cd "$repo" && pwd -P)"
  git -C "$repo" init -q -b main
  git -C "$repo" config user.email test@example.invalid
  git -C "$repo" config user.name Test
  printf 'base\n' >"$repo/file"
  git -C "$repo" add file
  git -C "$repo" commit -qm base
  base="$(git -C "$repo" rev-parse HEAD)"
  git -C "$repo" worktree add -qb "$branch" "$worktree"
  worktree="$(cd "$worktree" && pwd -P)"
  registration="$(register_fixture_worktree "$worktree" "$branch" "$uid" "fixture/repo")"
  printf 'task change\n' >>"$worktree/file"
  git -C "$worktree" add file
  git -C "$worktree" commit -qm task-change
  branch_tip="$(git -C "$worktree" rev-parse HEAD)"

  if [[ "$mode" == patch_equivalence ]]; then
    printf 'task change\n' >>"$repo/file"
    git -C "$repo" add file
    git -C "$repo" commit -qm squash-integration
    main_commit="$(git -C "$repo" rev-parse HEAD)"
    patch_receipt=""
  else
    git -C "$repo" merge --ff-only "$branch" >/dev/null
    main_commit="$(git -C "$repo" rev-parse HEAD)"
    patch_receipt=""
  fi

  mkdir -p "$repo/.pm/github-project-sync"
  cat >"$repo/.pm/github-project-sync/tasks.json" <<EOF
{"version":1,"tasks":{"$uid":{"task_uid":"$uid","status":"done","workflow_phase":"main_sync","issue_number":1,"pr_number":1,"pr_url":"https://example.invalid/pull/1","repository":"fixture/repo","canonical_worktree":"$worktree","task_branch":"$branch","default_branch":"main","worktree_registration":$registration}}}
EOF
  assert_worktree_registration "$repo" "$uid" "$worktree"
  receipts="$(python3 "$ROOT_DIR/scripts/pm/canonical-receipt-root.py" --default-worktree "$repo" --task-uid "$uid" --create)"
  if [[ "$mode" == patch_equivalence ]]; then
    patch_receipt="$receipts/patch-equivalence-receipt.json"
    "$ROOT_DIR/scripts/pm/patch-equivalence-receipt.sh" --root "$repo" \
      --branch-tip "$branch_tip" --main-commit "$main_commit" --main-parent "$base" \
      >"$patch_receipt"
  fi
  observed_at="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  write_common_receipts "$repo" "$receipts" "$uid" "$branch_tip" "$main_commit" "$observed_at" "$mode" "$patch_receipt"
  write_gh_fixture "$root" "$observed_at"

  local cleanup_args=("$ROOT_DIR/scripts/pm/post-merge-cleanup.sh" --repo-root "$repo" --worktree "$worktree"
    --branch "$branch" --main-ref main --task-uid "$uid"
    --pr-receipt "$receipts/merge-receipt.json" --main-sync-receipt "$receipts/main-sync-receipt.json")
  [[ -z "$patch_receipt" ]] || cleanup_args+=(--patch-equivalence-receipt "$patch_receipt")
  cleanup_args+=(--terminal-receipt-output "$receipts/terminal-cleanup-receipt.json")
  if [[ "$legacy" == 1 ]]; then
    # A historical journal can only omit derived identities after its own
    # successful cleanup transaction. Do not manufacture a half-completed
    # crash record: run the owner normally, then project the durable result
    # back to the old schema and prove retry backfills from trusted evidence.
    env PATH="$root/bin:$PATH" TEST_HEAD_OID="$branch_tip" \
      bash "${cleanup_args[@]}" >"$root/legacy-first.out" 2>"$root/legacy-first.err" \
      || { cat "$root/legacy-first.err" >&2; return 1; }
    [[ ! -e "$worktree" ]]
    ! git -C "$repo" show-ref --verify --quiet "refs/heads/$branch"
python3 - "$receipts" "$repo" "$branch_tip" "$repo/.pm/github-project-sync/tasks.json" "$uid" <<'PY'
import hashlib,json,os,pathlib,subprocess,sys
root=pathlib.Path(sys.argv[1]); repo,tip,mapping_path,uid=sys.argv[2:]
common_raw=subprocess.check_output(["git","-C",repo,"rev-parse","--git-common-dir"],text=True).strip()
common=os.path.realpath(common_raw if os.path.isabs(common_raw) else os.path.join(repo,common_raw))
journal=json.loads((root/"cleanup-intent.json").read_text())
assert all(journal.get(key) is True for key in ("worktree_removed","branch_deleted","terminal_receipt_committed")),journal
assert journal.get("branch_tip")==tip and journal.get("worktree_common_dir")==common,journal
for name in ("merge-receipt.json","main-sync-receipt.json","patch-equivalence-receipt.json","terminal-cleanup-receipt.json"):
 path=root/name
 if path.exists():
  (root/(name+".before-retry.sha256")).write_text(hashlib.sha256(path.read_bytes()).hexdigest())
# Retry before the finalizer advances task truth to post_merge_done. The
# helper-generated terminal-cleanup receipt is already durable, so its exact
# bytes must survive the retry that upgrades historical journal metadata.
mapping=pathlib.Path(mapping_path); data=json.loads(mapping.read_text()); record=data["tasks"][uid]
assert record.get("workflow_phase") == "main_sync",record
assert "post_merge_done" not in record.get("phase_receipts",{}),record
mapping.write_text(json.dumps(data)+"\n",encoding="utf-8")
# Model the historical journal's old schema only after the completed helper
# wrote it; the test never authors trusted state or completion flags.
for key in ("worktree_common_dir","branch_tip","worktree_instance_id"):
 journal.pop(key,None)
(root/"cleanup-intent.json").write_text(json.dumps(journal)+"\n")
PY
    env PATH="$root/bin:$PATH" TEST_HEAD_OID="$branch_tip" \
      bash "${cleanup_args[@]}" >"$root/legacy-retry.out" 2>"$root/legacy-retry.err" \
      || { cat "$root/legacy-retry.err" >&2; return 1; }
    python3 - "$receipts" "$repo" "$branch_tip" <<'PY'
import hashlib,json,os,pathlib,subprocess,sys
root=pathlib.Path(sys.argv[1]); repo,tip=sys.argv[2:]
common_raw=subprocess.check_output(["git","-C",repo,"rev-parse","--git-common-dir"],text=True).strip()
common=os.path.realpath(common_raw if os.path.isabs(common_raw) else os.path.join(repo,common_raw))
journal=json.loads((root/"cleanup-intent.json").read_text())
assert journal.get("branch_tip")==tip and journal.get("worktree_common_dir")==common,journal
assert all(journal.get(key) is True for key in ("worktree_removed","branch_deleted","terminal_receipt_committed")),journal
for name in ("merge-receipt.json","main-sync-receipt.json","patch-equivalence-receipt.json","terminal-cleanup-receipt.json"):
 before=root/(name+".before-retry.sha256"); path=root/name
 if before.exists(): assert hashlib.sha256(path.read_bytes()).hexdigest()==before.read_text(),name
terminal=json.loads((root/"terminal-cleanup-receipt.json").read_text())
assert terminal.get("cleanup_intent_required") is True,terminal
assert terminal.get("task_uid")==journal.get("task_uid"),terminal
PY
    return 0
  fi
  set +e
  TEST_HEAD_OID="$branch_tip" run_worktree_crash "$root" "$repo" "$worktree" \
    "$receipts/cleanup-intent.json" "${cleanup_args[@]}" \
    >"$root/first.out" 2>"$root/first.err"
  local first_status=$?
  set -e
  [[ "$first_status" == 86 ]] || { cat "$root/first.err" >&2; echo "$name: expected crash fixture status 86, got $first_status" >&2; return 1; }
  [[ ! -e "$worktree" ]]
  git -C "$repo" show-ref --verify --quiet "refs/heads/$branch"
  python3 - "$receipts/cleanup-intent.json" <<'PY'
import json
import sys
journal = json.load(open(sys.argv[1], encoding="utf-8"))
assert journal["worktree_remove_started"] is True, journal
assert journal["worktree_removed"] is False, journal
assert journal["branch_deleted"] is False, journal
assert journal["terminal_receipt_committed"] is False, journal
PY

  if [[ "$reappear_before_retry" == 1 ]]; then
    git -C "$repo" worktree add -q "$worktree" "$branch"
    set +e
    env PATH="$root/bin:$PATH" TEST_HEAD_OID="$branch_tip" \
      bash "${cleanup_args[@]}" >"$root/reused-path.out" 2>"$root/reused-path.err"
    local reused_status=$?
    set -e
    [[ "$reused_status" != 0 ]]
    grep -F "path_reused_or_recreated" "$root/reused-path.err" >/dev/null
    [[ -d "$worktree" ]]
    git -C "$repo" show-ref --verify --quiet "refs/heads/$branch"
    [[ ! -e "$receipts/terminal-cleanup-receipt.json" ]]
    return 0
  fi
  env PATH="$root/bin:$PATH" TEST_HEAD_OID="$branch_tip" \
    bash "${cleanup_args[@]}" \
    >"$root/retry.out"
  [[ ! -e "$worktree" ]]
  ! git -C "$repo" show-ref --verify --quiet "refs/heads/$branch"
  python3 - "$receipts/cleanup-intent.json" "$receipts/terminal-cleanup-receipt.json" <<'PY'
import json
import sys
journal = json.load(open(sys.argv[1], encoding="utf-8"))
assert all(journal[key] for key in ("worktree_removed", "branch_deleted", "terminal_receipt_committed")), journal
terminal = json.load(open(sys.argv[2], encoding="utf-8"))
assert terminal.get("cleanup_intent_required") is True, terminal
PY
  if [[ "$reappear_after_terminal" == 1 ]]; then
    # Model a fully finalized task whose exact checkout is recreated later.
    # Reconciliation must remove the residue without changing terminal receipt
    # bytes already bound into task truth.
    python3 - "$repo/.pm/github-project-sync/tasks.json" "$uid" "$receipts/terminal-cleanup-receipt.json" <<'PY'
import hashlib,json,pathlib,sys
mapping=pathlib.Path(sys.argv[1]); data=json.loads(mapping.read_text()); r=data['tasks'][sys.argv[2]]
p=pathlib.Path(sys.argv[3]); receipt=json.loads(p.read_text())
r['workflow_phase']='post_merge_done'; r.setdefault('phase_receipts',{})['post_merge_done']=receipt
r.setdefault('phase_receipt_sha256',{})['post_merge_done']=hashlib.sha256(p.read_bytes()).hexdigest()
mapping.write_text(json.dumps(data)+'\n')
PY
    before="$(shasum -a 256 "$receipts/terminal-cleanup-receipt.json" | awk '{print $1}')"
    git -C "$repo" branch "$branch" "$branch_tip"
    git -C "$repo" worktree add -q "$worktree" "$branch"
    set +e
    env PATH="$root/bin:$PATH" TEST_HEAD_OID="$branch_tip" \
      bash "${cleanup_args[@]}" >"$root/terminal-reconcile.out" 2>"$root/terminal-reconcile.err"
    local reconcile_status=$?
    set -e
    [[ "$reconcile_status" != 0 ]]
    grep -F "path_reused_or_recreated" "$root/terminal-reconcile.err" >/dev/null
    after="$(shasum -a 256 "$receipts/terminal-cleanup-receipt.json" | awk '{print $1}')"
    [[ "$before" == "$after" ]]
    [[ -d "$worktree" ]]
    git -C "$repo" show-ref --verify --quiet "refs/heads/$branch"
  fi
}

run_stale_remote_tip_case() {
  local name="stale_remote_tip"
  local root="$TMPDIR/$name"
  local repo="$root/repo" remote="$root/origin.git" worktree="$root/task-worktree"
  local replacement_worktree="$root/replacement-worktree"
  local branch="task/cleanup-$name" replacement_branch="test/remote-$name"
  local uid="task_22222222222222222222222222222222"
  local base branch_tip main_commit replacement_tip receipts observed_at registration
  mkdir -p "$repo" "$root/bin"
  repo="$(cd "$repo" && pwd -P)"
  git init --bare -q "$remote"
  git -C "$remote" symbolic-ref HEAD refs/heads/main
  git -C "$repo" init -q -b main
  git -C "$repo" config user.email test@example.invalid
  git -C "$repo" config user.name Test
  printf 'base\n' >"$repo/file"
  git -C "$repo" add file
  git -C "$repo" commit -qm base
  base="$(git -C "$repo" rev-parse HEAD)"
  git -C "$repo" worktree add -qb "$branch" "$worktree"
  worktree="$(cd "$worktree" && pwd -P)"
  registration="$(register_fixture_worktree "$worktree" "$branch" "$uid" "eng-cc/oasis7")"
  printf 'task change\n' >>"$worktree/file"
  git -C "$worktree" add file
  git -C "$worktree" commit -qm task-change
  branch_tip="$(git -C "$worktree" rev-parse HEAD)"
  git -C "$repo" merge --ff-only "$branch" >/dev/null
  main_commit="$(git -C "$repo" rev-parse main)"
  git -C "$repo" remote add origin "$remote"
  git -C "$repo" push -q origin main "$branch"

  mkdir -p "$repo/.pm/github-project-sync"
  cat >"$repo/.pm/github-project-sync/tasks.json" <<EOF
{"version":1,"tasks":{"$uid":{"task_uid":"$uid","status":"done","workflow_phase":"main_sync","issue_number":1,"pr_number":1,"pr_url":"https://github.com/eng-cc/oasis7/pull/1","repository":"eng-cc/oasis7","canonical_worktree":"$worktree","task_branch":"$branch","default_branch":"main","worktree_registration":$registration}}}
EOF
  assert_worktree_registration "$repo" "$uid" "$worktree"
  receipts="$(python3 "$ROOT_DIR/scripts/pm/canonical-receipt-root.py" --default-worktree "$repo" --task-uid "$uid" --create)"
  observed_at="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  cat >"$receipts/merge-receipt.json" <<EOF
{"receipt_type":"oasis7_pr_merge","issuer":"github_live_query","evidence_mode":"production","repository":"eng-cc/oasis7","default_branch":"main","pr_number":1,"pr_url":"https://github.com/eng-cc/oasis7/pull/1","state":"MERGED","merged_at":"$observed_at","head_oid":"$branch_tip","base_ref":"main","observed_at":"$observed_at"}
EOF
  python3 - "$receipts/merge-receipt.json" "$receipts/main-sync-receipt.json" "$uid" "$main_commit" "$observed_at" <<'PY'
import hashlib,json,pathlib,sys
merge=pathlib.Path(sys.argv[1]); output=pathlib.Path(sys.argv[2])
output.write_text(json.dumps({
    "receipt_type":"oasis7_main_sync", "issuer":"post-merge-main-sync",
    "integration_mode":"ancestry", "task_uid":sys.argv[3],
    "repository":"eng-cc/oasis7", "default_branch":"main",
    "main_commit":sys.argv[4], "remote_main_commit":sys.argv[4],
    "merge_receipt_sha256":hashlib.sha256(merge.read_bytes()).hexdigest(),
    "observed_at":sys.argv[5],
})+"\n",encoding="utf-8")
PY
  cat >"$root/bin/gh" <<'EOF'
#!/usr/bin/env bash
case "$*" in
  "repo view --json nameWithOwner,defaultBranchRef")
    printf '%s\n' '{"nameWithOwner":"eng-cc/oasis7","defaultBranchRef":{"name":"main"}}'
    ;;
  *)
    printf '{"number":1,"url":"https://github.com/eng-cc/oasis7/pull/1","state":"MERGED","mergedAt":"%s","headRefOid":"%s","baseRefName":"main"}\n' \
      "$TEST_MERGED_AT" "${TEST_HEAD_OID:?}"
    ;;
esac
EOF
  chmod +x "$root/bin/gh"

  local cleanup_args=("$ROOT_DIR/scripts/pm/post-merge-cleanup.sh" --repo-root "$repo" --worktree "$worktree"
    --branch "$branch" --main-ref main --task-uid "$uid"
    --pr-receipt "$receipts/merge-receipt.json" --main-sync-receipt "$receipts/main-sync-receipt.json"
    --terminal-receipt-output "$receipts/terminal-cleanup-receipt.json")
  set +e
  TEST_HEAD_OID="$branch_tip" TEST_MERGED_AT="$observed_at" \
    run_worktree_crash "$root" "$repo" "$worktree" \
      "$receipts/cleanup-intent.json" "${cleanup_args[@]}" >"$root/first.out" 2>"$root/first.err"
  local first_status=$?
  set -e
  [[ "$first_status" == 86 ]] || { cat "$root/first.err" >&2; echo "stale remote: expected crash fixture status 86, got $first_status" >&2; return 1; }
  [[ ! -e "$worktree" ]]
  git -C "$repo" show-ref --verify --quiet "refs/heads/$branch"

  git -C "$repo" worktree add -qb "$replacement_branch" "$replacement_worktree" "$branch_tip"
  printf 'replacement tip\n' >>"$replacement_worktree/file"
  git -C "$replacement_worktree" add file
  git -C "$replacement_worktree" commit -qm replacement-tip
  replacement_tip="$(git -C "$replacement_worktree" rev-parse HEAD)"
  git -C "$repo" push -q --force origin "$replacement_branch:refs/heads/$branch"
  git -C "$repo" worktree remove --force "$replacement_worktree"
  git -C "$repo" branch -D "$replacement_branch" >/dev/null

  local merge_digest sync_digest
  merge_digest="$(python3 -c 'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' "$receipts/merge-receipt.json")"
  sync_digest="$(python3 -c 'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' "$receipts/main-sync-receipt.json")"
  set +e
  env PATH="$root/bin:$PATH" TEST_HEAD_OID="$branch_tip" TEST_MERGED_AT="$observed_at" \
    bash "${cleanup_args[@]}" >"$root/stale.out" 2>"$root/stale.err"
  local stale_status=$?
  set -e
  [[ "$stale_status" != 0 ]] || { echo "stale remote: cleanup must not delete a branch whose tip changed" >&2; return 1; }
  python3 - "$receipts/cleanup-intent.json" "$uid" "$branch" "$branch_tip" "$replacement_tip" <<'PY'
import json,sys
journal=json.load(open(sys.argv[1],encoding="utf-8"))
uid,branch,expected,observed=sys.argv[2:]
assert journal.get("task_uid")==uid and journal.get("branch")==branch, journal
assert journal.get("worktree_removed") is True and journal.get("branch_deleted") is True, journal
blocker=journal.get("remote_branch_blocker")
assert blocker and blocker.get("schema")=="oasis7_cleanup_blocker_v1", blocker
assert blocker.get("kind")=="remote_branch_tip_mismatch", blocker
assert blocker.get("branch")==branch, blocker
assert blocker.get("expected_tip")==expected, blocker
assert blocker.get("observed_tip")==observed, blocker
assert blocker.get("resolved") is False, blocker
assert journal.get("terminal_receipt_committed") is False, journal
PY
  [[ "$(git -C "$repo" ls-remote --heads origin "refs/heads/$branch" | awk 'NR==1 {print $1}')" == "$replacement_tip" ]]
  [[ ! -e "$receipts/terminal-cleanup-receipt.json" ]]
  [[ "$(python3 -c 'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' "$receipts/merge-receipt.json")" == "$merge_digest" ]]
  [[ "$(python3 -c 'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' "$receipts/main-sync-receipt.json")" == "$sync_digest" ]]

  set +e
  env PATH="$root/bin:$PATH" TEST_HEAD_OID="$branch_tip" TEST_MERGED_AT="$observed_at" \
    bash "${cleanup_args[@]}" >"$root/retry.out" 2>"$root/retry.err"
  local retry_status=$?
  set -e
  [[ "$retry_status" != 0 ]]
  grep -F "durable cleanup blocker: remote task branch tip disagrees with merged PR head" "$root/retry.err" >/dev/null
  python3 - "$receipts/cleanup-intent.json" "$branch_tip" "$replacement_tip" <<'PY'
import json,sys
journal=json.load(open(sys.argv[1],encoding="utf-8")); blocker=journal.get("remote_branch_blocker")
assert blocker and blocker.get("resolved") is False, journal
assert blocker.get("expected_tip")==sys.argv[2] and blocker.get("observed_tip")==sys.argv[3], blocker
assert journal.get("terminal_receipt_committed") is False, journal
PY

  # Once an operator or external process restores the merged exact head,
  # cleanup may safely delete that exact ref and resolve the durable blocker.
  git -C "$repo" push -q --force origin "$branch_tip:refs/heads/$branch" >/dev/null 2>&1
  env PATH="$root/bin:$PATH" TEST_HEAD_OID="$branch_tip" TEST_MERGED_AT="$observed_at" \
    bash "${cleanup_args[@]}" >"$root/recovered.out"
  ! git -C "$repo" ls-remote --heads origin "refs/heads/$branch" | grep -q .
  python3 - "$receipts/cleanup-intent.json" "$branch_tip" <<'PY'
import json,sys
journal=json.load(open(sys.argv[1],encoding="utf-8")); blocker=journal.get("remote_branch_blocker")
assert blocker and blocker.get("resolved") is True, journal
assert blocker.get("resolution")=="matching_tip_deleted", blocker
assert blocker.get("resolved_tip")==sys.argv[2], blocker
assert blocker.get("resolved_at"), blocker
assert journal.get("terminal_receipt_committed") is True, journal
PY
  [[ "$(python3 -c 'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' "$receipts/merge-receipt.json")" == "$merge_digest" ]]
  [[ "$(python3 -c 'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' "$receipts/main-sync-receipt.json")" == "$sync_digest" ]]
}

# A replaced remote branch tip is never deleted based on the old merge receipt;
# the blocker must survive retries and retain the immutable merge receipts.
run_stale_remote_tip_case

# Squash/rebase cleanup must resume with a force-delete only after the exact
# patch-equivalence proof has passed; plain branch -d cannot prove that state.
run_case patch_equivalence patch_equivalence 0 0 0
# A real #2692-style journal predates the identity fields and must resume only
# after the fresh patch-equivalence proof, then persist the derived identity.
run_case legacy_patch_equivalence patch_equivalence 0 0 1
# An exact canonical worktree may be recreated between journaled removal and
# retry; identity readback must reconcile it before the normal safe removal.
run_case reappeared_worktree ancestry 1 0 0
run_case terminal_reappeared_worktree ancestry 0 1 0

echo "post-merge-cleanup-resume.test: OK"
