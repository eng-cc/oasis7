# Gameplay 受控 Limited Preview 执行（2026-03-22） PRD v0.1

- 对应设计文档: `doc/game/gameplay/gameplay-limited-preview-execution-2026-03-22.design.md`
- 对应项目管理文档: `doc/game/gameplay/gameplay-limited-preview-execution-2026-03-22.project.md`

审计轮次: 1

## 1. Executive Summary

- Problem Statement: 当前 unified gate 已 `pass`，且对外 claim envelope 已收口为 `limited playable technical preview`，但团队还没有跑过一轮真实、受控、可回流的 limited preview 执行闭环，因此制作人仍缺少“这层口径在真实外部信号中是否稳定”的运营与质量证据。
- Proposed Solution: 新增 `PRD-GAME-010`，定义一轮 controlled builder-facing、可纠偏、可回流的 limited preview 执行流程；主信号面可采用经 `producer_system_designer` 批准的 GitHub issue 线程或其他 builder channel，把 `liveops_community` 的外放、`qa_engineer` 的 gate watch 与 `producer_system_designer` 的继续/暂停决策收成一条正式执行链。
- Success Criteria:
  - SC-1: `liveops_community` 完成 1 轮 controlled builder-facing callout，并显式使用 `limited playable technical preview` 与 GitHub CTA 口径。
  - SC-2: 本轮执行至少收集 3 条有效外部信号，并至少有 1 条进入 GitHub issue/PR 或内部 owner 跟进。
  - SC-3: 对外高可见度渠道中不得持续扩散 `closed beta / play now / live now / public launch` 误解；若出现误解，必须在同轮巡检中被纠偏。
  - SC-4: `qa_engineer` 必须基于 unified gate 当前真值、最近 7 天趋势和本轮回流信号，给出 `go / conditional go / no-go` 的持续守门结论。
  - SC-5: `producer_system_designer` 必须在本轮执行后形成继续维持、收紧节奏或触发下一轮阶段评审的正式裁决。

## 2. User Experience & Functionality

- User Personas:
  - `producer_system_designer`: 需要把“limited playable technical preview”从文案收口推进为一轮可审计的真实执行。
  - `liveops_community`: 需要在不越界的前提下做一次受控 builder 外放，并把信号完整回流。
  - `qa_engineer`: 需要证明当前受控预览链路在真实执行后仍未退化，而不是只保留静态 gate `pass`。
  - 外部 builder / tester: 需要一个明确、受控、非“公开上线式”承诺的反馈路径。
- User Scenarios & Frequency:
  - 受控外放执行：当前口径切换后先执行 1 轮。
  - 巡检与纠偏：每次外放至少覆盖 `T+15m / T+1h / T+4h / T+24h`。
  - QA 守门复核：每轮 limited preview 执行至少 1 次。
  - 制作人复盘：每轮 limited preview 执行后至少 1 次。
- User Stories:
  - PRD-GAME-010: As a 制作人与 limited preview owner, I want one controlled external execution loop, so that the team can validate the new claim envelope with real feedback before any broader stage or messaging move.
- Critical User Flows:
  1. Flow-LTP-001: `producer_system_designer` 冻结本轮 limited preview 目标、禁语与样本要求 -> 发起 `liveops_community` 与 `qa_engineer` handoff
  2. Flow-LTP-002: `liveops_community` 发起 controlled builder-facing thread（例如 GitHub issue） -> 按固定窗口巡检 -> 收集 signal / claim drift / incident
  3. Flow-LTP-003: 外部信号进入 `Blocking / Opportunity / Idea` 三类归档 -> 分别回流 `qa_engineer`、对应工程 owner、`producer_system_designer`
  4. Flow-LTP-004: `qa_engineer` 基于 unified gate 当前真值 + 本轮信号形成 `go / conditional go / no-go`
  5. Flow-LTP-005: `producer_system_designer` 读取 liveops 摘要与 QA 守门结论 -> 决定继续维持、收紧节奏或触发下一轮阶段评审
- Functional Specification Matrix:

| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 受控外放 callout | `callout_id`、`channel`、`message_status`、`candidate_id`、`cta_type` | 发布 controlled builder-facing callout（可采用 GitHub issue 线程），并绑定 GitHub 反馈出口 | `planned -> published -> monitored -> summarized` | 先用 approved 文案，再做巡检与回写 | `liveops_community` 执行，`producer_system_designer` 审核边界 |
| 信号归档与纠偏 | `signal_type`、`severity`、`claim_drift`、`owner`、`next_action` | 记录 comment/DM/issue/incident，并对误导性说法做纠偏 | `observed -> categorized -> escalated -> logged` | `Blocking` 优先于 `Opportunity`，`Opportunity` 优先于 `Idea` | `liveops_community` 归档，`producer_system_designer` 裁决高风险口径 |
| QA 持续守门 | `gate_status`、`lane_status`、`trend_window`、`failure_signature`、`recommendation` | 基于统一 gate 与新增信号输出 `go / conditional go / no-go` | `pass -> conditional_go -> block` | 任一硬 lane 回退即整体 `block` | `qa_engineer` 独立给阻断建议，`producer_system_designer` 最终决策 |
| 制作人复盘决策 | `preview_round_status`、`claim_envelope`、`signal_quality`、`next_step` | 读 liveops/QA 摘要后决定继续、收紧或升级评审 | `ready_to_run -> running -> reviewed -> continue/hold/reassess` | 先看 gate，再看 claim drift，再看信号质量 | 仅 `producer_system_designer` 可修改正式结论 |

- Acceptance Criteria:
  - AC-1: 本轮 limited preview 必须是 controlled builder-facing、受控执行，而不是“公开上线 / 公开宣发 / 广泛玩家开放”。
  - AC-2: 外放文案只允许使用 `limited playable technical preview`、`candidate access`、`builder feedback`、`GitHub issue/PR CTA` 一类表述。
  - AC-3: 任一高可见度渠道若出现 `closed beta / play now / live now / public launch / official integration announced` 一类误导性说法，必须在同轮巡检中被纠偏并留痕。
  - AC-4: 本轮执行至少沉淀 3 条有效信号，并按 `Blocking / Opportunity / Idea` 归档，且每条都具备 owner 与 next action。
  - AC-5: `qa_engineer` 必须输出标准化 `QA Weekly / Event Verdict`，覆盖 candidate ID、lane summary、trend 状态、新失败签名与 recommendation。
  - AC-6: 若 unified gate 任一硬 lane 回退、趋势跌破阈值或真实外部反馈出现可复现 blocking 问题，则本轮结论必须允许 `qa_engineer` 建议把 gate 从 `pass` 打回 `block`。
  - AC-7: `producer_system_designer` 必须在本轮执行后给出正式结论：继续维持 limited preview、收紧节奏，或触发下一轮阶段评审。
  - AC-8: 任务拆解、handoff、回流模板与 devlog 记录必须能追溯到 `PRD-GAME-010`。
- Non-Goals:
  - 不在本专题中直接升级到 `closed_beta_candidate` 或 `closed beta` 阶段。
  - 不把本轮执行等同为公开玩家开放、公开市场宣发或正式平台集成宣布。
  - 不在本轮新增新的 runtime / viewer / pure API 功能需求；本轮重点是执行、守门与回流。

## 3. AI System Requirements (If Applicable)

- Tool Requirements:
  - `doc/readme/governance/readme-closed-beta-candidate-runbook-2026-03-22.prd.md`
  - `doc/playability_test_result/templates/closed-beta-candidate-feedback-log-guide-2026-03-22.md`
  - `doc/playability_test_result/templates/closed-beta-candidate-incident-templates-2026-03-22.md`
  - `doc/testing/evidence/closed-beta-candidate-release-gate-2026-03-22.md`
- Evaluation Strategy:
  - 用“外放是否受控”“误导性说法是否被及时纠偏”“信号是否可回流”“QA 是否仍维持 gate 守门有效”四条线共同评估，不以曝光量或互动量作为成功主指标。

## 4. Technical Specifications

- Architecture Overview:
  - `PRD-GAME-010` 不新增玩法规则，而是定义一轮受控 limited preview 的执行治理层。
  - `liveops_community` 负责执行 controlled builder-facing callout、巡检、分级和回流。
  - `qa_engineer` 负责把本轮真实反馈与当前 unified gate 持续绑定，必要时建议回退为 `block`。
  - `producer_system_designer` 负责冻结 claim envelope、审阅高风险信号并给出 continue / hold / reassess 结论。
- Integration Points:
  - `doc/game/prd.md`
  - `doc/game/project.md`
  - `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.prd.md`
  - `doc/testing/evidence/closed-beta-candidate-release-gate-2026-03-22.md`
  - `doc/readme/governance/readme-closed-beta-candidate-runbook-2026-03-22.prd.md`
  - `doc/readme/governance/readme-moltbook-liveops-runbook-2026-03-21.md`
  - `doc/playability_test_result/templates/closed-beta-candidate-feedback-log-guide-2026-03-22.md`
  - `doc/playability_test_result/templates/closed-beta-candidate-incident-templates-2026-03-22.md`
- Edge Cases & Error Handling:
  - 外部讨论将当前状态误解为“大范围公开可玩”：必须立即纠偏，并以 claim drift 记录。
  - 首批反馈不足 3 条：允许延长 1 轮观察窗口，但不得伪造“反馈充足”结论。
  - 首批信号全部是低价值情绪互动：可继续观察，但 producer 不得据此宣称 limited preview 已验证成功。
  - 反馈暴露 candidate 主链路 blocking 问题：`qa_engineer` 必须评估是否将 unified gate 打回 `block`。
  - 单条高可见度误导性声明未被当轮巡检纠正：本轮视为 liveops 失控信号，不得继续扩大外放。
- Non-Functional Requirements:
  - NFR-LTP-1: 本轮 external callout 100% 使用 approved claim envelope，不得出现 `closed beta / play now / live now / public launch / official integration announced`。
  - NFR-LTP-2: 每一条有效外部信号都必须在同日归档到统一模板，并含 owner 与 next action。
  - NFR-LTP-3: claim drift 类高风险误解必须在同轮巡检中完成纠偏。
  - NFR-LTP-4: `qa_engineer` 的守门结论必须显式覆盖 5 条 lane、趋势窗口与新增 failure signature。
  - NFR-LTP-5: 本轮执行结论、任务状态与日志必须在 1 个工作日内同步到 game 根入口与 `devlog`。
- Security & Privacy:
  - 外部 builder 反馈与 incident 摘要遵循最小披露原则，不在公开渠道暴露内部世界状态、未公开候选细节或敏感环境信息。

## 5. Risks & Roadmap

- Phased Rollout:
  - MVP: 建立 `PRD-GAME-010`、handoff 与任务拆解，冻结受控执行边界。
  - v1.1: 完成第 1 轮 controlled builder-facing callout、巡检、反馈归档与 QA 守门结论。
  - v2.0: 基于第 1 轮真实样本，决定继续维持 limited preview、收紧节奏，还是触发下一轮阶段评审。
- Technical Risks:
  - 风险-1: 口径虽然改为 `limited playable technical preview`，但真实执行仍可能被外部理解为“已经上线”。
  - 风险-2: 当前 unified gate `pass` 可能因真实反馈暴露新的主链路问题而回退。
  - 风险-3: 团队若把 limited preview 执行误当成阶段升级，会提前释放超出当前成熟度的承诺。

## 6. Validation & Decision Record

- Test Plan & Traceability:

| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-GAME-010 | `TASK-GAME-035` | `test_tier_required` | 文档治理检查、根入口/索引/handoff/devlog 互链核验 | 受控 limited preview 执行治理层 |
| PRD-GAME-010 | `TASK-GAME-036` | `test_tier_required` | callout 文案边界检查、反馈/incident 模板回填、same-day devlog 回写 | liveops 执行、claim drift、反馈回流 |
| PRD-GAME-010 | `TASK-GAME-037` | `test_tier_required` | unified gate watch、趋势窗口更新、`QA Weekly / Event Verdict` 核验 | QA 守门、失败回退、继续/暂停建议 |
| PRD-GAME-010 | `TASK-GAME-038` | `test_tier_required` | producer 复盘结论、go/hold/reassess 决策与项目状态回写 | 下一轮阶段判断与受控预览节奏 |

- Decision Log:

| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-LTP-001 | 新增独立 `PRD-GAME-010` 作为受控 limited preview 执行专题 | 继续把执行动作塞回 `PRD-GAME-009` 既有准入专题 | `PRD-GAME-009` 已完成阶段准入治理；执行轮需要独立目标、失败信号与完成定义。 |
| DEC-LTP-002 | 先跑 1 轮 controlled builder-facing、可回流的外放，而不是直接扩大对外承接 | 在没有真实执行样本的情况下继续扩大 claim envelope | 当前需要先证明“口径在真实世界能否稳定成立”，而不是追求更大声量。 |
| DEC-LTP-004 | round-1 primary channel 可按执行真值切到 GitHub issue 线程 | 把“受控执行”狭义绑定为单一外部平台或继续受 Moltbook 可达性阻断 | 用户已明确要求把 round-1 渠道切到 GitHub issue；设计真值应收口为“controlled builder-facing channel”，而不是继续把平台选型写死。 |
| DEC-LTP-003 | 保持阶段不变，由 QA 做持续守门人 | 将 unified gate `pass` 直接解释为“可自动升级阶段” | gate `pass` 仅说明技术门收口，不代表真实 limited preview 执行已验证成功。 |
