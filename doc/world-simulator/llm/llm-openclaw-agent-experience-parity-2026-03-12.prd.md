# OpenClaw 与内置 Agent 体验等价（parity）验收方案（2026-03-12）

- 对应设计文档: `doc/world-simulator/llm/llm-openclaw-agent-experience-parity-2026-03-12.design.md`
- 对应项目管理文档: `doc/world-simulator/llm/llm-openclaw-agent-experience-parity-2026-03-12.project.md`

审计轮次: 2

## 1. Executive Summary
- Problem Statement: 现有 `Decision Provider` 与 `OpenClaw(Local HTTP)` 方案已经回答了“如何接入”和“首期如何启动 PoC”，但尚未把“对玩家来说必须获得与内置 agent 层等价的游戏体验”写成硬性目标。若没有独立的 parity 目标、场景矩阵和阻断线，`OpenClaw` 可能在技术上可接，却在体验上长期低于内置 agent。
- Proposed Solution: 新增 `OpenClaw vs 内置 Agent 体验等价（parity）` 专题，定义体验等价的范围、分层指标、场景矩阵、通过线与阻断线，并将 `OpenClaw(Local HTTP)` 的交付目标从“可玩 PoC”升级为“在指定场景下达到用户可感知等价”。本专题引用 `doc/world-simulator/prd/acceptance/openclaw-agent-parity-scenario-matrix-2026-03-12.md`、`doc/world-simulator/prd/acceptance/openclaw-agent-parity-score-card-2026-03-12.md`、`doc/world-simulator/prd/acceptance/openclaw-agent-parity-benchmark-protocol-2026-03-12.md`、`doc/world-simulator/prd/acceptance/openclaw-agent-parity-aggregation-template-2026-03-12.md` 与 `doc/world-simulator/llm/openclaw-agent-profile-oasis7_p0_low_freq_npc-2026-03-13.md` 作为统一场景、评分、聚合与玩法口径模板。只有通过 parity 验收的 provider 才允许进入默认体验或更大范围试点。
- Success Criteria:
  - SC-1: 对首期纳入范围的场景，`OpenClaw` 与内置 agent 的任务完成率差值不超过 5 个百分点。
  - SC-2: 对首期纳入范围的场景，真实在线 LLM provider 的 parity 时延采用“分层口径”而非单一绝对值硬门禁：行为等价硬门禁看 `relative_wait_gap`（OpenClaw 相对 builtin 的 `median_extra_wait_ms_gap` / `p95_extra_wait_ms_gap`），发布/扩面附加门槛看 `latency_class`。
  - SC-3: `OpenClaw` 的无效动作率、超时率、非法 schema 率均不得高于内置 agent 基线 2 倍以上，且绝对值必须低于阻断线。
  - SC-4: viewer/QA 对两类 provider 的 trace 可解释性与错误恢复路径保持一致，不出现“agent 直连 lane 下无法定位问题”的观测断层。
  - SC-5: 若行为等价已达标但 `latency_class` 仅达到 `B (experimental)`，则该 provider 只允许保留在 `experimental` 或受限试点，不得默认启用。
  - SC-6: 首期 `P0` parity 样本必须使用固定的 OpenClaw 玩法 profile（当前默认 `oasis7_p0_low_freq_npc`；旧别名 `oasis7_p0_low_freq_npc` 已移除），并在 summary / scorecard 中保留该 profile 标识，避免“同场景不同 skill”造成假性通过。
  - SC-7: 只有当行为等价硬门禁通过且 `latency_class` 达到 `A (default-candidate)` 时，才允许把该 provider 作为默认体验或推进更大范围扩面。

## 2. User Experience & Functionality
- User Personas:
  - 玩家 / 制作人：希望切换到 `OpenClaw` 后，不明显感觉到 agent 变笨、变慢、变脆弱。
  - `agent_engineer`：需要一套清晰的 parity 通过线，判断外部 provider 能否取代内置实现进入主体验。
  - `qa_engineer`：需要标准化场景、评分项和阻断结论模板，避免只凭主观印象判断“差不多”。
  - `producer_system_designer`：需要把“体验等价”作为产品目标，而不是纯技术可行性。
- User Scenarios & Frequency:
  - 版本候选评审：每次计划扩大 `OpenClaw` 覆盖范围前，至少执行一轮 parity 评审。
  - 核心玩法回归：每次 OpenClaw adapter、动作白名单、记忆注入、profile/skill 协议或 prompt 协议变化后执行。
  - 发布阻断判定：当目标场景未达 parity 时，明确给出“不允许默认启用”的结论。
- User Stories:
  - PRD-WORLD_SIMULATOR-038: As a 玩家 / 制作人, I want `OpenClaw`-driven agents to feel equivalent to built-in agents in scoped gameplay scenarios, so that switching provider does not noticeably degrade the game experience.
  - PRD-WORLD_SIMULATOR-038A: As an `agent_engineer`, I want parity harnesses and provider requests to carry a stable gameplay profile / skill id, so that benchmark evidence reflects a fixed玩法口径 instead of accidental prompt drift.
- Critical User Flows:
  1. Flow-PARITY-001（单场景对标）:
     `同一 observation fixture / 场景脚本 -> 内置 agent 运行 -> OpenClaw provider（固定 agent_profile）运行 -> 对比完成率、等待差值、无效动作率、trace 完整度`。
  2. Flow-PARITY-002（玩家试玩盲测）:
     `制作人或 QA 使用同一场景分别试玩 builtin / agent_direct_connect -> 记录主观评分与关键阻断差异 -> 汇总 parity 结论`。
  3. Flow-PARITY-003（阻断判定）:
     `行为等价硬门禁未通过 -> 标记为 parity failed -> provider 保持 experimental；行为等价通过但 latency_class=B -> 仅允许受限试点，不进入默认体验`。
  4. Flow-PARITY-004（扩面准入）:
     `P0 行为等价通过 -> 扩到多轮记忆场景 -> 再扩到多 agent 并发；默认启用仍需单独满足 latency_class=A`。
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| Parity 场景集 | `scenario_id/tier/goal` | QA 选择场景集执行对标 | `pending -> running -> passed/failed` | 先单 agent，再多轮记忆，再多 agent | QA / producer 可审阅 |
| 行为质量对标 | `completion_rate/invalid_action_rate/retry_count` | 自动对比 builtin vs OpenClaw | `bench_done -> compared` | 使用同批 observation/seed | 只读指标 |
| 时延体感对标 | `relative_wait_gap_ms/latency_class` | 记录玩家可感知等待 | `sampled -> aggregated` | 先看与 builtin 的相对差值，再判定 `A/B/C` rollout class | 只读指标 |
| 记忆连续性对标 | `memory_hit_quality/context_drift_count` | 检查多轮行为是否连续 | `session_done -> reviewed` | 按对话/任务回合统计 | 只读指标 |
| 观测与恢复对标 | `trace_completeness/error_recoverability` | QA 校验能否解释问题并恢复 | `issue_seen -> diagnosed -> recovered/blocked` | 缺任一观测面视为失败 | QA 裁定 |
| 发布准入 | `parity_status/release_gate` | 仅 parity 通过才允许默认启用 | `experimental -> gated -> default_ready` | 任一阻断线触发则回退 experimental | producer 决策 |
- Acceptance Criteria:
  - AC-1: 文档定义“体验等价”的正式口径：以玩家可感知行为结果、等待体感、记忆连续性、错误恢复与可解释性为主，不要求内部实现完全一致。
  - AC-2: 文档定义至少 3 层 parity 场景：`P0 低频单 NPC`、`P1 多轮记忆/对话`、`P2 多 agent 并发`。
  - AC-3: 文档为每层场景给出行为等价硬门禁与发布/默认启用附加门槛；未通过时必须明确保持 `experimental`，不得默默视作“差不多可用”。
  - AC-4: 文档定义 builtin 与 OpenClaw 的对标指标与采样方法，避免不同输入条件导致比较失真。
  - AC-5: 文档要求 QA/producer 双视角输出 parity 结论：自动指标 + 主观试玩评分。
  - AC-6: 文档要求 `OpenClaw(Local HTTP)` 专题与 `Decision Provider` 专题后续任务都以 parity 为上线目标，而非仅以“接通”作为完成条件。
- Non-Goals:
  - 不要求 `OpenClaw` 与内置 agent 在内部 prompt、工具栈或 memory backend 上实现完全一致。
  - 不把高频战斗、经济关键路径在首轮 parity 中纳入必须通过范围。
  - 不在本专题直接定义 `OpenClaw` 的安装包分发与商业化方案。

## 3. AI System Requirements (If Applicable)
- Tool Requirements:
  - parity 评估必须可读取 `AgentDecisionTrace`、provider diagnostics、错误码与动作结果。
  - 若使用真实 `OpenClaw`，必须能导出最小 trace 摘要，以保证可解释性比较。
- Evaluation Strategy:
  - `test_tier_required`: fixture 对标、mock provider、自动指标比较、错误恢复演练。
  - `test_tier_full`: 真实 `OpenClaw(Local HTTP)` 低频 NPC + 多轮记忆试玩，输出自动指标与主观评分卡。

## 4. Technical Specifications
- Architecture Overview:
  - parity 评估层建立在 `Decision Provider` 与 `OpenClaw(Local HTTP)` 方案之上，不新增新的 provider 传输层。
  - 使用统一 `scenario fixture + observation log + trace collector + score aggregator` 对 builtin 和 OpenClaw 两条链路做对照。
- Integration Points:
  - `doc/world-simulator/llm/llm-decision-provider-standard-openclaw-feasibility-2026-03-12.prd.md`
  - `doc/world-simulator/llm/llm-openclaw-local-http-provider-integration-2026-03-12.prd.md`
  - `doc/world-simulator/prd/acceptance/openclaw-agent-parity-scenario-matrix-2026-03-12.md`
  - `doc/world-simulator/prd/acceptance/openclaw-agent-parity-score-card-2026-03-12.md`
  - `doc/world-simulator/prd/acceptance/openclaw-agent-parity-benchmark-protocol-2026-03-12.md`
  - `doc/world-simulator/prd/acceptance/openclaw-agent-parity-aggregation-template-2026-03-12.md`
- `crates/oasis7/src/simulator/agent.rs`
- `crates/oasis7/src/simulator/memory.rs`
- `crates/oasis7_proto/src/viewer.rs`
- Parity Scope Definition:
  - `P0 低频单 NPC`：移动、观察、对话、简单交互。
  - `P1 多轮记忆/对话`：跨 3~5 轮任务目标保持、失败后重试、上下文连续性。
  - `P2 多 agent 并发`：2~5 个低频 agent 并发，不要求高频战斗 parity。
- Metrics & Thresholds:
  - 行为等价硬门禁:
    - `completion_rate_gap <= 5pp`
    - `invalid_action_rate <= 3%` 且不超过 builtin 2 倍
    - `timeout_rate <= 2%`
    - `relative_wait_gap_median <= 5000ms`
    - `relative_wait_gap_p95 <= 8000ms`
    - `trace_completeness >= 95%`
    - `recoverable_error_resolution_rate >= 90%`
  - 发布/默认启用附加门槛:
    - `latency_class A (default-candidate)`: OpenClaw `median_extra_wait_ms <= 500ms` 且 `p95_extra_wait_ms <= 1500ms`
    - `latency_class B (experimental-only)`: OpenClaw `median_extra_wait_ms <= 15000ms` 且 `p95_extra_wait_ms <= 20000ms`
    - `latency_class C (blocked)`: 超出 `B` 的上限，不允许默认启用，也不允许进入更大范围扩面
- Edge Cases & Error Handling:
  - 若 builtin 和 OpenClaw 场景输入不一致，则该轮样本作废，不计入 parity 结论。
  - 若 `OpenClaw` 请求成功但返回持续 `Wait` 造成任务停滞，应计入 completion gap，而不是简单视作“无错误”。
  - 若 builtin 本身也慢于 `latency_class A`，不得据此直接判定 parity 失败；应先看 `relative_wait_gap` 是否满足行为等价硬门禁，再看 OpenClaw 是否仅可保留在 `experimental`。
  - 若用户主观评分与自动指标明显冲突，必须要求 `qa_engineer` 输出失败签名解释，不得只采信单一维度。
- Non-Functional Requirements:
  - NFR-1: parity 评估脚本必须支持固定 fixture / seed / timeout，以保证对比可复现。
  - NFR-2: 主观评分卡必须与自动指标一并归档，不允许只有截图结论没有数值。
  - NFR-3: parity 结果必须能追溯到具体 provider 版本、adapter 版本、协议版本与 `agent_profile`。
- Security & Privacy:
  - parity 采样不得输出用户本地 token、敏感 provider 配置或完整私密会话内容。

## 5. Risks & Roadmap
- Phased Rollout:
  - M1 (2026-03-12): 定义 parity 目标、指标与场景层级。
  - M2: 为 `P0` 场景落地自动对标与主观评分卡。
  - M3: `P0` 行为等价硬门禁通过后推进 `P1` 多轮记忆 parity；若仅达到 `latency_class B`，继续保持 `experimental`。
  - M4: `P1` 通过后推进 `P2` 多 agent 并发 parity。
  - M5: 仅当目标层级行为等价通过且 `latency_class` 达到 `A`，才允许扩大默认覆盖范围。
- Technical Risks:
  - 风险-1: 若 observation / trace 抽样不足，可能出现“指标看起来接近，体验仍明显不同”的误判。
  - 风险-2: OpenClaw 延迟与外部模型波动可能让 parity 难以长期稳定维持。
  - 风险-3: 若只做低频场景 parity，就过早宣称“整体等价”，会产生口径风险。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-WORLD_SIMULATOR-038 | TASK-WORLD_SIMULATOR-114 | `test_tier_required` | `./scripts/doc-governance-check.sh` | 模块文档入口、parity 目标建模、上线口径 |
| PRD-WORLD_SIMULATOR-038 | T1/T2/T3 | `test_tier_required` | fixture benchmark + score aggregation + error recovery drills | parity 指标、场景覆盖、失败签名 |
| PRD-WORLD_SIMULATOR-038 | TASK-WORLD_SIMULATOR-157 | `test_tier_required` | `./scripts/doc-governance-check.sh` + `fix3` evidence rescore | 分层 latency gate、行为等价复签、默认启用口径 |
| PRD-WORLD_SIMULATOR-038 | T4/T5 | `test_tier_full` | 真实 `OpenClaw(Local HTTP)` 对标试玩 + QA/producer 评分卡 | 体验等价结论、默认启用准入 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-PARITY-001 | 把“体验等价”定义为独立专题和上线门禁 | 仅保留“能接入/能运行”的技术目标 | 技术接通不等于玩家体验等价，必须有独立准入标准。 |
| DEC-PARITY-002 | parity 以玩家可感知结果和恢复体验为准 | 要求内部实现、prompt、memory 完全一致 | 产品目标是体验等价，不是实现完全同构。 |
| DEC-PARITY-003 | 分层推进 `P0 -> P1 -> P2` parity | 一次性宣称全量 agent parity | 分层更可验证，也更符合当前 OpenClaw 首期低频接入边界。 |
| DEC-PARITY-004 | 对真实在线 LLM provider 采用“行为等价硬门禁 + rollout latency_class”双层口径 | 继续用单一绝对时延阈值作为 parity fail | 在线模型时延存在天然波动；产品上应把“能否默认启用”与“是否已经行为等价”分开治理。 |
