---
name: systematic-debugging
description: Use when you hit a bug, failing test, broken script, unexpected diff, or behavior regression. Focuses on reproduction, narrowing the failure surface, validating hypotheses, and only then patching the root cause.
---

# Systematic Debugging

## When to Use

Use this skill before proposing or applying a fix for:

- failing tests
- broken scripts
- runtime regressions
- UI behavior mismatches
- unexpected command output

## Core Workflow

1. Reproduce the failure.
2. Narrow the scope:
   - which command
   - which file or module
   - which environment or precondition
3. Collect the failure signature:
   - exit code
   - stack trace
   - assertion diff
   - screenshot or browser state when relevant
4. Form one concrete hypothesis.
5. Run the smallest probe that can confirm or falsify it.
6. Patch the root cause, not the surface symptom.
7. Re-run the failing check first, then the wider regression set.

## Preferred Tactics

- Start with the exact failing command.
- Use targeted reads and searches before broad edits.
- Keep one active hypothesis at a time when possible.
- If multiple causes are plausible, rank them and test the cheapest first.

## Oasis7-Specific Surfaces

- Document / PM failures:
  - `./scripts/pm/lint.sh`
  - `./scripts/doc-governance-check.sh`
  - `git diff --check`
- Workflow failures:
  - `./scripts/pm/task-closeout.sh`
  - `./scripts/prepare-task-pr.sh`
  - `./scripts/pr-review-thread-closeout.sh`
- Web / viewer failures:
  - repo-owned browser checks
  - `agent-browser` when the failure is browser-visible

## Output Rules

- State the reproduced failure first.
- Name the hypothesis you are testing.
- Say what evidence changed your confidence.
- After the fix, separate:
  - reproduced cause
  - patch applied
  - verification rerun

## Guardrails

- Do not shotgun multiple speculative fixes into one patch.
- Do not rewrite broad surfaces before reproducing the issue.
- Do not stop at "probably fixed"; rerun the relevant check.
