# P2P 移动轻客户端权威状态设计

- 对应需求文档: `doc/p2p/network/p2p-mobile-light-client-authoritative-state.prd.md`
- 对应GitHub Issue/Project task truth: GitHub Issue / GitHub Project

## 分层与数据流

1. 移动客户端持有短期 session key，提交带签名的 intent；gateway/relay 负责鉴权、限流与重放保护，但不成为权威状态来源。
2. sequencer 依 tick/seq 排序，authoritative simulator 产出 delta、`state_root` 与 `data_root`；客户端只渲染、插值预测和按权威结果纠偏。
3. 批次承诺进入 `pending`，在满足确认条件且无未解决 challenge 时进入 `confirmed`；最终化前保留 watcher 复算、challenge 与 resolve 的 fail-closed 分支。
4. 断线客户端以最近稳定 `snapshot_hash` 和 `log_cursor` 追平。hash/cursor 不连续或 reorg epoch 变化时，丢弃分叉视图，回退 stable batch 后重新取得快照与增量。
5. session revoke/rotate 改变写入授权；旧 key 的所有 intent/控制请求在 gateway 与权威入口均被拒绝。

## 状态与故障边界

- `pending/confirmed/final` 的展示和消费必须绑定同一批次状态；任何缺失的根、签名、challenge 结果或恢复证据都保持非 final，而不是乐观升级。
- challenge、resolve 和处罚语义由对应 runtime/chain authority 实现；本设计只要求移动路径按其结果 fail closed，不另造仲裁或 finality 真值。
- snapshot 或日志损坏是恢复失败信号，必须进入重新获取/人工诊断路径；不能以进程重启、复用旧 cursor 或忽略 hash mismatch 伪造追平成功。
- session key、node identity、consensus/finality signer 和 governance signer 必须保持逻辑分离。移动端或 relay 的 session key 不得取得节点治理或出块权。

## 运维与观测边界

- 可观测项至少包括 intent reject/idempotent 结果、批次根校验、challenge 状态、reorg epoch、snapshot/cursor 校验与 session revoke/rotate 拒绝原因；它们是诊断输入，不能单独推出网络健康或发布就绪。
- 真实节点、拓扑、服务合同、状态采样、restore/rollback drill 与 rollout 由 P2P node/network runbook、runtime 和 ops evidence 维护。本专题不定义生产 inventory、主备切换、SLA 或恢复演练已完成。
