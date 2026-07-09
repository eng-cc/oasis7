# M4 资源与产品系统 P0：共享中间件竞争 + 运输优先级（2026-02-27）

- 对应设计文档: `doc/world-simulator/m4/m4-resource-product-system-p0-shared-bottleneck-logistics-priority-2026-02-27.design.md`
- 对应项目管理文档: `doc/world-simulator/m4/m4-resource-product-system-p0-shared-bottleneck-logistics-priority-2026-02-27.project.md`

审计轮次: 5

## 1. Executive Summary
- 在不破坏现有 runtime 主流程的前提下，实现 P0 两个可玩性增强点：
  - 共享中间件竞争：排产队列对关键中间件短缺更敏感。
  - 运输优先级：物流在途任务支持优先级并可观测。
- 物流优先级必须解释“为什么这批材料值得 urgent / standard”，以及它会怎样影响当前产线阻塞和同 tick 吞吐。
- 保持向后兼容：旧快照、旧事件、旧动作输入不因新增字段失败。

## 2. User Experience & Functionality

### In Scope
- runtime `RecipeJob` 增加 `bottleneck_tags`，并接入完工排序优先级推断。
- runtime `MaterialTransit` 增加 `priority`（`urgent` / `standard`），并接入在途完工排序与 SLA 统计。
- 玩家侧调运预览应展示 `priority_class`、`why_this_priority`、`throughput_slot_impact`、`blocked_recipe_before/after` 和 `recommended_transfer_action`，避免只在完成排序后才发现优先级影响。
- 补齐 `test_tier_required` 覆盖：
  - bottleneck 竞争导致完工顺序调整。
  - 物流优先级导致同 tick 完工顺序调整。
  - 物流优先级字段在事件中可观测。

### Out of Scope
- 不改 `oasis7_wasm_abi` 配方结构。
- 不做市场撮合机制重构。
- 不做 viewer 大面板改版，仅保障事件语义可观测。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications

### 1) 共享中间件标签
- `DomainEvent::RecipeStarted` 新增 `bottleneck_tags: Vec<String>`。
- `DomainEvent::RecipeCompleted` 新增 `bottleneck_tags: Vec<String>`。
- `RecipeJobState` 新增同名字段，默认空数组（兼容旧快照）。

标签来源（runtime 推断）：
- 对 `plan.consume` 的材料名做归类，首版覆盖：
  - `iron_ingot`
  - `copper_wire`
  - `control_chip`
  - `motor_mk1`

### 2) 物流优先级
- 新增 `MaterialTransitPriority`：`urgent` / `standard`（默认 `standard`）。
- `DomainEvent::MaterialTransferred`、`MaterialTransitStarted`、`MaterialTransitCompleted` 新增 `priority` 字段（带默认值，兼容旧事件）。
- `MaterialTransitJobState` 新增 `priority` 字段（带默认值，兼容旧快照）。
- 提交前 `transfer_impact_preview` 应至少解释：
  - `priority_class`
  - `why_this_priority`
  - `throughput_slot_impact`
  - `blocked_recipe_before`
  - `blocked_recipe_after`
  - `recommended_transfer_action`
  - `delivery_eta_ticks`
  - `loss_amount`

优先级来源（runtime 推断，首版）：
- 材料名包含 `survival/lifeline/critical/repair/maintenance/oxygen/water/emergency` 关键词时标记为 `urgent`，否则 `standard`。

### 3) 排序与观测
- 在途物流完工排序：`ready_at -> priority -> job_id`。
- 物流 SLA 指标新增 urgent 维度计数（完成/按时/延迟）。
- 排产完工排序保持原结构，新增“bottleneck 压力提升”逻辑：
  - 当 job 存在 `bottleneck_tags` 且对应材料总库存低于阈值时，优先级提升一级（例如 `scale -> energy`）。

## 5. Risks & Roadmap
- P0-T0：文档与任务建档。
- P0-T1：代码接线（事件/状态/排序/指标）。
- P0-T2：测试补齐（required）并回归。
- P0-T3：文档与日志收口。

### Technical Risks
- 兼容风险：事件和状态结构新增字段可能影响旧数据回放。
- 策略风险：阈值设置过激导致队列抖动。
- 可读性风险：若玩家只看到 `urgent / standard` 标签，无法判断该优先级是否值得占用吞吐或是否能及时解除瓶颈。
- 回归风险：排序变化会影响现有部分测试的事件顺序断言。

缓解：
- 所有新增字段加 `serde(default)`。
- 阈值先保守，优先做可观测再调参。
- `transfer_impact_preview` 只冻结玩家侧说明，不改变优先级关键词、排序规则、吞吐公式或事件 ABI。
- 先补单测再跑 required 回归。

## 6. Validation & Decision Record
- 追溯: 对应同名 `.project.md`，保持原文约束语义不变。
- DEC-M4-P0-001: 物流优先级必须随调运 quote 解释其产线影响；`urgent / standard` 不能只是事件排序字段。本决策不改变优先级推断、排序、吞吐或损耗公式。
