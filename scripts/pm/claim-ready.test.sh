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

echo "claim-ready.test: OK"
