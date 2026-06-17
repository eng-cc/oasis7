# task_cb987cd0fdfb4ecc98a6ddde7d96204c Execution Log

- task_uid: task_cb987cd0fdfb4ecc98a6ddde7d96204c
- title: Plan unified world terminology upgrade
- owner_role: tpm
- worktree_hint: /Users/scc/ccwork/worktrees/oasis7-core-unified-world-terminology-upgrade-plan

<!-- Append entries using:
Example:
  ## YYYY-MM-DD HH:MM:SS CST / role_name
  - 完成内容: ...
  - 遗留事项: ...
  - Action: ...
  - Validation Command: ...
  - Expected Result: ...
  - Actual Result: ...
  - Blocker / Next Action: ...
-->

## 2026-06-16 17:45:00 CST / tpm
- 完成内容: WORKFLOW BOOTSTRAP DECIDED. Repository state impact: planning/documentation change requested; created dedicated task worktree `/Users/scc/ccwork/worktrees/oasis7-core-unified-world-terminology-upgrade-plan` on branch `task/core-unified-world-terminology-upgrade-plan`. Task truth: `.pm/tasks/task_cb987cd0fdfb4ecc98a6ddde7d96204c.yaml`, owner role `tpm` as workflow coordinator only. Formal docs: `AGENTS.md`, `doc/engineering/workflow/source-of-truth.md`, `testing-manual.md`.
- 遗留事项: Await professional slice execution or fallback evidence path before presenting specialist-owned findings.
- Action: WORKFLOW ROUTE DECIDED. Current phase: repository documentation planning change with professional product/runtime/QA/ops judgment. Selected workflow surface: `repo-owned-workflow-router` step 0 for bounded professional slices, followed by minimal documentation execution in the canonical worktree. Skipped TDD because this round creates an upgrade plan document, not behavior-changing code.
- Action: TPM TODO decomposition: (1) inventory current `shared_devnet` / network-tier / large-world terminology surfaces; (2) dispatch product, runtime, QA, and blockchain ops slices; (3) create a repo plan file that makes "unified persistent world" the default product model while preserving historical evidence and compatibility constraints; (4) verify docs with lint/diff checks.
- Action: Subagent Slice Plan 1. Role: `producer_system_designer`. Slice type: product/world-model terminology migration plan. Intended model configuration: workflow source-of-truth Default subagent runtime. Actual dispatched model/reasoning: inherited/unverified unless tool reports otherwise. Context delivery mode: full-thread/full-history fork. Mandatory context checklist/packet: identity and authority: `AGENTS.md`; workflow governance: `doc/engineering/workflow/source-of-truth.md`; task truth: this task UID/worktree; user intent: create an upgrade plan so default game model is one unified persistent world and `shared_devnet`-style terms disappear from product language; scoped repo context: `doc/core/*`, `doc/game/*`, `README.md`, `world-rule.md`, `doc/p2p/*`, `testing-manual.md`; collaboration boundary: no edits, return findings/plan requirements only. Write scope: none. Return contract: recommended terminology, non-goals, acceptance criteria, residual risks. Formal sink: this execution log via TPM integration.
- Action: Subagent Slice Plan 2. Role: `runtime_engineer`. Slice type: runtime feasibility and compatibility boundary. Intended model configuration: workflow source-of-truth Default subagent runtime. Actual dispatched model/reasoning: inherited/unverified unless tool reports otherwise. Context delivery mode: full-thread/full-history fork. Mandatory context checklist/packet: identity and authority: `AGENTS.md`; workflow governance: `doc/engineering/workflow/source-of-truth.md`; task truth: this task UID/worktree; user intent: plan migration from environment-tier naming to unified-world default while keeping implementation safe; scoped repo context: `crates/oasis7*`, `doc/world-runtime/*`, p2p/network-tier scripts/docs; collaboration boundary: no edits. Return contract: runtime constraints, naming aliases, phased code/config cleanup, verification gates. Formal sink: this execution log via TPM integration.
- Action: Subagent Slice Plan 3. Role: `qa_engineer`. Slice type: QA claim boundary and migration validation. Intended model configuration: workflow source-of-truth Default subagent runtime. Actual dispatched model/reasoning: inherited/unverified unless tool reports otherwise. Context delivery mode: full-thread/full-history fork. Mandatory context checklist/packet: identity and authority: `AGENTS.md`; workflow governance: `doc/engineering/workflow/source-of-truth.md`; task truth: this task UID/worktree; user intent: ensure terminology cleanup does not overclaim readiness; scoped repo context: `testing-manual.md`, `doc/testing/*`; collaboration boundary: no edits. Return contract: test evidence matrix, forbidden claims, migration acceptance gates. Formal sink: this execution log via TPM integration.
- Action: Subagent Slice Plan 4. Role: `blockchain_ops_engineer`. Slice type: ops/network-tier migration boundary. Intended model configuration: workflow source-of-truth Default subagent runtime. Actual dispatched model/reasoning: inherited/unverified unless tool reports otherwise. Context delivery mode: full-thread/full-history fork. Mandatory context checklist/packet: identity and authority: `AGENTS.md`; workflow governance: `doc/engineering/workflow/source-of-truth.md`; task truth: this task UID/worktree; user intent: avoid user-facing `shared_devnet` language while keeping node deployment, manifests, evidence, and rollback compatible; scoped repo context: `doc/p2p/*`, testnet scripts, network-tier manifests/templates; collaboration boundary: no edits. Return contract: migration stages, compatibility aliases, operator risks, verification commands. Formal sink: this execution log via TPM integration.
- Validation Command: `git status --short --branch`; inspect task truth and role slice contracts.
- Expected Result: A plan file can be created with a clear distinction between product-default unified-world language, engineering compatibility aliases, historical evidence retention, and later code/script migration.
- Actual Result: Pending subagent dispatch.
- Blocker / Next Action: Continue with repository evidence gathering; if subagent dispatch capability remains unavailable, record limitation and keep specialist attribution boundary explicit.

## 2026-06-16 17:56:00 CST / tpm
- 完成内容: Created the unified persistent world terminology upgrade plan and wired it into core discovery surfaces.
- 遗留事项: Full specialist review remains required before implementing the Phase 1-4 migration work, especially the runtime/ops compatibility changes.
- Action: Professional slice result integrated from `producer_system_designer`. The slice completed and directly landed `doc/core/unified-persistent-world-terminology-upgrade-plan-2026-06-16.md`, plus discovery updates in `doc/core/README.md`, `doc/core/prd.index.md`, `doc/core/prd.md`, `doc/core/project.md`, and `doc/.governance/module-root-md-allowlist.txt`.
- Action: Product decision captured: active player/product language defaults to "统一持久大世界 / unified persistent world"; environment names, network tiers, and `world_id` remain technical dimensions and must not be presented as multiple player worlds.
- Action: Compatibility boundary captured: legacy network/dev environment terms may remain only in historical evidence, compatibility aliases, migration notes, operator docs, scripts, and tests, with explicit `legacy / historical / rehearsal / compatibility / operator / manifest` qualification.
- Validation Command: `./scripts/doc-governance-check.sh && git diff --check`
- Expected Result: Full doc governance and whitespace checks pass.
- Actual Result: Earlier run was interrupted after multiple polling windows with no script output; no pass/fail verdict observed from `doc-governance-check.sh`.
- Blocker / Next Action: Use lighter observable checks in this turn and rerun full doc governance with an observed result before final closeout.

## 2026-06-16 17:58:00 CST / tpm
- 完成内容: Verified the newly created core plan file and core discovery edits with lightweight checks after the full doc governance command hung without output.
- 遗留事项: Full doc governance still needed an observed pass/fail result at this point.
- Action: Confirmed the main workspace `/Users/scc/ccwork/oasis7` is clean after correcting an initial patch-location mistake; all intended core edits now live in the task worktree.
- Validation Command: `test -f doc/core/unified-persistent-world-terminology-upgrade-plan-2026-06-16.md && rg -n "统一持久大世界|unified persistent world|Legacy / Forbidden Product Terms|Upgrade Phases|Acceptance Criteria|Residual Risks" doc/core/unified-persistent-world-terminology-upgrade-plan-2026-06-16.md doc/core/README.md doc/core/prd.index.md doc/core/prd.md doc/core/project.md`
- Expected Result: Plan file exists and key terms/sections are discoverable from the plan and core entrypoints.
- Actual Result: Passed; `rg` found the new plan, canonical terms, legacy/forbidden term section, upgrade phases, acceptance criteria, and residual risks.
- Validation Command: `git diff --check -- doc/core/unified-persistent-world-terminology-upgrade-plan-2026-06-16.md doc/core/README.md doc/core/prd.index.md doc/core/prd.md doc/core/project.md .pm/tasks/task_cb987cd0fdfb4ecc98a6ddde7d96204c.execution.md`
- Expected Result: No whitespace errors in files changed by this TPM integration step.
- Actual Result: Passed with no output.
- Blocker / Next Action: Full `./scripts/doc-governance-check.sh` still needs a successful observed run before PR/closeout if required by the next workflow phase.
- Blocker / Next Action: Continue with repository evidence gathering; if subagent dispatch capability remains unavailable, record limitation and keep specialist attribution boundary explicit.

## 2026-06-16 18:02:00 CST / tpm
- 完成内容: Integrated QA claim-boundary findings into the plan acceptance criteria and residual-risk language.
- 遗留事项: QA must participate again before any Phase 1-4 implementation claims are marked ready.
- Action: Professional slice result integrated from `qa_engineer`. QA boundary: "统一持久大世界" is the default product model, not proof of public launch, production/mainnet settlement, unlimited capacity, open faucet, validator admission, or current live readiness.
- Action: QA acceptance matrix: active player/product docs should avoid legacy shared-development-network terms unless the surrounding text clearly marks them as legacy/historical/rehearsal/compatibility; technical docs may retain them only as classified evidence, aliases, migration targets, or operator/runbook terms.
- Action: QA verification requirements captured: scan active product/player-facing docs, keep compatibility scans for `crates`, scripts, testing docs, and p2p docs, preserve manifest/schema aliases, and gate readiness claims separately from terminology migration.
- Validation Command: `rg -n "统一持久大世界|unified persistent world|Legacy / Forbidden Product Terms|Upgrade Phases|Acceptance Criteria|Residual Risks" doc/core/unified-persistent-world-terminology-upgrade-plan-2026-06-16.md doc/core/README.md doc/core/prd.index.md doc/core/prd.md doc/core/project.md`
- Expected Result: QA-relevant sections are discoverable from the new plan and core entrypoints.
- Actual Result: Passed.
- Blocker / Next Action: Complete final validation and keep Phase 3 runtime/ops compatibility as a future specialist-gated task.

## 2026-06-16 18:05:00 CST / tpm
- 完成内容: Removed the duplicate p2p-local plan draft and kept the core plan as the single upgrade-plan file for this task.
- 遗留事项: Runtime and blockchain-ops roles must participate again before implementing any code/script/manifest migration in Phase 3; this task remains a planning and discovery-wiring task.
- Action: Reconciled concurrent edits in the task worktree. The authoritative plan is `doc/core/unified-persistent-world-terminology-upgrade-plan-2026-06-16.md`; p2p remains a referenced technical domain inside the core plan instead of owning a second plan.
- Action: Professional slice result integrated from `blockchain_ops_engineer`. The slice confirmed the canonical plan location, verified that product language defaults to "统一持久大世界 / unified persistent world", and confirmed that local/test/production, `world_id`, and network tier are ops/runtime dimensions rather than player-facing multiple-world narratives.
- Action: Professional slice result integrated from `runtime_engineer`. The slice confirmed the canonical plan location and accepted the runtime boundary: `world_id`, network tier, test candidate window, and environment remain technical implementation dimensions and must not be packaged as multiple player worlds.
- Action: Attribution boundary: producer, QA, runtime, and blockchain-ops findings are integrated for the upgrade plan. Later Phase 1-4 implementation still needs fresh role review and verification before code/script/manifest changes are claimed ready.
- Validation Command: `git diff --check`; `./scripts/pm/lint.sh`; `./scripts/doc-governance-check.sh`.
- Expected Result: Core plan remains discoverable, duplicate p2p plan is absent, task-local PM lint issues are resolved, and doc governance passes or returns actionable issues.
- Actual Result: Superseded by the final verification entry below.
- Blocker / Next Action: Final validation entry records the observed results; leave unrelated historical PM lint failures untouched.

## 2026-06-16 18:08:00 CST / tpm
- 完成内容: Final verification pass for the unified persistent world terminology upgrade plan.
- 遗留事项: Professional role review is still required before executing the later migration phases; this task only creates the upgrade plan and discovery wiring.
- Action: Confirmed the single authoritative plan is `doc/core/unified-persistent-world-terminology-upgrade-plan-2026-06-16.md`; no duplicate p2p plan remains.
- Validation Command: `git diff --check`
- Expected Result: No whitespace errors.
- Actual Result: Passed with no output.
- Validation Command: `./scripts/doc-governance-check.sh`
- Expected Result: Documentation governance accepts the new module-root plan after allowlist update.
- Actual Result: Passed with `doc-governance-check: OK`.
- Validation Command: `./scripts/pm/claim-ready.sh --claim-type task_complete --verify-command "./scripts/doc-governance-check.sh"`
- Expected Result: Fresh task-complete verification is allowed after doc governance passes.
- Actual Result: Passed; claim summary reported `status: verified`, `verification_exit_code: 0`, and `allowed_to_claim: true` at `2026-06-16T18:12:12+08:00`.
- Validation Command: `./scripts/pm/lint.sh 2>&1 | rg 'task_cb987cd0fdfb4ecc98a6ddde7d96204c' || true`
- Expected Result: No task-local PM lint findings for this task UID.
- Actual Result: Passed with no output; repo-wide PM lint still has unrelated historical task-log failures outside this task.
- Blocker / Next Action: Ready to hand back the plan file; do not claim full Phase 1-4 terminology migration is complete.

## 2026-06-16 18:20:00 CST / tpm
- 完成内容: WORKFLOW ROUTE DECIDED for follow-up document review requested by user: inspect related docs for residual old semantics after the unified persistent world terminology plan landed.
- 遗留事项: Review findings must distinguish player/product-facing semantic residue from internal technical legacy/compatibility terms. Any professional conclusion must be attributed to the relevant role slice.
- Action: Current phase: task-bound read-only professional/domain review. Selected workflow surface: `repo-owned-workflow-router` step 0, with bounded professional slices plus local evidence collection. No new task/worktree was created because this request continues the already-bound task/worktree and `.pm` task truth.
- Action: TPM TODO decomposition: (1) scan active player/product docs for old shared-devnet/multi-world semantics; (2) scan technical docs for unqualified old terms that could leak into product claims; (3) dispatch producer, QA, runtime, and blockchain-ops review slices; (4) integrate findings with file/line evidence; (5) do not edit files unless the review identifies a small obvious documentation fix and the task remains scoped.
- Action: Subagent Slice Plan 5. Role: `producer_system_designer`. Slice type: product/default-world semantic residue review. Intended model configuration: workflow source-of-truth Default subagent runtime. Actual dispatched model/reasoning: inherited/unverified unless tool reports otherwise. Context delivery mode: full-thread/full-history fork. Mandatory context checklist/packet: identity and authority: `AGENTS.md`; workflow governance: `doc/engineering/workflow/source-of-truth.md`; task truth: this task UID/worktree; user intent: review related docs for residual old semantics after making the whole game default to one unified persistent world; scoped repo context: `README.md`, `world-rule.md`, `doc/core/**`, `doc/game/**`, `doc/readme/**`, `site/**`, and the new plan; collaboration boundary: read-only review, no edits. Write scope: none. Return contract: findings only, with severity, file/line, why it violates or risks old multi-world semantics, and whether it is product-facing or acceptable legacy context. Formal sink: this execution log via TPM integration.
- Action: Subagent Slice Plan 6. Role: `qa_engineer`. Slice type: claim/readiness old-semantics review. Intended model configuration: workflow source-of-truth Default subagent runtime. Actual dispatched model/reasoning: inherited/unverified unless tool reports otherwise. Context delivery mode: full-thread/full-history fork. Mandatory context checklist/packet: identity and authority: `AGENTS.md`; workflow governance: `doc/engineering/workflow/source-of-truth.md`; task truth: this task UID/worktree; user intent: catch old semantics or overclaims that conflict with unified persistent world while preserving readiness gates; scoped repo context: `testing-manual.md`, `doc/testing/**`, `doc/playability_test_result/**`, `doc/core/unified-persistent-world-terminology-upgrade-plan-2026-06-16.md`; collaboration boundary: read-only review, no edits. Write scope: none. Return contract: findings with file/line, claim risk, severity, and recommended wording boundary. Formal sink: this execution log via TPM integration.
- Action: Subagent Slice Plan 7. Role: `runtime_engineer`. Slice type: runtime terminology compatibility leak review. Intended model configuration: workflow source-of-truth Default subagent runtime. Actual dispatched model/reasoning: inherited/unverified unless tool reports otherwise. Context delivery mode: full-thread/full-history fork. Mandatory context checklist/packet: identity and authority: `AGENTS.md`; workflow governance: `doc/engineering/workflow/source-of-truth.md`; task truth: this task UID/worktree; user intent: find docs where runtime `world_id`, environment, or tier terminology still reads like multiple player worlds; scoped repo context: `doc/world-runtime/**`, `crates/oasis7*/**` docs/comments only if needed, `doc/core/**`, and the new plan; collaboration boundary: read-only review, no edits. Write scope: none. Return contract: findings with file/line, whether technical usage is acceptable, and any recommended qualifier. Formal sink: this execution log via TPM integration.
- Action: Subagent Slice Plan 8. Role: `blockchain_ops_engineer`. Slice type: network-tier/operator terminology leak review. Intended model configuration: workflow source-of-truth Default subagent runtime. Actual dispatched model/reasoning: inherited/unverified unless tool reports otherwise. Context delivery mode: full-thread/full-history fork. Mandatory context checklist/packet: identity and authority: `AGENTS.md`; workflow governance: `doc/engineering/workflow/source-of-truth.md`; task truth: this task UID/worktree; user intent: identify network-tier/devnet wording that still implies multiple player worlds or active old shared-devnet semantics; scoped repo context: `doc/p2p/**`, `doc/engineering/governance/**`, `testing-manual.md`, scripts docs/runbooks if needed; collaboration boundary: read-only review, no edits. Write scope: none. Return contract: findings with file/line, whether acceptable operator legacy, and recommended qualifier or migration note. Formal sink: this execution log via TPM integration.
- Validation Command: `git status --short --branch`
- Expected Result: Review starts in the already-bound task worktree with no unrelated worktree switch.
- Actual Result: In progress.
- Blocker / Next Action: Dispatch slices and run local semantic scans.

## 2026-06-16 18:34:00 CST / tpm
- 完成内容: Integrated read-only review findings for residual old semantics after the unified persistent world plan.
- 遗留事项: No fixes applied in this review pass. Follow-up implementation should prioritize player/public templates and active entrypoints before historical evidence files.
- Action: `producer_system_designer` finding integrated: public homepage still says "hosted web join" in `site/index.html` and `site/en/index.html`; core historical review logs and the new plan's forbidden-term examples are acceptable context but should be allowlisted or qualified in future terminology lint.
- Action: `qa_engineer` findings integrated: `doc/testing/templates/hosted-world-share-announcement-template.md` and `doc/testing/templates/hosted-world-share-correction-template.md` still use `hosted world` as copyable player/announcement object; shared-network reusable templates still default to old shared-devnet tracks without legacy/rehearsal framing; `testing-manual.md` itself is acceptable because it explicitly marks `shared_devnet` as legacy/rehearsal and not public launch readiness.
- Action: `blockchain_ops_engineer` findings integrated: p2p README and PRD index still use `hosted world` as active entrypoint/topic label; p2p main PRD uses `hosted world` as active capability name and needs a glossary/SC qualifier; formal network tier docs are acceptable operator/runtime usage but should explicitly state tier is a runtime/verification carrier for the unified persistent world, not a player-facing world model.
- Action: `runtime_engineer` findings integrated: `crates/oasis7/src/bin/oasis7_chain_runtime/cli.rs` exposes old default `world_id = "live-llm_bootstrap"` and generic "world identifier" help as an active operator-facing default; web launcher network-tier strings should be qualified as technical tier config; world-runtime evidence/project entries using "formal-release default world", "execution world", or "local world" need technical qualifiers where active.
- Validation Command: `rg -n -i "shared[_ -]?devnet|shared network|public[_ -]?testnet world|local[_ -]?devnet world|hosted world|hosted web join|multiple worlds|multi-world|world selector|世界选择|多个世界|共享开发|共享网络" README.md world-rule.md site doc/core doc/game doc/readme doc/testing doc/p2p doc/world-runtime testing-manual.md`
- Expected Result: Residual old semantics can be separated into actionable active/player-facing findings, acceptable legacy/evidence contexts, and technical terms needing qualifiers.
- Actual Result: Completed; actionable findings are summarized above and will be reported to the user with file/line evidence.
- Blocker / Next Action: Close review agents and return findings. Do not claim terminology migration is complete until P1/P2 active templates, site copy, p2p entrypoints, and runtime default/help are remediated.

## 2026-06-16 19:20:00 CST / tpm
- 完成内容: Applied cleanup for the actionable residual old-semantics findings from the review pass.
- 遗留事项: Historical evidence files, old review logs, legacy topic filenames, and code-internal `execution world` storage terms still intentionally exist. They are not treated as active player-world semantics unless surfaced without legacy/operator/runtime qualification.
- Action: Updated active runtime/operator wording: `DEFAULT_WORLD_ID` now defaults to `oasis7-unified-world-v1`; `--world-id` help describes a technical runtime partition id for the unified persistent world; launcher network-tier help/errors now say chain runtime technical tier.
- Action: Updated public/player-facing copy and templates: site hosted join text now says public web/player entry; hosted announcement/correction/operator templates now describe hosted access/player entry into the unified persistent world; shared-network templates are marked legacy rehearsal and not valid for active product/player-facing unified-world claims.
- Action: Updated p2p/world-runtime/testing docs: p2p README/index/project/PRD now use hosted player access and mark old hosted-world labels as legacy topic labels; formal network-tier docs state tiers are operator/runtime carriers for the unified persistent world, not player world models; world-runtime docs now say committed execution state/cache or technical `world_id`; `testing-manual.md` network-tier and governance evidence wording now uses operator/runtime tier and committed execution-state.
- Validation Command: `rg -n "hosted world|shared world|live-llm_bootstrap|default world|local world|release default world|world identifier|目标层级|三层目标网络|三层目标模型" doc/core doc/p2p doc/world-runtime doc/testing/templates doc/testing/evidence/README.md testing-manual.md site crates/oasis7/src/bin/oasis7_chain_runtime/cli.rs crates/oasis7/src/bin/oasis7_chain_runtime/oasis7_chain_runtime_tests.rs crates/oasis7/src/bin/oasis7_web_launcher.rs crates/oasis7/src/bin/oasis7_web_launcher/control_plane.rs crates/oasis7/src/bin/oasis7_web_launcher/gui_agent_api.rs -g '!doc/core/unified-persistent-world-terminology-upgrade-plan-2026-06-16.md' -g '!doc/p2p/blockchain/p2p-shared-network-*' -g '!doc/p2p/blockchain/p2p-hosted-world-*'`
- Expected Result: No remaining active player/product copy that treats hosted/shared/default/local worlds as separate player worlds.
- Actual Result: Passed with expected residuals only: historical core review logs, story outline "目标层级", p2p legacy-label statements for old hosted-world topic names, and the intentional SC-26A/DEC-P2P-017 legacy explanation.
- Validation Command: `git diff --check`
- Expected Result: No whitespace errors.
- Actual Result: Passed with no output.
- Validation Command: `./scripts/cargo-dev.sh check -p oasis7 --bin oasis7_chain_runtime --bin oasis7_web_launcher`
- Expected Result: Affected runtime binaries still compile.
- Actual Result: Passed after waiting for shared Cargo cache/build locks; only pre-existing warnings were observed (`oasis7_node` unused import/dead code; `oasis7_web_launcher` unused import/dead code).
- Validation Command: `./scripts/doc-governance-check.sh`
- Expected Result: Full doc governance accepts the cleanup.
- Actual Result: Not completed in the final cleanup pass: repeated instances produced no output under high system load, and a 120-second alarm run exited `142`. Earlier in this task, before the cleanup edits, the same command had passed with `doc-governance-check: OK`; current cleanup still needs a fresh observed pass before PR/closeout.
- Validation Command: `./scripts/cargo-dev.sh test -p oasis7 --bin oasis7_chain_runtime parse_options_defaults -- --nocapture`
- Expected Result: The updated default `world_id` assertion passes.
- Actual Result: Not completed: two direct runs were terminated with `143` after long silent compile/wait periods; a heartbeat-wrapped raw cargo run was stopped manually after several minutes because another worktree's build held the shared build directory lock. No assertion failure was observed.
- Blocker / Next Action: Before PR/closeout, rerun `./scripts/doc-governance-check.sh` and the targeted `parse_options_defaults` test once the external build/lock pressure clears.

## 2026-06-16 19:38:00 CST / tpm
- 完成内容: Re-ran residual terminology scans across files and code after the cleanup pass.
- 遗留事项: This entry records scan results only; no additional cleanup edits were applied in this pass.
- Validation Command: `rg -n -i "shared[_ -]?devnet|shared[_ -]?network|shared world|hosted world|hosted web join|live-llm_bootstrap|default world|local world|release default world|world identifier|multiple worlds|multi-world|world selector|世界选择|多个世界|共享开发|共享网络|三层目标模型|三层目标网络|目标层级" . --glob '!target/**' --glob '!third_party/**' --glob '!output/**' --glob '!.git/**' --glob '!doc/testing/evidence/**' --glob '!doc/core/unified-persistent-world-terminology-upgrade-plan-2026-06-16.md'`
- Expected Result: Surface remaining active and historical residuals after excluding generated/build output and the new plan's deliberate forbidden-term examples.
- Actual Result: Completed. Highest-count residuals were legacy/rehearsal scripts (`scripts/shared-devnet-rehearsal.sh`, smoke variants, `scripts/shared-devnet-blocker-packet.sh`), active hosted-access topic docs (`doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.{prd,design,project,runbook}.md`), network-tier manifest compatibility code (`crates/oasis7/src/network_tier_manifest.rs`, `scripts/network-tier-manifest.sh`), testing templates/entrypoints, and historical/legacy p2p docs.
- Validation Command: `rg -n -i "hosted world|hosted web join|live-llm_bootstrap|world identifier|default world|local world|release default world|三层目标模型|三层目标网络" site crates scripts doc/testing/README.md doc/testing/prd.index.md doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.* doc/world-runtime doc/world-simulator testing-manual.md --glob '!doc/testing/evidence/**'`
- Expected Result: Confirm whether previously fixed active default/help/site copy still has old wording.
- Actual Result: Active code default/help and site copy are clean for `live-llm_bootstrap`, `world identifier`, and `hosted web join`. Remaining notable hits: the 2026-03-25 hosted-access topic docs still use `hosted world` as an active capability name; `scripts/site-homepage-claim-check.sh` still denies "hosted web join" in claim-check fixtures; `doc/world-simulator/launcher/game-client-launcher-chain-runtime-decouple-2026-02-28.prd.md` still records the old `--world-id` default `live-llm_bootstrap`; `doc/world-simulator/viewer/viewer-manual.manual.md` still says "formal-release default world"; one runtime-live test assertion says "local world" in an error message.
- Validation Command: `rg -n "shared_devnet|shared-network|shared network" crates scripts doc/testing/templates doc/testing/README.md doc/testing/prd.index.md doc/p2p doc/engineering/governance/environment-lanes-and-inventory-2026-05-29.md testing-manual.md --glob '!doc/testing/evidence/**' --glob '!doc/p2p/blockchain/p2p-shared-network-*'`
- Expected Result: Separate acceptable legacy/rehearsal compatibility from active product/player-facing leaks.
- Actual Result: Completed. Acceptable/intentional residuals include explicitly qualified legacy/rehearsal environment docs and testing-manual notes. Needs follow-up decision: `shared_devnet` remains an accepted manifest tier in `crates/oasis7/src/network_tier_manifest.rs` and `scripts/network-tier-manifest.sh`; shared-devnet rehearsal scripts still expose old names by default; testing README/index still use `hosted-world` / `p2p-shared-network` as hot-path labels; `doc/testing/templates/network-tier-shared-devnet.example.json` and related lane TSV templates remain reusable old-name examples.
- Blocker / Next Action: If continuing cleanup, prioritize (1) hosted-access topic docs, (2) testing README/index labels, (3) old world-simulator PRD/manual wording, (4) runtime test assertion text, and then make a specialist-gated decision on whether `shared_devnet` manifest/script compatibility should stay as legacy accepted input or move behind explicit legacy naming.

## 2026-06-16 20:02:00 CST / tpm
- 完成内容: Cleaned the active residual terminology identified in the 19:38 scan.
- 遗留事项: File paths and topic slugs containing `hosted-world` remain as legacy topic identifiers for traceability. `shared_devnet` manifest/script compatibility remains intentionally unmodified pending a runtime/blockchain-ops compatibility decision.
- Action: Reworded the 2026-03-25 hosted-access PRD/design/project/runbook so the active concept is `hosted player access` / `hosted access`, not `hosted world` as a player-world object.
- Action: Reworded testing module entry labels from `hosted-world` / `p2p-shared-network` to `hosted access` / `legacy p2p shared-network rehearsal`.
- Action: Reworded old world-simulator docs that still stated `live-llm_bootstrap`, `formal-release default world`, and runtime-live test text that said `local world`.
- Action: Reworded p2p main PRD, hosted public join custody docs, network reachability design, and viewer auth text from `hosted-world` / `hosted-ready` to hosted access terminology.
- Validation Command: `rg -n -i "hosted world|hosted-world|hosted-ready|hosted_ready|hosted web join|live-llm_bootstrap|world identifier|default world|local world|release default world|三层目标模型|三层目标网络" site crates scripts doc/testing/README.md doc/testing/prd.index.md doc/p2p doc/world-runtime doc/world-simulator testing-manual.md --glob '!doc/testing/evidence/**' --glob '!doc/p2p/blockchain/p2p-shared-network-*' --glob '!doc/core/unified-persistent-world-terminology-upgrade-plan-2026-06-16.md'`
- Expected Result: No active old product/world semantics remain outside traceable legacy identifiers or false-positive technical variables.
- Actual Result: Passed with expected residuals only: file/topic paths containing `p2p-hosted-world...`, evidence/template paths such as `doc/testing/evidence/hosted-world-*` and `doc/testing/templates/hosted-world-*`, explicit legacy labels in `doc/p2p/README.md` and `doc/p2p/prd.md`, plus a false-positive local shell variable `local world_id` in `scripts/p2p-longrun-soak.sh`.
- Validation Command: `rg -n -i "hosted player access access|join a hosted player access|玩家可访问 hosted player access|hosted-world 路径|hosted-world 权限|hosted-world admission|hosted-world 平面|hosted-world session|hosted-world 安全|hosted-world abuse|hosted-world player" doc/p2p doc/testing crates scripts --glob '!doc/testing/evidence/**'`
- Expected Result: No obvious awkward mechanical replacement artifacts remain.
- Actual Result: Passed with no output.
- Validation Command: `git diff --check`
- Expected Result: No whitespace errors.
- Actual Result: Passed with no output.
- Validation Command: `./scripts/site-homepage-claim-check.sh`
- Expected Result: Homepage claim/parity fixtures still match the updated hosted-access/public-player-entry copy.
- Actual Result: Passed with `ok: homepage claim/parity, metadata, and no-js navigation markers are present` after aligning the script fixture to the existing public web player entry wording.
- Validation Command: `./scripts/doc-governance-check.sh`
- Expected Result: Full doc governance accepts the cleanup.
- Actual Result: Not completed in this turn; the command produced no output for multiple polling windows and was interrupted with Ctrl-C to avoid leaving a long-running silent process.
- Blocker / Next Action: Run doc governance and any focused tests before PR/closeout; decide separately whether to migrate legacy manifest/script tier compatibility.

## 2026-06-16 20:18:00 CST / tpm
- 完成内容: Cleaned the last active `hosted-ready` residuals from hosted access share/correction templates after the follow-up scan.
- 遗留事项: File paths and topic slugs containing `hosted-world` remain as legacy identifiers for traceability. `shared_devnet` manifest/script/test compatibility remains intentionally unmodified pending a separate runtime/blockchain-ops compatibility decision.
- Action: Reworded `doc/testing/templates/hosted-world-share-correction-template.md` so the usage rule forbids upgrading preview copy to hosted access readiness / production commitments without using the old `hosted-ready` term.
- Action: Reworded `doc/testing/templates/hosted-world-share-announcement-template.md` with the same hosted access readiness / production commitment boundary.
- Validation Command: `rg -n -i "hosted world|hosted-world|hosted-ready|hosted_ready|hosted web join|live-llm_bootstrap|default world|local world|release default world|world identifier|三层目标模型|三层目标网络" site crates scripts doc/core doc/p2p doc/testing doc/world-runtime doc/world-simulator testing-manual.md --glob '!doc/testing/evidence/**' --glob '!doc/core/reviews/**' --glob '!doc/core/unified-persistent-world-terminology-upgrade-plan-2026-06-16.md' --glob '!doc/p2p/blockchain/p2p-shared-network-*' --glob '!target/**' --glob '!third_party/**'`
- Expected Result: No active old player/product world semantics remain; residuals are only traceable path/topic slugs, explicit legacy labels, or technical false positives.
- Actual Result: Passed with expected residuals only: hosted-world file/template/evidence paths and explicit legacy labels, plus false-positive shell `local world_id` in `scripts/p2p-longrun-soak.sh`.
- Validation Command: `rg -n "shared_devnet|shared-network|shared network" crates scripts doc/testing/templates doc/testing/README.md doc/testing/prd.index.md doc/p2p doc/engineering/governance/environment-lanes-and-inventory-2026-05-29.md testing-manual.md --glob '!doc/testing/evidence/**' --glob '!doc/p2p/blockchain/p2p-shared-network-*' --glob '!target/**' --glob '!third_party/**'`
- Expected Result: Remaining `shared_devnet` / shared-network occurrences are compatibility, explicitly legacy/rehearsal, or historical testing-gate terminology.
- Actual Result: Completed. Residuals are legacy/rehearsal docs/templates, shared-devnet scripts/smokes, network-tier manifest compatibility/tests, and p2p historical evidence references; no new player-facing world-model leak was identified.
- Validation Command: `git diff --check`
- Expected Result: No whitespace errors.
- Actual Result: Passed with no output.
- Blocker / Next Action: Before PR/closeout, rerun full doc governance and focused runtime tests when the shared build/doc-check environment is responsive.

## 2026-06-16 20:26:00 CST / tpm
- 完成内容: Re-ran residual file/code scans after the hosted-ready template cleanup.
- 遗留事项: No additional cleanup edits were applied in this scan pass. Remaining hits are path/topic slugs, explicit legacy labels, compatibility scripts/tests, or known false positives.
- Validation Command: `rg -n -i "hosted world|hosted-world|hosted-ready|hosted_ready|hosted web join|live-llm_bootstrap|shared world|default world|local world|release default world|world identifier|multiple worlds|multi-world|world selector|世界选择|多个世界|共享大世界|共享世界|共享开发|三层目标模型|三层目标网络" site crates scripts doc/core doc/p2p doc/testing doc/world-runtime doc/world-simulator testing-manual.md --glob '!doc/testing/evidence/**' --glob '!doc/core/reviews/**' --glob '!doc/core/unified-persistent-world-terminology-upgrade-plan-2026-06-16.md' --glob '!doc/p2p/blockchain/p2p-shared-network-*' --glob '!target/**' --glob '!third_party/**'`
- Expected Result: Confirm that active old player-world terminology does not reappear after cleanup.
- Actual Result: Completed. Remaining hits are `hosted-world` path/topic references, explicit hosted-world legacy labels, `local world_id` shell-variable false positive, and already-qualified formal network-tier notes that state tiers are not multiple player worlds.
- Validation Command: `rg -n "shared_devnet|shared-network|shared network" crates scripts doc/testing/templates doc/testing/README.md doc/testing/prd.index.md doc/p2p doc/engineering/governance/environment-lanes-and-inventory-2026-05-29.md testing-manual.md --glob '!doc/testing/evidence/**' --glob '!doc/p2p/blockchain/p2p-shared-network-*' --glob '!target/**' --glob '!third_party/**'`
- Expected Result: Separate accepted legacy/rehearsal compatibility from active product leaks.
- Actual Result: Completed. Remaining hits are explicitly legacy/rehearsal docs, old shared-network gate templates/scripts, network-tier manifest compatibility/tests, and p2p historical evidence/gate references.
- Validation Command: `rg -n -i "hosted-ready|hosted_ready|hosted web join|live-llm_bootstrap|world identifier" . --glob '!target/**' --glob '!third_party/**' --glob '!output/**' --glob '!.git/**' --glob '!doc/testing/evidence/**' --glob '!doc/core/unified-persistent-world-terminology-upgrade-plan-2026-06-16.md'`
- Expected Result: The highest-risk stale terms have no remaining hits outside deliberate evidence/plan exclusions.
- Actual Result: Passed with no output.
- Blocker / Next Action: If further cleanup is requested, the next real scope is a separate compatibility decision on `shared_devnet` / shared-network script and manifest support, not another terminology-only edit.

## 2026-06-16 23:22:00 CST / tpm
- 完成内容: Migrated the active `shared_devnet` / `shared-network` manifest and script compatibility layer toward public-testnet rehearsal / network-rehearsal terminology.
- 遗留事项: Legacy wrapper script filenames remain for compatibility only: `scripts/shared-network-track-gate.sh`, `scripts/shared-devnet-rehearsal.sh`, and `scripts/shared-devnet-blocker-packet.sh` now print migration warnings and forward to canonical scripts. Historical evidence paths were not renamed.
- Action: Runtime slice integrated: manifest canonical tier set is now `local_devnet | public_testnet | mainnet`; old shared-devnet semantics are represented by `tier=public_testnet` plus `status=rehearsal`, while release/gate track names may use `public_testnet_rehearsal`.
- Action: Blockchain-ops slice integrated: added canonical operator entrypoints `scripts/network-rehearsal-track-gate.sh`, `scripts/public-testnet-rehearsal.sh`, and `scripts/public-testnet-rehearsal-blocker-packet.sh`; old active scripts are now thin wrappers.
- Action: QA slice integrated: added canonical public-testnet rehearsal fixture files, fixed missing fixture paths, and made smoke scripts call existing canonical entrypoints.
- Action: Updated Rust/runtime compatibility: removed `shared_devnet` from manifest validation and chain runtime readiness/peer-head policy branches; updated promotion fixtures to use `local_devnet` and `public_testnet_rehearsal_pass`.
- Action: Updated script/tooling compatibility: `scripts/network-tier-manifest.sh` rejects `shared_devnet` and `public_testnet_rehearsal` as manifest tiers with a migration hint to use `--tier public_testnet --status rehearsal`; rehearsal scripts and smoke outputs now use `network-rehearsal` and `public_testnet_rehearsal`.
- Validation Command: `./scripts/network-tier-manifest-smoke.sh`
- Expected Result: Network-tier manifest tooling accepts canonical public-testnet rehearsal fixtures and rejects unsupported legacy/coarse gates as expected.
- Actual Result: Passed with `network-tier-manifest smoke passed`.
- Validation Command: `./scripts/shared-network-track-gate-smoke.sh`
- Expected Result: Legacy smoke path succeeds through canonical `network-rehearsal-track-gate.sh`.
- Actual Result: Passed with `network rehearsal track gate smoke checks passed`.
- Validation Command: `./scripts/shared-devnet-blocker-packet-smoke.sh`
- Expected Result: Legacy smoke path succeeds through canonical `public-testnet-rehearsal-blocker-packet.sh`.
- Actual Result: Passed with `public-testnet-rehearsal blocker packet smoke checks passed`.
- Validation Command: `./scripts/shared-devnet-rehearsal-smoke.sh`
- Expected Result: Legacy smoke path succeeds through canonical `public-testnet-rehearsal.sh` and generated outputs do not use old shared-devnet schema wording.
- Actual Result: Passed with `public-testnet-rehearsal rehearsal smoke checks passed`.
- Validation Command: `./scripts/release-candidate-bundle-smoke.sh`
- Expected Result: Release candidate bundle smoke accepts the canonical `public_testnet_rehearsal` track.
- Actual Result: Passed with `release candidate bundle smoke checks passed`.
- Validation Command: `./scripts/check-script-executable-bits.sh`
- Expected Result: Newly added canonical scripts and wrapper scripts are executable where required.
- Actual Result: Passed with `ok: required release scripts are tracked and executable`.
- Validation Command: `bash -n scripts/network-tier-manifest.sh scripts/network-rehearsal-track-gate.sh scripts/public-testnet-rehearsal.sh scripts/public-testnet-rehearsal-blocker-packet.sh scripts/shared-network-track-gate.sh scripts/shared-devnet-rehearsal.sh scripts/shared-devnet-blocker-packet.sh scripts/shared-network-track-gate-smoke.sh scripts/shared-devnet-rehearsal-smoke.sh scripts/shared-devnet-blocker-packet-smoke.sh scripts/network-tier-manifest-smoke.sh`
- Expected Result: Touched shell scripts parse under the local shell parser.
- Actual Result: Passed with no output.
- Validation Command: `rg -n "shared_devnet|shared-network|shared network|shared-devnet" crates scripts doc/testing/templates doc/testing/README.md doc/testing/prd.index.md testing-manual.md doc/p2p/prd.md --glob '!target/**' --glob '!third_party/**'`
- Expected Result: Remaining active-code hits are only legacy wrapper warnings or explicit manifest migration rejection, not canonical operator usage or output schema.
- Actual Result: Passed with expected residuals only: three legacy wrapper warnings and the manifest-tool rejection guard for old tier inputs.
- Validation Command: `rg -n '"tier": "public_testnet_rehearsal"|public_testnet_rehearsal_rehearsal|public-testnet-rehearsal-rehearsal' . --glob '!target/**' --glob '!third_party/**' --glob '!output/**' --glob '!.git/**'`
- Expected Result: No manifest fixture uses `public_testnet_rehearsal` as a tier and no double-rehearsal names remain.
- Actual Result: Passed with no output.
- Validation Command: `git diff --check`
- Expected Result: No whitespace errors.
- Actual Result: Passed with no output.
- Validation Command: `./scripts/release-gate-smoke.sh`
- Expected Result: Release gate smoke still runs after track rename.
- Actual Result: Not completed because local `/usr/bin/env bash` rejected the pre-existing `declare -A` usage in `scripts/release-gate.sh` with `declare: -A: invalid option`; this appears to be an existing Bash-version compatibility issue rather than a migration assertion failure.
- Validation Command: `./scripts/cargo-dev.sh test -p oasis7 network_tier_manifest -- --nocapture`
- Expected Result: Rust manifest unit tests pass after removing `shared_devnet` tier support.
- Actual Result: Not completed. The run waited on the shared build lock, then compiled with existing warnings, then stayed silent for several minutes; it was interrupted with Ctrl-C to avoid leaving a long-running session. No assertion failure was observed before interruption.
- Blocker / Next Action: Before PR/closeout, rerun Rust network-tier tests in a quiet build window and either run `release-gate-smoke` with a Bash version that supports associative arrays or fix that pre-existing shell compatibility separately.

## 2026-06-16 23:45:00 CST / tpm
- 完成内容: Investigated root causes for the previously incomplete / failing verification boundaries.
- 遗留事项: Full `./scripts/release-gate-smoke.sh` still reaches downstream Bash-4-only dependencies in `scripts/p2p-longrun-soak.sh` (`mapfile`, `declare -A`) and likely `scripts/s10-five-node-game-soak.sh` (`declare -A`). That broader compatibility cleanup is outside this migration patch unless we explicitly decide to harden all longrun scripts for macOS Bash 3.2.
- Action: Reproduced the original release-gate failure. Local `/usr/bin/env bash` resolves to GNU Bash 3.2.57; `scripts/release-gate.sh` used `declare -A`, which is Bash 4+. Root cause: the script claimed portable bash but used Bash 4 associative arrays.
- Action: Patched `scripts/release-gate.sh` to use Bash-3-compatible per-step field helper variables instead of associative arrays.
- Action: Re-ran `./scripts/release-gate.sh --dry-run --skip-s9 --skip-s10 --out-dir .tmp/release_gate_root_probe`; it passed and wrote `.tmp/release_gate_root_probe/20260616-234003/release-gate-summary.md`, confirming the release-gate root script no longer fails on `declare -A`.
- Action: Re-ran `./scripts/release-gate-smoke.sh`; it progressed past `release-gate.sh --dry-run` and then failed in `scripts/p2p-longrun-soak.sh` with `mapfile: command not found`. Root cause: downstream longrun scripts also require Bash 4 features under macOS Bash 3.2.
- Action: Removed the remaining `mapfile` usage from `scripts/public-testnet-rehearsal.sh` so the newly added canonical rehearsal script stays Bash-3-compatible.
- Action: Re-ran `./scripts/shared-devnet-rehearsal-smoke.sh`; it passed with `public-testnet-rehearsal rehearsal smoke checks passed`.
- Action: Re-ran `rg -n "mapfile|declare -A" scripts/release-gate.sh scripts/public-testnet-rehearsal.sh scripts/network-rehearsal-track-gate.sh scripts/public-testnet-rehearsal-blocker-packet.sh`; it passed with no output.
- Action: Re-ran `git diff --check`; it passed with no output.
- Action: Re-ran `./scripts/cargo-dev.sh test -p oasis7 --lib network_tier_manifest -- --nocapture`; it passed all 4 manifest unit tests.
- Action: Re-ran `./scripts/cargo-dev.sh test -p oasis7 --bin oasis7_chain_runtime network_tier -- --list`; the previous `DEFAULT_WORLD_ID` import error was fixed by importing `DEFAULT_WORLD_ID` at the bin root, but the bin test build still fails on unrelated existing API drift: missing `oasis7_node::LiveTransportTransition`, missing `oasis7_node::LiveTransportTransitionCounters`, and missing `NodeConsensusSnapshot.storage_challenge_network_degraded_{height,reason}` fields referenced by `status_payload.rs` and observability tests.
- Validation Command: `./scripts/release-gate.sh --dry-run --skip-s9 --skip-s10 --out-dir .tmp/release_gate_root_probe`
- Expected Result: The release-gate root script no longer fails under macOS Bash 3.2.
- Actual Result: Passed; dry-run summary emitted successfully.
- Validation Command: `./scripts/release-gate-smoke.sh`
- Expected Result: Determine whether the old failure was fully fixed or whether another downstream compatibility issue remains.
- Actual Result: Advanced past the original `declare -A` failure, then failed at `scripts/p2p-longrun-soak.sh:561` with `mapfile: command not found`; root cause is downstream Bash 4 dependency, not the manifest/script migration.
- Validation Command: `./scripts/cargo-dev.sh test -p oasis7 --lib network_tier_manifest -- --nocapture`
- Expected Result: Verify the manifest migration unit tests without compiling unrelated bin tests.
- Actual Result: Passed; 4 passed, 0 failed, with existing warnings only.
- Validation Command: `./scripts/cargo-dev.sh test -p oasis7 --bin oasis7_chain_runtime network_tier -- --list`
- Expected Result: Check whether the chain runtime network-tier bin test surface compiles after fixing `DEFAULT_WORLD_ID`.
- Actual Result: Failed on unrelated status/observability API drift after the `DEFAULT_WORLD_ID` issue was fixed.
- Blocker / Next Action: For this migration, the verified passing boundary is the library manifest tests plus the rehearsal/gate smokes. Full release-gate smoke requires a separate Bash 3 compatibility pass over longrun scripts, and chain-runtime bin tests require a separate status/observability API drift fix.

## 2026-06-17 00:05:00 CST / tpm
- 完成内容: Added a code-layer follow-up TODO to the unified persistent world terminology upgrade plan.
- 遗留事项: This is a planned follow-up scope, not completed implementation. It should be handled as a separate code-layer task/slice before claiming all runtime/bin/release-gate verification complete.
- Action: Added `Phase 3B: Code-layer Follow-up TODO` to `doc/core/unified-persistent-world-terminology-upgrade-plan-2026-06-16.md`.
- Action: Follow-up scope covers `oasis7_chain_runtime` status/observability API drift, downstream release-gate longrun Bash runtime compatibility, legacy wrapper retirement strategy, and code-level terminology/compatibility scan.
- Validation Command: Manual doc update review.
- Expected Result: The plan clearly distinguishes completed documentation/operator-entry migration from remaining code-layer work.
- Actual Result: Completed; Phase 3B now captures code-layer TODO, suggested verification, and acceptance criteria.
- Blocker / Next Action: Create or bind a separate follow-up task before implementing Phase 3B code-layer fixes.

## 2026-06-17 09:29:00 CST / tpm
- 完成内容: Promoted the Phase 3B code-layer follow-up into the global `.pm` TODO/task surface.
- 遗留事项: Follow-up implementation has not started; this entry only records global TODO/task creation.
- Action: Ran `./scripts/pm/capture-todo.sh --create-task` using `doc/core/unified-persistent-world-terminology-upgrade-plan-2026-06-16.md` as the source ref.
- Action: Created signal `SIG-PM-0068` and candidate task `task_acb7e3599b4242628a7ac99a62628d55`.
- Action: Candidate task owner is `runtime_engineer`; handoff roles are `blockchain_ops_engineer`, `qa_engineer`, and `repository_health_engineer`.
- Action: Candidate task acceptance covers chain-runtime network-tier bin test discovery, release-gate shell runtime compatibility, legacy wrapper retirement strategy, and code-level `shared_devnet` regression scans.
- Validation Command: `sed -n '1,120p' .pm/tasks/task_acb7e3599b4242628a7ac99a62628d55.yaml`; `sed -n '1,120p' .pm/roles/runtime_engineer/backlog/candidate.yaml`
- Expected Result: The new TODO appears as a canonical `.pm` candidate task and in the runtime engineer candidate backlog.
- Actual Result: Passed; task YAML and runtime engineer candidate backlog both reference `task_acb7e3599b4242628a7ac99a62628d55`.
- Blocker / Next Action: Use the new candidate task as the global follow-up entry when starting code-layer implementation.

## 2026-06-17 09:39:00 CST / tpm
- 完成内容: Started the pre-PR local role review gate for the unified world terminology migration branch.
- 遗留事项: Review results must be integrated and any valid findings addressed before `prepare-task-pr.sh --create`.
- Action: Review Trigger: pre-PR local role review.
- Action: Review Scope: full branch diff from `main` to `task/core-unified-world-terminology-upgrade-plan`, covering `.pm` task/TODO state, core/p2p/testing/world-runtime docs, site copy, network-tier manifest code, public-testnet rehearsal scripts, legacy wrapper scripts, release-gate Bash compatibility patch, and focused Rust/viewer terminology updates.
- Action: Review Roles: `repository_health_engineer`, `qa_engineer`, `blockchain_ops_engineer`.
- Action: Role Selection Basis: repository-health is required for cross-cutting docs/scripts/code contracts and large terminology cleanup; QA is required because the PR claim depends on verification boundaries and explicitly documented non-passing gates; blockchain-ops is required because manifest tiers, rehearsal/public-testnet operator scripts, and legacy wrapper migration are changed. Gameplay, visual-interaction, viewer, wasm, agent, and liveops/community are skipped because this branch does not change gameplay rules, visual interaction behavior, WASM platform contracts, agent runtime behavior, or external community/incident commitments beyond existing template wording.
- Action: Review Question: confirm whether the branch is ready to enter PR creation with the current compatibility boundaries, residual old-term handling, verification evidence, and global TODO follow-up.
- Action: Evidence Available: passing focused script smokes, Rust manifest library tests, `git diff --check`, residual `rg` scans, documented root causes for non-passing release-gate-smoke and chain-runtime bin test discovery, and global follow-up task `task_acb7e3599b4242628a7ac99a62628d55`.
- Action: Expected Return Contract: `findings` with severity/file/line/reason, or `no_findings`, plus `residual_risk`.
- Validation Command: Subagent review dispatch for the three listed roles.
- Expected Result: Each role returns findings or no_findings with residual risk before PR creation.
- Actual Result: Pending.
- Blocker / Next Action: Integrate role outputs, patch valid findings if any, then record the passed pre-PR evidence packet.

## 2026-06-17 09:50:00 CST / tpm
- 完成内容: Integrated pre-PR local role review findings and patched valid merge-risk issues before PR creation.
- 遗留事项: Branch still needs sync with current `main` before final preflight, because stale-base review found unrelated deletions in the PR diff.
- Action: `repository_health_engineer`, `qa_engineer`, and `blockchain_ops_engineer` review outputs were received. Valid findings: stale branch would delete unrelated current-main files; `PR.md` contained unrelated old PR evidence; canonical public-testnet/network-rehearsal scripts still emitted old shared-devnet/shared-network headings/schema; release candidate bundles allowed unsupported old `shared_devnet` track values; old shared-devnet template paths remained prominent as validation entries.
- Action: Replaced `PR.md` with task-specific evidence for `task_cb987cd0fdfb4ecc98a6ddde7d96204c`.
- Action: Added release-candidate-bundle track allowlisting so create/validate only accept `public_testnet_rehearsal`, `staging`, and `canary`; old `shared_devnet`/`shared_network` inputs now fail with a migration hint.
- Action: Changed canonical network rehearsal gate schema/title to `oasis7.network_rehearsal_track_gate.v1` and `Network Rehearsal Track Gate Summary`.
- Action: Changed canonical public-testnet rehearsal and blocker-packet generated markdown titles away from old shared-devnet/shared-network headings.
- Action: Removed old shared-devnet template files and updated active network-tier docs to reference canonical public-testnet rehearsal templates instead.
- Validation Command: `./scripts/release-candidate-bundle.sh create --bundle .tmp/invalid-shared-devnet-bundle.json --candidate-id invalid-old-track --track shared_devnet --runtime-build-ref scripts/release-candidate-bundle.sh --world-snapshot-ref scripts --governance-manifest-ref doc/testing/templates/network-tier-public-testnet-rehearsal.example.json --allow-dirty-worktree`
- Expected Result: Old `shared_devnet` track is rejected before bundle creation.
- Actual Result: Passed as a negative test; command exited 2 with `error: unsupported --track: shared_devnet` and migration hint.
- Validation Command: `rg -n "Shared Devnet|Shared Network|oasis7\\.shared_network_track_gate|\"track\": \"shared_devnet\"" .tmp/shared_network_track_gate_smoke .tmp/public_testnet_rehearsal_smoke .tmp/public_testnet_rehearsal_blocker_packet_smoke --glob '!**/*.log'`
- Expected Result: Fresh canonical generated artifacts do not contain old generated titles/schema/track truth.
- Actual Result: Passed with no output.
- Validation Command: `rg -n "network-tier-shared-devnet\\.example\\.json|shared-devnet-genesis\\.example\\.json|shared-devnet-bootstrap\\.example\\.txt" . --glob '!target/**' --glob '!third_party/**' --glob '!.git/**'`
- Expected Result: Old shared-devnet template paths are no longer referenced as active validation entries.
- Actual Result: Passed with no output.
- Validation Command: `./scripts/release-candidate-bundle-smoke.sh`; `./scripts/shared-network-track-gate-smoke.sh`; `./scripts/shared-devnet-rehearsal-smoke.sh`; `./scripts/shared-devnet-blocker-packet-smoke.sh`; `./scripts/network-tier-manifest-smoke.sh`; `./scripts/network-tier-manifest.sh validate --manifest doc/testing/templates/network-tier-public-testnet-rehearsal.example.json`; `bash -n scripts/release-candidate-bundle.sh scripts/network-rehearsal-track-gate.sh scripts/public-testnet-rehearsal.sh scripts/public-testnet-rehearsal-blocker-packet.sh`
- Expected Result: Review-finding fixes do not regress the focused migration smoke paths.
- Actual Result: Passed.
- Blocker / Next Action: Commit the repaired branch, merge current `main`, rerun targeted validation, and then record the final pre-PR review packet.
