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
  -> repo-owned-workflow-router -> producer orchestrate / role subagent dispatch
  -> task-closeout -> prepare-task-pr -> review-thread-closeout

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

TASK_WORKTREE_JSON="$("$ROOT_DIR/scripts/pm/new-task-worktree-bootstrap-smoke.sh" --json)"
REQUIRED_TIER_JSON="$("$ROOT_DIR/scripts/pm/required-tier-smoke.sh" --json)"
"$ROOT_DIR/scripts/pm/claim-ready.test.sh" >/dev/null
"$ROOT_DIR/scripts/prepare-task-pr.test.sh" >/dev/null
"$ROOT_DIR/scripts/pr-review-thread-closeout.test.sh" >/dev/null

SUBAGENT_CONTRACT_JSON="$(python3 - "$ROOT_DIR" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

root = Path(sys.argv[1])

checks = [
    (
        root / ".agents/skills/default-workflow-bootstrap/SKILL.md",
        [
            "## Task Classification",
            "## Isolation Decision",
            "## Task Truth",
            "## Routed Next Phase",
            "./scripts/new-task-worktree.sh",
            "./.agents/skills/repo-owned-workflow-router/SKILL.md",
        ],
    ),
    (
        root / "AGENTS.md",
        [
            "default-workflow-bootstrap",
            "判断 trivial/non-trivial、是否已具备隔离 task worktree / `.pm` task 真值",
            "`producer_system_designer` orchestrator + 角色 subagents",
            "formal sink",
            "liveops_community` 必须参与至少一个 slice",
            "requesting-repo-owned-review/SKILL.md",
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
        root / ".agents/skills/requesting-repo-owned-review/SKILL.md",
        [
            "Repo-owned review is a supplement, not a replacement.",
            "findings",
            "no_findings",
            "residual_risk",
        ],
    ),
    (
        root / ".agents/skills/repo-owned-workflow-router/SKILL.md",
        [
            "## Subagent Slice Plan (If Needed)",
            "- role:",
            "- slice type:",
            "- write scope:",
            "- return contract:",
            "- formal sink / writeback surface:",
            "- integration owner:",
            "- integration order:",
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

print(json.dumps({"status": "ok", "surfaces": surfaces}, ensure_ascii=False))
PY
)"

RESULT_JSON="$(python3 - "$TASK_WORKTREE_JSON" "$SUBAGENT_CONTRACT_JSON" "$REQUIRED_TIER_JSON" <<'PY'
from __future__ import annotations

import json
import sys

task_worktree = json.loads(sys.argv[1])
subagent_contract = json.loads(sys.argv[2])
required_tier = json.loads(sys.argv[3])

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
    "workflow_path": "default-workflow-bootstrap -> new-task-worktree -> workflow-report -> repo-owned-workflow-router -> producer orchestrate / role subagent dispatch -> task-closeout -> prepare-task-pr -> review-thread-closeout",
    "fixture_scope": "repo-owned bootstrap/routing surface checks, isolated worktree bootstrap smoke, PM runtime smoke, and fake-gh PR helper tests",
    "expected_agent_behavior": [
        "new non-trivial work first routes through a repo-owned bootstrap surface rather than an external bootstrap",
        "bootstrap distinguishes trivial vs non-trivial work and ensures isolated task truth exists before routing",
        "task worktree bootstrap stays source-clean and starts the target task",
        "subagent dispatch remains bound to owner/write-scope/return-contract/formal-sink surfaces",
        "high-risk local diffs can request repo-owned review packets without replacing GitHub PR review",
        "done closeout refuses to proceed without fresh verification",
        "PR preflight stays the default GitHub PR entrypoint",
        "review-thread closeout reports unresolved/resolved thread state without conflating merge readiness",
    ],
    "verification_surface": [segment["id"] for segment in segments],
    "failure_signature": [
        "default bootstrap surface disappears or no longer points new non-trivial work into repo-owned task truth",
        "task-closeout allows done closeout without verify-command",
        "subagent contract markers disappear from AGENTS or handoff/router surfaces",
        "repo-owned review-request surface disappears or stops separating local review from GitHub review",
        "prepare-task-pr local fixture no longer creates the expected PR command path",
        "review-thread closeout helper stops reporting unresolved/resolved state correctly",
    ],
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
