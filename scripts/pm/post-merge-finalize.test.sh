#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TMPDIR="$(mktemp -d)"; trap 'rm -rf "$TMPDIR"' EXIT
FIXTURE="$TMPDIR/repo"; UID_VALUE="task_11111111111111111111111111111111"
mkdir -p "$FIXTURE/.pm/github-project-sync" "$FIXTURE/scripts/pm" "$TMPDIR/bin" "$TMPDIR/canonical-task-worktree"
git init -q -b main "$FIXTURE"
RECEIPT_ROOT="$(python3 "$ROOT_DIR/scripts/pm/canonical-receipt-root.py" --default-worktree "$FIXTURE" --task-uid "$UID_VALUE" --create)"
TERMINAL="$RECEIPT_ROOT/terminal-cleanup-receipt.json"
cat >"$FIXTURE/.pm/github-project-sync/tasks.json" <<EOF
{"tasks":{"$UID_VALUE":{"task_uid":"$UID_VALUE","repository":"fixture/repo","canonical_worktree":"$TMPDIR/canonical-task-worktree","task_branch":"task/finalize","default_branch":"main","issue_number":11,"pr_number":22,"workflow_phase":"main_sync","merge_receipt":{"state":"MERGED"},"phase_receipts":{"main_sync":{"receipt_type":"oasis7_main_sync"}}}}}
EOF
cat >"$TERMINAL" <<EOF
{"receipt_type":"oasis7_terminal_cleanup","issuer":"post-merge-cleanup","cleanup_intent_required":true,"task_uid":"$UID_VALUE","repository":"fixture/repo","issue_number":11,"pr_number":22,"worktree":"$TMPDIR/canonical-task-worktree","branch":"task/finalize"}
EOF
cat >"$RECEIPT_ROOT/cleanup-intent.json" <<EOF
{"receipt_type":"oasis7_cleanup_intent","task_uid":"$UID_VALUE","repository":"fixture/repo","worktree":"$TMPDIR/canonical-task-worktree","branch":"task/finalize","worktree_removed":true,"branch_deleted":true,"terminal_receipt_committed":true}
EOF
cat >"$FIXTURE/scripts/pm/github-project-task.py" <<'PY'
#!/usr/bin/env python3
import json,pathlib,sys
root=pathlib.Path(sys.argv[2]); uid=sys.argv[sys.argv.index('--task-uid')+1]
receipt=pathlib.Path(sys.argv[sys.argv.index('--receipt-json')+1])
p=root/'.pm/github-project-sync/tasks.json'; m=json.loads(p.read_text()); r=m['tasks'][uid]
r['workflow_phase']='post_merge_done'; r.setdefault('phase_receipts',{})['post_merge_done']=json.loads(receipt.read_text())
p.write_text(json.dumps(m)+'\n'); print('{}')
PY
chmod +x "$FIXTURE/scripts/pm/github-project-task.py"
cat >"$TMPDIR/bin/gh" <<'SH'
#!/usr/bin/env bash
printf '%s\n' "$*" >>"$GH_LOG"
if [[ "$*" == issue\ comment* ]]; then
  prev=""; for arg in "$@"; do [[ "$prev" == --body-file ]] && cp "$arg" "$LIVE_BODY"; prev="$arg"; done
  printf '%s\n' 'https://example.invalid/issues/11#issuecomment-1'
elif [[ "$*" == api* ]]; then
  python3 - "$LIVE_BODY" <<'PY'
import json,sys
print(json.dumps([[{"id":1,"html_url":"https://example.invalid/issues/11#issuecomment-1","body":open(sys.argv[1]).read()}]]))
PY
elif [[ "$*" == issue\ view* ]]; then printf '{"state":"CLOSED"}\n'; else printf '{}\n'; fi
SH
chmod +x "$TMPDIR/bin/gh"; export PATH="$TMPDIR/bin:$PATH" GH_LOG="$TMPDIR/gh.log" LIVE_BODY="$TMPDIR/live-comment-body"

python3 "$ROOT_DIR/scripts/pm/post-merge-finalize.py" --repo-root "$FIXTURE" \
  --task-uid "$UID_VALUE" --terminal-receipt "$TERMINAL" >"$TMPDIR/first.json"
python3 "$ROOT_DIR/scripts/pm/post-merge-finalize.py" --repo-root "$FIXTURE" \
  --task-uid "$UID_VALUE" --terminal-receipt "$TERMINAL" >"$TMPDIR/retry.json"
python3 - "$TMPDIR/first.json" "$TMPDIR/retry.json" "$FIXTURE/.pm/github-project-sync/tasks.json" <<'PY'
import json,sys
first=json.loads(open(sys.argv[1]).read().splitlines()[-1]); retry=json.loads(open(sys.argv[2]).read().splitlines()[-1]); mapping=json.load(open(sys.argv[3]))
assert first['status']=='finalized',first
assert retry['status']=='already_finalized',retry
r=next(iter(mapping['tasks'].values())); assert r['workflow_phase']=='post_merge_done',r
PY
[[ "$(grep -c '^issue close 11 -R fixture/repo --reason completed$' "$GH_LOG")" == 1 ]]

# Current-protocol terminal receipts require their matching cleanup journal,
# even when the terminal task mapping is already committed.
cp "$TERMINAL" "$TMPDIR/terminal.current.json"
cp "$RECEIPT_ROOT/cleanup-intent.json" "$TMPDIR/cleanup-intent.current.json"
rm "$RECEIPT_ROOT/cleanup-intent.json"
before_missing_intent="$(wc -l <"$GH_LOG")"
if python3 "$ROOT_DIR/scripts/pm/post-merge-finalize.py" --repo-root "$FIXTURE" \
  --task-uid "$UID_VALUE" --terminal-receipt "$TERMINAL" >/dev/null 2>"$TMPDIR/missing-current-intent.err"; then
  echo "expected current-protocol terminal receipt to require cleanup intent" >&2; exit 1
fi
grep -F "current-protocol terminal receipt requires cleanup intent" "$TMPDIR/missing-current-intent.err" >/dev/null
[[ "$(wc -l <"$GH_LOG")" == "$before_missing_intent" ]]
cp "$TMPDIR/cleanup-intent.current.json" "$RECEIPT_ROOT/cleanup-intent.json"

# A terminal-shaped receipt cannot bypass a durable unresolved cleanup blocker.
python3 - "$RECEIPT_ROOT/cleanup-intent.json" "$UID_VALUE" "$TMPDIR/canonical-task-worktree" <<'PY'
import json,pathlib,sys
path=pathlib.Path(sys.argv[1]); uid,worktree=sys.argv[2:]
path.write_text(json.dumps({
    "receipt_type":"oasis7_cleanup_intent", "task_uid":uid,
    "repository":"fixture/repo", "worktree":str(pathlib.Path(worktree).resolve()),
    "branch":"task/finalize", "branch_tip":"a"*40,
    "remote_branch_blocker":{
        "schema":"oasis7_cleanup_blocker_v1", "kind":"remote_branch_tip_mismatch",
        "branch":"task/finalize", "expected_tip":"a"*40,
        "observed_tip":"b"*40, "resolved":False,
    },
})+"\n",encoding="utf-8")
PY
before_blocked="$(wc -l <"$GH_LOG")"
if python3 "$ROOT_DIR/scripts/pm/post-merge-finalize.py" --repo-root "$FIXTURE" \
  --task-uid "$UID_VALUE" --terminal-receipt "$TERMINAL" >/dev/null 2>"$TMPDIR/blocked.err"; then
  echo "expected finalizer to reject unresolved cleanup blocker" >&2; exit 1
fi
grep -F "unresolved durable blocker" "$TMPDIR/blocked.err" >/dev/null
[[ "$(wc -l <"$GH_LOG")" == "$before_blocked" ]]
python3 - "$RECEIPT_ROOT/cleanup-intent.json" <<'PY'
import json,sys
path=sys.argv[1]; intent=json.load(open(path,encoding="utf-8"))
intent["remote_branch_blocker"].update(
    resolved=True, resolution="matching_tip_deleted", resolved_tip="a"*40,
    resolved_at="2026-09-23T00:00:00+00:00",
)
json.dump(intent,open(path,"w",encoding="utf-8"))
PY
python3 "$ROOT_DIR/scripts/pm/post-merge-finalize.py" --repo-root "$FIXTURE" \
  --task-uid "$UID_VALUE" --terminal-receipt "$TERMINAL" >/dev/null

cp "$TERMINAL" "$TMPDIR/terminal.valid.json"
python3 - "$TERMINAL" <<'PY'
import json,sys
r=json.load(open(sys.argv[1])); r['task_uid']='task_22222222222222222222222222222222'; json.dump(r,open(sys.argv[1],'w'))
PY
if python3 "$ROOT_DIR/scripts/pm/post-merge-finalize.py" --repo-root "$FIXTURE" \
  --task-uid "$UID_VALUE" --terminal-receipt "$TERMINAL" >/dev/null 2>"$TMPDIR/forged.err"; then
  echo "expected mismatched terminal receipt to fail" >&2; exit 1
fi
grep -Fqi 'mismatch' "$TMPDIR/forged.err"
cp "$TMPDIR/terminal.valid.json" "$TERMINAL"

python3 - "$FIXTURE/.pm/github-project-sync/tasks.json" <<'PY'
import json,sys
path=sys.argv[1]
mapping=json.load(open(path))
record=next(iter(mapping["tasks"].values()))
record["workflow_phase"]="main_sync"
record.get("phase_receipts",{}).pop("post_merge_done",None)
record.get("phase_receipt_sha256",{}).pop("post_merge_done",None)
json.dump(mapping,open(path,"w"))
PY
python3 - "$TERMINAL" <<'PY'
import json,sys
path=sys.argv[1]
receipt=json.load(open(path))
receipt["worktree"]="/tmp/forged-task-worktree"
receipt["branch"]="task/forged"
json.dump(receipt,open(path,"w"))
PY
if python3 "$ROOT_DIR/scripts/pm/post-merge-finalize.py" --repo-root "$FIXTURE" \
  --task-uid "$UID_VALUE" --terminal-receipt "$TERMINAL" >/dev/null 2>"$TMPDIR/identity.err"; then
  echo "expected mismatched terminal worktree/branch identity to fail" >&2; exit 1
fi
grep -Eqi 'worktree|branch|identity' "$TMPDIR/identity.err"
cp "$TMPDIR/terminal.valid.json" "$TERMINAL"

# Production-like records reject digest substitution before any finalizer effect.
python3 - "$FIXTURE/.pm/github-project-sync/tasks.json" "$TERMINAL" <<'PY'
import json,pathlib,sys
p=pathlib.Path(sys.argv[1]); terminal=pathlib.Path(sys.argv[2]); m=json.loads(p.read_text()); r=next(iter(m['tasks'].values()))
r['repository']='eng-cc/oasis7'; r['workflow_phase']='main_sync'; r['merge_receipt_sha256']='a'*64
r['phase_receipt_sha256']={'main_sync':'b'*64}; r['phase_receipts']={'main_sync':{'receipt_type':'oasis7_main_sync'}}
p.write_text(json.dumps(m)+'\n')
t=json.loads(terminal.read_text()); t['repository']='eng-cc/oasis7'; t['merge_receipt_sha256']='0'*64; t['main_sync_receipt_sha256']='b'*64
terminal.write_text(json.dumps(t)+'\n')
PY
if python3 "$ROOT_DIR/scripts/pm/post-merge-finalize.py" --repo-root "$FIXTURE" \
  --task-uid "$UID_VALUE" --terminal-receipt "$TERMINAL" >/dev/null 2>"$TMPDIR/digest.err"; then
  echo "expected forged merge digest to fail" >&2; exit 1
fi
grep -Fqi 'merge_receipt_sha256 mismatch' "$TMPDIR/digest.err"

# Generic set-phase is not terminal authority, even with a terminal-shaped receipt.
if python3 "$ROOT_DIR/scripts/pm/github-project-task.py" set-phase "$FIXTURE" --repo eng-cc/oasis7 \
  --task-uid "$UID_VALUE" --phase post_merge_done --receipt-json "$TERMINAL" --json \
  >/dev/null 2>"$TMPDIR/set-phase.err"; then
  echo "expected generic set-phase post_merge_done to fail" >&2; exit 1
fi
if ! grep -Eqi 'not allowed|invalid choice' "$TMPDIR/set-phase.err"; then
  echo "expected transition-policy rejection, got:" >&2; cat "$TMPDIR/set-phase.err" >&2; exit 1
fi

# A markerless legacy receipt with no journal is blocked while pending, but is
# eligible for idempotent finalization after its exact terminal bytes and digest
# are already bound into task truth.
python3 - "$FIXTURE/.pm/github-project-sync/tasks.json" "$TERMINAL" "$RECEIPT_ROOT/cleanup-intent.json" <<'PY'
import hashlib,json,pathlib,sys
mapping=pathlib.Path(sys.argv[1]); terminal=pathlib.Path(sys.argv[2]); intent=pathlib.Path(sys.argv[3])
receipt=json.loads(terminal.read_text(encoding="utf-8")); receipt.pop("cleanup_intent_required",None)
receipt["repository"]="fixture/repo"
receipt.pop("merge_receipt_sha256",None); receipt.pop("main_sync_receipt_sha256",None)
terminal.write_text(json.dumps(receipt)+"\n",encoding="utf-8")
intent.unlink(missing_ok=True)
m=json.loads(mapping.read_text(encoding="utf-8")); r=m["tasks"]["task_11111111111111111111111111111111"]
r.update(repository="fixture/repo",workflow_phase="main_sync",merge_receipt={"state":"MERGED"})
r.pop("merge_receipt_sha256",None)
r["phase_receipts"]={"main_sync":{"receipt_type":"oasis7_main_sync"}}
r["phase_receipt_sha256"]={}
mapping.write_text(json.dumps(m)+"\n",encoding="utf-8")
PY
before_legacy_pending="$(wc -l <"$GH_LOG")"
if python3 "$ROOT_DIR/scripts/pm/post-merge-finalize.py" --repo-root "$FIXTURE" \
  --task-uid "$UID_VALUE" --terminal-receipt "$TERMINAL" >/dev/null 2>"$TMPDIR/missing-legacy-intent.err"; then
  echo "expected pending markerless legacy receipt without cleanup intent to fail closed" >&2; exit 1
fi
grep -F "legacy terminal receipt without cleanup intent requires an already-finalized task" "$TMPDIR/missing-legacy-intent.err" >/dev/null || {
  cat "$TMPDIR/missing-legacy-intent.err" >&2; exit 1;
}
[[ "$(wc -l <"$GH_LOG")" == "$before_legacy_pending" ]]
python3 - "$FIXTURE/.pm/github-project-sync/tasks.json" "$TERMINAL" <<'PY'
import hashlib,json,pathlib,sys
mapping=pathlib.Path(sys.argv[1]); terminal=pathlib.Path(sys.argv[2]); receipt=json.loads(terminal.read_text(encoding="utf-8"))
m=json.loads(mapping.read_text(encoding="utf-8")); r=m["tasks"]["task_11111111111111111111111111111111"]
r["workflow_phase"]="post_merge_done"
r.setdefault("phase_receipts",{})["post_merge_done"]=receipt
r.setdefault("phase_receipt_sha256",{})["post_merge_done"]=hashlib.sha256(terminal.read_bytes()).hexdigest()
mapping.write_text(json.dumps(m)+"\n",encoding="utf-8")
PY
python3 "$ROOT_DIR/scripts/pm/post-merge-finalize.py" --repo-root "$FIXTURE" \
  --task-uid "$UID_VALUE" --terminal-receipt "$TERMINAL" >"$TMPDIR/legacy-finalized.json"
python3 - "$TMPDIR/legacy-finalized.json" <<'PY'
import json,sys
result=json.loads(open(sys.argv[1],encoding="utf-8").read().splitlines()[-1])
assert result["status"]=="already_finalized",result
PY
echo "post-merge-finalize.test: OK"
