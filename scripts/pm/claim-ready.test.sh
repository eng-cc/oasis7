#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

TMPDIR="$(mktemp -d)"
export OASIS7_ALLOW_FIXTURE_VERIFICATION_PROFILE=1
cleanup() {
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

SUCCESS_JSON="$TMPDIR/success.json"
"$ROOT_DIR/scripts/pm/claim-ready.sh" \
  --claim-type tests_passed \
  --verify-command "printf 'fresh-ok\n'" \
  --json >"$SUCCESS_JSON"

python3 - "$SUCCESS_JSON" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
if payload["claim_type"] != "tests_passed":
    raise SystemExit("expected tests_passed claim type")
if payload["verification_exit_code"] != 0:
    raise SystemExit("expected success exit code")
if payload["status"] != "verified":
    raise SystemExit("expected verified status")
if payload["allowed_to_claim"] is not True:
    raise SystemExit("expected allowed_to_claim=true")
if payload["verification_epoch_stable"] is not True:
    raise SystemExit("expected stable repository verification epoch")
if "tests" not in payload["claim_message"]:
    raise SystemExit("expected success message to mention tests")
PY

FAIL_JSON="$TMPDIR/fail.json"
set +e
"$ROOT_DIR/scripts/pm/claim-ready.sh" \
  --claim-type tests_passed \
  --verify-command "printf 'boom\n' >&2; exit 7" \
  --json >"$FAIL_JSON"
FAIL_STATUS=$?
set -e

if [[ "$FAIL_STATUS" != "7" ]]; then
  echo "expected exit status 7 on failed verification, got $FAIL_STATUS" >&2
  exit 1
fi

python3 - "$FAIL_JSON" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
if payload["claim_type"] != "tests_passed":
    raise SystemExit("expected tests_passed claim type")
if payload["verification_exit_code"] != 7:
    raise SystemExit("expected failure exit code")
if payload["status"] != "blocked":
    raise SystemExit("expected blocked status")
if payload["allowed_to_claim"] is not False:
    raise SystemExit("expected allowed_to_claim=false")
if payload["claim_message"] != "Do not claim tests passed.":
    raise SystemExit("expected blocked message for tests_passed")
PY

mkdir -p "$TMPDIR/bin"
cat > "$TMPDIR/bin/gh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >> "${TEST_GH_LOG:?}"
case "$*" in
  "issue list -R eng-cc/oasis7 --search task_11111111111111111111111111111111 in:body --json number --limit 5")
    printf '[{"number":123}]\n'
    ;;
  "issue list -R eng-cc/oasis7 --search task_22222222222222222222222222222222 in:body --json number --limit 5")
    printf '[{"number":124}]\n'
    ;;
  "issue view 123 -R eng-cc/oasis7 --json body")
    printf '{"body":"<!-- oasis7-pm-task -->\\ntask_uid: task_11111111111111111111111111111111\\n\\nTask metadata:\\n- status: `ready`\\n"}\n'
    ;;
  "issue view 123 -R eng-cc/oasis7 --json body,number,title,url")
    printf '{"body":"<!-- oasis7-pm-task -->\\ntask_uid: task_11111111111111111111111111111111\\n\\nTask metadata:\\n- owner_role: `tpm`\\n- module: `engineering`\\n- status: `ready`\\n- priority: `P2`\\n- worktree_hint: `/tmp/no-cache`\\n","number":123,"title":"[PM] No-cache claim","url":"https://github.com/eng-cc/oasis7/issues/123"}\n'
    ;;
  "issue comment 123 -R eng-cc/oasis7 --body-file "*)
    cat "${@: -1}" > "${TEST_GH_COMMENT_BODY:?}"
    printf 'https://github.com/eng-cc/oasis7/issues/123#issuecomment-fixture\n'
    ;;
  "issue view 124 -R eng-cc/oasis7 --json body,number,title,url")
    echo "transient issue view failure" >&2
    exit 1
    ;;
  "issue comment 124 -R eng-cc/oasis7 --body-file "*)
    cat "${@: -1}" > "${TEST_GH_COMMENT_BODY_124:?}"
    printf 'https://github.com/eng-cc/oasis7/issues/124#issuecomment-fixture\n'
    ;;
  "project item-list 1 --owner eng-cc --limit 1000 --format json")
    printf '{"items":[{"id":"ITEM_123","content":{"url":"https://github.com/eng-cc/oasis7/issues/123","body":"<!-- oasis7-pm-task -->\\ntask_uid: task_11111111111111111111111111111111\\n"}}]}\n'
    ;;
  *)
    echo "unexpected gh invocation: $*" >&2
    exit 9
    ;;
esac
EOF
chmod +x "$TMPDIR/bin/gh"
NO_CACHE_ROOT="$TMPDIR/no-cache-root"
mkdir -p "$NO_CACHE_ROOT"
git -C "$NO_CACHE_ROOT" init -q
git -C "$NO_CACHE_ROOT" config user.email test@example.com
git -C "$NO_CACHE_ROOT" config user.name Test
git -C "$NO_CACHE_ROOT" commit --allow-empty -qm initial
TEST_GH_LOG="$TMPDIR/gh.log" TEST_GH_COMMENT_BODY="$TMPDIR/comment.md" PATH="$TMPDIR/bin:$PATH" PM_ROOT_DIR="$NO_CACHE_ROOT" \
  "$ROOT_DIR/scripts/pm/claim-ready.sh" \
  --claim-type ready_for_pr \
  --verification-profile fixture_repository_state \
  --verify-command "true" \
  --task-uid task_11111111111111111111111111111111 \
  --json >"$TMPDIR/no-cache-claim.json"

python3 - "$TMPDIR/no-cache-claim.json" "$TMPDIR/gh.log" "$TMPDIR/comment.md" "$NO_CACHE_ROOT/.pm/github-project-sync/tasks.json" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
gh_log = Path(sys.argv[2]).read_text(encoding="utf-8")
comment = Path(sys.argv[3]).read_text(encoding="utf-8")
mapping_path = Path(sys.argv[4])
if payload["status"] != "verified":
    raise SystemExit(f"expected verified no-cache claim, got {payload}")
if "issue list -R eng-cc/oasis7 --search task_11111111111111111111111111111111 in:body --json number --limit 5" not in gh_log:
    raise SystemExit(f"expected no-cache issue search, got {gh_log}")
if "<!-- oasis7-pm-claim-verification -->" not in comment:
    raise SystemExit(f"expected claim verification comment body, got {comment}")
if mapping_path.exists():
    raise SystemExit("claim-ready must not create or refresh the optional local task cache")
PY

NO_CACHE_FAIL_ROOT="$TMPDIR/no-cache-fail-root"
mkdir -p "$NO_CACHE_FAIL_ROOT"
git -C "$NO_CACHE_FAIL_ROOT" init -q
git -C "$NO_CACHE_FAIL_ROOT" config user.email test@example.com
git -C "$NO_CACHE_FAIL_ROOT" config user.name Test
git -C "$NO_CACHE_FAIL_ROOT" commit --allow-empty -qm initial
TEST_GH_LOG="$TMPDIR/gh-fail.log" TEST_GH_COMMENT_BODY="$TMPDIR/comment-unused.md" TEST_GH_COMMENT_BODY_124="$TMPDIR/comment-124.md" PATH="$TMPDIR/bin:$PATH" PM_ROOT_DIR="$NO_CACHE_FAIL_ROOT" \
  "$ROOT_DIR/scripts/pm/claim-ready.sh" \
  --claim-type ready_for_pr \
  --verification-profile fixture_repository_state \
  --verify-command "true" \
  --task-uid task_22222222222222222222222222222222 \
  --json >"$TMPDIR/no-cache-fail-claim.json"

python3 - "$TMPDIR/no-cache-fail-claim.json" "$TMPDIR/gh-fail.log" "$TMPDIR/comment-124.md" "$NO_CACHE_FAIL_ROOT/.pm/github-project-sync/tasks.json" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
gh_log = Path(sys.argv[2]).read_text(encoding="utf-8")
comment = Path(sys.argv[3]).read_text(encoding="utf-8")
mapping_path = Path(sys.argv[4])
if payload["status"] != "verified":
    raise SystemExit(f"expected verified claim despite issue view failure, got {payload}")
if mapping_path.exists():
    raise SystemExit("claim-ready must not create cache when issue body could not be fetched and verified")
if "project item-list" in gh_log:
    raise SystemExit(f"claim-ready must not recover Project item without verified issue body, got {gh_log}")
if "<!-- oasis7-pm-claim-verification -->" not in comment:
    raise SystemExit(f"expected fallback claim comment body, got {comment}")
PY

EPOCH_ROOT="$TMPDIR/epoch-root"
mkdir -p "$EPOCH_ROOT"
git -C "$EPOCH_ROOT" init -q
git -C "$EPOCH_ROOT" config user.email test@example.com
git -C "$EPOCH_ROOT" config user.name Test
printf 'before\n' >"$EPOCH_ROOT/tracked.txt"
git -C "$EPOCH_ROOT" add tracked.txt
git -C "$EPOCH_ROOT" commit -qm initial
set +e
PM_ROOT_DIR="$EPOCH_ROOT" "$ROOT_DIR/scripts/pm/claim-ready.sh" \
  --claim-type task_complete \
  --verification-profile fixture_repository_state \
  --verify-command "printf 'during-verify\\n' >> tracked.txt" \
  --json >"$TMPDIR/epoch-drift.json" 2>"$TMPDIR/epoch-drift.err"
EPOCH_STATUS=$?
set -e
if [[ "$EPOCH_STATUS" != "86" ]]; then
  echo "expected epoch drift status 86, got $EPOCH_STATUS" >&2
  exit 1
fi
python3 - "$TMPDIR/epoch-drift.json" <<'PY'
import json, sys
payload = json.load(open(sys.argv[1], encoding="utf-8"))
if payload["verification_epoch_stable"] is not False or payload["allowed_to_claim"] is not False:
    raise SystemExit(f"expected blocked unstable epoch, got {payload}")
if payload["repository_fingerprint_before"] == payload["repository_fingerprint_after"]:
    raise SystemExit("expected distinct epoch fingerprints")
PY
grep -F "repository state changed during verification epoch" "$TMPDIR/epoch-drift.err" >/dev/null

IMMUTABLE_ROOT="$TMPDIR/immutable-root"
mkdir -p "$IMMUTABLE_ROOT"
git -C "$IMMUTABLE_ROOT" init -q
git -C "$IMMUTABLE_ROOT" config user.email test@example.com
git -C "$IMMUTABLE_ROOT" config user.name Test
printf 'frozen\n' >"$IMMUTABLE_ROOT/tracked.txt"
git -C "$IMMUTABLE_ROOT" add tracked.txt
git -C "$IMMUTABLE_ROOT" commit -qm frozen
(
  while [[ ! -e "$TMPDIR/immutable-verify-started" ]]; do sleep 0.01; done
  printf 'transient-live-change\n' >"$IMMUTABLE_ROOT/tracked.txt"
  sleep 0.05
  printf 'frozen\n' >"$IMMUTABLE_ROOT/tracked.txt"
) &
MUTATOR_PID=$!
PM_ROOT_DIR="$IMMUTABLE_ROOT" "$ROOT_DIR/scripts/pm/claim-ready.sh" \
  --claim-type task_complete \
  --verification-profile fixture_repository_state \
  --verify-command "touch '$TMPDIR/immutable-verify-started'; sleep 0.2; test \"\$(cat tracked.txt)\" = frozen" \
  --json >"$TMPDIR/immutable-claim.json"
wait "$MUTATOR_PID"
python3 - "$TMPDIR/immutable-claim.json" <<'PY'
import json, sys
payload=json.load(open(sys.argv[1], encoding="utf-8"))
assert payload["status"] == "verified", payload
assert payload["verification_mode"] == "detached_frozen_tree", payload
assert payload["frozen_source_head"] and payload["frozen_source_tree"], payload
PY

RANGE_BAD_ROOT="$TMPDIR/range-bad-root"
mkdir -p "$RANGE_BAD_ROOT"
git -C "$RANGE_BAD_ROOT" init -q
git -C "$RANGE_BAD_ROOT" config user.email test@example.com
git -C "$RANGE_BAD_ROOT" config user.name Test
printf 'base\n' >"$RANGE_BAD_ROOT/file.txt"
git -C "$RANGE_BAD_ROOT" add file.txt && git -C "$RANGE_BAD_ROOT" commit -qm base
printf 'trailing whitespace   \n' >>"$RANGE_BAD_ROOT/file.txt"
git -C "$RANGE_BAD_ROOT" add file.txt && git -C "$RANGE_BAD_ROOT" commit -qm bad
set +e
PM_ROOT_DIR="$RANGE_BAD_ROOT" "$ROOT_DIR/scripts/pm/claim-ready.sh" \
  --claim-type task_complete --verification-profile fixture_repository_state --comparison-ref HEAD^ --verify-command true --json \
  >"$TMPDIR/range-bad.json" 2>"$TMPDIR/range-bad.err"
RANGE_BAD_STATUS=$?
set -e
[[ "$RANGE_BAD_STATUS" != "0" ]]
grep -F "immutable comparison range failed git diff --check" "$TMPDIR/range-bad.err" >/dev/null

RANGE_CLEAN_ROOT="$TMPDIR/range-clean-root"
mkdir -p "$RANGE_CLEAN_ROOT"
git -C "$RANGE_CLEAN_ROOT" init -q
git -C "$RANGE_CLEAN_ROOT" config user.email test@example.com
git -C "$RANGE_CLEAN_ROOT" config user.name Test
printf 'base\n' >"$RANGE_CLEAN_ROOT/file.txt"
git -C "$RANGE_CLEAN_ROOT" add file.txt && git -C "$RANGE_CLEAN_ROOT" commit -qm base
printf 'clean change\n' >>"$RANGE_CLEAN_ROOT/file.txt"
git -C "$RANGE_CLEAN_ROOT" add file.txt && git -C "$RANGE_CLEAN_ROOT" commit -qm clean
PM_ROOT_DIR="$RANGE_CLEAN_ROOT" "$ROOT_DIR/scripts/pm/claim-ready.sh" \
  --claim-type task_complete --verification-profile fixture_repository_state --comparison-ref HEAD^ --verify-command true --json \
  >"$TMPDIR/range-clean.json"
python3 - "$TMPDIR/range-clean.json" <<'PY'
import json, sys
payload=json.load(open(sys.argv[1], encoding="utf-8"))
assert payload["status"] == "verified", payload
assert payload["comparison_ref"] == "HEAD^", payload
PY

# A caller-supplied readiness receipt is accepted only when its gate epoch is
# exactly the epoch recomputed by the fresh live gate.  Comparing merely
# repo/PR/head lets a stale decision from the same head survive policy/comment
# changes.
python3 - "$ROOT_DIR/scripts/pm/claim-ready.sh" <<'PY'
import re,sys
text=open(sys.argv[1],encoding='utf-8').read()
match=re.search(r'for key in \(([^\n]+)\):\n\s+if str\(supplied', text)
if not match or '"gate_epoch"' not in match.group(1):
    raise SystemExit('RED claim-ready: supplied gate_epoch is not compared with the fresh live gate epoch')
if '--admin-merge-authorized' in text:
    raise SystemExit('claim-ready must not require a per-task admin merge selection flag')
PY

# The workflow-behavior profile includes the task-transition fixture whose fake
# GitHub client kills its parent.  The target must name that interrupt target
# explicitly, isolate the fixture, and keep the eval caller alive.
python3 - "$ROOT_DIR/scripts/pm/workflow-behavior-eval.sh" "$ROOT_DIR/scripts/pm/github-project-task.test.sh" <<'PY'
from pathlib import Path
import sys
eval_text=Path(sys.argv[1]).read_text(encoding="utf-8")
fixture_text=Path(sys.argv[2]).read_text(encoding="utf-8")
if "GH_INTERRUPT_TARGET" not in fixture_text:
    raise SystemExit("RED claim-ready: github-project-task fixture kills an implicit PPID instead of an explicit interrupt target")
if "run_interrupt_isolated" not in eval_text or 'run_interrupt_isolated "$ROOT_DIR/scripts/pm/github-project-task.test.sh"' not in eval_text:
    raise SystemExit("RED claim-ready: workflow_behavior does not isolate github-project-task's parent-kill fixture")
if "caller-survived" not in eval_text:
    raise SystemExit("RED claim-ready: workflow_behavior has no caller-continuation proof after the isolated interrupt")
PY

echo "claim-ready.test: OK"
