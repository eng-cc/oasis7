#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${PM_ROOT_DIR:-$(cd "$SCRIPT_DIR/../.." && pwd)}"
SCHEMA_PATH="$SCRIPT_DIR/schemas/codex-working-memory.schema.json"

TASK_ID=""
ROLE=""
SESSION_ID=""
WORKTREE_HINT=""
THREAD_NAME_PATTERN=""
CODEX_DIR="${CODEX_DIR:-$HOME/.codex}"
CODEX_BIN="${CODEX_BIN:-codex}"
MODEL=""
EXPIRES_DAYS=2
JSON_OUTPUT=0
PREPARE_ONLY=0
FULL_SCAN=0

usage() {
  cat <<'USAGE'
Usage: ./scripts/pm/codex-working-memory.sh --task-id TASK-PM-0001 --role producer_system_designer [--session-id <session_id> | --thread-name-pattern <pattern> | --worktree-hint <hint>] [options]

Deterministically preprocess one Codex session transcript (sort + redact), then call `codex exec`
to extract task-scoped working_memory entries, and finally write them to `.pm/working_memory/<task_id>.yaml`.

Options:
  --task-id <id>          Required task id
  --role <role>           Required role
  --session-id <id>       Optional Codex session id; if omitted, try registry or pattern resolution
  --worktree-hint <hint>  Optional worktree hint written into working_memory header
  --thread-name-pattern <pattern>
                          Optional fallback pattern for session resolution
  --codex-dir <path>      Codex home directory (default: ~/.codex)
  --codex-bin <path>      Codex CLI binary (default: codex)
  --model <name>          Optional model override passed to `codex exec`
  --expires-days <days>   Working memory TTL in days (default: 2)
  --full-scan             Ignore task watermark and rescan the whole session
  --prepare-only          Only emit the preprocessed transcript JSON and stop
  --json                  Print machine-readable result
  -h, --help              Show help
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --task-id)
      TASK_ID="$2"
      shift 2
      ;;
    --role)
      ROLE="$2"
      shift 2
      ;;
    --session-id)
      SESSION_ID="$2"
      shift 2
      ;;
    --worktree-hint)
      WORKTREE_HINT="$2"
      shift 2
      ;;
    --thread-name-pattern)
      THREAD_NAME_PATTERN="$2"
      shift 2
      ;;
    --codex-dir)
      CODEX_DIR="$2"
      shift 2
      ;;
    --codex-bin)
      CODEX_BIN="$2"
      shift 2
      ;;
    --model)
      MODEL="$2"
      shift 2
      ;;
    --expires-days)
      EXPIRES_DAYS="$2"
      shift 2
      ;;
    --full-scan)
      FULL_SCAN=1
      shift
      ;;
    --prepare-only)
      PREPARE_ONLY=1
      shift
      ;;
    --json)
      JSON_OUTPUT=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "codex-working-memory: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -z "$TASK_ID" || -z "$ROLE" ]]; then
  echo "codex-working-memory: --task-id and --role are required" >&2
  usage >&2
  exit 2
fi

TMPDIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

PREPARED_JSON="$TMPDIR/prepared.json"
LLM_OUTPUT_JSON="$TMPDIR/llm-output.json"
IMPORT_RESULT_JSON="$TMPDIR/import-result.json"
PROMPT_FILE="$TMPDIR/prompt.txt"
EXISTING_REPORT_JSON="$TMPDIR/existing-working-memory-report.json"

python3 "$SCRIPT_DIR/pm_store.py" working-memory-report "$ROOT_DIR" --task-id "$TASK_ID" --json > "$EXISTING_REPORT_JSON"

PREVIOUS_SESSION_ID="$(python3 -c 'import json,sys; data=json.load(open(sys.argv[1], encoding="utf-8")); task=data.get("tasks", {}).get(sys.argv[2], {}); print(task.get("source_session_id") or "")' "$EXISTING_REPORT_JSON" "$TASK_ID")"
LAST_EXTRACTED_TS="$(python3 -c 'import json,sys; data=json.load(open(sys.argv[1], encoding="utf-8")); task=data.get("tasks", {}).get(sys.argv[2], {}); print(task.get("last_extracted_ts") or "")' "$EXISTING_REPORT_JSON" "$TASK_ID")"

CURRENT_WATERMARK_SESSION_ID="$SESSION_ID"
if [[ -z "$CURRENT_WATERMARK_SESSION_ID" && -n "$PREVIOUS_SESSION_ID" ]]; then
  CURRENT_WATERMARK_SESSION_ID="$PREVIOUS_SESSION_ID"
fi

TRANSCRIPT_ARGS=(
  codex-transcript-report
  "$ROOT_DIR"
  --task-id "$TASK_ID"
  --codex-dir "$CODEX_DIR"
  --json
)
if [[ -n "$SESSION_ID" ]]; then
  TRANSCRIPT_ARGS+=(--session-id "$SESSION_ID")
fi
if [[ -n "$WORKTREE_HINT" ]]; then
  TRANSCRIPT_ARGS+=(--worktree-hint "$WORKTREE_HINT")
fi
if [[ -n "$THREAD_NAME_PATTERN" ]]; then
  TRANSCRIPT_ARGS+=(--thread-name-pattern "$THREAD_NAME_PATTERN")
fi
if [[ "$FULL_SCAN" != "1" && -n "$LAST_EXTRACTED_TS" && -n "$PREVIOUS_SESSION_ID" && "$CURRENT_WATERMARK_SESSION_ID" == "$PREVIOUS_SESSION_ID" ]]; then
  TRANSCRIPT_ARGS+=(--after-ts "$LAST_EXTRACTED_TS")
fi

python3 "$SCRIPT_DIR/pm_store.py" "${TRANSCRIPT_ARGS[@]}" > "$PREPARED_JSON"

if [[ "$PREPARE_ONLY" == "1" ]]; then
  cat "$PREPARED_JSON"
  exit 0
fi

cat > "$PROMPT_FILE" <<EOF
你会收到一个经过本地脚本排序和脱敏后的 Codex 会话 transcript JSON。
任务：仅提炼 task-scoped working_memory entries。

硬约束：
1. 只能输出 JSON，且必须满足给定 schema。
2. 只能输出 entry_kind in: attempt, hypothesis, decision, open_question, next_step
3. summary 必须简短、具体、可复用，不要复制大段原文。
4. source_refs 必须直接复用输入 messages 里的 source_ref。
5. 不要输出长期 memory、signal、task、建议动作或解释文字。
6. 如果没有足够信息，输出 {"entries": []}

Transcript JSON:
EOF
cat "$PREPARED_JSON" >> "$PROMPT_FILE"

MESSAGE_COUNT="$(python3 -c 'import json,sys; data=json.load(open(sys.argv[1], encoding="utf-8")); print(data.get("message_count", 0))' "$PREPARED_JSON")"
if [[ "$MESSAGE_COUNT" == "0" ]]; then
  printf '{"entries":[]}\n' > "$LLM_OUTPUT_JSON"
else
  CODEX_ARGS=(
    exec
    --ephemeral
    --skip-git-repo-check
    -C "$ROOT_DIR"
    --output-schema "$SCHEMA_PATH"
    -o "$LLM_OUTPUT_JSON"
  )

  if [[ -n "$MODEL" ]]; then
    CODEX_ARGS+=(--model "$MODEL")
  fi

  "$CODEX_BIN" "${CODEX_ARGS[@]}" - < "$PROMPT_FILE" >/dev/null
fi

IMPORT_ARGS=(
  import-working-memory
  "$ROOT_DIR"
  --task-id "$TASK_ID"
  --role "$ROLE"
  --input-json "$LLM_OUTPUT_JSON"
  --expires-days "$EXPIRES_DAYS"
  --session-id "$(python3 -c 'import json,sys; data=json.load(open(sys.argv[1], encoding="utf-8")); print(data["session"]["id"])' "$PREPARED_JSON")"
  --thread-name "$(python3 -c 'import json,sys; data=json.load(open(sys.argv[1], encoding="utf-8")); print(data["session"].get("thread_name") or "")' "$PREPARED_JSON")"
  --codex-dir "$CODEX_DIR"
  --mapping-updated-at "$(python3 -c 'import json,sys; data=json.load(open(sys.argv[1], encoding="utf-8")); print(data["session"].get("updated_at") or "")' "$PREPARED_JSON")"
  --transcript-source "$(python3 -c 'import json,sys; data=json.load(open(sys.argv[1], encoding="utf-8")); print(data.get("transcript_source") or "")' "$PREPARED_JSON")"
  --captured-until-ts "$(python3 -c 'import json,sys; data=json.load(open(sys.argv[1], encoding="utf-8")); messages=data.get("messages") or []; print(messages[-1]["ts"] if messages else "")' "$PREPARED_JSON")"
  --json
)
if [[ -n "$WORKTREE_HINT" ]]; then
  IMPORT_ARGS+=(--worktree-hint "$WORKTREE_HINT")
fi

python3 "$SCRIPT_DIR/pm_store.py" "${IMPORT_ARGS[@]}" > "$IMPORT_RESULT_JSON"

if [[ "$JSON_OUTPUT" == "1" ]]; then
  python3 - "$PREPARED_JSON" "$LLM_OUTPUT_JSON" "$IMPORT_RESULT_JSON" <<'PY'
import json
import sys
prepared = json.load(open(sys.argv[1], encoding="utf-8"))
llm_output = json.load(open(sys.argv[2], encoding="utf-8"))
import_result = json.load(open(sys.argv[3], encoding="utf-8"))
print(json.dumps({
    "prepared": prepared,
    "llm_output": llm_output,
    "import_result": import_result,
}, ensure_ascii=False, indent=2))
PY
else
  python3 - "$IMPORT_RESULT_JSON" <<'PY'
import json
import sys
payload = json.load(open(sys.argv[1], encoding="utf-8"))
print(
    "codex-working-memory: "
    f"added={payload['added']} skipped={payload['skipped']} "
    f"path={payload['path']}"
)
PY
fi
