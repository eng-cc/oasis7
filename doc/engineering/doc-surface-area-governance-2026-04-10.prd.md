# 文档体量治理与活跃阅读面收敛（2026-04-10）

- 对应设计文档: `doc/engineering/doc-surface-area-governance-2026-04-10.design.md`
- 对应项目管理文档: `doc/engineering/doc-surface-area-governance-2026-04-10.project.md`

审计轮次: 1

## 目标
- 把“`doc/` 文档太多，默认阅读路径失去聚焦”的问题升级为正式 engineering 治理专题。
- 冻结 `活跃真值 / 审计留痕 / 历史归档 / 兼容跳转` 四层消费模型，避免把所有可检索文档都暴露为默认入口。
- 为后续高密度模块减重提供统一裁定口径，优先压缩阅读面，再决定是否做路径迁移或批量改写。

## 范围
- 覆盖 root README、模块 README、模块 `prd.md/design.md/project.md/prd.index.md` 的默认阅读面规则。
- 覆盖 `reviews/`、`governance/`、`evidence/`、`doc/devlog/`、root legacy redirect 的消费层分类。
- 覆盖高密度模块的触发条件、优先级和后续任务拆解口径。
- 不直接执行跨模块大规模迁移、不重建 `archive/` 目录树、不在本批修改门禁脚本逻辑。

## 接口 / 数据
- 追踪主键: `PRD-ENGINEERING-024`
- 模块主入口: `doc/engineering/prd.md`
- 模块执行入口: `doc/engineering/project.md`
- 规范基线: `doc/engineering/doc-structure-standard.prd.md`
- 关键导航入口: `doc/README.md`、`doc/<module>/README.md`、`doc/<module>/prd.index.md`
- 体量快照（2026-04-10）:
  - `doc/` 文件总数: 1720
  - 高密度模块: `world-simulator` 547、`p2p` 269、`testing` 182
  - 高密度子目录: `world-simulator/viewer` 296

## 里程碑
- M1 (2026-04-10): 建立专题 `prd/design/project`，冻结四层消费模型与默认入口减重规则。
- M2: 为 `world-simulator / p2p / testing` 建立首批活跃阅读面收敛任务。
- M3: 视第一批收敛结果再决定是否需要追加索引重分层、round 治理或脚本门禁扩展。

## 风险
- 若只冻结概念、不生成后续模块任务，专题会停留在说明层，无法实际减轻阅读负担。
- 若过早批量迁移文件路径，容易引入引用断链、allowlist 漂移和 review 噪声。

## 1. Executive Summary
- Problem Statement: `doc/` 规模已经显著扩大，活跃规格、审计台账、历史归档和兼容跳转混在同一阅读面里，导致项目经理或模块 owner 很难在短时间内得到“what / where / next / risk”。
- Proposed Solution: 先把“文档体量治理”升级为正式专题，冻结四层消费模型与默认入口减重规则，要求根入口和模块入口只暴露活跃真值与最小必要跳转，其余材料保留可检索性但退出主阅读面。
- Success Criteria:
  - SC-1: 正式文档明确区分 `活跃真值 / 审计留痕 / 历史归档 / 兼容跳转` 四层消费模型。
  - SC-2: 默认阅读面规则要求 `doc/README.md` 与模块 README 不再把 `doc/devlog/`、round review、审计证据作为主入口项。
  - SC-3: 高密度模块的首批优先级与触发阈值形成正式口径，可直接拆后续任务。
  - SC-4: engineering 主 PRD/project、模块 README、文件级索引与 allowlist 均完成回写并通过治理检查。
  - SC-5: 后续模块减重任务可以在 15 分钟内基于本专题判断“先压入口、还是再迁移路径”。

## 2. User Experience & Functionality
- User Personas:
  - 项目经理 / `producer_system_designer`：需要先看当前真值和下一步，而不是先被审计材料淹没。
  - 模块 owner：需要知道哪些文档必须作为主入口暴露，哪些只保留检索和证据职责。
  - 文档治理评审者：需要统一裁定标准，决定“这篇文档该留在阅读面还是退出阅读面”。
- User Scenarios & Frequency:
  - 项目状态盘点：每次用户问“这个模块该看什么、是不是太多”时触发。
  - 模块入口维护：每次 README / `prd.index.md` 增长到难以浏览时触发。
  - 审计/归档治理：每轮 round review、evidence 累积或 legacy redirect 收口后触发。
- User Stories:
  - PRD-ENGINEERING-024-001: As a 项目经理, I want root/module entry docs to expose only current truth and next-step selectors, so that I can answer `what / where / next / risk` quickly.
  - PRD-ENGINEERING-024-002: As a 模块 owner, I want a stable way to classify docs into active truth vs audit/archive/redirect, so that I know what belongs on the default reading path.
  - PRD-ENGINEERING-024-003: As a 文档治理评审者, I want density triggers and priority rules, so that high-volume modules are reduced in a predictable order.
- Critical User Flows:
  1. Flow-DSA-001:
     `用户想快速了解模块当前状态 -> 先读 doc/README.md -> 再读模块 README/prd/project -> 只在需要追证时跳到 reviews/governance/evidence`
  2. Flow-DSA-002:
     `新增或评审一篇文档 -> 判断它回答的是“现在怎么做”还是“如何证明/回顾/兼容” -> 归类到四层消费模型 -> 决定是否进入默认阅读面`
  3. Flow-DSA-003:
     `模块体量持续增长 -> 触发高密度阈值 -> 生成减重候选 -> 先调整 README/prd.index/default selectors -> 再评估是否需要文件迁移`
  4. Flow-DSA-004:
     `需要历史上下文 -> 从活跃入口定向跳转到 round review / evidence / devlog -> 读取后返回活跃真值，不把历史材料重新升格为主入口`
- Functional Specification Matrix:

| 对象/能力 | 字段定义 | 动作/行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 活跃真值 | 模块主入口、活跃专题 PRD/Design/Project、高频 manual/runbook | 保留在 root README、模块 README、`prd.index.md` 的默认阅读路径中 | `draft -> active -> maintained` | 优先回答 `what / where / next / risk` | 模块 owner 维护，所有贡献者可读 |
| 审计留痕 | `reviews/`、`governance/`、`evidence/`、checklists、round 台账 | 保留可检索性和定向引用，但不直接进入默认主入口列表 | `active evidence -> referenced -> background-only` | 仅在需要证明、复盘、审计时进入阅读链路 | owner/评审者可更新，默认读者按需进入 |
| 历史归档 | `doc/devlog/`、已收口历史专题、旧批次复盘 | 只保留追溯和归档职责，不作为运行态真值或默认阅读入口 | `active -> archived` | 默认不进入 root/module README 主列表 | 维护者可保留，默认读者仅在回溯时使用 |
| 兼容跳转 | root legacy redirect、软 redirect 提示页 | 仅提供主入口跳转与最小声明，不承载业务正文 | `legacy -> redirected -> minimal` | 标题清晰、正文最小、必须指向 canonical 入口 | 维护者可改，评审者可裁定是否仍需保留 |
| 高密度触发器 | 模块文件数、子目录文件数、默认入口项数 | 达阈值后必须生成减重任务或治理说明 | `normal -> crowded -> action_required` | 优先模块级文件数，再看子目录热点与默认入口冗长度 | `producer_system_designer` 裁定优先级，模块 owner 执行 |
- Acceptance Criteria:
  - AC-1: 专题正文明确给出四层消费模型的定义、边界和默认处理规则。
  - AC-2: 专题明确说明“先压入口面，再决定是否迁路径”的执行顺序。
  - AC-3: 专题明确 `reviews/governance/evidence` 与 `doc/devlog` 默认退出主阅读面的规则。
  - AC-4: 专题明确 root legacy redirect 只保留最小兼容跳转，不再扩展为业务正文。
  - AC-5: 专题明确首批高风险模块优先级，并给出密度触发条件。
  - AC-6: engineering 主 PRD/project、README、`prd.index.md` 与 allowlist 完成同步回写。
  - AC-7: 专题不要求本批即执行大规模路径迁移，也不重新引入 `archive/` 目录树。
  - AC-8: `scripts/doc-governance-check.sh` 在本批文档变更后通过。
- Non-Goals:
  - 不在本批直接改写 `doc/README.md` 或多个模块 README 的入口结构。
  - 不在本批迁移 `reviews/`、`governance/`、`evidence/` 或 `doc/devlog/` 的存储路径。
  - 不把历史归档重新升格为 `.pm` 或正式 PRD 的运行态真值。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 无新增 AI 专属工具要求；主要依赖 `rg`、`find`、`scripts/doc-governance-check.sh` 与人工结构评审。
- Evaluation Strategy:
  - 用仓库快照统计验证高密度模块与子目录识别是否准确。
  - 用入口文档人工抽查验证“what / where / next / risk”是否可在 15 分钟内回答。

## 4. Technical Specifications
- Architecture Overview:
  - 该专题不改变 `doc/` 的主目录树，而是在“消费层”增加显式分层。
  - root README、模块 README、模块 `prd/design/project/prd.index` 组成默认阅读面。
  - round review、governance、evidence、devlog、redirect 保持可检索，但默认退出主阅读面。
- Integration Points:
  - `doc/README.md`
  - `doc/<module>/README.md`
  - `doc/<module>/prd.md`
  - `doc/<module>/project.md`
  - `doc/<module>/prd.index.md`
  - `doc/engineering/doc-structure-standard.prd.md`
  - `doc/core/reviews/*`
  - `doc/devlog/*`
- Edge Cases & Error Handling:
  - 高价值审计材料仍被频繁引用：允许保留专题内直链，但不应提升为模块 README 默认入口项。
  - root redirect 仍承担外部链接兼容：允许继续保留，但正文必须维持最小跳转结构。
  - 规模不大但路径混乱的模块：优先做入口澄清，不因绝对文件数低就忽略阅读面问题。
  - 大目录里同时存在活跃真值和审计材料：先通过 `README` / `prd.index` 做入口分流，再评估是否需要路径层面的二次治理。
- Non-Functional Requirements:
  - NFR-1: 默认阅读路径必须支持在 15 分钟内回答模块的 `what / where / next / risk`。
  - NFR-2: 新增治理专题必须在 5 分钟内判断其是否属于活跃真值、审计留痕、历史归档或兼容跳转。
  - NFR-3: 高密度模块一旦被标记为 `action_required`，必须在对应主项目或专题项目中有明确后续任务。
  - NFR-4: 本专题与主入口回写后的单文件长度均不得突破现有 Markdown 1000 行治理上限。
  - NFR-5: 本专题不得引入会破坏现有引用稳定性的批量路径调整。
- Security & Privacy:
  - 本专题只调整文档消费层，不新增敏感数据。
  - 历史归档和审计证据仍需遵守原有脱敏与引用边界，不因退出主阅读面而降低保护要求。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (2026-04-10): 冻结四层消费模型和默认入口减重规则。
  - v1.1: 为 `world-simulator / p2p / testing` 拆出首批活跃阅读面收敛任务。
  - v2.0: 视第一批结果决定是否把入口分层规则进一步纳入 review round 或脚本门禁。
- Technical Risks:
  - 风险-1: 如果模块 README 不按专题规则回写，正式口径会与实际入口继续漂移。
  - 风险-2: 如果把“退出主阅读面”误解为“删除文档”，可能破坏历史追溯和证据可达性。
  - 风险-3: 如果后续模块任务长期不立项，体量治理仍然只停留在概念层。

## 6. Validation & Decision Record
- Test Plan & Traceability:

| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-ENGINEERING-024 | TASK-ENGINEERING-106/107/108/109/110/111/112/114/115 | `test_tier_required` | 专题三件套互链、engineering 主 PRD/project/README/`prd.index.md` 回写、模块 README / `prd.index.md` 入口减重、低密度模块复核结论回写、`scripts/doc-governance-check.sh` 通过、人工核对默认阅读面分层表述 | 仓库文档消费层治理、项目经理视角导航与后续模块减重任务拆解 |

- Decision Log:
  - DEC-DSA-001: 选择“消费层四分法”而不是立即迁移目录树，因为当前最直接的问题是默认阅读面过宽，而不是文件暂时放在哪里。
  - DEC-DSA-002: 选择先改 root/module 入口与索引规则，而不是先改 `reviews/governance/evidence` 存储路径，因为入口面收紧对现有引用破坏最小。
  - DEC-DSA-003: 选择把 `doc/devlog/` 明确压回历史归档层，而不是继续把它当作项目经理阅读路径的一部分，因为 `.pm` 和正式 PRD/project 已经承接运行态真值。
