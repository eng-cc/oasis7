# `doc/devlog` 历史压缩与入口收口（2026-04-17）

- 对应设计文档: `doc/engineering/doc-governance/devlog-history-compaction-2026-04-17.design.md`
- 对应项目管理文档: `doc/engineering/doc-governance/devlog-history-compaction-2026-04-17.project.md`

审计轮次: 1

## 1. Executive Summary
- Problem Statement: `PRD-ENGINEERING-025` 已经明确 `doc/devlog` 是当前文档维护成本的第一优先级对象，但仓库里仍保留 57 份日文件，集中在 `2026-02`（26 份）与 `2026-03`（30 份），且至少 13 份单文件超过 1000 行、最大达到 3288 行。当前仓库只有“`doc/devlog` 是历史归档”的声明，没有一个 canonical 入口来回答“该从哪一天开始看、哪些是高体量热点、哪些应该避免盲扫”。
- Proposed Solution: 建立 `devlog-history-compaction` 专题，冻结 `doc/devlog` 的 archive/index/retention 边界；在不删除任何日文件的前提下新增 `doc/devlog/README.md` 作为 canonical 入口，按月份、重文件和使用场景分流历史日志，并把上游 `PRD-ENGINEERING-025` 的第一条 follow-up 正式收口。
- Success Criteria:
  - SC-1: engineering 存在正式专题，明确 `doc/devlog` 从“历史归档声明”升级为“首个已执行的维护成本 follow-up”。
  - SC-2: `doc/devlog/README.md` 成为 canonical 入口，能够按月导航全部 57 份日文件，而不是让读者按文件系统盲扫。
  - SC-3: `doc/devlog/README.md` 明确列出高体量热点日文件，帮助读者优先判断哪些天适合先读摘要/先跳过。
  - SC-4: 保持历史追溯完整，不删除既有日文件，也不把 `doc/devlog` 重新升格为运行态真值。
  - SC-5: engineering 主 PRD、主项目、README、索引与 `doc-corpus-maintenance-governance` 项目页完成回写，明确 `doc/devlog` follow-up 已完成，下一步转入 `world-simulator` 路径级治理。

## 2. User Experience & Functionality
- User Personas:
  - 项目经理 / `producer_system_designer`: 需要快速判断 devlog 的历史窗口，而不是一份份顺扫 57 个日文件。
  - 文档治理评审者: 需要明确哪些历史日志是高体量热点，哪些只需要按月归档保留。
  - 模块 owner: 需要知道旧“回写 devlog”口径现在只能走历史入口，不能继续当运行态真值。
- User Scenarios & Frequency:
  - 追溯 2026-02 / 2026-03 集中改动期: 在做架构回溯、治理复盘或历史证据补读时触发。
  - 审查高体量日文件: 在考虑月度/阶段摘要或下一轮压缩动作时触发。
  - 说明当前约束: 当有人继续把 `doc/devlog` 当活跃入口或要求写新日文件时触发。
- User Stories:
  - PRD-ENGINEERING-026: As a 项目经理/文档治理评审者, I want a canonical `doc/devlog` archive entrypoint, so that I can navigate historical daily logs by month and hotspot instead of scanning day files blindly.
- Critical User Flows:
  1. Flow-DVC-001:
     `进入 doc/devlog/README.md -> 先看月份分布和高体量热点 -> 再按月份进入具体日文件`
  2. Flow-DVC-002:
     `需要追某一轮高密度变更 -> 先看高体量日文件列表 -> 再决定是否读单日原文`
  3. Flow-DVC-003:
     `需要确认 devlog 当前职责 -> 读取 README 的 archive/source-of-truth 边界 -> 返回 project/task execution log`
- Functional Specification Matrix:

| 对象/能力 | 字段定义 | 动作/行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| `doc/devlog/README.md` | 月份分布、高体量热点、日文件导航、职责边界 | 作为 `doc/devlog` 的 canonical archive entrypoint | `missing -> canonical -> maintained` | 先按月份，再按重文件关注度 | 所有人可读，治理 owner 可更新 |
| 月度归档视图 | `2026-02` / `2026-03` / `2026-04` 文件清单 | 让读者按月进入，而不是平铺 57 个日文件 | `flat -> grouped` | 按月份升序，文件按日期升序 | 所有人可读 |
| 高体量热点表 | 日文件名、行数、建议用途 | 标出需要后续月度/阶段摘要关注的热点 | `unknown -> surfaced` | 按行数降序 | 评审者/owner 可引用 |
| archive/source-of-truth 边界 | 历史归档、非运行态真值、回链正式文档 | 防止继续把 devlog 当当前态入口 | `implicit -> explicit` | 固定出现在 README 首屏 | 所有人可读 |
- Acceptance Criteria:
  - AC-1: 存在一份正式 `devlog-history-compaction` 专题三件套，冻结问题定义、边界、第一批动作与验证方式。
  - AC-2: `doc/devlog/README.md` 明确说明 `doc/devlog` 只承担历史归档职责，不再作为运行态真值。
  - AC-3: `doc/devlog/README.md` 按月份列出全部现有日文件，并给出高体量热点表。
  - AC-4: 本批不删除任何现有日文件，不批量改写历史正文，也不把历史回写重新迁回日文件要求。
  - AC-5: engineering 根入口、主项目、索引与 `doc-corpus-maintenance-governance` 项目页能够直接指向该专题与 `doc/devlog/README.md`。
- Non-Goals:
  - 不在本批直接把 57 份日文件压缩成月报。
  - 不在本批清理全仓历史“回写 devlog”措辞。
  - 不在本批建立新的 merge gate。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 主要依赖 `find`、`wc`、`python3` 做日文件统计，配合 Markdown 入口文档回写。
- Evaluation Strategy:
  - 复算 `doc/devlog` 文件数、月份分布和高体量文件，确认 README 与专题正文中的数值可重现。
  - 人工验证从 `doc/devlog/README.md` 可以在 2 分钟内定位某个月或某个重文件。

## 4. Technical Specifications
- Architecture Overview:
  - `doc/devlog/*.md` 继续保留原始日文件。
  - `doc/devlog/README.md` 成为唯一推荐入口，承接月度导航和 archive 边界。
  - `devlog-history-compaction` 负责上游治理口径与下一步月度/阶段摘要的拆项基线。
- Integration Points:
  - `doc/devlog/README.md`
  - `doc/README.md`
  - `doc/engineering/prd.md`
  - `doc/engineering/project.md`
  - `doc/engineering/README.md`
  - `doc/engineering/prd.index.md`
  - `doc/engineering/doc-governance/doc-corpus-maintenance-governance-2026-04-17.project.md`
- Edge Cases & Error Handling:
  - 未来继续增加新的历史日文件: 允许保留，但必须更新 `doc/devlog/README.md` 的月份导航。
  - 某天日志行数过大: 先进入高体量热点表，下一轮再决定是否抽月度/阶段摘要。
  - 历史文档中仍残留“回写 devlog”口径: 作为后续治理债记录，不在本批强行批量清理。
- Non-Functional Requirements:
  - NFR-1: `doc/devlog/README.md` 必须在单屏内说明 `doc/devlog` 的职责边界。
  - NFR-2: 新专题与 README 均不得突破 Markdown 1000 行门禁。
  - NFR-3: 新入口必须保持纯 Markdown，可直接被仓库静态阅读链路消费。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (2026-04-17): 建立专题三件套与 `doc/devlog/README.md`，先解决“盲扫 57 天”的入口问题。
  - v1.1: 若高体量日文件仍频繁被追读，再拆月度/阶段摘要专题。
  - v1.2: 将 lingering 的“回写 devlog”旧口径从 active 文档中清出，只保留历史上下文。
- Technical Risks:
  - 风险-1: 只有 README 导航，没有月度摘要时，重文件阅读成本仍然高。
  - 风险-2: 若继续新增大量日文件而不维护 README，入口会再次失效。
  - 风险-3: 若把历史压缩误解为删除原文，会损害历史追溯。

## 6. Validation & Decision Record
- Test Plan & Traceability:

| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-ENGINEERING-026 | `devlog-history-compaction` | `test_tier_required` | 专题三件套互链、`doc/devlog/README.md` 月份导航与高体量热点表、engineering 根入口/主项目/索引回写、`doc-governance-check.sh` 通过 | `doc/devlog` 历史入口、`PRD-ENGINEERING-025` follow-up 收口与后续月度摘要拆项 |

- Decision Log:
  - DEC-DVC-001: 选择先建立 `doc/devlog/README.md` 入口，而不是先直接合并 57 份日文件，因为当前最直接的问题是无导航而不是原文是否存在。
  - DEC-DVC-002: 选择保留所有日文件，只做按月和重文件分流，而不是立即压缩为月报，因为历史原文仍有追溯价值。
  - DEC-DVC-003: 选择将 `doc/devlog` 作为 `PRD-ENGINEERING-025` 的第一条已执行 follow-up，而不是继续停留在“待处理”状态，以便明确下一步转向 `world-simulator`。
