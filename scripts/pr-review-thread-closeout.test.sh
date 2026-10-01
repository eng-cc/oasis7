#!/usr/bin/env bash
# This fixture must remain compatible with POSIX and Git Bash with native Windows Python.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

SCRATCH_ROOT="$ROOT_DIR/.pm/scratch"
SCRATCH_ROOT_PREEXISTED=0
if [[ -e "$SCRATCH_ROOT" ]]; then
  SCRATCH_ROOT_PREEXISTED=1
fi
mkdir -p "$SCRATCH_ROOT"
TMPDIR="$(mktemp -d "$SCRATCH_ROOT/pr-review-thread-closeout-test.XXXXXX")"
cleanup() {
  local status=$?
  rm -rf "$TMPDIR"
  if [[ "$SCRATCH_ROOT_PREEXISTED" == "0" ]]; then
    rmdir "$SCRATCH_ROOT" 2>/dev/null || true
  fi
  exit "$status"
}
trap cleanup EXIT

TEST_ROOT="$TMPDIR/repo"
mkdir -p "$TEST_ROOT/scripts/pm" "$TEST_ROOT/bin"
cp "$ROOT_DIR/scripts/pr-review-thread-closeout.sh" "$TEST_ROOT/scripts/pr-review-thread-closeout.sh"
cp "$ROOT_DIR/scripts/pm/github_pr_snapshot.py" "$TEST_ROOT/scripts/pm/github_pr_snapshot.py"
cp "$ROOT_DIR/scripts/pm/fixtures/github_api_test_adapter.py" "$TEST_ROOT/scripts/pm/github_api.py"
cat > "$TEST_ROOT/scripts/worktree-harness-lib.sh" <<'SH'
wh_require_git_worktree() {
  git rev-parse --is-inside-work-tree >/dev/null 2>&1 || exit 1
}
SH
git -C "$TEST_ROOT" init -q
git -C "$TEST_ROOT" remote add origin https://github.com/eng-cc/oasis7.git

cat > "$TMPDIR/state.json" <<'EOF'
{
  "reviewThreadsPageInfo": {"hasNextPage": false, "endCursor": null},
  "threads": [
    {"id":"PRRT_1","isResolved":false,"isOutdated":false,"path":"doc/scripts/prd.md","line":111,"originalLine":111,"startLine":null,"originalStartLine":null,"comments":{"pageInfo":{"hasNextPage":false,"endCursor":null},"nodes":[{"id":"C_1","body":"Need a helper for review-thread closeout.","createdAt":"2026-04-23T12:00:00Z","url":"https://example.test/thread/1","author":{"login":"reviewer-a"}}]}},
    {"id":"PRRT_2","isResolved":false,"isOutdated":true,"path":"doc/scripts/project.md","line":250,"originalLine":249,"startLine":null,"originalStartLine":null,"comments":{"pageInfo":{"hasNextPage":false,"endCursor":null},"nodes":[{"id":"C_2","body":"Please update the project row too.","createdAt":"2026-04-23T12:10:00Z","url":"https://example.test/thread/2","author":{"login":"reviewer-b"}}]}},
    {"id":"PRRT_3","isResolved":true,"isOutdated":false,"path":"doc/engineering/project.md","line":148,"originalLine":148,"startLine":null,"originalStartLine":null,"comments":{"pageInfo":{"hasNextPage":false,"endCursor":null},"nodes":[{"id":"C_3","body":"Fixed in latest push.","createdAt":"2026-04-23T12:20:00Z","url":"https://example.test/thread/3","author":{"login":"reviewer-c"}}]}}
  ]
}
EOF

cat > "$TEST_ROOT/bin/gh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
STATE_FILE="$GH_FIXTURE_STATE"
QUERY=""
THREAD_ID=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    -f)
      [[ "$2" == query=* ]] && QUERY="${2#query=}"
      [[ "$2" == threadId=* ]] && THREAD_ID="${2#threadId=}"
      shift 2
      ;;
    -F)
      [[ "$2" == threadId=* ]] && THREAD_ID="${2#threadId=}"
      shift 2
      ;;
    *) shift ;;
  esac
done

if [[ "$QUERY" == *"resolveReviewThread"* ]]; then
  echo mutation >> "$GH_FIXTURE_CALLS"
  python3 - "$STATE_FILE" "$THREAD_ID" <<'PY'
import json,sys
from pathlib import Path
path=Path(sys.argv[1]); thread_id=sys.argv[2]
payload=json.loads(path.read_text())
for thread in payload["threads"]:
    if thread["id"] == thread_id:
        thread["isResolved"] = True
        break
path.write_text(json.dumps(payload))
print(json.dumps({"data":{"resolveReviewThread":{"thread":{"id":thread_id,"isResolved":True}}}}))
PY
  exit 0
fi

echo snapshot >> "$GH_FIXTURE_CALLS"
python3 - "$STATE_FILE" "$QUERY" <<'PY'
import json,sys
from pathlib import Path
state=json.loads(Path(sys.argv[1]).read_text())
threads=state["threads"]
for thread in threads:
    thread["comments"]["pageInfo"] = {"hasNextPage":False,"endCursor":None}
data={"viewer":{"login":"fixture-user"},"rateLimit":{"cost":1,"remaining":4999,"used":1,"resetAt":"2026-04-23T13:00:00Z","limit":5000},"repository":{"nameWithOwner":"eng-cc/oasis7","pullRequest":{"number":145,"url":"https://github.com/eng-cc/oasis7/pull/145","state":"OPEN","isDraft":False,"body":"","mergeable":"MERGEABLE","mergeStateStatus":"BLOCKED","reviewDecision":"REVIEW_REQUIRED","headRefName":"task/test","headRefOid":"0123456789abcdef","baseRefName":"main","baseRefOid":"fedcba9876543210","comments":{"pageInfo":{"hasNextPage":False,"endCursor":None},"nodes":[]},"reviews":{"pageInfo":{"hasNextPage":False,"endCursor":None},"nodes":[]},"reviewThreads":{"pageInfo":state.get("reviewThreadsPageInfo",{"hasNextPage":False,"endCursor":None}),"nodes":threads},"commits":{"nodes":[{"commit":{"oid":"0123456789abcdef","statusCheckRollup":{"contexts":{"pageInfo":{"hasNextPage":False,"endCursor":None},"nodes":[]}}}}]}}}}
print(json.dumps({"data":data}))
PY
SH
chmod +x "$TEST_ROOT/bin/gh"

export PATH="$TEST_ROOT/bin:$PATH"
export GH_FIXTURE_STATE="$TMPDIR/state.json"
export GH_FIXTURE_CALLS="$TMPDIR/calls.log"
: > "$GH_FIXTURE_CALLS"

REPORT_FILE="$TMPDIR/report.json"
"$TEST_ROOT/scripts/pr-review-thread-closeout.sh" 145 --json --unresolved-only > "$REPORT_FILE"
python3 - "$REPORT_FILE" <<'PY'
import json,sys
from pathlib import Path
payload=json.loads(Path(sys.argv[1]).read_text())
assert payload["pr"]["number"] == 145
assert payload["summary"]["total_threads"] == 3
assert payload["summary"]["reported_threads"] == 2
assert payload["summary"]["unresolved_threads"] == 2
assert not payload["summary"]["partial_scan"]
assert all(not thread["is_resolved"] for thread in payload["threads"])
assert payload["threads"][0]["latest_comment"]["body"] == "Need a helper for review-thread closeout."
PY

SUMMARY_FILE="$TMPDIR/summary.json"
"$TEST_ROOT/scripts/pr-review-thread-closeout.sh" 145 --json --summary > "$SUMMARY_FILE"
python3 - "$SUMMARY_FILE" <<'PY'
import json,sys
from pathlib import Path
path=Path(sys.argv[1]); payload=json.loads(path.read_text())
assert payload["summary"]["summary"] is True
assert payload["summary"]["reported_threads"] == 3
assert all(thread["latest_comment"] is None for thread in payload["threads"])
assert path.stat().st_size < 20_000
PY

RESOLVE_FILE="$TMPDIR/resolve.json"
"$TEST_ROOT/scripts/pr-review-thread-closeout.sh" 145 --json --resolve-all-unresolved > "$RESOLVE_FILE"
python3 - "$RESOLVE_FILE" "$GH_FIXTURE_CALLS" <<'PY'
import json,sys
from pathlib import Path
payload=json.loads(Path(sys.argv[1]).read_text())
calls=Path(sys.argv[2]).read_text().splitlines()
assert payload["resolved_now"]["count"] == 2
assert payload["summary"]["unresolved_threads"] == 0
assert payload["summary"]["resolved_threads"] == 3
assert calls[-4:] == ["snapshot", "mutation", "mutation", "snapshot"], calls
PY

python3 - "$TMPDIR/state.json" <<'PY'
import json,sys
from pathlib import Path
path=Path(sys.argv[1]); payload=json.loads(path.read_text())
payload["reviewThreadsPageInfo"]={"hasNextPage":True,"endCursor":"cursor-1"}
path.write_text(json.dumps(payload))
PY
PARTIAL_ERR="$TMPDIR/partial.err"
if "$TEST_ROOT/scripts/pr-review-thread-closeout.sh" 145 --json --unresolved-only > "$TMPDIR/partial.json" 2>"$PARTIAL_ERR"; then
  echo "expected partial review-thread scan to fail" >&2
  exit 1
fi
python3 - "$PARTIAL_ERR" <<'PY'
import json,sys
from pathlib import Path
payload=json.loads(Path(sys.argv[1]).read_text())
assert payload["reason"] == "incomplete_snapshot", payload
PY

set +e
GH_FIXTURE_EXTERNAL_WAIT=1 "$TEST_ROOT/scripts/pr-review-thread-closeout.sh" 145 --json \
  >"$TMPDIR/external-wait.json" 2>"$TMPDIR/external-wait.err"
EXTERNAL_WAIT_STATUS=$?
set -e
if [[ "$EXTERNAL_WAIT_STATUS" -ne 75 ]]; then
  echo "expected external-wait exit 75, got $EXTERNAL_WAIT_STATUS" >&2
  cat "$TMPDIR/external-wait.err" >&2
  exit 1
fi
python3 - "$TMPDIR/external-wait.err" <<'PY'
import json,sys
from pathlib import Path
payload=json.loads(Path(sys.argv[1]).read_text())
assert payload["status"] == "external_wait", payload
assert payload["retry_after_seconds"] == 120, payload
PY

python3 - "$ROOT_DIR/scripts/pr-review-thread-closeout.sh" <<'PY'
import pathlib,re,sys
source=pathlib.Path(sys.argv[1]).read_text(encoding='utf-8')
preamble=source.split('ROOT_DIR=',1)[0]
assert 'MSYS*|MINGW*|CYGWIN*' in preamble,preamble
assert 'export TMPDIR' in preamble,preamble
assert not re.search(r'^\s*\*\)',preamble,re.M),preamble
PY

python3 "$ROOT_DIR/scripts/pm/github_pr_snapshot.test.py"

echo "pr-review-thread-closeout.test: OK"
