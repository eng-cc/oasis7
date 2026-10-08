#!/usr/bin/env bash
set -euo pipefail
export OASIS7_TEST_ALLOW_UNATTESTED_DISPATCH_RECEIPTS=1

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${PM_ROOT_DIR:-$(cd "$SCRIPT_DIR/../.." && pwd)}"
if [[ "${OASIS7_REC_RED_ONLY:-0}" == "1" ]]; then
  export PYTHONPATH="$ROOT_DIR/scripts/pm${PYTHONPATH:+:$PYTHONPATH}"
fi

TMPDIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

mkdir -p "$TMPDIR/.pm/github-project-sync" "$TMPDIR/bin"
cp "$ROOT_DIR/scripts/pm/github-project-task.py" "$TMPDIR/github-project-task.py"
cp "$ROOT_DIR/scripts/pm/task_primary_package.py" "$TMPDIR/task_primary_package.py"
cp "$ROOT_DIR/scripts/pm/closed_duplicate_candidate_guard.py" "$TMPDIR/closed_duplicate_candidate_guard.py"
cp "$ROOT_DIR/scripts/pm/task_complete_claim.py" "$TMPDIR/task_complete_claim.py"
cp "$ROOT_DIR/scripts/pm/loop_leaf_result.py" "$TMPDIR/loop_leaf_result.py"
cp "$ROOT_DIR/scripts/pm/github-project-sync.py" "$TMPDIR/github-project-sync.py"
cp "$ROOT_DIR/scripts/pm/workflow-durable-store.py" "$TMPDIR/workflow-durable-store.py"
cp "$ROOT_DIR/scripts/pm/loop_leaf_result.py" "$TMPDIR/loop_leaf_result.py"
cp "$ROOT_DIR/scripts/pm/closed_duplicate_candidate_guard.py" "$TMPDIR/closed_duplicate_candidate_guard.py"
cp "$ROOT_DIR/scripts/pm/retire-closed-duplicate-candidate.py" "$TMPDIR/retire-closed-duplicate-candidate.py"
cp "$ROOT_DIR/scripts/pm/fixtures/github_api_test_adapter.py" "$TMPDIR/github_api.py"
cp "$ROOT_DIR/scripts/pm/portable_file_lock.py" "$TMPDIR/portable_file_lock.py"
cp "$ROOT_DIR/scripts/pm/claim-ready.sh" "$TMPDIR/claim-ready.sh"
cp "$ROOT_DIR/scripts/pm/loop_leaf_result.py" "$ROOT_DIR/scripts/pm/workflow-durable-store.py" \
  "$ROOT_DIR/scripts/pm/closed_duplicate_candidate_guard.py" "$TMPDIR/"
export PYTHONPATH="$TMPDIR${PYTHONPATH:+:$PYTHONPATH}"
mkdir -p "$TMPDIR/scripts"
cp -R "$ROOT_DIR/scripts/pm" "$TMPDIR/scripts/pm"
cp "$ROOT_DIR/scripts/pm/fixtures/github_api_test_adapter.py" "$TMPDIR/scripts/pm/github_api.py"
if [[ "${OASIS7_REC_RED_ONLY:-0}" == "1" ]]; then
  mkdir -p "$TMPDIR/.agents/roles" "$TMPDIR/doc/engineering/workflow"
  python3 - "$TMPDIR/scripts/pm" <<'PY_INNER'
import pathlib, shutil, sys
for path in pathlib.Path(sys.argv[1]).rglob("__pycache__"):
    shutil.rmtree(path)
PY_INNER
  cp "$ROOT_DIR/AGENTS.md" "$TMPDIR/AGENTS.md"
  cp "$ROOT_DIR/doc/engineering/workflow/source-of-truth.md" "$TMPDIR/doc/engineering/workflow/source-of-truth.md"
  cp "$ROOT_DIR/.agents/roles/"{runtime_engineer,repository_health_engineer,qa_engineer}.md "$TMPDIR/.agents/roles/"
fi

cat > "$TMPDIR/bin/gh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%q ' "$@" >> "$GH_CALL_LOG"
printf '\n' >> "$GH_CALL_LOG"
case "$*" in
  "api user")
    if [[ "${OASIS7_REC_CASE:-}" == "guard_c1_writer_scope_drift" \
          || "${OASIS7_REC_CASE:-}" == "guard_c1_writer_unknown_journal_drift" \
          || "${OASIS7_REC_CASE:-}" == "guard_c1_writer_reordered_journal_drift" \
          || "${OASIS7_REC_CASE:-}" == "guard_c1_writer_step_without_vector" \
          || "${OASIS7_REC_CASE:-}" == "guard_c1_writer_unreachable_step_suffix" \
          || "${OASIS7_REC_CASE:-}" == "guard_c1_writer_binding_comment_without_predecessors" \
          || "${OASIS7_REC_CASE:-}" == "guard_c1_writer_observed_vector_live_before" \
          || "${OASIS7_REC_CASE:-}" == "guard_c1_writer_uncertain_step_with_observation" ]]; then
      reads=0
      [[ ! -f "$GH_REC_AUTH_STATE_FILE.api-user-reads" ]] || reads="$(cat "$GH_REC_AUTH_STATE_FILE.api-user-reads")"
      reads=$((reads + 1))
      printf '%s\n' "$reads" >"$GH_REC_AUTH_STATE_FILE.api-user-reads"
      if [[ "${OASIS7_REC_CASE:-}" == "guard_c1_writer_reordered_journal_drift" ]]; then
        python3 - "$GH_REC_AUTH_STATE_FILE" <<'PY'
import json, pathlib, sys
state=pathlib.Path(sys.argv[1]); root=state.parent
saved=root / "current-journal-after-drift.json"
if not saved.exists():
    journal_path=pathlib.Path((root / "current-journal-path.md").read_text())
    journal=json.loads(journal_path.read_text())
    binding=json.loads((root / "recovery-binding.json").read_text())
    publication_id=binding["publication_id"]
    parent_id="record-pr:" + publication_id
    vector_id="record-pr-vector:" + publication_id
    positions={action.get("action_id"):index for index,action in enumerate(journal["actions"])
        if isinstance(action,dict) and action.get("action_id") in {parent_id,vector_id}}
    if parent_id in positions and vector_id in positions:
        left,right=positions[parent_id],positions[vector_id]
        journal["actions"][left],journal["actions"][right]=journal["actions"][right],journal["actions"][left]
        raw=(json.dumps(journal,sort_keys=True,separators=(",",":"))+"\n").encode()
        journal_path.write_bytes(raw)
        saved.write_bytes(raw)
PY
      elif [[ "$reads" == "3" ]]; then
        if [[ "${OASIS7_REC_CASE:-}" == "guard_c1_writer_scope_drift" ]]; then
          python3 - "$GH_COMMENT_DIR/7103" <<'PY'
import pathlib, sys
path=pathlib.Path(sys.argv[1]); original=path.read_text()
changed=original.replace(
    "scripts/pm/github-project-task.py; scripts/pm/pr_projection_publish.py",
    "scripts/pm/github-project-task.py; scripts/pm/pr_projection_publish.py; scripts/pm/unapproved-helper.py",
)
assert changed != original, "C1 writer scope drift fixture missed its approved helper-path row"
path.write_text(changed)
PY
        elif [[ "${OASIS7_REC_CASE:-}" == "guard_c1_writer_step_without_vector" ]]; then
          python3 - "$GH_REC_AUTH_STATE_FILE" <<'PY'
import json, pathlib, sys
state=pathlib.Path(sys.argv[1]); root=state.parent
journal_path=pathlib.Path((root / "current-journal-path.md").read_text())
journal=json.loads(journal_path.read_text())
binding=json.loads((root / "recovery-binding.json").read_text())
publication_id=binding["publication_id"]
parent_id="record-pr:" + publication_id
vector_id="record-pr-vector:" + publication_id
assert any(action.get("action_id") == parent_id for action in journal["actions"])
assert not any(action.get("action_id") == vector_id for action in journal["actions"])
journal["actions"].append({
    "action_id":"record-pr-step:" + publication_id + ":issue",
    "kind":"record_pr_transition_step",
    "expected":{"injected":"step without its required vector"},
    "state":"intent",
})
raw=(json.dumps(journal, sort_keys=True, separators=(",", ":")) + "\n").encode()
journal_path.write_bytes(raw)
(root / "current-journal-after-drift.json").write_bytes(raw)
PY
        elif [[ "${OASIS7_REC_CASE:-}" == "guard_c1_writer_unreachable_step_suffix" ]]; then
          python3 - "$GH_REC_AUTH_STATE_FILE" <<'PY'
import json, pathlib, sys
state=pathlib.Path(sys.argv[1]); root=state.parent
journal_path=pathlib.Path((root / "current-journal-path.md").read_text())
journal=json.loads(journal_path.read_text())
publication=json.loads((root / "recovery-publication.json").read_text())
binding=json.loads((root / "recovery-binding.json").read_text())
publication_id=publication["publication_id"]
task_uid=publication["task_uid"]
pr_number=binding["pr_number"]
pr_url=binding["pr_url"]
issue_before={"status":"committed","workflow_phase":"execution","pr_number":None,"pr_url":None}
issue_target={"status":"committed","workflow_phase":"verification","pr_number":pr_number,"pr_url":pr_url}
project_before={"task_uid":task_uid,"status":"In Progress","pm_status":"committed","workflow_phase":"execution","pr":""}
project_target={**project_before,"workflow_phase":"verification","pr":pr_url}
from pr_projection_transition import ISSUE_FIRST
from pr_projection_record_pr import _next_expected
sequence=ISSUE_FIRST
vector_expected={"publication_id":publication_id,"task_uid":task_uid,
    "pr_number":pr_number,"pr_url":pr_url,
    "source_head_oid":publication["source_head_oid"],
    "source_scope_oid":publication["source_scope_oid"],
    "projection_digest":publication["projection_digest"],"sequence":sequence}
step="project:PR"
step_expected={"publication_id":publication_id,"task_uid":task_uid,
    "pr_number":pr_number,"pr_url":pr_url,
    "source_head_oid":publication["source_head_oid"],
    "source_scope_oid":publication["source_scope_oid"],
    "projection_digest":publication["projection_digest"],"sequence":sequence,
    **_next_expected(sequence,step,issue_before,issue_target,project_before,project_target)}
assert any(action.get("action_id") == "record-pr:" + publication_id for action in journal["actions"])
assert not any(action.get("action_id") == "record-pr-vector:" + publication_id for action in journal["actions"])
journal["actions"].extend([
    {"action_id":"record-pr-vector:" + publication_id,"kind":"record_pr_vector",
     "expected":vector_expected,"state":"intent"},
    {"action_id":"record-pr-step:" + publication_id + ":" + step,
     "kind":"record_pr_transition_step","expected":step_expected,"state":"intent"},
])
raw=(json.dumps(journal,sort_keys=True,separators=(",", ":"))+"\n").encode()
journal_path.write_bytes(raw)
(root / "current-journal-after-drift.json").write_bytes(raw)
PY
        elif [[ "${OASIS7_REC_CASE:-}" == "guard_c1_writer_observed_vector_live_before" ]]; then
          python3 - "$GH_REC_AUTH_STATE_FILE" <<'PY'
import json, pathlib, sys
state=pathlib.Path(sys.argv[1]); root=state.parent
journal_path=pathlib.Path((root / "current-journal-path.md").read_text())
journal=json.loads(journal_path.read_bytes())
publication=json.loads((root / "recovery-publication.json").read_text())
binding=json.loads((root / "recovery-binding.json").read_text())
publication_id=publication["publication_id"]
task_uid=publication["task_uid"]
pr_number=binding["pr_number"]
pr_url=binding["pr_url"]
issue_before={"status":"committed","workflow_phase":"execution","pr_number":None,"pr_url":None}
issue_target={"status":"committed","workflow_phase":"verification","pr_number":pr_number,"pr_url":pr_url}
project_before={"task_uid":task_uid,"status":"In Progress","pm_status":"committed","workflow_phase":"execution","pr":""}
project_target={**project_before,"workflow_phase":"verification","pr":pr_url}
from pr_projection_transition import ISSUE_FIRST
from pr_projection_record_pr import _next_expected
sequence=ISSUE_FIRST
vector_expected={"publication_id":publication_id,"task_uid":task_uid,
    "pr_number":pr_number,"pr_url":pr_url,
    "source_head_oid":publication["source_head_oid"],
    "source_scope_oid":publication["source_scope_oid"],
    "projection_digest":publication["projection_digest"],"sequence":sequence}
assert any(action.get("action_id") == "record-pr:" + publication_id for action in journal["actions"])
assert not any(action.get("action_id") == "record-pr-vector:" + publication_id for action in journal["actions"])
rows=[{"action_id":"record-pr-vector:" + publication_id,
       "kind":"record_pr_vector","expected":vector_expected,"state":"observed",
       "observed":{"sequence":sequence,"task":issue_target,"project":project_target}}]
for step in ("issue","project:Workflow Phase","project:PR"):
    step_expected={"publication_id":publication_id,"task_uid":task_uid,
        "pr_number":pr_number,"pr_url":pr_url,
        "source_head_oid":publication["source_head_oid"],
        "source_scope_oid":publication["source_scope_oid"],
        "projection_digest":publication["projection_digest"],"sequence":sequence,
        **_next_expected(sequence,step,issue_before,issue_target,project_before,project_target)}
    if step == "issue":
        readback=issue_target
    else:
        field=step.removeprefix("project:")
        key="workflow_phase" if field == "Workflow Phase" else "pr"
        readback={"task_uid":task_uid,"field":field,"value":project_target[key]}
    rows.append({"action_id":"record-pr-step:" + publication_id + ":" + step,
        "kind":"record_pr_transition_step","expected":step_expected,"state":"observed",
        "observed":{"step":step,"readback":readback}})
journal["actions"].extend(rows)
raw=(json.dumps(journal,sort_keys=True,separators=(",",":"))+"\n").encode()
journal_path.write_bytes(raw)
(root / "current-journal-after-drift.json").write_bytes(raw)
PY
        elif [[ "${OASIS7_REC_CASE:-}" == "guard_c1_writer_binding_comment_without_predecessors" ]]; then
          python3 - "$GH_REC_AUTH_STATE_FILE" <<'PY'
import json, pathlib, sys
state=pathlib.Path(sys.argv[1]); root=state.parent
journal_path=pathlib.Path((root / "current-journal-path.md").read_text())
journal=json.loads(journal_path.read_text())
publication=json.loads((root / "recovery-publication.json").read_text())
binding=json.loads((root / "recovery-binding.json").read_text())
publication_id=publication["publication_id"]
task_uid=publication["task_uid"]
import pr_projection_publication as publication_module
journal["actions"].append({
    "action_id":"record-pr-comment:" + publication_id + ":publication-binding",
    "kind":"record_pr_comment",
    "expected":{"publication_id":publication_id,"task_uid":task_uid,
        "issue_number":2001,"body":publication_module.publication_binding_comment(binding)},
    "state":"intent",
})
raw=(json.dumps(journal,sort_keys=True,separators=(",", ":"))+"\n").encode()
journal_path.write_bytes(raw)
(root / "current-journal-after-drift.json").write_bytes(raw)
PY
        elif [[ "${OASIS7_REC_CASE:-}" == "guard_c1_writer_uncertain_step_with_observation" ]]; then
          python3 - "$GH_REC_AUTH_STATE_FILE" <<'PY'
import json, pathlib, sys
state=pathlib.Path(sys.argv[1]); root=state.parent
journal_path=pathlib.Path((root / "current-journal-path.md").read_text())
journal=json.loads(journal_path.read_text())
publication=json.loads((root / "recovery-publication.json").read_text())
binding=json.loads((root / "recovery-binding.json").read_text())
publication_id=publication["publication_id"]
task_uid=publication["task_uid"]
pr_number=binding["pr_number"]
pr_url=binding["pr_url"]
issue_before={"status":"committed","workflow_phase":"execution","pr_number":None,"pr_url":None}
issue_target={"status":"committed","workflow_phase":"verification","pr_number":pr_number,"pr_url":pr_url}
project_before={"task_uid":task_uid,"status":"In Progress","pm_status":"committed","workflow_phase":"execution","pr":""}
project_target={**project_before,"workflow_phase":"verification","pr":pr_url}
from pr_projection_transition import ISSUE_FIRST
from pr_projection_record_pr import _next_expected
sequence=ISSUE_FIRST
vector_expected={"publication_id":publication_id,"task_uid":task_uid,
    "pr_number":pr_number,"pr_url":pr_url,
    "source_head_oid":publication["source_head_oid"],
    "source_scope_oid":publication["source_scope_oid"],
    "projection_digest":publication["projection_digest"],"sequence":sequence}
step="issue"
step_expected={"publication_id":publication_id,"task_uid":task_uid,
    "pr_number":pr_number,"pr_url":pr_url,
    "source_head_oid":publication["source_head_oid"],
    "source_scope_oid":publication["source_scope_oid"],
    "projection_digest":publication["projection_digest"],"sequence":sequence,
    **_next_expected(sequence,step,issue_before,issue_target,project_before,project_target)}
journal["actions"].extend([
    {"action_id":"record-pr-vector:" + publication_id,"kind":"record_pr_vector",
     "expected":vector_expected,"state":"intent"},
    {"action_id":"record-pr-step:" + publication_id + ":issue",
     "kind":"record_pr_transition_step","expected":step_expected,
     "state":"uncertain","observed":{"step":"issue","readback":issue_target}},
])
raw=(json.dumps(journal,sort_keys=True,separators=(",", ":"))+"\n").encode()
journal_path.write_bytes(raw)
(root / "current-journal-after-drift.json").write_bytes(raw)
PY
        else
          python3 - "$GH_REC_AUTH_STATE_FILE" <<'PY'
import json, pathlib, sys
state=pathlib.Path(sys.argv[1])
root=state.parent
journal_path=pathlib.Path((root / "current-journal-path.md").read_text())
journal=json.loads(journal_path.read_text())
journal["actions"].append({"action_id":"unauthorized-after-admission",
    "kind":"unapproved_fixture_action", "expected":{"source":"test"}, "state":"intent"})
raw=(json.dumps(journal, sort_keys=True, separators=(",", ":")) + "\n").encode()
journal_path.write_bytes(raw)
(root / "current-journal-after-drift.json").write_bytes(raw)
PY
        fi
      fi
    fi
    python3 - "$GH_REC_AUTH_STATE_FILE" <<'PY'
import json, sys
state=json.load(open(sys.argv[1])); print(json.dumps({"login":state["actor"],"id":7}))
PY
    ;;
  "api repos/eng-cc/oasis7")
    python3 - "$GH_REC_AUTH_STATE_FILE" "$GH_PR_BASE_BRANCH" <<'PY'
import json, sys
state=json.load(open(sys.argv[1])); print(json.dumps({"id":7,"full_name":"eng-cc/oasis7",
    "default_branch":sys.argv[2],"permissions":{"push":state["repo_push"]}}))
PY
    ;;
  "api repos/eng-cc/oasis7/collaborators/eng-cc/permission")
    printf '{"permission":"admin"}\n'
    ;;
  api\ graphql*)
    python3 - "$GH_MAPPING_PATH" "$*" <<'PY'
import base64, json, os, sys
m=json.load(open(sys.argv[1])); uid,next_record=next(iter(m["tasks"].items())); pm_status=next_record["status"]
if (os.environ.get("OASIS7_REC_CASE") == "guard_project_item_content_late_drift"
        and "content" in sys.argv[2]):
    from pathlib import Path
    auth_path=os.environ.get("GH_REC_AUTH_STATE_FILE")
    content_path=os.environ.get("GH_REC_PROJECT_ITEM_CONTENT_FILE")
    if auth_path and content_path and Path(content_path + ".drifted").is_file():
        root=Path(auth_path).parent
        journal_path=Path((root / "current-journal-path.md").read_text())
        snapshot=root / "current-journal-drift-boundary.json"
        if not snapshot.exists():
            snapshot.write_bytes(journal_path.read_bytes())
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
project_pr=read_state("GH_PROJECT_PR_STATE_FILE", next_record.get("pr_url") or "")
if project_pr: field_nodes.append({"text":project_pr,"field":{"name":"PR"}})
auth_path=os.environ.get("GH_REC_AUTH_STATE_FILE")
if auth_path and os.path.exists(auth_path):
    from pathlib import Path
    identity_path=Path(auth_path).parent / "repository-field.json"
    identity=json.loads(identity_path.read_text()) if identity_path.exists() else {"id":"R_fixture_oasis7","nameWithOwner":"eng-cc/oasis7"}
    # Real GraphQL returns an empty object when its union member is not selected.
    field_nodes.append({"__typename":"ProjectV2ItemFieldRepositoryValue","field":{"name":"Repository"},"repository":identity}
        if "ProjectV2ItemFieldRepositoryValue" in sys.argv[2] else {})
project_item={"id":next_record.get("project_item_id") or "ITEM_ID","isArchived":False,"project":{"id":"PROJECT_ID","number":1,"owner":{"login":"eng-cc"}},"fieldValues":{"pageInfo":{"hasNextPage":False,"endCursor":None},"nodes":field_nodes}}
issue={"id":"ISSUE_ID","number":next_record["issue_number"],"url":next_record["issue_url"],"body":f"task_uid: {uid}","projectItems":{"pageInfo":{"hasNextPage":False,"endCursor":None},"nodes":[project_item]}}
issue_body_path=os.environ.get("GH_ISSUE_BODY_STATE_FILE")
if issue_body_path and os.path.exists(issue_body_path):
    issue["body"]=open(issue_body_path).read()
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
live_issue_body_path=os.environ.get("GH_ISSUE_BODY_STATE_FILE")
live_issue_body=open(live_issue_body_path).read() if live_issue_body_path and os.path.exists(live_issue_body_path) else ""
project_content={"__typename":"Issue","number":next_record["issue_number"],
    "title":"[PM] "+next_record["title"],"url":next_record["issue_url"],
    "body":issue["body"],"repository":{"nameWithOwner":"eng-cc/oasis7"}}
content_path=os.environ.get("GH_REC_PROJECT_ITEM_CONTENT_FILE")
if content_path and os.path.exists(content_path):
    project_content=json.load(open(content_path))
# Match GraphQL's selected-field behavior: content is present only if requested.
if "content" in sys.argv[2]:
    if project_content is not None:
        project_item["content"]=project_content
data={"nodes":[project_item],"s0":{"nodes":[issue]}}
if os.environ.get("GH_REC_AUTH_STATE_FILE"):
    auth=json.load(open(os.environ["GH_REC_AUTH_STATE_FILE"]))
    live_body=open(os.environ["GH_ISSUE_BODY_STATE_FILE"]).read()
    issue.update(id="ISSUE_ID",state=auth["issue_state"].upper(),body=live_body,
                 viewerCanUpdate=auth["issue_can_update"])
    project={"id":"PROJECT_ID","number":1,"title":"oasis7 Engineering PM",
             "owner":{"login":"eng-cc"},"viewerCanUpdate":auth["project_can_update"],
             "items":{"pageInfo":{"hasNextPage":False},"nodes":[project_item]}}
    project_item["project"].update(viewerCanUpdate=auth["project_can_update"])
    issue["projectItems"]["nodes"][0]["project"].update(viewerCanUpdate=auth["project_can_update"])
    data.update(viewer={"login":auth["actor"]},node=project,project=project,
                repository={"issue":issue,"projectV2":project},
                organization={"projectV2":project},user={"projectV2":project})
    if "node(id:" in sys.argv[2]:
        data["node"] = project_item
print(json.dumps({"data":data}))
PY
    ;;
  "api repos/eng-cc/oasis7/issues/2001/comments --paginate --slurp"|"api repos/eng-cc/oasis7/issues/2001/comments?per_page=100 --paginate --slurp")
    python3 - "$GH_COMMENT_DIR" "${OASIS7_REC_CASE:-}" <<'PY'
import json, pathlib, sys
directory = pathlib.Path(sys.argv[1])
case = sys.argv[2]
import os
scope_drift_armed = (case != "guard_scope_comment_drift" or pathlib.Path(
    os.environ.get("GH_REC_SCOPE_DRIFT_ARMED_FILE", "/no-such-oasis7-scope-arm")).is_file())
if case in {"guard_scope_comment_drift", "guard_c1_timestamp_recovery_drift", "guard_recovery_context_drift"} and scope_drift_armed:
    counter = directory.parent / "gh-comment-read-count.txt"
    reads = int(counter.read_text()) if counter.exists() else 0
    reads += 1
    counter.write_text(str(reads))
    if case == "guard_scope_comment_drift" and reads == 2:
        scope_comment = directory / "7103"
        original = scope_comment.read_text()
        changed = original.replace(
            "scripts/pm/github-project-task.py; scripts/pm/pr_projection_publish.py",
            "scripts/pm/github-project-task.py; scripts/pm/pr_projection_publish.py; scripts/pm/unapproved-helper.py",
        )
        assert changed != original, "scope drift fixture did not find its approved helper-path row"
        scope_comment.write_text(changed)
    if case == "guard_recovery_context_drift" and reads == 5:
        scope_comment = directory / "7103"
        original = scope_comment.read_text()
        changed = original.replace(
            "scripts/pm/github-project-task.py; scripts/pm/pr_projection_publish.py",
            "scripts/pm/github-project-task.py; scripts/pm/pr_projection_publish.py; scripts/pm/unapproved-helper.py",
        )
        assert changed != original, "post-admission scope drift fixture did not find its approved helper-path row"
        scope_comment.write_text(changed)
    if case == "guard_c1_timestamp_recovery_drift" and reads == 2:
        (directory.parent / "c1-server-timestamp.txt").write_text("2026-10-01T00:00:01Z")
comments = []
timestamp_path = directory.parent / "c1-server-timestamp.txt"
c1_timestamp = timestamp_path.read_text().strip() if timestamp_path.exists() else "2026-10-01T00:00:00Z"
for path in sorted(directory.iterdir(), key=lambda item: int(item.name) if item.name.isdecimal() else -1):
    if not path.name.isdecimal():
        continue
    body = path.read_text()
    comment_id = int(path.name)
    metadata_path = directory / f"comment-{comment_id}.json"
    if metadata_path.is_file():
        item = json.loads(metadata_path.read_text(encoding="utf-8"))
    else:
        item = {
            "id": comment_id,
            "user": {"login": "eng-cc", "type": "User"},
            "issue_url": "https://api.github.com/repos/eng-cc/oasis7/issues/2001",
            "html_url": f"https://github.com/eng-cc/oasis7/issues/2001#issuecomment-{comment_id}",
        }
    item["body"] = body
    item.setdefault("author_association", "OWNER")
    if "<!-- oasis7-ci-publication/v1 -->" in body:
        item["created_at"] = c1_timestamp
        item["updated_at"] = c1_timestamp
    comments.append(item)
print(json.dumps([comments]))
PY
    ;;
  "api repos/eng-cc/oasis7/issues/2001")
    if [[ "${GH_REC_FAIL_FINAL_ISSUE:-0}" == "1" ]] && grep -Fq -- '- pr_number: `2001`' "$GH_ISSUE_BODY_STATE_FILE"; then
      echo 'injected final authoritative Issue readback failure' >&2
      exit 78
    fi
    python3 - "$GH_ISSUE_BODY_STATE_FILE" <<'PY'
import json, os, pathlib, sys
auth=json.load(open(os.environ["GH_REC_AUTH_STATE_FILE"])) if os.environ.get("GH_REC_AUTH_STATE_FILE") else {}
print(json.dumps({"id":7,"node_id":"ISSUE_ID","number":2001,"state":auth.get("issue_state","open"),
    "html_url":"https://github.com/eng-cc/oasis7/issues/2001","body":pathlib.Path(sys.argv[1]).read_text(),
    "user":{"login":"eng-cc","type":"User"}}))
PY
    ;;
  api\ repos/eng-cc/oasis7/pulls/2001)
    reads=0
    [[ ! -f "$GH_PR_READ_COUNT_FILE" ]] || reads="$(cat "$GH_PR_READ_COUNT_FILE")"
    reads=$((reads + 1))
    printf '%s\n' "$reads" >"$GH_PR_READ_COUNT_FILE"
    draft=false
    if [[ "${OASIS7_REC_RED_ONLY:-0}" == "1" ]]; then draft=true; fi
    [[ "$reads" != "1" ]] || draft=true
    python3 - "$GH_PR_HEAD_SHA" "$GH_PR_TASK_BRANCH" "$GH_PR_BASE_BRANCH" "$draft" <<'PY'
import json, os, sys
sha, task_branch, base_branch, draft = sys.argv[1:]
auth=json.load(open(os.environ["GH_REC_AUTH_STATE_FILE"])) if os.environ.get("GH_REC_AUTH_STATE_FILE") else {}
print(json.dumps({
    "number": 2001,
    "html_url": "https://github.com/eng-cc/oasis7/pull/2001",
    "state": auth.get("pr_state","open"),
    "merged_at": "2026-10-01T00:00:00Z" if auth.get("pr_merged") else None,
    "created_at": "2026-10-01T00:00:02Z",
    "updated_at": "2026-10-01T00:00:03Z",
    "draft": draft == "true",
    "user": {"login":"eng-cc","type":"User"},
    "head": {"repo": {"full_name": "eng-cc/oasis7"}, "ref": task_branch, "sha": sha},
    "base": {"repo": {"full_name": "eng-cc/oasis7"}, "ref": base_branch},
    "body": __import__("pathlib").Path(__import__("os").environ["GH_REC_PR_BODY_FILE"]).read_text()
        if __import__("os").environ.get("GH_REC_PR_BODY_FILE") else "",
}))
PY
    ;;
  "issue create -R eng-cc/oasis7 --title "*)
    cp "${@: -1}" "$GH_ISSUE_BODY_STATE_FILE"
    printf 'https://github.com/eng-cc/oasis7/issues/2001\n'
    ;;
  issue\ list\ -R\ eng-cc/oasis7\ --state\ all\ --search\ task_*\ in:body\ --json\ number,url,title,state\ --limit\ 5)
    if [[ "$*" == *"task_99999999999999999999999999999999"* ]]; then
      printf '[{"number":2003,"state":"OPEN","title":"[PM] No-cache task","url":"https://github.com/eng-cc/oasis7/issues/2003"}]\n'
    elif [[ "$*" == *"task_44444444444444444444444444444444"* ]]; then
      printf '[{"number":2004,"state":"OPEN","title":"[PM] Partial cache done closeout","url":"https://github.com/eng-cc/oasis7/issues/2004"}]\n'
    elif [[ "$*" == *"task_55555555555555555555555555555555"* ]]; then
      printf '[{"number":2005,"state":"OPEN","title":"[PM] No-op Project field update","url":"https://github.com/eng-cc/oasis7/issues/2005"}]\n'
    elif [[ "$*" == *"task_66666666666666666666666666666666"* ]]; then
      printf '[{"number":2006,"state":"OPEN","title":"[PM] Missing Done option","url":"https://github.com/eng-cc/oasis7/issues/2006"}]\n'
    else
      printf '[{"number":2001,"state":"OPEN","title":"[PM] GitHub-backed lifecycle smoke","url":"https://github.com/eng-cc/oasis7/issues/2001"}]\n'
    fi
    ;;
  issue\ list\ -R\ eng-cc/oasis7\ --search\ task_*\ in:body\ --json\ number,url,title,state\ --limit\ 5)
    if [[ "$*" == *"task_99999999999999999999999999999999"* ]]; then
      printf '[{"number":2003,"state":"OPEN","title":"[PM] No-cache task","url":"https://github.com/eng-cc/oasis7/issues/2003"}]\n'
    elif [[ "$*" == *"task_44444444444444444444444444444444"* ]]; then
      printf '[{"number":2004,"state":"OPEN","title":"[PM] Partial cache done closeout","url":"https://github.com/eng-cc/oasis7/issues/2004"}]\n'
    elif [[ "$*" == *"task_55555555555555555555555555555555"* ]]; then
      printf '[{"number":2005,"state":"OPEN","title":"[PM] No-op Project field update","url":"https://github.com/eng-cc/oasis7/issues/2005"}]\n'
    elif [[ "$*" == *"task_66666666666666666666666666666666"* ]]; then
      printf '[{"number":2006,"state":"OPEN","title":"[PM] Missing Done option","url":"https://github.com/eng-cc/oasis7/issues/2006"}]\n'
    else
      printf '[{"number":2001,"state":"OPEN","title":"[PM] GitHub-backed lifecycle smoke","url":"https://github.com/eng-cc/oasis7/issues/2001"}]\n'
    fi
    ;;
  issue\ view\ 2001\ -R\ eng-cc/oasis7\ --json\ body,number,title,url,state,stateReason*)
    python3 - "$GH_ISSUE_BODY_STATE_FILE" "$GH_ISSUE_UPDATED_AT_FILE" <<PY
import json, os, pathlib, sys
body = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")
updated_at = pathlib.Path(sys.argv[2]).read_text(encoding="utf-8").strip()
auth = (json.loads(pathlib.Path(os.environ["GH_REC_AUTH_STATE_FILE"]).read_text(encoding="utf-8"))
        if os.environ.get("GH_REC_AUTH_STATE_FILE") else {})
print(json.dumps({
    "body": body,
    "number": 2001,
    "title": "[PM] GitHub-backed lifecycle smoke",
    "url": "https://github.com/eng-cc/oasis7/issues/2001",
    "state": auth.get("issue_state","OPEN").upper(),
    "stateReason": None,
    "updatedAt": updated_at,
}))
PY
    ;;
  "issue comment 2001 -R eng-cc/oasis7 --body-file "*)
    n=$(( $(wc -l < "$GH_COMMENT_LOG") + 1 ))
    mkdir -p "$GH_COMMENT_DIR"
    cat "${@: -1}" > "$GH_COMMENT_DIR/$n"
    python3 - "$GH_COMMENT_DIR/$n" "$GH_COMMENT_DIR/comment-$n.json" "$GH_ISSUE_UPDATED_AT_FILE" "$n" <<'PY'
from datetime import datetime, timedelta, timezone
import json, pathlib, re, sys
body_path, comment_path, issue_updated_path, raw_id = sys.argv[1:]
comment_id = int(raw_id)
body = pathlib.Path(body_path).read_text(encoding="utf-8")
verified = re.search(r"^Verified At: ([^\n]+)$", body, re.MULTILINE)
if verified:
    created_at = verified.group(1)
else:
    created_at = (datetime(2000, 1, 1, tzinfo=timezone.utc)
                  + timedelta(seconds=comment_id)).isoformat().replace("+00:00", "Z")
comment = {
    "id": comment_id,
    "body": body,
    "html_url": f"https://github.com/eng-cc/oasis7/issues/2001#issuecomment-{comment_id}",
    "issue_url": "https://api.github.com/repos/eng-cc/oasis7/issues/2001",
    "created_at": created_at,
    "updated_at": created_at,
    "user": {"login": "eng-cc"},
}
pathlib.Path(comment_path).write_text(json.dumps(comment), encoding="utf-8")
pathlib.Path(issue_updated_path).write_text(created_at + "\n", encoding="utf-8")
PY
    if [[ "${OASIS7_REC_CASE:-}" == "pending_project_content_drift" ]] && grep -Fq 'oasis7-ci-publication-binding/v1' "${@: -1}"; then
      printf 'blocked\n' >"$GH_PROJECT_PHASE_STATE_FILE"
    fi
    printf 'comment-%s\n' "$n" >> "$GH_COMMENT_LOG"
    printf 'https://github.com/eng-cc/oasis7/issues/2001#issuecomment-%s\n' "$n"
    ;;
  "api repos/eng-cc/oasis7/issues/2001/comments --paginate --slurp")
    python3 - "$GH_COMMENT_DIR" <<'PY'
import json, pathlib, sys
comment_dir = pathlib.Path(sys.argv[1])
comments = [
    json.loads(path.read_text(encoding="utf-8"))
    for path in sorted(comment_dir.glob("comment-*.json"),
                       key=lambda item: int(item.stem.split("-", 1)[1]))
]
print(json.dumps([comments]))
PY
    ;;
  api\ repos/eng-cc/oasis7/issues/comments/*)
    comment_id="${*: -1}"
    comment_id="${comment_id##*/}"
    python3 - "$GH_COMMENT_DIR/$comment_id" <<'PY'
import json, pathlib, sys
path=pathlib.Path(sys.argv[1])
metadata=path.parent / f"comment-{path.name}.json"
if metadata.is_file():
    comment=json.loads(metadata.read_text(encoding="utf-8"))
else:
    comment={"id":int(path.name),"user":{"login":"eng-cc"},
        "issue_url":"https://api.github.com/repos/eng-cc/oasis7/issues/2001",
        "html_url":"https://github.com/eng-cc/oasis7/issues/2001#issuecomment-"+path.name}
comment["body"]=path.read_text(encoding="utf-8")
comment.setdefault("author_association","OWNER")
print(json.dumps(comment))
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
    if [[ "${GH_INTERRUPT_AFTER_ISSUE_EDIT:-0}" == "1" ]]; then
      cp "${@: -1}" "$GH_ISSUE_BODY_STATE_FILE"
      kill -TERM "${GH_INTERRUPT_TARGET:?missing explicit interrupt target}"
      sleep 1
      exit 143
    fi
    if [[ "${GH_FAIL_ISSUE_EDIT:-0}" == "1" ]]; then
      echo "injected second-stage issue edit failure" >&2
      exit 77
    fi
    cp "${@: -1}" "$GH_ISSUE_BODY_STATE_FILE"
    printf '%s\n' '--- issue edit body ---' >> "$GH_EDIT_BODY_LOG"
    cat "${@: -1}" >> "$GH_EDIT_BODY_LOG"
    printf '\n' >> "$GH_EDIT_BODY_LOG"
    if [[ "${GH_APPLY_ISSUE_EDIT_THEN_INTERRUPT:-0}" == "1" ]]; then
      kill -TERM "${GH_INTERRUPT_TARGET:?missing explicit interrupt target}"
      sleep 1
      exit 143
    fi
    printf 'edited\n'
    ;;
  "issue list -R eng-cc/oasis7 --search task_99999999999999999999999999999999 in:body --json number,url,title,state --limit 5")
    printf '[{"number":2003,"state":"OPEN","title":"[PM] No-cache task","url":"https://github.com/eng-cc/oasis7/issues/2003"}]\n'
    ;;
  issue\ view\ 2003\ -R\ eng-cc/oasis7\ --json\ body,number,title,url,state,stateReason*)
    python3 - "$GH_CANONICAL_WORKTREE_HINT" <<'PY'
import json, sys
worktree = sys.argv[1]
body = """<!-- oasis7-pm-task -->
task_uid: task_99999999999999999999999999999999

GitHub-backed oasis7 PM task.

Task metadata:
- owner_role: `tpm`
- module: `engineering`
- status: `ready`
- workflow_phase: `pre_pr_ready`
- priority: `P2`
- worktree_hint: `""" + "`" + worktree + "`\n"
print(json.dumps({
    "body": body,
    "number": 2003,
    "title": "[PM] No-cache task",
    "url": "https://github.com/eng-cc/oasis7/issues/2003",
    "state": "OPEN",
    "stateReason": None,
    "updatedAt": "2026-10-01T00:00:00Z",
}))
PY
    ;;
  issue\ view\ 20[0-9][0-9]\ -R\ eng-cc/oasis7\ --json\ body,number,title,url,state,stateReason*)
    python3 - "$TMPDIR/github-project-task.py" "$GH_MAPPING_PATH" "${3}" "${GH_ISSUE_UPDATED_AT_FILE_2006:-}" <<'PY'
import importlib.util, json, pathlib, sys
script, mapping_path, number = sys.argv[1:4]
sys.path.insert(0, str(pathlib.Path(script).parent))
spec = importlib.util.spec_from_file_location("fixture_github_project_task", script)
assert spec and spec.loader
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
mapping = json.loads(pathlib.Path(mapping_path).read_text(encoding="utf-8"))
record = next((value for value in mapping.get("tasks", {}).values()
               if int(value.get("issue_number") or 0) == int(number)), None)
if record is None:
    if int(number) == 2003:
        uid = "task_99999999999999999999999999999999"
        print(json.dumps({
            "body": f"task_uid: {uid}\n- status: `committed`\n- workflow_phase: `execution`\n",
            "number": 2003,
            "title": "[PM] No-cache task",
            "url": "https://github.com/eng-cc/oasis7/issues/2003",
            "state": "OPEN",
            "stateReason": None,
            "updatedAt": "2026-10-01T00:00:00Z",
        }))
        raise SystemExit(0)
    raise SystemExit("fixture has no selected Issue mapping")
uid = str(record.get("task_uid") or "")
body = module.issue_body(module.task_from_record(uid, record))
updated_at = "2026-10-01T00:00:00Z"
if int(number) == 2006 and len(sys.argv) > 4 and sys.argv[4]:
    updated_at = pathlib.Path(sys.argv[4]).read_text(encoding="utf-8").strip()
print(json.dumps({
    "body": body,
    "number": int(number),
    "title": "[PM] " + str(record.get("title") or ""),
    "url": str(record.get("issue_url") or ""),
    "state": "OPEN",
    "stateReason": None,
    "updatedAt": updated_at,
}))
PY
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
    n=$(( $(wc -l < "$GH_COMMENT_LOG") + 1 ))
    mkdir -p "$GH_COMMENT_DIR"
    cat "${@: -1}" > "$GH_COMMENT_DIR/$n"
    python3 - "$GH_COMMENT_DIR/$n" "$GH_COMMENT_DIR/comment-$n.json" "$GH_ISSUE_UPDATED_AT_FILE_2006" "$n" <<'PY'
from datetime import datetime, timedelta, timezone
import json, pathlib, re, sys
body_path, comment_path, issue_updated_path, raw_id = sys.argv[1:]
comment_id = int(raw_id)
body = pathlib.Path(body_path).read_text(encoding="utf-8")
verified = re.search(r"^Verified At: ([^\n]+)$", body, re.MULTILINE)
if verified:
    created_at = verified.group(1)
else:
    created_at = (datetime(2000, 1, 1, tzinfo=timezone.utc)
                  + timedelta(seconds=comment_id)).isoformat().replace("+00:00", "Z")
comment = {
    "id": comment_id,
    "body": body,
    "html_url": f"https://github.com/eng-cc/oasis7/issues/2006#issuecomment-{comment_id}",
    "issue_url": "https://api.github.com/repos/eng-cc/oasis7/issues/2006",
    "created_at": created_at,
    "updated_at": created_at,
    "user": {"login": "eng-cc"},
}
pathlib.Path(comment_path).write_text(json.dumps(comment), encoding="utf-8")
pathlib.Path(issue_updated_path).write_text(created_at + "\n", encoding="utf-8")
PY
    printf 'comment-%s\n' "$n" >> "$GH_COMMENT_LOG"
    printf 'https://github.com/eng-cc/oasis7/issues/2006#issuecomment-%s\n' "$n"
    ;;
  "api repos/eng-cc/oasis7/issues/2006/comments --paginate --slurp")
    python3 - "$GH_COMMENT_DIR" <<'PY'
import json, pathlib, sys
comment_dir = pathlib.Path(sys.argv[1])
comments = [
    item for path in sorted(comment_dir.glob("comment-*.json"),
                             key=lambda item: int(item.stem.split("-", 1)[1]))
    if (item := json.loads(path.read_text(encoding="utf-8"))).get("issue_url")
       == "https://api.github.com/repos/eng-cc/oasis7/issues/2006"
]
print(json.dumps([comments]))
PY
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
    if [[ "$*" == *"--field-id FIELD_PR "* ]] && [[ -n "${GH_PROJECT_PR_STATE_FILE:-}" ]]; then
      python3 - "$GH_PROJECT_PR_STATE_FILE" "$@" <<'PY'
import pathlib, sys
arguments=sys.argv[2:]
pathlib.Path(sys.argv[1]).write_text(arguments[arguments.index("--text")+1])
PY
    fi
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
      if [[ "${GH_INTERRUPT_AFTER_PHASE_EDIT:-0}" == "1" && "$*" == *OPT_VERIFICATION* ]]; then
        kill -TERM "${GH_INTERRUPT_TARGET:?missing explicit interrupt target}"
        sleep 1
        exit 143
      fi
    fi
    if [[ "${OASIS7_REC_CASE:-}" == "guard_project_item_content_late_drift" \
          && -e "${GH_REC_PROJECT_CONTENT_DRIFT_ARMED_FILE:-/no-such-oasis7-drift-arm}" \
          && ! -e "${GH_REC_PROJECT_ITEM_CONTENT_FILE:-/no-such-oasis7-content}.drifted" ]]; then
      cat > "$GH_REC_PROJECT_ITEM_CONTENT_FILE" <<'JSON'
{"__typename":"Issue","number":2002,"url":"https://github.com/eng-cc/oasis7/issues/2002","repository":{"nameWithOwner":"eng-cc/oasis7"}}
JSON
      : > "${GH_REC_PROJECT_ITEM_CONTENT_FILE}.drifted"
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
printf '*.json\n*.log\n*.md\n*.err\n.pm/\n__pycache__/\nworktree/\ngh-comments/\nproject-live-state\nproject-live-status\nproject-live-phase\npr-read-count\nxcrun_db\ntmp*\n' > "$TMPDIR/.gitignore"
git -C "$TMPDIR" init -q
git -C "$TMPDIR" config user.email test@example.com
git -C "$TMPDIR" config user.name Test
git -C "$TMPDIR" add .
if [[ "${OASIS7_REC_RED_ONLY:-0}" == "1" ]]; then
  git -C "$TMPDIR" add -f scripts/pm AGENTS.md .agents/roles doc/engineering/workflow/source-of-truth.md
fi
git -C "$TMPDIR" commit -qm initial
# The temporary repository itself is the fixture's canonical worktree. Its
# root is registered and owns the task cache/evidence files used by the test.
CANONICAL_WORKTREE_HINT="$(cd "$TMPDIR" && pwd -P)"
export PATH="$TMPDIR/bin:$PATH"
export PYTHONPATH="$TMPDIR${PYTHONPATH:+:$PYTHONPATH}"
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
export GH_ISSUE_BODY_STATE_FILE="$TMPDIR/issue-live-body.md"
export GH_ISSUE_UPDATED_AT_FILE="$TMPDIR/.git/issue-updated-at"
export GH_ISSUE_UPDATED_AT_FILE_2006="$TMPDIR/.git/issue-updated-at-2006"
export GH_CANONICAL_WORKTREE_HINT="$CANONICAL_WORKTREE_HINT"
export GH_PR_READ_COUNT_FILE="$TMPDIR/pr-read-count"
export GH_PR_HEAD_SHA="$(git -C "$TMPDIR" rev-parse HEAD)"
export GH_PR_TASK_BRANCH="$(git -C "$TMPDIR" branch --show-current)"
export GH_PR_BASE_BRANCH="$GH_PR_TASK_BRANCH"
export OASIS7_ALLOW_FIXTURE_VERIFICATION_PROFILE=1
: > "$GH_CALL_LOG"
: > "$GH_COMMENT_LOG"
: > "$GH_EDIT_BODY_LOG"
mkdir -p "$GH_COMMENT_DIR"
printf '2026-10-01T00:00:00Z\n' > "$GH_ISSUE_UPDATED_AT_FILE"
printf '2026-10-01T00:00:00Z\n' > "$GH_ISSUE_UPDATED_AT_FILE_2006"

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
  --worktree-hint "$CANONICAL_WORKTREE_HINT" \
  --json > "$NEW_JSON"

TASK_UID="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["task_uid"])' "$NEW_JSON")"

if [[ "${OASIS7_REC_RED_ONLY:-0}" != "1" ]]; then
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
PRIMARY_GH_MAPPING_PATH="$GH_MAPPING_PATH"
# This focused fixture also reuses Issue #2001; seed its live body from the
# selected mapping before exercising that mapping's independent lifecycle.
python3 - "$TMPDIR/github-project-task.py" "$MOVE_PHASE_ROOT/.pm/github-project-sync/tasks.json" "$GH_ISSUE_BODY_STATE_FILE" <<'PY'
import importlib.util, json, pathlib, sys
script, mapping_path, issue_path = sys.argv[1:]
sys.path.insert(0, str(pathlib.Path(script).parent))
spec = importlib.util.spec_from_file_location("fixture_github_project_task", script)
assert spec and spec.loader
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
mapping = json.loads(pathlib.Path(mapping_path).read_text(encoding="utf-8"))
uid, record = next(iter(mapping["tasks"].items()))
pathlib.Path(issue_path).write_text(
    module.issue_body(module.task_from_record(uid, record)), encoding="utf-8"
)
PY
export GH_MAPPING_PATH="$MOVE_PHASE_ROOT/.pm/github-project-sync/tasks.json"
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
export GH_MAPPING_PATH="$PRIMARY_GH_MAPPING_PATH"
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
# The focused move-phase fixture reuses Issue #2001. Restore the primary
# fixture's authoritative Issue body explicitly when switching mappings back;
# issue view below always returns this persisted live state.
python3 - "$TMPDIR/github-project-task.py" "$GH_MAPPING_PATH" "$GH_ISSUE_BODY_STATE_FILE" "$GH_PROJECT_PHASE_STATE_FILE" <<'PY'
import importlib.util, json, pathlib, sys
script, mapping_path, issue_path, phase_path = sys.argv[1:]
sys.path.insert(0, str(pathlib.Path(script).parent))
spec = importlib.util.spec_from_file_location("fixture_github_project_task", script)
assert spec and spec.loader
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
mapping = json.loads(pathlib.Path(mapping_path).read_text(encoding="utf-8"))
uid, record = next(iter(mapping["tasks"].items()))
task = module.task_from_record(uid, record)
if not task.get("workflow_phase"):
    task["workflow_phase"] = pathlib.Path(phase_path).read_text(encoding="utf-8").strip()
pathlib.Path(issue_path).write_text(module.issue_body(task), encoding="utf-8")
PY

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
fi

# Focused REC entrypoint: actual record-pr, genuine ancestor/current journals,
# and independently persisted Project-post / Issue-pre GitHub IO.
if [[ "${OASIS7_REC_RED_ONLY:-0}" == "1" ]]; then
  export PYTHONPATH="$TMPDIR/scripts/pm"
  python3 - "$GH_MAPPING_PATH" "$TASK_UID" "$GH_ISSUE_BODY_STATE_FILE" <<'PY'
import importlib.util, json, pathlib, sys
spec = importlib.util.spec_from_file_location("task_helper", pathlib.Path(sys.argv[1]).parents[2] / "github-project-task.py")
helper = importlib.util.module_from_spec(spec)
spec.loader.exec_module(helper)
path = pathlib.Path(sys.argv[1]); mapping = json.loads(path.read_text())
record = mapping["tasks"][sys.argv[2]]
mapping["project"] = {"owner": "eng-cc", "number": 1, "id": "PROJECT_ID", "repo": "eng-cc/oasis7"}
record.update(status="committed", workflow_phase="execution")
pathlib.Path(sys.argv[3]).write_text(helper.issue_body(helper.task_from_record(sys.argv[2], record)))
# Real interrupted publication was refreshed from Project before recovery;
# the cache consequently contains Project's poststate, not Issue's prestate.
record.update(workflow_phase="verification")
path.write_text(json.dumps(mapping))
PY
  REC_OLD_HEAD="$(git -C "$TMPDIR" rev-parse HEAD)"
  REC_PUBLICATION_OLD_HEAD="$REC_OLD_HEAD"
  if [[ "${OASIS7_REC_CASE:-}" == "guard_old_nonancestor" ]]; then
    REC_PUBLICATION_OLD_HEAD="$(git -C "$TMPDIR" commit-tree "HEAD^{tree}" -m unrelated-publication-source)"
  fi
  git -C "$TMPDIR" commit --allow-empty -qm current-recovery-source
  export GH_PR_HEAD_SHA="$(git -C "$TMPDIR" rev-parse HEAD)"
  python3 - "$TMPDIR" "$TASK_UID" "$REC_OLD_HEAD" "$GH_PR_HEAD_SHA" "$GH_PR_TASK_BRANCH" "$REC_PUBLICATION_OLD_HEAD" <<'PY'
import json, pathlib, sys
import pr_projection_publication as publication
import pr_projection_journal as journal
from projection_publication_contract import digest
root, uid, old_head, current_head, branch, old_publication_head = sys.argv[1:]
root = pathlib.Path(root)
task_record = json.loads((root / ".pm/github-project-sync/tasks.json").read_text())["tasks"][uid]
task_binding = task_record.get("loop_binding")
task_epoch = (task_binding.get("bootstrap_epoch") if isinstance(task_binding, dict)
              else task_record.get("bootstrap_epoch"))
for index, head in enumerate((old_publication_head, current_head), 1):
    value = publication.build_task_publication(
        repository="eng-cc/oasis7", repository_id=7, task_uid=uid,
        bootstrap_epoch=task_epoch, source_repository_id=7, source_ref=branch,
        target_ref=branch, source_head_oid=head, source_scope_oid=old_head,
        planner_authority_oid=old_head, planner_config_sha256="sha256:" + "c" * 64,
        policy_digest=digest({"policy": "fixture"}), projection_digest=digest({"head": head}))
    local = journal.open_journal(root / ".git", value["repository"], branch,
        value["publication_id"], task_uid=uid, source_head_oid=head,
        scope_base_oid=old_head, projection_digest=value["projection_digest"])
    with local.locked():
        action = "record-pr:" + value["publication_id"]
        local.intent(action, "record_pr", {"publication_id": value["publication_id"],
            "task_uid": uid, "pr_number": 2001})
        local.uncertain(action, "NETWORK_UNCERTAIN")
    (root / "gh-comments" / str(1000 + index)).write_text(publication.publication_comment(value))
    if index == 1:
        (root / "old-journal-before.json").write_bytes(local.path.read_bytes())
        (root / "old-journal-path.md").write_text(str(local.path))
    else:
        (root / "recovery-publication.json").write_text(json.dumps(value))
        binding = publication.build_publication_binding(value, 2001,
            "https://github.com/eng-cc/oasis7/pull/2001")
        (root / "recovery-binding.json").write_text(json.dumps(binding))
        (root / "current-journal-path.md").write_text(str(local.path))
        _, marker = publication.prepare(task_uid=uid, source_head_oid=head,
            scope_base_oid=old_head, projection_digest=value["projection_digest"])
        (root / "recovery-pr-body.md").write_text(f"Task: {uid}\nRefs #2001\n\n" + marker)
PY
  export GH_REC_PR_BODY_FILE="$TMPDIR/recovery-pr-body.md"
  export GH_PROJECT_PR_STATE_FILE="$TMPDIR/project-pr.md"
  printf 'https://github.com/eng-cc/oasis7/pull/2001\n' >"$GH_PROJECT_PR_STATE_FILE"
  printf 'committed\n' >"$GH_PROJECT_STATE_FILE"
  printf 'In Progress\n' >"$GH_PROJECT_STATUS_STATE_FILE"
  printf 'verification\n' >"$GH_PROJECT_PHASE_STATE_FILE"
export GH_REC_AUTH_STATE_FILE="$TMPDIR/recovery-auth-state.json"
export GH_REC_PROJECT_ITEM_CONTENT_FILE="$TMPDIR/project-item-content.json"
export GH_REC_PROJECT_CONTENT_DRIFT_ARMED_FILE="$TMPDIR/project-content-drift-armed"
export GH_REC_SCOPE_DRIFT_ARMED_FILE="$TMPDIR/scope-comment-drift-armed"
  python3 - "$TMPDIR" "$TASK_UID" "$REC_OLD_HEAD" "$GH_PR_HEAD_SHA" "$GH_PR_TASK_BRANCH" <<'PY'
import hashlib, importlib.util, json, os, pathlib, subprocess, sys
root, uid, old_head, head, branch = pathlib.Path(sys.argv[1]).resolve(), *sys.argv[2:]
def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()
def sha(raw):
    return hashlib.sha256(raw).hexdigest()
def git(*args):
    return subprocess.check_output(["git", "-C", str(root), *args], text=True).strip()
def run_validator(command):
    environment=dict(os.environ); environment.pop("OASIS7_TEST_ALLOW_UNATTESTED_DISPATCH_RECEIPTS", None)
    result=subprocess.run(command, cwd=root, env=environment, capture_output=True, text=True)
    assert result.returncode == 0, (command, result.stdout, result.stderr)
    return json.loads(result.stdout) if result.stdout.lstrip().startswith("{") else {"status":"passed","stdout":result.stdout}
state={"actor":"eng-cc","repo_push":True,"issue_can_update":True,"project_can_update":True,
       "issue_state":"open","pr_state":"open","pr_merged":False}
(root / "recovery-auth-state.json").write_text(json.dumps(state))
manifest=[]
for row in git("ls-tree", "-r", head, "--", "scripts/pm").splitlines():
    metadata, path=row.split("\t"); mode, kind, oid=metadata.split()
    assert kind == "blob" and mode in {"100644","100755"}, row
    assert git("hash-object", str(root / path)) == oid, path
    manifest.append({"path":path,"mode":mode,"blob_oid":oid})
assert manifest == sorted(manifest, key=lambda item:item["path"].encode("ascii"))
closure_sha=sha(canonical(manifest))
task=json.loads((root / ".pm/github-project-sync/tasks.json").read_text())["tasks"][uid]
review=root / ".pm/scratch" / uid / "recovery-role-review"; review.mkdir(parents=True)
roles=("runtime_engineer","repository_health_engineer","qa_engineer")
returns=[]; ledger=[]
packet_spec=importlib.util.spec_from_file_location("fixture_packet",root / "scripts/pm/subagent-task-packet.py")
packet_module=importlib.util.module_from_spec(packet_spec); packet_spec.loader.exec_module(packet_module)
for index, role in enumerate(roles, 1):
    slice_id=f"00000000-0000-4000-8000-{index:012d}"
    identity={"task_uid":uid,"repository":"eng-cc/oasis7","project_item_id":"ITEM_ID",
        "task_status":"committed","issue_url":task["issue_url"],"worktree":str(root),
        "branch":branch,"head":head,"base_ref":old_head,"base_sha":old_head,
        "base_binding":"immutable_oid","packet_producer":"tpm","primary_package":task["primary_package"]}
    contract={"slice_id":slice_id,"role":role,"slice_type":"professional_review","owner_role":task["owner_role"],
        "integration_owner":"tpm","integration_order":"REC bounded review then exact action binding",
        "context_delivery_mode":"minimal_head_bound_task_packet","intended_model_configuration":"inherit current parent selection",
        "actual_dispatched_model_reasoning":"fixture unobserved","actual_runtime_evidence_reason":"fixture adapter inactive",
        "role_activation":"message_assigned_adapter_inactive","write_scope":"scratch-only bounded helper review",
        "return_contract":"exact immutable helper closure verdict","validation_command":"human-operated local provenance",
        "formal_sink":task["issue_url"],"full_history_escalation_reason":""}
    context={"user_intent":"existing approved same-PR metadata recovery","work_item":"REC bounded helper review",
        "non_goals":"no CI/review/Ready authority","acceptance_target":"exact immutable helper closure",
        "evidence_summary":"isolated authentic Git/helper/role artifact fixture","collaboration_boundary":"fixture scratch only",
        "governance_refs":["AGENTS.md","doc/engineering/workflow/source-of-truth.md",f".agents/roles/{role}.md"],
        "scoped_refs":["scripts/pm/github-project-task.py","scripts/pm/pr_projection_publish.py"]}
    packet={"schema":"oasis7-subagent-task-packet/v1","identity":identity,"slice":contract,"context":context}
    packet["packet_digest"]=packet_module.canonical_digest(packet)
    packet_path=root / ".pm/scratch" / uid / "slice-packets" / f"{slice_id}.json"
    packet_path.parent.mkdir(parents=True,exist_ok=True); packet_path.write_bytes(canonical(packet)+b"\n")
    run_validator([sys.executable,str(root / "scripts/pm/subagent-task-packet.py"),"validate",str(packet_path)])
    returned={"task_uid":uid,"role":role,"slice_id":slice_id,"head":head,"status":"completed",
        "scope_verdict":"approved","risk_verdict":"approved","findings":"no_findings","residual_risk":"isolated fixture",
        "helper_source_oid":head,"helper_closure_sha256":closure_sha,"activation":"message-assigned",
        "context_delivery":"minimal_head_bound_task_packet","actual_runtime":"fixture inherited unobserved"}
    return_path=review / f"{role}.return.json"; return_path.write_bytes(canonical(returned)+b"\n")
    return_sha=sha(return_path.read_bytes())
    ledger.append(dict(returned,artifact_digest=return_sha,artifacts=[str(return_path.relative_to(root))]))
    returns.append({"role":role,"slice_id":slice_id,"packet_sha256":sha(packet_path.read_bytes()),
        "source_head_oid":head,"return_path":str(return_path.relative_to(root)),"return_sha256":return_sha,"verdict":"approved"})
ledger_path=review / "ledger.jsonl"; ledger_path.write_bytes(b"".join(canonical(row)+b"\n" for row in ledger))
verdict=run_validator([sys.executable,str(root / "scripts/pm/validate-review-provenance.py"),
    "--root",str(root),"--task-uid",uid,"--ledger",str(ledger_path),"--roles",",".join(roles),
    "--source-head",head,"--mode","human-operated"])
assert verdict["status"] == "passed" and verdict["mode"] == "human-operated", verdict
(review / "validated-provenance.json").write_bytes(canonical(verdict)+b"\n")
# Only synthetic instance identities belong in this isolated GitHub stub.
# Model the existing user-evidence JSON framing and all eight Plan-Gap fields.
def plan_row(step, acceptance, dependencies, command, evidence, writes, exclusions, role_slices):
    return dict(step_id=step,acceptance_refs=acceptance,dependencies=dependencies,
        verification_command=command,verification_evidence=evidence,write_scope=writes,
        out_of_scope=exclusions,required_role_slices=role_slices)
rows=[
    plan_row("REC-SPEC","fixture Task original acceptance plus same-PR publication recovery",
        "existing user approval; same fixture PR; immutable historical journal",
        "workflow documentation contracts and runtime/QA API closure",
        "source-only contract tests and bounded runtime/QA returns",
        "doc/engineering/workflow/source-of-truth.md only by repository_health_engineer",
        "gate waiver; activation; raw task/cache/Project patches; another PR",
        "repository_health_engineer author; runtime_engineer and qa_engineer API review"),
    plan_row("REC-RED","partial Issue/Project write recovery exact journal/UID/H/PR and complete poststate",
        "REC-SPEC closure","focused genuine github-project-task/publication RED tests",
        "immutable test patch digest and actual exit logs; independent QA",
        "scripts/pm/github-project-task.test.sh and scripts/pm/pr_projection_publication.test.py by runtime_engineer",
        "production helpers before admitted GREEN; loosened assertions",
        "runtime_engineer test author; qa_engineer independent RED"),
    plan_row("REC-GREEN","authentic journal-constrained partial-poststate reconciliation; reject drift and missing readback",
        "actual RED confirmed plus TPM GREEN admission","focused REC cases and ordinary task/publication regressions",
        "runtime and independent QA GREEN logs/digests; immutable reviewed helper closure",
        "scripts/pm/github-project-task.py; scripts/pm/pr_projection_publish.py only if required for journal transport/recovery",
        "other helpers; activation; forged authority; historical journal mutation",
        "runtime_engineer implementation; qa_engineer independent; repository_health_engineer conformance"),
    plan_row("REC-RESTORE","fresh current-H unique fixture PR reciprocal binding through canonical execution authority",
        "REC-SPEC/RED/GREEN closure and executable-route confirmation",
        "canonical selected-task refresh/audit/publication resume and live four-surface readback",
        "complete exact poststate; unique reciprocal binding; current action observed only after final readback",
        "canonical tasktruth/journal through approved helpers only",
        "raw cache/body/Project patches; rollback; CI before binding; second PR",
        "repository_health_engineer authority review; runtime_engineer route; TPM execution"),
    plan_row("REC-DELIVER","fixture original acceptance and bounded recovery delivered in same Task/same PR",
        "REC-RESTORE complete","fresh freeze, involved-role review, required CI and canonical merge/finalization",
        "current source/base/scope/projection roles, CI, receipts and cleanup",
        "sameTask samePR only; original two CLI paths retained, total seven approved paths max",
        "second PR; historical receipt reuse; activation; unrelated delivery completion",
        "runtime_engineer/repository_health_engineer/qa_engineer formal review; TPM integration")]
row_keys={"step_id","acceptance_refs","dependencies","verification_command","verification_evidence",
          "write_scope","out_of_scope","required_role_slices"}
assert all(set(row)==row_keys and all(isinstance(value,str) and value for value in row.values()) for row in rows)
assert [row["step_id"] for row in rows]==["REC-SPEC","REC-RED","REC-GREEN","REC-RESTORE","REC-DELIVER"]
fixture_identity=f"Task UID {uid}; Issue https://github.com/eng-cc/oasis7/issues/2001; same PR https://github.com/eng-cc/oasis7/pull/2001"
frozen_red_digest=sha(b"synthetic isolated frozen RED patch")
scope_bodies={
    7101:"WP2 bounded dormant CLI artifact output prerequisite\n"+fixture_identity+"\nPlan-Gap entries beforewrites:\n"
         "CLI-RED acceptance_refs original artifact output acceptance; dependencies actual focused CLI RED; "
         "verification_command focused main-output unittest; verification_evidence actual exit/log/test patch digest; "
         "write_scope scripts/pm/ci-reuse-validation.test.py ONLY; out_of_scope production helpers/schema/activation; "
         "required_role_slices runtime_engineer author and independent qa_engineer.\n"
         "CLI-GREEN acceptance_refs verified payload mode to artifact output while disabled; dependencies immutable RED plus QA; "
         "verification_command same focused cases and producer/contract/readback negatives; verification_evidence actual GREEN logs/diff; "
         "write_scope scripts/pm/ci-reuse-validation.py ONLY; out_of_scope authority/schema/activation; "
         "required_role_slices runtime_engineer implementation and independent qa_engineer.",
    7102:"CLI-RED completed and independently confirmed. Frozen synthetic test digest "+frozen_red_digest+". "
         "CLI-GREEN admission: runtime_engineer may modify ONLY scripts/pm/ci-reuse-validation.py; original tests remain immutable. "
         "No other source, policy, schema, commits, dispatch or activation. "+fixture_identity,
    7103:"User authority intake: explicit same-PR source-first recovery; existing original acceptance retained; "
         "original two CLI paths from synthetic history retained; total seven approved paths max. No gate exceptions.\n"
         +fixture_identity+"\nPlan-Gap Evidence:\n"+json.dumps(rows,indent=2),
    7104:"REC-SPEC COMPLETE with bounded runtime_engineer and independent qa_engineer API closure. "
         "REC-RED admission: ONLY scripts/pm/github-project-task.test.sh and scripts/pm/pr_projection_publication.test.py. "
         "Frozen synthetic RED digest "+frozen_red_digest+"; no production helpers until a new TPM GREEN admission. "
         "Preserve exact final readback, pending uncertainty and immutable historical journal; no activation. "+fixture_identity,
    7105:"REC-RED COMPLETE / GREEN ADMITTED: frozen synthetic test patch "+frozen_red_digest+"; actual focused RED and independent QA confirmed. "
         "Runtime may implement ONLY scripts/pm/github-project-task.py and scripts/pm/pr_projection_publish.py (publisher only when journal transport/recovery requires it). "
         "Frozen tests immutable; exact independent final Issue/Project/PR/unique binding readback before current action observed; "
         "retain pending state on uncertainty; no historical journal mutation, activation or other helper scope. "+fixture_identity}
scope=[]
for comment_id, body in scope_bodies.items():
    (root / "gh-comments" / str(comment_id)).write_text(body)
    scope.append({"comment_id":comment_id,"comment_url":f"https://github.com/eng-cc/oasis7/issues/2001#issuecomment-{comment_id}",
                  "body_sha256":sha(body.encode()),"author_login":"eng-cc"})
def action(locator, comment_id):
    journal_path=pathlib.Path((root / locator).read_text())
    value=json.loads(journal_path.read_text()); intent_body=(root / "gh-comments" / str(comment_id)).read_text()
    import pr_projection_publication as publication
    intent=publication.parse_publication_comment(intent_body)
    return {"publication_id":intent["publication_id"],"action_id":value["actions"][0]["action_id"],
        "journal_sha256":sha(journal_path.read_bytes()),"H":intent["source_head_oid"],"B":intent["planner_authority_oid"],
        "S":intent["source_scope_oid"],"D":intent["projection_digest"],"intent_comment_id":comment_id,
        "intent_body_sha256":sha(intent_body.encode())}
unrelated_issue={key:task.get(key) for key in ("task_uid","owner_role","module","priority","worktree_hint","primary_package")}
unrelated_project={"Task UID":uid,"Owner Role":task["owner_role"],"Module":task["module"],"Priority":task["priority"],
                   "Canonical Worktree":task["worktree_hint"],"Test Tier Required":"n/a",
                   "Repository":{"type":"repository","id":"R_fixture_oasis7","name_with_owner":"eng-cc/oasis7"}}
admission={"schema":"oasis7-publication-recovery-admission/v1","operation":"record_pr_publication_recovery",
    "identity":{"repository":"eng-cc/oasis7","task_uid":uid,"issue_number":2001,"issue_url":task["issue_url"],
        "pr_number":2001,"pr_url":"https://github.com/eng-cc/oasis7/pull/2001","project_id":"PROJECT_ID",
        "project_item_id":"ITEM_ID","canonical_worktree":str(root),"source_ref":branch,"target_ref":branch},
    "scope_evidence":scope,"current_action":action("current-journal-path.md",1002),"predecessor":action("old-journal-path.md",1001),
    "helper_review":{"helper_source_oid":head,"helper_closure_sha256":closure_sha,"closure_manifest":manifest,
        "ledger_path":str(ledger_path.relative_to(root)),"ledger_sha256":sha(ledger_path.read_bytes()),"role_returns":returns},
    "unrelated_snapshot":{"issue_sha256":sha(canonical(unrelated_issue)),"project_sha256":sha(canonical(unrelated_project))}}
(root / "recovery-admission.json").write_bytes(canonical(admission)+b"\n")
(root / "gh-comments/9001").write_text("<!-- oasis7-publication-recovery-admission/v1 -->\n```json\n"+canonical(admission).decode()+"\n```\n")
print("REC auth fixture: immutable Git closure and three local role packets/returns validated (human-operated)")
PY
  REC_CASE="${OASIS7_REC_CASE:-project_post_issue_pre}"
  python3 - "$TMPDIR" "$TASK_UID" "$REC_CASE" <<'PY'
import importlib.util, json, pathlib, sys
root, uid, case = pathlib.Path(sys.argv[1]), sys.argv[2], sys.argv[3]
spec = importlib.util.spec_from_file_location("rec_task", root / "github-project-task.py")
helper = importlib.util.module_from_spec(spec); spec.loader.exec_module(helper)
path = root / ".pm/github-project-sync/tasks.json"; mapping = json.loads(path.read_text())
record = mapping["tasks"][uid]
if case in {"all_pre", "cache_pre_project_post", "pending_final_readback", "pending_project_content_drift", "idempotent_repeat", "interrupted_retry_accumulates_pending_steps", "guard_issue_body_proof_after_write", "guard_c1_writer_unreachable_step_suffix", "guard_c1_writer_binding_comment_without_predecessors", "guard_c1_writer_observed_vector_live_before", "guard_c1_writer_uncertain_step_with_observation"}:
    record.update(workflow_phase="execution")
    path.write_text(json.dumps(mapping))
if case in {"all_post", "project_pre_issue_post"} or (case.startswith("guard_") and case not in {"guard_c1_writer_unreachable_step_suffix", "guard_c1_writer_binding_comment_without_predecessors", "guard_c1_writer_observed_vector_live_before", "guard_c1_writer_uncertain_step_with_observation"}):
    issue = dict(record, workflow_phase="verification", pr_number=2001,
                 pr_url="https://github.com/eng-cc/oasis7/pull/2001")
    (root / "issue-live-body.md").write_text(helper.issue_body(helper.task_from_record(uid, issue)))
if case in {"guard_c1_writer_unreachable_step_suffix", "guard_c1_writer_binding_comment_without_predecessors", "guard_c1_writer_observed_vector_live_before", "guard_c1_writer_uncertain_step_with_observation", "interrupted_retry_accumulates_pending_steps", "guard_issue_body_proof_after_write"}:
    issue = dict(record, workflow_phase="execution", pr_number=None, pr_url=None)
    (root / "issue-live-body.md").write_text(helper.issue_body(helper.task_from_record(uid, issue)))
if case in {"all_pre", "project_pre_issue_post", "guard_project_item_content_late_drift", "guard_c1_writer_unreachable_step_suffix", "guard_c1_writer_binding_comment_without_predecessors", "guard_c1_writer_observed_vector_live_before", "guard_c1_writer_uncertain_step_with_observation", "interrupted_retry_accumulates_pending_steps", "guard_issue_body_proof_after_write"}:
    (root / "project-live-phase").write_text("execution\n")
    (root / "project-pr.md").write_text("")
if case == "guard_fieldwise_issue_mixed":
    issue = dict(record, workflow_phase="verification", pr_number=None, pr_url=None)
    (root / "issue-live-body.md").write_text(helper.issue_body(helper.task_from_record(uid, issue)))
if case == "guard_missing_old_journal":
    pathlib.Path((root / "old-journal-path.md").read_text()).unlink()
if case == "guard_old_not_uncertain":
    old = pathlib.Path((root / "old-journal-path.md").read_text())
    value = json.loads(old.read_text()); value["actions"][0]["state"] = "observed"
    old.write_text(json.dumps(value)); (root / "old-journal-before.json").write_bytes(old.read_bytes())
if case in {"guard_old_action_tuple", "guard_current_journal_identity"}:
    locator = "old-journal-path.md" if case == "guard_old_action_tuple" else "current-journal-path.md"
    local = pathlib.Path((root / locator).read_text()); value = json.loads(local.read_text())
    if case == "guard_old_action_tuple":
        value["actions"][0]["expected"]["pr_number"] = 9999
    else:
        value["identity"]["projection_digest"] = "sha256:" + "f" * 64
    local.write_text(json.dumps(value))
    if case == "guard_old_action_tuple":
        (root / "old-journal-before.json").write_bytes(local.read_bytes())
if case == "guard_duplicate_current_intent":
    (root / "gh-comments/1003").write_bytes((root / "gh-comments/1002").read_bytes())
if case == "guard_missing_current_intent":
    (root / "gh-comments/1002").unlink()
if case == "guard_unrelated_issue_drift":
    body = (root / "issue-live-body.md").read_text().replace("- priority: `P2`", "- priority: `P1`")
    (root / "issue-live-body.md").write_text(body)
if case == "guard_unrelated_project_drift":
    (root / "project-live-status").write_text("Done\n")
if case == "guard_repository_identity_drift":
    (root / "repository-field.json").write_text(json.dumps({"id":"R_different","nameWithOwner":"eng-cc/different"}))
if case == "guard_repository_identity_malformed":
    (root / "repository-field.json").write_text(json.dumps({"id":"R_fixture_oasis7"}))
if case == "guard_project_item_content_wrong":
    (root / "project-item-content.json").write_text(json.dumps({
        "__typename":"Issue","number":2002,
        "url":"https://github.com/eng-cc/oasis7/issues/2002",
        "repository":{"nameWithOwner":"eng-cc/oasis7"}}))
if case == "guard_project_item_content_missing":
    (root / "project-item-content.json").write_text("null")
if case == "guard_project_item_content_nonissue":
    (root / "project-item-content.json").write_text(json.dumps({"__typename":"PullRequest"}))
if case == "guard_project_item_content_cross_repository":
    (root / "project-item-content.json").write_text(json.dumps({
        "__typename":"Issue","number":2001,
        "url":"https://github.com/eng-cc/oasis7/issues/2001",
        "repository":{"nameWithOwner":"eng-cc/other"}}))
if case == "guard_pr_task_refs_drift":
    (root / "recovery-pr-body.md").write_text("Task: task_" + "9" * 32 + "\nRefs #9999\n")
# Negative variants alter only fake live IO or its authenticated TPM binding;
# production code receives no environment authority grant.
auth_path=root / "recovery-auth-state.json"; auth=json.loads(auth_path.read_text())
if case == "guard_issue_permission": auth["issue_can_update"]=False
if case == "guard_project_permission": auth["project_can_update"]=False
if case == "guard_actor_mismatch": auth["actor"]="different-actor"
if case == "guard_issue_closed": auth["issue_state"]="closed"
if case == "guard_pr_closed": auth["pr_state"]="closed"
if case == "guard_pr_merged": auth["pr_merged"]=True
auth_path.write_text(json.dumps(auth))
admission_path=root / "recovery-admission.json"; admission=json.loads(admission_path.read_text())
if case == "guard_step_scope":
    scope_path=root / "gh-comments/7103"
    body=scope_path.read_text().replace("scripts/pm/github-project-task.py; scripts/pm/pr_projection_publish.py", "scripts/pm/unapproved-helper.py")
    scope_path.write_text(body)
    import hashlib
    next(item for item in admission["scope_evidence"] if item["comment_id"] == 7103)["body_sha256"]=hashlib.sha256(body.encode()).hexdigest()
if case == "guard_helper_source": admission["helper_review"]["helper_source_oid"]="f" * 40
if case == "guard_helper_closure_digest": admission["helper_review"]["helper_closure_sha256"]="f" * 64
if case == "guard_old_raw_journal_hash": admission["predecessor"]["journal_sha256"]="f" * 64
if case == "guard_current_raw_journal_hash": admission["current_action"]["journal_sha256"]="f" * 64
if case == "guard_role_return_digest": admission["helper_review"]["role_returns"][0]["return_sha256"]="f" * 64
if case in {"guard_observed_payload_raw_hash_equal", "guard_binding_shape_raw_hash_equal",
            "guard_invalid_global_phase_raw_hash_equal", "guard_invalid_global_disposition_raw_hash_equal"}:
    import hashlib
    current_path=pathlib.Path((root / "current-journal-path.md").read_text())
    journal=json.loads(current_path.read_text())
    action=next(a for a in journal["actions"] if a["action_id"] == admission["current_action"]["action_id"])
    if case in {"guard_invalid_global_phase_raw_hash_equal", "guard_invalid_global_disposition_raw_hash_equal"}:
        binding_id="reciprocal-binding:" + admission["current_action"]["publication_id"]
        assert not any(a.get("action_id") == binding_id for a in journal["actions"]), "fixture unexpectedly has a reciprocal binding action"
        action["state"]="intent" if case == "guard_invalid_global_phase_raw_hash_equal" else "uncertain"
        action.pop("observed", None)
        if case == "guard_invalid_global_phase_raw_hash_equal":
            journal["phase"]="CONFLICT"
            journal["disposition"]=None
        else:
            journal["phase"]="PREPARED"
            journal["disposition"]="CONFLICT"
    elif case == "guard_observed_payload_raw_hash_equal":
        action["state"]="observed"
        action["observed"]={"pr_number":9999}
    else:
        action["state"]="observed"
        action["observed"]={"pr_number":2001}
        binding=json.loads((root / "recovery-binding.json").read_text())
        journal["actions"].append({
            "action_id":"reciprocal-binding:"+binding["publication_id"],
            "kind":"publish_reciprocal_binding",
            "state":"observed",
            "expected":{"publication_id":binding["publication_id"],
                "pr_number":binding["pr_number"],"binding_digest":binding["binding_digest"]},
            "observed":{"binding_digest":binding["binding_digest"],"extra":"malformed"}})
        journal["phase"]="METADATA_CONFIRMED"
        journal["disposition"]=None
    current_path.write_text(json.dumps(journal,sort_keys=True,separators=(",",":")))
    current_raw=current_path.read_bytes()
    admission["current_action"]["journal_sha256"]=hashlib.sha256(current_raw).hexdigest()
if case == "guard_noncanonical_observed_journal":
    import hashlib
    current_path=pathlib.Path((root / "current-journal-path.md").read_text())
    original=current_path.read_bytes()
    assert hashlib.sha256(original).hexdigest() == admission["current_action"]["journal_sha256"]
    journal=json.loads(original)
    action=next(a for a in journal["actions"] if a["action_id"] == admission["current_action"]["action_id"])
    action["state"]="observed"
    action["observed"]={"pr_number":2001}
    journal["phase"]="METADATA_CONFIRMED"
    journal["disposition"]=None
    canonical=(json.dumps(journal,sort_keys=True,separators=(",",":"))+"\n").encode()
    rewritten=(json.dumps(journal,indent=2)+"\n").encode()
    assert rewritten != canonical and json.loads(rewritten) == json.loads(canonical)
    current_path.write_bytes(rewritten)
admission_path.write_text(json.dumps(admission,sort_keys=True,separators=(",",":")))
(root / "gh-comments/9001").write_text("<!-- oasis7-publication-recovery-admission/v1 -->\n```json\n"+json.dumps(admission,sort_keys=True,separators=(",",":"))+"\n```\n")
if case in {"guard_observed_payload_raw_hash_equal", "guard_binding_shape_raw_hash_equal",
            "guard_invalid_global_phase_raw_hash_equal", "guard_invalid_global_disposition_raw_hash_equal"}:
    current_path=pathlib.Path((root / "current-journal-path.md").read_text())
    current_raw=current_path.read_bytes()
    assert admission["current_action"]["journal_sha256"] == hashlib.sha256(current_raw).hexdigest(), "fixture admission does not bind exact malformed journal bytes"
if case in {"guard_observed_payload_raw_hash_equal", "guard_binding_shape_raw_hash_equal",
            "guard_invalid_global_phase_raw_hash_equal", "guard_invalid_global_disposition_raw_hash_equal",
            "guard_scope_comment_drift", "guard_recovery_context_drift", "guard_noncanonical_observed_journal",
            "guard_project_item_content_wrong", "guard_project_item_content_missing",
            "guard_project_item_content_nonissue", "guard_project_item_content_cross_repository",
            "guard_c1_timestamp_recovery_drift", "guard_recovery_context_drift",
            "guard_project_item_content_late_drift", "guard_c1_writer_scope_drift",
            "guard_c1_writer_unknown_journal_drift", "guard_c1_writer_reordered_journal_drift",
            "guard_c1_writer_step_without_vector", "guard_c1_writer_unreachable_step_suffix",
            "guard_c1_writer_binding_comment_without_predecessors", "guard_c1_writer_observed_vector_live_before",
            "guard_c1_writer_uncertain_step_with_observation", "guard_fieldwise_issue_mixed"}:
    current_path=pathlib.Path((root / "current-journal-path.md").read_text())
    (root / "current-journal-before.json").write_bytes(current_path.read_bytes())
PY
  if [[ "$REC_CASE" == "pending_final_readback" || "$REC_CASE" == "pending_project_content_drift" || "$REC_CASE" == "idempotent_repeat" ]]; then
    if [[ "$REC_CASE" == "pending_final_readback" ]]; then export GH_REC_FAIL_FINAL_ISSUE=1; fi
    python3 - "$TMPDIR" "$TASK_UID" "$REC_CASE" <<'PY'
import argparse, json, pathlib, sys
import pr_projection_journal as journal
import pr_projection_publication as publication
import pr_projection_publish as publisher
root, uid, case = pathlib.Path(sys.argv[1]), sys.argv[2], sys.argv[3]
value = json.loads((root / "recovery-publication.json").read_text())
args = argparse.Namespace(repo=value["repository"], issue_number=2001, task_uid=uid,
    task_helper=str(root / "scripts/pm/github-project-task.py"), existing_ready_update=False,
    source_ref=value["source_ref"], target_ref=value["target_ref"])
adapter = publisher.GitHubPublicationAdapter(root, args, value)
local = journal.open_journal(root / ".git", value["repository"], value["source_ref"],
    value["publication_id"], task_uid=uid, source_head_oid=value["source_head_oid"],
    scope_base_oid=value["source_scope_oid"], projection_digest=value["projection_digest"])
with local.locked():
    pr = adapter.read_pr(value["repository"], 2001)
    if case in {"pending_final_readback", "pending_project_content_drift"}:
        try:
            publication._record_and_bind(adapter, local, value, pr)
        except publication.PublicationError as error:
            assert error.code == "NETWORK_UNCERTAIN", error
        else:
            raise AssertionError("missing or mismatched final authoritative readback must retain pending state")
        action = next(a for a in local.read()["actions"] if a["kind"] == "record_pr")
        assert action["state"] == "uncertain", action
        assert '- pr_number: `2001`' in (root / "issue-live-body.md").read_text(), "failure occurred before metadata publication"
        assert "injected final authoritative Issue readback failure" in (root / "gh-calls.log").read_text() or any(
            "oasis7-ci-publication-binding/v1" in p.read_text() for p in (root / "gh-comments").iterdir()), "no final-read boundary reached"
    else:
        publication._record_and_bind(adapter, local, value, pr)
        before = local.path.read_bytes()
        comments_before = sorted(p.read_bytes() for p in (root / "gh-comments").iterdir())
        publication._record_and_bind(adapter, local, value, pr)
        current = next(a for a in local.read()["actions"] if a["kind"] == "record_pr")
        assert current["state"] == "observed", current
        assert before == local.path.read_bytes(), "repeat changed confirmed journal"
        assert comments_before == sorted(p.read_bytes() for p in (root / "gh-comments").iterdir()), "repeat duplicated publication evidence"
assert pathlib.Path((root / "old-journal-path.md").read_text()).read_bytes() == (root / "old-journal-before.json").read_bytes()
print("PASS test_rec_" + case)
PY
    exit 0
  fi
  if [[ "$REC_CASE" == "guard_pr_head_drift" ]]; then export GH_PR_HEAD_SHA="$REC_OLD_HEAD"; fi
if [[ "$REC_CASE" == "guard_project_item_content_late_drift" ]]; then : >"$GH_REC_PROJECT_CONTENT_DRIFT_ARMED_FILE"; fi
if [[ "$REC_CASE" == "guard_scope_comment_drift" ]]; then
  python3 - "$TMPDIR" <<'PY'
import json, pathlib, sys
root=pathlib.Path(sys.argv[1]); comments=root / "gh-comments"
admission=json.loads((root / "recovery-admission.json").read_text())
rows=admission["scope_evidence"]
assert len(rows) == 5, f"scope drift fixture expected five seeded comments, found {len(rows)}"
assert all((comments / str(row["comment_id"])).is_file() for row in rows), "scope evidence comment was not seeded before drift arm"
PY
  printf '0\n' >"$TMPDIR/gh-comment-read-count.txt"
  : >"$GH_REC_SCOPE_DRIFT_ARMED_FILE"
fi
  if [[ "$REC_CASE" == "guard_api_external_wait" ]]; then export GH_FIXTURE_EXTERNAL_WAIT=1; fi
  if [[ "$REC_CASE" == "guard_recovery_context_drift" ]]; then printf '0\n' >"$TMPDIR/gh-comment-read-count.txt"; fi
  rm -f "$GH_REC_AUTH_STATE_FILE.api-user-reads"
  REC_CALLS_BEFORE="$(wc -l < "$GH_CALL_LOG")"
  cp "$GH_MAPPING_PATH" "$TMPDIR/rec-mapping-before.json"
  set +e
  if [[ "$REC_CASE" == "guard_c1_writer_observed_vector_live_before" ]]; then
    python3 - "$TMPDIR/scripts/pm/github-project-task.py" "$TMPDIR/rec-check-returns.jsonl" \
      record-pr "$TMPDIR" --repo eng-cc/oasis7 --project-owner eng-cc --project-number 1 \
      --task-uid "$TASK_UID" --pr-url "https://github.com/eng-cc/oasis7/pull/2001" \
      --draft-candidate --publication-binding-json "$TMPDIR/recovery-binding.json" --json \
      >"$TMPDIR/rec-record.json" 2>"$TMPDIR/rec-record.err" <<'PY'
import importlib.util, json, pathlib, sys
script, trace_path, *cli_args = sys.argv[1:]
spec=importlib.util.spec_from_file_location("rec_check_boundary", script)
module=importlib.util.module_from_spec(spec); sys.modules[spec.name]=module; spec.loader.exec_module(module)
original=module.PublicationRecoveryAuthority.check
def append_trace(path, event):
    with open(path,"a",encoding="utf-8") as stream:
        stream.write(json.dumps(event)+"\n")
def traced_check(self, *args, **kwargs):
    try:
        result=original(self, *args, **kwargs)
    except BaseException as exc:
        locator=pathlib.Path(self.root) / "current-journal-path.md"
        journal=pathlib.Path(locator.read_text()).read_bytes()
        append_trace(trace_path,{"event":"raise","journal_sha256":self.sha(journal),
            "error":type(exc).__name__ + ": " + str(exc),"final":bool(kwargs.get("final",False))})
        raise
    locator=pathlib.Path(self.root) / "current-journal-path.md"
    journal=pathlib.Path(locator.read_text()).read_bytes()
    append_trace(trace_path,{"event":"return","journal_sha256":self.sha(journal),
        "final":bool(kwargs.get("final",False))})
    return result
module.PublicationRecoveryAuthority.check=traced_check
sys.argv=[script,*cli_args]
raise SystemExit(module.main())
PY
  elif [[ "$REC_CASE" == "interrupted_retry_accumulates_pending_steps" ]]; then
    GH_INTERRUPT_AFTER_ISSUE_EDIT=1 bash -c 'export GH_INTERRUPT_TARGET=$$; exec python3 "$@"' bash \
      "$TMPDIR/scripts/pm/github-project-task.py" record-pr "$TMPDIR" \
      --repo eng-cc/oasis7 --project-owner eng-cc --project-number 1 \
      --task-uid "$TASK_UID" --pr-url "https://github.com/eng-cc/oasis7/pull/2001" \
      --draft-candidate --publication-binding-json "$TMPDIR/recovery-binding.json" --json \
      >"$TMPDIR/rec-record-first.json" 2>"$TMPDIR/rec-record-first.err"
    FIRST_STATUS=$?
    set -e
    if [[ "$FIRST_STATUS" != "143" ]]; then
      echo "FAIL test_rec_interrupted_retry_accumulates_pending_steps: first Issue write did not interrupt after applying (exit=$FIRST_STATUS)" >&2
      cat "$TMPDIR/rec-record-first.err" >&2
      exit 1
    fi
    python3 - "$TMPDIR" <<'PY'
import json, pathlib, sys
root=pathlib.Path(sys.argv[1])
journal=json.loads(pathlib.Path((root / "current-journal-path.md").read_text()).read_bytes())
binding=json.loads((root / "recovery-binding.json").read_text())
rows={row.get("action_id"):row for row in journal["actions"]}
issue_step="record-pr-step:"+binding["publication_id"]+":issue"
assert rows[issue_step]["state"] == "intent", rows[issue_step]
assert "- pr_number: `2001`" in (root / "issue-live-body.md").read_text(), "Issue effect was not applied before interruption"
PY
    set +e
    GH_INTERRUPT_AFTER_PHASE_EDIT=1 bash -c 'export GH_INTERRUPT_TARGET=$$; exec python3 "$@"' bash \
      "$TMPDIR/scripts/pm/github-project-task.py" record-pr "$TMPDIR" \
      --repo eng-cc/oasis7 --project-owner eng-cc --project-number 1 \
      --task-uid "$TASK_UID" --pr-url "https://github.com/eng-cc/oasis7/pull/2001" \
      --draft-candidate --publication-binding-json "$TMPDIR/recovery-binding.json" --json \
      >"$TMPDIR/rec-record-second.json" 2>"$TMPDIR/rec-record-second.err"
    SECOND_STATUS=$?
    set -e
    if [[ "$SECOND_STATUS" != "143" ]]; then
      echo "FAIL test_rec_interrupted_retry_accumulates_pending_steps: retry did not apply Workflow Phase then interrupt (exit=$SECOND_STATUS)" >&2
      cat "$TMPDIR/rec-record-second.err" >&2
      exit 1
    fi
    python3 - "$TMPDIR" <<'PY'
import json, pathlib, sys
root=pathlib.Path(sys.argv[1])
journal=json.loads(pathlib.Path((root / "current-journal-path.md").read_text()).read_bytes())
binding=json.loads((root / "recovery-binding.json").read_text())
rows={row.get("action_id"):row for row in journal["actions"]}
prefix="record-pr-step:"+binding["publication_id"]+":"
assert rows[prefix+"issue"]["state"] == "intent", rows[prefix+"issue"]
assert rows[prefix+"project:Workflow Phase"]["state"] == "intent", rows[prefix+"project:Workflow Phase"]
assert (root / "project-live-phase").read_text().strip() == "verification", "Workflow Phase effect was not applied before interruption"
PY
    set +e
    python3 "$TMPDIR/scripts/pm/github-project-task.py" record-pr "$TMPDIR" \
      --repo eng-cc/oasis7 --project-owner eng-cc --project-number 1 \
      --task-uid "$TASK_UID" --pr-url "https://github.com/eng-cc/oasis7/pull/2001" \
      --draft-candidate --publication-binding-json "$TMPDIR/recovery-binding.json" --json \
      >"$TMPDIR/rec-record.json" 2>"$TMPDIR/rec-record.err"
    REC_STATUS=$?
  elif [[ "$REC_CASE" == "guard_issue_body_proof_after_write" ]]; then
    cp "$GH_ISSUE_BODY_STATE_FILE" "$TMPDIR/issue-body-before-record-pr.txt"
    GH_INTERRUPT_AFTER_ISSUE_EDIT=1 bash -c 'export GH_INTERRUPT_TARGET=$$; exec python3 "$@"' bash \
      "$TMPDIR/scripts/pm/github-project-task.py" record-pr "$TMPDIR" \
      --repo eng-cc/oasis7 --project-owner eng-cc --project-number 1 \
      --task-uid "$TASK_UID" --pr-url "https://github.com/eng-cc/oasis7/pull/2001" \
      --draft-candidate --publication-binding-json "$TMPDIR/recovery-binding.json" --json \
      >"$TMPDIR/rec-record-first.json" 2>"$TMPDIR/rec-record-first.err"
    FIRST_STATUS=$?
    set -e
    if [[ "$FIRST_STATUS" != "143" ]]; then
      echo "FAIL test_rec_guard_issue_body_proof_after_write: first Issue write did not interrupt after applying (exit=$FIRST_STATUS)" >&2
      cat "$TMPDIR/rec-record-first.err" >&2
      exit 1
    fi
    cp "$GH_ISSUE_BODY_STATE_FILE" "$TMPDIR/issue-body-target-after-first.txt"
    python3 - "$TMPDIR/issue-live-body.md" <<'PY'
import pathlib, sys
path=pathlib.Path(sys.argv[1])
path.write_bytes(path.read_bytes() + b"\n<!-- unowned Issue content changed after the recorded Issue effect -->\n")
PY
    cp "$(cat "$TMPDIR/current-journal-path.md")" "$TMPDIR/issue-body-journal-before-retry.json"
    cp "$TMPDIR/.pm/github-project-sync/tasks.json" "$TMPDIR/issue-body-mapping-before-retry.json"
    cp "$GH_ISSUE_BODY_STATE_FILE" "$TMPDIR/issue-body-before-retry.txt"
    RETRY_CALLS_BEFORE="$(wc -l < "$GH_CALL_LOG")"
    set +e
    python3 "$TMPDIR/scripts/pm/github-project-task.py" record-pr "$TMPDIR" \
      --repo eng-cc/oasis7 --project-owner eng-cc --project-number 1 \
      --task-uid "$TASK_UID" --pr-url "https://github.com/eng-cc/oasis7/pull/2001" \
      --draft-candidate --publication-binding-json "$TMPDIR/recovery-binding.json" --json \
      >"$TMPDIR/rec-record-retry.json" 2>"$TMPDIR/rec-record-retry.err"
    RETRY_STATUS=$?
    set -e
    tail -n +$((RETRY_CALLS_BEFORE + 1)) "$GH_CALL_LOG" >"$TMPDIR/issue-body-retry-calls.log"
    if [[ "$RETRY_STATUS" == "0" ]]; then
      echo "FAIL test_rec_guard_issue_body_proof_after_write: changed non-owned Issue body was accepted after an interrupted Issue write" >&2
      cat "$TMPDIR/rec-record-retry.err" >&2
      exit 1
    fi
    python3 - "$TMPDIR" <<'PY'
import hashlib, json, pathlib, sys
root=pathlib.Path(sys.argv[1])
journal_path=pathlib.Path((root / "current-journal-path.md").read_text())
journal=json.loads(journal_path.read_bytes())
binding=json.loads((root / "recovery-binding.json").read_text())
action_id="record-pr-step:"+binding["publication_id"]+":issue"
rows=[action for action in journal["actions"] if action.get("action_id") == action_id]
assert len(rows) == 1 and rows[0]["state"] == "intent", rows
proof=rows[0]["expected"].get("issue_body_proof")
assert isinstance(proof, dict), "Issue step intent lacks durable full-body proof before the Issue effect"
assert set(proof) == {"schema", "before_body", "before_body_sha256", "target_body", "target_body_sha256", "writer_default_merge_hold"}, proof
assert proof["schema"] == "oasis7-record-pr-issue-body-proof/v1", proof
before=(root / "issue-body-before-record-pr.txt").read_bytes()
target=(root / "issue-body-target-after-first.txt").read_bytes()
assert proof["before_body"].encode("utf-8") == before
assert proof["target_body"].encode("utf-8") == target
assert proof["before_body_sha256"] == hashlib.sha256(before).hexdigest()
assert proof["target_body_sha256"] == hashlib.sha256(target).hexdigest()
hold=proof["writer_default_merge_hold"]
assert isinstance(hold, dict) and set(hold) == {"kind", "active", "requester", "reason", "resume_authority", "recorded_at"}, hold
assert hold["kind"] == "normal_pr_ci_watch" and hold["active"] is False
assert isinstance(hold["recorded_at"], str) and hold["recorded_at"]
PY
    if grep -Eq '^issue (edit|comment) |^project item-edit ' "$TMPDIR/issue-body-retry-calls.log"; then
      echo "FAIL test_rec_guard_issue_body_proof_after_write: body drift retry wrote remote metadata" >&2
      cat "$TMPDIR/issue-body-retry-calls.log" >&2
      exit 1
    fi
    python3 - "$TMPDIR" <<'PY'
import pathlib, sys
root=pathlib.Path(sys.argv[1])
journal_path=pathlib.Path((root / "current-journal-path.md").read_text())
assert journal_path.read_bytes() == (root / "issue-body-journal-before-retry.json").read_bytes()
assert (root / "issue-live-body.md").read_bytes() == (root / "issue-body-before-retry.txt").read_bytes()
assert (root / ".pm/github-project-sync/tasks.json").read_bytes() == (root / "issue-body-mapping-before-retry.json").read_bytes()
PY
    echo "PASS test_rec_guard_issue_body_proof_after_write rejected non-owned body drift before retry effects"
    exit 0
  else
    python3 "$TMPDIR/scripts/pm/github-project-task.py" record-pr "$TMPDIR" \
      --repo eng-cc/oasis7 --project-owner eng-cc --project-number 1 \
      --task-uid "$TASK_UID" --pr-url "https://github.com/eng-cc/oasis7/pull/2001" \
      --draft-candidate --publication-binding-json "$TMPDIR/recovery-binding.json" --json \
      >"$TMPDIR/rec-record.json" 2>"$TMPDIR/rec-record.err"
  fi
  REC_STATUS=$?
  set -e
  tail -n +$((REC_CALLS_BEFORE + 1)) "$GH_CALL_LOG" >"$TMPDIR/rec-calls.log"
  if [[ "$REC_CASE" == "guard_api_external_wait" ]]; then
    if [[ "$REC_STATUS" != "75" ]] || ! grep -Fq '"status": "external_wait"' "$TMPDIR/rec-record.err"; then
      echo "FAIL test_rec_guard_api_external_wait: typed APIError was flattened; exit=$REC_STATUS" >&2
      cat "$TMPDIR/rec-record.err" >&2
      exit 1
    fi
    if grep -Eq '^issue (edit|comment) |^project item-edit ' "$TMPDIR/rec-calls.log"; then
      echo "FAIL test_rec_guard_api_external_wait: read uncertainty reached a metadata writer" >&2
      cat "$TMPDIR/rec-calls.log" >&2
      exit 1
    fi
    python3 - "$TMPDIR" <<'PY'
import pathlib, sys
root=pathlib.Path(sys.argv[1])
assert (root / ".pm/github-project-sync/tasks.json").read_bytes() == (root / "rec-mapping-before.json").read_bytes()
assert pathlib.Path((root / "old-journal-path.md").read_text()).read_bytes() == (root / "old-journal-before.json").read_bytes()
assert pathlib.Path((root / "current-journal-path.md").read_text()).read_bytes() == (root / "current-journal-before.json").read_bytes()
PY
    echo "PASS test_rec_guard_api_external_wait retained exit 75 before metadata writes"
    exit 0
  fi
  if [[ "$REC_CASE" == "guard_c1_timestamp_recovery_drift" ]] \
      && ! grep -Fq 'C1 publication metadata changed during recovery' "$TMPDIR/rec-record.err"; then
    echo "FAIL test_rec_guard_c1_timestamp_recovery_drift: raw C1 immutability guard was not reached" >&2
    cat "$TMPDIR/rec-record.err" >&2
    exit 1
  fi
  if [[ "$REC_CASE" == "guard_recovery_context_drift" ]] \
      && ! grep -Fq 'scope comment digest mismatch' "$TMPDIR/rec-record.err"; then
    echo "FAIL test_rec_guard_recovery_context_drift: admitted recovery scope drift was not rejected at the recovery check" >&2
    cat "$TMPDIR/rec-record.err" >&2
    exit 1
  fi
  if [[ "$REC_CASE" == "guard_c1_writer_scope_drift" ]]; then
    if ! grep -Fq 'scope comment digest mismatch' "$TMPDIR/rec-record.err"; then
      echo "FAIL test_rec_guard_c1_writer_scope_drift: C1 writer did not recheck changed recovery scope" >&2
      cat "$TMPDIR/rec-record.err" >&2
      exit 1
    fi
    python3 - "$GH_REC_AUTH_STATE_FILE.api-user-reads" "$TMPDIR" <<'PY'
import hashlib, json, pathlib, sys
reads=int(pathlib.Path(sys.argv[1]).read_text())
root=pathlib.Path(sys.argv[2]); admission=json.loads((root / "recovery-admission.json").read_text())
ref=next(item for item in admission["scope_evidence"] if item["comment_id"] == 7103)
body=(root / "gh-comments/7103").read_bytes()
assert reads >= 4, f"fixture did not reach the C1 writer's fresh recovery check: api user reads={reads}"
assert hashlib.sha256(body).hexdigest() != ref["body_sha256"], "fixture did not change the admitted scope comment"
assert b"scripts/pm/unapproved-helper.py" in body, "scope drift lacks the unauthorized path"
PY
  fi
  if [[ "$REC_CASE" == "guard_c1_writer_unknown_journal_drift" ]]; then
    if ! grep -Fq 'current publication journal differs from its bounded C1 writer postimage' "$TMPDIR/rec-record.err"; then
      echo "FAIL test_rec_guard_c1_writer_unknown_journal_drift: unknown journal postimage was not rejected by recovery authority" >&2
      cat "$TMPDIR/rec-record.err" >&2
      exit 1
    fi
    python3 - "$GH_REC_AUTH_STATE_FILE.api-user-reads" "$TMPDIR" <<'PY'
import pathlib, sys
reads=int(pathlib.Path(sys.argv[1]).read_text())
root=pathlib.Path(sys.argv[2])
actual=pathlib.Path((root / "current-journal-path.md").read_text()).read_bytes()
expected=(root / "current-journal-after-drift.json").read_bytes()
assert reads >= 4, f"fixture did not reach the C1 writer's fresh recovery check: api user reads={reads}"
assert actual == expected, "rejected check changed the externally injected journal postimage"
PY
  fi
  if [[ "$REC_CASE" == "guard_c1_writer_reordered_journal_drift" ]]; then
    python3 - "$GH_REC_AUTH_STATE_FILE.api-user-reads" "$TMPDIR" <<'PY'
import json, pathlib, sys
reads=int(pathlib.Path(sys.argv[1]).read_text())
root=pathlib.Path(sys.argv[2]); saved=root / "current-journal-after-drift.json"
assert reads >= 6, f"fixture did not reach a journal with parent and vector actions: reads={reads}"
assert saved.is_file(), "fixture did not reorder two genuine record-pr action rows"
before=json.loads((root / "current-journal-before.json").read_bytes())
drifted=json.loads(saved.read_bytes())
binding=json.loads((root / "recovery-binding.json").read_text())
prefix="record-pr:"
vector="record-pr-vector:"
publication_id=binding["publication_id"]
before_ids=[action.get("action_id") for action in before["actions"]]
drifted_ids=[action.get("action_id") for action in drifted["actions"]]
assert prefix+publication_id in before_ids and vector+publication_id in drifted_ids
assert drifted_ids.index(vector+publication_id) < drifted_ids.index(prefix+publication_id), drifted_ids
PY
    if ! grep -Fq 'current publication journal differs from its bounded C1 writer postimage' "$TMPDIR/rec-record.err"; then
      echo "FAIL test_rec_guard_c1_writer_reordered_journal_drift: recovery check did not reject reordered writer actions before another journal write (record-pr exit=$REC_STATUS)" >&2
      grep -E '^issue (edit|comment) |^project item-edit ' "$TMPDIR/rec-calls.log" >&2 || true
      cat "$TMPDIR/rec-record.err" >&2
      exit 1
    fi
    python3 - "$GH_REC_AUTH_STATE_FILE.api-user-reads" "$TMPDIR" <<'PY'
import pathlib, sys
reads=int(pathlib.Path(sys.argv[1]).read_text())
root=pathlib.Path(sys.argv[2])
saved=root / "current-journal-after-drift.json"
actual=pathlib.Path((root / "current-journal-path.md").read_text()).read_bytes()
assert actual == saved.read_bytes(), "recovery check changed the reordered journal before rejection"
PY
  fi
  if [[ "$REC_CASE" == "guard_c1_writer_step_without_vector" ]]; then
    if ! grep -Fq 'current publication journal differs from its bounded C1 writer postimage' "$TMPDIR/rec-record.err"; then
      python3 - "$GH_REC_AUTH_STATE_FILE.api-user-reads" "$TMPDIR" <<'PY' >&2
import hashlib, json, pathlib, sys
reads=int(pathlib.Path(sys.argv[1]).read_text())
root=pathlib.Path(sys.argv[2]); saved=root / "current-journal-after-drift.json"
actual=pathlib.Path((root / "current-journal-path.md").read_text()).read_bytes()
expected=saved.read_bytes()
print("step-only fixture reached read3; api_user_reads=", reads,
      "injected_journal_sha256=", hashlib.sha256(expected).hexdigest(),
      "final_journal_sha256=", hashlib.sha256(actual).hexdigest(),
      "journal_unchanged=", actual == expected)
injected=json.loads(expected); binding=json.loads((root / "recovery-binding.json").read_text())
publication_id=binding["publication_id"]
ids=[action.get("action_id") for action in injected["actions"]]
print("admitted_parent_present=", "record-pr:" + publication_id in ids,
      "vector_absent=", "record-pr-vector:" + publication_id not in ids,
      "step_present=", "record-pr-step:" + publication_id + ":issue" in ids)
PY
      echo "FAIL test_rec_guard_c1_writer_step_without_vector: recovery check accepted a transition step without its vector (record-pr exit=$REC_STATUS); metadata writes:" >&2
      grep -E '^issue (edit|comment) |^project item-edit ' "$TMPDIR/rec-calls.log" >&2 || true
      cat "$TMPDIR/rec-record.err" >&2
      exit 1
    fi
    python3 - "$GH_REC_AUTH_STATE_FILE.api-user-reads" "$TMPDIR" <<'PY'
import json, pathlib, sys
reads=int(pathlib.Path(sys.argv[1]).read_text())
root=pathlib.Path(sys.argv[2]); saved=root / "current-journal-after-drift.json"
assert reads >= 4, f"fixture did not reach a fresh C1 writer recovery check: api user reads={reads}"
journal=json.loads(saved.read_bytes())
binding=json.loads((root / "recovery-binding.json").read_text())
publication_id=binding["publication_id"]
ids=[action.get("action_id") for action in journal["actions"]]
assert "record-pr:" + publication_id in ids
assert "record-pr-vector:" + publication_id not in ids
assert "record-pr-step:" + publication_id + ":issue" in ids
actual=pathlib.Path((root / "current-journal-path.md").read_text()).read_bytes()
assert actual == saved.read_bytes(), "rejected check changed the step-only journal postimage"
PY
  fi
  if [[ "$REC_CASE" == "guard_c1_writer_unreachable_step_suffix" ]]; then
    if ! grep -Fq 'current publication journal differs from its bounded C1 writer postimage' "$TMPDIR/rec-record.err"; then
      python3 - "$GH_REC_AUTH_STATE_FILE.api-user-reads" "$TMPDIR" <<'PY' >&2
import hashlib, pathlib, sys
reads=int(pathlib.Path(sys.argv[1]).read_text())
root=pathlib.Path(sys.argv[2]); saved=root / "current-journal-after-drift.json"
actual=pathlib.Path((root / "current-journal-path.md").read_text()).read_bytes()
expected=saved.read_bytes()
print("unreachable suffix fixture reached read3; api_user_reads=",reads,
      "injected_journal_sha256=",hashlib.sha256(expected).hexdigest(),
      "final_journal_sha256=",hashlib.sha256(actual).hexdigest(),
      "journal_unchanged=",actual == expected)
PY
      echo "FAIL test_rec_guard_c1_writer_unreachable_step_suffix: writer accepted an Issue-first vector with only the later PR step intent (record-pr exit=$REC_STATUS); metadata writes:" >&2
      grep -E '^issue (edit|comment) |^project item-edit ' "$TMPDIR/rec-calls.log" >&2 || true
      cat "$TMPDIR/rec-record.err" >&2
      exit 1
    fi
    python3 - "$GH_REC_AUTH_STATE_FILE.api-user-reads" "$TMPDIR" <<'PY'
import json, pathlib, sys
reads=int(pathlib.Path(sys.argv[1]).read_text())
root=pathlib.Path(sys.argv[2]); saved=root / "current-journal-after-drift.json"
actual=pathlib.Path((root / "current-journal-path.md").read_text()).read_bytes()
binding=json.loads((root / "recovery-binding.json").read_text())
publication_id=binding["publication_id"]
journal=json.loads(saved.read_bytes())
ids=[action.get("action_id") for action in journal["actions"]]
assert reads >= 4, f"fixture did not reach a fresh C1 writer recovery check: api user reads={reads}"
assert "record-pr:" + publication_id in ids
assert "record-pr-vector:" + publication_id in ids
assert "record-pr-step:" + publication_id + ":project:PR" in ids
assert "record-pr-step:" + publication_id + ":issue" not in ids
assert "record-pr-step:" + publication_id + ":project:Workflow Phase" not in ids
assert actual == saved.read_bytes(), "rejected check changed the unreachable step suffix journal"
PY
  fi
  if [[ "$REC_CASE" == "guard_c1_writer_observed_vector_live_before" ]]; then
    python3 - "$GH_REC_AUTH_STATE_FILE.api-user-reads" "$TMPDIR" "$TMPDIR/rec-check-returns.jsonl" "$REC_STATUS" <<'PY'
import hashlib, json, pathlib, sys
reads=int(pathlib.Path(sys.argv[1]).read_text())
root=pathlib.Path(sys.argv[2]); trace=pathlib.Path(sys.argv[3])
saved=(root / "current-journal-after-drift.json").read_bytes()
actual=pathlib.Path((root / "current-journal-path.md").read_text()).read_bytes()
injected_sha=hashlib.sha256(saved).hexdigest()
events=[json.loads(line) for line in trace.read_text().splitlines()] if trace.exists() else []
at_injection=[event for event in events if event.get("journal_sha256") == injected_sha]
accepted=[event for event in at_injection if event.get("event") == "return"]
rejected=[event for event in at_injection if event.get("event") == "raise"]
assert reads >= 4, f"fixture did not reach fresh C1 writer check: {reads}"
assert not accepted, f"real PublicationRecoveryAuthority.check returned for injected postimage sha256={injected_sha}: {accepted}; events={events}"
assert rejected, f"no raised real recovery check was observed for injected postimage sha256={injected_sha}; events={events}"
assert pathlib.Path(sys.argv[3]).exists(), "real recovery check trace was not written"
if int(sys.argv[4]) == 0:
    raise AssertionError("record-pr unexpectedly accepted an invalid observed postimage")
assert actual == saved, "rejected readback mismatch changed the injected journal"
calls=(root / "rec-calls.log").read_text().splitlines()
writes=[line for line in calls if line.startswith(("issue edit ","issue comment ","project item-edit "))]
assert not writes, f"rejected readback mismatch wrote remote metadata: {writes}"
PY
  fi
  if [[ "$REC_CASE" == "guard_c1_writer_binding_comment_without_predecessors" \
      || "$REC_CASE" == "guard_c1_writer_uncertain_step_with_observation" ]]; then
    if ! grep -Fq 'current publication journal differs from its bounded C1 writer postimage' "$TMPDIR/rec-record.err"; then
      echo "FAIL test_rec_$REC_CASE: C1 writer accepted an action with an unsatisfied predecessor or impossible state (record-pr exit=$REC_STATUS)" >&2
      grep -E '^issue (edit|comment) |^project item-edit ' "$TMPDIR/rec-calls.log" >&2 || true
      cat "$TMPDIR/rec-record.err" >&2
      exit 1
    fi
    python3 - "$GH_REC_AUTH_STATE_FILE.api-user-reads" "$TMPDIR" "$REC_CASE" <<'PY'
import json, pathlib, sys
reads=int(pathlib.Path(sys.argv[1]).read_text())
root=pathlib.Path(sys.argv[2]); case=sys.argv[3]
saved=root / "current-journal-after-drift.json"
actual=pathlib.Path((root / "current-journal-path.md").read_text()).read_bytes()
journal=json.loads(saved.read_bytes())
binding=json.loads((root / "recovery-binding.json").read_text())
publication_id=binding["publication_id"]
ids=[action.get("action_id") for action in journal["actions"]]
assert reads >= 4, f"fixture did not reach a fresh C1 writer recovery check: api user reads={reads}"
assert actual == saved.read_bytes(), "rejected check changed the injected action journal"
if case == "guard_c1_writer_binding_comment_without_predecessors":
    assert "record-pr-comment:" + publication_id + ":publication-binding" in ids
    assert "record-pr-vector:" + publication_id not in ids
    assert "record-pr-comment:" + publication_id + ":lifecycle-evidence" not in ids
else:
    row=next(action for action in journal["actions"]
             if action.get("action_id") == "record-pr-step:" + publication_id + ":issue")
    assert row["state"] == "uncertain" and row.get("observed") == {
        "step":"issue","readback":{"status":"committed","workflow_phase":"verification",
            "pr_number":2001,"pr_url":"https://github.com/eng-cc/oasis7/pull/2001"}}
PY
  fi
  if [[ "$REC_CASE" == guard_project_item_content_* ]]; then
    EXPECTED_CONTENT_ERROR="selected Project item content does not match canonical Task Issue"
    if ! grep -Fq "$EXPECTED_CONTENT_ERROR" "$TMPDIR/rec-record.err"; then
      echo "FAIL test_rec_$REC_CASE: Project content guard was not reached" >&2
      cat "$TMPDIR/rec-record.err" >&2
      exit 1
    fi
    sed 's/\\ / /g' "$TMPDIR/rec-calls.log" >"$TMPDIR/rec-calls-normalized.log"
  fi
  if [[ "$REC_CASE" == "guard_project_item_content_late_drift" ]]; then
    if [[ "$REC_STATUS" == "0" ]]; then
      echo "FAIL test_rec_$REC_CASE: content drift after a Project edit was accepted" >&2
      exit 1
    fi
    PROJECT_WRITES="$(grep -Ec '^project item-edit ' "$TMPDIR/rec-calls-normalized.log" || true)"
    if [[ "$PROJECT_WRITES" != "1" ]] || grep -Eq '^issue (edit|comment) ' "$TMPDIR/rec-calls-normalized.log"; then
      echo "FAIL test_rec_$REC_CASE: unexpected metadata writes after fresh content drift" >&2
      cat "$TMPDIR/rec-calls-normalized.log" >&2
      exit 1
    fi
    python3 - "$TMPDIR" <<'PY'
import pathlib, sys
root=pathlib.Path(sys.argv[1])
assert (root / ".pm/github-project-sync/tasks.json").read_bytes() == (root / "rec-mapping-before.json").read_bytes(), "late content drift changed mapping"
old=pathlib.Path((root / "old-journal-path.md").read_text())
assert old.read_bytes() == (root / "old-journal-before.json").read_bytes(), "late content drift changed historical journal"
current=pathlib.Path((root / "current-journal-path.md").read_text())
boundary=root / "current-journal-drift-boundary.json"
assert boundary.is_file(), "late content drift was not captured at the fresh read boundary"
assert current.read_bytes() == boundary.read_bytes(), "late content drift changed the journal after the legitimate first Project write"
PY
    echo "PASS test_rec_$REC_CASE rejected after first Project edit before later metadata writes"
    exit 0
  fi
  if [[ "$REC_CASE" == "guard_repository_identity_drift" || "$REC_CASE" == "guard_repository_identity_malformed" ]]; then
    EXPECTED_REPOSITORY_ERROR="unrelated Issue/Project snapshot drift"
    if [[ "$REC_CASE" == "guard_repository_identity_malformed" ]]; then
      EXPECTED_REPOSITORY_ERROR="malformed Project Repository field"
    fi
    if ! grep -Fq "$EXPECTED_REPOSITORY_ERROR" "$TMPDIR/rec-record.err"; then
      echo "FAIL test_rec_$REC_CASE: repository guard was not reached" >&2
      cat "$TMPDIR/rec-record.err" >&2
      exit 1
    fi
  fi
  if [[ "$REC_CASE" == "guard_scope_comment_drift" ]]; then
    python3 - "$TMPDIR" <<'PY'
import hashlib, json, pathlib, sys
root=pathlib.Path(sys.argv[1]); comments=root / "gh-comments"
reads=int((root / "gh-comment-read-count.txt").read_text())
admission=json.loads((root / "recovery-admission.json").read_text())
ref=next(item for item in admission["scope_evidence"] if item["comment_id"] == 7103)
body=(comments / "7103").read_bytes()
assert reads >= 2, f"fixture did not reach a fresh comment read: {reads}"
assert hashlib.sha256(body).hexdigest() != ref["body_sha256"], "fixture did not drift bound scope comment 7103"
assert b"scripts/pm/unapproved-helper.py" in body, "fixture drift lacks the unauthorized path"
PY
  fi
  if [[ "$REC_CASE" == "guard_recovery_context_drift" ]]; then
    python3 - "$TMPDIR" <<'PY'
import hashlib, json, pathlib, sys
root=pathlib.Path(sys.argv[1]); comments=root / "gh-comments"
reads=int((root / "gh-comment-read-count.txt").read_text())
admission=json.loads((root / "recovery-admission.json").read_text())
ref=next(item for item in admission["scope_evidence"] if item["comment_id"] == 7103)
body=(comments / "7103").read_bytes()
assert reads >= 5, f"fixture did not cross recovery admission into the C1 writer: {reads}"
assert hashlib.sha256(body).hexdigest() != ref["body_sha256"], "fixture did not drift bound scope comment 7103"
assert b"scripts/pm/unapproved-helper.py" in body, "fixture drift lacks the unauthorized path"
PY
  fi
  if [[ "$REC_CASE" == guard_* ]]; then
    if [[ "$REC_STATUS" == "0" ]]; then
      echo "FAIL test_rec_$REC_CASE: invalid recovery was accepted" >&2
      exit 1
    fi
    CALLS_TO_CHECK="$TMPDIR/rec-calls.log"
    if [[ "$REC_CASE" == guard_project_item_content_* ]]; then CALLS_TO_CHECK="$TMPDIR/rec-calls-normalized.log"; fi
    if grep -Eq '^issue (edit|comment) |^project item-edit ' "$CALLS_TO_CHECK"; then
      echo "FAIL test_rec_$REC_CASE: rejected recovery wrote metadata" >&2
      cat "$CALLS_TO_CHECK" >&2
      exit 1
    fi
    python3 - "$TMPDIR" "$REC_CASE" <<'PY'
import pathlib, sys
root=pathlib.Path(sys.argv[1]); case=sys.argv[2]
assert (root / ".pm/github-project-sync/tasks.json").read_bytes() == (root / "rec-mapping-before.json").read_bytes(), "rejected recovery changed mapping"
old=pathlib.Path((root / "old-journal-path.md").read_text())
if case == "guard_missing_old_journal":
    assert not old.exists(), "rejected recovery recreated historical journal"
else:
    assert old.read_bytes() == (root / "old-journal-before.json").read_bytes(), "rejected recovery changed historical journal"
if case in {"guard_observed_payload_raw_hash_equal", "guard_binding_shape_raw_hash_equal",
            "guard_invalid_global_phase_raw_hash_equal", "guard_invalid_global_disposition_raw_hash_equal",
            "guard_scope_comment_drift", "guard_recovery_context_drift", "guard_noncanonical_observed_journal",
            "guard_c1_writer_scope_drift",
            "guard_c1_writer_unknown_journal_drift", "guard_c1_writer_reordered_journal_drift",
            "guard_c1_writer_step_without_vector", "guard_c1_writer_unreachable_step_suffix",
            "guard_c1_writer_observed_vector_live_before",
            "guard_c1_writer_binding_comment_without_predecessors",
            "guard_c1_writer_uncertain_step_with_observation",
            "guard_project_item_content_wrong", "guard_project_item_content_missing",
            "guard_project_item_content_nonissue", "guard_project_item_content_cross_repository"}:
    current=pathlib.Path((root / "current-journal-path.md").read_text())
    expected=(root / "current-journal-after-drift.json").read_bytes() if case in {"guard_c1_writer_unknown_journal_drift", "guard_c1_writer_reordered_journal_drift", "guard_c1_writer_step_without_vector", "guard_c1_writer_unreachable_step_suffix", "guard_c1_writer_observed_vector_live_before", "guard_c1_writer_binding_comment_without_predecessors", "guard_c1_writer_uncertain_step_with_observation"} else (root / "current-journal-before.json").read_bytes()
    assert current.read_bytes() == expected, "rejected recovery changed the current journal"
PY
    echo "PASS test_rec_$REC_CASE rejected before writes"
    exit 0
  fi
  if [[ "$REC_STATUS" != "0" ]]; then
    echo "FAIL test_rec_${REC_CASE}_reconciles_current_action: record-pr exit=$REC_STATUS" >&2
    cat "$TMPDIR/rec-record.err" >&2
    exit 1
  fi
  python3 - "$TMPDIR" "$TASK_UID" <<'PY'
import json, pathlib, sys
root = pathlib.Path(sys.argv[1])
r = json.loads((root / ".pm/github-project-sync/tasks.json").read_text())["tasks"][sys.argv[2]]
assert (r["status"], r["workflow_phase"], r["pr_number"]) == ("committed", "verification", 2001), r
assert pathlib.Path((root / "old-journal-path.md").read_text()).read_bytes() == (root / "old-journal-before.json").read_bytes()
issue_body = (root / "issue-live-body.md").read_text()
for field in ('- status: `committed`', '- workflow_phase: `verification`',
              '- pr_number: `2001`', '- pr_url: `https://github.com/eng-cc/oasis7/pull/2001`'):
    assert field in issue_body, (field, issue_body)
for name, expected in {"project-live-state":"committed", "project-live-status":"In Progress",
                       "project-live-phase":"verification", "project-pr.md":"https://github.com/eng-cc/oasis7/pull/2001"}.items():
    assert (root / name).read_text().strip() == expected, (name, (root / name).read_text())
import pr_projection_publication as publication
expected_binding = json.loads((root / "recovery-binding.json").read_text())
bindings = [publication.parse_publication_binding_comment(p.read_text())
            for p in (root / "gh-comments").iterdir()
            if p.name.isdecimal() and "<!-- oasis7-ci-publication-binding/v1 -->" in p.read_text()]
assert [b for b in bindings if b["publication_id"] == expected_binding["publication_id"]] == [expected_binding], bindings
# A successful CLI result must prove all remote poststates after the final
# metadata effect. Pre-write admission reads cannot confirm publication.
calls = (root / "rec-calls.log").read_text().splitlines()
last_write = max(i for i, call in enumerate(calls)
                 if call.startswith(("issue edit ", "issue comment ", "project item-edit ")))
final_reads = calls[last_write + 1:]
required = {
    "Issue": "issue view 2001 ",
    "Project": "api graphql ",
    "PR": "api repos/eng-cc/oasis7/pulls/2001",
    "unique binding": "api repos/eng-cc/oasis7/issues/2001/comments ",
}
missing = [surface for surface, prefix in required.items()
           if not any(call.startswith(prefix) for call in final_reads)]
assert not missing, ("FAIL test_rec_project_post_issue_pre_final_authoritative_readback: "
                     f"record-pr succeeded without final {missing}; final_reads={final_reads!r}")
PY
  echo "PASS test_rec_project_post_issue_pre_reconciles_current_action"
  exit 0
fi

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

# Preserve an aligned snapshot for the independent retry fixture and for the
# later lifecycle checks. The split-state case below is asserted before this
# snapshot is reused to initialize separate disposable fixtures.
PRE_SPLIT_DIR="$TMPDIR/.pm/scratch/pre-split-fixture"
mkdir -p "$PRE_SPLIT_DIR"
PRE_SPLIT_MAPPING="$PRE_SPLIT_DIR/tasks.json"
PRE_SPLIT_ISSUE_BODY="$PRE_SPLIT_DIR/issue-body.md"
PRE_SPLIT_PROJECT_STATE="$PRE_SPLIT_DIR/project-state"
PRE_SPLIT_PROJECT_STATUS="$PRE_SPLIT_DIR/project-status"
PRE_SPLIT_PROJECT_PHASE="$PRE_SPLIT_DIR/project-phase"
PRE_SPLIT_ISSUE_UPDATED_AT="$PRE_SPLIT_DIR/issue-updated-at"
PRE_SPLIT_COMMENT_LOG="$PRE_SPLIT_DIR/comment-log"
PRE_SPLIT_COMMENT_DIR="$PRE_SPLIT_DIR/comment-dir"
cp "$TMPDIR/.pm/github-project-sync/tasks.json" "$PRE_SPLIT_MAPPING"
cp "$GH_ISSUE_BODY_STATE_FILE" "$PRE_SPLIT_ISSUE_BODY"
cp "$GH_PROJECT_STATE_FILE" "$PRE_SPLIT_PROJECT_STATE"
cp "$GH_PROJECT_STATUS_STATE_FILE" "$PRE_SPLIT_PROJECT_STATUS"
cp "$GH_PROJECT_PHASE_STATE_FILE" "$PRE_SPLIT_PROJECT_PHASE"
cp "$GH_ISSUE_UPDATED_AT_FILE" "$PRE_SPLIT_ISSUE_UPDATED_AT"
cp "$GH_COMMENT_LOG" "$PRE_SPLIT_COMMENT_LOG"
cp -R "$GH_COMMENT_DIR" "$PRE_SPLIT_COMMENT_DIR"

CACHE_BEFORE_FAILURE="$(shasum -a 256 "$TMPDIR/.pm/github-project-sync/tasks.json" | awk '{print $1}')"
set +e
GH_FAIL_ISSUE_EDIT=1 PM_ROOT_DIR="$TMPDIR" "$TMPDIR/scripts/pm/task-closeout.sh" \
  --role tpm --task-uid "$TASK_UID" --verification-profile fixture_repository_state --review-packet-file "$REVIEW_PACKET" --json \
  >"$TMPDIR/failed-closeout.json" 2>"$TMPDIR/failed-closeout.err"
FAILED_CLOSEOUT_STATUS=$?
set -e
[[ "$FAILED_CLOSEOUT_STATUS" != "0" ]]
CACHE_AFTER_FAILURE="$(shasum -a 256 "$TMPDIR/.pm/github-project-sync/tasks.json" | awk '{print $1}')"
[[ "$CACHE_BEFORE_FAILURE" == "$CACHE_AFTER_FAILURE" ]]
for field in '- status: `committed`' '- workflow_phase: `verification`'; do
  if ! grep -Fq -- "$field" "$GH_ISSUE_BODY_STATE_FILE"; then
    echo "expected injected Issue edit failure to retain Issue field: $field" >&2
    cat "$GH_ISSUE_BODY_STATE_FILE" >&2
    exit 1
  fi
done
if [[ "$(cat "$GH_PROJECT_STATE_FILE")" != "ready" \
   || "$(cat "$GH_PROJECT_STATUS_STATE_FILE")" != "Ready / PR" \
   || "$(cat "$GH_PROJECT_PHASE_STATE_FILE")" != "pre_pr_ready" ]]; then
  echo "expected partial remote Project state to be ready/pre_pr_ready before refresh" >&2
  cat "$TMPDIR/failed-closeout.err" >&2
  cat "$GH_CALL_LOG" >&2
  exit 1
fi
GRAPHQL_BEFORE="$(grep -c 'api graphql' "$GH_CALL_LOG" || true)"
PM_ROOT_DIR="$TMPDIR" "$TMPDIR/scripts/pm/refresh-task-cache.sh" \
  --task-uid "$TASK_UID" --json >"$TMPDIR/refreshed-after-partial.json"
GRAPHQL_AFTER="$(grep -c 'api graphql' "$GH_CALL_LOG" || true)"
[[ $((GRAPHQL_AFTER - GRAPHQL_BEFORE)) == 1 ]]
python3 - "$TMPDIR/.pm/github-project-sync/tasks.json" "$TASK_UID" <<'PY'
import json, sys
record=json.load(open(sys.argv[1],encoding="utf-8"))["tasks"][sys.argv[2]]
assert record["status"] == "ready", record
assert record["workflow_phase"] == "pre_pr_ready", record
assert record["project_status"] == "Ready / PR", record
PY
for field in '- status: `committed`' '- workflow_phase: `verification`'; do
  if ! grep -Fq -- "$field" "$GH_ISSUE_BODY_STATE_FILE"; then
    echo "expected authoritative Issue body to retain field after refresh: $field" >&2
    cat "$GH_ISSUE_BODY_STATE_FILE" >&2
    exit 1
  fi
done
set +e
PM_ROOT_DIR="$TMPDIR" "$TMPDIR/scripts/pm/github-project-workflow.sh" \
  --json audit --task-uid "$TASK_UID" >"$TMPDIR/audit-after-refresh.json" 2>"$TMPDIR/audit-after-refresh.err"
PARTIAL_AUDIT_STATUS=$?
set -e
# Refresh reconciles the Project projection but does not rewrite Issue authority.
# The selected-task audit must expose that remaining split brain before retry.
if [[ "$PARTIAL_AUDIT_STATUS" == "0" ]]; then
  echo "github-project-task.test: expected audit to reject Project-only closeout recovery" >&2
  exit 1
fi
python3 - "$TMPDIR/audit-after-refresh.json" "$TASK_UID" <<'PY'
import json, pathlib, sys
result=json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
uid=sys.argv[2]
expected=f"{uid}: cached workflow_phase drift; refresh explicitly from authoritative GitHub issue"
assert result.get("status") == "failed", result
assert result.get("errors") == [expected], result
selected=result.get("selected_task")
assert selected and selected.get("target") == "ready", selected
assert selected.get("workflow_phase") == "pre_pr_ready", selected
PY
# Restore the original committed projection with the ordinary supported
# `move-task` writer. This reconciles the Issue, Project, and cache without
# inventing Issue content or bypassing the selected-task audit.
PM_ROOT_DIR="$TMPDIR" python3 "$TMPDIR/github-project-task.py" move-task "$TMPDIR" \
  --repo eng-cc/oasis7 --project-owner eng-cc --project-number 1 \
  --task-uid "$TASK_UID" --to-status committed --json >"$TMPDIR/compensated-after-partial.json"
python3 - "$TMPDIR/.pm/github-project-sync/tasks.json" "$TASK_UID" "$GH_ISSUE_BODY_STATE_FILE" "$GH_PROJECT_PHASE_STATE_FILE" "$GH_PROJECT_STATE_FILE" "$GH_PROJECT_STATUS_STATE_FILE" <<'PY'
import json, pathlib, sys
record=json.load(open(sys.argv[1],encoding="utf-8"))["tasks"][sys.argv[2]]
issue=pathlib.Path(sys.argv[3]).read_text(encoding="utf-8")
phase=pathlib.Path(sys.argv[4]).read_text(encoding="utf-8").strip()
pm_status=pathlib.Path(sys.argv[5]).read_text(encoding="utf-8").strip()
status=pathlib.Path(sys.argv[6]).read_text(encoding="utf-8").strip()
assert (record["status"], record["workflow_phase"]) == ("committed", "execution"), record
assert "- status: `committed`" in issue and "- workflow_phase: `execution`" in issue, issue
assert phase == "execution" and pm_status == "committed" and status == "In Progress", (phase, pm_status, status)
PY
PM_ROOT_DIR="$TMPDIR" "$TMPDIR/scripts/pm/github-project-workflow.sh" \
  --json audit --task-uid "$TASK_UID" >"$TMPDIR/audit-after-compensation.json"
python3 - "$TMPDIR/audit-after-compensation.json" <<'PY'
import json, sys
result=json.load(open(sys.argv[1],encoding="utf-8"))
assert result["status"] == "ok" and result["errors"] == [], result
PY
# Bind the SIGTERM immutability check to the refreshed local-cache baseline;
# the authoritative audit above confirms that the ordinary compensation has
# restored matching Issue/Project/cache state before closeout is retried.
CACHE_BEFORE_INTERRUPT="$(shasum -a 256 "$TMPDIR/.pm/github-project-sync/tasks.json" | awk '{print $1}')"

set +e
GH_INTERRUPT_AFTER_ISSUE_EDIT=1 PM_ROOT_DIR="$TMPDIR" /bin/bash -c \
  'export GH_INTERRUPT_TARGET=$$; exec "$@"' bash "$TMPDIR/scripts/pm/task-closeout.sh" \
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
GRAPHQL_BEFORE_INTERRUPT_REFRESH="$(grep -c 'api graphql' "$GH_CALL_LOG" || true)"
PM_ROOT_DIR="$TMPDIR" "$TMPDIR/scripts/pm/refresh-task-cache.sh" \
  --task-uid "$TASK_UID" --json >"$TMPDIR/refreshed-after-interrupt.json"
GRAPHQL_AFTER_INTERRUPT_REFRESH="$(grep -c 'api graphql' "$GH_CALL_LOG" || true)"
[[ $((GRAPHQL_AFTER_INTERRUPT_REFRESH - GRAPHQL_BEFORE_INTERRUPT_REFRESH)) == 1 ]]
python3 - "$TMPDIR/.pm/github-project-sync/tasks.json" "$TASK_UID" "$GH_ISSUE_BODY_STATE_FILE" "$GH_PROJECT_PHASE_STATE_FILE" "$GH_PROJECT_STATE_FILE" "$GH_PROJECT_STATUS_STATE_FILE" <<'PY'
import json, pathlib, sys
record=json.load(open(sys.argv[1],encoding="utf-8"))["tasks"][sys.argv[2]]
issue=pathlib.Path(sys.argv[3]).read_text(encoding="utf-8")
phase=pathlib.Path(sys.argv[4]).read_text(encoding="utf-8").strip()
pm_status=pathlib.Path(sys.argv[5]).read_text(encoding="utf-8").strip()
status=pathlib.Path(sys.argv[6]).read_text(encoding="utf-8").strip()
assert (record["status"], record["workflow_phase"]) == ("ready", "pre_pr_ready"), record
assert "- status: `ready`" in issue and "- workflow_phase: `pre_pr_ready`" in issue, issue
assert phase == "pre_pr_ready" and pm_status == "ready" and status == "Ready / PR", (phase, pm_status, status)
PY
PM_ROOT_DIR="$TMPDIR" "$TMPDIR/scripts/pm/github-project-workflow.sh" \
  --json audit --task-uid "$TASK_UID" >"$TMPDIR/audit-after-interrupt-refresh.json"
python3 - "$TMPDIR/audit-after-interrupt-refresh.json" <<'PY'
import json, sys
result=json.load(open(sys.argv[1],encoding="utf-8"))
assert result["status"] == "ok" and result["errors"] == [], result
PY
# The failed Issue edit above remains a complete negative case: refresh sees
# the Project-ahead / Issue-behind split and the selected audit rejects it.
# Exercise response-loss retry separately with a second synthetic task and an
# isolated root whose Issue, Project, and cache begin aligned.
(
  set -euo pipefail
  ALIGNED_ROOT="$TMPDIR/worktree/aligned-prepr-retry-root"
  ALIGNED_UID="task_88888888888888888888888888888888"
  mkdir -p "$TMPDIR/worktree"
  git -C "$TMPDIR" worktree add -b aligned-prepr-retry "$ALIGNED_ROOT" HEAD >/dev/null 2>&1
  ALIGNED_DATA="$ALIGNED_ROOT/.pm/scratch/aligned-prepr-fixture"
  mkdir -p "$ALIGNED_DATA" "$ALIGNED_ROOT/.pm/github-project-sync"

  cp "$PRE_SPLIT_MAPPING" "$ALIGNED_ROOT/.pm/github-project-sync/tasks.json"
  ALIGNED_BRANCH="$(git -C "$ALIGNED_ROOT" branch --show-current)"
  python3 - "$ALIGNED_ROOT/.pm/github-project-sync/tasks.json" "$ALIGNED_UID" "$ALIGNED_ROOT" "$ALIGNED_BRANCH" <<'PY'
import copy, json, pathlib, sys
path, uid, root, branch = pathlib.Path(sys.argv[1]), sys.argv[2], pathlib.Path(sys.argv[3]), sys.argv[4]
mapping = json.loads(path.read_text(encoding="utf-8"))
source = next(iter(mapping["tasks"].values()))
record = copy.deepcopy(source)
record["task_uid"] = uid
record["title"] = "Aligned applied-write interruption fixture"
record["worktree_hint"] = str(root.resolve())
record["canonical_worktree"] = str(root.resolve())
record["task_branch"] = branch
record["status"] = "committed"
record["workflow_phase"] = "verification"
for key in (
    "pr_url", "pr_number", "completion_mode", "non_pr_completion_evidence",
    "non_pr_completion_evidence_sha256", "non_pr_completion_evidence_file",
    "last_closed_at", "last_evidence_at", "last_claim_verification_at",
    "claim_verifications", "evidence_comments",
):
    record.pop(key, None)
mapping["tasks"] = {uid: record}
path.write_text(json.dumps(mapping) + "\n", encoding="utf-8")
PY

  export GH_MAPPING_PATH="$ALIGNED_ROOT/.pm/github-project-sync/tasks.json"
  export GH_PROJECT_STATE_FILE="$ALIGNED_DATA/project-live-state"
  export GH_PROJECT_STATUS_STATE_FILE="$ALIGNED_DATA/project-live-status"
  export GH_PROJECT_PHASE_STATE_FILE="$ALIGNED_DATA/project-live-phase"
  export GH_PROJECT_PR_STATE_FILE="$ALIGNED_DATA/project-live-pr"
  export GH_ISSUE_BODY_STATE_FILE="$ALIGNED_DATA/issue-live-body.md"
  export GH_ISSUE_UPDATED_AT_FILE="$ALIGNED_DATA/issue-updated-at"
  export GH_CALL_LOG="$ALIGNED_DATA/gh-calls.log"
  export GH_COMMENT_LOG="$ALIGNED_DATA/gh-comments.log"
  export GH_COMMENT_DIR="$ALIGNED_DATA/gh-comments"
  export GH_EDIT_BODY_LOG="$ALIGNED_DATA/issue-body-edited.md"
  export GH_CANONICAL_WORKTREE_HINT="$ALIGNED_ROOT"
  unset GH_REC_AUTH_STATE_FILE GH_REC_PROJECT_ITEM_CONTENT_FILE GH_REC_FAIL_FINAL_ISSUE OASIS7_REC_CASE || true
  printf 'committed\n' >"$GH_PROJECT_STATE_FILE"
  printf 'In Progress\n' >"$GH_PROJECT_STATUS_STATE_FILE"
  printf 'verification\n' >"$GH_PROJECT_PHASE_STATE_FILE"
  : >"$GH_PROJECT_PR_STATE_FILE"
  : >"$GH_CALL_LOG"
  : >"$GH_COMMENT_LOG"
  mkdir -p "$GH_COMMENT_DIR"
  printf '2026-10-01T00:00:00Z\n' >"$GH_ISSUE_UPDATED_AT_FILE"
  python3 - "$TMPDIR/github-project-task.py" "$GH_MAPPING_PATH" "$ALIGNED_UID" "$GH_ISSUE_BODY_STATE_FILE" <<'PY'
import importlib.util, json, pathlib, sys
script, mapping_path, uid, issue_path = sys.argv[1:]
spec = importlib.util.spec_from_file_location("aligned_fixture_task_helper", script)
assert spec and spec.loader
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
mapping = json.loads(pathlib.Path(mapping_path).read_text(encoding="utf-8"))
record = mapping["tasks"][uid]
pathlib.Path(issue_path).write_text(
    module.issue_body(module.task_from_record(uid, record)), encoding="utf-8"
)
PY

  ALIGNED_HEAD="$(git -C "$ALIGNED_ROOT" rev-parse HEAD)"
  ALIGNED_LEDGER_DIR="$ALIGNED_ROOT/.pm/scratch/$ALIGNED_UID"
  ALIGNED_REVIEW_PACKET="$ALIGNED_DATA/review-packet.md"
  mkdir -p "$ALIGNED_LEDGER_DIR" "$ALIGNED_ROOT/.pm/scratch/$ALIGNED_UID/review-plans"
  printf 'fixture return\n' >"$ALIGNED_LEDGER_DIR/return.md"
  ALIGNED_RETURN_SHA="$(shasum -a 256 "$ALIGNED_LEDGER_DIR/return.md" | awk '{print $1}')"
  printf '{"receipt_type":"oasis7_subagent_dispatch","issuer":"codex_runtime","dispatch_id":"22222222-2222-4222-8222-222222222222","role":"repository_health_engineer","source_head":"%s","contract_digest":"%064d"}\n' "$ALIGNED_HEAD" 0 >"$ALIGNED_LEDGER_DIR/dispatch.json"
  printf '{"task_uid":"%s","role":"repository_health_engineer","status":"completed","head":"%s","slice_id":"22222222-2222-4222-8222-222222222222","dispatch_receipt":".pm/scratch/%s/dispatch.json","activation":"message-assigned","context_delivery":"full-history","actual_runtime":"inherited/unverified: fixture","artifact_digest":"%s","scope_verdict":"approved","risk_verdict":"approved","findings":"no_findings","residual_risk":"fixture","artifacts":[".pm/scratch/%s/return.md"]}\n' "$ALIGNED_UID" "$ALIGNED_HEAD" "$ALIGNED_UID" "$ALIGNED_RETURN_SHA" "$ALIGNED_UID" >"$ALIGNED_LEDGER_DIR/slice-ledger.jsonl"
  ALIGNED_REVIEW_PLAN="$ALIGNED_ROOT/.pm/scratch/$ALIGNED_UID/review-plans/fixture.json"
  python3 - "$ALIGNED_REVIEW_PLAN" "$ALIGNED_HEAD" "$ALIGNED_UID" <<'PY'
import json, sys
path, head, uid = sys.argv[1:]
json.dump({
  "schema": "oasis7-review-plan/v1", "task_uid": uid, "frozen_head": head,
  "comparison_ref": "HEAD", "comparison_oid": head,
  "relevant_evidence_digest": "a" * 64,
  "roles": ["repository_health_engineer"],
  "expected_slices": [{"role": "repository_health_engineer", "slice_id": "22222222-2222-4222-8222-222222222222"}],
  "epoch": "b" * 64, "batch_path": ".pm/scratch/%s/review-batches/%s.json" % (uid, "b" * 64),
  "preflight": {"status": "incomplete", "ledger_path": ".pm/scratch/%s/slice-ledger.jsonl" % uid},
}, open(path, "w"))
PY
  printf -- '- Pre-PR Local Role Review: passed\n- Source Head: %s\n- Review Plan: .pm/scratch/%s/review-plans/fixture.json\n- Review Roles: stale_compatibility_role\n- Slice Ledger: n/a; derived from Review Plan\n' "$ALIGNED_HEAD" "$ALIGNED_UID" >"$ALIGNED_REVIEW_PACKET"

  for field in '- status: `committed`' '- workflow_phase: `verification`'; do
    grep -Fq -- "$field" "$GH_ISSUE_BODY_STATE_FILE"
  done
  [[ "$(cat "$GH_PROJECT_STATE_FILE")" == "committed" ]]
  [[ "$(cat "$GH_PROJECT_STATUS_STATE_FILE")" == "In Progress" ]]
  [[ "$(cat "$GH_PROJECT_PHASE_STATE_FILE")" == "verification" ]]
  ALIGNED_CACHE_BEFORE="$(shasum -a 256 "$GH_MAPPING_PATH" | awk '{print $1}')"
  set +e
  GH_APPLY_ISSUE_EDIT_THEN_INTERRUPT=1 PM_ROOT_DIR="$ALIGNED_ROOT" /bin/bash -c \
    'export GH_INTERRUPT_TARGET=$$; exec "$@"' bash "$TMPDIR/scripts/pm/task-closeout.sh" \
    --role tpm --task-uid "$ALIGNED_UID" --verification-profile fixture_repository_state --review-packet-file "$ALIGNED_REVIEW_PACKET" --json \
    >"$ALIGNED_DATA/interrupted-closeout.json" 2>"$ALIGNED_DATA/interrupted-closeout.err"
  ALIGNED_INTERRUPT_STATUS=$?
  set -e
  [[ "$ALIGNED_INTERRUPT_STATUS" != "0" ]]
  ALIGNED_CACHE_AFTER="$(shasum -a 256 "$GH_MAPPING_PATH" | awk '{print $1}')"
  [[ "$ALIGNED_CACHE_BEFORE" == "$ALIGNED_CACHE_AFTER" ]]
  if [[ "$(cat "$GH_PROJECT_STATE_FILE")" != "ready" ]]; then
    echo "github-project-task.test: independent applied-write fixture failed before Project transition" >&2
    cat "$ALIGNED_DATA/interrupted-closeout.err" >&2
    cat "$GH_CALL_LOG" >&2
    exit 1
  fi
  [[ "$(cat "$GH_PROJECT_STATE_FILE")" == "ready" ]]
  [[ "$(cat "$GH_PROJECT_STATUS_STATE_FILE")" == "Ready / PR" ]]
  [[ "$(cat "$GH_PROJECT_PHASE_STATE_FILE")" == "pre_pr_ready" ]]
  cp "$GH_ISSUE_BODY_STATE_FILE" "$ALIGNED_DATA/issue-after-applied-interruption.md"
  python3 - "$GH_ISSUE_BODY_STATE_FILE" "$ALIGNED_UID" <<'PY'
import pathlib, sys
body = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")
assert f"task_uid: {sys.argv[2]}" in body, body
assert "- status: `ready`" in body, body
assert "- workflow_phase: `pre_pr_ready`" in body, body
PY

  "$TMPDIR/bin/gh" issue view 2001 -R eng-cc/oasis7 --json body,number,title,url,state,stateReason \
    >"$ALIGNED_DATA/issue-after-interruption-readback.json"
  python3 - "$ALIGNED_DATA/issue-after-interruption-readback.json" "$ALIGNED_UID" <<'PY'
import json, pathlib, sys
readback = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
assert readback["number"] == 2001 and readback["state"] == "OPEN", readback
assert f"task_uid: {sys.argv[2]}" in readback["body"], readback
assert "- status: `ready`" in readback["body"], readback
assert "- workflow_phase: `pre_pr_ready`" in readback["body"], readback
PY

  PM_ROOT_DIR="$ALIGNED_ROOT" "$TMPDIR/scripts/pm/refresh-task-cache.sh" \
    --task-uid "$ALIGNED_UID" --json >"$ALIGNED_DATA/refreshed-after-interruption.json"
  python3 - "$GH_MAPPING_PATH" "$ALIGNED_UID" <<'PY'
import json, pathlib, sys
record=json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))["tasks"][sys.argv[2]]
assert record["status"] == "ready", record
assert record["workflow_phase"] == "pre_pr_ready", record
assert record["project_status"] == "Ready / PR", record
PY
  PM_ROOT_DIR="$ALIGNED_ROOT" "$TMPDIR/scripts/pm/github-project-workflow.sh" \
    --json audit --task-uid "$ALIGNED_UID" >"$ALIGNED_DATA/audit-after-refresh.json"
  python3 - "$ALIGNED_DATA/audit-after-refresh.json" "$ALIGNED_UID" <<'PY'
import json, pathlib, sys
audit=json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
assert audit.get("status") == "ok", audit
selected=audit.get("selected_task")
assert selected and selected.get("task_uid") == sys.argv[2], selected
assert selected.get("target") == "ready", selected
assert selected.get("workflow_phase") == "pre_pr_ready", selected
PY

  COMMENTS_BEFORE_RETRY="$(wc -l < "$GH_COMMENT_LOG" | tr -d ' ')"
  PM_ROOT_DIR="$ALIGNED_ROOT" "$TMPDIR/scripts/pm/task-closeout.sh" \
    --role tpm --task-uid "$ALIGNED_UID" --verification-profile fixture_repository_state \
    --review-packet-file "$ALIGNED_REVIEW_PACKET" --json >"$ALIGNED_DATA/retried-closeout.json"
  COMMENTS_AFTER_RETRY="$(wc -l < "$GH_COMMENT_LOG" | tr -d ' ')"
  python3 - "$ALIGNED_DATA/retried-closeout.json" "$ALIGNED_UID" "$GH_ISSUE_BODY_STATE_FILE" <<'PY'
import json, pathlib, sys
result=json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
assert result["task_uid"] == sys.argv[2], result
assert result["target_status"] == "ready" and result["final_status"] == "ready", result
body=pathlib.Path(sys.argv[3]).read_text(encoding="utf-8")
assert body.count("- status: `ready`") == 1, body
assert body.count("- workflow_phase: `pre_pr_ready`") == 1, body
PY
  printf 'observed aligned pre-PR evidence comments: after applied interruption=%s; after retry=%s\n' \
    "$COMMENTS_BEFORE_RETRY" "$COMMENTS_AFTER_RETRY"
)

# End the negative and independent retry cases. The later legacy PR lifecycle
# checks are a separate fixture scenario; initialize its cache, Issue, and
# Project together at ready/pre_pr_ready. This reset is not a recovery path
# for the failed split task.
cp "$PRE_SPLIT_MAPPING" "$TMPDIR/.pm/github-project-sync/tasks.json"
python3 - "$TMPDIR/github-project-task.py" "$TMPDIR/.pm/github-project-sync/tasks.json" "$TASK_UID" "$GH_ISSUE_BODY_STATE_FILE" <<'PY'
import hashlib, importlib.util, json, pathlib, sys
script, mapping_path, uid, issue_path = sys.argv[1:]
spec = importlib.util.spec_from_file_location("fresh_legacy_lifecycle_fixture", script)
assert spec and spec.loader
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
mapping_file = pathlib.Path(mapping_path)
mapping = json.loads(mapping_file.read_text(encoding="utf-8"))
record = mapping["tasks"][uid]
record["status"] = "ready"
record["workflow_phase"] = "pre_pr_ready"
record["completion_mode"] = "non_pr_task"
record["non_pr_completion_evidence"] = "persisted fixture completion truth"
record["non_pr_completion_evidence_sha256"] = hashlib.sha256(
    (record["non_pr_completion_evidence"] + "\n").encode("utf-8")
).hexdigest()
evidence_path = pathlib.Path(record["non_pr_completion_evidence_file"])
evidence_path.write_text(record["non_pr_completion_evidence"] + "\n", encoding="utf-8")
for key in ("pr_url", "pr_number"):
    record.pop(key, None)
mapping_file.write_text(json.dumps(mapping) + "\n", encoding="utf-8")
pathlib.Path(issue_path).write_text(
    module.issue_body(module.task_from_record(uid, record)), encoding="utf-8"
)
PY
LEGACY_READY_INITIAL_BODY="$TMPDIR/legacy-ready-initial-issue.md"
cp "$GH_ISSUE_BODY_STATE_FILE" "$LEGACY_READY_INITIAL_BODY"
printf 'ready\n' >"$GH_PROJECT_STATE_FILE"
printf 'Ready / PR\n' >"$GH_PROJECT_STATUS_STATE_FILE"
printf 'pre_pr_ready\n' >"$GH_PROJECT_PHASE_STATE_FILE"
if [[ -n "${GH_PROJECT_PR_STATE_FILE:-}" ]]; then : >"$GH_PROJECT_PR_STATE_FILE"; fi
cp "$PRE_SPLIT_ISSUE_UPDATED_AT" "$GH_ISSUE_UPDATED_AT_FILE"
cp "$PRE_SPLIT_COMMENT_LOG" "$GH_COMMENT_LOG"
rm -rf "$GH_COMMENT_DIR"
cp -R "$PRE_SPLIT_COMMENT_DIR" "$GH_COMMENT_DIR"

python3 - "$GH_ISSUE_BODY_STATE_FILE" "$GH_PROJECT_PHASE_STATE_FILE" "$GH_PROJECT_STATUS_STATE_FILE" <<'PY'
import pathlib, sys
issue=pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")
phase=pathlib.Path(sys.argv[2]).read_text(encoding="utf-8").strip()
status=pathlib.Path(sys.argv[3]).read_text(encoding="utf-8").strip()
assert "- status: `ready`" in issue and "- workflow_phase: `pre_pr_ready`" in issue, issue
assert phase == "pre_pr_ready", phase
assert status == "Ready / PR", status
PY
PM_ROOT_DIR="$TMPDIR" "$TMPDIR/scripts/pm/github-project-workflow.sh" \
  --json audit --task-uid "$TASK_UID" >"$TMPDIR/audit-after-retry.json"
python3 - "$TMPDIR/audit-after-retry.json" <<'PY'
import json, sys
result=json.load(open(sys.argv[1],encoding="utf-8"))
assert result["status"] == "ok" and result["errors"] == [], result
PY

python3 "$TMPDIR/github-project-task.py" record-pr "$TMPDIR" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --task-uid "$TASK_UID" \
  --pr-url "https://github.com/eng-cc/oasis7/pull/2001" \
  --json > "$TMPDIR/record-pr.json"

python3 - "$TMPDIR/.pm/github-project-sync/tasks.json" "$TASK_UID" "$GH_CALL_LOG" <<'PY'
import json, pathlib, sys
record=json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))["tasks"][sys.argv[2]]
calls=pathlib.Path(sys.argv[3]).read_text(encoding="utf-8")
assert record["status"] == "pr_watch", record
assert record["workflow_phase"] == "pr_watch", record
assert "OPT_PR_WATCH_PHASE" in calls, calls
PY

python3 - "$TMPDIR/.pm/github-project-sync/tasks.json" "$TASK_UID" "$GH_ISSUE_BODY_STATE_FILE" <<'PY'
import base64, hashlib, json, pathlib, sys
record=json.load(open(sys.argv[1],encoding='utf-8'))['tasks'][sys.argv[2]]
body=pathlib.Path(sys.argv[3]).read_text(encoding='utf-8')
evidence=record.get('non_pr_completion_evidence')
assert record.get('completion_mode') == 'non_pr_task', record
assert evidence == 'persisted fixture completion truth', record
encoded=base64.urlsafe_b64encode(evidence.encode('utf-8')).decode('ascii').rstrip('=')
assert f'- non_pr_completion_evidence_b64: `{encoded}`' in body, body
digest=hashlib.sha256((evidence+'\n').encode('utf-8')).hexdigest()
assert record.get('non_pr_completion_evidence_sha256') == digest, record
assert f'- non_pr_completion_evidence_sha256: `{digest}`' in body, body
PY

set +e
PM_ROOT_DIR="$TMPDIR" "$TMPDIR/scripts/pm/task-closeout.sh" \
  --role tpm \
  --task-uid "$TASK_UID" \
  --to-status done \
  --verification-profile repository_required \
  --claim-type task_complete \
  --json > "$TMPDIR/done-closeout.json" 2> "$TMPDIR/done-closeout.err"
DONE_CLOSEOUT_STATUS=$?
set -e
if [[ "$DONE_CLOSEOUT_STATUS" != "0" ]]; then
  set +e
  PM_ROOT_DIR="$TMPDIR" "$TMPDIR/scripts/pm/github-project-workflow.sh" \
    --json audit --task-uid "$TASK_UID" \
    > "$TMPDIR/done-closeout-transition-audit.json" 2> "$TMPDIR/done-closeout-transition-audit.err"
  set -e
  echo "github-project-task.test: independent lifecycle done-closeout failed; selected audit follows" >&2
  cat "$TMPDIR/done-closeout.err" >&2
  cat "$TMPDIR/done-closeout-transition-audit.json" >&2
  cat "$TMPDIR/done-closeout-transition-audit.err" >&2
  exit 1
fi

python3 - "$TMPDIR/.pm/github-project-sync/tasks.json" "$TASK_UID" "$GH_CALL_LOG" "$GH_COMMENT_LOG" "$TMPDIR/issue-body-edited.md" "$LEGACY_READY_INITIAL_BODY" <<'PY'
import json, pathlib, sys
mapping = json.loads(pathlib.Path(sys.argv[1]).read_text())
uid = sys.argv[2]
calls = pathlib.Path(sys.argv[3]).read_text()
comments = pathlib.Path(sys.argv[4]).read_text().splitlines()
edited_body = pathlib.Path(sys.argv[5]).read_text()
initial_body = pathlib.Path(sys.argv[6]).read_text()
record = mapping["tasks"][uid]
assert record["issue_url"] == "https://github.com/eng-cc/oasis7/issues/2001", record
assert record["project_item_id"] == "ITEM_ID", record
assert record["status"] == "done", record
assert record["pr_url"] == "https://github.com/eng-cc/oasis7/pull/2001", record
assert record["pr_number"] == 2001, record
assert record["worktree_hint"] == str(pathlib.Path(sys.argv[1]).parents[2].resolve()), record
assert len(comments) >= 7, comments
assert record["claim_verifications"][-1]["claim_type"] == "task_complete", record
assert record["claim_verifications"][-1]["status"] == "verified", record
assert record["claim_verifications"][-1]["verification_profile"] == "repository_required", record
assert record["claim_verifications"][-1]["verify_command"] == "true", record
assert record["claim_verifications"][-1]["repository_head"] == record["claim_verifications"][-1]["frozen_source_head"], record
assert "api repos/eng-cc/oasis7/issues/2001/comments --paginate --slurp" in calls, calls
assert "issue create" in calls, calls
assert "issue edit 2001" in calls, calls
assert "issue close 2001" not in calls, calls
assert f"task_uid: {uid}" in edited_body, edited_body
assert f"task_uid: {uid}" in initial_body, initial_body
assert "- status: `ready`" in initial_body, initial_body
assert "- workflow_phase: `pre_pr_ready`" in initial_body, initial_body
assert "- status: `committed`" in edited_body, edited_body
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
grep -Fq "canonical task-closeout.sh" "$TMPDIR/no-cache-move.err" || {
  cat "$TMPDIR/no-cache-move.err" >&2
  exit 1
}

NO_CACHE_RECORD_CALLS_BEFORE="$(wc -l < "$GH_CALL_LOG")"
set +e
python3 "$TMPDIR/github-project-task.py" record-pr "$NO_CACHE_ROOT" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --task-uid "$NO_CACHE_UID" \
  --pr-url "https://github.com/eng-cc/oasis7/pull/2003" \
  --json > "$TMPDIR/no-cache-record-pr.json" 2> "$TMPDIR/no-cache-record-pr.err"
NO_CACHE_RECORD_PR_STATUS=$?
set -e
[[ "$NO_CACHE_RECORD_PR_STATUS" != "0" ]]
grep -Fq "record-pr: cached task repository identity is missing or mismatched" "$TMPDIR/no-cache-record-pr.err"
[[ ! -e "$NO_CACHE_ROOT/.pm/github-project-sync/tasks.json" ]]
tail -n +$((NO_CACHE_RECORD_CALLS_BEFORE + 1)) "$GH_CALL_LOG" > "$TMPDIR/no-cache-record-pr-calls.log"
if grep -Eq 'issue (edit|comment)|project item-edit|api repos/eng-cc/oasis7/pulls/' "$TMPDIR/no-cache-record-pr-calls.log"; then
  echo "github-project-task.test: record-pr must not mutate Issue/Project/PR state without recoverable canonical task identity" >&2
  cat "$TMPDIR/no-cache-record-pr-calls.log" >&2
  exit 1
fi

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
      "workflow_phase": "pr_watch",
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

export GH_MAPPING_PATH="$PARTIAL_ROOT/.pm/github-project-sync/tasks.json"
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
export GH_MAPPING_PATH="$PRIMARY_GH_MAPPING_PATH"

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
      "workflow_phase": "pr_watch",
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

export GH_MAPPING_PATH="$NOOP_PROJECT_ROOT/.pm/github-project-sync/tasks.json"
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
export GH_MAPPING_PATH="$PRIMARY_GH_MAPPING_PATH"
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
      "workflow_phase": "pr_watch",
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

export GH_MAPPING_PATH="$MISSING_OPTION_ROOT/.pm/github-project-sync/tasks.json"
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
# fine terminal sequencing remains in the local mapping and receipts. Generate
# the claim from claim-ready so this path exercises the same canonical claim and
# exact live Issue-comment readback as production closeout.
PM_ROOT_DIR="$MISSING_OPTION_ROOT" "$ROOT_DIR/scripts/pm/claim-ready.sh" \
  --task-uid "$MISSING_OPTION_UID" \
  --verification-profile repository_required \
  --claim-type task_complete \
  --json > "$TMPDIR/missing-option-claim.json"
CLOSEOUT_CLAIM="$(cat "$TMPDIR/missing-option-claim.json")"
cp "$GH_ISSUE_UPDATED_AT_FILE_2006" "$TMPDIR/missing-option-claim-issue-updated-at.txt"
if ! python3 "$TMPDIR/github-project-task.py" closeout-task "$MISSING_OPTION_ROOT" \
  --repo eng-cc/oasis7 --project-owner eng-cc --project-number 3 \
  --task-uid "$MISSING_OPTION_UID" --role tpm --to-status done \
  --claim-json "$CLOSEOUT_CLAIM" --json >"$TMPDIR/missing-option-closeout.json" 2>"$TMPDIR/missing-option-closeout.err"; then
  echo "github-project-task.test: remedial task_done closeout must map the coarse Project done option and advance" >&2
  cat "$TMPDIR/missing-option-closeout.err" >&2
  exit 1
fi
export GH_MAPPING_PATH="$PRIMARY_GH_MAPPING_PATH"
python3 - "$MISSING_OPTION_ROOT/.pm/github-project-sync/tasks.json" "$MISSING_OPTION_UID" "$TMPDIR/missing-option-closeout.json" "$TMPDIR/missing-option-claim.json" "$GH_COMMENT_DIR" "$TMPDIR/missing-option-claim-issue-updated-at.txt" <<'PY'
import json,pathlib,re,sys
record=json.load(open(sys.argv[1],encoding="utf-8"))["tasks"][sys.argv[2]]
payload=json.load(open(sys.argv[3],encoding="utf-8"))
claim=json.load(open(sys.argv[4],encoding="utf-8"))
comment_dir=pathlib.Path(sys.argv[5])
issue_updated_at=pathlib.Path(sys.argv[6]).read_text(encoding="utf-8").strip()
assert record["status"] == "done" and record["workflow_phase"] == "task_done", record
assert payload["updated_field_values"] == 3, payload
assert record["claim_verifications"][-1] == claim, (record["claim_verifications"][-1], claim)
assert claim["claim_type"] == "task_complete" and claim["status"] == "verified", claim
assert claim["verification_profile"] == "repository_required", claim
assert claim["verify_command"] == "true", claim
assert claim["verification_mode"] == "detached_frozen_tree", claim
assert claim["repository_head"] == claim["frozen_source_head"], claim
assert re.fullmatch(r"[0-9a-f]{40,64}", claim["frozen_source_head"]), claim
assert re.fullmatch(r"[0-9a-f]{64}", claim["repository_fingerprint_before"]), claim
assert claim["repository_fingerprint_before"] == claim["repository_fingerprint_after"], claim
comments = [json.loads(path.read_text(encoding="utf-8"))
            for path in sorted(comment_dir.glob("comment-*.json"))]
expected_body = "\n".join((
    "<!-- oasis7-pm-claim-verification -->",
    f"Task UID: {sys.argv[2]}",
    f"Claim Type: {claim['claim_type']}",
    f"Verified At: {claim['verified_at']}",
    f"Verification Exit Code: {claim['verification_exit_code']}",
    f"Verification Status: {claim['status']}",
    f"Verify Command: {claim['verify_command']}",
    f"Claim Message: {claim['claim_message']}",
    "",
))
matches = [comment for comment in comments
           if comment.get("issue_url") == "https://api.github.com/repos/eng-cc/oasis7/issues/2006"
           and comment.get("body") == expected_body]
assert len(matches) == 1, matches
comment = matches[0]
assert type(comment["id"]) is int and comment["id"] > 0, comment
assert comment["html_url"] == f"https://github.com/eng-cc/oasis7/issues/2006#issuecomment-{comment['id']}", comment
assert comment["created_at"] == comment["updated_at"] == claim["verified_at"] == issue_updated_at, comment
PY
if ! grep -Fq 'api repos/eng-cc/oasis7/issues/2006/comments --paginate --slurp' "$GH_CALL_LOG"; then
  echo "github-project-task.test: remedial closeout must read back the exact Issue claim comment" >&2
  cat "$GH_CALL_LOG" >&2
  exit 1
fi

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

sys.path.insert(0, str(pathlib.Path(sys.argv[1]).parent))
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
    module.project_refresh_graphql = lambda _query, _variables, **_kwargs: {"data": {"nodes": [project_node]}}
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
import importlib.util, pathlib, sys
sys.path.insert(0, str(pathlib.Path(sys.argv[1]).parent))
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
