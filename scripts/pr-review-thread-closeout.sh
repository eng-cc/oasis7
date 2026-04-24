#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
source "$ROOT_DIR/scripts/worktree-harness-lib.sh"

usage() {
  cat <<'USAGE'
Usage: ./scripts/pr-review-thread-closeout.sh [pr-number] [options]

Inspect GitHub PR review threads for the current PR (or one explicit PR), and
optionally resolve selected threads as part of the same-PR comment closeout
loop.

Default conventions:
- PR: infer from current branch via `gh pr view`
- action: report only
- standard path: inspect threads -> patch/validate/push -> resolve threads -> recheck PR state

Options:
  --unresolved-only          Only report unresolved threads
  --resolve-thread <id>      Resolve one explicit review thread id (repeatable)
  --resolve-all-unresolved   Resolve every currently unresolved review thread
  --json                     Print machine-readable JSON summary only
  -h, --help                 Show help

Examples:
  ./scripts/pr-review-thread-closeout.sh
  ./scripts/pr-review-thread-closeout.sh 145 --json
  ./scripts/pr-review-thread-closeout.sh --unresolved-only
  ./scripts/pr-review-thread-closeout.sh --resolve-thread PRRT_kwDOGA
  ./scripts/pr-review-thread-closeout.sh --resolve-all-unresolved --json
USAGE
}

die() {
  echo "pr-review-thread-closeout: $*" >&2
  exit 1
}

command -v gh >/dev/null 2>&1 || die "`gh` is required"

OUTPUT_JSON=0
UNRESOLVED_ONLY=0
RESOLVE_ALL=0
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
  die "expected at most one optional [pr-number]"
fi
if [[ "$RESOLVE_ALL" == "1" && "${#RESOLVE_THREAD_IDS[@]}" -gt 0 ]]; then
  die "--resolve-all-unresolved cannot be combined with --resolve-thread"
fi
PR_SELECTOR="${POSITIONAL[0]:-}"
if [[ -z "$PR_SELECTOR" ]]; then
  wh_require_git_worktree
fi

PR_VIEW_ARGS=(pr view)
if [[ -n "$PR_SELECTOR" ]]; then
  PR_VIEW_ARGS+=("$PR_SELECTOR")
fi
PR_VIEW_ARGS+=(--json number,url,headRefName,baseRefName,reviewDecision,mergeStateStatus)

fetch_pr_view_json() {
  gh "${PR_VIEW_ARGS[@]}"
}

TMP_DIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

PR_VIEW_FILE="$TMP_DIR/pr-view.json"
REPO_FILE="$TMP_DIR/repo.json"
THREADS_FILE="$TMP_DIR/threads.json"
REPORT_FILE="$TMP_DIR/report.json"

THREAD_QUERY="$(cat <<'EOF'
query($owner:String!, $repo:String!, $number:Int!) {
  repository(owner:$owner, name:$repo) {
    pullRequest(number:$number) {
      reviewThreads(first:100) {
        nodes {
          id
          isResolved
          isOutdated
          path
          line
          originalLine
          startLine
          originalStartLine
          comments(first:20) {
            nodes {
              id
              body
              createdAt
              url
              author {
                login
              }
            }
          }
        }
      }
    }
  }
}
EOF
)"

RESOLVE_QUERY="$(cat <<'EOF'
mutation($threadId:ID!) {
  resolveReviewThread(input: {threadId: $threadId}) {
    thread {
      id
      isResolved
    }
  }
}
EOF
)"

render_report_file() {
  python3 - "$1" "$2" "$3" > "$REPORT_FILE" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

pr_view = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
threads_payload = json.loads(Path(sys.argv[2]).read_text(encoding="utf-8"))
unresolved_only = sys.argv[3] == "1"

threads = (
    threads_payload.get("data", {})
    .get("repository", {})
    .get("pullRequest", {})
    .get("reviewThreads", {})
    .get("nodes", [])
)

entries: list[dict[str, object]] = []
for thread in threads:
    comments = thread.get("comments", {}).get("nodes", [])
    latest_comment = comments[-1] if comments else None
    entries.append(
        {
            "id": thread.get("id"),
            "is_resolved": bool(thread.get("isResolved")),
            "is_outdated": bool(thread.get("isOutdated")),
            "path": thread.get("path"),
            "line": thread.get("line"),
            "original_line": thread.get("originalLine"),
            "start_line": thread.get("startLine"),
            "original_start_line": thread.get("originalStartLine"),
            "comment_count": len(comments),
            "latest_comment": {
                "author": (latest_comment or {}).get("author", {}).get("login"),
                "body": (latest_comment or {}).get("body"),
                "created_at": (latest_comment or {}).get("createdAt"),
                "url": (latest_comment or {}).get("url"),
            }
            if latest_comment
            else None,
        }
    )

reported_entries = [
    entry for entry in entries if not unresolved_only or not entry["is_resolved"]
]

payload = {
    "pr": {
        "number": pr_view.get("number"),
        "url": pr_view.get("url"),
        "head_ref": pr_view.get("headRefName"),
        "base_ref": pr_view.get("baseRefName"),
        "review_decision": pr_view.get("reviewDecision"),
        "merge_state_status": pr_view.get("mergeStateStatus"),
    },
    "summary": {
        "total_threads": len(entries),
        "unresolved_threads": sum(1 for entry in entries if not entry["is_resolved"]),
        "resolved_threads": sum(1 for entry in entries if entry["is_resolved"]),
        "reported_threads": len(reported_entries),
        "unresolved_only": unresolved_only,
    },
    "resolved_now": {
        "count": 0,
        "thread_ids": [],
    },
    "threads": reported_entries,
}
print(json.dumps(payload, ensure_ascii=True, indent=2))
PY
}

refresh_pr_view_file() {
  fetch_pr_view_json > "$PR_VIEW_FILE"
}

refresh_threads_file() {
  gh api graphql \
    -f query="$THREAD_QUERY" \
    -F owner="$OWNER_LOGIN" \
    -F repo="$REPO_NAME" \
    -F number="$PR_NUMBER" > "$THREADS_FILE"
}

annotate_resolved_now() {
  python3 - "$REPORT_FILE" "${THREAD_IDS_TO_RESOLVE[@]}" > "$REPORT_FILE.next" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
thread_ids = sys.argv[2:]
payload["resolved_now"]["count"] = len(thread_ids)
payload["resolved_now"]["thread_ids"] = thread_ids
print(json.dumps(payload, ensure_ascii=True, indent=2))
PY
  mv "$REPORT_FILE.next" "$REPORT_FILE"
}

refresh_pr_view_file
gh repo view --json owner,name > "$REPO_FILE"

OWNER_LOGIN="$(python3 - "$REPO_FILE" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
print(payload["owner"]["login"])
PY
)"
REPO_NAME="$(python3 - "$REPO_FILE" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
print(payload["name"])
PY
)"
PR_NUMBER="$(python3 - "$PR_VIEW_FILE" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
print(payload["number"])
PY
)"

refresh_threads_file
render_report_file "$PR_VIEW_FILE" "$THREADS_FILE" "$UNRESOLVED_ONLY"

mapfile -t ALL_UNRESOLVED_IDS < <(python3 - "$THREADS_FILE" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
threads = (
    payload.get("data", {})
    .get("repository", {})
    .get("pullRequest", {})
    .get("reviewThreads", {})
    .get("nodes", [])
)
for thread in threads:
    if not thread.get("isResolved"):
        print(thread.get("id"))
PY
)

THREAD_IDS_TO_RESOLVE=()
if [[ "$RESOLVE_ALL" == "1" ]]; then
  THREAD_IDS_TO_RESOLVE=("${ALL_UNRESOLVED_IDS[@]}")
elif [[ "${#RESOLVE_THREAD_IDS[@]}" -gt 0 ]]; then
  THREAD_IDS_TO_RESOLVE=("${RESOLVE_THREAD_IDS[@]}")
fi

if [[ "${#THREAD_IDS_TO_RESOLVE[@]}" -gt 0 ]]; then
  for thread_id in "${THREAD_IDS_TO_RESOLVE[@]}"; do
    if ! printf '%s\n' "${ALL_UNRESOLVED_IDS[@]}" | grep -qx "$thread_id"; then
      die "thread is not currently unresolved on PR #$PR_NUMBER: $thread_id"
    fi
    gh api graphql -f query="$RESOLVE_QUERY" -F threadId="$thread_id" >/dev/null
  done
  refresh_pr_view_file
  refresh_threads_file
  render_report_file "$PR_VIEW_FILE" "$THREADS_FILE" "$UNRESOLVED_ONLY"
  annotate_resolved_now
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
