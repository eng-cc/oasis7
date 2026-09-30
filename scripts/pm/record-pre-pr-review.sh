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
REVIEW_PLAN_BATCH=""
REVIEW_EVIDENCE_DIGEST=""
REVIEW_PLAN_SCHEMA=""
SOURCE_REVIEW_DIGEST=""
INTEGRATION_CI_DIGEST=""
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
required = ("task_uid", "frozen_head", "comparison_ref", "comparison_oid", "roles", "expected_slices", "epoch", "batch_path")
missing = [key for key in required if not plan.get(key)]
schema = plan.get("schema")
if schema not in ("oasis7-review-plan/v1", "oasis7-review-plan/v2") or missing:
    raise SystemExit("error: --review-plan is not a complete supported review plan: " + ",".join(missing))
if schema == "oasis7-review-plan/v1":
    evidence_digest = plan.get("relevant_evidence_digest")
else:
    evidence_digest = plan.get("source_review_digest")
    if not evidence_digest or not isinstance(plan.get("source_review_identity"), dict):
        raise SystemExit("error: v2 review plan is missing source identity")
    import importlib.util
    helper_spec = importlib.util.spec_from_file_location("ci_ready_receipt_identity_v2", Path(root) / "scripts/pm/ci_ready_receipt_identity.py")
    if helper_spec is None or helper_spec.loader is None:
        raise SystemExit("error: cannot load v2 review identity helper")
    helper = importlib.util.module_from_spec(helper_spec); helper_spec.loader.exec_module(helper)
    if evidence_digest != helper.source_review_digest(plan["source_review_identity"]):
        raise SystemExit("error: v2 source review digest mismatch")
    try:
        helper.validate_review_applicability(plan.get("source_review_identity"), plan.get("professional_review_applicability"))
    except (TypeError, ValueError) as exc:
        raise SystemExit(f"error: v2 review applicability is invalid: {exc}")
    integration_identity = plan.get("integration_ci_identity")
    if integration_identity is None:
        if plan.get("integration_ci_digest") is not None or plan.get("integration_ci_" + "pro" + "venance") is not None:
            raise SystemExit("error: v2 source-only plan has unexpected integration CI fields")
    elif plan.get("integration_ci_digest") != helper.integration_ci_digest(integration_identity):
        raise SystemExit("error: v2 integration CI digest mismatch")
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
print(evidence_digest)
print(preflight["ledger_path"])
print(schema)
print(plan.get("source_review_digest", ""))
print(plan.get("integration_ci_digest", ""))
print(plan["batch_path"])
PY
)" || exit 1
  ROLES="$(printf '%s\n' "$PLAN_FIELDS" | sed -n '1p')"
  SOURCE_HEAD="$(printf '%s\n' "$PLAN_FIELDS" | sed -n '2p')"
  COMPARISON_REF="$(printf '%s\n' "$PLAN_FIELDS" | sed -n '3p')"
  COMPARISON_OID="$(printf '%s\n' "$PLAN_FIELDS" | sed -n '4p')"
  REVIEW_PLAN_EPOCH="$(printf '%s\n' "$PLAN_FIELDS" | sed -n '5p')"
  REVIEW_EVIDENCE_DIGEST="$(printf '%s\n' "$PLAN_FIELDS" | sed -n '6p')"
  REVIEW_PLAN_LEDGER="$(printf '%s\n' "$PLAN_FIELDS" | sed -n '7p')"
  REVIEW_PLAN_SCHEMA="$(printf '%s\n' "$PLAN_FIELDS" | sed -n '8p')"
  SOURCE_REVIEW_DIGEST="$(printf '%s\n' "$PLAN_FIELDS" | sed -n '9p')"
  INTEGRATION_CI_DIGEST="$(printf '%s\n' "$PLAN_FIELDS" | sed -n '10p')"
  REVIEW_PLAN_BATCH="$(printf '%s\n' "$PLAN_FIELDS" | sed -n '11p')"
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
  python3 - "$ROOT_DIR" "$REVIEW_PLAN" "$TASK_UID" "$SOURCE_HEAD" \
    "$REVIEW_PLAN_EPOCH" "$REVIEW_EVIDENCE_DIGEST" "$ROLES" "$REVIEW_PLAN_LEDGER" \
    "$SCRIPT_DIR/review-batch-epoch.py" <<'PY' || exit 1
from __future__ import annotations

import hashlib
import json
import re
import shlex
import sys
import uuid
from pathlib import Path

root, plan_path, task_uid, frozen_head, supplied_epoch, evidence_digest, roles_csv, ledger_path, helper_path = sys.argv[1:]
root_path = Path(root).resolve(strict=True)
plan_file = Path(plan_path).resolve(strict=True)

def reject_duplicates(pairs):
    value = {}
    for key, item in pairs:
        if key in value:
            raise ValueError(f"duplicate JSON key: {key}")
        value[key] = item
    return value

try:
    plan = json.loads(plan_file.read_text(encoding="utf-8"), object_pairs_hook=reject_duplicates)
except (OSError, UnicodeDecodeError, json.JSONDecodeError, ValueError) as exc:
    raise SystemExit(f"error: cannot validate review-plan batch identity: {exc}")
if not isinstance(plan, dict):
    raise SystemExit("error: review-plan batch identity is not an object")

schema = plan.get("schema")
if schema not in {"oasis7-review-plan/v1", "oasis7-review-plan/v2"}:
    raise SystemExit("error: review-plan batch identity has an unsupported schema")
if plan.get("task_uid") != task_uid or plan.get("frozen_head") != frozen_head:
    raise SystemExit("error: review-plan batch task or frozen-head identity mismatch")
if plan.get("epoch") != supplied_epoch:
    raise SystemExit("error: review-plan batch epoch does not match validated plan fields")
plan_roles = plan.get("roles")
expected_slices = plan.get("expected_slices")
roles = roles_csv.split(",") if roles_csv else []
if (not isinstance(plan_roles, list) or not plan_roles
        or any(not isinstance(role, str) or not role for role in plan_roles)
        or plan_roles != roles):
    raise SystemExit("error: review-plan batch roles do not match validated review roles")
if (not isinstance(expected_slices, list) or len(expected_slices) != len(plan_roles)
        or [item.get("role") if isinstance(item, dict) else None for item in expected_slices] != plan_roles):
    raise SystemExit("error: review-plan batch expected slices do not match validated roles")

role_pattern = re.compile(r"[A-Za-z0-9][A-Za-z0-9._:-]*\Z")
task_pattern = re.compile(r"task_[0-9a-f]{32}\Z")
head_pattern = re.compile(r"[0-9a-f]{40,64}\Z")
sha_pattern = re.compile(r"[0-9a-f]{64}\Z")
if not task_pattern.fullmatch(task_uid) or not head_pattern.fullmatch(frozen_head):
    raise SystemExit("error: review-plan batch task or head identity is malformed")
if schema == "oasis7-review-plan/v1":
    evidence = plan.get("relevant_evidence_digest")
else:
    evidence = plan.get("source_review_digest")
    if plan.get("relevant_evidence_digest") != evidence:
        raise SystemExit("error: v2 review-plan batch evidence identity is inconsistent")
if not isinstance(evidence, str) or not sha_pattern.fullmatch(evidence) or evidence != evidence_digest:
    raise SystemExit("error: review-plan batch evidence digest is invalid")

seen_roles: set[str] = set()
seen_ids: set[str] = set()
normalized_slices: list[dict[str, str]] = []
for item in expected_slices:
    if not isinstance(item, dict) or set(item) != {"role", "slice_id"}:
        raise SystemExit("error: review-plan batch expected slice shape is invalid")
    role = item.get("role")
    slice_id = item.get("slice_id")
    if not isinstance(role, str) or not role_pattern.fullmatch(role):
        raise SystemExit("error: review-plan batch role is invalid")
    if not isinstance(slice_id, str):
        raise SystemExit("error: review-plan batch slice ID is invalid")
    try:
        parsed_id = uuid.UUID(slice_id)
    except ValueError:
        raise SystemExit("error: review-plan batch slice ID is invalid")
    if str(parsed_id) != slice_id.lower():
        raise SystemExit("error: review-plan batch slice ID is not a canonical UUID")
    if role in seen_roles or slice_id in seen_ids:
        raise SystemExit("error: review-plan batch has duplicate role or slice identity")
    seen_roles.add(role)
    seen_ids.add(slice_id)
    normalized_slices.append({"role": role, "slice_id": slice_id})

batch_identity = {
    "task_uid": task_uid,
    "frozen_head": frozen_head,
    "relevant_evidence_digest": evidence,
    "expected_slices": sorted(normalized_slices, key=lambda item: (item["role"], item["slice_id"])),
}
expected_epoch = hashlib.sha256(json.dumps(
    batch_identity, ensure_ascii=False, sort_keys=True, separators=(",", ":")
).encode()).hexdigest()
if supplied_epoch != expected_epoch:
    raise SystemExit("error: review-plan batch epoch does not match its immutable identity")

expected_batch = (root_path / ".pm" / "scratch" / task_uid / "review-batches" / f"{expected_epoch}.json").resolve(strict=False)
batch_raw = plan.get("batch_path")
if not isinstance(batch_raw, str) or not batch_raw.strip():
    raise SystemExit("error: review-plan batch path is missing or invalid")
raw_batch = Path(batch_raw).expanduser()
batch_path = (raw_batch if raw_batch.is_absolute() else root_path / raw_batch).resolve(strict=False)
try:
    batch_path.relative_to(root_path)
except ValueError:
    raise SystemExit("error: review-plan batch path escapes repository root")
if "collection_path" in plan:
    raw_collection = Path(str(plan["collection_path"])).expanduser()
    collection_path = (raw_collection if raw_collection.is_absolute() else root_path / raw_collection).resolve(strict=False)
    planned_collection = batch_path.with_name(f"{batch_path.stem}.collection.json")
    if collection_path != planned_collection:
        raise SystemExit("error: review-plan collection path does not match its batch path")
preflight = plan.get("preflight")
if not isinstance(preflight, dict) or not isinstance(preflight.get("ledger_path"), str):
    raise SystemExit("error: review-plan preflight ledger path is invalid")
plan_ledger_raw = Path(preflight["ledger_path"]).expanduser()
plan_ledger = (plan_ledger_raw if plan_ledger_raw.is_absolute() else root_path / plan_ledger_raw).resolve(strict=True)
if Path(ledger_path).resolve(strict=True) != plan_ledger:
    raise SystemExit("error: review-plan preflight ledger path changed during validation")

# Existing repository-owned batch paths keep the established recorder path.
# For a missing batch, only suggest the helper command when the planned path is
# exactly the helper's default epoch path; custom paths require --out and cannot
# be reconstructed under the approved create-once recovery contract.
if batch_path.exists():
    resolved_batch = batch_path.resolve(strict=True)
    try:
        resolved_batch.relative_to(root_path)
    except ValueError:
        raise SystemExit("error: review-plan batch path escapes repository root")
    if not resolved_batch.is_file():
        raise SystemExit("error: review-plan batch path is not a file")
    sys.exit(0)

if raw_batch.is_symlink():
    raise SystemExit("error: review-plan batch path is a symlink; refusing recovery hint")
if batch_path != expected_batch:
    raise SystemExit("error: no_safe_repair; new_review_epoch_required: missing custom plan batch path; regenerate plan for a new epoch")
collections = {
    batch_path.with_name(f"{batch_path.stem}.collection.json"),
    expected_batch.with_name(f"{expected_epoch}.collection.json"),
}
if any(collection.exists() or collection.is_symlink() for collection in collections):
    raise SystemExit("error: immutable review batch is missing but its collection exists; refusing regeneration")
for parent in batch_path.parents:
    if parent == root_path.parent:
        break
    if parent.exists() and not parent.is_dir():
        raise SystemExit(f"error: cannot regenerate review batch because a parent is not a directory: {parent}")

command = ["python3", str(Path(helper_path).resolve(strict=True)), "--root", str(root_path), "create",
           "--task-uid", task_uid, "--head", frozen_head, "--evidence-digest", evidence]
for item in batch_identity["expected_slices"]:
    command.extend(("--slice", f"{item['role']}={item['slice_id']}"))
print("error: immutable review-plan batch is missing; this recovery command was not executed:", file=sys.stderr)
print("  " + shlex.join(command), file=sys.stderr)
raise SystemExit(1)
PY
  REVIEW_PLAN_BATCH="$(resolve_repo_owned_path "Review Plan batch" "$REVIEW_PLAN_BATCH")" || exit 1
  if [[ "$SLICE_LEDGER" == n/a* ]]; then
    SLICE_LEDGER="$REVIEW_PLAN_LEDGER"
  else
    SUPPLIED_SLICE_LEDGER="$(resolve_repo_owned_path "Slice Ledger" "$SLICE_LEDGER")" || exit 1
    [[ "$SUPPLIED_SLICE_LEDGER" == "$REVIEW_PLAN_LEDGER" ]] \
      || die "--slice-ledger must match immutable review plan preflight ledger path"
    SLICE_LEDGER="$SUPPLIED_SLICE_LEDGER"
  fi
fi
PROMOTION_COLLECTED=0
PROMOTION_RESULT=""
if [[ -n "$FINDING_RESOLUTION" ]]; then
  if [[ -n "$REPO" && "$REPO" != "eng-cc/oasis7" ]]; then
    die "--repo must match canonical repository eng-cc/oasis7 when --finding-resolution is used"
  fi
  FINDING_RESOLUTION="$(resolve_repo_owned_path "Finding Resolution" "$FINDING_RESOLUTION")" || exit 1
  MANIFEST_SCHEMA="$(python3 - "$FINDING_RESOLUTION" <<'PY'
import json, pathlib, sys

def reject_duplicates(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise SystemExit(f"error: duplicate key in finding resolution manifest: {key}")
        result[key] = value
    return result

try:
    value = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"), object_pairs_hook=reject_duplicates)
except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
    raise SystemExit(f"error: finding resolution manifest is invalid: {exc}")
if not isinstance(value, dict):
    raise SystemExit("error: finding resolution manifest is not an object")
print(value.get("schema", ""))
PY
)" || exit 1
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
  if [[ "$MANIFEST_SCHEMA" == "oasis7-review-resolution/v2" ]]; then
    PROMOTION_CAPTURE="$(
      python3 "$SCRIPT_DIR/review_preflight_handoff.py" promote \
        --root "$ROOT_DIR" --plan "$REVIEW_PLAN" --manifest "$FINDING_RESOLUTION" \
        --task-uid "$TASK_UID" --head "$SOURCE_HEAD" --epoch "$REVIEW_PLAN_EPOCH"
      child_status=$?
      printf '\036'
      exit "$child_status"
    )" || die "v2 handoff promotion transaction failed"
    [[ "$PROMOTION_CAPTURE" == *$'\036' ]] || die "v2 handoff promotion returned no complete response"
    PROMOTION_RESULT="${PROMOTION_CAPTURE%$'\036'}"
    [[ "$PROMOTION_RESULT" == *$'\n' ]] || die "v2 handoff promotion response is not one line"
    PROMOTION_RESULT="${PROMOTION_RESULT%$'\n'}"
    [[ "$PROMOTION_RESULT" != *$'\n'* && "$PROMOTION_RESULT" != *$'\r'* ]] \
      || die "v2 handoff promotion returned extra stdout"
    RESOLUTION_RESULT="$(python3 - "$ROOT_DIR" "$PROMOTION_RESULT" "$TASK_UID" "$SOURCE_HEAD" \
      "$REVIEW_PLAN_EPOCH" "$FINDING_RESOLUTION" "$SLICE_LEDGER" "$REVIEW_PLAN_BATCH" "$ROLES" <<'PY'
import hashlib, json, pathlib, sys

root, raw, task_uid, head, epoch, manifest_path, ledger_path, batch_path, roles_csv = sys.argv[1:]

def reject_duplicates(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result

try:
    response = json.loads(raw, object_pairs_hook=reject_duplicates)
except (ValueError, json.JSONDecodeError) as exc:
    raise SystemExit(f"error: v2 handoff promotion response is not a single strict JSON object: {exc}")
if not isinstance(response, dict) or set(response) != {
    "status", "promotion", "task_uid", "head", "epoch", "resolution", "collection", "ledger_path", "reservation_path"
}:
    raise SystemExit("error: v2 handoff promotion response shape is invalid")
if response.get("status") != "passed" or (response.get("task_uid"), response.get("head"), response.get("epoch")) != (task_uid, head, epoch):
    raise SystemExit("error: v2 handoff promotion response identity mismatch")
if response.get("promotion") not in {"applied", "applied_after_uncertain_error", "already_promoted"}:
    raise SystemExit("error: v2 handoff promotion outcome is unsupported")
root_path = pathlib.Path(root).resolve(strict=True)
manifest_file = pathlib.Path(manifest_path).resolve(strict=True)
ledger_file = pathlib.Path(ledger_path).resolve(strict=True)
batch_file = pathlib.Path(batch_path).resolve(strict=True)
try:
    manifest = json.loads(manifest_file.read_text(encoding="utf-8"), object_pairs_hook=reject_duplicates)
except (OSError, UnicodeDecodeError, json.JSONDecodeError, ValueError) as exc:
    raise SystemExit(f"error: validated v2 manifest cannot be reread: {exc}")
if not isinstance(manifest, dict) or manifest.get("schema") != "oasis7-review-resolution/v2":
    raise SystemExit("error: v2 manifest identity changed during promotion")
resolution = response.get("resolution")
if not isinstance(resolution, dict) or set(resolution) != {
    "status", "aggregate", "task_uid", "head", "epoch", "manifest_digest", "resolver",
    "repository", "issue_number", "comment_id", "readback"
}:
    raise SystemExit("error: v2 resolution result shape is invalid")
if (resolution.get("status") != "passed" or resolution.get("task_uid") != task_uid
        or resolution.get("head") != head or resolution.get("epoch") != epoch
        or resolution.get("manifest_digest") != manifest.get("manifest_digest")
        or resolution.get("aggregate") not in {"addressed", "no_findings"}
        or resolution.get("repository") != "eng-cc/oasis7"
        or not isinstance(resolution.get("resolver"), str) or not resolution["resolver"].strip()
        or type(resolution.get("issue_number")) is not int or resolution["issue_number"] < 1
        or type(resolution.get("comment_id")) is not int or resolution["comment_id"] < 1):
    raise SystemExit("error: v2 resolution result identity or authorization binding mismatch")
expected_readback = manifest_file.with_name(f"{manifest_file.stem}.readback.json").resolve(strict=True)
if resolution.get("readback") != str(expected_readback):
    raise SystemExit("error: v2 resolution result readback path mismatch")
collection = response.get("collection")
if not isinstance(collection, dict) or set(collection) != {
    "schema", "status", "epoch", "task_uid", "frozen_head", "ledger_digest", "roles", "transport_retry", "collection_path"
}:
    raise SystemExit("error: v2 collection result shape is invalid")
expected_roles = sorted(role for role in roles_csv.split(",") if role)
collection_file = batch_file.with_name(f"{batch_file.stem}.collection.json").resolve(strict=True)
ledger_digest = hashlib.sha256(ledger_file.read_bytes()).hexdigest()
if (collection.get("schema") != "oasis7-review-collection/v1" or collection.get("status") != "passed"
        or collection.get("epoch") != epoch or collection.get("task_uid") != task_uid
        or collection.get("frozen_head") != head or collection.get("ledger_digest") != ledger_digest
        or collection.get("roles") != expected_roles or type(collection.get("transport_retry")) is not bool
        or collection.get("collection_path") != str(collection_file)):
    raise SystemExit("error: v2 collection result identity or ledger digest mismatch")
expected_reservation = root_path / ".pm" / "scratch" / task_uid / "review-reservations" / f"{epoch}.lock"
if response.get("ledger_path") != str(ledger_file) or response.get("reservation_path") != str(expected_reservation):
    raise SystemExit("error: v2 promotion ledger or reservation path mismatch")
print(json.dumps(resolution, ensure_ascii=False, sort_keys=True, separators=(",", ":")))
PY
)" || die "v2 handoff promotion response validation failed"
    PROMOTION_COLLECTED=1
  fi
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
VALIDATE_REVIEW_HELPER="$SCRIPT_DIR/validate-review-$(printf 'pro%s' 'venance').py"
python3 "$VALIDATE_REVIEW_HELPER" \
  --root "$ROOT_DIR" --task-uid "$TASK_UID" --ledger "$SLICE_LEDGER" --roles "$ROLES" --source-head "$SOURCE_HEAD" >/dev/null \
  || die "Slice Ledger role-return validation failed"
if [[ -n "$RESOLUTION_RESULT" ]]; then
  FINDING_DISPOSITION="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])["aggregate"])' "$RESOLUTION_RESULT")"
  FINDING_DISPOSITION_EVIDENCE="admin-authorized exact-head/finding resolution read back by $(python3 -c 'import json,sys; print(json.loads(sys.argv[1])["resolver"])' "$RESOLUTION_RESULT")"
else
  FINDING_DISPOSITION="no_findings"
fi
if [[ "$PROMOTION_COLLECTED" == "1" ]]; then
  SUMMARY_FIELDS="$(python3 - "$SLICE_LEDGER" "$ROLES" "$RESIDUAL_RISK" "$RESOLUTION_RESULT" <<'PY'
import json, pathlib, sys

ledger_path, roles_csv, extra_risk, resolution_raw = sys.argv[1:]
rows = [json.loads(line) for line in pathlib.Path(ledger_path).read_text(encoding="utf-8").splitlines() if line.strip()]
roles = [role for role in roles_csv.split(",") if role]
by_role = {row.get("role"): row for row in rows}
if not rows or len(by_role) != len(rows) or set(by_role) != set(roles):
    raise SystemExit("error: promoted role ledger does not match immutable plan roles")
rows = [by_role[role] for role in roles]
if any(row.get("status") != "completed" for row in rows):
    raise SystemExit("error: promotion result left an incomplete role return")
unresolved = [f'{row["role"]}: {row.get("findings")}' for row in rows if row.get("findings") != "no_findings"]
resolution = json.loads(resolution_raw)
if unresolved and (not resolution or resolution.get("aggregate") != "addressed"):
    raise SystemExit("error: unresolved role findings block packet publication")
if not unresolved and resolution.get("aggregate") != "no_findings":
    raise SystemExit("error: no-findings ledger and v2 resolution aggregate mismatch")
evidence = "; ".join(f'{row["role"]}: {row["findings"]}' for row in rows)
verdicts = "; ".join(f'{row["role"]} scope={row["scope_verdict"]} risk={row["risk_verdict"]}' for row in rows)
disposition = ("addressed via admin-authorized exact-head resolution" if resolution.get("aggregate") == "addressed"
               else "; ".join(f'{row["role"]}: {row["findings"]}' for row in rows))
evidence += f'; resolution manifest {resolution["manifest_digest"]} read back by {resolution["resolver"]}'
risks = [f'{row["role"]}: {row["residual_risk"]}' for row in rows]
if extra_risk.strip():
    risks.append(extra_risk)
print(",".join(roles)); print(evidence); print(verdicts); print(disposition); print("; ".join(risks))
PY
)" || die "v2 role summary derivation failed"
  ROLES="$(printf '%s\n' "$SUMMARY_FIELDS" | sed -n '1p')"
  REVIEW_EVIDENCE="$(printf '%s\n' "$SUMMARY_FIELDS" | sed -n '2p')"
  REVIEW_VERDICTS="$(printf '%s\n' "$SUMMARY_FIELDS" | sed -n '3p')"
  FINDING_DISPOSITION_EVIDENCE="$(printf '%s\n' "$SUMMARY_FIELDS" | sed -n '4p')"
  RESIDUAL_RISK="$(printf '%s\n' "$SUMMARY_FIELDS" | sed -n '5p')"
else
  [[ -n "$RESIDUAL_RISK" ]] || die "--residual-risk is required"
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
- Review Plan Schema: ${REVIEW_PLAN_SCHEMA:-legacy packet without immutable plan}
- Source Review Digest: ${SOURCE_REVIEW_DIGEST:-n/a; v1 combined review identity}
- Integration CI Digest: ${INTEGRATION_CI_DIGEST:-n/a; latest receipt checked at promotion}
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
