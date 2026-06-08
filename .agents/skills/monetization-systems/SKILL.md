---
name: monetization-systems
version: "2.0.0"
description: Use when designing or implementing game monetization systems such as IAP, battle passes, economy sinks/sources, pricing, retention offers, compliance, or monetization metrics.
sasmp_version: "1.3.0"
bonded_agent: 07-game-publishing
bond_type: PRIMARY_BOND

parameters:
  - name: model
    type: string
    required: false
    validation:
      enum: [premium, f2p, freemium, subscription, ad_supported]
  - name: platform
    type: string
    required: false
    validation:
      enum: [mobile, pc, console, web]

retry_policy:
  enabled: true
  max_attempts: 3
  backoff: exponential

observability:
  log_events: [start, complete, error, purchase, refund]
  metrics: [arpu, arppu, conversion_rate, ltv, retention]
---

# Monetization Systems

## When to Use

Use this skill when:

- a task touches IAP, battle pass, pricing, economy balance, retention offers, ads, or monetization metrics
- a feature needs ethical monetization and compliance review
- the team needs implementation guidance for purchase flows or economy instrumentation

Do not use this skill when:

- the task is pure gameplay economy with no monetization or business metric impact
- the request needs legal advice beyond repo/product guidance

## Core Workflow

1. Clarify player value, fairness boundary, and compliance constraints before proposing mechanics.
2. Read `references/full-guidance.md` for models, IAP implementation, battle pass, metrics, troubleshooting, and compliance detail.
3. Route external/player-facing promises through liveops_community when messaging is involved.

## Supporting Files

- `references/full-guidance.md`: detailed original guidance, examples, patterns, and command/reference material.

## Oasis7-Specific Surfaces

- economy design docs
- purchase flow implementation
- metrics dashboards or event schemas
- `references/full-guidance.md`

## Known Failure Modes

- Revenue mechanics that undermine trust are product risk even if technically correct.
- Purchase success must be verified against authoritative entitlement state, not just UI success.
- Regional compliance and refund/restore flows are part of the feature, not post-launch polish.

## Guardrails

- Keep this entrypoint concise; move heavy examples or catalog material to supporting files.
- Do not bypass oasis7 task/worktree truth or professional role ownership when the workflow requires it.
- Do not present reference material as verified project behavior without checking the current repo state.

## Verification

- Run purchase/economy entitlement tests or document the unavailable verification surface.
- Run `./scripts/lint-skills.sh` after skill edits.
