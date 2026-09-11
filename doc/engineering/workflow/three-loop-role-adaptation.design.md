# Three-Loop Role Adaptation — Proposed Design

Version: **v0.2.0**
Last Updated: **2026-09-11**
Status: **non-normative R2 system design; scoped role migration and split-review implementation pending compatible helper validation and activation**

The [common role contract](./source-of-truth.md#three-loop-role-proposal) is normative for this proposed change; the existing effective lifecycle remains active until compatible implementation is reviewed and explicitly enabled. The [completion and dependency proposal](./three-loop-completion-and-dependency.design.md) supplies the separate no-PR/accepted-dependency and classification prerequisites. R2 narrows QA/runtime writeback to explicit task scope and splits immutable source review identity from latest integration CI; it changes no model, permission, credential, global loop activation or runtime authority. The user-authorized consolidation delivers its reviewed specification and compatible implementation in the same held PR, with separate frozen evidence for each stage. Held PR #3645 and its candidate baseline are reference material, not authority for this task or proof of activation. R2 approval does not release any merge hold or authorize the four-document migration.

## 1. Four dimensions and one leaf identity

| Dimension | Decision | Boundary |
| --- | --- | --- |
| `loop` | Whether the delivered asset belongs to product, system or code | Effective asset policy is the sole path-classification authority; role names and file extensions do not override it |
| `owner_role` | Who is accountable for the professional result | One owner per leaf; TPM coordination is not universal professional ownership |
| Existing slice type | Whether this slice analyzes, designs, implements, verifies, reviews or performs an authorized operation | Use the existing slice contract; no parallel state machine or registry |
| `completion_mode` | Existing PR delivery or classified non-PR evidence completion | Reuse existing producers and validators; proposed helper gaps are not solved by role text |

A user-authorized requirement may comprise a finite set of linked leaves. Each leaf retains its own task UID, canonical worktree, owner and delivery chain; a non-PR chain does not require a dummy PR. Starting one leaf does not authorize creation or execution of discovered follow-up work. The same TPM role may coordinate separate user-started sessions without creating a project-wide single executor or a global per-loop lock.

Actual editing must satisfy all five constraints simultaneously: professional responsibility, effective loop ownership, task `write_scope`, slice permission, and current effective policy/authorization. `Owns` describes expertise, not a directory grant. A runtime specialist may review product feasibility and a gameplay specialist may examine implementation acceptance read-only; neither participation grants edits to another loop. Cross-domain technical work selects one appropriate domain owner and affected-role slices, not a permanent architect or a default repository-health catch-all.

## 2. Retained roles and proposed professional boundaries

Current inventory has exactly the following 12 role-card IDs and 11 professional adapters. TPM has no standalone adapter. The table specifies target responsibilities, not a new path registry; all asset writes still follow the effective classification policy.

| Canonical ID | Target responsibility and typical outputs | Excluded responsibility |
| --- | --- | --- |
| `tpm` | Manual task binding, bounded professional dispatch, dependency coordination, evidence integration and existing delivery/cleanup helpers | Professional design verdicts, replacing QA/domain review, automatic follow-up tasks |
| `producer_system_designer` | Producer/world-rule/economy design: product goals, non-goals, world semantics, long-term economy, governance, priorities and acceptance; product-constraint review in other loops | General technical architecture, runtime/ABI implementation, expanding user authority or overriding world invariants |
| `gameplay_designer` | Product gameplay loops, player verbs/rewards, progression, numerical balance, failure recovery and playability acceptance | Project-wide priority or unilateral long-term economy decisions, engineering implementation |
| `game_visual_interaction_designer` | Product player flows, visual hierarchy, feedback, readability and visual acceptance; separate experience judgment during implementation | Viewer implementation, world rules or QA release decision |
| `runtime_engineer` | System state machines, determinism, persistence/recovery and interfaces; separate code implementation/tests/performance tasks | Product-rule creation, WASM-exclusive contracts or node operation execution |
| `wasm_platform_engineer` | System ABI, permissions, metering, compatibility and module lifecycle; corresponding code implementation/tests | World goals, in-game Agent strategy or node deployment |
| `agent_engineer` | In-game Agent perception, memory, planning, feedback, cost and evaluation design; implementation, strategies, provider adaptation and tests | Repository Codex configuration, development subagent dispatch or harness governance |
| `viewer_engineer` | Viewer/Launcher/Web technical architecture and interfaces; UI/site implementation, observability and browser-validation entrypoints | Redefining player powers, gameplay rules, visual direction or public promises |
| `qa_engineer` | Test strategy, independent verification and quality conclusions; system verification plans/manuals; code tests, fixtures and validation tools; authorized non-PR verification evidence | Substituting domain business fixes, lowering acceptance, authorizing deployment or merge |
| `blockchain_ops_engineer` | System node/environment/upgrade/recovery runbooks; code operational scripts; explicitly authorized non-PR deployment, rollback and readback | Consensus/runtime mechanisms, QA release verdicts or community promises |
| `liveops_community` | Player impact, feedback, communication drafts, channel procedures and operational retrospectives in the appropriate product/system loop; authorized non-PR communication | Technical deployment, code implementation or independent promises of new rules/release dates |
| `repository_health_engineer` | System workflow/governance/harness design; code PM helpers, CI, configuration, role/skill projections and validators; separate independent health/contract/debt review | Domain correctness, TPM live selection/dispatch, QA release judgment or self-authorizing candidate policy |

Keep the producer ID unchanged while clarifying its display/registry description as producer/world-rule/economy design. “Game system” means product semantics, not ownership of every technical design. Gameplay numerical balance and producer economy judgments still require their applicable joint review. Existing world constraints such as resource conservation, irreversible time and Agent uniqueness remain unchanged; open world-design questions are not resolved by a role description.

Repository-health harness implementation is a proposed explicit responsibility, beyond the current card's audit/advice emphasis. It covers mechanism implementation in authorized system/code leaves; TPM still selects and dispatches the actual slices during a task. The in-game Agent role is not a development-harness role merely because both use “Agent.” Technical ownership of site rendering does not authorize new public promises: LiveOps, product/gameplay or visual expertise participates according to actual impact and existing review minima.

## 3. Shared execution and evidence contract

At entry, read the actual loop, owner, manual request scope, immutable input versions, write scope, slice contract and completion mode from the existing task/packet authorities. Do not infer new authority from a role name, stale session or candidate file. System engineering tasks deliver design; code tasks deliver implementation, tests or configuration. Reading other domains and running authorized verification does not authorize incidental cross-loop edits.

Current packets bind owner, slice type, write scope and runtime/activation descriptions; they do not yet provide all proposed structured loop, completion-mode and input-contract-version fields. R2 must add only demonstrated missing context and compatible validation. Current provenance validators check structural slice/runtime/digest evidence; they do not by themselves prove an actual independent observation or native client isolation.

Return upstream ambiguities, design contradictions, documentation debt and implementation deviations with evidence, affected contract versions and recommended professional owner. TPM coordinates only within existing user authorization; otherwise record a recommendation rather than create/start a task. Do not lower acceptance or rewrite upstream rules to legitimize implementation behavior. A new upstream draft alone does not invalidate a still-authorized, unrevoked pinned input.

Repository edits follow the existing PR route. No-file research, review, test execution or operational verification may return `changed_files=[]` and use the existing formal evidence sink. Empty `write_scope` and accepted dependency consumption require the separate compatible helper implementation; no fake scope or empty PR may bypass missing capability. Non-PR work can still have external effects: deployment, rollback and communication each require the exact target, version/content, action bounds and explicit authorization, sanctioned execution and readback. Tool or permission absence is a blocker, not implied permission.

Task evidence, runtime cache and authoritative product/system documentation are distinct assets. Write GitHub evidence only when the slice authorizes it; otherwise return it to TPM. Extract useful experience as a candidate for an authorized existing helper. Do not impose unconditional `.pm`, testing-manual, README, runbook or upstream-rule edits on every professional role. Never hand-edit shared caches, locks, permissions or journals to remove a blocker. Current examples that need bounded R2 projection changes include runtime's upstream-rule writeback, QA's unconditional formal-test-document writeback and TPM's default heartbeat clause.

Returns retain the existing contract: task/slice identity, loop, pinned inputs/versions, conclusion or findings, actual changed paths, commands and observations, unexecuted checks with reasons, risks and necessary follow-up recommendations. A designed future behavior has no passing implementation test merely because its acceptance plan exists. A completed slice is not a completed leaf, and a completed leaf is not release of the linked requirement group.

For the proposed manual entry, stable waits, exhausted budget, blocked dependencies, user pause and completion record resumable facts and return without heartbeat, timed continuation, background process or automatic next task. Do not rewrite current effective lifecycle behavior from this candidate document; R2 must reconcile the relevant executable projections before enabling the new rule. Professional disagreement returns to matching roles; new product authority or unresolved cross-domain tradeoffs return to the user rather than TPM deciding as a substitute expert.

## 4. Review impact and actual independence

Retain `review-role-selector.py`, canonical review plans and task packets. Existing mechanical/workflow/domain/external-message/code-path and risk rules remain a floor; do not replace them with a fixed committee per loop. Product/world/gameplay promises require the affected producer/gameplay roles; visual interaction requires visual and applicable Viewer/QA evidence; technical contracts require their domain and affected interface owners. Harness/role/adapter/validator changes retain repository health and QA, and each changed professional adapter requires that professional role's review. Public communication retains LiveOps and any affected promise/quality owner. Ordinary tasks need only their affected roles; changing all 11 adapters requires actual coverage of all 11 affected professional roles.

Review slices are read-only for delivery sources by default. Findings return to an implementation slice; changed HEAD invalidates affected frozen evidence and requires fresh checks/review. Formal independent review must use a distinct non-author slice with the exact frozen inputs and actual observation/dispatch evidence. An implementation slice's selfassessment is never repackaged as formal review. Different role names do not prove independence, and the same role ID may legitimately appear in separate implementation and non-author review slices. QA and affected-domain review remain necessary; test authors or verification-plan authors also receive independent review according to risk.

### 4.1 Split source review and integration identity

The canonical source contract is [split-source-review-integration-contract](./source-of-truth.md#split-source-review-integration-contract). The v2 producer emits a source identity with `task_uid`, `bootstrap_epoch`, repository/PR, `source_head_oid`, fixed `source_scope_oid`, changed-path digest, ordered role IDs, `role_contract_digest`, `review_policy_digest`, and `input_contract_digest`; `source_review_digest` hashes that canonical object. The integration producer separately emits repository/task/PR/source-head, `integration_base_oid`, trusted workflow/ref/SHA, dispatch request and attempt IDs/times, check IDs, planner digest, complete `tested_tree_oid`, and conclusion; `integration_ci_digest` hashes that canonical object. The source scope OID is never substituted with the integration base.

The minimum safe reuse rule is deliberately conservative: a completed professional review may be reused only when every v2 source identity field, the accepted complete `tested_tree_oid`, and the live task/PR/role/policy/input authority still match. A target advance still requires a fresh latest exact integration dispatch and current-PR readback; reuse is accepted only when that receipt succeeds and its complete tested tree equals the accepted snapshot. A changed tested tree, source head or scope, role/policy/input digest, new finding, unknown dependency/authority closure, wrong execution ref/base, current-PR drift, new failure/pending/uncertain state, or legacy v1 combined plan requires a full new review epoch. This task does not infer dependency closure from path names or activate v2 globally.

The S5 implementation producers/consumers are bounded to `scripts/pm/ci_ready_receipt_identity.py`, `scripts/pm/ci-ready-receipt.py`, `scripts/pm/review-plan.py`, `scripts/pm/review-role-selector.py`, and `scripts/pm/review-batch-epoch.py`, with admission/closeout consumers in `scripts/pm/subagent-task-packet.py`, `scripts/pm/review-closeout.sh`, `scripts/pm/record-pre-pr-review.sh`, `scripts/prepare-task-pr.sh`, and `scripts/pm/task-closeout.sh`. The v1 path remains strict and usable for this PR; these files must not change source-review reuse into a bypass of latest integration validation.

Current tasks use effective role/configuration and specification versions. Candidate cards/adapters may be edited, tested and reviewed but cannot grant themselves immediate authority. Enablement follows the existing review/activation path, without silent in-flight owner, loop or policy switches; an authorized migration requires explicit new evidence epoch. Adapter text and offline validation are not proof of actual model availability, dispatch, native filesystem isolation or client enforcement.

## 5. Bounded R2 implementation mapping and prerequisites

| Later code surface | Permitted adaptation | Preserved boundary |
| --- | --- | --- |
| `.agents/roles/*.md` and existing templates | Retain all IDs and the existing card structure; reference the common contract and encode professional differences; QA/runtime checklists grant formal-doc writeback only when task scope and authority explicitly allow it | Executable instruction assets are code, even when Markdown; no extra authority registry |
| `.codex/agents/*.toml` | Project the 11 professional cards consistently | No added TPM adapter or model/reasoning/permission changes |
| `.codex/config.toml` | Necessary descriptions and existing references only | No new role, concurrency setting or runtime grant |
| `AGENTS.md` and existing bootstrap/router/execution/finish/review skills | Reconcile conflicting blanket writeback/heartbeat text and convey the actual task context | No duplicated lifecycle or automatic routing service |
| Existing packet, selector/plan and validators | Add only demonstrated missing context/boundary checks | Do not replace current review minima or receipt authority |
| Existing adapter/role-fit tests and focused regressions | Positive professional dispatch and denied-boundary scenarios, not keyword matching alone | Fixtures do not prove client activation or completed external operations |

Keep Mission, Execution Mode, Owns, Does Not Own, Inputs, Outputs, Decisions, Done Criteria, Codex Adapter Projection, Recommended Skills and Checklist in role cards. Source holds common rules, cards professional differences and adapters matching projections. This division must not become three independent routing truths.

For this migration the direct role-card files are `.agents/roles/qa_engineer.md` and `.agents/roles/runtime_engineer.md`. Their adapters remain unchanged because the structured projection fields do not change; `.codex/config.toml` and adapter model/reasoning values remain unchanged. QA may write `doc/testing/*` or `doc/playability_test_result/*` only when that task's explicit `write_scope`, slice permission and QA authority include the same verification loop; otherwise it returns evidence to TPM. Runtime may write `doc/world-runtime/*` and upstream rule documentation only when the task explicitly authorizes the same behavior change and authority; cross-loop or upstream findings return through the existing evidence handoff.

R1 is the system-document stage in the single user-authorized PR. Before R2 implementation, freeze and independently review an **immutable R1 specification revision within the same PR**, and bind that exact revision, consumed clauses and review evidence to the implementation stage. A mutable PR link or unreviewed candidate HEAD is insufficient; a separate prior merge to main is not required by this explicit consolidation authorization. This staging exception does not make candidate policy effective or grant runtime authority. Do not automatically create a task or start R2 merely because consolidation is complete. Preserve holds and the existing gates. The separate completion-helper and document-migration prerequisites remain explicit; role responsibility/output adaptation may be implemented without falsely claiming unsupported non-PR end-to-end behavior. Enable only after compatible executable paths, affected-role review, CI and actual client evidence required by the existing activation process. No new service, scheduler, database, MCP server, unattended supervisor, model/reasoning tier, concurrency quota, sandbox or external credential permission is part of R1/R2.

## 6. Later acceptance evidence

| Scenario | Required positive observation | Required refusal or limitation |
| --- | --- | --- |
| Inventory/projections | Same 12 IDs, 11 working professional adapter references and consistent cards | No TPM adapter, architect, loop administrator or renamed ID |
| Runtime system leaf | Only approved design assets; interfaces, failure semantics and verification plan | No implementation write or claim that future tests passed |
| Runtime code leaf | Only approved implementation/tests; upstream concerns returned as findings | No forced upstream-rule or cross-loop document edit |
| In-game Agent change | Agent engineering owns corresponding behavior with affected runtime/QA participation | No harness-driven repository-health default assignment |
| Codex/harness change | Repository-health design/implementation, TPM coordination, independent QA/role review | No implementation selfassessment masquerading as review |
| QA non-PR verification | Zero source edits with exact version-bound evidence and actual existing completion path when supported | Missing helper capability explicitly blocks end-to-end acceptance |
| Node operation and announcement | Ops and LiveOps each act within their own explicit operation/content authorization and read back results | QA quality verdict does not grant either operation permission |
| Review independence | Different non-author slice identity, frozen inputs and actual observations | Renamed implementation result or role-name-only independence rejected |
| v2 review reuse | Source identity and complete trusted `tested_tree_oid` match, with fresh latest integration success and current-PR readback | Any tested-tree/authority/closure uncertainty or legacy v1 plan requires full review |
| Scoped QA/runtime writeback | Explicit task/slice authority permits the affected professional document and same-loop evidence | Checklist ownership alone never grants a cross-loop or unconditional write |
| Wait/completion | Resumable facts returned, no automatic wake or next task | No heartbeat, scheduler or silent continuation under the proposed manual entry |
| Candidate role version | Current authority remains pinned; activation/migration is explicit | No self-granted scope or silent in-flight hot switch |
| Common contract/projections | No unconditional cross-loop writeback; cards and adapters agree | Tests of strings alone do not establish behavior |
| Delivery/activation status | Design approval, implementation merge, client verification and enablement recorded separately | None substitutes for another or releases an existing hold |

These are future implementation acceptance conditions, not results claimed by R1. Current documentation verification checks proposal consistency and governance only. World-rule changes, mixed-document migration, production operations and general PR lifecycle reconstruction remain outside this role-adaptation scope.
