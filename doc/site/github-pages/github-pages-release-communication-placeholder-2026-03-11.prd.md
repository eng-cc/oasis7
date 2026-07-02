# oasis7: 站点公开发布口径占位（2026-03-11）

- 对应设计文档: `doc/site/github-pages/github-pages-release-communication-placeholder-2026-03-11.design.md`
- 对应项目管理文档: `doc/site/github-pages/github-pages-release-communication-placeholder-2026-03-11.project.md`

审计轮次: 5

## 1. Executive Summary
- Problem Statement: `readme` 模块已经形成版本候选的简报、公告底稿与模板，但公开站点入口仍停留在“技术预览 / 下载包”层，没有明确告诉访问者“当前公开说明仍在准备中、下载页不等于正式对外发布”。这会让站点公开入口和新建立的 release communication 体系脱节。
- Proposed Solution: 在 `site` 的首页与文档入口页增加统一的公开说明占位，明确当前处于“对外说明准备态”、下载/发布说明链接仍仅代表技术预览构建信息，并为后续正式 announcement / changelog 留出稳定入口位置。
- Success Criteria:
  - SC-1: CN/EN 首页与文档入口页都出现统一的“公开说明准备态”占位文案。
  - SC-2: 占位文案不削弱“技术预览（尚不可玩）”主口径，只补充发布沟通状态。
  - SC-3: 访问者可以区分“构建包发布说明”与“面向玩家/社区的正式公告”不是同一层含义。
  - SC-4: `site` 主项目能够追踪该公开入口补位任务。

## 2. User Experience & Functionality
- User Personas:
  - 新访问者：需要理解当前公开页面仍处于技术预览与说明准备态。
  - `liveops_community`：需要站点入口为后续正式公告保留一致的安全说明位。
  - `producer_system_designer`：需要确保公开站点不抢跑内部 release communication 边界。
- User Scenarios & Frequency:
  - 首次访问首页：看到当前状态与公开说明准备态。
  - 进入文档中心：知道文档页支持技术预览协作，不代表已开放玩家体验。
  - 查看下载区：理解 GitHub release notes 仅代表构建说明，不代表正式对外公告。
- User Stories:
  - PRD-SITE-005: As a 新访问者, I want the homepage to distinguish preview build notes from public release messaging, so that I do not mistake technical packages for a live launch.
  - PRD-SITE-006: As a `liveops_community`, I want a placeholder for upcoming public communication on the site, so that future announcement rollout has a stable public anchor.
  - PRD-SITE-007: As a `producer_system_designer`, I want site copy to stay aligned with current candidate posture, so that public promises do not outrun internal review status.
- Critical User Flows:
  1. `访问首页 -> 看到技术预览口径 -> 看到公开说明准备态占位 -> 再决定是否下载构建包`
  2. `进入文档入口 -> 看到当前仅是技术预览协作入口 -> 了解正式公告仍待后续发布`
  3. `点击 release notes -> 理解其是构建说明入口，而非玩家公告入口`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| public communication placeholder | 当前状态、说明层级、后续入口提示 | 在公开入口显示说明位 | `absent -> visible -> maintained` | 状态提示优先于营销表述 | `viewer_engineer` 落站点，`producer_system_designer` 复核 |
| release notes disclaimer | release notes 含义、非正式公告提示 | 补充下载区说明 | `implicit -> explicit` | 下载说明后紧跟风险提示 | 发布责任人复核 |
| docs hub posture note | 文档入口是否代表正式公告 | 在 docs hero / note 处补充说明 | `implicit -> explicit` | 与首页口径同构 | 站点维护者可改 |
- Acceptance Criteria:
  - AC-1: 产出站点公开发布口径占位专题 PRD / Design / Project。
  - AC-2: `site/index.html`、`site/en/index.html`、`site/doc/cn/index.html`、`site/doc/en/index.html` 都新增公开说明准备态占位。
  - AC-3: 占位文案明确区分构建说明与正式公告。
  - AC-4: `doc/site/project.md` 能追踪该任务与站点状态。
- Non-Goals:
  - 不直接发布正式公告。
  - 不改 GitHub Releases 内容。
  - 不改变“技术预览（尚不可玩）”主口径。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 该专题位于 `site/github-pages`，连接 `readme` 的 release communication 能力与公开站点入口，为未来正式外部说明保留一致锚点。
- Integration Points:
  - `site/index.html`
  - `site/en/index.html`
  - `site/doc/cn/index.html`
  - `site/doc/en/index.html`
  - `doc/readme/governance/readme-release-communication-template-2026-03-11.prd.md`
  - `doc/readme/governance/readme-release-announcement-template-2026-03-11.prd.md`
- Edge Cases & Error Handling:
  - 如果未来正式公告已发布：占位文案应切换为正式公告入口，而不是继续停留在“准备态”。
  - 如果内部候选状态变化：占位文案必须与最新口径同步，不得继续沿用过时说明。
  - 如果 release notes 仍只有技术包说明：必须继续保留“非正式公告”提示。
- Non-Functional Requirements:
  - NFR-SITE-5-1: 四个公开入口页在同一天内完成文案同步。
  - NFR-SITE-5-2: “正式公告未发布”含义必须在 1 次阅读内可理解。
  - NFR-SITE-5-3: 不引入新的站点脚本或交互依赖。
- Security & Privacy: 站点只公开当前允许公开的状态与说明层级，不暴露内部 review 文档细节。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (`SITE-COMM-1`): 为公开入口补上说明准备态占位。
  - v1.1 (`SITE-COMM-2`): 将正式 announcement / changelog 链接替换占位说明。
  - v2.0 (`SITE-COMM-3`): 评估是否将公开说明状态自动化同步到站点构建流程。
- Technical Risks:
  - 风险-1: 若占位文案过强，可能被误读为“即将正式发布”。
  - 风险-2: 若不补占位，公开站点与 release communication 体系会继续割裂。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-SITE-005 | `TASK-SITE-010` | `test_tier_required` | 检查首页/下载区已区分构建说明与正式公告 | 公开站点状态理解 |
| PRD-SITE-006 | `TASK-SITE-010` | `test_tier_required` | 检查站点存在统一“公开说明准备态”占位 | 发布沟通入口一致性 |
| PRD-SITE-007 | `TASK-SITE-010` | `test_tier_required` | 检查技术预览主口径未被削弱且与新占位并存 | 对外承诺边界控制 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| `DEC-SITE-005` | 在站点公开入口新增说明占位 | 继续只保留下载与技术预览说明 | 需要让站点跟上新的 release communication 链。 |
| `DEC-SITE-006` | 占位只说明“准备态”，不直接链接内部评审文档 | 直接把内部治理文档暴露到公开站点 | 对外入口应保持简洁且不泄露内部治理结构。 |
| `DEC-SITE-007` | 继续保守表达“技术预览 + 公告准备态” | 站点先行暗示正式发布临近 | 公开承诺必须晚于内部裁决与正式沟通动作。 |
