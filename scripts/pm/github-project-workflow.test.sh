#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${PM_ROOT_DIR:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

TMPDIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

mkdir -p "$TMPDIR/.pm/tasks" "$TMPDIR/.pm/github-project-sync" "$TMPDIR/bin"
cp "$ROOT_DIR/scripts/pm/github-project-workflow.py" "$TMPDIR/github-project-workflow.py"
cp "$ROOT_DIR/scripts/pm/github-project-sync.py" "$TMPDIR/github-project-sync.py"

cat > "$TMPDIR/.pm/tasks/task_11111111111111111111111111111111.yaml" <<'YAML'
task_uid: task_11111111111111111111111111111111
title: "active task"
owner_role: tpm
module: engineering
worktree_hint: /tmp/worktree
execution_log_path: .pm/tasks/task_11111111111111111111111111111111.execution.md
status: committed
priority: P2
source_signal: null
source_refs: []
doc_refs: []
related_prd: []
acceptance: []
handoff_to: []
updated_at: 2026-06-29T00:00:00+08:00
YAML

cat > "$TMPDIR/.pm/github-project-sync/tasks.json" <<'JSON'
{
  "version": 1,
  "project": {
    "id": "PROJECT_ID",
    "number": 1,
    "owner": "eng-cc",
    "repo": "eng-cc/oasis7"
  },
  "tasks": {
    "task_11111111111111111111111111111111": {
      "task_uid": "task_11111111111111111111111111111111",
      "issue_url": "https://github.com/eng-cc/oasis7/issues/101",
      "issue_number": 101,
      "project_item_id": "ITEM_ID",
      "title": "active task",
      "status": "committed",
      "priority": "P2",
      "module": "engineering",
      "owner_role": "tpm",
      "worktree_hint": "/tmp/worktree"
    }
  }
}
JSON

cat > "$TMPDIR/bin/gh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
case "$*" in
  api\ graphql*)
    if [[ "$*" == *"rateLimit"* ]]; then
      printf '{"data":{"rateLimit":{"remaining":5000,"resetAt":"2099-01-01T00:00:00Z"}}}\n'
      exit 0
    fi
    project_id="PROJECT_ID"
    if [[ "${GH_FAKE_WRONG_PROJECT:-0}" == "1" ]]; then
      project_id="OTHER_PROJECT_ID"
    fi
    page_info='"pageInfo":{"hasNextPage":false},'
    case "${GH_FAKE_PAGE_INFO_MODE:-complete}" in
      missing) page_info='' ;;
      null) page_info='"pageInfo":null,' ;;
      invalid) page_info='"pageInfo":{"hasNextPage":"false"},' ;;
      truncated) page_info='"pageInfo":{"hasNextPage":true},' ;;
      complete) ;;
      *) echo "unexpected GH_FAKE_PAGE_INFO_MODE" >&2; exit 9 ;;
    esac
    if [[ "${GH_FAKE_DRIFT:-0}" == "1" ]]; then
      printf '{"data":{"nodes":[{"id":"ITEM_ID","project":{"id":"%s","number":1,"owner":{"login":"%s"}},"content":{"body":"task_uid: task_11111111111111111111111111111111","number":101,"url":"https://github.com/eng-cc/oasis7/issues/101"},"fieldValues":{%s"nodes":[{"name":"In Progress","field":{"name":"Status"}},{"text":"task_11111111111111111111111111111111","field":{"name":"Task UID"}},{"name":"tpm","field":{"name":"Owner Role"}},{"name":"engineering","field":{"name":"Module"}},{"name":"blocked","field":{"name":"PM Status"}},{"name":"blocked","field":{"name":"Workflow Phase"}},{"name":"P2","field":{"name":"Priority"}},{"text":"/tmp/worktree","field":{"name":"Canonical Worktree"}},{"name":"n/a","field":{"name":"Test Tier Required"}}]}}]}}\n' "$project_id" "${GH_FAKE_PROJECT_OWNER:-eng-cc}" "$page_info"
    else
      printf '{"data":{"nodes":[{"id":"ITEM_ID","project":{"id":"%s","number":1,"owner":{"login":"%s"}},"content":{"body":"task_uid: task_11111111111111111111111111111111","number":101,"url":"https://github.com/eng-cc/oasis7/issues/101"},"fieldValues":{%s"nodes":[{"name":"In Progress","field":{"name":"Status"}},{"text":"task_11111111111111111111111111111111","field":{"name":"Task UID"}},{"name":"tpm","field":{"name":"Owner Role"}},{"name":"engineering","field":{"name":"Module"}},{"name":"committed","field":{"name":"PM Status"}},{"name":"execution","field":{"name":"Workflow Phase"}},{"name":"P2","field":{"name":"Priority"}},{"text":"/tmp/worktree","field":{"name":"Canonical Worktree"}},{"name":"n/a","field":{"name":"Test Tier Required"}}]}}]}}\n' "$project_id" "${GH_FAKE_PROJECT_OWNER:-eng-cc}" "$page_info"
    fi
    ;;
  *)
    echo "unexpected gh invocation: $*" >&2
    exit 9
    ;;
esac
SH
chmod +x "$TMPDIR/bin/gh"
export PATH="$TMPDIR/bin:$PATH"

python3 - "$TMPDIR/github-project-workflow.py" <<'PY'
import importlib.util, sys
spec = importlib.util.spec_from_file_location("workflow", sys.argv[1])
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
assert module.normalized_acceptance("task_uid: task_x\n") == []
assert module.normalized_acceptance("Acceptance:\n\nNext:\n") == []
assert module.normalized_acceptance("Acceptance:\n- [ ] first\n- [x] second\n") == ["first", "second"]
assert module.normalized_acceptance("Acceptance:\n- first\n- second\n") == ["first", "second"]
draft = {"status": "committed", "workflow_phase": "verification"}
assert module.expected_project_values(draft)["Workflow Phase"] == "verification"
ordinary = {"status": "committed", "workflow_phase": "execution"}
assert module.expected_project_values(ordinary)["Workflow Phase"] == "execution"
legacy_trace = module.normalized_issue_traceability(
    "- completion_mode: `non_pr_task`\n"
    "- non_pr_completion_evidence_b64: `bGVnYWN5IGV2aWRlbmNl`\n"
)
assert legacy_trace["non_pr_completion_evidence"] == "legacy evidence", legacy_trace
assert "non_pr_completion_evidence_sha256" not in legacy_trace, legacy_trace
PY

AUDIT_JSON="$TMPDIR/audit.json"
set +e
python3 "$TMPDIR/github-project-workflow.py" "$TMPDIR" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --mapping "$TMPDIR/.pm/github-project-sync/tasks.json" \
  --json \
  audit --global-maintenance > "$AUDIT_JSON"
RETIRED_EXIT=$?
set -e
[[ "$RETIRED_EXIT" == "1" ]]

python3 - "$AUDIT_JSON" <<'PY'
import json, pathlib, sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert payload["status"] == "failed", payload
assert any("retired .pm/tasks files present" in item for item in payload["errors"]), payload
PY

rm -rf "$TMPDIR/.pm/tasks"

python3 "$TMPDIR/github-project-workflow.py" "$TMPDIR" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --mapping "$TMPDIR/.pm/github-project-sync/tasks.json" \
  --json \
  audit --global-maintenance > "$AUDIT_JSON"

python3 - "$AUDIT_JSON" <<'PY'
import json, pathlib, sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert payload["status"] == "ok", payload
assert payload["selected_count"] == 1, payload
assert payload["errors"] == [], payload
PY

DRIFT_JSON="$TMPDIR/drift.json"
set +e
GH_FAKE_DRIFT=1 python3 "$TMPDIR/github-project-workflow.py" "$TMPDIR" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --mapping "$TMPDIR/.pm/github-project-sync/tasks.json" \
  --json \
  audit --global-maintenance > "$DRIFT_JSON"
DRIFT_EXIT=$?
set -e
[[ "$DRIFT_EXIT" == "1" ]]

python3 - "$DRIFT_JSON" <<'PY'
import json, pathlib, sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert payload["status"] == "failed", payload
assert any("PM Status" in item for item in payload["errors"]), payload
PY

WRONG_PROJECT_JSON="$TMPDIR/wrong-project.json"
set +e
GH_FAKE_WRONG_PROJECT=1 python3 "$TMPDIR/github-project-workflow.py" "$TMPDIR" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --mapping "$TMPDIR/.pm/github-project-sync/tasks.json" \
  --json \
  audit --global-maintenance > "$WRONG_PROJECT_JSON"
WRONG_PROJECT_EXIT=$?
set -e
[[ "$WRONG_PROJECT_EXIT" == "1" ]]

python3 - "$WRONG_PROJECT_JSON" <<'PY'
import json, pathlib, sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert payload["status"] == "failed", payload
assert any("project_id" in item for item in payload["errors"]), payload
PY

OWNER_DRIFT_JSON="$TMPDIR/owner-drift.json"
set +e
GH_FAKE_PROJECT_OWNER=foreign-owner python3 "$TMPDIR/github-project-workflow.py" "$TMPDIR" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --mapping "$TMPDIR/.pm/github-project-sync/tasks.json" \
  --json \
  audit --global-maintenance > "$OWNER_DRIFT_JSON"
OWNER_DRIFT_EXIT=$?
set -e
[[ "$OWNER_DRIFT_EXIT" == "1" ]]

python3 - "$OWNER_DRIFT_JSON" <<'PY'
import json, pathlib, sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert payload["status"] == "failed", payload
assert any("project_owner" in item for item in payload["errors"]), payload
PY

for page_info_mode in missing null invalid truncated; do
  PAGE_INFO_JSON="$TMPDIR/page-info-$page_info_mode.json"
  set +e
  GH_FAKE_PAGE_INFO_MODE="$page_info_mode" \
    python3 "$TMPDIR/github-project-workflow.py" "$TMPDIR" \
      --repo eng-cc/oasis7 \
      --project-owner eng-cc \
      --project-number 1 \
      --mapping "$TMPDIR/.pm/github-project-sync/tasks.json" \
      --json \
      audit --global-maintenance > "$PAGE_INFO_JSON"
  PAGE_INFO_EXIT=$?
  set -e
  [[ "$PAGE_INFO_EXIT" == "1" ]]
  python3 - "$PAGE_INFO_JSON" <<'PY'
import json, pathlib, sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert payload["status"] == "failed", payload
assert any(
    "pagination is incomplete or unknown" in item
    for item in payload["errors"]
), payload
PY
done

cat > "$TMPDIR/bin/gh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
case "$*" in
  api\ graphql*)
    printf '{"data":{"rateLimit":{"remaining":5000,"resetAt":"2099-01-01T00:00:00Z"}}}\n'
    ;;
  "project item-list 1 --owner eng-cc --limit 1000 --format json")
    printf '{"totalCount":1,"items":[{"id":"ITEM_ID","content":{"body":"task_uid: task_11111111111111111111111111111111","number":101,"url":"https://github.com/eng-cc/oasis7/issues/101"},"status":"In Progress","task UID":"task_11111111111111111111111111111111","owner Role":"tpm","module":"engineering","pM Status":"committed","workflow Phase":"execution","priority":"P2","canonical Worktree":"/tmp/worktree","test Tier Required":"n/a"}]}\n'
    ;;
  *)
    echo "unexpected gh invocation: $*" >&2
    exit 9
    ;;
esac
SH
chmod +x "$TMPDIR/bin/gh"

STEP3_JSON="$TMPDIR/step3.json"
python3 "$TMPDIR/github-project-workflow.py" "$TMPDIR" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --mapping "$TMPDIR/.pm/github-project-sync/tasks.json" \
  --json \
  step3-gate > "$STEP3_JSON"

python3 - "$STEP3_JSON" <<'PY'
import json, pathlib, sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert payload["status"] == "ok", payload
assert payload["selected_count"] == 1, payload
assert payload["project_item_count"] == 1, payload
assert payload["errors"] == [], payload
PY

MAPPING_ONLY="$TMPDIR/mapping-only"
mkdir -p "$MAPPING_ONLY/.pm/github-project-sync"
cat > "$MAPPING_ONLY/.pm/github-project-sync/tasks.json" <<'JSON'
{
  "version": 1,
  "project": {
    "id": "PROJECT_ID",
    "number": 1,
    "owner": "eng-cc",
    "repo": "eng-cc/oasis7"
  },
  "tasks": {
    "task_33333333333333333333333333333333": {
      "task_uid": "task_33333333333333333333333333333333",
      "issue_url": "https://github.com/eng-cc/oasis7/issues/303",
      "issue_number": 303,
      "project_item_id": "MAPPING_ITEM_ID",
      "title": "mapping only active task",
      "status": "committed",
      "priority": "P2",
      "module": "engineering",
      "owner_role": "tpm",
      "worktree_hint": "/tmp/mapping-worktree"
    }
  }
}
JSON
cat > "$TMPDIR/bin/gh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
case "$*" in
  api\ graphql*)
    if [[ "$*" == *"rateLimit"* ]]; then printf '{"data":{"rateLimit":{"remaining":5000,"resetAt":"2099-01-01T00:00:00Z"}}}\n'; exit 0; fi
    if [[ "${GH_FAKE_METADATA_DRIFT:-0}" == "1" ]]; then
      printf '{"data":{"nodes":[{"id":"MAPPING_ITEM_ID","project":{"id":"PROJECT_ID","number":1,"owner":{"login":"eng-cc"}},"content":{"title":"[PM] authoritative changed title","body":"task_uid: task_33333333333333333333333333333333\\nAcceptance:\\n- authoritative acceptance\\n","number":303,"url":"https://github.com/eng-cc/oasis7/issues/303"},"fieldValues":{"pageInfo":{"hasNextPage":false},"nodes":[{"name":"In Progress","field":{"name":"Status"}},{"text":"task_33333333333333333333333333333333","field":{"name":"Task UID"}},{"name":"tpm","field":{"name":"Owner Role"}},{"name":"engineering","field":{"name":"Module"}},{"name":"committed","field":{"name":"PM Status"}},{"name":"execution","field":{"name":"Workflow Phase"}},{"name":"P2","field":{"name":"Priority"}},{"text":"/tmp/mapping-worktree","field":{"name":"Canonical Worktree"}},{"name":"n/a","field":{"name":"Test Tier Required"}}]}}]}}\n'
    elif [[ "${GH_FAKE_CANONICAL_EVIDENCE:-0}" == "1" || "${GH_FAKE_TRACE_DRIFT:-0}" == "1" || "${GH_FAKE_TRACE_CONTENT_DRIFT:-0}" == "1" || "${GH_FAKE_TRACE_DIGEST_DRIFT:-0}" == "1" || "${GH_FAKE_WORKFLOW_PHASE_DRIFT:-0}" == "1" ]]; then
      python3 - <<'PY'
import base64
import hashlib
import json
import os
uid = "task_33333333333333333333333333333333"
canonical_evidence_probe = os.environ.get("GH_FAKE_CANONICAL_EVIDENCE") == "1"
content_drift = os.environ.get("GH_FAKE_TRACE_CONTENT_DRIFT") == "1"
digest_drift = os.environ.get("GH_FAKE_TRACE_DIGEST_DRIFT") == "1"
workflow_phase_drift = os.environ.get("GH_FAKE_WORKFLOW_PHASE_DRIFT") == "1"
cached_evidence = "cached evidence"
if canonical_evidence_probe:
    evidence = "canonical evidence"
    evidence_digest = hashlib.sha256((evidence + "\n").encode()).hexdigest()
elif content_drift:
    evidence = "tampered evidence"
    evidence_digest = hashlib.sha256((cached_evidence + "\n").encode()).hexdigest()
elif digest_drift:
    evidence = cached_evidence
    evidence_digest = "0" * 64
elif workflow_phase_drift:
    evidence = cached_evidence
    evidence_digest = hashlib.sha256((cached_evidence + "\n").encode()).hexdigest()
else:
    evidence = "live evidence"
    evidence_digest = "98694058bf71ded2899ac6011b78f767907a9baea31a618ec1dc66facca357d0"
encoded_evidence = base64.urlsafe_b64encode(evidence.encode()).decode().rstrip("=")
workflow_phase = "verification" if workflow_phase_drift else "execution"
body = """<!-- oasis7-pm-task -->
task_uid: task_33333333333333333333333333333333

GitHub-backed oasis7 PM task.

Task metadata:
- owner_role: `tpm`
- module: `engineering`
- status: `committed`
- workflow_phase: `{workflow_phase}`
- priority: `P2`
- worktree_hint: `/tmp/mapping-worktree`
- completion_mode: `non_pr_task`
- non_pr_completion_evidence_b64: `{encoded_evidence}`
- non_pr_completion_evidence_sha256: `{evidence_digest}`

Source refs:
- `source-live`

Doc refs:
- `doc/live.md`

Related PRD:
- `doc/live.prd.md`

Acceptance:
- authoritative acceptance
""".format(workflow_phase=workflow_phase, encoded_evidence=encoded_evidence, evidence_digest=evidence_digest)
fields = [
    {"name": "In Progress", "field": {"name": "Status"}},
    {"text": uid, "field": {"name": "Task UID"}},
    {"name": "tpm", "field": {"name": "Owner Role"}},
    {"name": "engineering", "field": {"name": "Module"}},
    {"name": "committed", "field": {"name": "PM Status"}},
    {"name": "execution", "field": {"name": "Workflow Phase"}},
    {"name": "P2", "field": {"name": "Priority"}},
    {"text": "/tmp/mapping-worktree", "field": {"name": "Canonical Worktree"}},
    {"name": "n/a", "field": {"name": "Test Tier Required"}},
]
node = {"id": "MAPPING_ITEM_ID", "project": {"id": "PROJECT_ID", "number": 1, "owner": {"login": "eng-cc"}}, "content": {"title": "[PM] mapping only active task", "body": body, "number": 303, "url": "https://github.com/eng-cc/oasis7/issues/303"}, "fieldValues": {"pageInfo": {"hasNextPage": False}, "nodes": fields}}
print(json.dumps({"data": {"nodes": [node]}}))
PY
    elif [[ "${GH_FAKE_MALFORMED_TRACE:-0}" == "1" || "${GH_FAKE_INVALID_UTF8:-0}" == "1" ]]; then
      python3 - <<'PY'
import json, os
uid = "task_33333333333333333333333333333333"
encoded = "!!!" if os.environ.get("GH_FAKE_MALFORMED_TRACE") == "1" else "//4"
body = f"""<!-- oasis7-pm-task -->
task_uid: {uid}

GitHub-backed oasis7 PM task.

Task metadata:
- owner_role: `tpm`
- module: `engineering`
- status: `committed`
- workflow_phase: `execution`
- priority: `P2`
- worktree_hint: `/tmp/mapping-worktree`
- completion_mode: `non_pr_task`
- non_pr_completion_evidence_b64: `{encoded}`

Acceptance:
- authoritative acceptance
"""
fields = [
    {"name": "In Progress", "field": {"name": "Status"}},
    {"text": uid, "field": {"name": "Task UID"}},
    {"name": "tpm", "field": {"name": "Owner Role"}},
    {"name": "engineering", "field": {"name": "Module"}},
    {"name": "committed", "field": {"name": "PM Status"}},
    {"name": "execution", "field": {"name": "Workflow Phase"}},
    {"name": "P2", "field": {"name": "Priority"}},
    {"text": "/tmp/mapping-worktree", "field": {"name": "Canonical Worktree"}},
    {"name": "n/a", "field": {"name": "Test Tier Required"}},
]
node = {"id": "MAPPING_ITEM_ID", "project": {"id": "PROJECT_ID", "number": 1, "owner": {"login": "eng-cc"}}, "content": {"title": "[PM] mapping only active task", "body": body, "number": 303, "url": "https://github.com/eng-cc/oasis7/issues/303"}, "fieldValues": {"pageInfo": {"hasNextPage": False}, "nodes": fields}}
print(json.dumps({"data": {"nodes": [node]}}))
PY
    elif [[ "${GH_FAKE_MAPPING_DRIFT:-0}" == "1" ]]; then
      printf '{"data":{"nodes":[{"id":"MAPPING_ITEM_ID","project":{"id":"PROJECT_ID","number":1,"owner":{"login":"eng-cc"}},"content":{"body":"task_uid: task_33333333333333333333333333333333","number":303,"url":"https://github.com/eng-cc/oasis7/issues/303"},"fieldValues":{"pageInfo":{"hasNextPage":false},"nodes":[{"name":"In Progress","field":{"name":"Status"}},{"text":"task_33333333333333333333333333333333","field":{"name":"Task UID"}},{"name":"tpm","field":{"name":"Owner Role"}},{"name":"engineering","field":{"name":"Module"}},{"name":"blocked","field":{"name":"PM Status"}},{"name":"blocked","field":{"name":"Workflow Phase"}},{"name":"P2","field":{"name":"Priority"}},{"text":"/tmp/mapping-worktree","field":{"name":"Canonical Worktree"}},{"name":"n/a","field":{"name":"Test Tier Required"}}]}}]}}\n'
    else
      printf '{"data":{"nodes":[{"id":"MAPPING_ITEM_ID","project":{"id":"PROJECT_ID","number":1,"owner":{"login":"eng-cc"}},"content":{"body":"task_uid: task_33333333333333333333333333333333","number":303,"url":"https://github.com/eng-cc/oasis7/issues/303"},"fieldValues":{"pageInfo":{"hasNextPage":false},"nodes":[{"name":"In Progress","field":{"name":"Status"}},{"text":"task_33333333333333333333333333333333","field":{"name":"Task UID"}},{"name":"tpm","field":{"name":"Owner Role"}},{"name":"engineering","field":{"name":"Module"}},{"name":"committed","field":{"name":"PM Status"}},{"name":"execution","field":{"name":"Workflow Phase"}},{"name":"P2","field":{"name":"Priority"}},{"text":"/tmp/mapping-worktree","field":{"name":"Canonical Worktree"}},{"name":"n/a","field":{"name":"Test Tier Required"}}]}}]}}\n'
    fi
    ;;
  *)
    echo "unexpected gh invocation: $*" >&2
    exit 9
    ;;
esac
SH
chmod +x "$TMPDIR/bin/gh"

MAPPING_ONLY_JSON="$TMPDIR/mapping-only.json"
MAPPING_BEFORE_SHA="$(shasum -a 256 "$MAPPING_ONLY/.pm/github-project-sync/tasks.json" | awk '{print $1}')"
python3 "$TMPDIR/github-project-workflow.py" "$MAPPING_ONLY" \
  --mapping "$MAPPING_ONLY/.pm/github-project-sync/tasks.json" \
  --json \
  audit --global-maintenance > "$MAPPING_ONLY_JSON"
MAPPING_AFTER_SHA="$(shasum -a 256 "$MAPPING_ONLY/.pm/github-project-sync/tasks.json" | awk '{print $1}')"
[[ "$MAPPING_BEFORE_SHA" == "$MAPPING_AFTER_SHA" ]]

python3 - "$MAPPING_ONLY_JSON" <<'PY'
import json, pathlib, sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert payload["status"] == "ok", payload
assert payload["selected_count"] == 1, payload
assert payload["errors"] == [], payload
assert payload["project_owner"] == "eng-cc", payload
assert payload["project_number"] == 1, payload
PY

METADATA_DRIFT_JSON="$TMPDIR/metadata-drift.json"
set +e
GH_FAKE_METADATA_DRIFT=1 python3 "$TMPDIR/github-project-workflow.py" "$MAPPING_ONLY" \
  --mapping "$MAPPING_ONLY/.pm/github-project-sync/tasks.json" \
  --json audit --global-maintenance > "$METADATA_DRIFT_JSON"
METADATA_DRIFT_EXIT=$?
set -e
[[ "$METADATA_DRIFT_EXIT" == "1" ]]
python3 - "$METADATA_DRIFT_JSON" <<'PY'
import json, pathlib, sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert any("cached title drift" in item for item in payload["errors"]), payload
assert any("cached acceptance drift" in item for item in payload["errors"]), payload
PY

# Optional Project fields cannot clear Issue-authoritative traceability. When
# the live Issue carries those fields, selected-task audit must detect every
# authoritative mismatch instead of treating the Project projection as a
# substitute.
python3 - "$MAPPING_ONLY/.pm/github-project-sync/tasks.json" <<'PY'
import json, pathlib, sys
path = pathlib.Path(sys.argv[1])
payload = json.loads(path.read_text(encoding="utf-8"))
record = payload["tasks"]["task_33333333333333333333333333333333"]
record.update({
    "doc_refs": ["doc/cached.md"],
    "related_prd": ["doc/cached.prd.md"],
    "completion_mode": "pr_task",
    "non_pr_completion_evidence_sha256": "a" * 64,
    "acceptance": ["authoritative acceptance"],
})
path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
PY
TRACE_DRIFT_JSON="$TMPDIR/traceability-drift.json"
set +e
GH_FAKE_TRACE_DRIFT=1 python3 "$TMPDIR/github-project-workflow.py" "$MAPPING_ONLY" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --mapping "$MAPPING_ONLY/.pm/github-project-sync/tasks.json" \
  --json audit --task-uid task_33333333333333333333333333333333 > "$TRACE_DRIFT_JSON"
TRACE_DRIFT_EXIT=$?
set -e
[[ "$TRACE_DRIFT_EXIT" == "1" ]]
python3 - "$TRACE_DRIFT_JSON" <<'PY'
import json, pathlib, sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert payload["status"] == "failed", payload
for marker in ("doc_refs", "related_prd", "completion_mode", "non_pr_completion_evidence_sha256"):
    assert any(marker in item for item in payload["errors"]), (marker, payload)
PY

python3 - "$MAPPING_ONLY/.pm/github-project-sync/tasks.json" <<'PY'
import hashlib
import json
import pathlib
import sys
path = pathlib.Path(sys.argv[1])
payload = json.loads(path.read_text(encoding="utf-8"))
record = payload["tasks"]["task_33333333333333333333333333333333"]
record.update({
    "workflow_phase": "execution",
    "completion_mode": "non_pr_task",
    "non_pr_completion_evidence": "cached evidence",
    "non_pr_completion_evidence_sha256": hashlib.sha256(b"cached evidence\n").hexdigest(),
    "doc_refs": ["doc/live.md"],
    "related_prd": ["doc/live.prd.md"],
})
path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
PY

REGRESSION_FAILURES=0
TRACE_CONTENT_DRIFT_JSON="$TMPDIR/trace-content-drift.json"
set +e
GH_FAKE_TRACE_CONTENT_DRIFT=1 python3 "$TMPDIR/github-project-workflow.py" "$MAPPING_ONLY" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --mapping "$MAPPING_ONLY/.pm/github-project-sync/tasks.json" \
  --json audit --task-uid task_33333333333333333333333333333333 > "$TRACE_CONTENT_DRIFT_JSON"
TRACE_CONTENT_DRIFT_EXIT=$?
set -e
if [[ "$TRACE_CONTENT_DRIFT_EXIT" != "1" ]]; then
  echo "non-PR evidence content drift unexpectedly passed audit" >&2
  REGRESSION_FAILURES=1
else
  python3 - "$TRACE_CONTENT_DRIFT_JSON" <<'PY'
import json
import pathlib
import sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert any("non_pr_completion_evidence" in item for item in payload["errors"]), payload
PY
fi

TRACE_DIGEST_DRIFT_JSON="$TMPDIR/trace-digest-drift.json"
set +e
GH_FAKE_TRACE_DIGEST_DRIFT=1 python3 "$TMPDIR/github-project-workflow.py" "$MAPPING_ONLY" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --mapping "$MAPPING_ONLY/.pm/github-project-sync/tasks.json" \
  --json audit --task-uid task_33333333333333333333333333333333 > "$TRACE_DIGEST_DRIFT_JSON"
TRACE_DIGEST_DRIFT_EXIT=$?
set -e
if [[ "$TRACE_DIGEST_DRIFT_EXIT" != "1" ]]; then
  echo "non-PR evidence digest drift unexpectedly passed audit" >&2
  REGRESSION_FAILURES=1
else
  python3 - "$TRACE_DIGEST_DRIFT_JSON" <<'PY'
import json
import pathlib
import sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert any("non_pr_completion_evidence_sha256" in item for item in payload["errors"]), payload
PY
fi

TRACE_WORKFLOW_PHASE_JSON="$TMPDIR/trace-workflow-phase-drift.json"
set +e
GH_FAKE_WORKFLOW_PHASE_DRIFT=1 python3 "$TMPDIR/github-project-workflow.py" "$MAPPING_ONLY" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --mapping "$MAPPING_ONLY/.pm/github-project-sync/tasks.json" \
  --json audit --task-uid task_33333333333333333333333333333333 > "$TRACE_WORKFLOW_PHASE_JSON"
TRACE_WORKFLOW_PHASE_EXIT=$?
set -e
if [[ "$TRACE_WORKFLOW_PHASE_EXIT" != "1" ]]; then
  echo "workflow_phase drift unexpectedly passed audit" >&2
  REGRESSION_FAILURES=1
else
  python3 - "$TRACE_WORKFLOW_PHASE_JSON" <<'PY'
import json
import pathlib
import sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert any("workflow_phase" in item for item in payload["errors"]), payload
PY
fi
[[ "$REGRESSION_FAILURES" == "0" ]] || exit 1

# Malformed evidence encodings are projection loss, not an empty optional
# value. Both invalid base64 and valid base64 with invalid UTF-8 must expose
# the stable diagnostic so refresh/audit cannot silently clear evidence.
for trace_mode in malformed invalid_utf8; do
  TRACE_LOSS_JSON="$TMPDIR/traceability-loss-$trace_mode.json"
  set +e
  if [[ "$trace_mode" == "malformed" ]]; then
    GH_FAKE_MALFORMED_TRACE=1 python3 "$TMPDIR/github-project-workflow.py" "$MAPPING_ONLY" \
      --repo eng-cc/oasis7 \
      --project-owner eng-cc \
      --project-number 1 \
      --mapping "$MAPPING_ONLY/.pm/github-project-sync/tasks.json" \
      --json audit --task-uid task_33333333333333333333333333333333 > "$TRACE_LOSS_JSON"
  else
    GH_FAKE_INVALID_UTF8=1 python3 "$TMPDIR/github-project-workflow.py" "$MAPPING_ONLY" \
      --repo eng-cc/oasis7 \
      --project-owner eng-cc \
      --project-number 1 \
      --mapping "$MAPPING_ONLY/.pm/github-project-sync/tasks.json" \
      --json audit --task-uid task_33333333333333333333333333333333 > "$TRACE_LOSS_JSON"
  fi
  TRACE_LOSS_EXIT=$?
  set -e
  [[ "$TRACE_LOSS_EXIT" == "1" ]]
  python3 - "$TRACE_LOSS_JSON" <<'PY'
import json, pathlib, sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert payload["status"] == "failed", payload
assert any("trace-projection-loss" in item for item in payload["errors"]), payload
PY
done

# Identity-bound non-PR evidence must be read back from its canonical file at
# audit time. Keep the Issue/cache projections unchanged while exercising the
# positive, tampered, missing, and unsafe-path cases.
CANONICAL_EVIDENCE_FILE="$MAPPING_ONLY/.pm/scratch/task_33333333333333333333333333333333/non-pr-completion-evidence.txt"
mkdir -p "$(dirname "$CANONICAL_EVIDENCE_FILE")"
printf '%s\n' 'canonical evidence' > "$CANONICAL_EVIDENCE_FILE"
python3 - "$MAPPING_ONLY/.pm/github-project-sync/tasks.json" "$MAPPING_ONLY" "$CANONICAL_EVIDENCE_FILE" <<'PY'
import hashlib
import json
import pathlib
import sys
path = pathlib.Path(sys.argv[1])
payload = json.loads(path.read_text(encoding="utf-8"))
record = payload["tasks"]["task_33333333333333333333333333333333"]
evidence = "canonical evidence"
record.update({
    "canonical_worktree": str(pathlib.Path(sys.argv[2]).resolve()),
    "non_pr_completion_evidence": evidence,
    "non_pr_completion_evidence_file": str(pathlib.Path(sys.argv[3]).resolve()),
    "non_pr_completion_evidence_sha256": hashlib.sha256(
        (evidence + "\n").encode("utf-8")
    ).hexdigest(),
})
path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
PY

CANONICAL_OK_JSON="$TMPDIR/canonical-evidence-ok.json"
set +e
GH_FAKE_CANONICAL_EVIDENCE=1 python3 "$TMPDIR/github-project-workflow.py" "$MAPPING_ONLY" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --mapping "$MAPPING_ONLY/.pm/github-project-sync/tasks.json" \
  --json audit --task-uid task_33333333333333333333333333333333 > "$CANONICAL_OK_JSON"
CANONICAL_OK_EXIT=$?
set -e
[[ "$CANONICAL_OK_EXIT" == "0" ]]
python3 - "$CANONICAL_OK_JSON" <<'PY'
import json
import pathlib
import sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert payload["status"] == "ok", payload
assert payload["errors"] == [], payload
PY

printf '%s\n' 'tampered canonical evidence' > "$CANONICAL_EVIDENCE_FILE"
CANONICAL_TAMPER_JSON="$TMPDIR/canonical-evidence-tamper.json"
set +e
GH_FAKE_CANONICAL_EVIDENCE=1 python3 "$TMPDIR/github-project-workflow.py" "$MAPPING_ONLY" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --mapping "$MAPPING_ONLY/.pm/github-project-sync/tasks.json" \
  --json audit --task-uid task_33333333333333333333333333333333 > "$CANONICAL_TAMPER_JSON"
CANONICAL_TAMPER_EXIT=$?
set -e
[[ "$CANONICAL_TAMPER_EXIT" == "1" ]]
python3 - "$CANONICAL_TAMPER_JSON" <<'PY'
import json
import pathlib
import sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert payload["status"] == "failed", payload
assert any("canonical non_pr_completion_evidence" in item for item in payload["errors"]), payload
assert any("content disagrees" in item for item in payload["errors"]), payload
assert any("digest disagrees" in item for item in payload["errors"]), payload
PY

rm "$CANONICAL_EVIDENCE_FILE"
CANONICAL_MISSING_JSON="$TMPDIR/canonical-evidence-missing.json"
set +e
GH_FAKE_CANONICAL_EVIDENCE=1 python3 "$TMPDIR/github-project-workflow.py" "$MAPPING_ONLY" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --mapping "$MAPPING_ONLY/.pm/github-project-sync/tasks.json" \
  --json audit --task-uid task_33333333333333333333333333333333 > "$CANONICAL_MISSING_JSON"
CANONICAL_MISSING_EXIT=$?
set -e
[[ "$CANONICAL_MISSING_EXIT" == "1" ]]
python3 - "$CANONICAL_MISSING_JSON" <<'PY'
import json
import pathlib
import sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert payload["status"] == "failed", payload
assert any("canonical non_pr_completion_evidence file" in item for item in payload["errors"]), payload
PY

printf '%s\n' 'canonical evidence' > "$CANONICAL_EVIDENCE_FILE"
python3 - "$MAPPING_ONLY/.pm/github-project-sync/tasks.json" "$MAPPING_ONLY/.pm/unsafe-evidence.txt" <<'PY'
import json
import pathlib
import sys
path = pathlib.Path(sys.argv[1])
payload = json.loads(path.read_text(encoding="utf-8"))
payload["tasks"]["task_33333333333333333333333333333333"]["non_pr_completion_evidence_file"] = str(pathlib.Path(sys.argv[2]).resolve())
path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
PY
CANONICAL_UNSAFE_JSON="$TMPDIR/canonical-evidence-unsafe.json"
set +e
GH_FAKE_CANONICAL_EVIDENCE=1 python3 "$TMPDIR/github-project-workflow.py" "$MAPPING_ONLY" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --mapping "$MAPPING_ONLY/.pm/github-project-sync/tasks.json" \
  --json audit --task-uid task_33333333333333333333333333333333 > "$CANONICAL_UNSAFE_JSON"
CANONICAL_UNSAFE_EXIT=$?
set -e
[[ "$CANONICAL_UNSAFE_EXIT" == "1" ]]
python3 - "$CANONICAL_UNSAFE_JSON" <<'PY'
import json
import pathlib
import sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert payload["status"] == "failed", payload
assert any("canonical non_pr_completion_evidence path" in item for item in payload["errors"]), payload
PY

python3 - "$MAPPING_ONLY/.pm/github-project-sync/tasks.json" <<'PY'
import json, pathlib, sys
path = pathlib.Path(sys.argv[1])
payload = json.loads(path.read_text(encoding="utf-8"))
record = payload["tasks"]["task_33333333333333333333333333333333"]
record["acceptance"] = []
for key in ("doc_refs", "related_prd", "workflow_phase", "completion_mode", "canonical_worktree", "non_pr_completion_evidence", "non_pr_completion_evidence_file", "non_pr_completion_evidence_sha256"):
    record.pop(key, None)
path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
PY

TASK_AUDIT_JSON="$TMPDIR/task-audit.json"
python3 "$TMPDIR/github-project-workflow.py" "$MAPPING_ONLY" \
  --mapping "$MAPPING_ONLY/.pm/github-project-sync/tasks.json" \
  --json \
  audit --task-uid task_33333333333333333333333333333333 > "$TASK_AUDIT_JSON"

python3 - "$TASK_AUDIT_JSON" <<'PY'
import json, pathlib, sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert payload["status"] == "ok", payload
assert payload["task_uid"] == "task_33333333333333333333333333333333", payload
assert payload["selected_count"] == 1, payload
assert payload["selected_statuses"] == ["blocked", "candidate", "committed", "deferred", "done", "pr_watch", "ready"], payload
assert payload["errors"] == [], payload
PY

MAPPING_DRIFT_JSON="$TMPDIR/mapping-drift.json"
set +e
GH_FAKE_MAPPING_DRIFT=1 python3 "$TMPDIR/github-project-workflow.py" "$MAPPING_ONLY" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --mapping "$MAPPING_ONLY/.pm/github-project-sync/tasks.json" \
  --json \
  audit --global-maintenance > "$MAPPING_DRIFT_JSON"
MAPPING_DRIFT_EXIT=$?
set -e
[[ "$MAPPING_DRIFT_EXIT" == "1" ]]

python3 - "$MAPPING_DRIFT_JSON" <<'PY'
import json, pathlib, sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert payload["status"] == "failed", payload
assert any("PM Status" in item for item in payload["errors"]), payload
PY

ARCHIVE_RECOVERY="$TMPDIR/archive-recovery"
mkdir -p "$ARCHIVE_RECOVERY/.pm/github-project-sync"
cat > "$ARCHIVE_RECOVERY/.pm/github-project-sync/tasks.json" <<'JSON'
{
  "version": 1,
  "project": {
    "id": "PROJECT_ID",
    "number": 1,
    "owner": "eng-cc",
    "repo": "eng-cc/oasis7"
  },
  "tasks": {}
}
JSON
cat > "$ARCHIVE_RECOVERY/.pm/github-project-sync/task-archive.jsonl" <<'JSONL'
{"task":{"task_uid":"task_44444444444444444444444444444444","title":"archive committed task","owner_role":"tpm","module":"engineering","worktree_hint":"/tmp/archive-worktree","status":"committed","priority":"P2"},"task_path":".pm/tasks/task_44444444444444444444444444444444.yaml","execution_log_path":".pm/tasks/task_44444444444444444444444444444444.execution.md"}
JSONL
cat > "$TMPDIR/bin/gh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
case "$*" in
  "project item-list 1 --owner eng-cc --limit 1000 --format json")
    echo "unexpected full Project scan during selected audit recovery" >&2
    exit 9
    ;;
  api\ graphql*)
    if [[ "$*" == *"rateLimit"* ]]; then printf '{"data":{"rateLimit":{"remaining":5000,"resetAt":"2099-01-01T00:00:00Z"}}}\n'; exit 0; fi
    if [[ "$*" == *"ids[]=ARCHIVE_ITEM_ID"* ]]; then
      printf '{"data":{"nodes":[{"id":"ARCHIVE_ITEM_ID","project":{"id":"PROJECT_ID","number":1,"owner":{"login":"eng-cc"}},"content":{"body":"task_uid: task_44444444444444444444444444444444","number":404,"url":"https://github.com/eng-cc/oasis7/issues/404"},"fieldValues":{"pageInfo":{"hasNextPage":false},"nodes":[{"name":"In Progress","field":{"name":"Status"}},{"text":"task_44444444444444444444444444444444","field":{"name":"Task UID"}},{"name":"tpm","field":{"name":"Owner Role"}},{"name":"engineering","field":{"name":"Module"}},{"name":"committed","field":{"name":"PM Status"}},{"name":"execution","field":{"name":"Workflow Phase"}},{"name":"P2","field":{"name":"Priority"}},{"text":"/tmp/archive-worktree","field":{"name":"Canonical Worktree"}},{"name":"n/a","field":{"name":"Test Tier Required"}}]}}]}}\n'
    else
      printf '{"data":{"s0":{"nodes":[{"number":404,"body":"task_uid: task_44444444444444444444444444444444","url":"https://github.com/eng-cc/oasis7/issues/404","projectItems":{"nodes":[{"id":"WRONG_ARCHIVE_ITEM_ID","project":{"id":"OTHER_PROJECT_ID","number":1}},{"id":"ARCHIVE_ITEM_ID","project":{"id":"PROJECT_ID","number":1}}]}}]}}}\n'
    fi
    ;;
  *)
    echo "unexpected gh invocation: $*" >&2
    exit 9
    ;;
esac
SH
chmod +x "$TMPDIR/bin/gh"

ARCHIVE_RECOVERY_JSON="$TMPDIR/archive-recovery.json"
ARCHIVE_MAPPING_BEFORE_SHA="$(shasum -a 256 "$ARCHIVE_RECOVERY/.pm/github-project-sync/tasks.json" | awk '{print $1}')"
set +e
python3 "$TMPDIR/github-project-workflow.py" "$ARCHIVE_RECOVERY" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --mapping "$ARCHIVE_RECOVERY/.pm/github-project-sync/tasks.json" \
  --status committed \
  --json \
  audit --global-maintenance > "$ARCHIVE_RECOVERY_JSON"
ARCHIVE_RECOVERY_EXIT=$?
set -e
[[ "$ARCHIVE_RECOVERY_EXIT" == "1" ]]
ARCHIVE_MAPPING_AFTER_SHA="$(shasum -a 256 "$ARCHIVE_RECOVERY/.pm/github-project-sync/tasks.json" | awk '{print $1}')"
[[ "$ARCHIVE_MAPPING_BEFORE_SHA" == "$ARCHIVE_MAPPING_AFTER_SHA" ]]

python3 - "$ARCHIVE_RECOVERY_JSON" "$ARCHIVE_RECOVERY/.pm/github-project-sync/tasks.json" <<'PY'
import json, pathlib, sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
uid = "task_44444444444444444444444444444444"
assert payload["status"] == "failed", payload
assert payload["selected_count"] == 1, payload
assert any("missing mapping record" in item for item in payload["errors"]), payload
PY

echo "github-project-workflow.test: OK"
