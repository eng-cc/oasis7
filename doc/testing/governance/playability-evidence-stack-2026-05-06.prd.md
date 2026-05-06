# oasis7: 好玩性证据栈（2026-05-06）

- 对应设计文档: `doc/testing/governance/playability-evidence-stack-2026-05-06.design.md`
- 对应项目管理文档: `doc/testing/governance/playability-evidence-stack-2026-05-06.project.md`

审计轮次: 3

## 1. Executive Summary
- Problem Statement: 自动化测试已经能稳定覆盖回归、协议、性能、长稳和部分玩家路径，但“自动化绿灯”仍不等于“游戏已经好玩”。如果没有一套明确的证据栈把自动化、遥测、A/B、synthetic 角色扮演与真人试玩分层，团队很容易把“没坏”“世界在动”“agent 预测会继续玩”“真人真的想继续玩”混写成同一种结论。
- Proposed Solution: 建立 `playability evidence stack` 专题，正式定义 oasis7 的分层好玩性证据体系，明确每一层能证明什么、不能证明什么、如何组合成阶段性 go/hold/block 结论，以及当前仓库已有脚本/文档应挂在哪一层；其中当前 `L4` 正式拆成 `L4A synthetic internal playability review` 与 `L4B structured human playtest`。
- Success Criteria:
  - SC-1: 专题文档明确声明“没有单一自动化方案能够保证游戏好玩”，且给出 oasis7 的正式替代口径。
  - SC-2: 至少定义 `automation baseline / agent probe / telemetry & experiments / L4A synthetic internal playability review / L4B structured human playtests / limited preview live signals`，并为每层列出可证明与不可证明边界。
  - SC-3: `software_safe`、`pure_api`、`--no-llm observer/debug only`、`run-producer-playtest.sh`、playability card、`player leverage` rubric 和 limited preview 现有治理口径都被映射进同一套证据栈。
  - SC-4: 模块根入口 `doc/testing/prd.md` / `project.md` / `README.md` / `prd.index.md` 能把读者导向该专题。
  - SC-5: 专题文档明确声明“所有内部人工评审环节都可以优先由对应标准角色 subagent 补齐”，同时保留“这不等价于真实外部玩家验证”的硬边界。
  - SC-6: 专题文档明确声明 simulated player personas 与标准角色 subagents 属于 `L4A` 的核心输入，不新增正式角色，也不替代 `L4B` 真人试玩。
  - SC-7: 专题文档明确写出 `L4A != L4B`，并保留未来 calibration 扩展位，但当前不宣称 `L4A` 已可替代 `L4B`。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`: 需要知道哪些信号只是在证明“没坏”，哪些信号才足以支撑“值得继续玩”。
  - `qa_engineer`: 需要统一地给出 `pass/watch/block` 结论，而不是让每次 playability 讨论都临时换标准。
  - `runtime_engineer` / `viewer_engineer` / `agent_engineer`: 需要知道自己补的是“可靠性证据”还是“玩法证据”，避免做了自动化就误以为好玩问题已关闭。
  - `liveops_community`: 需要知道 limited preview 和真实玩家反馈在整套证据栈里的位置，不把少量外部正反馈误写成全面放行。
- User Scenarios & Frequency:
  - 新玩法或关键体验切片收口前：先判断当前证据只到哪一层。
  - 发布评审或阶段升级前：检查是否已经具备跨层证据组合，而不是只看 required/full。
  - 玩法争议复盘时：把“自动化已通过但人类觉得无聊”拆成可定位的问题。
  - 需要多人内部评审时：按 `qa_engineer` / `producer_system_designer` / `viewer_engineer` / `agent_engineer` / `liveops_community` 角色分别开 subagent 补齐内部评审意见。
  - 需要模拟多个玩家风格时：开启 `simulated player persona panel` 产出风格化体验假设，完成 `L4A`。
  - 需要回答“真人会不会继续玩”时：升级到 `L4B structured human playtest`。
- User Stories:
  - PRD-TESTING-PLAYABILITY-001: As a `producer_system_designer`, I want a canonical evidence stack for gameplay fun, so that I can make stage decisions without conflating reliability with fun.
  - PRD-TESTING-PLAYABILITY-002: As a `qa_engineer`, I want each evidence layer to have explicit proof boundaries, so that I can block overclaims early.
  - PRD-TESTING-PLAYABILITY-003: As an implementation owner, I want existing scripts and reports mapped into that stack, so that I know what evidence gap is still open.
  - PRD-TESTING-PLAYABILITY-004: As a release reviewer, I want a clear combination rule for go/hold/block, so that no single metric or single playtest overrides the rest of the stack.
  - PRD-TESTING-PLAYABILITY-005: As a workflow owner, I want each internal human-review step to be delegatable to the matching standard-role subagent, so that multi-role review can scale without weakening the evidence boundary.
  - PRD-TESTING-PLAYABILITY-006: As a gameplay reviewer, I want a reusable simulated player persona panel, so that internal review can test more than one player mindset without inventing new formal roles.
  - PRD-TESTING-PLAYABILITY-007: As a stage owner, I want `L4A` and `L4B` split explicitly, so that synthetic roleplay and human willingness claims stop being conflated.
- Critical User Flows:
  1. `识别体验目标 -> 选择对应玩家 surface -> 先跑自动化基线 -> 判断是否已具备继续收集更高层证据的前置条件`
  2. `收集 agent probe / telemetry / synthetic review / 真人试玩 / limited preview 信号 -> 填写统一 evidence packet -> 标记每层结论`
  3. `producer_system_designer` 汇总多层结论 -> 输出 `go/watch/hold/block`，并明确“当前只证明了什么”
  4. `识别需要人工内部评审的环节 -> 按标准角色开对应 subagent`
  5. `若需要多风格主观体验假设 -> 开 simulated player persona panel + 标准角色 subagent -> 形成 L4A`
  6. `若需要真人主观继续游玩证据 -> 执行 `run-producer-playtest.sh` + playability card -> 形成 L4B`
  7. `汇总各角色评审 -> 保留外部真实验证边界`
- Functional Specification Matrix:
| 证据层 | 主要输入 | 可以证明 | 不能证明 | oasis7 当前锚点 | 默认 owner |
| --- | --- | --- | --- | --- | --- |
| L1 自动化基线 | `required/full`、协议回归、Web 闭环脚本、长稳 smoke | 没坏、可重复、主链路能走通、阻断签名稳定可复现 | 玩家是否觉得有趣、是否愿意继续玩 | `testing-manual.md`、`scripts/ci-tests.sh`、`viewer-software-safe-step-regression.sh` | `qa_engineer` |
| L2 Agent/fixture probe | 脚本化 step/chat/progression、场景推进、受控 bot/fixture 探针 | 可达性、卡点、节奏断点、是否存在“玩家动作后世界无响应” | 情绪价值、审美、长期动机 | `player leverage` rubric、`world_activity_only`、`snapshot.player_gameplay` | `qa_engineer` + 实现 owner |
| L3 遥测与实验 | progression funnel、停留时长、回流率、A/B、行为事件 | 某方案是否比另一方案更好；玩家在哪些环节退出 | 指标提升是否真的等于“更好玩”；样本外原因解释 | 本专题先冻结字段与决策口径，不在本轮实现采集系统 | `qa_engineer` + `producer_system_designer` |
| L4A synthetic internal playability review | 标准角色 subagent review、simulated persona panel、formal player surface 上的 scripted/UI closure、synthetic continuation prediction | 以当前内部模型看，哪些风格玩家可能想继续玩、会在哪掉线、哪些 claim 只在 synthetic 里成立 | 真人在真实时间/耐心/机会成本下是否真的愿意继续玩 | `worktree-harness.sh`、S6 Web UI 闭环、role review cards、persona cards | `producer_system_designer` + `qa_engineer` |
| L4B structured human playtest | playability 卡片、制作人试玩、QA headed rerun、受控访谈 | 真实人类是否看得懂、是否感到有杠杆、是否想继续玩、阻塞点是否可解释 | 大规模外部市场反应 | `run-producer-playtest.sh`、`doc/playability_test_result/card_*.md` | `producer_system_designer` + `qa_engineer` |
| L5 受控外部信号 | limited preview、liveops 反馈、真实玩家 session | 在真实环境下，当前 claim envelope 是否成立 | 广泛市场成功、长期留存已被证明 | `technical preview` / limited preview 口径、liveops signal 回流 | `liveops_community` + `producer_system_designer` |
- Internal role-subagent mapping:
  - `qa_engineer` subagent: 负责回归、阻断、`player leverage` / `world_activity_only` 审查。
  - `producer_system_designer` subagent: 负责玩法目标、claim envelope 与“当前证据是否足以声称值得继续玩”。
  - `viewer_engineer` subagent: 负责交互可读性、首屏信息负担、反馈是否可见。
  - `agent_engineer` subagent: 负责 agent 行为是否真的支撑玩家体验，而不只是让世界自己运转。
  - `liveops_community` subagent: 负责 limited preview 口径、外部反馈归档与风险回流。
  - `runtime_engineer` / `wasm_platform_engineer` subagent: 负责实现约束、determinism、平台限制是否让高层体验结论失真。
- Simulated player persona panel:
  - `new_player_confused` / `impatient_action_player` / `systems_optimizer` / `narrative_curiosity_player` / `chaos_tester`
  - 作为 `L4A` 的核心输入之一，不是正式组织角色。
  - persona cards 必须先回流到标准角色 review，不能直接成为 `L4B` 或最终 stage verdict。
- Subagent governance rules:
  - 所有内部人工评审环节，默认都可以优先委托给对应标准角色 subagent。
  - 正式 execution log、handoff 和结论只允许使用 `.agents/roles/*.md` 中已存在的标准角色名，不新增 `player` 这类非标准角色。
  - 若需要“玩家视角”批评，应进入 `simulated player persona panel`，并继续由标准角色 subagent 收口，不得写成正式独立角色。
  - subagent + persona panel 可共同构成 `L4A`，但不得冒充 `L4B` 或 L5 真实外部信号。
- Layer rules:
  - L1/L2 是“能否继续验证”的前置层，不得单独给出“已证明好玩”。
  - L3 可以证明“方案 A 比方案 B 更有效”，但仍不能跳过 `L4A/L4B` 的主观体验判断。
  - `L4A` 是当前仓库内最强的 synthetic 内部判断层，可以表达“内部高强度模拟预测会继续玩”，但不等价于真人继续游玩意愿。
  - `L4B` 是当前仓库内最强的人类内部判断层，可以表达“真人实际继续游玩已被观察到”，但仍不自动等价于外部市场验证。
  - L5 只允许在受控 claim envelope 内升级信心，不允许把少量反馈写成“已完成普适验证”。
- Combination rules:
  - `block`: 任何一个 formal 玩家 surface 在 L1 就不稳定，或 L2 证明玩家动作没有稳定杠杆。
  - `hold`: L1-L3 通过，但 `L4A` 仍无法证明 synthetic continue，或 `L4B` 明确显示真人不想继续玩。
  - `watch`: `L4A` 基本成立，但 `L4B` 尚未执行、样本薄，或 `L4A/L4B` 对部分节点仍有分歧。
  - `go`: 只在目标 claim envelope 下，L1-L3 通过、`L4A` 与 `L4B` 都没有高价值反证，且 L5 没有出现新的高价值反证时给出。
- Current oasis7 policy bindings:
  - active LLM access 才是正式游玩 lane；`--no-llm` 只允许记为 observer/debug。
  - `software_safe` 与 `pure_api` 都属于 formal 玩家 surface，必须回答同一组 `snapshot.player_gameplay` 问题。
  - `world_activity_only=yes` 的样本不得支撑“玩家已有 meaningful participation”。
  - 即使自动化通过、世界时间推进，只要 `L4A/L4B` 仍不能证明玩家拥有稳定杠杆和继续动机，就不能把项目升级成“已证明好玩”。
  - 对应标准角色的 subagent 可以补齐所有内部 synthetic 评审环节，形成 `L4A`，但不能被记作真人 `L4B` 或真实外部玩家。
  - simulated player personas 只能帮助解释“哪类玩家可能掉线 / 困惑 / 无聊”，属于 `L4A`，不能替代 `L4B` 真人试玩卡片或外部会话。
- Acceptance Criteria:
  - AC-1: 专题文档明确写出 `L4A/L4B` 在内的证据栈与组合规则。
  - AC-2: 至少列出 `software_safe`、`pure_api`、`--no-llm`、`run-producer-playtest.sh`、playability card、`player leverage` rubric、limited preview 这 7 个现有锚点。
  - AC-3: 明确声明“自动化只能保证没坏/可回归，不能单独保证好玩”。
  - AC-4: `doc/testing/prd.md` 与 `doc/testing/project.md` 映射该专题，并给出模块级追踪条目。
  - AC-5: `doc/testing/README.md` 与 `doc/testing/prd.index.md` 把“如何判断自动化是否足以支撑好玩结论”的读者导向该专题。
  - AC-6: 明确写出标准角色 subagent 的适用范围、`player` 非标准角色限制，以及“subagent review != 真实外部玩家验证”的硬边界。
  - AC-7: 明确 simulated player persona panel 的定位、固定 persona 清单，以及其与 `L4A/L4B/L5` 的边界。
  - AC-8: 明确 `L4A` 与 `L4B` 的 operator 入口、claim 名称与当前非替代边界。
- Non-Goals:
  - 不在本轮实现新的遥测 SDK、实验平台或外部问卷系统。
  - 不把该专题写成某一个玩法切片的结果报告。
  - 不宣称当前仓库已经满足 `go`。
  - 不宣称当前 `L4A` 已可替代 `L4B`。
  - 不为内部评审新增 `player` 这类非标准正式角色。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 本专题是 testing/governance 层的判断框架，不直接新增测试代码；它把现有自动化、playability 文档、信任门/留存门、制作人试玩和 limited preview 信号统一到一个 evidence stack。
- Integration Points:
  - `doc/testing/prd.md`
  - `doc/testing/project.md`
  - `doc/testing/governance/playability-subagent-review-system-2026-05-06.prd.md`
  - `doc/testing/governance/playability-simulated-player-persona-panel-2026-05-06.prd.md`
  - `doc/testing/governance/playability-l4-synthetic-human-split-2026-05-06.prd.md`
  - `testing-manual.md`
  - `doc/testing/manual/web-ui-agent-browser-closure-manual.manual.md`
  - `doc/testing/evidence/gameplay-ten-minute-trust-gate-2026-04-09.md`
  - `doc/playability_test_result/README.md`
  - `doc/playability_test_result/playability_test_card.md`
  - `doc/game/prd.md`
  - `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.prd.md`
- Edge Cases & Error Handling:
  - 自动化全绿，但真人试玩仍觉得无聊或缺乏目标：必须记为 `L1 pass / L4A maybe-pass / L4B hold`，不能回退成“待观察的小问题”。
  - 世界很活跃，但玩家动作没有造成稳定可解释的后果：必须标记 `world_activity_only` 或 `player_leverage=block`。
  - 某个 A/B 指标更优，但真人试玩反馈更差：优先记为“L3 与 L4B 冲突”，要求补充解释，而不是直接按指标放行。
  - 少量外部正反馈与内部留存门冲突：仍以 formal lane 的门禁与 blocker 为准，外部反馈只作为 L5 旁证。
  - 多个角色 subagent 都给出正面结论，但还没有真实外部反馈：仍只能停留在内部证据完成，不得上抬成外部验证完成。
  - 多个 simulated personas 与 role subagents 都给出正面反应，但没有任何真人试玩：仍只能记为 `L4A pass / L4B missing`，不得替代 `L4B`。
- Non-Functional Requirements:
  - NFR-PES-1: 审查者必须能在 60 秒内看懂每层证据的证明边界。
  - NFR-PES-2: 所有正式玩法结论都必须能指出“当前到达了哪一层、还缺哪一层”。
  - NFR-PES-3: 模块根文档与该专题之间的互链必须可达，且通过 `doc-governance-check`。
- Security & Privacy: 真人试玩与外部信号的记录应继续遵循现有脱敏与最小化采集原则。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (`PES-1`): 建立五层证据栈、组合规则与 oasis7 当前锚点映射。
  - v1.1 (`PES-2`): 把正式 evidence packet 字段进一步补齐到 L3/L5，避免只有 narrative 没有层级结论。
  - v2.0 (`PES-3`): 视需要再引入实验/遥测自动汇总，但仍保持 `L4B/L5` 不被自动化替代。
  - v2.1 (`PES-4`): 若未来建立 calibration ledger，再讨论是否提升 `L4A` 的决策权重。
- Technical Risks:
  - 风险-1: 团队可能继续把“自动化全绿”简写成“已经好玩”，导致该专题只写不执行。
  - 风险-2: 若 L3 迟迟没有统一字段，后续仍会靠零散指标做过度解释。
  - 风险-3: 若 `L4B` 真人试玩卡片质量不稳，证据栈会在最关键一层失真。
  - 风险-4: 若团队偷换 `L4A` 和 `L4B` 的用词，split 会流于纸面。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-TESTING-PLAYABILITY-001 | `playability-evidence-stack-2026-05-06` / `PES-1` | `test_tier_required` | `rg` 检查五层证据、自动化边界与组合规则 | testing/governance 玩法结论口径 |
| PRD-TESTING-PLAYABILITY-002 | `playability-evidence-stack-2026-05-06` / `PES-1/2` | `test_tier_required` | 抽查 `software_safe` / `pure_api` / `--no-llm` / `player leverage` / limited preview 是否已映射 | formal surface 与门禁边界 |
| PRD-TESTING-PLAYABILITY-003 | `playability-evidence-stack-2026-05-06` / `PES-1/2` | `test_tier_required` | 检查模块根入口、索引与专题互链 | 文档导航与追溯一致性 |
| PRD-TESTING-PLAYABILITY-004 | `playability-evidence-stack-2026-05-06` / `PES-2/3` | `test_tier_required` | 抽样检查 project/README/prd.index/current window summary 是否同步 | 模块级治理执行力 |
| PRD-TESTING-PLAYABILITY-005 | `playability-evidence-stack-2026-05-06` / `PES-3/4` | `test_tier_required` | 抽查标准角色 subagent 映射、非标准 `player` 限制与 L5 边界说明 | 多角色内部评审治理边界 |
| PRD-TESTING-PLAYABILITY-006 | `playability-evidence-stack-2026-05-06` / `PES-4` | `test_tier_required` | 抽查 simulated persona panel 定位、persona 清单与 L4-supporting 边界 | 内部玩家视角治理边界 |
| PRD-TESTING-PLAYABILITY-007 | `playability-evidence-stack-2026-05-06` / `PES-4/5` | `test_tier_required` | 抽查 `L4A/L4B` 分层、operator 入口与非替代边界 | synthetic/human 证据拆分 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| `DEC-PES-001` | 明确写成“没有单一自动化方案能保证好玩” | 把自动化继续包装成足以证明玩法质量 | 这会持续混淆可靠性结论和玩法结论。 |
| `DEC-PES-002` | 用五层证据栈表达从内部到外部、从客观到主观的递进关系 | 把所有信号平铺成同权 checklist | 平铺 checklist 容易让低层证据越权替代高层证据。 |
| `DEC-PES-003` | 保留 L4 真人试玩作为当前仓内最高权重内部判断层 | 试图用 L3 实验或 L2 bot probe 替代人类体验判断 | 当前工具链可以辅助判断，但不能代替“玩家是否觉得值得继续玩”。 |
| `DEC-PES-004` | 所有内部人工评审默认可委托给对应标准角色 subagent | 为每个“玩家视角”额外创造非标准正式角色 | 当前仓库已有标准角色体系，新增非标准角色会破坏 execution log / handoff / PM 约束。 |
| `DEC-PES-005` | simulated personas 只作为 L4-supporting 的内部假设面板 | 把 simulated persona panel 升格为独立证据层或正式角色 | 这样会模糊 persona 假设与真人试玩之间的证明强度差异。 |
| `DEC-PES-006` | 把 `L4` 拆成 `L4A synthetic` 与 `L4B human` | 继续把 synthetic 与 human 共用一个 `L4` 标签 | 不拆层会持续混淆“agent 预测继续玩”和“真人实际继续玩”。 |
