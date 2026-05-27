# oasis7: 工程门禁趋势跟踪（2026-03-11）

- 对应设计文档: `doc/engineering/governance/engineering-governance-trend-tracking-2026-03-11.design.md`
- 对应项目管理文档: `doc/engineering/governance/engineering-governance-trend-tracking-2026-03-11.project.md`

审计轮次: 4

## 1. Executive Summary
- Problem Statement: engineering 模块已经逐步补齐双向互链、任务测试分层、引用路径可达与角色名白名单等治理门禁，但主项目仍缺统一趋势入口来回答“违规是在减少，还是只靠单次集中修复收口”。缺少趋势统计时，评审只能看到某次门禁通过，无法判断治理质量是否稳定、修复是否足够快、已修问题是否反复回归。
- Proposed Solution: 建立 engineering 门禁趋势专题，统一定义 `违规率`、`修复时长`、`回归率` 的统计口径、样本字段、红黄绿阈值与 baseline 发布方式，并以 `TASK-ENGINEERING-017/018/019` 形成首份可审计基线。
- Success Criteria:
  - SC-1: `TASK-ENGINEERING-003` 具备唯一权威文档入口，可追溯到 `doc/engineering/project.md`。
  - SC-2: 三项指标均有明确公式、样本范围、阈值与解释方式。
  - SC-3: 首份 baseline 至少覆盖 3 个近期 engineering 门禁样本，并能追溯到 project/devlog。
  - SC-4: baseline 可被 `producer_system_designer` / `qa_engineer` 后续续写，不依赖口头解释。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`：需要判断工程治理是在持续收敛还是只是阶段性止血。
  - `qa_engineer`：需要一个固定基线，用于复核门禁增强是否真正降低文档治理风险。
  - 工程维护者：需要知道问题集中在违规残留、修复速度还是回归控制。
- User Scenarios & Frequency:
  - 每完成一次工程治理门禁任务后：追加一个趋势样本。
  - 周度或阶段复盘时：读取近窗口 baseline，判断是否需要补治理动作。
  - 季度治理审查前：以趋势基线为输入，判断是否需要调门禁或补模板。
- User Stories:
  - PRD-ENGINEERING-TREND-001: As a `producer_system_designer`, I want a canonical governance trend baseline, so that I can assess whether engineering governance is actually converging.
  - PRD-ENGINEERING-TREND-002: As a `qa_engineer`, I want auditable formulas for violation rate, repair duration, and regression rate, so that I can defend review decisions.
  - PRD-ENGINEERING-TREND-003: As an 工程维护者, I want each baseline sample linked to concrete project/devlog evidence, so that I can reproduce why a metric changed.
- Critical User Flows:
  1. `治理任务收口 -> 读取 project / devlog / evidence -> 归档 trend sample`
  2. `计算违规率 / 修复时长 / 回归率 -> 输出红黄绿状态 -> 发布 baseline`
  3. `阶段评审读取 baseline -> 识别偏红指标 -> 决定是否补门禁或调整流程`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| trend sample 归档 | `sample_id`、任务、规则范围、样本基数、残留违规数、发现/关闭日期、回归数、证据入口 | 每个治理收口任务回写一行样本 | `identified -> measured -> baselined` | 按关闭日期倒序 | `producer_system_designer` / `qa_engineer` 可写 |
| 违规率 | `remaining_violations / audited_scope_total` | 基于样本表复算 | `unknown -> green/yellow/red` | 聚合窗口内按总残留数 / 总样本基数 | 评审者维护阈值 |
| 修复时长 | `close_date - identify_date` | 按样本自然日计算均值 | `unknown -> green/yellow/red` | 样本缺日期则标记 `incomplete_sample` 并不纳入均值 | 评审者与维护者可续写 |
| 回归率 | `regressed_samples / closed_samples` | 对照后续样本与门禁记录 | `unknown -> green/yellow/red` | 同一规则在后续样本再次触发计 1 | 评审者可裁定 |
| baseline 发布 | 窗口、样本表、阈值、结论、建议动作 | 生成 Markdown baseline | `draft -> published -> superseded` | 窗口按日期稳定排序 | 全员可读 |
- Acceptance Criteria:
  - AC-1: baseline 文档显式写出 3 个指标公式、阈值、样本数与统计窗口。
  - AC-2: 首份 baseline 至少引用 3 个 engineering 门禁任务样本。
  - AC-3: 每个样本均可追溯到 `doc/engineering/project.md` 或 `doc/devlog/*.md` 的明确记录。
  - AC-4: `TASK-ENGINEERING-003` 完成后，engineering 主项目下一任务推进到 `TASK-ENGINEERING-004`。
- Non-Goals:
  - 不在本轮实现自动汇总脚本。
  - 不追溯所有历史工程任务，只覆盖近期门禁增强样本。
  - 不把趋势统计直接外推为代码质量全貌。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 该专题基于 engineering 主项目、门禁脚本演进记录与 devlog 样本，手工沉淀工程门禁趋势基线；优先保证可审计与可续写，再考虑自动化。
- Integration Points:
  - `doc/engineering/project.md`
  - `doc/engineering/prd.md`
  - `doc/devlog/README.md`
  - `doc/devlog/README.md`
  - `scripts/doc-governance-check.sh`
  - `doc/engineering/evidence/engineering-governance-trend-baseline-2026-03-11.md`
- Edge Cases & Error Handling:
  - 某个任务只引入门禁、没有明确发现日期：可使用该任务在 devlog 的首条记录时间作为发现时间。
  - 同一任务同时修复多类问题：允许在单条样本中用 `audited_scope_total` 与备注说明覆盖范围。
  - 样本全部为绿但窗口很小：baseline 必须显式写出 `small_sample` 风险。
- Non-Functional Requirements:
  - NFR-EGT-1: 指标必须能由仓内文档手工复算。
  - NFR-EGT-2: baseline 必须可被 `rg` 快速检索到样本与结论。
  - NFR-EGT-3: 后续追加样本不要求修改已有门禁脚本。
- Security & Privacy: 仅使用仓内治理文档与脚本记录，不涉及凭据与外部数据。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (`EGT-1`): 冻结样本字段、公式、阈值与首份 baseline。
  - v1.1 (`EGT-2`): 将 `TASK-ENGINEERING-004` 季度审查模板接入本趋势基线。
  - v2.0 (`EGT-3`): 若样本数持续增长，再评估自动汇总脚本或 CI 导出。
- Technical Risks:
  - 风险-1: 当前样本主要来自近期门禁增强任务，窗口较窄，更适合作为方向性基线。
  - 风险-2: 若后续没有稳定追加样本，趋势结论会退化为静态报告。
  - 风险-3: 某些治理问题跨多个任务修复，需在备注里写清口径，避免重复计数。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-ENGINEERING-TREND-001 | `TASK-ENGINEERING-003` / `EGT-1` | `test_tier_required` | 检查指标定义、阈值与样本字段存在 | engineering 治理可读性 |
| PRD-ENGINEERING-TREND-002 | `TASK-ENGINEERING-003` / `EGT-1` | `test_tier_required` | 检查 baseline 对 `TASK-ENGINEERING-017/018/019` 的追溯与复算 | 工程门禁可审计性 |
| PRD-ENGINEERING-TREND-003 | `TASK-ENGINEERING-003` / `EGT-1` | `test_tier_required` | 检查 handoff 与主项目状态回写 | engineering 项目追踪一致性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| `DEC-EGT-001` | 先做文档化 baseline | 直接实现自动报表脚本 | 当前任务重点是审计口径冻结，不是工具链扩展。 |
| `DEC-EGT-002` | baseline 先覆盖近期门禁增强样本 | 回填全部历史治理任务 | 先用近因样本建立稳定格式，更利于持续续写。 |
| `DEC-EGT-003` | 指标采用“违规率 + 修复时长 + 回归率”三件套 | 只看门禁通过/失败 | 通过/失败不足以解释治理趋势，缺乏持续改进信息。 |
