---
name: writing-repo-owned-skills
description: Use when creating or moving a repo-owned skill surface under .agents/skills or skills, replacing upstream guidance, or editing trigger wording, helper references, or governance links.
---

# Writing Repo-Owned Skills

> PM truth: skill edits must not redefine the GitHub Project-backed PM contract; link back to `doc/engineering/workflow/source-of-truth.md#123-github-project-backed-pm-contract` and keep this surface operational.

## When to Use

Use this skill when:

- adding a new local skill under `.agents/skills/`
- adding or moving non-default specialist library material under root `skills/`
- localizing part of an upstream skill into repo-owned guidance
- editing an existing local skill that references repo paths, commands, helpers, or workflow truth

Do not use this skill for:

- one-off task notes
- module-only conventions better written in `AGENTS.md` or module docs
- rules that should be enforced by script or lint instead of prose

## Core Rule

Local skills must strengthen oasis7 repo truth, not create a parallel workflow.

If the content would be better owned by `AGENTS.md`, a PRD/design/evidence document, a handoff template, GitHub-backed task truth, or a script check, put it there instead of creating a new skill.

## Authoring Workflow

1. Decide the surface:
   - default-loadable repo-owned workflow / helper under `.agents/skills/`
   - non-default specialist library/archive material under root `skills/`
   - source-of-truth-promoted specialist wrapper under `.agents/skills/`
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
7. If moving a default-loadable skill into root `skills/`, update
   `doc/engineering/workflow/source-of-truth.md`, `.agents/skills/README.md`,
   role cards, and any hard-coded script paths before deleting the old
   `.agents/skills` entrypoint.

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
- any process that competes with `AGENTS.md + GitHub-backed task truth + GitHub PR review`

## Oasis7-Specific Surfaces

- authoring entrypoint: `.agents/skills/README.md`
- template: `.agents/skills/templates/SKILL.template.md`
- checklist: `.agents/skills/checklists/skill-authoring-checklist.md`
- default-loadable skill surface: `.agents/skills/*/SKILL.md`
- non-default specialist library surface: `skills/*/SKILL.md`
- governance topics:
  - `doc/engineering/workflow/source-of-truth.md`
  - `.agents/skills/README.md`

## Verification

Before claiming the skill is ready:

- confirm referenced commands / paths exist
- run `./scripts/lint-skills.sh`
- run `./scripts/doc-governance-check.sh`
- run `./scripts/pm/lint.sh`
- run `git diff --check`

If the skill introduces or documents a helper-driven workflow, also run at least one representative command or check tied to that workflow.

## Guardrails

- Do not copy upstream install / publishing text unless oasis7 actually uses it.
- Do not summarize the entire workflow in `description`; keep it as trigger wording only.
- Do not create a skill just because the topic is important; create one only if it is reusable and repo-owned.
- Do not leave bounded borrowing implicit; say what remains deferred or rejected.

## Known Failure Modes

- Letting `SKILL.md` become the whole manual; keep entrypoints concise and move detailed examples, catalogues, or command matrices to `references/`.
- Writing a broad capability description that does not start with `Use when`; this makes skill discovery less predictable.
- Adding supporting files without mentioning when to read them from the entrypoint.
- Adding prose rules for something a script can enforce; prefer a lint/check when the rule is mechanical.
