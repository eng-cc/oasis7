# viewer-pixel-world-commercial-rendering-loop-2026-05-28 项目管理

- 对应需求文档: `doc/world-simulator/viewer/viewer-pixel-world-commercial-rendering-loop-2026-05-28.prd.md`
- 对应设计文档: `doc/world-simulator/viewer/viewer-pixel-world-commercial-rendering-loop-2026-05-28.design.md`
- Owner role: `producer_system_designer`
- Implementation slice: `viewer_engineer`
- `.pm` task: `.pm/tasks/task_b399bf37eff94c44a300c55f5db739d3.yaml`

## 任务拆解
- [x] viewer-pixel-world-commercial-design (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 冻结商业化 pixel-world 信息架构、player leverage 口径与非目标。 Trace: .pm/tasks/task_b399bf37eff94c44a300c55f5db739d3.yaml
- [x] viewer-pixel-world-commercial-host-dto (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 为 host render state 增加 `commercial_surface`，从现有 gameplay summary / agents / links / fragments 派生目标、下一步和玩家杠杆。 Trace: .pm/tasks/task_b399bf37eff94c44a300c55f5db739d3.yaml
- [x] viewer-pixel-world-commercial-ui (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 重排 PixelWorldHost 首屏 HUD，默认折叠 renderer diagnostics，并补 fallback route 表达。 Trace: .pm/tasks/task_b399bf37eff94c44a300c55f5db739d3.yaml
- [x] viewer-pixel-world-commercial-regression (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 回跑 targeted UI test、software-safe build、PM/doc/diff hygiene，并记录执行证据。 Trace: .pm/tasks/task_b399bf37eff94c44a300c55f5db739d3.yaml

## 依赖
- `viewer-web-entry-visual-redesign-2026-05-12` 提供世界优先入口结构。
- `viewer-pixel-world-semantic-positioning-2026-05-26` 提供 agent derived position 与 links。
- `viewer-pixel-world-fragment-lod-2026-05-27` 提供 Fragment terrain 背景层与 Location logic-anchor 口径。

## 状态
- 当前状态: local implementation complete; pending PR closeout。
- Canonical worktree: `worktrees/oasis7-world-simulator-commercial-pixel-world-rendering-loop`。
- 当前 owner: `producer_system_designer`。

## 影响文件
- `crates/oasis7_viewer/software_safe_src/pixel_world_host.jsx`
- `crates/oasis7_viewer/software_safe_src/pixel_world_host.test.jsx`
- `crates/oasis7_viewer/software_safe.html`
- `crates/oasis7_viewer/viewer.js`
- `crates/oasis7_viewer/software_safe.js`
- `doc/world-simulator/viewer/viewer-pixel-world-commercial-rendering-loop-2026-05-28.*.md`
- `doc/world-simulator/viewer/README.md`
- `doc/world-simulator/prd.md`
- `doc/world-simulator/project.md`
- `doc/world-simulator/prd.index.md`

## 验证计划
- `npm --prefix crates/oasis7_viewer run test:ui -- pixel_world_host`
- `npm --prefix crates/oasis7_viewer run build:software-safe`
- `./scripts/doc-governance-check.sh`
- `./scripts/pm/lint.sh`
- `git diff --check`

## 进展日志
- 2026-05-28 16:02 CST: task/worktree 已由 `scripts/new-task-worktree.sh` 建立，owner role 为 `producer_system_designer`。
- 2026-05-28 16:17 CST: `commercial_surface` host DTO、商业化 HUD、fallback route line 与 renderer diagnostics 折叠已实现；`pixel_world_host` targeted UI tests 通过。
- 2026-05-28 16:18 CST: `build:software-safe` 已刷新 checked-in viewer artifact；desktop/mobile browser smoke 通过，移动端无横向溢出且 diagnostics 默认折叠。
