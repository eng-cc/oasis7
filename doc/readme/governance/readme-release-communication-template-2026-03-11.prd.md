# oasis7: 对外口径简报模板（2026-03-11）

- 对应设计文档: `doc/readme/governance/readme-release-communication-template-2026-03-11.design.md`
- 对应项目管理文档: `doc/readme/governance/readme-release-communication-template-2026-03-11.project.md`

审计轮次: 4

## 1. Executive Summary
- Problem Statement: 已经有首份版本候选对外口径简报，但如果每次新候选都从头撰写，`liveops_community` 与 `producer_system_designer` 仍会重复讨论结构、边界和禁用表述。
- Proposed Solution: 抽象一份对外口径简报模板，固定状态摘要、禁用表述、残余风险、回滚口径和审批链字段，使后续候选只需填充实际内容。
- Success Criteria:
  - SC-1: 模板覆盖状态摘要、风险边界、禁用表述、rollback note、审批链五类必填字段。
  - SC-2: 模板能直接映射到历史压缩的 `readme-release-candidate-communication-brief-2026-03-11.prd.md`。
  - SC-3: `liveops_community` 后续可在不重写结构的前提下生成新版本简报。
  - SC-4: readme 主项目可追踪模板化闭环，而非停留在单次实例。

## 2. User Experience & Functionality
- User Personas:
  - `liveops_community`：需要快速生成下一份对外口径简报。
  - `producer_system_designer`：需要固定审核面，避免每次都重新讨论必填项。
  - `qa_engineer`：需要知道哪些字段必须能回链到内部证据。
- User Scenarios & Frequency:
  - 新版本候选 ready / go 后：从模板生成简报。
  - 口径复核时：用模板核对缺失字段。
  - 事故回退时：沿模板中的 rollback 字段快速更新。
- User Stories:
  - PRD-README-TEMPLATE-001: As a `liveops_community`, I want a reusable release communication template, so that each candidate brief starts from a stable structure.
  - PRD-README-TEMPLATE-002: As a `producer_system_designer`, I want required fields and forbidden-claim slots predefined, so that review stays fast and consistent.
  - PRD-README-TEMPLATE-003: As a `qa_engineer`, I want evidence-link fields in the template, so that external messaging can be traced back to internal review artifacts.
- Critical User Flows:
  1. `复制模板 -> 填写候选 ID / 内部结论 -> 回写状态摘要`
  2. `补齐禁用表述 / 风险 / rollback -> 提交审核`
  3. `完成 liveops -> producer 审核 -> 作为对外沟通底稿`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| brief template header | 候选 ID、内部来源、owner、review role、当前状态 | 复制后填写实例 | `template -> instantiated -> reviewed` | 头部字段必须先填 | `liveops_community` 维护 |
| required sections | 状态摘要、禁用表述、风险、回滚、使用说明 | 逐节填写 | `empty -> drafted -> approved` | 先事实后边界后风险 | `producer_system_designer` 审核 |
| evidence link slots | 内部 go/no-go / readiness 路径 | 绑定内部依据 | `missing -> linked` | 缺任一关键链接不得 approved | `qa_engineer` 可复核 |
- Acceptance Criteria:
  - AC-1: 产出模板专题 PRD / Design / Project。
  - AC-2: 产出一份可复制复用的对外口径简报模板。
  - AC-3: 模板明确必填字段与禁止跳过的链接字段。
  - AC-4: `doc/readme/project.md` 能追踪模板化任务完成。
- Non-Goals:
  - 不生成新的具体候选简报实例。
  - 不取代正式 announcement/changelog 模板。
  - 不改变现有实例结论。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 该模板专题位于 readme/governance，复用首份 communication brief 的结构，把单次实例上升为固定模板能力。
- Integration Points:
  - `doc/readme/governance/readme-release-candidate-communication-brief-2026-03-11.prd.md`
  - `doc/readme/governance/readme-release-candidate-communication-brief-2026-03-11.project.md`
  - GitHub task issue evidence comments for pre-PR local role review evidence in future instantiated release communication tasks
  - `doc/readme/project.md`
- Edge Cases & Error Handling:
  - 实例缺失内部来源：模板必须显式标红“不得发布”。
  - 某节不适用：必须写 `N/A + 原因`，不得留空。
  - 候选状态变化：模板要求更新日期和版本，不允许复用旧状态。
- Non-Functional Requirements:
  - NFR-TEMPLATE-1: 模板实例化时间应 <= 15 分钟。
  - NFR-TEMPLATE-2: 模板字段可在一次 `rg` 检查中完整发现。
  - NFR-TEMPLATE-3: 模板必须保持与首份实例结构一致。
- Security & Privacy: 模板不预填敏感路径或运行目录，只保留字段占位与说明。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (`COMM-T1`): 抽出首份简报模板。
  - v1.1 (`COMM-T2`): 评估是否联动 announcement / changelog 模板。
  - v2.0 (`COMM-T3`): 若形成固定节奏，纳入 readme 周期治理。
- Technical Risks:
  - 风险-1: 若模板过重，LiveOps 会回到临时自由发挥。
  - 风险-2: 若模板与实例结构脱节，后续复用会失效。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-README-TEMPLATE-001 | `TASK-README-007` | `test_tier_required` | 检查模板与现有简报结构互链 | 口径模板复用能力 |
| PRD-README-TEMPLATE-002 | `TASK-README-007` | `test_tier_required` | 检查禁用表述 / 风险 / rollback / 审批字段存在 | 审核一致性 |
| PRD-README-TEMPLATE-003 | `TASK-README-007` | `test_tier_required` | 检查 evidence-link 字段存在 | 对外口径可追溯性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| `DEC-TEMPLATE-001` | 从首份实例反推模板 | 继续每次手写简报 | 有现成实例可抽象，复用成本最低。 |
| `DEC-TEMPLATE-002` | 模板保留 evidence-link 字段 | 只保留外部文案字段 | 对外口径必须可回链内部证据。 |
| `DEC-TEMPLATE-003` | 模板继续由 liveops 起草、producer 审核 | 模板仅作为 readme 静态示例 | 审核链是模板能力的一部分。 |
