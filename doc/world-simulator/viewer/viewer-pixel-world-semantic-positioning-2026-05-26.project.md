# viewer-pixel-world-semantic-positioning-2026-05-26 项目管理

- 对应需求文档: `doc/world-simulator/viewer/viewer-pixel-world-semantic-positioning-2026-05-26.prd.md`
- 对应设计文档: `doc/world-simulator/viewer/viewer-pixel-world-semantic-positioning-2026-05-26.design.md`
- 对应 `.pm` task: `.pm/tasks/task_4ade083740bc4d9f9f9bb742a7ce153f.yaml`

## 状态
- [x] viewer-pixel-world-semantic-positioning-design (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 冻结 sparse snapshot 下 agent 坐标派生、source 标记、关系线与 fallback DOM 定位合同。 Trace: .pm/tasks/task_4ade083740bc4d9f9f9bb742a7ce153f.yaml
- [x] viewer-pixel-world-location-derived-agent-position (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 在 `pixel_world_host.jsx` 中实现 agent `location_derived` 坐标、DTO source badge、world-coordinate fallback positioning。 Trace: .pm/tasks/task_4ade083740bc4d9f9f9bb742a7ce153f.yaml
- [x] viewer-pixel-world-semantic-positioning-regression (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 补 sparse snapshot Vitest，验证派生坐标稳定、关系线生成、renderer fallback contract 保持。 Trace: .pm/tasks/task_4ade083740bc4d9f9f9bb742a7ce153f.yaml

## 任务拆解
1. Detailed design: 描述当前 pixel-world pipeline、sparse snapshot 风险、DTO source priority、派生算法和验证计划。
2. Host DTO implementation: 在 `pixel_world_host.jsx` 中将 agent 坐标解析改为 snapshot-first、location-derived-second、missing-last。
3. Fallback rendering implementation: 让 host fallback DOM 使用 world-coordinate percentage placement，而不是只按列表 index 排布。
4. Regression: 用 targeted Vitest 锁定 sparse snapshot 派生坐标、关系线和 explicit fallback surface。

## 依赖
- 上游输入依赖: runtime snapshot 中已有 `agent.location_id`、`location.pos`、`snapshot.config.space`。
- 渲染依赖: `pixel_world_bridge` 继续消费现有 DTO，不要求 wasm API 变化。
- 验证依赖: `crates/oasis7_viewer` npm dependencies and repo-owned doc governance scripts.

## 产物文件
- `crates/oasis7_viewer/software_safe_src/pixel_world_host.jsx`
- `crates/oasis7_viewer/software_safe_src/pixel_world_host.test.jsx`
- `doc/world-simulator/viewer/viewer-pixel-world-semantic-positioning-2026-05-26.prd.md`
- `doc/world-simulator/viewer/viewer-pixel-world-semantic-positioning-2026-05-26.design.md`
- `doc/world-simulator/viewer/viewer-pixel-world-semantic-positioning-2026-05-26.project.md`

## 验收命令
- `npm --prefix crates/oasis7_viewer run test:ui -- pixel_world_host`
- `npm --prefix crates/oasis7_viewer run build:software-safe`
- `./scripts/doc-governance-check.sh`
- `git diff --check`
