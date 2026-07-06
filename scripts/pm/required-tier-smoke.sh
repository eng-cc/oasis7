#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

OUTPUT_JSON=0
KEEP_TEMP=0

usage() {
  cat <<'USAGE'
Usage: ./scripts/pm/required-tier-smoke.sh [--json] [--keep-temp]

Run an isolated required-tier validation chain for the file-based PM runtime:
  seed evidence -> task execution log -> signal -> task/memory -> blocked task -> workflow/role/stage report -> task closeout helper with fresh verification

Options:
  --json       Print machine-readable JSON summary
  --keep-temp  Keep the temporary PM root for inspection
  -h, --help   Show help
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --json)
      OUTPUT_JSON=1
      shift
      ;;
    --keep-temp)
      KEEP_TEMP=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "required-tier-smoke: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

TMPDIR="$(mktemp -d)"
cleanup() {
  if [[ "$KEEP_TEMP" != "1" ]]; then
    rm -rf "$TMPDIR"
  fi
}
trap cleanup EXIT

mkdir -p "$TMPDIR/scripts" "$TMPDIR/bin"
cp -R "$ROOT_DIR/.pm" "$TMPDIR/.pm"
cp -R "$ROOT_DIR/.agents" "$TMPDIR/.agents"
cp -R "$ROOT_DIR/scripts/pm" "$TMPDIR/scripts/pm"
mkdir -p "$TMPDIR/.pm/evidence" "$TMPDIR/.pm/shared/memory" "$TMPDIR/.pm/stage"
: > "$TMPDIR/.pm/github-project-sync/task-archive.jsonl"

cat > "$TMPDIR/bin/gh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%q ' "$@" >> "$GH_CALL_LOG"
printf '\n' >> "$GH_CALL_LOG"

next_issue_url() {
  local current
  current="$(cat "$GH_MOCK_COUNTER")"
  current=$((current + 1))
  printf '%s\n' "$current" > "$GH_MOCK_COUNTER"
  printf 'https://github.com/eng-cc/oasis7/issues/%s\n' "$current"
}

if [[ "$1" == "issue" && "$2" == "create" ]]; then
  next_issue_url
  exit 0
fi

if [[ "$1" == "issue" && "$2" == "comment" ]]; then
  printf 'https://github.com/eng-cc/oasis7/issues/%s#issuecomment-1\n' "$3"
  exit 0
fi

if [[ "$1" == "issue" && "$2" == "edit" ]]; then
  printf 'https://github.com/eng-cc/oasis7/issues/%s\n' "$3"
  exit 0
fi

if [[ "$1" == "project" && "$2" == "view" ]]; then
  printf '{"id":"PROJECT_ID","number":1,"title":"oasis7 Engineering PM","url":"https://github.com/users/eng-cc/projects/1"}\n'
  exit 0
fi

if [[ "$1" == "project" && "$2" == "field-list" ]]; then
  cat <<'JSON'
{"fields":[
{"id":"FIELD_STATUS","name":"Status","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_TODO","name":"Todo"},{"id":"OPT_BLOCKED","name":"Blocked"},{"id":"OPT_READY","name":"Ready / PR"},{"id":"OPT_PR_WATCH","name":"PR Watch"},{"id":"OPT_IN_PROGRESS","name":"In Progress"},{"id":"OPT_DONE","name":"Done"}]},
{"id":"FIELD_TASK_UID","name":"Task UID","type":"ProjectV2Field"},
{"id":"FIELD_OWNER_ROLE","name":"Owner Role","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_TPM","name":"tpm"},{"id":"OPT_PRODUCER","name":"producer_system_designer"},{"id":"OPT_QA","name":"qa_engineer"},{"id":"OPT_LIVEOPS","name":"liveops_community"}]},
{"id":"FIELD_MODULE","name":"Module","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_ENGINEERING","name":"engineering"}]},
{"id":"FIELD_PM_STATUS","name":"PM Status","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_CANDIDATE","name":"candidate"},{"id":"OPT_COMMITTED","name":"committed"},{"id":"OPT_BLOCKED_PM","name":"blocked"},{"id":"OPT_READY_PM","name":"ready"},{"id":"OPT_PR_WATCH_PM","name":"pr_watch"},{"id":"OPT_DONE_PM","name":"done"},{"id":"OPT_DEFERRED","name":"deferred"}]},
{"id":"FIELD_WORKFLOW_PHASE","name":"Workflow Phase","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_EXECUTION","name":"execution"},{"id":"OPT_BLOCKED_PHASE","name":"blocked"},{"id":"OPT_CLOSEOUT","name":"closeout"},{"id":"OPT_PR_WATCH_PHASE","name":"pr_watch"},{"id":"OPT_DONE_PHASE","name":"done"}]},
{"id":"FIELD_PRIORITY","name":"Priority","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_P0","name":"P0"},{"id":"OPT_P1","name":"P1"},{"id":"OPT_P2","name":"P2"},{"id":"OPT_P3","name":"P3"}]},
{"id":"FIELD_BLOCKED","name":"Blocked Reason","type":"ProjectV2Field"},
{"id":"FIELD_WORKTREE","name":"Canonical Worktree","type":"ProjectV2Field"},
{"id":"FIELD_PR","name":"PR","type":"ProjectV2Field"},
{"id":"FIELD_TIER","name":"Test Tier Required","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_NA","name":"n/a"}]},
{"id":"FIELD_UPDATED","name":"Last PM Update","type":"ProjectV2Field"}]}
JSON
  exit 0
fi

if [[ "$1" == "project" && "$2" == "item-add" ]]; then
  url=""
  while [[ $# -gt 0 ]]; do
    if [[ "$1" == "--url" ]]; then
      url="$2"
      break
    fi
    shift
  done
  issue="${url##*/}"
  printf '{"id":"ITEM_%s","content":{"url":"%s"}}\n' "$issue" "$url"
  exit 0
fi

if [[ "$1" == "project" && "$2" == "item-edit" ]]; then
  printf '{}\n'
  exit 0
fi

echo "unexpected gh invocation: $*" >&2
exit 9
SH
chmod +x "$TMPDIR/bin/gh"
export PATH="$TMPDIR/bin:$PATH"
export GH_CALL_LOG="$TMPDIR/gh-calls.log"
export GH_MOCK_COUNTER="$TMPDIR/gh-issue-counter"
printf '4000\n' > "$GH_MOCK_COUNTER"
: > "$GH_CALL_LOG"

python3 - "$TMPDIR" "$ROOT_DIR" <<'PY'
from pathlib import Path
import json
import hashlib
import re
import shutil
import sys

root = Path(sys.argv[1])
source_root = Path(sys.argv[2])


def rewrite_missing_absolute_source_refs() -> None:
    replacement_dir = root / ".pm/evidence/portable-source-refs"
    absolute_ref_pattern = re.compile(r"/(?:home|Users)/[^\s\"']+")
    for path in (root / ".pm").rglob("*.yaml"):
        text = path.read_text(encoding="utf-8")
        replacements: dict[str, str] = {}
        for match in absolute_ref_pattern.finditer(text):
            raw_ref = match.group(0).rstrip(",")
            raw_path = raw_ref.split("#", 1)[0]
            if Path(raw_path).exists():
                continue
            digest = hashlib.sha256(raw_path.encode("utf-8")).hexdigest()[:16]
            replacement = replacement_dir / f"{digest}.jsonl"
            replacement.parent.mkdir(parents=True, exist_ok=True)
            if not replacement.exists():
                replacement.write_text('{"portable_placeholder": true}\n', encoding="utf-8")
            fragment = raw_ref[len(raw_path):]
            replacements[raw_ref] = f"{replacement.relative_to(root)}{fragment}"
        for old, new in replacements.items():
            text = text.replace(old, new)
        if replacements:
            path.write_text(text, encoding="utf-8")


rewrite_missing_absolute_source_refs()


def parse_inline_list(value):
    value = value.strip()
    if not (value.startswith("[") and value.endswith("]")):
        return None
    inner = value[1:-1].strip()
    if not inner:
        return []
    return [item.strip().strip('"').strip("'") for item in inner.split(",") if item.strip()]


def parse_simple_yaml(path: Path):
    parsed = {}
    current_list_key = None
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.rstrip()
        if not line or line.lstrip().startswith("#"):
            continue
        if line.startswith("  - ") and current_list_key:
            parsed.setdefault(current_list_key, []).append(line[4:].strip().strip('"'))
            continue
        current_list_key = None
        if line.startswith(" ") or ":" not in line:
            continue
        key, value = line.split(":", 1)
        value = value.strip()
        inline_list = parse_inline_list(value)
        if inline_list is not None:
            parsed[key] = inline_list
        elif value == "":
            parsed[key] = []
            current_list_key = key
        elif value == "null":
            parsed[key] = None
        else:
            parsed[key] = value.strip('"')
    return parsed


def iter_source_refs(path: Path):
    current_list_key = None
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        stripped = raw_line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if current_list_key and stripped.startswith("- "):
            yield stripped[2:].strip().strip('"')
            continue
        current_list_key = None
        if ":" not in stripped:
            continue
        key, value = stripped.split(":", 1)
        key = key.strip()
        value = value.strip()
        inline_list = parse_inline_list(value)
        if key in {"source_refs", "updated_from"} and inline_list is not None:
            for item in inline_list:
                yield item
            continue
        if key in {"source_refs", "updated_from"} and not value:
            current_list_key = key
            continue
        if key == "source_ref" and value and value != "null":
            yield value.strip('"')


def mirror_source_ref(source_ref: str) -> None:
    path = str(source_ref).split("#", 1)[0].strip()
    if not path:
        return
    if path.startswith(("http://", "https://")):
        return
    resolved = Path(path).expanduser()
    if resolved.is_absolute():
        return
    target = root / resolved
    if target.exists():
        return
    source = source_root / resolved
    if not source.exists():
        return
    target.parent.mkdir(parents=True, exist_ok=True)
    if source.is_dir():
        shutil.copytree(source, target)
    else:
        shutil.copy2(source, target)


for task_path in (root / ".pm/tasks").glob("*.yaml"):
    payload = parse_simple_yaml(task_path)
    for source_ref in payload.get("source_refs") or []:
        mirror_source_ref(str(source_ref))
    execution_log = payload.get("execution_log_path")
    if execution_log:
        mirror_source_ref(str(execution_log))

for memory_path in list((root / ".pm/roles").glob("*/memory/*.yaml")) + list((root / ".pm/shared/memory").glob("*.yaml")):
    for source_ref in iter_source_refs(memory_path):
        mirror_source_ref(str(source_ref))

for working_memory_path in (root / ".pm/working_memory").glob("*.yaml"):
    for source_ref in iter_source_refs(working_memory_path):
        mirror_source_ref(str(source_ref))

for stage_path in (root / ".pm/stage").glob("*.yaml"):
    for source_ref in iter_source_refs(stage_path):
        mirror_source_ref(str(source_ref))

signals_path = root / ".pm/inbox/signals.jsonl"
if signals_path.exists():
    for raw_line in signals_path.read_text(encoding="utf-8").splitlines():
        raw_line = raw_line.strip()
        if not raw_line:
            continue
        mirror_source_ref(str(json.loads(raw_line).get("source_ref") or ""))
PY

python3 - "$TMPDIR" <<'PY'
from pathlib import Path
import sys

root = Path(sys.argv[1])

for active_path in (root / ".pm/roles").glob("*/memory/active.yaml"):
    role = active_path.parts[-3]
    active_path.write_text(
        f"version: 1\nrole: {role}\nkind: memory_active\nrecords: []\n",
        encoding="utf-8",
    )

for superseded_path in (root / ".pm/roles").glob("*/memory/superseded.yaml"):
    role = superseded_path.parts[-3]
    superseded_path.write_text(
        f"version: 1\nrole: {role}\nkind: memory_superseded\nrecords: []\n",
        encoding="utf-8",
    )

(root / ".pm/shared/memory/active.yaml").write_text(
    "version: 1\nscope: shared\nkind: memory_active\nrecords: []\n",
    encoding="utf-8",
)
(root / ".pm/shared/memory/superseded.yaml").write_text(
    "version: 1\nscope: shared\nkind: memory_superseded\nrecords: []\n",
    encoding="utf-8",
)
(root / ".pm/stage/current.yaml").write_text(
    "version: 1\ncurrent_stage: null\ncandidate_stage: null\nclaim_envelope: null\ndecision_date: null\nupdated_from: []\nblocking_tasks: []\n",
    encoding="utf-8",
)
(root / ".pm/stage/gate.yaml").write_text(
    "version: 1\ngate_id: null\nstatus: draft\nlane_status: []\nblocking_tasks: []\nupdated_from: []\n",
    encoding="utf-8",
)

for path in (root / ".pm/tasks").glob("*.yaml"):
    path.unlink()
for path in (root / ".pm/tasks").glob("*.execution.md"):
    path.unlink()
for path in (root / ".pm/working_memory").glob("*.yaml"):
    path.unlink()

(root / ".pm/registry/tasks.yaml").write_text(
    'version: 2\nidentity_key: task_uid\ngenerated_from: ".pm/tasks/*.yaml"\ntasks: []\n',
    encoding="utf-8",
)

for backlog_path in (root / ".pm/roles").glob("*/backlog/*.yaml"):
    role = backlog_path.parts[-3]
    status = backlog_path.stem
    backlog_path.write_text(
        f"version: 1\nrole: {role}\nstatus: {status}\ntasks: []\n",
        encoding="utf-8",
    )
PY

cat > "$TMPDIR/.pm/evidence/bootstrap.md" <<'EOF'
# bootstrap evidence

- issue: viewer smoke blocked on startup bridge init
EOF

SIGNAL_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/promote-signal.sh" \
  --source-type bootstrap_evidence \
  --source-ref .pm/evidence/bootstrap.md \
  --role-hint qa_engineer \
  --severity high \
  --summary "viewer smoke blocked on startup bridge init" \
  --create-task \
  --related-prd doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.design.md \
  --acceptance "blocked task exists in qa backlog" \
  --json)"

TASK_UID="$(python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["task"]["task_uid"])' <<<"$SIGNAL_JSON")"
TASK_LOG_PATH=".pm/evidence/${TASK_UID}.execution.md"
cat > "$TMPDIR/$TASK_LOG_PATH" <<EOF
# $TASK_UID Execution Log

- task_uid: $TASK_UID
- title: viewer smoke blocked on startup bridge init
- owner_role: qa_engineer
- worktree_hint: null

## 2026-03-30 22:30:00 CST / qa_engineer
- 完成内容: viewer smoke blocked on startup bridge init.
- 遗留事项: needs escalation into candidate task and stage gate.
- Action: 记录 viewer smoke 启动阻断并将其提升到 blocked task。
- Validation Command: PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/promote-signal.sh" --source-type bootstrap_evidence --source-ref .pm/evidence/bootstrap.md --role-hint qa_engineer --severity high --summary "viewer smoke blocked on startup bridge init" --create-task --json
- Expected Result: 生成 candidate task 与 execution log 证据，后续可以转 blocked 并挂到 stage gate。
- Actual Result: 已生成 task、execution log 与 bootstrap evidence 引用，随后任务被转入 blocked backlog。
- Blocker / Next Action: needs escalation into candidate task and stage gate.
EOF

MOVE_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/move-task.sh" \
  --task-uid "$TASK_UID" \
  --to-status blocked \
  --json)"

QA_MEMORY_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/promote-memory.sh" \
  --signal-id "$(python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["signal_id"])' <<<"$SIGNAL_JSON")" \
  --role qa_engineer \
  --topic viewer.startup.blocker \
  --tag failure_signature \
  --tag gate \
  --promotion-reason failure_signature \
  --effective-at 2026-03-20T10:00:00+08:00 \
  --json)"

PRODUCER_SIGNAL_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/promote-signal.sh" \
  --source-type task_execution_log \
  --source-ref "$TASK_LOG_PATH" \
  --role-hint producer_system_designer \
  --severity medium \
  --summary "current stage remains internal_playable_alpha_late" \
  --json)"

PRODUCER_SIGNAL_ID="$(python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["signal_id"])' <<<"$PRODUCER_SIGNAL_JSON")"

PRODUCER_MEMORY_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/promote-memory.sh" \
  --signal-id "$PRODUCER_SIGNAL_ID" \
  --role producer_system_designer \
  --topic stage.current \
  --tag stage \
  --tag claim_envelope \
  --promotion-reason stage_decision \
  --json)"

SHARED_SIGNAL_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/promote-signal.sh" \
  --source-type task_execution_log \
  --source-ref "$TASK_LOG_PATH" \
  --role-hint producer_system_designer \
  --severity medium \
  --summary "claim envelope remains internal_only" \
  --json)"

SHARED_SIGNAL_ID="$(python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["signal_id"])' <<<"$SHARED_SIGNAL_JSON")"

SHARED_MEMORY_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/promote-memory.sh" \
  --signal-id "$SHARED_SIGNAL_ID" \
  --scope shared \
  --role producer_system_designer \
  --topic gate.claim_envelope \
  --tag claim_envelope \
  --promotion-reason stage_decision \
  --json)"

NOISE_SIGNAL_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/promote-signal.sh" \
  --source-type task_execution_log \
  --source-ref "$TASK_LOG_PATH" \
  --role-hint qa_engineer \
  --severity low \
  --summary "reran smoke once after cache clear" \
  --json)"

NOISE_SIGNAL_ID="$(python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["signal_id"])' <<<"$NOISE_SIGNAL_JSON")"

REJECTED_MEMORY_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/promote-memory.sh" \
  --signal-id "$NOISE_SIGNAL_ID" \
  --role qa_engineer \
  --reject-reason one_off_operation \
  --json)"

python3 - "$REJECTED_MEMORY_JSON" <<'PY'
import json
import sys

payload = json.loads(sys.argv[1])
if payload.get("state_sink") != "github_intake_cache":
    raise SystemExit(f"memory rejection should report the actual local mirror sink: {payload}")
PY

MAPPING_ONLY_SIGNAL_ID="sig_mapping_only_000000000000000000000001"
python3 - "$TMPDIR" "$MAPPING_ONLY_SIGNAL_ID" <<'PY'
from __future__ import annotations

import json
from collections import OrderedDict
from pathlib import Path
import sys

root = Path(sys.argv[1])
signal_id = sys.argv[2]
mapping_path = root / ".pm/github-project-sync/tasks.json"
mapping = json.loads(mapping_path.read_text(encoding="utf-8"), object_pairs_hook=OrderedDict)
tasks = mapping.setdefault("tasks", OrderedDict())
tasks["task_mapping_only_00000000000000000001"] = OrderedDict(
    [
        ("task_uid", "task_mapping_only_00000000000000000001"),
        ("issue_url", "https://github.com/eng-cc/oasis7/issues/2999"),
        ("status", "candidate"),
        ("owner_role", "qa_engineer"),
        ("title", "mapping-only signal should fail closed"),
        ("source_signal", signal_id),
        ("source_type", "reflection"),
        ("source_refs", ["doc/engineering/workflow/source-of-truth.md"]),
        ("severity", "low"),
    ]
)
mapping_path.write_text(json.dumps(mapping, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
PY

MAPPING_ONLY_STDERR="$TMPDIR/mapping-only-promote-memory.stderr"
if PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/promote-memory.sh" \
  --signal-id "$MAPPING_ONLY_SIGNAL_ID" \
  --role qa_engineer \
  --reject-reason one_off_operation \
  --json > /dev/null 2>"$MAPPING_ONLY_STDERR"; then
  echo "required-tier-smoke: expected mapping-only GitHub signal memory decision to fail closed" >&2
  exit 1
fi
if ! grep -q "requires local intake mirror entry" "$MAPPING_ONLY_STDERR"; then
  echo "required-tier-smoke: mapping-only memory decision failure did not mention missing intake mirror entry" >&2
  cat "$MAPPING_ONLY_STDERR" >&2
  exit 1
fi
python3 - "$TMPDIR" <<'PY'
from __future__ import annotations

import json
from pathlib import Path
import sys

root = Path(sys.argv[1])
mapping_path = root / ".pm/github-project-sync/tasks.json"
mapping = json.loads(mapping_path.read_text(encoding="utf-8"))
tasks = mapping.get("tasks") or {}
tasks.pop("task_mapping_only_00000000000000000001", None)
mapping_path.write_text(json.dumps(mapping, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
PY

LIVEOPS_SIGNAL_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/promote-signal.sh" \
  --source-type incident \
  --source-ref "$TASK_LOG_PATH" \
  --role-hint liveops_community \
  --severity high \
  --summary "community escalation still needs owner follow-up" \
  --json)"

cat > "$TMPDIR/.pm/roles/producer_system_designer/memory/superseded.yaml" <<EOF
version: 1
role: producer_system_designer
kind: memory_superseded
records:
  - id: MEM-PRODUCER-0000
    role: producer_system_designer
    topic: stage.current
    summary: "current stage remained internal_playable_alpha_mid"
    source_refs:
      - $TASK_LOG_PATH
    tags:
      - stage
    effective_at: 2026-03-15T10:00:00+08:00
    last_reviewed_at: 2026-03-20T10:00:00+08:00
    status: superseded
    confidence: confirmed
    promotion_reason: stage_decision
    superseded_by: MEM-PRODUCER-0001
    superseded_at: 2026-03-30T10:00:00+08:00
    supersede_reason: stage_upgraded
EOF

if PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/stage-lint.sh" >/dev/null 2>&1; then
  echo "required-tier-smoke: expected stage-lint to fail before canonical stage files are updated" >&2
  exit 1
fi

SET_STAGE_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/set-stage.sh" \
  --current-stage internal_playable_alpha_late \
  --candidate-stage limited_preview_readiness \
  --claim-envelope internal_only \
  --decision-date 2026-03-30 \
  --gate-id GATE-ALPHA-001 \
  --gate-status blocked \
  --lane-status qa=blocked \
  --lane-status liveops=monitor \
  --blocking-task "$TASK_UID" \
  --source-ref "$TASK_LOG_PATH" \
  --json)"

FAILED_SET_STAGE_STDERR="$TMPDIR/set-stage-fail.stderr"
if PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/set-stage.sh" \
  --clear-blocking-tasks \
  --source-ref "$TASK_LOG_PATH" \
  --json > /dev/null 2>"$FAILED_SET_STAGE_STDERR"; then
  echo "required-tier-smoke: expected set-stage to fail when clearing a still-blocked task from blocking_tasks" >&2
  exit 1
fi
python3 - "$TMPDIR" "$TASK_UID" <<'PY'
from pathlib import Path
import sys

root = Path(sys.argv[1])
task_uid = sys.argv[2]

for path_str in (".pm/stage/current.yaml", ".pm/stage/gate.yaml"):
    text = (root / path_str).read_text(encoding="utf-8")
    if f"blocking_tasks:\n  - {task_uid}\n" not in text:
        raise SystemExit(f"failed set-stage should not persist cleared blocker in {path_str}")
PY

python3 - "$TMPDIR" "$TASK_UID" <<'PY'
from pathlib import Path
import sys

root = Path(sys.argv[1])
task_uid = sys.argv[2]

for path_str in (".pm/stage/current.yaml", ".pm/stage/gate.yaml"):
    path = root / path_str
    text = path.read_text(encoding="utf-8")
    needle = f"blocking_tasks:\n  - {task_uid}\n"
    if needle not in text:
        raise SystemExit(f"expected blocking task entry not found in {path}")
    path.write_text(text.replace(needle, "blocking_tasks: []\n"), encoding="utf-8")
PY

STAGE_DRIFT_STDERR="$TMPDIR/stage-drift.stderr"
if PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/stage-lint.sh" > /dev/null 2>"$STAGE_DRIFT_STDERR"; then
  echo "required-tier-smoke: expected stage-lint to fail when blocked task drifts out of stage/gate blocking_tasks" >&2
  exit 1
fi
if ! grep -q "blocked task missing from stage/gate blocking_tasks: $TASK_UID" "$STAGE_DRIFT_STDERR"; then
  echo "required-tier-smoke: stage drift failure did not mention missing blocked task $TASK_UID" >&2
  exit 1
fi

SET_STAGE_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/set-stage.sh" \
  --current-stage internal_playable_alpha_late \
  --candidate-stage limited_preview_readiness \
  --claim-envelope internal_only \
  --decision-date 2026-03-30 \
  --gate-id GATE-ALPHA-001 \
  --gate-status blocked \
  --lane-status qa=blocked \
  --lane-status liveops=monitor \
  --blocking-task "$TASK_UID" \
  --source-ref "$TASK_LOG_PATH" \
  --json)"

BROKEN_BACKLOG="$TMPDIR/.pm/roles/qa_engineer/backlog/blocked.yaml"
cp "$BROKEN_BACKLOG" "$BROKEN_BACKLOG.bak"
printf 'this is not a valid backlog doc\n' > "$BROKEN_BACKLOG"
WORKFLOW_FAIL_STDERR="$TMPDIR/workflow-report-fail.stderr"
if ! PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/workflow-report.sh" --role qa_engineer --phase start --task-uid "$TASK_UID" --json > /dev/null 2>"$WORKFLOW_FAIL_STDERR"; then
  echo "required-tier-smoke: expected workflow-report to regenerate malformed generated backlog view" >&2
  cat "$WORKFLOW_FAIL_STDERR" >&2
  exit 1
fi
mv "$BROKEN_BACKLOG.bak" "$BROKEN_BACKLOG"

python3 - "$TMPDIR" <<'PY'
from pathlib import Path
import sys

root = Path(sys.argv[1])

registry_path = root / ".pm/registry/tasks.yaml"
if registry_path.exists():
    registry_path.unlink()

for backlog_path in (root / ".pm/roles").glob("*/backlog/*.yaml"):
    backlog_path.unlink()
PY

REGEN_ROLE_REPORT_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/role-report.sh" --role qa_engineer --json)"

python3 - "$TMPDIR" "$TASK_UID" "$REGEN_ROLE_REPORT_JSON" <<'PY'
from __future__ import annotations

import json
from pathlib import Path
import sys

root = Path(sys.argv[1])
task_uid = sys.argv[2]
report = json.loads(sys.argv[3])

registry_path = root / ".pm/registry/tasks.yaml"
if not registry_path.exists():
    raise SystemExit("role-report should regenerate .pm/registry/tasks.yaml when it is missing")

for role_dir in sorted((root / ".pm/roles").glob("*")):
    if not role_dir.is_dir():
        continue
    for lane in ("candidate", "committed", "blocked", "done"):
        backlog_path = role_dir / "backlog" / f"{lane}.yaml"
        if not backlog_path.exists():
            raise SystemExit(f"role-report should regenerate missing backlog view: {backlog_path}")

qa_payload = report["roles"]["qa_engineer"]
if qa_payload["backlog_counts"]["blocked"] != 1:
    raise SystemExit("role-report should still report one blocked qa task after regenerating views")
blocked_tasks = qa_payload["tasks"]["blocked"]
if len(blocked_tasks) != 1 or blocked_tasks[0]["task_uid"] != task_uid:
    raise SystemExit("role-report regenerated views but lost the blocked qa task entry")
PY

WORKFLOW_START_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/workflow-report.sh" --role qa_engineer --phase start --task-uid "$TASK_UID" --json)"
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/memory-lint.sh" >/dev/null
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/lint.sh" >/dev/null
MEMORY_REPORT_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/memory-report.sh" --json)"
ROLE_REPORT_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/role-report.sh" --role qa_engineer --json)"
APPEND_LOG_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/append-execution-log.sh" \
  --task-uid "$TASK_UID" \
  --role qa_engineer \
  --completed "required-tier smoke appended structured evidence" \
  --pending "none" \
  --action "exercise append-execution-log wrapper" \
  --validation-command "workflow-lint --phase current" \
  --expected-result "current-task lint accepts a started task without closeout or PR evidence" \
  --actual-result "append command wrote a complete execution-log entry" \
  --blocker-next-action "none" \
  --json)"
APPEND_CROSS_ROLE_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/append-execution-log.sh" \
  --task-uid "$TASK_UID" \
  --role agent_engineer \
  --completed "required-tier smoke appended cross-role structured evidence" \
  --pending "none" \
  --action "exercise append-execution-log wrapper for a non-owner role" \
  --validation-command "role-report --task-uid" \
  --expected-result "task collaboration view includes the non-owner role execution entry" \
  --actual-result "append command wrote a complete non-owner role execution-log entry" \
  --blocker-next-action "none" \
  --json)"
WORKFLOW_CURRENT_LINT_STDOUT="$TMPDIR/workflow-current-lint.stdout"
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/workflow-lint.sh" --task-uid "$TASK_UID" --phase current >"$WORKFLOW_CURRENT_LINT_STDOUT"
ROLE_REPORT_TASK_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/role-report.sh" --role qa_engineer --task-uid "$TASK_UID" --json)"
EMPTY_LOG_TASK_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/new-task.sh" \
  --owner-role qa_engineer \
  --title "empty execution log current lint fixture" \
  --priority P2 \
  --source-ref .pm/evidence/bootstrap.md \
  --related-prd doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.design.md \
  --acceptance "current workflow lint fails when execution log only has the generated template" \
  --json)"
EMPTY_LOG_TASK_UID="$(python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["task_uid"])' <<<"$EMPTY_LOG_TASK_JSON")"
EMPTY_LOG_CURRENT_LINT_STDOUT="$TMPDIR/empty-log-current-lint.stdout"
set +e
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/workflow-lint.sh" --task-uid "$EMPTY_LOG_TASK_UID" --phase current >"$EMPTY_LOG_CURRENT_LINT_STDOUT" 2>&1
EMPTY_LOG_CURRENT_LINT_STATUS=$?
set -e
WORKFLOW_CLOSE_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/workflow-report.sh" --role qa_engineer --phase close --task-uid "$TASK_UID" --json)"
mkdir -p "$TMPDIR/.pm/working_memory"
cat > "$TMPDIR/.pm/working_memory/$TASK_UID.yaml" <<EOF
version: 1
task_uid: $TASK_UID
role: qa_engineer
worktree_hint: null
entries:
  - entry_id: WM-0001
    entry_kind: decision
    summary: "viewer startup blocker should be reflected into follow-up review"
    source_refs:
      - $TASK_LOG_PATH
    captured_at: 2026-03-30T22:40:00+08:00
    expires_at: 2026-04-01T22:40:00+08:00
    promoted_to: []
EOF
WORKFLOW_CLOSE_WITH_WM_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/workflow-report.sh" --role qa_engineer --phase close --task-uid "$TASK_UID" --json)"
WORKFLOW_REVIEW_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/workflow-report.sh" --role producer_system_designer --phase review --json)"
STAGE_REPORT_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/stage-report.sh" --json)"

CLOSEOUT_TASK_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/new-task.sh" \
  --owner-role qa_engineer \
  --title "closeout helper smoke task" \
  --priority P2 \
  --source-ref .pm/evidence/bootstrap.md \
  --related-prd doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.design.md \
  --acceptance "task closeout helper can close the task only after fresh verification" \
  --json)"
CLOSEOUT_TASK_UID="$(python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["task_uid"])' <<<"$CLOSEOUT_TASK_JSON")"
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/move-task.sh" --task-uid "$CLOSEOUT_TASK_UID" --to-status committed >/dev/null
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/workflow-report.sh" --role qa_engineer --phase start --task-uid "$CLOSEOUT_TASK_UID" --json >/dev/null
set +e
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/task-closeout.sh" --role qa_engineer --task-uid "$CLOSEOUT_TASK_UID" > /dev/null 2>"$TMPDIR/task-closeout-missing-verify.err"
TASK_CLOSEOUT_MISSING_VERIFY_STATUS=$?
set -e
if [[ "$TASK_CLOSEOUT_MISSING_VERIFY_STATUS" == "0" ]]; then
  echo "required-tier-smoke: expected task-closeout to reject done closeout without --verify-command" >&2
  exit 1
fi
TASK_CLOSEOUT_NO_VERIFY_STATE_JSON="$(python3 - "$TMPDIR" "$CLOSEOUT_TASK_UID" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
task_uid = sys.argv[2]
task_path = root / ".pm" / "tasks" / f"{task_uid}.yaml"

if task_path.is_file():
    fields = {}
    for raw in task_path.read_text(encoding="utf-8").splitlines():
        if not raw or raw.startswith(" ") or raw.startswith("-"):
            continue
        key, sep, value = raw.partition(":")
        if not sep:
            continue
        fields[key.strip()] = value.strip()
else:
    mapping = json.loads((root / ".pm/github-project-sync/tasks.json").read_text(encoding="utf-8"))
    fields = (mapping.get("tasks") or {}).get(task_uid) or {}

print(
    json.dumps(
        {
            "status": fields.get("status"),
            "last_closed_at": fields.get("last_closed_at"),
            "last_verified_at": fields.get("last_verified_at"),
            "last_verification_status": fields.get("last_verification_status"),
        },
        ensure_ascii=False,
    )
)
PY
)"
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/workflow-report.sh" --role qa_engineer --phase close --task-uid "$CLOSEOUT_TASK_UID" --json >/dev/null
set +e
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/move-task.sh" --task-uid "$CLOSEOUT_TASK_UID" --to-status done >/dev/null 2>"$TMPDIR/task-closeout-bypass.err"
TASK_CLOSEOUT_BYPASS_STATUS=$?
set -e
TASK_CLOSEOUT_BYPASS_STATE_JSON="$(python3 - "$TMPDIR" "$CLOSEOUT_TASK_UID" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
task_uid = sys.argv[2]
task_path = root / ".pm" / "tasks" / f"{task_uid}.yaml"

if task_path.is_file():
    fields = {}
    for raw in task_path.read_text(encoding="utf-8").splitlines():
        if not raw or raw.startswith(" ") or raw.startswith("-"):
            continue
        key, sep, value = raw.partition(":")
        if not sep:
            continue
        fields[key.strip()] = value.strip()
else:
    mapping = json.loads((root / ".pm/github-project-sync/tasks.json").read_text(encoding="utf-8"))
    fields = (mapping.get("tasks") or {}).get(task_uid) or {}

print(
    json.dumps(
        {
            "status": fields.get("status"),
            "last_closed_at": fields.get("last_closed_at"),
            "last_verified_at": fields.get("last_verified_at"),
            "last_verification_status": fields.get("last_verification_status"),
        },
        ensure_ascii=False,
    )
)
PY
)"
TASK_CLOSEOUT_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/task-closeout.sh" --role qa_engineer --task-uid "$CLOSEOUT_TASK_UID" --verify-command "printf 'closeout verification ok\n'" --json)"
set +e
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/claim-ready.sh" \
  --claim-type ready_for_pr \
  --verify-command "touch '$TMPDIR/closed-task-ready-side-effect'" \
  --task-uid "$CLOSEOUT_TASK_UID" \
  --json >"$TMPDIR/closed-task-ready-claim.json" 2>"$TMPDIR/closed-task-ready-claim.err"
CLOSED_TASK_READY_CLAIM_STATUS=$?
set -e
CLOSED_TASK_READY_CLAIM_STATE_JSON="$(python3 - "$TMPDIR" "$CLOSEOUT_TASK_UID" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
task_uid = sys.argv[2]
task_path = root / ".pm" / "tasks" / f"{task_uid}.yaml"

if task_path.is_file():
    fields = {}
    for raw in task_path.read_text(encoding="utf-8").splitlines():
        if not raw or raw.startswith(" ") or raw.startswith("-"):
            continue
        key, sep, value = raw.partition(":")
        if not sep:
            continue
        fields[key.strip()] = value.strip().strip('"')
else:
    mapping = json.loads((root / ".pm/github-project-sync/tasks.json").read_text(encoding="utf-8"))
    fields = (mapping.get("tasks") or {}).get(task_uid) or {}

print(
    json.dumps(
        {
            "last_claim_type": fields.get("last_claim_type"),
            "last_verify_command": fields.get("last_verify_command"),
            "last_verified_at": fields.get("last_verified_at"),
            "last_verification_status": fields.get("last_verification_status"),
        },
        ensure_ascii=False,
    )
)
PY
)"
CLOSED_TASK_READY_CLAIM_STDERR="$(cat "$TMPDIR/closed-task-ready-claim.err")"

MISSING_ACTUAL_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/new-task.sh" \
  --owner-role qa_engineer \
  --title "missing actual result lint fixture" \
  --priority P2 \
  --source-ref .pm/evidence/bootstrap.md \
  --json)"
MISSING_ACTUAL_TASK_UID="$(python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["task_uid"])' <<<"$MISSING_ACTUAL_JSON")"
MISSING_ACTUAL_LOG_PATH=".pm/evidence/${MISSING_ACTUAL_TASK_UID}.execution.md"
python3 - "$TMPDIR" "$MISSING_ACTUAL_TASK_UID" "$MISSING_ACTUAL_LOG_PATH" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
task_uid = sys.argv[2]
log_path = sys.argv[3]
mapping_path = root / ".pm/github-project-sync/tasks.json"
mapping = json.loads(mapping_path.read_text(encoding="utf-8"))
record = (mapping.get("tasks") or {}).get(task_uid)
if not record:
    raise SystemExit(f"missing mapping record for {task_uid}")
record["execution_log_path"] = log_path
record["updated_at"] = "2026-03-30T23:10:00+08:00"
mapping_path.write_text(json.dumps(mapping, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY
cat > "$TMPDIR/$MISSING_ACTUAL_LOG_PATH" <<EOF
# $MISSING_ACTUAL_TASK_UID Execution Log

- task_uid: $MISSING_ACTUAL_TASK_UID
- title: missing actual result lint fixture
- owner_role: qa_engineer
- worktree_hint: null

<!-- Append entries using:
Example:
  ## YYYY-MM-DD HH:MM:SS CST / role_name
  - 完成内容: ...
  - 遗留事项: ...
  - Action: ...
  - Validation Command: ...
  - Expected Result: ...
  - Actual Result: ...
  - Blocker / Next Action: ...
-->

## 2026-05-23 00:05:00 CST / qa_engineer
- 完成内容: prepared a negative fixture for step-evidence lint.
- 遗留事项: lint should reject this entry because Actual Result is missing.
- Action: 为 task-execution-log-lint 构造缺失 Actual Result 的 started task。
- Validation Command: PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/task-execution-log-lint.sh"
- Expected Result: lint 以明确的 missing Actual Result failure 拒绝该 task。
- Blocker / Next Action: remove the fixture after the negative assertion if future smoke refactors need a clean PM root.
EOF
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/move-task.sh" --task-uid "$MISSING_ACTUAL_TASK_UID" --to-status committed >/dev/null
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/workflow-report.sh" --role qa_engineer --phase start --task-uid "$MISSING_ACTUAL_TASK_UID" --json >/dev/null
set +e
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/workflow-lint.sh" --task-uid "$MISSING_ACTUAL_TASK_UID" --phase current >"$TMPDIR/task-execution-log-missing-actual.err" 2>&1
TASK_EXECUTION_LOG_MISSING_ACTUAL_STATUS=$?
set -e
if [[ "$TASK_EXECUTION_LOG_MISSING_ACTUAL_STATUS" == "0" ]]; then
  echo "required-tier-smoke: expected workflow-lint to reject a started task missing Actual Result" >&2
  exit 1
fi
if ! rg -q "missing Actual Result" "$TMPDIR/task-execution-log-missing-actual.err"; then
  echo "required-tier-smoke: expected missing Actual Result failure signature" >&2
  exit 1
fi

RESULT_JSON="$(python3 - "$TMPDIR" "$SIGNAL_JSON" "$MOVE_JSON" "$QA_MEMORY_JSON" "$PRODUCER_MEMORY_JSON" "$SHARED_MEMORY_JSON" "$REJECTED_MEMORY_JSON" "$LIVEOPS_SIGNAL_JSON" "$SET_STAGE_JSON" "$MEMORY_REPORT_JSON" "$ROLE_REPORT_JSON" "$REGEN_ROLE_REPORT_JSON" "$WORKFLOW_START_JSON" "$WORKFLOW_CLOSE_JSON" "$WORKFLOW_CLOSE_WITH_WM_JSON" "$WORKFLOW_REVIEW_JSON" "$STAGE_REPORT_JSON" "$TASK_CLOSEOUT_JSON" "$CLOSEOUT_TASK_UID" "$TASK_CLOSEOUT_MISSING_VERIFY_STATUS" "$TASK_CLOSEOUT_NO_VERIFY_STATE_JSON" "$TASK_CLOSEOUT_BYPASS_STATUS" "$TASK_CLOSEOUT_BYPASS_STATE_JSON" "$CLOSED_TASK_READY_CLAIM_STATUS" "$CLOSED_TASK_READY_CLAIM_STATE_JSON" "$CLOSED_TASK_READY_CLAIM_STDERR" "$APPEND_LOG_JSON" "$ROLE_REPORT_TASK_JSON" "$WORKFLOW_CURRENT_LINT_STDOUT" "$APPEND_CROSS_ROLE_JSON" "$EMPTY_LOG_CURRENT_LINT_STATUS" "$EMPTY_LOG_CURRENT_LINT_STDOUT" "$TASK_EXECUTION_LOG_MISSING_ACTUAL_STATUS" "$TMPDIR/task-execution-log-missing-actual.err" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

signal_payload = json.loads(sys.argv[2])
move_payload = json.loads(sys.argv[3])
qa_memory = json.loads(sys.argv[4])
producer_memory = json.loads(sys.argv[5])
shared_memory = json.loads(sys.argv[6])
rejected_memory = json.loads(sys.argv[7])
liveops_signal = json.loads(sys.argv[8])
set_stage = json.loads(sys.argv[9])
memory_report = json.loads(sys.argv[10])
role_report = json.loads(sys.argv[11])
regen_role_report = json.loads(sys.argv[12])
workflow_start = json.loads(sys.argv[13])
workflow_close = json.loads(sys.argv[14])
workflow_close_with_wm = json.loads(sys.argv[15])
workflow_review = json.loads(sys.argv[16])
stage_report = json.loads(sys.argv[17])
task_closeout = json.loads(sys.argv[18])
closeout_task_uid = sys.argv[19]
missing_verify_status = int(sys.argv[20])
missing_verify_state = json.loads(sys.argv[21])
bypass_status = int(sys.argv[22])
bypass_state = json.loads(sys.argv[23])
closed_task_ready_claim_status = int(sys.argv[24])
closed_task_ready_claim_state = json.loads(sys.argv[25])
closed_task_ready_claim_stderr = sys.argv[26]
append_log = json.loads(sys.argv[27])
role_report_task = json.loads(sys.argv[28])
workflow_current_lint_stdout = sys.argv[29]
append_cross_role = json.loads(sys.argv[30])
empty_log_lint_status = int(sys.argv[31])
empty_log_lint_stdout = sys.argv[32]
empty_lint_text = open(empty_log_lint_stdout, encoding="utf-8").read()
missing_actual_lint_status = int(sys.argv[33])
missing_actual_lint_stdout = sys.argv[34]
missing_actual_lint_text = open(missing_actual_lint_stdout, encoding="utf-8").read()

if "signal_summary" not in workflow_start:
    if workflow_start.get("status") != "ok" or workflow_start.get("phase") != "start":
        raise SystemExit("GitHub-backed workflow start should record start evidence")
    if workflow_close.get("status") != "ok" or workflow_close.get("phase") != "close":
        raise SystemExit("GitHub-backed workflow close should record close evidence")
    if workflow_review.get("status") != "ok" or workflow_review.get("phase") != "review":
        raise SystemExit("GitHub-backed workflow review should return an authoritative review payload")
    if append_log.get("task_uid") != move_payload["task_uid"]:
        raise SystemExit("append-execution-log should append to the explicit GitHub-backed task")
    if append_cross_role.get("task_uid") != move_payload["task_uid"]:
        raise SystemExit("append-execution-log should allow canonical non-owner GitHub-backed entries")
    if missing_actual_lint_status == 0 or "missing Actual Result" not in missing_actual_lint_text:
        raise SystemExit("workflow-lint current should reject the missing Actual Result fixture")
    if missing_verify_status == 0:
        raise SystemExit("task closeout helper should fail when ready closeout omits --verify-command")
    if missing_verify_state["status"] != "committed":
        raise SystemExit("task closeout helper should leave task status unchanged when verification is missing")
    if bypass_status == 0:
        raise SystemExit("direct move-task done closeout should fail without persisted task_complete evidence")
    if bypass_state["status"] != "committed":
        raise SystemExit("direct move-task done closeout should leave task status unchanged")
    if task_closeout["task_uid"] != closeout_task_uid:
        raise SystemExit("task closeout helper should report the closed task uid")
    if task_closeout["final_status"] != "ready":
        raise SystemExit("task closeout helper should move GitHub-backed tasks to ready by default")
    if task_closeout["claim_verification"]["status"] != "verified":
        raise SystemExit("task closeout helper should include verified claim evidence after fresh verification")
    if task_closeout["claim_verification"]["claim_type"] != "ready_for_pr":
        raise SystemExit("task closeout helper should default GitHub-backed ready closeout to ready_for_pr")
    if closed_task_ready_claim_status != 0:
        raise SystemExit("ready task should accept later ready_for_pr claim evidence")
    if closed_task_ready_claim_state["last_claim_type"] != "ready_for_pr":
        raise SystemExit("ready task should persist ready_for_pr claim evidence")
    if task_closeout["recommended_next_command"] != "./scripts/prepare-task-pr.sh":
        raise SystemExit("task closeout helper should point to prepare-task-pr as the next step")
    print(
        json.dumps(
            {
                "temp_root": sys.argv[1],
                "signal": signal_payload,
                "move": move_payload,
                "qa_memory": qa_memory,
                "producer_memory": producer_memory,
                "shared_memory": shared_memory,
                "rejected_memory": rejected_memory,
                "liveops_signal": liveops_signal,
                "set_stage": set_stage,
                "memory_report": memory_report,
                "role_report": role_report,
                "regen_role_report": regen_role_report,
                "workflow_start": workflow_start,
                "workflow_close": workflow_close,
                "workflow_close_with_wm": workflow_close_with_wm,
                "workflow_review": workflow_review,
                "stage_report": stage_report,
                "task_closeout": task_closeout,
                "append_log": append_log,
                "append_cross_role": append_cross_role,
                "role_report_task": role_report_task,
            },
            ensure_ascii=False,
        )
    )
    raise SystemExit(0)

if workflow_start["signal_summary"]["pending_count"] != 0:
    raise SystemExit("qa workflow start should not treat rejected signal as pending")
if workflow_start["task_context"]["task_uid"] != move_payload["task_uid"]:
    raise SystemExit("workflow start should bind explicit task_uid")
if append_log["task_uid"] != move_payload["task_uid"]:
    raise SystemExit("append-execution-log should append to the explicit task")
if append_cross_role["role"] != "agent_engineer":
    raise SystemExit("append-execution-log should allow canonical non-owner role entries")
if "phase=current" not in open(workflow_current_lint_stdout, encoding="utf-8").read():
    raise SystemExit("workflow-lint --phase current should report current phase success")
empty_lint_text = open(empty_log_lint_stdout, encoding="utf-8").read()
if empty_log_lint_status == 0:
    raise SystemExit("workflow-lint --phase current should fail when the execution log has only template text")
if "execution log missing real entries" not in empty_lint_text:
    raise SystemExit("workflow-lint --phase current should reject template-only execution logs")
task_collaboration = role_report_task.get("task_collaboration")
if not task_collaboration:
    raise SystemExit("role-report --task-uid should include task_collaboration")
if task_collaboration["task_uid"] != move_payload["task_uid"]:
    raise SystemExit("task_collaboration task_uid mismatch")
if task_collaboration["owner_role"] != "qa_engineer":
    raise SystemExit("task_collaboration owner_role mismatch")
if "qa_engineer" not in task_collaboration["execution_roles"]:
    raise SystemExit("task_collaboration should include execution log role")
if "agent_engineer" not in task_collaboration["execution_roles"]:
    raise SystemExit("task_collaboration should include non-owner execution log role")
if task_collaboration["execution_entry_count"] < 3:
    raise SystemExit("task_collaboration should count workflow start plus appended evidence entries")
if not workflow_start["task_context"]["last_started_at"]:
    raise SystemExit("workflow start should record last_started_at")
if not workflow_close["task_context"]["last_closed_at"]:
    raise SystemExit("workflow close should record last_closed_at")
if workflow_close["working_memory_summary"]["entry_count"] != 0:
    raise SystemExit("workflow close should use task-scoped working_memory counts for explicit task_uid")
if workflow_review["signal_summary"]["pending_count"] != 1:
    raise SystemExit("producer workflow review should see one cross-role pending signal")
pending = workflow_review["signal_summary"]["pending_signals"]
if len(pending) != 1 or pending[0]["signal_id"] != liveops_signal["signal_id"]:
    raise SystemExit("producer workflow review missing expected liveops pending signal")
if pending[0]["role_hint"] != "liveops_community":
    raise SystemExit("producer workflow review pending signal role mismatch")
if any(item.get("id") == "review-signals" and "command" in item for item in workflow_review["checklist"]):
    raise SystemExit("workflow review checklist should not suggest promote-signal for pending signal handling")
if any(item.get("id") == "triage-signals" and "command" in item for item in workflow_start["checklist"]):
    raise SystemExit("workflow start checklist should not suggest promote-signal for pending signal handling")
if any(item.get("id") == "codex-review" for item in workflow_close["checklist"]):
    raise SystemExit("workflow close checklist should no longer require local codex review")
if not any(item.get("id") == "prepare-pr-review" for item in workflow_close["checklist"]):
    raise SystemExit("workflow close checklist should point to pre-PR local role review plus GitHub PR watch/fix/merge")
if not any(item.get("id") == "fresh-claim-verification" for item in workflow_close["checklist"]):
    raise SystemExit("workflow close checklist should require fresh claim verification before PR-readiness claims")
claim_verify_items = [item for item in workflow_close["checklist"] if item.get("id") == "fresh-claim-verification"]
if claim_verify_items[0].get("command") != "./scripts/pm/claim-ready.sh --claim-type ready_for_pr --verify-command '<fresh verification command>'":
    raise SystemExit("workflow close checklist should point to claim-ready helper with an explicit verification placeholder")
prepare_items = [item for item in workflow_close["checklist"] if item.get("id") == "prepare-pr-review"]
if prepare_items[0].get("command") != "./scripts/prepare-task-pr.sh":
    raise SystemExit("workflow close PR review checklist should point to prepare-task-pr.sh")
prepare_summary = prepare_items[0].get("summary", "")
if "Pre-PR Local Role Review: passed" not in prepare_summary:
    raise SystemExit("workflow close PR review checklist should require local role review evidence before prepare-task-pr")
for marker in ("required checks", "mergeability", "PR comments", "unresolved review threads", "REVIEW_REQUIRED", "不是 block 项", "repo admin merge path", "manual packaging/release CI"):
    if marker not in prepare_summary:
        raise SystemExit(f"workflow close PR review checklist should mention post-PR watch/merge marker: {marker}")
if not any(item.get("id") == "bootstrap-working-memory" for item in workflow_close["checklist"]):
    raise SystemExit("workflow close checklist should suggest bootstrapping working_memory when the current task has no entries")
bootstrap_items = [item for item in workflow_close["checklist"] if item.get("id") == "bootstrap-working-memory"]
if bootstrap_items[0].get("command") != f"./scripts/pm/codex-working-memory.sh --task-uid {move_payload['task_uid']} --role qa_engineer --session-id <session_id>":
    raise SystemExit("workflow close bootstrap command should require an explicit session_id by default")
if any(item.get("id") == "review-working-memory" for item in workflow_close["checklist"]):
    raise SystemExit("workflow close checklist should not suggest reviewing working_memory when the current task has no entries")
if any(item.get("id") == "autoflow-working-memory" for item in workflow_close["checklist"]):
    raise SystemExit("workflow close checklist should not suggest autoflow before the current task has working_memory entries")
if workflow_close_with_wm["working_memory_summary"]["entry_count"] != 1:
    raise SystemExit("workflow close with seeded working_memory should report one task-scoped entry")
if any(item.get("id") == "bootstrap-working-memory" for item in workflow_close_with_wm["checklist"]):
    raise SystemExit("workflow close with seeded working_memory should not suggest bootstrap")
if not any(item.get("id") == "review-working-memory" for item in workflow_close_with_wm["checklist"]):
    raise SystemExit("workflow close with seeded working_memory should suggest reviewing task-scoped working_memory")
if not any(item.get("id") == "autoflow-working-memory" for item in workflow_close_with_wm["checklist"]):
    raise SystemExit("workflow close with seeded working_memory should suggest autoflow for task-scoped working_memory")
if regen_role_report["roles"]["qa_engineer"]["backlog_counts"]["blocked"] != 1:
    raise SystemExit("regenerated role report should keep qa blocked count")
if missing_verify_status == 0:
    raise SystemExit("task closeout helper should fail when done closeout omits --verify-command")
if missing_verify_state["status"] != "committed":
    raise SystemExit("task closeout helper should leave task status unchanged when verification is missing")
if missing_verify_state["last_closed_at"] not in {None, "null"}:
    raise SystemExit("task closeout helper should not write last_closed_at when verification is missing")
if missing_verify_state["last_verified_at"] not in {None, "null"}:
    raise SystemExit("task closeout helper should not write last_verified_at when verification is missing")
if bypass_status == 0:
    raise SystemExit("direct move-task done closeout should fail without persisted claim evidence")
if bypass_state["status"] != "committed":
    raise SystemExit("direct move-task done closeout should leave task status unchanged")
if not bypass_state["last_closed_at"]:
    raise SystemExit("workflow close evidence may exist before move-task, but move-task should still reject missing verification")
if bypass_state["last_verified_at"] not in {None, "null"}:
    raise SystemExit("direct move-task done closeout should not invent verification evidence")
if bypass_state["last_verification_status"] not in {None, "null"}:
    raise SystemExit("direct move-task done closeout should not invent verification status")
if task_closeout["task_uid"] != closeout_task_uid:
    raise SystemExit("task closeout helper should report the closed task uid")
if task_closeout["previous_status"] != "committed":
    raise SystemExit("task closeout helper should preserve the pre-close status in its summary")
if task_closeout["final_status"] != "done":
    raise SystemExit("task closeout helper should move the task to done by default")
if not task_closeout["last_closed_at"]:
    raise SystemExit("task closeout helper should record last_closed_at")
if task_closeout["claim_verification"]["status"] != "verified":
    raise SystemExit("task closeout helper should include verified claim evidence after fresh verification")
if task_closeout["claim_verification"]["claim_type"] != "task_complete":
    raise SystemExit("task closeout helper should default to task_complete claim verification")
if task_closeout["claim_verification"]["verify_command"] != "printf 'closeout verification ok\\n'":
    raise SystemExit("task closeout helper should report the exact fresh verification command")
if task_closeout["claim_verification"]["task_uid"] != closeout_task_uid:
    raise SystemExit("task closeout helper should bind claim verification evidence to the same task uid")
workflow_task_context = task_closeout["workflow_close"]["task_context"]
if workflow_task_context["last_claim_type"] != "task_complete":
    raise SystemExit("workflow close task context should expose persisted task_complete evidence")
if workflow_task_context["last_verification_status"] != "verified":
    raise SystemExit("workflow close task context should expose verified task evidence")
if workflow_task_context["last_verification_exit_code"] != 0:
    raise SystemExit("workflow close task context should expose zero-exit claim evidence")
if task_closeout["pm_lint"]["status"] != "ok":
    raise SystemExit("task closeout helper should run pm lint by default")
if task_closeout["recommended_next_command"] != "./scripts/prepare-task-pr.sh":
    raise SystemExit("task closeout helper should point to prepare-task-pr as the next step")
if task_closeout["workflow_close"]["task_context"]["task_uid"] != closeout_task_uid:
    raise SystemExit("task closeout helper should return workflow close evidence for the same task")
if task_closeout["move_task"]["to_status"] != "done":
    raise SystemExit("task closeout helper should report the final move-task status")
if closed_task_ready_claim_status == 0:
    raise SystemExit("closed done task should reject later non-completion claim evidence")
if "closed task claim evidence is immutable" not in closed_task_ready_claim_stderr:
    raise SystemExit("closed done task claim rejection should explain immutable claim evidence")
if Path(sys.argv[1], "closed-task-ready-side-effect").exists():
    raise SystemExit("closed done task should reject non-completion claims before running verification")
if closed_task_ready_claim_state["last_claim_type"] != "task_complete":
    raise SystemExit("closed done task should preserve task_complete claim evidence")
if "closed-task-ready-side-effect" in str(closed_task_ready_claim_state["last_verify_command"]):
    raise SystemExit("closed done task should not persist post-closeout readiness verification command")
if "closeout verification ok" not in str(closed_task_ready_claim_state["last_verify_command"]):
    raise SystemExit("closed done task should preserve closeout verification command")

print(
    json.dumps(
        {
            "temp_root": sys.argv[1],
            "signal": signal_payload,
            "move": move_payload,
            "qa_memory": qa_memory,
            "producer_memory": producer_memory,
            "shared_memory": shared_memory,
            "rejected_memory": rejected_memory,
            "liveops_signal": liveops_signal,
            "set_stage": set_stage,
            "memory_report": memory_report,
            "role_report": role_report,
            "regen_role_report": regen_role_report,
            "workflow_start": workflow_start,
            "workflow_close": workflow_close,
            "workflow_close_with_wm": workflow_close_with_wm,
            "workflow_review": workflow_review,
            "stage_report": stage_report,
            "task_closeout": task_closeout,
            "append_log": append_log,
            "append_cross_role": append_cross_role,
            "role_report_task": role_report_task,
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

python3 - <<'PY' "$RESULT_JSON" "$TMPDIR"
from __future__ import annotations

import json
import sys

payload = json.loads(sys.argv[1])
temp_root = sys.argv[2]
stage = payload["stage_report"]
signal_id = payload["signal"]["signal_id"]
task_uid = payload["move"]["task_uid"]

print("required-tier smoke: OK")
print(f"- temp_root: {temp_root}")
print(f"- signal_id: {signal_id}")
print(f"- task_uid: {task_uid}")
print(f"- current_stage: {stage['current_stage']}")
print(f"- gate_status: {stage['gate']['status']}")
print(f"- blocked_tasks: {len(stage['blocking_tasks'])}")
print(f"- producer_active_memory: {len(stage['memory_inputs']['producer_active'])}")
print(f"- shared_active_memory: {len(stage['memory_inputs']['shared_active'])}")
print(f"- needs_review_memory: {payload['memory_report']['counts']['needs_review']}")
print(f"- superseded_memory: {payload['memory_report']['counts']['superseded']}")
print(f"- qa_blocked_tasks: {payload['role_report']['roles']['qa_engineer']['backlog_counts']['blocked']}")
workflow_start_signal_summary = payload["workflow_start"].get("signal_summary") or {"pending_count": "github-backed"}
workflow_close_checklist = payload["workflow_close"].get("checklist") or []
workflow_review_signal_summary = payload["workflow_review"].get("signal_summary") or {"pending_count": "github-backed"}
workflow_review_checklist = payload["workflow_review"].get("checklist") or []
print(f"- qa_pending_signals: {workflow_start_signal_summary['pending_count']}")
print(f"- qa_close_actions: {len(workflow_close_checklist)}")
print(f"- closeout_helper_final_status: {payload['task_closeout']['final_status']}")
print(f"- producer_pending_signals: {workflow_review_signal_summary['pending_count']}")
print(f"- producer_review_actions: {len(workflow_review_checklist)}")
print(f"- rejected_memory_signal: {payload['rejected_memory']['signal_id']}")
PY
