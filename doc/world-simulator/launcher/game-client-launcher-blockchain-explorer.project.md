# 客户端启动器区块链浏览器执行台账（当前 authority）

> 对应需求: `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer.prd.md`
> 对应设计: `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer.design.md`

## 状态

- 状态：`documented_current_authority`。
- 本轮完成：建立 stable explorer PRD/design/project 三件套，并把 launcher landing、world-simulator 索引和模块项目入口改指向它。
- 本轮未做：不修改 `oasis7_client_launcher`、`oasis7_web_launcher`、runtime、DOM、UI、测试或五组源三件套；更不把文档迁移写成 mainnet/readiness/public/settlement/validator/no-reset/full-archive 承诺。

## 任务拆解

- [x] launcher-blockchain-explorer-stable-authority (PRD-WORLD_SIMULATOR-024) [test_tier_required]: 收敛 explorer 概览、七视图、只读控制面与状态呈现的当前 authority。 Trace: https://github.com/eng-cc/oasis7/issues/2580 (task_50841597d34a4d51a5511303d5b44a60)
- [x] launcher-blockchain-explorer-routing-cutover (PRD-WORLD_SIMULATOR-024) [test_tier_required]: 将 Launcher landing、world-simulator PRD index/project 默认路由指向 stable authority。 Trace: https://github.com/eng-cc/oasis7/issues/2580 (task_50841597d34a4d51a5511303d5b44a60)
- [ ] launcher-blockchain-explorer-source-retirement (PRD-WORLD_SIMULATOR-024) [test_tier_required]: 在独立 migration-governance slice 完成语义回填、活跃引用修复与 focused evidence 后，再决定是否删除五组历史源三件套。 Trace: https://github.com/eng-cc/oasis7/issues/2580 (task_50841597d34a4d51a5511303d5b44a60)

## 依赖

- 当前行为事实依赖 `oasis7_client_launcher` 的 explorer 视图模块、`oasis7_web_launcher` 的既有代理及 runtime 的既有只读 explorer 查询。
- 文档路由依赖 `doc/world-simulator/launcher/README.md`、`doc/world-simulator/prd.index.md` 与 `doc/world-simulator/project.md`。
- 可见交互验收由 `testing-manual.md` S6 和 game_visual_interaction_designer 的后续规格定义；本轮不触发该验收。

## 当前验收边界

- 文档迁移验收：`./scripts/doc-governance-check.sh`、`./scripts/readme-link-check.sh` 和 `git diff --check`。
- 现有行为不因本轮重新验收；未来改动按实现路径运行 launcher、Web、wasm 测试，触达可见界面时附加 `testing-manual.md` S6 浏览器证据。
- S6 截图不适用：本轮不改变可见表面、DOM、交互或行为。

## 后续事项

- 若要新增 API、长期 archive、保留/重置语义、网络 readiness、validator、结算或公开服务结论，必须由对应 runtime/blockchain-ops/LiveOps 专题与当前任务证据授权，不能从本文推断。
- 若要改变信息层级、窄屏行为、CTA 或状态恢复提示，先由 game_visual_interaction_designer 给出屏幕验收，再经 TPM 协调 Viewer 控制边界。
- 五组 source triplet 在当前仓库仍可精确检索；删除决策与 provenance 以后续独立治理切片为准。
