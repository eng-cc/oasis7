# Viewer Pixel World Bridge 渲染优化设计（2026-05-17）

- 对应需求文档: `doc/world-simulator/viewer/viewer-pixel-world-bridge-render-optimization-2026-05-17.prd.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-pixel-world-bridge-render-optimization-2026-05-17.project.md`

## 1. 设计定位
将 `pixel_world_bridge` 从“每帧重建整个 Bevy scene”的临时实现，收口到可持续扩展的增量更新结构，同时保持当前 Web 宿主合同与 wasm-only 出货边界不变。

## 2. 设计结构
- 共享态同步层：继续沿用 `render_version + input_events` 的宿主同步模式。
- runtime cache 层：新增 grid key、location entity map、agent entity map。
- scene update 层：从“全量重建”切到“按类型增量更新 + 按需回收”。
- hit-test cache 层：保留现有矩形命中模型，但与实体池更新同帧重建。

## 3. 关键接口 / 入口
- `BevyRuntimeState`
- `render_scene`
- `spawn_grid`（重构为 grid reconciliation）
- location / agent sprite 更新路径

## 4. 约束与边界
- 不改 `pixel_world_runtime_module_wasm.js` 的宿主消费方式。
- 不恢复 `pixel_world_bevy_bridge.js` JS renderer fallback。
- 不引入新的 render DTO 字段。
- 不在本轮切换到 camera transform / shader grid / picking plugin。

## 5. 演进计划
1. 先拆实体类型与 runtime cache。
2. 再实现 grid/location/agent 的增量 reconcile。
3. 最后补最小测试与 task/log 收口。
