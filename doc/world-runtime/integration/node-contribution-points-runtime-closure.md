# oasis7 Runtime：节点贡献积分运行时闭环（设计文档）

审计轮次: 4

## 目标
- 将节点积分从“离线样本测试”推进到“运行时采样闭环”：
  - 从 `NodeRuntime::snapshot` 周期采样节点运行状态；
  - 结合 DistFS 本地目录存储量，生成节点贡献样本；
  - 按 epoch 结算积分并输出报告。
- 在多节点测试中验证该闭环可稳定运行。

## 范围

### In Scope
- 新增运行时采样器 `NodePointsRuntimeCollector`：
  - 输入：`NodeSnapshot` + `effective_storage_bytes` + `observed_at_unix_ms`。
  - 输出：`EpochSettlementReport`。
- 新增本地存储量测算函数，用于采样 `effective_storage_bytes`。
- 新增多节点运行时闭环集成测试（启动多个 `NodeRuntime` 进行实际采样/结算）。

### Out of Scope
- 生产级采样探针（真实挑战证明、远端存储可读性证明）。
- Viewer UI 展示节点积分榜。
- 链上资产结算与可转账收益。

## 接口 / 数据

### 运行时观测输入（草案）
```rust
NodePointsRuntimeObservation {
  node_id: String,
  role: NodeRole,
  worker_poll_count: u64, // 来自 NodeSnapshot.tick_count（兼容字段）
  consensus_last_observed_tick: u64,
  running: bool,
  observed_at_unix_ms: i64,
  has_error: bool,
  effective_storage_bytes: u64,
}
```

### 采样器接口（草案）
```rust
NodePointsRuntimeCollector::observe_snapshot(
  snapshot: &NodeSnapshot,
  effective_storage_bytes: u64,
  observed_at_unix_ms: i64,
) -> Option<EpochSettlementReport>

NodePointsRuntimeCollector::force_settle() -> Option<EpochSettlementReport>
```

### 采样映射策略（MVP）
- `worker_poll_count`（兼容来源: `NodeSnapshot.tick_count`）差分映射为基础义务计算量 `self_sim_compute_units`；该值仅表示 worker 轮询推进，不等价于共识 tick。
- 角色导向额外贡献估算：
  - `Sequencer`：优先记为 `world_maintenance_compute_units`；
  - `Observer`：优先记为 `delegated_sim_compute_units`；
  - `Storage`：按较低比例记为 `delegated_sim_compute_units`。
- `running` + 观测时间差分累计 `uptime_seconds`。
- `last_error` 映射为 `verify_pass_ratio` 降级与 `explicit_penalty_points`。
- `effective_storage_bytes` 取 epoch 内观测最大值。

## 里程碑
- NCPR-1：设计文档 + GitHub Issue/Project task truth。
- NCPR-2：运行时采样器与存储测算实现。
- NCPR-3：多节点运行时闭环集成测试。
- NCPR-4：回归、文档与 devlog 收口。

## 风险
- 角色导向映射属于启发式估算，不代表最终经济策略；后续应替换为可验证贡献事件。
- 目录存储量采样无法直接代表“可挑战可读”的有效存储，需要后续接入挑战机制。
- 观测周期过长会导致时间差分粗糙，过短会增加采样开销，需要参数化。
