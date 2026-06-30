#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${PM_ROOT_DIR:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

usage() {
  cat <<'USAGE'
Usage: ./scripts/pm/slice-ledger.sh --task-uid <uid> [options]

Append or print a lightweight JSONL resume map for role slices. The ledger is
ignored scratch evidence; GitHub task issue evidence comments remain canonical.

Options:
  --task-uid <uid>          Task UID
  --role <role>             Slice role for an appended entry
  --status <status>         Slice status for an appended entry
  --base <ref>              Optional reviewed base ref
  --head <ref>              Optional reviewed head ref
  --artifact <path>         Optional linked artifact path; may repeat
  --verdict <text>          Optional dual/verdict summary
  --residual-risk <text>    Optional residual risk summary
  --next-action <text>      Optional next action
  --print                   Print the ledger path and exit
  -h, --help                Show help
USAGE
}

die() {
  echo "slice-ledger: $*" >&2
  exit 1
}

TASK_UID=""
ROLE=""
STATUS=""
BASE_REF=""
HEAD_REF=""
VERDICT=""
RESIDUAL_RISK=""
NEXT_ACTION=""
PRINT_ONLY=0
ARTIFACTS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --task-uid)
      TASK_UID="${2:-}"
      shift 2
      ;;
    --role)
      ROLE="${2:-}"
      shift 2
      ;;
    --status)
      STATUS="${2:-}"
      shift 2
      ;;
    --base)
      BASE_REF="${2:-}"
      shift 2
      ;;
    --head)
      HEAD_REF="${2:-}"
      shift 2
      ;;
    --artifact)
      ARTIFACTS+=("${2:-}")
      shift 2
      ;;
    --verdict)
      VERDICT="${2:-}"
      shift 2
      ;;
    --residual-risk)
      RESIDUAL_RISK="${2:-}"
      shift 2
      ;;
    --next-action)
      NEXT_ACTION="${2:-}"
      shift 2
      ;;
    --print)
      PRINT_ONLY=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "unknown argument: $1"
      ;;
  esac
done

[[ -n "$TASK_UID" ]] || die "--task-uid is required"
[[ "$TASK_UID" =~ ^task_[0-9a-f]{32}$ ]] || die "invalid --task-uid: $TASK_UID"

LEDGER_DIR="$ROOT_DIR/.pm/scratch/$TASK_UID"
LEDGER_PATH="$LEDGER_DIR/slice-ledger.jsonl"
mkdir -p "$LEDGER_DIR"
printf '*\n' > "$ROOT_DIR/.pm/scratch/.gitignore"

if [[ "$PRINT_ONLY" == "1" ]]; then
  printf '%s\n' "$LEDGER_PATH"
  exit 0
fi

[[ -n "$ROLE" ]] || die "--role is required unless --print is used"
[[ -n "$STATUS" ]] || die "--status is required unless --print is used"

python3 - "$LEDGER_PATH" "$TASK_UID" "$ROLE" "$STATUS" "$BASE_REF" "$HEAD_REF" "$VERDICT" "$RESIDUAL_RISK" "$NEXT_ACTION" "${ARTIFACTS[@]}" <<'PY'
from __future__ import annotations

from datetime import datetime, timezone
import json
from pathlib import Path
import sys

ledger_path = Path(sys.argv[1])
entry = {
    "recorded_at": datetime.now(timezone.utc).isoformat(),
    "task_uid": sys.argv[2],
    "role": sys.argv[3],
    "status": sys.argv[4],
    "base": sys.argv[5],
    "head": sys.argv[6],
    "verdict": sys.argv[7],
    "residual_risk": sys.argv[8],
    "next_action": sys.argv[9],
    "artifacts": sys.argv[10:],
}
ledger_path.parent.mkdir(parents=True, exist_ok=True)
with ledger_path.open("a", encoding="utf-8") as handle:
    handle.write(json.dumps(entry, ensure_ascii=False, sort_keys=True) + "\n")
PY

printf 'Slice Ledger: %s\n' "$LEDGER_PATH"
