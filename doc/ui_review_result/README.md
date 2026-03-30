# ui_review_result 目录说明

审计轮次: 9

## 入口
- 评审列表：`doc/ui_review_result/ui_review_list.md`
- 当前示例卡片：`doc/ui_review_result/card_2026_03_06_11_50_29.md`
- 评分模板来源：`doc/world-simulator/prd/acceptance/visual-review-score-card.md`

## 定位结论
- 本目录是 `viewer_engineer` 维护的活跃评审样本池，不是正式模块，也不承担长期知识库职责。
- 保留在根级例外目录的原因：其内容是短周期 UI/视觉打分卡与样本索引，生命周期短，且需要与 `output/visual_review/*` 一一对应。
- 正式体验结论、可复用规则与长期口径应回写到 `doc/world-simulator/**`、`doc/playability_test_result/**` 或对应 PRD / project，而不是长期堆积在本目录。

## 目录职责
- 沉淀 UI / 视觉评审结果，结构对齐 `doc/playability_test_result` 的卡片化留痕方式。
- 为 `world-simulator` 的界面体验评审提供可追溯卡片入口。
- 仅保留当前活跃轮次样本，不承担长期归档职责。

## 当前内容
- `ui_review_list.md`：UI 评审条目列表与待处理入口。
- `card_2026_03_06_11_50_29.md`：当前首张待评审卡片。

## 维护约定
- 新增 UI 评审卡后，需同步更新 `ui_review_list.md` 与本目录说明。
- 正式评审口径以 `doc/world-simulator/prd/acceptance/visual-review-score-card.md` 为准。
- 历史卡片不在本目录长期归档；如需长期沉淀，应由所属模块专题文档引用承接。
- 进入条件：当前轮次需要保留可评分的 UI/视觉样本卡。
- 退出条件：当样本对应的体验结论已回写正式模块文档，且无继续迭代需求时，应清空或替换为新的活跃样本，不在此处形成伪模块历史库。
