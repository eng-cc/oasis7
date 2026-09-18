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
import base64, json, os, sys
m=json.load(open(sys.argv[1])); uid,next_record=next(iter(m["tasks"].items())); pm_status=next_record["status"]
def read_state(name, fallback):
    path=os.environ.get(name)
    if path and os.path.exists(path):
        return open(path).read().strip() or fallback
    return fallback
# Project lifecycle fields are independent: draft candidates change Workflow
# Phase to verification without changing PM Status=committed. A scalar fake
# state used to conflate the two and fail the selected-task audit too early.
pm_status=read_state("GH_PROJECT_STATE_FILE", pm_status)
status=read_state("GH_PROJECT_STATUS_STATE_FILE", {"committed":"In Progress","ready":"Ready / PR","pr_watch":"PR Watch","done":"In Progress"}.get(pm_status,"Todo"))
phase=read_state("GH_PROJECT_PHASE_STATE_FILE", next_record.get("workflow_phase") or {"committed":"execution","ready":"pre_pr_ready","pr_watch":"pr_watch","done":"done"}.get(pm_status,"execution"))
field_nodes=[{"name":status,"field":{"name":"Status"}},{"text":uid,"field":{"name":"Task UID"}},{"name":next_record["owner_role"],"field":{"name":"Owner Role"}},{"name":next_record["module"],"field":{"name":"Module"}},{"name":pm_status,"field":{"name":"PM Status"}},{"name":phase,"field":{"name":"Workflow Phase"}},{"name":next_record["priority"],"field":{"name":"Priority"}},{"text":next_record["worktree_hint"],"field":{"name":"Canonical Worktree"}},{"name":"n/a","field":{"name":"Test Tier Required"}}]
if next_record.get("pr_url"): field_nodes.append({"text":next_record["pr_url"],"field":{"name":"PR"}})
project_item={"id":next_record.get("project_item_id") or "ITEM_ID","project":{"id":"PROJECT_ID","number":1,"owner":{"login":"eng-cc"}},"fieldValues":{"pageInfo":{"hasNextPage":False},"nodes":field_nodes}}
issue={"number":next_record["issue_number"],"url":next_record["issue_url"],"body":f"task_uid: {uid}","projectItems":{"nodes":[project_item]}}
# Keep both GraphQL response shapes used by the bounded workflow commands:
# classify/refresh search uses the aliased s0 search result, while the selected
# audit fetches the bound Project item through the top-level nodes result.
trace_lines = [f"task_uid: {uid}", "Task metadata:"]
for key in ("workflow_phase", "completion_mode", "non_pr_completion_evidence_sha256"):
    value = next_record.get(key)
    if value:
        trace_lines.append(f"- {key}: `{value}`")
evidence = next_record.get("non_pr_completion_evidence")
if evidence is not None:
    encoded = base64.urlsafe_b64encode(str(evidence).encode("utf-8")).decode("ascii").rstrip("=")
    trace_lines.append(f"- non_pr_completion_evidence_b64: `{encoded}`")
trace_lines.append("Acceptance:")
project_item["content"]={"body":"\n".join(trace_lines)+"\n","number":next_record["issue_number"],"title":"[PM] "+next_record["title"],"url":next_record["issue_url"]}
print(json.dumps({"data":{"nodes":[project_item],"s0":{"nodes":[issue]}}}))
PY
    ;;
  "issue create -R eng-cc/oasis7 --title "*)
    printf 'https://github.com/eng-cc/oasis7/issues/2001\n'
    ;;
  issue\ list\ -R\ eng-cc/oasis7\ --state\ all\ --search\ task_*\ in:body\ --json\ number,url,title,state\ --limit\ 5)
    if [[ "$*" == *"task_99999999999999999999999999999999"* ]]; then
      printf '[{"number":2003,"state":"OPEN","title":"[PM] No-cache task","url":"https://github.com/eng-cc/oasis7/issues/2003"}]\n'
    else
      printf '[{"number":2001,"state":"OPEN","title":"[PM] GitHub-backed lifecycle smoke","url":"https://github.com/eng-cc/oasis7/issues/2001"}]\n'
    fi
    ;;
  issue\ list\ -R\ eng-cc/oasis7\ --search\ task_*\ in:body\ --json\ number,url,title,state\ --limit\ 5)
    if [[ "$*" == *"task_99999999999999999999999999999999"* ]]; then
      printf '[{"number":2003,"state":"OPEN","title":"[PM] No-cache task","url":"https://github.com/eng-cc/oasis7/issues/2003"}]\n'
    else
      printf '[{"number":2001,"state":"OPEN","title":"[PM] GitHub-backed lifecycle smoke","url":"https://github.com/eng-cc/oasis7/issues/2001"}]\n'
    fi
    ;;
  "issue view 2001 -R eng-cc/oasis7 --json body,number,title,url,state,stateReason")
    uid="$(python3 -c 'import json,os; print(next(iter(json.load(open(os.environ["GH_MAPPING_PATH"]))["tasks"])))')"
    printf '{"body":"task_uid: %s\\nTask metadata:\\n- owner_role: `tpm`\\n- module: `engineering`\\n- status: `candidate`\\n- workflow_phase: `bootstrap`\\n- priority: `P2`\\n- worktree_hint: `%s/worktree`\\nAcceptance:\\n","number":2001,"title":"[PM] GitHub-backed lifecycle smoke","url":"https://github.com/eng-cc/oasis7/issues/2001","state":"OPEN","stateReason":null}\n' "$uid" "$(dirname "$(dirname "$(dirname "$GH_MAPPING_PATH")")")"
    ;;
  "issue comment 2001 -R eng-cc/oasis7 --body-file "*)
    n=$(( $(wc -l < "$GH_COMMENT_LOG") + 1 ))
    mkdir -p "$GH_COMMENT_DIR"
    cat "${@: -1}" > "$GH_COMMENT_DIR/$n"
    printf 'comment-%s\n' "$n" >> "$GH_COMMENT_LOG"
    printf 'https://github.com/eng-cc/oasis7/issues/2001#issuecomment-%s\n' "$n"
    ;;
  api\ repos/eng-cc/oasis7/issues/comments/*)
    comment_id="${*: -1}"
    comment_id="${comment_id##*/}"
    python3 - "$GH_COMMENT_DIR/$comment_id" <<'PY'
import json, pathlib, sys
print(json.dumps({"body": pathlib.Path(sys.argv[1]).read_text()}))
PY
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
  "issue view 2003 -R eng-cc/oasis7 --json body,number,title,url,state,stateReason")
    cat <<'JSON'
{"body":"<!-- oasis7-pm-task -->\ntask_uid: task_99999999999999999999999999999999\n\nGitHub-backed oasis7 PM task.\n\nTask metadata:\n- owner_role: `tpm`\n- module: `engineering`\n- status: `ready`\n- workflow_phase: `pre_pr_ready`\n- priority: `P2`\n- worktree_hint: `/tmp/no-cache-worktree`\n","number":2003,"title":"[PM] No-cache task","url":"https://github.com/eng-cc/oasis7/issues/2003","state":"OPEN","stateReason":null}
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
{"id":"FIELD_PM_STATUS","name":"PM Status","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_CANDIDATE","name":"candidate"},{"id":"OPT_COMMITTED","name":"committed"},{"id":"OPT_READY_PM","name":"ready"},{"id":"OPT_PR_WATCH_PM","name":"pr_watch"},{"id":"OPT_DONE","name":"done"},{"id":"OPT_DEFERRED_PM","name":"deferred"}]},
{"id":"FIELD_WORKFLOW_PHASE","name":"Workflow Phase","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_EXECUTION","name":"execution"},{"id":"OPT_VERIFICATION","name":"verification"},{"id":"OPT_PRE_PR_READY","name":"pre_pr_ready"},{"id":"OPT_PR_WATCH_PHASE","name":"pr_watch"},{"id":"OPT_BLOCKED_PHASE","name":"blocked"},{"id":"OPT_DONE_PHASE","name":"done"}]},
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
    if [[ "$*" == *"--field-id FIELD_STATUS"* ]]; then
      case "$*" in
        *OPT_TODO*) printf 'Todo\n' >"$GH_PROJECT_STATUS_STATE_FILE" ;;
        *OPT_IN_PROGRESS*) printf 'In Progress\n' >"$GH_PROJECT_STATUS_STATE_FILE" ;;
        *OPT_READY*) printf 'Ready / PR\n' >"$GH_PROJECT_STATUS_STATE_FILE" ;;
        *OPT_PR_WATCH*) printf 'PR Watch\n' >"$GH_PROJECT_STATUS_STATE_FILE" ;;
        *OPT_DONE_STATUS*) printf 'Done\n' >"$GH_PROJECT_STATUS_STATE_FILE" ;;
      esac
    elif [[ "$*" == *"--field-id FIELD_PM_STATUS"* ]]; then
      if [[ "$*" == *"OPT_CANDIDATE"* ]]; then
        printf 'candidate\n' >"$GH_PROJECT_STATE_FILE"
      elif [[ "$*" == *"OPT_COMMITTED"* ]]; then
        printf 'committed\n' >"$GH_PROJECT_STATE_FILE"
      elif [[ "$*" == *"OPT_READY_PM"* ]]; then
        printf 'ready\n' >"$GH_PROJECT_STATE_FILE"
      elif [[ "$*" == *"OPT_PR_WATCH_PM"* ]]; then
        printf 'pr_watch\n' >"$GH_PROJECT_STATE_FILE"
      elif [[ "$*" == *"OPT_DONE"* ]]; then
        printf 'done\n' >"$GH_PROJECT_STATE_FILE"
      elif [[ "$*" == *"OPT_DEFERRED_PM"* ]]; then
        printf 'deferred\n' >"$GH_PROJECT_STATE_FILE"
      fi
    elif [[ "$*" == *"--field-id FIELD_WORKFLOW_PHASE"* ]]; then
      case "$*" in
        *OPT_EXECUTION*) printf 'execution\n' >"$GH_PROJECT_PHASE_STATE_FILE" ;;
        *OPT_VERIFICATION*) printf 'verification\n' >"$GH_PROJECT_PHASE_STATE_FILE" ;;
        *OPT_PRE_PR_READY*) printf 'pre_pr_ready\n' >"$GH_PROJECT_PHASE_STATE_FILE" ;;
        *OPT_PR_WATCH_PHASE*) printf 'pr_watch\n' >"$GH_PROJECT_PHASE_STATE_FILE" ;;
        *OPT_BLOCKED_PHASE*) printf 'blocked\n' >"$GH_PROJECT_PHASE_STATE_FILE" ;;
        *OPT_DONE_PHASE*) printf 'done\n' >"$GH_PROJECT_PHASE_STATE_FILE" ;;
      esac
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
printf '*.json\n*.log\n*.md\n*.err\n.pm/\nworktree/\ngh-comments/\nproject-live-state\nproject-live-status\nproject-live-phase\nxcrun_db\ntmp*\n' > "$TMPDIR/.gitignore"
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
export GH_PROJECT_STATUS_STATE_FILE="$TMPDIR/project-live-status"
printf 'Todo\n' >"$GH_PROJECT_STATUS_STATE_FILE"
export GH_PROJECT_PHASE_STATE_FILE="$TMPDIR/project-live-phase"
printf 'execution\n' >"$GH_PROJECT_PHASE_STATE_FILE"
export GH_COMMENT_LOG="$TMPDIR/gh-comments.log"
export GH_COMMENT_DIR="$TMPDIR/gh-comments"
export GH_EDIT_BODY_LOG="$TMPDIR/issue-body-edited.md"
export OASIS7_ALLOW_FIXTURE_VERIFICATION_PROFILE=1
: > "$GH_CALL_LOG"
: > "$GH_COMMENT_LOG"
: > "$GH_EDIT_BODY_LOG"
mkdir -p "$GH_COMMENT_DIR"

NEW_JSON="$TMPDIR/new.json"
python3 "$TMPDIR/github-project-task.py" new-task "$TMPDIR" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --owner-role tpm \
  --title "GitHub-backed lifecycle smoke" \
  --module engineering \
  --priority P2 \
  --primary-package oasis7 \
  --source-ref doc/engineering/workflow/source-of-truth.md \
  --worktree-hint "$TMPDIR/worktree" \
  --json > "$NEW_JSON"

TASK_UID="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["task_uid"])' "$NEW_JSON")"

# RED: public move-task must derive Workflow Phase from the new PM Status. A
# stale execution phase would otherwise publish the invalid Done/deferred/
# execution combination instead of the canonical Done/deferred/blocked pair.
MOVE_PHASE_ROOT="$TMPDIR/move-phase"
MOVE_PHASE_UID="task_77777777777777777777777777777777"
mkdir -p "$MOVE_PHASE_ROOT/.pm/github-project-sync"
python3 - "$MOVE_PHASE_ROOT/.pm/github-project-sync/tasks.json" "$MOVE_PHASE_UID" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
uid = sys.argv[2]
payload = {
    "version": 1,
    "tasks": {
        uid: {
            "task_uid": uid,
            "title": "Move-task phase mapping",
            "owner_role": "tpm",
            "module": "engineering",
            "status": "committed",
            "workflow_phase": "execution",
            "priority": "P2",
            "worktree_hint": "/tmp/move-phase-worktree",
            "issue_url": "https://github.com/eng-cc/oasis7/issues/2001",
            "issue_number": 2001,
            "project_item_id": "ITEM_ID",
        },
    },
}
path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
PY
printf 'committed\n' >"$GH_PROJECT_STATE_FILE"
printf 'In Progress\n' >"$GH_PROJECT_STATUS_STATE_FILE"
printf 'execution\n' >"$GH_PROJECT_PHASE_STATE_FILE"
set +e
python3 "$TMPDIR/github-project-task.py" move-task "$MOVE_PHASE_ROOT" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --task-uid "$MOVE_PHASE_UID" \
  --to-status deferred \
  --json >"$TMPDIR/move-phase.json" 2>"$TMPDIR/move-phase.err"
MOVE_PHASE_STATUS=$?
set -e
if [[ "$MOVE_PHASE_STATUS" != "0" ]]; then
  echo "github-project-task.test: move-task phase mapping fixture unexpectedly failed" >&2
  cat "$TMPDIR/move-phase.err" >&2
  exit 1
fi
python3 - "$MOVE_PHASE_ROOT/.pm/github-project-sync/tasks.json" "$MOVE_PHASE_UID" <<'PY'
import json
import sys

record = json.load(open(sys.argv[1], encoding="utf-8"))["tasks"][sys.argv[2]]
assert record["status"] == "deferred", record
assert record["workflow_phase"] == "blocked", record
PY
[[ "$(cat "$GH_PROJECT_STATUS_STATE_FILE")" == "Done" ]]
[[ "$(cat "$GH_PROJECT_STATE_FILE")" == "deferred" ]]
[[ "$(cat "$GH_PROJECT_PHASE_STATE_FILE")" == "blocked" ]]
printf 'candidate\n' >"$GH_PROJECT_STATE_FILE"
printf 'Todo\n' >"$GH_PROJECT_STATUS_STATE_FILE"
printf 'execution\n' >"$GH_PROJECT_PHASE_STATE_FILE"

# RED: non-PR classification must reject a live Project lifecycle that drifts
# from the authoritative Issue before editing the Issue or local cache.
CLASSIFY_MAPPING_BEFORE="$(shasum -a 256 "$TMPDIR/.pm/github-project-sync/tasks.json" | awk '{print $1}')"
CLASSIFY_CALLS_BEFORE="$(wc -l < "$GH_CALL_LOG")"
printf 'pr_watch\n' >"$GH_PROJECT_STATE_FILE"
printf 'PR Watch\n' >"$GH_PROJECT_STATUS_STATE_FILE"
printf 'pr_watch\n' >"$GH_PROJECT_PHASE_STATE_FILE"
set +e
python3 "$TMPDIR/github-project-task.py" classify-non-pr-task "$TMPDIR" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --task-uid "$TASK_UID" \
  --evidence "Read-only workflow audit completed without a PR." \
  --json > "$TMPDIR/classify-non-pr-drift.json" 2> "$TMPDIR/classify-non-pr-drift.err"
CLASSIFY_DRIFT_STATUS=$?
set -e
if [[ "$CLASSIFY_DRIFT_STATUS" == "0" ]]; then
  echo "github-project-task.test: expected Issue/Project lifecycle drift to fail closed" >&2
  exit 1
fi
grep -Eqi 'Project|drift' "$TMPDIR/classify-non-pr-drift.err"
CLASSIFY_MAPPING_AFTER="$(shasum -a 256 "$TMPDIR/.pm/github-project-sync/tasks.json" | awk '{print $1}')"
[[ "$CLASSIFY_MAPPING_BEFORE" == "$CLASSIFY_MAPPING_AFTER" ]]
CLASSIFY_NEW_CALLS="$TMPDIR/classify-non-pr-new-calls.log"
tail -n +$((CLASSIFY_CALLS_BEFORE + 1)) "$GH_CALL_LOG" >"$CLASSIFY_NEW_CALLS"
if grep -Eq 'issue (edit|comment)' "$CLASSIFY_NEW_CALLS"; then
  echo "github-project-task.test: Project drift must fail before Issue mutation" >&2
  cat "$CLASSIFY_NEW_CALLS" >&2
  exit 1
fi
printf 'candidate\n' >"$GH_PROJECT_STATE_FILE"
printf 'Todo\n' >"$GH_PROJECT_STATUS_STATE_FILE"
printf 'execution\n' >"$GH_PROJECT_PHASE_STATE_FILE"

# RED: the public non-PR classification command must exist and persist a
# runtime-verified issue-comment receipt plus refreshed task fields.
python3 "$TMPDIR/github-project-task.py" classify-non-pr-task "$TMPDIR" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --task-uid "$TASK_UID" \
  --evidence "Read-only workflow audit completed without a PR." \
  --json > "$TMPDIR/classify-non-pr.json"
python3 - "$TMPDIR/.pm/github-project-sync/tasks.json" "$TASK_UID" "$TMPDIR/classify-non-pr.json" <<'PY'
import json, pathlib, sys
mapping = json.loads(pathlib.Path(sys.argv[1]).read_text())
payload = json.loads(pathlib.Path(sys.argv[3]).read_text())
record = mapping["tasks"][sys.argv[2]]
assert record["completion_mode"] == "non_pr_task", record
assert record["non_pr_completion_evidence"] == "Read-only workflow audit completed without a PR.", record
assert pathlib.Path(record["non_pr_completion_evidence_file"]).read_text().strip() == record["non_pr_completion_evidence"], record
assert payload["status"] == "ok" and payload["comment_url"], payload
assert payload["comment_readback_verified"] is True, payload
PY
REVIEW_PACKET="$TMPDIR/review-packet.md"
HEAD_SHA="$(git -C "$TMPDIR" rev-parse HEAD)"
LEDGER_DIR="$TMPDIR/.pm/scratch/$TASK_UID"
mkdir -p "$LEDGER_DIR"
printf 'fixture return\n' >"$LEDGER_DIR/return.md"
RETURN_SHA="$(shasum -a 256 "$LEDGER_DIR/return.md" | awk '{print $1}')"
printf '{"receipt_type":"oasis7_subagent_dispatch","issuer":"codex_runtime","dispatch_id":"11111111-1111-4111-8111-111111111111","role":"repository_health_engineer","source_head":"%s","contract_digest":"%064d"}\n' "$HEAD_SHA" 0 >"$LEDGER_DIR/dispatch.json"
printf '{"task_uid":"%s","role":"repository_health_engineer","status":"completed","head":"%s","slice_id":"11111111-1111-4111-8111-111111111111","dispatch_receipt":".pm/scratch/%s/dispatch.json","activation":"message-assigned","context_delivery":"full-history","actual_runtime":"inherited/unverified: fixture","artifact_digest":"%s","scope_verdict":"approved","risk_verdict":"approved","findings":"no_findings","residual_risk":"fixture","artifacts":[".pm/scratch/%s/return.md"]}\n' "$TASK_UID" "$HEAD_SHA" "$TASK_UID" "$RETURN_SHA" "$TASK_UID" >"$LEDGER_DIR/slice-ledger.jsonl"
REVIEW_PLAN="$TMPDIR/.pm/scratch/$TASK_UID/review-plans/fixture.json"
mkdir -p "$(dirname "$REVIEW_PLAN")"
python3 - "$REVIEW_PLAN" "$HEAD_SHA" "$TASK_UID" "$LEDGER_DIR" <<'PY'
import json, sys
path, head, uid, ledger_dir = sys.argv[1:]
json.dump({
  "schema": "oasis7-review-plan/v1", "task_uid": uid, "frozen_head": head,
  "comparison_ref": "HEAD", "comparison_oid": head,
  "relevant_evidence_digest": "a" * 64,
  "roles": ["repository_health_engineer"],
  "expected_slices": [{"role": "repository_health_engineer", "slice_id": "11111111-1111-4111-8111-111111111111"}],
  "epoch": "b" * 64, "batch_path": ".pm/scratch/%s/review-batches/%s.json" % (uid, "b" * 64),
  "preflight": {"status": "incomplete", "ledger_path": ".pm/scratch/%s/slice-ledger.jsonl" % uid},
}, open(path, "w"))
PY
# Deliberately retain a stale compatibility role in the packet: closeout must
# derive the authoritative role and ledger from Review Plan.
printf -- "- Pre-PR Local Role Review: passed\n- Source Head: %s\n- Review Plan: .pm/scratch/%s/review-plans/fixture.json\n- Review Roles: stale_compatibility_role\n- Slice Ledger: n/a; derived from Review Plan\n" "$HEAD_SHA" "$TASK_UID" >"$REVIEW_PACKET"

python3 "$TMPDIR/github-project-task.py" move-task "$TMPDIR" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --task-uid "$TASK_UID" \
  --to-status committed \
  --json > "$TMPDIR/move-committed.json"

python3 "$TMPDIR/github-project-task.py" record-pr "$TMPDIR" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --task-uid "$TASK_UID" \
  --pr-url "https://github.com/eng-cc/oasis7/pull/2001" \
  --draft-candidate \
  --json > "$TMPDIR/record-draft.json"
python3 - "$TMPDIR/.pm/github-project-sync/tasks.json" "$TASK_UID" "$GH_CALL_LOG" <<'PY'
import json, pathlib, sys
record = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))["tasks"][sys.argv[2]]
calls = pathlib.Path(sys.argv[3]).read_text(encoding="utf-8")
if record.get("status") != "committed" or record.get("workflow_phase") != "verification":
    raise SystemExit(f"draft candidate did not persist committed/verification truth: {record}")
if "OPT_VERIFICATION" not in calls:
    raise SystemExit("draft candidate did not project Workflow Phase=verification")
PY

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
# Refresh intentionally reconciles the partial remote Project state and rewrites
# the local mapping. Bind the SIGTERM immutability check to that new baseline,
# not to the pre-refresh cache captured for the failed closeout.
CACHE_BEFORE_INTERRUPT="$(shasum -a 256 "$TMPDIR/.pm/github-project-sync/tasks.json" | awk '{print $1}')"

set +e
GH_INTERRUPT_ISSUE_EDIT=1 PM_ROOT_DIR="$TMPDIR" /bin/bash -c \
  'export GH_INTERRUPT_TARGET=$$; exec "$@"' bash "$ROOT_DIR/scripts/pm/task-closeout.sh" \
  --role tpm --task-uid "$TASK_UID" --verification-profile fixture_repository_state --review-packet-file "$REVIEW_PACKET" --json \
  >"$TMPDIR/interrupted-closeout.json" 2>"$TMPDIR/interrupted-closeout.err"
INTERRUPTED_CLOSEOUT_STATUS=$?
set -e
[[ "$INTERRUPTED_CLOSEOUT_STATUS" != "0" ]]
CACHE_AFTER_INTERRUPT="$(shasum -a 256 "$TMPDIR/.pm/github-project-sync/tasks.json" | awk '{print $1}')"
if [[ "$CACHE_BEFORE_INTERRUPT" != "$CACHE_AFTER_INTERRUPT" ]]; then
  echo "github-project-task.test: interrupted closeout changed mapping" >&2
  exit 1
fi

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
import hashlib, json, pathlib, sys
p=sys.argv[1]; m=json.load(open(p,encoding='utf-8')); r=m['tasks'][sys.argv[2]]
r['completion_mode']='non_pr_task'; r['non_pr_completion_evidence']='persisted fixture completion truth'
r['non_pr_completion_evidence_sha256'] = hashlib.sha256(
    (r['non_pr_completion_evidence'] + '\n').encode('utf-8')
).hexdigest()
pathlib.Path(r['non_pr_completion_evidence_file']).write_text(
    r['non_pr_completion_evidence'] + '\n', encoding='utf-8'
)
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
set +e
python3 "$TMPDIR/github-project-task.py" move-task "$NO_CACHE_ROOT" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --task-uid "$NO_CACHE_UID" \
  --to-status ready \
  --json > "$TMPDIR/no-cache-move.json" 2> "$TMPDIR/no-cache-move.err"
NO_CACHE_MOVE_STATUS=$?
set -e
[[ "$NO_CACHE_MOVE_STATUS" != "0" ]]
grep -Fq "canonical task-closeout.sh" "$TMPDIR/no-cache-move.err"

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

# Cross-layer traceability fields must survive the Issue serializer/parser and
# remain section-scoped. The Project projection is intentionally coarse; the
# Issue/cache retain the finer terminal and evidence authority.
python3 - "$TMPDIR/github-project-task.py" <<'PY'
import hashlib
import importlib.util
import json
import pathlib
import sys
import tempfile
from argparse import Namespace

spec = importlib.util.spec_from_file_location("task_impl", sys.argv[1])
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
uid = "task_88888888888888888888888888888888"
evidence = "Read-only workflow audit completed without a PR."
record = {
    "title": "Traceability round-trip",
    "owner_role": "repository_health_engineer",
    "module": "engineering",
    "status": "done",
    "workflow_phase": "task_done",
    "priority": "P2",
    "completion_mode": "non_pr_task",
    "non_pr_completion_evidence": evidence,
    "non_pr_completion_evidence_sha256": hashlib.sha256((evidence + "\n").encode()).hexdigest(),
    "source_refs": ["doc/engineering/workflow/source-of-truth.md#traceability-record-contract"],
    "doc_refs": [
        "doc/engineering/doc-governance/project-management-record-standard.design.md#3",
        "doc/engineering/doc-governance/system-design-writing-standard.design.md#11",
    ],
    "related_prd": ["doc/product/oasis7.prd.md#REQ-TRACE"],
    "acceptance": ["Project done does not erase task_done or evidence."],
}
task = module.task_from_record(uid, record)
body = module.issue_body(task)
assert "Doc refs:\n- `doc/engineering/doc-governance/project-management-record-standard.design.md#3`\n- `doc/engineering/doc-governance/system-design-writing-standard.design.md#11`" in body, body
assert "Related PRD:\n- `doc/product/oasis7.prd.md#REQ-TRACE`" in body, body
assert f"- non_pr_completion_evidence_sha256: `{record['non_pr_completion_evidence_sha256']}`" in body, body
parsed = module.issue_task_fields(body)
assert parsed["source_refs"] == record["source_refs"], parsed
assert parsed["doc_refs"] == record["doc_refs"], parsed
assert parsed["related_prd"] == record["related_prd"], parsed
assert parsed["acceptance"] == record["acceptance"], parsed
assert parsed["completion_mode"] == "non_pr_task", parsed
assert parsed["non_pr_completion_evidence"] == evidence, parsed
assert parsed["non_pr_completion_evidence_sha256"] == record["non_pr_completion_evidence_sha256"], parsed

section_body = """Source refs:
- `source-only`

Doc refs:
- `doc-only`

Related PRD:
- `prd-only`

Acceptance:
- accepted-only
"""
section_fields = module.issue_task_fields(section_body)
assert section_fields["source_refs"] == ["source-only"], section_fields
assert section_fields["doc_refs"] == ["doc-only"], section_fields
assert section_fields["related_prd"] == ["prd-only"], section_fields
assert section_fields["acceptance"] == ["accepted-only"], section_fields

# A terminal non-PR cache entry with a missing identity-bound evidence file is
# lossy. Refresh must stop with the stable diagnostic rather than silently
# clearing completion mode/evidence authority.
with tempfile.TemporaryDirectory() as temp:
    root = pathlib.Path(temp)
    cache = root / ".pm/github-project-sync/tasks.json"
    cache.parent.mkdir(parents=True)
    worktree = root / "task-worktree"
    worktree.mkdir()
    evidence_path = worktree / ".pm/scratch" / uid / "non-pr-completion-evidence.txt"
    evidence_path.parent.mkdir(parents=True)
    evidence_path.write_text(evidence + "\n", encoding="utf-8")
    cached = {
        "task_uid": uid,
        "title": record["title"],
        "owner_role": record["owner_role"],
        "module": record["module"],
        "status": "done",
        "workflow_phase": "task_done",
        "completion_mode": "non_pr_task",
        "non_pr_completion_evidence": evidence,
        "non_pr_completion_evidence_file": str(evidence_path),
        "non_pr_completion_evidence_sha256": record["non_pr_completion_evidence_sha256"],
        "doc_refs": record["doc_refs"],
        "related_prd": record["related_prd"],
        "worktree_hint": str(worktree),
        "repository": "eng-cc/oasis7",
        "canonical_worktree": str(worktree),
        "task_branch": "task/traceability-round-trip",
        "default_branch": "main",
        "issue_number": 808,
        "issue_url": "https://github.com/eng-cc/oasis7/issues/808",
        "project_item_id": "TRACE_ITEM",
    }
    cache.write_text(json.dumps({"version": 1, "project": {"id": "PROJECT_ID", "number": 1, "owner": "eng-cc", "repo": "eng-cc/oasis7"}, "tasks": {uid: cached}}) + "\n", encoding="utf-8")
    issue = dict(module.issue_task_fields(module.issue_body(module.task_from_record(uid, record))))
    issue.update({"task_uid": uid, "title": record["title"], "issue_number": 808, "issue_url": cached["issue_url"], "issue_state": "OPEN"})
    project_node = {"id": "TRACE_ITEM", "project": {"id": "PROJECT_ID", "number": 1, "owner": {"login": "eng-cc"}}, "fieldValues": {"pageInfo": {"hasNextPage": False}, "nodes": [
        {"name": "Done", "field": {"name": "Status"}},
        {"name": "done", "field": {"name": "PM Status"}},
        {"name": "done", "field": {"name": "Workflow Phase"}},
    ]}}
    module.github_issue_record = lambda _repo, _uid: issue
    module.project_refresh_graphql = lambda _query, _variables: {"data": {"nodes": [project_node]}}
    module.authoritative_repository_identity = lambda _root, _repo, _hint: {
        "repository": "eng-cc/oasis7", "canonical_worktree": str(worktree),
        "task_branch": "task/traceability-round-trip", "default_branch": "main",
    }
    module.pending_non_merge_phase = lambda *_args: None
    module.loop_lineage_path = lambda *_args: root / "missing-lineage.json"
    args = Namespace(root=root, mapping=str(cache), task_uid=uid, repo="eng-cc/oasis7", project_owner="eng-cc", project_number=1, json=True)

    # Positive Issue -> coarse Project done -> cache refresh round-trip. The
    # Issue's fine terminal phase and all traceability/evidence fields must be
    # reconstructed without loss.
    module.command_refresh_task(args)
    refreshed = json.loads(cache.read_text(encoding="utf-8"))["tasks"][uid]
    assert refreshed["status"] == "done", refreshed
    assert refreshed["workflow_phase"] == "task_done", refreshed
    assert refreshed["completion_mode"] == "non_pr_task", refreshed
    assert refreshed["non_pr_completion_evidence"] == evidence, refreshed
    assert refreshed["non_pr_completion_evidence_sha256"] == record["non_pr_completion_evidence_sha256"], refreshed
    assert refreshed["non_pr_completion_evidence_file"] == str(evidence_path.resolve()), refreshed
    assert refreshed["doc_refs"] == record["doc_refs"], refreshed
    assert refreshed["related_prd"] == record["related_prd"], refreshed

    # I-1: an explicit non-terminal live Issue phase must not fall back to a
    # stale fine-terminal cache phase when Project exposes coarse `done`.
    captured = []
    def fail(message):
        captured.append(message)
        raise SystemExit(1)
    module.die = fail
    issue["workflow_phase"] = "execution"
    try:
        module.command_refresh_task(args)
    except SystemExit:
        pass
    else:
        raise AssertionError("stale terminal cache phase unexpectedly survived live Issue phase")
    assert captured and captured[0].startswith("trace-projection-loss:"), captured

    # I-1: even when the cache loses its phase key, the live Issue phase remains
    # the fine terminal authority and cannot be replaced by Project `done`.
    issue["workflow_phase"] = "task_done"
    cache_payload = json.loads(cache.read_text(encoding="utf-8"))
    cache_payload["tasks"][uid].pop("workflow_phase", None)
    cache.write_text(json.dumps(cache_payload) + "\n", encoding="utf-8")
    module.command_refresh_task(args)
    refreshed = json.loads(cache.read_text(encoding="utf-8"))["tasks"][uid]
    assert refreshed["workflow_phase"] == "task_done", refreshed

    # A coarse Project `done` without a fine terminal Issue or cache phase is
    # ambiguous and must fail closed instead of manufacturing `done`.
    captured.clear()
    issue["workflow_phase"] = "execution"
    cache_payload = json.loads(cache.read_text(encoding="utf-8"))
    cache_payload["tasks"][uid].pop("workflow_phase", None)
    cache.write_text(json.dumps(cache_payload) + "\n", encoding="utf-8")
    try:
        module.command_refresh_task(args)
    except SystemExit:
        pass
    else:
        raise AssertionError("ambiguous terminal refresh unexpectedly succeeded")
    assert captured and captured[0].startswith("trace-projection-loss:"), captured

    # I-3: parser failures retain context, and refresh surfaces the same
    # stable projection-loss diagnostic even before evidence reconstruction.
    for encoded in ("!!!", "//4"):
        parsed_loss = module.issue_task_fields(
            f"- non_pr_completion_evidence_b64: `{encoded}`\n"
        )
        assert parsed_loss.get("trace_projection_error") == (
            "malformed non-PR completion evidence encoding"
        ), parsed_loss
    cache_payload = json.loads(cache.read_text(encoding="utf-8"))
    cache_record = cache_payload["tasks"][uid]
    cache_record.update({"status": "committed", "workflow_phase": "execution"})
    for key in ("non_pr_completion_evidence", "non_pr_completion_evidence_file", "non_pr_completion_evidence_sha256"):
        cache_record.pop(key, None)
    cache.write_text(json.dumps(cache_payload) + "\n", encoding="utf-8")
    issue.update({"status": "committed", "workflow_phase": "execution"})
    for key in ("non_pr_completion_evidence", "non_pr_completion_evidence_sha256"):
        issue.pop(key, None)
    issue["trace_projection_error"] = "malformed non-PR completion evidence encoding"
    project_node["fieldValues"]["nodes"][1]["name"] = "committed"
    project_node["fieldValues"]["nodes"][2]["name"] = "execution"
    captured.clear()
    try:
        module.command_refresh_task(args)
    except SystemExit:
        pass
    else:
        raise AssertionError("malformed evidence refresh unexpectedly succeeded")
    assert captured and captured[0].startswith("trace-projection-loss:"), captured

    captured.clear()
    issue.pop("trace_projection_error")
    issue.update({"status": "done", "workflow_phase": "task_done",
                  "non_pr_completion_evidence": evidence,
                  "non_pr_completion_evidence_sha256": record["non_pr_completion_evidence_sha256"]})
    project_node["fieldValues"]["nodes"][1]["name"] = "done"
    project_node["fieldValues"]["nodes"][2]["name"] = "done"
    cache_payload = json.loads(cache.read_text(encoding="utf-8"))
    cache_record = cache_payload["tasks"][uid]
    cache_record.update({"status": cached["status"], "workflow_phase": cached["workflow_phase"],
                         "non_pr_completion_evidence": cached["non_pr_completion_evidence"],
                         "non_pr_completion_evidence_file": cached["non_pr_completion_evidence_file"],
                         "non_pr_completion_evidence_sha256": cached["non_pr_completion_evidence_sha256"]})
    cache.write_text(json.dumps(cache_payload) + "\n", encoding="utf-8")
    evidence_path.unlink()
    try:
        module.command_refresh_task(args)
    except SystemExit:
        pass
    else:
        raise AssertionError("lossy terminal refresh unexpectedly succeeded")
    assert captured and captured[0].startswith("trace-projection-loss:"), captured
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
