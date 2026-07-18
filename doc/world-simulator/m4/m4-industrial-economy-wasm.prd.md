# M4 社会经济系统：工业链路与 WASM 模块化（Recipe/Product/Factory）

- 对应设计文档: `doc/world-simulator/m4/m4-industrial-economy-wasm.design.md`
- 对应项目管理文档: `doc/world-simulator/m4/m4-industrial-economy-wasm.project.md`

审计轮次: 5

## 1. Executive Summary

构建一套可演化的 M4 社会经济机制，满足以下约束：
- 从基础资源采集到多级制成品的完整闭环。
- 生产能力不预置，必须通过“先造设备再扩产”的方式逐步建立工业体系。
- 每个**配方**、**制成品**、**工厂**均可由独立 WASM 模块定义，并通过统一接口接入。
- 玩家或 AI 可以提交新模块，经过治理后动态扩展经济系统。

该设计参考《异星工厂》与《工业巨头》的核心体验：
- 资源链分层与瓶颈驱动。
- 产能建设先于利润释放。
- 物流、能耗、设备维护共同决定经济效率。

## 2. User Experience & Functionality

### In Scope（M4-ECO V1）
- 资源分层与制成品分类（基础资源 -> 中间品 -> 终端制成品）。
- 工厂分级建造机制（设施建造依赖上一级产物与能力）。
- 配方执行模型（输入/输出/时长/能耗/副产物/质量）。
- Recipe/Product/Factory 的 WASM 模块接口与数据契约。
- 模块治理、兼容版本、审计与确定性约束。
- 面向后续扩展的事件与动作协议。

### In Scope（M4-ECO V2：对外发布治理平台）
- 将“模块发布单/审批单”升级为 runtime 一等公民动作与事件，支持完整审计链路。
- 将 `material/product/recipe profile` 的变更纳入治理动作与门禁，不依赖直接内存 upsert 作为产品链路。
- 引入多角色审批策略（角色绑定 + 必需角色集合）并作为模块发布生效前置条件。
- 提供模块实例回滚动作，支持按实例回滚到历史版本并写入审计事件。
- 对外发布门禁收口：full 测试、m4/m5 工件 hash 校验、Web strict 闭环、长稳回归。

### Out of Scope（V1 不做）
- 全量市场金融系统（期货、信用衍生品、复杂税制）。
- 超大规模自动物流寻路优化（仅定义接口，不落地算法细节）。
- 多世界跨服贸易清算。

## 设计原则

- 最小可信内核：内核只负责账本、不变量、审计；经济语义由模块提供。
- 产能先行：制成品产出必须依赖已建成且在线的工厂能力。
- 确定性优先：同输入同种子必须得到同样生产结果，支持回放与审计。
- 插件优先：新增“配方/制成品/工厂”无需改内核代码，仅新增 WASM 模块。
- 渐进复杂度：先最小闭环，再扩展质量、维护、自动化、品牌、合同等机制。

### M4 builtin 维护合同

- Recipe、Product、Factory builtin 复用按模块类型划分的实现模板；具体模块以声明式参数为主，公共 action 解析、判定顺序、reject reason 与输出格式不得分叉复制。
- Bootstrap 安装由 descriptor/catalog 驱动，新增、替换或删除 builtin 时不得回退为调用点手工注册。
- Descriptor module ID、builtin ID catalog、artifact hash 与 identity manifest 必须保持一致，并由 repo-owned 检查在安装或发布前 fail closed；任一局部列表相等都不能单独证明 canonical artifact 合法。
- 模板或 bootstrap 重构必须保持既有动作判定顺序、拒绝语义、输出合同与回放结果；通用 canonical build、hash、identity 和 release policy 继续由 `doc/world-runtime/prd.md` 及其 WASM 专题拥有。

### 市场、硬件、数据与治理不变量

- 工厂能力必须承担可审计的磨损、维护和回收闭环；持续高负载可以放大折旧压力，但具体速率、阈值和返还比例属于可调平衡参数，不在本 PRD 冻结。
- 数据采集必须支付电力等明确成本；跨 Agent 数据转移或数据合约结算必须先取得 owner 授权。拒绝时保持账本不变，并向上层提供可解释、可恢复的权限结果。
- 合约与声誉奖励必须限制同一参与方重复刷取和窗口内无界增发；冷却、窗口、上限等数值由当前 runtime/config 拥有，产品层只冻结“奖励有界且可审计”的不变量。
- 禁区、配额、税费和电费等治理政策可以拒绝行动或改变结算，但必须经过授权治理路径、接受边界校验并产生可回放事件；不得以局部收益绕过治理约束。
- 电力交易继续采用 `m4-power-system.prd.md` 的纯供需定价，不引入峰谷时段因子；价格公式与 clamp 边界以该长期权威为准。

## 机制总览

每个 tick 的经济执行按固定顺序进行：
1. 资源输入确认：检查库存、能量、工厂可用产能。
2. 配方求值：Recipe 模块计算本批次可执行量、损耗与副产物。
3. 工厂约束：Factory 模块裁剪吞吐（槽位、效率、维护状态、功率上限）。
4. 制成品约束：Product 模块校验产物属性（质量、保质期、堆叠规则）。
5. 账本提交：原料扣减、产物入库、副产物处理、事件落盘。

## 资源与制成品层级

下表是供设计沟通使用的长期分层语义与代表性示例，不是完整可运行目录，也不证明每个条目都已有 recipe/product/module。当前 Profile、bootstrap descriptor、模块 ID、hash 与 identity 清单分别以 ABI/runtime 代码和 canonical manifest 为执行真值。

| 层级 | 类别 | 示例 | 主要来源 | 主要去向 |
| --- | --- | --- | --- | --- |
| T0 | 基础资源 | 矿石、冰、硅酸盐、碳质块 | 开采/采集 | T1 初加工 |
| T1 | 初级加工品 | 金属锭、纯水、聚合前体、晶圆坯 | 熔炼/提纯/裂解 | T2 部件加工 |
| T2 | 标准部件 | 齿轮、管线、线缆、基板、电池单元 | 机加工/化工线 | T3 功能组件 |
| T3 | 功能组件 | 电机、控制器、传感器、动力包 | 组装车间 | T4 终端制成品 |
| T4 | 终端制成品 | 工业机器人、运输无人机、模块机柜 | 总装线 | 工厂升级/市场交易 |
| T5 | 基础设施件 | 工厂核心、物流节点、能源站套件 | 专项配方 | 新工厂建造与扩建 |

## 工厂渐进建造机制

工业能力必须按阶段解锁，不允许“开局全工厂”。

### 阶段 S0：生存级加工
- 可用设施：便携拆解器、手工装配台。
- 能力：T0 -> 少量 T1/T2。
- 目标：制造第一批固定式矿机与熔炉组件。

### 阶段 S1：基础采炼
- 新工厂：采矿站、熔炼炉、基础仓储。
- 解锁条件：完成 `factory.miner.mk1` 与 `factory.smelter.mk1` 建造配方。
- 能力：稳定产出金属锭、基础构件，形成持续性原料供给。

### 阶段 S2：化工与材料
- 新工厂：化工反应器、精炼站。
- 解锁条件：具备持续电力 + 压力容器 + 控温部件。
- 能力：T1 -> T2（聚合材料、液体化学品、功能介质）。

### 阶段 S3：精密制造
- 新工厂：精密机加工中心、电子装配线。
- 解锁条件：高纯材料、稳定能源、基础自动化控制单元。
- 能力：T2 -> T3（电机、控制器、传感器）。

### 阶段 S4：系统总装
- 新工厂：系统总装厂、质量检测站。
- 解锁条件：多工厂协同能力与供应链稳定度阈值。
- 能力：T3 -> T4/T5（终端制成品与下一阶段工厂核心件）。

## 配方执行机制

每个配方由独立 Recipe 模块定义，最小执行要素：
- 输入：多种原料与最小批量。
- 输出：主产物与副产物。
- 周期：每批次生产时长（tick）。
- 能耗：静态功耗 + 批次功耗。
- 工厂要求：允许执行的工厂标签/等级。
- 环境要求：温度、压力、辐射等可选约束。

标准吞吐计算（建议口径）：

`effective_batches = floor(base_batches * factory_efficiency * power_factor * maintenance_factor * operator_factor)`

质量计算（可选）：

`quality_score = base_quality + factory_bonus + material_bonus + stochastic_term(seed)`

其中 `seed` 必须来源于可回放上下文（world seed + event id），禁止真实随机源。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications

### 1) 统一模块分类

- `Recipe` 模块：定义“如何把输入加工成输出”。
- `Product` 模块：定义“产物属性与存储/交易行为”。
- `Factory` 模块：定义“设施建造规则与产线能力边界”。

模块命名建议：
- `m4.recipe.<chain>.<name>`
- `m4.product.<category>.<name>`
- `m4.factory.<tier>.<name>`

### 2) 核心结构（接口草案）

```rust
pub enum EconomyModuleKind {
    Recipe,
    Product,
    Factory,
}

pub struct MaterialStack {
    pub kind: String,
    pub amount: i64,
}

pub struct RecipeModuleSpec {
    pub recipe_id: String,
    pub display_name: String,
    pub inputs: Vec<MaterialStack>,
    pub outputs: Vec<MaterialStack>,
    pub byproducts: Vec<MaterialStack>,
    pub cycle_ticks: u32,
    pub power_per_cycle: i64,
    pub allowed_factory_tags: Vec<String>,
    pub min_factory_tier: u8,
}

pub struct ProductModuleSpec {
    pub product_id: String,
    pub display_name: String,
    pub category: String,
    pub stack_limit: u32,
    pub decay_per_tick_bps: u32,
    pub quality_levels: Vec<String>,
    pub tradable: bool,
}

pub struct FactoryModuleSpec {
    pub factory_id: String,
    pub display_name: String,
    pub tier: u8,
    pub tags: Vec<String>,
    pub build_cost: Vec<MaterialStack>,
    pub build_time_ticks: u32,
    pub base_power_draw: i64,
    pub recipe_slots: u16,
    pub throughput_bps: u32,
    pub maintenance_per_tick: i64,
}
```

### 3) 运行时请求/响应（接口草案）

```rust
pub struct RecipeExecutionRequest {
    pub recipe_id: String,
    pub factory_id: String,
    pub desired_batches: u32,
    pub available_inputs: Vec<MaterialStack>,
    pub available_power: i64,
    pub deterministic_seed: u64,
}

pub struct RecipeExecutionPlan {
    pub accepted_batches: u32,
    pub consume: Vec<MaterialStack>,
    pub produce: Vec<MaterialStack>,
    pub byproducts: Vec<MaterialStack>,
    pub power_required: i64,
    pub duration_ticks: u32,
    pub reject_reason: Option<String>,
}

pub struct FactoryBuildRequest {
    pub factory_id: String,
    pub site_id: String,
    pub builder: String,
    pub available_inputs: Vec<MaterialStack>,
    pub available_power: i64,
}

pub struct FactoryBuildDecision {
    pub accepted: bool,
    pub consume: Vec<MaterialStack>,
    pub duration_ticks: u32,
    pub reject_reason: Option<String>,
}

pub struct ProductValidationRequest {
    pub product_id: String,
    pub stack: MaterialStack,
    pub deterministic_seed: u64,
}

pub struct ProductValidationDecision {
    pub product_id: String,
    pub accepted: bool,
    pub notes: Vec<String>,
    pub stack_limit: u32,
    pub tradable: bool,
    pub quality_levels: Vec<String>,
}
```

### 4) Rust Trait 约定（模块作者侧）

```rust
pub trait RecipeModuleApi {
    fn describe_recipe(&self) -> RecipeModuleSpec;
    fn evaluate_recipe(&self, req: RecipeExecutionRequest) -> RecipeExecutionPlan;
}

pub trait ProductModuleApi {
    fn describe_product(&self) -> ProductModuleSpec;
    fn evaluate_product(&self, req: ProductValidationRequest) -> ProductValidationDecision;
}

pub trait FactoryModuleApi {
    fn describe_factory(&self) -> FactoryModuleSpec;
    fn evaluate_build(&self, req: FactoryBuildRequest) -> FactoryBuildDecision;
}
```

## 动作与事件协议（草案）

### 动作
- `action.economy.build_factory`
- `action.economy.schedule_recipe`
- `action.economy.validate_product`
- `action.economy.validate_product_with_module`
- `action.economy.transfer_inventory`
- `action.economy.maintain_factory`

### 事件
- `domain.economy.factory_built`
- `domain.economy.recipe_started`
- `domain.economy.recipe_completed`
- `domain.economy.product_validated`
- `domain.economy.factory_degraded`

所有事件要求可回放，且由模块输出到统一审计链路。

## 模块治理与兼容

- 模块提交：玩家/AI 提交 `wasm_hash + manifest + spec`。
- 治理流程：`propose -> shadow -> approve -> apply`。
- 兼容策略：
  - ABI 版本：`wasm-1`（底层）+ `economy-v1`（领域层）。
  - 向后兼容字段仅新增不破坏；破坏性变更通过新版本模块 ID 发布。
- 安全约束：
  - 禁止模块直接写账本；只能输出意图，由内核做最终提交。
  - 禁止真实时间和系统随机数；使用上下文种子。
  - 输出大小、effect/emits 数量受 `ModuleLimits` 限制。

### 模块发布单（V2）

- 新增动作：
  - `action.module_release.submit`
  - `action.module_release.shadow`
  - `action.module_release.approve_role`
  - `action.module_release.reject`
  - `action.module_release.apply`
- 新增事件：
  - `domain.module_release.requested`
  - `domain.module_release.shadowed`
  - `domain.module_release.role_approved`
  - `domain.module_release.rejected`
  - `domain.module_release.applied`
- 发布单状态机：
  - `requested -> shadowed -> partially_approved -> approved -> applied`
  - 任意前置状态可进入 `rejected`
- 发布单约束：
  - `apply` 前必须满足必需角色集合全部达成（见“多角色审批策略”）。
  - `apply` 内部仍走模块治理闭环（manifest 更新审计不可绕过）。

### Profile 治理动作（V2）

- 新增动作：
  - `action.economy.govern_material_profile`
  - `action.economy.govern_product_profile`
  - `action.economy.govern_recipe_profile`
- 新增事件：
  - `domain.economy.material_profile_governed`
  - `domain.economy.product_profile_governed`
  - `domain.economy.recipe_profile_governed`
- 治理门禁：
  - 必须携带 `proposal_id`，且 proposal 状态为 `approved|applied`。
  - 仅允许字段级白名单更新（避免破坏性配置注入）。

### 多角色审批策略（V2）

- 角色绑定：`agent_id -> role_set`（例：`security`、`economy`、`runtime`）。
- 发布单必需角色：默认 `["security", "economy", "runtime"]`，可在发布单提交时按策略收敛。
- 同一角色仅计一次有效审批；未绑定该角色的审批请求拒绝。
- 角色缺失时不可 `apply`。

### 回滚策略（V2）

- 新增动作：`action.module.rollback_instance`
- 新增事件：`domain.module.rollback_applied`
- 回滚约束：
  - 仅允许回滚到同 `module_id` 的历史已注册版本。
  - 回滚也走治理审计闭环，不允许直接篡改实例状态。

## 测试与验收

V1 需要覆盖以下测试组：
- 接口契约测试：Recipe/Product/Factory 结构体序列化反序列化稳定。
- 决策一致性测试：同输入与同 seed 的执行计划一致。
- 账本守恒测试：输入扣减与输出增加严格平衡（含副产物）。
- 工厂门槛测试：未达工厂等级或标签不匹配时配方必须拒绝。
- 构建链路测试：S0 -> S4 解锁顺序可重复通过。

V2 需要新增以下测试组：
- 模块发布单状态机测试：提交/影子校验/角色审批/拒绝/应用状态迁移可回放。
- 多角色审批门禁测试：角色不满足时 `apply` 必须拒绝；满足后可应用。
- Profile 治理门禁测试：`proposal_id` 不合法或未批准时拒绝更新。
- 回滚测试：实例可回滚到历史版本，且写入审计事件与 proposal 追溯信息。
- 发布门禁集成测试：`full + m4/m5 hash + Web strict + 长稳` 结果可审计。

## 当前实现进展（2026-02-14）

- ABI 层已提供 Recipe/Product/Factory 接口类型与 trait 草案（`oasis7_wasm_abi`）。
- runtime 已落地最小执行闭环：
  - 新动作：`BuildFactory`、`ScheduleRecipe`
  - 新事件：`FactoryBuildStarted`、`FactoryBuilt`、`RecipeStarted`、`RecipeCompleted`
  - 新状态：材料库存、工厂实例、建造队列、配方队列
  - 新流程：`step` 每 tick 自动结算到期建造与排产任务
- runtime 已接入模块在线评估路径：
  - 新动作：`BuildFactoryWithModule`、`ScheduleRecipeWithModule`
  - 执行流程：`step_with_modules` 中先调用指定 WASM 模块求值，再转为 `BuildFactory/ScheduleRecipe` 落地
  - 模块输出契约（emit kind）：
    - `economy.factory_build_decision`
    - `economy.recipe_execution_plan`
  - 非法模块输出（缺失 emit / 多 emit / 解码失败）统一记录 `ModuleCallFailed(InvalidOutput)`
- runtime 已接入 Product 模块在线校验路径：
  - 新动作：`ValidateProductWithModule`（模块求值）-> `ValidateProduct`（落地）
  - 新事件：`ProductValidated`
  - 模块输出契约（emit kind）：`economy.product_validation`
  - 模块拒绝统一映射为 `ActionRejected(RuleDenied)`，并保留模块 notes
- runtime 已接入 Product 校验自动闭环：
  - `step_with_modules` 的配方完工结算阶段会自动对产出执行 Product 模块校验（若存在匹配模块）
  - 产物模块解析策略：内置 `m4.product.*` 显式映射优先，随后按 `*.product.*.<product_kind>` 后缀规则匹配扩展模块
  - 若任一产物校验失败，则该配方批次产物与副产物均不入账（输入与能耗仍按已执行批次扣减）
- 已覆盖 runtime 定向测试：建造时序、排产时序、产线容量限流、库存与电力扣减、完工产出入账。
- 已提供内置 M4 工业模块包（WASM 工件 + 治理安装入口）：
  - 工厂模块：`m4.factory.miner.mk1`、`m4.factory.smelter.mk1`、`m4.factory.assembler.mk1`
  - 配方模块：`m4.recipe.smelter.iron_ingot`、`m4.recipe.smelter.copper_wire`、`m4.recipe.assembler.gear`、`m4.recipe.assembler.control_chip`、`m4.recipe.assembler.motor_mk1`、`m4.recipe.assembler.logistics_drone`
  - 制成品模块：`m4.product.material.iron_ingot`、`m4.product.component.control_chip`、`m4.product.component.motor_mk1`、`m4.product.finished.logistics_drone`
- 已在 runtime 提供一键治理装载：
  - `World::install_m4_economy_bootstrap_modules(actor)`
  - 模块 manifest 统一使用 `M4_ECONOMY_MODULE_VERSION = 0.1.0`，并受 `ModuleLimits` 约束
- 已完成真实 wasm 执行链路回归：基础资源 -> 熔炼/装配 -> `logistics_drone` 终端制成品。
- 2026-03-05：启动 V2（方案B）实施，进入 E9~E13（发布单、profile 治理、多角色审批、回滚、发布门禁）。
- 2026-03-05：完成 E9（模块发布单动作/事件/状态机），新增 submit/shadow/approve_role/reject/apply 全链路审计与状态回放。
- 2026-03-05：完成 E10（profile 治理动作），新增 `proposal_id` 门禁与 `*_profile_governed` 事件落账闭环。
- 2026-03-05：完成 E11（多角色审批策略），新增 `agent -> roles` 绑定与越权审批拒绝门禁。
- 2026-03-05：完成 E12（模块实例回滚），新增 `rollback_module_instance` 动作与 `ModuleRollbackApplied` 审计事件，回滚路径复用治理闭环并校验历史版本兼容性。
- 2026-03-05：完成 E13（发布门禁收口），新增 `scripts/release-gate.sh` 与 `scripts/release-gate-smoke.sh`，将 gate 接入 `release-packages` workflow 前置并收口 S7 TODO 口径。
- 2026-03-05：完成 E14（对外发布演练与门禁稳定性修复），`release-gate --quick` 实跑通过并修复门禁脚本中的测试隔离与 Web strict 抗噪声细节。

## 5. Risks & Roadmap

- M4-E1：完成机制与接口设计（本文件 + 项目文档）。
- M4-E2：落地 ABI 数据结构与基础测试。
- M4-E3：接入 runtime 动作/事件最小闭环（build_factory/schedule_recipe）。
- M4-E4：完成首批内置示例模块（最少 6 配方、4 制成品、3 工厂）。
- M4-E5：开放玩家/AI 自定义模块接入与治理模板。
- M4-E9：模块发布单动作/事件/状态机落地。
- M4-E10：profile 治理动作落地（proposal 门禁）。
- M4-E11：多角色审批策略落地。
- M4-E12：模块实例回滚能力落地。
- M4-E13：发布门禁脚本与工作流收口。
- M4-E14：对外发布演练与门禁稳定性修复。

### Technical Risks

- 复杂度激增：配方树与工厂约束会迅速膨胀，需要阶段化范围控制。
- 模块质量参差：外部模块可能性能差或语义冲突，需要 shadow 校验与评分机制。
- 平衡性风险：高阶配方收益过高会导致经济塌缩，需要参数治理。
- 性能风险：模块数量增加会拉长 tick 时延，需要缓存与调用预算。
- 可解释性风险：若缺少标准事件与诊断字段，难以定位产能瓶颈。
- 治理僵局风险：多角色审批可能导致发布排队，需要超时与拒绝快速路径。
- 回滚滥用风险：高频回滚会破坏经济预期，需要角色审计与频率门限。

## 6. Validation & Decision Record

- PRD-ID 追溯：
  - `PRD-M4-E9` 模块发布单动作/事件/状态机
  - `PRD-M4-E10` profile 治理动作与 proposal 门禁
  - `PRD-M4-E11` 多角色审批策略
  - `PRD-M4-E12` 模块实例回滚能力
  - `PRD-M4-E13` 发布门禁收口
  - `PRD-M4-E14` 对外发布演练与门禁稳定性修复
- 测试分层：
  - `test_tier_required`：E9/E10/E11 的状态机与拒绝路径单测
  - `test_tier_full`：E12 回滚与 E13 发布门禁联动回归
- 关键决策：
  - 采用“发布单 + 角色审批 + 内部治理闭环”而非“直接 install/upgrade exposed API”，以保持审计完整与对外可运营性。
  - profile 变更走动作门禁，不把 `World::upsert_*_profile` 作为外部产品入口，避免绕过治理。
