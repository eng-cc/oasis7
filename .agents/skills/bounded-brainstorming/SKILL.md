---
name: bounded-brainstorming
description: Use when the task is still ambiguous, needs scope decomposition or option comparison, or is inherently visual enough to justify a visual companion before implementation. Converts upstream brainstorming into a repo-owned, non-mandatory ideation step.
---

# Bounded Brainstorming

Use this skill when the next move is not implementation yet, but decision-shaping.

## When to Use

Use this skill when:

- the user explicitly asks for brainstorming, idea exploration, architecture options, or plan-only output
- the task is still too large or fuzzy to implement safely
- the work is UI-heavy, structure-heavy, or comparison-heavy enough that 2-3 options are useful
- a visual companion may help decide IA, wireframe, layout, or state presentation before implementation

Do not use this skill when:

- the user already gave a concrete implementation task and the scope is clear enough to start
- the task already has chosen direction in `prd.md`, `project.md`, handoff, or `.pm`
- you are using brainstorming only as a way to delay implementation

## Required Rules

1. This is an optional pre-implementation layer, not a universal gate.
2. Do not force per-section approval, mandatory spec drafting, or transition into a second planning system.
3. Keep the output anchored to repo truth: chosen direction must flow back into `prd.md`, `project.md`, a handoff, or `.pm` evidence when it affects scope.
4. Prefer 2-3 concrete approaches with tradeoffs and one clear recommendation.
5. Only use a visual companion when the problem is inherently visual; do not turn browser mockups into default ceremony.
6. If scope is too large, split it into smaller, executable slices before implementation starts.

## Core Workflow

1. Restate the decision to make.
2. Decide whether the task is:
   - ready to implement now
   - too large and needs decomposition
   - ambiguous enough to need option comparison
   - visual enough to justify a visual companion
3. If decomposition is needed, split the work into smaller slices with clear boundaries.
4. Produce 2-3 approaches:
   - short description
   - main tradeoff
   - why it fits or does not fit oasis7 workflow truth
5. Recommend one approach.
6. If the problem is visual, decide whether to use `agent-browser` or another repo-owned visual step to compare layouts, IA, or states.
7. Write the chosen direction back into repo truth before implementation:
   - `prd.md` if scope/behavior/boundary changed
   - `project.md` if execution path or affected surfaces changed
   - handoff if another role/subagent will execute it
   - `.pm/tasks/<TASK-UID>.execution.md` if the decision matters to current task truth

## Expected Output

```markdown
BOUNDED BRAINSTORMING COMPLETE

## Decision
- Problem:
- Why this needs brainstorming now:

## Scope
- Ready now / needs split:
- Proposed slices:

## Options
1. [Option name] - [tradeoff summary]
2. [Option name] - [tradeoff summary]
3. [Option name] - [tradeoff summary, if needed]

## Recommendation
- Chosen direction:
- Why:

## Visual Companion
- Needed: yes/no
- If yes: artifact type and target question

## Repo Truth Writeback
- `prd.md`:
- `project.md`:
- handoff / `.pm`:
```

## Guardrails

- Do not leave the result as chat-only guidance if it changes task truth.
- Do not turn this into mandatory upfront ceremony for every request.
- Do not present option lists without a recommendation unless the user explicitly wants open-ended exploration.
