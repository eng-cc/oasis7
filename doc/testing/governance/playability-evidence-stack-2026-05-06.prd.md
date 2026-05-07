# oasis7: 好玩性证据栈（2026-05-06）

- 对应设计文档: `doc/testing/governance/playability-evidence-stack-2026-05-06.design.md`
- 对应项目管理文档: `doc/testing/governance/playability-evidence-stack-2026-05-06.project.md`

审计轮次: 4

## 目标
- 建立 oasis7 的正式好玩性证据栈，明确每层能证明什么、不能证明什么。
- 把 `L4` 正式收口为 `L4A synthetic`、`L4B embodied agent` 两层，并把内部真人试玩降为 `L4B` 可选校准。
- 为当前仓库脚本、卡片、subagent 评审和外部信号提供统一映射口径。

## 范围
- 覆盖自动化、agent probe、遥测/实验、synthetic review、agent 实玩、内部真人校准、limited preview 信号的组合规则。
- 覆盖 `software_safe`、`pure_api`、`run-producer-playtest.sh`、`prepare-playability-l4-review.sh`、`run-playability-l4b-agent.sh` 等现有仓库入口。
- 不覆盖新的遥测 SDK、外部招募平台或“自动替代真人”的自治系统实现。

## 接口 / 数据
- 专题 PRD: `doc/testing/governance/playability-evidence-stack-2026-05-06.prd.md`
- 专题设计文档: `doc/testing/governance/playability-evidence-stack-2026-05-06.design.md`
- 专题项目管理文档: `doc/testing/governance/playability-evidence-stack-2026-05-06.project.md`
- 关联专题:
  - `doc/testing/governance/playability-subagent-review-system-2026-05-06.prd.md`
  - `doc/testing/governance/playability-simulated-player-persona-panel-2026-05-06.prd.md`
  - `doc/testing/governance/playability-l4-synthetic-human-split-2026-05-06.prd.md`
- operator 入口:
  - `testing-manual.md`
  - `scripts/prepare-playability-l4-review.sh`
  - `scripts/run-producer-playtest.sh`
  - `scripts/run-playability-l4b-agent.sh`

## 里程碑
- M1 (2026-05-06): 冻结证据层级、组合规则与当前脚本映射。
- M2: 继续补齐 evidence packet 字段与更严格的层级回填。
- M3: 若需要，再引入 calibration ledger 讨论低层到高层的权重升级。

## 风险
- 团队继续把“自动化全绿”简写成“已经好玩”。
- 团队继续混用 `L4A/L4B/L5` 的术语，导致 release/stage 结论失真。
- `L4B` 卡片或其可选内部真人校准质量不稳，导致高层证据失真。

## 1. Executive Summary
- Problem Statement: 自动化测试已经能稳定覆盖回归、协议、性能、长稳和部分玩家路径，但“自动化绿灯”仍不等于“游戏已经好玩”。如果没有一套明确的证据栈把自动化、遥测、A/B、synthetic 角色扮演、agent 实际试玩，以及真实人类 / 真实环境验证分层，团队很容易把“没坏”“世界在动”“agent 预测会继续玩”“agent 真玩了一轮”“真人真的想继续玩”混写成同一种结论。
- Proposed Solution: 建立 `playability evidence stack` 专题，正式定义 oasis7 的分层好玩性证据体系，明确每一层能证明什么、不能证明什么、如何组合成阶段性 `go/watch/hold/block` 结论，以及当前仓库已有脚本/文档应挂在哪一层；其中当前 `L4` 正式收口为 `L4A synthetic internal playability review` 与 `L4B embodied agent playtest`，内部真人试玩只作为 `L4B` 的可选校准证据，`L5` 保留给真实人类 / 受控外部信号。
- Success Criteria:
  - SC-1: 专题文档明确声明“没有单一自动化方案能够保证游戏好玩”。
  - SC-2: 至少定义 `automation baseline / agent probe / telemetry & experiments / L4A synthetic / L4B embodied agent / limited preview live signals`，并为每层列出可证明与不可证明边界。
  - SC-3: `software_safe`、`pure_api`、`--no-llm observer/debug only`、`run-producer-playtest.sh`、playability card、`player leverage` rubric，以及 limited preview 现有治理口径都被映射进同一套证据栈。
  - SC-4: 模块根入口 `doc/testing/prd.md` / `project.md` / `README.md` / `prd.index.md` 能把读者导向该专题。
  - SC-5: 专题文档明确声明“所有内部评审环节都可以优先由对应标准角色 subagent 补齐”，同时保留“这不等价于真实外部玩家验证”的硬边界。
  - SC-6: 专题文档明确声明 simulated player personas 与标准角色 subagents 属于 `L4A` 的核心输入，不新增正式角色，也不替代 `L4B` 或 `L5`。
  - SC-7: 专题文档明确写出 `L4A != L4B != L5`，并保留未来 calibration 扩展位，但当前不宣称低层已可替代高层。
  - SC-8: 专题文档明确当前已经提供 repo-local `L4` scaffold 入口 `./scripts/prepare-playability-l4-review.sh`，可以在单个 worktree 内生成 `L4A` packet / role cards / persona cards / `L4B` agent 卡副本 / 可选内部真人佐证 notes / summary。
  - SC-9: 专题文档明确当前已经提供 repo-local `L4B` 执行入口 `./scripts/run-playability-l4b-agent.sh`，可以在同一 artifact 目录内实际执行 embodied-agent run 并落盘状态/截图/summary/card prefill。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`: 需要知道哪些信号只是在证明“没坏”，哪些信号才足以支撑“值得继续玩”。
  - `qa_engineer`: 需要统一地给出 `pass/watch/block` 结论，而不是让每次 playability 讨论都临时换标准。
  - `runtime_engineer` / `viewer_engineer` / `agent_engineer`: 需要知道自己补的是“可靠性证据”还是“玩法证据”。
  - `liveops_community`: 需要知道 limited preview 和真实玩家反馈在整套证据栈里的位置，不把少量外部正反馈误写成全面放行。
- User Stories:
  - PRD-TESTING-PLAYABILITY-001: As a `producer_system_designer`, I want a canonical evidence stack for gameplay fun, so that I can make stage decisions without conflating reliability with fun.
  - PRD-TESTING-PLAYABILITY-002: As a `qa_engineer`, I want each evidence layer to have explicit proof boundaries, so that I can block overclaims early.
  - PRD-TESTING-PLAYABILITY-003: As an implementation owner, I want existing scripts and reports mapped into that stack, so that I know what evidence gap is still open.
  - PRD-TESTING-PLAYABILITY-004: As a release reviewer, I want a clear combination rule for `go/watch/hold/block`, so that no single metric or single playtest overrides the rest of the stack.
  - PRD-TESTING-PLAYABILITY-005: As a workflow owner, I want each internal review step to be delegatable to the matching standard-role subagent, so that multi-role review can scale without weakening the evidence boundary.
  - PRD-TESTING-PLAYABILITY-006: As a gameplay reviewer, I want a reusable simulated player persona panel, so that internal review can test more than one player mindset without inventing new formal roles.
  - PRD-TESTING-PLAYABILITY-007: As a stage owner, I want `L4A/L4B/L5` boundaries written explicitly, so that synthetic roleplay, embodied agent play, and real-human willingness claims stop being conflated.
- Critical User Flows:
  1. `识别体验目标 -> 选择对应玩家 surface -> 先跑自动化基线 -> 判断是否已具备继续收集更高层证据的前置条件`
  2. `收集 agent probe / telemetry / synthetic review / embodied agent playtest / human playtest / limited preview 信号 -> 填写统一 evidence packet -> 标记每层结论`
  3. `producer_system_designer` 汇总多层结论 -> 输出 `go/watch/hold/block`，并明确“当前只证明了什么”`
  4. `识别需要内部评审的环节 -> 按标准角色开对应 subagent`
  5. `若需要多风格主观体验假设 -> 开 simulated player persona panel + 标准角色 subagent -> 形成 L4A`
  6. `若需要 agent 真的进游戏操作一轮 -> 执行 run-playability-l4b-agent.sh（内部调用 run-producer-playtest）+ agent card -> 形成 L4B`
  7. `若需要内部人类校准 -> 复用同一入口，把结果写成 L4B corroboration / contradiction，而不是新层`
  8. `若需要真实人类 / 真实环境证据 -> 升级到 L5`
  9. `若需要一轮完整 L4 -> 先运行 prepare-playability-l4-review.sh 生成 packet / cards / summary scaffold，再把 L4A/L4B 证据收口到同一 artifact 目录`

## 3. Functional Specification Matrix
| 证据层 | 主要输入 | 可以证明 | 不能证明 | oasis7 当前锚点 | 默认 owner |
| --- | --- | --- | --- | --- | --- |
| L1 自动化基线 | `required/full`、协议回归、Web 闭环脚本、长稳 smoke | 没坏、可重复、主链路能走通、阻断签名稳定可复现 | 玩家是否觉得有趣、是否愿意继续玩 | `testing-manual.md`、`scripts/ci-tests.sh`、`viewer-software-safe-step-regression.sh` | `qa_engineer` |
| L2 Agent/fixture probe | 脚本化 step/chat/progression、场景推进、受控 bot/fixture 探针 | 可达性、卡点、节奏断点、是否存在“玩家动作后世界无响应” | 情绪价值、审美、长期动机 | `player leverage` rubric、`world_activity_only`、`snapshot.player_gameplay` | `qa_engineer` + 实现 owner |
| L3 遥测与实验 | progression funnel、停留时长、回流率、A/B、行为事件 | 某方案是否比另一方案更好；玩家在哪些环节退出 | 指标提升是否真的等于“更好玩”；样本外原因解释 | 本专题先冻结字段与决策口径，不在本轮实现采集系统 | `qa_engineer` + `producer_system_designer` |
| L4A synthetic internal playability review | 标准角色 subagent review、simulated persona panel、formal player surface 上的 scripted/UI closure、synthetic continuation prediction | 以当前内部模型看，哪些风格玩家可能想继续玩、会在哪掉线、哪些 claim 只在 synthetic 里成立 | agent 是否真的在真实操作链路中继续玩；真人是否真的愿意继续玩 | `prepare-playability-l4-review.sh`、`worktree-harness.sh`、S6 Web UI 闭环、role review cards、persona cards | `producer_system_designer` + `qa_engineer` |
| L4B embodied agent playtest | agent playtest 卡、agent 实际操作链路、session 证据、可选内部真人校准 notes | agent 是否在真实游玩入口中执行了实际操作，并表现出继续玩的倾向与可解释杠杆判断；内部真人 spot-check 是否 corroborate 该判断 | 真实外部人类在真实时间/耐心/机会成本下是否真的愿意继续玩 | `prepare-playability-l4-review.sh`、`run-playability-l4b-agent.sh`、`run-producer-playtest.sh`、`l4b-agent-playtest-card.md`、`optional-internal-human-corroboration.md` | `producer_system_designer` + `qa_engineer` |
| L5 受控外部信号 | limited preview、liveops 反馈、真实玩家 session | 在真实环境下，当前 claim envelope 是否成立 | 广泛市场成功、长期留存已被证明 | `technical preview` / limited preview 口径、liveops signal 回流 | `liveops_community` + `producer_system_designer` |

## 4. Governance Rules
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
  - persona cards 必须先回流到标准角色 review，不能直接成为 `L4B`、`L5` 或最终 stage verdict。
- Layer rules:
  - L1/L2 是“能否继续验证”的前置层，不得单独给出“已证明好玩”。
  - L3 可以证明“方案 A 比方案 B 更有效”，但仍不能跳过 `L4A/L4B/L5` 的主观体验判断。
  - `L4A` 是当前仓库内最强的 synthetic 内部判断层，可以表达“内部高强度模拟预测会继续玩”，但不等价于真实操作链路中的继续行为。
  - `L4B` 是当前仓库内最强的正式内部判断层，可以表达“agent 实际继续游玩已被观察到”，并应尽量逼近真人评审效果；内部真人试玩如果存在，只能作为 `L4B` 的校准或反证。
- Current oasis7 policy bindings:
  - active LLM access 才是正式游玩 lane；`--no-llm` 只允许记为 observer/debug。
  - `software_safe` 与 `pure_api` 都属于 formal 玩家 surface，必须回答同一组 `snapshot.player_gameplay` 问题。
  - `world_activity_only=yes` 的样本不得支撑“玩家已有 meaningful participation”。
  - 即使自动化通过、世界时间推进，只要 `L4A/L4B/L5` 仍不能证明玩家拥有稳定杠杆和继续动机，就不能把项目升级成“已证明好玩”。
  - 对应标准角色的 subagent 可以补齐所有内部 synthetic 评审环节，形成 `L4A`，但不能被记作 `L4B`、`L5` 或真实外部玩家。
  - simulated player personas 只能帮助解释“哪类玩家可能掉线 / 困惑 / 无聊”，属于 `L4A`，不能替代 `L4B` agent 试玩卡、可选内部真人校准或外部会话。
  - `prepare-playability-l4-review.sh` 只负责把完整 `L4` packet / cards / summary / commands 固定到同一 worktree；生成 scaffold 不等于 `L4A` 或 `L4B` 已完成。
  - `run-playability-l4b-agent.sh` 负责在上述 scaffold 内执行一轮最小正式 `L4B` embodied-agent run，并把关键观测点回填成稳定 evidence；它减少手工拼装，但仍不自动替代 `L4A` 角色/persona 评审或 `L5` 真实人类验证。

## 5. Combination Rules
- `block`: 任何一个 formal 玩家 surface 在 L1 就不稳定，或 L2 证明玩家动作没有稳定杠杆。
- `hold`: L1-L3 通过，但 `L4A` 仍无法证明 synthetic continue，或 `L4B` 明确给出高价值反证。
- `watch`: `L4A` 基本成立，但 `L4B` 尚未执行、样本薄，或 `L4B` 与可选内部真人校准对部分节点仍有分歧。
- `go`: 只在目标 claim envelope 下，L1-L3 通过、目标层所需的 `L4A/L4B` 都没有高价值反证，且 L5 没有出现新的高价值反证时给出。

## 6. Acceptance Criteria
- AC-1: 专题文档明确写出 `L4A/L4B/L5` 在内的证据栈与组合规则。
- AC-2: 至少列出 `software_safe`、`pure_api`、`--no-llm`、`run-producer-playtest.sh`、playability card、`player leverage` rubric、limited preview 这 7 个现有锚点。
- AC-3: 明确声明“自动化只能保证没坏/可回归，不能单独保证好玩”。
- AC-4: `doc/testing/prd.md` 与 `doc/testing/project.md` 映射该专题，并给出模块级追踪条目。
- AC-5: `doc/testing/README.md` 与 `doc/testing/prd.index.md` 把“如何判断自动化是否足以支撑好玩结论”的读者导向该专题。
- AC-6: 明确写出标准角色 subagent 的适用范围、`player` 非标准角色限制，以及“subagent review != 真实外部玩家验证”的硬边界。
- AC-7: 明确 simulated player persona panel 的定位、固定 persona 清单，以及其与 `L4A/L4B/L5` 的边界。
- AC-8: 明确 `L4A`、`L4B` 与可选内部真人校准的 operator 入口、claim 名称与当前非替代边界。
- AC-9: 明确写出 `./scripts/prepare-playability-l4-review.sh` 是当前 repo-local 的完整 `L4` scaffold 入口，只负责产物准备与命令收口，不等于自动完成评审。
- AC-10: 明确写出 `./scripts/run-playability-l4b-agent.sh` 是当前 repo-local 的正式 `L4B` embodied-agent 执行入口，会在同一 artifact 目录内落 `summary/state/screenshot/card prefill`，但不自动替代 `L4A` 评审或 `L5` 真实验证。

## 7. Non-Goals
- 不在本轮实现新的遥测 SDK、实验平台或外部问卷系统。
- 不把该专题写成某一个玩法切片的结果报告。
- 不宣称当前仓库已经满足 `go`。
- 不宣称当前 `L4A` 已可替代 `L4B`，也不宣称 `L4B` 已可替代 `L5`。
- 不为内部评审新增 `player` 这类非标准正式角色。

## 8. Technical Specifications
- Integration Points:
  - `doc/testing/prd.md`
  - `doc/testing/project.md`
  - `doc/testing/governance/playability-subagent-review-system-2026-05-06.prd.md`
  - `doc/testing/governance/playability-simulated-player-persona-panel-2026-05-06.prd.md`
  - `doc/testing/governance/playability-l4-synthetic-human-split-2026-05-06.prd.md`
  - `testing-manual.md`
  - `scripts/prepare-playability-l4-review.sh`
  - `doc/testing/manual/web-ui-agent-browser-closure-manual.manual.md`
  - `doc/playability_test_result/README.md`
  - `doc/playability_test_result/playability_test_card.md`
- Edge Cases & Error Handling:
  - 自动化全绿，但 `L4B` agent 试玩仍觉得无聊或缺乏目标：必须记为“低层 pass / 高层 hold”，不能回退成“待观察的小问题”。
  - 世界很活跃，但玩家动作没有造成稳定可解释的后果：必须标记 `world_activity_only` 或 `player_leverage=block`。
  - 某个 A/B 指标更优，但内部真人校准或 `L5` 反馈更差：优先记为“L3 与高层主观证据冲突”，要求补充解释，而不是直接按指标放行。
  - 多个 simulated personas 与 role subagents 都给出正面反应，但没有任何 agent 实玩：仍只能记为 `L4A pass / L4B missing`，不得替代 `L4B`。
  - `L4B` 已正面，但没有任何 `L5` 样本：仍只能记为 `L4B pass / L5 missing`，不得替代 `L5`。
  - 只生成了 `L4` scaffold，但 packet / role cards / persona cards / summary 仍未填写：只能记为“operator 准备完成”，不能记为 `L4A` 或 `L4B` 已完成。
- Non-Functional Requirements:
  - NFR-PES-1: 审查者必须能在 60 秒内看懂每层证据的证明边界。
  - NFR-PES-2: 所有正式玩法结论都必须能指出“当前到达了哪一层、还缺哪一层”。
  - NFR-PES-3: 模块根文档与该专题之间的互链必须可达，且通过 `doc-governance-check`。
- Security & Privacy: agent 试玩、真人试玩与外部信号的记录应继续遵循现有脱敏与最小化采集原则。

## 9. Risks & Roadmap
- Phased Rollout:
  - MVP (`PES-1`): 建立七层证据栈、组合规则与 oasis7 当前锚点映射。
  - v1.1 (`PES-2`): 把正式 evidence packet 字段进一步补齐到 L3/L5，避免只有 narrative 没有层级结论。
  - v2.0 (`PES-3`): 视需要再引入实验/遥测自动汇总，但仍保持 `L4B/L5` 不被自动化替代。
  - v2.1 (`PES-4`): 若未来建立 calibration ledger，再讨论是否提升 `L4A` 或 `L4B` 的决策权重。
- Technical Risks:
  - 风险-1: 团队可能继续把“自动化全绿”简写成“已经好玩”，导致该专题只写不执行。
  - 风险-2: 若 L3 迟迟没有统一字段，后续仍会靠零散指标做过度解释。
  - 风险-3: 若 `L4B` agent 试玩卡或其可选内部真人校准质量不稳，证据栈会在关键层失真。
  - 风险-4: 若团队偷换 `L4A`、`L4B` 和 `L5` 的用词，split 会流于纸面。

## 10. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-TESTING-PLAYABILITY-001 | `playability-evidence-stack-2026-05-06` / `PES-1` | `test_tier_required` | `rg` 检查证据层、自动化边界与组合规则 | testing/governance 玩法结论口径 |
| PRD-TESTING-PLAYABILITY-002 | `playability-evidence-stack-2026-05-06` / `PES-1/2` | `test_tier_required` | 抽查 `software_safe` / `pure_api` / `--no-llm` / `player leverage` / limited preview 是否已映射 | formal surface 与门禁边界 |
| PRD-TESTING-PLAYABILITY-003 | `playability-evidence-stack-2026-05-06` / `PES-1/2` | `test_tier_required` | 检查模块根入口、索引与专题互链 | 文档导航与追溯一致性 |
| PRD-TESTING-PLAYABILITY-004 | `playability-evidence-stack-2026-05-06` / `PES-2/3` | `test_tier_required` | 抽样检查 project/README/prd.index/current window summary 是否同步 | 模块级治理执行力 |
| PRD-TESTING-PLAYABILITY-005 | `playability-evidence-stack-2026-05-06` / `PES-3/4` | `test_tier_required` | 抽查标准角色 subagent 映射、非标准 `player` 限制与 L5 边界说明 | 多角色内部评审治理边界 |
| PRD-TESTING-PLAYABILITY-006 | `playability-evidence-stack-2026-05-06` / `PES-4` | `test_tier_required` | 抽查 simulated persona panel 定位、persona 清单与 L4-supporting 边界 | 内部玩家视角治理边界 |
| PRD-TESTING-PLAYABILITY-007 | `playability-evidence-stack-2026-05-06` / `PES-4/5` | `test_tier_required` | 抽查 `L4A/L4B/L5` 边界、operator 入口与非替代边界 | synthetic/agent/real-human 证据拆分 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| `DEC-PES-001` | 明确写成“没有单一自动化方案能保证好玩” | 把自动化继续包装成足以证明玩法质量 | 这会持续混淆可靠性结论和玩法结论。 |
| `DEC-PES-002` | 用多层证据栈表达从内部到外部、从客观到主观的递进关系 | 把所有信号平铺成同权 checklist | 平铺 checklist 容易让低层证据越权替代高层证据。 |
| `DEC-PES-003` | 保留 `L4B` agent 实操作为最高正式内部判断层，并把内部真人试玩降为其可选校准；真实人类验证留在 `L5` | 试图用 L3 实验、L2 bot probe，或单次内部真人试玩重新发明独立层级 | 当前工具链可以辅助判断，但不能代替“真实玩过之后是否还想继续玩”。 |
| `DEC-PES-004` | 所有内部评审默认可委托给对应标准角色 subagent | 为每个“玩家视角”额外创造非标准正式角色 | 当前仓库已有标准角色体系，新增非标准角色会破坏 execution log / handoff / PM 约束。 |
| `DEC-PES-005` | simulated personas 只作为 L4-supporting 的内部假设面板 | 把 simulated persona panel 升格为独立证据层或正式角色 | 这样会模糊 persona 假设与 agent/human 实玩之间的证明强度差异。 |
| `DEC-PES-006` | 把 `L4` 收口为 `L4A synthetic` 与 `L4B agent`，并把内部真人试玩降为 `L4B` 校准、把真实人类验证留在 `L5` | 继续把 synthetic、agentic、内部真人试玩与真实人类验证共用模糊标签 | 不拆边界会持续混淆“agent 预测继续玩”“agent 实际继续玩”和“真实人类实际继续玩”。 |
| `DEC-PES-007` | 当前先提供 repo-local `L4` scaffold，把 packet/card/summary 入口固定下来 | 继续依赖每次临时手写文件名和执行顺序 | 没有稳定入口就无法说当前 PR 已能在单个 worktree 内完整执行 `L4`。 |
