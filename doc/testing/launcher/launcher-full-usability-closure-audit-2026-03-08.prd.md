# oasis7: 启动器全功能可用性审查与闭环验收（2026-03-08）

- 对应设计文档: `doc/testing/launcher/launcher-full-usability-closure-audit-2026-03-08.design.md`
- 对应项目管理文档: `doc/testing/launcher/launcher-full-usability-closure-audit-2026-03-08.project.md`

审计轮次: 1


## 1. Executive Summary
- Problem Statement: 启动器相关能力已分批完成（脚本迁移、生命周期硬化、鉴权自动继承），但缺少“一次性横向全链路可用性复核 + 实际闭环复现”证据，难以快速判断当前可发布性。
- Proposed Solution: 基于现有 PRD 与测试手册执行启动器全功能审查，覆盖 CLI/脚本行为、生命周期与就绪、Web 闭环（agent-browser）以及迁移阻断路径，并沉淀审计结论与证据路径。
- Success Criteria:
  - SC-1: 启动器核心能力清单（脚本迁移/生命周期/鉴权注入/静态目录覆盖/阻断策略）均有逐项结论。
  - SC-2: 至少完成 1 次真实 Web 闭环（`oasis7_game_launcher + agent-browser`）并产出截图/日志/state 证据。
  - SC-3: 启动器相关 `test_tier_required` 回归命令执行并记录通过/失败。
  - SC-4: 若发现问题，提供可定位的故障签名、复现路径与影响范围分级。
  - SC-5: 审查结果可追溯到 PRD-ID -> Task -> Test。

## 2. User Experience & Functionality
- User Personas:
  - 发布负责人：需要快速判断“当前启动器是否可作为发布入口”。
  - 测试维护者：需要可复现的闭环证据与故障分级。
  - 脚本维护者：需要确认迁移后脚本行为与阻断策略仍然稳定。
- User Scenarios & Frequency:
  - 发布前审计：每个候选版本至少一次。
  - 重大启动链路改动后复核：每次改动后一次。
  - 故障复盘：出现“可启动但不可用”争议时按同口径复测。
- User Stories:
  - PRD-TESTING-LAUNCHER-REVIEW-001: As a 发布负责人, I want a single launcher usability verdict with evidence, so that release decisions are defensible.
  - PRD-TESTING-LAUNCHER-REVIEW-002: As a 测试维护者, I want real Web loop evidence from agent-browser, so that UI availability is not inferred from unit tests only.
  - PRD-TESTING-LAUNCHER-REVIEW-003: As a 脚本维护者, I want migration and block paths verified in one run, so that old entrypoints cannot regress silently.
- Critical User Flows:
  1. Flow-REVIEW-001: `启动 oasis7_game_launcher -> ready 探针通过 -> Web 主页可访问 -> Viewer 状态可读`
  2. Flow-REVIEW-002: `执行 agent-browser 语义动作 -> 采集 snapshot/console/screenshot/state -> 形成闭环证据`
  3. Flow-REVIEW-003: `执行 run-game-test/历史 viewer-release-qa-loop -> 验证迁移后脚本参数与产物路径`
  4. Flow-REVIEW-004: `执行 longrun 旧入口脚本 -> 命中阻断文案 -> 输出迁移方向`
  5. Flow-REVIEW-005: `汇总结果 -> 给出可用性 verdict（pass/conditional/fail）`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 启动器基础可用性 | `--scenario`、`--live-bind`、`--web-bind`、ready 日志 | 启动并检查端口与主页可达 | `booting -> ready/failed` | 先进程存活，再 ready，再页面 | 执行者可运行 |
| Web 闭环验收 | agent-browser `snapshot/eval/console/screenshot` | 以语义动作操作 Viewer 并采样 | `sampling -> evidenced` | 至少包含 screenshot + state + console | 执行者产出，发布者审阅 |
| 脚本迁移验证 | `run-game-test.sh`、历史 `viewer-release-qa-loop.sh` | 运行标准入口并确认行为一致 | `legacy-migrated -> verified` | 先 smoke 再 QA loop | 脚本维护者可复核 |
| 阻断路径验证 | `s10-five-node-game-soak.sh`、`p2p-longrun-soak.sh` | 触发旧路径失败并检查提示 | `legacy-call -> blocked-with-guidance` | fail-fast 优先于后续步骤 | 维护者定义文案 |
| 结论分级 | verdict、失败签名、影响范围 | 输出 pass/conditional/fail 与后续动作 | `raw-results -> triaged -> closed` | 先高风险失败，再中低风险 | 发布负责人确认 |
- Acceptance Criteria:
  - AC-1: 启动器功能清单与本次执行结果形成一一映射，不留“未判定”项。
  - AC-2: Web 闭环至少 1 次真实执行，证据包含 `snapshot + console + screenshot + state`。
  - AC-3: 启动器定向测试命令完成，结果按 `test_tier_required/test_tier_full` 分类记录。
  - AC-4: 迁移脚本与阻断脚本均有实际执行记录，结论可复现。
  - AC-5: 本次审查结论、风险项、后续动作写入项目管理与 devlog，满足可追溯性。
- Non-Goals:
  - 不在本任务引入新的启动器功能需求。
  - 不在本任务替代长期 soak/性能趋势分析。
  - 不在本任务中改写 testing 分层模型本身。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: `agent-browser` CLI 用于真实浏览器操作；仓库脚本 `run-game-test.sh`、`s10-five-node-game-soak.sh`、`p2p-longrun-soak.sh`。历史审查曾使用 `viewer-release-qa-loop.sh`，该脚本现已删除。
- Evaluation Strategy: 通过闭环动作成功率、脚本返回码、启动器就绪判定、故障签名可定位性评估审查质量。

## 4. Technical Specifications
- Architecture Overview: 本审查以 `oasis7_game_launcher` 为核心执行器，结合启动脚本与 agent-browser 形成“进程编排 -> Web 可达 -> 语义交互 -> 证据采集 -> 分级结论”的验证流水线。
- Integration Points:
  - `crates/oasis7/src/bin/oasis7_game_launcher.rs`
  - `crates/oasis7/src/bin/oasis7_game_launcher/oasis7_game_launcher_tests.rs`
  - `crates/oasis7_client_launcher/src/main.rs`
  - `scripts/run-game-test.sh`
  - 历史已删除：`scripts/viewer-release-qa-loop.sh`
  - `scripts/s10-five-node-game-soak.sh`
  - `scripts/p2p-longrun-soak.sh`
  - `testing-manual.md`
  - `doc/testing/manual/web-ui-agent-browser-closure-manual.prd.md`
- Edge Cases & Error Handling:
  - 启动后端口未监听：按 F1 处理并记录 launcher 日志。
  - 页面可开但 `__AW_TEST__` 不可用：标记 UI 语义层失败并保留 snapshot。
  - 仅截图通过但 state/console 异常：判定为 conditional/fail，不可直接放行。
  - 旧入口脚本未阻断：判定高风险回归并要求立即修复。
  - 静态目录配置不一致导致资源错配：记录 `--viewer-static-dir` 与 `OASIS7_GAME_STATIC_DIR` 实际来源并分级。
- Non-Functional Requirements:
  - NFR-REVIEW-1: 首个可用性初判应在执行开始后 10 分钟内产出。
  - NFR-REVIEW-2: 所有证据产物必须落在仓库既有目录（如 `output/playwright/`）并可追溯。
  - NFR-REVIEW-3: 审查报告对每项功能给出明确 verdict（pass/conditional/fail），覆盖率 100%。
  - NFR-REVIEW-4: 审查结论与手册口径冲突数为 0。
- Security & Privacy: 审查日志不得泄露敏感凭据；若触发鉴权相关日志，仅保留定位所需最小信息。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (LAUNCHREV-1): 建立审查专题 PRD 与项目管理文档。
  - v1.1 (LAUNCHREV-2): 完成启动器脚本/单测/阻断路径可用性审查。
  - v2.0 (LAUNCHREV-3): 完成真实 Web agent-browser 闭环与证据采样。
  - v2.1 (LAUNCHREV-4): 形成风险分级结论并完成文档与日志收口。
- Technical Risks:
  - 风险-1: 本地环境图形/端口状态导致闭环不稳定，可能引入假失败。
  - 风险-2: 仅执行轻量 smoke 可能遗漏长稳退化，需要明确边界。
  - 风险-3: 多入口脚本参数漂移可能导致“脚本可跑但结果口径不一致”。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-TESTING-LAUNCHER-REVIEW-001 | LAUNCHREV-1/2/4 | `test_tier_required` | 启动器功能清单审计 + 脚本可执行性检查 + 审计结论收口 | 启动入口可发布性 |
| PRD-TESTING-LAUNCHER-REVIEW-002 | LAUNCHREV-2/3 | `test_tier_required` | `oasis7_game_launcher` 定向测试 + agent-browser 真实闭环采样 | Web 可用性与 UI 真实性 |
| PRD-TESTING-LAUNCHER-REVIEW-003 | LAUNCHREV-2/4 | `test_tier_required` | 迁移脚本行为验证 + 阻断文案验证 + 风险分级 | 脚本兼容与误用防护 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-REVIEW-001 | 审查默认采用 `oasis7_game_launcher + agent-browser` 真闭环 | 仅依赖单元测试/日志静态审查 | 真闭环可验证真实用户路径，降低假通过。 |
| DEC-REVIEW-002 | 把迁移脚本与阻断脚本纳入同批审查 | 仅验证 happy path | 启动器可用性需同时覆盖误用防护能力。 |
| DEC-REVIEW-003 | 输出 pass/conditional/fail 分级结论 | 仅输出二元 pass/fail | 分级有助于发布方按风险做决策。 |

## 原文约束点映射（内容保真）
- 用户需求“全面审查启动器整套功能可用性并实际闭环使用” -> 第 1 章 Problem/Solution/SC。
- 启动器全能力面（迁移、生命周期、鉴权、阻断） -> 第 2 章场景/规格矩阵 + 第 4 章 Integration。
- 实际闭环要求（agent-browser 真浏览器） -> 第 2 章 Flow-REVIEW-002 + 第 3/4 章工具与架构。
- 审查输出必须可追溯 -> 第 6 章 Traceability 与 Decision Log。
