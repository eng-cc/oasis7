# oasis7: engineering 季度治理审查与修复节奏（2026-03-11）

- 对应设计文档: `doc/engineering/governance/engineering-quarterly-governance-review-cycle-2026-03-11.design.md`
- 对应项目管理文档: `doc/engineering/governance/engineering-quarterly-governance-review-cycle-2026-03-11.project.md`

审计轮次: 4

## 1. Executive Summary
- Problem Statement: `TASK-ENGINEERING-003` 已经建立 engineering 门禁趋势基线，但主项目仍缺“何时复查、谁主持、发现问题如何建项、如何把趋势基线真正转成治理动作”的固定节奏。如果没有季度审查流程，趋势 baseline 很容易停留在一次性报告，无法稳定驱动门禁演进。
- Proposed Solution: 建立 engineering 季度治理审查与修复节奏，定义季度审查触发、固定输入、角色协作、问题升级与修复记录模板，并将 `doc-governance-check.sh` 与 engineering trend baseline 作为标准输入。
- Success Criteria:
  - SC-1: 季度审查节奏明确 `谁/何时/检查什么/如何回写`。
  - SC-2: 至少包含季度审查模板与修复记录模板。
  - SC-3: 模板显式复用 `doc/engineering/evidence/engineering-governance-trend-baseline-2026-03-11.md` 与 `scripts/doc-governance-check.sh`。
  - SC-4: `doc/engineering/project.md` 可以据此关闭 `TASK-ENGINEERING-004` 并推进到剩余迁移任务。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`：需要一个稳定节奏保证 engineering 治理不是临时救火。
  - `qa_engineer`：需要知道何时复核趋势基线、何时抽样门禁输出、何时阻断收口。
  - 工程维护者：需要固定模板来登记治理问题、优先级与修复责任。
- User Scenarios & Frequency:
  - 每季度初：执行一次完整 engineering 治理审查。
  - 重大治理规则变化后：触发一次临时加审。
  - 季度结束：复盘修复项是否收口、趋势是否恶化。
- User Stories:
  - PRD-ENGINEERING-CYCLE-001: As a `producer_system_designer`, I want a quarterly governance review cadence, so that engineering rules evolve proactively instead of reactively.
  - PRD-ENGINEERING-CYCLE-002: As a `qa_engineer`, I want a fixed review template that consumes the trend baseline, so that each cycle is comparable and auditable.
  - PRD-ENGINEERING-CYCLE-003: As an 工程维护者, I want a remediation template, so that governance findings become actionable tracked work.
- Critical User Flows:
  1. `进入季度窗口 -> 读取 trend baseline + 运行 doc-governance-check -> 记录问题`
  2. `发现问题 -> 生成 remediation 项 -> 指定 owner / 优先级 / 截止时间`
  3. `季度结束 -> 复核修复项关闭情况 -> 把结果回写 project / .pm execution log / role review evidence`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 季度审查节奏 | 季度编号、触发时间、owner、联审角色、输入 | 到期或重大规则变化时触发 | `planned -> running -> closed` | 正常按季度执行，重大变化可插队 | owner 负责发起 |
| 审查模板 | 基线版本、检查项、结论、风险、阻断、回写文件 | 逐项填写 | `draft -> reviewed -> archived` | 高风险项先检查 | `producer_system_designer` / `qa_engineer` 共用 |
| 修复模板 | 问题ID、影响范围、责任人、优先级、截止时间、关闭证据 | 审查失败后立刻建项 | `open -> fixing -> closed` | 门禁失败 > 趋势恶化 > 模板缺口 | owner 裁定优先级 |
- Acceptance Criteria:
  - AC-1: 新专题明确季度节奏、触发条件与角色分工。
  - AC-2: 产出季度审查模板与修复记录模板，且显式引用 trend baseline 与 `doc-governance-check.sh`。
  - AC-3: 模板支持记录阻断项、趋势结论与 remediation owner。
  - AC-4: `doc/engineering/project.md` 回写完成态与下一任务状态。
- Non-Goals:
  - 不在本轮执行真实季度审查。
  - 不新增自动化调度系统。
  - 不重定义 `TASK-ENGINEERING-003` 已冻结的指标口径。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: `scripts/doc-governance-check.sh`。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 本专题将 engineering 门禁治理从“任务式修复”提升为“周期性审查流程”，以 trend baseline 作为趋势输入，以 `doc-governance-check.sh` 作为规则验证输入，以 remediation 模板作为问题闭环出口。
- Integration Points:
  - `doc/engineering/evidence/engineering-governance-trend-baseline-2026-03-11.md`
  - `scripts/doc-governance-check.sh`
  - `doc/engineering/governance/engineering-quarterly-review-template-2026-03-11.md`
  - `doc/engineering/governance/engineering-governance-remediation-log-template-2026-03-11.md`
  - `doc/engineering/project.md`
- Edge Cases & Error Handling:
  - 趋势为绿但门禁抽样失败：仍判定为 `fix_required`。
  - 门禁通过但趋势恶化：允许进入 `watchlist`，并要求下季度继续追踪。
  - 修复项跨模块：模板中必须指定 owner role 与回写位置。
- Non-Functional Requirements:
  - NFR-EQC-1: 审查模板应能在 20 分钟内完成一次最小执行。
  - NFR-EQC-2: 修复模板必须支持优先级、阻断与验证证据。
  - NFR-EQC-3: 节奏文档需可被 grep 快速检索。
- Security & Privacy: 模板仅记录仓内路径、问题和结论，不记录敏感凭据。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (`EQC-1`): 建立季度节奏与模板。
  - v1.1 (`EQC-2`): 执行首轮真实季度审查并归档样本。
  - v2.0 (`EQC-3`): 当样本规模足够时，再评估自动汇总或提醒机制。
- Technical Risks:
  - 风险-1: 若模板过重，季度执行率会下降。
  - 风险-2: 若 remediation owner 不清晰，问题会停留在文档层而非真正关闭。
  - 风险-3: 若 trend baseline 不续写，季度审查会退化为静态 checklist。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-ENGINEERING-CYCLE-001 | `TASK-ENGINEERING-004` / `EQC-1` | `test_tier_required` | 检查季度节奏字段、角色分工与触发条件 | engineering 治理节奏稳定性 |
| PRD-ENGINEERING-CYCLE-002 | `TASK-ENGINEERING-004` / `EQC-1` | `test_tier_required` | 检查审查模板引用 trend baseline 与 `doc-governance-check.sh` | QA / engineering 协作一致性 |
| PRD-ENGINEERING-CYCLE-003 | `TASK-ENGINEERING-004` / `EQC-1` | `test_tier_required` | 检查 remediation 模板字段、project 回写与 `.pm`/role-review 证据 | 问题闭环能力 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| `DEC-EQC-001` | 先固定季度模板与节奏 | 直接做一次临时复盘 | 没有模板时，季度治理难以持续复用。 |
| `DEC-EQC-002` | trend baseline 与 gate 脚本双输入 | 只看 baseline 或只跑脚本 | 趋势与即时规则检查缺一不可。 |
| `DEC-EQC-003` | remediation 独立建模板 | 只把问题写在历史 devlog | 季度治理问题需要正式追踪、owner 与 task evidence。 |
