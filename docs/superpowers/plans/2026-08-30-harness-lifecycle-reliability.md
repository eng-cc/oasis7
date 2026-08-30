# Harness Lifecycle Reliability Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make worktree harness state publication, port allocation, and shutdown deterministic under concurrent and failure conditions.

**Architecture:** Keep the current harness CLI and metadata schemas. Add reusable atomic-write and managed-process helpers, serialize allocation within the harness authority, and prove the public lifecycle through an injected fake launcher.

**Tech Stack:** Bash, Python 3 standard library, POSIX process groups, deterministic shell fixtures.

**Spec:** https://github.com/eng-cc/oasis7/issues/3573

## Global Constraints

- Never kill a process not proven to belong to the task harness.
- State and session metadata schemas remain backward compatible.
- Tests do not require Rust compilation, a real LLM provider, or a browser.
- Port coordination is bounded to repository-owned harness state and locks.
- `third_party/` remains read-only.

---

### Task 1: Atomic state and metadata publication

**Files:**
- Modify: `scripts/worktree-harness-lib.sh`
- Modify: `scripts/run-launcher-stack.sh`
- Modify: `scripts/worktree-harness-contract.test.sh`

**Interfaces:**
- Consumes: destination path and complete serialized record.
- Produces: same-directory temporary write, flush/close, and atomic replacement while preserving the existing record format.

- [x] **Step 1:** Add a failing concurrent reader/writer fixture requiring every observed `state.json` and `session.meta` record to parse completely.
- [x] **Step 2:** Run `rtk ./scripts/worktree-harness-contract.test.sh` and confirm the atomic-publication contract fails.
- [x] **Step 3:** Implement atomic replace helpers and route both writers through them.
- [x] **Step 4:** Rerun the focused test and record the atomic publication result (commit deferred to the integration owner).

### Task 2: Managed process-tree shutdown

**Files:**
- Modify: `scripts/worktree-harness-lib.sh`
- Modify: `scripts/worktree-harness.sh`
- Modify: `scripts/run-launcher-stack.sh`
- Modify: `scripts/worktree-harness-contract.test.sh`

**Interfaces:**
- Consumes: recorded harness-owned PID/process-group identity.
- Produces: bounded TERM, liveness polling, then KILL only for the same recorded process group.

- [x] **Step 1:** Add a fake parent that spawns a child and a failing assertion that `down` removes both.
- [x] **Step 2:** Run the focused contract test and verify the child remains alive under current behavior.
- [x] **Step 3:** Implement one reusable process-group termination helper and use it in harness and launcher cleanup.
- [x] **Step 4:** Rerun the regression, verify unrelated sentinel processes survive, and record the shutdown result (commit deferred to the integration owner).

### Task 3: Serialized port reservation

**Files:**
- Modify: `scripts/worktree-harness-lib.sh`
- Modify: `scripts/worktree-harness.sh`
- Test: `scripts/worktree-harness-contract.test.sh`

**Interfaces:**
- Consumes: harness root, candidate base port, required port offsets.
- Produces: lock-held allocation/reservation result that another concurrent `up` cannot also claim.

- [x] **Step 1:** Add a failing two-process allocation fixture synchronized at the current probe/release window.
- [x] **Step 2:** Run the fixture and observe duplicate allocation.
- [x] **Step 3:** Add per-harness locking and persist the selected allocation before releasing coordination authority; stale reservations must be recoverable only after owner liveness checks.
- [x] **Step 4:** Rerun concurrent and stale-owner cases and record the allocation result (commit deferred to the integration owner).

### Task 4: Public lifecycle acceptance lane

**Files:**
- Create: `scripts/worktree-harness-lifecycle.test.sh`
- Modify: `scripts/worktree-harness.sh`

**Interfaces:**
- Consumes: an explicit test-only launcher command environment variable and isolated temporary harness root.
- Produces: production-identical `up`, `status --json`, `url`, and `down` behavior without network or compiled binaries.

- [x] **Step 1:** Add the fake launcher and assertions for ready state, URL, timeout state, port release, and child cleanup; verify the current CLI lacks the injection seam.
- [x] **Step 2:** Implement the minimal explicit test-only launcher injection while retaining the production default.
- [x] **Step 3:** Run `rtk ./scripts/worktree-harness-lifecycle.test.sh`, `rtk ./scripts/worktree-harness-contract.test.sh`, `rtk ./scripts/run-launcher-stack-local-mock-lane.test.sh`, and `rtk git diff --check`.
- [x] **Step 4:** Record the acceptance result and return exact evidence for independent QA (commit deferred to the integration owner).
