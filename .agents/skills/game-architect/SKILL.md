---
name: game-architect
description: Use when planning a new game architecture, major game feature, refactor, or technical design document that needs structured requirement, technical design, and implementation-plan outputs.
---

# Game Architect

## When to Use

Use this skill when:

- a game architecture or major feature needs requirement, technical design, and implementation-planning documents
- the task needs explicit selection among architecture paradigms or system-specific references
- the output is documentation that will later feed implementation through the oasis7 workflow chain

Do not use this skill when:

- the user is asking for direct implementation with already-sufficient repo truth
- the question is a narrow gameplay, visual, runtime, viewer, or QA judgment owned by another role slice

## Core Workflow

1. Bind the work to the existing `.pm` task/worktree and record the route before producing architecture docs.
2. Use this `SKILL.md` as the entrypoint; read `references/full-guidance.md` for document templates, phase details, paradigm tables, and examples.
3. Write architecture outputs as supplements to `prd.md`, `project.md`, and `.pm` truth, never as a replacement for them.
4. Hand implementation back through `repo-owned-workflow-router` and `executing-project-tasks` before code changes.

## Supporting Files

- `references/full-guidance.md`: detailed original guidance, examples, patterns, and command/reference material.

## Oasis7-Specific Surfaces

- `architect/requirement.md`
- `architect/technical_design.md`
- `architect/implementation.md`
- `doc/engineering/workflow/source-of-truth.md`

## Known Failure Modes

- Architecture docs can drift into a second workflow; keep `.pm` execution truth canonical.
- Do not let this skill own runtime, gameplay, viewer, QA, or visual conclusions without the matching professional slice.
- Reference files are conditional; load only the system references that match the requested architecture problem.

## Guardrails

- Keep this entrypoint concise; move heavy examples or catalog material to supporting files.
- Do not bypass oasis7 task/worktree truth or professional role ownership when the workflow requires it.
- Do not present reference material as verified project behavior without checking the current repo state.

## Verification

- Confirm generated doc paths and referenced files exist.
- Run `./scripts/lint-skills.sh` after skill-surface edits.
