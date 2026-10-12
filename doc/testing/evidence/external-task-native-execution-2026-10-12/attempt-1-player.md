# Independent prepared-world playtest report

- Profile: bounded prepared_world, attached_codex_subagent.
- Isolation: instruction_only/source_visible. No implementation source, diff, service logs, owner keys, world files, other task history or server witness inspected.
- Formal Skill: skills/oasis7-play/SKILL.md (name oasis7-play).
- Delivered executable: target/debug/oasis7_game_client. Help provides no build/version hash; describe reports contract_version=1, capabilities reports api_revision=1.
- Usage: 9 CLI calls total, including help. No paid Runtime calls, no GUI, no world actions. Count differs from earlier interim 8-call status because a third same-operation context retry occurred before Stop arrived.

## Evidence

1. --help succeeded; ordinary native task-lane CLI supports describe/task/capabilities/context/move/outcome/lookup.
2. describe succeeded: readable=true, writable=true, recovering=false; world w1; genesis fixture-genesis-v1; view position=51.
3. task succeeded: task_status=active, task_revision=3, active_goal_revision=1, latest_goal_revision=1, executor_epoch=1. Full private goal unavailable.
4. capabilities succeeded: native.move_agent is the admitted side-effect proposal; complete Game HTTP catalog is not delivered.
5. context --operation play-fe567997-r1 failed with exit 1: private preparation rejected or unavailable; retry original operation.
6. Same exact operation retry failed with identical error.
7. lookup --context play-fe567997-r1 --kind prefix failed with exit 1: original operation is not in this device cache.
8. Third exact operation retry failed with identical preparation error. These are nine client calls including the initial help, describe, task and capabilities.

## Decision rounds and results

- Round 1: visible goal revision 1 from task metadata; context operation/correlation play-fe567997-r1. Complete private Context could not be read. No action was chosen from an unseen goal. No action correlation, receipt or canonical settled action exists; no observed world effect.
- Round 2: not started; owner goal update was not observed.
- Terminal status: context blocked, not passed. Stopped additional CLI calls on scenario owner instruction, retaining the original operation.

## Retained gaps

Full-goal-driven decision, canonical committed action receipt, original settled-action lookup, duplicate delivery without new side effect, new goal revision observation and second decision remain untested because private preparation failed. Public readable/writable availability does not establish native-task preparation authorization. Cause remains unknown through the authorized player interface. No terminal provider outcome was submitted because there was no usable Context.

## Fees

No paid Runtime invocation was made. CLI/runtime accounting and any infrastructure costs are unknown; no fee evidence was delivered.
