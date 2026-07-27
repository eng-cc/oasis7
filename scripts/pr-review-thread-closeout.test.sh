#!/usr/bin/env bash
# This fixture must remain compatible with POSIX and Git Bash with native Windows Python.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

mkdir -p "$ROOT_DIR/.pm/scratch"
TMPDIR="$(mktemp -d "$ROOT_DIR/.pm/scratch/pr-review-thread-closeout-test.XXXXXX")"
cleanup() {
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

mkdir -p "$TMPDIR/bin"
cat > "$TMPDIR/state.json" <<'EOF'
{
  "reviewThreadsPageInfo": {
    "hasNextPage": false,
    "endCursor": null
  },
  "threads": [
    {
      "id": "PRRT_1",
      "isResolved": false,
      "isOutdated": false,
      "path": "doc/scripts/prd.md",
      "line": 111,
      "originalLine": 111,
      "startLine": null,
      "originalStartLine": null,
      "comments": {
        "pageInfo": {
          "hasNextPage": false,
          "endCursor": null
        },
        "nodes": [
          {
            "id": "C_1",
            "body": "Need a helper for review-thread closeout.",
            "createdAt": "2026-04-23T12:00:00Z",
            "url": "https://example.test/thread/1",
            "author": { "login": "reviewer-a" }
          }
        ]
      }
    },
    {
      "id": "PRRT_2",
      "isResolved": false,
      "isOutdated": true,
      "path": "doc/scripts/project.md",
      "line": 250,
      "originalLine": 249,
      "startLine": null,
      "originalStartLine": null,
      "comments": {
        "pageInfo": {
          "hasNextPage": false,
          "endCursor": null
        },
        "nodes": [
          {
            "id": "C_2",
            "body": "Please update the project row too.",
            "createdAt": "2026-04-23T12:10:00Z",
            "url": "https://example.test/thread/2",
            "author": { "login": "reviewer-b" }
          }
        ]
      }
    },
    {
      "id": "PRRT_3",
      "isResolved": true,
      "isOutdated": false,
      "path": "doc/engineering/project.md",
      "line": 148,
      "originalLine": 148,
      "startLine": null,
      "originalStartLine": null,
      "comments": {
        "pageInfo": {
          "hasNextPage": false,
          "endCursor": null
        },
        "nodes": [
          {
            "id": "C_3",
            "body": "Fixed in latest push.",
            "createdAt": "2026-04-23T12:20:00Z",
            "url": "https://example.test/thread/3",
            "author": { "login": "reviewer-c" }
          }
        ]
      }
    }
  ]
}
EOF

cat > "$TMPDIR/bin/gh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

STATE_FILE="$(dirname "$0")/../state.json"

if [[ "$1" == "repo" && "$2" == "view" ]]; then
  printf '{"name":"oasis7","owner":{"login":"eng-cc"}}\n'
  exit 0
fi

if [[ "$1" == "pr" && "$2" == "view" ]]; then
  printf '{"number":145,"url":"https://github.com/eng-cc/oasis7/pull/145","headRefName":"task/test","baseRefName":"main","reviewDecision":"REVIEW_REQUIRED","mergeStateStatus":"BLOCKED"}\n'
  exit 0
fi

if [[ "$1" == "api" && "$2" == "graphql" ]]; then
  shift 2
  query=""
  thread_id=""
  while [[ $# -gt 0 ]]; do
    case "$1" in
      -f)
        if [[ "$2" == query=* ]]; then
          query="${2#query=}"
          shift 2
        else
          shift 2
        fi
        ;;
      -F)
        case "$2" in
          threadId=*)
            thread_id="${2#threadId=}"
            ;;
        esac
        shift 2
        ;;
      *)
        shift
        ;;
    esac
  done

  if [[ "$query" == *"resolveReviewThread"* ]]; then
    python3 - "$STATE_FILE" "$thread_id" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

state_path = Path(sys.argv[1])
thread_id = sys.argv[2]
payload = json.loads(state_path.read_text(encoding="utf-8"))
for thread in payload["threads"]:
    if thread["id"] == thread_id:
        thread["isResolved"] = True
        break
state_path.write_text(json.dumps(payload), encoding="utf-8")
print(json.dumps({"data": {"resolveReviewThread": {"thread": {"id": thread_id, "isResolved": True}}}}))
PY
    exit 0
  fi

  python3 - "$STATE_FILE" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

state_path = Path(sys.argv[1])
payload = json.loads(state_path.read_text(encoding="utf-8"))
print(json.dumps({"data": {"repository": {"pullRequest": {"reviewThreads": {"pageInfo": payload.get("reviewThreadsPageInfo", {"hasNextPage": False, "endCursor": None}), "nodes": payload["threads"]}}}}}))
PY
  exit 0
fi

echo "unexpected gh invocation: $*" >&2
exit 1
EOF
chmod +x "$TMPDIR/bin/gh"

python3 - "$TMPDIR/state.json" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

state_path = Path(sys.argv[1])
payload = json.loads(state_path.read_text(encoding="utf-8"))
payload["threads"][0]["comments"]["nodes"][0]["body"] = "A" * 1_500_000
payload["threads"][1]["comments"]["nodes"][0]["body"] = "B" * 1_500_000
state_path.write_text(json.dumps(payload), encoding="utf-8")
PY

REPORT_FILE="$TMPDIR/report.json"
PATH="$TMPDIR/bin:$PATH" env -u TMPDIR "$ROOT_DIR/scripts/pr-review-thread-closeout.sh" 145 --json --unresolved-only > "$REPORT_FILE"
python3 - "$REPORT_FILE" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
if payload["summary"]["total_threads"] != 3:
    raise SystemExit("expected total_threads=3")
if payload["summary"]["reported_threads"] != 2:
    raise SystemExit("expected unresolved-only report to contain 2 threads")
if payload["summary"]["unresolved_threads"] != 2:
    raise SystemExit("expected unresolved_threads=2")
if payload["summary"]["partial_scan"]:
    raise SystemExit("expected complete review-thread scan")
if any(thread["is_resolved"] for thread in payload["threads"]):
    raise SystemExit("unresolved-only report should not contain resolved threads")
PY

RESOLVE_FILE="$TMPDIR/resolve.json"
PATH="$TMPDIR/bin:$PATH" "$ROOT_DIR/scripts/pr-review-thread-closeout.sh" 145 --json --resolve-all-unresolved > "$RESOLVE_FILE"
python3 - "$RESOLVE_FILE" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
if payload["resolved_now"]["count"] != 2:
    raise SystemExit("expected resolve-all to resolve 2 threads")
if payload["summary"]["unresolved_threads"] != 0:
    raise SystemExit("expected unresolved_threads=0 after resolve-all")
if payload["summary"]["resolved_threads"] != 3:
    raise SystemExit("expected all 3 threads to be resolved after mutation")
PY

python3 - "$TMPDIR/state.json" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

state_path = Path(sys.argv[1])
payload = json.loads(state_path.read_text(encoding="utf-8"))
payload["reviewThreadsPageInfo"] = {"hasNextPage": True, "endCursor": "cursor-1"}
state_path.write_text(json.dumps(payload), encoding="utf-8")
PY

PARTIAL_ERR="$TMPDIR/partial.err"
if PATH="$TMPDIR/bin:$PATH" env -u TMPDIR "$ROOT_DIR/scripts/pr-review-thread-closeout.sh" 145 --json --unresolved-only > "$TMPDIR/partial.json" 2>"$PARTIAL_ERR"; then
  echo "expected partial review-thread scan to fail" >&2
  exit 1
fi
if ! grep -q "review thread scan is partial" "$PARTIAL_ERR"; then
  echo "expected explicit partial scan error" >&2
  cat "$PARTIAL_ERR" >&2
  exit 1
fi

python3 - "$ROOT_DIR/scripts/pr-review-thread-closeout.sh" <<'PY'
import pathlib,re,sys
source=pathlib.Path(sys.argv[1]).read_text(encoding='utf-8')
preamble=source.split('ROOT_DIR=',1)[0]
assert 'MSYS*|MINGW*|CYGWIN*' in preamble,preamble
assert 'export TMPDIR' in preamble,preamble
assert not re.search(r'^\s*\*\)',preamble,re.M),preamble
PY

echo "pr-review-thread-closeout.test: OK"
