# oasis7: README 入口链接有效性自动检查（2026-03-11）

- 对应设计文档: `doc/readme/governance/readme-link-check-automation-2026-03-11.design.md`
- 对应项目管理文档: `doc/readme/governance/readme-link-check-automation-2026-03-11.project.md`

审计轮次: 4

## 治理状态（2026-06-18）
- 状态：历史压缩专题，保留原址与互链用于追溯，不再作为默认活跃阅读面。
- 当前入口：`doc/readme/project.md`、`doc/readme/prd.index.md`、`doc/readme/governance/readme-consistency-audit-checklist-2026-03-11.prd.md` 与 `doc/readme/governance/readme-quarterly-review-cycle-2026-03-11.prd.md`。
- 保留原因：本文记录 `scripts/readme-link-check.sh` 的原始范围、验收标准和任务收口证据，不替代当前 README 治理节奏或后续复核专题。

## 1. Executive Summary
- Problem Statement: `TASK-README-002` 已定义人工巡检清单，但 `README.md` 与 `doc/README.md` 的入口链接仍缺最小自动检查任务。仅靠人工复核时，断链很容易在重构或移动文档后被漏掉。
- Proposed Solution: 新增 `scripts/readme-link-check.sh`，自动扫描 `README.md` 与 `doc/README.md` 中的本地 Markdown 链接，并在引用目标不存在时失败。
- Success Criteria:
  - SC-1: 脚本默认检查 `README.md` 与 `doc/README.md`。
  - SC-2: 本地 Markdown 链接断链时脚本返回非零并输出具体来源文件与目标路径。
  - SC-3: `doc/readme/project.md` 可以据此关闭 `TASK-README-003`。
  - SC-4: 脚本不误报 `http/https/mailto/tel` 外链。

## 2. User Experience & Functionality
- User Personas:
  - 文档维护者：需要快速发现顶层入口断链。
  - `qa_engineer`：需要一个可重复执行的最小入口检查命令。
  - 新贡献者：希望 README 入口永远可点开。
- User Scenarios & Frequency:
  - 每次 README / `doc/README.md` 更新后执行。
  - 发布前执行一遍最小入口检查。
  - 文档迁移或重命名后执行回归。
- User Stories:
  - PRD-README-LINK-001: As a 文档维护者, I want an automated local link check, so that README breakage is caught immediately.
  - PRD-README-LINK-002: As a `qa_engineer`, I want a deterministic script output, so that failures can be classified and fixed quickly.
  - PRD-README-LINK-003: As a 新贡献者, I want top-level docs links to work, so that onboarding is not interrupted by broken references.
- Critical User Flows:
  1. `更新 README -> 执行 scripts/readme-link-check.sh -> 获取 pass/fail 结论`
  2. `出现断链 -> 根据错误输出回写路径 -> 重跑脚本直至通过`
  3. `将脚本接入后续 readme 治理任务 -> 与人工清单协同使用`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| README 链接扫描 | 文件路径、引用、解析目标 | 读取 Markdown 链接并解析本地目标 | `unchecked -> pass/fail` | 仅扫描本地 Markdown 链接 | 全员可执行 |
| 失败输出 | 来源文件、原始引用、解析目标 | 断链时输出错误 | `fail_reported -> fixed` | 每条断链逐条输出 | 维护者修复 |
| 外链豁免 | `http/https/mailto/tel` | 自动跳过 | `ignored` | 外链不纳入本地断链范围 | 自动处理 |
- Acceptance Criteria:
  - AC-1: `scripts/readme-link-check.sh` 可执行并通过当前仓库。
  - AC-2: 脚本只检查 `README.md` 与 `doc/README.md`。
  - AC-3: readme 模块 project / index / handoff 完成回写。
  - AC-4: 脚本输出能指出断链来源文件与目标路径。
- Non-Goals:
  - 不检查所有 Markdown 文件。
  - 不检查外链可达性。
  - 不替代 `doc-governance-check.sh` 的全仓引用检查。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: Bash / grep / realpath。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 该脚本是 readme 模块的最小自动检查入口，只负责顶层 README 本地 Markdown 引用的存在性校验。
- Integration Points:
  - `scripts/readme-link-check.sh`
  - `README.md`
  - `doc/README.md`
  - `doc/readme/project.md`
- Edge Cases & Error Handling:
  - 锚点链接：忽略锚点后的局部段落，仅检查文件路径本体。
  - 查询参数：忽略 `?query` 后再检查本地目标。
  - 外链：全部跳过，不纳入失败。
  - 同文件相对路径：按引用文件所在目录解析。
- Non-Functional Requirements:
  - NFR-RL-1: 脚本在 Linux/macOS shell 环境可执行。
  - NFR-RL-2: 输出必须包含来源文件和解析后的目标路径。
  - NFR-RL-3: 默认运行时间应在秒级。
- Security & Privacy: 仅检查本地路径，不发起网络请求。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (`RL-1`): 实现 README / `doc/README.md` 本地链接检查脚本。
  - v1.1 (`RL-2`): 与人工巡检清单联动。
  - v2.0 (`RL-3`): 视需要扩展到更多入口文档。
- Technical Risks:
  - 风险-1: Markdown 链接解析若过度复杂，可能引入误报。
  - 风险-2: 若后续 README 使用更复杂语法，脚本需要同步升级。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-README-LINK-001 | `TASK-README-003` / `RL-1` | `test_tier_required` | 执行 `scripts/readme-link-check.sh` | README 入口可达性 |
| PRD-README-LINK-002 | `TASK-README-003` / `RL-1` | `test_tier_required` | 抽查失败输出字段设计 | QA 失败定位效率 |
| PRD-README-LINK-003 | `TASK-README-003` / `RL-1` | `test_tier_required` | project / index / handoff 互链检查 | readme 模块追踪完整性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| `DEC-RL-001` | 先只检查 `README.md` 与 `doc/README.md` | 一次性扫描全仓 | 先守住顶层入口，范围最稳。 |
| `DEC-RL-002` | 只检查本地链接存在性 | 同时检查网络外链可达 | 外链检查不稳定且不适合作为本轮最小门禁。 |
| `DEC-RL-003` | 独立脚本承接 readme 自动检查 | 继续挤进 `doc-governance-check.sh` | readme 模块需要独立可引用的最小入口。 |
