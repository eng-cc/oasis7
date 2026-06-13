# 游戏发布前测试（game-test）设计

- 对应需求文档: `doc/playability_test_result/game-test.prd.md`
- 对应项目管理文档: `doc/playability_test_result/game-test.project.md`

## 1. 设计定位
定义发布前真实玩家闭环测试设计，统一启动脚本、Playwright 进入方式、卡片填写与证据沉淀路径。

## 2. 设计结构
- 启动编排层：当前 worktree 内回归优先通过 `scripts/worktree-harness.sh up` 启动隔离栈；制作人 / 发布前人工验收优先通过 `scripts/run-producer-playtest.sh --open-headed` 启动真实产品链路；只有高级排障或专项回归需要直接复用 bundle / 底层 launcher bootstrap 时才使用 `scripts/run-launcher-stack.sh`。
- 真实游玩层：测试者只按玩家视角进入游戏并完成实际操作，不依赖其他实现文档。
- 证据沉淀层：将卡片、录屏与产物输出到 `doc/playability_test_result/` 与 `output/playwright/playability/`。
- 样本治理层：维护活跃样本与历史归档分层，降低旧结果对当前判断的噪音。

## 3. 关键接口 / 入口
- `./scripts/worktree-harness.sh up`
- `./scripts/run-producer-playtest.sh --open-headed`
- `./scripts/run-launcher-stack.sh --bundle-dir <bundle>`（底层 / 高级 / 专项回归入口）
- Playwright 打开 URL（带 `test_api=1`）
- `doc/playability_test_result/playability_test_card.md`
- `output/playwright/playability/`

## 4. 约束与边界
- 测试过程必须以真实玩家视角执行，不可用实现细节替代体验验证。
- `doc/playability_test_result/game-test.prd.md` 为用户锁定文档，本轮不修改。
- 样本归档与活跃视图必须保持可追溯且不过度堆积历史噪音。

## 5. 设计演进计划
- 先固定启动脚本与进入游戏入口。
- 再统一卡片、录屏和证据沉淀路径。
- 最后持续维护活跃样本/归档分层与复测门禁。
