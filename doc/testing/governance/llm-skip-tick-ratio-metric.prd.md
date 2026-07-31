# oasis7: LLM 跳过 Tick 占比指标

- 对应设计文档: `doc/testing/governance/llm-skip-tick-ratio-metric.design.md`
- 可变任务状态与历史: GitHub task issue evidence comments

审计轮次: 4


## 1. Executive Summary
- Problem Statement: 当前 LLM 运行链路缺少“跳过 LLM 调用 tick 占比”这一统一指标，无法量化 `execute_until` 等机制带来的调用节省效果。
- Proposed Solution: 在 demo、report 与长跑汇总脚本中统一输出 `llm_skipped_ticks` 与 `llm_skipped_tick_ratio_ppm`，并通过稳定口径与单测保障可消费性。
- Success Criteria:
  - SC-1: `oasis7_llm_agent_demo` 能输出跳过计数与百万分比占比指标。
  - SC-2: `scripts/llm-longrun-stress.sh` 在单场景与聚合输出中包含该指标。
  - SC-3: `report.json` 持久化包含新增字段，自动脚本可直接消费。
  - SC-4: 指标计算口径固定为 `llm_skipped_ticks / active_ticks` 并有测试覆盖。

## 2. User Experience & Functionality
- User Personas:
  - 测试维护者：需要量化 LLM 调用节省与运行行为变化。
  - 性能分析者：需要统一字段做跨场景横向比较。
  - 脚本维护者：需要稳定可解析的数据结构避免后处理分叉。
- User Scenarios & Frequency:
  - 长跑压测执行：每次 `llm-longrun-stress` 汇总时读取该指标。
  - 回归测试：每次改动 LLM 调度路径时验证指标口径稳定。
  - 日常调试：运行 `oasis7_llm_agent_demo` 时查看实时指标输出。
- User Stories:
  - PRD-TESTING-GOV-LLMSKIP-001: As a 测试维护者, I want skipped-tick metrics emitted in demo and reports, so that I can track how often LLM calls are skipped.
  - PRD-TESTING-GOV-LLMSKIP-002: As a 脚本维护者, I want stress summaries and aggregated outputs to include the same metric fields, so that automation remains stable.
  - PRD-TESTING-GOV-LLMSKIP-003: As a 性能分析者, I want a fixed ratio definition based on `active_ticks`, so that cross-run comparisons are consistent.
- Critical User Flows:
  1. Flow-LLMSKIP-001: `demo tick 执行 -> trace 采样判定是否 skipped -> 更新计数与占比`
  2. Flow-LLMSKIP-002: `report.json 持久化 -> stress 脚本读取 report/log -> 输出 summary/TSV/聚合 JSON`
  3. Flow-LLMSKIP-003: `回归测试运行 -> 校验计数和 ppm 计算 -> 防止口径回归`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| demo 指标统计 | `llm_skipped_ticks`、`llm_skipped_tick_ratio_ppm` | 每个 active tick 根据 trace 语义判定并累计 | `collecting -> finalized` | `ratio_ppm = skipped * 1_000_000 / active_ticks` | Runtime/测试维护者可调整统计实现 |
| skipped 判定规则 | `llm_input == None`、`skip_llm_with_active_execute_until` | 命中任一语义即记为 skipped | `trace -> classified` | 固定判定集合，新增路径需补规则 | 规则变更需同步测试 |
| stress 输出接入 | summary、TSV、aggregated summary/json 字段 | 解析 report/log 后填充新增字段 | `parsed -> merged -> exported` | 单场景与聚合字段名保持一致 | 脚本维护者可扩展输出 |
| 测试覆盖 | 单测断言计数与占比 | 执行测试命令验证口径 | `pending -> passed/failed` | 覆盖边界值与常规值 | CI 执行、维护者审阅 |
- Acceptance Criteria:
  - AC-1: `TraceCounts` 含 `llm_skipped_ticks` 与 `llm_skipped_tick_ratio_ppm` 字段。
  - AC-2: demo finalize 输出和 `report.json` 均包含新增指标。
  - AC-3: `scripts/llm-longrun-stress.sh` 的单场景与聚合输出均新增该指标。
  - AC-4: 单测覆盖 skipped 判定与 ppm 计算口径。
  - AC-5: 不影响 LLM 决策行为与现有 fail gate。
- Non-Goals:
  - 不改动 LLM 决策逻辑。
  - 不新增 fail gate 或阈值门禁。
  - 不改造 viewer live UI 展示。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（本专题仅增加可观测指标，不改动 AI 推理模型行为）。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 在 `oasis7_llm_agent_demo` 运行期采样 trace，统一写入 `TraceCounts`，由 stress 脚本从 report/log 读取并输出到场景级与聚合级报表。
- Integration Points:
  - `crates/oasis7/src/bin/oasis7_llm_agent_demo.rs`
  - `crates/oasis7/src/bin/oasis7_llm_agent_demo/tests.rs`
  - `scripts/llm-longrun-stress.sh`
- Edge Cases & Error Handling:
  - `active_ticks = 0`：占比计算回退为 0，避免除零。
  - trace 缺失或字段为空：按“无 LLM 请求语义”规则谨慎判定并记录。
  - 无 `jq` 环境：python/log 回退路径必须同步输出新增指标。
  - 新增跳过路径未纳入规则：通过单测失败暴露并补齐判定。
- Non-Functional Requirements:
  - NFR-LLMSKIP-1: 指标字段命名在 demo/report/stress 三端一致。
  - NFR-LLMSKIP-2: 占比计算口径稳定，跨版本可比。
  - NFR-LLMSKIP-3: 指标采集不改变既有执行路径语义。
  - NFR-LLMSKIP-4: 自动脚本无需额外手工转换即可消费。
- Security & Privacy: 指标仅统计调用行为，不新增敏感数据输出。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (LLMSKIP-1): 完成设计文档与任务建档。
  - v1.1 (LLMSKIP-2): demo 侧完成计数与占比统计输出。
  - v2.0 (LLMSKIP-3): stress 脚本接入新增指标并输出聚合结果。
  - v2.1 (LLMSKIP-4): 单测与脚本语法校验收口。
- Technical Risks:
  - 风险-1: trace 缺失导致分母语义偏差，需固定 `active_ticks` 口径。
  - 风险-2: 无 `jq` 回退路径遗漏字段，导致不同环境输出不一致。
  - 风险-3: 后续新增跳过路径但未更新判定规则，造成低估或高估。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-TESTING-GOV-LLMSKIP-001 | LLMSKIP-1/2 | `test_tier_required` | demo 输出字段与 report 持久化核验 | LLM 运行观测链路 |
| PRD-TESTING-GOV-LLMSKIP-002 | LLMSKIP-2/3 | `test_tier_required` | stress 单场景/聚合输出字段检查 | 长跑自动化报表 |
| PRD-TESTING-GOV-LLMSKIP-003 | LLMSKIP-3/4 | `test_tier_required` | 单测校验 skipped 计数与 ppm 计算 | 指标口径稳定性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-LLMSKIP-001 | 使用 `active_ticks` 作为占比分母 | 使用总 tick 或 wall-clock 时间 | `active_ticks` 与运行负载语义更一致。 |
| DEC-LLMSKIP-002 | 统一使用 ppm（百万分比）表达占比 | 浮点百分比字符串 | ppm 更稳定、便于脚本与测试比较。 |
| DEC-LLMSKIP-003 | 先做观测指标，不新增 fail gate | 同步加入阈值门禁 | 控制变更风险，先验证指标质量。 |

## 原文约束点映射（内容保真）
- 原“目标（量化 skipped tick 并多端消费）” -> 第 1 章 Problem/Solution/SC。
- 原“In Scope/Out of Scope” -> 第 2 章 AC 与 Non-Goals。
- 原“接口/数据（TraceCounts 字段、判定与公式）” -> 第 2 章规格矩阵 + 第 4 章 Integration。
- 原“里程碑 M1~M4” -> 第 5 章 Phased Rollout（LLMSKIP-1~4）。
- 原“风险（口径、兼容、演进）” -> 第 4 章 Edge Cases + 第 5 章 Technical Risks。
