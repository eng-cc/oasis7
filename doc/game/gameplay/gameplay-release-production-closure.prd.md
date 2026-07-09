# Gameplay 发行收口（社会经济治理 + 压测门禁 + 联盟战争生命周期）

- 对应设计文档: `doc/game/gameplay/gameplay-release-production-closure.design.md`
- 对应项目管理文档: `doc/game/gameplay/gameplay-release-production-closure.project.md`

审计轮次: 4


## ROUND-002 主从口径
- 主入口文档：`doc/game/gameplay/gameplay-top-level-design.prd.md`。
- 本文档仅维护发行收口增量需求，通用玩法顶层规格以主入口为准。

## 目标
- 将当前 Gameplay 从“协议可运行”推进到“可发行的生产级闭环”，聚焦非 viewer 路径。
- 落地三项发行缺口：
  - 社会经济治理最小闭环（经济合约、声誉、基础治理规则）。
  - 长跑压测玩法覆盖门禁工程化（不只看稳定性，还看玩法动作覆盖）。
  - 联盟生命周期与战争约束/后果（join/leave/dissolve、战争成本与结算后果）。

## 范围
### In Scope
- Runtime Gameplay 协议扩展：新增经济合约/声誉/治理规则动作与领域事件。
- Runtime 状态扩展：经济合约状态、声誉账本、治理经济参数。
- Gameplay 生命周期扩展：合约过期处理、战争后果结算增强。
- LLM 决策协议扩展：新增联盟生命周期动作的 schema、解析和执行指纹。
- 压测脚本扩展：新增玩法覆盖门禁参数与断言。

### Out of Scope
- Viewer/UI 展示改造。
- 全量经济大系统（硬件全链路、数据市场全产品化）一次性完成。
- 新增链上协议或网络协议。

## 接口/数据
### Runtime Action（新增）
- `OpenEconomicContract`
- `AcceptEconomicContract`
- `SettleEconomicContract`
- `JoinAlliance`
- `LeaveAlliance`
- `DissolveAlliance`

### Domain Event（新增）
- `EconomicContractOpened`
- `EconomicContractAccepted`
- `EconomicContractSettled`
- `EconomicContractExpired`
- `ReputationAdjusted`
- `AllianceJoined`
- `AllianceLeft`
- `AllianceDissolved`

### Runtime State（新增）
- `economic_contracts: BTreeMap<String, EconomicContractState>`
- `reputation_scores: BTreeMap<String, i64>`
- `gameplay_policy: GameplayPolicyState`
  - 含税率、活跃合约配额、禁区列表等最小治理参数。

### 压测脚本 CLI（新增）
- `--min-action-kinds <n>`：最少动作种类数。
- `--require-action-kind <kind>:<min_count>`（可重复）：关键动作覆盖门槛。
- `--release-gate`：启用发行口径默认阈值。

## 里程碑
- M0：设计与任务拆解落盘。
- M1：经济治理闭环上线（动作、状态、生命周期、测试）。
- M2：压测玩法覆盖门禁上线（脚本与手册更新）。
- M3：联盟生命周期与战争后果增强上线（runtime + llm parser + 测试）。

## 风险
- 协议面扩展可能影响历史序列化兼容。
  - 缓解：新增字段全部 `#[serde(default)]`，行为向后兼容。
- 新门禁可能导致既有压测脚本失败率上升。
  - 缓解：提供显式参数与 `--release-gate` 开关，默认兼容旧口径。
- 战争后果结算若直接扣减资源，可能引入边界溢出或负值。
  - 缓解：结算时做饱和扣减与非负保护，并补回归测试。
- 联盟生命周期与战争后果若只在最终战报中解释，会让玩家无法判断宣战时机、强度和替代路径。
  - 缓解：后续战争后果增强必须承接 `war_declaration_quote` / `conflict_outcome_preview`，在提交前展示胜算、冲突窗口、结算风险和是否先谈判/补强的推荐理由。

## 验收标准
- Runtime 定向测试覆盖新增协议主路径/拒绝路径，并全部通过。
- `llm-longrun-stress.sh` 能对玩法覆盖做可配置断言，失败信息可定位具体缺失动作。
- LLM 决策工具 schema 能声明并解析联盟生命周期动作，不破坏既有动作解析。

## 原文约束点映射（内容保真）
- 约束-1（目标与问题定义）：沿用原“目标”章节约束，不改变问题定义与解决方向。
- 约束-2（范围边界）：沿用原“范围”章节的 In Scope/Out of Scope 语义，不扩散到新增范围。
- 约束-3（接口/里程碑/风险）：沿用原接口字段、阶段节奏与风险口径，并保持可追溯。
