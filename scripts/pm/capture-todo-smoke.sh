#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

TMPDIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

mkdir -p "$TMPDIR/scripts" "$TMPDIR/bin"
cp -R "$ROOT_DIR/.pm" "$TMPDIR/.pm"
rm -f "$TMPDIR/.pm/inbox/signals.jsonl"
cp -R "$ROOT_DIR/.agents" "$TMPDIR/.agents"
cp -R "$ROOT_DIR/scripts/pm" "$TMPDIR/scripts/pm"
mkdir -p "$TMPDIR/.pm/evidence"
printf 'discovered pre-task todo source\n' > "$TMPDIR/.pm/evidence/discovery.md"

cat > "$TMPDIR/bin/gh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%q ' "$@" >> "$GH_CALL_LOG"
printf '\n' >> "$GH_CALL_LOG"
case "$*" in
  "issue create -R eng-cc/oasis7 --title [PM Signal] "*)
    printf 'https://github.com/eng-cc/oasis7/issues/3001\n'
    ;;
  "issue create -R eng-cc/oasis7 --title [PM] "*)
    printf 'https://github.com/eng-cc/oasis7/issues/3002\n'
    ;;
  "issue comment 3001 -R eng-cc/oasis7 --body "*)
    printf 'https://github.com/eng-cc/oasis7/issues/3001#issuecomment-1\n'
    ;;
  "project item-add 1 --owner eng-cc --url https://github.com/eng-cc/oasis7/issues/3002 --format json")
    printf '{"id":"ITEM_ID","content":{"url":"https://github.com/eng-cc/oasis7/issues/3002"}}\n'
    ;;
  "project view 1 --owner eng-cc --format json")
    printf '{"id":"PROJECT_ID","number":1,"title":"oasis7 Engineering PM","url":"https://github.com/users/eng-cc/projects/1"}\n'
    ;;
  "project field-list 1 --owner eng-cc --format json")
    cat <<'JSON'
{"fields":[
{"id":"FIELD_STATUS","name":"Status","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_TODO","name":"Todo"}]},
{"id":"FIELD_TASK_UID","name":"Task UID","type":"ProjectV2Field"},
{"id":"FIELD_OWNER_ROLE","name":"Owner Role","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_TPM","name":"tpm"},{"id":"OPT_QA","name":"qa_engineer"}]},
{"id":"FIELD_MODULE","name":"Module","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_ENGINEERING","name":"engineering"}]},
{"id":"FIELD_PM_STATUS","name":"PM Status","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_CANDIDATE","name":"candidate"}]},
{"id":"FIELD_WORKFLOW_PHASE","name":"Workflow Phase","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_EXECUTION","name":"execution"}]},
{"id":"FIELD_PRIORITY","name":"Priority","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_P2","name":"P2"},{"id":"OPT_P3","name":"P3"}]},
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
  api\ search/issues*)
    cat <<'JSON'
{"items":[{"number":3003,"html_url":"https://github.com/eng-cc/oasis7/issues/3003"}]}
JSON
    ;;
  api\ repos/eng-cc/oasis7/issues/3003)
    cat <<'JSON'
{"number":3003,"html_url":"https://github.com/eng-cc/oasis7/issues/3003","body":"<!-- oasis7-pm-signal -->\nsignal_id: SIG-GH-remote-only\n\nGitHub-backed oasis7 PM intake signal.\n\nSignal metadata:\n- source_type: `reflection`\n- source_ref: `.pm/evidence/discovery.md`\n- role_hint: `qa_engineer`\n- severity: `high`\n- promotion_state: `triaged`\n- memory_promotion_state: `pending`\n\nSummary:\nremote-only reflection from GitHub issue\n"}
JSON
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

PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/capture-todo.sh" \
  --signal-id SIG-GH-smoke1 \
  --source-ref .pm/evidence/discovery.md \
  --summary "capture a pre-task discovery with no passthrough args" \
  >"$TMPDIR/minimal.out"
grep -q "promote-signal: created SIG-GH-smoke1" "$TMPDIR/minimal.out" || {
  echo "capture-todo-smoke: minimal capture did not create an intake issue" >&2
  cat "$TMPDIR/minimal.out" >&2
  exit 1
}

SIGNAL_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/capture-todo.sh" \
  --signal-id SIG-GH-smoke2 \
  --source-ref .pm/evidence/discovery.md \
  --summary "capture a pre-task discovery without creating a task" \
  --json)"

python3 - "$TMPDIR" "$SIGNAL_JSON" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
payload = json.loads(sys.argv[2])
if payload["signal_id"] != "SIG-GH-smoke2":
    raise SystemExit("expected deterministic GitHub signal id")
if payload["promotion_state"] != "triaged":
    raise SystemExit("expected signal-only capture to remain triaged")
if payload["task"] is not None:
    raise SystemExit("default capture must not create a task")
if not payload["issue_url"].startswith("https://github.com/eng-cc/oasis7/issues/"):
    raise SystemExit("expected GitHub intake issue URL")
if (root / ".pm/inbox/signals.jsonl").exists():
    raise SystemExit("retired signal inbox must not be recreated")
PY

set +e
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/capture-todo.sh" \
  --signal-id SIG-GH-smoke2 \
  --source-ref .pm/evidence/discovery.md \
  --summary "duplicate signal id should be rejected" \
  --json >"$TMPDIR/duplicate.out" 2>"$TMPDIR/duplicate.err"
DUPLICATE_STATUS=$?
set -e
if [[ "$DUPLICATE_STATUS" == "0" ]]; then
  echo "capture-todo-smoke: expected duplicate signal id to fail" >&2
  exit 1
fi
grep -q "duplicate signal_id: SIG-GH-smoke2" "$TMPDIR/duplicate.err" || {
  echo "capture-todo-smoke: duplicate signal id failure did not explain the conflict" >&2
  cat "$TMPDIR/duplicate.err" >&2
  exit 1
}

rm -f "$TMPDIR/.pm/github-project-sync/intake-signals.json"
python3 - "$TMPDIR" <<'PY'
from __future__ import annotations

import json
from collections import OrderedDict
from pathlib import Path
import sys

root = Path(sys.argv[1])
mapping_path = root / ".pm/github-project-sync/tasks.json"
mapping_path.parent.mkdir(parents=True, exist_ok=True)
mapping = OrderedDict([("tasks", OrderedDict())])
if mapping_path.exists():
    mapping = json.loads(mapping_path.read_text(encoding="utf-8"), object_pairs_hook=OrderedDict)
tasks = mapping.setdefault("tasks", OrderedDict())
tasks["task_duplicate_signal_00000000000001"] = OrderedDict(
    [
        ("task_uid", "task_duplicate_signal_00000000000001"),
        ("issue_url", "https://github.com/eng-cc/oasis7/issues/3999"),
        ("project_item_id", "ITEM_DUPLICATE_SIGNAL"),
        ("status", "candidate"),
        ("owner_role", "qa_engineer"),
        ("title", "mapping duplicate signal"),
        ("source_signal", "SIG-GH-mapping-dupe"),
        ("source_type", "reflection"),
        ("source_refs", [".pm/evidence/discovery.md"]),
    ]
)
mapping_path.write_text(json.dumps(mapping, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
PY

set +e
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/capture-todo.sh" \
  --signal-id SIG-GH-mapping-dupe \
  --source-ref .pm/evidence/discovery.md \
  --summary "mapping duplicate signal id should be rejected" \
  --json >"$TMPDIR/mapping-duplicate.out" 2>"$TMPDIR/mapping-duplicate.err"
MAPPING_DUPLICATE_STATUS=$?
set -e
if [[ "$MAPPING_DUPLICATE_STATUS" == "0" ]]; then
  echo "capture-todo-smoke: expected mapping duplicate signal id to fail" >&2
  exit 1
fi
grep -q "duplicate signal_id: SIG-GH-mapping-dupe" "$TMPDIR/mapping-duplicate.err" || {
  echo "capture-todo-smoke: mapping duplicate signal id failure did not explain the conflict" >&2
  cat "$TMPDIR/mapping-duplicate.err" >&2
  exit 1
}

rm -f "$TMPDIR/.pm/github-project-sync/intake-signals.json"
REFLECTION_REPORT_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/reflection-report.sh" --json)"
python3 - "$TMPDIR" "$REFLECTION_REPORT_JSON" <<'PY'
from __future__ import annotations

import json
from pathlib import Path
import sys

root = Path(sys.argv[1])
report = json.loads(sys.argv[2])
cache_path = root / ".pm/github-project-sync/intake-signals.json"
if not cache_path.exists():
    print((root / "gh-calls.log").read_text(encoding="utf-8"), file=sys.stderr)
    raise SystemExit("reflection-report should rebuild missing GitHub intake mirror")
cache = json.loads(cache_path.read_text(encoding="utf-8"))
if "SIG-GH-remote-only" not in (cache.get("signals") or {}):
    raise SystemExit("rebuilt intake mirror missing remote-only signal")
if not any(item.get("signal_id") == "SIG-GH-remote-only" for item in report.get("items") or []):
    raise SystemExit("reflection-report missing remote-only GitHub signal")
PY

TASK_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/capture-todo.sh" \
  --signal-id SIG-GH-smoke3 \
  --source-ref .pm/evidence/discovery.md \
  --text "promote this discovery into a candidate task" \
  --role-hint tpm \
  --severity medium \
  --create-task \
  --title "Promote pre-task discovery" \
  --owner-role qa_engineer \
  --priority P2 \
  --acceptance "candidate task is created only when requested" \
  --json)"

python3 - "$TASK_JSON" <<'PY'
from __future__ import annotations

import json
import sys

payload = json.loads(sys.argv[1])
if payload["signal_id"] != "SIG-GH-smoke3":
    raise SystemExit("expected third deterministic signal id")
if payload["promotion_state"] != "promoted_candidate_task":
    raise SystemExit("expected promoted candidate task state")
task = payload["task"]
if not task:
    raise SystemExit("expected created task payload")
if task["source_signal"] != "SIG-GH-smoke3":
    raise SystemExit("expected task to reference source signal")
if task["owner_role"] != "qa_engineer":
    raise SystemExit("expected explicit owner role passthrough")
if task["priority"] != "P2":
    raise SystemExit("expected explicit priority passthrough")
source_refs = task["source_refs"]
if not any(ref.startswith("https://github.com/eng-cc/oasis7/issues/") for ref in source_refs):
    raise SystemExit("expected task source refs to link back to intake issue")
PY

echo "capture-todo-smoke: OK"
