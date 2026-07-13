#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

TMPDIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

TASK_A="task_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
TASK_B="task_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
TASK_HOLD="task_cccccccccccccccccccccccccccccccc"
TASK_BLOCKED="task_dddddddddddddddddddddddddddddddd"
MAPPING="$TMPDIR/tasks.json"
LOG="$TMPDIR/gh.log"
mkdir -p "$TMPDIR/bin"

cat > "$MAPPING" <<EOF
{
  "project": {
    "repo": "eng-cc/oasis7"
  },
  "tasks": {
    "$TASK_A": {
      "issue_number": 123,
      "issue_url": "https://github.com/eng-cc/oasis7/issues/123",
      "owner_role": "tpm",
      "priority": "P3",
      "project_item_id": "PVTI_A",
      "status": "pr_watch",
      "title": "open merged task"
    },
    "$TASK_B": {
      "issue_number": 124,
      "issue_url": "https://github.com/eng-cc/oasis7/issues/124",
      "owner_role": "tpm",
      "priority": "P3",
      "project_item_id": "PVTI_B",
      "status": "pr_watch",
      "title": "closed merged task"
    },
    "$TASK_HOLD": {
      "issue_number": 126,
      "issue_url": "https://github.com/eng-cc/oasis7/issues/126",
      "owner_role": "tpm",
      "priority": "P3",
      "project_item_id": "PVTI_HOLD",
      "status": "pr_watch",
      "title": "manual hold task"
    },
    "$TASK_BLOCKED": {
      "issue_number": 128,
      "issue_url": "https://github.com/eng-cc/oasis7/issues/128",
      "owner_role": "tpm",
      "priority": "P3",
      "status": "pr_watch",
      "title": "missing project item task"
    },
    "task_eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee": {
      "issue_number": 129,
      "issue_url": "https://github.com/eng-cc/oasis7/issues/129",
      "owner_role": "tpm",
      "priority": "P3",
      "project_item_id": "PVTI_NO_EVIDENCE",
      "status": "pr_watch",
      "title": "missing evidence task"
    },
    "task_ffffffffffffffffffffffffffffffff": {
      "issue_number": 130,
      "issue_url": "https://github.com/eng-cc/oasis7/issues/130",
      "owner_role": "tpm",
      "priority": "P3",
      "project_item_id": "PVTI_MIXED_EVIDENCE",
      "status": "pr_watch",
      "title": "mixed evidence task"
    }
  },
  "version": 1
}
EOF

cat > "$TMPDIR/bin/gh" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "\$*" >> "$LOG"

json_issue() {
  local number="\$1"
  local state="\$2"
  local uid="\$3"
  local status="\$4"
  local pr_number="\$5"
  local extra="\${6:-}"
  python3 - "\$number" "\$state" "\$uid" "\$status" "\$pr_number" "\$extra" <<'PY'
import json
import sys

number, state, uid, status, pr_number, extra = sys.argv[1:]
bt = chr(96)
body = f"""<!-- oasis7-pm-task -->
task_uid: {uid}

GitHub-backed oasis7 PM task.

Task metadata:
- owner_role: {bt}tpm{bt}
- module: {bt}engineering{bt}
- status: {bt}{status}{bt}
- priority: {bt}P3{bt}
- worktree_hint: {bt}/tmp/worktree{bt}
- pr_url: {bt}https://github.com/eng-cc/oasis7/pull/{pr_number}{bt}
- pr_number: {bt}{pr_number}{bt}

{extra}
"""
comments = []
if extra == "mixed_ready_evidence":
    other_uid = "task_00000000000000000000000000000000"
    comments = [
        {"body": f"Pre-PR Local Role Review: passed\nTask UID: {uid}\nReview Findings Disposition: no_findings"},
        {"body": f"<!-- oasis7-pm-claim-verification -->\nTask UID: {other_uid}\nClaim Type: ready_for_pr\nVerification Status: verified"},
        {"body": f"<!-- oasis7-pm-evidence -->\nTask UID: {uid}\nEvidence Phase: pre_pr_ready\nRole: tpm"},
    ]
elif extra != "no_ready_evidence":
    comments = [
        {"body": f"Pre-PR Local Role Review: passed\nTask UID: {uid}\nReview Findings Disposition: no_findings"},
        {"body": f"<!-- oasis7-pm-claim-verification -->\nTask UID: {uid}\nClaim Type: ready_for_pr\nVerification Status: verified"},
        {"body": f"<!-- oasis7-pm-evidence -->\nTask UID: {uid}\nEvidence Phase: pre_pr_ready\nRole: tpm"},
    ]
print(json.dumps({"body": body, "comments": comments, "number": int(number), "state": state, "title": f"issue {number}", "url": f"https://github.com/eng-cc/oasis7/issues/{number}"}))
PY
}

if [[ "\${1:-}" == "api" && "\${2:-}" == "graphql" ]]; then
  printf '{"data":{"rateLimit":{"remaining":5000,"resetAt":"2099-01-01T00:00:00Z"}}}\n'
  exit 0
fi
if [[ "\${1:-}" == "issue" && "\${2:-}" == "list" ]]; then
  printf '[{"number":123,"state":"OPEN","title":"open merged task","url":"https://github.com/eng-cc/oasis7/issues/123"},{"number":124,"state":"CLOSED","title":"closed merged task","url":"https://github.com/eng-cc/oasis7/issues/124"},{"number":125,"state":"OPEN","title":"ready task","url":"https://github.com/eng-cc/oasis7/issues/125"},{"number":126,"state":"OPEN","title":"hold task","url":"https://github.com/eng-cc/oasis7/issues/126"},{"number":128,"state":"OPEN","title":"blocked task","url":"https://github.com/eng-cc/oasis7/issues/128"},{"number":129,"state":"OPEN","title":"missing evidence task","url":"https://github.com/eng-cc/oasis7/issues/129"},{"number":130,"state":"OPEN","title":"mixed evidence task","url":"https://github.com/eng-cc/oasis7/issues/130"}]\n'
  exit 0
fi

if [[ "\${1:-}" == "issue" && "\${2:-}" == "view" ]]; then
  case "\${3:-}" in
    123) json_issue 123 OPEN "$TASK_A" pr_watch 999 ;;
    124) json_issue 124 CLOSED "$TASK_B" pr_watch 998 ;;
    125) json_issue 125 OPEN "$TASK_A" ready 999 ;;
    126) json_issue 126 OPEN "$TASK_HOLD" pr_watch 997 manual_packaging_ci_hold ;;
    128) json_issue 128 OPEN "$TASK_BLOCKED" pr_watch 996 ;;
    129) json_issue 129 OPEN task_eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee pr_watch 995 no_ready_evidence ;;
    130) json_issue 130 OPEN task_ffffffffffffffffffffffffffffffff pr_watch 994 mixed_ready_evidence ;;
    *) exit 1 ;;
  esac
  exit 0
fi

if [[ "\${1:-}" == "pr" && "\${2:-}" == "view" ]]; then
  case "\${3:-}" in
    999) printf '{"number":999,"state":"MERGED","mergedAt":"2026-07-05T00:00:00Z","url":"https://github.com/eng-cc/oasis7/pull/999","title":"merged"}\n' ;;
    998) printf '{"number":998,"state":"MERGED","mergedAt":"2026-07-05T00:00:00Z","url":"https://github.com/eng-cc/oasis7/pull/998","title":"merged"}\n' ;;
    997) printf '{"number":997,"state":"MERGED","mergedAt":"2026-07-05T00:00:00Z","url":"https://github.com/eng-cc/oasis7/pull/997","title":"merged"}\n' ;;
    996) printf '{"number":996,"state":"MERGED","mergedAt":"2026-07-05T00:00:00Z","url":"https://github.com/eng-cc/oasis7/pull/996","title":"merged"}\n' ;;
    995) printf '{"number":995,"state":"MERGED","mergedAt":"2026-07-05T00:00:00Z","url":"https://github.com/eng-cc/oasis7/pull/995","title":"merged"}\n' ;;
    994) printf '{"number":994,"state":"MERGED","mergedAt":"2026-07-05T00:00:00Z","url":"https://github.com/eng-cc/oasis7/pull/994","title":"merged"}\n' ;;
    *) exit 1 ;;
  esac
  exit 0
fi

if [[ "\$*" == "project view 1 --owner eng-cc --format json" ]]; then
  printf '{"id":"PROJECT_ID","number":1,"title":"fixture","url":"https://github.com/users/eng-cc/projects/1"}\n'
  exit 0
fi

if [[ "\$*" == "project field-list 1 --owner eng-cc --format json" ]]; then
  cat <<'JSON'
{"fields":[
{"id":"FIELD_STATUS","name":"Status","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_IN_PROGRESS","name":"In Progress"},{"id":"OPT_DONE","name":"Done"}]},
{"id":"FIELD_PM_STATUS","name":"PM Status","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_DONE_PM","name":"done"}]},
{"id":"FIELD_WORKFLOW_PHASE","name":"Workflow Phase","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_DONE_PHASE","name":"done"}]}
]}
JSON
  exit 0
fi

if [[ "\${1:-}" == "project" && "\${2:-}" == "item-edit" ]]; then
  printf '{}\n'
  exit 0
fi

if [[ "\${1:-}" == "issue" && "\${2:-}" == "edit" ]]; then
  printf 'edited\n'
  exit 0
fi

if [[ "\${1:-}" == "issue" && "\${2:-}" == "comment" ]]; then
  printf 'https://github.com/eng-cc/oasis7/issues/%s#issuecomment-fixture\n' "\${3:-0}"
  exit 0
fi

if [[ "\${1:-}" == "issue" && "\${2:-}" == "close" ]]; then
  printf 'closed\n'
  exit 0
fi

echo "unexpected gh invocation: \$*" >&2
exit 1
EOF
chmod +x "$TMPDIR/bin/gh"

PATH="$TMPDIR/bin:$PATH" "$ROOT_DIR/scripts/pm/audit-pr-watch-issues.sh" --mapping "$MAPPING" --global-maintenance --json > "$TMPDIR/dry-run.json"

python3 - "$TMPDIR/dry-run.json" <<'PY'
import json
import sys
from pathlib import Path

payload = json.loads(Path(sys.argv[1]).read_text())
by_number = {item["issue_number"]: item for item in payload["results"]}
assert by_number[123]["status"] == "would_advance", by_number
assert by_number[124]["status"] == "would_advance", by_number
assert by_number[126]["status"] == "skipped" and "manual hold" in by_number[126]["reason"], by_number
assert by_number[128]["status"] == "blocked" and "project_item_id" in by_number[128]["reason"], by_number
assert by_number[129]["status"] == "blocked" and "pre-PR review" in by_number[129]["reason"], by_number
assert by_number[130]["status"] == "blocked" and "pre-PR review" in by_number[130]["reason"], by_number
PY

: > "$LOG"
PATH="$TMPDIR/bin:$PATH" "$ROOT_DIR/scripts/pm/audit-pr-watch-issues.sh" --mapping "$MAPPING" --global-maintenance --close --json > "$TMPDIR/close.json"

python3 - "$TMPDIR/close.json" "$MAPPING" "$LOG" <<'PY'
import json
import sys
from pathlib import Path

payload = json.loads(Path(sys.argv[1]).read_text())
mapping = json.loads(Path(sys.argv[2]).read_text())
log = Path(sys.argv[3]).read_text().splitlines()
by_number = {item["issue_number"]: item for item in payload["results"]}
assert by_number[123]["status"] == "task_done", by_number
assert by_number[124]["status"] == "task_done", by_number
assert by_number[123]["next_action"] == "run canonical terminal runbook from post-merge-main-sync.sh", by_number
assert by_number[124]["next_action"] == "run canonical terminal runbook from post-merge-main-sync.sh", by_number
assert by_number[128]["status"] == "blocked", by_number
assert by_number[129]["status"] == "blocked", by_number
assert by_number[130]["status"] == "blocked", by_number
assert mapping["tasks"]["task_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"]["status"] == "done", mapping
assert mapping["tasks"]["task_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"]["status"] == "done", mapping
assert mapping["tasks"]["task_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"]["workflow_phase"] == "task_done", mapping
assert mapping["tasks"]["task_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"]["workflow_phase"] == "task_done", mapping
assert mapping["tasks"]["task_cccccccccccccccccccccccccccccccc"]["status"] == "pr_watch", mapping
assert mapping["tasks"]["task_ffffffffffffffffffffffffffffffff"]["status"] == "pr_watch", mapping
assert not any(line == "issue close 123 -R eng-cc/oasis7 --reason completed" for line in log), log
assert not any(line == "issue close 124 -R eng-cc/oasis7 --reason completed" for line in log), log
assert any(line.startswith("issue comment 123 -R eng-cc/oasis7") for line in log), log
assert any(line.startswith("issue comment 124 -R eng-cc/oasis7") for line in log), log
assert sum(1 for line in log if line.startswith("project item-edit")) >= 6, log
PY

echo "audit-pr-watch-issues.test: OK"
