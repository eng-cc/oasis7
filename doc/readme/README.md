# readme 文档索引

审计轮次: 8

## 入口
- PRD: `doc/readme/prd.md`
- 设计总览: `doc/readme/design.md`
- 标准执行入口: `doc/readme/project.md`
- 文件级索引: `doc/readme/prd.index.md`

## 模块职责
- 统一仓库对外说明口径与文档入口。
- 跟踪 README 与设计/实现的一致性缺口。
- 承接 release communication、公告底稿与根 README 状态同步等对外口径闭环。

## 消费分层
- `canonical`：正式对外口径、README 同步规则、release communication 与治理专题。
- `runbook`：渠道运营或窗口执行 SOP，回答“今天应该怎么执行”。
- `material`：帖子草案、邀请包、奖励包、轮播包等投放素材，回答“今天要发什么/给什么”。
- `execution_log`：某一轮真实执行记录与复盘，回答“上次怎么做、结果如何”。

## 主题文档
- `gap/`：README 与实现/流程间差距闭环。
- `production/`：生产口径与发布收口专题。
- `governance/`：规则层、资源模型、口径巡检、发布沟通与根 README 对齐专题。

## 当前推荐入口
- canonical：
  - `doc/readme/governance/readme-root-status-alignment-2026-03-11.prd.md`
  - `doc/readme/governance/readme-release-candidate-communication-brief-2026-03-11.prd.md`
  - `doc/readme/governance/readme-release-announcement-template-2026-03-11.prd.md`
- runbook：
  - `doc/readme/governance/readme-moltbook-liveops-runbook-2026-03-21.prd.md`
  - `doc/readme/governance/readme-closed-beta-candidate-runbook-2026-03-22.prd.md`
  - `doc/readme/governance/readme-xiaohongshu-liveops-runbook-2026-03-23.md`
- material：
  - `doc/readme/governance/readme-limited-preview-invite-pack-2026-03-22.md`
  - `doc/readme/governance/readme-limited-preview-contributor-reward-pack-2026-03-22.prd.md`
  - `doc/readme/governance/readme-xiaohongshu-intro-post-pack-2026-03-22.md`
  - `doc/readme/governance/readme-xiaohongshu-ai-persona-world-post-pack-2026-03-30.md`
- execution_log：
  - `doc/readme/governance/readme-limited-preview-round1-execution-2026-03-27.md`

## 近期专题
- canonical：
- `doc/readme/governance/readme-consistency-audit-checklist-2026-03-11.prd.md`
- `doc/readme/governance/readme-link-check-automation-2026-03-11.prd.md`
- `doc/readme/governance/readme-quarterly-review-cycle-2026-03-11.prd.md`
- `doc/readme/governance/readme-release-candidate-communication-brief-2026-03-11.prd.md`
- `doc/readme/governance/readme-release-communication-template-2026-03-11.prd.md`
- `doc/readme/governance/readme-release-announcement-draft-2026-03-11.prd.md`
- `doc/readme/governance/readme-release-announcement-template-2026-03-11.prd.md`
- `doc/readme/governance/readme-root-status-alignment-2026-03-11.prd.md`
- `doc/readme/governance/readme-moltbook-promotion-plan-2026-03-19.prd.md`
- `doc/readme/governance/readme-moltbook-post-drafts-2026-03-19.prd.md`
- `doc/readme/governance/readme-moltbook-liveops-runbook-2026-03-21.prd.md`
- `doc/readme/governance/readme-closed-beta-candidate-runbook-2026-03-22.prd.md`
- `doc/readme/governance/readme-limited-preview-contributor-reward-pack-2026-03-22.prd.md`
- `doc/readme/governance/readme-limited-preview-contributor-reward-ledger-2026-03-22.prd.md`
- material / execution：
- `doc/readme/governance/readme-limited-preview-invite-pack-2026-03-22.md`
- `doc/readme/governance/readme-limited-preview-round1-execution-2026-03-27.md`
- `doc/readme/governance/readme-xiaohongshu-intro-post-pack-2026-03-22.md`
- `doc/readme/governance/readme-xiaohongshu-team-roster-post-pack-2026-03-22.md`
- `doc/readme/governance/readme-xiaohongshu-game-intro-post-pack-2026-03-24.md`
- `doc/readme/governance/readme-xiaohongshu-player-boundary-post-pack-2026-03-25.md`
- `doc/readme/governance/readme-xiaohongshu-ai-laziness-game-mode-post-pack-2026-03-26.md`
- `doc/readme/governance/readme-xiaohongshu-spring-recruit-post-pack-2026-03-29.md`
- `doc/readme/governance/readme-xiaohongshu-spring-recruit-carousel-pack-2026-03-29.md`
- `doc/readme/governance/readme-xiaohongshu-ai-persona-world-post-pack-2026-03-30.md`
- `doc/readme/governance/readme-xiaohongshu-liveops-runbook-2026-03-23.md`

## 根目录收口
- 模块根目录主入口保留：`README.md`、`prd.md`、`design.md`、`project.md`、`prd.index.md`。
- 其余专题文档按主题下沉到 `gap/production/governance`。

## 维护约定
- 对外口径变更需同步 `README.md`、`site/` 与本模块文档。
- 新增专题后，需同步回写 `doc/readme/prd.index.md` 与本目录索引。
- 新增渠道素材或执行记录时，必须显式标注其属于 `material` 或 `execution_log`，不得与 `canonical` 权威口径混写成同一层级。
