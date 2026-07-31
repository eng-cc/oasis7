# P2P / 区块链签名策略与可恢复性硬化设计

- 对应需求文档：`doc/p2p/blockchain/p2p-blockchain-p2pfs-hardening.prd.md`
- 对应项目管理文档：GitHub Issue / GitHub Project

## 设计定位

本设计把 Phase 2–8 的分阶段 hardening 收为三个运行面：Node PoS state recovery、Action/Head signer policy、membership signer/keyring/recovery policy。当前代码与测试是行为真值；历史阶段文字只保留 provenance，不可覆盖当前 fail-closed 恢复合同。

2026-02-23 的 generic replication security hardening 同样已吸收：guard 在验证和写盘成功前不提交；writer/fetch 鉴权、bounded subscription 与签名优先 membership restore 遵循 PRD 的当前合同。该吸收不将这些局部防护提升为 production custody 或 mainnet readiness。

## 结构与边界

- 恢复层：仅缺失状态可默认初始化；有效陈旧同 world 状态可由 execution latest 前推；损坏、不可读或不可解析状态必须停止并暴露错误。
- Action/Head 层：HMAC 与 ed25519 可双栈存在；ed25519 使用 canonical 格式和 payload；强制动作签名需要 HMAC 或 allowlist 支撑。
- Membership 层：keyring 处理 active/multi-key、key-ID 和 revoked key；allowlist 仅在非空时强制 ed25519 signer 匹配，且不绕过 keyring、key-ID、revocation 或 requester policy。
- 公钥策略层：共享 32-byte hex 规范化、小写比较、重复拒绝和字段级错误，以避免 membership 与 sequencer 漂移。
- Replication guard 层：拒绝空 world/writer 和零 epoch/sequence；同 writer/epoch 的 sequence 严格递增，epoch 或 writer 切换从 sequence 1 开始且不允许 epoch 回退。
- Replay 层：逐条调用与在线路径相同的 `apply_replication_record`；只有 hash 校验和 store 写入成功后才提交 guard，首个失败立即终止并保留失败前的确定状态。
- Commit/execution 层：ordered actions 与 `action_root` 经 commit context 进入 node execution；result 的 block/state root 和持久 snapshot 字段用于恢复兼容。旧 viewer-live driver 与 bridge-default 接线只作历史 provenance。
- Block hash 层：execution bridge 对 canonical-CBOR `oasis7_proto::distributed::WorldBlock` 计算 commitment；现行字段以该类型的 `world_id/height/prev_block_hash/action_root/event_root/state_root/journal_ref/snapshot_ref/receipts_root/proposer_id/timestamp_ms/signature` 为准，旧路线图不能反向新增另一套 payload schema。
- DistFS proof 层：当前 gate 使用 `ReplicationNetworkEndpoint` 做 provider/DHT/fetch-route 探测，并区分 hard failure 与有界 degraded/fallback；历史 Phase C 的专用 request/proof envelope、challenge topics 与 challenge driver 未形成当前合同。

## 约束

- 不改变 `signature` 字段或新增 wire schema；不把兼容窗口误写为算法/协议切换完成。
- 不把 restore/restart 当成对损坏或分歧的自动修复。运维动作必须先保全证据并定位 code/config/deployment 根因。
- 本设计不提供生产 custody、拓扑、部署、rollback 或 release pass；这些分别由现行 runbook、证据与对应专业角色收口。

## 验证入口

- `crates/oasis7/src/bin/oasis7_chain_runtime/startup_reconcile.rs`
- `crates/oasis7_consensus/src/signature.rs`
- `crates/oasis7_consensus/src/ed25519_signer_policy.rs`
- `crates/oasis7_consensus/src/membership_logic.rs`
- `crates/oasis7_consensus/src/sequencer_mainloop.rs`
- `crates/oasis7_distfs/src/replication.rs`

## 历史演进

`MIG-046` 至 `MIG-052` 的详细阶段、决策和完成日期由同名 PRD 的交叉表保存。Phase 8 的共享化取代了 Phase 6/7 的重复实现，不删除其兼容边界或历史验证证据。

`MIG-058`、`MIG-064` 与 `MIG-065` 分别保存 production-grade 路线、Phase B commit-execution 与 Phase C challenge-network 的压缩历史；Phase C 是未形成当前能力的历史目标，而非已上线网络证明。
