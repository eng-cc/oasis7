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
2. Select only involved roles from changed surfaces and risk. Always include `repository_health_engineer` for workflow/governance/repository surfaces and `qa_engineer` for behavior or verification coverage.
3. Generate one fresh minimal task packet bound to the frozen review head, record its path/digest in GitHub task issue evidence comments, and dispatch all involved-role reviews in one batch from that packet. Do not fork full parent history unless a role has a recorded escalation reason.
4. Collect the batch once, then require each role to return `findings` or `no_findings`, plus `residual_risk`; resolve valid findings or reject them with evidence. Do not redispatch unchanged roles when the frozen HEAD and their relevant evidence digest are unchanged.
5. Record the canonical packet in the GitHub task issue and validate its frozen-head, role-complete ledger and artifacts with the repository helper.
6. Continue only when the canonical Pre-PR Ready gate passes. Require trusted runtime attestation only when operating the future unattended supervisor.

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
