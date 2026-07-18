# oasis7: README 与模块 PRD 口径一致性巡检清单（2026-03-11）

- 对应设计文档: `doc/readme/governance/readme-consistency-audit-checklist-2026-03-11.design.md`
- 对应项目管理文档: `doc/readme/governance/readme-consistency-audit-checklist-2026-03-11.project.md`

审计轮次: 4

## 1. Executive Summary
- Problem Statement: `README.md`、`doc/README.md` 与各模块 PRD 共同承担对外入口和项目总览角色，但当前 readme 模块缺少一份可执行的“口径一致性巡检清单”。没有统一检查项时，贡献者只能凭经验判断哪些表述要同步、哪些链接要复核，容易留下术语漂移与过期陈述。
- Proposed Solution: 建立 README 口径一致性巡检清单，覆盖“顶层叙事、状态口径、术语边界、入口链接、同步触发条件”五类检查项，并给出最小巡检方法。
- Success Criteria:
  - SC-1: 巡检清单能直接覆盖 `README.md`、`doc/README.md`、`doc/product/world-rules-core-gameplay/prd.md`、`testing-manual.md` 与模块主 PRD 的一致性复核。
  - SC-2: 每个检查项都明确“检查对象 / 通过条件 / 失败动作”。
  - SC-3: `doc/readme/project.md` 可以据此关闭 `TASK-README-002`。
  - SC-4: 后续 README 更新都能复用该清单，而不是重新定义检查口径。

## 2. User Experience & Functionality
- User Personas:
  - 新贡献者：需要知道 README 哪些信息是权威入口，哪些必须联动更新。
  - `producer_system_designer`：需要确保对外叙事不偏离真实产品状态与世界规则。
  - 文档维护者：需要一份固定清单快速做发布前巡检。
- User Scenarios & Frequency:
  - README 变更后：立即执行一次巡检。
  - 发布前：至少执行一次顶层入口巡检。
  - 模块 PRD / site 口径发生显著变化时：触发交叉复核。
- User Stories:
  - PRD-README-CHECK-001: As a 文档维护者, I want a reusable checklist, so that README sync is not ad hoc.
  - PRD-README-CHECK-002: As a `producer_system_designer`, I want readiness/status wording checked, so that external narrative stays truthful.
  - PRD-README-CHECK-003: As a 新贡献者, I want explicit source-of-truth references, so that I know which doc to update first.
- Critical User Flows:
  1. `发现 README 或模块 PRD 改动 -> 打开巡检清单 -> 逐项检查顶层口径与链接`
  2. `发现冲突 -> 回写 README 或模块 PRD -> 重跑链接/文档治理检查`
  3. `通过巡检 -> 回写 project / devlog -> 进入后续链接自动检查任务`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 巡检项 | 编号、检查对象、通过条件、失败动作 | 逐项核对并记录 | `pending -> pass/fail` | 入口/状态口径优先级最高 | 维护者可执行 |
| 权威源映射 | README 表述、对应模块 PRD / 手册路径 | 冲突时按权威源回写 | `drifted -> synced` | 以模块主 PRD / core 为准 | `producer_system_designer` 可裁定 |
| 同步触发条件 | 何时必须重跑清单 | 变更后触发 | `idle -> triggered -> cleared` | README / site / core 改动优先触发 | 全员可见 |
- Acceptance Criteria:
  - AC-1: 清单明确至少 5 个高优先级巡检项。
  - AC-2: 清单显式覆盖 README 顶层叙事、产品状态、入口链接、术语边界、触发条件。
  - AC-3: 清单引用现有权威源，而不是重复定义模块内容。
  - AC-4: `doc/readme/project.md` 回写该清单路径与验收命令。
- Non-Goals:
  - 不在本轮实现自动化链接检查脚本。
  - 不大改 `README.md` 正文结构。
  - 不替代后续季度审查节奏。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 本专题作为 readme 模块的“人工巡检模板”，连接 `README.md`、`doc/README.md`、核心手册与模块 PRD 的权威源关系。
- Integration Points:
  - `README.md`
  - `doc/README.md`
  - `doc/product/world-rules-core-gameplay/prd.md`
  - `testing-manual.md`
  - `doc/core/prd.md`
  - `doc/site/prd.md`
- Edge Cases & Error Handling:
  - 顶层 README 与模块 PRD 冲突：先按模块主 PRD / core 裁定，再回写 README。
  - 历史 redirect 仍存在：允许保留 redirect，但主入口必须在清单中显式标注。
  - “尚不可玩 / 技术预览”状态变化：必须与 site / core 联动复核。
- Non-Functional Requirements:
  - NFR-RC-1: 清单必须可被 grep 快速检索。
  - NFR-RC-2: 每个检查项都要附失败动作。
  - NFR-RC-3: 清单应可在 10 分钟内完成一次人工巡检。
- Security & Privacy: 巡检过程不得引入敏感配置或临时私有链接。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (`RC-1`): 建立人工巡检清单。
  - v1.1 (`RC-2`): 接入入口链接自动检查任务。
  - v2.0 (`RC-3`): 建立季度节奏与趋势指标。
- Technical Risks:
  - 风险-1: 若清单太长，维护者会跳过执行。
  - 风险-2: 若不绑定权威源，清单会退化成重复描述。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-README-CHECK-001 | `TASK-README-002` / `RC-1` | `test_tier_required` | 检查清单字段与检查项数量 | README 巡检执行一致性 |
| PRD-README-CHECK-002 | `TASK-README-002` / `RC-1` | `test_tier_required` | 抽样检查状态口径 / 权威源引用项 | 对外叙事真实性 |
| PRD-README-CHECK-003 | `TASK-README-002` / `RC-1` | `test_tier_required` | 检查 project / index / handoff 互链 | readme 模块追踪完整性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| `DEC-RC-001` | 先建人工巡检清单 | 直接上自动校验脚本 | 先固定检查口径，再自动化更稳。 |
| `DEC-RC-002` | 以模块主 PRD / core 作为权威源 | 以 README 自身作为最终裁定 | README 是入口层，不应反向统治模块需求。 |
| `DEC-RC-003` | 把产品状态口径列为高优先检查项 | 只检查链接和术语 | 当前对外信任风险不只来自断链，也来自状态误导。 |
