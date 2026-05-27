# Gameplay Layer Lifecycle Rules Closure（项目管理文档）

- 对应设计文档: `doc/game/gameplay/gameplay-layer-lifecycle-rules-closure.design.md`
- 对应需求文档: `doc/game/gameplay/gameplay-layer-lifecycle-rules-closure.prd.md`

审计轮次: 4

## 审计备注
- 主项目入口：`doc/game/gameplay/gameplay-top-level-design.project.md`
- 本文仅维护增量任务，不重复主项目文档任务编排。

## 任务拆解

### T0 设计建档
- [x] 新建设计文档：`doc/game/gameplay/gameplay-layer-lifecycle-rules-closure.prd.md`
- [x] 新建项目管理文档：`doc/game/gameplay/gameplay-layer-lifecycle-rules-closure.project.md`

### T1 协议与状态扩展
- [x] 扩展 `events.rs`：治理提案开启/结算、危机生成/超时、战争结算事件
- [x] 扩展 `gameplay_state.rs` 与 `state.rs`：生命周期状态持久化与事件应用
- [x] 扩展 `event_processing.rs`：治理提案开启与投票约束

### T2 Tick 生命周期推进
- [x] 新增 `world/gameplay_loop.rs` 并接入 `step.rs`
- [x] 在 tick 周期推进治理结算、危机生成/超时、战争自动结算
- [x] 扩展 `module_runtime_labels.rs` 标签

### T3 测试与收口
- [x] 新增/扩展 `runtime/tests/gameplay_protocol.rs` 生命周期测试（`test_tier_required`）
- [x] 跑 `cargo check` 与 gameplay 目标测试
- [x] 回写项目文档状态与 `doc/devlog/README.md`

## 依赖
- `README.md`
- `doc/game/gameplay/gameplay-top-level-design.prd.md`
- `doc/game/gameplay/gameplay-engineering-architecture.md`
- `doc/game/gameplay/gameplay-layer-war-governance-crisis-meta-closure.prd.md`
- `testing-manual.md`

## 状态
- 当前状态：`已完成`
- 已完成：T0、T1、T2、T3
- 进行中：无
- 阻塞项：无

## 迁移记录（2026-03-03）
- 已按 `TASK-ENGINEERING-014-D1 (PRD-ENGINEERING-006)` 从 legacy 命名迁移为 `.prd.md/.project.md`。
- 保留原任务拆解、依赖与状态语义，不改变既有结论。
