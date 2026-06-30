#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${PM_ROOT_DIR:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

TMPDIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

TASK_UID="task_11111111111111111111111111111111"
mkdir -p "$TMPDIR/scripts/pm" "$TMPDIR/.pm/github-project-sync" "$TMPDIR/bin"
cp "$ROOT_DIR/scripts/pm/fallback-evidence.sh" "$TMPDIR/scripts/pm/fallback-evidence.sh"

cat > "$TMPDIR/.pm/github-project-sync/tasks.json" <<EOF
{
  "project": {
    "repo": "example/oasis7"
  },
  "tasks": {
    "$TASK_UID": {
      "issue_number": 123,
      "issue_url": "https://github.com/example/oasis7/issues/123",
      "project_item_id": "PVTI_fixture",
      "status": "ready"
    }
  },
  "version": 1
}
EOF

cat > "$TMPDIR/bin/gh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >> "$GH_CALL_LOG"
case "$*" in
  "issue comment 123 -R example/oasis7 --body-file "*)
    cat "${@: -1}" >> "$GH_BODY_LOG"
    printf '\n---\n' >> "$GH_BODY_LOG"
    ;;
  *)
    echo "unexpected gh invocation: $*" >&2
    exit 9
    ;;
esac
SH
chmod +x "$TMPDIR/bin/gh"
export PATH="$TMPDIR/bin:$PATH"
export GH_CALL_LOG="$TMPDIR/gh-calls.log"
export GH_BODY_LOG="$TMPDIR/gh-body.log"
: > "$GH_CALL_LOG"
: > "$GH_BODY_LOG"

printf 'fallback body\n' | PM_ROOT_DIR="$TMPDIR" "$TMPDIR/scripts/pm/fallback-evidence.sh" create \
  --task-uid "$TASK_UID" \
  --reason "fixture GitHub comment outage" \
  --json > "$TMPDIR/create.json"
printf 'second fallback body\n' | PM_ROOT_DIR="$TMPDIR" "$TMPDIR/scripts/pm/fallback-evidence.sh" create \
  --task-uid "$TASK_UID" \
  --reason "fixture GitHub comment outage" \
  --json > "$TMPDIR/create-second.json"

set +e
PM_ROOT_DIR="$TMPDIR" "$TMPDIR/scripts/pm/fallback-evidence.sh" audit --task-uid "$TASK_UID" --json > "$TMPDIR/audit-before.json"
AUDIT_BEFORE=$?
set -e
if [[ "$AUDIT_BEFORE" == "0" ]]; then
  echo "fallback-evidence.test: expected audit to fail before replay" >&2
  exit 1
fi

PM_ROOT_DIR="$TMPDIR" "$TMPDIR/scripts/pm/fallback-evidence.sh" replay --task-uid "$TASK_UID" --json > "$TMPDIR/replay.json"
PM_ROOT_DIR="$TMPDIR" "$TMPDIR/scripts/pm/fallback-evidence.sh" audit --task-uid "$TASK_UID" --json > "$TMPDIR/audit-after.json"

python3 - "$TMPDIR/create.json" "$TMPDIR/create-second.json" "$TMPDIR/audit-before.json" "$TMPDIR/replay.json" "$TMPDIR/audit-after.json" "$GH_CALL_LOG" "$GH_BODY_LOG" <<'PY'
import json
import pathlib
import sys

created = json.loads(pathlib.Path(sys.argv[1]).read_text())
created_second = json.loads(pathlib.Path(sys.argv[2]).read_text())
before = json.loads(pathlib.Path(sys.argv[3]).read_text())
replayed = json.loads(pathlib.Path(sys.argv[4]).read_text())
after = json.loads(pathlib.Path(sys.argv[5]).read_text())
calls = pathlib.Path(sys.argv[6]).read_text()
body = pathlib.Path(sys.argv[7]).read_text()

assert created["status"] == "created", created
assert created_second["status"] == "created", created_second
assert created["paths"][0] != created_second["paths"][0], (created, created_second)
assert before["status"] == "unreplayed", before
assert replayed["status"] == "replayed", replayed
assert len(replayed["paths"]) == 2, replayed
assert after["status"] == "ok", after
assert "issue comment 123 -R example/oasis7" in calls, calls
assert "<!-- oasis7-pm-fallback-evidence -->" in body, body
assert "fallback body" in body, body
assert "second fallback body" in body, body
assert replayed["paths"][0].endswith(".replayed.md"), replayed
PY

echo "fallback-evidence.test: OK"
