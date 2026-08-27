#!/usr/bin/env bash
set -euo pipefail

# Regression coverage for terminal cleanup resume identities.  Each case uses
# the production helper through the isolated crash fixture.  The legacy case
# intentionally projects the durable journal to its historical schema so the
# test can prove one-time migration without granting production callers a
# journal-editing channel.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
REAL_GIT="$(command -v git)"
FAULT_FIXTURE="$ROOT_DIR/scripts/pm/fixtures/post-merge-cleanup-fault.sh"
TMPDIR="$(mktemp -d)"
cleanup() {
  for repo in "$TMPDIR"/*/repo; do
    [[ -d "$repo" ]] || continue
    worktree="$(dirname "$repo")/task-worktree"
    "$REAL_GIT" -C "$repo" worktree remove --force "$worktree" >/dev/null 2>&1 || true
  done
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

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
  local name="$1" mode="$2" reappear="$3" legacy="$4"
  local root="$TMPDIR/$name"
  local repo="$root/repo" worktree="$root/task-worktree"
  local branch="task/cleanup-$name" uid="task_11111111111111111111111111111111"
  local observed_at base branch_tip main_commit receipts patch_receipt
  mkdir -p "$repo"
  git -C "$repo" init -q -b main
  git -C "$repo" config user.email test@example.invalid
  git -C "$repo" config user.name Test
  printf 'base\n' >"$repo/file"
  git -C "$repo" add file
  git -C "$repo" commit -qm base
  base="$(git -C "$repo" rev-parse HEAD)"
  git -C "$repo" worktree add -qb "$branch" "$worktree"
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
{"version":1,"tasks":{"$uid":{"task_uid":"$uid","status":"done","issue_number":1,"pr_number":1,"pr_url":"https://example.invalid/pull/1","repository":"fixture/repo","canonical_worktree":"$worktree","task_branch":"$branch","default_branch":"main"}}}
EOF
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
  set +e
  env PATH="$root/bin:$PATH" TEST_HEAD_OID="$branch_tip" \
    "$FAULT_FIXTURE" --isolation-root "$root" --fault TPM_CLEANUP_FAULT_AFTER_WORKTREE_REMOVE -- \
    "${cleanup_args[@]}" \
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
assert journal["worktree_removed"] is True, journal
assert journal["branch_deleted"] is False, journal
assert journal["terminal_receipt_committed"] is False, journal
PY

  if [[ "$legacy" == 1 ]]; then
    python3 - "$receipts/cleanup-intent.json" <<'PY'
import json
import sys
path = sys.argv[1]
journal = json.load(open(path, encoding="utf-8"))
# Historical cleanup-intent.json had only the identity and boolean state
# fields.  This fixture models that durable artifact; production cleanup must
# backfill only after live identity/proof validation.
for key in ("worktree_common_dir", "branch_tip"):
    journal.pop(key, None)
open(path, "w", encoding="utf-8").write(json.dumps(journal) + "\n")
PY
  fi

  if [[ "$reappear" == 1 ]]; then
    git -C "$repo" worktree add -q "$worktree" "$branch"
  fi
  env PATH="$root/bin:$PATH" TEST_HEAD_OID="$branch_tip" \
    bash "${cleanup_args[@]}" \
    >"$root/retry.out"
  [[ ! -e "$worktree" ]]
  ! git -C "$repo" show-ref --verify --quiet "refs/heads/$branch"
  python3 - "$receipts/cleanup-intent.json" <<'PY'
import json
import sys
journal = json.load(open(sys.argv[1], encoding="utf-8"))
assert all(journal[key] for key in ("worktree_removed", "branch_deleted", "terminal_receipt_committed")), journal
PY
  if [[ "$legacy" == 1 ]]; then
    python3 - "$receipts/cleanup-intent.json" "$repo" "$branch_tip" <<'PY'
import json
import os
import subprocess
import sys
journal = json.load(open(sys.argv[1], encoding="utf-8"))
repo, branch_tip = sys.argv[2:]
raw_common = subprocess.check_output(
    ["git", "-C", repo, "rev-parse", "--git-common-dir"], text=True
).strip()
common = os.path.realpath(raw_common if os.path.isabs(raw_common) else os.path.join(repo, raw_common))
assert journal["branch_tip"] == branch_tip, journal
assert journal["worktree_common_dir"] == common, (journal, common)
PY
  fi
  if [[ "$reappear" == 1 ]]; then
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
    env PATH="$root/bin:$PATH" TEST_HEAD_OID="$branch_tip" bash "${cleanup_args[@]}" >"$root/terminal-reconcile.out"
    after="$(shasum -a 256 "$receipts/terminal-cleanup-receipt.json" | awk '{print $1}')"
    [[ "$before" == "$after" ]]
    [[ ! -e "$worktree" ]]
    ! git -C "$repo" show-ref --verify --quiet "refs/heads/$branch"
  fi
}

# Squash/rebase cleanup must resume with a force-delete only after the exact
# patch-equivalence proof has passed; plain branch -d cannot prove that state.
run_case patch_equivalence patch_equivalence 0 0
# A real #2692-style journal predates the identity fields and must resume only
# after the fresh patch-equivalence proof, then persist the derived identity.
run_case legacy_patch_equivalence patch_equivalence 0 1
# An exact canonical worktree may be recreated between journaled removal and
# retry; identity readback must reconcile it before the normal safe removal.
run_case reappeared_worktree ancestry 1 0

echo "post-merge-cleanup-resume.test: OK"
