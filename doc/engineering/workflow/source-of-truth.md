# Engineering Workflow Source of Truth
Version: **v1.12.0**
Last Updated: **2026-07-16**

## 0. Purpose
This file is the **only normative workflow specification** for engineering task execution in oasis7.

Mandatory rule:
1. Any workflow change must be edited in this file first.
2. After this file is updated, sync all related scripts/docs/skills to match.
3. PRs that change workflow scripts without updating this file are invalid.

### GitHub query budget and terminal defaults

- Terminal refresh, sync, PR-watch audit, and closeout require the selected `task_uid`; broad Project or repository issue traversal requires explicit `--global-maintenance`.
- Selected terminal audit is `./scripts/pm/audit-pr-watch-issues.sh --task-uid <task_uid> --json`; repository-wide repair is the separate operator action `./scripts/pm/audit-pr-watch-issues.sh --global-maintenance --json`, guarded by the live GraphQL budget before issue listing.
- Broad GraphQL reads live `rateLimit.remaining/resetAt` first and returns resumable `capability_blocked` when unknown or insufficient; stale cache is never authority to continue.
- Project metadata is cached per process run; field updates skip values proven unchanged by the same live snapshot.
- One-task refresh uses at most one Project GraphQL read including missing-item recovery; one PR-watch poll batches comments, reviews, threads, and checks; selected-task closeout reuses one audit while task, HEAD, and evidence inputs remain frozen.
- Commands expected to emit broad logs or search results must run through `./scripts/pm/bounded-command-output.py` when the repo-owned wrapper is available. The wrapper preserves the full stdout/stderr artifact, digest, exit status, and a bounded head/tail summary; truncation is explicit. This bounds only commands routed through the helper and does not claim control over Codex host logs, context windows, or tool `max_output_tokens`.
- <a id="stable-required-gate-wait"></a>Active PR/CI handling may use short one-shot checks. Once one current-HEAD read confirms that only a stable long-running required check or `required-gate` wait remains, the human-operated TPM on a Codex surface must stop polling in the active turn, yield it, and schedule a task-bound continuation/heartbeat for roughly ten minutes later. Repeating the same unchanged read, status query, agent-list, terminal poll, or wait call in that active turn is a workflow violation; a timeout without a meaningful state change is still unchanged state. Each wake performs one batched current-HEAD gate read. Unchanged state stays quiet and schedules the next heartbeat; any meaningful gate, HEAD, review, comment, thread, hold, query-certainty, or operator-input change cancels the stable cadence and immediately resumes normal gate handling. This continuation is not an unattended production supervisor. A non-Codex fallback must be finite, bounded, run outside model-turn polling where possible, and return control with a resumable wait instead of polling forever.

<a id="capability-and-ownership"></a>
## Capability status
**Current:** TPM is the accountable workflow coordinator / integrator. It advances the canonical lifecycle explicitly with repo helpers and professional subagent slices. The production supervisor is blocked; no current surface can run intake through merge and cleanup unattended.
**Target:** a production supervisor executes the durable lifecycle from intake through merge and cleanup while TPM remains accountable for coordination, evidence integration, and escalation.
| Capability | Status | Meaning |
| --- | --- | --- |
| Durable reducer/checkpoint and fail-closed phase gates | implemented | Safe local state and validation primitives exist. |
| Receipt-bound main-sync and safe-cleanup helpers | implemented | Production helpers validate durable receipts and fail closed. |
| Fake-GitHub lifecycle fixtures | test-only | They test reducers; they are not production evidence. |
| Human-operated pre-PR role review | implemented | TPM records frozen-head, role-complete review evidence after draft CI and before promotion. |
| Unattended pre-PR review attestation | blocked | No trusted runtime provenance-attestation producer exists; unattended automation must stop at `capability_blocked`. |
| Production supervisor from intake through merge | blocked | Without trusted production producers, automation is `capability_blocked`. |

## Lifecycle ownership
TPM is the accountable workflow coordinator / integrator and continuation owner.
A professional **phase owner** owns only its bounded slice; the **task owner role** remains
accountable for task outcome and evidence. Bootstrap establishes truth and the
router selects a phase; neither owns continuation.

The **target production supervisor** is the durable runtime executor, not an
accountability owner. It is `blocked`, so TPM coordinates each action explicitly.

<a id="canonical-lifecycle"></a>
## Canonical state machine
The production supervisor is a target runtime executor and is currently
blocked; the state machine below defines required order, not implemented
automation.

`bootstrap -> route -> professional execution -> freeze -> draft_candidate -> CI verify -> review -> pre_pr_ready -> promote_draft -> pr_watch/fix/reverify/review -> merge -> merge receipt -> task done -> main sync -> safe cleanup receipt -> post-merge finalize -> post_merge_done`

## Workflow states
- `running`: the recorded action authority is executing its typed action.
- `action_required`: a bounded action awaits an authorized consumer.
- `external_wait`: a trusted external condition has a durable resume condition.
- `capability_blocked`: missing machinery required by the selected execution mode, including runtime attestation for unattended automation.
- `completed`: terminal completion has been independently proven.
- `failed`: a non-retryable contract violation; stop and escalate. Recovery
  requires an authorized new evidence epoch or a fresh bootstrap, not resume.

This enumeration is closed; phase names and blocker reasons are separate fields.
Recorded action authority is task/checkpoint-bound permission to execute or
resume the typed action, not a role title.

<a id="canonical-gates"></a>
## Ready and Done
These are the only gate definitions in this specification.

Ordinary local `git commit` is not a gate: repository pre-commit hooks run no formatting or validation, and `scripts/pre-commit.sh` is only a silent legacy
compatibility entrypoint. CI required and frozen-head readiness remain authoritative.

<a id="freeze-gate"></a>
**Freeze gate.**

The final implementation head freezes one immutable tree; later code
  changes invalidate downstream verification and review.

<a id="pre-pr-ready-gate"></a>
**Pre-PR Ready.**

The draft candidate's trusted CI receipt and required involved-role review bind the same frozen reviewed PR head and have passed. Ready is a pre-PR gate, not PR creation or Done. The human-operated path requires a
GitHub task packet, all-required-role ledger, head binding, artifact digests,
findings dispositions, and residual risk. Runtime-issued provenance applies
only to unattended supervision, which remains `capability_blocked`. Fixtures
never satisfy a live task.

<a id="pr-creation-gate"></a>
**Draft candidate and promotion gate.**

A draft candidate opens after freeze for exact-head CI. Its receipt binds repository, task, PR, base/head OIDs, check/app/run, planner, conclusion, and time.
Any head change invalidates CI evidence and review; promotion requires same-head readiness.

<a id="post-pr-merge-ready-gate"></a>
**Post-PR merge-ready.**

The canonical live PR gate permits merge for its current head and epoch. It is
not terminal completion. It inspects all applicable required-check identities,
mergeability, requested changes, conversation comments, each reviewer's latest
effective review, and every paginated review thread. Permission, transport,
pagination, policy-discovery, or evidence-readback uncertainty fails closed.
Actionable comments require a current-head disposition; acknowledgements and
status chatter do not block. `REVIEW_REQUIRED` and `BEHIND` alone are
informational.

A successful gate emits a trusted receipt bound to issuer, repository, PR,
head, observation time, and gate epoch. Holds and dispositions are accepted
only from verified GitHub-backed evidence. When GitHub reports the current head
as `MERGEABLE`, `REVIEW_REQUIRED`, and either `BLOCKED` solely by approval or
`BEHIND`, repository standing policy selects the admin merge path by default
after a fresh recheck of required
checks, mergeability, requested changes, conversation comments, review threads,
and merge holds. This exact approval-only state does not require a per-task
authority comment, a caller flag, or another user prompt. The successful live
gate still emits a head-bound readiness receipt and `use_admin_merge: true`;
that receipt, not a local assertion, is the execution prerequisite.
`record-admin-merge-authority.sh` may preserve an optional head-bound audit note for exceptional requester context, but it is not an additional gate.
Admin merge remains forbidden for active holds, failed or missing
checks, requested changes, actionable comments, unresolved threads,
non-mergeable heads, or any blocking state not attributable to missing review
approval or the up-to-date protection represented by `BEHIND`. No separate collaboration/runtime producer is required for this repository-policy path.

<a id="post-merge-done-gate"></a>
**Terminal Done.**

A fresh merge receipt, task done truth, main sync, safe-cleanup receipt, and
post-merge finalization have completed in that order.

## State, gate, and PM mapping
| Workflow state | Gate meaning | GitHub Project status | Resume authority |
| --- | --- | --- | --- |
| `running` | trusted action executing | in progress | recorded action authority |
| `action_required` | authorized consumer required | in progress | authorized consumer |
| `external_wait` | external condition with resume authority | blocked | recorded authority |
| `capability_blocked` | required trusted producer missing | blocked | capability provider |
| `completed` | `post_merge_done` proven | done | none |
| `failed` | non-retryable contract failure | blocked | escalation authority may authorize a new epoch or rebootstrap |

## Documentation policy
`AGENTS.md`, `finishing-a-development-branch`, `tpm.md`, and `.pm/README.md`
are thin operational entrypoints. They may contain local triggers, commands,
I/O, and minimal safety invariants, but must not restate
[ownership](#lifecycle-ownership), [state machine](#canonical-state-machine),
[states](#workflow-states), [gates](#ready-and-done), or the
[review packet](#pre-pr-review-packet).

## 1. Phase Diagram
```mermaid
flowchart TD
  Z[Target production supervisor - currently blocked\ncheckpoint + lease + heartbeat + resume] --> A
  A[Bootstrap\nverify standard worktree + task truth] --> B[Router\nchoose current phase]
  B --> C{Need brainstorming?}
  C -- yes --> D[Bounded Brainstorming]
  C -- no --> E
  D --> E[Execution Gap Review + Atomic Plan]
  E --> F{Behavior changes\nwith stable harness?}
  F -- yes --> G[Behavior-first RED / TDD]
  F -- no --> H
  G --> H[Implementation + Slice Verification]
  H --> R[Freeze immutable implementation head]
  R --> J[Open or resume draft candidate]
  J --> I[Trusted CI required gate\nreceipt bound to frozen head]
  I --> M[Pre-PR Local Role Review\nrole-return ledger per required role]
  M --> Q[Pre-PR Ready gate\nhuman-operated evidence validated]
  Q --> X[Optional evidence-only commit\nrestart CI and review if HEAD changes]
  X --> J
  Q --> Y[PR creation state confirmed\npromote draft]
  Y --> N{PR purpose / merge hold?}
  N -- normal --> O[PR Watch/Fix/Merge Gate\nchecks + mergeability + all comment surfaces]
  N -- packaging --> P[Manual CI Hold\nrecord purpose + wait for operator/user]
  N -- user hold --> U[User-requested Merge Hold\nrecord authority + resume criterion]
  O --> V[Merge receipt -> task done\nmain sync -> safe-cleanup receipt]
  V --> W[Post-merge finalize -> post_merge_done]
  P --> O
  U --> O

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
| Bound task spans multiple lifecycle phases | `tpm-production-supervisor` target | Blocked until trusted collaboration, wake, action and validator producers exist | Durable reducer evidence plus explicit capability blocker |
| Scope is ambiguous, option-heavy, or visual enough to need ideation | `bounded-brainstorming` | Optional, risk-based | Brainstorming output or skip reason in GitHub task issue evidence comments/project |
| Behavior changes with a stable automated harness | `tdd-test-writer` | Conditional required when RED criteria are met; otherwise skip reason required | RED command, failing evidence, and handoff contract |
| Repo truth is ready and implementation proceeds step by step | `executing-project-tasks` | Required for non-trivial execution after route selection | Atomic step evidence in GitHub task issue evidence comments |
| Bug, failing test, broken script, unexpected diff, or regression appears | `systematic-debugging` | Conditional required before speculative fixes | Reproduction, narrowed hypothesis, fix evidence |
| Draft CI receipt is ready and branch is about to be promoted | `requesting-repo-owned-review` | Required before draft promotion; TPM dispatches fresh involved-role review against the receipt head | `Pre-PR Local Role Review: passed` packet |
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

- Product/planning docs: `skills/prd`, `skills/game-architect`; these may create planning artifacts, but the route, work items, and downstream handoff must still be recorded in GitHub task issue evidence comments.
- Game/domain implementation: `skills/game-design-theory`, `skills/gameplay-mechanics`, `skills/level-design`, `skills/particle-systems`, `skills/optimization-performance`, `skills/memory-management`, `skills/synchronization-algorithms`.
- Narrative/community/content: `skills/epic-story-orchestrator-zh`, `skills/content-creation`, `skills/humanizer-zh`, `skills/xiaohongshu-note-analyzer`.
- Browser/visual/content tools: `skills/agent-browser`, `skills/gpt-image-2`.
- Visual companion / Image2 target workflows are optional evidence, not universal gates. They may be used inside an existing task/worktree as visual target and screenshot-comparison evidence, but cannot replace implementation, real native/browser screenshots, interaction smoke, QA evidence, or PR review. Screenshot-only previews count as stable visual-comparison evidence, not real interaction coverage.

If a specialist skill is used, TPM must still bind it to the same owner, GitHub-backed task, canonical worktree, and PR chain through the subagent slice contract. TPM may route to specialist skills, but the specialist role owns the professional conclusion.

### 1.2.1 Friction Controls After Task Truth
The always-bootstrap rule protects traceability, but it must not turn every
small question into a heavyweight workflow.

- Once a request is already inside the bound task/worktree, pure fact/path/command-output lookup or mechanical evidence collection may use the objective-fact fast path: TPM answers directly and records a short GitHub issue evidence note only when the fact materially affects task truth, PR evidence, or a future decision.
- Bootstrap must emit one immutable machine-readable snapshot from the selected GitHub task mapping and live Git facts. It binds task UID, issue/Project identity/status, owner, repository, canonical worktree, branch, base, HEAD, request identity/acceptance, producer/time, and digest. Downstream slices validate and reference it instead of replaying the report. Task, worktree, branch, HEAD, request, or acceptance drift invalidates it; the snapshot is a cache of canonical truth, never a second mutable task store.
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
Reflection signal: use `capture-todo.sh` for an uncommitted cross-task idea;
same-task evidence stays in the bound GitHub task.

Detailed game-design documentation follows `doc/game/README.md`,
`doc/game/prd.md`, `doc/game/prd.index.md`, and matching topic documents.
Route design conclusions to the relevant professional roles; require
claim-to-owner-to-validation traceability instead of expanding this workflow
specification with a separate design method.
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
- Lifecycle wrappers use GitHub Issues/Project as task truth. Ordinary audit,
  readiness verification, and `claim-ready.sh` are read-only with respect to
  `.pm/github-project-sync/tasks.json`; only an explicit create/sync/refresh or
  lifecycle status mutation may rewrite that generated cache. Claim evidence
  uses claim-specific timestamps and must not refresh record-level
  `updated_at`. When refresh-capable GitHub access is available, audit fails on
  cached title or acceptance drift instead of making stale cache content look
  current.
- Execution evidence is recorded in GitHub task issue evidence comments.
- role memory, task-scoped `working_memory`, signals, stage/gate state, and
  this workflow source-of-truth remain repo-local unless a later source-of-truth
  update explicitly migrates them.
- `pm-lint` is read-only against the canonical workspace: it must capture one coherent full-`.pm` snapshot using before/after/source-vs-copy manifests with bounded retry/fail behavior, run every PM read against that single snapshot epoch, isolate Python bytecode under its temporary directory, and leave no ignored artifact behind. The surrounding guard must cover the complete `.pm` filesystem path set—including tracked, baseline-untracked, and ignored entries—with lstat file kind/mode/content or symlink target, while comparing exact Git index mode/OID/stage/path records separately. The guard records symlink state so type and retarget drift are visible; the lint source-manifest boundary must reject any `.pm` symlink before copying.

Project field taxonomy:

| Field | Owner | Meaning | Allowed / expected values |
| --- | --- | --- | --- |
| `Module` | TPM during task creation/routing | Large work queue and reporting group, not owner role or free tag | `engineering`, `game-strategy`, `visualization`, `chain-world-state-substrate` |
| GitHub Project built-in `Status` | TPM and Project views | Human cockpit lane for day-to-day queue visibility | `Todo`, `In Progress`, `Blocked`, `Ready / PR`, `PR Watch`, `Done` |
| `PM Status` | PM lifecycle scripts | Deterministic lifecycle state used by helpers/audits | `candidate`, `committed`, `blocked`, `ready`, `pr_watch`, `done`, `deferred` |
| `Workflow Phase` | Workflow helpers | Project cockpit stage, orthogonal to queue lane | `bootstrap`, `planning`, `execution`, `verification`, `pre_pr_review`, `pre_pr_ready`, `pr_watch`, `blocked`, `done` |
| Priority | Owner / TPM | Scheduling priority, not severity | repo-defined `P0`..`P3` values |

Scripts that sync Project state must keep GitHub built-in `Status`, custom `PM
Status`, and `Workflow Phase` aligned through deterministic mapping:
`candidate -> Todo/execution`, `committed -> In Progress/execution`,
`blocked -> Blocked/blocked`, `ready -> Ready / PR/pre_pr_ready`,
`pr_watch -> PR Watch/pr_watch`, `done -> In Progress/done`, and
`deferred -> Done/blocked`; internal receipts retain the finer
`task_done -> main_sync -> post_merge_done` sequence while the Project cockpit
uses the coarse `done` phase. For PM status `done`, only the finalizer advances
built-in Status to `Done`; `deferred` remains a separate non-completion archive
lane.
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
  create issue/project task, append evidence comments, and move Project status.

- `./scripts/pm/task-closeout.sh` defaults to `ready` / `ready_for_pr`.
  `ready` requires a passed review packet bound to the same frozen
  source head and required-role review-evidence ledger; arbitrary caller-provided
  success commands are not lifecycle proof. `done` requires a recorded merged
  PR, or a classified `non_pr_task`, plus verified `task_complete` evidence. It
  persists the trusted receipt and advances only to PM `done` / `task_done`;
  the issue stays open and processing follows the [terminal runbook](#terminal-runbook).

`post-merge-finalize.py` is the only `post_merge_done` and issue-close writer.
- `./scripts/prepare-task-pr.sh --create` records the created PR URL and moves
  the task to `pr_watch` when GitHub-backed mapping exists. PR creation is
  resumable: before creating, query all states using the exact head repository,
  head branch, and base branch. Reuse only an OPEN match and retry the missing
  task-record transition. A CLOSED exact match may be replaced but never
  reused. A MERGED exact match blocks replacement and requires task-truth
  reconciliation. A foreign repository's same-name head is never a match.
- `scripts/prepare-task-pr.sh` must read passed local role review packets from
  GitHub task issue evidence comments and mapping-backed task truth.
- `scripts/prepare-task-pr.sh --create` must include a non-closing GitHub task
  reference in the PR body and reject explicit bodies that contain an
  auto-close keyword for the task. The generated linkage is
  `Refs #<task-issue-number>` and is separate from `Task UID`; only the terminal
  finalizer closes the task issue.
- `scripts/pm/audit-pr-watch-issues.sh --task-uid <TASK-UID> --close` is the remedial post-merge
  audit for GitHub-backed tasks whose recorded PR is already merged but whose
  PM task issue/body/Project state still says `pr_watch`. The PR body
  PR linkage is not a closure mechanism; the audit remains the PM lifecycle
  synchronizer. Despite its retained CLI name, it advances only to
  `task_done`; it never writes `post_merge_done` or closes an issue. It may
  synchronize only PM task issues whose body
  contains the task marker, `status: pr_watch`, a recorded `pr_number` whose
  GitHub PR state is merged, and existing issue comments with passed pre-PR
  local role review plus verified ready closeout/claim evidence. The audit is
  remedial PM metadata synchronization, not independent professional completion
  judgment. It writes remedial sync evidence, updates Project/body state to
  `done` / `task_done`, then points TPM to the [terminal runbook](#terminal-runbook).
  Missing task issue mapping, missing required Project fields,
  missing existing review/ready evidence, unmerged PRs, non-`pr_watch` statuses,
  or manual hold markers are fail-closed blockers.
- `./scripts/pm/fallback-evidence.sh` is the replay/audit helper for temporary
  `.pm/scratch/<TASK-UID>/fallback-evidence/` packets; unreplayed fallback
  packets do not satisfy task truth and are rejected by PR-readiness lint.
- All future GitHub-backed create/move/report/closeout helpers must use
  deterministic `gh`/GitHub API paths, preserve or recover the `task_uid`
  mapping, and refuse ambiguous duplicate mappings.
- Remote task bootstrap is a journaled, resumable transaction. Before creating
  the issue, write a local journal keyed by the requested worktree/title/owner;
  the journal also stores a canonical immutable-request object and digest covering
  repository, title, owner role, worktree hint, module, priority, source refs,
  acceptance criteria, source signal/type/severity, document/PRD refs, and
  handoff roles. Every resume recomputes that digest and fails before any
  remote call when the request drifted; changing scope requires a new bootstrap.
  after each irreversible remote step, atomically persist task UID, issue URL,
  Project item ID, and next action. A retry must resume that journal or discover
  the unique task UID marker instead of creating another issue. Once any remote
  object exists, bootstrap failure must preserve the task worktree/branch and
  print the exact resume command; force cleanup is forbidden.

## 2. Responsibility Boundary
<a id="specialist-review-role-selection"></a>
- `tpm`: default main Agent / workflow coordinator / canonical integrator only; owns phase decision, role allocation, subagent dispatch, integration order, task-truth writeback, fresh-verification gate coordination, completion-claim coordination, and PR chain coordination.
- TPM is not a professional execution role. TPM must not be the source of domain/professional analysis, implementation, verification judgment, code review judgment, product/design judgment, runtime/wasm/viewer/agent/QA/repository-health judgment, or liveops/community messaging.
- Professional/domain work must be done by the matching bounded subagent slice. This includes:
  - product/system design by `producer_system_designer`
  - gameplay design by `gameplay_designer`
  - game visual direction, interaction feel, player-facing screen flow, and visual readability by `game_visual_interaction_designer`
  - runtime/gameplay/server logic by `runtime_engineer`
  - blockchain/node operations, deployment choreography, upgrade/rollback drills, fleet health baselines, and node runbooks by `blockchain_ops_engineer`
  - WASM/platform/ABI work by `wasm_platform_engineer`
  - in-world Agent perception/memory/planning/execution/feedback/behavior and its inference model/provider behavior by `agent_engineer`
  - Viewer/Web/UI work by `viewer_engineer`
  - verification strategy, test evidence, and release blocking judgment by `qa_engineer`
  - repository health stewardship, documentation/code alignment, semantic clarity, bug-risk surfacing, technical-debt triage, and repository Codex config/adapter/validation-contract audit by `repository_health_engineer`
  - external messaging, community feedback, incidents, player promises, release notes, and channel runbooks by `liveops_community`
- professional role subagents provide bounded slices only (analysis/implementation/verification/review/liveops messaging) and must return artifacts to the TPM owner chain.
- TPM may perform mechanical coordination edits to workflow governance surfaces, task logs, integration notes, and PR plumbing. If the work requires a professional conclusion, TPM must dispatch the matching role slice first and attribute the conclusion to that slice/evidence.
- For every request, TPM planning, work decomposition when needed, subagent slice contracts, and integration order are task execution truth and must be written to GitHub task issue evidence comments before the delegated work begins.
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

## 3. Lifecycle prerequisites and conditional phases
These items determine whether a phase may start; they are not a second gate
taxonomy. Lifecycle transitions use only the five [canonical gates](#canonical-gates).

### 3.1 Prerequisites
- Task truth: isolated worktree, bound GitHub-backed task, and owner role.
- Planning: scope, verification entry, TPM work items, slice contracts, and
  integration order recorded in task evidence.
- Execution evidence: each risky step records action, validation command,
  expected result, actual result, and any blocker.
- Completion claim: fresh current-round verification.

### 3.2 Conditional phases
- Bounded brainstorming when scope or architecture is ambiguous.
- TDD RED for behavior changes with a stable harness; otherwise record why it
  was skipped.
- LiveOps/community review for external messaging, incidents, or player
  commitments.

The current path uses explicit TPM continuation. The future unattended
executor is limited by the [unattended invariants](#appendix-a-target-supervisor-contract).
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

### 4.3.2 User-requested merge hold
- When the user says to create/push a PR but not merge it, record
  `user_requested_merge_hold` with requesting authority, timestamp, reason, and
  exact resume authority. This is not a packaging hold.
- The PR-state gate, ordinary merge path, and admin merge path must all fail
  closed while the hold is active. Only a later user/task-truth instruction from
  the recorded authority clears it.

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
- Post-merge cleanup must use the fail-closed cleanup helper. Cleanup requires:
  a fresh repository-generated PR receipt proving `MERGED`, task truth proving
  `done`, a clean task worktree, and the task branch tip contained in main. A
  squash/rebase merge may substitute a repository-generated patch-equivalence
  receipt bound to the exact branch tip and main tree; literal CLI state or
  mutable cache fields are not proof. The helper uses non-force
  `git worktree remove` and `git branch -d`; it never prints or executes
  `worktree remove --force` / `branch -D` as normal workflow guidance.
- Task worktrees created through `./scripts/new-task-worktree.sh` must create a git-ignored `target` symlink to the repo-family shared cargo target cache resolved by `./scripts/cargo-dev.sh --print-target-dir`, so direct cargo and the development wrapper share local build artifacts by default.
- When Rust commands encounter Cargo package-cache or build-directory locks, wait for the shared repo-family cache to become available; do not switch ad hoc to a fresh temporary `CARGO_TARGET_DIR` just to bypass the lock.

### 5.2 TPM planning and subagent dispatch
- For every request, TPM must record the current plan, work decomposition when needed, selected roles, and integration order in GitHub task issue evidence comments before dispatching professional subagent work.
- Project policy authorizes TPM to dispatch required bounded professional subagent slices directly whenever this workflow requires them; TPM must not pause for per-slice user permission. This project policy is an explicit standing user authorization to use subagents for workflow-required professional role slices; when a tool/runtime requires an "explicit user request for sub-agents, delegation, or parallel agent work", this policy satisfies that requirement for the matching repo-owned workflow slice. If the current runtime, connector, or tool policy still prevents actual subagent dispatch, TPM must record the intended dispatch, actual limitation, fallback evidence path, and attribution boundary in GitHub task issue evidence comments, and must not present TPM's own analysis as a professional role conclusion.
- Each subagent slice must declare role, slice type, intended model configuration, actual dispatched model/reasoning, context delivery mode, role activation, mandatory context checklist, write scope, return contract, validation command, GitHub task issue evidence sink, and integration order.
- The repository does not pin a default subagent model or reasoning effort in `.codex/config.toml`, because top-level `model` and `model_reasoning_effort` keys also change the root TPM session. Each `.codex/agents/<role>.toml` specialist adapter declares its intended model and reasoning effort for adapter-backed named-role activation only. A message-assigned fallback with `role activation: message-assigned; adapter inactive on this surface` does not consume that adapter pin and instead uses the runtime selected by the user or inherited from the parent session. When a dispatch surface supports explicit model selection, TPM may request a concrete model/reasoning pair and must record it as intended configuration; otherwise the fallback contract records `intended model: inherit current parent selection` and `actual model: inherited/unverified`.
- Every slice contract must record one explicit runtime outcome: the requested model/reasoning and its reason when selection is requested; the observed actual model/reasoning when the surface reports it; or `actual model: inherited/unverified` plus the capability reason when selection or observation is unavailable. A requested value must never be presented as the observed actual runtime without evidence.
- Context delivery defaults to a minimal, HEAD-bound task packet for bounded professional slices. Generate and validate immutable packets with `./scripts/pm/subagent-task-packet.py create|validate`; the helper writes under ignored `.pm/scratch/<TASK-UID>/slice-packets/` and fails closed on task/worktree/branch/HEAD drift. On surfaces that expose history controls, use no inherited history or the smallest recent-turn window that carries the packet; do not copy the full parent conversation by default. The packet must identify its task UID, canonical worktree, base ref, frozen/current HEAD, assigned role, context-delivery mode, packet producer/time, and the mandatory checklist below; it may link to exact GitHub evidence comments and repo paths instead of embedding long documents or command output. Full-thread/full-history delivery is an explicit escalation only when the bounded packet is insufficient for a named ambiguity, cross-turn user decision, safety/authority conflict, or evidence reconstruction. The slice contract must record the escalation reason and why targeted paths/comments cannot resolve it. A packet is invalid after HEAD, task truth, user intent, scope, or collaboration boundaries change; TPM must regenerate it rather than append an unbounded transcript.
- For one frozen HEAD and relevant-evidence digest, TPM records one expected role/slice batch before dispatch and performs one canonical collection after the batch returns. A repo-owned batch validator must reject missing, duplicate, stale-HEAD, wrong-epoch, or artifact-digest-mismatched returns. The same HEAD/evidence epoch must not be redispatched after a complete valid collection; transport failure may retry the same immutable slice identity and packet, while any HEAD or relevant-evidence change creates a new epoch and invalidates the old collection. The scratch ledger is audit evidence only; GitHub task issue comments remain canonical. This contract audits repository evidence and does not claim to control Codex host concurrency, `spawn_agent` timing, or mailbox cadence.
- Every expected slice identity is a canonical UUID allocated when the batch is created. Before dispatch, run the batch preflight to materialize an incomplete collector-valid artifact skeleton and ledger for every expected role. After reviewers complete those artifacts, `reconcile` validates every identity, status, disposition, finding, and residual-risk field and atomically publishes a digest-current completed ledger with human-operated provenance; an incomplete or mismatched artifact produces no receipt. Preflight never writes a passed collection receipt; only `collect` may do so after all immutable role returns are completed.
- Documentation review uses an explicit risk class rather than broad path fan-out: `mechanical-doc` and `workflow-doc` require repository health plus QA; `domain-semantic-doc` requires repository health plus one of the eight canonical domain specialists (`producer_system_designer`, `gameplay_designer`, `game_visual_interaction_designer`, `runtime_engineer`, `blockchain_ops_engineer`, `wasm_platform_engineer`, `agent_engineer`, or `viewer_engineer`), adding QA only when verification behavior or coverage changes; coordinator, QA, repository-health, LiveOps/community, and unknown roles are rejected as the domain role. `external-messaging` requires repository health plus LiveOps/community, adding QA only when verification changes. Unknown or mixed scope fails closed for manual role selection. Path inference remains the safety floor for non-document and unclassified changes.
- `prepare-task-pr.sh --review-change-class <class>` consumes that policy only
  when every changed path is documentation (`doc/**`, Markdown, or repository
  README surfaces). Non-document scope rejects the override and retains path
  inference. Domain-semantic classification also requires `--review-domain-role`;
  `--review-verification-affected` adds QA where the matrix specifies it.
- The mandatory context checklist must include:
  - identity and authority: assigned role, role card path, owner role, and TPM integration owner
  - workflow governance: `AGENTS.md`, `doc/engineering/workflow/source-of-truth.md`, and the selected workflow skills
  - task truth: current GitHub issue, GitHub Project item/status, `.pm/github-project-sync/tasks.json` mapping record, canonical worktree, branch, base ref, and PR link/status when present
  - user intent and acceptance target: original request summary, current work item, explicit non-goals, and done/verification expectations
  - scoped repo context: relevant `prd.md`, `project.md`, handoff, changed paths, current diff or evidence summary, and known constraints such as `third_party` read-only boundaries
  - collaboration boundary: sibling slices, write-scope conflicts, integration order, allowed commands, return contract, and formal sink
- `AGENTS.md` and the assigned role card are mandatory inputs for implementation, verification, review, or domain-specialist slices. A narrow read-only explorer slice may omit the role card only when the slice contract records the exemption reason and the exact files to inspect; it still runs after task/worktree bootstrap and records its sink in GitHub task issue evidence comments.
- TPM read-only exploration is allowed only to gather routing context, inspect task truth, or integrate returned evidence. It must not be reported as a professional finding unless a matching professional role slice owns or verifies that finding.
- TPM user-facing summaries must distinguish procedural synthesis from professional conclusions. Professional conclusions must be traceable to subagent artifacts, execution evidence, handoff, project/prd records, or PR evidence.
- Project docs, handoff files, signals, memory, and PR evidence may supplement GitHub task issue evidence comments, but they do not replace them for task execution truth.
- Codex-native specialist role adapters live under `.codex/agents/*.toml` and are registered by `[agents.<role>]` entries in `.codex/config.toml`. The matching `.agents/roles/<role>.md` remains the governance source of truth; an adapter is a concise operational projection for routing, non-goals, write/escalation rules, and the return contract. `repository_health_engineer` owns repository config/adapter projection and validation-contract audit, while `tpm` retains live role selection, dispatch, coordination, and integration. `agent_engineer` owns only the model/provider behavior used by in-world Agents, not repository Codex config or subagent dispatch. `tpm` remains the root coordinator and must not be registered as a selectable specialist role. Adapter text must not broaden the role card, create a second task/worktree/PR truth, or bypass the GitHub task issue evidence sink.
- The model and reasoning values in a specialist adapter are intended configuration, not an observation. Static TOML validation, deterministic projection checks, registry strict-loading, or the presence of those values does not prove named-role activation, model availability, or the actual dispatched runtime. TPM and reviewers must never report an adapter pin as the observed actual model/reasoning without runtime evidence from the active dispatch surface.
- Every specialist role card owns a structured `Codex Adapter Projection` with schema version, registry description, domain contract, operational constraints, and return contract. The adapter and registry description are deterministic renderings of those role-card fields; opaque paired hashes are not a semantic contract. The validator must compare the full rendered output, so replacing an adapter body and merely updating a digest cannot pass.
- Adapter registration is not proof of adapter activation. TPM may claim `adapter-backed` dispatch only when the active Codex surface exposes a named-role selector such as `agent_type` (or a documented equivalent), the requested role is passed through that selector, and the actual activation is observable. Codex 0.137 CLI can strictly load the registered role registry, but the current Desktop `spawn_agent` schema has no named-role selector and full-thread/full-history fork does not activate a registered adapter by itself.
- On a surface without named-role selection, TPM must use the task-packet message-assigned fallback: identify the professional role in the dispatched message, require the role card and slice contract as inputs, and record `role activation: message-assigned; adapter inactive on this surface`. The returned conclusion remains attributed to that professional slice, but TPM must not describe it as adapter-backed. If a surface supports named roles but cannot combine them with the minimal packet delivery mode, TPM chooses the mode required by the slice, records the capability tradeoff, and supplies the mandatory context checklist explicitly when named-role activation is selected.
- A specialist adapter must require the dispatched slice to read `AGENTS.md`, this source of truth, its role card, and the task contract before substantive work; stay within the explicit write scope; treat `third_party` as read-only; escalate cross-role decisions to TPM; and return role-attributed outcome, changed artifacts, findings, command evidence, uncertainty, residual risk, and requested follow-up. Direct GitHub evidence writes are allowed only when the slice contract explicitly authorizes them; otherwise the subagent returns the evidence packet to TPM for canonical writeback.
- If GitHub task issue comments are temporarily unavailable, TPM may write a temporary fallback packet under `.pm/scratch/<TASK-UID>/fallback-evidence/<timestamp>.md` and must record the intended GitHub issue target, reason for fallback, attribution boundary, and replay command. Fallback packets unblock evidence capture only; they do not satisfy task truth, pre-PR review, closeout, or completion claims until replayed to the GitHub task issue comments.
- If the plan changes during execution, TPM must append a GitHub issue evidence comment before continuing the changed work.
- After task truth exists, use the section 1.2.2 learning-intake ladder for discoveries and follow-ups: no-op, short GitHub issue evidence note, reflection signal, task-scoped `working_memory`, or owner-reviewed candidate task/memory promotion. Do not skip directly from a lightweight observation to committed task truth unless the owner explicitly selects that promotion.

### 5.2.1 Read-only specialist routing
- The task/worktree decision and the professional-slice decision are intentionally decoupled:
  - Task/worktree truth is required for every request.
  - Professional judgment controls whether a matching bounded role slice is required after bootstrap.
- Therefore, a read-only request must enter `default-workflow-bootstrap` first and may still require a professional role slice.
- Minimal read-only specialist slice contract:
  - role and slice type (`read_only_analysis`, `verification_judgment`, `review_judgment`, or `liveops_messaging`)
  - intended model configuration, defaulting to `inherit current parent selection` unless an explicit model-selection reason is recorded
  - actual dispatch model/reasoning, or `inherited/unverified` with the connector/tool limitation, stalled default context delivery, or unverifiable actual dispatch reason
  - context delivery mode, defaulting to a minimal HEAD-bound task packet; full-thread/full-history requires a recorded escalation reason
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

### 5.4 Claim and lifecycle transitions
Follow the [canonical gates](#ready-and-done) and
[terminal order](#canonical-state-machine); this section only names operational
helpers.

- Freeze a clean implementation commit. Verify an isolated snapshot of that
  exact tree and run `git diff --check <comparison-ref>...<frozen-head>`.
  A later relevant change starts a new evidence epoch.
- Use `claim-ready.sh` for fresh claim evidence. Multi-surface claims use one
  repository-owned composite verification command.
- A stale CI receipt whose repository, task, PR, base/head, check app/run,
  conclusion, and trusted planner all still match live authority may be refreshed
  explicitly once during closeout. The refresh changes only `observed_at`, writes
  a caller-owned temporary receipt, and never mutates the original. Any identity,
  planner, or conclusion drift fails closed and starts a new evidence epoch.
- After the [Pre-PR review packet](#pre-pr-review-packet) passes, run
  `task-closeout.sh --role <owner-role> --task-uid <TASK-UID> --comparison-ref <ref> --verification-profile <profile>`. Profiles are repo-owned;
  caller-authored commands cannot authorize a transition.
- On partial remote mutation, run `refresh-task-cache.sh`, audit the selected
  task, then retry. Never edit generated cache JSON.
- Closeout uses two bounded selected-task audits: one immediately before the
  transition and one postcondition readback immediately after it. The latter
  must structurally identify `task_uid`, target PM status, and workflow phase
  before success. Minimal `{status: ok}` audit fixtures are accepted only by
  the explicit `fixture_repository_state` verification profile. Neither
  audit performs a broad Project traversal.
- The done transition resolves the recorded PR and requires a fresh
  repository-generated merge receipt. An explicitly classified non-PR task uses
  its repository-owned completion profile.

<a id="terminal-runbook"></a>
### Terminal runbook

After merge, use `<canonical-task-worktree>` for task evidence and
`<canonical-default-worktree>` for sync, cleanup receipts, and finalization.
Helpers fail closed unless inputs bind the same repository, task, PR, head,
worktree, branch, and default branch as applicable.
Receipt identity is bound to the Git common-dir. Relocation or identity mismatch
is `capability_blocked` until trusted mapping migration authorizes a new epoch;
do not copy or edit receipt identity files.

```bash
cd <canonical-default-worktree>
RECEIPT_ROOT="$(python3 scripts/pm/canonical-receipt-root.py \
  --default-worktree <canonical-default-worktree> \
  --task-uid <TASK-UID> --create)"
```

1. Merge receipt — require live readback; retry this command with the same PR.
```bash
cd <canonical-task-worktree>
python3 scripts/pm/pr-merge-receipt.py <PR> --json \
  > "$RECEIPT_ROOT/merge-receipt.json"
```

2. Task done — require task readback; retry this command with the same receipt.
```bash
./scripts/pm/task-closeout.sh --role <owner-role> --task-uid <TASK-UID> \
  --to-status done --verification-profile <repository-owned-profile> \
  --pr-receipt "$RECEIPT_ROOT/merge-receipt.json"
```

3. Refresh mapping — require default-root readback; retry the same refresh.
```bash
cd <canonical-default-worktree>
./scripts/pm/refresh-task-cache.sh --task-uid <TASK-UID> --json
```

4. Main sync — require receipt readback; retry with the same bound inputs.
```bash
./scripts/pm/post-merge-main-sync.sh --repo-root <canonical-default-worktree> \
  --main-ref <default-branch> --task-uid <TASK-UID> \
  --pr-receipt "$RECEIPT_ROOT/merge-receipt.json" \
  --receipt-output "$RECEIPT_ROOT/main-sync-receipt.json"
```
Squash/rebase retry: run `bash ./scripts/pm/patch-equivalence-receipt.sh --root <canonical-default-worktree> --branch-tip <task-branch-tip> --main-commit <integration-commit-in-main-history> --main-parent <integration-first-parent> > "$RECEIPT_ROOT/patch-equivalence-receipt.json"`, then retry steps 4 and 5 with `--patch-equivalence-receipt "$RECEIPT_ROOT/patch-equivalence-receipt.json"`. Both helpers recompute the exact binary delta and bind its digest; later main commits are allowed only while the integration commit remains an ancestor, and caller-authored JSON is not authority.

5. Safe cleanup — require journal/receipt readback; resume by retrying this command.
```bash
./scripts/pm/post-merge-cleanup.sh --repo-root <canonical-default-worktree> \
  --worktree <canonical-task-worktree> --branch <canonical-task-branch> \
  --main-ref <default-branch> --task-uid <TASK-UID> \
  --pr-receipt "$RECEIPT_ROOT/merge-receipt.json" \
  --main-sync-receipt "$RECEIPT_ROOT/main-sync-receipt.json" \
  --terminal-receipt-output "$RECEIPT_ROOT/terminal-cleanup-receipt.json"
```

6. Finalize — require terminal issue/Project readback; retry to resume safely.
```bash
python3 ./scripts/pm/post-merge-finalize.py \
  --repo-root <canonical-default-worktree> --task-uid <TASK-UID> \
  --terminal-receipt "$RECEIPT_ROOT/terminal-cleanup-receipt.json"
```

| Step | Product | Required readback | Resume |
| --- | --- | --- | --- |
| 1 | merge receipt | live PR is merged | repeat step 1 |
| 2 | `task_done` | task remains open at `task_done` | repeat step 2 |
| 3 | refreshed mapping | canonical default-worktree task truth | repeat step 3 |
| 4 | main-sync receipt | local/default remote heads match and ancestry or exact patch equivalence is verified | repeat step 4 |
| 5 | cleanup receipt | intent journal proves cleanup | repeat step 5 |
| 6 | `post_merge_done` | phase persisted and issue closed | repeat step 6 |

All six steps require successful readback; never reorder them or substitute caller-authored receipts.
The finalizer closes its lock descriptor on every exit path but keeps the
task-scoped singleton lock file persistent and ignored. Reusing one stable
flock inode prevents waiting and newly arriving finalizers from overlapping.
Cleanup
diagnostics distinguish an absent path, a path that is not a Git worktree, a
worktree not registered by the canonical repository, and a Git common-dir
mismatch so operators can repair the correct boundary without guessing.

### 5.5 PR and review chain
Select every involved reviewer through the
[specialist review role selection](#specialist-review-role-selection), using
changed paths, task slice history, user-visible claims, and verification claims.
Each role returns `findings` or `no_findings` plus `residual_risk`; TPM records
evidence-backed dispositions. `prepare-task-pr.sh --create` verifies the
minimum path-inferred role set and the packet below. PR watch, holds, merge
authority, invalidation, and terminal behavior come only from the
[canonical gates](#ready-and-done) and
[terminal order](#canonical-state-machine).

#### Pre-PR review packet

A passed packet in GitHub task issue evidence comments contains:

- `Pre-PR Local Role Review: passed`
- `Task UID`, `Source Worktree`, `Source Branch`, `Source Head`, and
  `Comparison Ref`
- `Reviewed Changed Paths`, `Review Package`, and `Role Selection Basis`
- `Review Roles`, per-role `Review Evidence`, and dual `Review Verdicts`
- `Review Findings Disposition` and `Finding Disposition Evidence`
- `Verification Matrix`, `Visual Evidence`, `WASM Evidence`, `Ops Evidence`,
  and `LiveOps Evidence`, each with evidence or a reasoned exemption
- `Residual Risk` and `Slice Ledger`

The immutable ledger matches `Review Roles` and `Source Head`, with one return
per required role. Each human-operated return binds its slice ID, role,
activation/context mode, actual runtime or unverifiable reason, artifact digest,
both verdicts, disposition, and residual risk. The repository validates role
coverage, head binding, and artifact integrity; the GitHub task issue remains
the evidence sink. `n/a` ledgers and fixtures fail closed for live tasks.

An unattended supervisor additionally requires runtime-issued dispatch and
return attestation. Caller-authored receipts, issuer text, local fixtures, and
self-signed evidence cannot satisfy that target mode; missing attestation is
`capability_blocked` only for unattended automation.

For non-trivial diffs, `review-package.sh` writes under
`.pm/scratch/<TASK-UID>/review-packages/`. `record-pre-pr-review.sh` may
format a small workflow/docs packet but cannot replace role returns.
`slice-ledger.sh` is a resumable local index; GitHub task issue evidence
comments remain the formal sink.

<a id="appendix-a-target-supervisor-contract"></a>
### Appendix A: Unattended automation invariants

Future unattended automation does not change the current human-operated path:

- it must preserve canonical task/worktree/head/PR identity and lifecycle order;
- it must derive remote facts and mutation success from trusted readback;
- it must not manufacture professional judgment or accept caller-authored,
  self-signed, or fixture evidence as runtime authority; and
- missing required unattended machinery is `capability_blocked`.
