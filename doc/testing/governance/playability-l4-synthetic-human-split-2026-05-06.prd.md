# oasis7: 好玩性 L4 synthetic/agent/human 分层（2026-05-06）

- 对应设计文档: `doc/testing/governance/playability-l4-synthetic-human-split-2026-05-06.design.md`
- 对应项目管理文档: `doc/testing/governance/playability-l4-synthetic-human-split-2026-05-06.project.md`

审计轮次: 2

## 目标
- 把当前 evidence stack 中过于宽泛的 `L4` 收口成 `L4A synthetic internal playability review` 与 `L4B embodied agent playtest` 两层正式内部证据，并把内部真人试玩降为 `L4B` 可选校准。
- 明确 agent 完整角色扮演、标准角色 subagent、simulated personas、agent 实际进游戏操作、内部真人校准、以及 `L5` 真实人类 / 外部验证分别能证明什么，避免继续混写。

## 范围
- 覆盖 `L4A/L4B` 的定义、输入、输出、组合规则、升级边界和 operator 入口，以及内部真人校准与 `L5` 的关系。
- 覆盖当前 repo-local `L4` scaffold 入口如何把 `L4A` packet / cards、`L4B` agent 卡、可选内部真人佐证 notes、summary 与推荐命令收口到同一 worktree。
- 不覆盖新的真人招募平台、外部用户研究系统或“完全自动替代真人”的自治 orchestration 实现。

## 接口 / 数据
- 专题 PRD: `doc/testing/governance/playability-l4-synthetic-human-split-2026-05-06.prd.md`
- 专题设计文档: `doc/testing/governance/playability-l4-synthetic-human-split-2026-05-06.design.md`
- 专题项目管理文档: `doc/testing/governance/playability-l4-synthetic-human-split-2026-05-06.project.md`
- 上游专题:
  - `doc/testing/governance/playability-evidence-stack-2026-05-06.prd.md`
  - `doc/testing/governance/playability-subagent-review-system-2026-05-06.prd.md`
  - `doc/testing/governance/playability-simulated-player-persona-panel-2026-05-06.prd.md`
- operator 入口:
  - `testing-manual.md`
  - `scripts/prepare-playability-l4-review.sh`
  - `scripts/run-producer-playtest.sh`
  - `doc/playability_test_result/playability_test_card.md`

## 里程碑
- M1 (2026-05-06): 冻结 `L4A/L4B/L5` 边界、内部真人校准定位、组合规则，以及 repo-local scaffold 入口。
- M2: 若后续要提升 `L4B` 或 `L4A` 的替代权重，再补 calibration 方案。

## 风险
- 若继续把 `L4` 写成单层，synthetic、agentic、real-human 结果会持续混写。
- 若把 `L4B` 具身 agent 试玩或内部真人校准直接包装成 `L5` 真人替代品，会污染 go/hold/watch/block 结论。

## 1. Executive Summary
- Problem Statement: 当前 evidence stack 已经明确“自动化不能单独证明好玩”，但旧 `L4` 仍把三种不同强度的证据混在一起：`L4A` 的高强度内部模拟、agent 实际进游戏操作后的 `L4B`，以及内部真人 spot-check。若继续共用一个 `L4` 标签，团队会反复争论“agent 真的玩了一轮，到底算不算比 synthetic 更强”“内部真人试玩到底是不是新的正式层”“真人没玩之前能不能收口”。
- Proposed Solution: 把 `L4` 正式收口成两层：`L4A synthetic internal playability review` 与 `L4B embodied agent playtest`。`L4A` 收纳标准角色 subagent、simulated personas、formal surface 上的 synthetic continuation prediction；`L4B` 收纳 agent 真实进入产品并执行操作链路后的继续游玩判断，并允许内部真人试玩只作为 `L4B` 的可选校准；真实人类 / 真实环境验证统一归 `L5`。
- Success Criteria:
  - SC-1: 文档明确给出 `L4A/L4B/L5` 的输入、输出、证明边界和 operator 入口。
  - SC-2: 文档明确写出 `agent 完整角色扮演 != agent 实际试玩 != 真实人类继续游玩意愿已被证明`。
  - SC-3: `playability evidence stack`、`persona panel`、`subagent review system` 与 `testing-manual` 都同步改成同一口径。
  - SC-4: `L4B` 可以成为“agent 实际试玩已经发生”的正式 verdict，并应尽量逼近真人评审效果，但不能冒充 `L5`。
  - SC-5: 文档明确保留未来 calibration 扩展位，但当前不宣称 `L4A` 或 `L4B` 已可替代更高层。
  - SC-6: 当前仓库提供 `./scripts/prepare-playability-l4-review.sh`，可以在单个 worktree 内稳定生成完整 `L4` scaffold，并把 `L4A`、`L4B` 与可选内部真人校准证据路径对齐到同一 artifact 目录。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`: 需要知道当前结论是“模拟上看起来想继续玩”“agent 真的继续操作了”“真实人类真的想继续玩”中的哪一层。
  - `qa_engineer`: 需要阻止团队把 `L4A` 写成 `L4B`，或把 `L4B` / 内部真人校准写成 `L5`。
  - `viewer_engineer` / `agent_engineer` / `runtime_engineer`: 需要知道自己补的是 `L4A`、`L4B` 还是 `L5` 前的内部校准证据。
  - `liveops_community`: 需要知道哪些内部正面结论还不能拿去对外表达成“已被真实玩家验证”。
- User Stories:
  - PRD-TESTING-L4SPLIT-001: As a `producer_system_designer`, I want `L4A/L4B/L5` boundaries written explicitly, so that synthetic、agentic、real-human claims stop collapsing into one label.
  - PRD-TESTING-L4SPLIT-002: As a `qa_engineer`, I want synthetic roleplay evidence bounded below embodied agent playtest, and embodied agent playtest bounded below `L5` real-human validation, so that overclaiming can be blocked.
  - PRD-TESTING-L4SPLIT-003: As an operator, I want clear entrypoints for `L4A` / `L4B`, plus optional internal human calibration notes, so that I know which command and artifact set belongs to which layer.
  - PRD-TESTING-L4SPLIT-004: As a future workflow owner, I want a documented path for calibration-based promotion, so that stronger synthetic or embodied-agent evidence can be discussed without prematurely changing current claims.
- Critical User Flows:
  1. `识别当前要回答的问题是 synthetic 预测、agent 实际试玩，还是 human 实际继续游玩意愿`
  2. `若要在一个 worktree 内准备完整 L4 -> 先执行 prepare-playability-l4-review.sh 生成 packet / cards / summary / commands scaffold`
  3. `若先跑内部模拟 -> 执行 L4A -> 输出 synthetic verdict`
  4. `若需要 agent 真的进游戏操作一轮 -> 升级到 L4B -> 输出 embodied-agent verdict`
  5. `若需要内部真人校准 -> 复用同一入口记录 corroboration / contradiction`
  6. `若需要真实人类 / 真实环境证据 -> 升级到 L5`
  7. `producer_system_designer` 汇总 L4A/L4B 与可选校准 -> 决定当前是 `synthetic_ready` / `agent_ready` / `external_ready`

## 3. Functional Specification Matrix
| 层级 | 主要输入 | 可以证明 | 不能证明 | 默认入口 | 默认 owner |
| --- | --- | --- | --- | --- | --- |
| `L4A synthetic internal playability review` | 标准角色 subagent review cards、simulated persona cards、formal player surface 上的 scripted/UI closure、synthetic continuation prediction | 以当前内部模型看，哪些风格玩家大概率想继续玩，哪些断点会掉线，哪些 claim 只在 synthetic 里成立 | agent 是否真的在真实操作链路中继续玩；真人是否真的愿意继续玩 | `prepare-playability-l4-review.sh`、`testing-manual.md`、`worktree-harness.sh`、S6 Web UI 闭环、subagent/persona panel 输出 | `producer_system_designer` + `qa_engineer` |
| `L4B embodied agent playtest` | agent 实际打开产品、执行真实操作链路、agent playtest card、play session 证据、可选内部真人校准 notes | agent 是否在真实游玩入口中执行了实际操作，并表现出继续玩的倾向与可解释杠杆判断；内部真人 spot-check 是否 corroborate 该判断 | 真实外部人类是否真的愿意继续玩 | `prepare-playability-l4-review.sh`、`run-producer-playtest.sh`、`l4b-agent-playtest-card.md`、`optional-internal-human-corroboration.md` / `doc/playability_test_result/playability_test_card.md` | `producer_system_designer` + `qa_engineer` |

## 4. Claim Rules
- 允许的 `L4A` 结论:
  - `synthetic_continue_likely`
  - `synthetic_continue_unclear`
  - `synthetic_continue_unlikely`
- 允许的 `L4B` 结论:
  - `agent_continue_observed`
  - `agent_continue_mixed`
  - `agent_continue_not_observed`
- 允许的内部真人校准结论:
  - `corroborates_l4b`
  - `mixed`
  - `contradicts_l4b`
- 禁止写法:
  - 只有 `L4A` 结果，却写成“agent 已经试玩通过”或“真人试玩已通过”
  - 只有 `L4B` 或内部真人校准结果，却写成“真实玩家验证已通过”
  - 只有 persona/subagent 结果，却写成“真实玩家验证已通过”

## 5. Escalation And Calibration Boundary
- 必须从 `L4A` 升级到 `L4B` 的情况:
  - 玩法 claim 需要使用“agent 真实进游戏操作后仍想继续”这类措辞
  - 仅靠 synthetic 无法说明第一条真实操作链路是否成立
- 必须从 `L4B` 升级到 `L5` 的情况:
  - 玩法 claim 需要使用“真实人类想继续玩”这类措辞
  - release / stage 结论需要比 `agent_ready` 更强
- 当前不允许的推断:
  - `L4A pass => L4B pass`
  - `L4B pass => L5 pass`
- 当前 repo-local scaffold 的边界:
  - `prepare-playability-l4-review.sh` 只负责生成 packet / cards / summary / commands / manifest。
  - 生成了 scaffold 不等于 `L4A` 或 `L4B` 已完成。
- 保留的未来扩展:
  - 若未来建立 calibration ledger，且长期证明 `L4A` 对 `L4B`、或 `L4B` 对 `L5` 具备可接受预测力，才可讨论提升低层的决策权重；本轮不做此 claim。

## 6. Acceptance Criteria
- AC-1: 新专题明确写出 `L4A/L4B/L5` 的定义、输入、输出和 claim 边界。
- AC-2: `playability evidence stack` 专题正式拆出 `L4A/L4B/L5` 边界。
- AC-3: `testing-manual.md` 的 L4/L5 operator 入口同步区分 `L4A`、`L4B` 与 `L5`。
- AC-4: `simulated player persona panel` 与 `subagent review system` 都改成显式服务 `L4A`，但允许它们把结果升级到 `L4B`。
- AC-5: 文档明确保留 calibration 扩展位，但不宣称当前已完成。
- AC-6: 文档明确写出 `./scripts/prepare-playability-l4-review.sh` 是当前完整 `L4` scaffold 入口，以及“scaffold != 已完成更高层”的硬边界。

## 7. Non-Goals
- 不在本轮让 `L4A` 直接替代 `L4B`。
- 不在本轮让 `L4B` 直接替代 `L5`。
- 不实现 calibration ledger、统计模型或外部研究平台。
- 不改变 `L5` 的定义。

## 8. Technical Specifications
- Integration Points:
  - `doc/testing/governance/playability-evidence-stack-2026-05-06.prd.md`
  - `doc/testing/governance/playability-subagent-review-system-2026-05-06.prd.md`
  - `doc/testing/governance/playability-simulated-player-persona-panel-2026-05-06.prd.md`
  - `doc/testing/prd.md`
  - `doc/testing/project.md`
  - `doc/testing/README.md`
  - `doc/testing/prd.index.md`
  - `testing-manual.md`
  - `scripts/prepare-playability-l4-review.sh`
  - `doc/testing/templates/playability-l4-review-packet-template.md`
  - `doc/testing/templates/playability-l4-summary-template.md`
- Edge Cases:
  - `L4A` 全正面、`L4B` 未执行：只能写 `synthetic_ready`，不能写 `agent_ready`。
  - `L4B` 全正面、`L5` 未执行：只能写 `agent_ready`，不能写 `external_ready`。
  - `L4A` 与 `L4B` 冲突：必须显式记录 divergence。
  - 内部真人校准与 `L4B` 冲突：必须显式记录 divergence，并保持 `L5 missing`。
  - 已生成 `L4` scaffold，但 `commands.sh` 未执行且卡片仍为空：只能写 operator scaffold ready，不能写任何 layer verdict。
- Non-Functional Requirements:
  - NFR-L4SPLIT-1: 审查者必须能在 60 秒内看懂当前结论是 `L4A`、`L4B` 还是 `L5` 前的内部校准。
  - NFR-L4SPLIT-2: operator 必须能在 5 分钟内知道该跑 `worktree-harness/S6`、`run-producer-playtest + agent card`，还是补一份可选内部真人校准 notes。
  - NFR-L4SPLIT-3: 任何正式玩法结论都不能再把 `synthetic`、`agentic` 与 `real-human` 混写成一个未分层的 `L4` verdict。

## 9. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-TESTING-L4SPLIT-001 | `playability-l4-synthetic-human-split-2026-05-06` / `L4S-1` | `test_tier_required` | 抽查 `L4A/L4B/L5` 边界、输入输出与 claim 规则 | 玩法证据分层口径 |
| PRD-TESTING-L4SPLIT-002 | `playability-l4-synthetic-human-split-2026-05-06` / `L4S-1/2` | `test_tier_required` | 抽查 evidence stack / subagent / persona / manual 互相引用 | 文档真值一致性 |
| PRD-TESTING-L4SPLIT-003 | `playability-l4-synthetic-human-split-2026-05-06` / `L4S-2` | `test_tier_required` | 抽查 operator 入口与边界说明 | operator 执行清晰度 |
| PRD-TESTING-L4SPLIT-004 | `playability-l4-synthetic-human-split-2026-05-06` / `L4S-2/3` | `test_tier_required` | 抽查 calibration 扩展位与当前非承诺边界 | synthetic/agent/human 替代 claim 边界 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| `DEC-L4S-001` | 把当前 `L4` 收口成 `L4A/L4B`，并把真实人类验证留在 `L5` | 继续维持单层 `L4` | 单层会持续混淆 synthetic、agentic 与 real-human 证据。 |
| `DEC-L4S-002` | `L4B` 定义为具身 agent 试玩，不再等同真人试玩 | 继续把 agent 实操也记成 `L4A` | 用户当前产品口径要求“agent 实际玩游戏”成为独立更强层。 |
| `DEC-L4S-003` | 内部真人试玩只保留为 `L4B` 可选校准，真实人类验证统一归 `L5` | 把内部真人试玩继续升格成独立正式层 | 用户当前口径要求 `L4B` 成为最高正式内部层，同时保留 `L5` 真实验证边界。 |
| `DEC-L4S-004` | 为未来 calibration 留扩展位，但当前不宣称已完成 | 现在就把 `L4A` 或 `L4B` 提升为更高层替代品 | 当前仓库没有跨层校准证据。 |
| `DEC-L4S-005` | 当前先提供 repo-local scaffold，把 `L4A`、`L4B` 与可选内部真人校准产物放进同一 artifact 目录 | 继续让 operator 临时拼接 packet / cards / summary / playtest card | 没有统一入口就无法把“完整 L4 可执行”作为当前 PR 的正式能力。 |
