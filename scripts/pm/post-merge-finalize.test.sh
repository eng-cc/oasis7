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
  printf '%s\n' "${LIVE_COMMENT_URL:-https://example.invalid/issues/11#issuecomment-1}"
elif [[ "$*" == api* ]]; then
  python3 - "$LIVE_BODY" "${LIVE_COMMENT_URL:-https://example.invalid/issues/11#issuecomment-1}" "${LIVE_COMMENT_ID:-1}" <<'PY'
import json,sys
print(json.dumps([[{"id":int(sys.argv[3]),"html_url":sys.argv[2],"body":open(sys.argv[1]).read()}]]))
PY
elif [[ "$*" == issue\ view* ]]; then printf '{"state":"CLOSED"}\n'; else printf '{}\n'; fi
SH
chmod +x "$TMPDIR/bin/gh"; export PATH="$TMPDIR/bin:$PATH" GH_LOG="$TMPDIR/gh.log" LIVE_BODY="$TMPDIR/live-comment-body"
: >"$GH_LOG"

# A matching current-protocol journal is insufficient unless it proves every
# cleanup step completed; rejection must precede GitHub calls and task writes.
cp "$RECEIPT_ROOT/cleanup-intent.json" "$TMPDIR/cleanup-intent.complete.json"
for flag in worktree_removed branch_deleted terminal_receipt_committed; do
  for state in false missing; do
    python3 - "$RECEIPT_ROOT/cleanup-intent.json" "$flag" "$state" <<'PY'
import json,sys
from pathlib import Path
path=Path(sys.argv[1]); flag,state=sys.argv[2:]
journal=json.loads(path.read_text(encoding="utf-8"))
if state=="false": journal[flag]=False
else: journal.pop(flag,None)
path.write_text(json.dumps(journal)+"\n",encoding="utf-8")
PY
    cp "$FIXTURE/.pm/github-project-sync/tasks.json" "$TMPDIR/mapping.before"
    before_incomplete="$(wc -l <"$GH_LOG")"
    if python3 "$ROOT_DIR/scripts/pm/post-merge-finalize.py" --repo-root "$FIXTURE" \
      --task-uid "$UID_VALUE" --terminal-receipt "$TERMINAL" >/dev/null 2>"$TMPDIR/incomplete.err"; then
      echo "expected finalizer to reject $state cleanup intent flag $flag" >&2; exit 1
    fi
    grep -F "cleanup intent progress is incomplete: $flag" "$TMPDIR/incomplete.err" >/dev/null || {
      cat "$TMPDIR/incomplete.err" >&2; exit 1;
    }
    [[ "$(wc -l <"$GH_LOG")" == "$before_incomplete" ]]
    cmp -s "$TMPDIR/mapping.before" "$FIXTURE/.pm/github-project-sync/tasks.json"
    cp "$TMPDIR/cleanup-intent.complete.json" "$RECEIPT_ROOT/cleanup-intent.json"
  done
done

python3 "$ROOT_DIR/scripts/pm/post-merge-finalize.py" --repo-root "$FIXTURE" \
  --task-uid "$UID_VALUE" --terminal-receipt "$TERMINAL" >"$TMPDIR/first.json"
ACCEPTED_TERMINAL_SHA="$(shasum -a 256 "$TERMINAL" | awk '{print $1}')"
ACCEPTED_COMMENT_SHA="$(shasum -a 256 "$LIVE_BODY" | awk '{print $1}')"
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
[[ "$(shasum -a 256 "$TERMINAL" | awk '{print $1}')" == "$ACCEPTED_TERMINAL_SHA" ]]
[[ "$(shasum -a 256 "$LIVE_BODY" | awk '{print $1}')" == "$ACCEPTED_COMMENT_SHA" ]]

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
    "worktree_removed":True, "branch_deleted":True, "terminal_receipt_committed":True,
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

# Exercise an original markerless historical v1 proof independently from the
# current-protocol fixture. Its receipt and accepted comment bytes are authored
# once, then read back unchanged after historical terminal state is selected.
LEGACY_FIXTURE="$TMPDIR/legacy-repo"
LEGACY_UID="task_22222222222222222222222222222222"
LEGACY_WORKTREE="$TMPDIR/legacy-task-worktree"
LEGACY_BODY="$TMPDIR/legacy-live-comment-body"
LEGACY_COMMENT_URL="https://example.invalid/issues/33#issuecomment-33"
mkdir -p "$LEGACY_FIXTURE/.pm/github-project-sync" "$LEGACY_WORKTREE"
git init -q -b main "$LEGACY_FIXTURE"
LEGACY_RECEIPT_ROOT="$(python3 "$ROOT_DIR/scripts/pm/canonical-receipt-root.py" \
  --default-worktree "$LEGACY_FIXTURE" --task-uid "$LEGACY_UID" --create)"
LEGACY_TERMINAL="$LEGACY_RECEIPT_ROOT/terminal-cleanup-receipt.json"
python3 - "$LEGACY_FIXTURE/.pm/github-project-sync/tasks.json" "$LEGACY_TERMINAL" \
  "$LEGACY_RECEIPT_ROOT/finalizer-ledger.json" "$LEGACY_UID" "$LEGACY_WORKTREE" \
  "$LEGACY_BODY" "$LEGACY_COMMENT_URL" "$ROOT_DIR/scripts/pm" <<'PY'
import hashlib,json,pathlib,sys
mapping_path,terminal_path,ledger_path=map(pathlib.Path,sys.argv[1:4])
uid,worktree,body_path,comment_url=sys.argv[4:8]; body_path=pathlib.Path(body_path)
sys.path.insert(0,sys.argv[8])
from terminal_proof import receipt_chain_digest
repository="fixture/repo"; issue=33; pr=44; pr_url=f"https://example.invalid/pull/{pr}"
terminal={"receipt_type":"oasis7_terminal_cleanup","issuer":"post-merge-cleanup",
          "task_uid":uid,"repository":repository,"issue_number":issue,"pr_number":pr,
          "worktree":worktree,"branch":"task/legacy-markerless"}
raw=(json.dumps(terminal,sort_keys=True)+"\n").encode("utf-8")
terminal_path.write_bytes(raw); terminal_digest=hashlib.sha256(raw).hexdigest()
operation_id=hashlib.sha256(f"{uid}:post_merge_done:evidence_comment".encode()).hexdigest()
chain=receipt_chain_digest(uid,repository,issue,pr,pr_url,"","",terminal_digest)
body=("<!-- oasis7-pm-evidence -->\n"+f"Operation-ID: {operation_id}\nTask UID: {uid}\nEvidence Phase: post_merge_done\n"
      "Receipt Chain Version: 1\nReceipt Type: oasis7_terminal_cleanup\nReceipt Issuer: post-merge-cleanup\n"
      f"PR Number: {pr}\nPR URL: {pr_url}\nMerge Receipt SHA256: \nMain Sync Receipt SHA256: \n"
      f"Terminal Receipt SHA256: {terminal_digest}\nReceipt Chain Digest: {chain}\n"
      "Role: tpm\nCompleted: receipt-bound terminal finalization.\n")
body_path.write_text(body,encoding="utf-8")
record={"task_uid":uid,"status":"done","repository":repository,"issue_number":issue,
        "issue_url":f"https://example.invalid/issues/{issue}","pr_number":pr,"pr_url":pr_url,
        "canonical_worktree":worktree,"task_branch":"task/legacy-markerless","default_branch":"main",
        "workflow_phase":"main_sync","merge_receipt":{"state":"MERGED"},
        "phase_receipts":{"main_sync":{"receipt_type":"oasis7_main_sync"}}}
mapping_path.write_text(json.dumps({"version":1,"tasks":{uid:record}})+"\n",encoding="utf-8")
ledger={"schema":"oasis7_finalizer_ledger_v1","task_uid":uid,"revision":2,
        "operations":{"evidence_comment":{"operation_id":operation_id,"effect":"evidence_comment",
          "intent":True,"action":True,"readback":True,"committed":True,"result":comment_url}}}
ledger_path.write_text(json.dumps(ledger)+"\n",encoding="utf-8")
PY
LEGACY_TERMINAL_SHA="$(shasum -a 256 "$LEGACY_TERMINAL" | awk '{print $1}')"
LEGACY_COMMENT_SHA="$(shasum -a 256 "$LEGACY_BODY" | awk '{print $1}')"
before_legacy_pending="$(wc -l <"$GH_LOG")"
if python3 "$ROOT_DIR/scripts/pm/post-merge-finalize.py" --repo-root "$LEGACY_FIXTURE" \
  --task-uid "$LEGACY_UID" --terminal-receipt "$LEGACY_TERMINAL" >/dev/null 2>"$TMPDIR/missing-legacy-intent.err"; then
  echo "expected pending markerless legacy receipt without cleanup intent to fail closed" >&2; exit 1
fi
grep -F "legacy terminal receipt without cleanup intent requires an already-finalized task" \
  "$TMPDIR/missing-legacy-intent.err" >/dev/null || {
  cat "$TMPDIR/missing-legacy-intent.err" >&2; exit 1;
}
[[ "$(wc -l <"$GH_LOG")" == "$before_legacy_pending" ]]
python3 - "$LEGACY_FIXTURE/.pm/github-project-sync/tasks.json" "$LEGACY_TERMINAL" "$LEGACY_UID" <<'PY'
import hashlib,json,pathlib,sys
mapping=pathlib.Path(sys.argv[1]); terminal=pathlib.Path(sys.argv[2]); uid=sys.argv[3]
receipt=json.loads(terminal.read_text(encoding="utf-8")); raw=terminal.read_bytes()
m=json.loads(mapping.read_text(encoding="utf-8")); r=m["tasks"][uid]
r["workflow_phase"]="post_merge_done"
r.setdefault("phase_receipts",{})["post_merge_done"]=receipt
r.setdefault("phase_receipt_sha256",{})["post_merge_done"]=hashlib.sha256(raw).hexdigest()
mapping.write_text(json.dumps(m)+"\n",encoding="utf-8")
PY
cp "$LEGACY_FIXTURE/.pm/github-project-sync/tasks.json" "$TMPDIR/legacy-mapping.accepted"
before_legacy_accept="$(wc -l <"$GH_LOG")"
LIVE_BODY="$LEGACY_BODY" LIVE_COMMENT_URL="$LEGACY_COMMENT_URL" LIVE_COMMENT_ID=33 \
  python3 "$ROOT_DIR/scripts/pm/post-merge-finalize.py" --repo-root "$LEGACY_FIXTURE" \
  --task-uid "$LEGACY_UID" --terminal-receipt "$LEGACY_TERMINAL" >"$TMPDIR/legacy-finalized.json"
python3 - "$TMPDIR/legacy-finalized.json" <<'PY'
import json,sys
result=json.loads(open(sys.argv[1],encoding="utf-8").read().splitlines()[-1])
assert result["status"]=="already_finalized",result
PY
[[ "$(shasum -a 256 "$LEGACY_TERMINAL" | awk '{print $1}')" == "$LEGACY_TERMINAL_SHA" ]]
[[ "$(shasum -a 256 "$LEGACY_BODY" | awk '{print $1}')" == "$LEGACY_COMMENT_SHA" ]]
cmp -s "$TMPDIR/legacy-mapping.accepted" "$LEGACY_FIXTURE/.pm/github-project-sync/tasks.json"
[[ "$(wc -l <"$GH_LOG")" -gt "$before_legacy_accept" ]]
! grep -F 'issue close 33 -R fixture/repo --reason completed' "$GH_LOG" >/dev/null
echo "post-merge-finalize.test: OK"
