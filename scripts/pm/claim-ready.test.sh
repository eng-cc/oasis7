#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

TMPDIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

SUCCESS_JSON="$TMPDIR/success.json"
"$ROOT_DIR/scripts/pm/claim-ready.sh" \
  --claim-type ready_for_pr \
  --verify-command "printf 'fresh-ok\n'" \
  --json >"$SUCCESS_JSON"

python3 - "$SUCCESS_JSON" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
if payload["claim_type"] != "ready_for_pr":
    raise SystemExit("expected ready_for_pr claim type")
if payload["verification_exit_code"] != 0:
    raise SystemExit("expected success exit code")
if payload["status"] != "verified":
    raise SystemExit("expected verified status")
if payload["allowed_to_claim"] is not True:
    raise SystemExit("expected allowed_to_claim=true")
if "ready for PR" not in payload["claim_message"]:
    raise SystemExit("expected success message to mention ready for PR")
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
TEST_GH_LOG="$TMPDIR/gh.log" TEST_GH_COMMENT_BODY="$TMPDIR/comment.md" PATH="$TMPDIR/bin:$PATH" PM_ROOT_DIR="$NO_CACHE_ROOT" \
  "$ROOT_DIR/scripts/pm/claim-ready.sh" \
  --claim-type ready_for_pr \
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
mapping = json.loads(Path(sys.argv[4]).read_text(encoding="utf-8"))
if payload["status"] != "verified":
    raise SystemExit(f"expected verified no-cache claim, got {payload}")
if "issue list -R eng-cc/oasis7 --search task_11111111111111111111111111111111 in:body --json number --limit 5" not in gh_log:
    raise SystemExit(f"expected no-cache issue search, got {gh_log}")
if "<!-- oasis7-pm-claim-verification -->" not in comment:
    raise SystemExit(f"expected claim verification comment body, got {comment}")
record = mapping["tasks"]["task_11111111111111111111111111111111"]
if record["status"] != "ready":
    raise SystemExit(f"expected recovered ready status, got {record}")
if record["project_item_id"] != "ITEM_123":
    raise SystemExit(f"expected recovered project item id, got {record}")
if record["claim_verifications"][-1]["claim_type"] != "ready_for_pr":
    raise SystemExit(f"expected persisted no-cache claim verification, got {record}")
PY

NO_CACHE_FAIL_ROOT="$TMPDIR/no-cache-fail-root"
mkdir -p "$NO_CACHE_FAIL_ROOT"
TEST_GH_LOG="$TMPDIR/gh-fail.log" TEST_GH_COMMENT_BODY="$TMPDIR/comment-unused.md" TEST_GH_COMMENT_BODY_124="$TMPDIR/comment-124.md" PATH="$TMPDIR/bin:$PATH" PM_ROOT_DIR="$NO_CACHE_FAIL_ROOT" \
  "$ROOT_DIR/scripts/pm/claim-ready.sh" \
  --claim-type ready_for_pr \
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

echo "claim-ready.test: OK"
