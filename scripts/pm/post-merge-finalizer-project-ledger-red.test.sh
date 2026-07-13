#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"; TMP="$(mktemp -d)"; trap 'rm -rf "$TMP"' EXIT
REPO="$TMP/repo"; UID_VALUE="task_eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"; mkdir -p "$REPO/.pm/github-project-sync" "$TMP/task" "$TMP/bin"
git init -q -b main "$REPO"; RECEIPT_ROOT="$(python3 "$ROOT_DIR/scripts/pm/canonical-receipt-root.py" --default-worktree "$REPO" --task-uid "$UID_VALUE" --create)"
cat >"$REPO/.pm/github-project-sync/tasks.json" <<EOF
{"project":{"owner":"fixture","number":1,"id":"P1"},"tasks":{"$UID_VALUE":{"task_uid":"$UID_VALUE","status":"done","owner_role":"qa_engineer","module":"engineering","repository":"fixture/repo","canonical_worktree":"$TMP/task","issue_number":11,"pr_number":22,"project_item_id":"ITEM1","workflow_phase":"main_sync","merge_receipt":{"state":"MERGED"},"phase_receipts":{"main_sync":{"receipt_type":"oasis7_main_sync"}}}}}
EOF
cat >"$RECEIPT_ROOT/terminal-cleanup-receipt.json" <<EOF
{"receipt_type":"oasis7_terminal_cleanup","issuer":"post-merge-cleanup","task_uid":"$UID_VALUE","repository":"fixture/repo","issue_number":11,"pr_number":22}
EOF
cat >"$TMP/bin/gh" <<'SH'
#!/usr/bin/env bash
printf '%s\n' "$*" >>"$GH_LOG"
case "$*" in
  "project view 1 --owner fixture --format json") printf '%s\n' '{"id":"P1"}' ;;
  "project field-list 1 --owner fixture --format json")
    printf '%s\n' '{"fields":[{"name":"Status","id":"F_STATUS","options":[{"name":"Done","id":"O_DONE"}]},{"name":"PM Status","id":"F_PM","options":[{"name":"done","id":"O_PM_DONE"}]},{"name":"Workflow Phase","id":"F_PHASE","options":[{"name":"done","id":"O_PHASE_DONE"}]}]}' ;;
  project\ item-edit*)
    field=""; prev=""; for arg in "$@"; do [[ "$prev" == --field-id ]] && field="$arg"; prev="$arg"; done
    printf '%s\n' "$field" >>"$EDIT_LOG"; printf '%s\n' "$field" >>"$REMOTE_STATE"; printf '%s\n' '{}'
    if [[ "$(wc -l <"$REMOTE_STATE" | tr -d ' ')" == 3 && ! -e "$CRASHED" ]]; then : >"$CRASHED"; kill -KILL "$PPID"; fi ;;
  "project item-list 1 --owner fixture --limit 1000 --format json")
    echo 'full Project item-list readback is forbidden for a bound item id' >&2; exit 91 ;;
  issue\ comment*)
    prev=""; for arg in "$@"; do [[ "$prev" == --body-file ]] && cp "$arg" "$LIVE_BODY"; prev="$arg"; done
    printf '%s\n' 'https://example.invalid/issues/11#issuecomment-1' ;;
  issue\ view*) [[ -e "$ISSUE_CLOSED" ]] && printf '%s\n' '{"state":"CLOSED"}' || printf '%s\n' '{"state":"OPEN"}' ;;
  issue\ close*) : >"$ISSUE_CLOSED"; printf '%s\n' '{}' ;;
  api\ graphql*)
    printf '%s\n' '{"data":{"nodes":[{"id":"ITEM1","project":{"id":"P1","number":1},"content":{"body":"task_uid: task_eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee","number":11,"title":"fixture","url":"https://example.invalid/issues/11"},"fieldValues":{"nodes":[{"name":"Done","field":{"name":"Status"}},{"name":"done","field":{"name":"PM Status"}},{"name":"done","field":{"name":"Workflow Phase"}}]}}]}}' ;;
  api*) python3 - "$LIVE_BODY" <<'PY'
import json,sys
print(json.dumps([[{"id":1,"html_url":"https://example.invalid/issues/11#issuecomment-1","body":open(sys.argv[1]).read()}]]))
PY
    ;;
  *) printf '%s\n' '{}' ;;
esac
SH
chmod +x "$TMP/bin/gh"; export PATH="$TMP/bin:$PATH" GH_LOG="$TMP/gh.log" EDIT_LOG="$TMP/edit.log" REMOTE_STATE="$TMP/remote-state" CRASHED="$TMP/crashed" ISSUE_CLOSED="$TMP/issue-closed" LIVE_BODY="$TMP/live-comment-body"
set +e
python3 "$ROOT_DIR/scripts/pm/post-merge-finalize.py" --repo-root "$REPO" --task-uid "$UID_VALUE" --terminal-receipt "$RECEIPT_ROOT/terminal-cleanup-receipt.json" >/dev/null 2>&1
first=$?; set -e; [[ "$first" != 0 ]]
python3 "$ROOT_DIR/scripts/pm/post-merge-finalize.py" --repo-root "$REPO" --task-uid "$UID_VALUE" --terminal-receipt "$RECEIPT_ROOT/terminal-cleanup-receipt.json" >/dev/null
for field in F_STATUS F_PM F_PHASE; do
  [[ "$(grep -c "^$field$" "$EDIT_LOG")" -le 1 ]] || { echo "retry duplicated Project edit: $field" >&2; cat "$GH_LOG" >&2; exit 1; }
done
python3 - "$RECEIPT_ROOT/finalizer-ledger.json" "$REPO/.pm/github-project-sync/tasks.json" "$UID_VALUE" <<'PY'
import json,sys
l=json.load(open(sys.argv[1])); op=l['operations']['project_update']
assert op.get('operation_id') and op.get('intent') and op.get('committed'),op
r=json.load(open(sys.argv[2]))['tasks'][sys.argv[3]]; assert r['workflow_phase']=='post_merge_done',r
PY
grep -q '^issue close 11 -R fixture/repo --reason completed$' "$GH_LOG"
grep -q '^api graphql ' "$GH_LOG"
! grep -q '^project item-list ' "$GH_LOG"
echo 'post-merge-finalizer-project-ledger-red.test: OK'
