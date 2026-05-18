# viewer-pixel-world-bridge-render-optimization-2026-05-17 项目管理

- 对应需求文档: `doc/world-simulator/viewer/viewer-pixel-world-bridge-render-optimization-2026-05-17.prd.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [ ] viewer-pixel-world-bridge-runtime-cache (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 为 `pixel_world_bridge` 增加 grid/layout key 与 location/agent entity cache，移除 per-frame 全量清场前置。 Trace: .pm/tasks/task_40310c312e9f4681805b5b74b30cac9a.yaml
- [ ] viewer-pixel-world-bridge-incremental-reconcile (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 将 grid/location/agent 渲染改为增量更新与按需回收，保持 `mount|update|tick|unmount`、hover/select、camera 事件契约不变。 Trace: .pm/tasks/task_40310c312e9f4681805b5b74b30cac9a.yaml
- [ ] viewer-pixel-world-bridge-regression-recheck (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 回跑 wasm check、前端 repo-owned 测试与 diff hygiene，确认优化后 runtime 行为与 fallback 合同稳定。 Trace: .pm/tasks/task_40310c312e9f4681805b5b74b30cac9a.yaml

## 依赖
- `doc/world-simulator/viewer/viewer-web-entry-visual-redesign-2026-05-12.prd.md`
- `crates/pixel_world_bridge/src/lib.rs`
- `crates/oasis7_viewer/software_safe_src/pixel_world_runtime_module_wasm.js`
- `crates/oasis7_viewer/software_safe_src/pixel_world_host.jsx`
- `scripts/viewer-pixel-world-wasm-regression.sh`

## 状态
- 更新日期: 2026-05-17
- 当前状态: done
- 下一任务: `none`
- 最新完成: `viewer-pixel-world-bridge-regression-recheck`

## 实施顺序
1. 先补专题文档和模块主项目 trace，冻结本轮 P0 优化边界。
2. 再改 `pixel_world_bridge` 为常驻 grid + entity cache。
3. 最后回跑 targeted 验证并更新 execution log。

## 完成记录
- 已完成 `viewer-pixel-world-bridge-runtime-cache`：`pixel_world_bridge` 从 per-frame 全量 `despawn + spawn` 收口到 grid layout cache 与 entity cache。
- 已完成 `viewer-pixel-world-bridge-incremental-reconcile`：grid/location/agent 改为增量 reconcile 与按需回收，宿主合同保持不变。
- 已完成 `viewer-pixel-world-bridge-regression-recheck`：补齐 repo-owned wasm test runner wrapper，确认 host tests、wasm target unit tests、wasm target check 与 viewer UI regression 全部通过。
