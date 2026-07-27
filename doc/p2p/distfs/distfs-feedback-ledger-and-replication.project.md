# DistFS 反馈账本与复制（项目与历史追溯）

- 对应需求文档: `doc/p2p/distfs/distfs-feedback-ledger-and-replication.prd.md`
- 对应设计文档: `doc/p2p/distfs/distfs-feedback-ledger-and-replication.design.md`

## 任务拆解

| 历史专题 | 已完成范围 | 当前归属 |
| --- | --- | --- |
| 2026-03 公开账本源三件套 | append-only root/event、Ed25519、nonce、audit 限流与公开读 | 本专题的账本与 anti-abuse 合同。 |
| 2026-03 复制桥接源三件套 | announce/outbox、hash fetch、复制 ingest 与幂等 | 本专题的复制数据流。 |
| 2026-03 NodeRuntime 接线源三件套 | `NodeFeedbackP2pConfig`、tick drain/publish、runtime 接线 | 本专题的配置、lane 前提与有界运行面。 |
| 2026-03 chain-runtime replication autowire | binary pre-start 默认挂载、effective-no-bootstrap 本地 fallback、显式 topology fail-closed | 本专题的 runtime wiring 与单机/多节点边界。 |

三个源三件套已删除；completed provenance 仅保留在 Git 与 `.pm` task evidence。不得将历史完成范围重新列为 active root entry，或从其完成状态推断当前部署/readiness。

## 依赖

- `crates/oasis7_distfs/src/{feedback.rs,feedback/replication.rs,feedback_p2p.rs}`
- `crates/oasis7_node/src/{feedback_runtime.rs,node_runtime_core.rs,lib.rs,types.rs}`
- 既有 replication config/network、BlobState lane policy，以及其签名/allowlist 合同。
- `crates/oasis7/src/bin/oasis7_chain_runtime.rs` 与 `crates/oasis7_node/src/libp2p_replication_network/` 的默认挂载、peer admission 与 fallback 合同。

## 状态

历史实现范围已完成；本三件套是该范围的当前稳定专业 authority。它只记录当前语义和验证责任，不新增发布状态或就绪承诺。

## 当前验证责任

- `env -u RUSTC_WRAPPER cargo test -p oasis7_distfs --lib`：反馈账本、签名、nonce、复制 ingest 和 announce 的受影响回归。
- `env -u RUSTC_WRAPPER cargo test -p oasis7_node --lib feedback`，以及涉及 lane/replication 启动合同的定向 node 回归：配置前提、BlobState publish+subscribe gate、有界 announce 与局部失败行为。
- binary/runtime wiring 变化需覆盖 pre-start attach、effective peer list、no-peer fallback 与显式 topology `NetworkProtocolUnavailable`；local fallback 通过不能替代多节点运行证据。
- 文档变更运行 `./scripts/doc-governance-check.sh && ./scripts/readme-link-check.sh && git diff --check`。

这些命令是模块级维护证据，不是生产健康基线、恢复演练、QA 放行或公开网络承诺。涉及实际节点角色、lane、签名/allowlist 或恢复语义的改动需分别取得 runtime、ops 与 QA 的对应证据。
