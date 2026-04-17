# 文档存量维护成本治理（2026-04-17）

- 对应设计文档: `doc/engineering/doc-governance/doc-corpus-maintenance-governance-2026-04-17.design.md`
- 对应项目管理文档: `doc/engineering/doc-governance/doc-corpus-maintenance-governance-2026-04-17.project.md`

审计轮次: 1

## 1. Executive Summary
- Problem Statement: `PRD-ENGINEERING-024` 已经把根入口和模块 README 的默认阅读面从“混乱长表”收紧到 `what / where / next / risk`，但仓库文档债已经进入第二阶段。以 2026-04-17 本任务启动前快照计，`doc/` 下有 1730 份 Markdown，`world-simulator` 549、`p2p` 269、`testing` 178，`doc/devlog/` 仍保留 57 份日文件，最大单文件 3288 行。入口虽然可读，但查找、同步、审计、项目页回写和历史追溯的维护成本仍在持续抬高。
- Proposed Solution: 建立“文档存量维护成本治理”专题，冻结从“阅读面噪音”转向“文档存量与维护成本”的阶段判断；新增可复算的库存报告脚本，统一统计总量、模块密度、热点子目录、`doc/devlog` backlog 与非归档近限长文件，并把后续动作明确收敛为 `历史压缩 / 路径级治理 / 近限文件拆分 / 季度复核输入` 四类。
- Success Criteria:
  - SC-1: engineering 存在正式专题，明确“阅读面治理已完成，但维护成本债仍未收口”的阶段判断。
  - SC-2: 仓库存在一条可复算的 `doc` 库存报告入口，至少输出总量、模块密度、热点子目录、`doc/devlog` backlog 与非归档近限长文件。
  - SC-3: 专题明确四类 follow-up 动作: `历史压缩`、`路径级治理`、`近限文件拆分`、`季度复核/门禁扩展评估`。
  - SC-4: engineering 根入口、主 PRD、主项目、文件级索引与 `doc-surface-area-governance` 旧专题完成 handoff 回写。
  - SC-5: 后续新任务可以在 15 分钟内根据库存报告判断“先压历史、先拆热点路径、还是先拆近限文件”。

## 2. User Experience & Functionality
- User Personas:
  - 项目经理 / `producer_system_designer`: 需要知道当前文档债处于哪个阶段，避免继续把“入口干净”误判成“治理完成”。
  - 模块 owner: 需要可复算的体量快照来判断该做路径治理、拆文件还是只做入口说明。
  - 文档治理评审者: 需要统一阈值来判断何时必须建项，而不是继续容忍体量自然膨胀。
- User Scenarios & Frequency:
  - 阶段复盘: 每次有人质疑“文档是不是又太多了”时触发。
  - 模块路径治理立项: 每次 `world-simulator / p2p / testing` 等高密度模块继续增量时触发。
  - 历史归档治理: 每次 `doc/devlog/`、历史专题或 evidence 累积到影响维护效率时触发。
- User Stories:
  - PRD-ENGINEERING-025: As a 项目经理/模块 owner, I want doc corpus maintenance cost formalized, so that I can see when doc governance has shifted from reading-surface clutter to path-level maintenance debt and open the right follow-up task.
- Critical User Flows:
  1. Flow-DCM-001:
     `运行 doc inventory report -> 看总量/模块密度/热点子目录/近限文件 -> 判断当前债属于历史压缩、路径级治理还是拆文件`
  2. Flow-DCM-002:
     `发现 doc/devlog backlog 继续膨胀 -> 建 devlog 压缩或索引重构任务 -> 保留追溯能力但退出日文件堆叠式维护`
  3. Flow-DCM-003:
     `发现模块热点目录持续超阈值 -> 建路径级治理任务 -> 先按对象边界拆层，再决定是否合并/重定向历史专题`
  4. Flow-DCM-004:
     `发现非归档文档接近 1000 行门禁 -> 在触发失败前先拆分 project/index/manual 或把历史内容退回归档层`
- Functional Specification Matrix:

| 对象/能力 | 字段定义 | 动作/行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| `doc` 库存报告 | 总量、模块计数、热点子目录、`doc/devlog` backlog、近限文件 | 按当前仓库快照生成可复算 Markdown 报告 | `generated -> reviewed -> referenced` | 先看总量，再看高密度模块与近限文件 | 所有人可读，维护者可更新脚本 |
| 历史压缩候选 | `doc/devlog` 文件数、最大文件行数、重复回写热点 | 建项压缩日文件或补中间索引 | `normal -> crowded -> action_required` | `doc/devlog` 文件数优先，其次看最大单文件 | `producer_system_designer` 裁定，owner 执行 |
| 路径级治理候选 | 模块总量、子目录热点、对象边界是否混叠 | 建对象级二次分层或迁移专题 | `normal -> crowded -> action_required` | 模块总量优先，热点子目录次之 | 模块 owner 执行，工程治理裁定 |
| 近限文件候选 | 非归档文档行数、职责混叠、是否逼近门禁 | 在超 1000 行前先拆文档 | `normal -> near_limit -> split_required` | 按行数降序和职责混叠度排序 | 文档 owner 执行 |
| 季度治理输入 | 最新库存报告、已建项 follow-up、未处理热点 | 决定是否追加脚本门禁或新的治理 round | `observed -> triaged -> governed` | 先处理仍在增长的热点 | `producer_system_designer` / `qa_engineer` 复核 |
- Acceptance Criteria:
  - AC-1: 专题正文明确区分“阅读面治理”与“存量维护成本治理”的问题边界。
  - AC-2: 专题给出 2026-04-17 任务启动前快照的正式基线: `doc/` 1730 份 Markdown、`world-simulator` 549、`p2p` 269、`testing` 178、`doc/devlog` 57、最大单文件 3288 行。
  - AC-3: 专题明确四类后续动作及其触发条件，而不是只保留“以后再看”的泛化描述。
  - AC-4: 仓库新增 `scripts/doc-inventory-report.sh` 并可在当前仓库输出上述体量快照。
  - AC-5: engineering 根入口、主 PRD、主项目、文件级索引与 `doc-surface-area-governance` 已同步回写新专题入口。
  - AC-6: 本专题不要求本批直接删除历史文档，也不允许把 evidence/历史追溯误删为“减重”。
  - AC-7: 本专题明确 `doc/devlog` 仍属历史归档层，但其 backlog 已升级为维护成本对象，需要单独治理而不是继续忽略。
- Non-Goals:
  - 不在本批直接重写 `world-simulator / p2p / testing` 的全部专题树。
  - 不在本批直接删除 `doc/devlog/*.md` 或历史 evidence。
  - 不把 `doc inventory report` 直接升级成 merge gate；是否进门禁要留给后续 round 判断。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 主要依赖 `find`、`wc`、`python3` 与 `scripts/doc-inventory-report.sh` 生成库存快照。
- Evaluation Strategy:
  - 用库存报告复算当前仓库基线，确认专题中的体量数据可重现。
  - 用人工抽样验证报告能直接指出后续建项优先级。

## 4. Technical Specifications
- Architecture Overview:
  - `PRD-ENGINEERING-024` 负责“默认阅读面减重”。
  - `PRD-ENGINEERING-025` 负责“文档存量维护成本治理”。
  - 新脚本只负责生成库存快照，不直接修改文档树。
- Integration Points:
  - `scripts/doc-inventory-report.sh`
  - `doc/engineering/prd.md`
  - `doc/engineering/project.md`
  - `doc/engineering/README.md`
  - `doc/engineering/prd.index.md`
  - `doc/engineering/doc-governance/doc-surface-area-governance-2026-04-10.project.md`
  - `scripts/doc-governance-check.sh`
- Edge Cases & Error Handling:
  - 模块入口已减重但路径树仍爆炸: 视为 `PRD-ENGINEERING-025` 的治理对象，不回退到只改 README 的处理方式。
  - `doc/devlog` 不再是运行态真值，但 backlog 继续增长: 不能因为“只是归档”就无限搁置，必须进入历史压缩治理。
  - 非归档文档还没触发 1000 行门禁，但已接近阈值: 视为 `near_limit`，应优先拆分而不是等门禁失败。
  - 报告输出与人工数值不一致: 以脚本结果为准，并在同批修正统计口径。
- Non-Functional Requirements:
  - NFR-1: 库存报告必须在当前仓库 10 秒内完成。
  - NFR-2: 库存报告输出必须为 Markdown，便于直接贴入评审、执行日志或阶段复盘。
  - NFR-3: 新专题与根入口回写后，任何单文档长度都不得突破现有 Markdown 1000 行门禁。
  - NFR-4: 本专题不得弱化 `PRD-ENGINEERING-024` 的四层消费模型，而应在其上追加维护成本视角。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (2026-04-17): 建立 `PRD/design/project` 三件套与库存报告脚本，冻结阶段判断与 follow-up 分类。
  - v1.1: 拆 `doc/devlog` 历史压缩专题，收口 57 份日文件的维护方式。
  - v1.2: 拆 `world-simulator / p2p / testing` 的路径级治理任务，优先处理热点子目录。
  - v1.3: 若库存报告在季度复核里持续显示增长，再评估是否追加门禁/基线。
- Technical Risks:
  - 风险-1: 如果只有报告没有 follow-up 建项，维护成本仍会继续累积。
  - 风险-2: 如果把“存量治理”误做成批量删除，可能破坏历史追溯与证据链。
  - 风险-3: 如果只盯总量、不看热点子目录和近限文件，会错过真正抬高维护成本的局部热点。

## 6. Validation & Decision Record
- Test Plan & Traceability:

| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-ENGINEERING-025 | `doc-corpus-maintenance-governance` | `test_tier_required` | 新专题三件套互链、`scripts/doc-inventory-report.sh` 输出当前仓库体量快照、engineering 根入口/主项目/索引回写、`doc-governance-check.sh` 通过 | 文档治理阶段判断、后续路径级治理优先级与季度复盘输入 |

- Decision Log:
  - DEC-DCM-001: 选择把“入口减重后的维护成本”单独立题，而不是继续塞进 `PRD-ENGINEERING-024`，因为两者解决的是不同阶段的问题。
  - DEC-DCM-002: 选择先补库存报告和建项规则，而不是立即大规模迁文档路径，因为当前最缺的是统一裁定依据。
  - DEC-DCM-003: 选择把 `doc/devlog` backlog 重新纳入治理对象，而不是继续以“历史归档”名义长期豁免，因为它已经在抬高查找和同步成本。
