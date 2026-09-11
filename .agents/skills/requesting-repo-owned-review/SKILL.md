---
name: requesting-repo-owned-review
description: Use when a branch is about to create a PR and needs fresh involved-role review.
---

# Requesting Repo-Owned Review

Canonical contract: [pre-PR review packet](../../../doc/engineering/workflow/source-of-truth.md#pre-pr-review-packet), [Freeze](../../../doc/engineering/workflow/source-of-truth.md#freeze-gate), [Pre-PR Ready](../../../doc/engineering/workflow/source-of-truth.md#pre-pr-ready-gate).

Pre-PR local role review is required after the draft candidate has same-head CI evidence and before promotion. It supplements, never replaces, GitHub checks, comments, requested changes, or mergeability. The v2 plan separates immutable source professional review identity from the latest integration CI identity; the v1 combined path remains strict for legacy plans and this migration.

## When to Use

Use after implementation freeze and before the canonical Pre-PR Ready gate.

## Procedure

1. Freeze the implementation head and comparison ref using the canonical Freeze gate.
2. Classify documentation changes with `./scripts/pm/review-role-selector.py`: mechanical/workflow docs use repository health plus QA; domain-semantic docs use repository health plus one canonical domain specialist (never TPM, QA, repository health, LiveOps/community, or an unknown role) and add QA only when verification changes; external messaging uses repository health plus LiveOps/community and adds QA only when verification changes. Unknown/mixed scope requires one or more ordered `--manual-role <canonical-review-role>` values; missing, duplicate, TPM, and unknown roles fail closed, and explicit documentation classes reject manual roles. Preserve changed-path inference as the safety floor for non-document or unclassified changes.
3. Prepare only non-authoritative role-selection inputs while same-head draft CI is pending. Once the trusted receipt exists, run `./scripts/pm/review-plan.py --task-uid <task_uid> --head <frozen_head> --comparison-ref <canonical_base_ref> --ci-ready-receipt <receipt.json> --change-class <class> [--domain-role <role>] [--manual-role <role> ...] [--verification-affected] [--preflight-dir <dir>] [--prior-review-plan <completed-plan>]`. The v2 helper records a fixed source scope (`source_scope_oid`), ordered roles, `changed_paths_digest`, `role_contract_digest`, `review_policy_digest`, and `input_contract_digest` as `source_review_identity`; it records the accepted complete `tested_tree_oid` separately from the integration `base_oid`. Never hash the whole receipt file because `observed_at` is renewable liveness evidence. Before selecting roles or creating immutable artifacts, it requires `receipt.head_oid` to equal the frozen head and `receipt.base_oid` to remain an available ancestor; the source scope owns professional review, while the symbolic comparison ref is audit provenance and may move later. It composes the selector with the immutable batch contract, allocates/reuses canonical UUIDs, and emits only per-role `packet_refs`, not full task packets. Manual-role order is preserved and bound into the plan identity. `--preflight-dir` creates only incomplete collector-valid artifact/ledger skeletons and never a collection receipt or passed result. The optional prior plan must be a completed collected review; the resulting `incremental_review_context` is advisory and binds the actual prior-head-to-current-head patch digest. Record plan/batch paths and digests in GitHub task issue evidence comments. Identical v2 source identity inputs reuse the source plan only when the accepted complete tested tree is unchanged; legacy v1 and any authority drift require a distinct plan/epoch. This preparation must not dispatch formal review before CI passes.
4. Confirm the plan still binds the task/head/evidence/roles. Generate one fresh minimal task packet per involved role at the plan's reference-only locations, passing `--base <canonical_base_ref> --frozen-base-oid <receipt.base_oid>` and, when the plan carries incremental context, `--review-plan <plan>` so the packet embeds the validated advisory context. Immediately before each specialist spawn, run `./scripts/pm/subagent-task-packet.py review-admission --packet <packet> --review-plan <plan> --bootstrap-snapshot <snapshot>` and require its `admitted` JSON result; it validates immutable bootstrap-epoch identity and the plan's canonical batch/complete role set while the fresh packet/plan bind the receipt base and review HEAD. Live HEAD, base-object/ancestry, receipt authority, batch/plan role set, packet, role, or slice drift fails closed; later symbolic-ref movement alone does not. All expected roles remain required: affected roles may focus on the recorded delta, while other roles must record fresh impact confirmation in their existing evidence and residual risk. Uncertainty or authority/policy drift requires full assessment. The result is ephemeral evidence for that immediate dispatch, not a durable authority receipt. Then dispatch the immutable expected batch. Do not fork full parent history unless a role has a recorded escalation reason.
5. Require each role to return `findings` or `no_findings`, plus `residual_risk`; resolve valid findings or reject them with evidence. Do not manually reconcile or collect the batch during the normal path: the closeout facade performs those operations once after all structured returns are present. Do not redispatch a complete unchanged source identity epoch. A transport retry reuses the same immutable batch and slice identities. Reused source review never substitutes for integration: a target advance requires a fresh latest exact dispatch/attempt, successful conclusion, exact current-PR readback, and complete `tested_tree_oid` equality with the accepted snapshot; a changed tree, new finding, unknown dependency/authority closure, wrong execution ref/base, current-PR drift, failure, pending or uncertain readback requires full re-review.
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
