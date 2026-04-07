# Local Provider vs builtin P0 parity T4 结论（2026-03-17）

- owner: `qa_engineer`
- 联审: `producer_system_designer`、`runtime_engineer`、`viewer_engineer`
- 关联 PRD: `PRD-WORLD_SIMULATOR-038`
- 关联任务: `doc/world-simulator/llm/llm-provider-agent-experience-parity-2026-03-12.project.md` T4
- 结论状态: `failed`

## 1. 批次信息
- benchmark_run_id: `provider_builtin_parity_20260317_t4d`
- parity_tier: `P0`
- 场景: `P0-001` / `llm_bootstrap`
- seed / ticks / timeout: `5 / 4 / 15000ms`
- Local Provider profile: `oasis7_p0_low_freq_npc`
- 执行日期: `2026-03-17`
- 执行角色: `qa_engineer` / `producer_system_designer`

## 2. 样本覆盖
| Scenario-ID | provider | sample_count | valid_samples | invalid_fixture | benchmark_status |
| --- | --- | --- | --- | --- | --- |
| P0-001 | builtin | 1 | 1 | 0 | `failed` |
| P0-001 | provider_loopback_http | 1 | 1 | 0 | `passed` |

## 3. 核心指标并排对比
| 指标 | builtin | Local Provider | gap / 备注 | 是否达标 |
| --- | --- | --- | --- | --- |
| completion_rate | `0.0` | `1.0` | `+100pp`；超过 P0 通过线 `<= 5pp` | [ ] |
| invalid_action_rate | `0.0` | `0.0` | 无差异 | [x] |
| timeout_rate | `0.0` | `0.0` | 无差异 | [x] |
| median_extra_wait_ms | `11751` | `6024` | Local Provider 更低，但 builtin 已远超 `500ms` 通过线 | [ ] |
| p95_extra_wait_ms | `16296` | `6332` | Local Provider 更低，但 builtin 已远超 `1500ms` 通过线 | [ ] |
| trace_completeness | `1.0` | `1.0` | 无差异 | [x] |
| recoverable_error_resolution_rate | `1.0` | `1.0` | 两侧均未出现未恢复错误 | [x] |
| context_drift_count | `0` | `0` | 无差异 | [x] |

## 4. 失败签名汇总
| error_code | builtin count | Local Provider count | 是否阻断 | 备注 |
| --- | --- | --- | --- | --- |
| provider_unreachable | 0 | 0 | [ ] | `t4d` 正式结论批次未触发；此前 `t4c` 在 `--agent-provider-connect-timeout-ms=3000` 下出现 4 次，属于 operator/harness 风险，不纳入最终 parity 结论 |
| timeout | 0 | 0 | [ ] |  |
| invalid_action_schema | 0 | 0 | [ ] |  |
| context_drift | 0 | 0 | [ ] |  |
| session_cross_talk | 0 | 0 | [ ] |  |
| completion_rate_gap_exceeded | 1 | 0 | [x] | builtin 未完成 `P0-001` 巡游移动，Local Provider 完成 |

## 5. 关键证据
- 自动 benchmark 证据路径: `output/provider_parity/provider_builtin_parity_20260317_t4d/summary`
- 聚合对比: `output/provider_parity/provider_builtin_parity_20260317_t4d/summary/combined.csv`
- 失败摘要: `output/provider_parity/provider_builtin_parity_20260317_t4d/summary/failures.md`
- builtin 样本 summary: `output/provider_parity/provider_builtin_parity_20260317_t4d/samples/builtin/sample_1/summary/P0-001.builtin.json`
- Local Provider 样本 summary: `output/provider_parity/provider_builtin_parity_20260317_t4d/samples/provider_loopback_http/sample_1/summary/P0-001.provider_loopback_http.json`
- scorecard links: `output/provider_parity/provider_builtin_parity_20260317_t4d/scorecard-links.md`

## 6. QA 结论
- 自动指标结论：`failed`。
- 原因：在同一场景 / 同一 seed / 同一 tick budget 下，builtin `completion_rate=0%`，Local Provider `completion_rate=100%`，`completion_rate_gap=100pp`，明显超出 `P0` 通过线 `<= 5pp`。
- 体感判断：当前本机 builtin/Local Provider 结果口径明显不同，玩家/QA 能感知 provider 已切换，因此不能宣称“体验等价”。
- 风险补充：`scripts/provider-parity-p0.sh` 当前默认 `--agent-provider-connect-timeout-ms=3000`，会在真实 Local Provider 批处理中制造假性 `provider_unreachable`；本次正式结论使用 `15000ms` 连接超时重跑后得出。

## 7. Producer 结论
- 结论：保持 `experimental`，不允许基于当前样本把 Local Provider 标记为“与 builtin 体验等价”或“允许默认启用”。
- 决策依据：`PRD-WORLD_SIMULATOR-038` 的体验等价目标是“切 provider 不明显改变玩家感知结果”；当前样本中 builtin 未完成而 Local Provider 完成，差异足够大，尚不满足该门槛。
- 与 `PRD-WORLD_SIMULATOR-040` 的关系：`PRD-WORLD_SIMULATOR-040` 已冻结默认回归模式为 `headless_agent`，该策略保持不变；本结论只约束“是否达到 builtin/Local Provider parity”。

## 8. 最终建议
- 最终结论：`failed`
- 建议状态：保持 `experimental`
- 必修项:
  - 复核 builtin 当前运行配置与基线模型，解释 `P0-001` 未完成的原因。
  - 将真实 Local Provider parity 批处理的连接超时默认值与 `oasis7` 成功口径对齐，避免 `3000ms` 假性 `provider_unreachable` 干扰结论。
  - 在上述问题修复后重跑 `P0-001` 至少一轮同批次 builtin/Local Provider 对照，再决定是否继续推进 `P0` 扩面。
- 可延期项:
  - `P0-002~P0-005` 扩面采样。
  - 主观评分卡细化到更多场景。

## 9. 后续修复追踪（2026-03-17 / fix2）
- 修复内容：`agent_engineer` 已在 `crates/oasis7/src/bin/oasis7_provider_parity_bench.rs` 为 builtin parity lane 增加 `P0-001` 巡游 guardrail，并将 `oasis7_provider_parity_bench` / `scripts/provider-parity-p0.sh` 默认 connect-timeout 对齐到 `15000ms`。
- 复验批次：`provider_builtin_parity_20260317_fix2`。
- 复验结果：builtin `completion_rate=1.0`、`move_agent=4`、`timeout_rate=0.0`；Local Provider `completion_rate=0.0`、`timeout_rate=1.0`、`timeout=4`。
- 结论变化：builtin 基线退化问题已收口，但正式 T4 双签结论暂不改写；当前剩余阻断已收敛为真实 `Local Provider(Local HTTP)` 连续 timeout。
- 后续建议：继续排查 bridge / local_http provider / runtime agent 的请求链路超时来源，待真实 Local Provider 样本恢复可行动作后，再发起新的 parity 结论批次。

## 10. timeout 修复追踪（2026-03-17 / fix3）
- 修复内容：`agent_engineer` 已在 `crates/oasis7/src/bin/oasis7_provider_local_bridge.rs` 增加 `provider gateway call agent` timeout 时的 `provider agent --local` fallback，并用 `sha256(session_key)` 生成稳定 `session_id`，避免 gateway RPC 卡死时整轮样本降级为 `timeout`。
- 复验批次：`output/provider_parity/provider_builtin_parity_20260317_fix3`。
- 复验结果：builtin `completion_rate=1.0`、`timeout_rate=0.0`、`move_agent=4`；Local Provider `completion_rate=1.0`、`timeout_rate=0.0`、`move_agent=4`。
- 仍未达线项：builtin `median_extra_wait_ms=9900`、`p95_extra_wait_ms=10597`；Local Provider `median_extra_wait_ms=13957`、`p95_extra_wait_ms=14062`，两侧都显著高于 `P0` 通过线 `median<=500 / p95<=1500`。
- 结论变化：timeout 阻断已收口，completion gap 也已回到 `0pp`；但由于 absolute wait latency 仍远高于 `P0` 通过线，正式 T4/T5 口径继续保持 `failed / experimental`。
- 后续建议：优先压缩 builtin / Local Provider 的 prompt 体积与调用链开销，避免在 `P0-001` 上继续以 >9s 的单步等待进入下一层 parity。

## 11. 审计轮次 2 / 分层 latency gate 追加结论（2026-03-17）
- 适用口径：按 `PRD-WORLD_SIMULATOR-038` 审计轮次 2，真实在线 LLM provider 先看行为等价硬门禁（`completion_rate` / `invalid_action_rate` / `timeout_rate` / `relative_wait_gap`），再看发布 / 默认启用附加门槛（`latency_class`）。
- `fix3` 复验批次：`output/provider_parity/provider_builtin_parity_20260317_fix3`。
- 关键数值：builtin `median_extra_wait_ms=9900`、`p95_extra_wait_ms=10597`；Local Provider `median_extra_wait_ms=13957`、`p95_extra_wait_ms=14062`；相对 gap 为 `median=4057ms`、`p95=3465ms`。
- 追加 QA 结论：`fix3` 已满足行为等价硬门禁，故“behavior parity”可记为 `pass`；但 Local Provider 绝对等待仅达到 `latency_class B (experimental-only)`，默认启用门槛仍为 `not passed`。
- 追加 Producer 结论：保留原始 `t4d` `failed` 历史结论用于问题追踪，不做删除；在审计轮次 2 下，`fix3` 的正式产品口径更新为 `behavior_parity_pass / latency_class B / keep experimental`，允许受限试点，不允许默认启用。
- 后续建议：继续压缩 Local Provider prompt/调用链开销，把 `median_extra_wait_ms` / `p95_extra_wait_ms` 收敛到 `latency_class A`（`median<=500ms`、`p95<=1500ms`）后，再重签默认启用与扩面结论。
