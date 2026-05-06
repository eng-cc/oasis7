# oasis7: 好玩性 L4 synthetic/human 分层（2026-05-06）

- 对应设计文档: `doc/testing/governance/playability-l4-synthetic-human-split-2026-05-06.design.md`
- 对应项目管理文档: `doc/testing/governance/playability-l4-synthetic-human-split-2026-05-06.project.md`

审计轮次: 1

## 目标
- 把当前 evidence stack 中过于宽泛的 `L4` 拆成 `L4A synthetic internal playability review` 与 `L4B structured human playtest`。
- 明确 agent 完整角色扮演、标准角色 subagent 和 simulated personas 能提升到什么强度，以及为什么仍不能直接与真人试玩 claim 混写。

## 范围
- 覆盖 `L4A/L4B` 的定义、输入、输出、组合规则、升级边界和 operator 入口。
- 不覆盖新的真人招募平台、外部用户研究系统或自动 orchestration 工具实现。

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
  - `scripts/run-producer-playtest.sh`
  - `doc/playability_test_result/playability_test_card.md`

## 里程碑
- M1 (2026-05-06): 冻结 `L4A/L4B` 定义、claim boundary 和组合规则。
- M2: 若后续要提升 `L4A` 权重，再补 synthetic-to-human calibration 方案。

## 风险
- 若继续把 `L4` 写成单层，agent 结果和真人结果会持续混写。
- 若把 `L4A` 直接包装成真人试玩替代品，会污染 go/hold/watch/block 结论。

## 1. Executive Summary
- Problem Statement: 当前 evidence stack 已经明确“自动化不能单独证明好玩”，但 `L4` 仍把两种本质不同的证据混在一起：一类是由 subagent、simulated personas 和 scripted/UI closure 组成的高强度内部模拟；另一类是真实人类在有时间、耐心和机会成本约束下的主观继续游玩意愿。如果继续共用一个 `L4` 标签，团队会反复争论“agent 说想继续玩，到底算不算真人试玩”。
- Proposed Solution: 把 `L4` 正式拆成两层：`L4A synthetic internal playability review` 与 `L4B structured human playtest`。前者收纳标准角色 subagent、simulated personas、真实 formal surface 上的 synthetic continuation prediction；后者只收纳真人试玩卡、制作人/QA 人类实际继续游玩与主观反馈。
- Success Criteria:
  - SC-1: 文档明确给出 `L4A` 与 `L4B` 的输入、输出、证明边界和 operator 入口。
  - SC-2: 文档明确写出 `agent 完整角色扮演 != 真人继续游玩意愿已被证明`。
  - SC-3: `playability evidence stack`、`persona panel`、`subagent review system` 与 `testing-manual` 都同步改成同一口径。
  - SC-4: `L4A` 可以成为内部高强度模拟完成的正式 verdict，但不能冒充 `L4B`。
  - SC-5: 文档明确保留未来“做 synthetic-to-human calibration 后再提升 L4A 权重”的扩展位，但当前不宣称已经完成。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`: 需要知道当前结论是“模拟上看起来想继续玩”还是“真人真的想继续玩”。
  - `qa_engineer`: 需要阻止团队把 synthetic 结果写成 human-validated claim。
  - `viewer_engineer` / `agent_engineer` / `runtime_engineer`: 需要知道自己补的是 `L4A` 还是 `L4B` 的证据。
  - `liveops_community`: 需要知道哪些内部正面结论还不能拿去对外表达成“已被玩家验证”。
- User Scenarios & Frequency:
  - 玩法争议集中在“agent 都说会继续玩，为什么还要人测”时。
  - 日常迭代希望尽量减少人参与，但仍要保持证据边界时。
  - 发布前需要解释当前到底只到 `L4A`，还是已经到 `L4B` 时。
- User Stories:
  - PRD-TESTING-L4SPLIT-001: As a `producer_system_designer`, I want `L4A/L4B` split explicitly, so that synthetic and human claims stop collapsing into one label.
  - PRD-TESTING-L4SPLIT-002: As a `qa_engineer`, I want synthetic roleplay evidence bounded below human playtest evidence, so that overclaiming can be blocked.
  - PRD-TESTING-L4SPLIT-003: As an operator, I want clear entrypoints for `L4A` and `L4B`, so that I know which command and artifact set belongs to which layer.
  - PRD-TESTING-L4SPLIT-004: As a future workflow owner, I want a documented path for calibration-based promotion, so that stronger synthetic evidence can be discussed without prematurely changing current claims.
- Critical User Flows:
  1. `识别当前要回答的问题是 synthetic 预测，还是 human 实际继续游玩意愿`
  2. `若先跑内部模拟 -> 执行 L4A -> 输出 synthetic verdict`
  3. `若需要真人主观证据 -> 升级到 L4B -> 输出 human verdict`
  4. `producer_system_designer` 汇总 L4A/L4B -> 决定当前是 synthetic_ready / human_ready / external_ready`

## 3. Functional Specification Matrix
| 层级 | 主要输入 | 可以证明 | 不能证明 | 默认入口 | 默认 owner |
| --- | --- | --- | --- | --- | --- |
| `L4A synthetic internal playability review` | 标准角色 subagent review cards、simulated persona cards、formal player surface 上的 scripted/UI closure、synthetic continuation prediction | 以当前内部模型看，哪些风格玩家大概率想继续玩，哪些断点会掉线，哪些 claim 只在 synthetic 里成立 | 真人在真实时间/耐心成本下是否真的愿意继续玩 | `testing-manual.md`、`worktree-harness.sh`、S6 Web UI 闭环、subagent/persona panel 输出 | `producer_system_designer` + `qa_engineer` |
| `L4B structured human playtest` | 制作人试玩、QA headed rerun、playability card、受控访谈 | 真实人类是否看懂、是否感到有杠杆、是否想继续玩、阻塞是否可解释 | 广泛外部市场反应 | `run-producer-playtest.sh`、`doc/playability_test_result/playability_test_card.md` | `producer_system_designer` + `qa_engineer` |

## 4. Claim Rules
- 允许的 `L4A` 结论:
  - `synthetic_continue_likely`
  - `synthetic_continue_unclear`
  - `synthetic_continue_unlikely`
- 允许的 `L4B` 结论:
  - `human_continue_observed`
  - `human_continue_mixed`
  - `human_continue_not_observed`
- 禁止写法:
  - 只有 `L4A` 结果，却写成“玩家已经证明想继续玩”
  - 只有 persona/subagent 结果，却写成“真人试玩已通过”

## 5. Escalation And Calibration Boundary
- 必须从 `L4A` 升级到 `L4B` 的情况:
  - 玩法 claim 需要使用“真人想继续玩”这类措辞
  - `L4A` 内部分歧很大，或 synthetic 结果高度依赖 persona 假设
  - release / stage 结论需要比 `synthetic_ready` 更强
- 当前不允许的推断:
  - `L4A pass => L4B pass`
- 保留的未来扩展:
  - 若未来建立 synthetic-to-human calibration ledger，且长期证明 `L4A` 对 `L4B` 具备可接受预测力，才可讨论提升 `L4A` 的决策权重；本轮不做此 claim。

## 6. Acceptance Criteria
- AC-1: 新专题明确写出 `L4A` 与 `L4B` 的定义、输入、输出和 claim 边界。
- AC-2: `playability evidence stack` 专题正式拆出 `L4A/L4B`。
- AC-3: `testing-manual.md` 的 L4 operator 入口同步区分 `L4A` 与 `L4B`。
- AC-4: `simulated player persona panel` 与 `subagent review system` 都改成显式服务 `L4A`。
- AC-5: 文档明确保留 calibration 扩展位，但不宣称当前已完成。

## 7. Non-Goals
- 不在本轮让 `L4A` 直接替代 `L4B`。
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
- Edge Cases:
  - `L4A` 全正面、`L4B` 未执行：只能写 `synthetic_ready`，不能写 `human_ready`。
  - `L4A` 与 `L4B` 冲突：优先记为“synthetic-human divergence”，不得直接用 `L4A` 覆盖真人反馈。
  - `L4B` 样本很少：可以提升到 `human_continue_mixed`，但不能直接把它扩展成 `L5` 结论。
- Non-Functional Requirements:
  - NFR-L4SPLIT-1: 审查者必须能在 60 秒内看懂当前结论是 `L4A` 还是 `L4B`。
  - NFR-L4SPLIT-2: operator 必须能在 5 分钟内知道该跑 `worktree-harness/S6` 还是 `run-producer-playtest/playability card`。
  - NFR-L4SPLIT-3: 任何正式玩法结论都不能再把 `synthetic` 与 `human` 混写成一个未分层的 `L4` verdict。

## 9. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-TESTING-L4SPLIT-001 | `playability-l4-synthetic-human-split-2026-05-06` / `L4S-1` | `test_tier_required` | 抽查 `L4A/L4B` 定义、输入输出与 claim 规则 | 玩法证据分层口径 |
| PRD-TESTING-L4SPLIT-002 | `playability-l4-synthetic-human-split-2026-05-06` / `L4S-1/2` | `test_tier_required` | 抽查 evidence stack / subagent / persona / manual 互相引用 | 文档真值一致性 |
| PRD-TESTING-L4SPLIT-003 | `playability-l4-synthetic-human-split-2026-05-06` / `L4S-2` | `test_tier_required` | 抽查 operator 入口与边界说明 | operator 执行清晰度 |
| PRD-TESTING-L4SPLIT-004 | `playability-l4-synthetic-human-split-2026-05-06` / `L4S-2/3` | `test_tier_required` | 抽查 calibration 扩展位与当前非承诺边界 | synthetic 替代 claim 边界 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| `DEC-L4S-001` | 把当前 `L4` 拆成 `L4A/L4B` | 继续维持单层 `L4` | 单层会持续混淆 synthetic 与 human 证据。 |
| `DEC-L4S-002` | `L4A` 允许高权重 synthetic 结论，但不冒充 `L4B` | 把 agent 角色扮演直接记成真人试玩 | 角色扮演仍缺真人时间/耐心/机会成本约束。 |
| `DEC-L4S-003` | 为未来 calibration 留扩展位，但当前不宣称已完成 | 现在就把 `L4A` 提升为 `L4B` 替代品 | 当前仓库没有 synthetic-to-human 校准证据。 |
