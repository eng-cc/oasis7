# oasis7: Chain Runtime 反馈网络复制层自动挂载修复（2026-03-02）（项目管理）

- 对应设计文档: `doc/testing/longrun/chain-runtime-feedback-replication-network-autowire-2026-03-02.design.md`
- 对应需求文档: `doc/testing/longrun/chain-runtime-feedback-replication-network-autowire-2026-03-02.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] AUTONET-1 (PRD-TESTING-LONGRUN-AUTONET-001/003): 完成专题设计文档与项目管理文档建档。
- [x] AUTONET-2 (PRD-TESTING-LONGRUN-AUTONET-001/002): 修复 `oasis7_chain_runtime` 启动前自动挂载默认 replication network，并启用 no-peer 本地 fallback。
- [x] AUTONET-3 (PRD-TESTING-LONGRUN-AUTONET-002/003): 完成单测、`cargo check`、启动烟测与 devlog 收口。
- [x] AUTONET-4 (PRD-TESTING-004): 专题文档按 strict schema 人工重写，并切换命名到 `.prd.md/.project.md`。

## 依赖
- doc/testing/longrun/chain-runtime-feedback-replication-network-autowire-2026-03-02.prd.md
- `crates/oasis7/src/bin/oasis7_chain_runtime.rs`
- `crates/oasis7/src/bin/oasis7_chain_runtime/oasis7_chain_runtime_tests.rs`
- `doc/testing/prd.md`
- `doc/testing/project.md`
- `doc/devlog/README.md`
- `doc/devlog/README.md`

## 状态
- 更新日期：2026-03-03
- 当前阶段：已完成
- 阻塞项：无
- 下一步：无（当前专题已收口）
