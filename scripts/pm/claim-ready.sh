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
  --verification-profile <name> Repository-owned named verification profile
  --task-uid <task_uid>      Persist the verification result into one task file
  --comparison-ref <ref>     Base ref for immutable range hygiene; derived from origin/main or main when omitted
  --pr-gate-json <path>      Fresh pr-lifecycle-gate JSON (required for ready_for_merge)
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
COMPARISON_REF=""
PR_GATE_JSON=""
VERIFICATION_PROFILE=""

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
    --verification-profile) VERIFICATION_PROFILE="${2:-}"; shift 2 ;;
    --json)
      OUTPUT_JSON=1
      shift
      ;;
    --task-uid)
      TASK_UID="${2:-}"
      shift 2
      ;;
    --comparison-ref)
      COMPARISON_REF="${2:-}"
      shift 2
      ;;
    --pr-gate-json) PR_GATE_JSON="${2:-}"; shift 2 ;;
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
if [[ -n "$VERIFICATION_PROFILE" ]]; then
  case "$VERIFICATION_PROFILE" in
    codex_subagent_role_fit) VERIFY_COMMAND="./scripts/pm/verify-codex-subagent-role-fit.sh" ;;
    workflow_behavior) VERIFY_COMMAND="./scripts/pm/workflow-behavior-eval.sh" ;;
    repository_required) VERIFY_COMMAND="./scripts/ci-tests.sh required" ;;
    fixture_repository_state)
      [[ "${OASIS7_ALLOW_FIXTURE_VERIFICATION_PROFILE:-0}" == "1" ]] || die "fixture_repository_state is test-only"
      VERIFY_COMMAND="${OASIS7_FIXTURE_VERIFICATION_COMMAND:-${VERIFY_COMMAND:-test -f .gitignore}}"
      ;;
    *) die "unknown repository verification profile: $VERIFICATION_PROFILE" ;;
  esac
elif [[ "$CLAIM_TYPE" == "task_complete" || "$CLAIM_TYPE" == "ready_for_pr" || "$CLAIM_TYPE" == "ready_for_merge" ]]; then
  die "$CLAIM_TYPE requires --verification-profile; arbitrary --verify-command is not lifecycle proof"
fi
[[ -n "$VERIFY_COMMAND" ]] || die "--verify-command or --verification-profile is required"

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

if [[ "$CLAIM_LABEL" == "ready_for_merge" ]]; then
  [[ -n "$PR_GATE_JSON" && -f "$PR_GATE_JSON" ]] || die "ready_for_merge requires --pr-gate-json from pr-lifecycle-gate.py"
  [[ -n "$TASK_UID" ]] || die "ready_for_merge requires --task-uid for live gate revalidation"
  python3 - "$PR_GATE_JSON" <<'PY'
import datetime as dt, json, re, sys
p = json.load(open(sys.argv[1], encoding="utf-8"))
if p.get("ready_for_merge") is not True or p.get("status") != "ready" or p.get("blockers"):
    raise SystemExit("claim-ready: PR lifecycle gate is not ready")
r = p.get("readiness_receipt")
if not isinstance(r, dict) or r.get("receipt_type") != "oasis7_pr_lifecycle_ready" or r.get("issuer") != "oasis7_pr_lifecycle_gate/v1":
    raise SystemExit("claim-ready: trusted live-gate readiness receipt is missing")
required = ("repository", "pr_number", "head_oid", "observed_at", "gate_epoch")
if any(not str(r.get(key) or "").strip() for key in required):
    raise SystemExit("claim-ready: readiness receipt is not bound to repo/pr/head/time/epoch")
if not re.fullmatch(r"[0-9a-f]{64}", str(r["gate_epoch"])):
    raise SystemExit("claim-ready: invalid readiness gate epoch")
try:
    observed = dt.datetime.fromisoformat(str(r["observed_at"]).replace("Z", "+00:00"))
except ValueError as exc:
    raise SystemExit("claim-ready: invalid readiness observed_at") from exc
age = dt.datetime.now(dt.timezone.utc) - observed.astimezone(dt.timezone.utc)
if age.total_seconds() < -30 or age.total_seconds() > 600:
    raise SystemExit("claim-ready: stale readiness receipt; rerun the live PR lifecycle gate")
PY
  PR_NUMBER="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["readiness_receipt"]["pr_number"])' "$PR_GATE_JSON")"
  USE_ADMIN_MERGE="$(python3 -c 'import json,sys; p=json.load(open(sys.argv[1])); print("true" if p.get("admin_merge_authorized") is True and p.get("use_admin_merge") is True else "false")' "$PR_GATE_JSON")"
  LIVE_GATE_JSON="$(mktemp)"
  LIVE_GATE_ARGS=("$PR_NUMBER" --root "$ROOT_DIR" --task-uid "$TASK_UID" --json)
  [[ "$USE_ADMIN_MERGE" != "true" ]] || LIVE_GATE_ARGS+=(--admin-merge-authorized)
  if ! python3 "$SCRIPT_DIR/pr-lifecycle-gate.py" "${LIVE_GATE_ARGS[@]}" >"$LIVE_GATE_JSON"; then
    rm -f "$LIVE_GATE_JSON"
    die "live PR lifecycle gate is not ready; rerun watch/fix before claiming merge readiness"
  fi
  python3 - "$PR_GATE_JSON" "$LIVE_GATE_JSON" <<'PY'
import json, sys
supplied=json.load(open(sys.argv[1],encoding="utf-8"))["readiness_receipt"]
live=json.load(open(sys.argv[2],encoding="utf-8")).get("readiness_receipt") or {}
for key in ("issuer","repository","pr_number","head_oid","gate_epoch"):
    if str(supplied.get(key)) != str(live.get(key)):
        raise SystemExit(f"claim-ready: readiness receipt drifted at {key}; use the fresh live gate output")
PY
  rm -f "$LIVE_GATE_JSON"
fi

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
FINGERPRINT_BEFORE="$(mktemp)"
FINGERPRINT_AFTER="$(mktemp)"
VERIFY_ROOT="$ROOT_DIR"
VERIFY_WORKTREE=""
cleanup() {
  if [[ -n "$VERIFY_WORKTREE" ]]; then
    git -C "$ROOT_DIR" worktree remove --force "$VERIFY_WORKTREE" >/dev/null 2>&1 || true
  fi
  rm -f "$STDOUT_CAPTURE" "$STDERR_CAPTURE" "$FINGERPRINT_BEFORE" "$FINGERPRINT_AFTER"
}
trap cleanup EXIT

FROZEN_HEAD=""
FROZEN_TREE=""
VERIFICATION_MODE="live_nonfinal"
if [[ "$CLAIM_LABEL" == "task_complete" || "$CLAIM_LABEL" == "ready_for_pr" ]]; then
  git -C "$ROOT_DIR" rev-parse --is-inside-work-tree >/dev/null 2>&1 \
    || die "$CLAIM_LABEL requires a Git worktree with an immutable committed source"
  DIRTY_IMPLEMENTATION_PATHS="$(git -C "$ROOT_DIR" status --porcelain --untracked-files=all | awk '
    { path=substr($0,4) }
    path == ".pm/github-project-sync/tasks.json" { next }
    path == ".pm/registry/tasks.yaml" { next }
    path ~ /^\.pm\/roles\/[^\/]+\/backlog\// { next }
    { print }
  ')"
  [[ -z "$DIRTY_IMPLEMENTATION_PATHS" ]] \
    || die "$CLAIM_LABEL requires a clean implementation-freeze commit; only generated task-cache/backlog evidence may differ: $DIRTY_IMPLEMENTATION_PATHS"
  FROZEN_HEAD="$(git -C "$ROOT_DIR" rev-parse HEAD)"
  FROZEN_TREE="$(git -C "$ROOT_DIR" rev-parse 'HEAD^{tree}')"
  if [[ -z "$COMPARISON_REF" ]]; then
    if git -C "$ROOT_DIR" rev-parse --verify refs/remotes/origin/main >/dev/null 2>&1; then
      COMPARISON_REF="refs/remotes/origin/main"
    elif git -C "$ROOT_DIR" rev-parse --verify main >/dev/null 2>&1; then
      COMPARISON_REF="main"
    elif git -C "$ROOT_DIR" rev-parse --verify HEAD^ >/dev/null 2>&1; then
      COMPARISON_REF="HEAD^"
    else
      COMPARISON_REF="$FROZEN_HEAD"
    fi
  fi
  git -C "$ROOT_DIR" rev-parse --verify "$COMPARISON_REF" >/dev/null 2>&1 \
    || die "comparison ref is not resolvable: $COMPARISON_REF"
  git -C "$ROOT_DIR" diff --check "$COMPARISON_REF...$FROZEN_HEAD" \
    || die "immutable comparison range failed git diff --check: $COMPARISON_REF...$FROZEN_HEAD"
  VERIFY_WORKTREE="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-claim-snapshot.XXXXXX")"
  rmdir "$VERIFY_WORKTREE"
  git -C "$ROOT_DIR" worktree add --detach "$VERIFY_WORKTREE" "$FROZEN_HEAD" >/dev/null
  if [[ -f "$ROOT_DIR/.pm/github-project-sync/tasks.json" ]]; then
    mkdir -p "$VERIFY_WORKTREE/.pm/github-project-sync"
    cp "$ROOT_DIR/.pm/github-project-sync/tasks.json" "$VERIFY_WORKTREE/.pm/github-project-sync/tasks.json"
  fi
  VERIFY_ROOT="$VERIFY_WORKTREE"
  VERIFICATION_MODE="detached_frozen_tree"
fi

python3 "$SCRIPT_DIR/repo-state-fingerprint.py" "$VERIFY_ROOT" >"$FINGERPRINT_BEFORE"

set +e
(
  cd "$VERIFY_ROOT"
  OASIS7_CLAIM_COMPARISON_REF="$COMPARISON_REF" /bin/bash -lc "$VERIFY_COMMAND"
) >"$STDOUT_CAPTURE" 2>"$STDERR_CAPTURE"
VERIFY_EXIT_CODE=$?
set -e

python3 "$SCRIPT_DIR/repo-state-fingerprint.py" "$VERIFY_ROOT" >"$FINGERPRINT_AFTER"
FINGERPRINT_BEFORE_SHA="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["sha256"])' "$FINGERPRINT_BEFORE")"
FINGERPRINT_AFTER_SHA="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["sha256"])' "$FINGERPRINT_AFTER")"
EPOCH_STABLE=true
if [[ "$FINGERPRINT_BEFORE_SHA" != "$FINGERPRINT_AFTER_SHA" ]]; then
  EPOCH_STABLE=false
  if [[ "$VERIFY_EXIT_CODE" == "0" ]]; then
    VERIFY_EXIT_CODE=86
  fi
  printf 'claim-ready: repository state changed during verification epoch\n' >&2
fi

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
python3 - "$CLAIM_LABEL" "$VERIFY_COMMAND" "$VERIFIED_AT" "$VERIFY_EXIT_CODE" "$STATUS" "$ALLOWED_TO_CLAIM" "$CLAIM_MESSAGE" "$BLOCKED_PHRASE" "$SUCCESS_PHRASE" "$TASK_UID" "$FINGERPRINT_BEFORE_SHA" "$FINGERPRINT_AFTER_SHA" "$EPOCH_STABLE" "$VERIFICATION_MODE" "$FROZEN_HEAD" "$FROZEN_TREE" "$COMPARISON_REF" "$VERIFICATION_PROFILE" <<'PY'
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
    "repository_fingerprint_before": sys.argv[11],
    "repository_fingerprint_after": sys.argv[12],
    "verification_epoch_stable": sys.argv[13] == "true",
    "verification_mode": sys.argv[14],
    "frozen_source_head": sys.argv[15] or None,
    "frozen_source_tree": sys.argv[16] or None,
    "comparison_ref": sys.argv[17] or None,
    "verification_profile": sys.argv[18] or None,
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
    python3 - "$ROOT_DIR" "$TASK_UID" "$RESULT_JSON" <<'PY'
from __future__ import annotations

import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

root = Path(sys.argv[1])
task_uid = sys.argv[2]
claim = json.loads(sys.argv[3])
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
    issue_number = int(hits[0]["number"])
    try:
        issue_payload = subprocess.check_output(
            [
                "gh",
                "issue",
                "view",
                str(issue_number),
                "-R",
                repo,
                "--json",
                "body,number,title,url",
            ],
            text=True,
            stderr=subprocess.PIPE,
            timeout=180,
        )
        issue = json.loads(issue_payload)
    except (subprocess.CalledProcessError, json.JSONDecodeError, subprocess.TimeoutExpired):
        issue = None

    if issue is not None:
        body_text = str(issue.get("body") or "")
        if not re.search(rf"^task_uid:\s*{re.escape(task_uid)}$", body_text, re.MULTILINE):
            issue = None

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
            ["gh", "issue", "comment", str(issue_number), "-R", repo, "--body-file", str(body_path)],
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
