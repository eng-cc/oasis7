# Gameplay Base Runtime / WASM Layer Split（设计文档）

- 对应设计文档: `doc/game/gameplay/gameplay-base-runtime-wasm-layer-split.design.md`
- 对应项目管理文档: `doc/game/gameplay/gameplay-base-runtime-wasm-layer-split.project.md`
审计轮次: 4


## ROUND-002 历史记录与当前范围
- ROUND-002 曾将 `doc/game/gameplay/gameplay-top-level-design.prd.md` 列为阅读路径；该历史记录不构成当前 authority 或主从关系。
- 本文在 Base Runtime / WASM Layer Split 的声明范围内维护专题合同；活跃基线与路由回 `doc/game/prd.md`，当前执行状态回相应 project/evidence。

## 目标
- 将 `oasis7` runtime 的模块治理与执行架构拆分为两个明确边界：
  - 基础层（Base Runtime Layer）：世界不变量、模块通用校验、资源/权限约束。
  - Gameplay 层（WASM Gameplay Layer）：仅承载 gameplay 角色模块的契约校验、槽位冲突治理、模式就绪度观测。
- 保持现有行为语义与测试结果不变，优先做“架构分层重构”而非玩法规则扩展。
- 为后续 War/Governance/Crisis/Economic/Meta 五类模块落地提供稳定边界，避免继续耦合进单一大文件。

## 范围

### In Scope
- `crates/oasis7/src/runtime/world` 内部重构：
  - 抽离基础层模块治理校验实现。
  - 抽离 gameplay 层校验与观测实现。
  - 明确基础层调用 gameplay 层的边界点。
- 不改变外部 API 语义（`World::gameplay_modules`、`World::gameplay_mode_readiness` 等行为保持一致）。
- 通过现有 runtime 单测回归验证不回归。

### Out of Scope
- 不新增具体战争/政治/危机玩法规则。
- 不修改共识协议与网络协议。
- 不重构 `oasis7_wasm_abi` 数据结构。

## 接口/数据
- 基础层接口（目标归属）：
  - `validate_module_changes`
  - `shadow_validate_module_changes`
  - `validate_module_manifest`
  - `validate_module_abi_contract`
  - `validate_module_limits`
  - `active_module_manifest`
- Gameplay 层接口（目标归属）：
  - `validate_gameplay_contract_for_manifest`
  - `validate_gameplay_activation_conflicts`
  - `gameplay_modules`
  - `gameplay_mode_readiness`
- 关键数据保持不变：
  - `ModuleRole::Gameplay`
  - `ModuleAbiContract.gameplay`
  - `GameplayContract`
  - `GameplayModuleKind`

## 里程碑
- M0：完成设计与项目管理建档。
- M1：完成 runtime 代码分层拆分，编译通过。
- M2：完成 gameplay/runtime 关键测试回归，文档与 devlog 收口。

## 风险
- 私有方法跨文件可见性处理不当，可能导致编译错误。
- 拆分过程中若遗漏调用路径，可能出现模块治理行为回归。
- 若只做文件移动不建立边界调用约束，后续仍会再次耦合。

## 原文约束点映射（内容保真）
- 约束-1（目标与问题定义）：沿用原“目标”章节约束，不改变问题定义与解决方向。
- 约束-2（范围边界）：沿用原“范围”章节的 In Scope/Out of Scope 语义，不扩散到新增范围。
- 约束-3（接口/里程碑/风险）：沿用原接口字段、阶段节奏与风险口径，并保持可追溯。
