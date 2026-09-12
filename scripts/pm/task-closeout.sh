#!/usr/bin/env bash
# Cross-platform maintenance: keep temporary paths readable by Git Bash and native Windows Python.
set -euo pipefail

case "$(uname -s)" in
  MSYS*|MINGW*|CYGWIN*)
    if [[ -z "${TMPDIR:-}" || "$TMPDIR" == "/tmp" || "$TMPDIR" == "/tmp/" ]]; then
      TMPDIR="$(cygpath -m "${TEMP:-${TMP:-/tmp}}")"
    elif [[ "$TMPDIR" == /* ]]; then
      TMPDIR="$(cygpath -m "$TMPDIR")"
    fi
    export TMPDIR
    ;;
esac

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${PM_ROOT_DIR:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

usage() {
  cat <<'USAGE'
Usage: ./scripts/pm/task-closeout.sh --role <role> --task-uid <task_uid> [options]

Record GitHub Project-backed workflow closeout without recreating repo-local
.pm/tasks files. By default this marks the branch ready for PR, not final done.

Options:
  --role <role>           Owner role for workflow close evidence
  --task-uid <task_uid>   Task to close
  --to-status <status>    Target task status: ready, done, or deferred (default: ready)
  --verification-profile <name> Repository-owned named verification profile
  --claim-type <type>     Claim type (default: ready_for_pr for ready, task_complete for done)
  --comparison-ref <ref>  Immutable diff base; should match the review packet Comparison Ref
  --review-packet-file <path> Passed review packet bound to frozen HEAD (required for ready)
  --ci-ready-receipt <path> Trusted CI receipt bound to the reviewed draft head
  --pr-receipt <path>    Trusted merged-PR receipt (required for PR-backed done)
  --traceability-mode <leaf|aggregate>
                          Optional coordinating-record preflight mode
  --traceability-record <path>
                          Frozen coordinating record JSON
  --traceability-candidate <path>
                          Aggregate candidate/evidence JSON
  --no-lint               Accepted for compatibility; legacy PM lint is not run
  --json                  Print machine-readable JSON summary only
  -h, --help              Show help

Recovery after partial remote closeout:
  ./scripts/pm/refresh-task-cache.sh --task-uid <task_uid> --json
  ./scripts/pm/github-project-workflow.sh --json audit --task-uid <task_uid>
  # then retry task-closeout with the same frozen source/comparison ref
USAGE
}

die() {
  echo "task-closeout: $*" >&2
  exit 1
}

ROLE=""
TASK_UID=""
TARGET_STATUS="ready"
VERIFY_COMMAND=""
VERIFICATION_PROFILE=""
CLAIM_TYPE=""
COMPARISON_REF=""
OUTPUT_JSON=0
REVIEW_PACKET_FILE=""
CI_READY_RECEIPT=""
PR_MERGE_RECEIPT=""
REVIEW_PLAN_SCHEMA=""
TRACEABILITY_MODE=""
TRACEABILITY_RECORD=""
TRACEABILITY_CANDIDATE=""
TRACEABILITY_RESULT_JSON='{"status":"skipped","reason":"ordinary lifecycle closeout"}'

while [[ $# -gt 0 ]]; do
  case "$1" in
    --role)
      ROLE="${2:-}"
      shift 2
      ;;
    --task-uid)
      TASK_UID="${2:-}"
      shift 2
      ;;
    --to-status)
      TARGET_STATUS="${2:-}"
      shift 2
      ;;
    --verify-command)
      VERIFY_COMMAND="${2:-}"
      shift 2
      ;;
    --verification-profile) VERIFICATION_PROFILE="${2:-}"; shift 2 ;;
    --claim-type)
      CLAIM_TYPE="${2:-}"
      shift 2
      ;;
    --comparison-ref)
      COMPARISON_REF="${2:-}"
      shift 2
      ;;
    --review-packet-file) REVIEW_PACKET_FILE="${2:-}"; shift 2 ;;
    --ci-ready-receipt) CI_READY_RECEIPT="${2:-}"; shift 2 ;;
    --pr-receipt) PR_MERGE_RECEIPT="${2:-}"; shift 2 ;;
    --traceability-mode)
      TRACEABILITY_MODE="${2:-}"
      shift 2
      ;;
    --traceability-record)
      TRACEABILITY_RECORD="${2:-}"
      shift 2
      ;;
    --traceability-candidate)
      TRACEABILITY_CANDIDATE="${2:-}"
      shift 2
      ;;
    --no-lint)
      shift
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

[[ -n "$ROLE" ]] || die "--role is required"
[[ -n "$TASK_UID" ]] || die "--task-uid is required"
[[ "$TARGET_STATUS" == "ready" || "$TARGET_STATUS" == "done" || "$TARGET_STATUS" == "deferred" ]] || die "--to-status must be ready, done, or deferred"
[[ -z "$TRACEABILITY_MODE" || "$TRACEABILITY_MODE" == "leaf" || "$TRACEABILITY_MODE" == "aggregate" ]] \
  || die "--traceability-mode must be leaf or aggregate"
if [[ -n "$TRACEABILITY_RECORD" && ! -f "$TRACEABILITY_RECORD" ]]; then
  die "traceability record cannot be read: $TRACEABILITY_RECORD"
fi
if [[ -n "$TRACEABILITY_CANDIDATE" && ! -f "$TRACEABILITY_CANDIDATE" ]]; then
  die "traceability candidate cannot be read: $TRACEABILITY_CANDIDATE"
fi
if [[ -z "$CLAIM_TYPE" ]]; then
  if [[ "$TARGET_STATUS" == "done" ]]; then
    CLAIM_TYPE="task_complete"
  else
    CLAIM_TYPE="ready_for_pr"
  fi
fi
if [[ "$TARGET_STATUS" != "deferred" && -z "$VERIFICATION_PROFILE" ]]; then
  die "--verification-profile is required when --to-status is ready or done"
fi
if [[ -n "$VERIFY_COMMAND" ]]; then
  die "--verify-command is not accepted for lifecycle transitions; select a repository-owned --verification-profile"
fi
if [[ "$TARGET_STATUS" == "done" && "$CLAIM_TYPE" != "task_complete" ]]; then
  die "--claim-type must be task_complete when --to-status is done"
fi
if [[ "$TARGET_STATUS" == "ready" && "$CLAIM_TYPE" != "ready_for_pr" ]]; then
  die "--claim-type must be ready_for_pr when --to-status is ready"
fi

# Ordinary lifecycle closeout does not need a traceability context read. A
# declared binding or traceability field is the local, bounded signal that the
# transition audit must also provide selected-task traceability context.
TRACEABILITY_CONTEXT_REQUIRED=0
if [[ -n "$TRACEABILITY_MODE" || -n "$TRACEABILITY_RECORD" || -n "$TRACEABILITY_CANDIDATE" ]]; then
  TRACEABILITY_CONTEXT_REQUIRED=1
else
  TRACEABILITY_CONTEXT_REQUIRED="$(python3 - "$ROOT_DIR/.pm/github-project-sync/tasks.json" "$TASK_UID" <<'PY'
import json, sys
try:
    mapping = json.load(open(sys.argv[1], encoding='utf-8'))
    task = (mapping.get('tasks') or {}).get(sys.argv[2]) or {}
except (OSError, ValueError, TypeError):
    task = {}
fields = (
    task.get('loop_binding'), task.get('traceability_mode'),
    task.get('traceability_record'), task.get('coordination_record'),
    task.get('traceability_candidate'), task.get('aggregate_candidate'),
)
print('1' if any(fields) or task.get('completion_mode') == 'aggregate' else '0')
PY
)" || die "cannot determine whether traceability context is required"
fi

selected_task_audit() {
  # The selected-task audit is read-only. The marker lets fixture callers
  # return traceability context without counting it as a lifecycle audit.
  local context_only="${1:-0}"
  OASIS7_TRACEABILITY_CONTEXT_ONLY="$context_only" \
    "$SCRIPT_DIR/github-project-workflow.sh" --json audit --task-uid "$TASK_UID"
}

SELECTED_TRACEABILITY_CONTEXT_JSON=""

run_traceability_preflight() {
  # This is a projection boundary only. The core helper owns all record,
  # candidate, reader, and digest validation; closeout enforces its result
  # after selected-task context and before any remote lifecycle write.
  local output
  output="$(python3 - "$ROOT_DIR" "$TASK_UID" "$TRACEABILITY_MODE" "$TRACEABILITY_RECORD" "$TRACEABILITY_CANDIDATE" "$SCRIPT_DIR" "$SELECTED_TRACEABILITY_CONTEXT_JSON" <<'PY'
import contextlib
import importlib.util
import json
import subprocess
import sys
from pathlib import Path

# Loading the detached effective helper must not write interpreter bytecode into
# that temporary Git worktree. Cleanup must continue to reject real dirty files.
sys.dont_write_bytecode = True

root = Path(sys.argv[1]).resolve()
task_uid, requested_mode = sys.argv[2], sys.argv[3]
record_arg, candidate_arg, script_dir = sys.argv[4], sys.argv[5], Path(sys.argv[6]).resolve()
try:
    selected_context = json.loads(sys.argv[7])
except (TypeError, ValueError, json.JSONDecodeError) as exc:
    raise SystemExit('selected live task audit returned invalid JSON: ' + str(exc))
if not isinstance(selected_context, dict) or selected_context.get('status') != 'ok':
    raise SystemExit('selected live task audit is not authoritative')
selected_task = selected_context.get('selected_task')
if not isinstance(selected_task, dict):
    raise SystemExit('selected live task audit omitted selected task context')
live_uid = selected_task.get('task_uid') or selected_context.get('task_uid')
if live_uid != task_uid:
    raise SystemExit('selected task UID does not match requested task')
mapping_path = root / '.pm/github-project-sync/tasks.json'

try:
    mapping = json.loads(mapping_path.read_text(encoding='utf-8'))
    task = (mapping.get('tasks') or {})[task_uid]
except (OSError, ValueError, KeyError, TypeError) as exc:
    raise SystemExit('traceability task context unavailable: ' + str(exc))
if not isinstance(task, dict):
    raise SystemExit('traceability task context is not an object')
if task.get('task_uid', task_uid) != task_uid:
    raise SystemExit('local selected task UID does not match requested task')

binding = task.get('loop_binding') if isinstance(task.get('loop_binding'), dict) else {}
live_binding_value = selected_task.get('loop_binding') or selected_context.get('loop_binding')
live_binding = live_binding_value if isinstance(live_binding_value, dict) else None
live_change_id = selected_task.get('change_id') or selected_context.get('change_id')
if live_change_id is not None and task.get('change_id') not in (None, live_change_id):
    raise SystemExit('selected live task change_id does not match local task context')
if live_binding is not None and binding and live_binding != binding:
    raise SystemExit('selected live task loop binding does not match local task context')
authoritative_binding = live_binding or binding
declared_ref = (selected_task.get('coordination_ref')
                or task.get('coordination_ref')
                or authoritative_binding.get('coordination_ref'))
declared_record = (selected_task.get('traceability_record')
                   or task.get('traceability_record')
                   or task.get('coordination_record'))
declared_candidate = (selected_task.get('traceability_candidate')
                      or selected_task.get('aggregate_candidate')
                      or task.get('traceability_candidate')
                      or task.get('aggregate_candidate'))
record = None
if record_arg:
    try:
        record = json.loads(Path(record_arg).expanduser().resolve().read_text(encoding='utf-8'))
    except (OSError, ValueError) as exc:
        raise SystemExit('traceability coordinating record cannot be read: ' + str(exc))
elif isinstance(declared_record, dict):
    record = declared_record

if record is not None:
    if record.get('task_uid') != task_uid:
        raise SystemExit('coordinating record does not match selected task UID')
    if live_change_id is not None and record.get('change_id') != live_change_id:
        raise SystemExit('coordinating record change_id does not match selected task')
    if isinstance(authoritative_binding, dict):
        if authoritative_binding.get('task_uid') not in (None, task_uid):
            raise SystemExit('selected task binding UID does not match requested task')
        binding_change_id = authoritative_binding.get('change_id')
        if binding_change_id is not None and record.get('change_id') != binding_change_id:
            raise SystemExit('coordinating record change_id does not match selected task binding')
        binding_ref = authoritative_binding.get('coordination_ref')
        record_ref = record.get('coordination_ref')
        if isinstance(binding_ref, dict) and record_ref != binding_ref:
            raise SystemExit('coordinating record authority does not match selected task binding')

declared_aggregate = (
    requested_mode == 'aggregate'
    or selected_task.get('traceability_mode') == 'aggregate'
    or selected_task.get('completion_mode') == 'aggregate'
    or selected_context.get('traceability_mode') == 'aggregate'
    or selected_context.get('completion_mode') == 'aggregate'
    or task.get('traceability_mode') == 'aggregate'
    or task.get('completion_mode') == 'aggregate'
)
live_mode = (selected_task.get('traceability_mode')
             or selected_task.get('completion_mode')
             or selected_context.get('traceability_mode')
             or selected_context.get('completion_mode'))
if live_mode in {'leaf', 'aggregate'} and requested_mode and requested_mode != live_mode:
    raise SystemExit('traceability closeout mode does not match selected live task context')
if live_mode == 'leaf' and declared_aggregate:
    raise SystemExit('aggregate traceability context does not match selected live task')
mode = live_mode or requested_mode or ('aggregate' if declared_aggregate else ('leaf' if record is not None else ''))
if not requested_mode and not declared_aggregate and isinstance(declared_ref, dict):
    mode = 'leaf'
if declared_aggregate and mode == 'leaf':
    raise SystemExit('traceability closeout cannot downgrade aggregate context to leaf')
if not mode:
    print(json.dumps({'status': 'skipped', 'reason': 'ordinary lifecycle closeout'}, sort_keys=True))
    raise SystemExit(0)
if record is None:
    raise SystemExit('traceability ' + mode + ' closeout requires coordinating record')
if mode == 'aggregate' and not candidate_arg and declared_candidate is None:
    raise SystemExit('traceability aggregate closeout requires aggregate candidate')

candidate = None
evidence = []
if candidate_arg:
    try:
        candidate_payload = json.loads(Path(candidate_arg).expanduser().resolve().read_text(encoding='utf-8'))
    except (OSError, ValueError) as exc:
        raise SystemExit('traceability aggregate candidate cannot be read: ' + str(exc))
elif isinstance(declared_candidate, dict):
    candidate_payload = declared_candidate
else:
    candidate_payload = None
if mode == 'aggregate':
    if not isinstance(candidate_payload, dict):
        raise SystemExit('traceability aggregate candidate is not an object')
    # Only one explicit envelope shape is accepted. The core validates every
    # candidate/evidence identity; a CLI field cannot replace the record.
    candidate = candidate_payload.get('candidate', candidate_payload.get('aggregate_candidate', candidate_payload))
    evidence = candidate_payload.get('evidence', candidate_payload.get('leaf_evidence', []))
    if not isinstance(candidate, dict) or not isinstance(evidence, list):
        raise SystemExit('traceability aggregate candidate/evidence envelope is invalid')
    if isinstance(declared_candidate, dict) and candidate_payload != declared_candidate:
        raise SystemExit('traceability aggregate candidate does not match declared candidate')

record_source_commit = ((record.get('coordination_ref') or {}).get('source_commit')
                        if isinstance(record, dict) else None)
effective_binding = authoritative_binding if isinstance(authoritative_binding, dict) else binding
effective_tool_commit = effective_binding.get('policy_commit') if isinstance(effective_binding, dict) else None
if not isinstance(record_source_commit, str):
    raise SystemExit('traceability closeout requires immutable record_source_commit')
if not isinstance(effective_tool_commit, str):
    raise SystemExit('traceability closeout requires immutable effective_tool_commit')
import tempfile

def run_pinned_preflight():
    tool_root = None
    worktree_added = False
    failure = None
    result = None
    temporary = None
    try:
        temporary = tempfile.TemporaryDirectory(prefix='oasis7-loop-tools-')
        with contextlib.nullcontext(temporary.name) as temporary_path:
            tool_root = Path(temporary_path) / 'tools'
            subprocess.run(
                ['git', '-C', str(root), 'fetch', '--no-tags', 'origin', 'main:refs/remotes/origin/main'],
                check=True,
                capture_output=True,
            )
            subprocess.run(
                ['git', '-C', str(root), 'worktree', 'add', '--detach', str(tool_root), effective_tool_commit],
                check=True,
                capture_output=True,
            )
            worktree_added = True
            tool_scripts = tool_root / 'scripts' / 'pm'
            if not tool_scripts.is_dir():
                raise ValueError('effective traceability tool root is unavailable')
            sys.path.insert(0, str(tool_scripts))
            facade_spec = importlib.util.spec_from_file_location(
                'closeout_loop_facade', tool_scripts / 'loop.py'
            )
            if facade_spec is None or facade_spec.loader is None:
                raise ValueError('effective loop facade cannot be loaded')
            facade = importlib.util.module_from_spec(facade_spec)
            facade_spec.loader.exec_module(facade)
            trusted_loader = getattr(facade, 'trusted_module', None)
            if not callable(trusted_loader):
                raise ValueError('effective loop facade lacks trusted_module')
            helper = trusted_loader(tool_root, root, effective_binding, 'loop_traceability')

            authority_factory = getattr(helper, 'live_authority_reader', None)
            if callable(authority_factory):
                raw_authority_reader = authority_factory(root)
            else:
                authority = (getattr(helper, 'GitHubAuthorityReader', None)
                             or getattr(helper, 'GitHubAuthority', None))
                raw_authority_reader = authority(root) if callable(authority) else None
            if not callable(raw_authority_reader):
                def raw_authority_reader(reference):
                    raise RuntimeError('live traceability authority reader unavailable')

            def live_authority_reader(reference):
                value = raw_authority_reader(reference)
                if (getattr(live_authority_reader, 'reader_kind', None) is None
                        and isinstance(value, dict)
                        and value.get('reader_kind') == 'github_live_query'):
                    live_authority_reader.reader_kind = 'github_live_query'
                return value

            if getattr(raw_authority_reader, 'reader_kind', None) == 'github_live_query':
                live_authority_reader.reader_kind = 'github_live_query'

            def live_contract_reader(reference):
                factory = getattr(helper, 'live_contract_reader', None)
                if callable(factory):
                    return factory(root)(reference)
                authority = (getattr(helper, 'ImmutableSourceReader', None)
                             or getattr(helper, 'GitHubContractReader', None))
                if callable(authority):
                    return authority(root, record_source_commit)(reference)
                raise RuntimeError('live traceability contract reader unavailable')

            if mode == 'aggregate':
                validate = getattr(helper, 'validate_aggregate', None)
                if not callable(validate):
                    raise ValueError('effective traceability helper lacks validate_aggregate')
                result = validate(
                    record,
                    candidate,
                    evidence,
                    authority_reader=live_authority_reader,
                    contract_reader=live_contract_reader,
                    source_commit=effective_tool_commit,
                    effective_tool_commit=effective_tool_commit,
                    record_source_commit=record_source_commit,
                )
            else:
                validate = getattr(helper, 'validate_leaf', None)
                if not callable(validate):
                    raise ValueError('effective traceability helper lacks validate_leaf')
                result = validate(
                    record,
                    effective_binding,
                    authority_reader=live_authority_reader,
                    contract_reader=live_contract_reader,
                    source_commit=effective_tool_commit,
                    effective_tool_commit=effective_tool_commit,
                    record_source_commit=record_source_commit,
                )
            if not isinstance(result, dict):
                raise ValueError('traceability preflight returned no structured result')
            if result.get('status') != 'passed':
                blockers = result.get('blockers') or ['traceability preflight blocked']
                raise ValueError('; '.join(str(item) for item in blockers))
    except (OSError, ValueError, KeyError, TypeError, RuntimeError, ImportError, subprocess.CalledProcessError) as exc:
        failure = str(exc)
    finally:
        if worktree_added and tool_root is not None:
            try:
                cleanup = subprocess.run(
                    ['git', '-C', str(root), 'worktree', 'remove', str(tool_root)],
                    text=True,
                    capture_output=True,
                )
                cleanup_failure = None
                if cleanup.returncode:
                    detail = (cleanup.stderr or cleanup.stdout or '').strip()
                    cleanup_failure = 'effective traceability tool worktree cleanup failed'
                    if detail:
                        cleanup_failure += ': ' + detail
            except OSError as exc:
                cleanup_failure = 'effective traceability tool worktree cleanup failed: ' + str(exc)
            if cleanup_failure:
                failure = (failure + '; ' if failure else '') + cleanup_failure
        if temporary is not None:
            try:
                temporary.cleanup()
            except OSError as exc:
                cleanup_failure = 'effective traceability temporary directory cleanup failed: ' + str(exc)
                failure = (failure + '; ' if failure else '') + cleanup_failure
    if failure:
        raise SystemExit(failure)
    print(json.dumps({'status': 'passed', 'mode': mode, 'reader_kind': 'github_live_query'}, sort_keys=True))

run_pinned_preflight()
PY
  )" || die "traceability $TRACEABILITY_MODE preflight failed before closeout mutation"
  TRACEABILITY_RESULT_JSON="$output"
}

if [[ "$TARGET_STATUS" == "ready" ]]; then
  if [[ "$VERIFICATION_PROFILE" != "fixture_repository_state" ]]; then
    [[ -n "$CI_READY_RECEIPT" && -f "$CI_READY_RECEIPT" ]] || die "ready closeout requires --ci-ready-receipt"
  fi
  [[ -n "$REVIEW_PACKET_FILE" && -f "$REVIEW_PACKET_FILE" ]] || die "ready closeout requires --review-packet-file"
  FROZEN_HEAD="$(git -C "$ROOT_DIR" rev-parse HEAD)"
  grep -q 'Pre-PR Local Role Review: passed' "$REVIEW_PACKET_FILE" || die "review packet is not passed"
  grep -q "Source Head: $FROZEN_HEAD" "$REVIEW_PACKET_FILE" || die "review packet is not bound to current frozen HEAD"
  grep -Eq 'Slice Ledger: [^[:space:]].*slice-ledger.*\.jsonl|Review Plan: [^[:space:]].*\.json' "$REVIEW_PACKET_FILE" \
    || die "review packet lacks an authoritative review plan or machine-checkable role-return ledger; regenerate with ./scripts/pm/review-plan.py --preflight-dir <dir> and rerun record-pre-pr-review"
  REVIEW_FIELDS="$(python3 - "$REVIEW_PACKET_FILE" <<'PY'
import re,sys
t=open(sys.argv[1],encoding='utf-8').read()
def f(k):
 m=re.search(rf'^- {re.escape(k)}:\s*(.+)$',t,re.M); return m.group(1).strip() if m else ''
print('roles='+f('Review Roles'))
print('head='+f('Source Head'))
print('ledger='+f('Slice Ledger'))
print('plan='+f('Review Plan'))
print('plan_schema='+f('Review Plan Schema'))
print('evidence_digest='+f('Review Evidence Digest'))
PY
)"
  REVIEW_ROLES="$(printf '%s\n' "$REVIEW_FIELDS" | sed -n 's/^roles=//p')"
  REVIEW_HEAD="$(printf '%s\n' "$REVIEW_FIELDS" | sed -n 's/^head=//p')"
  REVIEW_LEDGER="$(printf '%s\n' "$REVIEW_FIELDS" | sed -n 's/^ledger=//p')"
  REVIEW_PLAN="$(printf '%s\n' "$REVIEW_FIELDS" | sed -n 's/^plan=//p')"
  REVIEW_PLAN_SCHEMA="$(printf '%s\n' "$REVIEW_FIELDS" | sed -n 's/^plan_schema=//p')"
  REVIEW_EVIDENCE_DIGEST="$(printf '%s\n' "$REVIEW_FIELDS" | sed -n 's/^evidence_digest=//p')"
  if [[ -n "$REVIEW_PLAN" && "$REVIEW_PLAN" != n/a* ]]; then
    PLAN_FIELDS="$(python3 - "$ROOT_DIR" "$REVIEW_PLAN" "$TASK_UID" <<'PY'
import json, sys
from pathlib import Path
root, raw, task_uid = Path(sys.argv[1]).resolve(), Path(sys.argv[2]).expanduser(), sys.argv[3]
candidate = raw if raw.is_absolute() else root / raw
try:
    path = candidate.resolve(strict=True)
except OSError as exc:
    raise SystemExit(f"review packet Review Plan cannot be resolved: {exc}")
try:
    path.relative_to(root)
except ValueError:
    raise SystemExit(f"review packet Review Plan escapes repository root: {raw}")
if not path.is_file():
    raise SystemExit(f"review packet Review Plan is not a file: {raw}")
try:
    plan = json.loads(path.read_text(encoding='utf-8'))
except (OSError, json.JSONDecodeError) as exc:
    raise SystemExit(f"review packet Review Plan cannot be read: {exc}")
if plan.get('schema') not in ('oasis7-review-plan/v1', 'oasis7-review-plan/v2') or not isinstance(plan.get('roles'), list) or not plan.get('roles'):
    raise SystemExit('review packet Review Plan is not a complete supported review plan')
if plan.get('schema') == 'oasis7-review-plan/v2':
    import importlib.util
    spec=importlib.util.spec_from_file_location('ci_ready_receipt_identity_v2', root/'scripts/pm/ci_ready_receipt_identity.py')
    if spec is None or spec.loader is None: raise SystemExit('cannot load v2 review identity helper')
    helper=importlib.util.module_from_spec(spec); spec.loader.exec_module(helper)
    if plan.get('source_review_digest') != helper.source_review_digest(plan.get('source_review_identity')): raise SystemExit('v2 source review digest mismatch')
    if plan.get('integration_ci_digest') != helper.integration_ci_digest(plan.get('integration_ci_identity')): raise SystemExit('v2 integration CI digest mismatch')
    try:
        helper.validate_source_review_epoch(plan, root=root, task_uid=task_uid)
    except (OSError, TypeError, ValueError) as exc:
        raise SystemExit(f'v2 canonical bootstrap epoch validation failed: {exc}')
preflight = plan.get('preflight') or {}
ledger = preflight.get('ledger_path') if isinstance(preflight, dict) else ''
print(','.join(str(role) for role in plan['roles']))
print(str(ledger or ''))
print(plan.get('schema'))
PY
)" || die "review packet Review Plan is invalid; regenerate with ./scripts/pm/review-plan.py --preflight-dir <dir> and rerun record-pre-pr-review"
    REVIEW_ROLES="$(printf '%s\n' "$PLAN_FIELDS" | sed -n '1p')"
    PLAN_LEDGER="$(printf '%s\n' "$PLAN_FIELDS" | sed -n '2p')"
    REVIEW_PLAN_SCHEMA="$(printf '%s\n' "$PLAN_FIELDS" | sed -n '3p')"
    if [[ "$REVIEW_LEDGER" == n/a* || -z "$REVIEW_LEDGER" ]]; then
      REVIEW_LEDGER="$PLAN_LEDGER"
    fi
  fi
  if [[ -z "$REVIEW_LEDGER" || "$REVIEW_LEDGER" == n/a* ]]; then
    die "review packet has no derived preflight ledger; regenerate with ./scripts/pm/review-plan.py --root . --task-uid $TASK_UID --head $REVIEW_HEAD --comparison-ref <comparison-ref> --comparison-oid <comparison-oid> --evidence-digest <sha256> --change-class <change-class> --preflight-dir .pm/scratch/$TASK_UID/review-preflight, then rerun record-pre-pr-review"
  fi
  if [[ "$VERIFICATION_PROFILE" != "fixture_repository_state" ]]; then
    [[ "$REVIEW_EVIDENCE_DIGEST" =~ ^[0-9a-f]{64}$ ]] || die "review packet lacks a canonical Review Evidence Digest"
  fi
  REVIEW_LEDGER_PATH="$REVIEW_LEDGER"
  reviewed_source_head="$REVIEW_HEAD"
  ci_receipt_head="$FROZEN_HEAD"
  if [[ -n "$CI_READY_RECEIPT" ]]; then ci_receipt_head="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1],encoding="utf-8")).get("head_oid", ""))' "$CI_READY_RECEIPT")"; fi
  same_head=false
  [[ "$ci_receipt_head" == "$FROZEN_HEAD" && "$reviewed_source_head" == "$FROZEN_HEAD" ]] && same_head=true
  [[ "$same_head" == true ]] || die "ci_ready_receipt, reviewed_source_head, and frozen HEAD must satisfy same_head"
  [[ "$REVIEW_LEDGER_PATH" == /* ]] || REVIEW_LEDGER_PATH="$ROOT_DIR/$REVIEW_LEDGER_PATH"
  python3 "$SCRIPT_DIR/validate-review-provenance.py" --root "$ROOT_DIR" --task-uid "$TASK_UID" --ledger "$REVIEW_LEDGER" --roles "$REVIEW_ROLES" --source-head "$REVIEW_HEAD" >/dev/null \
    || die "ready closeout role-return validation failed (roles=$REVIEW_ROLES ledger=$REVIEW_LEDGER); regenerate the immutable review plan/preflight with ./scripts/pm/review-plan.py --preflight-dir <dir>, rerun record-pre-pr-review, and retry task-closeout"
fi
if [[ "$TARGET_STATUS" == "done" ]]; then
  RECORDED_PR_NUMBER="$(python3 - "$ROOT_DIR/.pm/github-project-sync/tasks.json" "$TASK_UID" <<'PY'
import json,sys
r=(json.load(open(sys.argv[1],encoding='utf-8')).get('tasks') or {}).get(sys.argv[2]) or {}
print('' if r.get('completion_mode')=='non_pr_task' and r.get('non_pr_completion_evidence') else (r.get('pr_number') or ''))
PY
)"
  [[ -z "$RECORDED_PR_NUMBER" || -f "$PR_MERGE_RECEIPT" ]] \
    || die "PR-backed done requires an existing caller-owned --pr-receipt"
  LIVE_PR_RECEIPT=""
  if [[ -n "$RECORDED_PR_NUMBER" ]]; then
    LIVE_PR_RECEIPT="$(mktemp)"
    trap 'rm -f "$LIVE_PR_RECEIPT"' EXIT
    python3 "$SCRIPT_DIR/pr-merge-receipt.py" "$RECORDED_PR_NUMBER" --json >"$LIVE_PR_RECEIPT" \
      || die "done transition fresh recorded-PR merge query failed"
  fi
  python3 - "$ROOT_DIR/.pm/github-project-sync/tasks.json" "$TASK_UID" "$PR_MERGE_RECEIPT" "$LIVE_PR_RECEIPT" <<'PY'
import datetime as d,json,sys
mapping=json.load(open(sys.argv[1],encoding='utf-8')); r=(mapping.get('tasks') or {}).get(sys.argv[2]) or {}
if r.get('completion_mode')=='non_pr_task' and r.get('non_pr_completion_evidence'):
 raise SystemExit(0)
if not r.get('pr_url') or not r.get('pr_number'): raise SystemExit('task-closeout: done requires a recorded PR in task truth')
if not sys.argv[3]: raise SystemExit('task-closeout: done requires --pr-merge-receipt')
p=json.load(open(sys.argv[3],encoding='utf-8'))
if p.get('receipt_type')!='oasis7_pr_merge' or p.get('issuer')!='github_live_query' or p.get('evidence_mode')!='production' or p.get('state')!='MERGED': raise SystemExit('task-closeout: invalid merged PR receipt')
for key in ('repository','default_branch','pr_number','pr_url','merged_at','head_oid','base_ref','observed_at'):
 if not p.get(key): raise SystemExit(f'task-closeout: merged PR receipt is missing {key}')
if str(p.get('pr_number'))!=str(r.get('pr_number')) or p.get('pr_url')!=r.get('pr_url'): raise SystemExit('task-closeout: merged PR receipt does not match recorded PR')
for receipt_key, record_key in (('repository','repository'),('default_branch','default_branch')):
 if not r.get(record_key) or p.get(receipt_key)!=r.get(record_key): raise SystemExit(f'task-closeout: merged PR receipt {receipt_key} does not match task truth')
if p.get('base_ref')!=r.get('default_branch'): raise SystemExit('task-closeout: merged PR receipt base does not match task truth')
live=json.load(open(sys.argv[4],encoding='utf-8'))
for key in ('receipt_type','issuer','evidence_mode','repository','default_branch','pr_number','pr_url','state','merged_at','head_oid','base_ref'):
 if p.get(key)!=live.get(key): raise SystemExit(f'task-closeout: supplied merge receipt disagrees with fresh live query: {key}')
seen=d.datetime.fromisoformat(str(p.get('observed_at')).replace('Z','+00:00'))
age=(d.datetime.now(d.timezone.utc)-seen).total_seconds()
if age < -30 or age>600: raise SystemExit('task-closeout: merged PR receipt is stale')
PY
  [[ -z "$LIVE_PR_RECEIPT" ]] || rm -f "$LIVE_PR_RECEIPT"
  trap - EXIT
fi

# A caller-owned CI receipt is immutable. Every v2 closeout performs one
# complete live same-identity integration validation, even inside its observation
# window; the legacy v1 path refreshes only when its observation window expires.
REFRESHED_CI_READY_RECEIPT=""
if [[ "$TARGET_STATUS" == "ready" && -n "$CI_READY_RECEIPT" && "$VERIFICATION_PROFILE" != "fixture_repository_state" ]]; then
  CI_RECEIPT_STALE="$(python3 - "$CI_READY_RECEIPT" <<'PY'
import datetime as d,json,sys
r=json.load(open(sys.argv[1],encoding='utf-8'))
seen=d.datetime.fromisoformat(str(r.get('observed_at','')).replace('Z','+00:00'))
print('1' if not 0 <= (d.datetime.now(d.timezone.utc)-seen).total_seconds() <= 600 else '0')
PY
)" || die "ci-ready receipt observed_at is invalid"
  if [[ "$REVIEW_PLAN_SCHEMA" == "oasis7-review-plan/v2" || "$CI_RECEIPT_STALE" == "1" ]]; then
    REFRESHED_CI_READY_RECEIPT="$(mktemp)"
    trap 'rm -f "$REFRESHED_CI_READY_RECEIPT"' EXIT
    CI_IDENTITY_JSON="$(python3 - "$CI_READY_RECEIPT" <<'PY'
import json,sys
r=json.load(open(sys.argv[1],encoding='utf-8'))
print(json.dumps([r.get(k,'') for k in ('repository','task_issue_number','pr_number','check_name','check_app_id','planner_digest','integration_run_id')]))
PY
)"
    CI_REPOSITORY="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])[0])' "$CI_IDENTITY_JSON")"
    CI_TASK_ISSUE="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])[1])' "$CI_IDENTITY_JSON")"
    CI_PR_NUMBER="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])[2])' "$CI_IDENTITY_JSON")"
    CI_CHECK_NAME="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])[3])' "$CI_IDENTITY_JSON")"
    CI_CHECK_APP="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])[4])' "$CI_IDENTITY_JSON")"
    CI_PLANNER_DIGEST="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])[5])' "$CI_IDENTITY_JSON")"
    CI_INTEGRATION_RUN_ID="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])[6])' "$CI_IDENTITY_JSON")"
    CI_RECEIPT_ARGS=()
    if [[ "$REVIEW_PLAN_SCHEMA" == "oasis7-review-plan/v2" ]]; then
      [[ "$CI_INTEGRATION_RUN_ID" =~ ^[0-9]+$ ]] || die "v2 ci-ready receipt lacks the current integration request/run identity"
      CI_RECEIPT_ARGS+=(--integration-run-id "$CI_INTEGRATION_RUN_ID")
    fi
    python3 "$SCRIPT_DIR/ci-ready-receipt.py" \
      --repository "$CI_REPOSITORY" --task-uid "$TASK_UID" \
      --task-issue-number "$CI_TASK_ISSUE" --pr-number "$CI_PR_NUMBER" \
      --check-name "$CI_CHECK_NAME" --check-app-id "$CI_CHECK_APP" \
      --planner-digest "$CI_PLANNER_DIGEST" --receipt "$CI_READY_RECEIPT" \
      --refresh-same-identity "${CI_RECEIPT_ARGS[@]}" --json >"$REFRESHED_CI_READY_RECEIPT" \
      || die "stale ci-ready receipt failed same-identity refresh"
    CI_READY_RECEIPT="$REFRESHED_CI_READY_RECEIPT"
  fi
fi

if [[ "$TARGET_STATUS" == "ready" && -n "$CI_READY_RECEIPT" && "$VERIFICATION_PROFILE" != "fixture_repository_state" ]]; then
  if [[ "$REVIEW_PLAN_SCHEMA" == "oasis7-review-plan/v2" ]]; then
    python3 - "$ROOT_DIR" "$REVIEW_PLAN" "$CI_READY_RECEIPT" <<'PY' \
      || die "v2 source review cannot be reused: latest trusted integration tree or authority changed"
import importlib.util
import json
import sys
from pathlib import Path
root=Path(sys.argv[1]).resolve()
plan=json.loads(Path(sys.argv[2]).read_text(encoding='utf-8'))
receipt=json.loads(Path(sys.argv[3]).read_text(encoding='utf-8'))
spec=importlib.util.spec_from_file_location('ci_ready_receipt_identity_v2', root/'scripts/pm/ci_ready_receipt_identity.py')
if spec is None or spec.loader is None: raise SystemExit('v2 identity helper unavailable')
helper=importlib.util.module_from_spec(spec); spec.loader.exec_module(helper)
if not helper.can_reuse_source_review(plan, receipt): raise SystemExit('changed tested tree or integration authority requires full review')
PY
  else
    CI_REVIEW_EVIDENCE_DIGEST="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1],encoding="utf-8")).get("review_evidence_digest", ""))' "$CI_READY_RECEIPT")" \
      || die "cannot read ci-ready receipt review authority"
    [[ "$CI_REVIEW_EVIDENCE_DIGEST" =~ ^[0-9a-f]{64}$ ]] || die "ci-ready receipt lacks a canonical review evidence digest"
    [[ "$CI_REVIEW_EVIDENCE_DIGEST" == "$REVIEW_EVIDENCE_DIGEST" ]] \
      || die "ci-ready receipt authority does not match reviewed evidence digest"
  fi
fi

closeout_head_fingerprint() {
  git -C "$ROOT_DIR" rev-parse HEAD 2>/dev/null || printf '%s\n' "non-git-fixture"
}
sha256_file() {
  python3 - "$1" <<'PY'
import hashlib,pathlib,sys
print(hashlib.sha256(pathlib.Path(sys.argv[1]).read_bytes()).hexdigest())
PY
}
AUDIT_INPUT_HEAD="$(closeout_head_fingerprint)"
AUDIT_INPUT_MAPPING_SHA="$(sha256_file "$ROOT_DIR/.pm/github-project-sync/tasks.json")"
AUDIT_INPUT_REVIEW_SHA="$([[ -n "$REVIEW_PACKET_FILE" ]] && sha256_file "$REVIEW_PACKET_FILE" || printf none)"
AUDIT_INPUT_LEDGER_SHA="$([[ -n "${REVIEW_LEDGER_PATH:-}" ]] && sha256_file "$REVIEW_LEDGER_PATH" || printf none)"
AUDIT_INPUT_PR_RECEIPT_SHA="$([[ -n "$PR_MERGE_RECEIPT" ]] && sha256_file "$PR_MERGE_RECEIPT" || printf none)"
if [[ "$TARGET_STATUS" != "deferred" ]]; then
  CLAIM_ARGS=(--claim-type "$CLAIM_TYPE" --verification-profile "$VERIFICATION_PROFILE" --task-uid "$TASK_UID" --json)
  if [[ "$CLAIM_TYPE" == "ready_for_pr" && -n "$CI_READY_RECEIPT" ]]; then
    CLAIM_ARGS+=(--ci-ready-receipt "$CI_READY_RECEIPT")
  fi
  if [[ -n "$COMPARISON_REF" ]]; then
    CLAIM_ARGS+=(--comparison-ref "$COMPARISON_REF")
  fi
  CLAIM_READY_JSON="$("$SCRIPT_DIR/claim-ready.sh" "${CLAIM_ARGS[@]}")"
else
  CLAIM_READY_JSON="$(python3 - <<'PY'
import json
print(json.dumps({
  "claim_type": None,
  "verify_command": None,
  "verified_at": None,
  "verification_exit_code": None,
  "status": "skipped",
  "allowed_to_claim": True,
  "claim_message": "Fresh verification skipped because closeout target status is deferred."
}, sort_keys=True))
PY
)"
fi

CURRENT_HEAD="$(closeout_head_fingerprint)"
CURRENT_MAPPING_SHA="$(sha256_file "$ROOT_DIR/.pm/github-project-sync/tasks.json")"
CURRENT_REVIEW_SHA="$([[ -n "$REVIEW_PACKET_FILE" ]] && sha256_file "$REVIEW_PACKET_FILE" || printf none)"
CURRENT_LEDGER_SHA="$([[ -n "${REVIEW_LEDGER_PATH:-}" ]] && sha256_file "$REVIEW_LEDGER_PATH" || printf none)"
CURRENT_PR_RECEIPT_SHA="$([[ -n "$PR_MERGE_RECEIPT" ]] && sha256_file "$PR_MERGE_RECEIPT" || printf none)"
[[ "$CURRENT_HEAD" == "$AUDIT_INPUT_HEAD" && "$CURRENT_MAPPING_SHA" == "$AUDIT_INPUT_MAPPING_SHA" && \
   "$CURRENT_REVIEW_SHA" == "$AUDIT_INPUT_REVIEW_SHA" && "$CURRENT_LEDGER_SHA" == "$AUDIT_INPUT_LEDGER_SHA" && \
   "$CURRENT_PR_RECEIPT_SHA" == "$AUDIT_INPUT_PR_RECEIPT_SHA" ]] \
  || die "closeout inputs changed during verification; restart selected-task closeout"
# Traceability context is a separate read-only projection only for declared
# bound paths. Ordinary closeout therefore retains exactly the two lifecycle
# audits below; the transition audit remains the authority consumed by the
# remote mutation.
if [[ "$TRACEABILITY_CONTEXT_REQUIRED" == "1" ]]; then
  SELECTED_TRACEABILITY_CONTEXT_JSON="$(selected_task_audit 1)" \
    || die "selected live task audit failed before traceability context selection"
  run_traceability_preflight
fi

# Run exactly one authoritative selected live audit after claim/evidence inputs
# are proven stable, immediately before the transition that consumes it.
TASK_AUDIT_JSON="$(selected_task_audit 0)" \
  || die "selected-task audit failed at transition"
TRANSITION_AUDIT_JSON="$TASK_AUDIT_JSON"

CLOSEOUT_ARGS=(closeout-task "$ROOT_DIR" --task-uid "$TASK_UID" --role "$ROLE" \
  --to-status "$TARGET_STATUS" --claim-json "$CLAIM_READY_JSON")
[[ -z "$PR_MERGE_RECEIPT" ]] || CLOSEOUT_ARGS+=(--pr-receipt "$PR_MERGE_RECEIPT")
if ! CLOSEOUT_JSON="$(python3 "$SCRIPT_DIR/github-project-task.py" "${CLOSEOUT_ARGS[@]}" --json)"; then
  die "remote closeout was incomplete; run ./scripts/pm/refresh-task-cache.sh --task-uid $TASK_UID --json, verify selected-task audit, then retry task-closeout"
fi

# Independent selected-task postcondition readback. This is the second bounded
# task-scoped audit (after the pre-transition audit), never a broad Project read.
POSTCONDITION_AUDIT_JSON="$(selected_task_audit 0)" \
  || die "selected-task postcondition readback failed after closeout"
python3 - "$TASK_UID" "$TARGET_STATUS" "$CLOSEOUT_JSON" "$POSTCONDITION_AUDIT_JSON" "$VERIFICATION_PROFILE" <<'PY'
import json,sys
task_uid,target=json.loads(json.dumps(sys.argv[1])),sys.argv[2]
closeout=json.loads(sys.argv[3]); audit=json.loads(sys.argv[4])
VERIFICATION_PROFILE=sys.argv[5]
expected_phase={'ready':'pre_pr_ready','done':'task_done','deferred':'blocked'}[target]
# github-project-workflow audit is itself the selected live task/Project
# postcondition authority. Its successful exit is required above; older fixture
# adapters return only {"status":"ok"}, while production returns richer detail.
if audit.get('status') == 'ok' and set(audit) == {'status'}:
 if VERIFICATION_PROFILE != 'fixture_repository_state':
  raise SystemExit('task-closeout: live selected-task postcondition audit lacks structured task readback')
else:
 readback=audit.get('selected_task') if isinstance(audit.get('selected_task'),dict) else audit
 if (readback.get('task_uid') != task_uid or readback.get('target') != target
     or readback.get('workflow_phase') != expected_phase):
  raise SystemExit('task-closeout: selected-task postcondition audit lacks expected status/phase')
PY

RESULT_JSON="$(python3 - "$ROLE" "$TARGET_STATUS" "$CLAIM_READY_JSON" "$TASK_AUDIT_JSON" "$CLOSEOUT_JSON" "$POSTCONDITION_AUDIT_JSON" "$TRACEABILITY_RESULT_JSON" <<'PY'
import json
import sys

role = sys.argv[1]
target_status = sys.argv[2]
claim = json.loads(sys.argv[3])
audit = json.loads(sys.argv[4])
closeout = json.loads(sys.argv[5])
postcondition = json.loads(sys.argv[6])
traceability = json.loads(sys.argv[7])
payload = {
    "task_uid": closeout["task_uid"],
    "role": role,
    "target_status": target_status,
    "final_status": closeout["status"],
    "issue_url": closeout.get("issue_url"),
    "execution_log_path": closeout.get("issue_url"),
    "claim_verification": claim,
    "task_audit": audit,
    "workflow_close": closeout,
    "postcondition_readback": postcondition,
    "traceability_preflight": traceability,
    "move_task": closeout,
    "pm_lint": {"status": "skipped", "ran": False, "reason": "repo-local .pm/tasks retired"},
    "recommended_next_command": "./scripts/prepare-task-pr.sh",
}
print(json.dumps(payload, indent=2, sort_keys=True))
PY
)"

if [[ "$OUTPUT_JSON" == "1" ]]; then
  printf '%s\n' "$RESULT_JSON"
  exit 0
fi

python3 - "$RESULT_JSON" <<'PY'
import json
import sys

payload = json.loads(sys.argv[1])
print("task closeout summary")
print(f"- task_uid: {payload['task_uid']}")
print(f"- role: {payload['role']}")
print(f"- final_status: {payload['final_status']}")
print(f"- issue_url: {payload['issue_url']}")
print(f"- claim_verification_status: {payload['claim_verification']['status']}")
print(f"- pm_lint: {payload['pm_lint']['status']}")
print(f"- next_step: {payload['recommended_next_command']}")
PY
