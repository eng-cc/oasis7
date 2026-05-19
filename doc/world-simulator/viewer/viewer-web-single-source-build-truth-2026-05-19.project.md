# viewer-web-single-source-build-truth-2026-05-19 项目管理

- 对应设计文档: `doc/world-simulator/viewer/viewer-web-single-source-build-truth-2026-05-19.design.md`
- 对应需求文档: `doc/world-simulator/viewer/viewer-web-single-source-build-truth-2026-05-19.prd.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] viewer-web-legacy-core-module-split (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 将 `legacy_core.js` 收口为 facade，并把 state/auth/gameplay/rendering 主实现拆到 `software_safe_src/` 子模块。 Trace: .pm/tasks/task_97820fd5e09a450aadcf988a968faad8.yaml
- [x] viewer-web-canonical-bundle-truth (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 将 canonical Viewer Web bundle 真值收口为 `viewer.js`，把 `software_safe.js` 改成显式 compat alias。 Trace: .pm/tasks/task_97820fd5e09a450aadcf988a968faad8.yaml
- [x] viewer-web-generated-runtime-flow-sync (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 对齐 `pixel-world-bridge/` generated runtime 与 dist / bundle / freshness helper，统一 canonical -> compat 复制方向。 Trace: .pm/tasks/task_97820fd5e09a450aadcf988a968faad8.yaml
- [x] viewer-web-regression-recheck (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 回跑 UI tests、build、repo-owned Node contract test 与 diff hygiene，确认拆分与 build flow 不回归。 Trace: .pm/tasks/task_97820fd5e09a450aadcf988a968faad8.yaml

## 范围备注
- 代码：
  - `crates/oasis7_viewer/software_safe_src/**`
  - `crates/oasis7_viewer/scripts/finalize-software-safe-build.mjs`
  - `crates/oasis7_viewer/viewer.js`
  - `crates/oasis7_viewer/software_safe.js`
  - `crates/oasis7_viewer/pixel-world-bridge/**`
  - `scripts/run-viewer-web.sh`
  - `scripts/agent-browser-lib.sh`
  - `scripts/build-game-launcher-bundle.sh`
  - `scripts/bundle-freshness-lib.sh`
- 验证：
  - `npm --prefix crates/oasis7_viewer run test:ui`
  - `npm --prefix crates/oasis7_viewer run build:software-safe`
  - `node crates/oasis7_viewer/scripts/software-safe-feedback-contract.test.mjs`
  - `git diff --check`

## 依赖
- `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.{prd,design}.md`：提供 viewer canonical/compat 命名约束。
- `crates/oasis7_viewer/scripts/finalize-software-safe-build.mjs`：作为 generated bundle 与 `pixel-world-bridge/` runtime 的唯一写入口。
- `scripts/{run-viewer-web.sh,agent-browser-lib.sh,build-game-launcher-bundle.sh,bundle-freshness-lib.sh}`：需要同步消费 canonical `viewer.js`。

## 状态
- 更新日期: 2026-05-19
- 当前状态: local_verification_passed
- 当前任务: `viewer-web-regression-recheck`
- 说明: 本专题已在本地完成模块拆分、canonical bundle flow 收口与回归验证；仍待 task closeout / commit / PR 流程。
