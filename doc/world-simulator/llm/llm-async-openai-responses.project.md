# Retired Direct Model Path Project Tombstone

- 对应设计文档: `doc/world-simulator/llm/llm-async-openai-responses.design.md`
- 对应需求文档: `doc/world-simulator/llm/llm-async-openai-responses.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）
本历史 project 已转为 tombstone，不再新增 active project task row。实际执行记录见当前 MVP playtest readiness hardening `.pm` execution log；本文件仅保留以下边界说明:

- Historical game-side direct model client scope is retired.
- Game-side direct client implementation and demo/probe entrypoints are removed.
- Game-side direct model SDK dependency is removed from `crates/oasis7`.
- This file remains only as a governance tombstone for historical path continuity.

## 依赖
- No active game-side dependencies.
- Provider bridge dependencies are owned outside this game-side direct-client tombstone.

## 状态
- 当前阶段：retired
- 下一步：无游戏端实施项
- 最近更新：MVP playtest readiness hardening 期间确认游戏端直连模型客户端彻底废弃

## 后续优化跟踪（基于 30 tick 实跑）
- 无。后续模型能力优化应进入 provider bridge 或 provider-backed gameplay task。

## 状态（补充）
- 当前阶段：retired
- 最近更新：直连客户端路径废弃确认
