# README P1 缺口收口：分布式网络主路径生产化（项目管理文档）

- 对应设计文档: `doc/p2p/network/readme-p1-network-production-hardening.design.md`
- 对应需求文档: `doc/p2p/network/readme-p1-network-production-hardening.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）
- [x] T0 (PRD-P2P-MIG-086)：输出设计文档（`doc/p2p/network/readme-p1-network-production-hardening.prd.md`）与项目管理文档（本文件）。
- [x] T1 (PRD-P2P-MIG-086)：实现 libp2p request 多 peer 轮换重试 + 无 peer 可控回退策略，并补测试。
- [x] T2 (PRD-P2P-MIG-086)：实现 node 共识消息 libp2p pubsub 主路径（ingest/broadcast）并补测试。
- [x] T3 (PRD-P2P-MIG-086)：执行 `env -u RUSTC_WRAPPER cargo test -p oasis7_node` + `env -u RUSTC_WRAPPER cargo check`，回写文档/devlog 收口。
- [x] node-network-authority-consolidation (PRD-P2P-MIG-105-001) [test_tier_required]: 承接原 net-stack-unification-readme 专题 ID，冻结 native `oasis7_net` 单向依赖、公开 API、peer 轮换/失败分类、默认无 peer 错误及显式本地 fallback。 Trace: #2652 (task_33241c6a236149efbe1790f03e1cc1f6)
- [ ] node-wasm-libp2p-compile-guard (PRD-P2P-MIG-104-001) [test_tier_required]: 承接原 wasm32-libp2p-compile-guard 专题 ID，以 `env -u RUSTC_WRAPPER cargo check -p oasis7_node --target wasm32-unknown-unknown --features libp2p` 作为 wasm32 API/target cfg 编译护栏，并消除当前 `getrandom 0.2` 缺少 `js` feature 的依赖闭包阻断。 Trace: #2652 (task_33241c6a236149efbe1790f03e1cc1f6)

## 依赖
- T2 依赖 T1（先稳定网络请求层，再接共识主循环）。
- T3 依赖 T1/T2 全部完成。

## 状态
- 最近更新：2026-07-27（专业权威合并）
- 当前阶段：网络主路径与专业权威合并已完成（T0~T4）；package-level wasm32 编译绿灯仍未完成。
- 阻塞项：2026-07-27 新鲜运行 wasm32 编译护栏时，依赖闭包中的 `getrandom 0.2` 因未启用 `js` feature 失败。
- 下一步：由独立实现任务修复 wasm32 依赖 feature 闭包后，重跑 T5 命令；本次文档迁移不把历史完成态误报为当前绿灯。
