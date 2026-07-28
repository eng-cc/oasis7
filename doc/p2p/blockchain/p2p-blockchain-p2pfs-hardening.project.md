# P2P / 区块链签名策略与可恢复性硬化项目记录

- 对应需求文档：`doc/p2p/blockchain/p2p-blockchain-p2pfs-hardening.prd.md`
- 对应设计文档：`doc/p2p/blockchain/p2p-blockchain-p2pfs-hardening.design.md`

## 任务拆解与历史完成记录

| 历史任务 | PRD-ID | 状态 | 日期 | 证据主题 |
| --- | --- | --- | --- | --- |
| HP2-0..3 | `PRD-P2P-MIG-046` | completed | 2026-02-16 | PoS gossip、状态持久化、重启续跑。 |
| HP3-0..3 | `PRD-P2P-MIG-047` | completed | 2026-02-17 | Action/Head 双栈签名与 sequencer 接线。 |
| HP4-0..3 | `PRD-P2P-MIG-048` | completed | 2026-02-17 | membership 双栈/keyring/publish-sync。 |
| HP5-0..2 | `PRD-P2P-MIG-049` | completed | 2026-02-17 | membership signer allowlist 策略。 |
| HP6-0..2 | `PRD-P2P-MIG-050` | completed | 2026-02-17 | membership policy 规范化与 fail-fast。 |
| HP7-0..2 | `PRD-P2P-MIG-051` | completed | 2026-02-17 | sequencer allowlist 规范化与 fail-fast。 |
| HP8-0..2 | `PRD-P2P-MIG-052` | completed | 2026-02-17 | 共享 signer policy 工具与回归。 |
| PRG-1..4 | `PRD-P2P-MIG-058` | historical-completed | 2026-02-18 | production-grade blockchain/P2PFS 路线、链式哈希与签名结算的阶段 provenance。 |
| PRG-B | `PRD-P2P-MIG-064` | historical-completed | 2026-02-19 | commit-execution context/result 与 snapshot 接线；旧 viewer-live/bridge 入口已 superseded。 |
| PRG-C | `PRD-P2P-MIG-065` | retired-future-gap | 2026-02-19 | 跨节点 DistFS challenge/proof network 仅为历史目标，当前未形成 network driver/topic/envelope。 |

## 依赖

- 当前行为锚点：PRD 所列 startup-reconcile、signature、membership、sequencer 源码与单元测试。
- 系统级安全/readiness 边界：`doc/p2p/blockchain/p2p-mainnet-security-governance-readiness.prd.md`。
- generic replication security authority consolidation 已吸收 2026-02-23 guard 原子性、writer/fetch 鉴权、有界队列与 signed restore 合同；不改变 production custody/mainnet 判定。
- 节点 inventory、health/readiness、state-sync/restore、upgrade/rollback 的运行面证据：对应 runbook 与 `doc/testing/evidence/`。

## 状态

- Phase 2–8 的历史任务均已完成；该状态仅保留工程 provenance，不构成当前 production、recovery 或 release readiness。
- 旧 Phase 2 损坏状态 fallback 及 distributed-runtime 的 HMAC-only 描述已标记为 superseded；当前合同为损坏状态 fail-closed、缺失状态才允许默认初始化。
- production-grade roadmap 与 Phase B/Phase C 独立三件套已在 2026-07-28 归并。PRG-C 的历史完成标签不代表当前实现完成；跨节点 challenge/proof networking 仍需新 runtime + blockchain ops + QA 专题才能重新进入交付。
- 此处记录的是历史工程完成，不是当前生产/恢复/发布 gate。节点 inventory、health/readiness 采样、state-sync/restore drill、升级或 rollback 证据需要进入正式 runbook 和 `doc/testing/evidence/`。
- 非 readiness：production keystore/signer custody、governance signer externalization、genesis binding 与 ceremony/QA gate 仍由 mainnet readiness 专题追踪；不得由本记录推导 mainnet-grade。
