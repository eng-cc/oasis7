---
name: requesting-repo-owned-review
description: Use when a branch is about to create a PR and needs fresh involved-role review.
---

# Requesting Repo-Owned Review

Canonical contract: [pre-PR review packet](../../../doc/engineering/workflow/source-of-truth.md#pre-pr-review-packet), [Freeze](../../../doc/engineering/workflow/source-of-truth.md#freeze-gate), [Pre-PR Ready](../../../doc/engineering/workflow/source-of-truth.md#pre-pr-ready-gate).

Pre-PR local role review is required for the frozen source head before promotion. The default v2 path creates a source-only plan from the verified impact projection, then runs source-bound PR CI and professional review concurrently. Ordinary changes may retain a successful PR check whose recorded target base is older than current main when the source head, target ref, check identity, source-review applicability, and mergeability remain valid. High-risk or unknown impact uses trusted exact-current-target integration CI. Pre-PR Ready and promotion fail closed until the applicable current CI identity and role-complete review identity join; review supplements, never replaces, GitHub checks, comments, requested changes, or mergeability. Explicit v1 is compatibility-only.

## When to Use

Use after implementation freeze and before the canonical Pre-PR Ready gate.

## Procedure

1. Freeze the implementation head and comparison ref using the canonical Freeze gate. Before entering this expensive cycle, run the smallest real boundary loop applicable to the change: configuration generation through the real parser/admission path, viewer through the runtime protocol, persistence write through real recovery, or the standard skill command through the actual helper output. These loops are early contract checks; they do not replace final trusted integration, environment/browser, or professional-review evidence.
2. Classify documentation changes with `./scripts/pm/review-role-selector.py`: mechanical/workflow docs use repository health plus QA; domain-semantic docs use repository health plus one canonical domain specialist (never TPM, QA, repository health, LiveOps/community, or an unknown role) and add QA only when verification changes; external messaging uses repository health plus LiveOps/community and adds QA only when verification changes. Unknown/mixed scope requires one or more ordered `--manual-role <canonical-review-role>` values; missing, duplicate, TPM, and unknown roles fail closed, and explicit documentation classes reject manual roles. Preserve changed-path inference as the safety floor for non-document or unclassified changes.
3. After freeze, generate one digest-bound impact projection from write scope, consumed contracts, public semantics, affected consumers, tests, CI capabilities and required roles; unknown closure widens conservatively. Create the default v2 source plan with `review-plan.py --impact-projection <projection.json> ... --preflight-dir <dir>`; the helper derives source identity from bound task truth. Admit the packets and dispatch the complete role batch while exact-head CI runs independently. CI planner, role selector, plan, admission and closeout must bind the same projection digest. `--source-review-input` is compatibility input and must carry that exact projection digest; explicit `--review-schema oasis7-review-plan/v1` remains compatibility-only, and legacy `--evidence-digest` cannot satisfy v2.
4. Confirm the plan still binds the task/head/evidence/roles. Generate one fresh minimal task packet per involved role at the plan's reference-only locations, passing `--base <canonical_base_ref> --frozen-base-oid <receipt.base_oid>` and, when the plan carries incremental context, `--review-plan <plan>` so the packet embeds the validated context and role obligation. Immediately before each specialist spawn, run `./scripts/pm/subagent-task-packet.py review-admission --packet <packet> --review-plan <plan> --bootstrap-snapshot <snapshot>` and require its `admitted` JSON result; it validates immutable bootstrap-epoch identity and the plan's canonical batch/complete role set while the fresh packet/plan bind the receipt base and review HEAD. Live HEAD, base-object/ancestry, receipt authority, batch/plan role set, packet, role, or slice drift fails closed; later symbolic-ref movement alone does not. All expected roles remain required: `full_review` roles assess the current delta, while `impact_confirmation` roles must record fresh confirmation against their bound scope digest; uncertainty or authority/policy drift escalates to `full_review`. The result is ephemeral evidence for that immediate dispatch, not a durable authority receipt. Then dispatch the immutable expected batch. Do not fork full parent history unless a role has a recorded escalation reason.
5. Require each role to return `findings` or `no_findings`, plus `residual_risk`; every structured finding must carry typed triage (`blocking` or `nonblocking`) with an evidence basis. A blocking finding needs a repair or evidence-backed rejection and may not be marked `non_actionable`; nonblocking findings may be marked `non_actionable` only with rationale and residual-risk/revisit evidence. Resolve valid findings or reject them with evidence. Do not manually reconcile or collect the batch during the normal path: the closeout facade performs those operations once after all structured returns are present. Do not redispatch a complete unchanged source identity epoch. A transport retry reuses the same immutable batch and slice identities. Reused source review joins with the applicable current PR CI identity. An unrelated target advance does not require a new dispatch or source re-review in ordinary mode. High-risk or unknown impact, a verified related consumer/contract change, a workflow/check policy change, a merge conflict, wrong execution ref/base, source-head or target-ref drift, failure, pending or uncertain readback requires the strict integration path and any affected review epoch.
Before Pre-PR Ready or promotion, perform the fail-closed join against the
current PR: the applicable source-bound PR CI or strict integration identity
and the role-complete source-review identity must bind the same frozen head
and applicable impact projection. Missing, pending, uncertain, stale,
drifted, wrong-head, wrong-app, wrong-ref, failed, or unreadable CI/review
evidence blocks. A legacy `--evidence-digest` or audit-only shadow result
never satisfies this join.

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
