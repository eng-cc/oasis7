# Non-Viewer 长稳运行内存安全与可追溯冷归档硬化（项目管理）

- 对应设计文档: `doc/headless-runtime/nonviewer/nonviewer-longrun-traceable-memory-archive-hardening-2026-02-23.design.md`
- 对应需求文档: `doc/headless-runtime/nonviewer/nonviewer-longrun-traceable-memory-archive-hardening-2026-02-23.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] T-MIG-20260303 (PRD-ENGINEERING-006): 逐篇阅读旧文档并完成人工重写迁移到 `.prd` 命名。
### T0 建档
- [x] 设计文档：`doc/headless-runtime/nonviewer/nonviewer-longrun-traceable-memory-archive-hardening-2026-02-23.prd.md`
- [x] 项目文档：`doc/headless-runtime/nonviewer/nonviewer-longrun-traceable-memory-archive-hardening-2026-02-23.project.md`

### T1 Node 内存边界治理（1/2/3）
- [x] `oasis7_node`：引擎 pending actions 有界化（不丢追溯）
- [x] `oasis7_node`：gossip dynamic peer TTL + 容量治理
- [x] `oasis7_node`：committed batches 热窗口有界化
- [x] 补充/更新单测

### T2 Consensus 日志与 dead-letter 冷归档（4/5）
- [x] `oasis7_consensus`：文件型 audit/alert/event 日志热窗口 + CAS 冷归档
- [x] `oasis7_consensus`：dead-letter archive 改 CAS 冷归档
- [x] 补充/更新单测

### T3 Node replication commit 冷归档（6）
- [x] `oasis7_node`：commit message 热窗口 + CAS 冷归档
- [x] `oasis7_node`：按高度读取回退到 cold refs/CAS
- [x] 补充/更新单测

### T4 收口
- [x] required-tier 回归
- [x] 设计/项目文档状态更新
- [x] `doc/devlog/README.md` 追加任务日志

## 依赖
- `crates/oasis7_node/src/node_runtime_core.rs`
- `crates/oasis7_node/src/lib.rs`
- `crates/oasis7_node/src/lib_impl_part1.rs`
- `crates/oasis7_node/src/gossip_udp.rs`
- `crates/oasis7_node/src/types.rs`
- `crates/oasis7_node/src/replication.rs`
- `crates/oasis7_consensus/src/membership_reconciliation.rs`
- `crates/oasis7_consensus/src/membership_split_part1.rs`
- `crates/oasis7_consensus/src/membership_recovery/dead_letter.rs`
- `crates/oasis7_consensus/src/membership_recovery/replay_archive_federated.rs`
- `crates/oasis7_distfs/src/lib.rs`（复用）

## 状态
- 当前状态：`已完成`
- 已完成：T0、T1、T2、T3、T4
- 进行中：无
- 未开始：无
- 阻塞项：无
