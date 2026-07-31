---
name: level-design
version: "2.0.0"
description: Use when designing or evaluating game levels, pacing, difficulty progression, spatial layout, environmental storytelling, encounters, or traversal flow.
sasmp_version: "1.3.0"
bonded_agent: 01-game-designer
bond_type: PRIMARY_BOND

parameters:
  - name: level_type
    type: string
    required: false
    validation:
      enum: [linear, open_world, hub, procedural, puzzle]
  - name: game_genre
    type: string
    required: false
    validation:
      enum: [action, adventure, puzzle, platformer, fps, rpg]

retry_policy:
  enabled: true
  max_attempts: 3
  backoff: exponential

observability:
  log_events: [start, complete, error]
  metrics: [completion_time, death_count, exploration_percentage]
---

# Level Design

## When to Use

Use this skill when:

- level pacing, layout, traversal, difficulty, encounters, or environmental storytelling are in scope
- a task needs reusable level design principles, critique, or level-flow planning
- a game_visual_interaction_designer or gameplay slice needs level readability and pacing reference material

Do not use this skill when:

- the task is pure runtime implementation with no spatial or player-flow decision
- the request is only static UI styling or copywriting

## Core Workflow

1. Identify the level goal, player verb, target pacing, and verification surface.
2. Read `references/full-guidance.md` only when you need detailed patterns, whitebox process, guidance techniques, difficulty progression, metrics, or troubleshooting.
3. Tie recommendations to observable player flow, spatial readability, and challenge progression.
4. Route professional conclusions through the matching oasis7 role slice when project governance requires it.

## Supporting Files

- `references/full-guidance.md`: detailed original guidance, examples, level patterns, metrics, and troubleshooting.

## Oasis7-Specific Surfaces

- `.agents/roles/game_visual_interaction_designer.md`
- `.agents/roles/gameplay_designer.md`
- GitHub Project task status and issue evidence comments
- visual/playtest evidence for level-flow changes

## Known Failure Modes

- A level can be theoretically well structured but still unreadable in the actual camera, UI, or movement model.
- Difficulty curves without replayable scenarios are hard to validate and easy to regress.
- Environmental storytelling can obscure objective clarity if player guidance is not checked in-context.

## Guardrails

- Tie level recommendations to player goals, constraints, and observable flow.
- Do not present abstract level theory as verified playtest evidence.
- Keep this entrypoint concise; move heavy examples or catalog material to supporting files.

## Verification

- Run the focused playtest, screenshot review, or level-flow check tied to the task when available.
- Run `./scripts/lint-skills.sh` after skill-surface edits.
