#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"; TMPDIR="$(mktemp -d)"; trap 'rm -rf "$TMPDIR"' EXIT
[[ ! -e "$ROOT_DIR/relative-main-sync.json" ]] || { echo "leaked relative main-sync artifact in repository root" >&2; exit 1; }
REMOTE="$TMPDIR/origin.git"; DEFAULT="$TMPDIR/default"; TASK="$TMPDIR/task"
UID_VALUE="task_11111111111111111111111111111111"; BRANCH="task/dual-terminal"
git init -q --bare "$REMOTE"; git clone -q "$REMOTE" "$DEFAULT"; git -C "$DEFAULT" config user.email test@example.invalid; git -C "$DEFAULT" config user.name Test
RECEIPTS="$(python3 "$ROOT_DIR/scripts/pm/canonical-receipt-root.py" --default-worktree "$DEFAULT" --task-uid "$UID_VALUE" --create)"
git -C "$DEFAULT" switch -qc main; mkdir -p "$DEFAULT/scripts/pm"; printf '.pm/\n' >"$DEFAULT/.gitignore"; printf 'base\n' >"$DEFAULT/file"
cat >"$DEFAULT/scripts/pm/github-project-task.py" <<'PY'
#!/usr/bin/env python3
import json,pathlib,sys
root=pathlib.Path(sys.argv[2]); uid=sys.argv[sys.argv.index('--task-uid')+1]; receipt=pathlib.Path(sys.argv[sys.argv.index('--receipt-json')+1])
p=root/'.pm/github-project-sync/tasks.json'; m=json.loads(p.read_text()); r=m['tasks'][uid]
phase='post_merge_done' if sys.argv[1]=='finalize-phase' else sys.argv[sys.argv.index('--phase')+1]
r['workflow_phase']=phase; r.setdefault('phase_receipts',{})[phase]=json.loads(receipt.read_text())
p.write_text(json.dumps(m)+'\n'); print('{}')
PY
chmod +x "$DEFAULT/scripts/pm/github-project-task.py"; git -C "$DEFAULT" add .; git -C "$DEFAULT" commit -qm base; git -C "$DEFAULT" push -q -u origin main
git -C "$DEFAULT" worktree add -qb "$BRANCH" "$TASK"; printf 'merged\n' >>"$TASK/file"; git -C "$TASK" commit -qam merged
HEAD_OID="$(git -C "$TASK" rev-parse HEAD)"; git -C "$DEFAULT" merge --ff-only "$BRANCH" >/dev/null; git -C "$DEFAULT" push -q origin main; git -C "$DEFAULT" reset -q --hard HEAD^
mkdir -p "$TASK/.pm/github-project-sync"
cat >"$TASK/.pm/github-project-sync/tasks.json" <<EOF
{"tasks":{"$UID_VALUE":{"task_uid":"$UID_VALUE","status":"done","workflow_phase":"task_done","repository":"fixture/repo","issue_number":11,"pr_number":7,"pr_url":"https://example.invalid/pull/7","canonical_worktree":"$TASK","task_branch":"$BRANCH","default_branch":"main"}}}
EOF
NOW="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"; cat >"$RECEIPTS/merge-receipt.json" <<EOF
{"receipt_type":"oasis7_pr_merge","issuer":"github_live_query","evidence_mode":"production","repository":"fixture/repo","default_branch":"main","pr_number":7,"pr_url":"https://example.invalid/pull/7","state":"MERGED","merged_at":"$NOW","head_oid":"$HEAD_OID","base_ref":"main","observed_at":"$NOW"}
EOF
python3 - "$TASK/.pm/github-project-sync/tasks.json" "$RECEIPTS/merge-receipt.json" <<'PY'
import hashlib,json,sys
p=sys.argv[1]; m=json.load(open(p)); r=next(iter(m['tasks'].values())); r['merge_receipt']=json.load(open(sys.argv[2])); r['merge_receipt_sha256']=hashlib.sha256(open(sys.argv[2],'rb').read()).hexdigest(); open(p,'w').write(json.dumps(m)+'\n')
PY
# Simulate authoritative refresh/readback into the default worktree. The merge
# receipt is durable local authority and is not recoverable from Issue/Project
# fields, so the refreshed default-worktree cache does not contain it yet.
mkdir -p "$DEFAULT/.pm/github-project-sync"; cp "$TASK/.pm/github-project-sync/tasks.json" "$DEFAULT/.pm/github-project-sync/tasks.json"
python3 - "$DEFAULT/.pm/github-project-sync/tasks.json" <<'PY'
import json,sys
p=sys.argv[1]; mapping=json.load(open(p)); r=next(iter(mapping['tasks'].values()))
r.pop('merge_receipt',None); r.pop('merge_receipt_sha256',None)
open(p,'w').write(json.dumps(mapping)+'\n')
assert r['status']=='done' and r['workflow_phase']=='task_done' and not r.get('merge_receipt'),r
PY
mkdir -p "$TMPDIR/bin"; cat >"$TMPDIR/bin/gh" <<EOF
#!/usr/bin/env bash
if [[ "\$*" == "repo view --json nameWithOwner,defaultBranchRef" ]]; then printf '%s\n' '{"nameWithOwner":"fixture/repo","defaultBranchRef":{"name":"main"}}';
elif [[ "\$*" == issue\ view* ]]; then printf '%s\n' '{"state":"CLOSED"}';
elif [[ "\$*" == issue\ comment* ]]; then cp "\${!#}" "$TMPDIR/comment.body"; printf '%s\n' 'https://github.com/fixture/repo/issues/11#issuecomment-1';
elif [[ "\$*" == api\ repos/fixture/repo/issues/11/comments* ]]; then python3 -c 'import json,pathlib,sys; print(json.dumps([[{"id":1,"html_url":"https://github.com/fixture/repo/issues/11#issuecomment-1","body":pathlib.Path(sys.argv[1]).read_text()}]]))' "$TMPDIR/comment.body";
else printf '%s\n' '{"number":7,"url":"https://example.invalid/pull/7","state":"MERGED","mergedAt":"$NOW","headRefOid":"$HEAD_OID","baseRefName":"main"}'; fi
EOF
chmod +x "$TMPDIR/bin/gh"; export PATH="$TMPDIR/bin:$PATH"
BEFORE_SYNC="$(git -C "$DEFAULT" rev-parse main)"
CALLER_CWD="$TMPDIR/isolated-caller"; mkdir -p "$CALLER_CWD"; cd "$CALLER_CWD"
for bad_output in "relative-main-sync.json" "$TASK/main-sync.json"; do
  if "$ROOT_DIR/scripts/pm/post-merge-main-sync.sh" --repo-root "$DEFAULT" --main-ref main \
    --task-uid "$UID_VALUE" --pr-receipt "$RECEIPTS/merge-receipt.json" --receipt-output "$bad_output" \
    >"$RECEIPTS/bad-sync.out" 2>"$RECEIPTS/bad-sync.err"; then
    echo "expected unsafe main-sync receipt output to fail: $bad_output" >&2; exit 1
  fi
  [[ "$(git -C "$DEFAULT" rev-parse main)" == "$BEFORE_SYNC" ]]
  grep -Eiq 'absolute|task worktree|receipt.*path' "$RECEIPTS/bad-sync.err"
done
[[ ! -e "$CALLER_CWD/relative-main-sync.json" ]]
[[ ! -e "$ROOT_DIR/relative-main-sync.json" ]]
"$ROOT_DIR/scripts/pm/post-merge-main-sync.sh" --repo-root "$DEFAULT" --main-ref main --task-uid "$UID_VALUE" --pr-receipt "$RECEIPTS/merge-receipt.json" --receipt-output "$RECEIPTS/main-sync-receipt.json"
python3 - "$DEFAULT/.pm/github-project-sync/tasks.json" "$RECEIPTS/merge-receipt.json" <<'PY'
import hashlib,json,pathlib,sys
r=next(iter(json.load(open(sys.argv[1]))['tasks'].values()))
assert r['merge_receipt']==json.load(open(sys.argv[2])),r
assert r['merge_receipt_sha256']==hashlib.sha256(pathlib.Path(sys.argv[2]).read_bytes()).hexdigest(),r
PY
for bad_output in "relative-terminal.json" "$TASK/terminal.json"; do
  if "$ROOT_DIR/scripts/pm/post-merge-cleanup.sh" --repo-root "$DEFAULT" --worktree "$TASK" --branch "$BRANCH" --main-ref main \
    --task-uid "$UID_VALUE" --pr-receipt "$RECEIPTS/merge-receipt.json" --main-sync-receipt "$RECEIPTS/main-sync-receipt.json" \
    --terminal-receipt-output "$bad_output" >"$RECEIPTS/bad-cleanup.out" 2>"$RECEIPTS/bad-cleanup.err"; then
    echo "expected unsafe cleanup receipt output to fail: $bad_output" >&2; exit 1
  fi
  [[ -d "$TASK" ]]; git -C "$DEFAULT" show-ref --verify --quiet "refs/heads/$BRANCH"
  [[ ! -e "${bad_output}.intent.json" ]]
  grep -Eiq 'absolute|task worktree|receipt.*path' "$RECEIPTS/bad-cleanup.err"
done
"$ROOT_DIR/scripts/pm/post-merge-cleanup.sh" --repo-root "$DEFAULT" --worktree "$TASK" --branch "$BRANCH" --main-ref main --task-uid "$UID_VALUE" --pr-receipt "$RECEIPTS/merge-receipt.json" --main-sync-receipt "$RECEIPTS/main-sync-receipt.json" --terminal-receipt-output "$RECEIPTS/terminal-cleanup-receipt.json"
python3 "$ROOT_DIR/scripts/pm/post-merge-finalize.py" --repo-root "$DEFAULT" --task-uid "$UID_VALUE" --terminal-receipt "$RECEIPTS/terminal-cleanup-receipt.json"
python3 - "$DEFAULT/.pm/github-project-sync/tasks.json" <<'PY'
import json,sys
r=next(iter(json.load(open(sys.argv[1]))['tasks'].values())); assert r['workflow_phase']=='post_merge_done',r
PY
echo "terminal-dual-worktree.test: OK"
