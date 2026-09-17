---
name: requesting-repo-owned-review
description: Use when a branch is about to create a PR and needs fresh involved-role review.
---

# Requesting Repo-Owned Review

Canonical contract: [pre-PR review packet](../../../doc/engineering/workflow/source-of-truth.md#pre-pr-review-packet), [Freeze](../../../doc/engineering/workflow/source-of-truth.md#freeze-gate), [Pre-PR Ready](../../../doc/engineering/workflow/source-of-truth.md#pre-pr-ready-gate).

Pre-PR local role review is required for the frozen source head before promotion. The concurrent CI/review contract is staged but not activated: the current formal v2 plan still requires the trusted exact-head CI receipt before dispatch. Pre-PR Ready and promotion require a fail-closed join of current CI and review identities; review supplements, never replaces, GitHub checks, comments, requested changes, or mergeability. The v2 plan separates immutable source professional-review identity from the latest integration-CI identity; the v1 combined path remains strict for legacy plans and this migration.

## When to Use

Use after implementation freeze and before the canonical Pre-PR Ready gate.

## Procedure

1. Freeze the implementation head and comparison ref using the canonical Freeze gate. Before entering this expensive cycle, run the smallest real boundary loop applicable to the change: configuration generation through the real parser/admission path, viewer through the runtime protocol, persistence write through real recovery, or the standard skill command through the actual helper output. These loops are early contract checks; they do not replace final trusted integration, environment/browser, or professional-review evidence.
2. Classify documentation changes with `./scripts/pm/review-role-selector.py`: mechanical/workflow docs use repository health plus QA; domain-semantic docs use repository health plus one canonical domain specialist (never TPM, QA, repository health, LiveOps/community, or an unknown role) and add QA only when verification changes; external messaging uses repository health plus LiveOps/community and adds QA only when verification changes. Unknown/mixed scope requires one or more ordered `--manual-role <canonical-review-role>` values; missing, duplicate, TPM, and unknown roles fail closed, and explicit documentation classes reject manual roles. Preserve changed-path inference as the safety floor for non-document or unclassified changes.
3. After the frozen head is recorded, the concurrent CI/review target remains staged, not activated. On the active path, start the trusted exact-head integration CI first; while it is pending, TPM may prepare only non-authoritative role-selection inputs and the shared digest-bound impact projection. After the trusted receipt succeeds, create the v2 plan and dispatch the independent formal professional-review branch. Both branches must bind the same source head but retain separate evidence identities. Prepare role-selection inputs and the shared digest-bound impact projection from write scope, consumed contracts, changed public semantics, affected consumers, tests, CI capabilities and required roles; unknown or incomplete dependency/consumer closure escalates conservatively to the broader checks/review. Run `./scripts/pm/review-plan.py --task-uid <task_uid> --head <frozen_head> --comparison-ref <canonical_base_ref> --ci-ready-receipt <receipt.json> --change-class <class> [--domain-role <role>] [--manual-role <role> ...] [--verification-affected] [--preflight-dir <dir>] [--prior-review-plan <completed-plan>] [--impacted-role <canonical-role> ...]` for the compatible v2 formal plan path. The v2 helper records a fixed source scope (`source_scope_oid`), ordered roles, `changed_paths_digest`, `role_contract_digest`, `review_policy_digest`, and `input_contract_digest` as `source_review_identity`; it records the accepted complete `tested_tree_oid` separately from the integration `base_oid`. Never hash the whole receipt file because `observed_at` is renewable liveness evidence. Before selecting roles or creating immutable artifacts, require `receipt.head_oid` to equal the frozen head and `receipt.base_oid` to remain an available ancestor; the source scope owns professional review, while the symbolic comparison ref is audit provenance and may move later. It composes the selector with the immutable batch contract, allocates/reuses canonical UUIDs, and emits only per-role `packet_refs`, not full task packets. Manual-role order is preserved and bound into the plan identity. `--preflight-dir` creates only incomplete collector-valid artifact/ledger skeletons and never a collection receipt or passed result. The optional prior plan must be a completed collected review; when an incremental context is requested, omit `--impacted-role` for a full review with `unknown_impact` escalation, or name canonical impacted roles for a scoped review. The context records per-role `full_review` or `impact_confirmation` obligations bound to prior/current heads, the delta-path digest, and a scope digest; it is an enforceable admission input, not advisory text. The legacy `--evidence-digest` input is compatibility/non-formal evidence only and cannot create a formal v2 review or satisfy the CI/review join. Record plan/batch paths and digests in GitHub task issue evidence comments. Identical v2 source identity inputs reuse the source plan only when the accepted complete tested tree is unchanged; legacy v1 and any authority drift require a distinct plan/epoch. The v2 source-review applicability shadow decision remains audit-only until explicit compatible activation and cannot authorize this branch.
4. Confirm the plan still binds the task/head/evidence/roles. Generate one fresh minimal task packet per involved role at the plan's reference-only locations, passing `--base <canonical_base_ref> --frozen-base-oid <receipt.base_oid>` and, when the plan carries incremental context, `--review-plan <plan>` so the packet embeds the validated context and role obligation. Immediately before each specialist spawn, run `./scripts/pm/subagent-task-packet.py review-admission --packet <packet> --review-plan <plan> --bootstrap-snapshot <snapshot>` and require its `admitted` JSON result; it validates immutable bootstrap-epoch identity and the plan's canonical batch/complete role set while the fresh packet/plan bind the receipt base and review HEAD. Live HEAD, base-object/ancestry, receipt authority, batch/plan role set, packet, role, or slice drift fails closed; later symbolic-ref movement alone does not. All expected roles remain required: `full_review` roles assess the current delta, while `impact_confirmation` roles must record fresh confirmation against their bound scope digest; uncertainty or authority/policy drift escalates to `full_review`. The result is ephemeral evidence for that immediate dispatch, not a durable authority receipt. Then dispatch the immutable expected batch. Do not fork full parent history unless a role has a recorded escalation reason.
5. Require each role to return `findings` or `no_findings`, plus `residual_risk`; every structured finding must carry typed triage (`blocking` or `nonblocking`) with an evidence basis. A blocking finding needs a repair or evidence-backed rejection and may not be marked `non_actionable`; nonblocking findings may be marked `non_actionable` only with rationale and residual-risk/revisit evidence. Resolve valid findings or reject them with evidence. Do not manually reconcile or collect the batch during the normal path: the closeout facade performs those operations once after all structured returns are present. Do not redispatch a complete unchanged source identity epoch. A transport retry reuses the same immutable batch and slice identities. Reused source review never substitutes for integration: a target advance requires a fresh latest exact dispatch/attempt, successful conclusion, exact current-PR readback, and complete `tested_tree_oid` equality with the accepted snapshot; a changed tree, new finding, unknown dependency/authority closure, wrong execution ref/base, current-PR drift, failure, pending or uncertain readback requires full re-review.
The target concurrent review branch remains capability-staged. On the active
path, dispatch formal review only after the trusted receipt creates the v2 plan
and immutable source-bound admission succeeds. Before Pre-PR Ready or promotion, perform the fail-closed
join against the current PR: the trusted integration identity and the
role-complete source-review identity must bind the same frozen head and
applicable impact projection. Missing, pending, uncertain, stale, drifted,
wrong-head, wrong-base, failed, or unreadable CI/review evidence blocks. A
legacy `--evidence-digest` or audit-only shadow result never satisfies this
join.

6. After structured role artifacts are complete, use
   `./scripts/pm/review-closeout.sh --task-uid <uid> --review-plan <plan>
   --role-returns <ledger>` as the normal operator entry. It
   reconciles/collects the immutable batch and delegates canonical packet
   generation to `record-pre-pr-review.sh`; use the lower-level helpers
   directly only for recovery. The validators still bind task/head/ref/OID/role
   identity and require the immutable comparison OID to remain available.
   Validate the resulting frozen-head, role-complete ledger and artifacts with
   the repository helper.
7. Continue only when the canonical Pre-PR Ready gate passes. Require trusted runtime attestation only when operating the future unattended supervisor.

Role selection exceptions:

- include `agent_engineer` only when in-world Agent perception, planning, tools, prompt/policy, or agent-facing runtime behavior changed
- repository Codex config/adapter projection/validation contracts require `repository_health_engineer` and `qa_engineer`
- for `.codex/agents/<role>.toml`, require `repository_health_engineer`, `qa_engineer`, and the matching canonical `<role>`
- include `liveops_community` for external messaging, community impact, incidents, player commitments, or channel runbooks

## Return Contract

- reviewed comparison range and frozen head
- involved roles and immutable returns
- findings disposition and residual risk
- canonical packet evidence link, or a canonical blocker with resume instruction

Do not use chat-only review or local fixture output as live task evidence. Do not resolve GitHub review threads solely from this local review. Self-signed evidence never substitutes for runtime attestation in unattended mode.

## Guardrails

Do not omit involved roles or record a passed result before findings are closed.

## Known Failure Modes

Stale-head review; hand-authored attestation; chat-only evidence; confusing local review with GitHub merge readiness.
