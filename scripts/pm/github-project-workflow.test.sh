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
  "tasks": {
    "task_11111111111111111111111111111111": {
      "task_uid": "task_11111111111111111111111111111111",
      "issue_url": "https://github.com/eng-cc/oasis7/issues/101",
      "issue_number": 101,
      "project_item_id": "ITEM_ID"
    }
  }
}
JSON

cat > "$TMPDIR/bin/gh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
case "$*" in
  "project item-list 1 --owner eng-cc --limit 1000 --format json")
    if [[ "${GH_FAKE_DRIFT:-0}" == "1" ]]; then
      printf '{"totalCount":1,"items":[{"id":"ITEM_ID","content":{"body":"task_uid: task_11111111111111111111111111111111","number":101,"url":"https://github.com/eng-cc/oasis7/issues/101"},"status":"In Progress","task UID":"task_11111111111111111111111111111111","owner Role":"tpm","module":"engineering","pM Status":"blocked","workflow Phase":"blocked","priority":"P2","canonical Worktree":"/tmp/worktree","test Tier Required":"n/a"}]}\n'
    else
      printf '{"totalCount":1,"items":[{"id":"ITEM_ID","content":{"body":"task_uid: task_11111111111111111111111111111111","number":101,"url":"https://github.com/eng-cc/oasis7/issues/101"},"status":"In Progress","task UID":"task_11111111111111111111111111111111","owner Role":"tpm","module":"engineering","pM Status":"committed","workflow Phase":"execution","priority":"P2","canonical Worktree":"/tmp/worktree","test Tier Required":"n/a"}]}\n'
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

AUDIT_JSON="$TMPDIR/audit.json"
python3 "$TMPDIR/github-project-workflow.py" "$TMPDIR" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --mapping "$TMPDIR/.pm/github-project-sync/tasks.json" \
  --json \
  audit > "$AUDIT_JSON"

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
  audit > "$DRIFT_JSON"
DRIFT_EXIT=$?
set -e
[[ "$DRIFT_EXIT" == "1" ]]

python3 - "$DRIFT_JSON" <<'PY'
import json, pathlib, sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert payload["status"] == "failed", payload
assert any("PM Status" in item for item in payload["errors"]), payload
PY

MAPPING_ONLY="$TMPDIR/mapping-only"
mkdir -p "$MAPPING_ONLY/.pm/github-project-sync"
cat > "$MAPPING_ONLY/.pm/github-project-sync/tasks.json" <<'JSON'
{
  "version": 1,
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
  "project item-list 1 --owner eng-cc --limit 1000 --format json")
    if [[ "${GH_FAKE_MAPPING_DRIFT:-0}" == "1" ]]; then
      printf '{"totalCount":1,"items":[{"id":"MAPPING_ITEM_ID","content":{"body":"task_uid: task_33333333333333333333333333333333","number":303,"url":"https://github.com/eng-cc/oasis7/issues/303"},"status":"In Progress","task UID":"task_33333333333333333333333333333333","owner Role":"tpm","module":"engineering","pM Status":"blocked","workflow Phase":"blocked","priority":"P2","canonical Worktree":"/tmp/mapping-worktree","test Tier Required":"n/a"}]}\n'
    else
      printf '{"totalCount":1,"items":[{"id":"MAPPING_ITEM_ID","content":{"body":"task_uid: task_33333333333333333333333333333333","number":303,"url":"https://github.com/eng-cc/oasis7/issues/303"},"status":"In Progress","task UID":"task_33333333333333333333333333333333","owner Role":"tpm","module":"engineering","pM Status":"committed","workflow Phase":"execution","priority":"P2","canonical Worktree":"/tmp/mapping-worktree","test Tier Required":"n/a"}]}\n'
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
python3 "$TMPDIR/github-project-workflow.py" "$MAPPING_ONLY" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --mapping "$MAPPING_ONLY/.pm/github-project-sync/tasks.json" \
  --json \
  audit > "$MAPPING_ONLY_JSON"

python3 - "$MAPPING_ONLY_JSON" <<'PY'
import json, pathlib, sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert payload["status"] == "ok", payload
assert payload["selected_count"] == 1, payload
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
  audit > "$MAPPING_DRIFT_JSON"
MAPPING_DRIFT_EXIT=$?
set -e
[[ "$MAPPING_DRIFT_EXIT" == "1" ]]

python3 - "$MAPPING_DRIFT_JSON" <<'PY'
import json, pathlib, sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert payload["status"] == "failed", payload
assert any("PM Status" in item for item in payload["errors"]), payload
PY

echo "github-project-workflow.test: OK"
