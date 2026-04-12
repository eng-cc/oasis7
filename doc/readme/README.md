# readme 文档索引

审计轮次: 12

## 从这里开始
- 想先回答 README 模块在管什么、哪些内容属于正式对外口径：`doc/readme/prd.md`
- 想看当前执行任务、最新完成项与后续活跃动作：`doc/readme/project.md`
- 想按子域或文件名继续下钻，而不是从长名单里逐条找：`doc/readme/prd.index.md`
- 想直接看当前高频 liveops/operator 入口：`doc/readme/governance/readme-moltbook-liveops-runbook-2026-03-21.prd.md`
- 想直接看当前小红书持续运营入口：`doc/readme/governance/readme-xiaohongshu-liveops-runbook-2026-03-23.md`
- 想直接看当前小红书博主 / 微信公众号绿洲币激励入口：`doc/readme/governance/readme-xiaohongshu-wechat-promoter-oasis-coin-incentive-pack-2026-04-12.md`

## 入口
- PRD: `doc/readme/prd.md`
- 设计总览: `doc/readme/design.md`
- 标准执行入口: `doc/readme/project.md`
- 文件级索引: `doc/readme/prd.index.md`

## 入口分工
- `README.md` 只承担 landing page 职责：告诉读者先去哪个权威入口，不重复长表索引内容。
- `prd.md` 是模块权威规格入口，适合先理解 README 对外口径、缺口治理与运营内容边界。
- `project.md` 是执行台账，适合确认当前活跃专题、收口状态与最新完成项。
- `prd.index.md` 是定向检索索引，适合已经知道主题后按子域或文件名继续下钻，不是新读者的首读入口。
- liveops runbook / material / execution 文档按需进入，不再在模块 README 首屏平铺成长名单。

## 活跃阅读面边界
- 当前页只保留 `what / where / next / risk` 所需入口，不再直接罗列 canonical、material 与近期专题的长名单。
- 高频 active 入口保留在 `prd.md`、`project.md`、`prd.index.md` 与当前仍在执行的 runbook 文档。
- 审计留痕、历史背景、素材包与执行记录继续保留可检索性，但默认从 `prd.index.md` 或具体专题路径进入。

## 模块职责
- 统一仓库对外说明口径与文档入口。
- 跟踪 README 与设计/实现的一致性缺口。
- 承接 release communication、公告底稿、运营 runbook 与根 README 状态同步等对外口径闭环。

## 热点子域导航（2026-04-12 快照）
- `governance/`（93）：根 README 对齐、release communication、渠道运营 runbook、奖励与 invite 包。
- `gap/`（27）：README 与实现/流程间差距闭环，适合 owner 排查口径或能力缺口时进入。
- `production/`（12）：生产收口、阶段边界与 readiness 主题。

## 高密度提示
- `doc/readme/` 当前共有 137 份文件；默认入口不再尝试把 canonical、material 与 execution 专题全部摊平展示。
- 需要完整活跃专题清单时，进入 `doc/readme/prd.index.md`；需要素材包、执行记录或历史专题时，再按具体子域进入。

## 根目录收口
- 模块根目录主入口保留：`README.md`、`prd.md`、`design.md`、`project.md`、`prd.index.md`。
- 其余专题文档按主题下沉到 `gap/`、`production/`、`governance/`。

## 共享约定
- 模块根入口、专题落位与 README/legacy redirect 的共享规则统一以 `doc/engineering/doc-structure-standard.design.md` 为准。
- 对外口径、运营路径或 README 状态同步规则变化时，优先更新 `doc/readme/prd.md` 与 `doc/readme/project.md`；新增专题后，再同步回写 `doc/readme/prd.index.md`。
