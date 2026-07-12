#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TMP="$(mktemp -d)"; trap 'rm -rf "$TMP"' EXIT
REPO="$TMP/repo"; UID_VALUE="task_77777777777777777777777777777777"
git init -q -b main "$REPO"; mkdir -p "$REPO/.pm/github-project-sync" "$TMP/task" "$TMP/bin"
ROOT="$(python3 "$ROOT_DIR/scripts/pm/canonical-receipt-root.py" --default-worktree "$REPO" --task-uid "$UID_VALUE" --create)"
cat >"$REPO/.pm/github-project-sync/tasks.json" <<EOF
{"tasks":{"$UID_VALUE":{"task_uid":"$UID_VALUE","repository":"fixture/repo","canonical_worktree":"$TMP/task","issue_number":11,"pr_number":22,"workflow_phase":"main_sync","merge_receipt":{"state":"MERGED"},"phase_receipts":{"main_sync":{"receipt_type":"oasis7_main_sync"}}}}}
EOF
cat >"$ROOT/terminal-cleanup-receipt.json" <<EOF
{"receipt_type":"oasis7_terminal_cleanup","issuer":"post-merge-cleanup","task_uid":"$UID_VALUE","repository":"fixture/repo","issue_number":11,"pr_number":22}
EOF
cat >"$TMP/bin/gh" <<'SH'
#!/usr/bin/env bash
printf '%s\n' "$*" >>"$GH_LOG"
case "$*" in
  issue\ comment*) printf '%s\n' 'https://example.invalid/issues/11#issuecomment-1' ;;
  api*) printf '%s\n' '[[{"id":1,"html_url":"https://example.invalid/issues/11#issuecomment-1","body":"wrong task and missing operation marker"}]]' ;;
  *) printf '%s\n' '{}' ;;
esac
SH
chmod +x "$TMP/bin/gh"; export PATH="$TMP/bin:$PATH" GH_LOG="$TMP/gh.log"
if python3 "$ROOT_DIR/scripts/pm/post-merge-finalize.py" --repo-root "$REPO" --task-uid "$UID_VALUE" \
  --terminal-receipt "$ROOT/terminal-cleanup-receipt.json" >/dev/null 2>"$TMP/error"; then
  echo "created comment without matching live body must fail" >&2; exit 1
fi
python3 - "$ROOT/finalizer-ledger.json" <<'PY'
import json,sys
ledger=json.load(open(sys.argv[1])); entry=ledger['operations']['evidence_comment']
assert not entry.get('committed'),entry
PY
! grep -q '^issue close ' "$GH_LOG"
echo 'post-merge-finalizer-comment-readback-red.test: OK'
