# oasis7 Runtime：Observer 同步源策略化（项目管理文档）

- 对应设计文档: `doc/p2p/observer/observer-sync-source-mode.design.md`
- 对应需求文档: `doc/p2p/observer/observer-sync-source-mode.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）
- [x] OSSM-1 (PRD-P2P-MIG-110)：设计文档与项目管理文档落地。
- [x] OSSM-2 (PRD-P2P-MIG-110)：实现 `HeadSyncSourceMode` 与 `ObserverClient` 模式化同步入口。
- [x] OSSM-3 (PRD-P2P-MIG-110)：补齐单元测试并完成 `oasis7_net` 回归。
- [x] OSSM-4 (PRD-P2P-MIG-110)：回写状态文档与 devlog。
- [x] observer-sync-dht-authority (PRD-P2P-MIG-109-001) [test_tier_required]: 承接原 DHT 组合链路专题，完成 `HeadSyncSourceModeWithDht`、有界回退、双错误上下文和实现追踪合并。 Trace: #2655 (task_e86ac688cfbf4cc78809fd78c401c6cc)

## 依赖
- doc/p2p/observer/observer-sync-source-mode.prd.md
- `crates/oasis7_net/src/observer.rs`
- `crates/oasis7_net/src/head_follow.rs`
- `crates/oasis7_net/src/lib.rs`
- `doc/p2p/prd.md`（早期 path-index source 未由当前 net facade 暴露的负向边界）

## 状态
- 当前阶段：Observer 同步源策略化完成（OSSM-1~OSSM-4 全部完成）。
- 下一步：如需重新激活 DHT/路径索引，先由 runtime owner 建 task、接回 crate facade 并补定向回归；在此之前不把 dormant source 外推为模块能力或 readiness。
- 最近更新：2026-07-27（专业权威合并）。
- 早期 path-index observer/bootstrap 专题仅作历史 provenance；当前 `oasis7_net` facade 未暴露该路径，不得把旧完成态写成 active fallback 或恢复保证。
