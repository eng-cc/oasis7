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
  --task-uid <task_uid>      Persist the verification result into one task file
  --json                     Print machine-readable JSON summary
  -h, --help                 Show help

Examples:
  ./scripts/pm/claim-ready.sh --claim-type tests_passed --verify-command "./scripts/doc-governance-check.sh"
  ./scripts/pm/claim-ready.sh --claim-type ready_for_pr --verify-command "OASIS7_CI_RUN_OASIS7_REQUIRED_TESTS=false ./scripts/ci-tests.sh required" --task-uid task_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx --json
USAGE
}

die() {
  echo "claim-ready: $*" >&2
  exit 1
}

CLAIM_TYPE=""
VERIFY_COMMAND=""
OUTPUT_JSON=0
TASK_UID=""

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
    --task-uid)
      TASK_UID="${2:-}"
      shift 2
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

if [[ -n "$TASK_UID" && "$CLAIM_LABEL" != "task_complete" ]]; then
  python3 - "$ROOT_DIR" "$TASK_UID" "$CLAIM_LABEL" <<'PY'
from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

root = Path(sys.argv[1])
task_uid = sys.argv[2]
claim_type = sys.argv[3]
task_path = root / ".pm" / "tasks" / f"{task_uid}.yaml"
mapping_path = root / ".pm/github-project-sync/tasks.json"

if not task_path.exists() and mapping_path.exists():
    mapping = json.loads(mapping_path.read_text(encoding="utf-8"))
    record = (mapping.get("tasks") or {}).get(task_uid) or {}
    task_status = str(record.get("status") or "")
    last_closed_at = str(record.get("last_closed_at") or "")
    if task_status in {"done", "deferred"} and last_closed_at:
        raise SystemExit(
            "claim-ready: closed GitHub-backed task claim evidence is immutable for non-completion claims: "
            f"{task_uid} status={task_status} claim_type={claim_type}"
        )
    raise SystemExit(0)
if not task_path.exists() and not mapping_path.exists():
    repo = "eng-cc/oasis7"
    try:
        payload = subprocess.check_output(
            [
                "gh",
                "issue",
                "list",
                "-R",
                repo,
                "--search",
                f"{task_uid} in:body",
                "--json",
                "number",
                "--limit",
                "5",
            ],
            text=True,
            stderr=subprocess.PIPE,
            timeout=180,
        )
        hits = json.loads(payload)
        if isinstance(hits, list) and len(hits) == 1:
            issue_payload = subprocess.check_output(
                ["gh", "issue", "view", str(hits[0].get("number") or ""), "-R", repo, "--json", "body"],
                text=True,
                stderr=subprocess.PIPE,
                timeout=180,
            )
            body = str(json.loads(issue_payload).get("body") or "")
            status_match = re.search(r"^- status: `([^`]+)`$", body, re.MULTILINE)
            task_status = status_match.group(1) if status_match else ""
            if task_status in {"done", "deferred"}:
                raise SystemExit(
                    "claim-ready: closed GitHub-backed task claim evidence is immutable for non-completion claims: "
                    f"{task_uid} status={task_status} claim_type={claim_type}"
                )
            raise SystemExit(0)
    except (subprocess.CalledProcessError, json.JSONDecodeError, subprocess.TimeoutExpired):
        pass
    raise SystemExit(0)

fields: dict[str, str] = {}
for raw in task_path.read_text(encoding="utf-8").splitlines():
    if not raw or raw.startswith(" ") or raw.startswith("-"):
        continue
    key, sep, value = raw.partition(":")
    if not sep:
        continue
    fields[key.strip()] = value.strip().strip('"')

task_status = fields.get("status", "")
last_closed_at = fields.get("last_closed_at", "")
if task_status in {"done", "deferred"} and last_closed_at:
    raise SystemExit(
        "claim-ready: closed task claim evidence is immutable for non-completion claims: "
        f"{task_uid} status={task_status} claim_type={claim_type}"
    )
PY
fi

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
python3 - "$CLAIM_LABEL" "$VERIFY_COMMAND" "$VERIFIED_AT" "$VERIFY_EXIT_CODE" "$STATUS" "$ALLOWED_TO_CLAIM" "$CLAIM_MESSAGE" "$BLOCKED_PHRASE" "$SUCCESS_PHRASE" "$TASK_UID" <<'PY'
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
    "task_uid": sys.argv[10] or None,
}
print(json.dumps(payload, ensure_ascii=False))
PY
)"

if [[ -n "$TASK_UID" ]]; then
  if [[ -f "$ROOT_DIR/.pm/github-project-sync/tasks.json" ]]; then
    python3 - "$ROOT_DIR" "$TASK_UID" "$RESULT_JSON" <<'PY'
from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from pathlib import Path

root = Path(sys.argv[1])
task_uid = sys.argv[2]
claim = json.loads(sys.argv[3])
mapping_path = root / ".pm/github-project-sync/tasks.json"
mapping = json.loads(mapping_path.read_text(encoding="utf-8"))
record = (mapping.get("tasks") or {}).get(task_uid)
if not record:
    raise SystemExit(0)
record.setdefault("claim_verifications", []).append(claim)
record["last_claim_type"] = claim["claim_type"]
record["last_verify_command"] = claim["verify_command"]
record["last_verified_at"] = claim["verified_at"]
record["last_verification_exit_code"] = claim["verification_exit_code"]
record["last_verification_status"] = claim["status"]
record["last_claim_verification_at"] = claim["verified_at"]
record["updated_at"] = claim["verified_at"]
mapping_path.write_text(json.dumps(mapping, indent=2, sort_keys=True) + "\n", encoding="utf-8")
repo = ((mapping.get("project") or {}).get("repo") or "eng-cc/oasis7")
issue_number = int(record.get("issue_number") or 0)
if issue_number:
    body = "\n".join(
        [
            "<!-- oasis7-pm-claim-verification -->",
            f"Task UID: {task_uid}",
            f"Claim Type: {claim['claim_type']}",
            f"Verified At: {claim['verified_at']}",
            f"Verification Exit Code: {claim['verification_exit_code']}",
            f"Verification Status: {claim['status']}",
            f"Verify Command: {claim['verify_command']}",
            f"Claim Message: {claim['claim_message']}",
            "",
        ]
    )
    with tempfile.NamedTemporaryFile("w", encoding="utf-8", delete=False) as handle:
        handle.write(body)
        body_path = Path(handle.name)
    try:
        subprocess.run(
            ["gh", "issue", "comment", str(issue_number), "-R", str(repo), "--body-file", str(body_path)],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            text=True,
            timeout=180,
        )
    finally:
        body_path.unlink(missing_ok=True)
PY
  elif [[ -f "$ROOT_DIR/.pm/tasks/$TASK_UID.yaml" ]]; then
    "$ROOT_DIR/scripts/pm/pm_store.py" record-task-claim-verification "$ROOT_DIR" \
      --task-uid "$TASK_UID" \
      --claim-type "$CLAIM_LABEL" \
      --verify-command "$VERIFY_COMMAND" \
      --verified-at "$VERIFIED_AT" \
      --verification-exit-code "$VERIFY_EXIT_CODE" \
      --verification-status "$STATUS" >/dev/null
  else
    python3 - "$TASK_UID" "$RESULT_JSON" <<'PY'
from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from pathlib import Path

task_uid = sys.argv[1]
claim = json.loads(sys.argv[2])
repo = "eng-cc/oasis7"
try:
    payload = subprocess.check_output(
        [
            "gh",
            "issue",
            "list",
            "-R",
            repo,
            "--search",
            f"{task_uid} in:body",
            "--json",
            "number",
            "--limit",
            "5",
        ],
        text=True,
        stderr=subprocess.PIPE,
        timeout=180,
    )
    hits = json.loads(payload)
except (subprocess.CalledProcessError, json.JSONDecodeError, subprocess.TimeoutExpired):
    hits = []
if isinstance(hits, list) and len(hits) == 1 and hits[0].get("number"):
    body = "\n".join(
        [
            "<!-- oasis7-pm-claim-verification -->",
            f"Task UID: {task_uid}",
            f"Claim Type: {claim['claim_type']}",
            f"Verified At: {claim['verified_at']}",
            f"Verification Exit Code: {claim['verification_exit_code']}",
            f"Verification Status: {claim['status']}",
            f"Verify Command: {claim['verify_command']}",
            f"Claim Message: {claim['claim_message']}",
            "",
        ]
    )
    with tempfile.NamedTemporaryFile("w", encoding="utf-8", delete=False) as handle:
        handle.write(body)
        body_path = Path(handle.name)
    try:
        subprocess.run(
            ["gh", "issue", "comment", str(hits[0]["number"]), "-R", repo, "--body-file", str(body_path)],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            text=True,
            timeout=180,
        )
    finally:
        body_path.unlink(missing_ok=True)
PY
  fi
fi

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
