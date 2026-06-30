#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${PM_ROOT_DIR:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

TMPDIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

mkdir -p "$TMPDIR/.pm/github-project-sync" "$TMPDIR/bin"
cp "$ROOT_DIR/scripts/pm/github-project-task.py" "$TMPDIR/github-project-task.py"
cp "$ROOT_DIR/scripts/pm/github-project-sync.py" "$TMPDIR/github-project-sync.py"
cp "$ROOT_DIR/scripts/pm/claim-ready.sh" "$TMPDIR/claim-ready.sh"

cat > "$TMPDIR/bin/gh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%q ' "$@" >> "$GH_CALL_LOG"
printf '\n' >> "$GH_CALL_LOG"
case "$*" in
  "issue create -R eng-cc/oasis7 --title "*)
    printf 'https://github.com/eng-cc/oasis7/issues/2001\n'
    ;;
  "issue comment 2001 -R eng-cc/oasis7 --body-file "*)
    n=$(( $(wc -l < "$GH_COMMENT_LOG") + 1 ))
    printf 'comment-%s\n' "$n" >> "$GH_COMMENT_LOG"
    printf 'https://github.com/eng-cc/oasis7/issues/2001#issuecomment-%s\n' "$n"
    ;;
  "issue edit 2001 -R eng-cc/oasis7 --body-file "*)
    printf '%s\n' '--- issue edit body ---' >> "$GH_EDIT_BODY_LOG"
    cat "${@: -1}" >> "$GH_EDIT_BODY_LOG"
    printf '\n' >> "$GH_EDIT_BODY_LOG"
    printf 'edited\n'
    ;;
  "issue list -R eng-cc/oasis7 --search task_99999999999999999999999999999999 in:body --json number,url,title,state --limit 5")
    printf '[{"number":2003,"state":"OPEN","title":"[PM] No-cache task","url":"https://github.com/eng-cc/oasis7/issues/2003"}]\n'
    ;;
  "issue view 2003 -R eng-cc/oasis7 --json body,number,title,url")
    cat <<'JSON'
{"body":"<!-- oasis7-pm-task -->\ntask_uid: task_99999999999999999999999999999999\n\nGitHub-backed oasis7 PM task.\n\nTask metadata:\n- owner_role: `tpm`\n- module: `engineering`\n- status: `ready`\n- priority: `P2`\n- worktree_hint: `/tmp/no-cache-worktree`\n","number":2003,"title":"[PM] No-cache task","url":"https://github.com/eng-cc/oasis7/issues/2003"}
JSON
    ;;
  "issue edit 2003 -R eng-cc/oasis7 --body-file "*)
    printf '%s\n' '--- issue edit body 2003 ---' >> "$GH_EDIT_BODY_LOG"
    cat "${@: -1}" >> "$GH_EDIT_BODY_LOG"
    printf '\n' >> "$GH_EDIT_BODY_LOG"
    printf 'edited\n'
    ;;
  "issue comment 2003 -R eng-cc/oasis7 --body-file "*)
    n=$(( $(wc -l < "$GH_COMMENT_LOG") + 1 ))
    printf 'comment-%s\n' "$n" >> "$GH_COMMENT_LOG"
    printf 'https://github.com/eng-cc/oasis7/issues/2003#issuecomment-%s\n' "$n"
    ;;
  "project item-add 1 --owner eng-cc --url https://github.com/eng-cc/oasis7/issues/2001 --format json")
    printf '{"id":"ITEM_ID","content":{"url":"https://github.com/eng-cc/oasis7/issues/2001"}}\n'
    ;;
  "project view 1 --owner eng-cc --format json")
    printf '{"id":"PROJECT_ID","number":1,"title":"oasis7 Engineering PM","url":"https://github.com/users/eng-cc/projects/1"}\n'
    ;;
  "project field-list 1 --owner eng-cc --format json")
    cat <<'JSON'
{"fields":[
{"id":"FIELD_STATUS","name":"Status","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_TODO","name":"Todo"},{"id":"OPT_IN_PROGRESS","name":"In Progress"},{"id":"OPT_BLOCKED_STATUS","name":"Blocked"},{"id":"OPT_READY","name":"Ready / PR"},{"id":"OPT_PR_WATCH","name":"PR Watch"},{"id":"OPT_DONE_STATUS","name":"Done"}]},
{"id":"FIELD_TASK_UID","name":"Task UID","type":"ProjectV2Field"},
{"id":"FIELD_OWNER_ROLE","name":"Owner Role","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_TPM","name":"tpm"}]},
{"id":"FIELD_MODULE","name":"Module","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_ENGINEERING","name":"engineering"}]},
{"id":"FIELD_PM_STATUS","name":"PM Status","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_CANDIDATE","name":"candidate"},{"id":"OPT_COMMITTED","name":"committed"},{"id":"OPT_READY_PM","name":"ready"},{"id":"OPT_PR_WATCH_PM","name":"pr_watch"},{"id":"OPT_DONE","name":"done"}]},
{"id":"FIELD_WORKFLOW_PHASE","name":"Workflow Phase","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_EXECUTION","name":"execution"},{"id":"OPT_CLOSEOUT","name":"closeout"},{"id":"OPT_PR_WATCH_PHASE","name":"pr_watch"},{"id":"OPT_DONE_PHASE","name":"done"}]},
{"id":"FIELD_PRIORITY","name":"Priority","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_P2","name":"P2"}]},
{"id":"FIELD_BLOCKED","name":"Blocked Reason","type":"ProjectV2Field"},
{"id":"FIELD_WORKTREE","name":"Canonical Worktree","type":"ProjectV2Field"},
{"id":"FIELD_PR","name":"PR","type":"ProjectV2Field"},
{"id":"FIELD_TIER","name":"Test Tier Required","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_NA","name":"n/a"}]},
{"id":"FIELD_UPDATED","name":"Last PM Update","type":"ProjectV2Field"}]}
JSON
    ;;
  project\ item-edit*)
    printf '{}\n'
    ;;
  *)
    echo "unexpected gh invocation: $*" >&2
    exit 9
    ;;
esac
SH
chmod +x "$TMPDIR/bin/gh"
export PATH="$TMPDIR/bin:$PATH"
export GH_CALL_LOG="$TMPDIR/gh-calls.log"
export GH_COMMENT_LOG="$TMPDIR/gh-comments.log"
export GH_EDIT_BODY_LOG="$TMPDIR/issue-body-edited.md"
: > "$GH_CALL_LOG"
: > "$GH_COMMENT_LOG"
: > "$GH_EDIT_BODY_LOG"

NEW_JSON="$TMPDIR/new.json"
python3 "$TMPDIR/github-project-task.py" new-task "$TMPDIR" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --owner-role tpm \
  --title "GitHub-backed lifecycle smoke" \
  --module engineering \
  --priority P2 \
  --source-ref doc/engineering/workflow/source-of-truth.md \
  --worktree-hint "$TMPDIR/worktree" \
  --json > "$NEW_JSON"

TASK_UID="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["task_uid"])' "$NEW_JSON")"

python3 "$TMPDIR/github-project-task.py" move-task "$TMPDIR" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --task-uid "$TASK_UID" \
  --to-status committed \
  --json > "$TMPDIR/move-committed.json"

python3 "$TMPDIR/github-project-task.py" workflow-report "$TMPDIR" \
  --repo eng-cc/oasis7 \
  --task-uid "$TASK_UID" \
  --role tpm \
  --phase start \
  --json > "$TMPDIR/start.json"

python3 "$TMPDIR/github-project-task.py" append-execution-log "$TMPDIR" \
  --repo eng-cc/oasis7 \
  --task-uid "$TASK_UID" \
  --role tpm \
  --completed "created GitHub-backed task lifecycle" \
  --pending "none" \
  --action "exercise active PM wrapper" \
  --validation-command "fake-gh lifecycle smoke" \
  --expected-result "comments and fields update" \
  --actual-result "comments and fields update" \
  --blocker-next-action "n/a" \
  --json > "$TMPDIR/append.json"

set +e
python3 "$TMPDIR/github-project-task.py" move-task "$TMPDIR" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --task-uid "$TASK_UID" \
  --to-status done \
  --json > "$TMPDIR/move-done-without-closeout.json" 2>"$TMPDIR/move-done-without-closeout.err"
MOVE_DONE_WITHOUT_CLOSEOUT_STATUS=$?
set -e
if [[ "$MOVE_DONE_WITHOUT_CLOSEOUT_STATUS" == "0" ]]; then
  echo "github-project-task.test: expected done move without closeout verification to fail" >&2
  exit 1
fi
if ! grep -Fq "refusing done without closeout" "$TMPDIR/move-done-without-closeout.err"; then
  echo "github-project-task.test: expected closeout verification failure message" >&2
  cat "$TMPDIR/move-done-without-closeout.err" >&2
  exit 1
fi

PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/task-closeout.sh" \
  --role tpm \
  --task-uid "$TASK_UID" \
  --verify-command "true" \
  --json > "$TMPDIR/closeout.json"

python3 "$TMPDIR/github-project-task.py" record-pr "$TMPDIR" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --task-uid "$TASK_UID" \
  --pr-url "https://github.com/eng-cc/oasis7/pull/2002" \
  --json > "$TMPDIR/record-pr.json"

python3 - "$TMPDIR/.pm/github-project-sync/tasks.json" "$TASK_UID" "$GH_CALL_LOG" "$GH_COMMENT_LOG" "$TMPDIR/issue-body-edited.md" <<'PY'
import json, pathlib, sys
mapping = json.loads(pathlib.Path(sys.argv[1]).read_text())
uid = sys.argv[2]
calls = pathlib.Path(sys.argv[3]).read_text()
comments = pathlib.Path(sys.argv[4]).read_text().splitlines()
edited_body = pathlib.Path(sys.argv[5]).read_text()
record = mapping["tasks"][uid]
assert record["issue_url"] == "https://github.com/eng-cc/oasis7/issues/2001", record
assert record["project_item_id"] == "ITEM_ID", record
assert record["status"] == "pr_watch", record
assert record["pr_url"] == "https://github.com/eng-cc/oasis7/pull/2002", record
assert record["pr_number"] == 2002, record
assert record["worktree_hint"].endswith("/worktree"), record
assert len(comments) == 5, comments
assert record["claim_verifications"][-1]["status"] == "verified", record
assert "issue create" in calls, calls
assert "issue edit 2001" in calls, calls
assert f"task_uid: {uid}" in edited_body, edited_body
assert "- status: `committed`" in edited_body, edited_body
assert "- status: `ready`" in edited_body, edited_body
assert "- status: `pr_watch`" in edited_body, edited_body
assert f"- worktree_hint: `{record['worktree_hint']}`" in edited_body, edited_body
assert "project item-add" in calls, calls
assert "project item-edit" in calls, calls
assert not pathlib.Path(sys.argv[1]).parent.parent.joinpath("tasks").exists(), "must not create .pm/tasks"
PY

NO_CACHE_ROOT="$TMPDIR/no-cache"
mkdir -p "$NO_CACHE_ROOT"
NO_CACHE_UID="task_99999999999999999999999999999999"
python3 "$TMPDIR/github-project-task.py" move-task "$NO_CACHE_ROOT" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --task-uid "$NO_CACHE_UID" \
  --to-status ready \
  --json > "$TMPDIR/no-cache-move.json"

python3 "$TMPDIR/github-project-task.py" record-pr "$NO_CACHE_ROOT" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --task-uid "$NO_CACHE_UID" \
  --pr-url "https://github.com/eng-cc/oasis7/pull/2003" \
  --json > "$TMPDIR/no-cache-record-pr.json"

python3 - "$NO_CACHE_ROOT/.pm/github-project-sync/tasks.json" "$TMPDIR/no-cache-record-pr.json" "$TMPDIR/issue-body-edited.md" <<'PY'
from __future__ import annotations

import json
import pathlib
import sys

mapping_path = pathlib.Path(sys.argv[1])
payload = json.loads(pathlib.Path(sys.argv[2]).read_text(encoding="utf-8"))
edited_body = pathlib.Path(sys.argv[3]).read_text(encoding="utf-8")
assert not mapping_path.exists(), "no-cache record-pr must not recreate committed mapping cache"
assert payload["status"] == "pr_watch", payload
assert payload["pr_number"] == 2003, payload
assert payload["updated_field_values"] == 0, payload
assert "- pr_url: `https://github.com/eng-cc/oasis7/pull/2003`" in edited_body, edited_body
assert "- pr_number: `2003`" in edited_body, edited_body
assert "- status: `pr_watch`" in edited_body, edited_body
PY

echo "github-project-task.test: OK"
