#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: ./scripts/pm/record-pre-pr-review.sh --task-uid <uid> --review-evidence <text> --review-verdicts <text> --finding-disposition-evidence <text> --verification <text> --residual-risk <text> [options]

Generate and optionally post a passed pre-PR local role review packet.

Options:
  --task-uid <uid>             Task UID.
  --issue <number>             GitHub issue number. Defaults from .pm/github-project-sync/tasks.json.
  --repo <owner/name>          GitHub repo. Defaults from project mapping or eng-cc/oasis7.
  --roles <csv>                Optional compatibility input; --review-plan roles are authoritative.
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
  --review-plan <path>         Immutable review plan; derives task/head/ref/OID/roles and preflight ledger.
  --finding-resolution <path>  Admin-authorized exact-head finding resolution manifest.
  --review-resolution <path>   Alias for --finding-resolution.
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
    local root_real value_real
    root_real="$(cd "$ROOT_DIR" && pwd -P)"
    value_real="$(python3 - "$value" <<'PY'
from pathlib import Path
import sys
print(Path(sys.argv[1]).expanduser().resolve(strict=False))
PY
)"
    case "$value_real" in
      "$root_real"/*)
        printf '%s\n' "${value_real#"$root_real"/}"
        ;;
      *)
        die "$label must not expose a local absolute path in GitHub issue evidence; use a repo-relative path or n/a"
        ;;
    esac
    return
  fi

  case "$value" in
    ./*) value="${value#./}" ;;
  esac
  printf '%s\n' "$value"
}

resolve_repo_owned_path() {
  local label="$1"
  local raw="$2"
  python3 - "$ROOT_DIR" "$raw" "$label" <<'PY'
from pathlib import Path
import sys

root = Path(sys.argv[1]).resolve()
raw = Path(sys.argv[2]).expanduser()
candidate = raw if raw.is_absolute() else root / raw
try:
    resolved = candidate.resolve(strict=True)
except OSError as exc:
    raise SystemExit(f"error: {sys.argv[3]} cannot be resolved: {exc}")
try:
    resolved.relative_to(root)
except ValueError:
    raise SystemExit(f"error: {sys.argv[3]} escapes repository root: {sys.argv[2]}")
if not resolved.is_file():
    raise SystemExit(f"error: {sys.argv[3]} is not a file: {sys.argv[2]}")
print(resolved)
PY
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
FINDING_RESOLUTION=""
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
REVIEW_EVIDENCE_DIGEST=""
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
    --finding-resolution|--review-resolution) FINDING_RESOLUTION="${2:-}"; shift 2 ;;
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
[[ -n "$REVIEW_EVIDENCE" ]] || die "--review-evidence is required"
[[ -n "$REVIEW_VERDICTS" ]] || die "--review-verdicts is required"
[[ -n "$FINDING_DISPOSITION_EVIDENCE" ]] || die "--finding-disposition-evidence is required"
[[ -n "$VERIFICATION" ]] || die "--verification is required"
[[ -n "$RESIDUAL_RISK" ]] || die "--residual-risk is required"

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
  REVIEW_PLAN="$(resolve_repo_owned_path "Review Plan" "$REVIEW_PLAN")" || exit 1
  PLAN_FIELDS="$(python3 - "$ROOT_DIR" "$REVIEW_PLAN" "$TASK_UID" "$ROLES" "$SOURCE_HEAD" "$COMPARISON_REF" "$COMPARISON_REF_EXPLICIT" "$COMPARISON_OID" <<'PY'
from __future__ import annotations
import json, subprocess, sys
from pathlib import Path

root, plan_path, task_uid, supplied_roles, supplied_head, supplied_ref, ref_explicit, supplied_oid = sys.argv[1:]
try:
    plan = json.loads(Path(plan_path).read_text(encoding="utf-8"))
except (OSError, json.JSONDecodeError) as exc:
    raise SystemExit(f"error: cannot read review plan {plan_path}: {exc}")
required = ("task_uid", "frozen_head", "comparison_ref", "comparison_oid", "roles", "expected_slices", "epoch", "batch_path", "relevant_evidence_digest")
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
preflight = plan.get("preflight")
if (not isinstance(preflight, dict) or not isinstance(preflight.get("ledger_path"), str)
        or not preflight["ledger_path"].strip()):
    raise SystemExit("error: --review-plan has no persisted preflight ledger")
canonical_roles = ",".join(roles)
if supplied_roles and supplied_roles != canonical_roles:
    raise SystemExit(f"error: --review-plan roles mismatch: expected {canonical_roles}, actual {supplied_roles}")
if supplied_head and supplied_head != plan["frozen_head"]:
    raise SystemExit(f"error: --review-plan source head mismatch: expected {plan['frozen_head']}, actual {supplied_head}")
if ref_explicit == "1" and supplied_ref != plan["comparison_ref"]:
    raise SystemExit(f"error: --review-plan comparison ref mismatch: expected {plan['comparison_ref']}, actual {supplied_ref}")
if supplied_oid and supplied_oid != plan["comparison_oid"]:
    raise SystemExit(f"error: --review-plan comparison OID mismatch: expected {plan['comparison_oid']}, actual {supplied_oid}")
resolved = subprocess.run(["git", "-C", root, "rev-parse", "--verify", f"{plan['comparison_oid']}^{{commit}}"], text=True, capture_output=True)
if resolved.returncode or resolved.stdout.strip() != plan["comparison_oid"]:
    raise SystemExit(f"error: review-plan comparison OID is not an available commit: {plan['comparison_oid']}")
print(canonical_roles)
print(plan["frozen_head"])
print(plan["comparison_ref"])
print(plan["comparison_oid"])
print(plan["epoch"])
print(plan["relevant_evidence_digest"])
print(preflight["ledger_path"])
PY
)" || exit 1
  ROLES="$(printf '%s\n' "$PLAN_FIELDS" | sed -n '1p')"
  SOURCE_HEAD="$(printf '%s\n' "$PLAN_FIELDS" | sed -n '2p')"
  COMPARISON_REF="$(printf '%s\n' "$PLAN_FIELDS" | sed -n '3p')"
  COMPARISON_OID="$(printf '%s\n' "$PLAN_FIELDS" | sed -n '4p')"
  REVIEW_PLAN_EPOCH="$(printf '%s\n' "$PLAN_FIELDS" | sed -n '5p')"
  REVIEW_EVIDENCE_DIGEST="$(printf '%s\n' "$PLAN_FIELDS" | sed -n '6p')"
  REVIEW_PLAN_LEDGER="$(printf '%s\n' "$PLAN_FIELDS" | sed -n '7p')"
fi
if [[ -z "$REVIEW_PLAN" ]]; then
  [[ -n "$ROLES" ]] || die "--roles is required when --review-plan is not supplied"
  [[ -z "$FINDING_RESOLUTION" ]] || die "--finding-resolution requires --review-plan"
fi
CURRENT_HEAD="$(git rev-parse HEAD)"
[[ "$SOURCE_HEAD" == "$CURRENT_HEAD" ]] || die "source head must be the current frozen HEAD: expected $CURRENT_HEAD, actual $SOURCE_HEAD"
[[ -n "$COMPARISON_OID" ]] || COMPARISON_OID="$(git rev-parse --verify "${COMPARISON_REF}^{commit}")"
RESOLVED_COMPARISON_OID="$(git rev-parse --verify "${COMPARISON_OID}^{commit}")" \
  || die "comparison OID is not an available commit: $COMPARISON_OID"
[[ "$COMPARISON_OID" == "$RESOLVED_COMPARISON_OID" ]] || die "comparison OID is not canonical: $COMPARISON_OID"
if [[ -z "$REVIEWED_PATHS" ]]; then
  REVIEWED_PATHS="$(git diff --name-only "$COMPARISON_OID"...HEAD | paste -sd ';' -)"
  REVIEWED_PATHS="${REVIEWED_PATHS:-n/a; no changed paths}"
fi
if [[ -z "$ROLE_BASIS" ]]; then
  ROLE_BASIS="changed paths, task history, verification claim, and explicit adjacent-role skips"
fi
REVIEW_PLAN_DISPLAY="$(sanitize_evidence_path_field "Review Plan" "$REVIEW_PLAN")"
if [[ -n "$REVIEW_PLAN" ]]; then
  REVIEW_PLAN_LEDGER="$(resolve_repo_owned_path "Review Plan preflight ledger" "$REVIEW_PLAN_LEDGER")" || exit 1
  if [[ "$SLICE_LEDGER" == n/a* ]]; then
    SLICE_LEDGER="$REVIEW_PLAN_LEDGER"
  else
    SUPPLIED_SLICE_LEDGER="$(resolve_repo_owned_path "Slice Ledger" "$SLICE_LEDGER")" || exit 1
    [[ "$SUPPLIED_SLICE_LEDGER" == "$REVIEW_PLAN_LEDGER" ]] \
      || die "--slice-ledger must match immutable review plan preflight ledger path"
    SLICE_LEDGER="$SUPPLIED_SLICE_LEDGER"
  fi
fi
if [[ -n "$FINDING_RESOLUTION" ]]; then
  if [[ -n "$REPO" && "$REPO" != "eng-cc/oasis7" ]]; then
    die "--repo must match canonical repository eng-cc/oasis7 when --finding-resolution is used"
  fi
  FINDING_RESOLUTION="$(resolve_repo_owned_path "Finding Resolution" "$FINDING_RESOLUTION")" || exit 1
  RESOLUTION_COMMAND=(python3 "$SCRIPT_DIR/review-findings-resolution.py" validate
    --root "$ROOT_DIR" --task-uid "$TASK_UID" --head "$SOURCE_HEAD"
    --ledger "$SLICE_LEDGER" --manifest "$FINDING_RESOLUTION")
  if [[ -n "$ISSUE_NUMBER" ]]; then
    RESOLUTION_COMMAND+=(--issue-number "$ISSUE_NUMBER")
  elif [[ -f "$ROOT_DIR/.pm/github-project-sync/tasks.json" ]]; then
    MAPPED_RESOLUTION_ISSUE="$(python3 - "$ROOT_DIR/.pm/github-project-sync/tasks.json" "$TASK_UID" <<'PY'
import json, sys
try:
    payload = json.loads(open(sys.argv[1], encoding="utf-8").read())
    issue = (payload.get("tasks") or {}).get(sys.argv[2], {}).get("issue_number")
except (OSError, TypeError, ValueError, json.JSONDecodeError):
    issue = None
if issue is not None:
    print(issue)
PY
)"
    if [[ -n "$MAPPED_RESOLUTION_ISSUE" ]]; then
      RESOLUTION_COMMAND+=(--issue-number "$MAPPED_RESOLUTION_ISSUE")
    fi
  fi
  RESOLUTION_RESULT="$("${RESOLUTION_COMMAND[@]}")" \
    || die "finding-resolution validation failed"
else
  RESOLUTION_RESULT=""
fi
REVIEW_PACKAGE="$(sanitize_evidence_path_field "Review Package" "$REVIEW_PACKAGE")"
SLICE_LEDGER="$(sanitize_evidence_path_field "Slice Ledger" "$SLICE_LEDGER")"
python3 - "$ROOT_DIR" "$SLICE_LEDGER" "$ROLES" "$SOURCE_HEAD" "$REVIEW_PLAN" "$([[ -n "$RESOLUTION_RESULT" ]] && echo 1 || echo 0)" <<'PY'
from __future__ import annotations
import hashlib, json, re, sys
from pathlib import Path

root, relative, roles_csv, source_head, review_plan = Path(sys.argv[1]), sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5]
resolution_validated = sys.argv[6] == "1"
def resolve_repo_path(raw: str, base: Path | None = None) -> Path:
    candidate = Path(raw).expanduser()
    options = [candidate] if candidate.is_absolute() else [root / candidate]
    if base is not None and not candidate.is_absolute():
        options.append(base.parent / candidate)
    for option in options:
        if option.is_file():
            resolved = option.resolve()
            try:
                resolved.relative_to(root.resolve())
            except ValueError:
                raise SystemExit(f"error: review artifact escapes repository root: {raw}")
            return resolved
    return options[0].resolve()
path = resolve_repo_path(relative)
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
    artifact = resolve_repo_path(str(artifacts[0]), path)
    if not artifact.is_file() or hashlib.sha256(artifact.read_bytes()).hexdigest() != digest:
        raise SystemExit(f"error: Slice Ledger artifact digest mismatch for {role}")
    try:
        artifact_payload = json.loads(artifact.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        if str(item.get("findings") or "") != "no_findings":
            raise SystemExit(
                f"error: unresolved role findings for {role}; "
                f"artifact has no structured disposition: {exc}"
            )
        if review_plan:
            raise SystemExit(
                f"error: plan-backed no_findings return for {role} must use a structured JSON artifact: {exc}"
            )
        # Human-operated legacy returns may bind opaque, digest-checked
        # evidence. They can preserve the no-findings path, but cannot assert
        # a resolved finding without a structured artifact disposition.
        seen[role] = item
        continue
    ledger_disposition = str(item.get("findings") or "")
    if not review_plan and ledger_disposition == "no_findings" and not (
        isinstance(artifact_payload, dict)
        and artifact_payload.get("schema") == "oasis7-review-return/v1"
    ):
        seen[role] = item
        continue
    if not isinstance(artifact_payload, dict):
        raise SystemExit(f"error: review artifact is not an object for {role}")
    artifact_identity = {
        "task_uid": str(item.get("task_uid") or ""),
        "role": role,
        "status": str(item.get("status") or ""),
        "head": source_head,
        "slice_id": str(item.get("slice_id") or ""),
    }
    for field, expected in artifact_identity.items():
        if artifact_payload.get(field) != expected:
            raise SystemExit(f"error: review artifact {field} mismatch for {role}")
    if item.get("epoch") and artifact_payload.get("epoch") != item["epoch"]:
        raise SystemExit(f"error: review artifact epoch mismatch for {role}")
    if review_plan and artifact_payload.get("epoch") != plan_epoch:
        raise SystemExit(f"error: review artifact epoch mismatch for {role}")
    artifact_disposition = artifact_payload.get("disposition")
    artifact_findings = artifact_payload.get("findings")
    if artifact_disposition not in {"findings", "no_findings"}:
        raise SystemExit(f"error: review artifact disposition is invalid for {role}")
    if not isinstance(artifact_findings, list):
        raise SystemExit(f"error: review artifact findings are invalid for {role}")
    if artifact_disposition == "findings" and not artifact_findings:
        raise SystemExit(f"error: review artifact findings are invalid for {role}")
    if artifact_disposition == "no_findings" and artifact_findings:
        raise SystemExit(f"error: no_findings artifact contains findings for {role}")
    if ledger_disposition != artifact_disposition:
        raise SystemExit(
            f"error: Slice Ledger/artifact disposition mismatch for {role}: "
            f"ledger={ledger_disposition}, artifact={artifact_disposition}"
        )
    if artifact_disposition == "findings" and not resolution_validated:
        raise SystemExit(
            f"error: unresolved role findings for {role}; "
            "no repository-owned resolution authority is present"
        )
    seen[role] = item
missing_roles = sorted(required - set(seen))
if missing_roles:
    raise SystemExit("error: Slice Ledger missing required role return: " + ",".join(missing_roles))
PY
python3 "$SCRIPT_DIR/validate-review-provenance.py" \
  --root "$ROOT_DIR" --task-uid "$TASK_UID" --ledger "$SLICE_LEDGER" --roles "$ROLES" --source-head "$SOURCE_HEAD" >/dev/null \
  || die "Slice Ledger role-return validation failed"
if [[ -n "$RESOLUTION_RESULT" ]]; then
  FINDING_DISPOSITION="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])["aggregate"])' "$RESOLUTION_RESULT")"
  FINDING_DISPOSITION_EVIDENCE="admin-authorized exact-head/finding resolution read back by $(python3 -c 'import json,sys; print(json.loads(sys.argv[1])["resolver"])' "$RESOLUTION_RESULT")"
else
  FINDING_DISPOSITION="no_findings"
fi
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
- Review Plan: ${REVIEW_PLAN_DISPLAY:-n/a; no immutable plan supplied}
- Review Evidence Digest: $REVIEW_EVIDENCE_DIGEST
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
