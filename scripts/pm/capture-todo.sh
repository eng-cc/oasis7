#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

usage() {
  cat <<'USAGE'
Usage: ./scripts/pm/capture-todo.sh --source-ref <path> (--summary <text> | --text <text>) [options]

Capture a lightweight pre-task TODO as a GitHub-backed reflection intake issue.
By default this only
creates a GitHub-backed reflection intake issue and does not create a candidate task unless --create-task is selected.

Required:
  --source-ref <path>         Primary source reference for the discovery

Required, choose one:
  --summary <text>            TODO text / discovery summary
  --text <text>               Alias for --summary

Optional:
  --role-hint <role>          Canonical role owner hint; default: tpm
  --severity <level>          low | medium | high | critical; default: low
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
  - This is an intake wrapper around ./scripts/pm/promote-signal.sh.
  - Pre-task TODOs are recorded with --source-type reflection.
  - Use --create-task only when the TODO is ready to become a candidate task.
USAGE
}

SOURCE_REF=""
SUMMARY=""
ROLE_HINT="tpm"
SEVERITY="low"
PROMOTE_ARGS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --source-ref)
      SOURCE_REF="${2:-}"
      shift 2
      ;;
    --summary|--text)
      SUMMARY="${2:-}"
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
    --signal-id|--title|--owner-role|--priority|--doc-ref|--related-prd|--acceptance|--handoff-to|--worktree-hint)
      PROMOTE_ARGS+=("$1" "${2:-}")
      shift 2
      ;;
    --create-task|--json)
      PROMOTE_ARGS+=("$1")
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "capture-todo: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

[[ -n "$SOURCE_REF" ]] || { echo "capture-todo: --source-ref is required" >&2; exit 2; }
[[ -n "$SUMMARY" ]] || { echo "capture-todo: --summary or --text is required" >&2; exit 2; }

COMMAND=(
  "$SCRIPT_DIR/promote-signal.sh"
  --source-type reflection
  --source-ref "$SOURCE_REF"
  --role-hint "$ROLE_HINT"
  --severity "$SEVERITY"
  --summary "$SUMMARY"
)
if [[ "${#PROMOTE_ARGS[@]}" -gt 0 ]]; then
  COMMAND+=("${PROMOTE_ARGS[@]}")
fi

exec "${COMMAND[@]}"
