# 生产级区块链 + P2PFS 路线图设计

- 对应需求文档: `doc/p2p/blockchain/production-grade-blockchain-p2pfs-roadmap.prd.md`
- 对应项目管理文档: `doc/p2p/blockchain/production-grade-blockchain-p2pfs-roadmap.project.md`

> 现行状态：本文保留为历史 P2PFS 路线图的设计追溯。当前链上大世界状态底座的聚合设计入口、测试层级与 claim boundary 以 `doc/p2p/design.md`、`testing-manual.md#s9a链上大世界状态底座自闭环` 和 `PRD-P2P-031` 为准；本文不单独证明 S9A `module_full`、`integration_required` 或 `release_full`。

## 1. 设计定位
定义把“可演示的区块链 + P2PFS + 节点收益”推进到可生产部署形态的路线图，优先收敛共识哈希、奖励结算签名和执行链路三类高风险缺口。

## 2. 设计结构
- 链式哈希层：把 `block_hash` 从占位字符串升级为带 `parent_hash` 的链式哈希。
- 结算签名层：为 `RewardSettlementEnvelope` 增加独立传输签名和消费前置验签。
- 执行收敛层：记录未来共识内生执行与 DistFS 证明网络化等后续阶段。
- 路线图分层：明确本轮可交付与后续演进边界。

## 3. 关键接口 / 入口
- `BlockHashPayload`
- `RewardSettlementEnvelope`
- `parent_block_hash`
- 路线图里程碑 PRG-M0~M6

## 4. 约束与边界
- 本轮只落实链式哈希和结算 envelope 签名，不进入共识内生执行。
- 需求侧交易化支付市场已明确移除，不再纳入路线图主线。
- 后续阶段跨 crate 同步演进需单独管理，不在本次设计里强行展开实现细节。
- 路线图文档要同时服务当前落地与长期方向对齐。

## 5. 设计演进计划
- 先落链式 `block_hash` 和持久化兼容。
- 再补 RewardSettlementEnvelope 传输签名。
- 最后把后续共识内生执行与证明网络化留作后续里程碑。
