# viewer-pixel-world-fragment-lod-2026-05-27 项目管理

- 对应需求文档: `doc/world-simulator/viewer/viewer-pixel-world-fragment-lod-2026-05-27.prd.md`
- 对应设计文档: `doc/world-simulator/viewer/viewer-pixel-world-fragment-lod-2026-05-27.design.md`
- Owner role: `viewer_engineer`
- Orchestrator: `producer_system_designer`
- `.pm` task: `.pm/tasks/task_428db5366f654c5e892ac300807cb9cc.yaml`

## 任务拆解
- [x] viewer-pixel-world-fragment-lod-design (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 冻结 Fragment terrain DTO、screen-space LOD、Location logic-anchor 分层方案。 Trace: .pm/tasks/task_428db5366f654c5e892ac300807cb9cc.yaml
- [x] viewer-pixel-world-fragment-lod-red-tests (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 补 RED 测试，覆盖 host DTO、fallback DOM 与 wasm LOD helper。 Trace: .pm/tasks/task_428db5366f654c5e892ac300807cb9cc.yaml
- [x] viewer-pixel-world-fragment-lod-implementation (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 实现 host 派生、fallback DOM、wasm bridge fragment terrain 渲染和 marker 降权。 Trace: .pm/tasks/task_428db5366f654c5e892ac300807cb9cc.yaml
- [x] viewer-pixel-world-fragment-lod-regression (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 回跑前端 UI 测试、wasm check、build/doc/diff hygiene，并记录证据。 Trace: .pm/tasks/task_428db5366f654c5e892ac300807cb9cc.yaml
- [x] viewer-pixel-world-bevy-test-system (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 补 Bevy App/ECS render-probe 测试，直接断言 `Sprite` / `Transform` 层级、尺寸、透明度与 stale cache 清理。 Trace: .pm/tasks/task_428db5366f654c5e892ac300807cb9cc.yaml
- [x] viewer-pixel-world-bevy-pixel-regression (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 补 native pixel regression probe，从 Bevy World 的 `Sprite` / `Transform` 栅格化 PNG，断言 raw RGBA hash、非背景像素、各层像素覆盖与采样颜色。 Trace: .pm/tasks/task_428db5366f654c5e892ac300807cb9cc.yaml

## 依赖
- `viewer-pixel-world-semantic-positioning-2026-05-26` 提供 host DTO、agent derived position、links 与 fallback DOM 基线。
- `viewer-fragment-element-rendering` 提供 fragment profile/block 与材料表达的历史设计约束。
- `pixel_world_bridge` checked-in generated runtime 继续由 `npm --prefix crates/oasis7_viewer run build:software-safe` 统一生成。

## 状态
- 当前状态: local verification passed; closeout status 以 `.pm` task 为准。
- Canonical worktree: 以 `.pm/tasks/task_428db5366f654c5e892ac300807cb9cc.yaml` 的 `worktree_hint` 为准。
- 当前 owner: `viewer_engineer`。

## 影响文件
- `crates/oasis7_viewer/software_safe_src/pixel_world_host.jsx`
- `crates/oasis7_viewer/software_safe_src/pixel_world_host.test.jsx`
- `crates/oasis7_viewer/software_safe.html`
- `crates/pixel_world_bridge/Cargo.toml`
- `crates/pixel_world_bridge/src/lib.rs`
- `crates/pixel_world_bridge/src/render.rs`
- `scripts/viewer-pixel-world-bevy-pixel-regression.sh`
- `scripts/viewer-pixel-world-bevy-render-probe.sh`
- `doc/world-simulator/viewer/viewer-pixel-world-fragment-lod-2026-05-27.*.md`
- `doc/world-simulator/viewer/README.md`
- `doc/world-simulator/prd.index.md`

## 验证计划
- `npm --prefix crates/oasis7_viewer run test:ui -- pixel_world_host`
- `env -u RUSTC_WRAPPER cargo test -p pixel_world_bridge --lib`
- `./scripts/viewer-pixel-world-bevy-render-probe.sh`
- `./scripts/viewer-pixel-world-bevy-pixel-regression.sh`
- `./scripts/viewer-pixel-world-fragment-visual-smoke.sh`
- `env -u RUSTC_WRAPPER cargo check -p pixel_world_bridge --target wasm32-unknown-unknown`
- `npm --prefix crates/oasis7_viewer run build:software-safe`
- `./scripts/doc-governance-check.sh`
- `git diff --check`

## 进展日志
- 2026-05-27 20:05 CST: task/worktree 已由 `scripts/new-task-worktree.sh` 建立，owner role 为 `viewer_engineer`。
- 2026-05-27 20:25 CST: `claim-ready` fresh verification 通过，覆盖前端 UI、Rust unit、wasm check、software-safe build、PM/doc/diff hygiene。
- 2026-05-28 14:15 CST: 补 Bevy App/ECS render-probe 测试与脚本，新增 native `summary.json` 证据层，用来证明 fragment/location/agent 已在 Bevy World 中按预期分层渲染。
- 2026-05-28 14:27 CST: 补 Bevy pixel regression probe，输出 `pixel-summary.json`、`pixel-regression.png` 与 zoomed crop；当前 raw RGBA FNV-1a hash 为 `9268d7d9fa5a4ff6`，非背景像素 413，fragment/location/agent 像素覆盖分别为 57/100/256。
