# oasis7: 脚本稳定性趋势跟踪指标（2026-03-11）

- 对应设计文档: `doc/scripts/governance/script-stability-trend-tracking-2026-03-11.design.md`
- 对应项目管理文档: `doc/scripts/governance/script-stability-trend-tracking-2026-03-11.project.md`

审计轮次: 4

## 1. Executive Summary
- Problem Statement: `TASK-SCRIPTS-002/003` 已分别解决“用哪个脚本”和“怎么调脚本”，但 scripts 模块还缺少一个能持续回答“脚本治理是否在变稳”的趋势视角。没有趋势指标，就无法判断入口漂移、契约缺口或 fallback 误用是否在下降。
- Proposed Solution: 建立 scripts 稳定性趋势专题，统一定义 `主入口覆盖率`、`参数契约覆盖率`、`fallback 围栏覆盖率`、`治理修复时长` 四项指标，并发布首份 baseline。
- Success Criteria:
  - SC-1: 四项指标具备明确公式、样本口径和红黄绿阈值。
  - SC-2: baseline 至少覆盖 `TASK-SCRIPTS-001/002/003` 三个近期治理样本。
  - SC-3: `doc/scripts/project.md` 能据此关闭 `TASK-SCRIPTS-004`。
  - SC-4: 后续 scripts 治理任务可按同一字段续写趋势。

## 2. User Experience & Functionality
- User Personas:
  - `runtime_engineer`：需要知道 scripts 治理是否真的降低了入口漂移与误用风险。
  - `qa_engineer`：需要知道 fallback 是否越来越受控。
  - CI 维护者：需要知道高频脚本契约覆盖是否持续提升。
- User Scenarios & Frequency:
  - 每完成一轮 scripts 治理任务时追加一条样本。
  - 阶段收口前读取 baseline 判断是否仍需补契约或清理入口漂移。
  - 出现脚本误用或 help 漂移时回查最近趋势。
- User Stories:
  - PRD-SCRIPTS-TREND-001: As a `runtime_engineer`, I want a stability metric set, so that I can see whether script governance is converging.
  - PRD-SCRIPTS-TREND-002: As a `qa_engineer`, I want fallback coverage tracked, so that Web-first and diagnostic boundaries remain explicit.
  - PRD-SCRIPTS-TREND-003: As a CI maintainer, I want contract coverage tracked, so that canonical script usage stays auditable.
- Critical User Flows:
  1. `完成 scripts 治理任务 -> 生成一条趋势样本 -> 回写 baseline`
  2. `读取 baseline -> 对照红黄绿阈值 -> 判断下一拍优先补入口/契约/趋势哪一项`
  3. `出现脚本误用 -> 回查样本来源 -> 定位缺口是入口、契约还是 fallback 围栏`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 主入口覆盖率 | `mapped_primary_scripts / target_primary_scripts` | 统计高频主入口是否均已归档 | `unknown -> green/yellow/red` | 以当前高频样本集为分母 | 全员可读 |
| 参数契约覆盖率 | `contracted_primary_scripts / target_primary_scripts` | 统计已补契约的主入口数量 | `unknown -> green/yellow/red` | 与主入口分母一致 | 维护者可更新 |
| fallback 围栏覆盖率 | `fenced_fallback_scripts / target_fallback_scripts` | 统计 fallback 是否有触发条件与替代主入口 | `unknown -> green/yellow/red` | 仅统计已识别 fallback | `qa_engineer` 可引用 |
| 治理修复时长 | `closed_at - detected_at` | 衡量从识别缺口到文档收口的耗时 | `unknown -> green/yellow/red` | 以自然日均值计算 | owner 可更新 |
- Acceptance Criteria:
  - AC-1: 专题文档明确四项趋势指标定义与阈值。
  - AC-2: baseline 文件包含至少 3 个样本并给出指标值。
  - AC-3: 样本可追溯到 scripts 模块 project / 专题 project / devlog。
  - AC-4: `doc/scripts/project.md` 将 `TASK-SCRIPTS-004` 标记完成。
- Non-Goals:
  - 不在本轮实现自动统计脚本。
  - 不统计所有低频历史脚本。
  - 不把趋势指标直接等同于脚本运行成功率。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 该专题基于 scripts 模块 project、专题 project 与 devlog 文档手工生成趋势样本，持续跟踪治理收敛情况，而不是运行时执行结果。
- Integration Points:
  - `doc/scripts/project.md`
  - `doc/scripts/governance/script-entry-layering-2026-03-11.project.md`
  - `doc/scripts/governance/script-parameter-contracts-2026-03-11.project.md`
  - `doc/devlog/README.md`
  - `doc/devlog/README.md`
- Edge Cases & Error Handling:
  - 样本只有任务完成结论、缺少发现日期：可记为 `incomplete_sample`，不纳入修复时长均值。
  - 单个任务同时补多个指标：允许在多个覆盖率分子中同时计数。
  - 指标全部为 100% 但样本量很小：baseline 必须显式声明当前只是首个基线，不代表长期稳态。
- Non-Functional Requirements:
  - NFR-SST-1: 样本与指标公式必须能由文档复算。
  - NFR-SST-2: baseline 必须可被 grep 快速检索。
  - NFR-SST-3: 后续追加样本不要求改动既有脚本实现。
- Security & Privacy: 仅使用仓内治理文档，不涉及敏感凭据。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (`SST-1`): 建立指标定义与首份 baseline。
  - v1.1 (`SST-2`): 按周追加 scripts 治理样本。
  - v2.0 (`SST-3`): 将关键帮助输出或脚本清单接入自动校验。
- Technical Risks:
  - 风险-1: 当前 baseline 样本数少，更适合作为方向性预警。
  - 风险-2: 若后续 scripts 数量快速增长，手工续写成本会上升。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-SCRIPTS-TREND-001 | `TASK-SCRIPTS-004` / `SST-1` | `test_tier_required` | 检查指标定义与阈值存在 | scripts 治理可读性 |
| PRD-SCRIPTS-TREND-002 | `TASK-SCRIPTS-004` / `SST-1` | `test_tier_required` | 检查 fallback 围栏覆盖率与样本明细 | Web-first / fallback 边界 |
| PRD-SCRIPTS-TREND-003 | `TASK-SCRIPTS-004` / `SST-1` | `test_tier_required` | 检查 baseline 样本对 `TASK-SCRIPTS-001/002/003` 的追溯 | CI / scripts 契约治理一致性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| `DEC-SST-001` | 先做治理趋势指标 | 直接统计所有脚本运行成功率 | 当前任务目标是治理稳定性，而非执行运行时遥测。 |
| `DEC-SST-002` | baseline 只用近期治理样本 | 回填所有历史脚本任务 | 先用近因样本建立可持续格式更稳。 |
| `DEC-SST-003` | 把 fallback 围栏单独成指标 | 混入主入口覆盖率 | Web-first / fallback 边界是 scripts 模块的核心治理风险。 |
