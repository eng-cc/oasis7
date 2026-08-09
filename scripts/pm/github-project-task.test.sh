#!/usr/bin/env bash
set -euo pipefail
export OASIS7_TEST_ALLOW_UNATTESTED_DISPATCH_RECEIPTS=1

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
cp "$ROOT_DIR/scripts/pm/portable_file_lock.py" "$TMPDIR/portable_file_lock.py"
cp "$ROOT_DIR/scripts/pm/claim-ready.sh" "$TMPDIR/claim-ready.sh"

cat > "$TMPDIR/bin/gh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%q ' "$@" >> "$GH_CALL_LOG"
printf '\n' >> "$GH_CALL_LOG"
case "$*" in
  api\ graphql*)
    python3 - "$GH_MAPPING_PATH" <<'PY'
import json, os, sys
m=json.load(open(sys.argv[1])); uid,next_record=next(iter(m["tasks"].items())); s=next_record["status"]
state_file=os.environ.get("GH_PROJECT_STATE_FILE")
if state_file and os.path.exists(state_file): s=open(state_file).read().strip() or s
status={"committed":"In Progress","ready":"Ready / PR","pr_watch":"PR Watch","done":"In Progress"}.get(s,"In Progress")
phase={"committed":"execution","ready":"pre_pr_ready","pr_watch":"pr_watch","done":"done"}.get(s,"execution")
nodes=[{"name":status,"field":{"name":"Status"}},{"text":uid,"field":{"name":"Task UID"}},{"name":next_record["owner_role"],"field":{"name":"Owner Role"}},{"name":next_record["module"],"field":{"name":"Module"}},{"name":s,"field":{"name":"PM Status"}},{"name":phase,"field":{"name":"Workflow Phase"}},{"name":next_record["priority"],"field":{"name":"Priority"}},{"text":next_record["worktree_hint"],"field":{"name":"Canonical Worktree"}},{"name":"n/a","field":{"name":"Test Tier Required"}}]
if next_record.get("pr_url"): nodes.append({"text":next_record["pr_url"],"field":{"name":"PR"}})
node={"id":next_record.get("project_item_id") or "ITEM_ID","project":{"id":"PROJECT_ID","number":1},"content":{"body":f"task_uid: {uid}","number":next_record["issue_number"],"title":"[PM] "+next_record["title"],"url":next_record["issue_url"]},"fieldValues":{"nodes":nodes}}
print(json.dumps({"data":{"nodes":[node]}}))
PY
    ;;
  "issue create -R eng-cc/oasis7 --title "*)
    printf 'https://github.com/eng-cc/oasis7/issues/2001\n'
    ;;
  issue\ list\ -R\ eng-cc/oasis7\ --search\ task_*\ in:body\ --json\ number,url,title,state\ --limit\ 5)
    if [[ "$*" == *"task_99999999999999999999999999999999"* ]]; then
      printf '[{"number":2003,"state":"OPEN","title":"[PM] No-cache task","url":"https://github.com/eng-cc/oasis7/issues/2003"}]\n'
    else
      printf '[{"number":2001,"state":"OPEN","title":"[PM] GitHub-backed lifecycle smoke","url":"https://github.com/eng-cc/oasis7/issues/2001"}]\n'
    fi
    ;;
  "issue view 2001 -R eng-cc/oasis7 --json body,number,title,url")
    uid="$(python3 -c 'import json,os; print(next(iter(json.load(open(os.environ["GH_MAPPING_PATH"]))["tasks"])))')"
    printf '{"body":"task_uid: %s\\nTask metadata:\\n- owner_role: `tpm`\\n- module: `engineering`\\n- status: `committed`\\n- priority: `P2`\\n- worktree_hint: `%s/worktree`\\nAcceptance:\\n","number":2001,"title":"[PM] GitHub-backed lifecycle smoke","url":"https://github.com/eng-cc/oasis7/issues/2001"}\n' "$uid" "$(dirname "$(dirname "$(dirname "$GH_MAPPING_PATH")")")"
    ;;
  "issue comment 2001 -R eng-cc/oasis7 --body-file "*)
    n=$(( $(wc -l < "$GH_COMMENT_LOG") + 1 ))
    printf 'comment-%s\n' "$n" >> "$GH_COMMENT_LOG"
    printf 'https://github.com/eng-cc/oasis7/issues/2001#issuecomment-%s\n' "$n"
    ;;
  "issue close 2001 -R eng-cc/oasis7 --reason completed")
    printf 'closed\n'
    ;;
  "issue edit 2001 -R eng-cc/oasis7 --body-file "*)
    if [[ "${GH_INTERRUPT_ISSUE_EDIT:-0}" == "1" ]]; then
      kill -TERM "${GH_INTERRUPT_TARGET:?missing explicit interrupt target}"
      sleep 1
      exit 143
    fi
    if [[ "${GH_FAIL_ISSUE_EDIT:-0}" == "1" ]]; then
      echo "injected second-stage issue edit failure" >&2
      exit 77
    fi
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
  "issue edit 2004 -R eng-cc/oasis7 --body-file "*)
    printf '%s\n' '--- issue edit body 2004 ---' >> "$GH_EDIT_BODY_LOG"
    cat "${@: -1}" >> "$GH_EDIT_BODY_LOG"
    printf '\n' >> "$GH_EDIT_BODY_LOG"
    printf 'edited\n'
    ;;
  "issue edit 2005 -R eng-cc/oasis7 --body-file "*|"issue edit 2006 -R eng-cc/oasis7 --body-file "*)
    printf 'edited\n'
    ;;
  "issue close 2004 -R eng-cc/oasis7 --reason completed")
    printf 'closed\n'
    ;;
  "issue comment 2003 -R eng-cc/oasis7 --body-file "*)
    n=$(( $(wc -l < "$GH_COMMENT_LOG") + 1 ))
    printf 'comment-%s\n' "$n" >> "$GH_COMMENT_LOG"
    printf 'https://github.com/eng-cc/oasis7/issues/2003#issuecomment-%s\n' "$n"
    ;;
  "issue comment 2006 -R eng-cc/oasis7 --body-file "*)
    printf 'https://github.com/eng-cc/oasis7/issues/2006#issuecomment-2006\n'
    ;;
  "project item-add 1 --owner eng-cc --url https://github.com/eng-cc/oasis7/issues/2001 --format json")
    printf '{"id":"ITEM_ID","content":{"url":"https://github.com/eng-cc/oasis7/issues/2001"}}\n'
    ;;
  "project view 1 --owner eng-cc --format json")
    printf '{"id":"PROJECT_ID","number":1,"title":"oasis7 Engineering PM","url":"https://github.com/users/eng-cc/projects/1"}\n'
    ;;
  "project view 2 --owner eng-cc --format json")
    printf '{"id":"PROJECT_ID_2","number":2,"title":"oasis7 Engineering PM Empty Fixture","url":"https://github.com/users/eng-cc/projects/2"}\n'
    ;;
  "project view 3 --owner eng-cc --format json")
    printf '{"id":"PROJECT_ID_3","number":3,"title":"oasis7 Engineering PM Missing Done Option Fixture","url":"https://github.com/users/eng-cc/projects/3"}\n'
    ;;
  "project field-list 1 --owner eng-cc --format json")
    cat <<'JSON'
{"fields":[
{"id":"FIELD_STATUS","name":"Status","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_TODO","name":"Todo"},{"id":"OPT_IN_PROGRESS","name":"In Progress"},{"id":"OPT_BLOCKED_STATUS","name":"Blocked"},{"id":"OPT_READY","name":"Ready / PR"},{"id":"OPT_PR_WATCH","name":"PR Watch"},{"id":"OPT_DONE_STATUS","name":"Done"}]},
{"id":"FIELD_TASK_UID","name":"Task UID","type":"ProjectV2Field"},
{"id":"FIELD_OWNER_ROLE","name":"Owner Role","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_TPM","name":"tpm"}]},
{"id":"FIELD_MODULE","name":"Module","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_ENGINEERING","name":"engineering"}]},
{"id":"FIELD_PM_STATUS","name":"PM Status","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_CANDIDATE","name":"candidate"},{"id":"OPT_COMMITTED","name":"committed"},{"id":"OPT_READY_PM","name":"ready"},{"id":"OPT_PR_WATCH_PM","name":"pr_watch"},{"id":"OPT_DONE","name":"done"}]},
{"id":"FIELD_WORKFLOW_PHASE","name":"Workflow Phase","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_EXECUTION","name":"execution"},{"id":"OPT_PRE_PR_READY","name":"pre_pr_ready"},{"id":"OPT_PR_WATCH_PHASE","name":"pr_watch"},{"id":"OPT_DONE_PHASE","name":"done"}]},
{"id":"FIELD_PRIORITY","name":"Priority","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_P2","name":"P2"}]},
{"id":"FIELD_BLOCKED","name":"Blocked Reason","type":"ProjectV2Field"},
{"id":"FIELD_WORKTREE","name":"Canonical Worktree","type":"ProjectV2Field"},
{"id":"FIELD_PR","name":"PR","type":"ProjectV2Field"},
{"id":"FIELD_TIER","name":"Test Tier Required","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_NA","name":"n/a"}]},
{"id":"FIELD_UPDATED","name":"Last PM Update","type":"ProjectV2Field"}]}
JSON
    ;;
  "project field-list 2 --owner eng-cc --format json")
    printf '{"fields":[]}\n'
    ;;
  "project field-list 3 --owner eng-cc --format json")
    cat <<'JSON'
{"fields":[
{"id":"FIELD_STATUS_3","name":"Status","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_TODO_3","name":"Todo"},{"id":"OPT_IN_PROGRESS_3","name":"In Progress"}]},
{"id":"FIELD_PM_STATUS_3","name":"PM Status","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_DONE_PM_3","name":"done"}]},
{"id":"FIELD_WORKFLOW_PHASE_3","name":"Workflow Phase","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_DONE_PHASE_3","name":"done"}]}
]}
JSON
    ;;
  "project item-list 1 --owner eng-cc --limit 1000 --format json")
    cat <<'JSON'
{"items":[
{"id":"ITEM_ID_2004","content":{"url":"https://github.com/eng-cc/oasis7/issues/2004","body":"<!-- oasis7-pm-task -->\ntask_uid: task_44444444444444444444444444444444\n\nGitHub-backed oasis7 PM task.\n"}}
]}
JSON
    ;;
  project\ item-edit*)
    if [[ "$*" == *"OPT_COMMITTED"* ]]; then
      printf 'committed\n' >"$GH_PROJECT_STATE_FILE"
    elif [[ "$*" == *"OPT_READY_PM"* ]]; then
      printf 'ready\n' >"$GH_PROJECT_STATE_FILE"
    elif [[ "$*" == *"OPT_PR_WATCH_PM"* ]]; then
      printf 'pr_watch\n' >"$GH_PROJECT_STATE_FILE"
    elif [[ "$*" == *"OPT_DONE"* ]]; then
      printf 'done\n' >"$GH_PROJECT_STATE_FILE"
    fi
    printf '{}\n'
    ;;
  *)
    echo "unexpected gh invocation: $*" >&2
    exit 9
    ;;
esac
SH
chmod +x "$TMPDIR/bin/gh"
rm -f "$TMPDIR/xcrun_db"
# The fixture's closeout interruption path can leave mktemp's Darwin `tmp*`
# scratch file in the fixture repository.  This is confined to the disposable
# fixture; the production freeze check still reports every other untracked path.
printf '*.json\n*.log\n*.md\n*.err\n.pm/\nworktree/\nproject-live-state\nxcrun_db\ntmp*\n' > "$TMPDIR/.gitignore"
git -C "$TMPDIR" init -q
git -C "$TMPDIR" config user.email test@example.com
git -C "$TMPDIR" config user.name Test
git -C "$TMPDIR" add .
git -C "$TMPDIR" commit -qm initial
export PATH="$TMPDIR/bin:$PATH"
export GH_CALL_LOG="$TMPDIR/gh-calls.log"
export GH_MAPPING_PATH="$TMPDIR/.pm/github-project-sync/tasks.json"
export GH_PROJECT_STATE_FILE="$TMPDIR/project-live-state"
printf 'candidate\n' >"$GH_PROJECT_STATE_FILE"
export GH_COMMENT_LOG="$TMPDIR/gh-comments.log"
export GH_EDIT_BODY_LOG="$TMPDIR/issue-body-edited.md"
export OASIS7_ALLOW_FIXTURE_VERIFICATION_PROFILE=1
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
REVIEW_PACKET="$TMPDIR/review-packet.md"
HEAD_SHA="$(git -C "$TMPDIR" rev-parse HEAD)"
LEDGER_DIR="$TMPDIR/.pm/scratch/$TASK_UID"
mkdir -p "$LEDGER_DIR"
printf 'fixture return\n' >"$LEDGER_DIR/return.md"
RETURN_SHA="$(shasum -a 256 "$LEDGER_DIR/return.md" | awk '{print $1}')"
printf '{"receipt_type":"oasis7_subagent_dispatch","issuer":"codex_runtime","dispatch_id":"11111111-1111-4111-8111-111111111111","role":"repository_health_engineer","source_head":"%s","contract_digest":"%064d"}\n' "$HEAD_SHA" 0 >"$LEDGER_DIR/dispatch.json"
printf '{"task_uid":"%s","role":"repository_health_engineer","status":"completed","head":"%s","slice_id":"11111111-1111-4111-8111-111111111111","dispatch_receipt":".pm/scratch/%s/dispatch.json","activation":"message-assigned","context_delivery":"full-history","actual_runtime":"inherited/unverified: fixture","artifact_digest":"%s","scope_verdict":"approved","risk_verdict":"approved","findings":"no_findings","residual_risk":"fixture","artifacts":[".pm/scratch/%s/return.md"]}\n' "$TASK_UID" "$HEAD_SHA" "$TASK_UID" "$RETURN_SHA" "$TASK_UID" >"$LEDGER_DIR/slice-ledger.jsonl"
printf -- "- Pre-PR Local Role Review: passed\n- Source Head: %s\n- Review Roles: repository_health_engineer\n- Slice Ledger: .pm/scratch/%s/slice-ledger.jsonl\n" "$HEAD_SHA" "$TASK_UID" >"$REVIEW_PACKET"

python3 "$TMPDIR/github-project-task.py" move-task "$TMPDIR" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --task-uid "$TASK_UID" \
  --to-status committed \
  --json > "$TMPDIR/move-committed.json"

MAPPING_BEFORE_MISSING_WORKFLOW_REPORT="$(shasum -a 256 "$TMPDIR/.pm/github-project-sync/tasks.json" | awk '{print $1}')"
for phase in start close; do
  set +e
  python3 "$TMPDIR/github-project-task.py" workflow-report "$TMPDIR" \
    --repo eng-cc/oasis7 \
    --role tpm \
    --phase "$phase" \
    --json >"$TMPDIR/workflow-report-$phase-without-task.json" 2>"$TMPDIR/workflow-report-$phase-without-task.err"
  MISSING_TASK_WORKFLOW_REPORT_STATUS=$?
  set -e
  if [[ "$MISSING_TASK_WORKFLOW_REPORT_STATUS" == "0" ]]; then
    echo "github-project-task.test: workflow-report $phase must reject missing --task-uid" >&2
    exit 1
  fi
  if ! grep -Fq -- "--task-uid is required" "$TMPDIR/workflow-report-$phase-without-task.err"; then
    echo "github-project-task.test: workflow-report $phase must explain the required task identity" >&2
    cat "$TMPDIR/workflow-report-$phase-without-task.err" >&2
    exit 1
  fi
done
MAPPING_AFTER_MISSING_WORKFLOW_REPORT="$(shasum -a 256 "$TMPDIR/.pm/github-project-sync/tasks.json" | awk '{print $1}')"
if [[ "$MAPPING_BEFORE_MISSING_WORKFLOW_REPORT" != "$MAPPING_AFTER_MISSING_WORKFLOW_REPORT" ]]; then
  echo "github-project-task.test: missing task identity must fail before workflow-report mutates the task mapping" >&2
  exit 1
fi
python3 "$TMPDIR/github-project-task.py" workflow-report "$TMPDIR" \
  --repo eng-cc/oasis7 \
  --role tpm \
  --phase review \
  --json > "$TMPDIR/workflow-report-review-without-task.json"
python3 - "$TMPDIR/workflow-report-review-without-task.json" <<'PY'
import json
import sys

payload = json.load(open(sys.argv[1], encoding="utf-8"))
if payload != {"phase": "review", "role": "tpm", "status": "ok", "task_source": "github_project"}:
    raise SystemExit("workflow-report review without --task-uid must remain supported")
PY

python3 "$TMPDIR/github-project-task.py" workflow-report "$TMPDIR" \
  --repo eng-cc/oasis7 \
  --task-uid "$TASK_UID" \
  --role tpm \
  --phase start \
  --json > "$TMPDIR/start.json"

python3 "$TMPDIR/github-project-task.py" workflow-report "$TMPDIR" \
  --repo eng-cc/oasis7 \
  --task-uid "$TASK_UID" \
  --role tpm \
  --phase close \
  --json > "$TMPDIR/report-close.json"
python3 - "$TMPDIR/.pm/github-project-sync/tasks.json" "$TASK_UID" "$TMPDIR/report-close.json" <<'PY'
import json
import sys

mapping = json.load(open(sys.argv[1], encoding="utf-8"))
record = mapping["tasks"][sys.argv[2]]
payload = json.load(open(sys.argv[3], encoding="utf-8"))
if not payload.get("last_workflow_report_close_at"):
    raise SystemExit("workflow-report close must return last_workflow_report_close_at")
if record.get("last_workflow_report_close_at") != payload["last_workflow_report_close_at"]:
    raise SystemExit("workflow-report close must persist its separate report timestamp")
if record.get("last_closed_at") not in {None, ""}:
    raise SystemExit("workflow-report close must not write last_closed_at; task closeout owns it")
PY

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

CACHE_BEFORE_FAILURE="$(shasum -a 256 "$TMPDIR/.pm/github-project-sync/tasks.json" | awk '{print $1}')"
set +e
GH_FAIL_ISSUE_EDIT=1 PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/task-closeout.sh" \
  --role tpm --task-uid "$TASK_UID" --verification-profile fixture_repository_state --review-packet-file "$REVIEW_PACKET" --json \
  >"$TMPDIR/failed-closeout.json" 2>"$TMPDIR/failed-closeout.err"
FAILED_CLOSEOUT_STATUS=$?
set -e
[[ "$FAILED_CLOSEOUT_STATUS" != "0" ]]
CACHE_AFTER_FAILURE="$(shasum -a 256 "$TMPDIR/.pm/github-project-sync/tasks.json" | awk '{print $1}')"
[[ "$CACHE_BEFORE_FAILURE" == "$CACHE_AFTER_FAILURE" ]]
if [[ "$(cat "$GH_PROJECT_STATE_FILE")" != "ready" ]]; then
  echo "expected partial remote Project state to be ready before refresh" >&2
  cat "$TMPDIR/failed-closeout.err" >&2
  cat "$GH_CALL_LOG" >&2
  exit 1
fi
GRAPHQL_BEFORE="$(grep -c 'api graphql' "$GH_CALL_LOG" || true)"
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/refresh-task-cache.sh" \
  --task-uid "$TASK_UID" --json >"$TMPDIR/refreshed-after-partial.json"
GRAPHQL_AFTER="$(grep -c 'api graphql' "$GH_CALL_LOG" || true)"
[[ $((GRAPHQL_AFTER - GRAPHQL_BEFORE)) == 1 ]]
python3 - "$TMPDIR/.pm/github-project-sync/tasks.json" "$TASK_UID" <<'PY'
import json, sys
record=json.load(open(sys.argv[1],encoding="utf-8"))["tasks"][sys.argv[2]]
assert record["status"] == "ready", record
assert record["workflow_phase"] == "pre_pr_ready", record
assert record["project_status"] == "Ready / PR", record
assert record["reconciled_from_project"] is True, record
PY
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/github-project-workflow.sh" \
  --json audit --task-uid "$TASK_UID" >"$TMPDIR/audit-after-refresh.json"

set +e
GH_INTERRUPT_ISSUE_EDIT=1 PM_ROOT_DIR="$TMPDIR" /bin/bash -c \
  'export GH_INTERRUPT_TARGET=$$; exec "$@"' bash "$ROOT_DIR/scripts/pm/task-closeout.sh" \
  --role tpm --task-uid "$TASK_UID" --verification-profile fixture_repository_state --review-packet-file "$REVIEW_PACKET" --json \
  >"$TMPDIR/interrupted-closeout.json" 2>"$TMPDIR/interrupted-closeout.err"
INTERRUPTED_CLOSEOUT_STATUS=$?
set -e
[[ "$INTERRUPTED_CLOSEOUT_STATUS" != "0" ]]
CACHE_AFTER_INTERRUPT="$(shasum -a 256 "$TMPDIR/.pm/github-project-sync/tasks.json" | awk '{print $1}')"
[[ "$CACHE_BEFORE_FAILURE" == "$CACHE_AFTER_INTERRUPT" ]]

PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/task-closeout.sh" \
  --role tpm \
  --task-uid "$TASK_UID" \
  --verification-profile fixture_repository_state \
  --review-packet-file "$REVIEW_PACKET" \
  --json > "$TMPDIR/closeout.json"

python3 "$TMPDIR/github-project-task.py" record-pr "$TMPDIR" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --task-uid "$TASK_UID" \
  --pr-url "https://github.com/eng-cc/oasis7/pull/2002" \
  --json > "$TMPDIR/record-pr.json"

python3 - "$TMPDIR/.pm/github-project-sync/tasks.json" "$TASK_UID" "$GH_CALL_LOG" <<'PY'
import json, pathlib, sys
record=json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))["tasks"][sys.argv[2]]
calls=pathlib.Path(sys.argv[3]).read_text(encoding="utf-8")
assert record["status"] == "pr_watch", record
assert record["workflow_phase"] == "pr_watch", record
assert "OPT_PR_WATCH_PHASE" in calls, calls
PY

python3 - "$TMPDIR/.pm/github-project-sync/tasks.json" "$TASK_UID" <<'PY'
import json,sys
p=sys.argv[1]; m=json.load(open(p,encoding='utf-8')); r=m['tasks'][sys.argv[2]]
r['completion_mode']='non_pr_task'; r['non_pr_completion_evidence']='persisted fixture completion truth'
open(p,'w',encoding='utf-8').write(json.dumps(m)+'\n')
PY

PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/task-closeout.sh" \
  --role tpm \
  --task-uid "$TASK_UID" \
  --to-status done \
  --verification-profile fixture_repository_state \
  --claim-type task_complete \
  --json > "$TMPDIR/done-closeout.json"

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
assert record["status"] == "done", record
assert record["pr_url"] == "https://github.com/eng-cc/oasis7/pull/2002", record
assert record["pr_number"] == 2002, record
assert record["worktree_hint"] == str(pathlib.Path(sys.argv[1]).parents[2].resolve()), record
assert len(comments) >= 7, comments
assert record["claim_verifications"][-1]["claim_type"] == "task_complete", record
assert record["claim_verifications"][-1]["status"] == "verified", record
assert "issue create" in calls, calls
assert "issue edit 2001" in calls, calls
assert "issue close 2001" not in calls, calls
assert f"task_uid: {uid}" in edited_body, edited_body
assert "- status: `committed`" in edited_body, edited_body
assert "- status: `ready`" in edited_body, edited_body
assert "- status: `pr_watch`" in edited_body, edited_body
assert "- status: `done`" in edited_body, edited_body
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
assert mapping_path.exists(), "record-pr must recover the target mapping cache under lock"
mapping = json.loads(mapping_path.read_text(encoding="utf-8"))
assert mapping["tasks"]["task_99999999999999999999999999999999"]["status"] == "pr_watch", mapping
assert mapping["tasks"]["task_99999999999999999999999999999999"]["workflow_phase"] == "pr_watch", mapping
assert payload["status"] == "pr_watch", payload
assert payload["pr_number"] == 2003, payload
assert payload["updated_field_values"] == 0, payload
assert "- pr_url: `https://github.com/eng-cc/oasis7/pull/2003`" in edited_body, edited_body
assert "- pr_number: `2003`" in edited_body, edited_body
assert "- status: `pr_watch`" in edited_body, edited_body
PY

PARTIAL_ROOT="$TMPDIR/partial-cache"
PARTIAL_UID="task_44444444444444444444444444444444"
mkdir -p "$PARTIAL_ROOT/.pm/github-project-sync"
cat > "$PARTIAL_ROOT/.pm/github-project-sync/tasks.json" <<JSON
{
  "tasks": {
    "$PARTIAL_UID": {
      "task_uid": "$PARTIAL_UID",
      "title": "Partial cache done closeout",
      "owner_role": "tpm",
      "module": "engineering",
      "status": "pr_watch",
      "priority": "P2",
      "worktree_hint": "/tmp/partial-cache-worktree",
      "issue_url": "https://github.com/eng-cc/oasis7/issues/2004",
      "issue_number": 2004,
      "last_closed_at": "2026-07-01T12:00:00+08:00",
      "claim_verifications": [
        {
          "claim_type": "task_complete",
          "verify_command": "true",
          "verified_at": "2026-07-01T12:00:00+08:00",
          "verification_exit_code": 0,
          "status": "verified",
          "allowed_to_claim": true,
          "claim_message": "Fresh verification passed; the task can now be claimed complete."
        }
      ]
    }
  },
  "version": 1
}
JSON

python3 "$TMPDIR/github-project-task.py" move-task "$PARTIAL_ROOT" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --task-uid "$PARTIAL_UID" \
  --to-status done \
  --json > "$TMPDIR/partial-done.json"

python3 - "$PARTIAL_ROOT/.pm/github-project-sync/tasks.json" "$TMPDIR/partial-done.json" "$GH_CALL_LOG" "$TMPDIR/issue-body-edited.md" <<'PY'
from __future__ import annotations

import json
import pathlib
import sys

mapping = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
payload = json.loads(pathlib.Path(sys.argv[2]).read_text(encoding="utf-8"))
calls = pathlib.Path(sys.argv[3]).read_text(encoding="utf-8")
edited_body = pathlib.Path(sys.argv[4]).read_text(encoding="utf-8")
record = mapping["tasks"]["task_44444444444444444444444444444444"]
assert record["status"] == "done", record
assert record["project_item_id"] == "ITEM_ID_2004", record
assert payload["updated_field_values"] == 0, payload
assert "project item-list 1 --owner eng-cc --limit 1000 --format json" in calls, calls
assert "issue close 2004 -R eng-cc/oasis7 --reason completed" not in calls, calls
assert "- status: `done`" in edited_body, edited_body
PY

NOOP_PROJECT_ROOT="$TMPDIR/noop-project"
NOOP_UID="task_55555555555555555555555555555555"
mkdir -p "$NOOP_PROJECT_ROOT/.pm/github-project-sync"
cat > "$NOOP_PROJECT_ROOT/.pm/github-project-sync/tasks.json" <<JSON
{
  "tasks": {
    "$NOOP_UID": {
      "task_uid": "$NOOP_UID",
      "title": "No-op Project field update",
      "owner_role": "tpm",
      "module": "engineering",
      "status": "pr_watch",
      "priority": "P2",
      "worktree_hint": "/tmp/noop-project-worktree",
      "issue_url": "https://github.com/eng-cc/oasis7/issues/2005",
      "issue_number": 2005,
      "project_item_id": "ITEM_ID_2005",
      "last_closed_at": "2026-07-01T12:00:00+08:00",
      "claim_verifications": [
        {
          "claim_type": "task_complete",
          "verify_command": "true",
          "verified_at": "2026-07-01T12:00:00+08:00",
          "verification_exit_code": 0,
          "status": "verified",
          "allowed_to_claim": true,
          "claim_message": "Fresh verification passed; the task can now be claimed complete."
        }
      ]
    }
  },
  "version": 1
}
JSON

set +e
python3 "$TMPDIR/github-project-task.py" move-task "$NOOP_PROJECT_ROOT" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 2 \
  --task-uid "$NOOP_UID" \
  --to-status done \
  --json > "$TMPDIR/noop-project-done.json" 2>"$TMPDIR/noop-project-done.err"
NOOP_PROJECT_DONE_STATUS=$?
set -e
if [[ "$NOOP_PROJECT_DONE_STATUS" != "0" ]]; then
  echo "github-project-task.test: intermediate task_done must not require terminal Project fields" >&2
  cat "$TMPDIR/noop-project-done.err" >&2
  exit 1
fi
if grep -Fq "issue close 2005" "$GH_CALL_LOG" || ! grep -Fq "issue edit 2005" "$GH_CALL_LOG"; then
  echo "github-project-task.test: task_done must update but not close issue 2005" >&2
  cat "$GH_CALL_LOG" >&2
  exit 1
fi

MISSING_OPTION_ROOT="$TMPDIR/missing-option"
MISSING_OPTION_UID="task_66666666666666666666666666666666"
mkdir -p "$MISSING_OPTION_ROOT/.pm/github-project-sync"
cat > "$MISSING_OPTION_ROOT/.pm/github-project-sync/tasks.json" <<JSON
{
  "tasks": {
    "$MISSING_OPTION_UID": {
      "task_uid": "$MISSING_OPTION_UID",
      "title": "Missing Done option",
      "owner_role": "tpm",
      "module": "engineering",
      "status": "pr_watch",
      "priority": "P2",
      "worktree_hint": "/tmp/missing-option-worktree",
      "issue_url": "https://github.com/eng-cc/oasis7/issues/2006",
      "issue_number": 2006,
      "project_item_id": "ITEM_ID_2006",
      "last_closed_at": "2026-07-01T12:00:00+08:00",
      "claim_verifications": [
        {
          "claim_type": "task_complete",
          "verify_command": "true",
          "verified_at": "2026-07-01T12:00:00+08:00",
          "verification_exit_code": 0,
          "status": "verified",
          "allowed_to_claim": true,
          "claim_message": "Fresh verification passed; the task can now be claimed complete."
        }
      ]
    }
  },
  "version": 1
}
JSON

set +e
python3 "$TMPDIR/github-project-task.py" move-task "$MISSING_OPTION_ROOT" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 3 \
  --task-uid "$MISSING_OPTION_UID" \
  --to-status done \
  --json > "$TMPDIR/missing-option-done.json" 2>"$TMPDIR/missing-option-done.err"
MISSING_OPTION_DONE_STATUS=$?
set -e
if [[ "$MISSING_OPTION_DONE_STATUS" != "0" ]]; then
  echo "github-project-task.test: task_done must not require terminal Done options" >&2
  cat "$TMPDIR/missing-option-done.err" >&2
  exit 1
fi
if grep -Fq "issue close 2006" "$GH_CALL_LOG" || ! grep -Fq "issue edit 2006" "$GH_CALL_LOG"; then
  echo "github-project-task.test: task_done must update but not close issue 2006" >&2
  cat "$GH_CALL_LOG" >&2
  exit 1
fi
if grep -Fq "ITEM_ID_2006" "$GH_CALL_LOG"; then
  echo "github-project-task.test: missing Done option must not edit Project item 2006" >&2
  cat "$GH_CALL_LOG" >&2
  exit 1
fi

# `task_done` is an intermediate terminal workflow state. A Project whose live
# schema exposes only the coarse `done` option must not strand remedial closeout;
# fine terminal sequencing remains in the local mapping and receipts.
CLOSEOUT_CLAIM='{"claim_type":"task_complete","status":"verified","allowed_to_claim":true,"verification_exit_code":0,"verified_at":"2026-07-01T12:00:00Z"}'
if ! python3 "$TMPDIR/github-project-task.py" closeout-task "$MISSING_OPTION_ROOT" \
  --repo eng-cc/oasis7 --project-owner eng-cc --project-number 3 \
  --task-uid "$MISSING_OPTION_UID" --role tpm --to-status done \
  --claim-json "$CLOSEOUT_CLAIM" --json >"$TMPDIR/missing-option-closeout.json" 2>"$TMPDIR/missing-option-closeout.err"; then
  echo "github-project-task.test: remedial task_done closeout must map the coarse Project done option and advance" >&2
  cat "$TMPDIR/missing-option-closeout.err" >&2
  exit 1
fi
python3 - "$MISSING_OPTION_ROOT/.pm/github-project-sync/tasks.json" "$MISSING_OPTION_UID" "$TMPDIR/missing-option-closeout.json" <<'PY'
import json,sys
record=json.load(open(sys.argv[1],encoding="utf-8"))["tasks"][sys.argv[2]]
payload=json.load(open(sys.argv[3],encoding="utf-8"))
assert record["status"] == "done" and record["workflow_phase"] == "task_done", record
assert payload["updated_field_values"] == 3, payload
PY

CONCURRENT_ROOT="$TMPDIR/concurrent-cache"
mkdir -p "$CONCURRENT_ROOT"
printf '{"version":1,"tasks":{}}\n' >"$CONCURRENT_ROOT/tasks.json"
for uid in task_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa task_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb; do
  python3 - "$TMPDIR/github-project-task.py" "$CONCURRENT_ROOT/tasks.json" "$uid" <<'PY' &
import importlib.util, sys
spec=importlib.util.spec_from_file_location("task_impl",sys.argv[1])
module=importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
module.merge_task_mapping(module.pathlib.Path(sys.argv[2]), sys.argv[3], {"task_uid":sys.argv[3],"status":"ready"})
PY
done
wait
python3 - "$CONCURRENT_ROOT/tasks.json" <<'PY'
import json, sys
tasks=json.load(open(sys.argv[1],encoding="utf-8"))["tasks"]
assert set(tasks) == {"task_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","task_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}, tasks
PY

echo "github-project-task.test: OK"
