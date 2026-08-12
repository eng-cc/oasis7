#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-subagent-role-fit-verify.XXXXXX")"
OASIS7_ROLE_FIT_SCRATCH="$TMP_DIR"
trap 'rm -rf "$TMP_DIR"' EXIT

TASK_UID=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --task-uid)
      TASK_UID="${2:-}"
      shift 2
      ;;
    -h|--help)
      echo "Usage: ./scripts/pm/verify-codex-subagent-role-fit.sh --task-uid <task_uid>"
      exit 0
      ;;
    *)
      echo "verify-codex-subagent-role-fit: unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

[[ "$TASK_UID" =~ ^task_[a-z0-9]{32}$ ]] || {
  echo "verify-codex-subagent-role-fit: --task-uid must be a 32-character task UID" >&2
  exit 2
}

cd "$ROOT_DIR"
./scripts/pm/github-project-workflow.sh --json audit \
  --task-uid "$TASK_UID" >/dev/null
./scripts/pm/workflow-behavior-eval.sh
./scripts/doc-governance-check.sh
./scripts/lint-skills.sh

bash -n \
  scripts/pm/claim-ready.sh \
  scripts/pm/task-closeout.sh \
  scripts/pm/post-merge-cleanup.sh \
  scripts/pm/slice-ledger.sh \
  scripts/pm/new-task-worktree-bootstrap-smoke.sh \
  scripts/new-task-worktree.sh \
  scripts/pm/verify-codex-subagent-role-fit.sh \
  scripts/pm/validate-codex-agent-config.test.sh \
  scripts/prepare-task-pr.sh \
  scripts/prepare-task-pr.test.sh

PYTHONPYCACHEPREFIX="$OASIS7_ROLE_FIT_SCRATCH/pycache" python3 -m py_compile \
  scripts/pm/repo-state-fingerprint.py \
  scripts/pm/github-project-task.py \
  scripts/pm/pr-lifecycle-gate.py \
  scripts/pm/validate-review-provenance.py
TOML_PYTHON="$(./scripts/pm/find-python-with-module.sh tomllib)"
PYTHONPYCACHEPREFIX="$OASIS7_ROLE_FIT_SCRATCH/pycache" "$TOML_PYTHON" -m py_compile \
  scripts/pm/render-codex-agent-config.py \
  scripts/pm/validate-codex-agent-config.py

if [[ -n "${OASIS7_CLAIM_COMPARISON_REF:-}" ]]; then
  git diff --check "${OASIS7_CLAIM_COMPARISON_REF}...HEAD"
else
  git diff --check
fi
echo "verify-codex-subagent-role-fit: PASS"
