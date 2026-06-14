# Story Workspace

本目录用于维护长篇小说《绿洲 2076》（英文名：oasis 2076）的站点侧资料。

世界背景、总纲、人物、调研、审稿意见、第一卷时间线锚点、第一卷四章章节卡和第一卷正文已重建；第一卷已整理为阅读版 v1.0-rc，发布清单见 `releases/volume-01-reading-version.md`。第二卷 Agent 归零与修宪路线已完成 `CH-037` 到 `CH-078` 的全卷规划、正文试写和卷级 closeout；第三卷温室 / 生活循环路线已完成 `CH-079` 到 `CH-118` 的全卷规划、正文试写和 focused review；`CH-118` 后的第三卷 / 后续路线已按 SOP 采纳到 `TL-119` 到 `TL-158` / `CH-119` 到 `CH-158`，并完成五章正文试写、focused review 最小补丁和 route closeout。第四卷公共节律路线已进入 scaffold，暂定覆盖 `TL-159` 到 `TL-198` / `CH-159` 到 `CH-198`，主问题是多个低规格公共入口如何同页让路而不排序。

## 目录

- `background/`: 世界背景已复审更新。
- `outline/`: 长篇总纲已重建到第一卷目标层级。
- `characters/`: 人物注册表已开始重建。
- `timeline/`: 时间线已开始重建，包含第一卷三十六场、第二卷 `TL-037` 到 `TL-078`，第三卷 `TL-079` 到 `TL-118`，第三卷后续 `TL-119` 到 `TL-158`，以及第四卷 scaffold `TL-159` 到 `TL-198`。
- `chapter-cards/`: 章节卡已开始重建，第一卷四章已拆完，共三十六场；第二卷五章已完成正文与章级 closeout；第三卷五章已完成章节卡、正文与 focused review；第三卷后续五章已拆为正式章级卡；第四卷公共节律路线已有总章节卡草案。
- `research/`: 调研笔记已开始重建。
- `reviews/`: 审稿意见已开始重建。
- `draft/`: 正文、后续草稿和章节写作 SOP 已开始重建。
- `releases/`: 公开阅读版发布清单与范围说明。

## 写作边界

- 当前 `background/world-background.md`、`outline/novel-outline.md`、`characters/character-registry.md`、`timeline/timeline.md` 与 `chapter-cards/` 下第一卷四章章节卡已包含已确认 canon；第一卷已按 `reviews/editorial-notes.md` 封存为稳定基线。第二卷 `CH-037` 到 `CH-078` 已作为 Agent 归零与铅笔条款低规格试运行 route 关闭；第三卷 `CH-079` 到 `CH-118` 已完成温室 / 生活循环路线的章节卡、写前定位、正文试写和 focused review，卷尾口径为低规格生活循环入口成立，不是生态自给完成。
- 新增人物、背景、时间线或正文前，应先明确其是否进入正式故事真值。
- 每次新增正文前，先补齐对应的大纲、时间线或角色动机，避免后期失控。
- 从章节卡进入正文时，先按 `draft/chapter-writing-sop.md` 执行社会口味调研、写前定位、正文试写、审稿、最小补丁和复审。
- 第三卷 / 后续路线 scaffold 入口：`outline/volume-03-followup-route-scaffold.md`、`chapter-cards/volume-03-followup-route-cards.md`、`draft/volume-03-followup-writing-positioning.md`；正式章级卡已拆至 `chapter-cards/volume-03-followup-chapter-01-cards.md` 到 `chapter-cards/volume-03-followup-chapter-05-cards.md`，五章写前定位与正文索引见 `draft/README.md`。
- 第四卷 / 公共节律路线 scaffold 入口：`outline/volume-04-route-scaffold.md`、`chapter-cards/volume-04-route-cards.md`、`draft/volume-04-writing-positioning.md`、`research/volume-04-route-research.md`；当前为 route scaffold，尚未拆正式章级卡或进入正文。

## 多方审稿 subagent 配置

涉及大背景、核心人物、卷级主线、章节路线或正文长段落时，应由 `producer_system_designer` 作为主笔 / 创作负责人汇总结论，并按需派生以下审稿 subagent：

- `story_structure_editor`: 审第一卷发动机、阶段目标、危机升级、结尾情感胜利和章节节奏是否成立。
- `worldbuilding_editor`: 审 Agent、OC、量子通信、模块化机器人、绿洲工业闭环和世界规则是否自洽。
- `character_editor`: 审老人、Agent、关系网、人物欲望和人物弧线是否清楚，职业背景是否只塑造特点而不限制能力。
- `target_reader_editor`: 从年轻读者情感需求审设定是否仍然表达向往、未来自我投射、纯粹友情和慢人文关怀。
- `technical_background_editor`: 审互联网前端、后端、运维、算法、运营、产品、市场、动画技术、采矿、3D 打印和机器人相关背景是否可信但不抢戏。
- `ethics_sensitivity_editor`: 审是否误写成老人苦难、年龄歧视、AI 替代真人关系、资源竞技、贫富排名、公司阴谋或赛博黑暗风。
- `canon_continuity_editor`: 审 `WR-*`、`CHAR-*`、时间线、术语、年龄、Agent 模块规则和已确认禁区是否漂移。
- `internet_culture_editor`: 审互联网行业梗、流程词和反差感是否自然，是否会心但不油滑，是否被转译成照顾人的动作。
- `style_language_editor`: 审文本是否有小说感，避免设定堆砌、产品文档味、互联网黑话和口号化主题表达。

审稿结果应回写到 `reviews/`，重大设定采纳后同步更新 `background/`、`outline/`、`characters/`、`timeline/` 或 `chapter-cards/` 中对应 canon。
