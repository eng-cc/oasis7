# Workflow Preflight and Terminal Readiness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the repository-owned workflow expose a public non-PR classification path, pre-merge terminal readiness, and one exact machine-readable resume command without weakening GitHub task authority.

**Architecture:** Extend the existing GitHub task adapter and terminal helpers; do not create another task store. The query surface reads canonical task mapping plus ignored snapshot/ledger artifacts, while every mutation is committed through the existing GitHub issue roundtrip and refreshed cache.

**Tech Stack:** Python 3 standard library, Bash, GitHub CLI fixtures, repository PM contract tests.

**Spec:** https://github.com/eng-cc/oasis7/issues/3570

## Global Constraints

- GitHub Project task truth and issue evidence remain authoritative.
- Callers never edit `.pm/github-project-sync/tasks.json` directly.
- Identity ambiguity, stale task truth, and task/PR mismatch fail closed.
- No merge authorization or trusted-receipt requirement is weakened.
- `third_party/` remains read-only.

---

### Task 1: Public non-PR classification

**Files:**
- Modify: `scripts/pm/github-project-task.py`
- Modify: `scripts/pm/github-project-task.test.sh`
- Modify: `scripts/pm/non-merge-finalize-functional.test.py`
- Modify: `doc/engineering/workflow/source-of-truth.md`

**Interfaces:**
- Consumes: bound `task_uid`, classification `non_pr_task`, non-empty evidence text, live GitHub issue identity.
- Produces: a runtime-verified issue-comment receipt and refreshed task fields `completion_mode=non_pr_task` plus `non_pr_completion_evidence`.

- [ ] **Step 1:** Add a failing adapter test invoking `classify-non-pr-task --task-uid <uid> --evidence <text> --json`; require issue-comment write/readback and refreshed cache fields.
- [ ] **Step 2:** Run `rtk ./scripts/pm/github-project-task.test.sh` and confirm failure is the missing subcommand.
- [ ] **Step 3:** Implement the subcommand using the existing issue-comment mutation/readback pattern and reject blank evidence, terminal tasks, PR-bound tasks, and identity drift.
- [ ] **Step 4:** Extend the non-merge functional fixture to classify through the public command before `task_complete` closeout and `non_pr_completed` finalization.
- [ ] **Step 5:** Rerun both focused tests and commit the independently working classification slice.

### Task 2: Pre-merge terminal readiness

**Files:**
- Modify: `scripts/pm/finalize-task.sh`
- Modify: `scripts/pm/finalize-task.test.sh`
- Modify: `scripts/pm/finalize-task-red.test.sh`
- Modify: `.agents/skills/finishing-a-development-branch/SKILL.md`

**Interfaces:**
- Consumes: `--task-uid`, `--pr`, canonical default worktree, live/refreshed task mapping.
- Produces: `--preflight --json` result containing `status`, bound identity fields, `blockers`, and an exact finalizer command; it performs no terminal mutation.

- [ ] **Step 1:** Add failing tests for a valid binding and for task/PR, worktree, branch, and repository mismatches before merge.
- [ ] **Step 2:** Run the two finalizer test scripts and confirm the new `--preflight` contract is absent.
- [ ] **Step 3:** Refactor the existing identity checks into the preflight path; emit JSON only after all required fields and live paths validate.
- [ ] **Step 4:** Make the normal finalizer call the same validator and document the mandatory pre-merge invocation in the finishing skill.
- [ ] **Step 5:** Rerun focused tests and commit the terminal-readiness slice.

### Task 3: Unified next and resume query

**Files:**
- Create: `scripts/pm/workflow-next.py`
- Create: `scripts/pm/workflow-next.test.py`
- Modify: `.agents/skills/executing-project-tasks/SKILL.md`
- Modify: `.agents/skills/finishing-a-development-branch/SKILL.md`
- Modify: `doc/engineering/workflow/source-of-truth.md`

**Interfaces:**
- Consumes: `--task-uid`, canonical task mapping, bootstrap snapshot, optional slice ledger and durable workflow checkpoint.
- Produces: JSON fields `task_uid`, `workflow_phase`, `identity_status`, `evidence_sources`, `blockers`, and one shell-token-array `next_command` selected only from supported public commands.

- [ ] **Step 1:** Write table-driven failing tests for bootstrap, execution, pre-PR, PR watch, merged-terminal, non-PR closeout, stale identity, and ambiguous state.
- [ ] **Step 2:** Run `rtk python3 scripts/pm/workflow-next.test.py` and confirm failure is the absent query helper.
- [ ] **Step 3:** Implement read-only state reduction; treat ledger/snapshot as evidence inputs, never mutation authority, and return no command on ambiguity.
- [ ] **Step 4:** Add skill/source-of-truth examples using the exact supported CLI syntax.
- [ ] **Step 5:** Rerun the focused test and commit the query slice.

### Task 4: Reachable TDD guidance and executable examples

**Files:**
- Modify: `.agents/skills/tdd-test-writer/SKILL.md`
- Modify: `scripts/pm/tpm-workflow-doc-contract.test.py`
- Modify: `scripts/pm/workflow-adversarial-contract.test.sh`

**Interfaces:**
- Consumes: canonical role inventory and command examples in touched workflow skills.
- Produces: role-reachable fallback guidance and tests that reject unsupported flags in canonical examples.

- [ ] **Step 1:** Add failing doc-contract assertions rejecting mandatory dispatch to an absent role and parsing each touched command example against `--help`/argument fixtures.
- [ ] **Step 2:** Run the focused doc-contract tests and observe the current unreachable `tdd_test_writer` reference fail.
- [ ] **Step 3:** Change RED ownership to the currently assigned professional implementation role with an explicit optional specialist only when registered.
- [ ] **Step 4:** Correct touched examples and run `rtk ./scripts/lint-skills.sh`, `rtk python3 scripts/pm/tpm-workflow-doc-contract.test.py`, `rtk ./scripts/pm/workflow-adversarial-contract.test.sh`, `rtk ./scripts/doc-governance-check.sh`, and `rtk git diff --check`.
- [ ] **Step 5:** Commit the governance/contract slice and return the full verification evidence for independent QA.
