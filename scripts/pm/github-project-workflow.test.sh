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
    if [[ "${GH_FAKE_DRIFT:-0}" == "1" ]]; then
      printf '{"data":{"nodes":[{"id":"ITEM_ID","project":{"id":"%s","number":1},"content":{"body":"task_uid: task_11111111111111111111111111111111","number":101,"url":"https://github.com/eng-cc/oasis7/issues/101"},"fieldValues":{"nodes":[{"name":"In Progress","field":{"name":"Status"}},{"text":"task_11111111111111111111111111111111","field":{"name":"Task UID"}},{"name":"tpm","field":{"name":"Owner Role"}},{"name":"engineering","field":{"name":"Module"}},{"name":"blocked","field":{"name":"PM Status"}},{"name":"blocked","field":{"name":"Workflow Phase"}},{"name":"P2","field":{"name":"Priority"}},{"text":"/tmp/worktree","field":{"name":"Canonical Worktree"}},{"name":"n/a","field":{"name":"Test Tier Required"}}]}}]}}\n' "$project_id"
    else
      printf '{"data":{"nodes":[{"id":"ITEM_ID","project":{"id":"%s","number":1},"content":{"body":"task_uid: task_11111111111111111111111111111111","number":101,"url":"https://github.com/eng-cc/oasis7/issues/101"},"fieldValues":{"nodes":[{"name":"In Progress","field":{"name":"Status"}},{"text":"task_11111111111111111111111111111111","field":{"name":"Task UID"}},{"name":"tpm","field":{"name":"Owner Role"}},{"name":"engineering","field":{"name":"Module"}},{"name":"committed","field":{"name":"PM Status"}},{"name":"execution","field":{"name":"Workflow Phase"}},{"name":"P2","field":{"name":"Priority"}},{"text":"/tmp/worktree","field":{"name":"Canonical Worktree"}},{"name":"n/a","field":{"name":"Test Tier Required"}}]}}]}}\n' "$project_id"
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
      printf '{"data":{"nodes":[{"id":"MAPPING_ITEM_ID","project":{"id":"PROJECT_ID","number":1},"content":{"title":"[PM] authoritative changed title","body":"task_uid: task_33333333333333333333333333333333\\nAcceptance:\\n- authoritative acceptance\\n","number":303,"url":"https://github.com/eng-cc/oasis7/issues/303"},"fieldValues":{"nodes":[{"name":"In Progress","field":{"name":"Status"}},{"text":"task_33333333333333333333333333333333","field":{"name":"Task UID"}},{"name":"tpm","field":{"name":"Owner Role"}},{"name":"engineering","field":{"name":"Module"}},{"name":"committed","field":{"name":"PM Status"}},{"name":"execution","field":{"name":"Workflow Phase"}},{"name":"P2","field":{"name":"Priority"}},{"text":"/tmp/mapping-worktree","field":{"name":"Canonical Worktree"}},{"name":"n/a","field":{"name":"Test Tier Required"}}]}}]}}\n'
    elif [[ "${GH_FAKE_MAPPING_DRIFT:-0}" == "1" ]]; then
      printf '{"data":{"nodes":[{"id":"MAPPING_ITEM_ID","project":{"id":"PROJECT_ID","number":1},"content":{"body":"task_uid: task_33333333333333333333333333333333","number":303,"url":"https://github.com/eng-cc/oasis7/issues/303"},"fieldValues":{"nodes":[{"name":"In Progress","field":{"name":"Status"}},{"text":"task_33333333333333333333333333333333","field":{"name":"Task UID"}},{"name":"tpm","field":{"name":"Owner Role"}},{"name":"engineering","field":{"name":"Module"}},{"name":"blocked","field":{"name":"PM Status"}},{"name":"blocked","field":{"name":"Workflow Phase"}},{"name":"P2","field":{"name":"Priority"}},{"text":"/tmp/mapping-worktree","field":{"name":"Canonical Worktree"}},{"name":"n/a","field":{"name":"Test Tier Required"}}]}}]}}\n'
    else
      printf '{"data":{"nodes":[{"id":"MAPPING_ITEM_ID","project":{"id":"PROJECT_ID","number":1},"content":{"body":"task_uid: task_33333333333333333333333333333333","number":303,"url":"https://github.com/eng-cc/oasis7/issues/303"},"fieldValues":{"nodes":[{"name":"In Progress","field":{"name":"Status"}},{"text":"task_33333333333333333333333333333333","field":{"name":"Task UID"}},{"name":"tpm","field":{"name":"Owner Role"}},{"name":"engineering","field":{"name":"Module"}},{"name":"committed","field":{"name":"PM Status"}},{"name":"execution","field":{"name":"Workflow Phase"}},{"name":"P2","field":{"name":"Priority"}},{"text":"/tmp/mapping-worktree","field":{"name":"Canonical Worktree"}},{"name":"n/a","field":{"name":"Test Tier Required"}}]}}]}}\n'
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
      printf '{"data":{"nodes":[{"id":"ARCHIVE_ITEM_ID","project":{"id":"PROJECT_ID","number":1},"content":{"body":"task_uid: task_44444444444444444444444444444444","number":404,"url":"https://github.com/eng-cc/oasis7/issues/404"},"fieldValues":{"nodes":[{"name":"In Progress","field":{"name":"Status"}},{"text":"task_44444444444444444444444444444444","field":{"name":"Task UID"}},{"name":"tpm","field":{"name":"Owner Role"}},{"name":"engineering","field":{"name":"Module"}},{"name":"committed","field":{"name":"PM Status"}},{"name":"execution","field":{"name":"Workflow Phase"}},{"name":"P2","field":{"name":"Priority"}},{"text":"/tmp/archive-worktree","field":{"name":"Canonical Worktree"}},{"name":"n/a","field":{"name":"Test Tier Required"}}]}}]}}\n'
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
