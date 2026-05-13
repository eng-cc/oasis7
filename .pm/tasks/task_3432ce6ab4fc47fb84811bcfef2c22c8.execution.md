# task_3432ce6ab4fc47fb84811bcfef2c22c8 Execution Log

- task_uid: task_3432ce6ab4fc47fb84811bcfef2c22c8
- title: viewer web ui automation baseline
- owner_role: viewer_engineer
- worktree_hint: /home/scc/worktrees/oasis7-viewer-web-ui-automation-baseline

<!-- Append entries using:
Example:
  ## YYYY-MM-DD HH:MM:SS CST / role_name
  - 完成内容: ...
  - 遗留事项: ...
-->

## 2026-05-13 21:05:33 CST / viewer_engineer
- 完成内容: 为 `crates/oasis7_viewer` 新增 `Vitest + @solidjs/testing-library + jsdom` UI 回归基线；将 `main.jsx` 改为“可 import 的 `AppShell`/`mountViewerApp` + 生产态自动挂载”，新增 `software_safe_src/main.test.jsx`，覆盖 `World / Targets / Command` 结构锚点、`Runtime Diagnostics` 降级面、`Agent Chat` surface 与 `Prompt Overrides` 展开后的 DOM 可达性。
- 完成内容: 更新 `crates/oasis7_viewer/package.json` / `package-lock.json` / `vitest.config.mjs` / `test/setup.js`，并把 UI 回归接入 `scripts/ci-tests.sh` 现有 `viewer software-safe contract` Node 入口；同时回写 `testing-manual.md` 与 `viewer-web-entry-visual-redesign-2026-05-12.{prd,project}.md`，使 required gate 口径与实现一致。
- 完成内容: 重新构建 `crates/oasis7_viewer/software_safe.js`，保证 source/bundle 不漂移。
- 验证结果: `npm --prefix crates/oasis7_viewer run test:ui` 通过；`node crates/oasis7_viewer/scripts/software-safe-feedback-contract.test.mjs` 通过；`npm --prefix crates/oasis7_viewer run build:software-safe` 通过；`bash -n scripts/ci-tests.sh` 通过；`./scripts/ci-tests.sh commit` 通过；`./scripts/doc-governance-check.sh` 与 `git diff --check` 通过。
- 遗留事项: 当前新增的是 repo-owned DOM/component regression，不替代 headed `agent-browser` / release strict 的真实页面回归；`Prompt Overrides` 的浏览器点击链路仍由现有 S6/release 脚本覆盖，本轮在组件层只对 toggle 后的 DOM 出现做 deterministic 断言。

## 2026-05-13 21:37:36 CST / viewer_engineer
- 完成内容: 根据 PR #215 review comment 收口 UI 测试自举细节：为 `software_safe` Vitest URL 与测试 helper 统一补 `connect=0`，避免 jsdom 环境触发 viewer websocket 连接/重连计时器；同时把 `mountViewerApp()` disposer 纳入 `afterEach` 级别 cleanup，避免跨测试遗留 `renderHook` 与 DOM side effect。
- 完成内容: 恢复 `main.jsx` 自动挂载路径的 fail-fast 语义：当真实页面缺少 `#app` 且未显式启用 `test_api=1` 时直接抛错；测试导入场景仍允许在 `test_api=1` 下先 import 再手动挂载 root。
- 完成内容: 对齐 `viewer-web-entry-visual-redesign-2026-05-12.project.md` 与 task 真值，把 `viewer-web-ui-automation-baseline` 标记为已完成，并将项目状态更新为 `done`。
- 验证结果: 待本轮 comment fix 完成后重新执行 `npm --prefix crates/oasis7_viewer run test:ui`、`node crates/oasis7_viewer/scripts/software-safe-feedback-contract.test.mjs`、`npm --prefix crates/oasis7_viewer run build:software-safe`、`./scripts/ci-tests.sh required`。
- 遗留事项: 浏览器层 headed regression 仍保持现有 `viewer-software-safe-chat-regression.sh` / `release-gate-web-strict.sh` 边界，本轮只收口 review comment 指向的 repo-owned UI/文档一致性问题。
