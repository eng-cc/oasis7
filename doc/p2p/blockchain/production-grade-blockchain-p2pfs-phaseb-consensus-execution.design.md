# 生产级区块链 + P2PFS Phase B 共识内生执行设计

- 对应需求文档: `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phaseb-consensus-execution.prd.md`
- 对应项目管理文档: `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phaseb-consensus-execution.project.md`

> 现行状态：本文是历史 P2PFS 路线图的 Phase B 设计追溯。当前链上大世界状态底座的聚合设计入口、测试层级与 claim boundary 以 `doc/p2p/design.md`、`testing-manual.md#s9a链上大世界状态底座自闭环` 和 `PRD-P2P-031` 为准；本文不单独证明 S9A `module_full`、`integration_required` 或 `release_full`。

## 1. 设计定位
定义节点共识主循环内生执行的 hook 接口、快照扩展、执行驱动接线与兼容 fallback 方案。

## 2. 设计结构
- 节点执行驱动：在 commit 后触发执行上下文并返回 execution 结果。
- 快照层：在共识快照与持久化快照中增加 execution 高度、块哈希与状态根。
- 接线层：由 `oasis7_viewer_live` 承接执行驱动并保留旧 bridge fallback。

## 3. 关键接口 / 入口
- `NodeExecutionCommitContext` / `NodeExecutionCommitResult`
- `NodeConsensusSnapshot` / `PosNodeStateSnapshot` 扩展字段
- 节点 commit hook 与 execution driver 接口

## 4. 约束与边界
- 节点内生执行不得破坏现有报表与 CAS 目录兼容。
- 重启恢复必须保留 execution 快照字段。
- bridge 并存阶段要避免重复执行。

## 5. 设计演进计划
- 先补齐专题 Design 与互链。
- 再按 Project 任务拆解推进实现与测试闭环。
