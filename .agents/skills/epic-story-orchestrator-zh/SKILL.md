---
name: epic-story-orchestrator-zh
description: Use when planning, extending, or drafting a Chinese long-form, world-heavy story or lore bible that needs reusable world rules, character registries, timeline control, chapter cards, consistency checks, and repo-tracked writeback.
allowed-tools:
  - Read
  - Write
  - Edit
license: MIT
metadata:
  language: zh-CN
  domain:
    - longform-fiction
    - worldbuilding
    - lore
  task_modes:
    - scaffold
    - extend
    - brainstorm
    - chapter-card
    - draft-scene
    - polish
    - audit
  inputs:
    required:
      - story_slug
      - task_mode
      - premise
      - canon_scope
    optional:
      - target_length
      - pov_mode
      - style_constraints
      - taboo_list
      - existing_refs
      - chapter_targets
  outputs:
    - world-bible.md
    - character-registry.md
    - timeline.md
    - plot-branches.md
    - chapter-cards/*.md
    - drafts/*.md
    - consistency-report.md
    - canon-log.md
  writeback:
    root: doc/game/lore/<story-slug>/
    append_only:
      - canon-log.md
      - .pm/tasks/<TASK-UID>.execution.md
  permissions:
    no_delete: true
    require_explicit_confirmation_for:
      - overwrite human-authored canon paragraphs
      - large-scale retcon
      - moving or deleting canon files
---

# Epic Story Orchestrator ZH

## Mission

Build and maintain a **single canonical narrative truth** for Chinese long-form stories in oasis7.

This skill orchestrates durable assets first, then prose slices. It never treats chat-only output as final truth.

## When to Use

Use this skill when:

- the user wants to create or extend a Chinese long-form, world-heavy story, lore bible, or macro background narrative
- the work needs reusable world rules, character registries, timeline control, chapter cards, or canon tracking
- the task should produce repo-tracked story truth instead of chat-only prose
- the story must stay consistent across many chapters, volumes, or parallel arcs

Do not use this skill when:

- the user only wants a short blurb, slogan, marketing copy, or one-paragraph synopsis
- the request is really a product/design spec better owned by `prd.md`, `project.md`, or `game-architect`
- the task is only polishing an existing Chinese paragraph and does not need story structure or writeback
- the output should remain temporary and must not become repo truth

## Output Contract (must always be explicit)

For each run, return all three blocks:

1. **Artifacts changed**
   - exact repo paths
   - operation (`create` / `update` / `append`)
2. **Traceability map**
   - chapter cards → `WR-*`, `CHAR-*`, `TL-*`, `PB-*`
3. **Risk & next action**
   - blocker (`none` if absent)
   - next recommended step

## Task Modes

- `scaffold`: create a new story space from premise
- `extend`: add or revise canon while preserving prior truth
- `brainstorm`: produce 2–3 plot directions with one recommendation
- `chapter-card`: create chapter/scene cards from existing canon
- `draft-scene`: draft a scene or chapter slice from approved cards
- `polish`: rewrite prose while preserving canon and tone
- `audit`: inspect canon drift, retcon impact, and missing references

## Canon Root

Default canonical root:

`doc/game/lore/<story-slug>/`

Recommended artifact layout:

- `world-bible.md`
- `character-registry.md`
- `timeline.md`
- `plot-branches.md`
- `chapter-cards/`
- `drafts/`
- `consistency-report.md`
- `canon-log.md`

Use stable IDs everywhere:

- world rules: `WR-*`
- factions: `FAC-*`
- locations: `LOC-*`
- technologies: `TEC-*`
- characters: `CHAR-*`
- timeline events: `TL-*`
- plot beats: `PB-*`
- chapter cards: `CH-*`

## One-Pass Perfect Workflow

### 0) Intake Gate

Collect the minimal context: `story_slug`, `task_mode`, `premise`, `canon_scope`.
If any required field is missing, stop and request it.

### 1) Route Gate

- If premise is ambiguous: run `brainstorm` first (2–3 options + 1 recommendation).
- If premise is clear: continue.

### 2) Canon Gate (strict order)

1. `world-bible.md`
2. `character-registry.md`
3. `timeline.md`

Do not produce chapter cards or prose that rely on undefined canon.

### 3) Plot Gate

Update `plot-branches.md` for major arc choices and append decision rationale to `canon-log.md`.

### 4) Card Gate

Create chapter cards before prose. Every card must cite at least one `TL-*` and one `CHAR-*`.

### 5) Draft Gate

Draft prose in slices:

- scene slice: 800–2000 Chinese characters
- chapter slice: 2000–6000 Chinese characters

### 6) Polish Gate

Optionally apply `humanizer-zh` for Chinese tone quality while preserving canon facts.

### 7) Audit Gate

Update `consistency-report.md` and `canon-log.md`.
Verify timeline consistency, character-state consistency, and term drift.

### 8) Formal Writeback Gate

If bound to `.pm` task, append execution note to `.pm/tasks/<TASK-UID>.execution.md` with:

- Action
- Validation Command
- Expected Result
- Actual Result
- Blocker
- Next Action

## Integration with Existing Local Surfaces

- governance entrypoint: `.agents/skills/README.md`
- skill authoring: `.agents/skills/writing-repo-owned-skills/SKILL.md`
- world/system decomposition: `.agents/skills/game-architect/SKILL.md`
- option comparison: `.agents/skills/bounded-brainstorming/SKILL.md`
- prose polish: `.agents/skills/humanizer-zh/SKILL.md`

This skill does **not** replace bootstrap/router/verification/closeout workflow surfaces.

## Supporting Files

- `templates/world-bible.template.md`
- `templates/character-registry.template.md`
- `templates/timeline.template.md`
- `templates/plot-branches.template.md`
- `templates/chapter-card.template.md`
- `templates/canon-log.template.md`
- `templates/consistency-report.template.md`

## Guardrails

- Do not create a second planning system outside repo truth.
- Do not overwrite existing canon silently.
- Do not delete canon files.
- Do not imitate a living author’s distinctive signature voice as a target.
- Do not output giant prose dumps before card-level planning.
- Keep default output language in Chinese unless explicitly requested otherwise.

## Verification

Run checks before claiming ready:

- `git diff --check`
- `./scripts/doc-governance-check.sh`
- `python3 .agents/skills/epic-story-orchestrator-zh/tests/validate_writeback.py`
- `bash .agents/skills/epic-story-orchestrator-zh/tests/run_smoke.sh`

For story assets, run reference scan:

- `rg -n 'WR-|FAC-|LOC-|TEC-|CHAR-|TL-|PB-|CH-' doc/game/lore/<story-slug>/`
