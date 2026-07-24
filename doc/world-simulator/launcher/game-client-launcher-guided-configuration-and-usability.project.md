# 客户端启动器引导配置与可用性执行台账（当前 authority）

> 对应需求: `doc/world-simulator/launcher/game-client-launcher-guided-configuration-and-usability.prd.md`
> 对应设计: `doc/world-simulator/launcher/game-client-launcher-guided-configuration-and-usability.design.md`

## 状态

- 状态：`documented_current_authority`。
- 本轮完成：建立 stable launcher PRD/design/project 三件套，修复 launcher landing、world-simulator PRD index/project 路由，并删除四组已吸收的 source triplet。
- 本轮未做：不修改 `oasis7_client_launcher`、`oasis7_web_launcher`、runtime、DOM、UI 或测试；不把历史完成、浏览器证据或 release/readiness 判断写成当前事实。

## 任务拆解

- [x] launcher-guided-configuration-stable-authority (PRD-WORLD_SIMULATOR-001/002/003/027/029/030) [test_tier_required]: 收敛语言/配置清晰度、事实状态、响应式自引导和既有下一步提示的当前 authority。 Trace: https://github.com/eng-cc/oasis7/issues/2583 (task_6840e029d65f43e7b9f58f8631fd40be)
- [x] launcher-guided-configuration-routing-cutover (PRD-WORLD_SIMULATOR-001/002/003/027/029/030) [test_tier_required]: 将 Launcher landing、world-simulator PRD index/project 默认路由指向 stable authority。 Trace: https://github.com/eng-cc/oasis7/issues/2583 (task_6840e029d65f43e7b9f58f8631fd40be)
- [x] launcher-guided-configuration-source-retirement (PRD-WORLD_SIMULATOR-001/002/003/027/029/030) [test_tier_required]: 已完成语义回填与活跃引用修复，删除四组 source triplet；追溯使用 Git 与 GitHub task issue evidence。 Trace: https://github.com/eng-cc/oasis7/issues/2583 (task_6840e029d65f43e7b9f58f8631fd40be)

## 依赖与验收边界

- 当前行为事实依赖 `oasis7_client_launcher` 的配置、状态与 self-guided 表现层；具体实现必须在需要时重新核验。
- 文档路由依赖 `doc/world-simulator/launcher/README.md`、`doc/world-simulator/prd.index.md` 与 `doc/world-simulator/project.md`。
- 文档迁移验收：`./scripts/doc-governance-check.sh`、`./scripts/readme-link-check.sh`、`python3 scripts/product-doc-governance-check.py` 与 `git diff --check`。
- S6 截图不适用：本轮不改变可见表面、DOM、交互或行为。未来可见变更按 `testing-manual.md` S6 取证，视觉/交互验收由 game_visual_interaction_designer 定义。

## 后续事项

- 新的配置规则、状态标签、引导 CTA、响应式布局、数据持久化或玩家入口结论，必须由对应专业角色在独立任务中授权并取证。
- runtime/chain execution-world/stale-world 主题仍分别由其保留 triplet 维护，本三件套不吸收或替代它们。
- 四组 source triplet 已退役删除；其历史任务和验证结果只通过 Git 与 GitHub task issue evidence 追溯。
