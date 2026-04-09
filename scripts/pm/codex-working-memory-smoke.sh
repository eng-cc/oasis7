#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

OUTPUT_JSON=0
KEEP_TEMP=0
SMOKE_TASK_UID="task_a878d035986f54a79dc65a383a87de1c"

usage() {
  cat <<'USAGE'
Usage: ./scripts/pm/codex-working-memory-smoke.sh [--json] [--keep-temp]

Run an isolated smoke for:
  ~/.codex JSONL -> deterministic preprocessing -> codex exec wrapper -> .pm/working_memory

Options:
  --json       Print machine-readable JSON summary
  --keep-temp  Keep the temporary root for inspection
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
      echo "codex-working-memory-smoke: unknown argument: $1" >&2
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

mkdir -p "$TMPDIR/scripts" "$TMPDIR/.codex"
cp -R "$ROOT_DIR/.pm" "$TMPDIR/.pm"
cp -R "$ROOT_DIR/.agents" "$TMPDIR/.agents"
cp -R "$ROOT_DIR/scripts/pm" "$TMPDIR/scripts/pm"

python3 - "$TMPDIR" <<'PY'
from __future__ import annotations
import json
from pathlib import Path
import sys

root = Path(sys.argv[1])

for path in (root / ".pm/tasks").glob("*.yaml"):
    path.unlink()
for path in (root / ".pm/tasks").glob("*.execution.md"):
    path.unlink()
for path in (root / ".pm/working_memory").glob("*.yaml"):
    path.unlink()

(root / ".pm/inbox/signals.jsonl").write_text("", encoding="utf-8")
(root / ".pm/registry/tasks.yaml").write_text(
    'version: 2\nidentity_key: task_uid\ngenerated_from: ".pm/tasks/*.yaml"\ntasks: []\n',
    encoding="utf-8",
)
(root / ".pm/registry/codex-sessions.yaml").write_text(
    "version: 1\nsessions: []\n",
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

for backlog_path in (root / ".pm/roles").glob("*/backlog/*.yaml"):
    role = backlog_path.parts[-3]
    status = backlog_path.stem
    backlog_path.write_text(
        f"version: 1\nrole: {role}\nstatus: {status}\ntasks: []\n",
        encoding="utf-8",
    )

for path in root.glob(".pm/**/*.yaml"):
    for line in path.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if not stripped.startswith("- "):
            continue
        value = stripped[2:]
        source_path = value.split("#", 1)[0]
        candidate = Path(source_path).expanduser()
        if candidate.is_absolute() or source_path.startswith("__"):
            continue
        target = root / source_path
        target.parent.mkdir(parents=True, exist_ok=True)
        if not target.exists():
            target.write_text(f"placeholder for {source_path}\n", encoding="utf-8")

testing_manual = root / "testing-manual.md"
if not testing_manual.exists():
    testing_manual.write_text("placeholder testing manual\n", encoding="utf-8")

for path in root.glob(".pm/tasks/*.execution.md"):
    lines = path.read_text(encoding="utf-8").splitlines()
    filtered = [
        line for line in lines
        if line not in {
            "<!-- Append entries using:",
            "## YYYY-MM-DD HH:MM:SS CST / role_name",
            "- 完成内容: ...",
            "- 遗留事项: ...",
            "-->",
        }
    ]
    text = "\n".join(filtered).rstrip() + "\n"
    if "\n## " in text or text.startswith("## "):
        path.write_text(text, encoding="utf-8")
        continue
    with path.open("w", encoding="utf-8") as handle:
        handle.write(text.rstrip() + "\n")
        handle.write("\n## 2026-03-31 18:00:00 CST / producer_system_designer\n")
        handle.write("- 完成内容: smoke fixture seeded a minimal execution-log entry for lint compatibility.\n")
        handle.write("- 遗留事项: none.\n")
PY

cat > "$TMPDIR/.codex/session_index.jsonl" <<'EOF'
{"id":"session-test-001","thread_name":"engineering memory extraction","updated_at":"2026-03-31T17:40:00Z"}
{"id":"session-test-002","thread_name":"engineering memory extraction fallback","updated_at":"2026-03-31T17:45:00Z"}
EOF

cat > "$TMPDIR/.codex/history.jsonl" <<'EOF'
{"session_id":"session-test-001","ts":200,"text":"决定 phase 1 先直接读 ~/.codex/history.jsonl，再做 working_memory 提炼。"}
{"session_id":"session-test-001","ts":100,"text":"先看一下 sk-test-secret-token-1234567890 是否会被脱敏。"}
{"session_id":"session-test-001","ts":300,"text":"下一步补一个 smoke，验证 codex exec wrapper 能写入 working_memory。"}
EOF

mkdir -p "$TMPDIR/.codex/sessions/2026/03/31"
cat > "$TMPDIR/.codex/sessions/2026/03/31/rollout-2026-03-31T17-45-00-session-test-002.jsonl" <<'EOF'
{"timestamp":"2026-03-31T09:45:00Z","type":"session_meta","payload":{"id":"session-test-002","timestamp":"2026-03-31T09:45:00Z","cwd":"/tmp/example","source":"vscode","title":"engineering memory extraction fallback"}}
{"timestamp":"2026-03-31T09:45:01Z","type":"event_msg","payload":{"type":"user_message","message":"还是先直接从.codex里读吧"}}
{"timestamp":"2026-03-31T09:45:02Z","type":"event_msg","payload":{"type":"agent_message","message":"先用本地脚本做确定性预处理，这里只做排序、脱敏。"}}
{"timestamp":"2026-03-31T09:45:03Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"继续做完，补一个 sessions fallback。"}]}}
EOF

cat > "$TMPDIR/fake-codex" <<EOF
#!/usr/bin/env bash
set -euo pipefail
OUT=""
while [[ \$# -gt 0 ]]; do
  case "\$1" in
    -o|--output-last-message)
      OUT="\$2"
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done
if [[ -z "\$OUT" ]]; then
  echo "fake-codex: missing output file" >&2
  exit 2
fi
cat >/dev/null
cat > "\$OUT" <<'JSON'
{
  "entries": [
    {
      "entry_kind": "decision",
      "summary": "phase 1 transcript source was fixed to direct ~/.codex reads",
      "source_refs": [
        "$TMPDIR/.codex/history.jsonl#session_id=session-test-001&ts=200"
      ]
    },
    {
      "entry_kind": "next_step",
      "summary": "add a smoke to validate the codex working_memory wrapper",
      "source_refs": [
        "$TMPDIR/.codex/history.jsonl#session_id=session-test-001&ts=300"
      ]
    }
  ]
}
JSON
EOF
chmod +x "$TMPDIR/fake-codex"

PREPARED_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/codex-transcript-report.sh" \
  --session-id session-test-001 \
  --codex-dir "$TMPDIR/.codex" \
  --json)"

PREPARED_JSON_FALLBACK="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/codex-transcript-report.sh" \
  --session-id session-test-002 \
  --codex-dir "$TMPDIR/.codex" \
  --json)"

RESULT_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/codex-working-memory.sh" \
  --task-uid "$SMOKE_TASK_UID" \
  --role producer_system_designer \
  --session-id session-test-001 \
  --worktree-hint codex-working-memory-smoke \
  --codex-dir "$TMPDIR/.codex" \
  --codex-bin "$TMPDIR/fake-codex" \
  --json)"

set +e
AUTO_SESSION_DISABLED_OUTPUT="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/codex-working-memory.sh" \
  --task-uid "$SMOKE_TASK_UID" \
  --role producer_system_designer \
  --worktree-hint codex-working-memory-smoke \
  --codex-dir "$TMPDIR/.codex" \
  --codex-bin "$TMPDIR/fake-codex" \
  --json 2>&1)"
AUTO_SESSION_DISABLED_STATUS=$?
set -e

RESULT_JSON_REGISTRY="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/codex-working-memory.sh" \
  --task-uid "$SMOKE_TASK_UID" \
  --role producer_system_designer \
  --allow-auto-session \
  --worktree-hint codex-working-memory-smoke \
  --codex-dir "$TMPDIR/.codex" \
  --codex-bin "$TMPDIR/fake-codex" \
  --json)"

DRY_RUN_SIGNAL_SHA_BEFORE="$(sha256sum "$TMPDIR/.pm/inbox/signals.jsonl" | awk '{print $1}')"
DRY_RUN_WM_SHA_BEFORE="$(sha256sum "$TMPDIR/.pm/working_memory/$SMOKE_TASK_UID.yaml" | awk '{print $1}')"
DRY_RUN_TASK_REGISTRY_SHA_BEFORE="$(sha256sum "$TMPDIR/.pm/registry/tasks.yaml" | awk '{print $1}')"
DRY_RUN_BACKLOG_SHA_BEFORE="$(sha256sum "$TMPDIR/.pm/roles/producer_system_designer/backlog/candidate.yaml" | awk '{print $1}')"
DRY_RUN_TASK_LIST_BEFORE="$(find "$TMPDIR/.pm/tasks" -maxdepth 1 -type f -printf '%f\n' | sort)"
DRY_RUN_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/working-memory-autoflow.sh" \
  --task-uid "$SMOKE_TASK_UID" \
  --entry-id WM-0002 \
  --severity medium \
  --priority P2 \
  --dry-run \
  --json)"
DRY_RUN_SIGNAL_SHA_AFTER="$(sha256sum "$TMPDIR/.pm/inbox/signals.jsonl" | awk '{print $1}')"
DRY_RUN_WM_SHA_AFTER="$(sha256sum "$TMPDIR/.pm/working_memory/$SMOKE_TASK_UID.yaml" | awk '{print $1}')"
DRY_RUN_TASK_REGISTRY_SHA_AFTER="$(sha256sum "$TMPDIR/.pm/registry/tasks.yaml" | awk '{print $1}')"
DRY_RUN_BACKLOG_SHA_AFTER="$(sha256sum "$TMPDIR/.pm/roles/producer_system_designer/backlog/candidate.yaml" | awk '{print $1}')"
DRY_RUN_TASK_LIST_AFTER="$(find "$TMPDIR/.pm/tasks" -maxdepth 1 -type f -printf '%f\n' | sort)"

SIGNAL_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/working-memory-to-signal.sh" \
  --task-uid "$SMOKE_TASK_UID" \
  --entry-id WM-0001 \
  --severity medium \
  --json)"

AUTOFLOW_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/working-memory-autoflow.sh" \
  --task-uid "$SMOKE_TASK_UID" \
  --entry-id WM-0002 \
  --severity medium \
  --priority P2 \
  --json)"

python3 - "$TMPDIR" "$AUTOFLOW_JSON" <<'PY'
from __future__ import annotations
import json
from pathlib import Path
import sys

root = Path(sys.argv[1])
payload = json.loads(sys.argv[2])
for item in payload.get("task_actions", []):
    if item.get("decision") != "created":
        continue
    task = item.get("task") or {}
    task_uid = task.get("task_uid")
    if not task_uid:
        continue
    path = root / f".pm/tasks/{task_uid}.execution.md"
    lines = path.read_text(encoding="utf-8").splitlines()
    filtered = [
        line for line in lines
        if line not in {
            "<!-- Append entries using:",
            "## YYYY-MM-DD HH:MM:SS CST / role_name",
            "- 完成内容: ...",
            "- 遗留事项: ...",
            "-->",
        }
    ]
    with path.open("w", encoding="utf-8") as handle:
        handle.write("\n".join(filtered).rstrip() + "\n")
        handle.write("\n## 2026-03-31 18:10:00 CST / producer_system_designer\n")
        handle.write("- 完成内容: smoke fixture backfilled the created candidate task execution log so pm-lint can validate the autoflow output.\n")
        handle.write("- 遗留事项: none.\n")
PY

PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/working-memory-lint.sh" >/dev/null
PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/lint.sh" >/dev/null
REPORT_JSON="$(PM_ROOT_DIR="$TMPDIR" "$ROOT_DIR/scripts/pm/working-memory-report.sh" --task-uid "$SMOKE_TASK_UID" --json)"

python3 - "$TMPDIR" "$SMOKE_TASK_UID" "$PREPARED_JSON" "$PREPARED_JSON_FALLBACK" "$RESULT_JSON" "$AUTO_SESSION_DISABLED_OUTPUT" "$AUTO_SESSION_DISABLED_STATUS" "$RESULT_JSON_REGISTRY" "$DRY_RUN_JSON" "$DRY_RUN_SIGNAL_SHA_BEFORE" "$DRY_RUN_SIGNAL_SHA_AFTER" "$DRY_RUN_WM_SHA_BEFORE" "$DRY_RUN_WM_SHA_AFTER" "$DRY_RUN_TASK_REGISTRY_SHA_BEFORE" "$DRY_RUN_TASK_REGISTRY_SHA_AFTER" "$DRY_RUN_BACKLOG_SHA_BEFORE" "$DRY_RUN_BACKLOG_SHA_AFTER" "$DRY_RUN_TASK_LIST_BEFORE" "$DRY_RUN_TASK_LIST_AFTER" "$SIGNAL_JSON" "$AUTOFLOW_JSON" "$REPORT_JSON" "$OUTPUT_JSON" <<'PY'
from __future__ import annotations
import json
import sys
from pathlib import Path

tmpdir = Path(sys.argv[1])
smoke_task_uid = sys.argv[2]
prepared = json.loads(sys.argv[3])
prepared_fallback = json.loads(sys.argv[4])
result = json.loads(sys.argv[5])
auto_session_disabled_output = sys.argv[6]
auto_session_disabled_status = int(sys.argv[7])
result_registry = json.loads(sys.argv[8])
dry_run = json.loads(sys.argv[9])
dry_run_signal_sha_before = sys.argv[10]
dry_run_signal_sha_after = sys.argv[11]
dry_run_wm_sha_before = sys.argv[12]
dry_run_wm_sha_after = sys.argv[13]
dry_run_task_registry_sha_before = sys.argv[14]
dry_run_task_registry_sha_after = sys.argv[15]
dry_run_backlog_sha_before = sys.argv[16]
dry_run_backlog_sha_after = sys.argv[17]
dry_run_task_list_before = sys.argv[18]
dry_run_task_list_after = sys.argv[19]
signal_json = json.loads(sys.argv[20])
autoflow_json = json.loads(sys.argv[21])
report = json.loads(sys.argv[22])
output_json = sys.argv[23] == "1"

assert prepared["messages"][0]["ts"] == 100
assert prepared["messages"][1]["ts"] == 200
assert prepared["messages"][0]["text"].count("[REDACTED_TOKEN]") == 1
assert prepared_fallback["transcript_source"] == "sessions_rollout"
assert prepared_fallback["message_count"] == 3
assert prepared_fallback["messages"][0]["text"] == "还是先直接从.codex里读吧"
assert prepared_fallback["messages"][2]["text"] == "继续做完，补一个 sessions fallback。"
assert result["import_result"]["added"] == 2
assert result["import_result"]["codex_session_mapping"]["session_id"] == "session-test-001"
assert auto_session_disabled_status != 0
assert "explicit --session-id is required by default" in auto_session_disabled_output
assert result_registry["prepared"]["message_count"] == 0
assert str(result_registry["prepared"]["after_ts"]) == "300"
assert result_registry["import_result"]["added"] == 0
assert result_registry["import_result"]["skipped"] == 0
assert result_registry["prepared"]["resolution_source"] == "registry"
assert dry_run["dry_run"] is True
assert dry_run["signal_result"]["applied"] is False
assert len(dry_run["signal_result"]["created"]) == 1
assert "signal_id" not in dry_run["signal_result"]["created"][0]
assert len(dry_run["task_actions"]) == 1
assert dry_run["task_actions"][0]["decision"] == "would_create"
assert dry_run_signal_sha_before == dry_run_signal_sha_after
assert dry_run_wm_sha_before == dry_run_wm_sha_after
assert dry_run_task_registry_sha_before == dry_run_task_registry_sha_after
assert dry_run_backlog_sha_before == dry_run_backlog_sha_after
assert dry_run_task_list_before == dry_run_task_list_after
assert len(signal_json["created"]) == 1
assert signal_json["created"][0]["signal_id"].startswith("SIG-PM-")
assert len(autoflow_json["signal_result"]["created"]) == 1
assert autoflow_json["signal_result"]["applied"] is True
assert len(autoflow_json["task_actions"]) == 1
assert autoflow_json["task_actions"][0]["decision"] == "created"
assert report["entry_count"] == 2
assert report["tasks"][smoke_task_uid]["source_session_id"] == "session-test-001"
assert str(report["tasks"][smoke_task_uid]["last_extracted_ts"]) == "300"
assert str(report["tasks"][smoke_task_uid]["captured_until_ts"]) == "300"
assert report["tasks"][smoke_task_uid]["entries"][0]["promoted_to"]
assert len(report["tasks"][smoke_task_uid]["entries"][1]["promoted_to"]) == 2

payload = {
    "tmpdir": str(tmpdir),
    "smoke_task_uid": smoke_task_uid,
    "prepared": prepared,
    "prepared_fallback": prepared_fallback,
    "result": result,
    "result_registry": result_registry,
    "dry_run": dry_run,
    "signal": signal_json,
    "autoflow": autoflow_json,
    "report": report,
}

if output_json:
    print(json.dumps(payload, ensure_ascii=False, indent=2))
else:
    print(
        "codex-working-memory-smoke: OK "
        f"(messages={prepared['message_count']} added={result['import_result']['added']} entries={report['entry_count']})"
    )
PY
