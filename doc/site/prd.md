# site PRD

审计轮次: 6

## 目标
- 建立 site 模块设计主文档，统一需求边界、技术方案与验收标准。
- 确保 site 模块后续改动可追溯到 PRD-ID、任务和测试。

## 范围
- 覆盖 site 模块当前能力设计、接口边界、测试口径与演进路线。
- 覆盖 PRD-ID 到 `doc/site/project.md` 的任务映射。
- 不覆盖实现代码逐行说明与历史过程记录。

## 接口 / 数据
- PRD 主入口: `doc/site/prd.md`
- 项目管理入口: `doc/site/project.md`
- 文件级索引: `doc/site/prd.index.md`
- 追踪主键: `PRD-SITE-xxx`
- 测试与发布参考: `testing-manual.md`

## 里程碑
- M1 (2026-03-03): 完成模块设计 PRD 主体重写与任务改造。
- M2 (2026-03-07): 完成站点手册镜像语义同步与状态文档回写。
- M3 (2026-03-07): 完成对外“可玩状态”口径与真实产品状态对齐。
- M4: 建立 PRD-ID -> Task -> Test 的长期追踪闭环。

## 风险
- 模块边界演进快，文档同步可能滞后。
- 指标口径不稳定会降低验收一致性。
## 1. Executive Summary
- Problem Statement: 站点页面、发布下载入口、文档镜像与 SEO 优化在多轮迭代中快速演进；若对外叙事与真实可玩状态不一致，会直接损伤用户信任。
- Proposed Solution: 将 site PRD 作为站点设计主入口，统一页面结构、内容同步、发布链路、质量门禁与“真实状态口径”约束。
- Success Criteria:
  - SC-1: 首页关键信息架构与模块设计口径保持一致。
  - SC-2: 发布下载链接在版本发布后可用率达到 100%。
  - SC-3: 关键页面的可访问性与性能指标持续满足目标阈值。
  - SC-4: 站点改动任务全部映射到 PRD-SITE-ID。
  - SC-5: 当产品尚不可玩时，首页与文档入口页 100% 显式标注“开发中技术预览 / Not playable yet”。
  - SC-6: 首页不得出现与真实状态冲突的“已可玩/赛季进行中”表述（违规数 0）。
  - SC-7: `doc/site/github-pages/**` 活跃专题中的当前校验命令、viewer crate 路径与 wasm 包名必须统一使用 `oasis7_viewer` / `crates/oasis7*` 口径；旧品牌 viewer 包名与源码路径仅允许保留在历史证据或外部原文引用中。
  - SC-8: 首页首屏与首个信息段必须在 30 秒内回答“这是什么游戏 / 玩家如何参与 / 现在可以做什么”，避免陌生访客把公开站点误读为纯工程状态页。

## 2. User Experience & Functionality
- User Personas:
  - 新访问者：需要快速理解项目价值与安装入口。
  - 技术用户：需要稳定访问文档与发布资产。
  - 站点维护者：需要统一发布与验收标准。
- User Scenarios & Frequency:
  - 首页信息浏览：每位新访问者首次访问执行。
  - 文档与下载访问：技术用户按需高频访问。
  - 发布前巡检：每次版本发布前执行一次完整检查。
  - 发布后回归：每次发布后执行稳定性与断链复核。
- User Stories:
  - PRD-SITE-001: As a 新访问者, I want a clear homepage narrative, so that I can understand the product quickly.
  - PRD-SITE-002: As a 技术用户, I want trustworthy download and docs links, so that I can install and verify efficiently.
  - PRD-SITE-003: As a 维护者, I want measurable quality gates, so that releases are predictable.
  - PRD-SITE-004: As a 对外内容维护者, I want marketing copy to match real product readiness, so that user trust is not damaged by over-claiming.
- PRD-SITE-005: As a 新访问者, I want the public site to distinguish preview build notes from formal public release messaging, so that I do not mistake technical artifacts for a live launch.
- PRD-SITE-006: As a `liveops_community`, I want a public placeholder for upcoming release communication, so that formal announcement rollout has a stable site anchor.
- PRD-SITE-007: As a `producer_system_designer`, I want public site copy to remain aligned with candidate posture, so that release promises never outrun internal review status.
- PRD-SITE-008: As a 新访问者, I want the public site and release downloads to use the canonical `oasis7` repo path and asset names, so that branding, links, and downloaded files stay consistent.
- PRD-SITE-009: As a 新访问者, I want the homepage to explain the game genre, player role, and current availability before technical validation details, so that I can decide within the first screen whether oasis7 is worth following.
- Critical User Flows:
  1. Flow-SITE-001: `访问首页 -> 理解价值与入口 -> 跳转安装/文档`
  2. Flow-SITE-002: `发布前执行链接检查 -> 处理断链 -> 复测通过`
  3. Flow-SITE-003: `发布后监控质量指标 -> 发现退化 -> 回滚或修复`
  4. Flow-SITE-004: `确认真实可玩状态 -> 回写首页/文档入口口径 -> 复核中英一致性 -> 发布`
  5. Flow-SITE-005: `陌生访客访问首页 -> 在首屏理解游戏类型/玩家角色/当前开放状态 -> 决定继续看玩法、技术预览或文档`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 首页信息架构 | 版块标题、类型定义、玩家角色、当前开放状态、入口链接、版本信息 | 点击跳转玩法说明/技术预览/文档/下载 | `draft -> published -> revised` | 先回答“是什么/怎么参与/现在能做什么”，再下沉技术细节 | 站点维护者可修改 |
| 发布下载链路 | 版本号、资产地址、校验信息 | 发布后自动校验可用性 | `prepared -> published -> verified` | 最新版本优先展示 | 发布负责人审批上线 |
| 质量门禁巡检 | 链接状态、性能指标、可访问性结果 | 巡检失败阻断发布 | `checking -> passed/blocked` | 严重问题优先修复 | 维护者可解阻断（需说明） |
| 对外状态口径 | 产品状态（playable / preview）、禁用词列表、状态标识文案 | 发布前校验首页与文档入口文案是否匹配真实状态 | `draft -> aligned -> published` | 真实状态字段优先于营销文案 | 模块维护者可改，发布责任人复核 |
- Acceptance Criteria:
  - AC-1: site PRD 定义页面层级、内容同步和发布链路。
  - AC-2: site project 文档任务映射 PRD-SITE-ID。
  - AC-3: 与 `site/doc` 与 GitHub Pages 相关设计文档口径一致。
  - AC-4: 发布前完成链接有效性与基础质量检查。
  - AC-5: 首页与文档入口页（`site/index.html`、`site/en/index.html`、`site/doc/cn/index.html`、`site/doc/en/index.html`）对“是否可玩”状态的表达必须一致且与真实状态相符。
  - AC-6: `doc/site/github-pages/**` 仍可读历史专题的首行标题必须统一使用 `oasis7` 品牌；旧 `oasis7*` 标题仅允许保留在正文历史上下文与证据原文中。
  - AC-7: `doc/site/github-pages/**` 活跃专题中的当前 `cargo check -p` 命令、viewer crate 路径与 wasm 包名必须写为 `oasis7_viewer` / `crates/oasis7*`；旧品牌 viewer 包名与源码路径仅允许保留在历史证据或外部原文引用中。
  - AC-8: `site/index.html` 与 `site/en/index.html` 的首屏和首个正文段必须明确交代游戏类型、玩家扮演的角色、核心差异点，以及“当前仍是技术预览”的边界，且中英结构保持同构。
- Non-Goals:
  - 不在 site PRD 中定义 runtime/p2p 低层实现。
  - 不覆盖内部测试流程细节（由 testing 模块负责）。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 静态站构建链路、链接检查、截图/视觉基线流程。
- Evaluation Strategy: 以页面可用性、链接有效率、发布后回滚次数与问题修复时长评估。

## 4. Technical Specifications
- Architecture Overview: site 模块负责对外展示与文档镜像层，与 readme/testing/world-simulator 等模块联动维护入口一致性。
- Integration Points:
  - `site/`
  - `site/doc/`
  - `doc/site/github-pages/`
  - `doc/site/manual/`
  - `doc/readme/prd.md`
- Edge Cases & Error Handling:
  - 断链：发现下载或文档断链时阻断发布并进入修复流程。
  - 空页面：关键页面内容缺失时展示维护提示并记录异常。
  - 权限不足：发布权限缺失时拒绝上线并提示责任人。
  - 超时：构建/巡检超时时输出中间结果并允许重试。
  - 并发发布：同版本并发发布时只允许一个发布会话生效。
  - 数据异常：版本元数据错误时不展示到公开页面。
  - 状态漂移：若产品尚不可玩但页面出现“已可玩/赛季进行中”表述，发布流程必须阻断并回写文案。
- Non-Functional Requirements:
  - NFR-SITE-1: 发布后关键链接可用率 100%。
  - NFR-SITE-2: 核心页面性能与可访问性指标达到门禁阈值。
  - NFR-SITE-3: 多语言内容口径一致并可追溯。
  - NFR-SITE-4: 发布回滚流程可在限定时间内执行。
  - NFR-SITE-5: 站点输出不得暴露内部敏感配置。
  - NFR-SITE-6: “可玩状态口径一致性”检查在发布前必须完成且误报率 <= 5%（按月统计）。
- Security & Privacy: 站点不得暴露内部凭据与敏感配置；下载链路需具备来源可验证性。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (2026-03-03): 固化站点信息架构与发布验收口径。
  - v1.1: 增加多语言内容一致性与截图回归基线。
  - v2.0: 建立站点发布质量趋势跟踪（性能、可访问性、失效率）。
- Technical Risks:
  - 风险-1: 内容更新频率高导致页面口径漂移。
  - 风险-2: 发布资产链接策略变化引入断链风险。
  - 风险-3: 对外文案过度承诺导致信任流失与社区负反馈。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-SITE-001 | TASK-SITE-001/002/005 | `test_tier_required` | 首页结构与导航检查 | 用户首次访问体验 |
| PRD-SITE-002 | TASK-SITE-002/003/005/006/020 | `test_tier_required` | 下载与文档链接巡检、手册镜像语义对齐校验 | 安装与文档可用性 |
| PRD-SITE-003 | TASK-SITE-003/004/005/007/020 | `test_tier_required` + `test_tier_full` | 发布门禁与回归节奏复核、项目状态回写核对 | 发布稳定性与回滚能力 |
| PRD-SITE-004 | TASK-SITE-008/009 | `test_tier_required` | 中英文首页与文档入口页状态文案一致性核验 | 对外信任与信息准确性 |
| PRD-SITE-005 | TASK-SITE-010 | `test_tier_required` | 首页/下载区已区分构建说明与正式公告 | 公开站点状态理解 |
| PRD-SITE-006 | TASK-SITE-010 | `test_tier_required` | 站点存在统一“公开说明准备态”占位 | 发布沟通入口一致性 |
| PRD-SITE-007 | TASK-SITE-010 | `test_tier_required` | 技术预览主口径与新占位并存且无冲突 | 对外承诺边界控制 |
| PRD-SITE-008 | TASK-SITE-015/016/017/018 | `test_tier_required` | `site/**`、GitHub Release 下载入口、release workflow 当前路径/包名与 `doc/site/github-pages/**` 历史专题标题全部切换到 `oasis7` 品牌与 `eng-cc/oasis7` 路径 | 对外品牌一致性、下载链路稳定性 |
| PRD-SITE-009 | homepage-game-explainer | `test_tier_required` | 中英首页首屏信息架构核对、文档治理检查、静态站点可视回归 | 新访客首访理解效率、公开定位清晰度 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-SITE-001 | 发布前强制执行质量巡检 | 发布后补检查 | 可提前发现阻断问题。 |
| DEC-SITE-002 | 下载链路绑定版本与校验信息 | 仅展示下载地址 | 可提升来源可信度。 |
| DEC-SITE-003 | 站点口径与 readme 联动维护 | 独立维护站点文案 | 可降低对外口径漂移。 |
| DEC-SITE-004 | 对外“可玩状态”按真实产品状态保守表达（未可玩即明确预览） | 继续使用高承诺营销口径 | 可避免误导用户并降低信任风险。 |
| DEC-SITE-005 | 站点先补“说明准备态”占位，再等待正式公告入口 | 直接在公开站点暗示正式发布已临近 | 站点公开承诺必须保持晚于内部正式沟通动作。 |
| DEC-SITE-006 | 公开站点、GitHub Pages canonical 与 release 资产名统一为 `oasis7` | 保留旧 `oasis7` 外显名称仅改仓库 slug | 外部访问者最先接触的是站点与下载名，品牌必须先在这一层完全一致。 |
| DEC-SITE-007 | 首页优先讲清“游戏是什么、玩家像什么、现在能做什么”，技术验证与下载说明下沉到后续版块 | 继续让首屏以技术预览、命令链路和工程状态为主 | 对陌生访客来说，先理解产品语义，再判断是否深入技术细节，才是更低摩擦的公开入口。 |
