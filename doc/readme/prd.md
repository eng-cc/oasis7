# readme PRD

审计轮次: 8

## 目标
- 建立 readme 模块设计主文档，统一需求边界、技术方案与验收标准。
- 确保 readme 模块后续改动可追溯到 PRD-ID、任务和测试。

## 范围
- 覆盖 readme 模块当前能力设计、接口边界、测试口径与演进路线。
- 覆盖 PRD-ID 到 `doc/readme/project.md` 的任务映射。
- 不覆盖实现代码逐行说明与历史过程记录。

## 接口 / 数据
- PRD 主入口: `doc/readme/prd.md`
- 项目管理入口: `doc/readme/project.md`
- 文件级索引: `doc/readme/prd.index.md`
- 追踪主键: `PRD-README-xxx`
- 测试与发布参考: `testing-manual.md`

## 里程碑
- M1 (2026-03-03): 完成模块设计 PRD 主体重写与任务改造。
- M2: 补齐模块设计验收清单与关键指标。
- M3: 建立 PRD-ID -> Task -> Test 的长期追踪闭环。

## 风险
- 模块边界演进快，文档同步可能滞后。
- 指标口径不稳定会降低验收一致性。
## 1. Executive Summary
- Problem Statement: README 与相关入口文档长期承载架构、运行、规则、发布口径，历史上容易出现口径漂移与链接失效。
- Proposed Solution: 将 readme 模块定义为“对外口径主控层”，统一入口信息、跨文档引用、术语定义与更新策略。
- Success Criteria:
  - SC-1: README 关键章节与模块 PRD 引用一致率达到 100%。
  - SC-2: 对外入口链接有效性检查持续通过。
- SC-3: 术语与架构描述变更在 1 个工作日内同步到 README 体系。
- SC-4: readme 相关变更全部具备 PRD-ID 与 devlog 追踪。
- SC-5: Closed beta candidate 的 liveops runbook与模板能直接维持 `limited playable technical preview` 口径，并服务 `prg-game-009` 的 evidence gate。
- SC-6: limited preview 贡献奖励流程必须具备可直接复用的 round ledger 模板，能够承接评分、审批、发放记录与归档。

## 2. User Experience & Functionality
- User Personas:
  - 新贡献者：需要快速理解系统边界与入口。
  - 外部评审者：需要准确获取当前实现状态与能力。
  - 维护者：需要低成本维护跨文档一致性。
- User Scenarios & Frequency:
  - 新人入项阅读：入项首日必读，建立整体认知。
  - 对外评审准备：每次外部评审前执行入口核对。
  - 文档同步巡检：每周至少 1 次。
  - 发布前口径复核：每个版本候选至少 1 次。
- User Stories:
  - PRD-README-001: As a 新贡献者, I want a reliable top-level narrative, so that onboarding time is reduced.
  - PRD-README-002: As an 评审者, I want consistent architecture statements, so that technical due diligence is faster.
  - PRD-README-003: As a 维护者, I want explicit sync rules, so that docs do not drift.
- PRD-README-004: As a `liveops_community`, I want an external communication brief anchored to internal release evidence, so that public-facing messaging stays consistent with current candidate status and risk boundaries.
- PRD-README-005: As a `liveops_community`, I want a reusable release communication template, so that future candidate briefs follow the same structure, evidence links, and review chain.
- PRD-README-006: As a `liveops_community`, I want an announcement/changelog draft derived from approved messaging, so that formal external copy can start from a safe, audited baseline.
- PRD-README-007: As a `liveops_community`, I want a reusable announcement/changelog template, so that future external drafts follow the same sections, source links, and review states.
- PRD-README-008: As a 仓库访客, I want the root README to reflect the current preview posture, so that I do not mistake the repo for a live release landing page.
- PRD-README-009: As a `producer_system_designer`, I want repo-home copy aligned with site and communication docs, so that public promises stay consistent.
- PRD-README-010: As a `liveops_community`, I want platform-specific promotion plans anchored to third-party channel mechanics and internal claim envelopes, so that outbound seeding can fit channel culture without over-promising.
- PRD-README-011: As a `liveops_community`, I want a first-wave Moltbook post pack derived from the approved platform plan, so that we can publish native posts and replies without improvising unsafe copy.
- PRD-README-012: As a `liveops_community`, I want Moltbook outreach to point builders back to GitHub issues and PRs after they inspect the preview, so that external interest can turn into actionable feedback and contributions.
- PRD-README-013: As a `liveops_community`, I want shorter feed-native Moltbook variants of the approved post pack, so that final publish copy reads like native posts instead of internal drafts.
- PRD-README-014: As a 仓库访客, I want the repo root and public entry docs to use the canonical `oasis7` brand while still explaining legacy internal identifiers, so that I do not confuse project branding with crate/bin compatibility names.
- PRD-README-015: As a `liveops_community`, I want a Moltbook liveops runbook for post-publish checks, replies, and signal triage, so that day-2 channel operations do not fall back to unsafe improvisation.
- PRD-README-016: As a `liveops_community`, I want a first-week Moltbook operating template layered onto the runbook, so that the first 7 days of channel activity have a concrete daily rhythm instead of generic SOP only.
- PRD-README-017: As a `liveops_community`, I want a closed beta candidate runbook plus feedback/incident templates, so that recruitment, feedback, and incident signals stay within the technical preview envelope while still feeding the unified release gate.
- PRD-README-018: As a `liveops_community`, I want real Moltbook execution lessons written back into the runbook, so that future posts can reuse what triggered discussion and avoid what triggered spam.
- PRD-README-019: As a `liveops_community`, I want a controlled limited preview execution pack, so that the first invite-only builder round can run with fixed callout copy, monitoring slots, signal buckets, and producer-facing summary fields.
- PRD-README-020: As a `liveops_community`, I want an early contributor reward pack for limited preview, so that reward eligibility, evidence fields, score bands, and forbidden phrases stay contribution-based and do not depend on invite-only or play-to-earn framing.
- PRD-README-021: As a `liveops_community`, I want a reusable Xiaohongshu intro post pack from the human developer perspective, so that the account can open with a clear identity, safe tone, and repeatable asset set instead of improvising its first post.
- PRD-README-022: As a `liveops_community`, I want a round-based contributor reward ledger template, so that real limited preview contributions can be reviewed, approved, distributed, and archived without falling back to ad-hoc notes.
- PRD-README-023: As a `liveops_community`, I want a reusable Xiaohongshu team-roster post pack, so that the second post can explain the current agent team structure with human-facing clarity instead of drifting into either dry role docs or vague AI talk.
- PRD-README-024: As a `liveops_community`, I want a Xiaohongshu liveops runbook aligned with the Moltbook model, so that human-facing channel operations can reuse a stable SOP for post checks, reply boundaries, interaction prompts, and signal feedback loops.
- PRD-README-025: As a `liveops_community`, I want a reusable Xiaohongshu game-intro post pack, so that the third post can explain what kind of game `oasis7` is in human-facing language without collapsing into either a full world-rule dump or unsafe release claims.
- PRD-README-026: As a `liveops_community`, I want a reusable Xiaohongshu player-boundary post pack, so that the fourth post can explain why players guide the world instead of directly puppeting one role, without slipping into either control-scheme jargon or unsafe playability claims.
- PRD-README-027: As a `liveops_community`, I want a reusable Xiaohongshu AI-laziness-to-game-mode post pack, so that the fifth post can start from a familiar AI-era feeling and pull the discussion toward what games should still ask humans to judge, choose, and carry.
- PRD-README-028: As a `liveops_community`, I want a reusable Xiaohongshu spring-recruit post pack from the studio lead perspective, so that we can borrow current job-market anxiety to talk about judgment, delivery, and player sense without drifting into either generic job-board content or hollow motivation copy.
- PRD-README-029: As a `liveops_community`, I want a reusable Xiaohongshu spring-recruit carousel pack derived from the approved long-form post, so that the same topic can publish as a mobile-native swipe format with stronger retention and comment prompts instead of only one long caption block.
- PRD-README-030: As a `liveops_community`, I want a reusable Xiaohongshu AI-persona-vs-world-actor post pack, so that we can borrow the current `AI人格` discussion heat to explain why `oasis7` wants agents that can act inside the world instead of only sounding like companions, without drifting into companion-product framing or unsafe release claims.
- PRD-README-031: As a `liveops_community`, I want a reusable Xiaohongshu AI-persona carousel pack derived from the approved seventh post, so that the same topic can publish as a 4-page mobile-native swipe deck with clearer pauses between “会聊天”“会行动”“如何判断” and the final comment hook, instead of only one cover plus long caption.
- Critical User Flows:
  1. Flow-RM-001: `阅读 README -> 跳转模块入口 -> 快速定位目标能力`
  2. Flow-RM-002: `检测口径变更 -> 更新入口文档 -> 校验链接 -> 发布同步`
  3. Flow-RM-003: `发布前执行巡检 -> 汇总冲突 -> 修复后复核`
  4. Flow-RM-004: `读取第三方平台当前机制 -> 绑定内部 claim envelope -> 生成平台适配推广方案 -> 回流 owner 审核`
  5. Flow-RM-005: `发布 Moltbook 帖子 -> 检查 /home / notifications / comments -> 分级回复或升级 -> 回写 devlog`
  6. Flow-RM-006: `按首周模板安排 day1-day7 发帖 / 巡检 / 跟评 / 周复盘 -> 将真实信号沉淀到 runbook 与 devlog`
  7. Flow-RM-007: `复盘真实帖子表现 -> 提炼有效讨论钩子与 spam 触发条件 -> 回写 runbook -> 调整下一帖策略`
  8. Flow-RM-008: `冻结 limited preview 口径 -> 选用 invite-only callout copy -> 按固定窗口巡检 -> 将信号按 Blocking / Opportunity / Idea 归档 -> 输出 producer 摘要`
  9. Flow-RM-009: `收集 early contributor signal -> 按评分模板记录证据 -> 输出 small/medium/large reward recommendation -> 检查对外禁语 -> 回流 producer 审核`
  10. Flow-RM-010: `确定小红书账号第一帖定位 -> 冻结标题/正文/封面/标签 -> 对齐“人类开发者 + agent 队友”叙事主语 -> 保存可复用素材包`
  11. Flow-RM-011: `确定第二篇队友介绍轮播结构 -> 为每位 agent 收口一句专业但有人味的角色说明 -> 产出轮播卡与可截图 HTML`
  12. Flow-RM-012: `结束一轮 limited preview -> 将可计分贡献抄入 reward ledger -> producer 审批档位 -> execution owner 回填发放记录 -> 台账归档`
  13. Flow-RM-013: `发布小红书内容 -> 按 T+15m / T+1h / T+4h / T+24h 巡检 -> 按状态误解 / 过度猜想 / 高质量互动分级 -> 回流下篇选题`
  14. Flow-RM-014: `确定第三篇“游戏是什么”表达 -> 冻结标题/正文/轮播页/互动问题 -> 用人类开发者口吻给出可想象的游戏轮廓 -> 保持 limited playable technical preview 边界`
  15. Flow-RM-015: `确定第四篇“玩家为什么不能直接控制角色”表达 -> 冻结标题/正文/轮播页/互动问题 -> 用人类开发者口吻解释玩家与 agent 的控制边界 -> 保持 limited playable technical preview 边界`
  16. Flow-RM-016: `确定第五篇“AI时代，你变"懒"了么”表达 -> 从熟悉的 AI 使用感切入 -> 收口到“游戏模式里什么还该交给人” -> 冻结标题/正文/轮播页/互动问题并避免滑向泛 AI 焦虑帖`
  17. Flow-RM-017: `确定第六篇“作为游戏工作室主理人，今年的春招视角”表达 -> 借春招/求职焦虑框架讲团队判断标准 -> 冻结标题/正文/封面/互动问题并避免滑向岗位汇总或空泛鸡汤；若平台标题上限受限，优先保证“游戏工作室主理人 + 春招”识别点`
  18. Flow-RM-018: `将第六篇长文版压成轮播版 -> 拆成封面/判断前提/核心标准/收束提问 6 页 -> 导出逐页 PNG -> 保持标题、关键词和互动问题一致`
  19. Flow-RM-019: `确定第七篇“AI人格很火，但我不想做陪聊搭子”表达 -> 借 AI人格 热词讲清“人格不只是会聊天，而是会在世界里行动、判断、协作和承担后果” -> 冻结标题/正文/互动问题并避免滑向情感陪伴产品讨论或完整可玩承诺`
  20. Flow-RM-020: `将第七篇长文版压成 4 页轮播版 -> 拆成封面/核心判断/行为例子/评论区站队页 -> 导出逐页 PNG -> 保持标题、边界与互动问题一致`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 顶层入口导航 | 模块名称、入口链接、摘要 | 点击跳转模块文档 | `draft -> published -> refreshed` | 入口按模块矩阵排序 | 所有人可读，维护者可更新 |
| 口径一致性同步 | 术语、架构描述、更新时间 | 检测冲突并回写更新 | `detected -> synced -> verified` | 核心入口优先同步 | 文档 owner 审核生效 |
| 链接可用性巡检 | 链接地址、状态、修复建议 | 自动检查并输出报告 | `checked -> broken/fixed` | 断链优先修复 | 维护者可处理 |
| 平台化推广方案 | `platform`、`audience`、`content_pillars`、`claim_boundary`、`cta`、`signal_tags` | 生成渠道适配的推广/运营方案 | `draft -> reviewed -> approved` | 先写平台机制，再写口径边界，再写动作节奏 | `liveops_community` 起草，`producer_system_designer` 审核 |
| 渠道运营 runbook | `check_window`、`signal_bucket`、`reply_boundary`、`escalation_owner`、`log_requirement` | 固化第三方渠道发帖后运营 SOP | `draft -> approved -> adopted` | 先定义检查顺序，再定义回复与升级边界 | `liveops_community` 维护，`producer_system_designer` 审核边界 |
| 首周运营模板 | `day_id`、`primary_post`、`check_slots`、`reply_goal`、`log_focus` | 把抽象 runbook 压实到 day1-day7 的执行模板 | `planned -> executed -> reviewed` | 先 identity / surfaces，再 proof / diary / builder hook，再 recap | `liveops_community` 执行，`producer_system_designer` 审核边界 |
| Closed Beta Candidate Runbook | `candidate_signal`、`release_gate_link`、`response_template`、`incident_level` | 招募、反馈、事故模板、FAQ | `tech_preview -> candidate_runbook -> gate_ready` | runbook步骤优先，监测其次 | `liveops_community` 维护，`producer_system_designer` 决定口径 |
| 实战运营经验 | `post_id`、`working_hook`、`spam_trigger`、`next_adjustment` | 把真实发帖结果沉淀成后续可复用规则 | `observed -> distilled -> adopted` | 先记录高信号模式，再记录高风险模式 | `liveops_community` 维护 |
| 受控预览执行包 | `round_id`、`callout_copy`、`check_slot`、`signal_bucket`、`claim_drift_flag`、`summary_field` | 把 limited preview 第一轮执行压成可直接照跑的操作包 | `draft -> execution_ready -> reused` | 先冻结文案，再冻结巡检，再冻结回流摘要 | `liveops_community` 维护，`producer_system_designer` 审核边界 |
| 早期贡献奖励操作包 | `contribution_type`、`score_band`、`evidence_field`、`reward_recommendation`、`forbidden_phrase` | 按贡献评分模板判断是否进入奖励建议池，并约束对外表达 | `signal -> scored -> reviewed -> recommended` | 不公布固定 token/point 比率；仅按贡献审计后给 `eligible-small/medium/large` 建议 | `liveops_community` 记录与初评，`producer_system_designer` 最终审批 |
| 贡献奖励台账 | `round_id`、`candidate_id`、`ledger_id`、`reward_account`、`recommended_band`、`review_status`、`approval_id`、`distribution_ref` | 将真实贡献逐条编目、审批、回填发放记录并归档 | `draft -> reviewed -> approved/rejected -> distributed -> archived` | 同一轮按 contributor 与 ledger id 去重；缺证据默认不得进入审批 | `liveops_community` 维护，`producer_system_designer` 审核，execution owner 回填发放记录 |
| 小红书帖子素材包 | `post_goal`、`title`、`body`、`cover_copy`、`carousel_outline`、`interaction_prompt`、`forbidden_phrase` | 固化单篇小红书可复用文案与轮播结构 | `draft -> reviewed -> ready_for_publish -> published` | 先确定单一帖子目标，再冻结标题/正文/互动问题；正文先给人可理解轮廓，再避免 world-rule dump 与上线口径 | `liveops_community` 起草，`producer_system_designer` 审核边界 |
- Acceptance Criteria:
  - AC-1: readme PRD 明确入口文档职责边界。
  - AC-2: readme project 文档维护同步任务与状态。
  - AC-3: README 与 `world-rule.md`、`testing-manual.md`、模块 PRD 的链接链路可用。
  - AC-4: 口径更新有明确触发条件与同步时限。
  - AC-5: 渠道化推广方案必须显式绑定内部公开口径边界，不得把 generic marketing 文案直接外推到第三方平台。
  - AC-6: `doc/readme/governance/**` 仍可读历史专题的首行标题必须统一使用 `oasis7` 品牌；旧 `oasis7*` 标题仅允许保留在正文历史上下文与证据原文中。
  - AC-7: `doc/readme/governance/**` 中仍作为当前公开口径使用的项目名必须统一写为 `oasis7`；旧 `oasis7` 仅允许保留在历史证据、兼容说明或外部原文引用中。
- AC-8: 若第三方渠道进入持续运营阶段，必须补齐独立 runbook，明确巡检入口、回复边界、升级路径与 `devlog` 回写方式。
- AC-9: 若渠道进入首周冷启动执行阶段，runbook 必须补齐 day1-day7 模板，明确每天的主动作、检查窗口、回复目标与记录重点。
- AC-10: 已新建 `closed beta candidate` runbook与 incident template，供招募/反馈/事故信号在 `limited playable technical preview` claim envelope 内沟通并可直接回流 unified release gate。
  - AC-11: Moltbook runbook 至少记录一轮真实执行后的“有效讨论钩子”和“高风险 spam 触发模式”，并明确下一轮如何调整。
- AC-12: 若团队进入 `limited playable technical preview` 的 invite-only 执行阶段，必须补齐受控执行包，明确 callout 文案、巡检窗口、信号分桶、claim drift 纠偏与 producer 摘要字段。
- AC-13: 若团队决定在 limited preview 阶段使用 early contributor reward，必须补齐独立操作包，明确评分模板、证据字段、审阅链、禁语与“无固定 token/point 汇率”边界。
- AC-14: 若团队开始记录真实贡献奖励轮次，必须补齐 round-based ledger，明确 round meta、逐条 row 状态、producer 审批引用、distribution ref 与 archive note。
- AC-15: 若小红书内容链路进入第四篇，必须补齐“玩家控制边界”素材包，明确标题、正文、轮播结构、互动问题，并把“玩家不能直接控制角色，但能给方向”的正式口径收进人类开发者叙事。
- AC-16: 若小红书内容链路进入第五篇，必须补齐“AI时代，你变"懒"了么”素材包，明确如何从 AI 使用习惯切到游戏模式讨论，并保持人类开发者视角与 `limited playable technical preview` 边界。
- AC-17: 若小红书内容链路进入第六篇，必须补齐“作为游戏工作室主理人，今年的春招视角”素材包，明确标题、正文、封面、互动问题、关键词与禁滑坡边界，并把春招热点收口到团队判断、完成度、AI 使用方式与玩家感觉；若平台标题长度受限，最终发布标题必须优先保留“游戏工作室主理人”和“春招”识别点。
- AC-18: 若第六篇需要轮播版，必须补齐独立轮播素材包，明确页数、逐页文案、HTML、逐页 PNG 与评论区收束页分工，并保持与长文版标题、边界和互动问题一致。
- AC-15: 若小红书进入“开始解释游戏是什么”的第三帖阶段，必须补齐独立素材包，明确标题、正文、轮播结构、互动问题与“不能写成完整设定说明书/不能暗示已上线”的边界。
- Non-Goals:
  - 不在 readme PRD 中替代各模块详细设计。
  - 不在 readme PRD 中定义测试用例细节。
  - 不在 readme PRD 中直接执行第三方平台广告采买或投放。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 文档链接检查、术语一致性校验、入口巡检脚本。
- Evaluation Strategy: 以链接可用率、口径冲突数、修复时长、评审返工率评估。

## 4. Technical Specifications
- Architecture Overview: readme 模块属于文档入口层，负责跨模块信息汇总、术语统一和导航稳定性。
- Integration Points:
  - `README.md`
  - `world-rule.md`
  - `testing-manual.md`
- `doc/README.md`
  - `doc/readme/governance/readme-limited-preview-contributor-reward-pack-2026-03-22.prd.md`
  - `doc/readme/governance/readme-limited-preview-contributor-reward-ledger-2026-03-22.prd.md`
  - `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.prd.md`
- Edge Cases & Error Handling:
  - 链接失效：断链必须在巡检报告中暴露并进入修复队列。
  - 口径冲突：冲突出现时禁止发布“已同步”状态。
  - 空入口：模块入口缺失时标记高优告警并补齐跳转。
  - 权限不足：非维护者不得直接修改对外核心描述。
  - 并发编辑：同文件并发更新时需合并后重跑链接检查。
  - 历史重定向：legacy redirect 必须保留指向并声明主入口。
  - 过度承诺：若对外文案把贡献奖励写成 `play-to-earn`、`airdrop for players`、`just play and earn token`，必须阻断发布。
- Non-Functional Requirements:
  - NFR-RM-1: 顶层入口链接可用率 100%。
  - NFR-RM-2: 术语冲突修复 SLA <= 1 个工作日。
  - NFR-RM-3: README 与模块 PRD 关键引用一致率 100%。
  - NFR-RM-4: 发布前口径巡检覆盖率 100%。
  - NFR-RM-5: 对外文档不得暴露敏感配置信息。
  - NFR-RM-6: 早期贡献奖励模板不得公开固定 token 数额、固定 token/point 比率或“玩多久给多少”的承诺。
  - NFR-RM-7: 真实贡献奖励 ledger 中每条非拒绝记录必须至少有 1 个有效 source/evidence link，且每条已发放记录都必须具备 `Approval ID` 与 `Distribution Ref`。
- Security & Privacy: 对外文档不得暴露敏感配置与密钥信息；示例配置需使用脱敏样例。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (2026-03-03): 固化 README 入口职责与同步流程。
  - v1.1: 增加自动化链接/术语巡检任务。
  - v2.0: 建立入口文档质量趋势指标（漂移率、修复时长）。
- Technical Risks:
  - 风险-1: 高频变更导致跨文档同步延迟。
  - 风险-2: 大范围重构时导航信息失真。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-README-001 | TASK-README-001/002/005 | `test_tier_required` | 入口结构与导航可达检查 | 新人入项与外部阅读体验 |
| PRD-README-002 | TASK-README-002/003/005 | `test_tier_required` | 架构口径一致性与链接巡检 | 技术评审效率与准确性 |
| PRD-README-003 | TASK-README-003/004/005 | `test_tier_required` | 同步流程与修复节奏抽样复核 | 文档长期稳定性 |
| PRD-README-004 | TASK-README-006 | `test_tier_required` | 对外口径简报、禁用表述与回滚口径抽样复核 | 版本候选外部沟通一致性 |
| PRD-README-005 | TASK-README-007 | `test_tier_required` | 对外口径模板、evidence-link 字段与审批链抽样复核 | 版本候选口径模板复用性 |
| PRD-README-006 | TASK-README-008 | `test_tier_required` | 公告 / changelog 底稿、draft 状态与 FAQ 结构抽样复核 | 对外发布底稿一致性 |
| PRD-README-007 | TASK-README-009 | `test_tier_required` | 公告模板、source links 与 review status 抽样复核 | 公告底稿模板复用性 |
| PRD-README-008 | TASK-README-010 | `test_tier_required` | 根 README 状态段含技术预览 / 尚不可玩 / 公告准备态 | 仓库首页状态理解 |
| PRD-README-009 | TASK-README-010 | `test_tier_required` | README 与 site / brief 口径一致 | 公开口径一致性 |
| PRD-README-010 | TASK-README-014 | `test_tier_required` | Moltbook 推广方案含平台现状、内容支柱、节奏、禁宣称项与回流机制 | 第三方渠道推广口径一致性 |
| PRD-README-011 | TASK-README-015 | `test_tier_required` | Moltbook 帖文包含首批主贴、评论模板、CTA 与禁宣称抽样复核 | 首批渠道文案执行安全性 |
| PRD-README-012 | TASK-README-016 | `test_tier_required` | Moltbook 方案与帖文明确 GitHub issue / PR CTA，且不与技术预览边界冲突 | 外部反馈回流与贡献转化 |
| PRD-README-013 | TASK-README-017 | `test_tier_required` | Moltbook 帖文主贴已压缩为更短的 feed-native 版本，且保留技术预览与 GitHub CTA 边界 | 最终发布文案可用性 |
| PRD-README-014 | TASK-README-018/019/020 | `test_tier_required` | 根 README、公开入口文档与 `doc/readme/governance/**` 的历史标题/当前公开口径统一为 `oasis7`，并显式区分内部兼容命名 | 仓库首页认知、公开品牌一致性 |
| PRD-README-015 | TASK-README-024 | `test_tier_required` | Moltbook runbook 明确发帖前、发帖后 24h、常规日与周复盘动作，并包含回复边界、升级路径与 `devlog` 回写要求 | 第三方渠道持续运营一致性 |
| PRD-README-016 | TASK-README-025 | `test_tier_required` | Moltbook runbook 追加 day1-day7 首周模板，覆盖主帖节奏、检查窗口、回复目标与日志重点 | 第三方渠道冷启动执行性 |
| PRD-README-017 | TASK-README-026 | `test_tier_required` | Closed beta candidate runbook + incident templates cover recruitment, feedback, and incident guardrails | Closed beta candidate recruiting/feedback/technical preview messaging |
| PRD-README-018 | TASK-README-027 | `test_tier_required` | Moltbook runbook 回写真实运营经验，明确哪些内容设计更易引发讨论、哪些自评动作更易触发 spam | 第三方渠道运营复用性与风控 |
| PRD-README-019 | TASK-README-029 | `test_tier_required` | invite-only limited preview execution pack 明确 callout copy、check slots、signal buckets、summary fields | 受控预览执行性与回流一致性 |
| PRD-README-020 | TASK-README-030 | `test_tier_required` | early contributor reward pack 明确评分模板、证据字段、奖励建议分级与禁语清单 | limited preview 贡献奖励执行性与对外口径安全性 |
| PRD-README-021 | TASK-README-031 | `test_tier_required` | 小红书首帖素材包明确标题、正文、封面文案、标签与“人类开发者视角”使用说明 | 新渠道冷启动识别度与口径稳定性 |
| PRD-README-022 | TASK-README-032 | `test_tier_required` | reward ledger 模板明确 round meta、逐条贡献记录、producer 审批、distribution ref 与归档字段 | limited preview 真实贡献奖励结算闭环 |
| PRD-README-023 | TASK-README-033 | `test_tier_required` | 小红书第二帖素材包明确 7 位 agent 队友卡文案、收束页与可截图 HTML | 渠道内容连续性与“agent 队伍”概念可理解性 |
| PRD-README-024 | TASK-README-034 | `test_tier_required` | 小红书 runbook 明确发帖前复核、发帖后 24h 巡检、评论分级、互动引导和 `devlog` 回写要求 | 人类向渠道的持续运营一致性 |
| PRD-README-025 | TASK-README-035 | `test_tier_required` | 小红书第三帖素材包明确“游戏是什么”的标题、正文、轮播结构、互动问题与技术预览边界 | 渠道内容从“谁在做”过渡到“在做什么游戏”的可理解性 |
| PRD-README-026 | TASK-README-036 | `test_tier_required` | 小红书第四帖素材包明确“玩家为什么不能直接控制角色”的标题、正文、轮播结构、互动问题与控制边界口径 | 渠道内容从“游戏是什么”过渡到“玩家如何介入世界”的可理解性 |
| PRD-README-027 | TASK-README-037 | `test_tier_required` | 小红书第五帖素材包明确“AI时代，你变"懒"了么”的标题、正文、轮播结构、互动问题与游戏模式讨论收口 | 渠道内容从“玩家怎么介入”过渡到“AI时代下游戏应该把什么留给人”的可讨论性 |
| PRD-README-028 | TASK-README-038/042 | `test_tier_required` | 小红书第六篇素材包明确“作为游戏工作室主理人，今年的春招视角”的最终发布标题、正文、封面文案、互动问题与热点借势边界 | 渠道内容从游戏判断延展到团队判断与 AI 时代用人标准的可讨论性 |
| PRD-README-029 | TASK-README-039 | `test_tier_required` | 小红书第六篇轮播版素材包明确逐页文案、HTML、逐页 PNG 与评论区收束页 | 第六篇从长文版扩展到 feed-native 轮播版的可发布性 |
| PRD-README-030 | TASK-README-044 | `test_tier_required` | 小红书第七篇素材包明确“AI人格很火，但我不想做陪聊搭子”的标题、正文、互动问题、关键词与“人格 = 会在世界里行动”边界 | 渠道内容从泛 AI 人格讨论重新收束到 `oasis7` 的 agent 设计判断与世界内行动逻辑 |
| PRD-README-031 | TASK-README-046/047/048/049 | `test_tier_required` | 小红书第七篇轮播版素材包明确 4 页逐页文案、HTML、逐页 PNG 与评论区收束页，并完成前两页重叠修复、后两页案例页/收束页美术收口，以及整组从“编辑部海报感”继续收口到更接近小红书原生图文卡片的发布视觉 | 第七篇从单图封面 + 长文版扩展到更适合小红书 feed 的轮播版发布形态，同时保证整组轮播的视觉质量、手机端读感与平台原生感 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-RM-001 | README 作为对外入口主控层 | 各模块对外独立叙述 | 容易产生口径漂移。 |
| DEC-RM-002 | 入口链接定期自动巡检 | 仅人工抽查 | 自动巡检可降低断链遗漏率。 |
| DEC-RM-003 | 口径更新设置同步时限 | 无明确时限 | 时限可提高协作可预测性。 |
| DEC-RM-004 | 候选级对外说明先走简报专题，再决定是否升级到 README / 公告 | 直接在 README 或外部公告里写最终口径 | 先用简报控边界，能避免对外承诺过早固化。 |
| DEC-RM-005 | 用模板沉淀 release communication 结构 | 每次按临时文案自由发挥 | 模板能提高后续候选的复用率与审阅效率。 |
| DEC-RM-006 | 在简报之后先落 announcement/changelog draft，再决定是否正式发布 | 直接将简报对外公开 | 底稿更接近外部文风，同时仍保留 draft 审核缓冲层。 |
| DEC-RM-007 | 将 announcement/changelog 底稿继续模板化 | 每个候选手写公告底稿 | 模板化能降低重写成本，并稳定审核结构。 |
| DEC-RM-008 | 根 README 只对齐状态段，不重写整份首页 | 为修正口径重做全部 README 文案 | 最小变更即可消除仓库首页与 site 的状态分叉。 |
| DEC-RM-009 | 用平台专题文档承接第三方渠道推广方案 | 直接复用一份通用社媒文案覆盖全部平台 | 第三方平台机制和社区文化差异明显，必须保留渠道化约束。 |
| DEC-RM-010 | 在渠道方案之后继续沉淀首批发帖草案与回复模板 | 真实发帖时临场手写文案 | 提前冻结主贴与回复模板，能显著降低 overclaim 风险。 |
| DEC-RM-011 | 将 GitHub issue / PR 作为 Moltbook builder CTA 的正式回流出口 | 只引导关注或私信，不给公开协作入口 | 仓库已有公开协作面，显式回流更利于把外部兴趣转成可追踪反馈。 |
| DEC-RM-012 | 在发布前把首批帖文继续压缩成 feed-native 短版 | 保留偏内部草案长度直接外发 | Moltbook 更适合短、硬、单一 CTA 的原生内容。 |
| DEC-RM-013 | 对外品牌统一为 `oasis7`，内部 crate/bin 暂保留兼容命名 | 同一轮同时重命名全部 crate/bin/script/env 标识 | 先统一用户可见品牌与下载入口，避免把品牌收口与大规模兼容迁移耦合。 |
| DEC-RM-014 | 将 Moltbook 日常运营动作沉淀为独立 runbook，而不是继续扩写角色卡 | 把巡检、回复、升级细节继续堆进角色卡或推广方案 | 角色卡应保持稳定职责边界；执行细节更适合 runbook 持续演进。 |
| DEC-RM-015 | 在已有 Moltbook runbook 内补首周运营模板，而不是再拆一份平行文档 | 单独再建一份“week-one playbook” | 首周模板属于 runbook 的执行层细化，放在同一文档更利于维护与回查。 |
| DEC-RM-016 | 新增 `PRD-README-017` 作为 closed beta candidate runbook专题 | 继续把 closed beta 运营信号写回 devlog / single thread | 独立 runbook 强制维持 `technical preview` 口径，避免提前升级 release claim。 |
| DEC-RM-017 | 将 Moltbook 实战复盘写回现有 runbook | 只在 devlog 留一次性记录 | runbook 才是后续运营会重复翻看的入口。 |
| DEC-RM-018 | 为 invite-only limited preview 新增独立执行包 | 继续只靠 handoff 和零散 devlog 驱动首轮外放 | 首轮 limited preview 需要固定文案、巡检窗口、分桶与摘要字段，才能避免执行漂移。 |
| DEC-RM-019 | 早期奖励模板只输出贡献评分与奖励建议档位，不公开固定 token 数额或 point 汇率 | 直接把 token 发放表做成外部宣传文案 | 当前阶段仍是 `limited playable technical preview`，需要避免过度金融化与过度承诺。 |
| DEC-RM-020 | 小红书首帖采用“人类开发者自我介绍”而非“项目功能介绍” | 首帖直接解释完整世界观或产品能力 | 小红书面向人，首帖更需要建立创作者身份与协作关系，而不是先堆产品信息。 |
| DEC-RM-021 | 第二篇采用“团队 roster 轮播卡”而非长文解释 | 继续用长文解释 agent 协作或直接贴角色职责原文 | 用户第二篇更需要快速理解“有哪些队友、为什么重要”，轮播卡比长文更适合这一层信息。 |
| DEC-RM-022 | 用 round-based reward ledger 承接真实贡献奖励结算 | 继续靠 issue 评论、聊天记录或零散表格临场结算 | 真实发放前必须把评分、审批、执行引用和归档收进统一模板，才能保持可审计性。 |
| DEC-RM-023 | 小红书持续运营细节独立沉淀为 runbook，而不是继续只留在素材包和角色卡 | 只补几条角色卡示例或继续靠帖子素材包驱动 | 小红书已经进入持续发帖和看反馈阶段，需要和 Moltbook 一样有稳定 SOP，才能复盘互动和误解模式。 |
| DEC-RM-024 | 第三篇采用“轻量游戏介绍 + 猜类型互动”而非完整设定说明书 | 直接把世界规则、技术架构与完整玩法一次讲清 | 用户到了第三篇需要先建立“这是什么游戏”的可想象轮廓，而不是被文档级信息密度劝退；同时仍要保持技术预览边界。 |
| DEC-RM-025 | 第四篇采用“玩家控制边界解释 + 站队式互动”而非继续泛讲世界设定 | 跳过玩家位置直接讲更多系统细节，或把第四篇写成输入操作说明 | 第三篇之后最自然的追问是“玩家到底怎么介入这个世界”；先把控制边界讲清，才能避免用户把项目误解成直接操控单角色的传统玩法。 |
| DEC-RM-026 | 第五篇采用“AI 使用感受 -> 游戏模式判断”而非直接泛聊 AI 让人变懒 | 只发一条泛 AI 焦虑/效率感想，或直接转去讲 `oasis7` 具体机制 | “AI时代，你变"懒"了么”本身容易引讨论，但如果不收口会太泛；把它拉回“游戏还该把什么留给人”更贴账号主线，也更能承接项目设计判断。 |
| DEC-RM-027 | 第六篇采用“春招判断标准”而非“岗位清单/求职资料” | 直接做免笔试汇总、岗位投递清单，或输出泛求职鸡汤 | 春招话题有热度，但账号主线仍是人类开发者视角下的团队判断与项目收口；借热点讲“我会看什么人”更贴现有内容链，也更容易带出高质量讨论。 |
| DEC-RM-028 | 第六篇轮播版采用“6 页短判断卡”而非把长文硬切成多页 | 直接把长文等分成 4-8 页纯文字截图 | 小红书轮播更吃“每页只讲一个判断”的手机端节奏；重新拆页比机械分段更利于读完率、收藏和评论承接。 |
| DEC-RM-029 | 第七篇采用“AI人格热词 + 不做陪聊搭子”的立场切入，而不是顺着热点泛聊情感陪伴或做 AI 产品点评 | 直接做“AI人格是什么”的抽象讨论，或把帖子写成陪伴型 AI 产品比较/安利 | `AI人格` 的平台热度足够高，但账号主线仍是游戏与 agent 设计判断；先用强立场把注意力拉住，再迅速收回“放进世界后会不会行动与承担后果”，更符合 `oasis7` 现有内容链。 |
| DEC-RM-030 | 第七篇轮播版采用“4 页分割判断卡”而非继续只发单图封面或把长文整段截图切成多页 | 维持仅有封面图+长文；或直接把长文逐段截成 4-6 页纯文字卡 | 这一篇最强的传播点在“会聊天 vs 会行动”的对照。把它压成 4 页，每页只停在一个判断，更利于 feed 内读完率和评论区站队。 |
