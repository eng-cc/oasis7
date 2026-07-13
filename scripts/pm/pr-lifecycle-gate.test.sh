#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

write_clean_fixture() {
  local path="$1"
  local hold="$2"
  cat >"$path" <<EOF
{
  "number": 2198,
  "url": "https://example.invalid/pull/2198",
  "state": "OPEN",
  "mergeable": "MERGEABLE",
  "mergeStateStatus": "CLEAN",
  "reviewDecision": "REVIEW_REQUIRED",
  "statusCheckRollup": [
    {"name": "required-gate", "status": "COMPLETED", "conclusion": "SUCCESS"}
  ],
  "comments": [],
  "reviews": [],
  "threads": [],
  "merge_hold": {"kind":"$hold","active": $([[ "$hold" == normal_pr_ci_watch ]] && echo false || echo true),"requester":"fixture-user","reason":"fixture reason","resume_authority":"fixture-user"}
}
EOF
}

write_clean_fixture "$TMPDIR/user-hold.json" user_requested_merge_hold
if python3 "$ROOT_DIR/scripts/pm/pr-lifecycle-gate.py" \
  --fixture "$TMPDIR/user-hold.json" --admin-merge-authorized --json \
  >"$TMPDIR/user-hold.out" 2>"$TMPDIR/user-hold.err"; then
  echo "expected user_requested_merge_hold to block merge even with admin authorization" >&2
  exit 1
fi
python3 - "$TMPDIR/user-hold.out" <<'PY'
import json, sys
payload = json.load(open(sys.argv[1], encoding="utf-8"))
assert payload["ready_for_merge"] is False, payload
assert any("user_requested_merge_hold" in item for item in payload["blockers"]), payload
PY

write_clean_fixture "$TMPDIR/comment.json" normal_pr_ci_watch
python3 - "$TMPDIR/comment.json" <<'PY'
import json, sys
path = sys.argv[1]
payload = json.load(open(path, encoding="utf-8"))
payload["comments"] = [{
    "id": "IC_kwDO_fixture",
    "body": "Please fix the cleanup race before merge.",
    "author": "reviewer",
    "actionable": True,
    "resolved": False,
}]
json.dump(payload, open(path, "w", encoding="utf-8"))
PY
if python3 "$ROOT_DIR/scripts/pm/pr-lifecycle-gate.py" \
  --fixture "$TMPDIR/comment.json" --json \
  >"$TMPDIR/comment.out" 2>"$TMPDIR/comment.err"; then
  echo "expected actionable general PR comment to block merge" >&2
  exit 1
fi
python3 - "$TMPDIR/comment.out" <<'PY'
import json, sys
payload = json.load(open(sys.argv[1], encoding="utf-8"))
assert payload["ready_for_merge"] is False, payload
assert any("conversation comment" in item for item in payload["blockers"]), payload
PY

write_clean_fixture "$TMPDIR/clean.json" normal_pr_ci_watch
python3 "$ROOT_DIR/scripts/pm/pr-lifecycle-gate.py" \
  --fixture "$TMPDIR/clean.json" --json >"$TMPDIR/clean.out"
python3 - "$TMPDIR/clean.out" <<'PY'
import json, sys
payload = json.load(open(sys.argv[1], encoding="utf-8"))
assert payload["ready_for_merge"] is True, payload
assert payload["blockers"] == [], payload
PY

echo "pr-lifecycle-gate.test: OK"
