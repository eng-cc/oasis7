# `doc/devlog` 历史压缩与入口收口（2026-04-17）

- 对应设计文档: `doc/engineering/doc-governance/devlog-history-compaction-2026-04-17.design.md`
- 对应项目管理文档: `doc/engineering/doc-governance/devlog-history-compaction-2026-04-17.project.md`

审计轮次: 2

## 1. Executive Summary
- Problem Statement: `PRD-ENGINEERING-025` 已经明确 `doc/devlog` 是当前文档维护成本的第一优先级对象；首批治理仅建立入口，仍保留 57 份日文件、42,309 行历史正文，集中在 `2026-02`（26 份）与 `2026-03`（30 份）。这些日文件已退出运行态真值，但仍持续制造搜索噪音、断链维护成本和重复证据面。
- Proposed Solution: 将 `doc/devlog/README.md` 升级为 compact archive summary，按月保留历史脉络、热点原因和当前职责边界；删除已摘要的 57 份 `2026-*.md` 日文件，并把仓库内仍指向具体日文件的引用收敛到 `doc/devlog/README.md`。
- Success Criteria:
  - SC-1: engineering 存在正式专题，明确 `doc/devlog` 从“历史归档声明”升级为“首个已执行的维护成本 follow-up”。
  - SC-2: `doc/devlog/README.md` 成为 canonical compact archive，能够说明被删除日文件的时间窗口、总体行数和月度主题。
  - SC-3: `doc/devlog/README.md` 明确列出原高体量热点及噪音原因，帮助读者判断历史上下文来源。
  - SC-4: 删除已摘要的 `doc/devlog/2026-*.md` 日文件，不把 `doc/devlog` 重新升格为运行态真值。
  - SC-5: engineering 主 PRD、主项目、README、索引与 `doc-corpus-maintenance-governance` 项目页完成回写，明确 `doc/devlog` follow-up 已完成，下一步转入 `world-simulator` 路径级治理。

## 2. User Experience & Functionality
- User Personas:
  - 项目经理 / `producer_system_designer`: 需要快速判断 devlog 的历史窗口，而不是一份份顺扫 57 个日文件。
  - 文档治理评审者: 需要明确哪些历史日志已被摘要替代，哪些当前状态必须回到 `.pm` 和模块 project。
  - 模块 owner: 需要知道旧“回写 devlog”口径现在只能走历史入口，不能继续当运行态真值。
- User Scenarios & Frequency:
  - 追溯 2026-02 / 2026-03 集中改动期: 在做架构回溯、治理复盘或历史证据补读时触发。
  - 审查高体量历史窗口: 在考虑旧任务背景、治理复盘或引用迁移时触发。
  - 说明当前约束: 当有人继续把 `doc/devlog` 当活跃入口或要求写新日文件时触发。
- User Stories:
  - PRD-ENGINEERING-026: As a 项目经理/文档治理评审者, I want a canonical `doc/devlog` compact archive summary, so that retired daily logs no longer stay in the active repository surface.
- Critical User Flows:
  1. Flow-DVC-001:
     `进入 doc/devlog/README.md -> 先看 retired corpus -> 阅读月度摘要 -> 回到模块 project / .pm 查当前真值`
  2. Flow-DVC-002:
     `需要追某一轮高密度变更 -> 先看 Former Hotspots -> 再查对应模块专题文档或 .pm execution log`
  3. Flow-DVC-003:
     `需要确认 devlog 当前职责 -> 读取 README 的 archive/source-of-truth 边界 -> 返回 project/task execution log`
- Functional Specification Matrix:

| 对象/能力 | 字段定义 | 动作/行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| `doc/devlog/README.md` | retired corpus、月度摘要、高体量热点、职责边界 | 作为 `doc/devlog` 的 canonical compact archive | `daily_files_present -> summarized -> retired` | 先按月份，再按重文件关注度 | 所有人可读，治理 owner 可更新 |
| 月度摘要视图 | `2026-02` / `2026-03` / `2026-04` 摘要 | 用主题摘要替代 57 个日文件 | `flat -> summarized` | 按月份升序 | 所有人可读 |
| 高体量热点表 | 原日文件名、原行数、噪音原因 | 标出已摘要删除的原高体量文件 | `unknown -> surfaced -> retired` | 按行数降序 | 评审者/owner 可引用 |
| archive/source-of-truth 边界 | 历史归档、非运行态真值、回链正式文档 | 防止继续把 devlog 当当前态入口 | `implicit -> explicit` | 固定出现在 README 首屏 | 所有人可读 |
- Acceptance Criteria:
  - AC-1: 存在一份正式 `devlog-history-compaction` 专题三件套，冻结问题定义、边界、第一批动作与验证方式。
  - AC-2: `doc/devlog/README.md` 明确说明 `doc/devlog` 只承担历史归档职责，不再作为运行态真值。
  - AC-3: `doc/devlog/README.md` 按月份摘要全部 retired 日文件，并给出原高体量热点表。
  - AC-4: 删除 `doc/devlog/2026-*.md` 日文件，并将活跃文档中的具体日文件引用收敛到 `doc/devlog/README.md`。
  - AC-5: engineering 根入口、主项目、索引与 `doc-corpus-maintenance-governance` 项目页能够直接指向该专题与 `doc/devlog/README.md`。
- Non-Goals:
  - 不在本批清理全仓历史“回写 devlog”措辞。
  - 不在本批建立新的 merge gate。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 主要依赖 `bash scripts/doc-inventory-report.sh`、`wc -l` 与 Markdown 入口文档回写。
- Evaluation Strategy:
  - 复算 retired 日文件删除后 `doc/devlog` 仅保留 `README.md`。
  - 验证仓库内不再存在指向 `doc/devlog/YYYY-MM-DD.md` 的活跃引用。

## 4. Technical Specifications
- Architecture Overview:
  - `doc/devlog/README.md` 成为唯一保留文件，承接月度摘要和 archive 边界。
  - `doc/devlog/2026-*.md` 日文件删除，不再作为 tracked archive。
  - `devlog-history-compaction` 负责上游治理口径与删除后的引用收敛基线。
- Integration Points:
  - `doc/devlog/README.md`
  - `doc/README.md`
  - `doc/engineering/prd.md`
  - `doc/engineering/project.md`
  - `doc/engineering/README.md`
  - `doc/engineering/prd.index.md`
  - `doc/engineering/doc-governance/doc-corpus-maintenance-governance-2026-04-17.project.md`
- Edge Cases & Error Handling:
  - 未来继续增加新的历史日文件: 默认不允许；应写入 `.pm/tasks/*.execution.md`，必要时更新 `doc/devlog/README.md` 摘要。
  - 某天历史上下文需要补充: 更新摘要或相关专题文档，而不是恢复日文件。
  - 历史文档中仍残留“回写 devlog”口径: 作为后续治理债记录，不在本批强行批量清理。
- Non-Functional Requirements:
  - NFR-1: `doc/devlog/README.md` 必须在单屏内说明 `doc/devlog` 的职责边界。
  - NFR-2: 新专题与 README 均不得突破 Markdown 1000 行门禁。
  - NFR-3: 新入口必须保持纯 Markdown，可直接被仓库静态阅读链路消费。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (2026-04-17): 建立专题三件套与 `doc/devlog/README.md`，先解决“盲扫 57 天”的入口问题。
  - v1.1 (2026-05-27): 用 `doc/devlog/README.md` 月度摘要替代并删除 57 份日文件。
  - v1.2: 将 lingering 的“回写 devlog”旧口径从 active 文档中清出，只保留历史上下文。
- Technical Risks:
  - 风险-1: 摘要会损失逐条执行细节，因此当前任务真值必须依赖 `.pm` execution log。
  - 风险-2: 若继续新增日文件，入口会再次失效。
  - 风险-3: 历史引用改到 summary 后，读者需要通过模块专题或 `.pm` 记录追细节。

## 6. Validation & Decision Record
- Test Plan & Traceability:

| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-ENGINEERING-026 | `devlog-history-compaction` | `test_tier_required` | `doc/devlog/README.md` 摘要存在、`doc/devlog/2026-*.md` 不存在、具体日文件引用清零、`doc-governance-check.sh` 通过 | `doc/devlog` 历史入口、`PRD-ENGINEERING-025` follow-up 收口 |

- Decision Log:
  - DEC-DVC-001: 2026-04-17 先建立 `doc/devlog/README.md` 入口，而不是先直接合并 57 份日文件，因为当时最直接的问题是无导航。
  - DEC-DVC-002: 2026-05-27 改为摘要替代并删除日文件，因为 `.pm` execution log 已成为运行态执行证据，日文件继续保留只会制造维护成本。
  - DEC-DVC-003: 选择将 `doc/devlog` 作为 `PRD-ENGINEERING-025` 的第一条已执行 follow-up，而不是继续停留在“待处理”状态，以便明确下一步转向 `world-simulator`。
