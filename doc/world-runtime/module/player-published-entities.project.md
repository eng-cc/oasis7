# oasis7 Runtime：玩家发布制成品的 WASM 模块与 Profile 治理闭环（项目管理文档）

- 对应设计文档: `doc/world-runtime/module/player-published-entities.design.md`
- 对应需求文档: `doc/world-runtime/module/player-published-entities.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] TASK-WORLD_RUNTIME-010 (PRD-WORLD_RUNTIME-010/012) [test_tier_required]: 发布单扩展 `profile_changes`，Apply 时落账 product/recipe profile，并补齐拒绝路径与回放测试。
- [x] TASK-WORLD_RUNTIME-011 (PRD-WORLD_RUNTIME-010/012) [test_tier_required]: 新增 `FactoryProfileV1` + `GovernFactoryProfile` 动作/事件/状态落账与字段白名单校验。
- [x] TASK-WORLD_RUNTIME-012 (PRD-WORLD_RUNTIME-011) [test_tier_full]: 三节点发布链路 SLA 测试（submit->apply p95 <= 60s）与报表。
- [x] TASK-WORLD_RUNTIME-013 (PRD-WORLD_RUNTIME-012) [test_tier_required]: 角色审批/签名/冲突校验覆盖与回归。
- [x] TASK-WORLD_RUNTIME-014 (PRD-WORLD_RUNTIME-012) [test_tier_required]: 更新冲突治理口径：发布单 profile 变更禁止覆盖既有 ID，要求 `Shadow/Apply` 双重拒绝并给出明确原因。
- [x] TASK-WORLD_RUNTIME-015 (PRD-WORLD_RUNTIME-012) [test_tier_required]: 实现 profile 覆盖拒绝（包括 payload 与现状完全一致的场景）并补齐 release shadow/apply 冲突拒绝回归测试。

## 依赖
- doc/world-runtime/module/player-published-entities.prd.md
- doc/world-runtime/wasm/wasm-interface.md
- doc/world-runtime/module/module-lifecycle.md
- doc/world-runtime/module/module-storage.prd.md
- doc/world-runtime/runtime/runtime-integration.md
- testing-manual.md

## 状态
- 记录日期: 2026-03-06
- 历史结论: TASK-WORLD_RUNTIME-010 至 TASK-WORLD_RUNTIME-015 已完成；本文件保留当时的实现拆解与 PRD-ID 对应关系。
- 当前任务真值: 后续任务、状态与证据以 GitHub task issue evidence comments 为准；本历史记录不声明当前 active 状态或下一任务。
