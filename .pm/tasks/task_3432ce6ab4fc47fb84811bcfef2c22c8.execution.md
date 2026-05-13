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
