# p2p PRD Project

审计轮次: 14

## 任务拆解（活跃面）
- 当前主线仍是非全公网 P2P substrate、reachability/role policy 与 claims boundary；具体执行优先级以 `## 状态` 和对应 topic project 为准。
- hosted world / hosted account、public testnet、bridge/newapi、network tier、主链 token 与 faucet/mint-ready 细项均已有独立 topic project；本页不再逐条复述每条子线的完成流水。

### 最近完成（保留一跳 Trace）
- [x] iroh-inspired-reachability-evidence-observability (PRD-P2P-024/025/030) [test_tier_required]: 落地 peer reachability contract、path behavior matrix taxonomy 与 bounded status/triad observability 三项，不引入 iroh 依赖、不替换 libp2p。 Trace: .pm/tasks/task_43a21163092541809de36036403d7c97.yaml
- [x] module-project-log-slimming (PRD-ENGINEERING-030) [test_tier_required]: 压缩 p2p 主项目页历史流水为当前/最近任务索引与历史追溯入口，保留模块判断、状态和一跳 task trace。 Trace: .pm/tasks/task_49ef9270afc646d98d4a8386c0888eab.yaml
- [x] p2p-network-runtime-hardening (PRD-P2P-001/003) [test_tier_required]: 收口 libp2p gossip publish 静默失败与 node replication fallback 分类分叉。 Trace: .pm/tasks/task_4d597c77a31b4411864f998159d8d5ec.yaml
- [x] subscribe-ack-udp-gossip-hardening (PRD-P2P-001/003) [test_tier_required]: 收口 `libp2p subscribe` dead-subscription 假成功与 UDP gossip datagram 语义歧义。 Trace: .pm/tasks/task_b518025590ae4e998726066eb862e17c.yaml
- [x] issue-182-replication-lib-regressions (PRD-P2P-001/003) [test_tier_required]: 修复 GitHub issue `#182` 中剩余 `oasis7_node --lib` regression。 Trace: .pm/tasks/task_b4c03075497348cfbcf30fcb4c970226.yaml
- [x] testnet-routing-peer-record-bootstrap (PRD-P2P-001/003) [test_tier_required]: 修复 reset public testnet observer bootstrap 窗口的 peer-record 交换触发。 Trace: .pm/tasks/task_f81d3e661b7048d8b2fe6987a544a368.yaml
- [x] testnet-auto-high-state-sync (PRD-P2P-001/003) [test_tier_required]: 允许 cold observer 自动探测 retained execution checkpoint。 Trace: .pm/tasks/task_761375d25bc24fe59147a853e8c8acb0.yaml
- [x] testnet-high-state-peer-retry (PRD-P2P-001/003) [test_tier_required]: 支持 observer 从 storage/full-storage peer 获得 checkpoint descriptor。 Trace: .pm/tasks/task_96c772c830e043f9b1e40b03e6f73d38.yaml
- [x] testnet-storage-challenge-degraded-readiness (PRD-P2P-001/003/028) [test_tier_required]: 修复 public testnet storage challenge 在 provider/DHT/fetch route retryable 不可用时误 hard-block sequencer 的问题。 Trace: .pm/tasks/task_8d92c7fdfbc742e3866ef1162faedd66.yaml
- [x] p2p-evidence-doc-cleanup (PRD-P2P-001/003) [test_tier_required]: 清理无外部引用的 generated shared-network gate 中间快照并刷新 p2p 首读证据链。 Trace: .pm/tasks/task_1f333657aba5468aa74bd2435fcbbbcf.yaml
- [x] p2p-peer-head-readiness-doc-cleanup (PRD-P2P-001/003) [test_tier_required]: 删除已失去当前权威入口职责的 peer-head readiness 历史叙事 design 文档。 Trace: .pm/tasks/task_56f67be67a5a43c09027cb224f1416ad.yaml
- [x] p2p-stale-evidence-reference-cleanup (PRD-P2P-001/003/028) [test_tier_required]: 删除被新 evidence/docs 取代的 stale evidence 引用。 Trace: .pm/tasks/task_deab30d82bd54824b5be64fac1b2c961.yaml

### 历史压缩索引
- PoS 时间锚定、viewer-live 旧入口删除、legacy path cleanup、early p2p schema/acceptance 历史：回看 `doc/p2p/prd.index.md` 与 `.pm/tasks/*.execution.md`。
- Mainnet-grade、signer custody、governance signer、genesis ceremony、claims policy、token/Oasis Coin 与 bridge/newapi 历史：回看 `doc/p2p/blockchain/`、`doc/p2p/token/` 与相关 topic project。
- Hosted world/account、public testnet、network tier、real-env triad、observability、state-sync 与 recovery guardrails 历史：回看 `doc/p2p/node/`、`doc/p2p/network/`、`doc/testing/evidence/` 与对应 task trace。
- 本主项目页只维护当前/最近任务索引；完整执行拆解、产物文件和验收命令以 topic project、runbook、testing evidence 与 task execution log 为准。

## 依赖
- 模块设计总览：`doc/p2p/design.md`
- doc/p2p/prd.index.md
- `doc/p2p/blockchain/production-grade-blockchain-p2pfs-roadmap.prd.md`
- `doc/p2p/distributed/distributed-hard-split-phase7.prd.md`
- `doc/p2p/network/p2p-mobile-light-client-authoritative-state-2026-03-06.prd.md`
- `doc/p2p/node/node-pos-slot-clock-real-time-2026-03-07.prd.md`
- `doc/p2p/node/node-pos-subslot-tick-pacing-2026-03-07.prd.md`
- `doc/p2p/node/node-pos-time-anchor-control-plane-alignment-2026-03-07.prd.md`
- `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.prd.md`
- `doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.prd.md`
- `doc/p2p/blockchain/p2p-mainnet-crypto-security-baseline-2026-03-23.prd.md`
- `doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.prd.md`
- `doc/p2p/blockchain/p2p-genesis-freeze-ceremony-qa-gate-2026-03-23.prd.md`
- `doc/p2p/blockchain/p2p-mainnet-public-claims-policy-2026-03-23.prd.md`
- `doc/p2p/blockchain/p2p-mainstream-public-chain-testing-benchmark-2026-03-24.prd.md`
- `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.prd.md`
- `doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.prd.md`

## 状态
- 当前状态: active（ROUND-027）
- 当前活跃子线: hosted world / hosted account、public testnet、bridge/newapi、network tier 和主链代币口径都已有独立 topic `*.project.md`；本页状态区不再逐条复述各子线最近完成项。
- Next: 保持 p2p 主项目页作为模块级 active verdict 与 trace hub；新增长流水应进入对应 topic project 或 `.pm/tasks/*.execution.md`，本页只保留近期一跳 Trace。
- 最新补充（2026-05-31 / mainstream sync recovery parity）: 平滑升级不再只依赖 systemd active。`scripts/p2p-upgrade-preflight.sh` 会检查 status 端点的 replication cursor、peer-head freshness、gap-sync blocked 和 policy lag，并用 trusted checkpoint / state-sync bundle 门槛保护落后节点恢复。当前代码支持 snapshot-only state-sync bundle 作为最小恢复输入：`--require-state-sync-bundle` 要求 bundle manifest、bundle dir、snapshot path/sha256 与 state_root；journal 仅在 manifest 提供 `journal_path` 时参与校验，不再作为最小 bundle 的必需字段。完整 seed/restore artifact 的 snapshot/journal/blob closure 仍属于更严格的 operator 恢复路径。Trace: `.pm/tasks/task_9051849e0c92424bb7f0ca972a7935cc.yaml`。
- 最新补充（2026-06-01 / mainstream sync recovery parity execution guardrails）: testnet 演练后继续补齐恢复执行层：restore script 执行前会 re-check 必需工具链、snapshot/journal/chunk sha256 与 chunks root，捕获 `systemctl show/status` 服务状态快照，备份 source/backup sha256 与 `path/type/size/mode/uid/gid` metadata manifest 并自动比对；生成 restore command plan 前会拒绝 shell-unsafe service name、data/backup/bundle 路径及 snapshot/journal/chunk 相对路径。fake execution drill 覆盖成功恢复、snapshot 篡改、chunk 篡改、restore 失败自动 rollback；真实 ECS/testnet 状态仍未被替换。
- PRD 质量门状态: strict schema 已对齐（含第 6 章验证与决策记录）。
- 说明: 本文档状态区只保留 active verdict、next step 和模块级判断；更早完成态继续以任务清单、topic project、evidence 与 execution log 为准。
