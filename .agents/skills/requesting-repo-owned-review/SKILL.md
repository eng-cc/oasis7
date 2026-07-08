---
name: requesting-repo-owned-review
description: Use when a branch is about to create a PR and must collect fresh involved-role subagent review before GitHub PR creation.
---

# Requesting Repo-Owned Review

> PM truth: PM workflow/script changes should include repository-health review and should treat GitHub Project audit evidence as part of the review target when queue/status semantics changed.

Use this skill when the work is about to enter the GitHub PR path.

The review is no longer optional before PR creation. TPM must create or dispatch
fresh local subagents for every involved relevant professional role, integrate
their review findings, and record the required evidence packet before
`prepare-task-pr.sh --create`.

## When to Use

Use this skill when:

- a branch is about to create a PR
- a major feature or workflow helper just landed locally
- multiple role slices were just integrated back into one canonical diff
- the next claim is `ready_for_pr`, `tests_passed`, or a broad behavioral assertion

Do not use this skill when:

- there is no PR creation path in the current task
- you are trying to replace GitHub PR required checks, requested-changes handling, comment closeout, or mergeability with only an internal review ritual
- no concrete review target, risk question, or evidence sink has been defined

## Core Rule

Pre-PR local role review is required before PR creation, but it is not a
replacement for GitHub required checks, requested-changes handling, comment
closeout, mergeability, or the repository/GitHub merge path. `REVIEW_REQUIRED`
is informational and is not a blocking item by itself. A review-approval-only
`mergeStateStatus=BLOCKED` may use an explicitly authorized repository admin
merge path, but this does not weaken checks, requested changes, comments, or
mergeability gates.

The formal path is:

`local involved-role subagent review -> prepare-task-pr -> GitHub required checks -> comment/requested-changes closeout -> mergeability -> merge`

## Workflow

1. Define the involved roles:
   - infer from changed paths, role ownership, task slice history, and user-facing claim
   - include `producer_system_designer` when scope, product contract, user promise, acceptance, or system-level semantics change
   - include `gameplay_designer` when gameplay rules, progression, balance, encounter/resource loops, or player verb semantics are touched
   - include `game_visual_interaction_designer` when visible UI/gameplay presentation, visual direction, interaction feel, player-facing screen flow, screenshot/visual-review surfaces, accessibility/readability, or UI-heavy claims are touched
   - include `runtime_engineer` when runtime/server/simulation/gameplay enforcement, replay, recovery, checkpoint, long-run behavior, or `crates/oasis7*` runtime paths are touched
   - include `blockchain_ops_engineer` when deployment, node ops, topology/inventory, service/host contracts, health baselines, upgrade/rollback/restore drills, packaging/release ops, or operator-facing runbooks are touched
   - include `wasm_platform_engineer` when `crates/oasis7_wasm_*`, builtin wasm modules, ABI/schema, manifest/hash, wasm build/receipt, wasm determinism workflows, or `doc/world-runtime/wasm/*` are touched
   - include `agent_engineer` when agent behavior, prompts, provider contracts, model/runtime config, subagent dispatch contracts, or agent tooling are touched
   - include `viewer_engineer` when Viewer/Web/UI/WebGPU/browser validation paths are touched
   - include `qa_engineer` when the PR changes verification helpers, testing docs, release/readiness claims, test strategy, or evidence sufficiency
   - include `repository_health_engineer` when the diff changes cross-cutting architecture, shared workflow surfaces, docs/code contracts, large refactors, repeated bug signatures, or known technical-debt boundaries
   - include `liveops_community` when external messaging, incidents, player promises, community feedback, release notes, or channel runbooks are touched
   - treat `scripts/prepare-task-pr.sh --create` required-role inference as the minimum mechanical backstop, not as a replacement for task-history or claim-based role selection
   - record explicit one-line skip rationale for adjacent roles that are plausible but not involved
2. Freeze the review target:
   - changed files or path set
   - exact question to answer
   - evidence already available
   - review package path from `./scripts/pm/review-package.sh --base <ref> --head <ref> --task-uid <TASK-UID>`, or explicit `n/a` with reason when the review target is not a git diff
   - slice ledger path from `./scripts/pm/slice-ledger.sh --task-uid <TASK-UID> --print`, or explicit `n/a` with reason for one-shot reviews without reusable slice state
3. Spawn or dispatch a fresh subagent for each involved role.
4. State the expected output contract:
   - `findings`
   - `no_findings`
   - scope/spec compliance verdict
   - role quality/risk verdict
   - `residual_risk`
5. Write the review request into GitHub task issue evidence comments before or while dispatching.
   - PR evidence documents and handoffs may link to or summarize that request
     only after the GitHub issue evidence sink exists.
   - If GitHub task issue comments are temporarily unavailable, use only the
     source-of-truth fallback evidence path and replay it to GitHub before
     treating the request as task truth.
6. Act on the result:
   - fix valid findings
   - record rejected/stale findings with code or doc evidence
   - keep residual risk explicit
7. Record the passed evidence packet only after all valid findings are resolved.
8. Only then continue to PR creation.

## Review Packet Template

```markdown
## YYYY-MM-DD HH:MM:SS CST / <role_name>
- Review Trigger: pre-PR local role review
- Review Scope: <paths / diff summary>
- Review Package: <repo-relative/scratch-relative path or n/a with reason>
- Review Roles: <comma-separated roles>
- Review Question: <what must this review confirm or challenge>
- Evidence Available: <tests / docs / screenshots / logs>
- Expected Return Contract: <findings | no_findings | scope/spec compliance verdict | role quality/risk verdict | residual_risk>
- Slice Ledger: <repo-relative/scratch-relative path or n/a with reason>
- Formal Sink: GitHub task issue evidence comments
```

## Passed Evidence Packet

Record this packet in GitHub task issue evidence comments after integrating the
role reviews and addressing findings:

```markdown
- Pre-PR Local Role Review: passed
- Task UID: <task_uid>
- Source Worktree: <task worktree name or repo-relative worktree hint; avoid local absolute paths in GitHub issue evidence>
- Source Branch: <branch>
- Source Head: <reviewed git sha; must be current source head or an ancestor whose later changes are only the task review evidence files>
- Comparison Ref: <base ref>
- Reviewed Changed Paths: <semicolon-separated paths or diff summary ref>
- Review Package: <repo-relative/scratch-relative path to review package or n/a with reason>
- Role Selection Basis: <changed paths + task slice history + explicit includes/skips>
- Review Roles: <comma-separated roles>
- Review Evidence: <per-role section or handoff refs>
- Review Verdicts: <per-role scope/spec compliance verdict + role quality/risk verdict>
- Review Findings Disposition: <addressed | no_findings>
- Finding Disposition Evidence: <fix refs or rejected/stale evidence refs>
- Verification Matrix: <changed surface -> required evidence -> observed evidence or explicit deferral>
- Visual Evidence: <screenshot/model visual review paths or n/a with exemption reason>
- WASM Evidence: <support crate/determinism evidence or n/a with reason>
- Ops Evidence: <readiness/rollback/runbook/operator evidence or n/a with reason>
- LiveOps Evidence: <messaging/release-note/status/community evidence or n/a with reason>
- Residual Risk: <text>
- Slice Ledger: <repo-relative/scratch-relative path to slice ledger or n/a with reason>
```

For small workflow/docs-only diffs, `./scripts/pm/record-pre-pr-review.sh` may
generate the packet shape after the involved role review outcome is known. It is
a formatting helper only; it does not replace dispatch, findings disposition, or
fresh verification.

## Output Rules

- Findings must be categorized by severity or merge risk.
- Each involved role should return two explicit verdicts: scope/spec compliance and role quality/risk. This is a review packet shape, not a replacement for role-specific professional ownership.
- `no_findings` still needs a short residual-risk statement when risk is not literally zero.
- If the review is stale or wrong, answer with concrete repo truth instead of silently ignoring it.
- Always separate:
  - local repo-owned review outcome
  - GitHub PR review readiness

## Known Failure Modes

- Treating the local role review as optional when the branch is already on the PR path.
- Recording a review request in chat only, leaving GitHub task issue evidence comments without the packet required by preflight.
- Selecting roles from convenience instead of changed paths, role ownership, task history, and user-facing claims.
- Resolving or dismissing GitHub review threads based only on local repo-owned review evidence.

## Guardrails

- Do not leave PR creation without a passed pre-PR local role review packet that includes required-role coverage and verification-matrix evidence.
- Do not claim that repo-owned review makes GitHub review unnecessary.
- Do not leave the review request or outcome as chat-only context.
- Do not resolve GitHub threads based solely on this local review packet.
- Do not paste large diffs into GitHub task issue evidence comments when a review package can be linked instead.
- Do not rely on the slice ledger as the only task truth; GitHub task issue evidence comments remain the mandatory sink.
