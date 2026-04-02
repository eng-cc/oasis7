# engineering PRD

审计轮次: 7

## 目标
- 建立 engineering 模块设计主文档，统一需求边界、技术方案与验收标准。
- 确保 engineering 模块后续改动可追溯到 PRD-ID、任务和测试。

## 范围
- 覆盖 engineering 模块当前能力设计、接口边界、测试口径与演进路线。
- 覆盖 PRD-ID 到 `doc/engineering/project.md` 的任务映射。
- 不覆盖实现代码逐行说明与历史过程记录。

## 接口 / 数据
- PRD 主入口: `doc/engineering/prd.md`
- 项目管理入口: `doc/engineering/project.md`
- 文件级索引: `doc/engineering/prd.index.md`
- 追踪主键: `PRD-ENGINEERING-xxx`
- 测试与发布参考: `testing-manual.md`

## 里程碑
- M1 (2026-03-03): 完成模块设计 PRD 主体重写与任务改造。
- M2: 补齐模块设计验收清单与关键指标。
- M3: 建立 PRD-ID -> Task -> Test 的长期追踪闭环。

## 风险
- 模块边界演进快，文档同步可能滞后。
- 指标口径不稳定会降低验收一致性。
## 1. Executive Summary
- Problem Statement: 工程规范分散在多个专题文档，导致文件体量控制、提交门禁、脚本治理与代码质量标准不够统一。
- Proposed Solution: 将 engineering 模块定义为工程治理主文档，统一维护规范、质量门禁、改造节奏与验收口径。
- Success Criteria:
  - SC-1: Rust 单文件超 1200 行新增违规数为 0。
  - SC-2: Markdown 单文件超 1000 行新增违规数为 0。
  - SC-3: `scripts/doc-governance-check.sh` 在 required gate 连续通过。
  - SC-4: 工程类任务 100% 映射到 PRD-ENGINEERING-ID。
  - SC-5: `doc/` 根目录与模块根目录平铺文档新增违规数为 0（allowlist 冻结机制）。
  - SC-6: 重点模块（world-simulator/p2p/world-runtime/testing/site/readme/scripts/game/headless-runtime）根目录平铺专题文档迁移完成并保持引用闭环。
  - SC-8: 完成四人并行迁移分工，待迁移清单有冻结快照且每日可追踪燃尽进度。
  - SC-9: 活跃文档 `doc/...*.md` 依赖路径断链数为 0。
  - SC-10: 全量 PRD 审读清单覆盖率 100%（当前 PRD 文档 708 份，含 `prd.md` 与 `project.md`）。
  - SC-11: 模块入口三件套（`prd.md`/`project.md`/`prd.index.md`）已读状态长期保持 100%。
  - SC-12: 文档-代码偏差在同批次回写闭环率 100%。
  - SC-13: 新增专题文档 100% 可按“目录表达对象、后缀表达职责”规则唯一落位，并能在 5 分钟内判断应创建 `*.prd.md`、`*.design.md`、`*.project.md` 中的哪一种。
  - SC-14: 角色职责入口统一收敛到 `.agents/roles/*.md`，根 `AGENTS.md` 仅保留 7 个组合角色入口与协作规则。
  - SC-15: 角色协作交接统一使用 `.agents/roles/templates/` 模板，确保 handoff 信息完整、可执行、可追溯。
  - SC-16: `AGENTS.md` 的开发工作流已升级为角色协作版，明确 owner role、handoff 触发条件、QA 与 LiveOps 回流路径。
  - SC-17: 任务执行日志 canonical 路径统一为 `.pm/tasks/TASK-PM-*.execution.md`，多角色执行时单条记录必须显式标注角色。
  - SC-18: task execution log / handoff 中的角色字段由 `.agents/roles/*.md` 白名单约束，新增别名违规数为 0。
  - SC-19: 当任务需要其他伙伴协作时，执行主体必须切换到标准角色视角并加载对应职责卡；`subagent` 只能作为 commit 前独立 review 机制，不得替代 owner / role 协作语义。
  - SC-20: `engineering` 模块治理专题标题对外统一使用 `oasis7` 品牌，不再在活跃/历史治理入口中混用 `oasis7` 标题。
  - SC-21: 仓库内文件化项目管理层 `.pm/` 建档完成后，7 个标准角色的长期 memory/backlog、signal inbox、task registry 与 stage/gate 汇总具备正式专题规格与任务追踪入口。
  - SC-22: `self-evolution` 后续补强在借鉴外部 memory/reflective-agent 方案时，必须显式冻结 adopted / rejected / deferred 边界，且不引入外部真值系统替代 `.pm/`。
  - SC-23: 每次 commit 前都必须启动一个独立 subagent review 当前 diff，并在提交前处理或显式记录 review findings；在 Codex 环境中，该动作默认指通过 `spawn_agent` 派生独立 review agent。
  - SC-24: commit 前 subagent review 在仓库流程层面属于默认执行步骤，不需要因为“只是执行这条既有流程”再单独向用户申请一次。
  - SC-24A: 根 `AGENTS.md`、engineering 主 PRD 与 `self-evolution` 正式追踪必须对该流程维持单一口径：commit 前先做独立 subagent review，再处理 findings 后提交。
  - SC-24B: `codex exec review --uncommitted` 仅可视为 shell 内自检，不得记作仓库要求的 subagent review；若运行环境禁止派生 agent，必须显式记录为运行环境阻断，而不是静默降级。
  - SC-25: `workflow-report --phase close --task-id <TASK-ID>` 的 working_memory 提示必须按当前 task 计数，而不是按角色全局计数；当当前 task 还没有 working_memory 时，应先提示 bootstrap/extract 入口，而不是直接提示 review/autoflow。

## 2. User Experience & Functionality
- User Personas:
  - 工程维护者：需要稳定规则来控制技术债。
  - 贡献开发者：需要清晰门槛和提交前检查路径。
  - 评审者：需要可量化判断变更是否合规。
- User Scenarios & Frequency:
  - 日常提交前检查：每次提交前执行，确认格式、结构与门禁符合要求。
  - CI 失败排查：每个异常流水线触发后执行，定位脚本与规则来源。
  - 规范迭代评审：每周至少 1 次，评估误报率和治理收益。
  - 季度治理复盘：每季度 1 次，回看违规趋势与修复效率。
- User Stories:
  - PRD-ENGINEERING-001: As an 工程维护者, I want enforceable file-size and structure limits, so that maintenance cost stays bounded.
  - PRD-ENGINEERING-002: As a 开发者, I want deterministic pre-commit checks, so that regressions are caught before CI.
  - PRD-ENGINEERING-003: As a 评审者, I want auditable governance evidence, so that review decisions are defensible.
  - PRD-ENGINEERING-004: As a 文档维护者, I want legacy docs migrated with per-doc manual review, so that content intent is preserved while converging to strict schema.
  - PRD-ENGINEERING-005: As a 协调人, I want one collaboration doc with principles and owner boundaries, so that parallel migration is deterministic.
  - PRD-ENGINEERING-006: As a 迁移执行人, I want non-overlapping migration scopes, so that I can avoid merge conflicts while moving fast.
  - PRD-ENGINEERING-007: As a 质量复核人, I want measurable acceptance gates for migrated docs, so that content fidelity is auditable.
  - PRD-ENGINEERING-008: As a 文档维护者, I want per-module file-level PRD indexes, so that active docs are reachable from the root doc tree.
  - PRD-ENGINEERING-009: As a 治理维护者, I want bidirectional PRD<->project references enforced by gate, so that traceability never drifts.
  - PRD-ENGINEERING-010: As a 评审者, I want explicit `test_tier_required/full` on module task items, so that task-to-test review is deterministic.
  - PRD-ENGINEERING-011: As a 文档维护者, I want doc path references validated in gate, so that migration-induced broken links are blocked before merge.
  - PRD-ENGINEERING-012: As a 文档治理维护者, I want a per-document read checklist for all PRDs, so that review coverage is auditable.
  - PRD-ENGINEERING-013: As a 模块负责人, I want code-first discrepancy handling, so that PRD behavior remains aligned with implementation.
  - PRD-ENGINEERING-014: As a 评审者, I want duplicate and upstream/downstream alignment checks, so that the PRD tree stays clear and non-conflicting.
  - PRD-ENGINEERING-015: As a 文档作者/评审者, I want one canonical document topology and role split, so that I can place new docs without guessing and keep detailed design discoverable.
  - PRD-ENGINEERING-021: As a 协作 owner, I want a file-based self-evolution management layer inside the repo, so that long-term role memory/backlog and stage inputs no longer depend on rereading scattered daily logs.
  - PRD-ENGINEERING-022: As a `producer_system_designer`, I want external memory and reflection patterns benchmarked against our file-native governance model, so that future self-evolution upgrades borrow structure without replacing repo-native truth.
- Critical User Flows:
  1. Flow-ENG-001: `提交前执行脚本 -> 通过 spawn_agent 启动独立 subagent review 当前 diff -> 修复 findings 并复测 -> 进入 CI`
  2. Flow-ENG-002: `CI 失败 -> 定位规则来源 -> 判断误报/真实问题 -> 更新脚本或文档`
  3. Flow-ENG-003: `季度复盘 -> 汇总违规趋势 -> 调整门禁阈值 -> 发布新治理基线`
  4. Flow-ENG-004: `逐篇阅读旧文档 -> 按 strict schema 重写 -> 内容保真复核 -> 更新任务与执行日志追踪`
  5. Flow-ENG-005: `冻结待迁移清单 -> 按 Owner-A/B/C/D 切分范围 -> 并行执行 -> 每日燃尽收口`
  6. Flow-ENG-006: `生成全量审读清单 -> 逐篇阅读并打勾 -> 核对代码/重复/上下游 -> 回写偏差并复跑门禁`
  7. Flow-ENG-007: `新专题提出 -> 选择模块/专题目录 -> 判断文档职责后创建同名 PRD/Design/Project -> 更新索引 -> 评审者按统一阅读顺序审查`
  8. Flow-ENG-008: `需要其他伙伴协作 -> 切换到对应标准角色视角 -> 加载角色职责卡确认输入/输出/Done -> 按该角色执行或交接 -> owner 回写 PRD/project/task execution log`
  9. Flow-ENG-009: `执行过程产生 QA/liveops/producer 高价值信号 -> 写入 signal inbox -> 提升为 role memory 或 candidate task -> 进入 stage/gate 汇总 -> owner 决定是否回写正式 PRD/project`
  10. Flow-ENG-010: `评估外部 memory/reflective-agent 方案 -> 冻结 adopted/rejected/deferred 边界 -> 将可借鉴对象映射到 .pm/doc 现有结构 -> 再拆实现任务`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 文档治理检查 | allowlist、模块根目录规则、根目录规则 | 执行 `doc-governance-check.sh` | `pass/fail` | 按违规严重度输出 | 所有人可执行，治理维护者可更新基线 |
| 提交前检查 | 格式、静态检查、测试分层触发 | pre-commit 自动执行 | `pending -> running -> blocked/passed` | 先 required 后 full | 贡献者可触发，CI 负责人可调整策略 |
| 工程趋势统计 | 违规率、修复时长、回归率 | 周期性生成报表并复盘 | `collecting -> reported -> actioned` | 按模块与时间排序 | 评审者与维护者可读写 |
| PRD 格式迁移 | 文档路径、迁移批次ID、原文关键约束点 | 人工阅读原文后按 strict schema 重写并复核 | `inventory -> migrated -> verified` | 默认按活跃文档优先、按模块分批 | 治理维护者可冻结批次，贡献者可提交迁移 |
| 并行迁移协作 | Owner、范围、快照日期、燃尽统计 | 依据协作方案分批推进迁移 | `planned -> in_progress -> done` | 目录前缀互斥，按负载均衡调整 | 协调人分配，Owner 执行，复核人抽检 |
| PRD 文件级索引 | 模块名、专题PRD路径、专题project路径 | 生成/更新模块索引并回写入口引用 | `missing -> indexed -> verified` | 活跃文档优先，按路径稳定排序 | 维护者可更新，所有贡献者可读 |
| 依赖路径可达门禁 | 引用文档路径、引用来源、豁免列表 | 校验 `doc/...*.md` 引用目标是否存在 | `pass/fail` | 默认全量校验，通配符/模板与白名单文件豁免 | 维护者维护豁免，提交者必须修复断链 |
| 文档分工与组织规范 | 对象层级（模块/专题/分册）、职责后缀（PRD/Design/Project/Runbook/Manual） | 为新主题选择落点并按规则建档 | `unclassified -> classified -> indexed -> reviewed` | 目录按领域/专题，文件按职责，优先同名三件套 | 作者可建档，评审者可裁定例外 |
| 任务测试分层标注 | 任务ID、PRD-ID、test tier | 在模块 `project.md` 显式写 tier | `unspecified -> specified -> audited` | 先模块主项目，再专题项目 | 模块维护者审核，贡献者执行 |
| 全量 PRD 审读清单 | 文档路径、阅读时刻、代码一致性、重复性、上下游状态、处理动作 | 逐篇阅读后更新清单并回写偏差 | `unread -> read -> aligned` | 入口优先、风险优先 | 维护者与评审者可写，贡献者可读 |
| 角色职责卡 | 角色名、使命、owner 范围、输入、输出、决策边界、完成定义、推荐技能、检查清单 | 更新 `.agents/roles/*.md` 并在根 `AGENTS.md` 维护入口映射 | `draft -> aligned -> adopted` | 默认按 7 个组合角色稳定排序；技能仅作推荐方法，不改变 owner role | 全体贡献者可读，角色 owner 与治理维护者可改 |
| 角色交接模板 | 交接标题、来源角色、目标角色、目标、上下文、输入、输出、截止、风险、阻断、验证、回写位置 | 从 `.agents/roles/templates/*.md` 复制填写并随任务流转 | `draft -> sent -> acknowledged -> delivered` | 默认先 brief 后 detailed，按风险等级决定是否升级 | 发起方负责填写，接收方负责确认，维护者可演进模板 |
| 角色协作工作流 | owner role、角色视角切换、职责卡加载、handoff 触发条件、执行顺序、QA/LiveOps 回流、pre-commit review 默认流程 | 当需要其他伙伴协作时，先切换到对应标准角色视角并加载职责卡，再按工作流执行；commit 前额外通过 `spawn_agent` 启动独立 subagent review 当前 diff | `defined -> adopted -> audited` | 默认按需求进入顺序执行，跨角色任务先定 owner 再流转；subagent 只允许作为 pre-commit review 辅助，不得替代角色协作规则 | 全体贡献者遵守，治理维护者可演进 |
| task execution log | `task_id`、日期、时刻、角色、完成内容、遗留事项 | 每个任务写入 `.pm/tasks/TASK-PM-*.execution.md`，并在条目级显式标角色 | `logged -> traceable -> audited` | 默认一任务一文件，按时间排序 | 全体贡献者可写，评审者可按任务/角色回溯 |
| 角色名白名单校验 | 角色名、来源文件、白名单来源 | 校验 task execution log / handoff 中角色名是否存在于 `.agents/roles/*.md` | `pass/fail` | 以角色文件名去后缀为唯一 canonical name | 治理维护者维护角色清单，提交者必须修复别名 |
| 文件化项目管理层 | 角色 registry、role memory/backlog、signal inbox、task registry、stage/gate 文件、task workflow evidence | 在仓库内维护 `.pm/` 运行态对象，并通过脚本做 scaffold/lint/report/promote/set-stage | `planned -> scaffolded -> adopted -> audited` | 默认按 `role_name`、`priority`、`updated_at` 排序；高严重度 signal 优先提升 | 治理维护者维护结构，owner role 维护自身 memory/backlog，producer 维护正式阶段结论 |
- Acceptance Criteria:
  - AC-1: engineering PRD 明确文件约束、脚本约束、测试分层约束。
  - AC-2: engineering project 文档维护任务拆解与状态。
  - AC-3: 与 `doc/scripts/precommit/pre-commit.prd.md`、`testing-manual.md` 的口径一致。
  - AC-4: 每次工程规范变更有对应 task execution log 记录。
  - AC-5: 新增工程治理专题若引入运行态治理层，必须明确与正式文档层的分工边界，并进入主项目追踪。
  - AC-6: 文档治理脚本校验 `doc/.governance/*-allowlist.txt`，可拦截 `doc/*.md` 与 `doc/<module>/*.md` 的非预期新增。
  - AC-7: `doc/core`、`doc/engineering`、`doc/game`、`doc/headless-runtime`、`doc/p2p`、`doc/playability_test_result`、`doc/readme`、`doc/scripts`、`doc/site`、`doc/testing`、`doc/world-runtime`、`doc/world-simulator` 模块根目录仅保留 `README.md` / `prd.md` / `project.md` / `prd.index.md` 与模块当前允许的活跃卡片文件。
  - AC-8: 每次迁移任务需附“原文关键约束点 -> 新文档章节”对照，确保内容不丢失。
  - AC-9: 并行迁移必须有公开分工表、待迁移快照和每日燃尽更新机制。
  - AC-10: 每个模块提供文件级 PRD 索引并在主入口可达，覆盖活跃专题 `*.prd.md/*.project.md`。
  - AC-11: 文档治理门禁必须校验专题 PRD/project 双向互链；缺失即失败。
  - AC-12: 模块 `project.md` 每个任务项必须显式标注 `test_tier_required` 或 `test_tier_full`（可为组合层级）。
  - AC-13: 文档治理门禁必须校验活跃文档 `doc/...*.md` 引用路径可达；断链必须阻断并修复。
  - AC-14: 需存在全量 PRD 审读清单（按模块拆分，单一清单口径），且每条已读记录包含阅读时刻和三类核对结论（代码/重复/上下游）。
  - AC-15: `.agents/roles/` 下需存在 7 个组合角色职责卡，覆盖制作/规则、runtime、WASM、Agent、Viewer、QA、LiveOps/社区。
  - AC-15A: 每张角色职责卡需显式给出推荐技能与典型使用场景，并声明“角色决定 owner、技能决定方法”，避免把技能误当职责边界。
  - AC-16: 根 `AGENTS.md` 的“分工”章节不再内嵌 12 个长描述，而是引用 7 个组合角色职责卡与使用约定。
  - AC-17: `.agents/roles/templates/` 下至少提供一套可直接复制使用的角色交接模板，并在根 `AGENTS.md` 可达。
  - AC-18: 根 `AGENTS.md` 的“开发工作流”章节应明确 owner role、handoff 使用时机、QA/LiveOps 责任和“用户要求不提交”时的例外处理。
  - AC-18A: 根 `AGENTS.md` 的“项目运行模式”需明确：需要其他伙伴协作时，执行主体必须切换到 `.agents/roles/*.md` 中的标准角色视角并加载对应职责描述，而非依赖未定义的 `sub agent` 能力。
  - AC-18B: 根 `AGENTS.md` 的 commit 前 subagent review 规则需只保留单一默认流程口径。
  - AC-19: 根 `AGENTS.md` 的 task execution log 规则需明确“一任务一文件、不按角色拆文件、条目级标角色”的约束。
  - AC-20: 文档治理门禁需阻断 task execution log / handoff 中未在 `.agents/roles/*.md` 注册的角色名。
  - AC-21: `doc/engineering/**` 仍可读治理专题标题统一使用 `oasis7` 品牌；旧 `oasis7` 仅允许出现在历史正文引用或实现兼容说明中。
  - AC-22: 外部 memory/reflective-agent 借鉴必须先在 `engineering/self-evolution` 专题中冻结 adopted/rejected/deferred 边界，再进入实现任务拆解。
- Non-Goals:
  - 不定义 gameplay/p2p/runtime 业务规则。
  - 不替代模块内部测试策略。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 文档治理脚本、CI 测试脚本、静态检查脚本。
- Evaluation Strategy: 通过 required/full gate 成功率、违规项统计、回归修复时长衡量工程治理有效性。

## 4. Technical Specifications
- Architecture Overview: engineering 模块聚焦工程流程与规范，不承载业务逻辑；通过脚本与门禁把规范落地到提交链路。
- Integration Points:
  - `scripts/doc-governance-check.sh`
  - `doc/scripts/precommit/pre-commit.prd.md`
  - `doc/scripts/precommit/precommit-remediation-playbook.prd.md`
  - `doc/.governance/doc-root-md-allowlist.txt`
  - `doc/.governance/module-root-md-allowlist.txt`
  - `doc/engineering/doc-migration/legacy-doc-migration-collaboration-2026-03-03.prd.md`
  - `doc/engineering/doc-migration/legacy-doc-migration-collaboration-2026-03-03.project.md`
  - `doc/engineering/doc-migration/legacy-doc-migration-backlog-2026-03-03.md`
  - `doc/engineering/prd-review/prd-full-system-audit-2026-03-03.prd.md`
  - `doc/engineering/prd-review/prd-full-system-audit-2026-03-03.project.md`
  - `doc/engineering/prd-review/checklists/active-*.md`
  - `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.prd.md`
  - `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.design.md`
  - `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.project.md`
  - `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.prd.md`
  - `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.design.md`
  - `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.project.md`
  - `scripts/doc-governance-check.sh`
  - `doc/*/README.md`
  - `testing-manual.md`
  - `.github/workflows/*`
- Edge Cases & Error Handling:
  - allowlist 漂移：检测到未登记新增时直接失败并提示最小修复路径。
  - 误报场景：规则误伤时保留失败证据并通过任务流程修订规则，不直接绕过。
  - 本地/CI 不一致：本地通过但 CI 失败时以 CI 结果为准并补环境对齐说明。
  - 脚本不可执行：缺依赖时给出明确安装建议与最小复现命令。
  - 并发修改冲突：同一规则多分支更新时以最新主干基线重放验证。
  - 新旧格式并存：迁移中允许 legacy 与 strict 共存，但每个迁移批次必须标注边界并回写追踪状态。
  - 批量迁移回归风险：结构改写可能造成引用断链，需附带路径扫描与脚本复核。
  - 根入口重定向迁移：`doc/game-test.project.md`、`doc/world-runtime.project.md`、`doc/world-simulator.project.md` 在 D2 阶段已完成收口；后续变更仅允许在 redirect 语义内维护，不恢复为业务正文入口。
  - 索引覆盖不足：专题文档未被入口索引时，必须在当批修复并补回链路。
  - 互链缺失：若 PRD 与 project 仅单向引用，会导致追溯断链，门禁需直接阻断。
  - 历史迁移快照：包含旧路径清单的迁移快照文档需通过白名单豁免，避免误判为断链。
  - 审读进度漂移：若已读清单不随批次更新，会导致“已完成”状态失真，必须在同提交更新清单。
  - 运行态真值冲突：若 `.pm/` 与正式 `doc/` 对同一阶段/任务给出不同口径，必须以正式文档为准并把 `.pm/` 记录标成待裁决。
- Non-Functional Requirements:
  - NFR-ENG-1: required 门禁平均执行时长 <= 10 分钟。
  - NFR-ENG-2: 文档治理误报率 <= 5%（按周统计）。
  - NFR-ENG-3: 新增工程任务 PRD-ID 映射覆盖率 100%。
  - NFR-ENG-4: 工程治理脚本在 Linux/macOS 环境均可执行。
  - NFR-ENG-5: 规则变更需附带可追溯说明与回归证据。
  - NFR-ENG-6: 活跃文档迁移任务必须包含“原文约束点清单 + 新文档章节映射 + 回归验证结果”三件套证据。
  - NFR-ENG-7: 并行迁移阶段每工作日至少完成 16 篇迁移（4 人 * 人均 4 篇）。
  - NFR-ENG-8: 全部模块文件级索引应在 1 次 `doc-governance-check` 执行内完成可达性校验。
  - NFR-ENG-9: 活跃专题 PRD/project 双向互链覆盖率 100%。
  - NFR-ENG-10: 模块主项目任务测试分层显式标注覆盖率 100%。
  - NFR-ENG-11: 活跃文档 `doc/...*.md` 引用路径可达性覆盖率 100%。
  - NFR-ENG-12: 全量审读清单中“已读且已核对”条目覆盖率按周单调提升，不得回退。
  - NFR-ENG-13: 根 `AGENTS.md` 与 `.agents/roles/*.md` 的角色映射一致率 100%，不得出现无入口角色或悬空引用。
  - NFR-ENG-14: 角色交接模板字段命名稳定，默认模板在 5 分钟内可完成填写并可被他人直接执行。
  - NFR-ENG-15: 开发工作流规则在单人执行与多角色协作两种场景下都应自洽，不得出现相互冲突的提交/回写要求。
  - NFR-ENG-16: 单日日志应同时支持时间线回放与角色维度检索，不得因角色拆分导致当日过程碎片化。
  - NFR-ENG-17: 角色名校验应零配置跟随 `.agents/roles/` 目录变化，不依赖重复维护的手写名单。
  - NFR-ENG-18: 协作执行语义应与当前 Codex/CLI 运行模式兼容，允许单一执行主体通过角色视角切换完成多角色闭环；commit 前 review 的仓库默认流程、角色协作语义与 no-commit 收口流程之间不得互相冲突。
  - NFR-ENG-19: 文件化项目管理层若落地，7 个标准角色的 role registry / task registry / stage/gate 汇总必须在 1 次 lint/report 执行内完成结构校验与引用可达性检查。

- Security & Privacy: 仅涉及工程流程元信息；涉及凭据的自动化流程必须遵守最小暴露原则并避免日志泄漏。
## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (2026-03-03): 固化工程规范与门禁指标。
  - Phase-1 进展（2026-03-03）: Owner-B 已完成 `doc/p2p/**` 115 篇待迁移文档的逐篇重写迁移。
  - v1.1: 补齐高频违规的自动修复建议与脚本化诊断。
  - v2.0: 建立工程规范趋势看板（违规率、修复时长、回归率）。
- Technical Risks:
  - 风险-1: 规范过严导致迭代效率下降。
  - 风险-2: 新脚本引入误报造成 CI 噪声。
  - 风险-3: 老文档迁移批次过大导致评审负担与引用回归风险提升。
  - 风险-4: 多人并行对同一目录写入造成冲突与重复迁移。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-ENGINEERING-001 | TASK-ENGINEERING-001/005/006/007 | `test_tier_required` | 文档结构检查、平铺治理脚本执行 | 文档组织一致性、工程可维护性 |
| PRD-ENGINEERING-002 | TASK-ENGINEERING-002/003/007 | `test_tier_required` + `test_tier_full` | pre-commit/CI 门禁联动校验 | 提交流程稳定性、回归拦截能力 |
| PRD-ENGINEERING-003 | TASK-ENGINEERING-003/004/007 | `test_tier_required` | 趋势统计与审查模板抽样检查 | 工程治理可审计性与长期演进 |
| PRD-ENGINEERING-004 | TASK-ENGINEERING-008/009 | `test_tier_required` | 原文约束点对照、迁移后治理脚本与引用扫描 | 文档格式一致性与内容保真 |
| PRD-ENGINEERING-005 | TASK-ENGINEERING-010 | `test_tier_required` | 协作主文档结构与分工边界校验 | 并行迁移入口一致性 |
| PRD-ENGINEERING-006 | TASK-ENGINEERING-011/012/013/013A/013B/013C/013D/014 | `test_tier_required` | 按 Owner 责任域抽样检查迁移提交 | 并行效率与冲突控制 |
| PRD-ENGINEERING-007 | TASK-ENGINEERING-015 | `test_tier_required` + `test_tier_full` | 全量迁移收尾扫描、命名与引用一致性验证 | 全仓文档治理收口质量 |
| PRD-ENGINEERING-008 | TASK-ENGINEERING-016 | `test_tier_required` | 12 模块文件级索引覆盖扫描、入口可达性检查 | 文档树可达性与导航一致性 |
| PRD-ENGINEERING-009 | TASK-ENGINEERING-017 | `test_tier_required` | `doc-governance-check` 双向互链门禁验证 | PRD/project 追溯完整性 |
| PRD-ENGINEERING-010 | TASK-ENGINEERING-018 | `test_tier_required` | 模块主项目任务项 tier 显式标注检查 | 任务到测试分层可审计性 |
| PRD-ENGINEERING-011 | TASK-ENGINEERING-019 | `test_tier_required` | 活跃文档引用路径可达性门禁与断链修复验证 | 文档树引用完整性与迁移稳定性 |
| PRD-ENGINEERING-012 | TASK-ENGINEERING-020/024 | `test_tier_required` | 全量审读清单覆盖率与入口文档已读率检查 | PRD 审读可追溯性 |
| PRD-ENGINEERING-013 | TASK-ENGINEERING-021/022 | `test_tier_required` | 代码一致性抽样与偏差回写核验 | 文档行为与实现一致性 |
| PRD-ENGINEERING-014 | TASK-ENGINEERING-022/023/024 | `test_tier_required` + `test_tier_full` | 重复治理记录与上下游链路可达性检查 | PRD 体系清晰度与跨模块对齐 |
| PRD-ENGINEERING-015 | TASK-ENGINEERING-025 | `test_tier_required` | 规范正文结构检查、模块入口回写、索引可达性检查 | 新增文档可发现性与详细设计落位一致性 |
| PRD-ENGINEERING-016 | TASK-ENGINEERING-030/036 | `test_tier_required` | 角色职责卡存在性、字段完整性、推荐技能区段与根 `AGENTS.md` 入口映射检查 | 人机协作分工清晰度与执行一致性 |
| PRD-ENGINEERING-017 | TASK-ENGINEERING-031 | `test_tier_required` | 交接模板存在性、字段完整性与入口可达性检查 | 跨角色协作质量与上下文传递稳定性 |
| PRD-ENGINEERING-018 | TASK-ENGINEERING-032/049 | `test_tier_required` | `AGENTS.md` 工作流章节与项目运行模式口径一致性检查；协作语义需显式落到角色视角切换与职责卡加载，且只允许把 subagent 用作 commit 前 review | 协作流程稳定性与执行确定性 |
| PRD-ENGINEERING-019 | TASK-ENGINEERING-033/096 | `test_tier_required` | task execution log 规则、任务级留痕格式与角色标记要求一致性检查 | 任务过程可追溯性与角色责任可读性 |
| PRD-ENGINEERING-020 | TASK-ENGINEERING-034/096 | `test_tier_required` | 白名单角色名门禁、模板字段枚举与 task execution log 角色标签检查 | 角色命名一致性与防漂移能力 |
| PRD-ENGINEERING-021 | TASK-ENGINEERING-074/075/076/077/078/079/080/081/082/083/084/085/092/093/094/095/096/097/098 | `test_tier_required` + `test_tier_full` | `self-evolution` 专题三件套、`.pm/` 结构 lint、task execution log schema、`set-stage`/stage drift 校验、`workflow-report --task-id` 留痕、signal promotion、workflow/role/stage report、subagent review 默认流程文案一致性、`spawn_agent`/shell-review 边界、task-scoped working_memory checklist 回归、角色扩容回归验证 | 仓库内项目管理运行层、阶段评审输入、QA/liveops 回流链、默认开发工作流 |
| PRD-ENGINEERING-022 | TASK-ENGINEERING-086/091 | `test_tier_required` | 外部方案借鉴边界专题三件套、working_memory 口径补充、phase 1 `.codex` transcript source 冻结（`session_index/history` 优先，`sessions rollout` fallback）、engineering 根入口回写、决策记录与引用闭环验证 | `self-evolution` 后续 memory/recall/working_memory/reflection 补强 |

- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-ENG-001 | 以脚本门禁落实规范 | 仅依赖人工评审 | 自动化一致性更高且可复现。 |
| DEC-ENG-002 | 保留 allowlist 冻结机制 | 完全开放文档新增 | 可控制结构漂移和历史债扩散。 |
| DEC-ENG-003 | required/full 分层验证 | 单层测试策略 | 兼顾效率与风险覆盖。 |
| DEC-ENG-004 | 老格式文档按批次渐进迁移并采用逐篇人工重写 | 一次性全量改写或自动脚本改写 | 人工重写更利于保留语义细节并控制内容质量。 |
| DEC-ENG-005 | 采用四人并行、目录前缀互斥分工推进大规模迁移 | 单人串行推进或随机切片 | 可兼顾迁移速度、冲突控制与审阅可追溯性。 |
| DEC-ENG-006 | Owner-D 先完成非根入口 60 篇，再单独收口 3 份根入口 redirect project 文档 | 在同一批次混合推进所有 63 篇 | 可减少根入口语义争议导致的回退频次，同时保持可追溯燃尽。 |
| DEC-ENG-007 | D2 完成后保留根入口 `.prd` redirect 并统一引用到新命名 | 恢复旧命名入口或删除 root redirect | 兼顾迁移收口一致性与历史入口兼容性。 |
| DEC-ENG-008 | 为全部模块增加文件级索引并纳入入口链路 | 仅保留目录级导航 | 文件级索引可显著降低“文档存在但不可达”问题。 |
| DEC-ENG-009 | 双向互链作为门禁硬规则 | 仅人工评审追溯关系 | 自动阻断可避免追溯链路长期漂移。 |
| DEC-ENG-010 | 模块任务项显式标注 `test_tier_required/full` | 仅在 PRD 总表声明 tier | 任务级标注更直接支撑评审与执行。 |
| DEC-ENG-011 | 将活跃文档引用路径可达性纳入门禁并维护最小豁免白名单 | 仅靠人工抽查断链 | 迁移后断链可自动阻断，减少隐性导航故障。 |
| DEC-ENG-012 | 采用全量逐篇审读清单（按模块拆分，单一清单口径） | 仅维护模块级进度百分比 | 逐篇清单可审计且可直接定位遗漏文档。 |
| DEC-ENG-013 | 审读偏差按代码实现回写文档 | 以历史文档条款反推代码变更 | 当前阶段先恢复“文档描述事实”可降低评审噪声。 |
| DEC-ENG-014 | 重复与上下游对齐问题在同批次完成修复与回填 | 跨批次累积处理 | 同批次闭环可避免问题扩散到下一轮审读。 |
| DEC-ENG-015 | 根 `AGENTS.md` 仅保留 7 个组合角色入口，详细职责下沉到 `.agents/roles/*.md` | 在根 `AGENTS.md` 内持续堆叠所有角色长描述 | 入口更短、更稳，且更便于按角色独立演进职责卡。 |
| DEC-ENG-016 | 为角色协作提供统一 handoff 模板，并放在 `.agents/roles/templates/` | 继续依赖自由格式口头/临时文本交接 | 统一模板能显著降低跨角色遗漏、返工和上下文漂移。 |
| DEC-ENG-017 | 将 `AGENTS.md` 工作流升级为角色协作版，并显式写入 handoff / QA / LiveOps / pre-commit subagent review / no-commit 例外 | 继续保留单线程开发表述，或让 subagent 替代 owner role | 当前仓库已引入角色职责卡与交接模板；需要继续由角色视角承接 owner 责任，同时用独立 subagent 提前暴露 commit 前风险。 |
| DEC-ENG-018 | 自我进化项目管理首期采用仓库内文件化运行层 | 直接将外部 PM/SaaS 作为主真值 | 当前仓库已具备 Git/worktree/正式文档治理链，先在 repo 内闭环更符合现有工程约束。 |
| DEC-ENG-019 | 执行日志收敛为 task-local `.pm/tasks/TASK-PM-*.execution.md`，并在条目级强制标注角色 | 继续维护集中式日表或按角色拆分日志文件 | `.pm` 已经以 task 为基本执行单元，任务日志跟 task file 同址更利于追溯、lint 与多 worktree 并发隔离。 |
| DEC-ENG-020 | 角色名通过 `.agents/roles/*.md` 自动生成白名单并由门禁校验 | 允许自由填写角色名或维护独立手写名单 | 自动从单一事实源派生，最不容易漂移。 |
| DEC-ENG-021 | 在每张角色职责卡内补充“推荐技能”区段，并明确“角色定 owner，技能定方法” | 仅在对话中临时口头说明角色与技能关系 | 关系落盘后更利于新人自助选择方法，也能降低角色/技能混用带来的协作歧义。 |
| DEC-ENG-021 | 将“需要其他伙伴协作”的默认执行语义收敛为“切换到标准角色视角并加载职责卡” | 保留“可开启 sub agent”表述 | 角色视角切换已被现有职责卡、handoff 模板与工作流规则完整支持，且不依赖额外运行时能力，执行口径更稳定。 |
