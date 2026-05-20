---
name: writing-repo-owned-skills
description: Use when creating a new skill under .agents/skills, replacing an upstream skill with repository-owned guidance, or editing a local skill whose trigger wording, helper references, or governance links may drift.
---

# Writing Repo-Owned Skills

## When to Use

Use this skill when:

- adding a new local skill under `.agents/skills/`
- localizing part of an upstream skill into repo-owned guidance
- editing an existing local skill that references repo paths, commands, helpers, or workflow truth

Do not use this skill for:

- one-off task notes
- module-only conventions better written in `AGENTS.md` or module docs
- rules that should be enforced by script or lint instead of prose

## Core Rule

Local skills must strengthen oasis7 repo truth, not create a parallel workflow.

If the content would be better owned by `AGENTS.md`, `prd.md`, `project.md`, a handoff template, or a script check, put it there instead of creating a new skill.

## Authoring Workflow

1. Decide the surface:
   - repo-owned workflow / helper
   - scenario-specific specialist capability
   - bounded replacement of an upstream skill
2. Start from:
   - `.agents/skills/templates/SKILL.template.md`
   - `.agents/skills/checklists/skill-authoring-checklist.md`
3. Write frontmatter carefully:
   - `name` uses lowercase letters, numbers, hyphens
   - `description` starts with `Use when...`
   - `description` only describes triggering conditions
4. Keep the body focused on:
   - when the skill applies
   - the repo-specific workflow or pattern
   - oasis7-specific commands, paths, helpers, or review boundaries
   - guardrails
5. Add supporting files only for heavy reference or reusable tools.
6. If the skill changes recommended practice, also update the relevant governance or role docs.

## Bounded Borrowing From Upstream `writing-skills`

Borrow:

- frontmatter discipline
- trigger-focused descriptions
- concise skill structure
- explicit supporting-file boundaries
- verification before declaring the skill ready

Do not directly borrow:

- mandatory subagent-based failing-test-first loops as a hard gate
- generic deployment advice unrelated to oasis7
- any process that competes with `AGENTS.md + .pm + GitHub PR review`

## Oasis7-Specific Surfaces

- authoring entrypoint: `.agents/skills/README.md`
- template: `.agents/skills/templates/SKILL.template.md`
- checklist: `.agents/skills/checklists/skill-authoring-checklist.md`
- governance topics:
  - `doc/engineering/self-evolution/skill-surface-replacement-governance-2026-05-19.prd.md`
  - `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.prd.md`

## Verification

Before claiming the skill is ready:

- confirm referenced commands / paths exist
- run `./scripts/doc-governance-check.sh`
- run `./scripts/pm/lint.sh`
- run `git diff --check`

If the skill introduces or documents a helper-driven workflow, also run at least one representative command or check tied to that workflow.

## Guardrails

- Do not copy upstream install / publishing text unless oasis7 actually uses it.
- Do not summarize the entire workflow in `description`; keep it as trigger wording only.
- Do not create a skill just because the topic is important; create one only if it is reusable and repo-owned.
- Do not leave bounded borrowing implicit; say what remains deferred or rejected.
