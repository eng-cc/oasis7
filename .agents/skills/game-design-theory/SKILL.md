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

Use it only inside an already-bound task/worktree and route professional
conclusions through the matching role slice, such as `gameplay_designer`,
`producer_system_designer`, `game_visual_interaction_designer`,
`agent_engineer`, or `liveops_community`.

## When to Use

Use this skill when:

- a role slice needs MDA, motivation, progression, balance, retention, or player
  psychology framing for an oasis7 decision
- a design discussion needs reusable theory language tied to current
  `doc/game/*`, PRD/project truth, task evidence, or playtest observations
- community or agent behavior signals need to be translated into experience
  hypotheses without making product or liveops promises

Do not use this skill when:

- the request is direct implementation with already-sufficient gameplay truth
- a generic game-design essay would not change the current task decision
- the conclusion should be owned by `gameplay_designer`,
  `producer_system_designer`, `agent_engineer`, or `liveops_community` without a
  theory aid

## Core Workflow

1. Start from current repo truth: `.pm` task, relevant PRD/project docs, role
   card, and any playtest or community evidence.
2. Pick the smallest useful theory lens, such as core loop, motivation,
   progression pressure, balance tradeoff, or player feedback timing.
3. Tie every recommendation to observable oasis7 behavior or a documented
   follow-up validation path.
4. Attribute professional conclusions to the owning role slice, not to this
   skill.

## Supporting Files

- `references/DESIGN_GUIDE.md`: compact upstream-tracking reference for generic
  theory terms. Read it only when the current slice needs a theory refresher.

## Known Failure Modes

- Treating generic theory as product evidence.
- Letting this skill replace the `gameplay_designer` or
  `producer_system_designer` role boundary.
- Making liveops or player-facing claims from theory without
  `liveops_community` review.

## Guardrails

- Keep the entrypoint tied to oasis7 repo truth; generic theory belongs in the
  supporting reference only.
- Do not produce final gameplay, product, agent, visual, or liveops conclusions
  without the owning professional role slice.
- Do not treat player psychology frameworks as validation evidence unless they
  are paired with task-specific acceptance, playtest, or community evidence.

## Verification

- Record the owning role, theory lens, evidence, and residual risk in the task
  execution log.
- Run `./scripts/lint-skills.sh` after skill-surface edits.
