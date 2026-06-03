# Story Workspace

本目录用于维护长篇小说《绿洲 2076》（英文名：oasis 2076）的站点侧资料。

世界背景、总纲、人物、调研、审稿意见、第一卷时间线锚点、第一卷四章章节卡和第一卷正文草稿已重建；第二卷 Agent 归零与修宪路线已完成全卷规划，第一章正文已试写到 `CH-046`。

## 目录

- `background/`: 世界背景已复审更新。
- `outline/`: 长篇总纲已重建到第一卷目标层级。
- `characters/`: 人物注册表已开始重建。
- `timeline/`: 时间线已开始重建，包含第一卷三十六场和第二卷 `TL-037` 到 `TL-078`。
- `chapter-cards/`: 章节卡已开始重建，第一卷四章已拆完，共三十六场；第二卷五章已规划到 `CH-037` 到 `CH-078`。
- `research/`: 调研笔记已开始重建。
- `reviews/`: 审稿意见已开始重建。
- `draft/`: 正文草稿和章节写作 SOP 已开始重建。

## 写作边界

- 当前 `background/world-background.md`、`outline/novel-outline.md`、`characters/character-registry.md`、`timeline/timeline.md` 与 `chapter-cards/` 下第一卷四章章节卡已包含已确认 canon；第一卷已按 `reviews/editorial-notes.md` 封存为稳定基线。第二卷当前已完成全卷规划和第一章正文试写；后续正文仍需逐章按 SOP 进入外部调研、写前定位、正文试写和 focused review。
- 新增人物、背景、时间线或正文前，应先明确其是否进入正式故事真值。
- 每次新增正文前，先补齐对应的大纲、时间线或角色动机，避免后期失控。
- 从章节卡进入正文时，先按 `draft/chapter-writing-sop.md` 执行社会口味调研、写前定位、正文试写、审稿、最小补丁和复审。

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
