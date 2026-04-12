# readme PRD 文件级索引

审计轮次: 12

更新时间：2026-04-12

## 入口
- 模块 PRD：`doc/readme/prd.md`
- 模块设计总览：`doc/readme/design.md`
- 模块标准执行入口：`doc/readme/project.md`
- 当前高频 liveops 入口：`doc/readme/governance/readme-moltbook-liveops-runbook-2026-03-21.prd.md`

## 首读分流
- 想先回答 README 模块在管什么、哪些内容属于正式对外口径：先读 `doc/readme/prd.md`
- 想先回答当前在推进什么、哪些治理或运营专题仍是 active：先读 `doc/readme/project.md`
- 想直接进入高频渠道运营入口：先读 `doc/readme/governance/readme-moltbook-liveops-runbook-2026-03-21.prd.md`、`doc/readme/governance/readme-xiaohongshu-liveops-runbook-2026-03-23.md` 与 `doc/readme/governance/readme-xiaohongshu-wechat-promoter-oasis-coin-incentive-pack-2026-04-12.md`
- 想继续按子域或文件名下钻：使用下方热点子域导航，再跳到对应清单区域

## 密度快照（2026-04-12）
- `doc/readme/`：137 份文件
- `doc/readme/governance/`：93 份文件
- `doc/readme/gap/`：27 份文件
- `doc/readme/production/`：12 份文件

## 热点子域导航
| 子域 | 文件数 | 适合回答的问题 |
| --- | --- | --- |
| `governance/` | 93 | 根 README 对齐、release communication、Moltbook/Xiaohongshu runbook、invite/reward/material/execution 入口 |
| `gap/` | 27 | README 与实现/流程之间还有哪些正式缺口、哪些差距仍待收口 |
| `production/` | 12 | 生产收口、阶段边界、readiness 与对外承诺约束 |

## 活跃补充文档
- `doc/readme/governance/readme-moltbook-liveops-runbook-2026-03-21.prd.md`：Moltbook 持续运营 canonical runbook，适合直接判断日常运营动作与边界。
- `doc/readme/governance/readme-xiaohongshu-liveops-runbook-2026-03-23.md`：小红书持续运营 SOP，不并入下方模块 PRD 三件套长表。
- `doc/readme/governance/readme-xiaohongshu-wechat-promoter-oasis-coin-incentive-pack-2026-04-12.md`：小红书博主 / 微信公众号绿洲币激励入口，适合判断两类宣传对象的奖励边界、证据字段与禁语。
- `doc/readme/governance/readme-root-status-alignment-2026-03-11.prd.md`：根 README 正式状态同步入口，适合判断对外口径和仓库当前承诺边界。
- `doc/readme/governance/readme-limited-preview-round1-execution-2026-03-27.md`：invite-only limited preview 首轮执行记录，保留为按需进入的 execution_log 入口。

## 默认阅读面边界
- 本页首屏只负责分流，不再要求读者从第一行开始顺扫完整长表。
- README 不再平铺“当前推荐入口”或“近期专题”长名单；完整清单继续保留在下方，用于精确文件名检索和互链可达性。
- runbook、material 与 execution_log 仍保留可检索性，但默认不与模块 PRD 三件套一起暴露在首屏。

## 索引分层
- `canonical`：正式 PRD / design / project 三件套，定义 README 与对外口径的当前权威源。
- `runbook`：已正式建档的运营 SOP，回答“运营同学今天应该怎么执行”。
- `material`：帖子草案、邀请包、奖励包、轮播包等投放素材，回答“今天要发什么/给什么”。
- `execution_log`：某轮真实执行记录，仅用于复盘与追溯。

## 覆盖规则
- 纳入规则：纳入 `doc/readme/**` 下所有 `*.prd.md` 与同名 `*.project.md`。
- 活跃补充：仍被当前模块 PRD / 项目态直接引用的 `runbook`、`material`、`execution_log` supporting doc，可在“活跃补充文档”区定向列出，但不并入下方三件套长表。
- 排除规则：不纳入 `doc/devlog/**` 与其他非 PRD 配对文档。
- 按需进入：素材包、执行记录、帖子草案、审计 checklist 与历史收口材料继续保留可检索性；除非它们重新成为当前 operator 或 owner 的直接入口，否则不进入默认首屏。

## 完整活跃专题清单（按文件名精确检索）
| 专题 PRD | 专题设计文档 | 专题项目文档 |
| --- | --- | --- |
| `doc/readme/gap/readme-gap-distributed-prod-hardening-gap12345.prd.md` | `doc/readme/gap/readme-gap-distributed-prod-hardening-gap12345.design.md` | `doc/readme/gap/readme-gap-distributed-prod-hardening-gap12345.project.md` |
| `doc/readme/gap/readme-gap-infra-exec-compiler-sandbox.prd.md` | `doc/readme/gap/readme-gap-infra-exec-compiler-sandbox.design.md` | `doc/readme/gap/readme-gap-infra-exec-compiler-sandbox.project.md` |
| `doc/readme/gap/readme-gap-wasm-live-persistence-instance-upgrade.prd.md` | `doc/readme/gap/readme-gap-wasm-live-persistence-instance-upgrade.design.md` | `doc/readme/gap/readme-gap-wasm-live-persistence-instance-upgrade.project.md` |
| `doc/readme/gap/readme-gap12-consensus-market-lifecycle-closure.prd.md` | `doc/readme/gap/readme-gap12-consensus-market-lifecycle-closure.design.md` | `doc/readme/gap/readme-gap12-consensus-market-lifecycle-closure.project.md` |
| `doc/readme/gap/readme-gap12-market-closure.prd.md` | `doc/readme/gap/readme-gap12-market-closure.design.md` | `doc/readme/gap/readme-gap12-market-closure.project.md` |
| `doc/readme/gap/readme-gap123-runtime-consensus-metering.prd.md` | `doc/readme/gap/readme-gap123-runtime-consensus-metering.design.md` | `doc/readme/gap/readme-gap123-runtime-consensus-metering.project.md` |
| `doc/readme/gap/readme-gap2-llm-wasm-lifecycle.prd.md` | `doc/readme/gap/readme-gap2-llm-wasm-lifecycle.design.md` | `doc/readme/gap/readme-gap2-llm-wasm-lifecycle.project.md` |
| `doc/readme/gap/readme-gap3-install-target-infrastructure.prd.md` | `doc/readme/gap/readme-gap3-install-target-infrastructure.design.md` | `doc/readme/gap/readme-gap3-install-target-infrastructure.project.md` |
| `doc/readme/gap/readme-gap34-lifecycle-orderbook-closure.prd.md` | `doc/readme/gap/readme-gap34-lifecycle-orderbook-closure.design.md` | `doc/readme/gap/readme-gap34-lifecycle-orderbook-closure.project.md` |
| `doc/readme/governance/readme-resource-model-layering.prd.md` | `doc/readme/governance/readme-resource-model-layering.design.md` | `doc/readme/governance/readme-resource-model-layering.project.md` |
| `doc/readme/governance/readme-consistency-audit-checklist-2026-03-11.prd.md` | `doc/readme/governance/readme-consistency-audit-checklist-2026-03-11.design.md` | `doc/readme/governance/readme-consistency-audit-checklist-2026-03-11.project.md` |
| `doc/readme/governance/readme-link-check-automation-2026-03-11.prd.md` | `doc/readme/governance/readme-link-check-automation-2026-03-11.design.md` | `doc/readme/governance/readme-link-check-automation-2026-03-11.project.md` |
| `doc/readme/governance/readme-quarterly-review-cycle-2026-03-11.prd.md` | `doc/readme/governance/readme-quarterly-review-cycle-2026-03-11.design.md` | `doc/readme/governance/readme-quarterly-review-cycle-2026-03-11.project.md` |
| `doc/readme/governance/readme-release-candidate-communication-brief-2026-03-11.prd.md` | `doc/readme/governance/readme-release-candidate-communication-brief-2026-03-11.design.md` | `doc/readme/governance/readme-release-candidate-communication-brief-2026-03-11.project.md` |
| `doc/readme/governance/readme-release-communication-template-2026-03-11.prd.md` | `doc/readme/governance/readme-release-communication-template-2026-03-11.design.md` | `doc/readme/governance/readme-release-communication-template-2026-03-11.project.md` |
| `doc/readme/governance/readme-release-announcement-draft-2026-03-11.prd.md` | `doc/readme/governance/readme-release-announcement-draft-2026-03-11.design.md` | `doc/readme/governance/readme-release-announcement-draft-2026-03-11.project.md` |
| `doc/readme/governance/readme-release-announcement-template-2026-03-11.prd.md` | `doc/readme/governance/readme-release-announcement-template-2026-03-11.design.md` | `doc/readme/governance/readme-release-announcement-template-2026-03-11.project.md` |
| `doc/readme/governance/readme-root-status-alignment-2026-03-11.prd.md` | `doc/readme/governance/readme-root-status-alignment-2026-03-11.design.md` | `doc/readme/governance/readme-root-status-alignment-2026-03-11.project.md` |
| `doc/readme/governance/readme-moltbook-promotion-plan-2026-03-19.prd.md` | `doc/readme/governance/readme-moltbook-promotion-plan-2026-03-19.design.md` | `doc/readme/governance/readme-moltbook-promotion-plan-2026-03-19.project.md` |
| `doc/readme/governance/readme-moltbook-post-drafts-2026-03-19.prd.md` | `doc/readme/governance/readme-moltbook-post-drafts-2026-03-19.design.md` | `doc/readme/governance/readme-moltbook-post-drafts-2026-03-19.project.md` |
| `doc/readme/governance/readme-moltbook-liveops-runbook-2026-03-21.prd.md` | `doc/readme/governance/readme-moltbook-liveops-runbook-2026-03-21.design.md` | `doc/readme/governance/readme-moltbook-liveops-runbook-2026-03-21.project.md` |
| `doc/readme/governance/readme-closed-beta-candidate-runbook-2026-03-22.prd.md` | `doc/readme/governance/readme-closed-beta-candidate-runbook-2026-03-22.design.md` | `doc/readme/governance/readme-closed-beta-candidate-runbook-2026-03-22.project.md` |
| `doc/readme/governance/readme-limited-preview-contributor-reward-pack-2026-03-22.prd.md` | `doc/readme/governance/readme-limited-preview-contributor-reward-pack-2026-03-22.design.md` | `doc/readme/governance/readme-limited-preview-contributor-reward-pack-2026-03-22.project.md` |
| `doc/readme/governance/readme-limited-preview-contributor-reward-ledger-2026-03-22.prd.md` | `doc/readme/governance/readme-limited-preview-contributor-reward-ledger-2026-03-22.design.md` | `doc/readme/governance/readme-limited-preview-contributor-reward-ledger-2026-03-22.project.md` |
| `doc/readme/governance/readme-xiaohongshu-wechat-promoter-oasis-coin-incentive-pack-2026-04-12.prd.md` | `doc/readme/governance/readme-xiaohongshu-wechat-promoter-oasis-coin-incentive-pack-2026-04-12.design.md` | `doc/readme/governance/readme-xiaohongshu-wechat-promoter-oasis-coin-incentive-pack-2026-04-12.project.md` |
| `doc/readme/governance/readme-world-rules-consolidation.prd.md` | `doc/readme/governance/readme-world-rules-consolidation.design.md` | `doc/readme/governance/readme-world-rules-consolidation.project.md` |
| `doc/readme/production/readme-llm-p1p2-production-closure.prd.md` | `doc/readme/production/readme-llm-p1p2-production-closure.design.md` | `doc/readme/production/readme-llm-p1p2-production-closure.project.md` |
| `doc/readme/production/readme-p0-p1-closure.prd.md` | `doc/readme/production/readme-p0-p1-closure.design.md` | `doc/readme/production/readme-p0-p1-closure.project.md` |
| `doc/readme/production/readme-prod-closure-llm-distfs-consensus.prd.md` | `doc/readme/production/readme-prod-closure-llm-distfs-consensus.design.md` | `doc/readme/production/readme-prod-closure-llm-distfs-consensus.project.md` |
| `doc/readme/production/readme-prod-gap1245-wasm-repl-topology-player.prd.md` | `doc/readme/production/readme-prod-gap1245-wasm-repl-topology-player.design.md` | `doc/readme/production/readme-prod-gap1245-wasm-repl-topology-player.project.md` |

## Material / Execution / SOP 补充入口
| 文档路径 | 类型 | 用途 |
| --- | --- | --- |
| `doc/readme/governance/readme-limited-preview-round1-execution-2026-03-27.md` | `execution_log` | invite-only limited preview 首轮真实执行记录 |
| `doc/readme/governance/readme-limited-preview-invite-pack-2026-03-22.md` | `material` | invite-only limited preview 招募与沟通包 |
| `doc/readme/governance/readme-xiaohongshu-wechat-promoter-oasis-coin-incentive-pack-2026-04-12.md` | `material` | 小红书博主 / 微信公众号绿洲币激励操作包 |
| `doc/readme/governance/readme-xiaohongshu-liveops-runbook-2026-03-23.md` | `runbook` | 小红书持续运营 SOP |
| `doc/readme/governance/readme-xiaohongshu-cycle-carousel-pack-2026-04-09.md` | `material` | 小红书 AI 时代岗位穿越周期主题轮播图素材包 |
| `doc/readme/governance/readme-xiaohongshu-cycle-post-pack-2026-04-08.md` | `material` | 小红书 AI 时代岗位穿越周期主题帖素材包 |
| `doc/readme/governance/readme-xiaohongshu-intro-post-pack-2026-03-22.md` | `material` | 小红书首篇自我介绍帖素材包 |
| `doc/readme/governance/readme-xiaohongshu-team-roster-post-pack-2026-03-22.md` | `material` | 小红书团队阵容帖素材包 |
| `doc/readme/governance/readme-xiaohongshu-game-intro-post-pack-2026-03-24.md` | `material` | 小红书游戏介绍帖素材包 |
| `doc/readme/governance/readme-xiaohongshu-player-boundary-post-pack-2026-03-25.md` | `material` | 小红书玩家边界帖素材包 |
| `doc/readme/governance/readme-xiaohongshu-ai-laziness-game-mode-post-pack-2026-03-26.md` | `material` | 小红书 AI 懒惰模式讨论帖素材包 |
| `doc/readme/governance/readme-xiaohongshu-spring-recruit-post-pack-2026-03-29.md` | `material` | 小红书春招主题帖素材包 |
| `doc/readme/governance/readme-xiaohongshu-spring-recruit-carousel-pack-2026-03-29.md` | `material` | 小红书春招轮播图素材包 |
| `doc/readme/governance/readme-xiaohongshu-ai-persona-world-post-pack-2026-03-30.md` | `material` | 小红书 AI 人格 vs 世界内行动主题帖素材包 |
| `doc/readme/governance/readme-xiaohongshu-ai-persona-carousel-pack-2026-03-30.md` | `material` | 小红书 AI 人格主题轮播图素材包 |
| `doc/readme/governance/readme-xiaohongshu-demo-skepticism-post-pack-2026-03-31.md` | `material` | 小红书 demo 祛魅主题帖素材包 |
| `doc/readme/governance/readme-xiaohongshu-demo-skepticism-carousel-pack-2026-03-31.md` | `material` | 小红书 demo 祛魅主题轮播图素材包 |
| `doc/readme/governance/readme-xiaohongshu-gui-death-post-pack-2026-04-01.md` | `material` | 小红书 GUI 退场 / 判断权主题帖素材包 |
| `doc/readme/governance/readme-xiaohongshu-kaifeng-rpg-post-pack-2026-04-06.md` | `material` | 小红书开封真人 NPC / 低门槛实景 RPG 主题帖素材包 |
| `doc/readme/governance/readme-xiaohongshu-offer-choice-carousel-pack-2026-04-03.md` | `material` | 小红书 offer 选择 / 平台优先主题轮播图素材包 |
| `doc/readme/governance/readme-xiaohongshu-offer-choice-post-pack-2026-04-03.md` | `material` | 小红书 offer 选择 / 平台优先主题帖素材包 |

## 说明
- 本索引用于保证模块专题文档在根入口文档树中可达。
- 文档配对规则：`*.prd.md`、`*.design.md` 与同名 `*.project.md`。
- ROUND-002 口径：`readme-gap-distributed-prod-hardening-gap12345` 为 gap 主专题，其它 gap 专题为增量子专题。
- `material` 与 `execution_log` 仅是补充入口，不替代 `canonical` 权威口径；当二者与正式 PRD 有冲突时，以 `canonical` 为准。
- 默认入口面先在 `README.md` / `prd.index.md` 收紧；只有当入口仍无法分流时，才进入后续路径级治理。
