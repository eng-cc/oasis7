# oasis7: 模拟玩家 persona 评审面板（2026-05-06）

- 对应设计文档: `doc/testing/governance/playability-simulated-player-persona-panel-2026-05-06.design.md`
- 对应项目管理文档: `doc/testing/governance/playability-simulated-player-persona-panel-2026-05-06.project.md`

审计轮次: 2

## 目标
- 把“agent 可以扮演多个不同风格的 player 视角”正式设计成一套可复用的 testing/governance 专题。
- 让内部 playability review 在不新增正式 `player` 角色的前提下，稳定补齐多种玩家主观反应假设，并作为 `L4A synthetic internal playability review` 的核心输入。

## 范围
- 覆盖 simulated player personas 的清单、输入输出 contract、触发方式、与标准角色 subagent 的协作边界。
- 不覆盖真实外部玩家招募、问卷平台或自动 orchestration 工具实现。

## 接口 / 数据
- 专题 PRD: `doc/testing/governance/playability-simulated-player-persona-panel-2026-05-06.prd.md`
- 专题设计文档: `doc/testing/governance/playability-simulated-player-persona-panel-2026-05-06.design.md`
- 专题项目管理文档: `doc/testing/governance/playability-simulated-player-persona-panel-2026-05-06.project.md`
- 上游专题:
  - `doc/testing/governance/playability-evidence-stack-2026-05-06.prd.md`
  - `doc/testing/governance/playability-subagent-review-system-2026-05-06.prd.md`
- 角色真值入口: `.agents/roles/*.md`

## 里程碑
- M1 (2026-05-06): 冻结 simulated player persona panel 的固定 persona 清单、输入输出 contract 与越权边界。
- M2: 若后续需要自动执行，再补 panel runbook / wrapper 层设计。

## 风险
- 若 persona 定义模糊，评审输出会退回泛泛而谈的“像玩家一样看看”。
- 若 persona panel 边界写不清，内部模拟视角会被误当成真实外部玩家验证。

## 1. Executive Summary
- Problem Statement: 标准角色 subagent 已经能覆盖专业判断，但它们不天然等于多样化玩家主观反应。若没有一层固定的 simulated player personas，团队还是会在“新手会不会迷路”“急性子玩家会不会立刻退出”“系统党会不会刷出无聊最优解”这些问题上临时想象，导致内部 playability review 缺乏可比较的玩家视角。
- Proposed Solution: 新增 `simulated player persona panel` 专题，把多种玩家风格设计成可复用的内部评审面板。它不新增正式组织角色，而是作为 `producer_system_designer` / `qa_engineer` / 相关角色 subagent 可调用的体验假设层，输出结构化 persona cards，供标准角色 subagent 汇总。
- Success Criteria:
  - SC-1: 至少定义 5 类固定 simulated personas，覆盖新手困惑、急性子行动派、系统优化派、叙事好奇派和混沌破坏派。
  - SC-2: 每个 persona 都有明确的输入、偏好/容忍度、必答问题、输出字段和不得越权边界。
  - SC-3: persona panel 被明确写成“内部体验假设层”，而不是新的正式 `.agents/roles/` 角色。
  - SC-4: 专题定义 persona panel 与标准角色 subagent 的协作方式、触发矩阵和 stop conditions。
  - SC-5: 专题明确 `persona simulation != 真实玩家验证`，不得替代 L5 外部信号。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`: 需要把“玩家可能怎么想”从临时脑补变成结构化内部假设。
  - `qa_engineer`: 需要知道 persona panel 能证明什么、不能证明什么，避免把模拟主观反应误当 blocker 真值。
  - `viewer_engineer`: 需要快速知道不同风格玩家在 UI / onboarding / feedback 上会卡在哪里。
  - `agent_engineer` / `runtime_engineer`: 需要判断不同玩家风格到底有没有真实杠杆，还是只是在看世界自己运转。
- User Scenarios & Frequency:
  - 新 onboarding / 首次体验 / retention 改动准备提交或发 PR 时。
  - 玩法争议集中在“有的人会觉得无聊/迷路/太慢/太吵”这类主观差异时。
  - 标准角色 review 已经齐全，但还需要补多种玩家视角假设时。
- User Stories:
  - PRD-TESTING-PERSONA-001: As a `producer_system_designer`, I want a reusable panel of simulated player personas, so that internal playability review can cover more than one player mindset.
  - PRD-TESTING-PERSONA-002: As a `qa_engineer`, I want persona outputs to be normalized and explicitly bounded, so that they remain hypotheses instead of fake validation.
  - PRD-TESTING-PERSONA-003: As a `viewer_engineer`, I want personas with distinct reading tolerance and feedback expectations, so that UI readability risks become concrete.
  - PRD-TESTING-PERSONA-004: As an implementation owner, I want a trigger matrix for when persona simulation is worth running, so that this panel does not become cargo-cult overhead.
  - PRD-TESTING-PERSONA-005: As a workflow owner, I want persona panel outputs to plug into role-based subagent review, so that internal review still closes through standard roles.
- Critical User Flows:
  1. `识别是否存在显著主观体验风险 -> 选择默认或定制 persona 子集 -> 组装 persona review packet`
  2. `并行执行多个 persona 模拟 -> 回收 persona cards -> 标记共性断点 / 风格分歧`
  3. `qa_engineer` / `producer_system_designer` / 命中的工程角色读取 persona cards -> 回写正式 role review card`
  4. `若需要更高信度 -> 从 `L4A` 升级到 `L4B structured human playtest` 或 L5 受控外部信号`

## 3. Persona Catalog
| persona_id | 核心风格 | 高敏感项 | 低容忍项 | 最关注的问题 | 默认使用场景 |
| --- | --- | --- | --- | --- | --- |
| `new_player_confused` | 首次进入、目标感弱、需要明确引导 | 首屏目标、下一步提示、反馈解释性 | 长文案、隐含规则、需要猜的交互 | “我现在到底该做什么？” | onboarding、首屏、首次 10 分钟 |
| `impatient_action_player` | 想立刻行动并看到结果 | 操作后反馈速度、结果可见性、节奏 | 等待、绕路、说明文字 | “我点了之后，世界马上因我而变了吗？” | retention、首个可用回路、快节奏玩法 |
| `systems_optimizer` | 会找最优解、刷法和漏洞 | 策略深度、可优化空间、资源 loop | 假选择、单一最优、无效 grind | “这里有没有值得我推敲的系统杠杆？” | 经济、资源、工艺、策略系统 |
| `narrative_curiosity_player` | 在意世界意义、角色动机和反馈语义 | 世界回应、后果解释、叙事 hooks | 冷冰冰反馈、无意义噪音 | “这个世界为什么值得我继续观察或介入？” | narrative、world feedback、agent drama |
| `chaos_tester` | 专门跳步骤、乱点、试边界 | 断路恢复、异常反馈、状态一致性 | 静默失败、假成功、软锁 | “如果我不按设计来，会发生什么坏事？” | 边界条件、恢复、容错、anti-softlock |

## 4. Persona Review Packet Contract
- Required fields:
  - `change_scope`
  - `target_experience_claim`
  - `formal_surfaces`
  - `artifact_paths`
  - `known_blockers`
  - `selected_personas`
  - `questions_to_probe`
- Optional fields:
  - `current_role_reviews`
  - `telemetry_hypothesis`
  - `outside_scope`
  - `expected_player_leverage`
- Persona card schema:
  - `persona_id`
  - `verdict`: `engaged/confused/bored/frustrated/exploitable`
  - `top_reactions`
  - `moment_of_dropoff`
  - `evidence_used`
  - `what_this_persona_does_not_prove`
  - `followup_questions`
  - `handoff_recommended_to`

## 5. Trigger Matrix
- Default-open:
  - 不默认强制每次都开 persona panel；只有当体验 claim 含明显主观差异风险时才建议开启。
- Open when changed surface includes:
  - onboarding / first-time comprehension / first-session retention:
    - `new_player_confused`
    - `impatient_action_player`
  - progression payoff / responsiveness / first reward loop:
    - `impatient_action_player`
  - economy / optimization / exploitability / dominant strategy:
    - `systems_optimizer`
    - `chaos_tester`
  - narrative hooks / world reactivity / agent meaning:
    - `narrative_curiosity_player`
  - recovery / edge-case navigation / softlock risk:
    - `chaos_tester`

## 6. Collaboration With Standard Role Subagents
- `producer_system_designer`:
  - 读取 persona panel 的共性断点和风格分歧，决定是否影响 `target_claim`。
- `qa_engineer`:
  - 判断 persona panel 是不是只提出假设，还是已经指出必须升级到真人试玩的高风险断点。
- `viewer_engineer`:
  - 吸收 `new_player_confused` / `impatient_action_player` / `narrative_curiosity_player` 的可读性和反馈问题。
- `agent_engineer`:
  - 吸收 `narrative_curiosity_player` / `impatient_action_player` 对 agent reactivity 的质疑。
- `runtime_engineer`:
  - 吸收 `chaos_tester` / `systems_optimizer` 暴露出的软锁、节奏塌陷或 contract 风险。
- `liveops_community`:
  - 只在需要准备对外说法时读取 persona panel 的 claim-risk，不把它当成外部反馈。

## 7. Escalation And Stop Conditions
- 必须立即升级到更高层验证的情况:
  - 多个 persona 都在同一关键节点给出 `confused` / `bored` / `frustrated`
  - `systems_optimizer` 或 `chaos_tester` 指出明显 exploit / dominant strategy / softlock 风险
  - `persona panel` 与标准角色 review 对“玩家是否拥有杠杆”得出相反结论
- 必须写明但不直接阻断的情况:
  - 只有单一 persona 负面，其余 persona 正常
  - persona 之间对同一节点出现明显风格分歧
- 永远不能越过的边界:
  - persona panel 只能输出内部体验假设与风险线索，不能写成“玩家已经验证喜欢”
  - persona panel 不能单独替代 `L4B structured human playtest` 或 L5 外部信号

## 8. Acceptance Criteria
- AC-1: 专题定义至少 5 类固定 persona，并写清各自偏好与低容忍项。
- AC-2: persona review packet 与 persona card schema 明确可复用。
- AC-3: persona panel 与标准角色 subagent 的协作方式、触发矩阵和升级条件写清。
- AC-4: 文档明确写出“persona panel 不是正式角色，不进入 `.agents/roles/`”。
- AC-5: 文档明确写出“persona simulation != 真实玩家验证”的硬边界。

## 9. Non-Goals
- 不在本轮新增正式 `player` 角色。
- 不把 persona panel 包装成外部用户研究平台。
- 不在本轮实现自动化执行器或评分模型。

## 10. Technical Specifications
- Integration Points:
  - `doc/testing/prd.md`
  - `doc/testing/project.md`
  - `doc/testing/governance/playability-evidence-stack-2026-05-06.prd.md`
  - `doc/testing/governance/playability-subagent-review-system-2026-05-06.prd.md`
  - `.agents/roles/*.md`
- Edge Cases:
  - 若 persona card 没有一手证据，只是转述 role review：必须标 `secondary_review_only`。
  - 若 selected personas 未覆盖本次改动最关键风险面：必须记为 `panel_incomplete`。
  - 若 persona panel 全正面，但没有任何真人试玩：仍只能记为 `L4A` 完成，不能宣称 `L4B` 已完成。
- Non-Functional Requirements:
  - NFR-SPP-1: 新读者应在 5 分钟内知道何时需要开 persona panel。
  - NFR-SPP-2: 每张 persona card 必须在 30 秒内看出“这个风格的玩家会在哪掉线”。
  - NFR-SPP-3: persona panel 的所有结论都必须能无歧义回接到标准角色 review。

## 11. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-TESTING-PERSONA-001 | `playability-simulated-player-persona-panel-2026-05-06` / `SPP-1` | `test_tier_required` | `rg` 检查 persona catalog 与边界定义 | 内部玩家视角设计完整性 |
| PRD-TESTING-PERSONA-002 | `playability-simulated-player-persona-panel-2026-05-06` / `SPP-1/2` | `test_tier_required` | 抽查 persona packet / card schema | 输入输出一致性 |
| PRD-TESTING-PERSONA-003 | `playability-simulated-player-persona-panel-2026-05-06` / `SPP-2` | `test_tier_required` | 抽查 trigger matrix 与 role handoff 规则 | persona panel 调度可执行性 |
| PRD-TESTING-PERSONA-004 | `playability-simulated-player-persona-panel-2026-05-06` / `SPP-2/3` | `test_tier_required` | 抽查非正式角色限制与 L4/L5 边界说明 | 内外部验证边界 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| `DEC-SPP-001` | 用 non-role persona panel 表达玩家视角 | 新增正式 `player` 角色 | 会破坏现有标准角色治理与 PM 约束。 |
| `DEC-SPP-002` | 固定 5 类高复用 persona | 每次临时想象不同玩家 | 临时脑补不可比较，也无法沉淀治理规范。 |
| `DEC-SPP-003` | persona 输出先汇入标准角色 review | persona panel 直接给最终放行结论 | 玩家主观模拟不应绕过 QA / producer / 工程角色收口。 |
| `DEC-SPP-004` | persona panel 只作为 `L4A` 内部体验假设层 | 把 persona simulation 当成真实玩家测试 | 证明强度不够，会污染 `L4B/L5` 边界。 |
