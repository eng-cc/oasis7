# 大世界基础设施 PRD

## 文档身份

- 产品模块：大世界基础设施
- 产品层唯一 PRD：`doc/product/world-infrastructure/prd.md`
- 产品模块总入口：`doc/product/README.md`
- 追踪主键：`PRD-PRODUCT-WORLD-INFRASTRUCTURE-xxx`
- 下层专业域：`doc/game/prd.md`、`doc/world-runtime/prd.md`、`doc/p2p/prd.md`

本文只承载产品承诺、范围 taxonomy、跨域 authority 与组合验收。专业域 PRD 继续拥有各自规则、实现契约、PRD-ID 和测试证据；任何下层专题都不得再自称“大世界基础设施”的并列产品总入口。

## 1. Why：产品问题与承诺

oasis7 已具备可编程区域设施、世界运行时、WASM、网络、共识与分布式存储等能力，但这些能力分散在多个专业域，读者无法从一个入口回答：玩家能建设什么、设施怎样持续影响区域、世界状态怎样持久且可审计，以及整个底座何时算产品闭环。

大世界基础设施的产品承诺是：玩家可在统一持久大世界中建设有限作用域、成本可解释、结果可审计的区域设施；这些设施由确定性运行时执行，由分布式状态底座保存与同步，并能在不破坏世界一致性和治理边界的前提下持续扩展。

## 2. What：范围与权威

### 2.1 三层范围

| 层级 | 产品责任 | 专业域权威 |
| --- | --- | --- |
| 区域设施层 | 设施发现、报价、commission、服务、维护、升级与回收；玩家收益、成本、作用域和反滥用边界可读 | `doc/game/prd.md` 及设施专题 PRD |
| 世界执行层 | 权威校验、确定性状态迁移、WASM/module 执行、事件、receipt、snapshot/replay 与治理 | `doc/world-runtime/prd.md` |
| 分布式状态层 | 网络、共识、最终性、DistFS、复制、gap/state sync、恢复与节点可观测性 | `doc/p2p/prd.md` |

### 2.2 设施生命周期

`发现 -> 报价 -> 建设/commission -> 运营/提供服务 -> 维护 -> 升级/迁移 -> 回收/退役`

每一步都必须具备明确前置条件、资源变化、权限边界、结构化失败和可追溯 receipt。当前首个正式冻结的设施专题 PRD 基线是 [`micro_depot`](../../game/gameplay/gameplay-wasm-backed-regional-infrastructure-micro-depot-2026-06-22.prd.md)；它验证区域专业化，不代表自由建造、任意 WASM 上传或全局治理授权。

### 2.3 产品原则

- world-first：设施改变统一世界中的真实状态，不制造第二套旁路世界。
- emergence-first：底座提供有限、可组合的规则能力，不预写全部结果。
- persistent：设施状态、成本和影响可恢复、可同步、可延续。
- auditable：报价、执行、失败、资源扣减和治理动作均有可复核证据。
- extensible：新增设施复用版本化 schema、module/WASM gate 与迁移规则，不绕过权威运行时。

## 3. 玩家边界与经济约束

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

验收证据由下层专业域提供：设施体验与经济走 `game`；确定性、WASM、receipt 和 replay 走 `world-runtime`；共识、复制、恢复与多节点一致性走 `p2p`。产品层只汇总是否形成端到端闭环，不复制各域测试步骤。

## 6. Non-Goals

- 不合并或重命名 `game`、`world-runtime`、`p2p` 等工程模块。
- 不把 `micro_depot` 改写成完整设施体系，也不改变其阶段、数值、gate 或 claim envelope。
- 不新增自由建造、任意 WASM 上传、默认全局治理权或无成本持久设施。
- 不在本文定义 runtime、WASM、共识、网络或存储实现细节。
- 不批量改写历史文档，不制造兼容 redirect 壳，不改变当前公开状态或发布声明。
