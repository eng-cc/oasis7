# 生产级区块链 + P2PFS Phase C DistFS 证明网络设计

- 对应需求文档: `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phasec-distfs-proof-network.prd.md`
- 对应项目管理文档: `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phasec-distfs-proof-network.project.md`

> 现行状态：本文是历史 P2PFS 路线图的 Phase C 设计追溯。当前链上大世界状态底座的聚合设计入口、测试层级与 claim boundary 以 `doc/p2p/design.md`、`testing-manual.md#s9a链上大世界状态底座自闭环` 和 `PRD-P2P-031` 为准；本文不单独证明 S9A `module_full`、`integration_required` 或 `release_full`。

## 1. 设计定位
定义 DistFS 证明网络的节点角色、proof 传播、验证链路与共识接入方式。

## 2. 设计结构
- 证明对象：把 DistFS 数据证明抽象为可验证 payload。
- 网络路径：proof 生成、传播、验证与归档的主链路。
- 共识接入：证明结果如何绑定到区块链/P2PFS 主链状态。

## 3. 关键接口 / 入口
- proof payload / verification result / network message
- 区块链与 DistFS 之间的校验桥接接口
- 归档与观测点

## 4. 约束与边界
- 证明网络与执行网络职责分离。
- proof 校验失败必须显式暴露，不能 silent drop。
- 网络协议变更需保持向后兼容策略。

## 5. 设计演进计划
- 先补齐专题 Design 与互链。
- 再按 Project 任务拆解推进实现与测试闭环。
