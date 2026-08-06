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
3. While same-head draft CI is pending, run `./scripts/pm/review-plan.py --task-uid <task_uid> --head <frozen_head> --comparison-ref <canonical_base_ref> --evidence-digest <relevant_evidence_digest> --change-class <class> [--domain-role <role>] [--manual-role <role> ...] [--verification-affected] [--preflight-dir <dir>]`. The helper resolves and records both `comparison_ref` and `comparison_oid`, composes the selector with the immutable batch contract, allocates/reuses canonical UUIDs, and emits only per-role `packet_refs`, not full task packets. Manual-role order is preserved and bound into the plan identity. `--preflight-dir` creates only incomplete collector-valid artifact/ledger skeletons and never a collection receipt or passed result. Record plan/batch paths and digests in GitHub task issue evidence comments. A retry with identical task/head/evidence/comparison/roles reuses the plan; any drift requires a distinct plan/epoch. This preparation must not dispatch formal review before CI passes.
4. Once the CI receipt for that frozen head is available, confirm the plan still binds the task/head/evidence/roles. Generate one fresh minimal task packet per involved role at the plan's reference-only locations. Immediately before each specialist spawn, run `./scripts/pm/subagent-task-packet.py review-admission --packet <packet> --review-plan <plan> --bootstrap-snapshot <snapshot>` and require its `admitted` JSON result; it validates immutable bootstrap-epoch identity and the plan's canonical batch/complete role set while the fresh packet/plan bind the later review HEAD and comparison. Any live task, snapshot identity, batch/plan role set, HEAD, comparison ref/OID, base, role, slice, or packet-reference drift fails closed and requires fresh valid inputs. The result is ephemeral evidence for that immediate dispatch, not a durable authority receipt. Then dispatch the immutable expected batch. Do not fork full parent history unless a role has a recorded escalation reason.
5. After every reviewer completes its preflight artifact, run `review-batch-epoch.py reconcile` to validate all returns and atomically publish the completed, digest-current human-operated ledger; incomplete or mismatched returns fail without a receipt. Then collect the batch once with `review-batch-epoch.py collect`. Require each role to return `findings` or `no_findings`, plus `residual_risk`; resolve valid findings or reject them with evidence. Do not redispatch a complete unchanged HEAD/evidence epoch. A transport retry reuses the same immutable batch and slice identities.
6. Record the canonical packet with `record-pre-pr-review.sh --review-plan <plan>` in the GitHub task issue. The helper re-resolves the planned comparison ref and rejects task/head/ref/OID/role mismatch before GitHub write; the packet records both comparison ref and OID. Validate its frozen-head, role-complete ledger and artifacts with the repository helper.
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
