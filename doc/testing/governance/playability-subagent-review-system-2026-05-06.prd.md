# oasis7: 好玩性 subagent 评审系统（2026-05-06）

- 对应设计文档: `doc/testing/governance/playability-subagent-review-system-2026-05-06.design.md`
- 对应项目管理文档: `doc/testing/governance/playability-subagent-review-system-2026-05-06.project.md`

审计轮次: 3

## 目标
- 把“多角色内部人工评审可以由标准角色 subagent 补齐”落成一份可执行的 testing/governance 专题。
- 统一标准角色 subagent 的输入、输出、调度顺序、升级边界与 stop conditions。
- 明确它们如何接收并收口 `simulated player persona panel` 产出的内部玩家视角假设，并共同形成 `L4A synthetic internal playability review`。

## 范围
- 覆盖内部 playability review 所需的标准角色 subagent 设计、review packet contract、output card schema、与 simulated persona panel 的 handoff contract，以及 orchestration 规则。
- 覆盖当前 repo-local scaffold 入口如何稳定生成 review packet、role review cards 和最终 summary。
- 不覆盖真实外部玩家研究方法，也不覆盖“自动执行所有角色评审”的自治 orchestration 实现。

## 接口 / 数据
- 专题 PRD: `doc/testing/governance/playability-subagent-review-system-2026-05-06.prd.md`
- 专题设计文档: `doc/testing/governance/playability-subagent-review-system-2026-05-06.design.md`
- 专题项目管理文档: `doc/testing/governance/playability-subagent-review-system-2026-05-06.project.md`
- 上游专题: `doc/testing/governance/playability-evidence-stack-2026-05-06.prd.md`
- 关联专题: `doc/testing/governance/playability-simulated-player-persona-panel-2026-05-06.prd.md`
- 角色真值入口: `.agents/roles/*.md`

## 里程碑
- M1 (2026-05-06): 冻结标准角色 subagent 清单、输入输出 contract、orchestration 规则，以及 repo-local scaffold 入口。
- M2: 若后续需要更强自动化，再补“自动执行角色评审 / 自动聚合 verdict”的扩展设计。

## 风险
- 若角色输入输出不统一，多角色 review 仍会退回临时口头协作。
- 若 internal review 与 external validation 边界写不清，subagent 结论容易被误当成真实玩家验证。

## 1. Executive Summary
- Problem Statement: 现有 `playability evidence stack` 已明确“内部人工评审可以用标准角色 subagent 补齐”，但还缺一份真正可执行的设计来回答：到底要设计哪些 subagent、每个 subagent 看什么输入、输出什么结论、谁负责汇总、哪些环节必须并行、哪些冲突需要升级。如果没有这一层设计，团队仍会回到“临时找几个视角看看”的松散模式。
- Proposed Solution: 新增 `playability subagent review system` 专题，把多角色内部评审正式设计成一个可复用系统：固定 subagent 清单、输入包、输出卡、调用顺序、变更面触发矩阵、冲突升级规则与 stop conditions；若需要模拟多种玩家风格，再把 `simulated player persona panel` 作为非正式角色层接入，并统一由标准角色 subagent 收口。
- Success Criteria:
  - SC-1: 至少定义 `producer_system_designer`、`qa_engineer`、`viewer_engineer`、`agent_engineer`、`runtime_engineer`、`wasm_platform_engineer`、`liveops_community` 七类标准角色 subagent。
  - SC-2: 每个 subagent 都有明确的输入、关注问题、输出字段、不得越权边界和完成定义。
  - SC-3: 系统定义一份统一 review packet，让各角色 review 可以并行，但汇总口径一致。
  - SC-4: 系统定义“默认必开角色”和“按 changed surface 追加角色”的触发矩阵。
  - SC-5: 系统明确 `subagent review = 内部证据编排`，仍不替代 L5 真实外部验证。
  - SC-6: 系统明确 simulated persona panel 不是正式角色，只能作为标准角色 review 的辅助输入层，并共同形成 `L4A`。
  - SC-7: 当前仓库提供 `./scripts/prepare-playability-l4-review.sh`，能在单个 worktree 内稳定生成 review packet、七类 role review cards、summary 和推荐命令文件。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`: 需要把多角色内部评审收敛成一套固定流程，而不是每次靠口头协调。
  - `qa_engineer`: 需要知道自己作为守门 review 的输入和输出格式，并在多角色意见冲突时保持阻断权。
  - 工程 owner: 需要知道哪些 subagent 需要跑、跑完算不算 review 完成、还差什么外部证据。
  - `liveops_community`: 需要清楚自己何时介入，以及何时只能做口径/风险评审，不能越权替代真实玩家反馈。
- User Scenarios & Frequency:
  - 新的 gameplay / onboarding / retention 变更准备提交或发 PR 时。
  - 需要跨 `runtime` / `agent` / `viewer` / `qa` / `producer` 多角色一起给玩法结论时。
  - 发生“自动化通过但体验争议很大”的争论时。
- User Stories:
  - PRD-TESTING-SUBAGENT-001: As a `producer_system_designer`, I want a fixed set of standard review subagents, so that internal playability review stops being ad hoc.
  - PRD-TESTING-SUBAGENT-002: As a `qa_engineer`, I want a normalized review packet and output schema, so that parallel role reviews remain comparable.
  - PRD-TESTING-SUBAGENT-003: As an implementation owner, I want a changed-surface trigger matrix, so that I only run the necessary role subagents.
  - PRD-TESTING-SUBAGENT-004: As a stage owner, I want explicit escalation and stop conditions, so that review conflicts or missing external evidence are not hidden.
- Critical User Flows:
  1. 识别当前变更的玩家 surface / 风险面 -> 运行 `prepare-playability-l4-review.sh` 或按同一 contract 组装 review packet -> 选择默认 + 按需 subagent
  2. `若需要补多风格玩家假设 -> 先运行 simulated persona panel -> 回流 persona cards`
  3. `并行执行角色 subagent review -> 回收统一输出卡 -> 标记 blocker / watch / claim drift`
  4. `producer_system_designer` 汇总各卡 -> 输出 `L4A synthetic` 内部阶段结论 -> 判断是否仍缺 `L4B/L5`，以及是否需要补可选内部真人校准

## 3. Functional Specification Matrix
| subagent | 默认/按需 | 主要输入 | 必答问题 | 输出 | 不得越权 |
| --- | --- | --- | --- | --- | --- |
| `producer_system_designer` review subagent | 默认 | PRD、玩法目标、claim envelope、其余角色 review card | 这次改动到底想让玩家更想继续玩什么？现有证据能不能支撑这个说法？ | `claim_assessment`, `evidence_gap`, `go/watch/hold/block` 建议 | 不替代 QA 的失败签名判定 |
| `qa_engineer` review subagent | 默认 | 自动化结果、playability card、failure signatures、smoke/runbook 结果 | 这次改动到底是“没坏”还是“足够稳定可以继续收更高层证据”？ | `qa_verdict`, `blockers`, `required_reruns`, `evidence_quality` | 不拍板规则方向 |
| `viewer_engineer` review subagent | 触达 UI/首屏/反馈时必开 | 截图、录像、UI state、文案、交互路径 | 玩家能不能看懂当前目标、后果和下一步？UI 有没有把世界噪音误当成有用反馈？ | `ux_findings`, `readability_risk`, `player_feedback_gap` | 不改 runtime 语义 |
| `agent_engineer` review subagent | 触达 LLM/agent 行为时必开 | agent trace、prompt/decision evidence、world changes | 是玩家在间接驱动 agent 产生后果，还是 agent 在自己运转？ | `agent_behavior_verdict`, `player_influence_gap`, `drift_risk` | 不改世界规则 |
| `runtime_engineer` review subagent | 触达 runtime/progression/replay 时必开 | step/progression evidence、replay/recovery results、contracts | 体验问题是不是 runtime contract/推进语义失真导致的？ | `runtime_constraint_report`, `determinism_risk`, `contract_blocker` | 不替代玩法判断 |
| `wasm_platform_engineer` review subagent | 触达 wasm/module/ABI 时按需 | module manifests、ABI diff、determinism evidence | 这次玩法问题是不是平台/模块边界造成的假阳性或假阴性？ | `platform_risk`, `abi_drift`, `module_limitations` | 不替代 runtime 或 producer 结论 |
| `liveops_community` review subagent | 触达对外口径/preview 时必开 | release wording、preview plan、community signals | 这次内部结论能不能安全对外表达？需要什么 preview 边界？ | `claim_envelope_risk`, `preview_readiness`, `feedback_capture_plan` | 不把内部模拟当真实外部反馈 |

## 4. Review Packet Contract
- Required fields:
  - `change_scope`
  - `formal_surfaces`: `software_safe` / `pure_api` / others
  - `target_claim`
  - `automated_evidence`
  - `playability_evidence`
  - `known_blockers`
  - `requested_roles`
  - `outside_scope`
- Optional fields:
  - `telemetry_hypothesis`
  - `limited_preview_context`
  - `artifact_paths`
  - `open_questions`
  - `persona_panel_request`
  - `persona_cards`
  - `target_l4_lane`: `L4A_only` / `L4A_then_L4B`
- Output card schema:
  - `role_name`
  - `verdict`: `pass/watch/hold/block`
  - `top_findings`
  - `evidence_used`
  - `what_this_does_not_prove`
  - `required_followups`
  - `claim_boundary_note`
- Current repo-local scaffold:
  - `./scripts/prepare-playability-l4-review.sh` 会一次性生成：
    - `l4-review-packet.md`
    - `role-review-cards/*.md`
    - `l4-summary.md`
    - `commands.sh`
    - `manifest.json`
  - 该脚本只负责产物预置、路径冻结和 `L4A/L4B` 命令推荐，并预留可选内部真人校准 notes；不会自动替代各角色做 verdict。

## 5. Trigger Matrix
- Always-open:
  - `producer_system_designer`
  - `qa_engineer`
- Open when changed surface includes:
  - UI / onboarding / readability / player feedback wording:
    - `viewer_engineer`
  - LLM / NPC / indirect-control / agent progression:
    - `agent_engineer`
  - progression contract / replay / state transition / longrun stability:
    - `runtime_engineer`
  - wasm modules / ABI / provider module boundaries:
    - `wasm_platform_engineer`
  - public preview wording / liveops lane / community signal plan:
    - `liveops_community`
- Open simulated persona panel when:
  - 本次体验 claim 主要风险是理解差异、节奏差异、系统偏好差异或叙事偏好差异
  - 仅靠标准角色专业判断不足以覆盖多个玩家风格假设

## 6. Sequencing Rules
- Phase A: `qa_engineer` + `producer_system_designer` 先确认 review packet 是否完整。
- Phase B: 若命中主观体验差异风险，先运行 simulated persona panel，回收 persona cards。
- Phase C: 其余按 changed surface 命中的角色 subagent 并行执行，并吸收 persona cards。
- Phase D: `qa_engineer` 先做阻断归纳，`producer_system_designer` 再做 `L4A synthetic` 结论汇总。
- Phase E: 若需要对外说法或 preview，`liveops_community` 在最终汇总后做 claim-envelope 复核。

## 7. Escalation And Stop Conditions
- 必须立即 `stop` 的情况:
  - `qa_engineer=block`
  - 任一 formal surface 的基础 contract 不成立
  - review packet 缺关键证据路径，导致其它角色只能猜
- 必须升级但不直接阻断的情况:
  - `viewer` 与 `agent` 对“玩家是否有杠杆”结论冲突
  - simulated persona panel 与标准角色 review 对“玩家为何掉线”给出相反解释
  - `producer` 想升级 claim，但 `liveops` 认为对外口径不安全
  - `runtime` / `wasm` 指出当前体验结论建立在不稳定 contract 上
- 永远不能被内部 subagent 跳过的 stop line:
  - 缺真实外部信号时，不得把结论写成“已被真实玩家验证”
  - 缺 `L4B` agent 实玩时，不得把 `L4A` synthetic 结论写成“agent 已经真实玩过且想继续玩”
  - 缺 `L5` 真实人类 / 线上样本时，不得把 `L4B` 结论写成“真实玩家已经想继续玩”

## 8. Acceptance Criteria
- AC-1: 新专题定义七类标准角色 subagent。
- AC-2: 每个 subagent 都有输入、必答问题、输出、越权边界。
- AC-3: review packet contract 与 output card schema 明确可复用。
- AC-4: trigger matrix、sequencing rules、stop conditions 都被写清。
- AC-5: 明确“内部 subagent review 全补齐”与“L5 外部真实验证完成”是两回事。
- AC-6: 明确写出当前 repo-local scaffold 入口与其边界: 能稳定生成 packet/card/summary，但不会自动完成 role verdict。

## 9. Non-Goals
- 不在本轮实现自动 orchestration 脚本。
- 不新增 `.agents/roles/` 之外的正式角色。
- 不把这套系统误写成真实玩家研究平台。

## 10. Technical Specifications
- Integration Points:
  - `doc/testing/governance/playability-evidence-stack-2026-05-06.prd.md`
  - `doc/testing/governance/playability-simulated-player-persona-panel-2026-05-06.prd.md`
  - `.agents/roles/*.md`
  - `doc/testing/prd.md`
  - `doc/testing/project.md`
  - `testing-manual.md`
  - `scripts/prepare-playability-l4-review.sh`
  - `doc/testing/templates/playability-l4-review-packet-template.md`
  - `doc/testing/templates/playability-l4-role-review-card-template.md`
- Edge Cases:
  - 若某角色输入为空，但 trigger matrix 命中：必须在输出卡显式写 `insufficient_input`，不能假装评审完成。
  - 若某角色 review 只基于别的角色结论，没有一手证据：必须标记为 `secondary_review_only`。
  - 若 persona cards 已存在，但角色 review 完全忽略其共性断点：必须说明忽略理由，不能静默跳过。
  - 若多个 subagent 全部正面，但没有任何 `L4B/L5` 证据：只能给“`L4A synthetic` 准备完成”，不能给“玩法已被证明”。
  - 若 scaffold 已生成，但命中的 role cards 未填写完：只能记为 packet/card 预置完成，不能记为 subagent review 已完成。
- Non-Functional Requirements:
  - NFR-SUB-1: 新读者应在 5 分钟内知道本次改动要开哪些角色 subagent。
  - NFR-SUB-2: 每张输出卡必须在 30 秒内看出该角色到底证明了什么、没证明什么。
  - NFR-SUB-3: 系统设计必须只依赖标准角色名，兼容现有 execution log / handoff / PM 约束。

## 11. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-TESTING-SUBAGENT-001 | `playability-subagent-review-system-2026-05-06` / `PSRS-1` | `test_tier_required` | `rg` 检查七类角色 subagent 定义与边界 | 多角色 review 设计完整性 |
| PRD-TESTING-SUBAGENT-002 | `playability-subagent-review-system-2026-05-06` / `PSRS-1/2` | `test_tier_required` | 抽查 review packet/output card schema | review 输入输出一致性 |
| PRD-TESTING-SUBAGENT-003 | `playability-subagent-review-system-2026-05-06` / `PSRS-2` | `test_tier_required` | 抽查 trigger matrix 与 sequencing rules | review 调度可执行性 |
| PRD-TESTING-SUBAGENT-004 | `playability-subagent-review-system-2026-05-06` / `PSRS-2/3` | `test_tier_required` | 抽查 escalation/stop conditions 与 L5 边界说明 | 内外部验证边界 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| `DEC-PSRS-001` | 只设计标准角色 subagent | 额外创造 `player` 正式 subagent | 与仓库角色治理冲突。 |
| `DEC-PSRS-002` | 用统一 review packet + output card | 每个角色自由发挥输出格式 | 无法汇总、无法比较、无法自动化后续治理。 |
| `DEC-PSRS-003` | 先 `qa + producer` 校验 packet，再并行其余角色 | 所有角色一上来全并行 | 输入包若不完整，会放大噪音和重复劳动。 |
| `DEC-PSRS-004` | 保留 `liveops_community` 作为 claim-envelope 终段复核 | 让工程角色直接决定对外表达 | 工程结论和对外口径不是同一边界。 |
| `DEC-PSRS-005` | 把 simulated personas 作为非正式玩家视角层接入标准角色 review | 把 `player` 升格成新的正式角色 | 会破坏现有角色治理边界，也会混淆内部模拟与外部验证。 |
| `DEC-PSRS-006` | 当前先提供 repo-local scaffold 生成 packet/card/summary | 等自动 orchestration 全做完后再给 operator 入口 | 没有稳定 scaffold，当前 PR 仍无法在单个 worktree 内完整准备 `L4A`。 |
