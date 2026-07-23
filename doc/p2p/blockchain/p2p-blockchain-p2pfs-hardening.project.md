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

## 依赖

- 当前行为锚点：PRD 所列 startup-reconcile、signature、membership、sequencer 源码与单元测试。
- 系统级安全/readiness 边界：`doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.prd.md`。
- 节点 inventory、health/readiness、state-sync/restore、upgrade/rollback 的运行面证据：对应 runbook 与 `doc/testing/evidence/`。

## 状态

- Phase 2–8 的历史任务均已完成；该状态仅保留工程 provenance，不构成当前 production、recovery 或 release readiness。
- 旧 Phase 2 损坏状态 fallback 及 distributed-runtime 的 HMAC-only 描述已标记为 superseded；当前合同为损坏状态 fail-closed、缺失状态才允许默认初始化。
- 此处记录的是历史工程完成，不是当前生产/恢复/发布 gate。节点 inventory、health/readiness 采样、state-sync/restore drill、升级或 rollback 证据需要进入正式 runbook 和 `doc/testing/evidence/`。
- 非 readiness：production keystore/signer custody、governance signer externalization、genesis binding 与 ceremony/QA gate 仍由 mainnet readiness 专题追踪；不得由本记录推导 mainnet-grade。
