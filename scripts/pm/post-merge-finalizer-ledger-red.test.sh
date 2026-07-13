#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TMP="$(mktemp -d)"; trap 'rm -rf "$TMP"' EXIT
REPO="$TMP/repo"; UID_VALUE="task_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
git init -q -b main "$REPO"
mkdir -p "$REPO/.pm/github-project-sync" "$TMP/task" "$TMP/bin"
RECEIPT_ROOT="$(python3 "$ROOT_DIR/scripts/pm/canonical-receipt-root.py" --default-worktree "$REPO" --task-uid "$UID_VALUE" --create)"
cat >"$REPO/.pm/github-project-sync/tasks.json" <<EOF
{"tasks":{"$UID_VALUE":{"task_uid":"$UID_VALUE","repository":"fixture/repo","canonical_worktree":"$TMP/task","issue_number":11,"pr_number":22,"workflow_phase":"main_sync","merge_receipt":{"state":"MERGED"},"phase_receipts":{"main_sync":{"receipt_type":"oasis7_main_sync"}}}}}
EOF
cat >"$RECEIPT_ROOT/terminal-cleanup-receipt.json" <<EOF
{"receipt_type":"oasis7_terminal_cleanup","issuer":"post-merge-cleanup","task_uid":"$UID_VALUE","repository":"fixture/repo","issue_number":11,"pr_number":22}
EOF
cat >"$TMP/bin/gh" <<'SH'
#!/usr/bin/env bash
printf '%s\n' "$*" >>"$GH_LOG"
if [[ "$*" == "issue comment 11 -R fixture/repo"* ]]; then
  prev=""; for arg in "$@"; do [[ "$prev" == --body-file ]] && cp "$arg" "$LIVE_BODY"; prev="$arg"; done
  printf '%s\n' 'https://example.invalid/issues/11#issuecomment-777'
  if [[ ! -e "$CRASHED" ]]; then : >"$CRASHED"; kill -KILL "$PPID"; fi
elif [[ "$*" == issue\ close* ]]; then
  : >"$ISSUE_CLOSED"; printf '%s\n' '{}'
elif [[ "$*" == issue\ view* ]]; then
  [[ -e "$ISSUE_CLOSED" ]] && printf '%s\n' '{"state":"CLOSED"}' || printf '%s\n' '{"state":"OPEN"}'
elif [[ "$*" == api* ]]; then
  python3 - "$LIVE_BODY" <<'PY'
import json,sys
print(json.dumps([[{"id":777,"html_url":"https://example.invalid/issues/11#issuecomment-777","body":open(sys.argv[1]).read()}]]))
PY
else
  printf '%s\n' '{}'
fi
SH
chmod +x "$TMP/bin/gh"; export PATH="$TMP/bin:$PATH" GH_LOG="$TMP/gh.log" CRASHED="$TMP/crashed" ISSUE_CLOSED="$TMP/issue-closed" LIVE_BODY="$TMP/live-comment-body"
set +e
python3 "$ROOT_DIR/scripts/pm/post-merge-finalize.py" --repo-root "$REPO" --task-uid "$UID_VALUE" \
  --terminal-receipt "$RECEIPT_ROOT/terminal-cleanup-receipt.json" >/dev/null 2>&1
first=$?
set -e
[[ "$first" != 0 ]] || { echo 'expected crash after successful remote comment' >&2; exit 1; }
python3 "$ROOT_DIR/scripts/pm/post-merge-finalize.py" --repo-root "$REPO" --task-uid "$UID_VALUE" \
  --terminal-receipt "$RECEIPT_ROOT/terminal-cleanup-receipt.json" >/dev/null
[[ "$(grep -c '^issue comment 11 -R fixture/repo ' "$GH_LOG")" == 1 ]] || {
  echo 'retry duplicated the successful evidence comment' >&2; cat "$GH_LOG" >&2; exit 1; }
python3 - "$REPO/.pm/github-project-sync/tasks.json" "$UID_VALUE" <<'PY'
import json,sys
r=json.load(open(sys.argv[1]))['tasks'][sys.argv[2]]
assert r['workflow_phase']=='post_merge_done',r
PY
grep -q '^issue close 11 -R fixture/repo --reason completed$' "$GH_LOG"
echo 'post-merge-finalizer-ledger-red.test: OK'
