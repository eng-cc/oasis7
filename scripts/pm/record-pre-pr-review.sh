#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: ./scripts/pm/record-pre-pr-review.sh --task-uid <uid> --roles <csv> --review-evidence <text> --review-verdicts <text> --finding-disposition-evidence <text> --verification <text> --residual-risk <text> [options]

Generate and optionally post a passed pre-PR local role review packet.

Options:
  --task-uid <uid>             Task UID.
  --issue <number>             GitHub issue number. Defaults from .pm/github-project-sync/tasks.json.
  --repo <owner/name>          GitHub repo. Defaults from project mapping or eng-cc/oasis7.
  --roles <csv>                Review roles included in the packet.
  --role-basis <text>          Role selection basis.
  --review-evidence <text>     Per-role evidence summary.
  --review-verdicts <text>     Per-role dual verdict summary.
  --finding-disposition-evidence <text>
                               Evidence for addressed/no_findings disposition.
  --verification <text>        Verification matrix / observed evidence.
  --residual-risk <text>       Residual risk.
  --finding-disposition <text> Review Findings Disposition value (default: no_findings).
  --reviewed-paths <text>      Reviewed Changed Paths value (default: git diff --name-only origin/main...HEAD).
  --review-package <text>      Review Package value; use repo-relative/scratch-relative paths or n/a.
  --slice-ledger <text>        Slice Ledger value; use repo-relative/scratch-relative paths or n/a.
  --visual-evidence <text>     Visual Evidence value.
  --wasm-evidence <text>       WASM Evidence value.
  --ops-evidence <text>        Ops Evidence value.
  --liveops-evidence <text>    LiveOps Evidence value.
  --comparison-ref <ref>       Comparison Ref value (default: refs/remotes/origin/main).
  --comparison-oid <oid>       Optional assertion for the resolved comparison ref OID.
  --review-plan <path>         Immutable review plan; validates task/head/ref/OID/roles.
  --source-head <sha>          Source Head value (default: HEAD).
  --source-branch <branch>     Source Branch value (default: current branch).
  --allow-dirty                Allow dirty working tree only when --reviewed-paths
                               and --source-head are explicitly supplied.
  --print-only                 Print packet instead of posting to GitHub issue.
  -h, --help                   Show help.
USAGE
}

die() {
  echo "error: $*" >&2
  exit 1
}

sanitize_evidence_path_field() {
  local label="$1"
  local value="$2"

  if [[ "$value" == /* ]]; then
    case "$value" in
      "$ROOT_DIR"/*)
        printf '%s\n' "${value#"$ROOT_DIR"/}"
        ;;
      *)
        die "$label must not expose a local absolute path in GitHub issue evidence; use a repo-relative path or n/a"
        ;;
    esac
    return
  fi

  printf '%s\n' "$value"
}

TASK_UID=""
ISSUE_NUMBER=""
REPO=""
ROLES=""
ROLE_BASIS=""
REVIEW_EVIDENCE=""
REVIEW_VERDICTS=""
VERIFICATION=""
RESIDUAL_RISK=""
FINDING_DISPOSITION="no_findings"
FINDING_DISPOSITION_EVIDENCE=""
REVIEWED_PATHS=""
REVIEW_PACKAGE="n/a; small docs/workflow diff"
SLICE_LEDGER="n/a; small docs/workflow diff"
VISUAL_EVIDENCE="n/a; no visible/player-facing UI surface"
WASM_EVIDENCE="n/a; no WASM surface"
OPS_EVIDENCE="n/a; no deployment/operator ops surface"
LIVEOPS_EVIDENCE="n/a; no external/player/community messaging surface"
COMPARISON_REF="refs/remotes/origin/main"
COMPARISON_OID=""
REVIEW_PLAN=""
SOURCE_HEAD=""
SOURCE_BRANCH=""
PRINT_ONLY="0"
ALLOW_DIRTY="0"
COMPARISON_REF_EXPLICIT="0"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --task-uid) TASK_UID="${2:-}"; shift 2 ;;
    --issue) ISSUE_NUMBER="${2:-}"; shift 2 ;;
    --repo) REPO="${2:-}"; shift 2 ;;
    --roles) ROLES="${2:-}"; shift 2 ;;
    --role-basis) ROLE_BASIS="${2:-}"; shift 2 ;;
    --review-evidence) REVIEW_EVIDENCE="${2:-}"; shift 2 ;;
    --review-verdicts) REVIEW_VERDICTS="${2:-}"; shift 2 ;;
    --finding-disposition-evidence) FINDING_DISPOSITION_EVIDENCE="${2:-}"; shift 2 ;;
    --verification) VERIFICATION="${2:-}"; shift 2 ;;
    --residual-risk) RESIDUAL_RISK="${2:-}"; shift 2 ;;
    --finding-disposition) FINDING_DISPOSITION="${2:-}"; shift 2 ;;
    --reviewed-paths) REVIEWED_PATHS="${2:-}"; shift 2 ;;
    --review-package) REVIEW_PACKAGE="${2:-}"; shift 2 ;;
    --slice-ledger) SLICE_LEDGER="${2:-}"; shift 2 ;;
    --visual-evidence) VISUAL_EVIDENCE="${2:-}"; shift 2 ;;
    --wasm-evidence) WASM_EVIDENCE="${2:-}"; shift 2 ;;
    --ops-evidence) OPS_EVIDENCE="${2:-}"; shift 2 ;;
    --liveops-evidence) LIVEOPS_EVIDENCE="${2:-}"; shift 2 ;;
    --comparison-ref) COMPARISON_REF="${2:-}"; COMPARISON_REF_EXPLICIT="1"; shift 2 ;;
    --comparison-oid) COMPARISON_OID="${2:-}"; shift 2 ;;
    --review-plan) REVIEW_PLAN="${2:-}"; shift 2 ;;
    --source-head) SOURCE_HEAD="${2:-}"; shift 2 ;;
    --source-branch) SOURCE_BRANCH="${2:-}"; shift 2 ;;
    --allow-dirty) ALLOW_DIRTY="1"; shift ;;
    --print-only) PRINT_ONLY="1"; shift ;;
    -h|--help) usage; exit 0 ;;
    *) die "unknown argument: $1" ;;
  esac
done

[[ -n "$TASK_UID" ]] || die "--task-uid is required"
[[ -n "$ROLES" || -n "$REVIEW_PLAN" ]] || die "--roles is required unless --review-plan is supplied"
[[ -n "$REVIEW_EVIDENCE" ]] || die "--review-evidence is required"
[[ -n "$REVIEW_VERDICTS" ]] || die "--review-verdicts is required"
[[ -n "$FINDING_DISPOSITION_EVIDENCE" ]] || die "--finding-disposition-evidence is required"
[[ -n "$VERIFICATION" ]] || die "--verification is required"
[[ -n "$RESIDUAL_RISK" ]] || die "--residual-risk is required"
[[ "$SLICE_LEDGER" != n/a* ]] || die "--slice-ledger must name a machine-checkable role-return JSONL file for a passed review"

if [[ -n "$(git status --porcelain)" ]]; then
  if [[ "$ALLOW_DIRTY" != "1" || -z "$REVIEWED_PATHS" || -z "$SOURCE_HEAD" ]]; then
    die "working tree is dirty; commit/stash changes before generating a passed packet, or pass --allow-dirty with explicit --reviewed-paths and --source-head"
  fi
fi

if [[ -z "$SOURCE_HEAD" ]]; then
  SOURCE_HEAD="$(git rev-parse HEAD)"
fi
if [[ -z "$SOURCE_BRANCH" ]]; then
  SOURCE_BRANCH="$(git rev-parse --abbrev-ref HEAD)"
fi
if [[ -n "$REVIEW_PLAN" ]]; then
  PLAN_FIELDS="$(python3 - "$ROOT_DIR" "$REVIEW_PLAN" "$TASK_UID" "$ROLES" "$SOURCE_HEAD" "$COMPARISON_REF" "$COMPARISON_REF_EXPLICIT" "$COMPARISON_OID" <<'PY'
from __future__ import annotations
import json, subprocess, sys
from pathlib import Path

root, plan_path, task_uid, supplied_roles, supplied_head, supplied_ref, ref_explicit, supplied_oid = sys.argv[1:]
try:
    plan = json.loads(Path(plan_path).read_text(encoding="utf-8"))
except (OSError, json.JSONDecodeError) as exc:
    raise SystemExit(f"error: cannot read review plan {plan_path}: {exc}")
required = ("task_uid", "frozen_head", "comparison_ref", "comparison_oid", "roles", "expected_slices", "epoch", "batch_path")
missing = [key for key in required if not plan.get(key)]
if plan.get("schema") != "oasis7-review-plan/v1" or missing:
    raise SystemExit("error: --review-plan is not a complete oasis7-review-plan/v1: " + ",".join(missing))
if plan["task_uid"] != task_uid:
    raise SystemExit(f"error: --review-plan task UID mismatch: expected {task_uid}, actual {plan['task_uid']}")
roles = plan["roles"]
if not isinstance(roles, list) or not roles or any(not isinstance(role, str) or not role for role in roles):
    raise SystemExit("error: --review-plan roles are invalid")
expected_slices = plan["expected_slices"]
if (not isinstance(expected_slices, list) or len(expected_slices) != len(roles)
        or [item.get("role") if isinstance(item, dict) else None for item in expected_slices] != roles
        or any(not isinstance(item, dict) or not isinstance(item.get("slice_id"), str) or not item["slice_id"] for item in expected_slices)):
    raise SystemExit("error: --review-plan expected slices are invalid")
canonical_roles = ",".join(roles)
if supplied_roles and supplied_roles != canonical_roles:
    raise SystemExit(f"error: --review-plan roles mismatch: expected {canonical_roles}, actual {supplied_roles}")
if supplied_head and supplied_head != plan["frozen_head"]:
    raise SystemExit(f"error: --review-plan source head mismatch: expected {plan['frozen_head']}, actual {supplied_head}")
if ref_explicit == "1" and supplied_ref != plan["comparison_ref"]:
    raise SystemExit(f"error: --review-plan comparison ref mismatch: expected {plan['comparison_ref']}, actual {supplied_ref}")
if supplied_oid and supplied_oid != plan["comparison_oid"]:
    raise SystemExit(f"error: --review-plan comparison OID mismatch: expected {plan['comparison_oid']}, actual {supplied_oid}")
resolved = subprocess.run(["git", "-C", root, "rev-parse", "--verify", f"{plan['comparison_ref']}^{{commit}}"], text=True, capture_output=True)
if resolved.returncode:
    raise SystemExit(f"error: cannot resolve review-plan comparison ref {plan['comparison_ref']}: {resolved.stderr.strip()}")
actual_oid = resolved.stdout.strip()
if actual_oid != plan["comparison_oid"]:
    raise SystemExit(f"error: review-plan comparison ref drift: expected {plan['comparison_oid']}, actual {actual_oid}")
print(canonical_roles)
print(plan["frozen_head"])
print(plan["comparison_ref"])
print(plan["comparison_oid"])
print(plan["epoch"])
PY
)" || exit 1
  ROLES="$(printf '%s\n' "$PLAN_FIELDS" | sed -n '1p')"
  SOURCE_HEAD="$(printf '%s\n' "$PLAN_FIELDS" | sed -n '2p')"
  COMPARISON_REF="$(printf '%s\n' "$PLAN_FIELDS" | sed -n '3p')"
  COMPARISON_OID="$(printf '%s\n' "$PLAN_FIELDS" | sed -n '4p')"
  REVIEW_PLAN_EPOCH="$(printf '%s\n' "$PLAN_FIELDS" | sed -n '5p')"
fi
CURRENT_HEAD="$(git rev-parse HEAD)"
[[ "$SOURCE_HEAD" == "$CURRENT_HEAD" ]] || die "source head must be the current frozen HEAD: expected $CURRENT_HEAD, actual $SOURCE_HEAD"
RESOLVED_COMPARISON_OID="$(git rev-parse --verify "${COMPARISON_REF}^{commit}")" \
  || die "cannot resolve comparison ref: $COMPARISON_REF"
if [[ -n "$COMPARISON_OID" && "$COMPARISON_OID" != "$RESOLVED_COMPARISON_OID" ]]; then
  die "comparison OID mismatch: expected $RESOLVED_COMPARISON_OID, actual $COMPARISON_OID"
fi
COMPARISON_OID="$RESOLVED_COMPARISON_OID"
if [[ -z "$REVIEWED_PATHS" ]]; then
  REVIEWED_PATHS="$(git diff --name-only "$COMPARISON_REF"...HEAD | paste -sd ';' -)"
  REVIEWED_PATHS="${REVIEWED_PATHS:-n/a; no changed paths}"
fi
if [[ -z "$ROLE_BASIS" ]]; then
  ROLE_BASIS="changed paths, task history, verification claim, and explicit adjacent-role skips"
fi
REVIEW_PACKAGE="$(sanitize_evidence_path_field "Review Package" "$REVIEW_PACKAGE")"
SLICE_LEDGER="$(sanitize_evidence_path_field "Slice Ledger" "$SLICE_LEDGER")"
python3 - "$ROOT_DIR" "$SLICE_LEDGER" "$ROLES" "$SOURCE_HEAD" "$REVIEW_PLAN" <<'PY'
from __future__ import annotations
import hashlib, json, re, sys
from pathlib import Path

root, relative, roles_csv, source_head, review_plan = Path(sys.argv[1]), sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5]
path = root / relative
if not path.is_file():
    raise SystemExit(f"error: Slice Ledger does not exist: {relative}")
required = {item.strip() for item in roles_csv.split(",") if item.strip()}
expected_slices = {}
plan_epoch = ""
if review_plan:
    try:
        plan = json.loads(Path(review_plan).read_text(encoding="utf-8"))
        expected_slices = {str(item["role"]): str(item["slice_id"]) for item in plan["expected_slices"]}
        plan_epoch = str(plan["epoch"])
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as exc:
        raise SystemExit(f"error: cannot validate Slice Ledger against review plan: {exc}")
    if set(expected_slices) != required:
        raise SystemExit("error: review-plan expected slices do not match required roles")
seen = {}
for line_number, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
    if not raw.strip():
        continue
    try:
        item = json.loads(raw)
    except json.JSONDecodeError as exc:
        raise SystemExit(f"error: invalid Slice Ledger JSON at line {line_number}: {exc}")
    role = str(item.get("role") or "")
    if role not in required or str(item.get("status") or "") not in {"completed", "passed"}:
        continue
    if role in seen:
        raise SystemExit(f"error: duplicate completed Slice Ledger return for role: {role}")
    mandatory = ("slice_id", "activation", "context_delivery", "actual_runtime", "artifact_digest", "scope_verdict", "risk_verdict", "findings", "residual_risk")
    missing = [key for key in mandatory if not str(item.get(key) or "").strip()]
    if missing:
        raise SystemExit(f"error: incomplete Slice Ledger return for {role}: {','.join(missing)}")
    slice_id = str(item["slice_id"])
    if slice_id.lower() in {"tpm", "self", "self-authored", role}:
        raise SystemExit(f"error: self-authored Slice Ledger identity is forbidden for {role}")
    if str(item.get("head") or "") != source_head:
        raise SystemExit(f"error: Slice Ledger source head mismatch for {role}")
    if review_plan and (str(item.get("slice_id") or "") != expected_slices[role]
                        or str(item.get("epoch") or item.get("review_epoch") or "") != plan_epoch):
        raise SystemExit(f"error: Slice Ledger review-plan identity mismatch for {role}")
    digest = str(item["artifact_digest"])
    if not re.fullmatch(r"[0-9a-f]{64}", digest):
        raise SystemExit(f"error: invalid artifact SHA-256 for {role}")
    artifacts = item.get("artifacts") or []
    if not artifacts:
        raise SystemExit(f"error: Slice Ledger has no returned artifact for {role}")
    artifact = root / str(artifacts[0])
    if not artifact.is_file() or hashlib.sha256(artifact.read_bytes()).hexdigest() != digest:
        raise SystemExit(f"error: Slice Ledger artifact digest mismatch for {role}")
    seen[role] = item
missing_roles = sorted(required - set(seen))
if missing_roles:
    raise SystemExit("error: Slice Ledger missing required role return: " + ",".join(missing_roles))
PY
python3 "$SCRIPT_DIR/validate-review-provenance.py" \
  --root "$ROOT_DIR" --task-uid "$TASK_UID" --ledger "$SLICE_LEDGER" --roles "$ROLES" --source-head "$SOURCE_HEAD" >/dev/null \
  || die "Slice Ledger role-return validation failed"
if [[ -z "$ISSUE_NUMBER" || -z "$REPO" ]]; then
  eval "$(python3 - "$TASK_UID" <<'PY'
from __future__ import annotations

import json
import shlex
import subprocess
import sys
from pathlib import Path

task_uid = sys.argv[1]
mapping_path = Path(".pm/github-project-sync/tasks.json")
repo = "eng-cc/oasis7"
issue = ""
if mapping_path.is_file():
    mapping = json.loads(mapping_path.read_text(encoding="utf-8"))
    project = mapping.get("project") or {}
    repo = str(project.get("repo") or repo)
    record = (mapping.get("tasks") or {}).get(task_uid) or {}
    issue = str(record.get("issue_number") or "")
if not issue:
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
    if isinstance(hits, list) and len(hits) == 1:
        issue = str(hits[0].get("number") or "")
print(f"MAPPED_REPO={shlex.quote(repo)}")
print(f"MAPPED_ISSUE={shlex.quote(issue)}")
PY
)"
  REPO="${REPO:-$MAPPED_REPO}"
  ISSUE_NUMBER="${ISSUE_NUMBER:-$MAPPED_ISSUE}"
fi

TIMESTAMP="$(date '+%Y-%m-%d %H:%M:%S %Z')"
SOURCE_WORKTREE_LABEL="$(basename "$PWD")"
PACKET="$(cat <<EOF
## $TIMESTAMP / tpm
- Pre-PR Local Role Review: passed
- Task UID: $TASK_UID
- Source Worktree: $SOURCE_WORKTREE_LABEL
- Source Branch: $SOURCE_BRANCH
- Source Head: $SOURCE_HEAD
- Comparison Ref: $COMPARISON_REF
- Comparison OID: $COMPARISON_OID
- Reviewed Changed Paths: $REVIEWED_PATHS
- Review Package: $REVIEW_PACKAGE
- Role Selection Basis: $ROLE_BASIS
- Review Roles: $ROLES
- Review Evidence: $REVIEW_EVIDENCE
- Review Verdicts: $REVIEW_VERDICTS
- Review Findings Disposition: $FINDING_DISPOSITION
- Finding Disposition Evidence: $FINDING_DISPOSITION_EVIDENCE
- Verification Matrix: $VERIFICATION
- Visual Evidence: $VISUAL_EVIDENCE
- WASM Evidence: $WASM_EVIDENCE
- Ops Evidence: $OPS_EVIDENCE
- LiveOps Evidence: $LIVEOPS_EVIDENCE
- Residual Risk: $RESIDUAL_RISK
- Slice Ledger: $SLICE_LEDGER
EOF
)"

if [[ "$PRINT_ONLY" == "1" ]]; then
  printf '%s\n' "$PACKET"
  exit 0
fi

[[ -n "$ISSUE_NUMBER" ]] || die "could not infer issue number; pass --issue or use --print-only"
gh issue comment "$ISSUE_NUMBER" -R "$REPO" --body "$PACKET"
