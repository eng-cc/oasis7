# DistFS 生产化硬化项目记录

- 对应需求文档：`doc/p2p/distfs/distfs-production-hardening.prd.md`
- 对应设计文档：`doc/p2p/distfs/distfs-production-hardening.design.md`

## 任务拆解与历史任务追溯

| 历史任务 | PRD-ID | 状态 | 保留证据主题 |
| --- | --- | --- | --- |
| DPH1-1..5 | PRD-P2P-MIG-067 | completed | 本地 CAS、索引审计、回收、manifest 和 `oasis7_distfs` 回归。 |
| DPH2-1..5 | PRD-P2P-MIG-068 | completed | 本地 challenge request/receipt/统计。 |
| DPH3-1..4 | PRD-P2P-MIG-069 | completed | probe 与 reward runtime 接线。 |
| DPH4-1..4 | PRD-P2P-MIG-070 | completed | cursor state、原子持久化和兼容默认化。 |
| DPH5-1..4 | PRD-P2P-MIG-071 | completed | chain-runtime 配置、aggregate report 和回归。 |
| DPH6-1..4 | PRD-P2P-MIG-072 | completed | adaptive backoff、预算和状态兼容。 |
| DPH7-1..4 | PRD-P2P-MIG-073 | completed | reason-aware 调度与配置治理。 |
| DPH8-1..4 | PRD-P2P-MIG-074 | completed | adaptive multiplier CLI 接线与校验。 |
| DPH9-1..4 | PRD-P2P-MIG-075 | completed | backoff decision state 与兼容回归。 |
| DFIO-1..4 | PRD-P2P-MIG-080 | completed / absorbed | FileStore、路径净化、CAS-first 原子索引与当前 blob 回收边界。 |

## 依赖与当前锚点

- 当前实现锚点为 PRD 所列 `oasis7_distfs` 和 `oasis7_chain_runtime` 文件及其单元测试。
- 早期“CAS”不是跨进程/跨节点一致性承诺；challenge probe 不是远程或多节点 attestation。
- Phase 5 的详细 epoch-report 字段由 aggregate checks/failures/ratio 的当前合同取代；Phase 8 CLI 属于 `oasis7_chain_runtime`；Phase 9 backoff 仅为本地 probe-state，并非对外 metrics telemetry。
- malformed/unreadable probe state 的 warning + default-state 行为仅维持 local scheduler best-effort continuity，不能被叙述为 checkpoint、state-sync 或生产恢复成功。
- builtin DistFS storage/API 的 artifact materialization 合同已迁入 `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.*`；本专题只保留通用 `FileStore` 与本地索引合同。

## 状态与非 readiness

- 所有 DPH 历史任务 completed 仅保留工程 provenance；不构成真实部署、topology、health baseline、upgrade/rollback、state-sync/restore drill、public_testnet 或 release verdict。
- 现行分布式韧性/NodeRuntime 边界见 `distfs-distributed-resilience`；真实环境、S9A 分层和正式 evidence/runbook 才能提供运行面结论。
- 后续若修改本专题运行时合同，应同时验证本地 `oasis7_distfs` 回归并按影响范围补 S9A 证据，不能以本项目历史完成态替代。
