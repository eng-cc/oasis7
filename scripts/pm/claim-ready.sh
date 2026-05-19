#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${PM_ROOT_DIR:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

usage() {
  cat <<'USAGE'
Usage: ./scripts/pm/claim-ready.sh --claim-type <type> --verify-command <command> [options]

Execute a fresh verification command immediately before making a completion or
readiness claim. The helper only permits the claim when the verification
command succeeds in the current run.

Claim types:
  task_complete    Verification required before claiming the task is complete
  tests_passed     Verification required before claiming tests passed
  ready_for_pr     Verification required before claiming the branch is ready for PR
  ready_for_merge  Verification required before claiming the PR is ready to merge

Options:
  --claim-type <type>        Claim category to guard
  --verify-command <cmd>     Fresh verification command to execute via `bash -lc`
  --json                     Print machine-readable JSON summary
  -h, --help                 Show help

Examples:
  ./scripts/pm/claim-ready.sh --claim-type tests_passed --verify-command "./scripts/doc-governance-check.sh"
  ./scripts/pm/claim-ready.sh --claim-type ready_for_pr --verify-command "OASIS7_CI_RUN_OASIS7_REQUIRED_TESTS=false ./scripts/ci-tests.sh required" --json
USAGE
}

die() {
  echo "claim-ready: $*" >&2
  exit 1
}

CLAIM_TYPE=""
VERIFY_COMMAND=""
OUTPUT_JSON=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --claim-type)
      CLAIM_TYPE="${2:-}"
      shift 2
      ;;
    --verify-command)
      VERIFY_COMMAND="${2:-}"
      shift 2
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
      die "unknown argument: $1"
      ;;
  esac
done

[[ -n "$CLAIM_TYPE" ]] || die "--claim-type is required"
[[ -n "$VERIFY_COMMAND" ]] || die "--verify-command is required"

CLAIM_LABEL=""
BLOCKED_PHRASE=""
SUCCESS_PHRASE=""
case "$CLAIM_TYPE" in
  task_complete)
    CLAIM_LABEL="task_complete"
    BLOCKED_PHRASE="Do not claim the task is complete."
    SUCCESS_PHRASE="Fresh verification passed; the task can now be claimed complete."
    ;;
  tests_passed)
    CLAIM_LABEL="tests_passed"
    BLOCKED_PHRASE="Do not claim tests passed."
    SUCCESS_PHRASE="Fresh verification passed; tests can now be claimed passed."
    ;;
  ready_for_pr)
    CLAIM_LABEL="ready_for_pr"
    BLOCKED_PHRASE="Do not claim the branch is ready for PR."
    SUCCESS_PHRASE="Fresh verification passed; the branch can now be claimed ready for PR."
    ;;
  ready_for_merge)
    CLAIM_LABEL="ready_for_merge"
    BLOCKED_PHRASE="Do not claim the PR is ready to merge."
    SUCCESS_PHRASE="Fresh verification passed; the PR can now be claimed ready to merge."
    ;;
  *)
    die "unsupported --claim-type: $CLAIM_TYPE"
    ;;
esac

STDOUT_CAPTURE="$(mktemp)"
STDERR_CAPTURE="$(mktemp)"
cleanup() {
  rm -f "$STDOUT_CAPTURE" "$STDERR_CAPTURE"
}
trap cleanup EXIT

set +e
(
  cd "$ROOT_DIR"
  /bin/bash -lc "$VERIFY_COMMAND"
) >"$STDOUT_CAPTURE" 2>"$STDERR_CAPTURE"
VERIFY_EXIT_CODE=$?
set -e

VERIFIED_AT="$(date -Iseconds)"
STATUS="verified"
ALLOWED_TO_CLAIM="true"
CLAIM_MESSAGE="$SUCCESS_PHRASE"
if [[ "$VERIFY_EXIT_CODE" != "0" ]]; then
  STATUS="blocked"
  ALLOWED_TO_CLAIM="false"
  CLAIM_MESSAGE="$BLOCKED_PHRASE"
fi

RESULT_JSON="$(
python3 - "$CLAIM_LABEL" "$VERIFY_COMMAND" "$VERIFIED_AT" "$VERIFY_EXIT_CODE" "$STATUS" "$ALLOWED_TO_CLAIM" "$CLAIM_MESSAGE" "$BLOCKED_PHRASE" "$SUCCESS_PHRASE" <<'PY'
from __future__ import annotations

import json
import sys

payload = {
    "claim_type": sys.argv[1],
    "verify_command": sys.argv[2],
    "verified_at": sys.argv[3],
    "verification_exit_code": int(sys.argv[4]),
    "status": sys.argv[5],
    "allowed_to_claim": sys.argv[6] == "true",
    "claim_message": sys.argv[7],
    "blocked_phrase": sys.argv[8],
    "success_phrase": sys.argv[9],
}
print(json.dumps(payload, ensure_ascii=False))
PY
)"

if [[ "$OUTPUT_JSON" == "1" ]]; then
  printf '%s\n' "$RESULT_JSON"
else
  if [[ -s "$STDOUT_CAPTURE" ]]; then
    cat "$STDOUT_CAPTURE"
  fi
  if [[ -s "$STDERR_CAPTURE" ]]; then
    cat "$STDERR_CAPTURE" >&2
  fi

  echo "claim verification summary"
  echo "- claim_type: $CLAIM_LABEL"
  echo "- verify_command: $VERIFY_COMMAND"
  echo "- verified_at: $VERIFIED_AT"
  echo "- verification_exit_code: $VERIFY_EXIT_CODE"
  echo "- status: $STATUS"
  echo "- allowed_to_claim: $ALLOWED_TO_CLAIM"
  echo "- claim_message: $CLAIM_MESSAGE"
fi

if [[ "$VERIFY_EXIT_CODE" != "0" ]]; then
  exit "$VERIFY_EXIT_CODE"
fi
