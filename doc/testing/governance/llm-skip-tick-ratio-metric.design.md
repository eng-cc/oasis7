# oasis7: LLM 跳过 Tick 占比指标设计

- 对应需求文档: [LLM 跳过 Tick 占比指标](llm-skip-tick-ratio-metric.prd.md)
- 可变任务状态与历史: PR、实际 CI 与评审记录（Issue 按需）

## 1. 设计定位

本指标用于观察 LLM 调用节省，不新增 fail gate、阈值或阻断策略，不改变 LLM 决策与既有 release gate。它由 demo trace 采集，经 report 持久化，进入 stress 单场景和聚合输出。

<a id="llmskip-count-output-design"></a>
## 2. 计数与计算

- 分母为 `active_ticks`，不是 wall-clock 或总请求数。
- 每个 active tick 命中 `llm_input == None` 或 `skip_llm_with_active_execute_until` 即累计一次 `llm_skipped_ticks`；同 tick 同时命中仍只计一次。
- `llm_skipped_tick_ratio_ppm = floor(llm_skipped_ticks * 1_000_000 / active_ticks)`；`active_ticks=0` 返回 0。采用足够宽/饱和的整数计算避免中间乘法溢出。
- 聚合先求和 skipped 与 active 计数，再按同式计算，不平均各场景的 ppm；保留单场景原值用于解释权重。

### 2.1 需求承接与分配表

| 上游 requirement / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [观测指标](llm-skip-tick-ratio-metric.prd.md#llmskip-observation-contract) | 两类 skipped、active 分母、ppm、聚合与无 jq fallback；不增 gate | [设计约束](#llmskip-count-output-design) | 对应模块维护者与 QA | 文档补齐不宣称实现或真实运行完成 |

## 3. 输出链路与边界

- demo 的 `TraceCounts` 提供计数与 ppm，终态日志输出同名字段；`report.json` 使用 `trace_counts.llm_skipped_ticks` 和 `trace_counts.llm_skipped_tick_ratio_ppm`，分母为 `active_ticks`。
- `llm-longrun-stress.sh` 从 report/log 提取相同指标，写 summary、TSV、聚合 JSON/summary；无 jq 时的 Python/log fallback 保留字段和计算口径。
- trace 缺失按 PRD 的无 LLM 请求语义判断，不能由日志文件不存在臆造有效观测；缺失的来源与实际零值须在诊断中区分。
- 采集失败与 fallback 的说明服务于结果解释，不引入本指标独有的阈值门禁；现有 fail gate 按原合同执行。

## 4. 验证设计

用离线 trace/report fixture 覆盖两类 skipped、同 tick 双命中、普通请求、零 active、非整除 ppm 与不同 active 数量的聚合；分别核对 jq、Python 和 log fallback 的相同输出。跨环境缺失路径不能写成已执行通过。

## 5. 演进边界

新增跳过路径时先补计数语义与回归，再同步所有输出消费者；本专题不定义调用调度算法、Viewer UI 或新的发布门禁。


## 11. 验证设计与可追溯性

### 11.1 验证映射表

| 上游 requirement / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source 或 ID、scenario/layer、candidate/environment | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [观测指标](llm-skip-tick-ratio-metric.prd.md#llmskip-observation-contract) | [设计约束](#llmskip-count-output-design) | 两类 skipped、active 分母、ppm、聚合与无 jq fallback；不增 gate | 候选版本的[验证入口](../../../testing-manual.md)：S8 的 demo/report/stress 同源字段核对；两类 skipped、零分母和不同权重 fixture；静态核对与实际执行分别记录 | 实施 PR 的候选、命令、fixture 与真实结果 | 不代签未执行回归、真实模型、浏览器、节点或发布验收 |
