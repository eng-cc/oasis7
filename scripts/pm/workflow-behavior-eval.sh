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
PM_ROLE_SNAPSHOT_DIR="$TMP_DIR/pm-role-snapshot"
OASIS7_WORKFLOW_EVAL_SCRATCH="$TMP_DIR"
cleanup() {
  local status=$?
  rm -rf "$TMP_DIR"
  exit "$status"
}
trap cleanup EXIT

run_interrupt_isolated() {
  "$@"
  : "caller-survived"
}

python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" snapshot \
  --root "$ROOT_DIR" --state "$PM_ROLE_SNAPSHOT_DIR" --pathspec .pm

TASK_WORKTREE_JSON_FILE="$TMP_DIR/task-worktree.json"
SUBAGENT_CONTRACT_JSON_FILE="$TMP_DIR/subagent-contract.json"
ROUTING_SCENARIOS_JSON_FILE="$TMP_DIR/routing-scenarios.json"
CODEX_AGENT_CONFIG_JSON_FILE="$TMP_DIR/codex-agent-config.json"

if ! TOML_PYTHON="$($ROOT_DIR/scripts/pm/find-python-with-module.sh tomllib)"; then
  echo "workflow-behavior-eval: complete TOML validation requires an available Python 3.11+ stdlib tomllib interpreter; python3 may remain 3.9" >&2
  exit 1
fi
"$TOML_PYTHON" "$ROOT_DIR/scripts/pm/validate-codex-agent-config.py" \
  --root "$ROOT_DIR" > "$CODEX_AGENT_CONFIG_JSON_FILE"
"$ROOT_DIR/scripts/pm/validate-codex-agent-config.test.sh" >/dev/null
"$ROOT_DIR/scripts/pm/guard-tracked-files.test.sh" >/dev/null
"$ROOT_DIR/scripts/pm/lint.test.sh" >/dev/null

"$ROOT_DIR/scripts/pm/new-task-worktree-bootstrap-smoke.sh" --json > "$TASK_WORKTREE_JSON_FILE"
run_interrupt_isolated "$ROOT_DIR/scripts/pm/github-project-task.test.sh" >/dev/null
"$ROOT_DIR/scripts/pm/github-project-sync.test.sh" >/dev/null
"$ROOT_DIR/scripts/pm/github-project-workflow.test.sh" >/dev/null
"$ROOT_DIR/scripts/pm/github-project-retire-tasks.test.sh" >/dev/null
"$ROOT_DIR/scripts/pm/audit-pr-watch-issues.test.sh" >/dev/null
"$ROOT_DIR/scripts/pm/fallback-evidence.test.sh" >/dev/null
"$ROOT_DIR/scripts/pm/claim-ready.test.sh" >/dev/null
bash "$ROOT_DIR/scripts/pm/claim-ready-ready-pr.test.sh" >/dev/null
"$ROOT_DIR/scripts/pm/workflow-lint.test.sh" >/dev/null
"$ROOT_DIR/scripts/pm/record-pre-pr-review.test.sh" >/dev/null
"$ROOT_DIR/scripts/pm/task-closeout-transition.test.sh" >/dev/null
"$ROOT_DIR/scripts/pm/task-closeout-profile.test.sh" >/dev/null
bash "$ROOT_DIR/scripts/pm/closeout-tmpdir-portability.test.sh" >/dev/null
"$ROOT_DIR/scripts/pm/pr-lifecycle-gate.test.sh" >/dev/null
"$ROOT_DIR/scripts/pm/pr-lifecycle-trust.test.sh" >/dev/null
python3 "$ROOT_DIR/scripts/pm/tpm-workflow-driver.test.py" >/dev/null
python3 "$ROOT_DIR/scripts/pm/tpm-workflow-doc-contract.test.py" >/dev/null
python3 "$ROOT_DIR/scripts/pm/tpm-production-supervisor.test.py" >/dev/null
python3 "$ROOT_DIR/scripts/pm/terminal-transition-order.test.py" >/dev/null
"$ROOT_DIR/scripts/pm/workflow-adversarial-contract.test.sh" >/dev/null
"$ROOT_DIR/scripts/pm/pr-policy-discovery-contract.test.sh" ruleset >/dev/null
"$ROOT_DIR/scripts/pm/pr-policy-discovery-contract.test.sh" none >/dev/null
"$ROOT_DIR/scripts/pm/pr-policy-discovery-contract.test.sh" denied >/dev/null
"$ROOT_DIR/scripts/pm/pr-policy-discovery-contract.test.sh" paged >/dev/null
"$ROOT_DIR/scripts/pm/pr-policy-discovery-contract.test.sh" filtered >/dev/null
"$ROOT_DIR/scripts/pm/pr-disposition-evidence.test.sh" forged-cache >/dev/null
"$ROOT_DIR/scripts/pm/pr-disposition-evidence.test.sh" top-review >/dev/null
"$ROOT_DIR/scripts/pm/pr-disposition-evidence.test.sh" writer >/dev/null
python3 "$ROOT_DIR/scripts/pm/pr-final-trust-red.test.py" >/dev/null
OASIS7_PM_TEST_SCRATCH="$OASIS7_WORKFLOW_EVAL_SCRATCH/bootstrap" \
  "$ROOT_DIR/scripts/pm/bootstrap-immutable-request.test.sh" >/dev/null
"$ROOT_DIR/scripts/pm/post-merge-main-sync.test.sh" >/dev/null
"$ROOT_DIR/scripts/pm/post-merge-main-sync-default-cache-recovery.test.sh" >/dev/null
python3 "$ROOT_DIR/scripts/pm/recover-terminal-task-mapping.test.py" >/dev/null
"$ROOT_DIR/scripts/pm/patch-equivalence-receipt.test.sh" >/dev/null
"$ROOT_DIR/scripts/pm/post-merge-cleanup.test.sh" >/dev/null
"$ROOT_DIR/scripts/pm/post-merge-cleanup-trust.test.sh" >/dev/null
"$ROOT_DIR/scripts/pm/review-provenance-trust.test.sh" >/dev/null
"$ROOT_DIR/scripts/pm/new-task-worktree-module-validation.test.sh" >/dev/null
"$ROOT_DIR/scripts/pm/new-task-worktree-partial-bootstrap.test.sh" >/dev/null
"$ROOT_DIR/scripts/prepare-task-pr.test.sh" >/dev/null
"$ROOT_DIR/scripts/pr-review-thread-closeout.test.sh" >/dev/null
if rg -n 'fresh verification -> pre-PR|commit 前.*review|workflow-report close -> move-task|closeout -> commit' \
  "$ROOT_DIR/.pm/README.md" \
  "$ROOT_DIR/doc/engineering/prd.md" \
  "$ROOT_DIR/doc/scripts/prd.md" \
  "$ROOT_DIR/doc/engineering/workflow/source-of-truth.md"; then
  echo "workflow-behavior-eval: active workflow docs retain retired live/uncommitted closeout wording" >&2
  exit 1
fi
if rg -n 'Commit exactly this task slice|closeout -> commit' \
  "$ROOT_DIR/.agents/skills/finishing-a-development-branch/SKILL.md"; then
  echo "workflow-behavior-eval: finishing/project surfaces retain generic post-review commit order" >&2
  exit 1
fi
for marker in \
  '## Freeze-Commit Gates' \
  '## Optional Evidence-Only Commit / PR-Prep Gates' \
  '## Post-PR / Pre-Merge Gates' \
  'git diff --check <Comparison Ref>...<Source Head>' \
  'evidence-only commit' \
  'Partial remote state recovers via refresh -> audit -> retry' \
  'Post-PR checks/comments/mergeability remain separate gates'; do
  if ! rg -F "$marker" \
    "$ROOT_DIR/.agents/skills/finishing-a-development-branch/SKILL.md" >/dev/null; then
    echo "workflow-behavior-eval: missing finishing gate marker: $marker" >&2
    exit 1
  fi
done
python3 - "$ROOT_DIR/.agents/skills/finishing-a-development-branch/SKILL.md" <<'PY'
from pathlib import Path
import sys
text=Path(sys.argv[1]).read_text(encoding="utf-8")
markers=[
    "## Freeze-Commit Gates",
    "## Optional Evidence-Only Commit / PR-Prep Gates",
    "## Post-PR / Pre-Merge Gates",
]
positions=[text.index(marker) for marker in markers]
if positions != sorted(positions):
    raise SystemExit("workflow-behavior-eval: finishing checklist temporal order is invalid")
freeze, prep, post = positions
review_packet = text.index("Pre-PR local role review packet recorded after immutable verification")
purpose_decision = text.index("Record the PR purpose decision after PR creation")
post_merge = text.index("## Post-Merge Cleanup")
if not prep < review_packet < post:
    raise SystemExit("workflow-behavior-eval: pre-PR review packet is outside PR-prep gate range")
if not post < purpose_decision < post_merge:
    raise SystemExit("workflow-behavior-eval: PR purpose decision is outside post-PR/pre-merge gate range")
PY
python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" check \
  --root "$ROOT_DIR" --state "$PM_ROLE_SNAPSHOT_DIR" --pathspec .pm >/dev/null

python3 - "$ROOT_DIR" "$CODEX_AGENT_CONFIG_JSON_FILE" > "$SUBAGENT_CONTRACT_JSON_FILE" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
bt = chr(96)
source_text = (root / "doc/engineering/workflow/source-of-truth.md").read_text(encoding="utf-8")
agent_config = json.loads(Path(sys.argv[2]).read_text(encoding="utf-8"))
if agent_config.get("status") != "ok":
    raise SystemExit("workflow-behavior-eval: Codex agent config validation did not pass")
runtime_policy_marker = "The repository does not pin a default subagent model or reasoning effort in `.codex/config.toml`"

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
            "Only `.agents/skills/*/SKILL.md` entries are default skill entrypoints.",
            "non-default specialist library material for professional method skills",
            "route, work items, and downstream handoff must still be recorded",
            "mandatory context checklist",
            "minimal, HEAD-bound task packet",
            "immutable machine-readable snapshot",
            "one expected role/slice batch",
            "bounded-command-output.py",
            "Full-thread/full-history delivery is an explicit escalation",
            "do not copy the full parent conversation by default",
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
            runtime_policy_marker,
            "Every slice contract must record one explicit runtime outcome",
            "A requested value must never be presented as the observed actual runtime without evidence",
            "as the default large-module marker",
            "`game-strategy`",
            "`visualization`",
            "`chain-world-state-substrate`",
            "Do not create a separate parent/planning surface",
            "Reflection signal: use `capture-todo.sh` for an uncommitted cross-task idea;",
            "### 1.2.3 GitHub Project-Backed PM Contract",
            "GitHub Issues + GitHub Project are the authoritative project-management",
            "Task UID` remains the stable internal identity",
            "github-project-workflow.sh ... sync",
            "github-project-workflow.sh ... audit",
            "github-project-workflow.sh ... step3-gate",
            "fallback-evidence.sh",
            "capture one coherent full-`.pm` snapshot",
            "complete `.pm` filesystem path set",
            "exact Git index mode/OID/stage/path records separately",
            "Ordinary audit,",
            "readiness verification, and `claim-ready.sh` are read-only",
            "cached title or acceptance drift",
        ],
    ),
    (
        root / ".pm/README.md",
        [
            "GitHub Project-Backed PM Operations",
            "GitHub Project 是 active work queue",
            "github-project-workflow.sh",
            "sync",
            "audit",
            "step3-gate",
            "GitHub issue number / Project item id 只是外部对象句柄",
        ],
    ),
    (
        root / "scripts/pm/github-project-workflow.py",
        [
            "GitHub Project-backed oasis7 PM workflow adapter.",
            "def command_sync",
            "def command_audit",
            "def command_step3_gate",
            "duplicate GitHub Project item",
        ],
    ),
    (
        root / "scripts/pm/github-project-workflow.sh",
        [
            "github-project-workflow.py",
        ],
    ),
    (
        root / ".agents/skills/default-workflow-bootstrap/SKILL.md",
        [
            "## Repository State Impact",
            "## Isolation Decision",
            "## Task Truth",
            "## Routed Next Phase",
            "GitHub issue evidence comment (mandatory)",
            "cannot replace the GitHub-backed task evidence sink for task truth",
            "./scripts/new-task-worktree.sh",
            "./.agents/skills/repo-owned-workflow-router/SKILL.md",
            "read-only/chat-only professional judgment",
            "Do force this bootstrap onto chat-only or read-only requests",
            "Do not treat read-only professional/domain questions as TPM-owned conclusions",
            "Already-bound micro-loop caveat:",
            "Learning Intake / Loop Closeout",
            "question or observation, evidence path or command",
        ],
    ),
    (
        root / "AGENTS.md",
        [
            "default-workflow-bootstrap",
            f"确认标准 task worktree / GitHub Project-backed task truth / owner role 真值",
            f"{bt}tpm{bt} 主 Agent + 专业角色 subagents",
            "TPM 的 TODO decomposition",
            "mandatory context checklist",
            "必须先写入 GitHub task issue evidence comments",
            "formal sink",
            f"{bt}liveops_community{bt} 必须参与至少一个 slice",
            "requesting-repo-owned-review/SKILL.md",
            "只读专业判断分流",
            "纯文件存在性、路径查找、命令输出复述",
            "任何用户请求第一步都必须创建或进入标准 task worktree",
            "subagent slice contracts",
            "Subagent runtime",
            "inherit current parent selection",
            "adapter inactive on this surface",
        ],
    ),
    (
        root / ".agents/roles/tpm.md",
        [
            "# Role: tpm",
            "TPM 只做 workflow coordination / integration",
            "默认由 `tpm` 作为新仓库变更任务的主 Agent、workflow coordinator / integrator",
            "每个用户请求必须先创建或进入标准 task worktree",
            "专业角色以 subagent 形式提供切片工作",
            "不得用 TPM 自己的判断替代专业 subagent 结论",
            "仓库不在 `.codex/config.toml` 固定",
            "adapter-backed",
            "adapter inactive on this surface",
            "派工前必须把当前 TODO",
            "mandatory context checklist",
            "workflow source-of-truth",
            "GitHub task issue evidence sink",
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
            "`inherit current parent selection` by default",
            "adapter inactive on this surface",
            "- mandatory context checklist:",
            "identity and authority:",
            "workflow governance:",
            "task truth:",
            "user intent:",
            "scoped repo context:",
            "collaboration boundary:",
            "- context exemption:",
            "除窄范围只读 explorer 且写明豁免原因外",
            "## Example (copy/paste)",
            "intended model configuration: `inherit current parent selection`",
            "actual dispatched model/reasoning: `inherited/unverified` because this dispatch surface cannot report the inherited runtime",
            "role activation: `message-assigned; adapter inactive on this surface`",
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
        root / "scripts/pm/find-python-with-module.sh",
        [
            "for generic_name in python python3",
            'for candidate in "$path_dir"/python*',
            "importlib.import_module",
        ],
    ),
    (
        root / "scripts/pm/guard-tracked-files.py",
        [
            'choices=("snapshot", "check")',
            '"git", "ls-files", "-z"',
            "tracked projection drift",
            "new index projection path",
            "removed index projection path",
            "new untracked projection artifact",
        ],
    ),
    (
        root / "scripts/pm/validate-codex-agent-config.py",
        [
            "load_renderer",
            "deterministic rendering",
            '"status": "registry_strict_loaded"',
            '"adapter_native_parse": "not_run"',
            "threading.Thread",
            "CODEX_AGENT_CONFIG_PROBE_TIMEOUT_SECONDS",
            "bounded stderr capture",
        ],
    ),
    (
        root / "scripts/pm/lint.sh",
        [
            "PM_LINT_ROOT",
            'export PM_ROOT_DIR="$PM_LINT_ROOT"',
            "All PM validation below reads one coherent snapshot epoch",
            "tree-manifest.py",
            "source .pm changed during snapshot attempt",
            "PYTHONPYCACHEPREFIX",
        ],
    ),
    (
        root / "doc/engineering/prd.md",
        [
            "完整专业角色 roster",
            "`gameplay_designer`",
            "`game_visual_interaction_designer`",
            "`blockchain_ops_engineer`",
            "`repository_health_engineer`",
        ],
    ),
    (
        root / ".agents/skills/requesting-repo-owned-review/SKILL.md",
        [
            "Pre-PR local role review is required after the draft candidate has same-head CI evidence and before promotion",
            "findings",
            "no_findings",
            "residual_risk",
            "include `agent_engineer` only when in-world Agent perception",
            "repository Codex config/adapter projection/validation contracts",
            "for `.codex/agents/<role>.toml`, require `repository_health_engineer`, `qa_engineer`, and the matching canonical `<role>`",
        ],
    ),
    (
        root / ".agents/skills/repo-owned-workflow-router/SKILL.md",
        [
            "## Subagent Slice Plan (If Needed)",
            "Do not treat specialist domain skills as mandatory default workflow phases",
            "- role:",
            "- slice type:",
            "- model configuration:",
            "`inherit current parent selection` by default",
            "adapter inactive on this surface",
            "- mandatory context checklist:",
            "identity and authority:",
            "workflow governance:",
            "task truth:",
            "user intent:",
            "scoped repo context:",
            "collaboration boundary:",
            "- write scope:",
            "- return contract:",
            "- formal sink / writeback surface:",
            "GitHub issue evidence comment (mandatory)",
            "- integration owner:",
            "- integration order:",
            "- context exemption:",
            "Do not treat specialist domain skills as mandatory default workflow phases",
            "Do not dispatch implementation, verification, review, or specialist subagents without `AGENTS.md`",
            "Already-bound read-only professional/domain judgment",
            "Pure fact lookup",
        ],
    ),
    (
        root / ".agents/skills/README.md",
        [
            "Canonical phase mapping lives in `doc/engineering/workflow/source-of-truth.md#11-skill-map-by-phase`",
            ".agents/skills/systematic-debugging/SKILL.md",
            ".agents/skills/receiving-code-review/SKILL.md",
            ".agents/skills/writing-repo-owned-skills/SKILL.md",
            "Root `skills/` is the non-default specialist library surface",
            "Specialist skills are domain-triggered through TPM routing",
            "Non-default specialist library material under root `skills/` is opt-in",
            "只读/聊天请求也默认进入 task/worktree bootstrap",
        ],
    ),
    (
        root / ".agents/skills/executing-project-tasks/SKILL.md",
        [
            "plan-gap review",
            "GitHub task issue evidence comments",
            "Keep mutable task planning only in GitHub-backed task truth",
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
        root / "skills/prd/SKILL.md",
        [
            "## Oasis7 Workflow Binding",
            "this skill is a specialist planning surface, not a standalone workflow",
            "GitHub task issue evidence comments",
        ],
    ),
    (
        root / "skills/game-architect/SKILL.md",
        [
            "## Oasis7 Workflow Binding",
            "not a second project workflow",
            "Architecture documents may supplement durable PRD/design and handoff truth",
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
            "runtime_policy": agent_config["runtime_policy"],
            "codex_agent_config": agent_config,
        },
        ensure_ascii=False,
    )
)
PY


python3 - "$ROOT_DIR" "$CODEX_AGENT_CONFIG_JSON_FILE" > "$ROUTING_SCENARIOS_JSON_FILE" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

root = Path(sys.argv[1])

surfaces = {
    ".codex/config.toml": (root / ".codex/config.toml").read_text(encoding="utf-8"),
    "AGENTS.md": (root / "AGENTS.md").read_text(encoding="utf-8"),
    ".agents/skills/default-workflow-bootstrap/SKILL.md": (
        root / ".agents/skills/default-workflow-bootstrap/SKILL.md"
    ).read_text(encoding="utf-8"),
    ".agents/skills/repo-owned-workflow-router/SKILL.md": (
        root / ".agents/skills/repo-owned-workflow-router/SKILL.md"
    ).read_text(encoding="utf-8"),
    ".agents/skills/README.md": (
        root / ".agents/skills/README.md"
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
    "skills/prd/SKILL.md": (
        root / "skills/prd/SKILL.md"
    ).read_text(encoding="utf-8"),
    "skills/game-architect/SKILL.md": (
        root / "skills/game-architect/SKILL.md"
    ).read_text(encoding="utf-8"),
    "scripts/pm/capture-todo.sh": (
        root / "scripts/pm/capture-todo.sh"
    ).read_text(encoding="utf-8"),
}
agent_config = json.loads(Path(sys.argv[2]).read_text(encoding="utf-8"))
runtime_policy = agent_config["runtime_policy"]

scenarios = [
    {
        "id": "read_only_request_requires_task_bootstrap",
        "expected_route": "default-workflow-bootstrap -> task truth before direct answer",
        "surface": ".agents/skills/default-workflow-bootstrap/SKILL.md",
        "required_markers": [
            "read-only/chat-only pure fact lookup: requires standard worktree + GitHub Project-backed task truth before direct answer",
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
            "Such read-only professional slices require the same GitHub-backed task and canonical task worktree as any other request.",
            "Their required sink is GitHub task issue evidence comments",
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
            "Record the slice contract in the GitHub issue evidence comment sink",
        ],
    },
    {
        "id": "already_bound_micro_loop_uses_minimal_learning_record",
        "expected_route": "bound task -> minimal learning-intake record without repeating full bootstrap/router packets",
        "surface": "doc/engineering/workflow/source-of-truth.md",
        "required_markers": [
            "### 1.2.2 Learning Intake / Loop Closeout",
            "Once a request is already inside the bound task/worktree",
            "records a short GitHub issue evidence note only when the fact materially",
            "append the follow-up evidence to the current",
            "unless it changes owner, scope, or PR chain",
            "same-task evidence stays in the bound GitHub task",
        ],
    },
    {
        "id": "capture_todo_defaults_to_reflection_without_task_creation",
        "expected_route": "learning intake -> reflection signal by default; candidate task only with explicit --create-task",
        "surface": "scripts/pm/capture-todo.sh",
        "required_markers": [
            "Capture a lightweight pre-task TODO as a GitHub-backed reflection intake issue.",
            "By default this only",
            "creates a GitHub-backed reflection intake issue and does not create a candidate task unless --create-task is selected.",
            "--create-task",
            "--source-type reflection",
        ],
    },
    {
        "id": "working_memory_supplements_not_replaces_issue_evidence",
        "expected_route": "learning intake -> task-scoped working_memory may supplement GitHub task issue evidence only",
        "surface": "doc/engineering/workflow/source-of-truth.md",
        "required_markers": [
            "task-scoped `working_memory`",
            "Execution evidence is recorded in GitHub task issue evidence comments.",
            "Historical project docs, handoff files, signals, memory, and PR evidence may supplement GitHub task issue evidence comments",
            "but they do not replace them for task execution truth.",
            "remain repo-local unless a later source-of-truth",
        ],
    },
    {
        "id": "repository_changing_request_requires_task_truth_before_router",
        "expected_route": "default-workflow-bootstrap -> task truth -> repo-owned-workflow-router",
        "surface": ".agents/skills/default-workflow-bootstrap/SKILL.md",
        "required_markers": [
            "repository-changing: requires standard worktree + GitHub Project-backed task truth before edits",
            "TPM is the default coordinator and continuation owner",
            "matching bounded subagent slices.",
            "dedicated worktree unless the user explicitly authorized reuse",
            "Once task truth exists, hand off to `repo-owned-workflow-router`",
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
            "Ambiguous, option-heavy, or materially visual scope -> optional `bounded-brainstorming`; do not route there when implementation-ready.",
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
            "the task already has written scope in a PRD/design, a handoff, or GitHub-backed task truth",
            "Run a brief plan-gap review before editing",
            "Keep mutable task planning only in GitHub-backed task truth",
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
            "Use after implementation freeze and before the canonical Pre-PR Ready gate.",
            "Record the canonical packet with `record-pre-pr-review.sh --review-plan <plan>` in the GitHub task issue.",
            "Require each role to return `findings` or `no_findings`, plus `residual_risk`",
            "Require trusted runtime attestation only when operating the future unattended supervisor.",
            "Record plan/batch paths and digests in GitHub task issue evidence comments.",
        ],
    },
    {
        "id": "subagent_dispatch_is_conditional_and_bounded",
        "expected_route": "TPM coordinates bounded professional role subagent slices",
        "surface": "AGENTS.md",
        "required_markers": [
            "其他专业角色必须以 subagent slice 形式参与",
            "TPM 的 TODO decomposition、subagent slice contracts、mandatory context checklist 和 integration order 必须先写入 GitHub task issue evidence comments",
            "其他 formal sink 只能补充，不能替代正式 task evidence sink",
        ],
    },
    {
        "id": "subagent_context_checklist_is_mandatory_before_dispatch",
        "expected_route": "TPM supplies identity, governance, task truth, user intent, repo context, and collaboration boundaries",
        "surface": "doc/engineering/workflow/source-of-truth.md",
        "required_markers": [
            "The mandatory context checklist must include:",
            "Context delivery defaults to a minimal, HEAD-bound task packet",
            "Full-thread/full-history delivery is an explicit escalation",
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
        "id": "subagent_inherited_runtime_is_recorded",
        "expected_route": f"TPM records the capability-aware runtime policy: {runtime_policy}",
        "surface": "doc/engineering/workflow/source-of-truth.md",
        "required_markers": [
            ".codex/config.toml",
            "intended model: inherit current parent selection",
            "actual model: inherited/unverified",
            "Every slice contract must record one explicit runtime outcome",
            "A requested value must never be presented as the observed actual runtime without evidence",
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
            "Only `.agents/skills/*/SKILL.md` entries are default skill entrypoints.",
            "skills/gameplay-mechanics",
            "If a specialist skill is used, TPM must still bind it to the same owner",
            "the specialist role owns the professional conclusion",
        ],
    },
    {
        "id": "root_skills_are_non_default_library_material",
        "expected_route": "TPM opts into root skills library material explicitly instead of treating it as a default trigger",
        "surface": ".agents/skills/README.md",
        "required_markers": [
            "Root `skills/` is the non-default specialist library surface",
            "Do not rely on root `skills/` material for automatic workflow routing.",
            "Non-default specialist library material under root `skills/` is opt-in",
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
        "expected_route": "prd/game-architect may supplement planning but not replace TPM/GitHub-backed task truth",
        "surface": "skills/prd/SKILL.md",
        "required_markers": [
            "this skill is a specialist planning surface, not a standalone workflow",
            "Record the PRD route, TODOs, and downstream handoff in GitHub task issue evidence comments.",
            "Do not treat PRD-only output as implementation-ready",
        ],
    },
    {
        "id": "game_architect_binds_back_to_tpm_pm_truth",
        "expected_route": "game-architect docs remain supplemental architecture planning",
        "surface": "skills/game-architect/SKILL.md",
        "required_markers": [
            "this skill is a specialist architecture-planning surface, not a second project workflow",
            "record the route, TODOs, and downstream execution handoff in GitHub task issue evidence comments",
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
            "If the content would be better owned by `AGENTS.md`, a PRD/design/evidence document, a handoff template, GitHub-backed task truth, or a script check",
            "If the skill introduces or documents a helper-driven workflow, also run at least one representative command or check tied to that workflow.",
        ],
    },
    {
        "id": "tpm_planning_requires_github_issue_evidence_before_dispatch",
        "expected_route": "TPM records TODO decomposition and slice contracts before delegated execution",
        "surface": ".agents/skills/repo-owned-workflow-router/SKILL.md",
        "required_markers": [
            "Read-only/chat-only requests enter this router after `default-workflow-bootstrap` has established task truth.",
            "Record the slice contract in the GitHub issue evidence comment sink before dispatch.",
            "formal sink / writeback surface: GitHub issue evidence comment (mandatory)",
        ],
    },
    {
        "id": "completion_claim_requires_fresh_verification",
        "expected_route": "verification-before-completion before done/tests-passed/ready-for-pr claims",
        "surface": ".agents/skills/verification-before-completion/SKILL.md",
        "required_markers": [
            "Run the verification command now, read the result now, and only then make the claim.",
            "Do not use stale output, partial output, or earlier successful runs as proof.",
            "current verification epoch",
            "./scripts/pm/claim-ready.sh",
        ],
    },
    {
        "id": "closeout_routes_to_local_role_review_then_github_pr_review",
        "expected_route": "finishing-a-development-branch -> local role review -> prepare-task-pr -> GitHub required checks/review -> merge/cleanup",
        "surface": ".agents/skills/finishing-a-development-branch/SKILL.md",
        "required_markers": [
            "Use `requesting-repo-owned-review`; resolve findings against that same head.",
            "--verification-profile <repository-owned-profile>",
            "--review-packet-file <canonical-review-packet.json>",
            "its schema is only at the canonical review-packet link",
            "./scripts/prepare-task-pr.sh --draft-candidate --create",
            "source-of-truth.md#canonical-state-machine",
            "source-of-truth.md#workflow-states",
            "source-of-truth.md#ready-and-done",
            "./scripts/pm/pr-lifecycle-gate.py <pr-number> --task-uid <task_uid> --json",
            "All interpretations, retry loops, dispositions and merge authorization come",
            "Do not land locally unless the user explicitly asks for local landing.",
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


RESULT_JSON="$(python3 - "$TASK_WORKTREE_JSON_FILE" "$SUBAGENT_CONTRACT_JSON_FILE" "$ROUTING_SCENARIOS_JSON_FILE" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

task_worktree = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
subagent_contract = json.loads(Path(sys.argv[2]).read_text(encoding="utf-8"))
routing_scenarios = json.loads(Path(sys.argv[3]).read_text(encoding="utf-8"))
runtime_policy = subagent_contract["runtime_policy"]

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
        "command": "complete TOML + Codex strict-load + negative fixture checks over specialist adapter config, then Python contract checks over AGENTS / handoff / router surfaces",
        "status": subagent_contract["status"],
        "evidence": {
            "surface_count": len(subagent_contract["surfaces"]),
            "codex_agent_config": subagent_contract["codex_agent_config"],
        },
    },
    {
        "id": "pm_projection_immutability",
        "command": "fail-only full-.pm guard compares the complete tracked/baseline-untracked/ignored path set with lstat kind/mode/content/symlink target and exact index mode/OID/stage/path separately; pm-lint rejects source symlinks before a manifest-stable retry/fail snapshot and uses temp pycache isolation",
        "status": "passed",
        "evidence": {
            "pathspec": ".pm",
            "tracked_projection_unchanged_before_cleanup": True,
            "new_untracked_projection_artifacts": False,
            "live_restore_enabled": False,
            "guard_records_symlink_state": True,
            "lint_source_symlinks_allowed": False,
            "python_cache_location": "temporary lint directory",
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
        "id": "github_backed_pm_runtime",
        "command": "./scripts/pm/github-project-task.test.sh && ./scripts/pm/github-project-sync.test.sh && ./scripts/pm/github-project-workflow.test.sh && ./scripts/pm/github-project-retire-tasks.test.sh && ./scripts/pm/audit-pr-watch-issues.test.sh && ./scripts/pm/fallback-evidence.test.sh",
        "status": "passed",
        "evidence": {
            "task_adapter": "passed",
            "sync_adapter": "passed",
            "workflow_adapter": "passed",
            "retire_archive_adapter": "passed",
            "pr_watch_merged_audit": "passed",
            "fallback_evidence_adapter": "passed",
        },
    },
    {
        "id": "completion_claim_gate",
        "command": "./scripts/pm/claim-ready.test.sh && ./scripts/pm/claim-ready-ready-pr.test.sh",
        "status": "passed",
        "evidence": {
            "helper": "claim-ready",
            "ready_pr_revalidation": "passed",
        },
    },
    {
        "id": "closeout_tmpdir_portability",
        "command": "./scripts/pm/closeout-tmpdir-portability.test.sh",
        "status": "passed",
        "evidence": {
            "helpers": ["claim-ready", "task-closeout"],
            "windows_native_tmpdir": "passed",
            "posix_tmpdir_preserved": "passed",
        },
    },
    {
        "id": "terminal_default_cache_recovery",
        "command": "./scripts/pm/post-merge-main-sync-default-cache-recovery.test.sh && python3 ./scripts/pm/recover-terminal-task-mapping.test.py",
        "status": "passed",
        "evidence": {
            "registered_canonical_worktree_import": "passed",
            "atomic_default_mapping_update": "passed",
            "conflict_and_identity_rejection": "passed",
        },
    },
    {
        "id": "post_pr_evidence_chain",
        "command": "./scripts/pm/workflow-lint.test.sh",
        "status": "passed",
        "evidence": {
            "helper": "workflow-lint",
            "root_pr_md_allowed": False,
        },
    },
    {
        "id": "pre_pr_review_packet_helper",
        "command": "./scripts/pm/record-pre-pr-review.test.sh",
        "status": "passed",
        "evidence": {
            "helper": "record-pre-pr-review",
            "requires_explicit_role_evidence": True,
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
    "fixture_scope": "repo-owned bootstrap/routing surface checks, isolated worktree bootstrap smoke, GitHub-backed PM runtime tests, and fake-gh PR helper tests",
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
        f"subagent slice contracts record the capability-aware runtime policy ({runtime_policy}) and distinguish intended from actual model selection",
        "named-role adapter activation is claimed only on a dispatch surface that exposes and uses a named-role selector; Desktop falls back to message-assigned role attribution",
        "PR creation requires local involved-role subagent review evidence before GitHub PR watch/fix/merge",
        "done closeout refuses to proceed without fresh verification",
        "done closeout updates GitHub issue metadata, Project task fields, and closes the GitHub task issue",
        "PR preflight stays the default GitHub PR entrypoint after local role review evidence",
        "normal PRs continue after creation into cursor-exhaustive required-check/comment/review/thread/mergeability watch, failure fixes, comment closeout, merge, and cleanup; REVIEW_REQUIRED and BEHIND are informational, while repository standing policy defaults to admin merge only for a freshly rechecked MERGEABLE review-approval-only BLOCKED or BEHIND state",
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
        "task-closeout allows done closeout without a repository-owned verification profile",
        "subagent contract markers disappear from AGENTS or handoff/router surfaces",
        f"subagent runtime markers disappear or no longer preserve the capability-aware policy ({runtime_policy})",
        "adapter registration is treated as activation, or Desktop full-history fallback loses its message-assigned attribution boundary",
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
