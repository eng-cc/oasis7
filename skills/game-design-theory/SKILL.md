---
name: game-design-theory
version: "2.0.0"
description: Use when an oasis7 gameplay, product, agent, visual, or liveops slice needs theory-backed framing for player motivation, core loops, progression, balance, or experience design.
sasmp_version: "1.3.0"
bonded_agent: 01-game-designer
bond_type: PRIMARY_BOND
---

# Game Design Theory

## Oasis7 Workflow Binding

In oasis7, this is a domain-triggered, non-default specialist aid. It does not
own gameplay, product, liveops, visual, or agent conclusions by itself.

Follow `doc/engineering/workflow/source-of-truth.md`. The responsible person or
main Agent may use this aid directly; involve the matching professional role
when needed, such as `gameplay_designer`, `producer_system_designer`,
`game_visual_interaction_designer`, `agent_engineer`, or `liveops_community`.
This skill does not require a pre-existing task or worktree for analysis.

## When to Use

Use this skill when:

- a role slice needs MDA, motivation, progression, balance, retention, or player
  psychology framing for an oasis7 decision
- a design discussion needs reusable theory language tied to current
  `doc/game/*`, current product/professional authority, actual PR/Task evidence, or playtest observations
- a `producer_system_designer` or `gameplay_designer` slice is turning the
  current `doc/game` tree into a design-bible-grade topic map and needs to
  separate player promise, player verbs, loops, progression pressure, feedback,
  balance risk, and validation evidence
- community or agent behavior signals need to be translated into experience
  hypotheses without making product or liveops promises

Do not use this skill when:

- the request is direct implementation with already-sufficient gameplay truth
- a generic game-design essay would not change the current task decision
- the conclusion should be owned by `gameplay_designer`,
  `producer_system_designer`, `agent_engineer`, or `liveops_community` without a
  theory aid

## Core Workflow

1. Start from the relevant current PRD/design, actual PR or existing Task when
   applicable, and any playtest or community evidence.
2. Pick the smallest useful theory lens, such as core loop, motivation,
   progression pressure, balance tradeoff, or player feedback timing.
3. For detailed game design documentation, express the lens as concrete doc
   slots: player promise, verbs, state/economy rules, failure/recovery,
   feedback surfaces, balance risks, scenario examples, and validation matrix.
4. Tie every recommendation to observable oasis7 behavior or a documented
   follow-up validation path.
5. Attribute professional conclusions to the owning role slice, not to this
   skill.

## Supporting Files

- `references/DESIGN_GUIDE.md`: compact upstream-tracking reference for generic
  theory terms. Read it only when the current slice needs a theory refresher.

## Known Failure Modes

- Treating generic theory as product evidence.
- Letting this skill replace the `gameplay_designer` or
  `producer_system_designer` role boundary.
- Expanding a root PRD into a giant manual instead of routing details to
  topic-level PRD/design and indexed evidence; mutable task records belong in the actual PR or existing Task.
- Making liveops or player-facing claims from theory without
  `liveops_community` review.

## Guardrails

- Keep the entrypoint tied to oasis7 repo truth; generic theory belongs in the
  supporting reference only.
- Keep gameplay, product, agent, visual and liveops conclusions within the
  relevant authority; use professional review where the task risk and current
  workflow require it, rather than making a fixed role slice a prerequisite.
- Do not treat player psychology frameworks as validation evidence unless they
  are paired with task-specific acceptance, playtest, or community evidence.

## Verification

- Record the theory lens, evidence and residual risk in the requested analysis
  deliverable or actual PR/existing Task; read-only analysis needs no remote comment.
- Run `./scripts/lint-skills.sh` after skill-surface edits.
