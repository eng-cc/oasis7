#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

OUTPUT_JSON=0

usage() {
  cat <<'USAGE'
Usage: ./scripts/pm/workflow-behavior-eval.sh [--json]

Run the repo-owned workflow behavior eval for the default oasis7 task chain:
  default-workflow-bootstrap -> new-task-worktree -> workflow-report
  -> repo-owned-workflow-router -> TPM coordinate/integrate only + professional role subagent dispatch
  -> task-closeout -> prepare-task-pr -> PR CI/comment watch/fix -> review-thread-closeout -> merge/cleanup

This eval reuses isolated fixture tests and PM smokes so the main chain stays
provable in local automation rather than only in prose.

Options:
  --json      Print machine-readable JSON summary
  -h, --help  Show help
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --json)
      OUTPUT_JSON=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "workflow-behavior-eval: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-workflow-eval.XXXXXX")"
cleanup() {
  local status=$?
  rm -rf "$TMP_DIR"
  exit "$status"
}
trap cleanup EXIT

TASK_WORKTREE_JSON_FILE="$TMP_DIR/task-worktree.json"
REQUIRED_TIER_JSON_FILE="$TMP_DIR/required-tier.json"
SUBAGENT_CONTRACT_JSON_FILE="$TMP_DIR/subagent-contract.json"
ROUTING_SCENARIOS_JSON_FILE="$TMP_DIR/routing-scenarios.json"

"$ROOT_DIR/scripts/pm/new-task-worktree-bootstrap-smoke.sh" --json > "$TASK_WORKTREE_JSON_FILE"
"$ROOT_DIR/scripts/pm/required-tier-smoke.sh" --json > "$REQUIRED_TIER_JSON_FILE"
"$ROOT_DIR/scripts/pm/claim-ready.test.sh" >/dev/null
"$ROOT_DIR/scripts/prepare-task-pr.test.sh" >/dev/null
"$ROOT_DIR/scripts/pr-review-thread-closeout.test.sh" >/dev/null

python3 - "$ROOT_DIR" > "$SUBAGENT_CONTRACT_JSON_FILE" <<'PY'
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

root = Path(sys.argv[1])
bt = chr(96)
source_text = (root / "doc/engineering/workflow/source-of-truth.md").read_text(encoding="utf-8")
default_runtime_match = re.search(
    r"Default subagent runtime is `([^`]+)` with `reasoning_effort=([^`]+)` "
    r"\(shorthand: `([^`]+)`\)",
    source_text,
)
if not default_runtime_match:
    raise SystemExit("workflow-behavior-eval: source-of-truth missing parseable Default subagent runtime policy")
default_model, default_reasoning, default_shorthand = default_runtime_match.groups()
default_runtime_marker = (
    f"Default subagent runtime is `{default_model}` with "
    f"`reasoning_effort={default_reasoning}`"
)

checks = [
    (
        root / "doc/engineering/workflow/source-of-truth.md",
        [
            "### 1.1 Skill Map by Phase",
            "default-workflow-bootstrap",
            "repo-owned-workflow-router",
            "systematic-debugging",
            "receiving-code-review",
            "writing-repo-owned-skills",
            "### 1.2 Specialist Skill Reachability",
            "Specialist skills are not mandatory workflow phases.",
            "route, TODOs, and downstream handoff must still be recorded",
            "mandatory context packet",
            "identity and authority",
            "workflow governance",
            "task truth",
            "user intent and acceptance target",
            "scoped repo context",
            "collaboration boundary",
            "`AGENTS.md` and the assigned role card are mandatory inputs",
            "Every user request must enter the standard worktree flow before any substantive handling begins",
            "Read-only professional/domain questions must be dispatched to the matching bounded professional role slice",
            "The task/worktree decision and the professional-slice decision are intentionally decoupled",
            default_runtime_marker,
            f"shorthand: `{default_shorthand}`",
            "Any non-default subagent model or reasoning effort must be recorded in the slice contract",
        ],
    ),
    (
        root / ".agents/skills/default-workflow-bootstrap/SKILL.md",
        [
            "## Repository State Impact",
            "## Isolation Decision",
            "## Task Truth",
            "## Routed Next Phase",
            ".pm` execution log (mandatory)",
            "cannot replace the `.pm` execution log for task truth",
            "./scripts/new-task-worktree.sh",
            "./.agents/skills/repo-owned-workflow-router/SKILL.md",
            "read-only/chat-only professional judgment",
            "Do force this bootstrap onto chat-only or read-only requests",
            "Do not treat read-only professional/domain questions as TPM-owned conclusions",
        ],
    ),
    (
        root / "AGENTS.md",
        [
            "default-workflow-bootstrap",
            f"确认标准 task worktree / {bt}.pm{bt} task / owner role 真值",
            f"{bt}tpm{bt} 主 Agent + 专业角色 subagents",
            "TPM 的 TODO decomposition",
            "mandatory context packet",
            "必须先写入 `.pm/tasks/<TASK-UID>.execution.md`",
            "formal sink",
            f"{bt}liveops_community{bt} 必须参与至少一个 slice",
            "requesting-repo-owned-review/SKILL.md",
            "只读专业判断分流",
            "纯文件存在性、路径查找、命令输出复述",
            "任何用户请求第一步都必须创建或进入标准 task worktree",
            "只读专业 slice 的 contract、证据和 sink 必须写入",
            "subagent 默认模型",
            "Default subagent runtime",
        ],
    ),
    (
        root / ".agents/roles/tpm.md",
        [
            "# Role: tpm",
            "TPM 只做 workflow coordination / integration",
            "默认由 `tpm` 作为新仓库变更任务的主 Agent 和 canonical workflow owner",
            "每个用户请求必须先创建或进入标准 task worktree",
            "专业角色以 subagent 形式提供切片工作",
            "不得用 TPM 自己的判断替代专业 subagent 结论",
            "Default subagent runtime",
            "派工前必须把当前 TODO",
            "mandatory context packet",
            "workflow source-of-truth",
            "mandatory `.pm` execution-log sink",
            "./scripts/pm/workflow-report.sh --phase start|close|review --role tpm",
        ],
    ),
    (
        root / ".pm/registry/roles.yaml",
        [
            "role_name: tpm",
            "memory_active_path: .pm/roles/tpm/memory/active.yaml",
            "done_path: .pm/roles/tpm/backlog/done.yaml",
        ],
    ),
    (
        root / ".agents/roles/templates/handoff-brief.md",
        [
            "- Write Scope:",
            "- Return Contract:",
            "- Formal Sink / Writeback Surface:",
            "- Integration Owner:",
        ],
    ),
    (
        root / ".agents/roles/templates/handoff-detailed.md",
        [
            "- Write Scope:",
            "- Return Contract:",
            "- Formal Sink / Writeback Surface:",
            "- Integration Owner:",
            "- Integration Order:",
        ],
    ),
    (
        root / ".agents/roles/templates/subagent-slice-card.md",
        [
            "- role:",
            "- model configuration:",
            "`Default subagent runtime` by default",
            "- mandatory context packet:",
            "identity and authority:",
            "workflow governance:",
            "task truth:",
            "user intent:",
            "scoped repo context:",
            "collaboration boundary:",
            "- context exemption:",
            "除窄范围只读 explorer 且写明豁免原因外",
        ],
    ),
    (
        root / "scripts/pm/pm_store.py",
        [
            '"tpm": "TPM"',
        ],
    ),
    (
        root / "scripts/pm/pm_store_reporting.py",
        [
            'ORCHESTRATOR_ROLES = {"producer_system_designer", "tpm"}',
            "role in ORCHESTRATOR_ROLES",
            "signal_role_filter = None if (phase == \"review\" and role in ORCHESTRATOR_ROLES) else role",
        ],
    ),
    (
        root / ".agents/skills/requesting-repo-owned-review/SKILL.md",
        [
            "Pre-PR local role review is required before PR creation",
            "findings",
            "no_findings",
            "residual_risk",
        ],
    ),
    (
        root / ".agents/skills/repo-owned-workflow-router/SKILL.md",
        [
            "## Subagent Slice Plan (If Needed)",
            "## Specialist Skills Considered",
            "- role:",
            "- slice type:",
            "- model configuration:",
            "`Default subagent runtime` by default",
            "- mandatory context packet:",
            "identity and authority:",
            "workflow governance:",
            "task truth:",
            "user intent:",
            "scoped repo context:",
            "collaboration boundary:",
            "- write scope:",
            "- return contract:",
            "- formal sink / writeback surface:",
            ".pm/tasks/<TASK-UID>.execution.md` (mandatory)",
            "- integration owner:",
            "- integration order:",
            "- context exemption:",
            "Do not treat specialist domain skills as mandatory default workflow phases",
            "Do not dispatch implementation, verification, review, or specialist subagents without `AGENTS.md`",
            "Already-bound read-only professional/domain judgment",
            "pure fact lookup",
        ],
    ),
    (
        root / ".agents/skills/README.md",
        [
            "Canonical phase mapping lives in `doc/engineering/workflow/source-of-truth.md#11-skill-map-by-phase`",
            ".agents/skills/systematic-debugging/SKILL.md",
            ".agents/skills/receiving-code-review/SKILL.md",
            ".agents/skills/writing-repo-owned-skills/SKILL.md",
            "Specialist skills are domain-triggered through TPM routing",
            "只读/聊天请求也默认进入 task/worktree bootstrap",
        ],
    ),
    (
        root / ".agents/skills/executing-project-tasks/SKILL.md",
        [
            "plan-gap review",
            ".pm/tasks/<TASK-UID>.execution.md",
            "Do not create a second planning system outside",
        ],
    ),
    (
        root / ".agents/skills/systematic-debugging/SKILL.md",
        [
            "Reproduce the failure.",
            "narrowing the failure surface",
            "Patch the root cause, not the surface symptom.",
        ],
    ),
    (
        root / ".agents/skills/receiving-code-review/SKILL.md",
        [
            "Inventory the active comments.",
            "keeps thread resolution separate from merge readiness",
            "\"Thread resolved\" is not the same as \"PR ready to merge\".",
        ],
    ),
    (
        root / ".agents/skills/writing-repo-owned-skills/SKILL.md",
        [
            "Local skills must strengthen oasis7 repo truth",
            "not create a parallel workflow",
            "If the skill introduces or documents a helper-driven workflow",
        ],
    ),
    (
        root / ".agents/skills/prd/SKILL.md",
        [
            "## Oasis7 Workflow Binding",
            "this skill is a specialist planning surface, not a standalone workflow",
            ".pm/tasks/<TASK-UID>.execution.md",
        ],
    ),
    (
        root / ".agents/skills/game-architect/SKILL.md",
        [
            "## Oasis7 Workflow Binding",
            "not a second project workflow",
            "Architecture documents may supplement `prd.md`, `project.md`, and handoff truth",
        ],
    ),
]

surfaces: list[dict[str, object]] = []
for path, markers in checks:
    text = path.read_text(encoding="utf-8")
    missing = [marker for marker in markers if marker not in text]
    if missing:
        raise SystemExit(f"workflow-behavior-eval: missing contract markers in {path}: {missing}")
    surfaces.append(
        {
            "path": str(path.relative_to(root)),
            "required_markers": markers,
            "status": "ok",
        }
    )

print(
    json.dumps(
        {
            "status": "ok",
            "surfaces": surfaces,
            "default_runtime": {
                "model": default_model,
                "reasoning_effort": default_reasoning,
                "shorthand": default_shorthand,
            },
        },
        ensure_ascii=False,
    )
)
PY


python3 - "$ROOT_DIR" > "$ROUTING_SCENARIOS_JSON_FILE" <<'PY'
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

root = Path(sys.argv[1])

surfaces = {
    "AGENTS.md": (root / "AGENTS.md").read_text(encoding="utf-8"),
    ".agents/skills/default-workflow-bootstrap/SKILL.md": (
        root / ".agents/skills/default-workflow-bootstrap/SKILL.md"
    ).read_text(encoding="utf-8"),
    ".agents/skills/repo-owned-workflow-router/SKILL.md": (
        root / ".agents/skills/repo-owned-workflow-router/SKILL.md"
    ).read_text(encoding="utf-8"),
    "doc/engineering/workflow/source-of-truth.md": (
        root / "doc/engineering/workflow/source-of-truth.md"
    ).read_text(encoding="utf-8"),
    ".agents/skills/bounded-brainstorming/SKILL.md": (
        root / ".agents/skills/bounded-brainstorming/SKILL.md"
    ).read_text(encoding="utf-8"),
    ".agents/skills/tdd-test-writer/SKILL.md": (
        root / ".agents/skills/tdd-test-writer/SKILL.md"
    ).read_text(encoding="utf-8"),
    ".agents/skills/verification-before-completion/SKILL.md": (
        root / ".agents/skills/verification-before-completion/SKILL.md"
    ).read_text(encoding="utf-8"),
    ".agents/skills/finishing-a-development-branch/SKILL.md": (
        root / ".agents/skills/finishing-a-development-branch/SKILL.md"
    ).read_text(encoding="utf-8"),
    ".agents/skills/executing-project-tasks/SKILL.md": (
        root / ".agents/skills/executing-project-tasks/SKILL.md"
    ).read_text(encoding="utf-8"),
    ".agents/skills/systematic-debugging/SKILL.md": (
        root / ".agents/skills/systematic-debugging/SKILL.md"
    ).read_text(encoding="utf-8"),
    ".agents/skills/requesting-repo-owned-review/SKILL.md": (
        root / ".agents/skills/requesting-repo-owned-review/SKILL.md"
    ).read_text(encoding="utf-8"),
    ".agents/skills/receiving-code-review/SKILL.md": (
        root / ".agents/skills/receiving-code-review/SKILL.md"
    ).read_text(encoding="utf-8"),
    ".agents/skills/writing-repo-owned-skills/SKILL.md": (
        root / ".agents/skills/writing-repo-owned-skills/SKILL.md"
    ).read_text(encoding="utf-8"),
    ".agents/skills/prd/SKILL.md": (
        root / ".agents/skills/prd/SKILL.md"
    ).read_text(encoding="utf-8"),
    ".agents/skills/game-architect/SKILL.md": (
        root / ".agents/skills/game-architect/SKILL.md"
    ).read_text(encoding="utf-8"),
}
default_runtime_match = re.search(
    r"Default subagent runtime is `([^`]+)` with `reasoning_effort=([^`]+)` "
    r"\(shorthand: `([^`]+)`\)",
    surfaces["doc/engineering/workflow/source-of-truth.md"],
)
if not default_runtime_match:
    raise SystemExit("workflow-behavior-eval: source-of-truth missing parseable Default subagent runtime policy")
default_model, default_reasoning, default_shorthand = default_runtime_match.groups()
default_runtime_marker = (
    f"Default subagent runtime is `{default_model}` with "
    f"`reasoning_effort={default_reasoning}`"
)

scenarios = [
    {
        "id": "read_only_request_requires_task_bootstrap",
        "expected_route": "default-workflow-bootstrap -> task truth before direct answer",
        "surface": ".agents/skills/default-workflow-bootstrap/SKILL.md",
        "required_markers": [
            "read-only/chat-only pure fact lookup: requires standard worktree + `.pm` task truth before direct answer",
            "Do force this bootstrap onto chat-only or read-only requests, even when they do not change repository state.",
        ],
    },
    {
        "id": "read_only_professional_judgment_uses_role_slice_after_task_bootstrap",
        "expected_route": "default-workflow-bootstrap -> task truth -> matching professional role slice",
        "surface": "doc/engineering/workflow/source-of-truth.md",
        "required_markers": [
            "Every user request must enter the standard worktree flow before any substantive handling begins",
            "Read-only/chat-only requests still split by judgment type after task truth exists:",
            "Read-only professional/domain questions must be dispatched to the matching bounded professional role slice",
            "Such read-only professional slices require the same `.pm` task and canonical task worktree as any other request.",
            "Their required sink is `.pm/tasks/<TASK-UID>.execution.md`",
            "TPM may gather raw files, commands, or repo context before dispatch only after bootstrap",
        ],
    },
    {
        "id": "read_only_professional_question_enters_router_after_bootstrap",
        "expected_route": "bootstrap first; router applies after task truth",
        "surface": ".agents/skills/repo-owned-workflow-router/SKILL.md",
        "required_markers": [
            "Read-only/chat-only requests enter this router after `default-workflow-bootstrap` has established task truth.",
            "Already-bound read-only professional/domain judgment",
            "read-only professional/domain judgments must already be task-bound",
            "Unbound read-only professional questions are invalid under the always-bootstrap workflow",
            "record the slice contract in `.pm`",
        ],
    },
    {
        "id": "repository_changing_request_requires_task_truth_before_router",
        "expected_route": "default-workflow-bootstrap -> task truth -> repo-owned-workflow-router",
        "surface": ".agents/skills/default-workflow-bootstrap/SKILL.md",
        "required_markers": [
            "repository-changing: requires standard worktree + `.pm` task truth before edits",
            "choose `tpm` as the default workflow owner role unless an existing bound task already has a valid owner",
            "professional work still requires matching bounded subagent slices",
            "create a dedicated worktree unless the user explicitly authorized reuse",
            "Once task truth exists, hand off to `repo-owned-workflow-router`.",
        ],
    },
    {
        "id": "clear_implementation_skips_brainstorming",
        "expected_route": "repo-owned-workflow-router -> executing-project-tasks",
        "surface": ".agents/skills/bounded-brainstorming/SKILL.md",
        "required_markers": [
            "This is an optional pre-implementation layer, not a universal gate.",
            "Do not use this skill when:",
            "the user already gave a concrete implementation task and the scope is clear enough to start",
        ],
    },
    {
        "id": "ambiguous_or_option_heavy_work_uses_bounded_brainstorming",
        "expected_route": "repo-owned-workflow-router -> bounded-brainstorming -> execution",
        "surface": ".agents/skills/repo-owned-workflow-router/SKILL.md",
        "required_markers": [
            "Use when direction is still fuzzy, scope is too large, or the problem is inherently option-heavy or visual.",
            "Do not route into `bounded-brainstorming` if the task is already implementation-ready.",
        ],
    },
    {
        "id": "docs_governance_skips_tdd_red_gate",
        "expected_route": "repo-owned-workflow-router -> executing-project-tasks without TDD RED",
        "surface": ".agents/skills/tdd-test-writer/SKILL.md",
        "required_markers": [
            "Do not treat this skill as a universal gate for:",
            "documentation / governance / planning-only tasks",
            "When you skip RED phase in oasis7, record the skip reason",
        ],
    },
    {
        "id": "behavior_change_with_stable_harness_uses_tdd_red_gate",
        "expected_route": "repo-owned-workflow-router -> tdd-test-writer -> execution",
        "surface": ".agents/skills/tdd-test-writer/SKILL.md",
        "required_markers": [
            "the task changes product, runtime, API, or UI behavior",
            "there is a stable automated test surface for that behavior",
            "a narrow RED command can be run locally in the current task worktree",
        ],
    },
    {
        "id": "execution_truth_ready_routes_to_executing_project_tasks",
        "expected_route": "repo-owned-workflow-router -> executing-project-tasks",
        "surface": ".agents/skills/executing-project-tasks/SKILL.md",
        "required_markers": [
            "the task already has written scope in `prd.md`, `project.md`, a handoff, or `.pm/tasks/<TASK-UID>.yaml`",
            "Run a brief plan-gap review before editing",
            "Do not create a second planning system outside `prd.md` / `project.md` / `.pm`.",
        ],
    },
    {
        "id": "observed_failure_routes_to_systematic_debugging",
        "expected_route": "repo-owned-workflow-router -> systematic-debugging before speculative fixes",
        "surface": ".agents/skills/systematic-debugging/SKILL.md",
        "required_markers": [
            "Reproduce the failure.",
            "Narrow the scope:",
            "Patch the root cause, not the surface symptom.",
        ],
    },
    {
        "id": "pre_pr_requires_repo_owned_role_review",
        "expected_route": "requesting-repo-owned-review -> prepare-task-pr -> GitHub PR watch/fix/merge",
        "surface": ".agents/skills/requesting-repo-owned-review/SKILL.md",
        "required_markers": [
            "a branch is about to create a PR",
            "a major feature or workflow helper just landed locally",
            "Pre-PR Local Role Review: passed",
            "Formal Sink: <execution log | PR evidence | handoff>",
        ],
    },
    {
        "id": "subagent_dispatch_is_conditional_and_bounded",
        "expected_route": "TPM coordinates bounded professional role subagent slices",
        "surface": "AGENTS.md",
        "required_markers": [
            "其他专业角色必须以 subagent slice 形式参与",
            "TPM 的 TODO decomposition、subagent slice contracts、mandatory context packet 和 integration order 必须先写入 `.pm/tasks/<TASK-UID>.execution.md`",
            "其他 formal sink 只能补充，不能替代 task execution log",
        ],
    },
    {
        "id": "subagent_context_packet_is_mandatory_before_dispatch",
        "expected_route": "TPM supplies identity, governance, task truth, user intent, repo context, and collaboration boundaries",
        "surface": "doc/engineering/workflow/source-of-truth.md",
        "required_markers": [
            "The mandatory context packet must include:",
            "identity and authority",
            "workflow governance",
            "task truth",
            "user intent and acceptance target",
            "scoped repo context",
            "collaboration boundary",
            "`AGENTS.md` and the assigned role card are mandatory inputs",
        ],
    },
    {
        "id": "subagent_default_model_is_recorded",
        "expected_route": f"TPM records {default_shorthand} as the source-of-truth default subagent model configuration",
        "surface": "doc/engineering/workflow/source-of-truth.md",
        "required_markers": [
            default_runtime_marker,
            f"shorthand: `{default_shorthand}`",
            "Any non-default subagent model or reasoning effort must be recorded in the slice contract",
        ],
    },
    {
        "id": "workflow_skill_map_covers_core_and_recovery_surfaces",
        "expected_route": "source-of-truth maps phases to required, conditional, optional, and review/debug skills",
        "surface": "doc/engineering/workflow/source-of-truth.md",
        "required_markers": [
            "### 1.1 Skill Map by Phase",
            "`systematic-debugging`",
            "`receiving-code-review`",
            "`writing-repo-owned-skills`",
            "Requiredness",
            "Formal evidence",
        ],
    },
    {
        "id": "specialist_skills_are_domain_triggered_not_default_phases",
        "expected_route": "TPM routes specialist skills only when task domain matches",
        "surface": "doc/engineering/workflow/source-of-truth.md",
        "required_markers": [
            "### 1.2 Specialist Skill Reachability",
            "Specialist skills are not mandatory workflow phases.",
            "If a specialist skill is used, TPM must still bind it to the same owner",
            "the specialist role owns the professional conclusion",
        ],
    },
    {
        "id": "tpm_is_coordination_only_not_professional_execution",
        "expected_route": "TPM coordinates while professional role slices own domain judgments",
        "surface": "doc/engineering/workflow/source-of-truth.md",
        "required_markers": [
            "TPM is not a professional execution role.",
            "TPM must not be the source of domain/professional analysis",
            "Professional/domain work must be done by the matching bounded subagent slice.",
            "TPM read-only exploration is allowed only to gather routing context",
            "Professional conclusions must be traceable to subagent artifacts",
        ],
    },
    {
        "id": "specialist_planning_skills_bind_back_to_tpm_pm_truth",
        "expected_route": "prd/game-architect may supplement planning but not replace TPM/.pm task truth",
        "surface": ".agents/skills/prd/SKILL.md",
        "required_markers": [
            "this skill is a specialist planning surface, not a standalone workflow",
            "Record the PRD route, TODOs, and downstream handoff in `.pm/tasks/<TASK-UID>.execution.md`.",
            "Do not treat PRD-only output as implementation-ready",
        ],
    },
    {
        "id": "game_architect_binds_back_to_tpm_pm_truth",
        "expected_route": "game-architect docs remain supplemental architecture planning",
        "surface": ".agents/skills/game-architect/SKILL.md",
        "required_markers": [
            "this skill is a specialist architecture-planning surface, not a second project workflow",
            "record the route, TODOs, and downstream execution handoff in `.pm/tasks/<TASK-UID>.execution.md`",
            "Implementation must still route through `repo-owned-workflow-router` and `executing-project-tasks`",
        ],
    },
    {
        "id": "github_review_feedback_routes_to_receiving_code_review",
        "expected_route": "receiving-code-review -> fix evidence -> verification-before-completion",
        "surface": ".agents/skills/receiving-code-review/SKILL.md",
        "required_markers": [
            "Inventory the active comments.",
            "Verify the comment against repo truth before editing.",
            "\"Thread resolved\" is not the same as \"PR ready to merge\".",
        ],
    },
    {
        "id": "local_skill_edit_routes_to_writing_repo_owned_skills",
        "expected_route": "writing-repo-owned-skills governs local skill surface edits",
        "surface": ".agents/skills/writing-repo-owned-skills/SKILL.md",
        "required_markers": [
            "Local skills must strengthen oasis7 repo truth, not create a parallel workflow.",
            "If the content would be better owned by `AGENTS.md`, `prd.md`, `project.md`, a handoff template, or a script check",
            "If the skill introduces or documents a helper-driven workflow, also run at least one representative command or check tied to that workflow.",
        ],
    },
    {
        "id": "tpm_planning_requires_task_execution_log_before_dispatch",
        "expected_route": "TPM records TODO decomposition and slice contracts before delegated execution",
        "surface": ".agents/skills/repo-owned-workflow-router/SKILL.md",
        "required_markers": [
            "TPM TODO decomposition and subagent slice contracts must be recorded in `.pm/tasks/<TASK-UID>.execution.md` before delegated execution begins.",
            "Read-only/chat-only requests enter this router after `default-workflow-bootstrap` has established task truth.",
            "mandatory `.pm` execution-log sink",
            "formal docs may supplement but not replace it",
        ],
    },
    {
        "id": "completion_claim_requires_fresh_verification",
        "expected_route": "verification-before-completion before done/tests-passed/ready-for-pr claims",
        "surface": ".agents/skills/verification-before-completion/SKILL.md",
        "required_markers": [
            "Run the verification command now, read the result now, and only then make the claim.",
            "Do not use stale output, partial output, or earlier successful runs as proof.",
            "./scripts/pm/claim-ready.sh",
        ],
    },
    {
        "id": "closeout_routes_to_local_role_review_then_github_pr_review",
        "expected_route": "finishing-a-development-branch -> local role review -> prepare-task-pr -> GitHub required checks/review -> merge/cleanup",
        "surface": ".agents/skills/finishing-a-development-branch/SKILL.md",
        "required_markers": [
            "Pre-PR Local Role Review: passed",
            "./scripts/prepare-task-pr.sh --create",
            "normal_pr_ci_watch",
            "manual_packaging_ci_hold",
            "Do not stop at PR creation for normal PRs; continue watching CI/review, fix failures, merge, and clean up.",
            "Do not merge a normal PR without first checking PR comments and review threads and resolving or answering actionable items.",
            "Do not land locally unless the user explicitly asks for local landing.",
            "Do not treat review-thread resolution as merge readiness.",
        ],
    },
]

evaluated: list[dict[str, object]] = []
for scenario in scenarios:
    text = surfaces[scenario["surface"]]
    missing = [
        marker
        for marker in scenario["required_markers"]
        if marker not in text
    ]
    if missing:
        raise SystemExit(
            "workflow-behavior-eval: routing scenario "
            f"{scenario['id']} missing markers in {scenario['surface']}: {missing}"
        )
    evaluated.append(
        {
            "id": scenario["id"],
            "expected_route": scenario["expected_route"],
            "surface": scenario["surface"],
            "status": "ok",
        }
    )

print(json.dumps({"status": "ok", "scenarios": evaluated}, ensure_ascii=False))
PY


RESULT_JSON="$(python3 - "$TASK_WORKTREE_JSON_FILE" "$SUBAGENT_CONTRACT_JSON_FILE" "$REQUIRED_TIER_JSON_FILE" "$ROUTING_SCENARIOS_JSON_FILE" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

task_worktree = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
subagent_contract = json.loads(Path(sys.argv[2]).read_text(encoding="utf-8"))
required_tier = json.loads(Path(sys.argv[3]).read_text(encoding="utf-8"))
routing_scenarios = json.loads(Path(sys.argv[4]).read_text(encoding="utf-8"))
default_shorthand = subagent_contract["default_runtime"]["shorthand"]

segments = [
    {
        "id": "default_workflow_bootstrap_surface",
        "command": "python contract check over default bootstrap / AGENTS surfaces",
        "status": subagent_contract["status"],
        "evidence": {
            "surface_count": len(subagent_contract["surfaces"]),
        },
    },
    {
        "id": "task_worktree",
        "command": "./scripts/pm/new-task-worktree-bootstrap-smoke.sh --json",
        "status": "passed",
        "evidence": {
            "task_uid": task_worktree["task_uid"],
            "workflow_started": task_worktree["workflow_started"],
        },
    },
    {
        "id": "subagent_contract_surface",
        "command": "python contract check over AGENTS / handoff / router surfaces",
        "status": subagent_contract["status"],
        "evidence": {
            "surface_count": len(subagent_contract["surfaces"]),
        },
    },
    {
        "id": "routing_scenario_contract",
        "command": "python contract check over workflow routing scenario surfaces",
        "status": routing_scenarios["status"],
        "evidence": {
            "scenario_count": len(routing_scenarios["scenarios"]),
            "scenario_ids": [
                scenario["id"]
                for scenario in routing_scenarios["scenarios"]
            ],
        },
    },
    {
        "id": "closeout_and_pm_runtime",
        "command": "./scripts/pm/required-tier-smoke.sh --json",
        "status": "passed",
        "evidence": {
            "task_uid": required_tier["task_closeout"]["task_uid"],
            "claim_verification_status": required_tier["task_closeout"]["claim_verification"]["status"],
            "final_status": required_tier["task_closeout"]["final_status"],
        },
    },
    {
        "id": "completion_claim_gate",
        "command": "./scripts/pm/claim-ready.test.sh",
        "status": "passed",
        "evidence": {
            "helper": "claim-ready",
        },
    },
    {
        "id": "pr_preflight",
        "command": "./scripts/prepare-task-pr.test.sh",
        "status": "passed",
        "evidence": {
            "helper": "prepare-task-pr",
        },
    },
    {
        "id": "review_thread_closeout",
        "command": "./scripts/pr-review-thread-closeout.test.sh",
        "status": "passed",
        "evidence": {
            "helper": "pr-review-thread-closeout",
        },
    },
]

payload = {
    "workflow_path": "default-workflow-bootstrap -> new-task-worktree -> workflow-report -> repo-owned-workflow-router -> TPM coordinate/integrate only + professional role subagent dispatch -> task-closeout -> prepare-task-pr -> PR CI/comment watch/fix -> review-thread-closeout -> merge/cleanup",
    "fixture_scope": "repo-owned bootstrap/routing surface checks, isolated worktree bootstrap smoke, PM runtime smoke, and fake-gh PR helper tests",
    "expected_agent_behavior": [
        "every user request first routes through a repo-owned bootstrap surface rather than an external bootstrap",
        "bootstrap creates or enters isolated task truth before fact lookup, chat answer, professional slice dispatch, or repository writeback",
        "read-only professional/domain questions use matching role slices after task/worktree bootstrap",
        "task worktree bootstrap stays source-clean and starts the target task",
        "read-only/chat-only requests are forced through task/worktree bootstrap",
        "TPM is the default main Agent / workflow coordinator / canonical integrator only",
        "TPM does not own professional/domain conclusions; matching professional role slices do",
        "brainstorming and TDD remain conditional while professional role work is represented as bounded subagent slices",
        "subagent dispatch remains bound to owner/write-scope/return-contract/formal-sink surfaces",
        f"subagent slice contracts record {default_shorthand} as the source-of-truth default model configuration unless an override reason is present",
        "PR creation requires local involved-role subagent review evidence before GitHub PR watch/fix/merge",
        "done closeout refuses to proceed without fresh verification",
        "PR preflight stays the default GitHub PR entrypoint after local role review evidence",
        "normal PRs continue after creation into required-check/comment/mergeability watch, failure fixes, comment closeout, authorized review-approval admin merge when policy allows, merge, and cleanup; REVIEW_REQUIRED is informational and not a blocker",
        "manual packaging/release CI PRs can pause before merge only when that purpose is explicit",
        "review-thread closeout reports unresolved/resolved thread state without conflating merge readiness",
    ],
    "verification_surface": [segment["id"] for segment in segments],
    "failure_signature": [
        "default bootstrap surface disappears or no longer points every user request into repo-owned task truth",
        "routing scenarios stop requiring read-only/chat-only work to establish task truth before answer or dispatch",
        "read-only professional/domain questions collapse back into TPM-owned conclusions",
        "TPM role or registry markers disappear from role surfaces",
        "optional brainstorming, TDD, or subagent gates drift into mandatory stages",
        "task-closeout allows done closeout without verify-command",
        "subagent contract markers disappear from AGENTS or handoff/router surfaces",
        f"subagent model configuration markers disappear or no longer default to the source-of-truth runtime ({default_shorthand})",
        "repo-owned review-request surface disappears or stops requiring pre-PR local role review evidence",
        "prepare-task-pr local fixture no longer creates the expected PR command path",
        "finishing branch guidance stops distinguishing normal PR CI watch from manual packaging CI hold",
        "review-thread closeout helper stops reporting unresolved/resolved state correctly",
    ],
    "routing_scenarios": routing_scenarios["scenarios"],
    "segments": segments,
}
print(json.dumps(payload, ensure_ascii=False))
PY
)"

if [[ "$OUTPUT_JSON" == "1" ]]; then
  printf '%s\n' "$RESULT_JSON"
  exit 0
fi

python3 - "$RESULT_JSON" <<'PY'
from __future__ import annotations

import json
import sys

payload = json.loads(sys.argv[1])

print("workflow behavior eval: OK")
print(f"- workflow_path: {payload['workflow_path']}")
for segment in payload["segments"]:
    print(f"- {segment['id']}: {segment['status']} ({segment['command']})")
PY
