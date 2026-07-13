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
cp "$ROOT_DIR/scripts/pm/github-project-retire-tasks.py" "$TMPDIR/github-project-retire-tasks.py"
cp "$ROOT_DIR/scripts/pm/github-project-workflow.py" "$TMPDIR/github-project-workflow.py"

cat > "$TMPDIR/.pm/tasks/task_11111111111111111111111111111111.yaml" <<'YAML'
task_uid: task_11111111111111111111111111111111
title: "retired task"
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
updated_at: 2026-06-30T00:00:00+08:00
YAML

cat > "$TMPDIR/.pm/tasks/task_11111111111111111111111111111111.execution.md" <<'MD'
# Task Execution Log: retired task

- evidence survives in archive
MD

cat > "$TMPDIR/.pm/github-project-sync/tasks.json" <<'JSON'
{
  "version": 1,
  "tasks": {
    "task_11111111111111111111111111111111": {
      "task_uid": "task_11111111111111111111111111111111",
      "issue_url": "https://github.com/eng-cc/oasis7/issues/101",
      "issue_number": 101,
      "project_item_id": "ITEM_ID"
    },
    "task_22222222222222222222222222222222": {
      "task_uid": "task_22222222222222222222222222222222",
      "issue_url": "https://github.com/eng-cc/oasis7/issues/202",
      "issue_number": 202,
      "project_item_id": "OLD_ITEM_ID",
      "status": "done",
      "workflow_phase": "post_merge_done"
    }
  }
}
JSON

cat > "$TMPDIR/.pm/github-project-sync/task-archive.jsonl" <<'JSONL'
{"archived_at":"2026-06-29T00:00:00+08:00","execution_log_path":".pm/tasks/task_22222222222222222222222222222222.execution.md","execution_log_sha256":"old-log","execution_log_text":"old evidence","github_project_mapping":{"issue_number":202,"issue_url":"https://github.com/eng-cc/oasis7/issues/202","project_item_id":"OLD_ITEM_ID"},"task":{"status":"done","task_uid":"task_22222222222222222222222222222222","title":"already archived task"},"task_path":".pm/tasks/task_22222222222222222222222222222222.yaml","task_sha256":"old-task","task_uid":"task_22222222222222222222222222222222"}
JSONL

RETIRE_JSON="$TMPDIR/retire.json"
python3 "$TMPDIR/github-project-retire-tasks.py" "$TMPDIR" \
  --mapping "$TMPDIR/.pm/github-project-sync/tasks.json" \
  --archive "$TMPDIR/.pm/github-project-sync/task-archive.jsonl" \
  --summary "$TMPDIR/.pm/github-project-sync/task-retirement-summary.json" \
  --delete \
  --json > "$RETIRE_JSON"

python3 - "$RETIRE_JSON" "$TMPDIR/.pm/github-project-sync/task-archive.jsonl" <<'PY'
import json, pathlib, sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
archive = pathlib.Path(sys.argv[2]).read_text().splitlines()
records = [json.loads(line) for line in archive]
record = next(item for item in records if item["task_uid"] == "task_11111111111111111111111111111111")
assert payload["status"] == "ok", payload
assert payload["selected_count"] == 2, payload
assert payload["deletion"]["deleted_count"] == 2, payload
assert len(records) == 2, records
assert any(item["task_uid"] == "task_22222222222222222222222222222222" for item in records), records
assert record["task"]["task_uid"] == "task_11111111111111111111111111111111", record
assert "evidence survives" in record["execution_log_text"], record
assert payload["archive_reused"] is False, payload
PY

[[ ! -e "$TMPDIR/.pm/tasks/task_11111111111111111111111111111111.yaml" ]]
[[ ! -e "$TMPDIR/.pm/tasks/task_11111111111111111111111111111111.execution.md" ]]

RERUN_JSON="$TMPDIR/rerun.json"
python3 "$TMPDIR/github-project-retire-tasks.py" "$TMPDIR" \
  --mapping "$TMPDIR/.pm/github-project-sync/tasks.json" \
  --archive "$TMPDIR/.pm/github-project-sync/task-archive.jsonl" \
  --summary "$TMPDIR/.pm/github-project-sync/task-retirement-summary.json" \
  --delete \
  --json > "$RERUN_JSON"

python3 - "$RERUN_JSON" "$TMPDIR/.pm/github-project-sync/task-archive.jsonl" <<'PY'
import json, pathlib, sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
archive = pathlib.Path(sys.argv[2]).read_text().splitlines()
assert payload["status"] == "ok", payload
assert payload["archive_reused"] is True, payload
assert payload["selected_count"] == 2, payload
assert len(archive) == 2, archive
PY

cat > "$TMPDIR/bin/gh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
case "$*" in
  "api graphql "*)
    printf '{"data":{"rateLimit":{"remaining":5000,"resetAt":"2099-01-01T00:00:00Z"}}}\n'
    ;;
  "project item-list 1 --owner eng-cc --limit 1000 --format json")
    printf '{"totalCount":2,"items":[{"id":"ITEM_ID","content":{"body":"task_uid: task_11111111111111111111111111111111","number":101,"url":"https://github.com/eng-cc/oasis7/issues/101"},"status":"In Progress","task UID":"task_11111111111111111111111111111111","owner Role":"tpm","module":"engineering","pM Status":"committed","workflow Phase":"execution","priority":"P2","canonical Worktree":"/tmp/worktree","test Tier Required":"n/a"},{"id":"OLD_ITEM_ID","content":{"body":"task_uid: task_22222222222222222222222222222222","number":202,"url":"https://github.com/eng-cc/oasis7/issues/202"},"status":"Done","task UID":"task_22222222222222222222222222222222","pM Status":"done","workflow Phase":"done","test Tier Required":"n/a"}]}\n'
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
  step3-gate > "$AUDIT_JSON"

python3 - "$AUDIT_JSON" <<'PY'
import json, pathlib, sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert payload["status"] == "ok", payload
assert payload["selected_count"] == 2, payload
assert payload["errors"] == [], payload
PY

echo "github-project-retire-tasks.test: OK"
