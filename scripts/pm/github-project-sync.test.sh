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
{"id":"FIELD_STATUS","name":"Status","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_TODO","name":"Todo"},{"id":"OPT_IN_PROGRESS","name":"In Progress"},{"id":"OPT_DONE_STATUS","name":"Done"}]},
{"id":"FIELD_TASK_UID","name":"Task UID","type":"ProjectV2Field"},
{"id":"FIELD_OWNER_ROLE","name":"Owner Role","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_TPM","name":"tpm"},{"id":"OPT_QA","name":"qa_engineer"}]},
{"id":"FIELD_MODULE","name":"Module","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_ENGINEERING","name":"engineering"},{"id":"OPT_VISUALIZATION","name":"visualization"}]},
{"id":"FIELD_PM_STATUS","name":"PM Status","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_COMMITTED","name":"committed"},{"id":"OPT_DONE","name":"done"}]},
{"id":"FIELD_WORKFLOW_PHASE","name":"Workflow Phase","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_EXECUTION","name":"execution"},{"id":"OPT_DONE_PHASE","name":"done"}]},
{"id":"FIELD_PRIORITY","name":"Priority","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_P2","name":"P2"},{"id":"OPT_P3","name":"P3"}]},
{"id":"FIELD_BLOCKED","name":"Blocked Reason","type":"ProjectV2Field"},
{"id":"FIELD_WORKTREE","name":"Canonical Worktree","type":"ProjectV2Field"},
{"id":"FIELD_PR","name":"PR","type":"ProjectV2Field"},
{"id":"FIELD_TIER","name":"Test Tier Required","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_NA","name":"n/a"}]},
{"id":"FIELD_UPDATED","name":"Last PM Update","type":"ProjectV2Field"}]}
JSON
    ;;
  "project item-list 1 --owner eng-cc --limit 1000 --format json")
    printf '{"items":[],"totalCount":0}\n'
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

DRY_JSON="$TMPDIR/dry.json"
python3 "$TMPDIR/github-project-sync.py" "$TMPDIR" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --mapping "$TMPDIR/.pm/github-project-sync/tasks.json" \
  --dry-run \
  --json > "$DRY_JSON"

python3 - "$DRY_JSON" "$GH_CALL_LOG" <<'PY'
import json, pathlib, sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
calls = pathlib.Path(sys.argv[2]).read_text()
assert payload["selected_count"] == 1, payload
assert payload["dry_run"] is True, payload
assert calls == "", calls
PY

APPLY_JSON="$TMPDIR/apply.json"
python3 "$TMPDIR/github-project-sync.py" "$TMPDIR" \
  --repo eng-cc/oasis7 \
  --project-owner eng-cc \
  --project-number 1 \
  --mapping "$TMPDIR/.pm/github-project-sync/tasks.json" \
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
