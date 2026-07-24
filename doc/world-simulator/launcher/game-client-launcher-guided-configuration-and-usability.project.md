# 客户端启动器引导配置与可用性执行台账（当前 authority）

> 对应需求: `doc/world-simulator/launcher/game-client-launcher-guided-configuration-and-usability.prd.md`
> 对应设计: `doc/world-simulator/launcher/game-client-launcher-guided-configuration-and-usability.design.md`

## 状态

- 状态：`documented_current_authority`。
- 本轮完成：建立 stable launcher PRD/design/project 三件套，修复 launcher landing、world-simulator 索引和模块项目入口，并删除四组已吸收的日期化 source triplet。
- 本轮未做：不修改 launcher、runtime、Web UI、DOM、测试或运行脚本；不把历史完成记录解释为当前行为、可用性或发布结论。

## 任务拆解

- [x] launcher-guided-configuration-stable-authority (PRD-WORLD_SIMULATOR-027/033/034) [test_tier_required]: 收敛既有引导配置、可诊断状态、runtime 托管路径和 stale execution-world 恢复边界。 Trace: https://github.com/eng-cc/oasis7/issues/2583 (task_6840e029d65f43e7b9f58f8631fd40be)
- [x] launcher-guided-configuration-routing-cutover (PRD-WORLD_SIMULATOR-027/033/034) [test_tier_required]: 将 Launcher landing、world-simulator PRD index/project 默认路由指向 stable authority。 Trace: https://github.com/eng-cc/oasis7/issues/2583 (task_6840e029d65f43e7b9f58f8631fd40be)
- [x] launcher-guided-configuration-source-retirement (PRD-WORLD_SIMULATOR-027/033/034) [test_tier_required]: 完成语义回填与活跃引用修复，删除四组已吸收的历史 source triplet；追溯使用 Git 与 GitHub task issue evidence。 Trace: https://github.com/eng-cc/oasis7/issues/2583 (task_6840e029d65f43e7b9f58f8631fd40be)

## 依赖与验收边界

- 当前行为事实依赖 launcher 表现层、既有控制面与 runtime；具体实现和操作路径必须在需要时重新核验。
- 文档路由依赖 `doc/world-simulator/launcher/README.md`、`doc/world-simulator/prd.index.md` 与 `doc/world-simulator/project.md`。
- 文档迁移验收：`./scripts/doc-governance-check.sh`、`./scripts/readme-link-check.sh`、`git diff --check`。
- S6 截图不适用：本轮不改变可见表面、DOM、交互或行为。未来可见变更按 `testing-manual.md` S6 取证，视觉/交互验收由 game_visual_interaction_designer 定义。

## 后续事项

- 任何当前启动、路径、恢复、runtime 一致性、数据删除、provider、hosted/login、network readiness 或发布结论，均需独立任务和当期专业证据授权。
- 四组 source triplet 已退役删除；其历史任务和验证结果只通过 Git 与 GitHub task issue evidence 追溯。
