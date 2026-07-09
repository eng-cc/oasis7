# M4 资源与产品系统 P3：分层档案化与链路扩展实现（2026-02-27）

- 对应设计文档: `doc/world-simulator/m4/m4-resource-product-system-p3-layer-profile-chain-expansion-2026-02-27.design.md`
- 对应项目管理文档: `doc/world-simulator/m4/m4-resource-product-system-p3-layer-profile-chain-expansion-2026-02-27.project.md`

审计轮次: 5

## 1. Executive Summary
- 将“资源与产品分层”从设计建议提升为可执行数据契约，覆盖 R1~R5。
- 在不破坏现有动作主路径的前提下，把关键工业行为改为“Profile 优先 + 旧逻辑兜底”。
- 扩展内置工业模块链路，补齐从原料到基础设施件的代表性可运行样例。

## 2. User Experience & Functionality

### In Scope
- ABI 新增并落地三类档案结构：`MaterialProfileV1`、`ProductProfileV1`、`RecipeProfileV1`。
- `WorldState` 持久化三类 Profile 目录，并在 M4 bootstrap 注入默认目录。
- runtime 接线：
  - `TransferMaterial` 优先级与运损按 `MaterialProfileV1` 计算。
  - `ScheduleRecipe` 的瓶颈标签/阶段门槛按 `RecipeProfileV1` 生效。
  - 排产优先级可消费 `ProductProfileV1.role_tag`（旧关键词策略作为兜底）。
  - `ValidateProductWithModule` / `ProductValidated` 应消费 `ProductProfileV1.role_tag / tradable / unlock_stage` 生成玩家侧 `validation_unlock_preview`。
- 内置模块扩展（R2~R5 代表链路）：
  - 新增 4 个 recipe 模块：`alloy_plate`、`sensor_pack`、`module_rack`、`factory_core`。
  - 新增 4 个 product 模块：`alloy_plate`、`sensor_pack`、`module_rack`、`factory_core`。
  - 同步 bootstrap 清单、hash/identity manifest 与一致性测试。

### Out of Scope
- 不引入复杂订单簿与撮合市场。
- 不重写 viewer 展现层。
- 不做跨服经济结算。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications

### 1) ABI 档案结构
- `MaterialProfileV1`
  - `kind: String`
  - `tier: u8`
  - `category: String`
  - `stack_limit: i64`
  - `transport_loss_class: String`（`low/medium/high`）
  - `decay_bps_per_tick: i64`
  - `default_priority: String`（`standard/urgent`）
- `ProductProfileV1`
  - `product_id: String`
  - `role_tag: String`（`survival/scale/explore/governance`）
  - `maintenance_sink: Vec<MaterialStack>`
  - `tradable: bool`
  - `unlock_stage: String`（`bootstrap/scale_out/governance`）
- `RecipeProfileV1`
  - `recipe_id: String`
  - `bottleneck_tags: Vec<String>`
  - `stage_gate: String`
  - `preferred_factory_tags: Vec<String>`

### 2) WorldState 持久化目录
- `material_profiles: BTreeMap<String, MaterialProfileV1>`
- `product_profiles: BTreeMap<String, ProductProfileV1>`
- `recipe_profiles: BTreeMap<String, RecipeProfileV1>`

### 3) Profile 驱动规则
- 运输优先级：
  - 优先读取 `MaterialProfileV1.default_priority`。
  - 无配置时回退到关键词规则。
- 运输运损：
  - `loss_bps = base_loss_bps * class_factor`。
  - `class_factor`: `low=1x`, `medium=2x`, `high=4x`。
- 配方阶段门槛：
  - 若 `RecipeProfileV1.stage_gate` 高于当前 `IndustryStage`，拒绝排产。
- 瓶颈标签：
  - 优先使用 `RecipeProfileV1.bottleneck_tags`，无配置回退旧推断逻辑。
- 排产优先级：
  - 若产物存在 `ProductProfileV1.role_tag`，优先转为排队优先级。
  - 无配置回退关键词策略。
- 产物校验预览：
  - `product_validation_quote` / `validation_unlock_preview` 应从 `ProductProfileV1` 与当前 `IndustryStage` 派生。
  - 字段：
    - `product_id`
    - `role_tag`
    - `unlock_stage_before`
    - `unlock_stage_after`
    - `tradable`
    - `capability_unlocked`
    - `next_use_case`
    - `stage_progress_delta`
    - `recommended_next_action`
    - `validation_value_class`: `capability_unlock / trade_value / stage_progress / low_immediate_value`
  - Edge case: 若产品存在 `role_tag / tradable / unlock_stage`，但校验结果没有说明能力解锁、阶段推进或下一步用途，标记为 `product_validation_preview_missing`。
  - Acceptance: R2-R5 链路完成产物校验时，玩家应能看懂该产品属于 survival/scale/explore/governance 哪类用途、是否可交易、推进了哪个阶段，以及下一步推荐如何使用。

### 4) R1~R5 默认目录（首版）
- R1 原料：`iron_ore`、`copper_ore`、`carbon_fuel`、`silicate_ore`、`rare_earth_raw`
- R2 中间材料：`iron_ingot`、`copper_wire`、`alloy_plate`、`polymer_resin`、`circuit_substrate`
- R3 功能部件：`gear`、`control_chip`、`motor_mk1`、`sensor_pack`、`power_core`
- R4 终端产品：`logistics_drone`、`field_repair_kit`、`survey_probe`、`module_rack`
- R5 基础设施件：`factory_core`、`relay_tower_kit`、`grid_buffer_pack`

## 5. Risks & Roadmap
- P3-T0：建档（设计文档 + 项目管理文档）。
- P3-T1：ABI/State Profile 结构与默认目录注入。
- P3-T2：Profile 驱动规则接线（优先级、运损、阶段门槛、瓶颈标签、role_tag 优先级）。
- P3-T3：扩展内置模块链路并同步 artifact 清单。
- P3-T4：回归、文档/devlog 收口与结项。

## 测试口径

### test_tier_required
- Profile 结构序列化兼容（含 legacy 缺字段反序列化）。
- `TransferMaterial` 的优先级与运损按 profile 生效。
- `ScheduleRecipe` 的 `stage_gate` 拒绝路径与 `bottleneck_tags` 覆盖路径。
- role_tag 驱动排产优先级，且旧关键词策略仍可回退。

### test_tier_full
- `install_m4_economy_bootstrap_modules` 覆盖新增 module id、hash、identity 一致性。
- 模块链路从 R2 到 R5 可排产并完成产物校验落账。
- `./scripts/sync-m4-builtin-wasm-artifacts.sh --check` 通过。

### Technical Risks
- Profile 接线后行为偏移导致旧策略表现退化。
- 新增模块与 artifact 清单不同步会触发 bootstrap 失败。
- 阶段门槛参数过严可能造成链路过早阻断。
- 可读性风险：如果 `role_tag / tradable / unlock_stage` 只服务 runtime 排产和测试，玩家仍可能不知道 `ProductValidated` 为什么重要。

缓解：
- 所有新规则保持“配置优先、默认兼容”。
- 通过 descriptor-vs-manifest 一致性测试做门禁。
- 首版仅给高阶配方设置阶段门槛，低阶链路保持可用。
- `validation_unlock_preview` 只冻结玩家侧解释合同，不重平衡产品链、配方或阶段门槛，也不新增完整产品目录或科技树。

## 6. Validation & Decision Record
- 追溯: 对应同名 `.project.md`，保持原文约束语义不变。
- DEC-M4-P3-001: `ProductProfileV1.role_tag / tradable / unlock_stage` 必须能转化为玩家侧产品验证收益预览；`ProductValidated` 不能只证明落账成功，还要说明能力解锁、阶段推进和下一步用途。本决策不改变 profile ABI、配方链或阶段门槛。
