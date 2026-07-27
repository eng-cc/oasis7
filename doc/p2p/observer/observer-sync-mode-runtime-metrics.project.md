# oasis7 Runtime：Observer 同步源运行态统计（项目管理文档）

- 对应设计文档: `doc/p2p/observer/observer-sync-mode-runtime-metrics.design.md`
- 对应需求文档: `doc/p2p/observer/observer-sync-mode-runtime-metrics.prd.md`

审计轮次: 5
## 专业权威口径
- 本文件统一维护运行态统计、自动记录桥接与模式可观测性的任务状态；被吸收源文件只从 Git history 与 GitHub task evidence 追溯。

## 任务拆解（含 PRD-ID 映射）
- [x] OSRM-1 (PRD-P2P-MIG-108)：设计文档与项目管理文档落地。
- [x] OSRM-2 (PRD-P2P-MIG-108)：实现运行态统计结构与导出接口。
- [x] OSRM-3 (PRD-P2P-MIG-108)：补齐单元测试并完成 `oasis7_net` 回归。
- [x] OSRM-4 (PRD-P2P-MIG-108)：回写状态文档与 devlog。
- [x] observer-metrics-bridge-authority (PRD-P2P-MIG-106-001) [test_tier_required]: 承接原统计桥接专题，完成四个单轮/follow 接口、逐报告记录、调用方持有 metrics 与兼容聚合语义。 Trace: #2655 (task_e86ac688cfbf4cc78809fd78c401c6cc)
- [x] observer-mode-observability-authority (PRD-P2P-MIG-107-001) [test_tier_required]: 承接原策略可观测性专题，完成非 DHT/DHT observed report、真实 fallback 标识和旧报告兼容边界。 Trace: #2655 (task_e86ac688cfbf4cc78809fd78c401c6cc)

## 依赖
- doc/p2p/observer/observer-sync-mode-runtime-metrics.prd.md
- `crates/oasis7_net/src/observer.rs`
- `crates/oasis7_net/src/lib.rs`

## 状态
- 当前阶段：Observer 同步源运行态统计完成（OSRM-1~OSRM-4 全部完成）。
- 下一步：将 `ObserverRuntimeMetrics` 接入 runtime 周期采样与 viewer/运维面板展示链路；本次合并不声明该面板已完成。
- 最近更新：2026-07-27（专业权威合并）。
