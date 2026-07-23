# P2P / 区块链签名策略与可恢复性硬化

> 历史整合说明：本专题整合 2026-02-16 至 2026-02-17 的 `blockchain-p2pfs-hardening-phase2` 至 `phase8`。它是这些已完成阶段的专业权威与追溯入口；阶段完成记录不构成当前 mainnet、生产恢复或 release readiness 结论。

- 对应设计文档：`doc/p2p/blockchain/p2p-blockchain-p2pfs-hardening.design.md`
- 对应项目管理文档：`doc/p2p/blockchain/p2p-blockchain-p2pfs-hardening.project.md`

## 1. 目标

本专题固定 Node PoS、membership 与 sequencer 的签名兼容、信任边界和状态恢复合同：在不改既有 `signature` 线协议字段的前提下，保留 HMAC 兼容路径，同时为 ed25519、keyring 和规范化 allowlist 提供 fail-fast 策略。

## 2. 范围

这不是生产 signer custody、CA/证书链、HSM/KMS、多签、密钥轮换审批平台或创世 ceremony 的完成声明。当前系统级安全/readiness 仍以 `p2p-mainnet-grade-readiness-hardening-2026-03-23.prd.md` 的 `not_mainnet_grade` 边界为准。

## 3. 接口 / 数据与当前运行合同

### Node PoS 状态与启动恢复

- 状态文件为 `<replication_root>/node_pos_state.json`，保存 next/committed height、slot、广播高度与 execution binding 等恢复所需状态。
- **缺失状态文件**本身允许默认初始化；没有可用 execution latest 或 latest 不属于当前 world 时，不创建伪造状态。
- **有效但落后的同 world 状态**可由 `execution_records/latest.json` 向前 reconciliation：最新 execution binding 完整时，committed/network height 与后续高度被原子更新。
- **损坏、不可读或不可解析的状态文件**为 fail-closed 错误，不能把重启、默认初始化或覆盖写入当作恢复替代。必须先保全证据并按运行手册/运行时恢复流程处理。
- 上述合同的验证锚点为 `crates/oasis7/src/bin/oasis7_chain_runtime/startup_reconcile.rs` 及其单元测试；Phase 2 的“损坏文件回退默认启动”表述仅是历史设计意图，已被当前行为取代。

### Action / Head 签名

- ed25519 签名串固定为 `ed25519:v1:<public_key_hex>:<signature_hex>`；公钥为 32-byte hex，签名字节为 64-byte hex；签名载荷为清空 `signature` 字段后的 canonical CBOR。
- `ActionEnvelope` 与 `WorldHeadAnnounce` 保持既有字段格式。`SequencerMainloopConfig` 可同时配置 HMAC 和 ed25519；迁移期间不应把历史 HMAC 路径错误描述为已删除。
- 当 `require_action_signature=true` 时，配置必须具备 HMAC signer 或非空的 action signer allowlist；否则配置校验 fail-fast。无 allowlist 时保持既有兼容语义，不得把它误写成无条件 ed25519 强制。

### Membership 签名、keyring 与恢复策略

- `MembershipDirectorySigner` 支持 HMAC 与 ed25519；snapshot/revocation 的签发和验签通过 keyring 支持 active key、多 key 验签、`signature_key_id`、吊销 key 拒绝及轮换窗口。
- restore/revocation policy 的 `accepted_signature_signer_public_keys` 为空时，不额外按 signer 公钥过滤；非空时签名必须为 `ed25519:v1` 且规范化后的 signer 公钥必须命中 allowlist，该检查只是身份过滤。已有 signature 且调用方提供 signer/keyring 时继续验签；已有 signature 且 `require_signature`、`require_signature_key_id` 或 accepted key-ID 策略需要 verifier 而未提供时必须 fail-closed；`require_signature=true` 则单独拒绝缺失 signature 的对象。
- 公钥与 allowlist 统一 trim、非空检查、hex/32-byte 检查、小写规范化和去重；重复、空值或非法值在加载/验证阶段 fail-fast，错误保留字段或索引定位信息。membership 与 sequencer 必须使用同一比较语义。
- key-ID 信任、吊销同步、requester 信任与签名策略为组合边界；allowlist 不是对 keyring、key-ID 或 revocation 校验的替代。

## 4. 里程碑与历史阶段证据交叉表

| 历史阶段 | PRD-ID | 完成日期 | 保留的结果/决策 |
| --- | --- | --- | --- |
| Phase 2 | `PRD-P2P-MIG-046` | 2026-02-16 | gossip 签名闭环、PoS 状态持久化与重启续跑；损坏文件 fallback 仅为已取代的历史表述。 |
| Phase 3 | `PRD-P2P-MIG-047` | 2026-02-17 | Action/Head ed25519 signer、canonical 签名串、HMAC 双栈兼容与 sequencer 接线。 |
| Phase 4 | `PRD-P2P-MIG-048` | 2026-02-17 | membership snapshot/revocation 双栈 signer、keyring、publish/sync 接线。 |
| Phase 5 | `PRD-P2P-MIG-049` | 2026-02-17 | membership restore/revocation signer-public-key allowlist 的可选强制边界。 |
| Phase 6 | `PRD-P2P-MIG-050` | 2026-02-17 | membership policy 规范化、误配显式失败和大小写无关比较。 |
| Phase 7 | `PRD-P2P-MIG-051` | 2026-02-17 | sequencer action signer allowlist 规范化和配置 fail-fast。 |
| Phase 8 | `PRD-P2P-MIG-052` | 2026-02-17 | 共享 ed25519 规范化工具，以及 membership/sequencer/signature 一致接线。 |

每一阶段的 HP 任务全部完成仅证明该阶段的实现/回归当时已收口；它不覆盖部署 inventory、节点拓扑、restore drill、生产密钥托管、治理 signer 外部化或 QA release verdict。

## 5. 风险、验证与运维要求

- 恢复改动至少覆盖：缺失状态、有效陈旧状态向前 reconciliation、异 world/latest 缺 binding 不写入、损坏文件 fail-closed。
- 签名策略改动至少覆盖：HMAC 兼容、ed25519 action/head 验签、membership snapshot/revocation keyring、allowlist 为空/命中/不命中、大小写规范化、重复和非法配置。
- 运维诊断必须保留稳定的拒绝原因；不得以重启掩盖状态损坏、配置误配或签名/信任根不一致。
- 真实环境的 topology、健康基线、升级、rollback、state-sync/restore drill 仍须由相应 runbook 与 `doc/testing/evidence/` 记录提供证据。

## 6. 非目标与非 readiness

- 不新增线协议字段、密码学算法、CA/证书链、HSM/KMS、远程 signer、多签或全面身份治理。
- 不把 local/config signer、HMAC/ed25519 原语或本专题历史完成状态宣称为 production custody、mainnet-grade 或 mint readiness。
- 不替代 runtime 对恢复机制实现的权威、QA 的放行结论或 LiveOps 的对外口径。
