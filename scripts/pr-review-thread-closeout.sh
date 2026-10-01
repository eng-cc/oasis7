#!/usr/bin/env bash
# Cross-platform maintenance: keep temporary paths readable by Git Bash and native Windows Python.
set -euo pipefail

case "$(uname -s)" in
  MSYS*|MINGW*|CYGWIN*)
    if [[ -z "${TMPDIR:-}" || "$TMPDIR" == "/tmp" || "$TMPDIR" == "/tmp/" ]]; then
      TMPDIR="$(cygpath -m "${TEMP:-${TMP:-/tmp}}")"
    elif [[ "$TMPDIR" == /* ]]; then
      TMPDIR="$(cygpath -m "$TMPDIR")"
    fi
    export TMPDIR
    ;;
esac

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
source "$ROOT_DIR/scripts/worktree-harness-lib.sh"

usage() {
  cat <<'USAGE'
Usage: ./scripts/pr-review-thread-closeout.sh [pr-number|pr-url|branch] [options]

Inspect GitHub PR review threads for the current PR (or one explicit PR), and
optionally resolve selected threads as part of the same-PR comment closeout
loop. Omitting the selector resolves the current branch in the origin repository.

Options:
  --unresolved-only          Only report unresolved threads
  --resolve-thread <id>      Resolve one explicit review thread id (repeatable)
  --resolve-all-unresolved   Resolve every currently unresolved review thread
  --summary                  Omit review-comment bodies from the API response
  --json                     Print machine-readable JSON summary only
  -h, --help                 Show help

Examples:
  ./scripts/pr-review-thread-closeout.sh
  ./scripts/pr-review-thread-closeout.sh 145 --json
  ./scripts/pr-review-thread-closeout.sh https://github.com/eng-cc/oasis7/pull/145 --summary
  ./scripts/pr-review-thread-closeout.sh --unresolved-only
  ./scripts/pr-review-thread-closeout.sh --resolve-thread PRRT_kwDOGA
  ./scripts/pr-review-thread-closeout.sh --resolve-all-unresolved --json
USAGE
}

die() {
  echo "pr-review-thread-closeout: $*" >&2
  exit 1
}

OUTPUT_JSON=0
UNRESOLVED_ONLY=0
RESOLVE_ALL=0
SUMMARY=0
RESOLVE_THREAD_IDS=()
POSITIONAL=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --unresolved-only)
      UNRESOLVED_ONLY=1
      shift
      ;;
    --resolve-thread)
      [[ $# -ge 2 ]] || die "--resolve-thread requires a thread id"
      [[ -n "${2:-}" && "${2:0:1}" != "-" ]] || die "--resolve-thread requires a thread id"
      RESOLVE_THREAD_IDS+=("$2")
      shift 2
      ;;
    --resolve-all-unresolved)
      RESOLVE_ALL=1
      shift
      ;;
    --summary)
      SUMMARY=1
      shift
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
      POSITIONAL+=("$1")
      shift
      ;;
  esac
done

if [[ "${#POSITIONAL[@]}" -gt 1 ]]; then
  die "expected at most one PR number, URL, or branch selector"
fi
if [[ "$RESOLVE_ALL" == "1" && "${#RESOLVE_THREAD_IDS[@]}" -gt 0 ]]; then
  die "--resolve-all-unresolved cannot be combined with --resolve-thread"
fi
if [[ "${#POSITIONAL[@]}" -eq 0 ]]; then
  wh_require_git_worktree
fi

TMP_DIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT
REPORT_FILE="$TMP_DIR/report.json"

COMMAND=(python3 "$ROOT_DIR/scripts/pm/github_pr_snapshot.py" closeout)
if [[ "${#POSITIONAL[@]}" -eq 1 ]]; then
  COMMAND+=("${POSITIONAL[0]}")
fi
[[ "$SUMMARY" == "1" ]] && COMMAND+=(--summary)
[[ "$UNRESOLVED_ONLY" == "1" ]] && COMMAND+=(--unresolved-only)
[[ "$RESOLVE_ALL" == "1" ]] && COMMAND+=(--resolve-all-unresolved)
RESOLVE_THREAD_COUNT=${#RESOLVE_THREAD_IDS[@]}
for ((index = 0; index < RESOLVE_THREAD_COUNT; index++)); do
  COMMAND+=(--resolve-thread "${RESOLVE_THREAD_IDS[$index]}")
done

# Preserve the adapter's exit 75 external-wait signal for callers and workflow routing.
set +e
"${COMMAND[@]}" >"$REPORT_FILE"
COMMAND_STATUS=$?
set -e
if [[ "$COMMAND_STATUS" -ne 0 ]]; then
  exit "$COMMAND_STATUS"
fi

if [[ "$OUTPUT_JSON" == "1" ]]; then
  cat "$REPORT_FILE"
  exit 0
fi

python3 - "$REPORT_FILE" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
print("pr review thread closeout")
print(f"- pr: #{payload['pr']['number']} {payload['pr']['url']}")
print(f"- head_ref: {payload['pr']['head_ref']}")
print(f"- base_ref: {payload['pr']['base_ref']}")
print(f"- review_decision: {payload['pr']['review_decision']}")
print(f"- merge_state_status: {payload['pr']['merge_state_status']}")
print(f"- total_threads: {payload['summary']['total_threads']}")
print(f"- unresolved_threads: {payload['summary']['unresolved_threads']}")
print(f"- resolved_threads: {payload['summary']['resolved_threads']}")
if payload["resolved_now"]["count"] > 0:
    print(f"- resolved_now: {payload['resolved_now']['count']}")
threads = payload["threads"]
if not threads:
    print("- details: none")
    raise SystemExit(0)
print("- details:")
for thread in threads:
    status = "resolved" if thread["is_resolved"] else "unresolved"
    path = thread["path"] or "(no path)"
    line = thread["line"] if thread["line"] is not None else thread["original_line"]
    location = f"{path}:{line}" if line is not None else path
    print(f"  - {status} | {location} | thread_id={thread['id']}")
    if thread["is_outdated"]:
        print("    outdated: true")
    latest = thread.get("latest_comment")
    if latest:
        author = latest.get("author") or "unknown"
        body = (latest.get("body") or "").strip().replace("\n", " ")
        if len(body) > 140:
            body = body[:137] + "..."
        print(f"    latest_comment_by: {author}")
        print(f"    latest_comment_url: {latest.get('url')}")
        print(f"    latest_comment: {body}")
PY
