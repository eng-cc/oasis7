# oasis7: 好玩性证据栈（2026-05-06）

- 对应设计文档: `doc/testing/governance/playability-evidence-stack-2026-05-06.design.md`
- 对应项目管理文档: `doc/testing/governance/playability-evidence-stack-2026-05-06.project.md`

审计轮次: 1

## 1. Executive Summary
- Problem Statement: 自动化测试已经能稳定覆盖回归、协议、性能、长稳和部分玩家路径，但“自动化绿灯”仍不等于“游戏已经好玩”。如果没有一套明确的证据栈把自动化、遥测、A/B 与真人试玩分层，团队很容易把“没坏”“世界在动”“玩家真的想继续玩”混写成同一种结论。
- Proposed Solution: 建立 `playability evidence stack` 专题，正式定义 oasis7 的五层好玩性证据体系，明确每一层能证明什么、不能证明什么、如何组合成阶段性 go/hold/block 结论，以及当前仓库已有脚本/文档应挂在哪一层。
- Success Criteria:
  - SC-1: 专题文档明确声明“没有单一自动化方案能够保证游戏好玩”，且给出 oasis7 的正式替代口径。
  - SC-2: 至少定义 `automation baseline / agent probe / telemetry & experiments / structured human playtests / limited preview live signals` 五层证据，并为每层列出可证明与不可证明边界。
  - SC-3: `software_safe`、`pure_api`、`--no-llm observer/debug only`、`run-producer-playtest.sh`、playability card、`player leverage` rubric 和 limited preview 现有治理口径都被映射进同一套证据栈。
  - SC-4: 模块根入口 `doc/testing/prd.md` / `project.md` / `README.md` / `prd.index.md` 能把读者导向该专题。

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
- User Stories:
  - PRD-TESTING-PLAYABILITY-001: As a `producer_system_designer`, I want a canonical evidence stack for gameplay fun, so that I can make stage decisions without conflating reliability with fun.
  - PRD-TESTING-PLAYABILITY-002: As a `qa_engineer`, I want each evidence layer to have explicit proof boundaries, so that I can block overclaims early.
  - PRD-TESTING-PLAYABILITY-003: As an implementation owner, I want existing scripts and reports mapped into that stack, so that I know what evidence gap is still open.
  - PRD-TESTING-PLAYABILITY-004: As a release reviewer, I want a clear combination rule for go/hold/block, so that no single metric or single playtest overrides the rest of the stack.
- Critical User Flows:
  1. `识别体验目标 -> 选择对应玩家 surface -> 先跑自动化基线 -> 判断是否已具备继续收集更高层证据的前置条件`
  2. `收集 agent probe / telemetry / 真人试玩 / limited preview 信号 -> 填写统一 evidence packet -> 标记每层结论`
  3. `producer_system_designer` 汇总多层结论 -> 输出 `go/watch/hold/block`，并明确“当前只证明了什么”
- Functional Specification Matrix:
| 证据层 | 主要输入 | 可以证明 | 不能证明 | oasis7 当前锚点 | 默认 owner |
| --- | --- | --- | --- | --- | --- |
| L1 自动化基线 | `required/full`、协议回归、Web 闭环脚本、长稳 smoke | 没坏、可重复、主链路能走通、阻断签名稳定可复现 | 玩家是否觉得有趣、是否愿意继续玩 | `testing-manual.md`、`scripts/ci-tests.sh`、`viewer-software-safe-step-regression.sh` | `qa_engineer` |
| L2 Agent/fixture probe | 脚本化 step/chat/progression、场景推进、受控 bot/fixture 探针 | 可达性、卡点、节奏断点、是否存在“玩家动作后世界无响应” | 情绪价值、审美、长期动机 | `player leverage` rubric、`world_activity_only`、`snapshot.player_gameplay` | `qa_engineer` + 实现 owner |
| L3 遥测与实验 | progression funnel、停留时长、回流率、A/B、行为事件 | 某方案是否比另一方案更好；玩家在哪些环节退出 | 指标提升是否真的等于“更好玩”；样本外原因解释 | 本专题先冻结字段与决策口径，不在本轮实现采集系统 | `qa_engineer` + `producer_system_designer` |
| L4 结构化真人试玩 | playability 卡片、制作人试玩、QA headed rerun、受控访谈 | 是否看得懂、是否感到有杠杆、是否想继续玩、阻塞点是否可解释 | 大规模外部市场反应 | `run-producer-playtest.sh`、`doc/playability_test_result/card_*.md` | `producer_system_designer` + `qa_engineer` |
| L5 受控外部信号 | limited preview、liveops 反馈、真实玩家 session | 在真实环境下，当前 claim envelope 是否成立 | 广泛市场成功、长期留存已被证明 | `technical preview` / limited preview 口径、liveops signal 回流 | `liveops_community` + `producer_system_designer` |
- Layer rules:
  - L1/L2 是“能否继续验证”的前置层，不得单独给出“已证明好玩”。
  - L3 可以证明“方案 A 比方案 B 更有效”，但仍不能跳过 L4 的玩家主观验证。
  - L4 是当前仓库内最接近“是否好玩”的正式内部判断层，但仍属于内部证据，不自动等价于外部市场验证。
  - L5 只允许在受控 claim envelope 内升级信心，不允许把少量反馈写成“已完成普适验证”。
- Combination rules:
  - `block`: 任何一个 formal 玩家 surface 在 L1 就不稳定，或 L2 证明玩家动作没有稳定杠杆。
  - `hold`: L1/L2 通过，但 L4 仍无法证明“玩家想继续玩”，或 L5 尚未收集到足够受控信号。
  - `watch`: L1-L4 基本成立，但 L5 样本量小、或 L3 仍显示某些分段掉队风险。
  - `go`: 只在目标 claim envelope 下，L1-L4 全部通过，且 L5 没有出现新的高价值反证时给出。
- Current oasis7 policy bindings:
  - active LLM access 才是正式游玩 lane；`--no-llm` 只允许记为 observer/debug。
  - `software_safe` 与 `pure_api` 都属于 formal 玩家 surface，必须回答同一组 `snapshot.player_gameplay` 问题。
  - `world_activity_only=yes` 的样本不得支撑“玩家已有 meaningful participation”。
  - 即使自动化通过、世界时间推进，只要 L4 仍不能证明玩家拥有稳定杠杆和继续动机，就不能把项目升级成“已证明好玩”。
- Acceptance Criteria:
  - AC-1: 专题文档明确写出五层证据栈与组合规则。
  - AC-2: 至少列出 `software_safe`、`pure_api`、`--no-llm`、`run-producer-playtest.sh`、playability card、`player leverage` rubric、limited preview 这 7 个现有锚点。
  - AC-3: 明确声明“自动化只能保证没坏/可回归，不能单独保证好玩”。
  - AC-4: `doc/testing/prd.md` 与 `doc/testing/project.md` 映射该专题，并给出模块级追踪条目。
  - AC-5: `doc/testing/README.md` 与 `doc/testing/prd.index.md` 把“如何判断自动化是否足以支撑好玩结论”的读者导向该专题。
- Non-Goals:
  - 不在本轮实现新的遥测 SDK、实验平台或外部问卷系统。
  - 不把该专题写成某一个玩法切片的结果报告。
  - 不宣称当前仓库已经满足 `go`。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 本专题是 testing/governance 层的判断框架，不直接新增测试代码；它把现有自动化、playability 文档、信任门/留存门、制作人试玩和 limited preview 信号统一到一个 evidence stack。
- Integration Points:
  - `doc/testing/prd.md`
  - `doc/testing/project.md`
  - `testing-manual.md`
  - `doc/testing/manual/web-ui-agent-browser-closure-manual.manual.md`
  - `doc/testing/evidence/gameplay-ten-minute-trust-gate-2026-04-09.md`
  - `doc/playability_test_result/README.md`
  - `doc/playability_test_result/playability_test_card.md`
  - `doc/game/prd.md`
  - `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.prd.md`
- Edge Cases & Error Handling:
  - 自动化全绿，但真人试玩仍觉得无聊或缺乏目标：必须记为 L1 pass / L4 hold，不能回退成“待观察的小问题”。
  - 世界很活跃，但玩家动作没有造成稳定可解释的后果：必须标记 `world_activity_only` 或 `player_leverage=block`。
  - 某个 A/B 指标更优，但真人试玩反馈更差：优先记为“L3 与 L4 冲突”，要求补充解释，而不是直接按指标放行。
  - 少量外部正反馈与内部留存门冲突：仍以 formal lane 的门禁与 blocker 为准，外部反馈只作为 L5 旁证。
- Non-Functional Requirements:
  - NFR-PES-1: 审查者必须能在 60 秒内看懂每层证据的证明边界。
  - NFR-PES-2: 所有正式玩法结论都必须能指出“当前到达了哪一层、还缺哪一层”。
  - NFR-PES-3: 模块根文档与该专题之间的互链必须可达，且通过 `doc-governance-check`。
- Security & Privacy: 真人试玩与外部信号的记录应继续遵循现有脱敏与最小化采集原则。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (`PES-1`): 建立五层证据栈、组合规则与 oasis7 当前锚点映射。
  - v1.1 (`PES-2`): 把正式 evidence packet 字段进一步补齐到 L3/L5，避免只有 narrative 没有层级结论。
  - v2.0 (`PES-3`): 视需要再引入实验/遥测自动汇总，但仍保持 L4/L5 不被自动化替代。
- Technical Risks:
  - 风险-1: 团队可能继续把“自动化全绿”简写成“已经好玩”，导致该专题只写不执行。
  - 风险-2: 若 L3 迟迟没有统一字段，后续仍会靠零散指标做过度解释。
  - 风险-3: 若 L4 真人试玩卡片质量不稳，证据栈会在最关键一层失真。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-TESTING-PLAYABILITY-001 | `playability-evidence-stack-2026-05-06` / `PES-1` | `test_tier_required` | `rg` 检查五层证据、自动化边界与组合规则 | testing/governance 玩法结论口径 |
| PRD-TESTING-PLAYABILITY-002 | `playability-evidence-stack-2026-05-06` / `PES-1/2` | `test_tier_required` | 抽查 `software_safe` / `pure_api` / `--no-llm` / `player leverage` / limited preview 是否已映射 | formal surface 与门禁边界 |
| PRD-TESTING-PLAYABILITY-003 | `playability-evidence-stack-2026-05-06` / `PES-1/2` | `test_tier_required` | 检查模块根入口、索引与专题互链 | 文档导航与追溯一致性 |
| PRD-TESTING-PLAYABILITY-004 | `playability-evidence-stack-2026-05-06` / `PES-2/3` | `test_tier_required` | 抽样检查 project/README/prd.index/current window summary 是否同步 | 模块级治理执行力 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| `DEC-PES-001` | 明确写成“没有单一自动化方案能保证好玩” | 把自动化继续包装成足以证明玩法质量 | 这会持续混淆可靠性结论和玩法结论。 |
| `DEC-PES-002` | 用五层证据栈表达从内部到外部、从客观到主观的递进关系 | 把所有信号平铺成同权 checklist | 平铺 checklist 容易让低层证据越权替代高层证据。 |
| `DEC-PES-003` | 保留 L4 真人试玩作为当前仓内最高权重内部判断层 | 试图用 L3 实验或 L2 bot probe 替代人类体验判断 | 当前工具链可以辅助判断，但不能代替“玩家是否觉得值得继续玩”。 |
