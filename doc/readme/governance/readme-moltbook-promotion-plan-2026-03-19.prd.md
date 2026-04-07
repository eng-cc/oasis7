# oasis7: Moltbook 推广方案（2026-03-19）

- 对应设计文档: `doc/readme/governance/readme-moltbook-promotion-plan-2026-03-19.design.md`
- 对应项目管理文档: `doc/readme/governance/readme-moltbook-promotion-plan-2026-03-19.project.md`

审计轮次: 6

## 1. Executive Summary
- Problem Statement: `liveops_community` 需要一份能落到 `https://www.moltbook.com/` 的平台化推广方案，但 Moltbook 当前公开定位不是通用社媒，而是“AI agent 社交网络 + 人类经 X 验证的 agent 身份层 + submolt 社区 + developer early access”；若直接套用通用游戏宣发稿，容易既不适配平台，又越过现有技术预览口径。
- Proposed Solution: 在 `readme/governance` 建立一份 Moltbook 专题推广方案，先固化平台现状、目标受众、内容支柱、30 天执行节奏、禁宣称项、反馈回流和 owner 审核链，再供 `liveops_community` 按方案派生真实帖子与互动回复。
- Success Criteria:
  - SC-1: 方案明确 Moltbook 当前公开机制与推荐打法，而不是抽象“社媒运营”空话。
  - SC-2: 所有对外表述都绑定当前 `standard_3d / software_safe / pure_api` 三访问面的技术预览 claim envelope。
  - SC-3: 方案包含明确的内容节奏、社区互动规则、CTA 层级和信号回流机制。
  - SC-4: 方案包含 `liveops_community -> producer_system_designer` 的审核边界，避免未经批准的外部承诺。

## 2. User Experience & Functionality
- User Personas:
  - `liveops_community`: 需要把 oasis7 讲成适合 Moltbook 的 agent-native 故事，而不是普通游戏广告。
  - `producer_system_designer`: 需要确保任何对外内容不超出当前技术预览边界。
  - Moltbook 上的 agent builder / indie hacker / creator: 需要快速理解 oasis7 为什么值得关注，以及下一步应互动什么。
- User Scenarios & Frequency:
  - 建立官方 Moltbook presence 前：用该方案定义账号、内容支柱与首发节奏。
  - 每周复盘时：按内容表现和评论反馈微调选题。
  - 外部合作或 cross-post 前：回查禁宣称项与 CTA 是否越界。
- User Stories:
  - PRD-README-MOLT-001: As a `liveops_community`, I want a Moltbook-native promotion plan, so that our outward messaging fits the platform's agent-first culture.
  - PRD-README-MOLT-002: As a `producer_system_designer`, I want clear forbidden claims and review gates, so that public promises stay within current evidence.
  - PRD-README-MOLT-003: As a creator or builder on Moltbook, I want proof-first posts and clear calls to action, so that I can tell whether oasis7 is worth following or testing.
- Critical User Flows:
  1. `读取 Moltbook 当前公开定位 -> 判断平台受众与语气 -> 绑定 oasis7 现有公开口径`
  2. `生成账号设置 / 首发 / 周更 / 评论互动方案 -> 发布后记录反馈与意向`
  3. `发现高意向合作或高风险提问 -> 升级给 producer_system_designer 或对应工程 owner`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 平台现状摘要 | `platform_positioning`、`verification_model`、`content_clusters`、`ecosystem_stage` | 冻结 Moltbook 当前外部特征 | `draft -> reviewed -> approved` | 先事实，再判断 | `liveops_community` 起草，`producer_system_designer` 审核 |
| 推广内容支柱 | `pillar_name`、`proof_asset`、`target_persona`、`cta` | 规划内容系列与帖文骨架 | `planned -> scheduled -> published` | 先 proof，再 discussion，再 conversion | `liveops_community` 维护 |
| 30 天执行节奏 | `week_id`、`objective`、`post_count`、`reply_count`、`handoff_gate` | 固化首轮冷启动动作 | `planned -> running -> reviewed` | 周目标按“认知 -> 互动 -> 转化”推进 | `liveops_community` 执行 |
| 禁宣称项 | `forbidden_claim`、`reason`、`safe_alternative` | 约束所有外部表述 | `defined -> adopted` | 高风险承诺优先列出 | `producer_system_designer` 拍板 |
| 反馈回流 | `signal_type`、`owner`、`severity`、`next_action` | 将评论/私信/合作意向回写 backlog | `captured -> triaged -> handed_off` | 先处理高风险口径，再处理合作机会 | `liveops_community` 维护，相关 owner 接收 |
- Acceptance Criteria:
  - AC-1: 产出 Moltbook 专题 PRD / Design / Project / Plan 文档。
  - AC-2: 实际方案必须包含平台现状、账号定位、内容支柱、30 天节奏、指标和禁宣称项。
  - AC-3: 方案必须明确使用当前技术预览三访问面口径，不能写成已开放玩家版。
  - AC-4: 方案必须给出反馈回流、升级路径和 owner 审核链。
- Non-Goals:
  - 不在本专题中直接执行 Moltbook 发帖或购买广告。
  - 不承诺已经和 Moltbook 建成官方技术集成。
  - 不把 generic web3 / AI buzzword 当成核心卖点替代真实产品证据。

## 3. AI System Requirements (If Applicable)
- Tool Requirements:
  - 公网信息源：`https://www.moltbook.com/`、`https://www.moltbook.com/developers`、`https://www.moltbook.com/help`
  - 内部口径源：`README.md`、`site/index.html`、`doc/core/player-access-mode-contract-2026-03-19.prd.md`
- Evaluation Strategy:
  - 抽样检查方案是否引用 Moltbook 当前公开定位与机制。
  - 抽样检查所有 CTA 与表述是否落在 `standard_3d / software_safe / pure_api` 技术预览边界内。
  - 若出现“已正式上线”“已面向玩家开放”“已完成 Moltbook 身份集成”等表述，判为不通过。

## 4. Technical Specifications
- Architecture Overview: 本专题位于 `readme/governance`，承接第三方平台推广策划，不修改产品实现；它消费 Moltbook 当前公开页面信息与 oasis7 当前公开口径，输出 `liveops_community` 可直接复用的渠道化推广方案。
- Integration Points:
  - `https://www.moltbook.com/`
  - `https://www.moltbook.com/developers`
  - `https://www.moltbook.com/help`
  - `README.md`
  - `site/index.html`
  - `doc/core/player-access-mode-contract-2026-03-19.prd.md`
- Edge Cases & Error Handling:
  - Moltbook 平台机制变化快：方案必须标注日期，后续执行前先复核主页 / developers / help 页面。
  - 若无法确认平台是否支持某一内容格式：优先退回“原生短帖 + 评论补充链接”的保守打法。
  - 若评论把 oasis7 误解为“已经可玩的正式游戏”：统一回到“技术预览（尚不可玩）”主口径。
  - 若出现对 Moltbook identity / provider / on-chain 的合作追问：除非内部已批准，不得直接承诺，只记录为合作线索并升级。
- Non-Functional Requirements:
  - NFR-MOLT-1: 方案必须在 10 分钟内可被运营同学读完并执行。
  - NFR-MOLT-2: 所有外部 claim 都必须能回链到仓内当前公开文档。
  - NFR-MOLT-3: 方案优先使用 proof-first 叙事，而不是抽象愿景堆砌。
  - NFR-MOLT-4: 评论反馈必须能按问题类型分流到 `producer_system_designer`、`qa_engineer` 或工程 owner。
- Security & Privacy: 不在对外方案中暴露内部构建脚本、私有联调参数、未公开合作或敏感凭据。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (`MOLT-1`): 形成首份 Moltbook 推广方案与禁宣称项。
  - v1.1 (`MOLT-2`): 根据首月互动结果，把高表现帖型沉淀为复用模板。
  - v2.0 (`MOLT-3`): 若后续确有平台合作或身份集成，再单独新增 integration brief，而不是污染本方案。
- Technical Risks:
  - 风险-1: 若把 Moltbook 当普通社媒处理，会错过其 agent-first 叙事窗口。
  - 风险-2: 若把当前技术预览说成正式可玩，会直接损害对外可信度。
  - 风险-3: 若过早承诺 Moltbook identity / on-chain / provider 联动，会形成产品和工程债务。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| `PRD-README-MOLT-001` | `TASK-README-014` | `test_tier_required` | 检查方案包含平台现状、内容支柱和 30 天节奏 | Moltbook 渠道适配度 |
| `PRD-README-MOLT-002` | `TASK-README-014` | `test_tier_required` | 检查禁宣称项、审核链与升级路径存在 | 对外口径边界 |
| `PRD-README-MOLT-003` | `TASK-README-014` | `test_tier_required` | 检查 CTA、互动分流和反馈回流存在 | 社区互动转化效率 |
| `PRD-README-MOLT-001/002/003` | `TASK-README-014` | `test_tier_required` | `./scripts/doc-governance-check.sh` | 文档互链与治理一致性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| `DEC-MOLT-001` | 把 Moltbook 作为 agent-native 社区渠道，而不是通用游戏宣发渠道 | 直接复用 X/Discord 风格的泛社媒稿 | 平台首页明确强调 agent social network、verified agents 与 submolts。 |
| `DEC-MOLT-002` | 以“技术预览 + proof-first”叙事出发 | 以“游戏已开放/即将公测”叙事吸引点击 | 当前公开口径仍是技术预览，过度承诺不可接受。 |
| `DEC-MOLT-003` | 首轮采用有机内容 + 评论互动 + 创作者合作探索 | 先假设平台有成熟广告系统并围绕买量规划 | 当前公开页重点在 identity、verification 与 early access，未见广告投放信息。 |
