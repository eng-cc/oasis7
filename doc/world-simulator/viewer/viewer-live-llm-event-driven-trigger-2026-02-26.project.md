# Viewer Live LLM 事件触发决策门控（项目管理）

- 对应设计文档: `doc/world-simulator/viewer/viewer-live-llm-event-driven-trigger-2026-02-26.design.md`
- 对应需求文档: `doc/world-simulator/viewer/viewer-live-llm-event-driven-trigger-2026-02-26.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）
- [x] T0 建档：设计文档与项目管理文档
- [x] T1 `LiveWorld` 增加 LLM 决策门控状态，并接入普通 live `step()`
- [x] T2 consensus 路径接入门控与提交后唤醒
- [x] T3 请求入口接入唤醒：`Play/Step/AgentChat/PromptControl Apply/Rollback`
- [x] T4 回归测试：验证空结果下不会重复累加空决策 tick

## 依赖
- doc/world-simulator/viewer/viewer-live-llm-event-driven-trigger-2026-02-26.prd.md
- `crates/oasis7/src/viewer/live_split_part1.rs`
- `crates/oasis7/src/viewer/live_split_part2.rs`
- `crates/oasis7/src/viewer/live/consensus_bridge.rs`
- `crates/oasis7/src/viewer/live/tests.rs`
- `testing-manual.md`

## 状态
- 最近更新：2026-03-06（ROUND-005 I5-001 字段补齐）
- 当前阶段：已完成（T0~T4）
- 验证结果：`live_world_llm_event_driven_gate_avoids_repeated_empty_ticks` 通过
- 玩法恢复语义：一次成功的 `Play`、`Step`、`AgentChat` 或 PromptControl 操作只取得一次决策机会，不保证一定产生 world event。若决策没有 action/event，世界进入等待下一次有效触发的静止态；玩家应以新的有效操作改变局面，而不是依赖自动重试制造事件。控制请求的 ACK 表示请求被接收，不表示已产生世界变化。
- 后续可选项：若需要进一步降载，可在播放循环层引入“长期空结果自动降频/暂停”策略；该策略必须保留上述“空结果可恢复、非自动刷事件”的玩家语义。
