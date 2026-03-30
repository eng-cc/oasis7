# playability_test_result 文档索引

审计轮次: 10

## 入口
- PRD: `doc/playability_test_result/prd.md`
- 设计总览: `doc/playability_test_result/design.md`
- 标准执行入口: `doc/playability_test_result/project.md`
- 文件级索引: `doc/playability_test_result/prd.index.md`

## 从这里开始
- 想先确认可玩性证据的字段、评分口径与发布引用边界：先读 `doc/playability_test_result/prd.md`。
- 想看这个模块当前是否还有活跃任务、最近收口了什么：先读 `doc/playability_test_result/project.md`。
- 想直接复用单次结果卡或人工执行说明：先读 `doc/playability_test_result/playability_test_card.md` 与 `doc/playability_test_result/playability_test_manual.md`。
- 想找专题回归卡组，而不是单次样本：进入 `doc/playability_test_result/topics/`，当前高频入口是 `industrial-onboarding-required-tier-cards-2026-03-15.md`。
- 想追溯正式发布证据或跨模块引用样例：进入 `doc/playability_test_result/evidence/`。

## 模块职责
- 维护可玩性反馈卡、评分口径、高优问题闭环与发布证据包格式。
- 承接 game / testing / core 之间的体验证据互链。
- 统一最近活跃轮次的卡片与正式模板入口。

## 关键文档
- `doc/playability_test_result/game-test.prd.md`
- `doc/playability_test_result/game-test.project.md`
- `doc/playability_test_result/playability_test_card.md`
- `doc/playability_test_result/playability_test_manual.md`
- `doc/playability_test_result/topics/industrial-onboarding-required-tier-cards-2026-03-15.md`
- `doc/playability_test_result/templates/`
- `doc/playability_test_result/evidence/`
- `doc/playability_test_result/topics/`

## 根目录收口
- 模块根目录主入口保留：`README.md`、`prd.md`、`design.md`、`project.md`、`prd.index.md`，并允许保留当前活跃轮次的单次结果卡。
- 专题回归卡组与专题执行资产放入子目录（如 `topics/`），避免继续把根目录当作专题方案堆放区。
- 历史卡片不再保留在仓库（`archive/` 目录已移除）。

## 维护约定
- 可玩性模板、评分口径、专题卡组或发布引用格式变化时，需同步更新 `prd.md` 与 `project.md`。
- 新增专题后，需同步回写 `doc/playability_test_result/prd.index.md` 与本目录索引。
- README 优先服务证据消费者与追溯读者，不替代 `evidence/`、`topics/` 或 `prd.index.md` 的详细清单。
