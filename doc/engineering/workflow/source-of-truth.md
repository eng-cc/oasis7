# Engineering Workflow Source of Truth

Version: **v1.5.6**
Last Updated: **2026-07-07**

## 0. Purpose
This file is the **only normative workflow specification** for engineering task execution in oasis7.

Mandatory rule:
1. Any workflow change must be edited in this file first.
2. After this file is updated, sync all related scripts/docs/skills to match.
3. PRs that change workflow scripts without updating this file are invalid.

## 1. Phase Diagram
```mermaid
flowchart TD
  A[Bootstrap\nverify standard worktree + task truth] --> B[Router\nchoose current phase]
  B --> C{Need brainstorming?}
  C -- yes --> D[Bounded Brainstorming]
  C -- no --> E
  D --> E[Execution Gap Review + Atomic Plan]
  E --> F{Behavior changes\nwith stable harness?}
  F -- yes --> G[Behavior-first RED / TDD]
  F -- no --> H
  G --> H[Implementation + Slice Verification]
  H --> I[Verification-before-completion\nfresh verification / claim-ready]
  I --> M[Pre-PR Local Role Review\ninvolved role subagents review diff]
  M --> J[Closeout\ncommit + PR create]
  J --> N{PR opened for\nmanual packaging CI only?}
  N -- no --> O[PR Watch/Fix/Merge\nwatch checks + reviews, fix failures, merge]
  N -- yes --> P[Manual CI Hold\nrecord purpose + wait for operator/user]
  O --> Q[Cleanup\nsync main + remove task worktree]
  P --> O

  I -->|fail| K[Rollback: debug/fix/replan]
  K --> E
  M -->|findings require changes| K
  O -->|PR check/requested-changes/comment failure| L[Review Fix Loop]
  L --> I
```

### 1.1 Skill Map by Phase
This map makes skill reachability explicit. TPM owns the route decision as a workflow coordination act and records the selected skill path in GitHub task issue evidence comments before delegated execution begins.

| Phase / trigger | Skill surface | Requiredness | Formal evidence |
| --- | --- | --- | --- |
| Any user request starts | `default-workflow-bootstrap` | Required before fact lookup, chat answer, professional slice dispatch, edits, verification, review, or external messaging unless already inside the bound task worktree | Bootstrap entry in GitHub task issue evidence comments |
| Read-only professional/domain question | Matching professional bounded slice under TPM coordination after task/worktree bootstrap | Required when the answer depends on product/design/gameplay/game-visual-interaction/runtime/blockchain-ops/WASM/agent/viewer/QA/repository-health/liveops judgment; skipped only for pure fact lookup after task truth exists | Role-tagged slice return recorded in GitHub task issue evidence comments and summarized to the user |
| Bound task needs next phase selection | `repo-owned-workflow-router` | Required after bootstrap and whenever phase is unclear | Route entry with selected/skipped skills in GitHub task issue evidence comments |
| Scope is ambiguous, option-heavy, or visual enough to need ideation | `bounded-brainstorming` | Optional, risk-based | Brainstorming output or skip reason in GitHub task issue evidence comments/project |
| Behavior changes with a stable automated harness | `tdd-test-writer` | Conditional required when RED criteria are met; otherwise skip reason required | RED command, failing evidence, and handoff contract |
| Repo truth is ready and implementation proceeds step by step | `executing-project-tasks` | Required for non-trivial execution after route selection | Atomic step evidence in GitHub task issue evidence comments |
| Bug, failing test, broken script, unexpected diff, or regression appears | `systematic-debugging` | Conditional required before speculative fixes | Reproduction, narrowed hypothesis, fix evidence |
| Branch is about to create a PR | `requesting-repo-owned-review` | Required before PR creation; TPM must spawn or dispatch fresh local subagents for all involved relevant roles, collect review findings/no-findings/residual risk, and address or explicitly reject actionable findings with evidence before continuing | `Pre-PR Local Role Review: passed` GitHub issue evidence packet with roles, review evidence, finding disposition, and residual risk |
| About to claim done/tests-pass/ready-for-PR/ready-to-merge | `verification-before-completion` | Required before completion claims | Fresh verification command/output or claim-ready evidence |
| Implementation is done and branch needs closeout/commit/PR/watch/merge | `finishing-a-development-branch` | Required for development branch closeout | Closeout output, commit, PR linkage, PR purpose decision, CI/review watch evidence, merge/cleanup evidence |
| GitHub PR receives review comments or requested changes | `receiving-code-review` | Required for actionable PR review feedback | Comment verification, fix evidence, thread status |
| Workflow skill/docs themselves are created or edited | `writing-repo-owned-skills` | Required for local skill surface changes | Source-of-truth-first sync plus `./scripts/lint-skills.sh` and governance checks |

### 1.2 Specialist Skill Reachability
Specialist skills are not mandatory workflow phases. They become reachable through TPM routing or professional subagent slice planning when the task domain matches their trigger.

The repository has two skill-like surfaces:

- `.agents/skills/`: default-loadable repo-owned workflow skill entrypoints. Keep workflow gates here.
- `skills/`: non-default specialist library material for professional method skills. Role cards and slice contracts may reference these skills, but they do not automatically trigger.

Only `.agents/skills/*/SKILL.md` entries are default skill entrypoints. Material under `skills/` may preserve `SKILL.md`-style structure for provenance and linting, but it is not part of the default workflow trigger set unless a `.agents/skills` wrapper or source-of-truth update explicitly promotes it back.

- Product/planning docs: `skills/prd`, `skills/game-architect`; these may create planning artifacts, but the route, TODOs, and downstream handoff must still be recorded in GitHub task issue evidence comments.
- Game/domain implementation: `skills/game-design-theory`, `skills/gameplay-mechanics`, `skills/level-design`, `skills/particle-systems`, `skills/optimization-performance`, `skills/memory-management`, `skills/synchronization-algorithms`.
- Narrative/community/content: `skills/epic-story-orchestrator-zh`, `skills/content-creation`, `skills/humanizer-zh`, `skills/xiaohongshu-note-analyzer`.
- Browser/visual/content tools: `skills/agent-browser`, `skills/gpt-image-2`.
- Visual companion / Image2 target workflows are optional evidence, not universal gates. They may be used inside an existing task/worktree as visual target and screenshot-comparison evidence, but cannot replace implementation, real native/browser screenshots, interaction smoke, QA evidence, or PR review. Screenshot-only previews count as stable visual-comparison evidence, not real interaction coverage.

If a specialist skill is used, TPM must still bind it to the same owner, GitHub-backed task, canonical worktree, and PR chain through the subagent slice contract. TPM may route to specialist skills, but the specialist role owns the professional conclusion.

### 1.2.1 Friction Controls After Task Truth
The always-bootstrap rule protects traceability, but it must not turn every
small question into a heavyweight workflow.

- Once a request is already inside the bound task/worktree, pure fact lookup,
  path lookup, command-output restatement, or mechanical evidence collection may
  use the objective-fact fast path: TPM gathers the evidence, answers directly,
  and records a short GitHub issue evidence note only when the fact materially
  affects task truth, PR evidence, or a future decision.
- Read-only professional/domain questions still require a matching role slice,
  but the default slice should be bounded and time-boxed: one concrete question,
  explicit evidence paths, `findings/no_findings` or verdict return, and no file
  edits unless the task is routed into execution.
- Do not dispatch a professional slice when the user is only asking for a
  mechanical GitHub status, file path, command output, or exact current fact that
  TPM can verify directly inside the bound task.
- Do not create a second parent/planning surface to answer a small follow-up
  inside an existing bound task; append the follow-up evidence to the current
  task unless it changes owner, scope, or PR chain.
- Use the GitHub Project `Module` field, mirrored in
  `.pm/github-project-sync/tasks.json`, as the default large-module marker for
  ordinary task grouping, reporting, and parallel work queues. It is a small
  enum, not a free tag or owner-role substitute. Current allowed values are
  `engineering`, `game-strategy`, `visualization`, and
  `chain-world-state-substrate`.
- Do not create a separate parent/planning surface merely to label a task's
  large module. Use a normal task with `module` unless the user explicitly asks
  for a separate coordination task.

### 1.2.2 Learning Intake / Loop Closeout
Loop engineering in this repository means each task should close the smallest
useful feedback loop: answer the immediate question, preserve reusable learning
at the right weight, and avoid promoting every observation into a new task.

After task truth exists, TPM and professional roles use this decision ladder
whenever a task produces a discovery, follow-up idea, repeated friction point,
or reusable failure signature:

1. **No-op**: choose this when the observation is transient, already captured by
   the current command output, or has no likely reuse value.
2. **Short GitHub issue evidence note**: choose this when the observation changes the
   current task's route, verification interpretation, PR evidence, or handoff,
   but should not outlive the task as a separate signal.
3. **Reflection signal**: choose this for useful follow-up ideas, repeated
   workflow friction, possible debt, or cross-task learning that is not yet
   ready to become committed task truth. Use
   `./scripts/pm/capture-todo.sh --source-ref <path> --summary "<text>"`.
   By default this creates a GitHub-backed `source_type=reflection` intake issue
   only and must not create a candidate task unless `--create-task` is
   explicitly selected. Local reports may use the ignored generated mirror
   `.pm/github-project-sync/intake-signals.json`; it is not PR truth.
4. **Task-scoped `working_memory`**: choose this when the current task needs a
   structured temporary memory or transcript-derived summary before closeout.
   `working_memory` supplements GitHub task issue evidence comments; it never
   replaces task truth.
5. **Candidate task or memory promotion**: choose this only after owner review
   when the signal or working memory is stable enough to become executable
   backlog work or long-term role memory. Promotion must preserve source refs
   and must not bypass owner, role, PRD/project, or GitHub-backed task truth.

### 1.2.2.1 Detailed Game Design Documentation Method
When a task asks to make game design docs "very detailed", "complete", or able
to express all game details, route it as a design-bible methodology task rather
than a simple doc expansion.

- Keep the module root docs as navigation and current-truth surfaces:
  `doc/game/README.md` routes readers, `doc/game/prd.md` holds the active
  gameplay baseline and authority boundary, `doc/game/project.md` holds current
  execution state, and `doc/game/prd.index.md` provides exact lookup.
- Do not solve detail gaps by moving every rule, matrix, sample, or history
  into the root PRD. Detailed rules belong in topic PRD/design/project triplets
  or explicitly labeled evidence/runbook/checklist supplements.
- A detailed game-design topic is not ready until it can answer:
  player promise, player verbs, loop timing, state model, resource/economy
  rules, failure/recovery, feedback surfaces, edge cases, balance risks,
  implementation authority, QA/playtest validation, and release-claim boundary.
- Topic additions or promotions must update the reachable tree in the same
  change set: relevant root baseline row, `gameplay/README.md` topic cluster,
  `prd.index.md` lookup row, and `project.md` only when current execution or
  gate status changes.
- Use professional slices for the design ownership:
  `producer_system_designer` owns product/system promises and stage boundaries;
  `gameplay_designer` owns player verbs, loops, progression, economy feel, and
  balance risks; `game_visual_interaction_designer`, `runtime_engineer`,
  `agent_engineer`, `viewer_engineer`, `qa_engineer`, and `liveops_community`
  join when the topic touches their surfaces.
- The verification expectation for a design-bible task is traceability, not
  code execution by default: every player-facing claim should link to a
  PRD-GAME id or topic section, an owning role, a validation path, and the
  evidence tier needed before it can affect release or public claims.

Learning intake is not a new mandatory gate before every answer. It is a
closeout habit for moments where the task produced reusable knowledge. When the
right answer is no-op or a short note, do not add extra process.

The minimum record for a micro loop inside an already-bound task is:
question or observation, evidence path or command, answer or decision, and
whether it changes task truth. Use the full bootstrap/router packets only when
the owner, scope, route, professional slice plan, or PR chain actually changes.

Same-thread continuation does not re-run a heavyweight bootstrap when the
current request is a direct continuation of the already-bound task. TPM must
still verify the worktree/task/issue binding, record only the new route or
evidence when it changes task truth, and create a new task only when owner,
scope, module, or PR chain changes.

### 1.2.3 GitHub Project-Backed PM Contract
GitHub Issues + GitHub Project are the authoritative project-management
surface for oasis7 tasks. Local files under `.pm/github-project-sync/` are
generated mirrors/caches for scripts and audits, not a parallel task queue or
per-PR truth artifact.

- GitHub Issue is the task collaboration envelope.
- GitHub Project item fields are authoritative for active queue/status views:
  module grouping, priority, PM status, workflow phase, blocked/ready/PR-watch
  cockpit views, and task-to-issue/project-item mapping.
- `Task UID` remains the stable internal identity. GitHub issue numbers and
  Project item IDs are external object handles, not replacements for `task_uid`.
- `.pm/github-project-sync/tasks.json`, when generated locally, is a
  deterministic mapping cache from `task_uid` to issue/project item handles. It
  is not required to be committed in ordinary task PRs; scripts must tolerate it
  being absent or refresh it from GitHub/task issue evidence.
- `.pm/github-project-sync/task-archive.jsonl` is the immutable repo-local
  archive for historical task metadata and evidence records. It is an audit
  bridge, not a planning queue.
- Lifecycle wrappers `new-task.sh`, `move-task.sh`, `append-execution-log.sh`,
  `workflow-report.sh`, `task-closeout.sh`, and `claim-ready.sh` use GitHub
  Issues/Project as task truth and may update a local generated mapping cache
  when present or explicitly refreshed.
- Execution evidence is recorded in GitHub task issue evidence comments.
- role memory, task-scoped `working_memory`, signals, stage/gate state, and
  this workflow source-of-truth remain repo-local unless a later source-of-truth
  update explicitly migrates them.

Project field taxonomy:

| Field | Owner | Meaning | Allowed / expected values |
| --- | --- | --- | --- |
| `Module` | TPM during task creation/routing | Large work queue and reporting group, not owner role or free tag | `engineering`, `game-strategy`, `visualization`, `chain-world-state-substrate` |
| GitHub Project built-in `Status` | TPM and Project views | Human cockpit lane for day-to-day queue visibility | `Todo`, `In Progress`, `Blocked`, `Ready / PR`, `PR Watch`, `Done` |
| `PM Status` | PM lifecycle scripts | Deterministic lifecycle state used by helpers/audits | `candidate`, `committed`, `blocked`, `ready`, `pr_watch`, `done`, `deferred` |
| `Workflow Phase` | Workflow helpers | Current workflow stage, orthogonal to queue lane | `bootstrap`, `planning`, `execution`, `verification`, `pre_pr_review`, `pr_watch`, `closeout`, `done` |
| Priority | Owner / TPM | Scheduling priority, not severity | repo-defined `P0`..`P3` values |

Scripts that sync Project state must keep GitHub built-in `Status`, custom `PM
Status`, and `Workflow Phase` aligned through deterministic mapping:
`candidate -> Todo/execution`, `committed -> In Progress/execution`,
`blocked -> Blocked/blocked`, `ready -> Ready / PR/closeout`,
`pr_watch -> PR Watch/pr_watch`, and `done|deferred -> Done/done`.
`Blocked`, `Ready / PR`, `PR Watch`, and `Done` are cockpit lanes, not modules
or owner roles.

Deterministic script contract:

- `./scripts/pm/github-project-workflow.sh ... sync` applies mapping/archive
  task metadata to GitHub Issues/Project items idempotently.
- `./scripts/pm/github-project-workflow.sh ... audit` verifies the selected
  task set, mapping, and GitHub Project item/field state agree. Its default
  path must be selected-task / mapping-targeted and must not list the full
  Project during ordinary task closeout or PR readiness checks. It loads task
  truth from archive + mapping and fails if local task-file artifacts reappear.
  For a single task, use
  `./scripts/pm/github-project-workflow.sh audit --task-uid <TASK-UID> --json`;
  the helper reads `repo`, `project-owner`, and `project-number` defaults from
  `.pm/github-project-sync/tasks.json` when they are present.
- `./scripts/pm/github-project-workflow.sh ... step3-gate` is the full
  historical coverage audit for the GitHub Project/mapping/archive set.
- `./scripts/pm/github-project-retire-tasks.sh --delete` maintains
  `.pm/github-project-sync/task-archive.jsonl`; if the archive already exists,
  it must preserve existing records and upsert only the task UIDs represented by
  the current input set.
- `./scripts/pm/github-project-task.py` is the active task lifecycle adapter:
  create issue/project task, append evidence comments, move Project status, and
  close task issues after fresh verification-backed `done` moves.
- `./scripts/pm/task-closeout.sh` defaults to `ready` / `ready_for_pr`
  closeout. Final `done` is reserved for post-PR merge/cleanup closeout or an
  explicitly non-PR task and still requires verified `task_complete` evidence;
  `done` closeout must update issue metadata, set GitHub Project task fields to
  `Done` / `done` / `done`, and close the GitHub task issue.
- `./scripts/prepare-task-pr.sh --create` records the created PR URL and moves
  the task to `pr_watch` when GitHub-backed mapping exists.
- `scripts/prepare-task-pr.sh` must read passed local role review packets from
  GitHub task issue evidence comments and mapping-backed task truth.
- `scripts/prepare-task-pr.sh --create` must include a GitHub auto-close
  keyword for the bound GitHub task issue in the PR body, or reject an explicit
  PR body file that omits the linkage. The normal generated linkage is
  `Closes #<task-issue-number>` and is separate from `Task UID`.
- `scripts/pm/audit-pr-watch-issues.sh --close` is the remedial post-merge
  audit for GitHub-backed tasks whose recorded PR is already merged but whose
  PM task issue/body/Project state still says `pr_watch`. The PR body
  auto-close keyword is only a missed-closure backstop; the audit remains the
  PM lifecycle synchronizer. It may synchronize only PM task issues whose body
  contains the task marker, `status: pr_watch`, a recorded `pr_number` whose
  GitHub PR state is merged, and existing issue comments with passed pre-PR
  local role review plus verified ready closeout/claim evidence. The audit is
  remedial PM metadata synchronization, not independent professional completion
  judgment. For OPEN issues it writes remedial sync evidence, updates
  Project/body state to `done`, then closes the issue; for already CLOSED issues
  it writes remedial sync evidence and updates Project/body state without
  issuing another close. Missing task issue mapping, missing Project Done fields,
  missing existing review/ready evidence, unmerged PRs, non-`pr_watch` statuses,
  or manual hold markers are fail-closed blockers.
- `./scripts/pm/fallback-evidence.sh` is the replay/audit helper for temporary
  `.pm/scratch/<TASK-UID>/fallback-evidence/` packets; unreplayed fallback
  packets do not satisfy task truth and are rejected by PR-readiness lint.
- All future GitHub-backed create/move/report/closeout helpers must use
  deterministic `gh`/GitHub API paths, preserve or recover the `task_uid`
  mapping, and refuse ambiguous duplicate mappings.

## 2. Responsibility Boundary
- `tpm`: default main Agent / workflow coordinator / canonical integrator only; owns phase decision, role allocation, subagent dispatch, integration order, task-truth writeback, fresh-verification gate coordination, completion-claim coordination, and PR chain coordination.
- TPM is not a professional execution role. TPM must not be the source of domain/professional analysis, implementation, verification judgment, code review judgment, product/design judgment, runtime/wasm/viewer/agent/QA/repository-health judgment, or liveops/community messaging.
- Professional/domain work must be done by the matching bounded subagent slice. This includes:
  - product/system design by `producer_system_designer`
  - gameplay design by `gameplay_designer`
  - game visual direction, interaction feel, player-facing screen flow, and visual readability by `game_visual_interaction_designer`
  - runtime/gameplay/server logic by `runtime_engineer`
  - blockchain/node operations, deployment choreography, upgrade/rollback drills, fleet health baselines, and node runbooks by `blockchain_ops_engineer`
  - WASM/platform/ABI work by `wasm_platform_engineer`
  - agent behavior/prompt/provider work by `agent_engineer`
  - Viewer/Web/UI work by `viewer_engineer`
  - verification strategy, test evidence, and release blocking judgment by `qa_engineer`
  - repository health stewardship, documentation/code alignment, semantic clarity, bug-risk surfacing, and technical-debt triage by `repository_health_engineer`
  - external messaging, community feedback, incidents, player promises, release notes, and channel runbooks by `liveops_community`
- professional role subagents provide bounded slices only (analysis/implementation/verification/review/liveops messaging) and must return artifacts to the TPM owner chain.
- TPM may perform mechanical orchestration edits to workflow governance surfaces, task logs, integration notes, and PR plumbing. If the work requires a professional conclusion, TPM must dispatch the matching role slice first and attribute the conclusion to that slice/evidence.
- For every request, TPM planning, TODO decomposition when needed, subagent slice contracts, and integration order are task execution truth and must be written to GitHub task issue evidence comments before the delegated work begins.
- Every user request must enter the standard worktree flow before any substantive handling begins, including chat-only answers, read-only inspection, fact lookup, professional slice dispatch, implementation, verification, review, and external messaging.
- The only allowed pre-bootstrap work is mechanical enough to create or enter the task truth: inspect current git/worktree state, choose or confirm the task/worktree, and run the bootstrap helper.
- Do not first classify a request as "read-only", "chat-only", "pure fact lookup", or "professional judgment" to decide whether task/worktree truth is needed. That classification happens only after bootstrap, inside the bound task/worktree, and only controls whether TPM can answer from objective evidence or must dispatch a professional slice.
- Read-only/chat-only requests still split by judgment type after task truth exists:
  - Pure fact lookup, path lookup, command-output restatement, or mechanical evidence collection may be handled by TPM inside the bound task worktree, as long as the answer does not present a professional/domain conclusion.
  - Read-only professional/domain questions must be dispatched to the matching bounded professional role slice before the answer is presented as authoritative. Examples: "does viewer have a performance collection/evaluation mechanism", "is this QA evidence release-blocking", "what runtime design risk is present", "is this gameplay loop balanced/readable", "is this documentation/code contract drifting", "what node-ops risk is present in this rollout", or "how should LiveOps message this incident".
  - Such read-only professional slices require the same GitHub-backed task and canonical task worktree as any other request. Their required sink is GitHub task issue evidence comments, plus the role-tagged user-facing answer.
  - TPM may gather raw files, commands, or repo context before dispatch only after bootstrap; the final user-facing answer must label TPM synthesis separately from professional role conclusions and cite the role/evidence that owns each professional conclusion.
- Canonical truth per user request must remain single-threaded:
  - one owner role
  - one GitHub-backed task
  - one canonical worktree
  - one PR chain

## 3. Gates
### 3.1 Required Gates (must pass)
1. **Task truth gate**: isolated worktree + bound GitHub-backed task + owner role confirmed.
2. **Planning gate**: PRD/project/execution truth aligned for scope and verification entry; TPM TODOs and any subagent slice plan are recorded in GitHub task issue evidence comments before execution.
3. **Execution gate**: atomic step evidence captured (`Action`, `Validation Command`, `Expected Result`, `Actual Result`, plus blocker fields when needed).
4. **Fresh verification gate**: current-round verification success before completion claims.
5. **Pre-PR local role review gate**: fresh local subagent review by all involved relevant roles, with actionable findings addressed or explicitly rejected with evidence.
6. **Closeout gate**: closeout metadata + task status transition + lint/governance checks + commit + PR creation.
7. **Post-PR watch/merge gate**: unless the PR is explicitly opened only to access manual-trigger packaging/release CI, watch normal PR required checks, mergeability, and PR comments/review threads to completion, fix failures through the review/debug loop, then merge and clean up. `REVIEW_REQUIRED` is informational and is not a blocking item by itself. `mergeStateStatus=BEHIND` is also informational by itself: if the PR stays mergeable, has no actionable comments/requested changes/blocking threads, and the repository/GitHub merge path accepts the merge without a local branch sync, the normal path may merge directly without rebasing first. If `mergeStateStatus=BLOCKED` is caused only by missing review approval, and the user/task policy explicitly authorizes skipping that approval, the normal path may use the repository's admin merge path after re-checking required checks, mergeability, requested changes, and PR comments/review threads. Normal task PRs must carry a GitHub auto-close link to the bound task issue; after merge, the task issue must be closed by the auto-close link, the final `done` closeout path, or the `audit-pr-watch-issues --close` remedial audit when an already-merged task still has stale `pr_watch` PM issue/body/Project state.

### 3.2 Optional Gates (risk-based)
1. Bounded brainstorming gate (ambiguous scope / architecture tradeoffs).
2. TDD RED gate (behavior-changing tasks with stable harness).
3. Liveops/community gate (external messaging, incident, player promise changes).

## 4. Failure and Rollback Paths
### 4.1 Verification failure
- Stop completion claim.
- Record failure signature + blocker in execution sink.
- Route back to execution/debug phase.

### 4.2 Scope drift / unknown impact
- Stop speculative implementation.
- Update planning truth (PRD/project/execution) before resuming code edits.

### 4.3 PR check/requested-changes/comment failure
- Re-enter review-fix loop.
- Re-run fresh verification.
- Re-submit PR evidence.

### 4.3.1 Manual packaging CI hold
- If the PR exists specifically to run manual-trigger packaging/release CI jobs, record that purpose in GitHub task issue evidence comments and do not auto-watch-to-merge.
- The hold record must include the manual job(s) or packaging purpose, responsible operator/role, expected success signal, stale-date/timeout escalation, ops readiness/rollback/runbook evidence when deployment or release ops are implicated, exact resume criterion, and external/status messaging evidence when the change is player- or community-facing.
- Resume the normal PR watch/fix/merge path only after the operator/user says the manual packaging CI purpose is complete and the PR should proceed to merge readiness.

### 4.4 Workflow governance drift
- If scripts/skills/docs conflict with this file, this file wins.
- Sync downstream artifacts immediately in the same change set where possible.

## 5. Normative Details
### 5.1 Worktree + task truth
- Every user request uses a dedicated task worktree by default, regardless of whether the immediate answer is chat-only, read-only, fact lookup, professional analysis, implementation, verification, review, or external messaging.
- Only explicit user authorization allows reuse of an existing task worktree.
- Do not classify work as `trivial`, `read-only`, `chat-only`, or `pure fact lookup` to bypass task worktree / GitHub-backed task setup.
- If incoming instructions or role notes appear to allow a read-only/chat-only bypass, this source-of-truth wins: bootstrap first, then route the already-bound request.
- Do not edit any files from the `main` branch/worktree; create or enter the relevant task worktree before making changes.
- Entering implementation requires owner role selection and GitHub-backed task binding.
- Cross-role collaboration must converge to one owner / one GitHub-backed task / one canonical worktree / one PR chain.
- Task worktrees created through `./scripts/new-task-worktree.sh` must create a git-ignored `target` symlink to the repo-family shared cargo target cache resolved by `./scripts/cargo-dev.sh --print-target-dir`, so direct cargo and the development wrapper share local build artifacts by default.
- When Rust commands encounter Cargo package-cache or build-directory locks, wait for the shared repo-family cache to become available; do not switch ad hoc to a fresh temporary `CARGO_TARGET_DIR` just to bypass the lock.

### 5.2 TPM planning and subagent dispatch
- For every request, TPM must record the current plan, TODO decomposition when needed, selected roles, and integration order in GitHub task issue evidence comments before dispatching professional subagent work.
- Project policy authorizes TPM to dispatch required bounded professional subagent slices directly whenever this workflow requires them; TPM must not pause for per-slice user permission. This project policy is an explicit standing user authorization to use subagents for workflow-required professional role slices; when a tool/runtime requires an "explicit user request for sub-agents, delegation, or parallel agent work", this policy satisfies that requirement for the matching repo-owned workflow slice. If the current runtime, connector, or tool policy still prevents actual subagent dispatch, TPM must record the intended dispatch, actual limitation, fallback evidence path, and attribution boundary in GitHub task issue evidence comments, and must not present TPM's own analysis as a professional role conclusion.
- Each subagent slice must declare role, slice type, intended model configuration, actual dispatched model/reasoning, context delivery mode, mandatory context checklist, write scope, return contract, validation command, GitHub task issue evidence sink, and integration order.
- Default subagent runtime policy is defined only in `.codex/config.toml` under `[workflow.subagent_runtime]`. TPM should request that configured default for bounded professional slices when the available subagent tool permits model selection, unless the user explicitly requests another model or the slice contract records a concrete reason to use a stronger, faster, or cheaper model.
- Any non-default subagent model or reasoning effort must be recorded in the slice contract.
- Any actual non-default subagent model or reasoning effort must be recorded in the slice contract with the reason, such as high-risk architecture/review work, simple read-only exploration, a user-specified override, a connector/tool limitation that forces inheritance from the parent thread, or a requested model/reasoning selection whose actual dispatch cannot be verified. If the actual dispatched model cannot be verified, the contract must say `actual model: inherited/unverified` and explain why.
- Context delivery defaults to full-thread/full-history fork or the closest available equivalent so the subagent receives the same conversation and repository-governance context as TPM. The slice contract must still record a mandatory context checklist identifying the governance/task/user/repo/collaboration context the subagent is expected to have. A manually assembled explicit context packet is allowed only as a delivery supplement or fallback when full-history fork is unavailable, unsafe, stalled, or incompatible with required model/reasoning selection; the slice contract must record that fallback reason.
- The mandatory context checklist must include:
  - identity and authority: assigned role, role card path, owner role, and TPM integration owner
  - workflow governance: `AGENTS.md`, `doc/engineering/workflow/source-of-truth.md`, and the selected workflow skills
  - task truth: current GitHub issue, GitHub Project item/status, `.pm/github-project-sync/tasks.json` mapping record, canonical worktree, branch, base ref, and PR link/status when present
  - user intent and acceptance target: original request summary, current TODO, explicit non-goals, and done/verification expectations
  - scoped repo context: relevant `prd.md`, `project.md`, handoff, changed paths, current diff or evidence summary, and known constraints such as `third_party` read-only boundaries
  - collaboration boundary: sibling slices, write-scope conflicts, integration order, allowed commands, return contract, and formal sink
- `AGENTS.md` and the assigned role card are mandatory inputs for implementation, verification, review, or domain-specialist slices. A narrow read-only explorer slice may omit the role card only when the slice contract records the exemption reason and the exact files to inspect; it still runs after task/worktree bootstrap and records its sink in GitHub task issue evidence comments.
- TPM read-only exploration is allowed only to gather routing context, inspect task truth, or integrate returned evidence. It must not be reported as a professional finding unless a matching professional role slice owns or verifies that finding.
- TPM user-facing summaries must distinguish procedural synthesis from professional conclusions. Professional conclusions must be traceable to subagent artifacts, execution evidence, handoff, project/prd records, or PR evidence.
- Project docs, handoff files, signals, memory, and PR evidence may supplement GitHub task issue evidence comments, but they do not replace them for task execution truth.
- If GitHub task issue comments are temporarily unavailable, TPM may write a temporary fallback packet under `.pm/scratch/<TASK-UID>/fallback-evidence/<timestamp>.md` and must record the intended GitHub issue target, reason for fallback, attribution boundary, and replay command. Fallback packets unblock evidence capture only; they do not satisfy task truth, pre-PR review, closeout, or completion claims until replayed to the GitHub task issue comments.
- If the plan changes during execution, TPM must append a GitHub issue evidence comment before continuing the changed work.
- Pre-task discoveries, loose TODOs, and follow-up ideas found before an owner decides to create a GitHub-backed task should be captured with `./scripts/pm/capture-todo.sh --source-ref <path> --summary "<text>"`. This records a GitHub-backed `source_type=reflection` intake issue by default and must not be treated as committed task truth until explicitly promoted with `--create-task` or another task-creation path. The retired `.pm/inbox/signals.jsonl` file must not be recreated; its last committed contents are preserved in `.pm/github-project-sync/signal-archive.jsonl` for migration/audit only.
- After task truth exists, use the section 1.2.2 learning-intake ladder for
  discoveries and follow-ups: no-op, short GitHub issue evidence note, reflection
  signal, task-scoped `working_memory`, or owner-reviewed candidate task/memory
  promotion. Do not skip directly from a lightweight observation to committed
  task truth unless the owner explicitly selects that promotion.

### 5.2.1 Read-only specialist routing
- The task/worktree decision and the professional-slice decision are intentionally decoupled:
  - Task/worktree truth is required for every request.
  - Professional judgment controls whether a matching bounded role slice is required after bootstrap.
- Therefore, a read-only request must enter `default-workflow-bootstrap` first and may still require a professional role slice.
- Minimal read-only specialist slice contract:
  - role and slice type (`read_only_analysis`, `verification_judgment`, `review_judgment`, or `liveops_messaging`)
  - intended model configuration, defaulting to the `Default subagent runtime` policy unless an override reason is recorded
  - actual dispatch model/reasoning, or `inherited/unverified` with the connector/tool limitation, stalled default context delivery, or unverifiable actual dispatch reason
  - context delivery mode, defaulting to full-thread/full-history fork unless a recorded fallback reason requires explicit context
  - exact question to answer and explicit non-goals
  - scoped files/commands/evidence to inspect
  - return contract with conclusion, evidence, uncertainty, and whether repository writeback is recommended
  - attribution rule for the final answer
- Read-only specialist slice contracts must be recorded in GitHub task issue evidence comments; chat/thread text may supplement but not replace the GitHub task sink.
- Pure evidence questions may be answered by TPM directly only after bootstrap and only when the user asks for an objective fact such as "does this file exist", "what command output says", or "which paths match this search".
- If a read-only specialist slice recommends changing repository state, TPM continues in the already-bound canonical task worktree and records the changed route in GitHub task issue evidence comments before applying changes.

### 5.3 Execution evidence
- Atomic steps should be recorded with `Action / Validation Command / Expected Result / Actual Result`.
- If blocked, also record `Blocker / Next Action`.
- These fields are mandatory for GitHub task issue evidence entries.
- Record evidence when it changes route, scope, status, claims, verification
  interpretation, blocker state, review findings, or user-visible decisions.
  Routine reads/searches that only support TPM routing can stay in the local
  transcript unless they become part of a claim or handoff.

### 5.4 Claim / closeout chain
- Before completion claims, run fresh verification (prefer `./scripts/pm/claim-ready.sh --claim-type <type> --verify-command "<cmd>"` when applicable).
- A verification epoch starts after the latest code/doc/script change that can
  affect the claim, after any valid review finding fix, or after any branch sync
  that changes the reviewed diff. Completion, ready-for-PR, and ready-to-merge
  claims must cite commands from the current epoch. Earlier successful output
  may be background context only.
- Do not move the task to final closeout / `done` before pre-PR local role review has passed when the task is on a PR path. The order is: fresh verification -> pre-PR local role review -> address findings -> final closeout/status packet -> commit -> PR preflight/create.
- Closeout should run `./scripts/pm/task-closeout.sh --role <owner_role> --task-uid <TASK-UID> --verify-command "<fresh cmd>"` (or equivalent manual chain) after valid local role-review findings are resolved. If a helper must be run earlier for a readiness packet, the GitHub issue evidence comment must label that packet as readiness evidence rather than final done state.
- For `done` closeout, fresh verification must be from the current round and post-review findings must be addressed or explicitly rejected with evidence. The final status move must preserve or recover the task mapping, update GitHub Project `Status` / `PM Status` / `Workflow Phase`, update the GitHub issue task body to `status: done`, and close the GitHub issue as completed.

### 5.5 PR and review chain
- Standard path is local role-subagent review + GitHub PR + required checks + PR comment/thread closeout + mergeability.
- The workflow no longer requests Copilot review as a PR helper step.
- Before PR creation, TPM must create or dispatch fresh local review subagents for every involved relevant professional role in the diff scope. Role sufficiency is based on changed paths, role ownership, task slice history, user-facing claims, verification claims, and explicit skip rationale for adjacent roles. Include `producer_system_designer` when scope, product contract, user promise, acceptance, or system-level semantics change; include `gameplay_designer` when gameplay rules, progression, balance, encounter/resource loops, or player verb semantics are touched; include `game_visual_interaction_designer` when visible UI/gameplay presentation, visual direction, interaction feel, player-facing screen flow, screenshot/visual-review surfaces, accessibility/readability, or UI-heavy claims are touched; include `runtime_engineer` when runtime/server/simulation/gameplay enforcement, replay, recovery, checkpoint, long-run behavior, or `crates/oasis7*` runtime paths are touched; include `blockchain_ops_engineer` when deployment, node ops, topology/inventory, service/host contracts, health baselines, upgrade/rollback/restore drills, packaging/release ops, or operator-facing runbooks are touched; include `wasm_platform_engineer` when `crates/oasis7_wasm_*`, builtin wasm modules, ABI/schema, manifest/hash, wasm build/receipt, wasm determinism workflows, or `doc/world-runtime/wasm/*` are touched; include `agent_engineer` when agent behavior, prompts, provider contracts, model/runtime config, subagent dispatch contracts, or agent tooling are touched; include `viewer_engineer` when Viewer/Web/UI/WebGPU/browser validation paths are touched; include `qa_engineer` when the PR changes verification helpers, testing docs, release/readiness claims, test strategy, or evidence sufficiency; include `repository_health_engineer` when the diff changes cross-cutting architecture, shared workflow surfaces, docs/code contracts, large refactors, repeated bug signatures, workflow scripts/skills, or known technical-debt boundaries; include `liveops_community` when external messaging, incidents, player promises, community feedback, release notes, or channel runbooks are touched. Do not add `qa_engineer` only because a PR has any changed file; add it when verification or evidence semantics are part of the changed surface or claim.
- `scripts/prepare-task-pr.sh --create` must mechanically reject a passed review packet when changed-path inference identifies required roles that are missing from `Review Roles`. This script check is a minimum backstop; TPM remains responsible for adding roles implied by task history and user-facing claims that path inference cannot see.
- Verification must map to the changed surface, not only to one generic command. Gameplay changes need playability/economy/motivation-loop evidence tied to `doc/game` truth; runtime changes need the relevant cargo checks/tests plus replay/recovery/checkpoint/long-run evidence where applicable; WASM ABI/platform changes need support-crate/executor tests and, for publishable or builtin module pipeline changes, deterministic build/gate evidence or an explicit defer-to-GitHub/manual evidence packet; UI/player-facing changes need S6 screenshot/model-visual-review evidence or an explicit visual-evidence exemption; release/manual packaging changes need first-class Ops Evidence covering readiness, rollback/runbook, and success/resume evidence; player- or community-facing changes need first-class LiveOps Evidence covering messaging, release-note/status, and audience impact.
- Each local role review must return `findings` or `no_findings` plus `residual_risk`. TPM must fix valid findings or record why a finding is stale/rejected with code or doc evidence before PR creation.
- `scripts/prepare-task-pr.sh --create` must refuse to create the PR unless the GitHub issue evidence comments contain a passed pre-PR local role review packet for the source worktree. The packet marker is:
  - `Pre-PR Local Role Review: passed`
  - `Task UID: <task_uid>`
  - `Source Worktree: <absolute path>`
  - `Source Branch: <branch>`
  - `Source Head: <reviewed git sha; must be current source head or an ancestor whose later changes are only the task review evidence files or generated PM task registry/backlog views>`
  - `Comparison Ref: <base ref>`
  - `Reviewed Changed Paths: <semicolon-separated paths or diff summary ref>`
  - `Review Package: <path to review package or n/a with reason>`
  - `Role Selection Basis: <changed paths + task slice history + explicit includes/skips>`
  - `Review Roles: <comma-separated roles>`
  - `Review Evidence: <per-role section or handoff refs>`
  - `Review Verdicts: <per-role scope/spec compliance verdict + role quality/risk verdict>`
  - `Review Findings Disposition: addressed` or `Review Findings Disposition: no_findings`
  - `Finding Disposition Evidence: <fix refs or rejected/stale evidence refs>`
  - `Verification Matrix: <changed surface -> required evidence -> observed evidence or explicit deferral>`
  - `Visual Evidence: <screenshot/model visual review paths or n/a with exemption reason>`
  - `WASM Evidence: <support crate/determinism evidence or n/a with reason>`
  - `Ops Evidence: <readiness/rollback/runbook/operator evidence or n/a with reason>`
  - `LiveOps Evidence: <messaging/release-note/status/community evidence or n/a with reason>`
  - `Residual Risk: <text>`
  - `Slice Ledger: <path to slice ledger or n/a with reason>`
- Pre-PR local role review should use file-based review packages for non-trivial diffs. `./scripts/pm/review-package.sh --base <ref> --head <ref> --task-uid <TASK-UID>` writes the commit list, stat summary, and contextual diff under ignored `.pm/scratch/<TASK-UID>/review-packages/`; GitHub issue evidence comments record only the path and summary. Use `n/a` only when the diff is empty or the review target is not a git diff, and record the reason.
- For small workflow/docs-only diffs, TPM may use `scripts/pm/record-pre-pr-review.sh` or an equivalent packet generator to avoid hand-copy errors, but the generated packet must still record role-selection basis, explicit `n/a` exemption reasons, observed verification, and residual risk.
- Pre-PR local role review verdicts must distinguish scope/spec compliance from role quality/risk for each reviewer role. The role remains the professional owner; this dual-verdict structure is a packet format, not permission to replace involved-role review with a generic reviewer.
- Long multi-slice tasks should maintain a lightweight slice ledger with `./scripts/pm/slice-ledger.sh --task-uid <TASK-UID> ...`. The ledger is an ignored JSONL resume map for slice status, artifact paths, verdicts, residual risk, and next action. GitHub task issue evidence comments remain canonical task truth and must link to the ledger rather than relying on it as the only sink. When a review dispatch needs more roles than the current subagent runtime can run concurrently, TPM must batch the roles, record batch order and priority, record timeout/no-payload policy before dispatch, and distinguish partial results from all-role completion.
- Before merge, explicitly check PR comments and review threads. If any actionable comments or unresolved blocking threads exist, fix + re-verify + resolve or answer them before the merge claim.
- After PR creation, TPM must record the PR purpose decision:
  - `normal_pr_ci_watch`: default. Use this unless the user or task truth says the PR was opened only to access manual-trigger packaging/release CI jobs.
  - `manual_packaging_ci_hold`: allowed only when the PR is explicitly created for manual-trigger packaging/release CI. Record manual job(s) or packaging purpose, responsible operator/role, expected success signal, stale-date/timeout escalation, ops readiness/rollback/runbook evidence when deployment or release ops are implicated, external/status messaging evidence when player- or community-facing, and the exact resume criterion. Stop before auto-merge until the operator/user resumes the normal path.
- For `normal_pr_ci_watch`, TPM continues without waiting for another user prompt: watch the PR's normal required checks, mergeability, review decisions, and PR comments/review threads. `REVIEW_REQUIRED` is a status signal to report, not a blocker. If checks fail, review requests changes, actionable comments appear, unresolved blocking review threads remain, or the merge API/branch protection rejects the merge for reasons other than review approval, route through the fix loop, rerun fresh verification, push fixes or answer/resolve comments, and continue watching.
- If GitHub reports `mergeStateStatus=BEHIND`, treat it as a branch-sync signal, not an automatic blocker. When the PR is still mergeable and the repository/GitHub merge path accepts the merge without requiring a local branch sync, TPM may merge directly after the same checks/comments/thread closeout steps. If GitHub refuses because the branch must be updated first or because a conflict/non-mergeable state exists, sync the branch to the current base, rerun fresh verification as needed, push, and continue watching.
- If GitHub reports `mergeStateStatus=BLOCKED` / `REVIEW_REQUIRED` only because review approval is missing, and the current user request or task truth explicitly authorizes skipping review approval, TPM may use the repository's admin merge path as part of the normal PR watch/merge flow. Before doing so, TPM must re-check that required checks pass, the PR is mergeable, no requested changes remain, PR comments/review threads have been checked, and no actionable comments or unresolved blocking review threads remain. Admin merge must not be used for failed checks, non-mergeable code state, requested changes, unresolved actionable comments/threads, manual packaging CI holds, or unrelated branch-protection failures.
- Once normal required checks pass, the PR is mergeable by the repository/GitHub merge path or by the authorized review-approval admin path above, PR comments/review threads have been checked, and no actionable comments, requested changes, or unresolved blocking review threads remain, merge the PR using the repository's configured merge method, then confirm the bound GitHub task issue and PM state reach `done` or run the post-merge `pr_watch` audit closeout if the merged PR left the issue open or left stale `pr_watch` metadata behind.
- After merge, sync local `main`, clean up task worktree/branch, and leave the GitHub task issue closed or with an explicit evidence comment explaining any intentionally retained manual hold.

## 6. Required Artifacts by Phase
- Bootstrap/Router: decision record in GitHub task issue evidence comments; project or handoff records may supplement it.
- Planning/Dispatch: TPM TODO decomposition and subagent slice contracts in GitHub task issue evidence comments.
- Execution: atomic evidence records per risky step.
- Verification: claim-ready command + output evidence.
- Pre-PR local role review: involved-role subagent review packet, review package path or explicit `n/a`, required-role coverage, per-role dual verdicts, finding disposition, verification matrix, visual/WASM/ops evidence or explicit exemptions, residual risk, and slice ledger path or explicit `n/a`.
- Closeout: closeout command output, task status update, pre-PR local role review evidence, PR linkage, PR purpose decision, CI/review watch evidence, merge evidence, cleanup evidence, and final GitHub task issue closure for `done`.
- Learning intake / loop closeout: only when reusable learning exists, record
  the chosen ladder step and evidence. Reflection signals, working memory, and
  promoted candidate tasks/memory supplement GitHub task issue evidence comments but do not
  replace it.

## 7. Change Log
- **v1.5.6 (2026-07-07)**
  - Split skill-like content into default-loadable `.agents/skills/*/SKILL.md`
    entrypoints and non-default root `skills/` specialist library/archive
    material.
  - Moved professional method skills out of the default `.agents/skills`
    trigger surface and into role-card-referenced `skills/` library entries.
  - Required source-of-truth promotion before any root `skills/` library
    material becomes a default `.agents/skills` trigger again.
- **v1.5.4 (2026-07-01)**
  - Made task-scoped GitHub Project audit copy-pasteable with
    `github-project-workflow.sh audit --task-uid <TASK-UID> --json`, backed by
    mapping-derived Project defaults.
- **v1.5.3 (2026-06-30)**
  - Added Project field taxonomy, same-thread continuation reuse, temporary
    fallback evidence replay rules, and current verification epoch semantics.
  - Renamed mandatory subagent context semantics to `mandatory context
    checklist`; explicit context packets are only fallback/supplement delivery
    artifacts.
  - Tightened pre-PR role sufficiency so QA review is required for verification
    and evidence semantics, not for every changed file by default.
- **v1.5.2 (2026-06-30)**
  - Hardened the GitHub Project-backed PM contract so audit/lint reject local
    task-file artifacts and active evidence uses GitHub task issue comments.
  - Required archive updates to preserve existing records and upsert only the
    task UIDs represented by the current input set.
- **v1.5.1 (2026-06-30)**
  - Completed the GitHub Project PM transition: GitHub Issues/Project plus
    `.pm/github-project-sync/tasks.json` became the active PM path, with
    historical records archived in `.pm/github-project-sync/task-archive.jsonl`.
  - Moved task create/evidence/status/closeout and PR preflight evidence reads
    to the GitHub-backed path.
- **v1.5.0 (2026-06-29)**
  - Introduced the GitHub Project-backed PM migration plan and deterministic
    sync/audit/step3-gate scripts.
- **v1.4.28 (2026-06-27)**
  - Added the Learning Intake / Loop Closeout ladder so task learning can flow
    to no-op, short GitHub issue evidence note, reflection signal, task-scoped
    `working_memory`, or owner-reviewed candidate task/memory without creating
    heavy ceremony for every small loop.
- **v1.4.27 (2026-06-24)**
  - Added friction controls after task truth so objective fact lookups, bounded read-only role slices, and small follow-ups do not become heavyweight workflow by default.
  - Added GitHub Project `module` as the default large-module marker for grouping, reporting, and parallel work queues.
  - Clarified that module-local test evidence does not imply integration or release readiness.
- **v1.4.26 (2026-06-23)**
  - Tightened pre-PR role inference backstops for producer product/system docs, viewer/launcher implementation/docs/scripts, and agent/subagent workflow contract surfaces.
  - Clarified semantic review evidence behavior by distinguishing generic `n/a` from explicit deferral/exemption reasons across runtime, gameplay, visual, ops, and liveops evidence.
  - Added regression coverage for role inference drift, generic `n/a` rejection, and explicit visual/ops/liveops deferral acceptance.
- **v1.4.25 (2026-06-23)**
  - Added first-class `Ops Evidence` and `LiveOps Evidence` pre-PR review packet fields and required semantic evidence checks for inferred professional roles.
  - Tightened PR helper backstops for CI workflow/shared scope planner repository-health review, builtin WASM module platform review, runtime gates for core WASM dependencies, and generated PM task-view post-review allowlisting.
  - Expanded changed-path role inference coverage for producer/product-system, runtime world docs, gameplay simulator docs, visual testing docs, liveops/readme/release/status docs, and ops topology/readiness/preflight/packaging surfaces.
  - Synced manual packaging CI hold requirements across failure/rollback and PR sections so stale/timeout, ops readiness, rollback/runbook, resume, and external/status messaging evidence are consistently required.
- **v1.4.24 (2026-06-22)**
  - Clarified pre-PR local role review before final closeout/commit/PR creation to avoid done-before-review churn.
  - Expanded involved-role selection triggers across all professional roles and required `prepare-task-pr.sh --create` to reject missing required roles inferred from changed paths.
  - Added surface-specific verification matrix expectations for gameplay, runtime, WASM, UI/visual, and manual packaging/release ops evidence.
  - Added batching/timeout policy for large all-role review dispatches and stronger manual packaging hold ownership/resume criteria.
- **v1.4.23 (2026-06-22)**
  - Added file-based review package and lightweight slice ledger artifacts for pre-PR local role review.
  - Required pre-PR role review packets to record `Review Package`, per-role dual `Review Verdicts`, and `Slice Ledger` fields.
  - Clarified that dual verdicts strengthen involved-role review and do not replace oasis7 professional role ownership with a generic reviewer.
- **v1.4.22 (2026-06-18)**
  - Removed the retired `xiaohongshu` automation skill from the specialist skill reachability list while keeping `xiaohongshu-note-analyzer`.
- **v1.4.21 (2026-06-11)**
  - Added `repository_health_engineer` as the professional role for repository health stewardship, documentation/code alignment, semantic clarity, bug-risk surfacing, and technical-debt triage.
  - Extended read-only professional routing and pre-PR local role review selection so repository-health judgment comes from the matching bounded role slice.
- **v1.4.20 (2026-06-08)**
  - Clarified that repo-owned workflow subagent policy is an explicit standing user authorization that satisfies tool-level explicit subagent/delegation request requirements for required professional slices.
- **v1.4.19 (2026-06-08)**
  - Clarified that oasis7 project policy authorizes TPM to dispatch required bounded professional subagent slices without per-slice user permission.
  - Required TPM to record intended dispatch, runtime/tool limitation, fallback evidence path, and attribution boundary when actual subagent dispatch is blocked by the current environment.
- **v1.4.18 (2026-06-08)**
  - Added `./scripts/lint-skills.sh` as the local skill-surface hygiene gate for entrypoint size, trigger-focused descriptions, supporting-file reachability, and core workflow failure-mode sections.
  - Clarified that `writing-repo-owned-skills` verification includes the skill lint gate alongside existing governance checks.
- **v1.4.17 (2026-06-07)**
  - Added `blockchain_ops_engineer` to the formal professional-role roster and read-only specialist routing matrix.
  - Synced canonical role enumerations so handoff and slice-card surfaces stay aligned with the standard role list.
- **v1.4.16 (2026-06-07)**
  - Added `gameplay_designer` as a formal professional role between system/product design and visual/interaction design.
  - Clarified that gameplay-loop, progression, balance, encounter/resource-loop, and player-verb judgments belong to `gameplay_designer`.
  - Extended pre-PR local review role selection guidance to include `gameplay_designer` when gameplay semantics are touched.
- **v1.4.15 (2026-06-04)**
  - Clarified that `mergeStateStatus=BEHIND` is informational by itself and does not force a local rebase when the PR remains mergeable and the repository/GitHub merge path accepts a direct merge.
  - Kept branch sync/rebase required only when GitHub actually refuses the behind branch because an update or conflict resolution is needed.
- **v1.4.14 (2026-06-04)**
  - Defined review-approval-only `mergeStateStatus=BLOCKED` as a normal admin merge path when user/task policy explicitly authorizes skipping review approval.
  - Kept admin merge forbidden for failed checks, non-mergeable code state, requested changes, unresolved actionable comments/threads, manual packaging CI holds, and unrelated branch-protection failures.
- **v1.4.13 (2026-06-04)**
  - Clarified that `REVIEW_REQUIRED` is informational and is not a blocking merge item by itself.
  - Kept actual blockers on required-check failures, requested changes, actionable/unresolved PR comments or review threads, non-mergeable state, and repository/GitHub merge rejection.
- **v1.4.12 (2026-06-04)**
  - Added the post-PR purpose decision: normal PRs proceed into CI/review watch, failure fix loop, merge, and cleanup without waiting for another prompt.
  - Added the manual packaging/release CI hold exception for PRs opened specifically to access manual-trigger packaging CI jobs.
  - Clarified that merge readiness requires an explicit PR comment/review-thread check before merge.
- **v1.4.11 (2026-06-03)**
  - Replaced the optional/risk-based local supplemental review gate with a required pre-PR local role-subagent review gate.
  - Removed the Copilot review request from the standard PR helper flow.
  - Required `prepare-task-pr.sh --create` to verify passed local role review evidence before creating a PR.
- **v1.4.10 (2026-06-03)**
  - Updated the default subagent runtime policy.
  - Consolidated synced guidance, templates, and workflow eval checks to reference the section 5.2 `Default subagent runtime` policy instead of duplicating the concrete model string.
  - Added the `capture-todo.sh` pre-task discovery intake path for loose TODOs and follow-up ideas that should become `reflection` signals before explicit task promotion. Current runtime stores those signals as GitHub-backed intake issues, not `.pm/inbox/signals.jsonl`.
  - 2026-06-08 amendment: moved the concrete default subagent runtime value into the repo-tracked `.codex/config.toml` `[workflow.subagent_runtime]` block so the model configuration has one canonical source.
  - 2026-06-08 amendment: updated the workflow source-of-truth and workflow eval contract to reference the config-backed policy instead of carrying the concrete runtime value in prose.
- **v1.4.9 (2026-06-02)**
  - Clarified that request-type classification cannot happen before task/worktree bootstrap; bootstrap happens first, then read-only/professional routing.
  - Required subagent slice contracts to distinguish intended default model from actual dispatched model, including inherited/unverified connector cases.
  - Made full-thread/full-history context the default subagent context delivery mode, with mandatory context checklist recording and explicit context packets as delivery supplement/fallback only when recorded.
- **v1.4.8 (2026-06-01)**
  - Required every user request to create or enter standard task worktree/task truth before substantive handling, including read-only, chat-only, and pure fact lookup requests.
  - Removed the read-only/chat-only bypass for `default-workflow-bootstrap`.
  - Clarified that professional routing still happens after bootstrap, but no request is handled outside task/worktree truth.
- **v1.4.7 (2026-06-01)**
  - Defined the sink for unbound read-only professional slices as the role-tagged user-facing answer or preserved chat/thread transcript. Superseded by v1.4.8, which forbids unbound read-only professional slices.
  - Clarified task-bound evidence sink expectations for repository-changing subagent work. Superseded by v1.4.8, which requires task truth for every request.
- **v1.4.6 (2026-06-01)**
  - Added the default subagent runtime policy.
  - Required slice contracts to record model configuration and reasons for non-default model/reasoning overrides.
- **v1.4.5 (2026-06-01)**
  - Split read-only/chat-only handling into pure fact lookup versus read-only professional/domain judgment.
  - Required matching professional role slices for read-only professional questions without forcing task/worktree bootstrap unless repository writeback follows. Superseded by v1.4.8, which forces bootstrap first.
  - Added a minimal read-only specialist slice contract and explicit examples.
- **v1.4.4 (2026-06-01)**
  - Clarified that TPM is a workflow coordinator/canonical integrator only, not a professional execution role.
  - Required professional/domain analysis, implementation, verification judgment, review judgment, and liveops/community messaging to come from matching bounded subagent slices.
  - Limited TPM read-only exploration to routing/integration context unless a professional slice owns or verifies the resulting conclusion.
- **v1.4.3 (2026-06-01)**
  - Defined the mandatory subagent context checklist so identity, workflow governance, task truth, user intent, repo context, and collaboration boundaries are provided before dispatch.
  - Required `AGENTS.md` and the assigned role card for non-read-only subagent slices, with explicit exemption reasons for narrow read-only explorer slices.
- **v1.4.2 (2026-06-01)**
  - Added the normative skill map by workflow phase so each core skill has an explicit trigger, requiredness level, and evidence sink.
  - Clarified that specialist skills are domain-triggered through TPM routing rather than mandatory default workflow phases.
- **v1.4.1 (2026-06-01)**
  - Tightened TPM planning governance: TODO decomposition, subagent slice contracts, and integration order must be written to the task evidence sink before delegated work begins.
  - Clarified that other formal sinks may supplement but cannot replace canonical task evidence.
- **v1.4.0 (2026-06-01)**
  - Added `tpm` as the default main Agent / orchestrator / canonical integrator.
  - Required all professional roles to participate as bounded subagent slices under TPM coordination.
- **v1.3.0 (2026-06-01)**
  - Removed the `trivial` / `non-trivial` workflow split for repository-changing work.
  - Required every repository-changing request to enter the standard task worktree + task flow before edits begin.
  - Kept read-only inspection and chat-only answers outside repository writeback requirements. Superseded by v1.4.8, which requires task/worktree truth for every request.
- **v1.2.3 (2026-05-28)**
  - Required all file edits to happen from a task worktree instead of the `main` branch/worktree.
- **v1.2.2 (2026-05-26)**
  - Required PR helper Copilot review requests to use a verifiable requested-reviewers API path and warn when the request does not stick.
- **v1.2.1 (2026-05-26)**
  - Tightened macOS local workflow compatibility: workflow helpers should avoid bash 4-only builtins and GNU-only `find -printf` in default gates.
  - Clarified that expired task-scoped working-memory entries keep structural lint coverage without requiring machine-local transcript source files to remain present.
- **v1.2.0 (2026-05-25)**
  - Required new task worktrees to link ignored `target` to the repo-family shared cargo target cache.
- **v1.1.0 (2026-05-25)**
  - Restored high-impact normative details that were previously only in `AGENTS.md` (worktree/task-truth policy, execution evidence fields, claim/closeout chain, PR/review chain).
  - Clarified workflow policy preservation during dedup.
- **v1.0.0 (2026-05-25)**
  - Created single source-of-truth workflow spec with phase diagram, role boundary, required/optional gates, and rollback paths.
  - Established policy: workflow changes must update this document first, then sync scripts/docs/skills.


## 8. Workflow Contract Checklist
This checklist records the active workflow contract so later edits do not lose
required task, review, verification, or merge semantics.

- [x] Single owner / single GitHub-backed task / single worktree / single PR chain.
- [x] Standard task worktree flow for every user request, with explicit-reuse-only policy.
- [x] Owner role selection and GitHub-backed task binding before implementation.
- [x] TPM planning/TODO decomposition and subagent slice contracts written to GitHub task issue evidence comments before delegated execution.
- [x] TPM is workflow coordinator/integrator only; professional findings and judgments must come from matching role slices.
- [x] Read-only professional/domain questions require matching bounded role slices after task/worktree bootstrap.
- [x] Subagent intended model configuration defaults to the `Default subagent runtime` policy; actual dispatched model/reasoning and non-default/inherited/unverified rationale are recorded.
- [x] Mandatory subagent context checklist includes identity, governance, task truth, user intent, scoped repo context, and collaboration boundaries.
- [x] Mandatory execution evidence fields and blocker recording.
- [x] Current-round fresh verification before completion claim.
- [x] Pre-PR local role-subagent review packet before PR creation.
- [x] Closeout command chain and `done` verification strictness.
- [x] Local role-subagent review + GitHub PR + required checks + PR comment/thread closeout + mergeability as formal review/merge boundary; `REVIEW_REQUIRED` is informational, not a blocker.
- [x] Normal task PR bodies include a GitHub auto-close link to the bound task issue, with a post-merge `pr_watch` audit fallback for already-merged PRs whose task issue remains open.
- [x] PR review fix loop: fix -> re-verify -> resolve threads -> merge claim.
- [x] Workflow phase-to-skill map preserves required, conditional, optional, and specialist skill reachability.
- [x] Module-local verification remains distinct from integration/release readiness claims.

If any item above changes, update this file first and then sync downstream docs/skills/scripts.
