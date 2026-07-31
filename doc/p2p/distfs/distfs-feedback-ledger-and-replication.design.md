# DistFS 反馈账本与复制设计

- 对应需求文档: `doc/p2p/distfs/distfs-feedback-ledger-and-replication.prd.md`
- 对应项目管理文档: GitHub Issue / GitHub Project

## 设计定位

以本地 append-only feedback store 为权威记录层，以轻量 gossip announce 作为发现提示，以受 replication 合同保护的 fetch-blob 作为数据传输层。announce 本身不承载内容，也不构成共识消息。

## 数据流

1. 本地 submit/append/tombstone 验证 Ed25519、expiry、nonce、大小与基础限流，写 root/event、nonce 和 audit，并生成 mutation receipt。
2. receipt 生成只含元数据和 blob reference 的 announce，进入有界 outbox。
3. 具备 replication 与 BlobState publish+subscribe 权限的 NodeRuntime 在 tick 中按上限发布/消费 announce。
4. 入站 announce 通过 replication fetch-blob 定向取回 blob；仅在 payload 存在且 BLAKE3 hash 与 reference 相同后解析并调用 replicated ingest。
5. replicated ingest 重放同一 append-only 和签名约束，并按 `feedback_id + event_id` 幂等；局部错误留在 runtime 错误观测，不中断 tick。

## 运维与安全边界

- `feedback_p2p` 是显式配置能力：缺 replication config、network endpoint 或 BlobState lane 权限即拒绝启动。operator 应修正角色/lanes/replication 合同，不以重启替代配置修复。
- chain-runtime binary 在启动 NodeRuntime 前自动挂载默认 replication network 和 maintenance DHT handle；默认 listen 使用 loopback ephemeral 地址。effective bootstrap peers 优先使用非空的显式 CLI 列表，否则使用已加载的 network-tier manifest 列表；仅当最终 effective 列表为空时，才启用 no-peer 本地 handler fallback。
- effective peer 列表非空时，transport 必须走真实 peer/admission 路径；没有 connected 或 admissible peer 时返回 `NetworkProtocolUnavailable`。显式 topology 不允许以本地 handler 掩盖连接、角色或 lane 缺陷。
- 入站和出站都受每 tick 正上限控制；待发队列的容量与出站上限关联。队列饱和、fetch 拒绝、hash 不匹配或签名失败均应作为可诊断失败处理，不能写入未经验证的记录。
- replication 的 server/request signing、allowlist 和 lane policy 是 fetch 授权边界；本设计只复用，不能以 feedback 的作者签名替代节点间 fetch 授权。
- audit/IP/public-key 限流仅是基础 anti-abuse，不是内容审核、隐私保护或生产容量保障。

## 非承诺

该设计不改变 consensus/runtime 状态机，不给反馈事件赋予 finality，不定义 checkpoint/restore，也不产生任何部署、readiness 或 release 结论。默认 no-peer fallback 只是单机 wiring；跨节点复制的成功也只证明受测路径在给定角色、lane、peer 与签名配置下可用。
