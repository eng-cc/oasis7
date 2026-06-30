#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
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
  --review-package <text>      Review Package value (default: n/a; small docs/workflow diff).
  --slice-ledger <text>        Slice Ledger value (default: n/a; small docs/workflow diff).
  --visual-evidence <text>     Visual Evidence value.
  --wasm-evidence <text>       WASM Evidence value.
  --ops-evidence <text>        Ops Evidence value.
  --liveops-evidence <text>    LiveOps Evidence value.
  --comparison-ref <ref>       Comparison Ref value (default: refs/remotes/origin/main).
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
SOURCE_HEAD=""
SOURCE_BRANCH=""
PRINT_ONLY="0"
ALLOW_DIRTY="0"

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
    --comparison-ref) COMPARISON_REF="${2:-}"; shift 2 ;;
    --source-head) SOURCE_HEAD="${2:-}"; shift 2 ;;
    --source-branch) SOURCE_BRANCH="${2:-}"; shift 2 ;;
    --allow-dirty) ALLOW_DIRTY="1"; shift ;;
    --print-only) PRINT_ONLY="1"; shift ;;
    -h|--help) usage; exit 0 ;;
    *) die "unknown argument: $1" ;;
  esac
done

[[ -n "$TASK_UID" ]] || die "--task-uid is required"
[[ -n "$ROLES" ]] || die "--roles is required"
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
if [[ -z "$REVIEWED_PATHS" ]]; then
  REVIEWED_PATHS="$(git diff --name-only "$COMPARISON_REF"...HEAD | paste -sd ';' -)"
  REVIEWED_PATHS="${REVIEWED_PATHS:-n/a; no changed paths}"
fi
if [[ -z "$ROLE_BASIS" ]]; then
  ROLE_BASIS="changed paths, task history, verification claim, and explicit adjacent-role skips"
fi
if [[ -z "$ISSUE_NUMBER" || -z "$REPO" ]]; then
  eval "$(python3 - "$TASK_UID" <<'PY'
from __future__ import annotations

import json
import shlex
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
print(f"MAPPED_REPO={shlex.quote(repo)}")
print(f"MAPPED_ISSUE={shlex.quote(issue)}")
PY
)"
  REPO="${REPO:-$MAPPED_REPO}"
  ISSUE_NUMBER="${ISSUE_NUMBER:-$MAPPED_ISSUE}"
fi

TIMESTAMP="$(date '+%Y-%m-%d %H:%M:%S %Z')"
PACKET="$(cat <<EOF
## $TIMESTAMP / tpm
- Pre-PR Local Role Review: passed
- Task UID: $TASK_UID
- Source Worktree: $PWD
- Source Branch: $SOURCE_BRANCH
- Source Head: $SOURCE_HEAD
- Comparison Ref: $COMPARISON_REF
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
