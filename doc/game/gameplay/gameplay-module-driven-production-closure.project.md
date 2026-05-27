# Gameplay Module-Driven Production Closure（项目管理文档）

- 对应设计文档: `doc/game/gameplay/gameplay-module-driven-production-closure.design.md`
- 对应需求文档: `doc/game/gameplay/gameplay-module-driven-production-closure.prd.md`

审计轮次: 4

## 审计备注
- 主项目入口：`doc/game/gameplay/gameplay-top-level-design.project.md`
- 本文仅维护增量任务，不重复主项目文档任务编排。

## 任务拆解

### T0 设计建档
- [x] 新建设计文档：`doc/game/gameplay/gameplay-module-driven-production-closure.prd.md`
- [x] 新建项目管理文档：`doc/game/gameplay/gameplay-module-driven-production-closure.project.md`

### T1 m5 WASM 模块生产实现（对应问题 1）
- [x] `m5_gameplay_war_core`：战争生命周期状态跟踪 + 自动结算 directive
- [x] `m5_gameplay_governance_council`：提案/投票状态跟踪 + 自动结算 directive
- [x] `m5_gameplay_crisis_cycle`：危机生成/超时 lifecycle directive
- [x] `m5_gameplay_economic_overlay`：经济叠加规则（基于 gameplay 事件触发奖励/惩罚）
- [x] `m5_gameplay_meta_progression`：元进度规则与 bonus grant directive
- [x] 同步 m5 内建 wasm artifacts/hash manifest

### T2 Runtime 模块驱动闭环（对应问题 2）
- [x] 在 `step_with_modules` 中切换 gameplay tick 到模块驱动路径
- [x] 新增 gameplay module emit 消费器（directive -> DomainEvent）
- [x] 保留无 gameplay tick module 时的 runtime fallback
- [x] 增加 required-tier 模块驱动生命周期测试

### T3 LLM/Simulator 动作协议补齐（对应问题 3）
- [x] 扩展 OpenAI decision schema 枚举与字段
- [x] 扩展 decision parser 到 gameplay actions
- [x] 更新 prompt action labels / execution controls 的新动作支持
- [x] 增加 required-tier 解析与动作流测试

### T4 文档与顶层任务收口（对应问题 5）
- [x] 回写 `doc/game/gameplay/gameplay-top-level-design.project.md` 未完成项
- [x] 固化 Gameplay 模块 `test_tier_required` 与 `test_tier_full` 测试矩阵引用
- [x] 回写 `doc/devlog/README.md`

## 依赖
- `doc/game/gameplay/gameplay-top-level-design.prd.md`
- `doc/game/gameplay/gameplay-top-level-design.project.md`
- `doc/game/gameplay/gameplay-engineering-architecture.md`
- `doc/game/gameplay/gameplay-layer-lifecycle-rules-closure.prd.md`
- `testing-manual.md`

## 状态
- 当前状态：`已完成`
- 已完成：T0、T1、T2、T3、T4
- 进行中：无
- 阻塞项：无

## 迁移记录（2026-03-03）
- 已按 `TASK-ENGINEERING-014-D1 (PRD-ENGINEERING-006)` 从 legacy 命名迁移为 `.prd.md/.project.md`。
- 保留原任务拆解、依赖与状态语义，不改变既有结论。
