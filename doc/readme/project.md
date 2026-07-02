# readme PRD Project

审计轮次: 13

## 任务拆解（活跃面）
- 当前无未完成主项目任务；后续若继续扩展主站白皮书页内容、站点内中英专题页、奖励台账或渠道运营素材，应新开独立 task 并回写对应 topic project。

### 最近完成（保留一跳 Trace）
- [x] readme-governance-legacy-release-surface-convergence (PRD-ENGINEERING-025/030) [test_tier_required]: 收敛 `doc/readme/governance/` 默认阅读面中的 2026-03 release-candidate / announcement draft / closed-beta candidate 旧语义；删除两个一次性 release 输出文件，把对应三件套降级到历史压缩清单，并保留 reward/ledger 当前政策入口。 Trace: #1806 (task_3a96bbd2f3274f639464f879b02d55d4)
- [x] module-project-log-slimming (PRD-ENGINEERING-030) [test_tier_required]: 压缩 readme 主项目页历史流水为当前/最近任务索引与历史追溯入口，保留模块状态和一跳 task trace。 Trace: .pm/tasks/task_49ef9270afc646d98d4a8386c0888eab.yaml
- [x] game-positioning-alignment (PRD-README-046) [test_tier_required]: 对齐根 README、world-rule 与站点首页中英首页公开定位，收口为“文明模拟游戏”。 Trace: .pm/tasks/task_774fd50ebd6d4c53bc94592dbe8554cc.yaml
- [x] xiaohongshu-token-usage-post-pack (PRD-README-047) [test_tier_required]: 补齐“项目累计 token 用量”素材包与单图封面。 Trace: .pm/tasks/task_79a3b9140bb54e73984e9893485614e7.yaml
- [x] whitepaper-style-overview (PRD-README-048) [test_tier_required]: 新增白皮书式项目总览并接入根 README 与 readme/governance 导航。 Trace: .pm/tasks/task_5963ea33d3854bef802154b2996bea89.yaml
- [x] site-whitepaper-entry-alignment (PRD-README-049) [test_tier_required]: 更新站点首页与文档中心公开 framing，并新增站内白皮书静态展示页。 Trace: .pm/tasks/task_5e52a8a4ece04bcb932054e907c235ed.yaml
- [x] readme-governance-path-governance (PRD-ENGINEERING-030) [test_tier_required]: 为 `doc/readme/governance/` 建立 canonical 子域入口。 Trace: .pm/tasks/task_d37f636846fa44449988240af8630454.yaml
- [x] xiaohongshu-loop-engineering-post-pack (PRD-README-050) [test_tier_required]: 新增 `Loop Engineering在游戏开发中的实践` 小红书素材包和预览图。 Trace: .pm/tasks/task_a9ab9b9760c24e1fac5a31a157404408.yaml
- [x] xiaohongshu-social-package-migration (PRD-README-051) [test_tier_required]: 将历史小红书内容文案、HTML 源和导出素材迁移为 `site/social/xiaohongshu/<post-slug>/` package 布局，并更新治理引用。 Trace: .pm/tasks/task_9eff28a3b80d452b99c6890a981dc4d6.yaml
- [x] xiaohongshu-ai-economy-visual-draft (PRD-README-052) [test_tier_required]: 新增 AI 经济繁荣/萧条视角分享的小红书文案、8 张竖版轮播图、生成脚本与上传顺序说明。 Trace: .pm/tasks/task_0745838e8c6546a09faba1ce8c602e33.yaml

### 历史压缩索引
- README consistency、link check、quarterly review、release communication 模板与 public positioning 历史：回看 `doc/readme/prd.index.md`、`doc/readme/governance/README.md` 与对应 task trace。
- Moltbook、小红书、closed beta、limited preview、reward intake/ledger 与 community packaging 历史：回看 `doc/readme/governance/`、站点素材目录与 GitHub task issue evidence comments。
- 本主项目页只维护当前/最近任务索引；完整素材正文、发布边界、证据和验收命令以 topic docs 与 GitHub task issue evidence comments 为准。

## 依赖
- `doc/readme/prd.index.md`、`README.md`、`world-rule.md`、`testing-manual.md`
- `doc/readme/gap/`、`doc/readme/production/`、`doc/readme/governance/`
- `.agents/skills/prd/check.md`

## 状态
- 更新日期: 2026-07-03
- 当前状态: completed
- 下一任务: 无（当前模块主项目无未完成任务；若后续继续扩展主站白皮书页内容或做站点内中英更多专题页，再新开独立任务。）
- 当前窗口摘要: 最近收口集中在 `readme-governance-legacy-release-surface-convergence`、`site-whitepaper-entry-alignment`、`whitepaper-style-overview` 与 `xiaohongshu-token-usage-post-pack`；对应详情保留在上方任务项、`doc/readme/governance/` 专题、GitHub task issue evidence comments 与站点改动中。
- 历史追溯: 更早完成项不再在本状态区按时间追加；需要追 reward / Moltbook / 小红书 / closed beta / release-candidate instance / public positioning 历史时，先从 `doc/readme/prd.index.md`、`doc/readme/governance/README.md` 与对应 task trace 进入。
- PRD / ROUND 状态: strict schema 已对齐（含第 6 章验证与决策记录）；gap 子簇主从化已完成（gap12345 主入口，其它 gap 专题增量维护）。
- 模块进展补充: 已补齐 README 口径一致性巡检、链接检查、季度审查模板、对外口径简报/公告底稿模板，以及 Moltbook 推广方案、主贴模板、GitHub 反馈 CTA 与更短 feed-native 版本。
- 说明: 本文档仅维护 readme 模块设计执行状态；过程记录在 `doc/devlog/README.md` 与 `doc/devlog/README.md`。
