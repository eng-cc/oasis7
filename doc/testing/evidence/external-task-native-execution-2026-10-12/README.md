# Native private task execution evidence

- Owner role: qa_engineer; observed 2026-10-12.
- Scope: targeted, prepared_world, attached_codex_subagent, native API; not AC-EXT-010 full.
- Independent isolation: instruction_only/source_visible. Players received the formal Skill, executable and own device config; no source, oracle, owner credentials or service storage.
- Actual model tier: test_tier_full, evidence_level L4B. Ordinary Rust/TCP/retention regressions are test_tier_required and do not substitute for model decisions.
- Artifact byte lengths and SHA-256 digests: [manifest](manifest.json). Copies retain the original failure and successful decision evidence; the first player report's absolute workspace prefix is normalized, with its original digest retained. No private body, salt or signing seed is published.

## Candidate and source scope

The successful player candidate is `b59d010556e39beaf1928eb1e95aa9aad099e180`, tree `f25c8b4ed963b516776b30ce6489a2779f11a609`, based on `19e4309b73a8ae2a5f1a8225cc6daa47dfe8d82d`. No dirty patch or binary change occurred during the attempt.

| Delivered artifact | SHA-256 |
| --- | --- |
| `oasis7_game_client` | `bf70e721eac27640d1b933173d335d3b7c8471f781a3ce481964de0e5a6cb911` |
| Native service test executable | `ae414cebd8c777e209b3c131fcedd7ab3f3d4c8f231daf16b5f727e0afac18a1` |
| Formal `skills/oasis7-play/SKILL.md` | `ebde4c3e8e7d610c5d6b6febcce8568204dad4d192e909de811c84816bfa0248` |

SourceSceneRecord: [external Runtime contract](../../../product/agents-world-simulation/external-agent-runtime-play.prd.md), AC-EXT-005 original result/duplicate behavior and the goal_applied subset of AC-EXT-010; candidate blob `6941e246ce1e6e1ba5f0041f6c008372cefbd45e`. [Private collaboration design](../../../world-runtime/runtime/player-runtime-collaboration.design.md#6-接口与数据合同), blob `9cdf30303aea303d2b05725c09d17edcff72ba1e`; [Game API implementation scope](../../../world-simulator/llm/external-agent-game-api.design.md#当前实施候选原生任务执行链), blob `72cce9c6038cd0986e14d5d53509c4248005d65d`. All blobs refer to the player candidate, not a moving main.

The fixture used world `w1`, genesis `fixture-genesis-v1`, branch `main`, one Agent, canonical goal 1/task/executor binding, an independently configured trusted service key and delegate device. Owner setup performed canonical creation/publication/binding; it did not choose or submit the player's movement actions. Actual NodeRuntime worker ticks and ordinary signed HTTP handlers produced execution receipts.

## Attempts and independent oracle

| Attempt | Actual result | Diagnosis or oracle |
| --- | --- | --- |
| 1, source `fe567997272f6f4bea8d7caf4dbdcb0001d2ce19` | Blocked before any complete private Context or action; 9 CLI calls | Real activation-height snapshot/journal had been compacted and collected beyond the DevLocal 32-block window. Current task/executor authority still validated. [Player](attempt-1-player.md), [witness](attempt-1-witness.json). |
| 2, source `b59d010556e39beaf1928eb1e95aa9aad099e180` | Two effective goal-driven decisions; 13 CLI calls | Goal 1 action:3 committed at height 45, position `(200,0,0)` cm; goal 2 action:4 committed at height 53, position `(-200,0,0)` cm. Fresh observations and actual canonical receipts match both chosen actions. [Player](attempt-2-player.md), [witness](attempt-2-witness.json). |

ST-02 native prepared-world entry and ST-04 targeted goal application/original replay are executed and passed for attempt 2. The world witness joins the player's action correlations to the real signed cognition payload, canonical receipt, distinct goal revisions and unchanged executor epoch 1; [parent oracle](oracle-summary.json) verifies both receipt IDs, the two cognition actions and the final position. Original action lookup and exact duplicate returned the same first correlation, receipt and committed height; they did not create another movement action. The final observation Context had a committed fixed `executor_aborted` outcome at height 58, distinct from both successful movements.

Repair before attempt 2 added derived live-goal/context CAS pins and regression coverage for retention, restart rebuild, replacement release and corrupt cache. A separate no-model CLI preflight retained a real stale-base prefix rejection (`scheduler base binding changed`, captured tick 9), then successfully captured a fresh Context and committed its fixed abort outcome: [original response](preflight-prefix-rejected.json), [world witness](preflight-witness.json). Canonical freshness checks were not relaxed; the Skill distinguishes settled rejection from an unknown original result.

## Cleanup and remaining validation

Attempt 1's actual owner Stop and delegation revoke both committed: [cleanup](attempt-1-cleanup.json). The first repaired no-model preflight later encountered an unknown cleanup lookup and leaked its scoped listener after panic; the owned process was terminated and a listener/node drop guard added. Ordinary immediate real-worker/TCP Stop/revoke and scoped panic regressions then passed.

Attempt 2's two goal actions passed, but that attempt's long-lived fixture cleanup failed: the original Stop result was not obtained before the bounded deadline, no committed authority cleanup is claimed. The listener and owned NodeRuntime exited through the drop guards; PID 91007 is gone. [Failure](attempt-2-cleanup-failed.json) preserves the original correlation and empty settled-authority list. Its first submit category was not captured and remains unknown.

Post-attempt ordinary diagnosis used the actual worker/TCP fixture at 100 ms ticks through real height 72. Stop and revoke both committed, so age alone did not reproduce the issue. A controlled TCP disconnect before original Stop admission then reproduced the cleanup algorithm defect: one `OutcomeUnknown` submission, 22 Unknown lookups, zero canonical Stop, deadline exceeded. Fix `5b61c36733c0c4791fdedc594ab872f9f6ba1a7c` treats uncertain submission as unconfirmed admission and resends only the original signed payload at most once per second. The same fault then produced one uncertain submit, one confirmed response, eight Unknown lookups, one committed Stop and one committed revoke. The regression verifies exactly one canonical Stop and executor epoch 2. Immediate, aged, dropped-submit and scoped panic cases all passed (4 passed, 1 explicit live ignored): [ordinary fault summary](cleanup-regressions.json). This is ordinary fault-repair evidence, not a third model attempt or a rewrite of attempt 2 cleanup.

After public evidence was retained, all three exited owned prepared-world fixtures, their private credentials/goal records and temporary full-goal CLI output were deleted. [Disposable cleanup](disposable-world-cleanup.json) records process/world closure; it does not manufacture canonical Stop/revoke receipts for the failed attempts. Shared Cargo storage was preserved.

Retained gaps: full Game HTTP writes/catalog, token pairing, full collaboration TaskSnapshot, social/approval/report flows, legacy Hosted goal consumer, cold entry, first production/production-block recovery, full disconnect/restart lifecycle, OpenClaw, GUI and production deployment. Full AC-EXT-010 remains unverified. No new paid external Runtime was started; infrastructure/model fees are unknown, not zero. Both model attempts exhausted this bounded scenario allowance; further cleanup diagnosis uses ordinary tests.


Delivery, ordinary test failures/repairs, integration and CI history are archived in [the original delivery PR](https://github.com/eng-cc/oasis7/pull/4628#issuecomment-6162407180). They do not change this run's actual source/build/Skill, verdict, budget, oracle or cleanup facts. No current delivery obligation is maintained in this historical run report.
