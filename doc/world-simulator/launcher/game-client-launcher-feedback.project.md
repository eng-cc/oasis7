# 客户端启动器反馈执行台账（当前 authority）

> 对应需求: `doc/world-simulator/launcher/game-client-launcher-feedback.prd.md`
> 对应设计: `doc/world-simulator/launcher/game-client-launcher-feedback.design.md`

## 状态

- 状态：`documented_current_authority`。
- 本轮完成：建立稳定 feedback PRD/design/project 三件套，并将 Launcher 与 world-simulator 入口改指向它。
- 本轮未做：不修改 `oasis7_client_launcher`、`oasis7_web_launcher`、runtime、DOM、UI 或测试；三个 2026-03 feedback 源三件套已在语义回填和活跃引用修复后删除，追溯仅保留在 Git 与 `.pm` task evidence。

## 任务拆解

- [x] launcher-feedback-stable-authority (PRD-WORLD_SIMULATOR-002) [test_tier_required]: 建立 feedback stable PRD/design/project 三件套。 Trace: https://github.com/eng-cc/oasis7/issues/2565 (task_720517b6203b456cb2179a6c01c700ea)
- [x] launcher-feedback-routing-cutover (PRD-WORLD_SIMULATOR-002) [test_tier_required]: 将 Launcher landing、world-simulator PRD/index/project 默认路由指向 stable authority。 Trace: https://github.com/eng-cc/oasis7/issues/2565 (task_720517b6203b456cb2179a6c01c700ea)
- [x] launcher-feedback-source-retirement (PRD-WORLD_SIMULATOR-002) [test_tier_required]: 修复活跃路由与精确索引后删除三个 2026-03 feedback 源三件套；Git 与 `.pm` task evidence 保留迁移 provenance。 Trace: https://github.com/eng-cc/oasis7/issues/2565 (task_720517b6203b456cb2179a6c01c700ea)

## 依赖

- 行为事实依赖 `oasis7_client_launcher` native/Web feedback 实现，以及 `oasis7_web_launcher` 到 chain runtime 的既有代理。
- 可见交互验收由 `testing-manual.md` S6 与 game_visual_interaction_designer 的后续规格定义；本轮没有触发该验收。

## 当前验收边界

- 文档迁移验收：`./scripts/doc-governance-check.sh`、`./scripts/readme-link-check.sh` 和 `git diff --check`。
- 行为验收不是本轮产物；未来变更按 PRD 所列 native、Web、wasm 与 S6 路径重新取证。
- S6 浏览器截图不适用：本轮没有可见表面、DOM 或行为改动。

## 后续事项

- 若要改善链不可用时的禁用态说明或就地修复 CTA，交由 game_visual_interaction_designer 定义屏幕验收，并经 TPM 协调 launcher 控制边界后另行实施。
- 若要改变 Ready 门控后的 remote/local fallback、反馈字段或 runtime 接口，需由 runtime/相关专业角色评审；不得从本文推断已获授权。
- 三个 2026-03 源三件套不再从 active repo 检索；完成态和早期决策证据仅在 Git 与 `.pm` task evidence 中追溯。
