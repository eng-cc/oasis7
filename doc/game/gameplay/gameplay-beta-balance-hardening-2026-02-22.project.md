# Gameplay 内测数值加固（项目管理文档）

- 对应设计文档: `doc/game/gameplay/gameplay-beta-balance-hardening-2026-02-22.design.md`
- 对应需求文档: `doc/game/gameplay/gameplay-beta-balance-hardening-2026-02-22.prd.md`
审计轮次: 4

## 审计备注
- 主项目入口：`doc/game/gameplay/gameplay-top-level-design.project.md`
- 本文仅维护增量任务。

## 任务拆解

### T0 建档
- [x] 新建设计文档：`doc/game/gameplay/gameplay-beta-balance-hardening-2026-02-22.prd.md`
- [x] 新建项目管理文档：`doc/game/gameplay/gameplay-beta-balance-hardening-2026-02-22.project.md`

### T1 治理投票权重上限
- [x] 为 `CastGovernanceVote.weight` 增加上限校验
- [x] 补充/更新拒绝路径测试

### T2 策略更新治理授权
- [x] 为 `UpdateGameplayPolicy` 增加治理授权门槛（已通过提案 + 最小总票重）
- [x] 补充授权/未授权测试

### T3 合约声誉去通胀
- [x] 调整成功结算声誉奖励公式（金额相关 + 质押约束 + 奖励上限）
- [x] 更新既有数值断言测试

### T4 收口
- [x] 跑定向测试并记录结果
- [x] 回写项目状态与文档
- [x] 更新 `doc/devlog/README.md`

## 依赖
- `crates/oasis7/src/runtime/world/event_processing/*`
- `crates/oasis7/src/runtime/tests/gameplay_protocol.rs`
- `doc/game/gameplay/gameplay-war-politics-mvp-baseline.md`

## 状态
- 当前状态：`已完成`
- 已完成：T0、T1、T2、T3、T4
- 进行中：无
- 未开始：无
- 阻塞项：无

## 迁移记录（2026-03-03）
- 已按 `TASK-ENGINEERING-014-D1 (PRD-ENGINEERING-006)` 从 legacy 命名迁移为 `.prd.md/.project.md`。
- 保留原任务拆解、依赖与状态语义，不改变既有结论。
