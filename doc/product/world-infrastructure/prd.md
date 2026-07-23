# 大世界基础设施 PRD

## 文档身份

- 产品模块：大世界基础设施
- 产品模块 slug：`world-infrastructure`
- 产品层唯一 PRD：`doc/product/world-infrastructure/prd.md`
- 产品模块总入口：`doc/product/README.md`
- Product PRD-ID：`PRD-PRODUCT-002`
- 生命周期：`active`
- Owner role：`producer_system_designer`
- Last reviewed：`2026-07-19`
- 后继文档：`无`
- 下层专业域：[`doc/game/prd.md`](../../game/prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/p2p/prd.md`](../../p2p/prd.md)

本文只承载产品承诺、范围 taxonomy、跨域 authority 与组合验收。专业域 PRD 继续拥有各自规则、实现契约、PRD-ID 和测试证据；任何下层专题都不得再自称“大世界基础设施”的并列产品总入口。

### 活跃产品专题

- [`世界连续性、治理与恢复`](world-continuity-governance-and-recovery.prd.md)：权威世界不分叉、恢复后玩家结果连续、治理不可旁路、紧急处置可申诉与长期世界放行。
- [`受治理的区域能力与扩展`](governed-regional-capabilities-and-extensions.prd.md)：有限区域设施如何解决可读压力，以及经治理的新工业能力如何安全进入同一持久世界。

## 1. 产品承诺

oasis7 已具备可编程区域设施、世界运行时、WASM、网络、共识与分布式存储等能力，但这些能力分散在多个专业域，读者无法从一个入口回答：玩家能建设什么、设施怎样持续影响区域、世界状态怎样持久且可审计，以及整个底座何时算产品闭环。

大世界基础设施的产品承诺是：玩家可在统一持久大世界中建设有限作用域、成本可解释、结果可审计的区域设施；这些设施由确定性运行时执行，由分布式状态底座保存与同步，并能在不破坏世界一致性和治理边界的前提下持续扩展。

工业成长中的关键承诺必须让玩家在提交前读懂成本、约束、预期效果以及下一步或恢复动作；执行结果必须确定、可审计，`ProductValidated` 还必须呈现与玩家相关的能力、用途和成长后果。区域专业化应形成可选择的协作与取舍压力，而不是对所有玩家征收同一种必付税。具体字段、平衡、ABI、profile、event 与 receipt schema 继续由 `game`、runtime 和 WASM 专业权威维护。

## 2. 范围与玩家边界

### 2.1 三层范围

| 层级 | 产品责任 | 专业域权威 |
| --- | --- | --- |
| 区域设施层 | 设施发现、报价、commission、服务、维护、升级与回收；玩家收益、成本、作用域和反滥用边界可读 | `doc/game/prd.md` 及设施专题 PRD |
| 世界执行层 | 权威校验、确定性状态迁移、WASM/module 执行、事件、receipt、snapshot/replay 与治理 | `doc/world-runtime/prd.md` |
| 分布式状态层 | 网络、共识、最终性、DistFS、复制、gap/state sync、恢复与节点可观测性 | `doc/p2p/prd.md` |

### 2.2 设施生命周期

`发现 -> 报价 -> 建设/commission -> 运营/提供服务 -> 维护 -> 升级/迁移 -> 回收/退役`

每一步都必须具备明确前置条件、资源变化、权限边界、结构化失败和可追溯 receipt。当前首个正式冻结的设施专题 PRD 基线是 [`micro_depot`](../../game/gameplay/gameplay-regional-infrastructure-micro-depot-contract.prd.md)；它验证区域专业化，不代表自由建造、任意 WASM 上传或全局治理授权。

### 2.3 产品原则

- world-first：设施改变统一世界中的真实状态，不制造第二套旁路世界。
- emergence-first：底座提供有限、可组合的规则能力，不预写全部结果。
- persistent：设施状态、成本和影响可恢复、可同步、可延续。
- auditable：报价、执行、失败、资源扣减和治理动作均有可复核证据。
- extensible：新增设施复用版本化 schema、module/WASM gate 与迁移规则，不绕过权威运行时。

### 2.4 统一世界模型与术语边界

oasis7 向玩家表达一个统一持久大世界；玩家行动、Agent、组织、工业与治理都发生在同一持久、可审计的世界叙事中。`viewer` / `pure_api` 是进入同一世界的玩家入口，不是不同世界类型；local / test / production environment 只说明运行阶段，也不是不同玩家世界；`world_id` 与 network tier 是专业技术分区，不是玩家可选择的世界品牌。

统一世界模型本身不证明特定拓扑、容量、可用性、public launch、mainnet 或其他 readiness。当前公开状态与 claim envelope 仍由根 [`README.md`](../../../README.md) 拥有；玩家规则、权威执行和分布式状态证明分别由 [`doc/game/prd.md`](../../game/prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md) 与 [`doc/p2p/prd.md`](../../p2p/prd.md) 维护，测试只作为这些 claim 的验证机制，不产生新的产品世界。

### 2.5 资源模型与模块扩展边界

当前已验证的通用资源类型为 `Electricity` 与 `Data`。世界中的材料、产物和记录可由具体 runtime 领域模型或经治理的模块表达；它们不会因此扩展通用资源类型，也不会自动获得统一的结算、转移、所有权或公开可用性承诺。

所有资源与领域记录仍须遵守权威校验、权限、审计、metering 和治理边界。具体成本、经济数值、数据结构、ABI、manifest、capability 与玩家可读规则继续由对应 gameplay、runtime 和 WASM 专业合同及证据定义，本产品层不把实现现状冻结为永久资源目录。

## 3. 权威与冲突处理

产品层拥有区域设施、世界执行与分布式状态组成的端到端承诺；`doc/game/prd.md`、`doc/world-runtime/prd.md` 与 `doc/p2p/prd.md` 分别拥有其玩家规则、执行合同与分布式状态证明。冲突时不得由产品层静默改写专业合同，必须由产品 owner 与相应专业 owner 形成显式跨域决策。

### 3.1 玩家边界与经济约束

玩家可以发现合格地点、比较建设与维护成本、commission 被允许的设施、消费其服务、查看区域影响与 receipt，并在规则允许时升级或回收。

玩家不能自由写入权威状态、上传任意未治理模块、绕过 claim/资源/upkeep、通过设施取得默认全局治理权，或把本地/测试环境包装成多个玩家世界。

每类设施必须说明安装成本、持续成本或耗尽边界、收益对象、作用半径、容量/吞吐、break-even 或低使用率风险、回收结果与反套利约束。数值平衡仍由对应 gameplay 专题和实现版本管理，本 PRD 不冻结新的经济数值。

## 4. 路线图

1. 单设施闭环：以 `micro_depot` 证明 quote、commission、有限库存/吞吐、服务、失败、reclaim 与 replay 一致。
2. 设施组合：在相同 authority 与 receipt 模型下扩展物流、维修、审计等有限作用域设施。
3. 区域网络：支持多设施依赖、区域专业化与可读的资源/维护压力，不把设施变成默认必点税。
4. 治理扩展：通过显式 module upgrade、migration 和治理审批演进能力，不破坏历史重放与世界最终性。

## 5. Done：成功标准与验收

- SC-1：从 `doc/product/README.md` 只能进入一个“大世界基础设施”产品 PRD。
- SC-2：产品 PRD 能同时回答区域设施、世界执行和分布式状态三层的职责与 authority。
- SC-3：至少一个设施具备从报价到退役的可达闭环，玩家可见成本、限制、收益和 receipt。
- SC-4：同一确认动作在 replay 后得到一致状态；失败不产生部分扣费、幽灵设施或未审计副作用。
- SC-5：设施状态可经 snapshot/replication/state sync 恢复，且 transport green 不能替代世界状态闭环证明。
- SC-6：新增设施必须映射到专业域 PRD-ID、`test_tier_required`，并在涉及多节点、迁移或长期一致性时补 `test_tier_full`。
- SC-7：工业成长的关键提交点在确认前可读成本、约束、效果与下一步或恢复动作；执行证据确定且可审计，产品校验结果能说明玩家能力、用途与成长后果，区域专业化不退化为普遍税负。
- SC-8：玩家可读说明将统一持久世界模型与入口、运行环境、`world_id`、network tier 和 readiness 分层表达，不把技术分区或环境包装成不同玩家世界，也不由产品术语推导未经证据支持的公开 claim。
- SC-9：至少一个区域设施样例以同一 facility/action/receipt identity 贯通玩家报价与确认、权威执行、持久化/replay、复制或 state sync、重连后的玩家可见结果与失败恢复；game、runtime 或 P2P 的孤立通过不得替代该端到端组合证据。
- SC-10：至少一条可达工业生命周期贯通资源获取或 sourcing、转化或生产、能力或区域服务用途，以及维护、恢复或退役；不得依赖未解释的预置收益，且全过程保持资源守恒、权限校验、回放一致和可读的下一决策。具体 ledger、运输、排程与产品校验动作顺序由专业域拥有。

验收证据由下层专业域提供：设施体验与经济走 `game`；确定性、WASM、receipt 和 replay 走 `world-runtime`；共识、复制、恢复与多节点一致性走 `p2p`。产品层只汇总是否形成端到端闭环，不复制各域测试步骤。

### 5.1 验收追踪

| 成功标准 | 专业 owner | 专业域 PRD-ID | 权威文档 | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- | --- |
| SC-1 | producer_system_designer | PRD-GAME-001 | `doc/game/prd.md` | 产品入口唯一性检查 | test_tier_required |
| SC-2 | producer_system_designer | PRD-GAME-002 / PRD-WORLD_RUNTIME-001 / PRD-P2P-001 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/p2p/prd.md` | 权威映射与反向链接检查 | test_tier_required |
| SC-3 | gameplay_designer | PRD-GAME-016 | `doc/game/prd.md` | 设施报价到退役玩法闭环证据 | test_tier_required |
| SC-4 | runtime_engineer | PRD-WORLD_RUNTIME-001 | `doc/world-runtime/prd.md` | receipt、失败原子性与 replay 回归 | test_tier_required |
| SC-5 | blockchain_ops_engineer | PRD-P2P-002 | `doc/p2p/prd.md` | snapshot、replication、state-sync 多节点恢复证据 | test_tier_full |
| SC-6 | qa_engineer | PRD-GAME-003 | `doc/game/prd.md` | PRD-ID 到 required/full 证据的发布门禁 | test_tier_required |
| SC-7 | gameplay_designer | PRD-GAME-002 | `doc/game/prd.md`; `doc/game/gameplay/gameplay-top-level-design.prd.md` | 工业提交前可读性、确定性结果、产品校验后果与区域专业化取舍证据 | test_tier_required |
| SC-8 | producer_system_designer | PRD-GAME-002 / PRD-WORLD_RUNTIME-001 / PRD-P2P-001 | `README.md`; `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/p2p/prd.md` | 世界模型、技术分区、运行环境与公开 claim 分层审计 | test_tier_required |
| SC-9 | producer_system_designer / gameplay_designer / runtime_engineer / blockchain_ops_engineer | PRD-GAME-016 / PRD-WORLD_RUNTIME-001 / PRD-P2P-002 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/p2p/prd.md` | 同一设施身份跨 quote、执行、receipt、replay、sync、重连与恢复的组合证据，包含适用的 S9/S10 长跑与恢复演练 | test_tier_full |
| SC-10 | gameplay_designer / runtime_engineer | PRD-GAME-002 / PRD-WORLD_RUNTIME-001 | `doc/game/prd.md`; `doc/world-runtime/prd.md` | 资源获取到工业用途及维护/退役的守恒、权限、回放和玩家下一步证据 | test_tier_required |

## 6. Non-Goals

- 不合并或重命名 `game`、`world-runtime`、`p2p` 等工程模块。
- 不把 `micro_depot` 改写成完整设施体系，也不改变其阶段、数值、gate 或 claim envelope。
- 不新增自由建造、任意 WASM 上传、默认全局治理权或无成本持久设施。
- 不在本文定义 runtime、WASM、共识、网络或存储实现细节。
- 不批量改写历史文档，不制造兼容 redirect 壳，不改变当前公开状态或发布声明。
