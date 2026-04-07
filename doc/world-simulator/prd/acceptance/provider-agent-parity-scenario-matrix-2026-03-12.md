# Local Provider vs 内置 Agent parity 场景矩阵（2026-03-12）

审计轮次: 2

## 目标
- 冻结 `Local Provider` 与内置 agent 的 parity 场景层级、样本选择与通过顺序，避免后续评估随意挑样。
- 为 `PRD-WORLD_SIMULATOR-038` 提供统一场景输入基线，使自动 benchmark 与主观试玩评分复用同一组任务目标。
- 定义 `P0/P1/P2` 的扩面门槛：只有当前层行为等价硬门禁通过，才允许进入下一层；只有达到 `latency_class A`，才允许默认启用。

## 范围
- In Scope:
  - `P0 低频单 NPC` 场景清单。
  - `P1 多轮记忆/对话` 场景清单。
  - `P2 多 agent 并发` 场景清单。
  - 每个场景的目标、采样次数、必采指标、阻断条件。
  - 行为等价硬门禁与发布/默认启用附加门槛。
- Out of Scope:
  - 不直接定义具体实现代码或 benchmark 脚本参数。
  - 不纳入高频战斗、经济关键路径或上百 agent 压测场景。

## 执行原则
1. builtin 与 Local Provider 必须使用同一 scenario fixture、相同 seed、相同 timeout budget。
2. 任一场景若输入条件不一致，则该轮样本作废并重跑。
3. 每层场景必须全部达成行为等价硬门禁，才允许判定该层 `passed`。
4. 任一阻断条件触发，则整层直接判定 `failed` 或 `blocked`。
5. 真实在线 LLM provider 的等待时延分两层治理：先看 `relative_wait_gap` 是否满足行为等价，再看 Local Provider 绝对等待是否仅能归为 `latency_class B` 或已达到 `latency_class A`。

## P0：低频单 NPC（首轮上线门槛）

| Scenario-ID | 场景名称 | 目标 | 推荐样本数 | 必采指标 | 阻断条件 |
| --- | --- | --- | --- | --- | --- |
| P0-001 | 巡游移动 | Agent 在相邻 location 间完成 3 次移动 | 20 | `completion_rate` `invalid_action_rate` `relative_wait_gap_ms` | 完成率差值 > 5pp |
| P0-002 | 近邻观察 | Agent 对周边目标执行观察/inspect 并输出稳定摘要 | 20 | `completion_rate` `trace_completeness` | trace 缺字段 > 5% |
| P0-003 | 简单对话 | Agent 与近邻执行 2~3 轮基础对话 | 20 | `completion_rate` `relative_wait_gap_ms` `recoverable_error_resolution_rate` | 玩家明显等待失控且无法恢复 |
| P0-004 | 简单交互 | Agent 完成单步交互（开关/拾取/使用类） | 20 | `completion_rate` `invalid_action_rate` | 非法动作率 > 3% |
| P0-005 | 拒绝路径恢复 | 注入 1 次 provider timeout 或 schema 错误，验证系统恢复 | 10 | `recoverable_error_resolution_rate` `trace_completeness` | 无法定位错误或无法回退 |

### P0 行为等价硬门禁
- `completion_rate_gap <= 5pp`
- `invalid_action_rate <= 3%` 且不超过 builtin 2 倍
- `timeout_rate <= 2%`
- `relative_wait_gap_median <= 5000ms`
- `relative_wait_gap_p95 <= 8000ms`
- `trace_completeness >= 95%`
- `recoverable_error_resolution_rate >= 90%`

### P0 发布 / 默认启用附加门槛
- `latency_class A (default-candidate)`: Local Provider `median_extra_wait_ms <= 500ms` 且 `p95_extra_wait_ms <= 1500ms`
- `latency_class B (experimental-only)`: Local Provider `median_extra_wait_ms <= 15000ms` 且 `p95_extra_wait_ms <= 20000ms`
- `latency_class C (blocked)`: 超出 `B` 上限

### P0 阻断线
- 任一场景 `completion_rate_gap > 10pp`
- 任一场景 `timeout_rate > 5%`
- 任一场景行为等价硬门禁未通过且无法通过失败签名解释为无效样本
- 任一批次 `latency_class = C`
- 任一场景出现“用户无法恢复且无清晰错误提示”
- 任一场景出现 runtime 权威绕过或未受白名单约束的动作执行

## P1：多轮记忆 / 对话连续性

| Scenario-ID | 场景名称 | 目标 | 推荐样本数 | 必采指标 | 阻断条件 |
| --- | --- | --- | --- | --- | --- |
| P1-001 | 任务目标保持 | Agent 在 3~5 轮内持续追踪同一短期目标 | 15 | `completion_rate` `context_drift_count` `relative_wait_gap_ms` | 频繁忘记当前目标 |
| P1-002 | 失败后重试 | Agent 在一次失败后能够依据反馈修正下一步动作 | 15 | `recoverable_error_resolution_rate` `retry_count` | 无法利用反馈修正 |
| P1-003 | 对话上下文连续 | Agent 在多轮对话中保持人物与话题一致性 | 15 | `memory_hit_quality` `trace_completeness` | 上下文明显漂移 |
| P1-004 | 记忆摘要预算 | 在受限 context budget 下仍能完成关键记忆命中 | 10 | `completion_rate` `context_drift_count` | 因摘要退化导致主目标明显失败 |

### P1 通过线
- 满足全部 P0 行为等价硬门禁
- `context_drift_count` 相比 builtin 增量不超过 1 次 / 会话
- `memory_hit_quality` 主观评分均值 >= 4/5
- 关键会话 `completion_rate_gap <= 5pp`
- 发布/默认启用仍沿用当前层 `latency_class A/B/C` 分级

### P1 阻断线
- 任一场景出现“反复忘记当前目标/角色关系”
- 任一场景出现“trace 无法解释为什么发生漂移”
- 任一批次 `latency_class = C`

## P2：多 agent 并发

| Scenario-ID | 场景名称 | 目标 | 推荐样本数 | 必采指标 | 阻断条件 |
| --- | --- | --- | --- | --- | --- |
| P2-001 | 双 Agent 巡游 | 2 个低频 agent 并发执行移动/观察 | 10 | `completion_rate` `timeout_rate` `relative_wait_gap_ms` | 并发下延迟显著失控 |
| P2-002 | 双 Agent 对话 | 2 个 agent 与 1 个环境目标交替对话 | 10 | `completion_rate` `trace_completeness` | trace 混淆到无法归因 |
| P2-003 | 五 Agent 混合低频行为 | 2~5 个 agent 同时执行移动/观察/简单交互 | 10 | `completion_rate` `relative_wait_gap_ms` `invalid_action_rate` | 并发下动作质量退化过大 |

### P2 通过线
- 满足全部 P0/P1 行为等价硬门禁
- `completion_rate_gap <= 8pp`
- `trace_completeness >= 95%`
- 并发场景无系统性 timeout 激增
- 发布/默认启用仍沿用当前层 `latency_class A/B/C` 分级

### P2 阻断线
- 任一场景出现会话串线、trace 归因错误或 provider session 交叉污染
- 并发下出现系统性 timeout 激增，导致玩家明显感知“agent 直连路径几乎不可玩”
- 任一批次 `latency_class = C`

## 证据要求
- 自动 benchmark 结果：JSON / CSV / 终端日志。
- viewer trace 摘要：最近动作、错误码、延迟、provider 名称、`agent_profile`。
- 主观评分卡：每层至少 1 份，由 `qa_engineer` 与 `producer_system_designer` 共审。
- 结论模板：`behavior_parity_pass / failed / blocked` + `latency_class` + 是否允许扩大覆盖范围。

## PRD-ID 映射
- `PRD-WORLD_SIMULATOR-038`: `P0/P1/P2` parity 场景层级、行为等价门禁与 rollout latency class。
- `PRD-WORLD_SIMULATOR-037`: `P0` 场景中的用户机 `Local Provider(Local HTTP)` 接入闭环。
- `PRD-WORLD_SIMULATOR-036`: `Decision Provider` 标准层的 fixture/trace/feedback 契约复用。

## 风险与约束
- 若直接跳过 P0 进入 P1/P2，容易把 provider 基础问题误判成“高复杂度自然波动”。
- 若主观试玩评分与自动 benchmark 不同时归档，后续很难复盘“为什么体感不一致”。
- 若 scenario fixture 本身不稳定，会污染 `relative_wait_gap` 与 completion 对比，导致假性结论。
- 若把 `latency_class B` 误当成“可默认启用”，会把真实在线模型的等待体感风险直接带入主体验。
