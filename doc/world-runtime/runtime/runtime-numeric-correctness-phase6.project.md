# oasis7 Runtime：Membership 协调租约与时间源窄化数值语义硬化（15 点清单第六阶段）项目管理文档

- 对应设计文档: `doc/world-runtime/runtime/runtime-numeric-correctness-phase6.design.md`
- 对应需求文档: `doc/world-runtime/runtime/runtime-numeric-correctness-phase6.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] T-MIG-20260303 (PRD-ENGINEERING-006): 逐篇阅读旧文档并完成人工重写迁移到 `.prd` 命名。
### T0 建档
- [x] 新建设计文档：`doc/world-runtime/runtime/runtime-numeric-correctness-phase6.prd.md`
- [x] 新建项目管理文档：`doc/world-runtime/runtime/runtime-numeric-correctness-phase6.project.md`

### T1 Membership 协调器 lease 受检加法
- [x] `membership_reconciliation` 的 in-memory coordinator `expires_at_ms` 改为受检加法。
- [x] `membership_recovery/stores` 的 store-backed coordinator `expires_at_ms` 改为受检加法。
- [x] 新增溢出拒绝且状态不变测试。

### T2 时间源毫秒窄化受检转换
- [x] 收口 phase6 范围内 `as_millis() as i64`（consensus/net/node）。
- [x] 统一 helper 与错误/夹逼语义，避免隐式截断。
- [x] 新增边界测试覆盖超大 `Duration`。

### T3 回归与收口
- [x] 运行 `oasis7_consensus`、`oasis7_net`、`oasis7_node` 定向回归测试。
- [x] 回写设计文档状态（M0~M3）。
- [x] 回写项目状态与 `doc/devlog/README.md`。

## 依赖
- `crates/oasis7_consensus/src/membership_reconciliation.rs`
- `crates/oasis7_consensus/src/membership_recovery/stores.rs`
- `crates/oasis7_consensus/src/membership_reconciliation_tests.rs`
- `crates/oasis7_consensus/src/membership_recovery_tests.rs`
- `crates/oasis7_net/src/dht.rs`
- `crates/oasis7_net/src/provider_cache.rs`
- `crates/oasis7_net/src/libp2p_net.rs`
- `crates/oasis7_net/src/gateway.rs`
- `crates/oasis7_net/src/index_store.rs`
- `crates/oasis7_net/src/dht_cache.rs`
- `crates/oasis7_node/src/runtime_util.rs`
- `crates/oasis7_consensus/src/dht.rs`

## 状态
- 当前状态：`已完成`
- 已完成：T0、T1、T2、T3
- 进行中：无
- 未开始：无
- 阻塞项：无
