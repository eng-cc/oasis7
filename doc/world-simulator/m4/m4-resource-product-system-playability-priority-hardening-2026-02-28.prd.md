# M4 资源产业链可玩性优先强化（2026-02-28）

- 对应设计文档: `doc/world-simulator/m4/m4-resource-product-system-playability-priority-hardening-2026-02-28.design.md`
- 对应项目管理文档: `doc/world-simulator/m4/m4-resource-product-system-playability-priority-hardening-2026-02-28.project.md`

审计轮次: 5

## 1. Executive Summary
- 以“可玩性优先”为原则，优先消除当前资源产业链的断链点和低决策密度问题。
- 在保持现有 runtime 兼容前提下，提升玩家在物流调度、阶段推进、长期经营上的策略空间。
- 将本轮改动约束为可直接进入 `test_tier_required/test_tier_full` 的可验证增量。

## 2. User Experience & Functionality

### In Scope
- 补齐 `polymer_resin` 内置配方链路，移除关键链路对预置库存的强依赖。
- 落地 `ProductProfile.unlock_stage` 阶段门槛校验（当前阶段不足时拒绝排产）。
- 为 `TransferMaterial` 提供显式优先级输入（保留旧推断逻辑兜底）。
- 落地 `ProductProfile.maintenance_sink` 的持续消耗逻辑并对高阶产物给出首版参数。
- 为 `ScheduleRecipe` 增加玩家侧 `schedule_quote` 合同，使维护消耗和稀缺延迟在确认前可比较。
- 当 `schedule_quote` 涉及高负载折旧或非零维护消耗时，补充本次排程前后的维护 runway 与停机/critical 临界点。

### Out of Scope
- 复杂订单簿/撮合市场。
- 跨服经济结算与链上治理协议扩展。
- Viewer 大规模交互重构。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications

### 1) 配方链路补齐
- 新增内置 recipe 模块：`m4.recipe.smelter.polymer_resin`。
- 新增 `recipe.smelter.polymer_resin` 到 bootstrap 模块清单与 recipe profile。

### 2) 产品阶段门槛
- 在 `ScheduleRecipe` 时对 `plan.produce` 对应 `ProductProfile.unlock_stage` 做校验。
- 当产品门槛高于当前 `industry_progress.stage` 时拒绝动作并输出拒绝原因。

### 3) 物流显式优先级
- 扩展 `Action::TransferMaterial`：新增可选 `priority`。
- 解析顺序：显式 `priority` > `MaterialProfile.default_priority` > 关键词推断。
- 保持默认兼容：不传 `priority` 时行为与旧版本一致。

### 4) 维护消耗接线
- 在 `ScheduleRecipe` 组装实际消耗时叠加 `maintenance_sink`：
  - 来源：`ProductProfile.maintenance_sink`
  - 作用对象：`plan.produce`
  - 合并策略：按材料种类聚合后与 `plan.consume` 一并校验/扣减。
- 首版参数：仅对中后期产物启用非零 sink，避免新手期开局硬阻断。

### 5) 排程报价合同
- `ScheduleRecipe` 在确认前应向玩家暴露最小 `schedule_quote`：
  - `base_duration_ticks`: 未受稀缺影响的基础时长。
  - `local_shortage_delay_ticks`: world fallback + bottleneck 导致的预计额外 tick。
  - `maintenance_sink_preview`: 本次排程预计叠加的维护消耗。
  - `depreciation_pressure_class`: 当前负载下的折旧压力档位，例如 `low / watch / high`。
  - `runway_before_ticks`: 当前负载/耐久下，提交前预计维护 runway。
  - `runway_after_ticks`: 提交本次排程后预计维护 runway。
  - `downtime_threshold_ppm`: 进入停机、critical 或强维护处理的耐久阈值。
  - `continue_production_risk`: 继续排产相对先维护/降载的最小风险说明。
  - `recommended_pre_step`: 降低延迟或维护压力的建议动作。
- Acceptance: 若本次排程会扣除非零 `maintenance_sink`、增加 `local_shortage_delay_ticks` 或提高高负载折旧压力，玩家确认前必须能看到代价原因、维护 runway 变化、停机/critical 临界点和至少一个避免/缓解方式。
- Edge case: 若 action 会成功提交但 quote 缺少维护或延迟预览，标记为 `schedule_quote_missing`，不得用执行后日志补充来替代确认前决策信息。
- Edge case: 若 quote 展示维护压力但缺少 `runway_before_ticks` / `runway_after_ticks` 或 `downtime_threshold_ppm`，标记为 `maintenance_runway_missing`。

## 5. Risks & Roadmap
- M0：设计/项目管理文档建档。
- M1：补齐 `polymer_resin` 配方链路 + bootstrap/artifact 同步。
- M2：`unlock_stage` + `TransferMaterial.priority` 接线。
- M3：`maintenance_sink` 接线与默认参数调优。
- M4：required/full 回归、文档收口与 devlog。

### Technical Risks
- 平衡风险：维护消耗过高可能导致中期停摆。
- 兼容风险：`TransferMaterial` 动作结构扩展可能影响旧序列化输入。
- 回归风险：新增消耗后会改变既有测试中的库存快照断言。
- 可玩性风险：维护 sink 与稀缺延迟若只在动作后结算，会把策略成本变成玩家不可预判的惩罚。
- 可读性风险：只展示 `depreciation_pressure_class` 会让玩家知道压力升高，却不知道还能安全生产多久或何时必须维护。

缓解：
- 采用“可选字段 + 默认值 + 兜底逻辑”。
- 对高阶产物先保守配置 sink，逐步加压。
- quote 只补玩家侧说明与验收，不在本专题重平衡维护 sink 或延迟阈值。
- runway 字段只作为排程前经营取舍信息，不新增完整维修 UI、不改变 `ScheduleRecipe` ABI，也不扩展工厂 runtime 状态结构。
- 先补 targeted 单测，再跑 required/full 回归。

## 6. Validation & Decision Record
- 追溯: 对应同名 `.project.md`，保持原文约束语义不变。
- DEC-M4-PLAY-001: 生产维护与稀缺压力必须在排程前变成可比较 quote；执行后 receipt 只能复核，不替代玩家提交前判断。
- DEC-M4-PLAY-002: `depreciation_pressure_class` 不能单独承担维护决策；涉及高负载折旧时，quote 必须给出维护 runway 前后变化与停机临界点。
