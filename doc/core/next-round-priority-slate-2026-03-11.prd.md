# oasis7: 下一轮跨模块优先级清单（2026-03-11）

- 对应设计文档: `doc/core/next-round-priority-slate-2026-03-11.design.md`
- 对应项目管理文档: `doc/core/next-round-priority-slate-2026-03-11.project.md`

审计轮次: 4

## 1. Executive Summary
- Problem Statement: 2026-03-11 本轮连续收口后，12 个模块主项目都已进入 completed，但仓内缺一个“下一轮先做什么、为什么先做、由谁接”的正式入口。若没有统一优先级清单，团队容易重新回到平均发力与隐式尾注推进。
- Proposed Solution: 在 core 建立下一轮跨模块优先级清单，统一定义 P0/P1/P2 候选、排序依据、owner role、输入输出与最高优先级建议，并将第一优先级固定为“发布候选 readiness 统一入口”。
- Success Criteria:
  - SC-1: 至少形成 3 档优先级与排序依据。
  - SC-2: 每个优先级项都写明 owner role、输入、输出与进入条件。
  - SC-3: 明确下一条执行主路径，并能回写到 `doc/core/project.md`。
  - SC-4: 后续新任务不再靠口头排序，而是从该清单进入。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`：需要用统一优先级而不是直觉来推进下一轮。
  - 模块 owner：需要知道为什么自己是现在做、稍后做，还是暂不做。
  - `qa_engineer`：需要知道下一轮验证入口先围绕哪条链路建立证据。
- User Scenarios & Frequency:
  - 一轮主项目收口后：生成下一轮优先级清单。
  - 新需求进入时：按该清单判断是否直接插队。
  - 周期性复盘时：校验当前排序是否仍成立。
- User Stories:
  - PRD-CORE-PRIORITY-001: As a `producer_system_designer`, I want a ranked next-round slate, so that I can prevent the team from diffusing effort.
  - PRD-CORE-PRIORITY-002: As a 模块 owner, I want owner/input/output stated for each priority, so that handoff starts with clear boundaries.
  - PRD-CORE-PRIORITY-003: As a `qa_engineer`, I want the top priority linked to an evidence path, so that verification focus is unambiguous.
- Critical User Flows:
  1. `确认本轮主项目全部 completed -> 汇总剩余高价值缺口 -> 形成候选池`
  2. `按发布影响 / 闭环依赖 / owner 就绪度排序 -> 划分 P0/P1/P2 -> 指定下一条主路径`
  3. `把第一优先级回写到 core project -> 后续 owner 按此开专题 PRD / project`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| priority candidate | 优先级、主题、owner、输入、输出、进入条件、阻断 | 收口后登记候选 | `candidate -> ranked -> selected` | 先看发布影响，再看依赖闭环，再看 owner 就绪度 | `producer_system_designer` 排序 |
| top priority selection | 第一优先级主题、理由、前置、证据入口 | 选为下一条执行主路径 | `selected -> planned` | 必须服务下一轮最短闭环 | `producer_system_designer` 决定，`qa_engineer` 复核 |
| handoff readiness | 发起角色、接收角色、输入、输出、done | 作为后续专题开工前置 | `defined -> acknowledged` | owner 边界清晰项优先 | 发起方填写，接收方确认 |
- Acceptance Criteria:
  - AC-1: 优先级清单至少包含 1 个 P0、1 个 P1、1 个 P2 候选。
  - AC-2: 第一优先级明确为“发布候选 readiness 统一入口”。
  - AC-3: `doc/core/project.md` 增加对应任务并把下一任务指向第一优先级执行项。
  - AC-4: 形成 `producer_system_designer -> qa_engineer` handoff。
- Non-Goals:
  - 不在本专题直接实现第一优先级功能本身。
  - 不重新打开已 completed 的旧模块主项目。
  - 不给所有模块同时派发新任务。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 该专题是 core 层的调度与裁剪入口，消费当前模块完成态、已有证据链与核心风险列表，输出下一轮排序与第一优先级建议。
- Integration Points:
  - 历史输入：2026-03-11 模块主项目收口总览已退役删除；当前追溯以本专题三件套、`doc/core/project.md`、`doc/core/prd.index.md` 与 GitHub task issue evidence comments 为准。
  - `doc/core/project.md`
  - `doc/core/prd.md`
  - 各模块 `project.md`
- Edge Cases & Error Handling:
  - 某模块虽 completed 但仍有隐式尾注：需先修正文档状态，再纳入排序。
  - 新需求紧急插队：必须说明为什么覆盖当前第一优先级。
  - owner 未就绪：候选只能保留在 P1/P2，不得直接升为当前执行主路径。
- Non-Functional Requirements:
  - NFR-CPS-1: 优先级清单必须能在一页内被快速审阅。
  - NFR-CPS-2: 每个候选都必须可追溯到具体模块或证据缺口。
  - NFR-CPS-3: 第一优先级切换后 1 个工作日内必须回写 core project。
- Security & Privacy: 仅处理仓内文档与治理信息，不涉及敏感数据。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (`CPS-1`): 形成首份下一轮优先级清单并选定第一优先级。
  - v1.1 (`CPS-2`): 将第一优先级拆成正式专题 PRD / project。
  - v2.0 (`CPS-3`): 建立周期性优先级复盘节奏。
- Technical Risks:
  - 风险-1: 若优先级清单过泛，会再次退化成“所有都重要”。
  - 风险-2: 若第一优先级缺少 owner 与证据入口，仍会回到口头推动。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-CORE-PRIORITY-001 | `TASK-CORE-016` | `test_tier_required` | 检查 P0/P1/P2 候选与排序依据存在 | 下一轮规划一致性 |
| PRD-CORE-PRIORITY-002 | `TASK-CORE-016` | `test_tier_required` | 检查 owner / 输入 / 输出 / 进入条件存在 | 跨角色 handoff 清晰度 |
| PRD-CORE-PRIORITY-003 | `TASK-CORE-016/017` | `test_tier_required` | 检查第一优先级已回写 core project | 下一轮执行入口稳定性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| `DEC-CPS-001` | 先做优先级清单，再拆第一优先级专题 | 直接随机选择一个模块继续写 | 先统一排序，才能避免再次平均发力。 |
| `DEC-CPS-002` | 第一优先级选择发布候选 readiness 统一入口 | 先做体验 polish 或新增 launcher 功能 | 当前最高价值缺口是统一候选级放行入口，而不是新增展示能力。 |
| `DEC-CPS-003` | 已 completed 模块不重新打开主项目 | 在旧模块主项目里继续追加隐式尾注 | 保持 completed 语义稳定，新需求走新任务。 |
