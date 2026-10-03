#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${PM_ROOT_DIR:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

TMPDIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

mkdir -p "$TMPDIR/.pm/tasks" "$TMPDIR/bin"
cp "$ROOT_DIR/scripts/pm/github-project-sync.py" "$TMPDIR/github-project-sync.py"
cp "$ROOT_DIR/scripts/pm/fixtures/github_api_test_adapter.py" "$TMPDIR/github_api.py"

cat > "$TMPDIR/.pm/tasks/task_11111111111111111111111111111111.yaml" <<'YAML'
task_uid: task_11111111111111111111111111111111
title: "sync active task"
owner_role: tpm
module: engineering
worktree_hint: /tmp/active
execution_log_path: .pm/tasks/task_11111111111111111111111111111111.execution.md
status: committed
priority: P2
source_signal: null
source_refs:
  - doc/engineering/workflow/source-of-truth.md
doc_refs: []
related_prd: []
acceptance:
  - mirrored issue exists
handoff_to: []
updated_at: 2026-06-29T00:00:00+08:00
YAML

cat > "$TMPDIR/.pm/tasks/task_22222222222222222222222222222222.yaml" <<'YAML'
task_uid: task_22222222222222222222222222222222
title: "skip done task"
owner_role: qa_engineer
module: visualization
worktree_hint: /tmp/done
execution_log_path: .pm/tasks/task_22222222222222222222222222222222.execution.md
status: done
priority: P3
source_signal: null
source_refs: []
doc_refs: []
related_prd: []
acceptance: []
handoff_to: []
updated_at: 2026-06-29T00:00:01+08:00
YAML

cat > "$TMPDIR/bin/gh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%q ' "$@" >> "$GH_CALL_LOG"
printf '\n' >> "$GH_CALL_LOG"
case "$*" in
  "project view 1 --owner eng-cc --format json")
    printf '{"id":"PROJECT_ID","number":1,"title":"oasis7 Engineering PM Mirror","url":"https://github.com/users/eng-cc/projects/1"}\n'
    ;;
  "project field-list 1 --owner eng-cc --format json")
    cat <<'JSON'
{"fields":[
{"id":"FIELD_STATUS","name":"Status","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_TODO","name":"Todo"},{"id":"OPT_IN_PROGRESS","name":"In Progress"},{"id":"OPT_BLOCKED_STATUS","name":"Blocked"},{"id":"OPT_READY","name":"Ready / PR"},{"id":"OPT_PR_WATCH","name":"PR Watch"},{"id":"OPT_DONE_STATUS","name":"Done"}]},
{"id":"FIELD_TASK_UID","name":"Task UID","type":"ProjectV2Field"},
{"id":"FIELD_OWNER_ROLE","name":"Owner Role","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_TPM","name":"tpm"},{"id":"OPT_QA","name":"qa_engineer"}]},
{"id":"FIELD_MODULE","name":"Module","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_ENGINEERING","name":"engineering"},{"id":"OPT_VISUALIZATION","name":"visualization"}]},
{"id":"FIELD_PM_STATUS","name":"PM Status","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_COMMITTED","name":"committed"},{"id":"OPT_READY_PM","name":"ready"},{"id":"OPT_PR_WATCH_PM","name":"pr_watch"},{"id":"OPT_DONE","name":"done"}]},
{"id":"FIELD_WORKFLOW_PHASE","name":"Workflow Phase","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_EXECUTION","name":"execution"},{"id":"OPT_PRE_PR_READY","name":"pre_pr_ready"},{"id":"OPT_PR_WATCH_PHASE","name":"pr_watch"},{"id":"OPT_DONE_PHASE","name":"done"}]},
{"id":"FIELD_PRIORITY","name":"Priority","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_P2","name":"P2"},{"id":"OPT_P3","name":"P3"}]},
{"id":"FIELD_BLOCKED","name":"Blocked Reason","type":"ProjectV2Field"},
{"id":"FIELD_WORKTREE","name":"Canonical Worktree","type":"ProjectV2Field"},
{"id":"FIELD_PR","name":"PR","type":"ProjectV2Field"},
{"id":"FIELD_TIER","name":"Test Tier Required","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_NA","name":"n/a"}]},
{"id":"FIELD_UPDATED","name":"Last PM Update","type":"ProjectV2Field"}]}
JSON
    ;;
  api\ graphql*)
    if [[ "${GH_FAKE_RECOVER_EXISTING:-0}" == "1" ]]; then
      printf '{"data":{"s0":{"nodes":[{"number":101,"body":"task_uid: task_11111111111111111111111111111111","url":"https://github.com/eng-cc/oasis7/issues/101","projectItems":{"nodes":[{"id":"WRONG_ITEM_ID","project":{"id":"OTHER_PROJECT_ID","number":1}},{"id":"ITEM_ID","project":{"id":"PROJECT_ID","number":1}}]}}]}}}\n'
    else
      printf '{"data":{"s0":{"nodes":[]}}}\n'
    fi
    ;;
  issue\ create*)
    printf 'https://github.com/eng-cc/oasis7/issues/101\n'
    ;;
  "project item-add 1 --owner eng-cc --url https://github.com/eng-cc/oasis7/issues/101 --format json")
    printf '{"id":"ITEM_ID","content":{"url":"https://github.com/eng-cc/oasis7/issues/101"}}\n'
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
: > "$GH_CALL_LOG"

python3 - "$TMPDIR/github-project-sync.py" <<'PY'
import importlib.util
import sys
from collections import OrderedDict

spec = importlib.util.spec_from_file_location("sync", sys.argv[1])
sync = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sync)
task = OrderedDict(task_uid="task_x", status="committed", owner_role="tpm", module="engineering", priority="P2")
current = {"PM Status": "blocked", "Owner Role": "tpm"}
confirmed = sync.confirmed_project_field_values(
    current,
    task,
    ["PM Status:missing_option:committed", "Owner Role:unchanged", "Last PM Update:deferred_date_field"],
)
assert confirmed["PM Status"] == "blocked", confirmed
assert confirmed["Owner Role"] == "tpm", confirmed
assert confirmed["Task UID"] == "task_x", confirmed
assert "Last PM Update" not in confirmed, confirmed
PY

python3 - "$TMPDIR/github-project-sync.py" <<'PY'
import importlib.util
import sys

spec = importlib.util.spec_from_file_location("sync", sys.argv[1])
sync = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sync)

response = {
        "node": {
            "project": {"id": "PROJECT_ID"},
            "fieldValues": {"nodes": [
                {"text": "task_11111111111111111111111111111111", "field": {"name": "Task UID"}},
                {"name": "In Progress", "field": {"name": "Status"}},
            ]},
        }
}
calls = []
class FakeClient:
    def graphql(self, query, variables=None, *, operation, mutation=False, context=None):
        calls.append((query, variables, operation, mutation, context))
        return response

fake_client = FakeClient()
sync.github_api_client = lambda token=None: fake_client
assert sync.github_api_client("TEST_TOKEN").__class__ is FakeClient
explicit = sync.github_api_module().GitHubAPIClient(token="TEST_TOKEN")
assert explicit.token == "TEST_TOKEN", explicit.token

values = sync.read_project_item_field_values("PROJECT_ID", "ITEM_ID")
assert values == {
    "Task UID": "task_11111111111111111111111111111111",
    "Status": "In Progress",
}, values
assert "$item: ID!" in calls[0][0], calls
assert "$project" not in calls[0][0], calls
assert "project { id }" in calls[0][0], calls
assert calls[0][1] == {"item": "ITEM_ID"}, calls
assert calls[0][2] == "project_sync_selected_item_readback", calls
assert calls[0][3] is False, calls

response["node"]["project"]["id"] = "OTHER_PROJECT_ID"
try:
    sync.read_project_item_field_values("PROJECT_ID", "ITEM_ID")
except RuntimeError as exc:
    assert str(exc) == "Project item belongs to a different Project", exc
else:
    raise AssertionError("wrong Project identity was accepted")
PY

python3 - "$ROOT_DIR/scripts/pm/github-project-sync.py" <<'PY'
import importlib.util
import sys

spec = importlib.util.spec_from_file_location("sync_live_project", sys.argv[1])
sync = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sync)
sync.github_token = lambda: "TEST_TOKEN"
membership_pages = [
    {"repository": {"issue": {
        "id": "ISSUE_NODE", "number": 101,
        "url": "https://github.com/eng-cc/oasis7/issues/101", "state": "OPEN",
        "body": "task_uid: task_11111111111111111111111111111111",
        "projectItems": {
            "nodes": [{"id": "OTHER_ITEM", "isArchived": False,
                       "project": {"id": "OTHER", "number": 1,
                                   "viewerCanUpdate": True, "owner": {"login": "eng-cc"}}}],
            "pageInfo": {"hasNextPage": True, "endCursor": "membership-next"},
        },
    }}},
    {"repository": {"issue": {
        "id": "ISSUE_NODE", "number": 101,
        "url": "https://github.com/eng-cc/oasis7/issues/101", "state": "OPEN",
        "body": "task_uid: task_11111111111111111111111111111111",
        "projectItems": {
            "nodes": [{"id": "ITEM_ID", "isArchived": False,
                       "project": {"id": "PROJECT_ID", "number": 1,
                                   "viewerCanUpdate": True, "owner": {"login": "eng-cc"}}}],
            "pageInfo": {"hasNextPage": False, "endCursor": None},
        },
    }}},
]
project_identity = {"id": "PROJECT_ID", "number": 1,
                    "viewerCanUpdate": True, "owner": {"login": "eng-cc"}}
field_pages = [
    {"node": {"id": "ITEM_ID", "isArchived": False, "project": project_identity,
              "fieldValues": {
                  "nodes": [{"name": "committed", "field": {"name": "PM Status"}}],
                  "pageInfo": {"hasNextPage": True, "endCursor": "fields-next"},
              }}},
    {"node": {"id": "ITEM_ID", "isArchived": False, "project": project_identity,
              "fieldValues": {
                  "nodes": [{"name": "execution", "field": {"name": "Workflow Phase"}}],
                  "pageInfo": {"hasNextPage": False, "endCursor": None},
              }}},
]
calls = []
def read_page(_token, query, variables=None, **_request_metadata):
    variables = variables or {}
    calls.append((query, variables))
    if "projectItems(first: 100" in query:
        return membership_pages.pop(0)
    return field_pages.pop(0)
sync.graphql_request = read_page
result = sync.read_live_issue_project_item("eng-cc/oasis7", 101, "PROJECT_ID", 1)
assert result["complete"] is True, result
assert result["issue"]["id"] == "ISSUE_NODE", result
assert result["project"] == {
    "id": "PROJECT_ID", "number": 1, "owner": "eng-cc", "viewer_can_update": True,
}, result
assert result["item"] == {
    "id": "ITEM_ID", "is_archived": False,
    "field_values": {"PM Status": "committed", "Workflow Phase": "execution"},
}, result
assert len(calls) == 4, calls
assert calls[0][1]["after"] is None and calls[1][1]["after"] == "membership-next", calls
assert calls[2][1]["after"] is None and calls[3][1]["after"] == "fields-next", calls

membership_pages[:] = [{"repository": {"issue": {
    "id": "ISSUE_NODE", "number": 101,
    "url": "https://github.com/eng-cc/oasis7/issues/101", "state": "OPEN",
    "body": "task_uid: task_11111111111111111111111111111111",
    "projectItems": {"nodes": [], "pageInfo": {"hasNextPage": True, "endCursor": None}},
}}}]
field_pages[:] = []
try:
    sync.read_live_issue_project_item("eng-cc/oasis7", 101, "PROJECT_ID", 1)
except RuntimeError as exc:
    assert "cursor is missing or repeated" in str(exc), exc
else:
    raise AssertionError("incomplete membership pagination was accepted")
PY

python3 - "$TMPDIR/github-project-sync.py" <<'PY'
import importlib.util
import sys
from collections import OrderedDict

spec = importlib.util.spec_from_file_location("sync", sys.argv[1])
sync = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sync)

task = OrderedDict(task_uid="task_11111111111111111111111111111111", status="committed",
                   owner_role="tpm", module="engineering", priority="P2",
                   worktree_hint="/tmp/active", updated_at="2026-06-29T00:00:00Z")
fields = {
    "Status": {"id": "STATUS", "options_by_name": {"In Progress": "STATUS_PROGRESS", "Todo": "STATUS_TODO"}},
    "Priority": {"id": "PRIORITY", "options_by_name": {"P2": "P2", "P3": "P3"}},
    "Blocked Reason": {"id": "BLOCKED_REASON"},
}
calls = []
class FakeClient:
    def graphql(self, query, variables=None, *, operation, mutation=False, context=None):
        calls.append((query, variables, operation, mutation, context))
        return {"updateProjectV2ItemFieldValue": {"projectV2Item": {"id": "ITEM_ID"}}}
    def rest(self, method, path, payload=None, *, operation, mutation=None, context=None):
        raise AssertionError("unexpected REST call")

client = FakeClient()
changed, skipped = sync.update_fields_direct(
    client, "PROJECT_ID", "ITEM_ID", task, fields,
    only_fields={"Status", "Priority"},
    current_values={"Status": "In Progress", "Priority": "P2"},
)
assert changed == 0 and set(skipped) == {"Status:unchanged", "Priority:unchanged"}, (changed, skipped)
assert calls == [], calls

changed, skipped = sync.update_fields_direct(
    client, "PROJECT_ID", "ITEM_ID", task, fields,
    only_fields={"Status", "Priority"},
    current_values={"Status": "Todo", "Priority": "P3"},
)
assert changed == 2 and skipped == [], (changed, skipped)
assert len(calls) == 1, calls
assert calls[0][2] == "project_sync_update_fields" and calls[0][3] is True, calls
assert "f0:" in calls[0][0] and "f1:" in calls[0][0], calls[0][0]

calls.clear()
changed, skipped = sync.update_fields_direct(
    client, "PROJECT_ID", "ITEM_ID", task, fields,
    only_fields={"Status"}, current_values={},
)
assert changed == 1 and skipped == [], (changed, skipped)
assert len(calls) == 1, calls

empty_task = OrderedDict(task, status="committed")
sync.edit_text_field = lambda *args: (_ for _ in ()).throw(AssertionError("empty field must not be written"))
changed, skipped = sync.update_fields(
    "PROJECT_ID", "ITEM_ID", empty_task, fields,
    only_fields={"Blocked Reason"}, current_values={"Blocked Reason": ""},
)
assert changed == 0 and skipped == ["Blocked Reason:unchanged"], (changed, skipped)
changed, skipped = sync.update_fields(
    "PROJECT_ID", "ITEM_ID", empty_task, fields,
    only_fields={"Blocked Reason"}, current_values={},
)
assert changed == 0 and skipped == ["Blocked Reason:empty_value"], (changed, skipped)
assert sync.confirmed_project_field_values(
    {}, empty_task, ["Blocked Reason:empty_value"], only_fields={"Blocked Reason"}
) == {}
print("github-project-sync.project-fields: OK")
PY

python3 - "$ROOT_DIR/scripts/pm/github-project-sync.py" "$ROOT_DIR/scripts/pm/github-project-task.py" <<'PY'
import contextlib
import importlib.util
import io
import pathlib
import sys

sync_path, task_path = map(pathlib.Path, sys.argv[1:])
sys.path.insert(0, str(sync_path.parent))
sync_spec = importlib.util.spec_from_file_location("sync_shared_client", sync_path)
sync = importlib.util.module_from_spec(sync_spec)
sync_spec.loader.exec_module(sync)

project = {"id": "PROJECT_ID", "number": 1, "owner": {"login": "eng-cc"},
           "viewerCanUpdate": True}
membership_pages = [
    {"repository": {"issue": {"id": "ISSUE_NODE", "number": 101,
        "url": "https://github.com/eng-cc/oasis7/issues/101", "state": "OPEN",
        "body": "task_uid: task_11111111111111111111111111111111",
        "projectItems": {"nodes": [{"id": "OTHER", "isArchived": False,
            "project": {**project, "id": "OTHER_PROJECT"}}],
            "pageInfo": {"hasNextPage": True, "endCursor": "membership-next"}}}}},
    {"repository": {"issue": {"id": "ISSUE_NODE", "number": 101,
        "url": "https://github.com/eng-cc/oasis7/issues/101", "state": "OPEN",
        "body": "task_uid: task_11111111111111111111111111111111",
        "projectItems": {"nodes": [{"id": "ITEM_ID", "isArchived": False,
            "project": project}], "pageInfo": {"hasNextPage": False, "endCursor": None}}}}},
]
field_pages = [
    {"node": {"id": "ITEM_ID", "isArchived": False, "project": project,
        "fieldValues": {"nodes": [{"name": "committed", "field": {"name": "PM Status"}}],
            "pageInfo": {"hasNextPage": True, "endCursor": "fields-next"}}}},
    {"node": {"id": "ITEM_ID", "isArchived": False, "project": project,
        "fieldValues": {"nodes": [{"name": "execution", "field": {"name": "Workflow Phase"}}],
            "pageInfo": {"hasNextPage": False, "endCursor": None}}}},
]

class PageClient:
    def __init__(self):
        self.calls = []
    def graphql(self, query, variables=None, *, operation, mutation=False, context=None):
        self.calls.append((query, variables, operation, mutation, context))
        if "projectItems(first: 100" in query:
            return membership_pages.pop(0)
        return field_pages.pop(0)
    def rest(self, *args, **kwargs):
        raise AssertionError("unexpected REST request")

client = PageClient()
sync.github_api_client = lambda token=None: client
result = sync.read_live_issue_project_item("eng-cc/oasis7", 101, "PROJECT_ID", 1)
assert result["complete"] is True, result
assert len(client.calls) == 4, client.calls
assert [call[1].get("after") for call in client.calls] == [None, "membership-next", None, "fields-next"]
assert all(call[3] is False for call in client.calls), client.calls
assert [call[2] for call in client.calls] == [
    "project_sync_live_issue_memberships", "project_sync_live_issue_memberships",
    "project_sync_live_item_fields", "project_sync_live_item_fields",
], client.calls
assert len({id(sync.github_api_client(token)) for token in (client, client, client, client)}) == 1

# The task helper must allow the typed shared-client failure to reach its
# process boundary instead of flattening external_wait into an ordinary die().
task_spec = importlib.util.spec_from_file_location("task_project_api_error", task_path)
task = importlib.util.module_from_spec(task_spec)
task_spec.loader.exec_module(task)
sync = task.load_sync_module()
task.load_sync_module = lambda: sync
api = sync.github_api_module()
rate_error = api.APIError("bounded external wait", kind="primary_rate_limit", retry_after_seconds=120)

class RateLimitedClient:
    def graphql(self, *args, **kwargs):
        raise rate_error
    def rest(self, *args, **kwargs):
        raise AssertionError("unexpected REST request")

sync.github_api_client = lambda token=None: RateLimitedClient()
sync.project_context = lambda owner, number: ("PROJECT_ID", {})
args = type("Args", (), {"project_owner": "eng-cc", "project_number": 1,
    "repo": "eng-cc/oasis7", "task_uid": "task_11111111111111111111111111111111"})()
record = {"task_uid": args.task_uid, "project_item_id": "ITEM_ID"}
try:
    task.validate_authoritative_project_fields(
        args, {"project": {"id": "PROJECT_ID"}}, record,
        {"issue_number": 101, "task_uid": args.task_uid, "status": "committed",
         "workflow_phase": "execution"},
    )
except api.APIError as exc:
    assert exc is rate_error and exc.exit_code == 75
else:
    raise AssertionError("task Project wrapper flattened typed external_wait")
print("github-project-sync.shared-client-and-api-error: OK")
PY

DRY_JSON="$TMPDIR/dry.json"
GH_FAKE_RECOVER_EXISTING=1 python3 "$TMPDIR/github-project-sync.py" "$TMPDIR" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --mapping "$TMPDIR/.pm/github-project-sync/tasks.json" \
  --task-uid task_11111111111111111111111111111111 \
  --dry-run \
  --json > "$DRY_JSON"

python3 - "$DRY_JSON" "$GH_CALL_LOG" <<'PY'
import json, pathlib, sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
calls = pathlib.Path(sys.argv[2]).read_text()
assert payload["selected_count"] == 1, payload
assert payload["dry_run"] is True, payload
assert payload["tasks"][0]["would_create_issue"] is False, payload
assert payload["tasks"][0]["would_add_item"] is False, payload
assert "api graphql" in calls, calls
assert "project item-list" not in calls, calls
assert "issue create" not in calls, calls
assert "project item-add" not in calls, calls
PY

APPLY_JSON="$TMPDIR/apply.json"
python3 "$TMPDIR/github-project-sync.py" "$TMPDIR" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --mapping "$TMPDIR/.pm/github-project-sync/tasks.json" \
  --task-uid task_11111111111111111111111111111111 \
  --apply \
  --json > "$APPLY_JSON"

python3 - "$APPLY_JSON" "$TMPDIR/.pm/github-project-sync/tasks.json" "$GH_CALL_LOG" <<'PY'
import json, pathlib, sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
mapping = json.loads(pathlib.Path(sys.argv[2]).read_text())
calls = pathlib.Path(sys.argv[3]).read_text()
uid = "task_11111111111111111111111111111111"
assert payload["created_issues"] == 1, payload
assert payload["added_items"] == 1, payload
assert payload["updated_field_values"] >= 7, payload
assert mapping["tasks"][uid]["issue_url"] == "https://github.com/eng-cc/oasis7/issues/101", mapping
assert mapping["tasks"][uid]["issue_number"] == 101, mapping
assert mapping["tasks"][uid]["worktree_hint"] == "/tmp/active", mapping
assert mapping["tasks"][uid]["execution_log_path"] == ".pm/tasks/task_11111111111111111111111111111111.execution.md", mapping
assert "issue create" in calls, calls
assert "project item-add" in calls, calls
assert "project item-edit" in calls, calls
PY

echo "github-project-sync.test: OK"
