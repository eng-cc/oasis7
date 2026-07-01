#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${PM_ROOT_DIR:-$(cd "$SCRIPT_DIR/../.." && pwd)}"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: ./scripts/pm/promote-signal.sh --source-type <type> --source-ref <path> --role-hint <role> --severity <level> --summary <text> [options]

Create a GitHub-backed reflection intake issue for one signal. When
--create-task is supplied, also create a candidate task through
./scripts/pm/new-task.sh.

Required:
  --source-type <type>        e.g. task_execution_log, incident, qa_block, community_feedback
  --source-ref <path>         Primary source reference
  --role-hint <role>          Canonical role owner hint
  --severity <level>          low | medium | high | critical
  --summary <text>            Signal summary

Optional:
  --signal-id <id>            Override auto-generated SIG-GH-* id
  --create-task               Also create a candidate task
  --title <title>             Task title; defaults to summary
  --owner-role <role>         Task owner; defaults to role_hint
  --priority <P0|P1|P2|P3>    Task priority; defaults from severity
  --doc-ref <path>            Related formal doc; repeatable
  --related-prd <path>        Related PRD; repeatable
  --acceptance <text>         Acceptance criterion; repeatable
  --handoff-to <role>         Suggested handoff role; repeatable
  --worktree-hint <name>      Optional worktree hint for created task
  --json                      Print machine-readable JSON summary
  -h, --help                  Show help

Notes:
  - Signals are represented by GitHub intake issues.
  - Without --create-task, the intake issue remains promotion_state=triaged.
  - With --create-task, the intake issue is linked to the candidate task.
  - The local .pm/inbox/signals.jsonl queue is retired and must not be recreated.
  - Use PM_ROOT_DIR=/tmp/... to smoke-test against a copied .pm tree.
USAGE
}

SIGNAL_ID=""
SOURCE_TYPE=""
SOURCE_REF=""
ROLE_HINT=""
SEVERITY=""
SUMMARY=""
CREATE_TASK=0
TASK_TITLE=""
OWNER_ROLE=""
PRIORITY=""
WORKTREE_HINT=""
OUTPUT_JSON=0
DOC_REFS=()
RELATED_PRDS=()
ACCEPTANCE=()
HANDOFF_TO=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --signal-id)
      SIGNAL_ID="${2:-}"
      shift 2
      ;;
    --source-type)
      SOURCE_TYPE="${2:-}"
      shift 2
      ;;
    --source-ref)
      SOURCE_REF="${2:-}"
      shift 2
      ;;
    --role-hint)
      ROLE_HINT="${2:-}"
      shift 2
      ;;
    --severity)
      SEVERITY="${2:-}"
      shift 2
      ;;
    --summary)
      SUMMARY="${2:-}"
      shift 2
      ;;
    --create-task)
      CREATE_TASK=1
      shift
      ;;
    --title)
      TASK_TITLE="${2:-}"
      shift 2
      ;;
    --owner-role)
      OWNER_ROLE="${2:-}"
      shift 2
      ;;
    --priority)
      PRIORITY="${2:-}"
      shift 2
      ;;
    --doc-ref)
      DOC_REFS+=("${2:-}")
      shift 2
      ;;
    --related-prd)
      RELATED_PRDS+=("${2:-}")
      shift 2
      ;;
    --acceptance)
      ACCEPTANCE+=("${2:-}")
      shift 2
      ;;
    --handoff-to)
      HANDOFF_TO+=("${2:-}")
      shift 2
      ;;
    --worktree-hint)
      WORKTREE_HINT="${2:-}"
      shift 2
      ;;
    --json)
      OUTPUT_JSON=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "promote-signal: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

[[ -n "$SOURCE_TYPE" ]] || { echo "promote-signal: --source-type is required" >&2; exit 2; }
[[ -n "$SOURCE_REF" ]] || { echo "promote-signal: --source-ref is required" >&2; exit 2; }
[[ -n "$ROLE_HINT" ]] || { echo "promote-signal: --role-hint is required" >&2; exit 2; }
[[ -n "$SEVERITY" ]] || { echo "promote-signal: --severity is required" >&2; exit 2; }
[[ -n "$SUMMARY" ]] || { echo "promote-signal: --summary is required" >&2; exit 2; }

case "$SEVERITY" in
  low|medium|high|critical) ;;
  *)
    echo "promote-signal: unsupported severity: $SEVERITY" >&2
    exit 2
    ;;
esac

[[ "$SOURCE_TYPE" != "devlog" ]] || {
  echo "promote-signal: source_type=devlog is no longer allowed for PM runtime objects" >&2
  exit 2
}

python3 - "$ROOT_DIR" "$SOURCE_REF" <<'PY'
from __future__ import annotations

import pathlib
import sys

root = pathlib.Path(sys.argv[1])
source_ref = sys.argv[2]
source_path = source_ref.split("#", 1)[0]
if not source_path:
    raise SystemExit("promote-signal: empty source_ref path")
if source_path.startswith(("http://", "https://")):
    raise SystemExit(0)
parts = pathlib.PurePosixPath(source_path.replace("\\", "/")).parts
if len(parts) >= 2 and parts[0] == "doc" and parts[1] == "devlog":
    raise SystemExit(
        f"promote-signal: doc/devlog archive cannot be used as PM runtime source_ref: {source_ref}"
    )
path = pathlib.Path(source_path).expanduser()
resolved = path if path.is_absolute() else root / path
if not resolved.exists():
    raise SystemExit(f"promote-signal: source_ref missing: {source_path}")
PY

grep -Fxq "$ROLE_HINT" < <(sed -n 's/^  - role_name: //p' .pm/registry/roles.yaml) || {
  echo "promote-signal: unknown role hint: $ROLE_HINT" >&2
  exit 2
}

if [[ -z "$SIGNAL_ID" ]]; then
  SIGNAL_ID="$(python3 - <<'PY'
from __future__ import annotations

import uuid

print(f"SIG-GH-{uuid.uuid4().hex[:12]}")
PY
)"
fi

python3 - "$ROOT_DIR" "$SIGNAL_ID" <<'PY'
from __future__ import annotations

import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
cache_path = root / ".pm/github-project-sync/intake-signals.json"
mapping_path = root / ".pm/github-project-sync/tasks.json"
signal_id = sys.argv[2]
if cache_path.exists():
    cache = json.loads(cache_path.read_text(encoding="utf-8"))
    signals = cache.get("signals") or {}
    if signal_id in signals:
        raise SystemExit(f"promote-signal: duplicate signal_id: {signal_id}")
if mapping_path.exists():
    mapping = json.loads(mapping_path.read_text(encoding="utf-8"))
    for record in (mapping.get("tasks") or {}).values():
        if str(record.get("source_signal") or "") == signal_id:
            raise SystemExit(f"promote-signal: duplicate signal_id: {signal_id}")
PY

INTAKE_BODY="$(mktemp)"
cleanup_intake_body() {
  rm -f "$INTAKE_BODY"
}
trap cleanup_intake_body EXIT

cat > "$INTAKE_BODY" <<EOF
<!-- oasis7-pm-signal -->
signal_id: $SIGNAL_ID

GitHub-backed oasis7 PM intake signal.

Signal metadata:
- source_type: \`$SOURCE_TYPE\`
- source_ref: \`$SOURCE_REF\`
- role_hint: \`$ROLE_HINT\`
- severity: \`$SEVERITY\`
- promotion_state: \`triaged\`
- memory_promotion_state: \`pending\`

Summary:
$SUMMARY
EOF

INTAKE_URL="$(gh issue create -R "${GITHUB_REPOSITORY:-eng-cc/oasis7}" --title "[PM Signal] $SUMMARY" --body-file "$INTAKE_BODY")"
INTAKE_ISSUE_NUMBER="${INTAKE_URL##*/}"
PROMOTION_STATE="triaged"
TASK_JSON="null"

if [[ -z "$PRIORITY" ]]; then
  case "$SEVERITY" in
    critical) PRIORITY="P0" ;;
    high) PRIORITY="P1" ;;
    medium) PRIORITY="P2" ;;
    low) PRIORITY="P3" ;;
  esac
fi

if [[ "$CREATE_TASK" == "1" ]]; then
  [[ -n "$TASK_TITLE" ]] || TASK_TITLE="$SUMMARY"
  [[ -n "$OWNER_ROLE" ]] || OWNER_ROLE="$ROLE_HINT"

  TASK_ARGS=(
    --owner-role "$OWNER_ROLE"
    --title "$TASK_TITLE"
    --priority "$PRIORITY"
    --source-signal "$SIGNAL_ID"
    --source-type "$SOURCE_TYPE"
    --severity "$SEVERITY"
    --source-ref "$SOURCE_REF"
    --source-ref "$INTAKE_URL"
    --json
  )

  if [[ "${#DOC_REFS[@]}" -gt 0 ]]; then
    for doc_ref in "${DOC_REFS[@]}"; do
      TASK_ARGS+=(--doc-ref "$doc_ref")
    done
  fi

  if [[ "${#RELATED_PRDS[@]}" -gt 0 ]]; then
    for related_prd in "${RELATED_PRDS[@]}"; do
      TASK_ARGS+=(--related-prd "$related_prd")
    done
  fi

  if [[ "${#ACCEPTANCE[@]}" -gt 0 ]]; then
    for acceptance_item in "${ACCEPTANCE[@]}"; do
      TASK_ARGS+=(--acceptance "$acceptance_item")
    done
  fi

  if [[ "${#HANDOFF_TO[@]}" -gt 0 ]]; then
    for handoff_role in "${HANDOFF_TO[@]}"; do
      TASK_ARGS+=(--handoff-to "$handoff_role")
    done
  fi

  if [[ -n "$WORKTREE_HINT" ]]; then
    TASK_ARGS+=(--worktree-hint "$WORKTREE_HINT")
  fi

  TASK_JSON="$("$SCRIPT_DIR/new-task.sh" "${TASK_ARGS[@]}")"
  PROMOTION_STATE="promoted_candidate_task"
  TASK_UID="$(python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["task_uid"])' <<<"$TASK_JSON")"
  gh issue comment "$INTAKE_ISSUE_NUMBER" -R "${GITHUB_REPOSITORY:-eng-cc/oasis7}" --body "Signal promoted to candidate task: $TASK_UID" >/dev/null
fi

python3 - "$ROOT_DIR" "$SIGNAL_ID" "$SOURCE_TYPE" "$SOURCE_REF" "$ROLE_HINT" "$SEVERITY" "$SUMMARY" "$PROMOTION_STATE" "$INTAKE_URL" "$TASK_JSON" <<'PY'
from __future__ import annotations

import json
import pathlib
import sys
from collections import OrderedDict

root = pathlib.Path(sys.argv[1])
signal_id = sys.argv[2]
task_payload = None if sys.argv[10] == "null" else json.loads(sys.argv[10])
cache_path = root / ".pm/github-project-sync/intake-signals.json"
if cache_path.exists():
    cache = json.loads(cache_path.read_text(encoding="utf-8"), object_pairs_hook=OrderedDict)
else:
    cache = OrderedDict([("version", 1), ("signals", OrderedDict())])
signals = cache.setdefault("signals", OrderedDict())
signals[signal_id] = OrderedDict(
    [
        ("signal_id", signal_id),
        ("source_type", sys.argv[3]),
        ("source_ref", sys.argv[4]),
        ("role_hint", sys.argv[5]),
        ("severity", sys.argv[6]),
        ("summary", sys.argv[7]),
        ("promotion_state", sys.argv[8]),
        ("memory_promotion_state", "pending"),
        ("issue_url", sys.argv[9]),
        ("task_uid", (task_payload or {}).get("task_uid", "")),
        ("task_url", (task_payload or {}).get("issue_url", "")),
    ]
)
cache_path.parent.mkdir(parents=True, exist_ok=True)
cache_path.write_text(json.dumps(cache, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
PY

RESULT_JSON="$(python3 - "$SIGNAL_ID" "$PROMOTION_STATE" "$INTAKE_URL" "$TASK_JSON" <<'PY'
from __future__ import annotations

import json
import sys

task_payload = None if sys.argv[4] == "null" else json.loads(sys.argv[4])
print(
    json.dumps(
        {
            "signal_id": sys.argv[1],
            "promotion_state": sys.argv[2],
            "issue_url": sys.argv[3],
            "task": task_payload,
        },
        ensure_ascii=False,
    )
)
PY
)"

if [[ "$OUTPUT_JSON" == "1" ]]; then
  printf '%s\n' "$RESULT_JSON"
  exit 0
fi

if [[ "$CREATE_TASK" == "1" ]]; then
  TASK_UID="$(python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["task"]["task_uid"])' <<<"$RESULT_JSON")"
  echo "promote-signal: created $SIGNAL_ID and candidate task $TASK_UID"
else
  echo "promote-signal: created $SIGNAL_ID"
fi
