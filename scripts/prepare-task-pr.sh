#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
source "$ROOT_DIR/scripts/worktree-harness-lib.sh"

usage() {
  cat <<'USAGE'
Usage: ./scripts/prepare-task-pr.sh [source-branch] [options]

Validate one task branch for GitHub PR closure, print the exact PR command, and
optionally push the branch plus open the PR through `gh`. The preflight summary
also reports a local required-gate validation recommendation, a claim-ready
helper command for fresh PR-readiness verification, local role-review evidence
status, plus planner reason summary derived from the current changed-path scope.
After PR creation, the default workflow continues into required-check/comment/
mergeability watch, failure fixes, merge, and cleanup unless the task explicitly
records that the PR exists only to run manual-trigger packaging/release CI.
Generated task PR bodies include a non-closing GitHub reference to the bound task issue
when a GitHub-backed task issue can be resolved. Explicit body files must carry
the same reference and must not auto-close the task before terminal finalization.
REVIEW_REQUIRED is reported as status but is not a blocking item by itself.
mergeStateStatus=BEHIND is advisory by itself; if GitHub can still merge the PR
cleanly, the workflow does not force a local rebase before merge.
When mergeStateStatus=BLOCKED is only missing review approval and user/task
policy explicitly allows skipping it, the normal flow may use repo admin merge
after re-checking checks, mergeability, requested changes, comments, and threads.

Default conventions:
- source branch: current branch
- base branch: main
- remote: origin
- standard path: implementation-freeze commit -> fresh verification -> local role-subagent review -> evidence-only closeout commit -> prepare-task-pr -> GitHub PR watch/fix/merge

Options:
  --base <branch>         Base branch for the PR (default: main)
  --remote <name>         Remote name for push / base comparison (default: origin)
  --create                Push branch if needed and run `gh pr create`
  --draft                 Add `--draft` when creating the PR
  --title <text>          Explicit PR title (default: use gh --fill)
  --body-file <path>      Pass an explicit PR body file to `gh pr create`
  --json                  Print machine-readable JSON summary only
  -h, --help              Show help

Examples:
  ./scripts/prepare-task-pr.sh
  ./scripts/prepare-task-pr.sh task/engineering-github-pr-landing-governance --json
  ./scripts/prepare-task-pr.sh --create --draft
USAGE
}

die() {
  echo "error: $*" >&2
  exit 1
}

infer_branch_from_head() {
  python3 - <<'PY'
from __future__ import annotations

import subprocess

branches = [
    line.strip()
    for line in subprocess.check_output(
        [
            "git",
            "for-each-ref",
            "--format=%(refname:short)",
            "--points-at",
            "HEAD",
            "refs/heads",
        ],
        text=True,
    ).splitlines()
    if line.strip()
]

if len(branches) == 1:
    print(branches[0])
PY
}

wh_require_git_worktree

BASE_BRANCH="main"
REMOTE_NAME="origin"
CREATE_PR=0
DRAFT_PR=0
OUTPUT_JSON=0
PR_TITLE=""
BODY_FILE=""
POSITIONAL=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --base)
      BASE_BRANCH="${2:-}"
      shift 2
      ;;
    --remote)
      REMOTE_NAME="${2:-}"
      shift 2
      ;;
    --create)
      CREATE_PR=1
      shift
      ;;
    --draft)
      DRAFT_PR=1
      shift
      ;;
    --title)
      PR_TITLE="${2:-}"
      shift 2
      ;;
    --body-file)
      BODY_FILE="${2:-}"
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
      POSITIONAL+=("$1")
      shift
      ;;
  esac
done

if [[ "${#POSITIONAL[@]}" -gt 1 ]]; then
  die "expected at most one optional [source-branch]"
fi

COMMON_GIT_DIR="$(cd "$(git rev-parse --git-common-dir)" && pwd -P)"
CANONICAL_REPO_ROOT="$(cd "$COMMON_GIT_DIR/.." && pwd -P)"
CURRENT_BRANCH="$(git branch --show-current)"
SOURCE_BRANCH="${POSITIONAL[0]:-}"

if [[ -z "$SOURCE_BRANCH" ]]; then
  if [[ -z "$CURRENT_BRANCH" ]]; then
    CURRENT_BRANCH="$(infer_branch_from_head)"
  fi
  [[ -n "$CURRENT_BRANCH" ]] || die "detached HEAD; pass [source-branch] explicitly"
  SOURCE_BRANCH="$CURRENT_BRANCH"
fi

[[ -n "$BASE_BRANCH" ]] || die "--base cannot be empty"
[[ -n "$REMOTE_NAME" ]] || die "--remote cannot be empty"
[[ "$SOURCE_BRANCH" != "$BASE_BRANCH" ]] || die "source and base branches must differ"

if [[ -n "$BODY_FILE" && ! -f "$BODY_FILE" ]]; then
  die "--body-file not found: $BODY_FILE"
fi

branch_checkout_path() {
  python3 - "$COMMON_GIT_DIR" "$1" <<'PY'
from __future__ import annotations

import subprocess
import sys

git_dir = sys.argv[1]
target = f"refs/heads/{sys.argv[2]}"
current: dict[str, str] = {}
raw = subprocess.check_output(
    ["git", f"--git-dir={git_dir}", "worktree", "list", "--porcelain"],
    text=True,
)

def emit(record: dict[str, str]) -> None:
    if record.get("branch") == target:
        print(record.get("worktree", ""))
        raise SystemExit(0)

for line in raw.splitlines():
    if not line:
        if current:
            emit(current)
            current = {}
        continue
    key, _, value = line.partition(" ")
    current[key] = value

if current:
    emit(current)

raise SystemExit(1)
PY
}

ensure_branch_exists() {
  git show-ref --verify --quiet "refs/heads/$1" || die "branch not found: $1"
}

ensure_clean_worktree() {
  local worktree_path=$1
  local label=$2
  if [[ -n "$(git -C "$worktree_path" status --short)" ]]; then
    die "$label worktree is dirty: $worktree_path"
  fi
}

render_cmd() {
  python3 - "$@" <<'PY'
from __future__ import annotations

import shlex
import sys

print(" ".join(shlex.quote(arg) for arg in sys.argv[1:]))
PY
}

plan_kv_get() {
  local output="$1"
  local key=""
  key="$2"
  printf '%s\n' "$output" | sed -n "s/^${key}=//p" | head -n 1
}

plan_kv_get_default() {
  local output="$1"
  local key="$2"
  local default_value="$3"
  local value=""
  value="$(plan_kv_get "$output" "$key")"
  printf '%s\n' "${value:-$default_value}"
}

append_unique_token() {
  local list="$1"
  local token="$2"
  if [[ -z "$list" ]]; then
    printf '%s' "$token"
    return 0
  fi
  case ",$list," in
    *",$token,"*) printf '%s' "$list" ;;
    *) printf '%s,%s' "$list" "$token" ;;
  esac
}

required_review_roles_from_paths() {
  local changed_paths_raw="$1"
  local roles=""
  local path=""

  while IFS= read -r path; do
    [[ -n "$path" ]] || continue
    case "$path" in
      AGENTS.md|.codex/config.toml|.codex/agents/*|doc/engineering/workflow/*|.agents/skills/*|skills/*|.agents/roles/*|.github/workflows/*|scripts/ci-tests.sh|scripts/plan-rust-required-scope.sh|scripts/plan-rust-required-scope.test.sh|scripts/prepare-task-pr.sh|scripts/pm/*|scripts/doc-governance-check.sh|scripts/lint-skills.sh)
        roles="$(append_unique_token "$roles" "repository_health_engineer")"
        ;;
    esac
    case "$path" in
      doc/engineering/workflow/*|.agents/skills/*|skills/*|doc/core/*|doc/core/**/*|doc/game/*|doc/game/**/*|doc/*prd*|doc/**/*prd*|doc/*project*|doc/**/*project*|doc/*acceptance*|doc/**/*acceptance*)
        roles="$(append_unique_token "$roles" "producer_system_designer")"
        ;;
    esac
    case "$path" in
      crates/oasis7|crates/oasis7/*|crates/oasis7/**/*|crates/oasis7_node|crates/oasis7_node/*|crates/oasis7_node/**/*|doc/world-runtime/*|doc/world-runtime/**/*)
        roles="$(append_unique_token "$roles" "runtime_engineer")"
        ;;
    esac
    case "$path" in
      crates/oasis7_wasm_*|crates/oasis7_wasm_*/*|crates/oasis7_wasm_*/**/*|crates/oasis7_builtin_wasm_modules|crates/oasis7_builtin_wasm_modules/*|crates/oasis7_builtin_wasm_modules/**/*|doc/world-runtime/wasm/*|doc/world-runtime/wasm/**/*|.github/workflows/wasm-determinism-gate.yml|scripts/plan-wasm-determinism-scope.sh)
        roles="$(append_unique_token "$roles" "wasm_platform_engineer")"
        ;;
    esac
    case "$path" in
      crates/oasis7_viewer|crates/oasis7_viewer/*|crates/oasis7_viewer/**/*|crates/oasis7/src/viewer/*|crates/oasis7/src/viewer/**/*|testing-manual.md|doc/testing/*|doc/testing/**/*|doc/world-simulator/viewer/*|doc/world-simulator/viewer/**/*|doc/world-simulator/launcher/*|doc/world-simulator/launcher/**/*|doc/*viewer*|doc/**/*viewer*|doc/*launcher*|doc/**/*launcher*|scripts/*viewer*|scripts/**/*viewer*|scripts/*launcher*|scripts/**/*launcher*)
        roles="$(append_unique_token "$roles" "viewer_engineer")"
      ;;
    esac
    case "$path" in
      crates/oasis7_viewer/*|crates/oasis7_viewer/**/*|testing-manual.md|doc/testing/*|doc/testing/**/*|doc/testing/templates/model-visual-review-card-template.md)
        roles="$(append_unique_token "$roles" "game_visual_interaction_designer")"
        ;;
    esac
    case "$path" in
      doc/game/*|doc/game/**/*|doc/world-simulator/*|doc/world-simulator/**/*|doc/playability_test_result/*|doc/playability_test_result/**/*)
        roles="$(append_unique_token "$roles" "gameplay_designer")"
        ;;
    esac
    case "$path" in
      doc/liveops/*|doc/liveops/**/*|doc/community/*|doc/community/**/*|doc/readme/*|doc/readme/**/*|doc/*incident*|doc/**/*incident*|doc/*runbook*|doc/**/*runbook*|doc/*release*|doc/**/*release*|doc/*changelog*|doc/**/*changelog*|doc/*announcement*|doc/**/*announcement*|doc/*status*|doc/**/*status*)
        roles="$(append_unique_token "$roles" "liveops_community")"
        ;;
    esac
    case "$path" in
      doc/*ops*|doc/**/*ops*|doc/*deploy*|doc/**/*deploy*|doc/*rollback*|doc/**/*rollback*|doc/*runbook*|doc/**/*runbook*|doc/*topology*|doc/**/*topology*|doc/*inventory*|doc/**/*inventory*|doc/*health*|doc/**/*health*|doc/*readiness*|doc/**/*readiness*|doc/*preflight*|doc/**/*preflight*|doc/*service*|doc/**/*service*|doc/*host*|doc/**/*host*|doc/*packaging*|doc/**/*packaging*|doc/*release*|doc/**/*release*|scripts/*deploy*|scripts/*rollback*|scripts/*preflight*|scripts/*packaging*|scripts/*release*)
        roles="$(append_unique_token "$roles" "blockchain_ops_engineer")"
        ;;
    esac
    case "$path" in
      crates/*agent*|crates/**/*agent*|doc/game/*agent*|doc/game/**/*agent*|doc/world-simulator/*agent*|doc/world-simulator/**/*agent*)
        roles="$(append_unique_token "$roles" "agent_engineer")"
        ;;
    esac
    case "$path" in
      .codex/agents/*.toml)
        local adapter_role="${path##*/}"
        adapter_role="${adapter_role%.toml}"
        case "$adapter_role" in
          producer_system_designer|gameplay_designer|game_visual_interaction_designer|runtime_engineer|blockchain_ops_engineer|wasm_platform_engineer|agent_engineer|viewer_engineer|qa_engineer|repository_health_engineer|liveops_community)
            roles="$(append_unique_token "$roles" "$adapter_role")"
            ;;
          *)
            echo "prepare-task-pr: unknown Codex specialist adapter basename: $path" >&2
            return 1
            ;;
        esac
        ;;
    esac
    case "$path" in
      .agents/roles/*.md)
        local role_card_role="${path##*/}"
        role_card_role="${role_card_role%.md}"
        case "$role_card_role" in
          producer_system_designer|gameplay_designer|game_visual_interaction_designer|runtime_engineer|blockchain_ops_engineer|wasm_platform_engineer|agent_engineer|viewer_engineer|qa_engineer|repository_health_engineer|liveops_community)
            roles="$(append_unique_token "$roles" "$role_card_role")"
            ;;
        esac
        ;;
    esac
    case "$path" in
      .codex/config.toml)
        for registry_role in \
          producer_system_designer gameplay_designer game_visual_interaction_designer \
          runtime_engineer blockchain_ops_engineer wasm_platform_engineer agent_engineer \
          viewer_engineer qa_engineer repository_health_engineer liveops_community; do
          roles="$(append_unique_token "$roles" "$registry_role")"
        done
        ;;
    esac
    case "$path" in
      AGENTS.md|.agents/roles/tpm.md|.agents/roles/templates/subagent-slice-card.md|.agents/skills/repo-owned-workflow-router/SKILL.md|.agents/skills/requesting-repo-owned-review/SKILL.md|.codex/config.toml|.codex/agents/*|.github/workflows/*|.github/workflows/**/*|scripts/prepare-task-pr.sh|scripts/pr-review-thread-closeout.sh|scripts/pm/claim-ready.sh|scripts/pm/task-closeout.sh|scripts/pm/workflow-lint.sh|scripts/pm/workflow-behavior-eval.sh|scripts/pm/validate-codex-agent-config.py|scripts/pm/*test*.sh|scripts/*test*.sh|scripts/plan-rust-required-scope.sh|scripts/ci-tests.sh|testing-manual.md|doc/testing/*|doc/testing/**/*|doc/*verification*|doc/**/*verification*|doc/*readiness*|doc/**/*readiness*|doc/engineering/workflow/*)
        roles="$(append_unique_token "$roles" "qa_engineer")"
        ;;
    esac
  done < <(printf '%s\n' "$changed_paths_raw" | tr ';' '\n')

  printf '%s' "$roles"
}

missing_required_review_roles() {
  local required_roles="$1"
  local actual_roles="$2"
  local missing=""
  local role=""
  local normalized_actual=",${actual_roles// /},"

  while IFS= read -r role; do
    [[ -n "$role" ]] || continue
    if [[ "$normalized_actual" != *",$role,"* ]]; then
      missing="$(append_unique_token "$missing" "$role")"
    fi
  done < <(printf '%s\n' "$required_roles" | tr ',' '\n')

  printf '%s' "$missing"
}

contains_role() {
  local roles="$1"
  local role="$2"
  [[ ",${roles// /}," == *",$role,"* ]]
}

text_has_any_keyword() {
  local text
  text="$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')"
  shift
  local keyword=""
  for keyword in "$@"; do
    if [[ "$text" == *"$keyword"* ]]; then
      return 0
    fi
  done
  return 1
}

text_has_explicit_exemption() {
  local text
  text="$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')"
  if [[ "$text" != *"n/a"* && "$text" != *"not applicable"* ]]; then
    return 1
  fi
  [[ "$text" == *"exemption reason"* \
    || "$text" == *"deferral reason"* \
    || "$text" == *"explicit exemption"* \
    || "$text" == *"explicit deferral"* \
    || "$text" == *"no rendered ui changed"* \
    || "$text" == *"no player-facing change"* \
    || "$text" == *"no deployment change"* \
    || "$text" == *"no release change"* \
    || "$text" == *"no public-facing change"* ]]
}

append_missing_marker() {
  local list="$1"
  local marker="$2"
  if [[ -z "$list" ]]; then
    printf '%s' "$marker"
  else
    printf '%s;%s' "$list" "$marker"
  fi
}

semantic_review_evidence_missing() {
  local required_roles="$1"
  local verification_matrix="$2"
  local visual_evidence="$3"
  local wasm_evidence="$4"
  local ops_evidence="$5"
  local liveops_evidence="$6"
  local missing=""

  if contains_role "$required_roles" "game_visual_interaction_designer"; then
    if text_has_explicit_exemption "$visual_evidence"; then
      :
    elif text_has_any_keyword "$visual_evidence" "n/a" "no visible surface" "not applicable"; then
      missing="$(append_missing_marker "$missing" "Visual Evidence must include screenshot/model-review evidence or a specific exemption for visual/UI paths")"
    elif ! text_has_any_keyword "$visual_evidence" "screenshot" "visual review" "model visual" "viewport" "s6"; then
      missing="$(append_missing_marker "$missing" "Visual Evidence missing screenshot/model-review/viewport/S6 evidence")"
    fi
  fi

  if contains_role "$required_roles" "wasm_platform_engineer"; then
    if ! text_has_any_keyword "$wasm_evidence" "support crate" "determinism" "wasm" "abi" "receipt" "hash" "n/a"; then
      missing="$(append_missing_marker "$missing" "WASM Evidence missing WASM support/determinism/ABI evidence or explicit n/a reason")"
    fi
  fi

  if contains_role "$required_roles" "blockchain_ops_engineer"; then
    if text_has_any_keyword "$ops_evidence" "n/a" "not applicable" && ! text_has_explicit_exemption "$ops_evidence"; then
      missing="$(append_missing_marker "$missing" "Ops Evidence must include readiness/rollback/runbook/operator/health evidence or an explicit exemption reason")"
    elif ! text_has_any_keyword "$ops_evidence" "readiness" "rollback" "runbook" "operator" "health" "preflight" && ! text_has_explicit_exemption "$ops_evidence"; then
      missing="$(append_missing_marker "$missing" "Ops Evidence missing readiness/rollback/runbook/operator/health evidence or explicit n/a reason")"
    fi
  fi

  if contains_role "$required_roles" "liveops_community"; then
    if text_has_any_keyword "$liveops_evidence" "n/a" "not applicable" && ! text_has_explicit_exemption "$liveops_evidence"; then
      missing="$(append_missing_marker "$missing" "LiveOps Evidence must include messaging/release-note/player/community evidence or an explicit exemption reason")"
    elif ! text_has_any_keyword "$liveops_evidence" "message" "release note" "player" "community" "announcement" "status" && ! text_has_explicit_exemption "$liveops_evidence"; then
      missing="$(append_missing_marker "$missing" "LiveOps Evidence missing messaging/release-note/player/community evidence or explicit n/a reason")"
    fi
  fi

  if contains_role "$required_roles" "runtime_engineer"; then
    if text_has_any_keyword "$verification_matrix" "n/a" "not applicable" && ! text_has_explicit_exemption "$verification_matrix"; then
      missing="$(append_missing_marker "$missing" "Verification Matrix must include runtime replay/recovery/checkpoint/long-run applicability or an explicit deferral reason")"
    elif ! text_has_any_keyword "$verification_matrix" "replay" "recovery" "checkpoint" "long-run" "longrun" "runtime" && ! text_has_explicit_exemption "$verification_matrix"; then
      missing="$(append_missing_marker "$missing" "Verification Matrix missing runtime replay/recovery/checkpoint/long-run applicability")"
    fi
  fi

  if contains_role "$required_roles" "gameplay_designer"; then
    if text_has_any_keyword "$verification_matrix" "n/a" "not applicable" && ! text_has_explicit_exemption "$verification_matrix"; then
      missing="$(append_missing_marker "$missing" "Verification Matrix must include gameplay playability/economy/motivation-loop applicability or an explicit deferral reason")"
    elif ! text_has_any_keyword "$verification_matrix" "playability" "economy" "motivation" "loop" "progression" "gameplay" && ! text_has_explicit_exemption "$verification_matrix"; then
      missing="$(append_missing_marker "$missing" "Verification Matrix missing gameplay playability/economy/motivation-loop applicability")"
    fi
  fi

  printf '%s' "$missing"
}

task_issue_number_from_review() {
  local task_uid="$1"
  local evidence_sink="$2"
  local source_worktree="$3"

  python3 - "$task_uid" "$evidence_sink" "$source_worktree" <<'PY'
from __future__ import annotations

from pathlib import Path
import json
import re
import sys

task_uid = sys.argv[1]
evidence_sink = sys.argv[2]
source_worktree = Path(sys.argv[3]).resolve()

match = re.search(r"/issues/(\d+)(?:$|[?#])", evidence_sink)
if match:
    print(match.group(1))
    raise SystemExit(0)

root = source_worktree
mapping_path = root / ".pm/github-project-sync/tasks.json"
if mapping_path.is_file():
    try:
        mapping = json.loads(mapping_path.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        mapping = {}
    record = (mapping.get("tasks") or {}).get(task_uid) or {}
    number = str(record.get("issue_number") or "").strip()
    if number.isdigit():
        print(number)
        raise SystemExit(0)

print("")
PY
}

body_file_has_task_reference() {
  local body_file="$1"
  local issue_number="$2"
  python3 - "$body_file" "$issue_number" <<'PY'
from __future__ import annotations

from pathlib import Path
import re
import sys

body = Path(sys.argv[1]).read_text(encoding="utf-8")
issue_number = re.escape(sys.argv[2])
reference = re.compile(rf"\b(refs?|references?)\s+#\s*{issue_number}\b", re.IGNORECASE)
auto_close = re.compile(rf"\b(close[sd]?|fix(e[sd])?|resolve[sd]?)\s+#\s*{issue_number}\b", re.IGNORECASE)
raise SystemExit(0 if reference.search(body) and not auto_close.search(body) else 1)
PY
}

local_role_review_status() {
  local source_worktree="$1"
  local source_branch="$2"
  local source_head="$3"
  local comparison_ref="$4"
  python3 - "$source_worktree" "$source_branch" "$source_head" "$comparison_ref" <<'PY'
from __future__ import annotations

from pathlib import Path
import json
import os
import re
import subprocess
import sys

source_worktree = Path(sys.argv[1]).resolve()
source_branch = sys.argv[2]
source_head = sys.argv[3]
comparison_ref = sys.argv[4]
root = source_worktree
tasks_dir = root / ".pm" / "tasks"

def parse_field(text: str, key: str) -> str:
    match = re.search(rf"^- {re.escape(key)}: (.+)$", text, re.MULTILINE)
    return match.group(1).strip() if match else ""

def review_packet_blocks(text: str) -> list[str]:
    lines = text.splitlines()
    blocks: list[str] = []
    current: list[str] = []
    in_block = False
    for line in lines:
        if line.startswith("## "):
            if in_block and current:
                blocks.append("\n".join(current))
            current = [line]
            in_block = False
            continue
        if current:
            current.append(line)
            if line == "- Pre-PR Local Role Review: passed":
                in_block = True
    if in_block and current:
        blocks.append("\n".join(current))
    return blocks

def emit(
    status: str,
    task_uid: str = "",
    log_path: str = "",
    reason: str = "",
    missing_markers: list[str] | None = None,
    review_roles: str = "",
    review_package: str = "",
    review_verdicts: str = "",
    findings_disposition: str = "",
    residual_risk: str = "",
    slice_ledger: str = "",
    verification_matrix: str = "",
    visual_evidence: str = "",
    wasm_evidence: str = "",
    ops_evidence: str = "",
    liveops_evidence: str = "",
    reviewed_source_head: str = "",
) -> None:
    print(f"status={status}")
    print(f"task_uid={task_uid}")
    print(f"evidence_sink={log_path}")
    print(f"execution_log_path={log_path}")
    print(f"reason={reason}")
    print(f"missing_markers={';'.join(missing_markers or [])}")
    print(f"review_roles={review_roles}")
    print(f"review_package={review_package}")
    print(f"review_verdicts={review_verdicts}")
    print(f"findings_disposition={findings_disposition}")
    print(f"residual_risk={residual_risk}")
    print(f"slice_ledger={slice_ledger}")
    print(f"verification_matrix={verification_matrix}")
    print(f"visual_evidence={visual_evidence}")
    print(f"wasm_evidence={wasm_evidence}")
    print(f"ops_evidence={ops_evidence}")
    print(f"liveops_evidence={liveops_evidence}")
    print(f"reviewed_source_head={reviewed_source_head}")
    raise SystemExit(0)

task_uid_re = re.compile(r"^task_[0-9a-f]{32}$")
candidates: list[tuple[str, Path, str]] = []
github_issue: dict[str, object] | None = None

def parse_issue_body_fields(body: str) -> dict[str, str]:
    fields: dict[str, str] = {}
    task_match = re.search(r"^task_uid:\s*(task_[0-9a-f]{32})$", body, re.MULTILINE)
    if task_match:
        fields["task_uid"] = task_match.group(1)
    for key in ("worktree_hint",):
        match = re.search(rf"^- {re.escape(key)}: `([^`]+)`$", body, re.MULTILINE)
        if match:
            fields[key] = match.group(1)
    return fields

def github_issue_for_worktree(source_worktree: Path) -> dict[str, object] | None:
    repo = "eng-cc/oasis7"
    search_terms = [str(source_worktree), source_worktree.name]
    for term in search_terms:
        try:
            search_payload = subprocess.check_output(
                [
                    "gh",
                    "issue",
                    "list",
                    "-R",
                    repo,
                    "--search",
                    f"{term} in:body",
                    "--json",
                    "number,url,title,state",
                    "--limit",
                    "10",
                ],
                text=True,
            )
            hits = json.loads(search_payload)
        except (subprocess.CalledProcessError, json.JSONDecodeError):
            continue
        if not isinstance(hits, list):
            continue
        matches: list[dict[str, object]] = []
        for hit in hits:
            if not isinstance(hit, dict) or not hit.get("number"):
                continue
            try:
                issue_payload = subprocess.check_output(
                    [
                        "gh",
                        "issue",
                        "view",
                        str(hit["number"]),
                        "-R",
                        repo,
                        "--json",
                        "body,number,title,url",
                    ],
                    text=True,
                )
                issue = json.loads(issue_payload)
            except (subprocess.CalledProcessError, json.JSONDecodeError):
                continue
            fields = parse_issue_body_fields(str(issue.get("body") or ""))
            if not worktree_hint_matches(fields.get("worktree_hint"), source_worktree):
                continue
            task_uid = fields.get("task_uid") or ""
            if not task_uid_re.fullmatch(task_uid):
                continue
            matches.append(
                {
                    "repo": repo,
                    "number": int(issue.get("number") or hit["number"]),
                    "url": str(issue.get("url") or hit.get("url") or ""),
                    "task_uid": task_uid,
                }
            )
        if len(matches) == 1:
            return matches[0]
        if len(matches) > 1:
            emit("missing", reason=f"multiple GitHub issues match worktree_hint {source_worktree}")
    return None

def worktree_hint_matches(raw_hint: object, source_worktree: Path) -> bool:
    hint = str(raw_hint or "").strip().strip('"')
    if not hint:
        return False
    if hint == str(source_worktree) or hint == source_worktree.name:
        return True
    hint_path = Path(hint).expanduser()
    candidates = {hint_path.name}
    if hint_path.is_absolute():
        try:
            candidates.add(str(hint_path.resolve()))
        except OSError:
            candidates.add(str(hint_path))
    return str(source_worktree) in candidates or source_worktree.name in candidates

def issue_comments_via_rest(repo: str, issue_number: int) -> list[dict[str, object]]:
    owner, _, name = repo.partition("/")
    if not owner or not name:
        raise subprocess.CalledProcessError(2, ["gh", "api", "invalid-repo"])
    payload = subprocess.check_output(
        [
            "gh",
            "api",
            f"repos/{owner}/{name}/issues/{issue_number}/comments",
            "--paginate",
        ],
        text=True,
    )
    if not payload.strip():
        return []
    decoder = json.JSONDecoder()
    comments: list[dict[str, object]] = []
    idx = 0
    while idx < len(payload):
        while idx < len(payload) and payload[idx].isspace():
            idx += 1
        if idx >= len(payload):
            break
        page, next_idx = decoder.raw_decode(payload, idx)
        if isinstance(page, list):
            comments.extend(comment for comment in page if isinstance(comment, dict))
        idx = next_idx
    return comments

task_files = sorted(tasks_dir.glob("task_*.yaml")) if tasks_dir.is_dir() else []
if task_files:
    if os.environ.get("PREPARE_TASK_PR_ALLOW_RETIRED_PM_TASKS") != "1":
        emit(
            "missing",
            reason=(
                "retired .pm/tasks files are present; GitHub Project mapping and "
                "issue evidence comments must be the active pre-PR evidence source"
            ),
        )
    for task_file in task_files:
        text = task_file.read_text(encoding="utf-8")
        task_uid = ""
        execution_log_path = ""
        worktree_hint = ""
        for line in text.splitlines():
            key, _, value = line.partition(":")
            value = value.strip().strip('"')
            if key == "task_uid":
                task_uid = value
            elif key == "execution_log_path":
                execution_log_path = value
            elif key == "worktree_hint":
                worktree_hint = value
        if not worktree_hint_matches(worktree_hint, source_worktree):
            continue
        if not task_uid_re.fullmatch(task_uid):
            continue
        if not execution_log_path:
            execution_log_path = f".pm/tasks/{task_uid}.execution.md"
        candidates.append((task_uid, root / execution_log_path, execution_log_path))
else:
    mapping_path = root / ".pm/github-project-sync/tasks.json"
    if mapping_path.is_file():
        mapping = json.loads(mapping_path.read_text(encoding="utf-8"))
        project = mapping.get("project") or {}
        repo_name = str(project.get("repo") or "eng-cc/oasis7")
        matched: list[dict[str, object]] = []
        for uid, record in sorted((mapping.get("tasks") or {}).items()):
            if worktree_hint_matches(record.get("worktree_hint"), source_worktree):
                record = dict(record)
                record["task_uid"] = uid
                matched.append(record)
        if not matched:
            emit("missing", reason=f"no GitHub Project task has worktree_hint {source_worktree}")
        if len(matched) > 1:
            emit("missing", reason=f"multiple GitHub Project tasks match worktree_hint {source_worktree}")
        record = matched[0]
        task_uid = str(record.get("task_uid") or "")
        if not task_uid_re.fullmatch(task_uid):
            emit("missing", reason=f"invalid mapped task_uid {task_uid}")
        github_issue = {
            "repo": repo_name,
            "number": int(record.get("issue_number") or 0),
            "url": str(record.get("issue_url") or ""),
        }
        candidates.append((task_uid, Path("/dev/null"), str(record.get("issue_url") or "")))
    else:
        if os.environ.get("PREPARE_TASK_PR_ALLOW_GITHUB_ISSUE_FALLBACK") != "1":
            emit(
                "missing",
                reason=(
                    ".pm task files and mapping cache absent; live GitHub issue fallback "
                    "requires PREPARE_TASK_PR_ALLOW_GITHUB_ISSUE_FALLBACK=1"
                ),
            )
        github_issue = github_issue_for_worktree(source_worktree)
        if not github_issue:
            emit("missing", reason=f".pm task files and mapping cache absent; no GitHub issue matches worktree_hint {source_worktree}")
        candidates.append((str(github_issue["task_uid"]), Path("/dev/null"), str(github_issue.get("url") or "")))

if not candidates:
    emit("missing", reason=f"no task has worktree_hint {source_worktree}")
if len(candidates) > 1:
    emit("missing", reason=f"multiple tasks match worktree_hint {source_worktree}")

task_uid, log_path, log_path_rel = candidates[0]
if github_issue:
    if not github_issue["number"]:
        emit("missing", task_uid=task_uid, log_path=log_path_rel, reason="mapped GitHub issue number missing")
    try:
        issue_payload = subprocess.check_output(
            [
                "gh",
                "issue",
                "view",
                str(github_issue["number"]),
                "-R",
                str(github_issue["repo"]),
                "--json",
                "comments",
            ],
            text=True,
        )
    except subprocess.CalledProcessError as exc:
        try:
            comments = issue_comments_via_rest(str(github_issue["repo"]), int(github_issue["number"]))
        except (subprocess.CalledProcessError, json.JSONDecodeError) as rest_exc:
            emit(
                "missing",
                task_uid=task_uid,
                log_path=log_path_rel,
                reason=f"failed to read GitHub issue comments: {exc}; REST fallback failed: {rest_exc}",
            )
    else:
        comments = json.loads(issue_payload).get("comments") or []
    text = "\n\n".join(str(comment.get("body") or "") for comment in comments)
else:
    if not log_path.is_file():
        emit("missing", task_uid=task_uid, log_path=log_path_rel, reason="execution log missing")
    text = log_path.read_text(encoding="utf-8")
blocks = review_packet_blocks(text)
if not blocks:
    emit(
        "missing",
        task_uid=task_uid,
        log_path=log_path_rel,
        reason="no pre-PR local role review packet found",
        missing_markers=["Pre-PR Local Role Review: passed"],
    )

required = {
    "Pre-PR Local Role Review": "passed",
    "Task UID": task_uid,
    "Source Branch": source_branch,
    "Comparison Ref": comparison_ref,
}

missing: list[str] = []
selected_block = blocks[-1]

for key, expected in required.items():
    if parse_field(selected_block, key) != expected:
        missing.append(f"{key}: {expected}")

source_worktree_field = parse_field(selected_block, "Source Worktree")
if not source_worktree_field:
    missing.append("Source Worktree")
elif not worktree_hint_matches(source_worktree_field, source_worktree):
    missing.append(f"Source Worktree: {source_worktree.name} or repo-relative worktree hint")

reviewed_source_head = parse_field(selected_block, "Source Head")
if not reviewed_source_head:
    missing.append("Source Head")
elif reviewed_source_head != source_head:
    allowed_evidence_paths = {
        log_path_rel,
        f".pm/tasks/{task_uid}.yaml",
        ".pm/registry/tasks.yaml",
        ".pm/github-project-sync/tasks.json",
    }
    try:
        subprocess.check_call(
            ["git", "merge-base", "--is-ancestor", reviewed_source_head, source_head],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        changed_since_review = subprocess.check_output(
            ["git", "diff", "--name-only", f"{reviewed_source_head}..{source_head}"],
            text=True,
        ).splitlines()
    except subprocess.CalledProcessError:
        missing.append(f"Source Head ancestor of {source_head}")
    else:
        disallowed = [
            path for path in changed_since_review
            if path not in allowed_evidence_paths
            and not (path.startswith(".pm/roles/") and "/backlog/" in path)
        ]
        if disallowed:
            missing.append("Source Head has post-review non-evidence changes: " + ",".join(disallowed))

for key in (
    "Reviewed Changed Paths",
    "Review Package",
    "Role Selection Basis",
    "Review Roles",
    "Review Evidence",
    "Review Verdicts",
    "Finding Disposition Evidence",
    "Verification Matrix",
    "Visual Evidence",
    "WASM Evidence",
    "Ops Evidence",
    "LiveOps Evidence",
    "Residual Risk",
    "Slice Ledger",
):
    if not parse_field(selected_block, key):
        missing.append(key)

findings_disposition = parse_field(selected_block, "Review Findings Disposition")
if findings_disposition not in {"addressed", "no_findings"}:
    missing.append("Review Findings Disposition: addressed|no_findings")

review_roles = parse_field(selected_block, "Review Roles")
review_package = parse_field(selected_block, "Review Package")
review_verdicts = parse_field(selected_block, "Review Verdicts")
residual_risk = parse_field(selected_block, "Residual Risk")
slice_ledger = parse_field(selected_block, "Slice Ledger")
verification_matrix = parse_field(selected_block, "Verification Matrix")
visual_evidence = parse_field(selected_block, "Visual Evidence")
wasm_evidence = parse_field(selected_block, "WASM Evidence")
ops_evidence = parse_field(selected_block, "Ops Evidence")
liveops_evidence = parse_field(selected_block, "LiveOps Evidence")

if missing:
    emit(
        "missing",
        task_uid=task_uid,
        log_path=log_path_rel,
        reason="missing required pre-PR local role review markers",
        missing_markers=missing,
        review_roles=review_roles,
        review_package=review_package,
        review_verdicts=review_verdicts,
        findings_disposition=findings_disposition,
        residual_risk=residual_risk,
        slice_ledger=slice_ledger,
        verification_matrix=verification_matrix,
        visual_evidence=visual_evidence,
        wasm_evidence=wasm_evidence,
        ops_evidence=ops_evidence,
        liveops_evidence=liveops_evidence,
        reviewed_source_head=reviewed_source_head,
    )

emit(
    "passed",
    task_uid=task_uid,
    log_path=log_path_rel,
    reason="matched source worktree, branch, head, and comparison ref",
    review_roles=review_roles,
    review_package=review_package,
    review_verdicts=review_verdicts,
    findings_disposition=findings_disposition,
    residual_risk=residual_risk,
    slice_ledger=slice_ledger,
    verification_matrix=verification_matrix,
    visual_evidence=visual_evidence,
    wasm_evidence=wasm_evidence,
    ops_evidence=ops_evidence,
    liveops_evidence=liveops_evidence,
    reviewed_source_head=reviewed_source_head,
)
PY
}

ensure_branch_exists "$SOURCE_BRANCH"
SOURCE_COMMIT_REF="refs/heads/${SOURCE_BRANCH}^{commit}"
SOURCE_HEAD="$(git rev-parse "$SOURCE_COMMIT_REF")"
CURRENT_HEAD_COMMIT_REF="HEAD^{commit}"
CURRENT_HEAD="$(git rev-parse "$CURRENT_HEAD_COMMIT_REF")"

SOURCE_WORKTREE="$(branch_checkout_path "$SOURCE_BRANCH" 2>/dev/null || true)"
if [[ -z "$SOURCE_WORKTREE" && "$CURRENT_HEAD" == "$SOURCE_HEAD" ]]; then
  SOURCE_WORKTREE="$(pwd -P)"
fi
[[ -n "$SOURCE_WORKTREE" ]] || die "source branch is not checked out in any worktree: $SOURCE_BRANCH"
ensure_clean_worktree "$SOURCE_WORKTREE" "source"

if [[ "$CREATE_PR" == "1" ]]; then
  git fetch --quiet "$REMOTE_NAME" "$BASE_BRANCH"
fi

LOCAL_BASE_REF=""
REMOTE_BASE_REF=""
if git show-ref --verify --quiet "refs/heads/$BASE_BRANCH"; then
  LOCAL_BASE_REF="refs/heads/$BASE_BRANCH"
fi
if git show-ref --verify --quiet "refs/remotes/$REMOTE_NAME/$BASE_BRANCH"; then
  REMOTE_BASE_REF="refs/remotes/$REMOTE_NAME/$BASE_BRANCH"
fi

COMPARISON_REF="$REMOTE_BASE_REF"
if [[ -z "$COMPARISON_REF" ]]; then
  COMPARISON_REF="$LOCAL_BASE_REF"
fi
[[ -n "$COMPARISON_REF" ]] || die "neither local nor remote base ref exists for $BASE_BRANCH"

COMPARISON_COMMIT_REF="${COMPARISON_REF}^{commit}"
COMPARISON_HEAD="$(git rev-parse "$COMPARISON_COMMIT_REF")"
BASE_WORKTREE=""
if [[ -n "$LOCAL_BASE_REF" ]]; then
  BASE_WORKTREE="$(branch_checkout_path "$BASE_BRANCH" 2>/dev/null || true)"
fi

read -r BEHIND_COUNT AHEAD_COUNT <<<"$(git rev-list --left-right --count "$COMPARISON_REF...$SOURCE_BRANCH")"
if [[ "$BEHIND_COUNT" != "0" ]]; then
  REBASE_REQUIRED=1
else
  REBASE_REQUIRED=0
fi

LOCAL_REQUIRED_SCOPE="unavailable"
LOCAL_REQUIRED_CHANGED_PATH_COUNT=0
LOCAL_REQUIRED_CHANGED_PATHS=""
LOCAL_REQUIRED_REASON_SUMMARY=""
LOCAL_REQUIRED_COMMAND=""
CLAIM_READY_COMMAND=""
LOCAL_REQUIRED_EXTRA_COMMANDS=()

if [[ -x "./scripts/plan-rust-required-scope.sh" ]]; then
  if RUST_SCOPE_OUTPUT="$(./scripts/plan-rust-required-scope.sh --event-name pull_request --base-ref "$COMPARISON_REF" --head-ref "$SOURCE_BRANCH" 2>/dev/null)"; then
    LOCAL_REQUIRED_SCOPE="$(plan_kv_get "$RUST_SCOPE_OUTPUT" "scope")"
    LOCAL_REQUIRED_SCOPE="${LOCAL_REQUIRED_SCOPE:-unavailable}"
    LOCAL_REQUIRED_CHANGED_PATH_COUNT="$(plan_kv_get "$RUST_SCOPE_OUTPUT" "changed_path_count")"
    LOCAL_REQUIRED_CHANGED_PATH_COUNT="${LOCAL_REQUIRED_CHANGED_PATH_COUNT:-0}"
    LOCAL_REQUIRED_CHANGED_PATHS="$(plan_kv_get "$RUST_SCOPE_OUTPUT" "changed_paths")"
    LOCAL_REQUIRED_REASON_SUMMARY="$(plan_kv_get "$RUST_SCOPE_OUTPUT" "reason_summary")"
    if [[ "$LOCAL_REQUIRED_SCOPE" != "minimal" ]]; then
      RUN_OASIS7_REQUIRED_TESTS="$(plan_kv_get_default "$RUST_SCOPE_OUTPUT" "run_oasis7_required_tests" "false")"
      RUN_CONSENSUS_TESTS="$(plan_kv_get_default "$RUST_SCOPE_OUTPUT" "run_consensus_tests" "false")"
      RUN_DISTFS_TESTS="$(plan_kv_get_default "$RUST_SCOPE_OUTPUT" "run_distfs_tests" "false")"
      RUN_OASIS7_NODE_TESTS="$(plan_kv_get_default "$RUST_SCOPE_OUTPUT" "run_oasis7_node_tests" "false")"
      RUN_OASIS7_NET_TESTS="$(plan_kv_get_default "$RUST_SCOPE_OUTPUT" "run_oasis7_net_tests" "false")"
      RUN_OASIS7_NET_LIBP2P_TESTS="$(plan_kv_get_default "$RUST_SCOPE_OUTPUT" "run_oasis7_net_libp2p_tests" "false")"
      RUN_VIEWER_CONTRACT_TESTS="$(plan_kv_get_default "$RUST_SCOPE_OUTPUT" "run_viewer_contract_tests" "false")"
      RUN_VIEWER_WASM_CHECK="$(plan_kv_get_default "$RUST_SCOPE_OUTPUT" "run_viewer_wasm_check" "false")"
      RUN_VIEWER_PERF_SMOKE="$(plan_kv_get_default "$RUST_SCOPE_OUTPUT" "run_viewer_perf_smoke" "false")"
      RUN_LAUNCHER_WEB_BUILD="$(plan_kv_get_default "$RUST_SCOPE_OUTPUT" "run_launcher_web_build" "false")"
      RUN_WORKSPACE_SUPPORT_CRATE_TESTS="$(plan_kv_get_default "$RUST_SCOPE_OUTPUT" "run_oasis7_workspace_support_crate_tests" "false")"
      LOCAL_REQUIRED_COMMAND="OASIS7_CI_RUN_OASIS7_REQUIRED_TESTS=$RUN_OASIS7_REQUIRED_TESTS \
OASIS7_CI_RUN_CONSENSUS_TESTS=$RUN_CONSENSUS_TESTS \
OASIS7_CI_RUN_DISTFS_TESTS=$RUN_DISTFS_TESTS \
OASIS7_CI_RUN_OASIS7_NODE_TESTS=$RUN_OASIS7_NODE_TESTS \
OASIS7_CI_RUN_OASIS7_NET_TESTS=$RUN_OASIS7_NET_TESTS \
OASIS7_CI_RUN_OASIS7_NET_LIBP2P_TESTS=$RUN_OASIS7_NET_LIBP2P_TESTS \
OASIS7_CI_RUN_VIEWER_CONTRACT_TESTS=$RUN_VIEWER_CONTRACT_TESTS \
OASIS7_CI_RUN_VIEWER_WASM_CHECK=$RUN_VIEWER_WASM_CHECK \
OASIS7_CI_RUN_VIEWER_PERF_SMOKE=$RUN_VIEWER_PERF_SMOKE \
OASIS7_CI_RUN_LAUNCHER_WEB_BUILD=$RUN_LAUNCHER_WEB_BUILD \
OASIS7_CI_RUN_WORKSPACE_SUPPORT_CRATE_TESTS=$RUN_WORKSPACE_SUPPORT_CRATE_TESTS \
./scripts/ci-tests.sh required"
    fi
    if [[ -z "$LOCAL_REQUIRED_COMMAND" ]]; then
      LOCAL_REQUIRED_COMMAND="git diff --check"
      case "$LOCAL_REQUIRED_CHANGED_PATHS" in
        *".agents/"*|*"AGENTS.md"*|*"doc/engineering/workflow/"*|*"scripts/pm/"*|*"scripts/prepare-task-pr.sh"*|*"scripts/pr-review-thread-closeout.sh"*)
          LOCAL_REQUIRED_COMMAND="./scripts/lint-skills.sh && ./scripts/pm/lint.sh && ./scripts/doc-governance-check.sh && git diff --check"
          ;;
      esac
    fi
    if [[ -n "$LOCAL_REQUIRED_COMMAND" ]]; then
      CLAIM_READY_COMMAND="$(render_cmd "./scripts/pm/claim-ready.sh" "--claim-type" "ready_for_pr" "--verification-profile" "repository_required")"
    fi
  fi
fi

REMOTE_SOURCE_REF=""
if git show-ref --verify --quiet "refs/remotes/$REMOTE_NAME/$SOURCE_BRANCH"; then
  REMOTE_SOURCE_REF="refs/remotes/$REMOTE_NAME/$SOURCE_BRANCH"
fi

LOCAL_ROLE_REVIEW_OUTPUT="$(local_role_review_status "$SOURCE_WORKTREE" "$SOURCE_BRANCH" "$SOURCE_HEAD" "$COMPARISON_REF")"
LOCAL_ROLE_REVIEW_STATUS="$(plan_kv_get "$LOCAL_ROLE_REVIEW_OUTPUT" "status")"
LOCAL_ROLE_REVIEW_TASK_UID="$(plan_kv_get "$LOCAL_ROLE_REVIEW_OUTPUT" "task_uid")"
LOCAL_ROLE_REVIEW_LOG_PATH="$(plan_kv_get "$LOCAL_ROLE_REVIEW_OUTPUT" "evidence_sink")"
LOCAL_ROLE_REVIEW_LOG_PATH="${LOCAL_ROLE_REVIEW_LOG_PATH:-$(plan_kv_get "$LOCAL_ROLE_REVIEW_OUTPUT" "execution_log_path")}"
LOCAL_ROLE_REVIEW_REASON="$(plan_kv_get "$LOCAL_ROLE_REVIEW_OUTPUT" "reason")"
LOCAL_ROLE_REVIEW_MISSING_MARKERS="$(plan_kv_get "$LOCAL_ROLE_REVIEW_OUTPUT" "missing_markers")"
LOCAL_ROLE_REVIEW_ROLES="$(plan_kv_get "$LOCAL_ROLE_REVIEW_OUTPUT" "review_roles")"
LOCAL_ROLE_REVIEW_PACKAGE="$(plan_kv_get "$LOCAL_ROLE_REVIEW_OUTPUT" "review_package")"
LOCAL_ROLE_REVIEW_VERDICTS="$(plan_kv_get "$LOCAL_ROLE_REVIEW_OUTPUT" "review_verdicts")"
LOCAL_ROLE_REVIEW_FINDINGS_DISPOSITION="$(plan_kv_get "$LOCAL_ROLE_REVIEW_OUTPUT" "findings_disposition")"
LOCAL_ROLE_REVIEW_RESIDUAL_RISK="$(plan_kv_get "$LOCAL_ROLE_REVIEW_OUTPUT" "residual_risk")"
LOCAL_ROLE_REVIEW_SLICE_LEDGER="$(plan_kv_get "$LOCAL_ROLE_REVIEW_OUTPUT" "slice_ledger")"
LOCAL_ROLE_REVIEW_VERIFICATION_MATRIX="$(plan_kv_get "$LOCAL_ROLE_REVIEW_OUTPUT" "verification_matrix")"
LOCAL_ROLE_REVIEW_VISUAL_EVIDENCE="$(plan_kv_get "$LOCAL_ROLE_REVIEW_OUTPUT" "visual_evidence")"
LOCAL_ROLE_REVIEW_WASM_EVIDENCE="$(plan_kv_get "$LOCAL_ROLE_REVIEW_OUTPUT" "wasm_evidence")"
LOCAL_ROLE_REVIEW_OPS_EVIDENCE="$(plan_kv_get "$LOCAL_ROLE_REVIEW_OUTPUT" "ops_evidence")"
LOCAL_ROLE_REVIEW_LIVEOPS_EVIDENCE="$(plan_kv_get "$LOCAL_ROLE_REVIEW_OUTPUT" "liveops_evidence")"
LOCAL_ROLE_REVIEW_SOURCE_HEAD="$(plan_kv_get "$LOCAL_ROLE_REVIEW_OUTPUT" "reviewed_source_head")"
REQUIRED_REVIEW_ROLES="$(required_review_roles_from_paths "$LOCAL_REQUIRED_CHANGED_PATHS")"
MISSING_REQUIRED_REVIEW_ROLES="$(missing_required_review_roles "$REQUIRED_REVIEW_ROLES" "$LOCAL_ROLE_REVIEW_ROLES")"
MISSING_SEMANTIC_REVIEW_EVIDENCE="$(semantic_review_evidence_missing "$REQUIRED_REVIEW_ROLES" "$LOCAL_ROLE_REVIEW_VERIFICATION_MATRIX" "$LOCAL_ROLE_REVIEW_VISUAL_EVIDENCE" "$LOCAL_ROLE_REVIEW_WASM_EVIDENCE" "$LOCAL_ROLE_REVIEW_OPS_EVIDENCE" "$LOCAL_ROLE_REVIEW_LIVEOPS_EVIDENCE")"

if [[ "$CREATE_PR" == "1" && "$LOCAL_ROLE_REVIEW_STATUS" != "passed" ]]; then
  die "missing passed pre-PR local role review evidence for $SOURCE_BRANCH at $SOURCE_HEAD ($LOCAL_ROLE_REVIEW_REASON; log: ${LOCAL_ROLE_REVIEW_LOG_PATH:-unknown}; missing: ${LOCAL_ROLE_REVIEW_MISSING_MARKERS:-unknown})"
fi

if [[ "$CREATE_PR" == "1" && -n "$MISSING_REQUIRED_REVIEW_ROLES" ]]; then
  die "pre-PR local role review is missing required role(s) inferred from changed paths: $MISSING_REQUIRED_REVIEW_ROLES (present: ${LOCAL_ROLE_REVIEW_ROLES:-none}; required: $REQUIRED_REVIEW_ROLES)"
fi

if [[ "$CREATE_PR" == "1" && -n "$MISSING_SEMANTIC_REVIEW_EVIDENCE" ]]; then
  die "pre-PR local role review is missing required semantic evidence for inferred roles: $MISSING_SEMANTIC_REVIEW_EVIDENCE"
fi
if [[ "$CREATE_PR" == "1" ]]; then
  [[ "$LOCAL_ROLE_REVIEW_SLICE_LEDGER" != n/a* ]] || die "pre-PR local role review requires a machine-checkable role-return ledger"
  python3 "$ROOT_DIR/scripts/pm/validate-review-provenance.py" \
    --root "$SOURCE_WORKTREE" \
    --task-uid "$LOCAL_ROLE_REVIEW_TASK_UID" \
    --ledger "$LOCAL_ROLE_REVIEW_SLICE_LEDGER" \
    --roles "$LOCAL_ROLE_REVIEW_ROLES" \
    --source-head "$LOCAL_ROLE_REVIEW_SOURCE_HEAD" >/dev/null \
    || die "pre-PR local role-return validation failed"
fi

WORKFLOW_LINT_ARGS=("--phase" "pr-ready" "--allow-unbound")
if [[ -n "$LOCAL_ROLE_REVIEW_TASK_UID" ]]; then
  WORKFLOW_LINT_ARGS=("--task-uid" "$LOCAL_ROLE_REVIEW_TASK_UID" "${WORKFLOW_LINT_ARGS[@]}")
fi

WORKFLOW_LINT_OUTPUT=""
WORKFLOW_LINT_BIN="${PREPARE_TASK_PR_WORKFLOW_LINT_PATH:-./scripts/pm/workflow-lint.sh}"
if ! WORKFLOW_LINT_OUTPUT="$(cd "$SOURCE_WORKTREE" && PM_ROOT_DIR="$SOURCE_WORKTREE" "$WORKFLOW_LINT_BIN" "${WORKFLOW_LINT_ARGS[@]}" 2>&1)"; then
  if [[ "$WORKFLOW_LINT_OUTPUT" == *"unknown arg: --phase"* ]]; then
    WORKFLOW_LINT_FALLBACK_ARGS=("--allow-unbound")
    if [[ -n "$LOCAL_ROLE_REVIEW_TASK_UID" ]]; then
      WORKFLOW_LINT_FALLBACK_ARGS=("--task-uid" "$LOCAL_ROLE_REVIEW_TASK_UID" "${WORKFLOW_LINT_FALLBACK_ARGS[@]}")
    fi
    WORKFLOW_LINT_OUTPUT="$(cd "$SOURCE_WORKTREE" && PM_ROOT_DIR="$SOURCE_WORKTREE" "$WORKFLOW_LINT_BIN" "${WORKFLOW_LINT_FALLBACK_ARGS[@]}" 2>&1)" || {
      cat >&2 <<EOF
error: workflow-lint preflight failed.
$WORKFLOW_LINT_OUTPUT
fix: apply the suggested repair command(s) above, then rerun ./scripts/prepare-task-pr.sh.
EOF
      exit 1
    }
  else
  cat >&2 <<EOF
error: workflow-lint preflight failed.
$WORKFLOW_LINT_OUTPUT
fix: apply the suggested repair command(s) above, then rerun ./scripts/prepare-task-pr.sh.
EOF
  exit 1
  fi
fi

UPSTREAM_REF="$(git rev-parse --abbrev-ref --symbolic-full-name "$SOURCE_BRANCH@{upstream}" 2>/dev/null || true)"
LOCAL_ONLY_COUNT="$AHEAD_COUNT"
REMOTE_ONLY_COUNT=0
if [[ -n "$REMOTE_SOURCE_REF" ]]; then
  read -r REMOTE_ONLY_COUNT LOCAL_ONLY_COUNT <<<"$(git rev-list --left-right --count "$REMOTE_SOURCE_REF...$SOURCE_BRANCH")"
fi

TASK_ISSUE_NUMBER=""
GENERATED_PR_BODY=""
if [[ -n "$LOCAL_ROLE_REVIEW_TASK_UID" ]]; then
  TASK_ISSUE_NUMBER="$(task_issue_number_from_review "$LOCAL_ROLE_REVIEW_TASK_UID" "$LOCAL_ROLE_REVIEW_LOG_PATH" "$SOURCE_WORKTREE")"
fi

if [[ -n "$TASK_ISSUE_NUMBER" ]]; then
  if [[ -n "$BODY_FILE" ]]; then
    if ! body_file_has_task_reference "$BODY_FILE" "$TASK_ISSUE_NUMBER"; then
      die "--body-file must include a non-closing GitHub task reference and no auto-close keyword, for example: Refs #$TASK_ISSUE_NUMBER"
    fi
  else
    GENERATED_PR_BODY="Task: ${LOCAL_ROLE_REVIEW_TASK_UID:-unknown}

Refs #$TASK_ISSUE_NUMBER

Generated by ./scripts/prepare-task-pr.sh."
  fi
elif [[ "$CREATE_PR" == "1" && -n "$LOCAL_ROLE_REVIEW_TASK_UID" ]]; then
  die "cannot resolve GitHub task issue number for $LOCAL_ROLE_REVIEW_TASK_UID; refusing to create a task PR without a task reference"
elif [[ -n "$PR_TITLE" && -z "$BODY_FILE" ]]; then
  GENERATED_PR_BODY="Task: ${LOCAL_ROLE_REVIEW_TASK_UID:-unknown}

Generated by ./scripts/prepare-task-pr.sh."
fi

CREATE_CMD=("gh" "pr" "create" "--base" "$BASE_BRANCH" "--head" "$SOURCE_BRANCH")
if [[ -n "$PR_TITLE" ]]; then
  CREATE_CMD+=("--title" "$PR_TITLE")
else
  CREATE_CMD+=("--fill")
fi
if [[ -n "$BODY_FILE" ]]; then
  CREATE_CMD+=("--body-file" "$BODY_FILE")
elif [[ -n "$GENERATED_PR_BODY" ]]; then
  CREATE_CMD+=("--body" "$GENERATED_PR_BODY")
fi
if [[ "$DRAFT_PR" == "1" ]]; then
  CREATE_CMD+=("--draft")
fi
CREATE_CMD_RENDERED="$(render_cmd "${CREATE_CMD[@]}")"

SYNC_CMD=""
if [[ -n "$BASE_WORKTREE" ]]; then
  SYNC_CMD="git -C $BASE_WORKTREE pull --ff-only $REMOTE_NAME $BASE_BRANCH"
fi
CLEANUP_CMD_1="RECEIPT_ROOT=\$(python3 $CANONICAL_REPO_ROOT/scripts/pm/canonical-receipt-root.py --default-worktree $CANONICAL_REPO_ROOT --task-uid ${LOCAL_ROLE_REVIEW_TASK_UID:-unknown} --create) && python3 $CANONICAL_REPO_ROOT/scripts/pm/pr-merge-receipt.py <pr-number> --json > \"\$RECEIPT_ROOT/merge-receipt.json\" && $CANONICAL_REPO_ROOT/scripts/pm/post-merge-main-sync.sh --repo-root $CANONICAL_REPO_ROOT --main-ref $BASE_BRANCH --task-uid ${LOCAL_ROLE_REVIEW_TASK_UID:-unknown} --pr-receipt \"\$RECEIPT_ROOT/merge-receipt.json\" --receipt-output \"\$RECEIPT_ROOT/main-sync-receipt.json\" && $CANONICAL_REPO_ROOT/scripts/pm/post-merge-cleanup.sh --repo-root $CANONICAL_REPO_ROOT --worktree $SOURCE_WORKTREE --branch $SOURCE_BRANCH --main-ref $BASE_BRANCH --task-uid ${LOCAL_ROLE_REVIEW_TASK_UID:-unknown} --pr-receipt \"\$RECEIPT_ROOT/merge-receipt.json\" --main-sync-receipt \"\$RECEIPT_ROOT/main-sync-receipt.json\" --terminal-receipt-output \"\$RECEIPT_ROOT/terminal-cleanup-receipt.json\""
CLEANUP_CMD_2=""

PR_URL=""
if [[ "$CREATE_PR" == "1" ]]; then
  command -v gh >/dev/null 2>&1 || die '`gh` not found in PATH'
  if [[ -z "$REMOTE_SOURCE_REF" ]]; then
    git -C "$SOURCE_WORKTREE" push -u "$REMOTE_NAME" "$SOURCE_BRANCH"
  elif [[ "$LOCAL_ONLY_COUNT" != "0" || "$REMOTE_ONLY_COUNT" != "0" ]]; then
    git -C "$SOURCE_WORKTREE" push "$REMOTE_NAME" "$SOURCE_BRANCH"
  fi
  CURRENT_REPO="$(gh repo view --json nameWithOwner --jq .nameWithOwner)"
  EXISTING_PR_JSON="$(gh pr list --state all --head "$SOURCE_BRANCH" --base "$BASE_BRANCH" --json url,headRefName,baseRefName,state,headRepository,headRepositoryOwner --limit 100)"
  PR_URL="$(python3 - "$SOURCE_BRANCH" "$BASE_BRANCH" "$CURRENT_REPO" "$EXISTING_PR_JSON" <<'PY'
import json, sys
head, base, repo, raw = sys.argv[1:]
owner, name = repo.split('/', 1)
def repo_identity(item):
    raw_owner=item.get('headRepositoryOwner') or {}
    raw_repo=item.get('headRepository') or {}
    return (raw_owner.get('login') if isinstance(raw_owner,dict) else raw_owner, raw_repo.get('name') if isinstance(raw_repo,dict) else raw_repo)
exact = [item for item in json.loads(raw) if item.get("headRefName") == head and item.get("baseRefName") == base and repo_identity(item) == (owner,name)]
if any(str(item.get('state')).upper() == 'MERGED' for item in exact):
    raise SystemExit("prepare-task-pr: exact repository/head/base PR is already MERGED; reconcile task truth instead of creating a replacement")
matches = [item for item in exact if item.get("state", "OPEN").upper() == "OPEN"]
if len(matches) > 1:
    raise SystemExit("prepare-task-pr: multiple OPEN PRs match exact repository/head/base")
print(matches[0]["url"] if matches else "")
PY
)"
  if [[ -z "$PR_URL" ]]; then
    PR_URL="$("${CREATE_CMD[@]}")"
  fi
  if [[ -n "$LOCAL_ROLE_REVIEW_TASK_UID" ]]; then
    RECORD_PR_ERR="$(mktemp)"
    if ! python3 "$ROOT_DIR/scripts/pm/github-project-task.py" record-pr "$SOURCE_WORKTREE" \
      --task-uid "$LOCAL_ROLE_REVIEW_TASK_UID" \
      --pr-url "$PR_URL" \
      --role tpm \
      --validation-command "$CREATE_CMD_RENDERED" \
      --json >/dev/null 2>"$RECORD_PR_ERR"; then
      RECORD_PR_FAILURE="$(cat "$RECORD_PR_ERR")"
      rm -f "$RECORD_PR_ERR"
      die "PR exists at $PR_URL but task record transition failed for $LOCAL_ROLE_REVIEW_TASK_UID: $RECORD_PR_FAILURE; recover with: python3 scripts/pm/github-project-task.py record-pr '$SOURCE_WORKTREE' --task-uid '$LOCAL_ROLE_REVIEW_TASK_UID' --pr-url '$PR_URL' --role tpm --validation-command 'resume exact head/base PR record' --json"
    fi
    rm -f "$RECORD_PR_ERR"
  fi
fi

LOCAL_REQUIRED_EXTRA_COMMANDS_JOINED="$(printf '%s;' ${LOCAL_REQUIRED_EXTRA_COMMANDS[@]+"${LOCAL_REQUIRED_EXTRA_COMMANDS[@]}"})"
SUMMARY_JSON="$(
python3 - "$SOURCE_BRANCH" "$SOURCE_WORKTREE" "$SOURCE_HEAD" "$BASE_BRANCH" "$COMPARISON_REF" "$COMPARISON_HEAD" "$REMOTE_NAME" "$AHEAD_COUNT" "$BEHIND_COUNT" "$REBASE_REQUIRED" "$UPSTREAM_REF" "$LOCAL_ONLY_COUNT" "$REMOTE_ONLY_COUNT" "$CREATE_CMD_RENDERED" "$SYNC_CMD" "$CLEANUP_CMD_1" "$CLEANUP_CMD_2" "$PR_URL" "$LOCAL_REQUIRED_SCOPE" "$LOCAL_REQUIRED_CHANGED_PATH_COUNT" "$LOCAL_REQUIRED_CHANGED_PATHS" "$LOCAL_REQUIRED_REASON_SUMMARY" "$LOCAL_REQUIRED_COMMAND" "$CLAIM_READY_COMMAND" "$LOCAL_REQUIRED_EXTRA_COMMANDS_JOINED" "$LOCAL_ROLE_REVIEW_STATUS" "$LOCAL_ROLE_REVIEW_TASK_UID" "$LOCAL_ROLE_REVIEW_LOG_PATH" "$LOCAL_ROLE_REVIEW_REASON" "$LOCAL_ROLE_REVIEW_MISSING_MARKERS" "$LOCAL_ROLE_REVIEW_ROLES" "$LOCAL_ROLE_REVIEW_PACKAGE" "$LOCAL_ROLE_REVIEW_VERDICTS" "$LOCAL_ROLE_REVIEW_FINDINGS_DISPOSITION" "$LOCAL_ROLE_REVIEW_RESIDUAL_RISK" "$LOCAL_ROLE_REVIEW_SLICE_LEDGER" "$REQUIRED_REVIEW_ROLES" "$MISSING_REQUIRED_REVIEW_ROLES" "$LOCAL_ROLE_REVIEW_VERIFICATION_MATRIX" "$LOCAL_ROLE_REVIEW_VISUAL_EVIDENCE" "$LOCAL_ROLE_REVIEW_WASM_EVIDENCE" "$LOCAL_ROLE_REVIEW_OPS_EVIDENCE" "$LOCAL_ROLE_REVIEW_LIVEOPS_EVIDENCE" "$MISSING_SEMANTIC_REVIEW_EVIDENCE" <<'PY'
from __future__ import annotations

import json
import sys

changed_paths = [path for path in sys.argv[21].split(";") if path]
reason_items = [reason for reason in sys.argv[22].split(";") if reason]
extra_commands = [cmd for cmd in sys.argv[25].split(";") if cmd]
missing_markers = [marker for marker in sys.argv[30].split(";") if marker]
missing_semantic_evidence = [marker for marker in sys.argv[44].split(";") if marker]

payload = {
    "source_branch": sys.argv[1],
    "source_worktree": sys.argv[2],
    "source_head": sys.argv[3],
    "base_branch": sys.argv[4],
    "comparison_ref": sys.argv[5],
    "comparison_head": sys.argv[6],
    "remote_name": sys.argv[7],
    "ahead_count": int(sys.argv[8]),
    "behind_count": int(sys.argv[9]),
    "rebase_required": sys.argv[10] == "1",
    "upstream_ref": sys.argv[11] or None,
    "unpushed_commit_count": int(sys.argv[12]),
    "remote_only_commit_count": int(sys.argv[13]),
    "create_command": sys.argv[14],
    "review_request_command": None,
    "post_merge_commands": [cmd for cmd in sys.argv[15:18] if cmd],
    "cleanup_commands": [cmd for cmd in sys.argv[15:18] if cmd],
    "pr_url": sys.argv[18] or None,
    "local_required_validation": {
        "scope": sys.argv[19],
        "changed_path_count": int(sys.argv[20]),
        "changed_paths": changed_paths,
        "reason_summary": sys.argv[22] or None,
        "reason_items": reason_items,
        "recommended_required_command": sys.argv[23] or None,
        "recommended_claim_ready_command": sys.argv[24] or None,
        "recommended_extra_commands": extra_commands,
    },
    "pre_pr_local_role_review": {
        "status": sys.argv[26],
        "task_uid": sys.argv[27] or None,
        "evidence_sink": sys.argv[28] or None,
        "execution_log_path": sys.argv[28] or None,
        "reason": sys.argv[29] or None,
        "missing_markers": missing_markers,
        "review_roles": sys.argv[31] or None,
        "review_package": sys.argv[32] or None,
        "review_verdicts": sys.argv[33] or None,
        "findings_disposition": sys.argv[34] or None,
        "residual_risk": sys.argv[35] or None,
        "slice_ledger": sys.argv[36] or None,
        "required_roles_from_changed_paths": sys.argv[37] or None,
        "missing_required_roles": sys.argv[38] or None,
        "verification_matrix": sys.argv[39] or None,
        "visual_evidence": sys.argv[40] or None,
        "wasm_evidence": sys.argv[41] or None,
        "ops_evidence": sys.argv[42] or None,
        "liveops_evidence": sys.argv[43] or None,
        "missing_semantic_evidence": missing_semantic_evidence,
    },
}
print(json.dumps(payload, ensure_ascii=False))
PY
)"

if [[ "$OUTPUT_JSON" == "1" ]]; then
  printf '%s\n' "$SUMMARY_JSON"
  exit 0
fi

REBASE_NOTE="no"
if [[ "$REBASE_REQUIRED" == "1" ]]; then
  REBASE_NOTE="suggested"
fi

cat <<INFO
Task PR preflight summary:
- source branch: $SOURCE_BRANCH
- source worktree: $SOURCE_WORKTREE
- source head: $SOURCE_HEAD
- base branch: $BASE_BRANCH
- comparison ref: $COMPARISON_REF
- remote: $REMOTE_NAME
- ahead of base: $AHEAD_COUNT
- behind base: $BEHIND_COUNT
- branch sync suggested: $REBASE_NOTE
- upstream: ${UPSTREAM_REF:-"(none)"}
- unpushed commits: $LOCAL_ONLY_COUNT
- remote-only commits on source: $REMOTE_ONLY_COUNT
- create command: $CREATE_CMD_RENDERED
INFO

echo
echo "Local Required Validation:"
echo "- scope: $LOCAL_REQUIRED_SCOPE"
echo "- changed paths: $LOCAL_REQUIRED_CHANGED_PATH_COUNT"
if [[ -n "$LOCAL_REQUIRED_REASON_SUMMARY" ]]; then
  echo "- planner reason summary: $LOCAL_REQUIRED_REASON_SUMMARY"
  while IFS= read -r reason_item; do
    [[ -n "$reason_item" ]] || continue
    echo "  - planner reason: $reason_item"
  done < <(printf '%s\n' "$LOCAL_REQUIRED_REASON_SUMMARY" | tr ';' '\n')
fi
if [[ -n "$LOCAL_REQUIRED_COMMAND" ]]; then
  echo "- recommended required command: $LOCAL_REQUIRED_COMMAND"
fi
if [[ -n "$CLAIM_READY_COMMAND" ]]; then
  echo "- recommended claim-ready command: $CLAIM_READY_COMMAND"
fi
if [[ "${#LOCAL_REQUIRED_EXTRA_COMMANDS[@]}" -gt 0 ]]; then
  for extra_cmd in "${LOCAL_REQUIRED_EXTRA_COMMANDS[@]}"; do
    echo "- recommended extra command: $extra_cmd"
  done
fi

echo
echo "Pre-PR Local Role Review:"
echo "- status: $LOCAL_ROLE_REVIEW_STATUS"
echo "- task uid: ${LOCAL_ROLE_REVIEW_TASK_UID:-"(none)"}"
echo "- evidence sink: ${LOCAL_ROLE_REVIEW_LOG_PATH:-"(none)"}"
echo "- reason: ${LOCAL_ROLE_REVIEW_REASON:-"(none)"}"
if [[ -n "$LOCAL_ROLE_REVIEW_MISSING_MARKERS" ]]; then
  echo "- missing markers: $LOCAL_ROLE_REVIEW_MISSING_MARKERS"
fi
if [[ -n "$LOCAL_ROLE_REVIEW_ROLES" ]]; then
  echo "- review roles: $LOCAL_ROLE_REVIEW_ROLES"
fi
if [[ -n "$REQUIRED_REVIEW_ROLES" ]]; then
  echo "- required roles from changed paths: $REQUIRED_REVIEW_ROLES"
fi
if [[ -n "$MISSING_REQUIRED_REVIEW_ROLES" ]]; then
  echo "- missing required roles: $MISSING_REQUIRED_REVIEW_ROLES"
fi
if [[ -n "$MISSING_SEMANTIC_REVIEW_EVIDENCE" ]]; then
  echo "- missing semantic evidence: $MISSING_SEMANTIC_REVIEW_EVIDENCE"
fi
if [[ -n "$LOCAL_ROLE_REVIEW_PACKAGE" ]]; then
  echo "- review package: $LOCAL_ROLE_REVIEW_PACKAGE"
fi
if [[ -n "$LOCAL_ROLE_REVIEW_VERDICTS" ]]; then
  echo "- review verdicts: $LOCAL_ROLE_REVIEW_VERDICTS"
fi
if [[ -n "$LOCAL_ROLE_REVIEW_FINDINGS_DISPOSITION" ]]; then
  echo "- findings disposition: $LOCAL_ROLE_REVIEW_FINDINGS_DISPOSITION"
fi
if [[ -n "$LOCAL_ROLE_REVIEW_VERIFICATION_MATRIX" ]]; then
  echo "- verification matrix: $LOCAL_ROLE_REVIEW_VERIFICATION_MATRIX"
fi
if [[ -n "$LOCAL_ROLE_REVIEW_VISUAL_EVIDENCE" ]]; then
  echo "- visual evidence: $LOCAL_ROLE_REVIEW_VISUAL_EVIDENCE"
fi
if [[ -n "$LOCAL_ROLE_REVIEW_WASM_EVIDENCE" ]]; then
  echo "- wasm evidence: $LOCAL_ROLE_REVIEW_WASM_EVIDENCE"
fi
if [[ -n "$LOCAL_ROLE_REVIEW_OPS_EVIDENCE" ]]; then
  echo "- ops evidence: $LOCAL_ROLE_REVIEW_OPS_EVIDENCE"
fi
if [[ -n "$LOCAL_ROLE_REVIEW_LIVEOPS_EVIDENCE" ]]; then
  echo "- liveops evidence: $LOCAL_ROLE_REVIEW_LIVEOPS_EVIDENCE"
fi
if [[ -n "$LOCAL_ROLE_REVIEW_RESIDUAL_RISK" ]]; then
  echo "- residual risk: $LOCAL_ROLE_REVIEW_RESIDUAL_RISK"
fi
if [[ -n "$LOCAL_ROLE_REVIEW_SLICE_LEDGER" ]]; then
  echo "- slice ledger: $LOCAL_ROLE_REVIEW_SLICE_LEDGER"
fi

if [[ "$REBASE_REQUIRED" == "1" ]]; then
  echo
  echo "Suggested branch sync before merge if GitHub later requires it:"
  echo "  git -C $SOURCE_WORKTREE rebase $COMPARISON_REF"
fi

if [[ "$CREATE_PR" == "1" ]]; then
  echo
  echo "Created PR:"
  echo "  $PR_URL"
fi

echo
echo "Post-Merge Cleanup:"
if [[ -n "$SYNC_CMD" ]]; then
  echo "  $SYNC_CMD"
else
  echo "  sync local $BASE_BRANCH manually in the worktree that keeps it checked out"
fi
echo "  $CLEANUP_CMD_1"
echo "  $CLEANUP_CMD_2"
