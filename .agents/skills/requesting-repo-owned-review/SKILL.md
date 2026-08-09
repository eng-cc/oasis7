---
name: requesting-repo-owned-review
description: Use when a branch is about to create a PR and needs fresh involved-role review.
---

# Requesting Repo-Owned Review

Canonical contract: [pre-PR review packet](../../../doc/engineering/workflow/source-of-truth.md#pre-pr-review-packet), [Freeze](../../../doc/engineering/workflow/source-of-truth.md#freeze-gate), [Pre-PR Ready](../../../doc/engineering/workflow/source-of-truth.md#pre-pr-ready-gate).

Pre-PR local role review is required after the draft candidate has same-head CI evidence and before promotion. It supplements, never replaces, GitHub checks, comments, requested changes, or mergeability.

## When to Use

Use after implementation freeze and before the canonical Pre-PR Ready gate.

## Procedure

1. Freeze the implementation head and comparison ref using the canonical Freeze gate.
2. Classify documentation changes with `./scripts/pm/review-role-selector.py`: mechanical/workflow docs use repository health plus QA; domain-semantic docs use repository health plus one canonical domain specialist (never TPM, QA, repository health, LiveOps/community, or an unknown role) and add QA only when verification changes; external messaging uses repository health plus LiveOps/community and adds QA only when verification changes. Unknown/mixed scope requires one or more ordered `--manual-role <canonical-review-role>` values; missing, duplicate, TPM, and unknown roles fail closed, and explicit documentation classes reject manual roles. Preserve changed-path inference as the safety floor for non-document or unclassified changes.
3. Prepare only non-authoritative role-selection inputs while same-head draft CI is pending. Once the trusted receipt exists, run `./scripts/pm/review-plan.py --task-uid <task_uid> --head <frozen_head> --comparison-ref <canonical_base_ref> --ci-ready-receipt <receipt.json> --change-class <class> [--domain-role <role>] [--manual-role <role> ...] [--verification-affected] [--preflight-dir <dir>]`. The helper derives the stable CI-authority digest; never hash the whole receipt file because `observed_at` is renewable liveness evidence. Before selecting roles or creating immutable artifacts, it requires `receipt.head_oid` to equal the frozen head and `receipt.base_oid` to remain an available ancestor; that immutable pair owns the reviewed range, while the symbolic comparison ref is audit provenance and may move later without invalidating the plan. It composes the selector with the immutable batch contract, allocates/reuses canonical UUIDs, and emits only per-role `packet_refs`, not full task packets. Manual-role order is preserved and bound into the plan identity. `--preflight-dir` creates only incomplete collector-valid artifact/ledger skeletons and never a collection receipt or passed result. Record plan/batch paths and digests in GitHub task issue evidence comments. A retry with identical task/head/CI authority/comparison/roles reuses the plan; any authority drift requires a distinct plan/epoch. This preparation must not dispatch formal review before CI passes.
4. Confirm the plan still binds the task/head/evidence/roles. Generate one fresh minimal task packet per involved role at the plan's reference-only locations, passing `--base <canonical_base_ref> --frozen-base-oid <receipt.base_oid>` so the packet keeps the receipt range while recording the symbolic ref only as provenance. Immediately before each specialist spawn, run `./scripts/pm/subagent-task-packet.py review-admission --packet <packet> --review-plan <plan> --bootstrap-snapshot <snapshot>` and require its `admitted` JSON result; it validates immutable bootstrap-epoch identity and the plan's canonical batch/complete role set while the fresh packet/plan bind the receipt base and review HEAD. Live HEAD, base-object/ancestry, receipt authority, batch/plan role set, packet, role, or slice drift fails closed; later symbolic-ref movement alone does not. The result is ephemeral evidence for that immediate dispatch, not a durable authority receipt. Then dispatch the immutable expected batch. Do not fork full parent history unless a role has a recorded escalation reason.
5. After every reviewer completes its preflight artifact, run `review-batch-epoch.py reconcile` to validate all returns and atomically publish the completed, digest-current human-operated ledger; incomplete or mismatched returns fail without a receipt. Then collect the batch once with `review-batch-epoch.py collect`. Require each role to return `findings` or `no_findings`, plus `residual_risk`; resolve valid findings or reject them with evidence. Do not redispatch a complete unchanged HEAD/evidence epoch. A transport retry reuses the same immutable batch and slice identities.
6. Record the canonical packet with `record-pre-pr-review.sh --review-plan <plan>` in the GitHub task issue. The helper validates task/head/ref/OID/role identity and that the immutable comparison OID remains an available commit; later symbolic-ref movement alone does not invalidate a completed review. The packet records both the audit ref and authoritative OID. Validate its frozen-head, role-complete ledger and artifacts with the repository helper.
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
