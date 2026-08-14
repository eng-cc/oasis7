#!/usr/bin/env bash
# Cross-platform test contract: partial bootstrap recovery remains valid on Windows Git Bash and Linux/macOS shells.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
REAL_GIT="$(command -v git)"
TMPDIR="$(mktemp -d)"
cleanup() {
  "$REAL_GIT" -C "$TMPDIR/repo" worktree remove --force "$TMPDIR/task-worktree" >/dev/null 2>&1 || true
  "$REAL_GIT" -C "$TMPDIR/repo" worktree remove --force "$TMPDIR/task-worktree-later" >/dev/null 2>&1 || true
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

REPO="$TMPDIR/repo"
TARGET="$TMPDIR/task-worktree"
BRANCH="task/engineering-partial-bootstrap"
mkdir -p "$REPO/scripts/pm"
cp "$ROOT_DIR/scripts/new-task-worktree.sh" "$REPO/scripts/new-task-worktree.sh"
cp "$ROOT_DIR/scripts/worktree-harness-lib.sh" "$REPO/scripts/worktree-harness-lib.sh"
cp "$ROOT_DIR/scripts/pm/find-python-with-module.sh" "$REPO/scripts/pm/find-python-with-module.sh"
cp "$ROOT_DIR/scripts/pm/pm_store.py" "$REPO/scripts/pm/pm_store.py"
cp "$ROOT_DIR/scripts/pm/resume-task-worktree-bootstrap.sh" "$REPO/scripts/pm/resume-task-worktree-bootstrap.sh"
cp "$ROOT_DIR/scripts/pm/bootstrap-task-snapshot.py" "$REPO/scripts/pm/bootstrap-task-snapshot.py"
chmod +x "$REPO/scripts/new-task-worktree.sh" "$REPO/scripts/pm/resume-task-worktree-bootstrap.sh"

cat >"$REPO/scripts/cargo-dev.sh" <<'EOF'
#!/usr/bin/env bash
[[ "${1:-}" == "--print-target-dir" ]]
printf '%s\n' "${TEST_SHARED_TARGET:?}"
EOF
cat >"$REPO/scripts/pm/new-task.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${TEST_BOOTSTRAP_STAGE:-remote}" == "later" ]]; then
  task_uid='task_22222222222222222222222222222222'
  journal_path="${TEST_BOOTSTRAP_JOURNAL_PATH:?}"
  mkdir -p "$(dirname "$journal_path")"
  cat >"$journal_path" <<JSON
{"version":2,"task_uid":"$task_uid","state":"project_item_added","next_action":"update_project_fields"}
JSON
  mkdir -p .pm/github-project-sync
  branch="$(git symbolic-ref --quiet --short HEAD)"
  cat >.pm/github-project-sync/tasks.json <<JSON
{"project":{"repo":"eng-cc/oasis7","owner":"eng-cc","number":1},"tasks":{"$task_uid":{"task_uid":"$task_uid","title":"partial later bootstrap fixture","issue_number":2200,"issue_url":"https://example.invalid/issues/2200","project_item_id":"ITEM-2200","status":"candidate","owner_role":"tpm","repository":"eng-cc/oasis7","canonical_worktree":"$PWD","task_branch":"$branch","default_branch":"main","acceptance":["resume bootstrap"],"bootstrap_epoch":1}}}
JSON
  printf '{"task_uid":"%s","task_path":"https://example.invalid/issues/2200","execution_log_path":"https://example.invalid/issues/2200","bootstrap_journal":"%s"}\n' "$task_uid" "$journal_path"
  exit 0
fi
mkdir -p .pm/scratch/bootstrap
mkdir -p .pm/scratch/bootstrap-journal
cat >.pm/scratch/bootstrap-journal/partial-remote.json <<'JSON'
{"version":2,"task_uid":"task_11111111111111111111111111111111","state":"issue_created","next_action":"add_project_item"}
JSON
cat >.pm/scratch/bootstrap/partial-remote.json <<'JSON'
{"task_uid":"task_11111111111111111111111111111111","issue_url":"https://example.invalid/issues/2198","stage":"issue_created"}
JSON
echo 'injected failure after remote issue creation' >&2
exit 77
EOF
cat >"$REPO/scripts/pm/move-task.sh" <<'EOF'
#!/usr/bin/env bash
if [[ "${TEST_RESUME_MODE:-0}" != "1" ]]; then
  exit 99
fi
python3 - "${PM_ROOT_DIR:?}/.pm/github-project-sync/tasks.json" "${2:?}" <<'PY'
import json
import sys
from pathlib import Path
path = Path(sys.argv[1])
payload = json.loads(path.read_text(encoding="utf-8"))
payload["tasks"][sys.argv[2]]["status"] = "committed"
path.write_text(json.dumps(payload) + "\n", encoding="utf-8")
PY
exit 0
EOF
cat >"$REPO/scripts/pm/refresh-task-cache.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
exit 0
EOF
cat >"$REPO/scripts/pm/workflow-report.sh" <<'EOF'
#!/usr/bin/env bash
if [[ "${TEST_RESUME_MODE:-0}" != "1" ]]; then
  exit 99
fi
python3 - "${PM_ROOT_DIR:?}/.pm/github-project-sync/tasks.json" "${6:?}" <<'PY'
import datetime as dt
import json
import sys
from pathlib import Path
path = Path(sys.argv[1])
payload = json.loads(path.read_text(encoding="utf-8"))
record = payload["tasks"][sys.argv[2]]
record.setdefault("last_started_at", dt.datetime.now(dt.timezone.utc).isoformat())
record["workflow_start_count"] = int(record.get("workflow_start_count") or 0) + 1
path.write_text(json.dumps(payload) + "\n", encoding="utf-8")
PY
exit 0
EOF
chmod +x "$REPO/scripts/cargo-dev.sh" "$REPO/scripts/pm/"*.sh

git -C "$REPO" init -q -b main
git -C "$REPO" config user.email test@example.invalid
git -C "$REPO" config user.name Test
git -C "$REPO" add .
git -C "$REPO" commit -qm fixture

set +e
(cd "$REPO" && TEST_SHARED_TARGET="$TMPDIR/shared-target" \
  ./scripts/new-task-worktree.sh engineering partial-bootstrap \
    --branch "$BRANCH" --path "$TARGET" \
    --pm-owner-role tpm --pm-title "partial bootstrap fixture" \
    --pm-source-ref doc/engineering/project.md) \
  >"$TMPDIR/bootstrap.out" 2>"$TMPDIR/bootstrap.err"
status=$?
set -e
if [[ "$status" != "77" ]]; then
  echo "expected injected bootstrap failure 77, got $status" >&2
  cat "$TMPDIR/bootstrap.err" >&2
  exit 1
fi

if [[ ! -d "$TARGET" ]]; then
  echo "partial remote bootstrap must preserve the canonical task worktree for recovery" >&2
  exit 1
fi
git -C "$REPO" show-ref --verify --quiet "refs/heads/$BRANCH"
test -f "$TARGET/.pm/scratch/bootstrap/partial-remote.json"
grep -F "resume-bootstrap" "$TMPDIR/bootstrap.err" >/dev/null
first_resume_line="$(grep '^resume-bootstrap-json: ' "$TMPDIR/bootstrap.err" | tail -n 1 || true)"
if [[ -z "$first_resume_line" ]]; then
  echo "initial partial bootstrap must expose a machine-readable resume contract" >&2
  exit 1
fi
python3 - "${first_resume_line#resume-bootstrap-json: }" <<'PY'
import json
import sys
contract = json.loads(sys.argv[1])
if contract.get("task_uid") != "task_11111111111111111111111111111111":
    raise SystemExit(f"initial resume contract lost the v2 journal task UID: {contract}")
if contract.get("journal_version") != 2:
    raise SystemExit(f"initial resume contract must bind journal version 2: {contract}")
if contract.get("idempotent") is not True:
    raise SystemExit(f"initial resume contract must be idempotent: {contract}")
PY
if grep -Fq "cleaned up created worktree" "$TMPDIR/bootstrap.err"; then
  echo "partial remote bootstrap must not claim destructive cleanup" >&2
  exit 1
fi

LATER_TARGET="$TMPDIR/task-worktree-later"
LATER_BRANCH="task/engineering-partial-bootstrap-later"
LATER_JOURNAL="$LATER_TARGET/.pm/scratch/bootstrap-journal/partial-later.json"
set +e
(cd "$REPO" && TEST_SHARED_TARGET="$TMPDIR/shared-target-later" \
  TEST_BOOTSTRAP_STAGE=later TEST_BOOTSTRAP_JOURNAL_PATH="$LATER_JOURNAL" \
  ./scripts/new-task-worktree.sh engineering partial-bootstrap-later \
    --branch "$LATER_BRANCH" --path "$LATER_TARGET" \
    --pm-owner-role tpm --pm-title "partial later bootstrap fixture" \
    --pm-source-ref doc/engineering/project.md) \
  >"$TMPDIR/bootstrap-later.out" 2>"$TMPDIR/bootstrap-later.err"
later_status=$?
set -e
if [[ "$later_status" != "99" ]]; then
  echo "expected injected later-stage bootstrap failure 99, got $later_status" >&2
  cat "$TMPDIR/bootstrap-later.err" >&2
  exit 1
fi
test -f "$LATER_JOURNAL"
later_resume_line="$(grep '^resume-bootstrap-json: ' "$TMPDIR/bootstrap-later.err" | tail -n 1 || true)"
if [[ -z "$later_resume_line" ]]; then
  echo "later-stage bootstrap must expose a machine-readable resume contract" >&2
  cat "$TMPDIR/bootstrap-later.err" >&2
  exit 1
fi
later_resume_json="${later_resume_line#resume-bootstrap-json: }"
python3 - "$later_resume_json" "$LATER_JOURNAL" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

contract = json.loads(sys.argv[1])
journal_path = Path(sys.argv[2]).resolve()
journal = json.loads(journal_path.read_text(encoding="utf-8"))
if contract.get("schema") != "oasis7.bootstrap_resume.v1":
    raise SystemExit(f"unexpected bootstrap resume schema: {contract}")
if contract.get("journal_path") != str(journal_path):
    raise SystemExit(f"resume contract must bind the existing journal path: {contract}")
if contract.get("journal_version") != journal.get("version") or contract.get("journal_version") != 2:
    raise SystemExit(f"resume contract must bind bootstrap journal version 2: {contract}")
if contract.get("task_uid") != journal.get("task_uid"):
    raise SystemExit(f"resume contract task UID must match the v2 journal: {contract}")
if contract.get("idempotent") is not True:
    raise SystemExit(f"resume contract must declare idempotent retry semantics: {contract}")
if not contract.get("next_action"):
    raise SystemExit(f"resume contract must expose the next bootstrap action: {contract}")
resume_argv = contract.get("resume_argv")
if not isinstance(resume_argv, list) or not all(isinstance(item, str) for item in resume_argv):
    raise SystemExit(f"resume contract must expose argv fields instead of only an unsafe shell string: {contract}")
if journal["task_uid"] not in resume_argv:
    raise SystemExit(f"resume argv must remain tied to the journal task UID: {contract}")
PY
python3 - "$later_resume_json" "$LATER_TARGET" "$TMPDIR/resume-first.json" "$TMPDIR/resume-second.json" <<'PY'
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

contract = json.loads(sys.argv[1])
target = Path(sys.argv[2])
first_path = Path(sys.argv[3])
second_path = Path(sys.argv[4])
resume_argv = contract.get("resume_argv")
if not isinstance(resume_argv, list) or not resume_argv:
    raise SystemExit(f"resume contract must provide an executable argv: {contract}")
env = dict(os.environ, TEST_RESUME_MODE="1", PM_ROOT_DIR=str(target))
for output_path in (first_path, second_path):
    with output_path.open("w", encoding="utf-8") as handle:
        subprocess.run(resume_argv, cwd=target, env=env, stdout=handle, check=True, text=True)
first = json.loads(first_path.read_text(encoding="utf-8"))
second = json.loads(second_path.read_text(encoding="utf-8"))
uid = "task_22222222222222222222222222222222"
if first.get("status") != "ok" or second.get("status") != "ok":
    raise SystemExit(f"resume helper must succeed on both attempts: {first} / {second}")
if first.get("task_uid") != uid or second.get("task_uid") != uid:
    raise SystemExit(f"resume helper changed task identity: {first} / {second}")
if not first.get("moved") or not first.get("workflow_started"):
    raise SystemExit(f"first resume must complete pending stages: {first}")
if second.get("moved") or second.get("workflow_started"):
    raise SystemExit(f"second resume must skip completed stages idempotently: {second}")
if first.get("bootstrap_snapshot_digest") != second.get("bootstrap_snapshot_digest"):
    raise SystemExit(f"resume must reuse the immutable bootstrap snapshot: {first} / {second}")
snapshot = target / ".pm" / "scratch" / uid / "bootstrap-task-snapshot.json"
if not snapshot.is_file():
    raise SystemExit(f"resume helper did not create the bootstrap snapshot: {snapshot}")
mapping = json.loads((target / ".pm" / "github-project-sync" / "tasks.json").read_text(encoding="utf-8"))
record = mapping["tasks"][uid]
if record.get("status") != "committed" or record.get("workflow_start_count") != 1:
    raise SystemExit(f"resume helper duplicated or skipped workflow start evidence: {record}")
PY
if grep -Fq "cleaned up created worktree" "$TMPDIR/bootstrap-later.err"; then
  echo "partial later bootstrap must not claim destructive cleanup" >&2
  exit 1
fi

echo "new-task-worktree-partial-bootstrap.test: OK"
