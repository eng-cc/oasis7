---
name: repo-owned-workflow-router
description: Use after default workflow bootstrap has established task truth, or whenever you need to decide which repo-owned workflow skill applies next. Routes the task through bounded brainstorming, behavior-first TDD, execution, verification, and closeout without replacing oasis7 root truth.
---

> Workflow authority: `doc/engineering/workflow/source-of-truth.md` is the single normative workflow spec. Keep this skill as short operational guidance only; if behavior changes, update source-of-truth first, then sync this file.


# Repo-Owned Workflow Router

Use this skill to decide which repo-owned workflow surface should drive the next phase of work.

## When to Use

Use this skill when:

- `default-workflow-bootstrap` has already confirmed repository-changing work has task truth
- a bootstrapped repository-changing task is starting implementation or needs phase selection
- you are unsure which local workflow skill should apply next
- the task needs to move across multiple phases, such as ideation -> implementation -> verification -> closeout
- the user wants the whole workflow chained together rather than treated as isolated skills

Do not use this skill when:

- the request is chat-only or read-only and does not change repository state
- you are already clearly inside one terminal phase and the next step is obvious

## Required Rules

1. This skill is a router, not an external bootstrap.
2. It must not replace `AGENTS.md`, `.pm`, `prd.md`, `project.md`, task execution logs, or GitHub PR review.
3. It only chooses and orders repo-owned workflow skills.
4. If the task truth changes, route decisions must be written back into `.pm/tasks/<TASK-UID>.execution.md`; formal docs may supplement but not replace it.
5. Use the narrowest applicable workflow surface; do not force every phase if it is not needed.
6. If the route implies multi-role or subagent-driven execution for task-bound or repository-changing work, the route output must also include a minimal slice contract: role, slice type, model configuration, mandatory context packet, write scope, return contract, mandatory `.pm` execution-log sink, and integration owner/order.
7. For task-bound or repository-changing work, TPM TODO decomposition and subagent slice contracts must be recorded in `.pm/tasks/<TASK-UID>.execution.md` before delegated execution begins.
8. TPM routing is coordination only. If the task needs professional/domain analysis, implementation, verification judgment, review judgment, or external messaging, route to the matching professional role slice before presenting that conclusion as authoritative.
9. Read-only/chat-only requests do not enter this router unless they are already bound to task truth or need repository writeback. However, read-only professional/domain questions still require the matching bounded role slice; TPM may dispatch that slice directly and use the role-tagged user-facing answer as the sink when no task/writeback exists.

## Routing Order

Check the task in this order:

0. Already-bound read-only professional/domain judgment
   - Apply this router step only when the read-only professional/domain judgment is already task-bound or writeback-driven.
   - Dispatch the matching bounded role slice; this router is not the entrypoint for unbound read-only professional questions.
   - Unbound read-only professional questions do not enter this router. TPM dispatches the matching bounded role slice directly and uses the role-tagged user-facing answer as the sink.
   - Skip professional dispatch only for pure fact lookup or command-output restatement.
1. `bounded-brainstorming`
   - Use when direction is still fuzzy, scope is too large, or the problem is inherently option-heavy or visual.
2. `tdd-test-writer`
   - Use when the task changes automatable behavior and has a stable automated test surface.
3. `executing-project-tasks`
   - Use when `prd.md` / `project.md` / handoff / `.pm` truth is ready and implementation should proceed step by step.
4. `systematic-debugging`
   - Use when a bug, failing test, broken script, unexpected diff, or regression appears before proposing fixes.
5. `requesting-repo-owned-review`
   - Use when a high-risk or major convergence diff needs local supplemental review before commit or before GitHub reviewers.
6. `verification-before-completion`
   - Use when the work is close to a claim such as “done”, “tests pass”, or “ready for PR”.
7. `finishing-a-development-branch`
   - Use when implementation and required verification are complete and the task should close out, commit, and move into PR handling.
8. `receiving-code-review`
   - Use when GitHub PR review comments or requested changes arrive.
9. `writing-repo-owned-skills`
   - Use when local repo-owned skill surfaces are created or edited.

## Routing Questions

Ask and answer these in order:

1. Is the direction already clear enough to implement?
2. Does the task need scope decomposition or 2-3 option comparison first?
3. Will the task change product/runtime/interaction behavior with a stable test surface?
4. Is the task already backed by sufficient repo truth to execute?
5. Did a bug, failing test, broken helper, unexpected diff, or regression appear?
6. Is the diff large or risky enough to need supplemental repo-owned review?
7. Is the next risk “implementation correctness” or “claim correctness”?
8. Is the task actually at closeout rather than execution?
9. Did GitHub review feedback arrive?
10. Is this task editing local skill surfaces or skill governance?

## Expected Output

```markdown
WORKFLOW ROUTE DECIDED

## Task Phase
- Current phase:
- Why:

## Selected Workflow Skills
1. [skill name] - [why now]
2. [skill name] - [why next]
3. [skill name] - [if needed]

## Skipped Workflow Skills
- [skill name] - [reason skipped]

## Specialist Skills Considered
- [skill name] - [domain trigger or reason skipped]

## Required Writeback
- `prd.md`:
- `project.md`:
- `.pm/tasks/<TASK-UID>.execution.md`:
- handoff / project supplement:

## Subagent Slice Plan (If Needed)
- role:
- slice type:
- model configuration: `gpt-5.4-medium` by default; record reason for any override
- mandatory context packet:
  - identity and authority:
  - workflow governance:
  - task truth:
  - user intent:
  - scoped repo context:
  - collaboration boundary:
- write scope:
- return contract:
- formal sink / writeback surface: `.pm/tasks/<TASK-UID>.execution.md` (mandatory)
- integration owner:
- integration order:
- context exemption:

## Next Action
- exact next step:
```

## Guardrails

- Do not route into `bounded-brainstorming` if the task is already implementation-ready.
- Do not route into `tdd-test-writer` for pure docs/governance work or when no stable harness exists.
- Do not route into `executing-project-tasks` if the plan truth is still missing key scope or validation details.
- Do not skip `systematic-debugging` when an observed failure needs reproduction and narrowing before a fix.
- Do not skip `verification-before-completion` when you are about to make a completion claim.
- Do not use this router as a replacement for closeout; switch to `finishing-a-development-branch` when the task is done.
- Do not treat specialist domain skills as mandatory default workflow phases; route to them only when the task domain matches their trigger.
- Do not dispatch implementation, verification, review, or specialist subagents without `AGENTS.md`, the assigned role card, workflow source-of-truth, current `.pm` task truth, and scoped repo context in the mandatory context packet.
- Do not let TPM direct exploration become a professional conclusion; professional findings must be owned or verified by the matching role slice.
