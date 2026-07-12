# P2P Token 文档导航

本目录收敛主链 Token 的创世分配、交易授权、治理分发、理想化交易目标态和服务额度 bridge。首次阅读先按要解决的问题选择一组专题；不要把历史完成状态或目标态草案误读成当前发行、公开流通或 mainnet readiness 结论。

## 从这里开始

| 需要回答的问题 | 首读专题 | 边界 |
| --- | --- | --- |
| 当前 `Oasis Coin / 绿洲币` 的创世总量、低流通、早期贡献奖励和 freeze gate 是什么？ | `mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.prd.md` | 当前创世与贡献奖励口径；实际地址绑定、mint 与 release readiness 仍以对应 project、freeze sheet 和 P2P 模块状态为准。 |
| 创世 bucket、controller slot、签名策略与尚待绑定项在哪里追溯？ | `mainchain-token-genesis-parameter-freeze-sheet-2026-03-22.md` | formal freeze companion；不是已完成 production mint 的声明。 |
| 已有 runtime 的 Token 分配、治理 bridge 与 release 语义如何演进？ | `mainchain-token-allocation-mechanism.prd.md` | 已完成的基础分配机制与 release companion；不替代当前创世 freeze 口径。 |
| phase-2 治理 bridge 分发的历史验收在哪里？ | `mainchain-token-allocation-mechanism-phase2-governance-bridge-distribution-2026-02-26.prd.md` | 已完成的增量专题，保留审计与互链。 |
| 签名交易授权与托管/自托管边界在哪里？ | `mainchain-token-signed-transaction-authorization-2026-03-23.prd.md` | Token 授权专题；生产 signer custody 的模块级边界仍回到 `doc/p2p/blockchain/README.md`。 |
| 不受当前实现约束的理想交易模型是什么？ | `mainchain-token-ideal-transaction-upgrade-2026-06-08.prd.md` | 目标态草案，不是当前 runtime contract。 |
| `OC -> LetAI Run OpenAPI` quota bridge 如何部署、演练与回滚？ | `mainchain-token-newapi-quota-bridge-2026-05-06.prd.md`，再读同名 `.runbook.md` | 专题规格与 operator companion；不替代 world-runtime 服务运行手册。 |

## 阅读与维护边界

- 每组 `*.prd.md`、`*.design.md`、`*.project.md` 分别保存规格、设计和执行证据；本页只承担首次分流，不复制其参数、命令或状态台账。
- 当前 P2P 模块总边界与活跃执行状态分别回到 `doc/p2p/prd.md` 与 `doc/p2p/project.md`；按精确文件名追溯使用 `doc/p2p/prd.index.md`。
- 历史 allocation、phase-2、release 与理想化交易文档仍有模块、测试或审计互链。它们不能仅因日期较早或已完成而删除；只有现行替代真值落盘、调用迁移完成并复核无活跃引用后才可退休。
- 新增 Token 专题时，先更新本页的分流/边界，再保留 `doc/p2p/prd.index.md` 的精确 triplet 行；共享目录规则以 `doc/engineering/doc-governance/doc-structure-standard.design.md` 为准。
