# Viewer Pixel World Bridge 渲染优化（2026-05-17）

- 对应项目管理文档: `doc/world-simulator/viewer/viewer-pixel-world-bridge-render-optimization-2026-05-17.project.md`
- 关联主专题:
  - `doc/world-simulator/viewer/viewer-web-entry-visual-redesign-2026-05-12.prd.md`
  - `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md`

审计轮次: 1

## 1. Executive Summary
- 保持当前 `pixel_world_bridge` 的 wasm-only 宿主合同不变，优先收口 Bevy 内核的首轮性能债。
- 将当前“每帧全量 despawn/spawn 全部 grid/location/agent”的实现改为增量更新，降低 ECS churn、命令提交和 wasm CPU 开销。
- 第一轮只做低风险 P0 优化：常驻网格、实体池、按需重建 hit region，不同时改宿主 API、world DTO 或页面交互语义。

## 目标
- 在不改变 `createPixelWorldBridge()` / `mount|update|unmount` / `camera_state_changed` / `hover_entity` / `select_entity` 契约的前提下，显著减少 `pixel_world_bridge` 每帧重建成本。
- 为后续继续做 CPU culling、spatial hit test、真正相机下沉和 shader grid 提供更稳定的代码结构。

## 2. User Experience & Functionality
- 范围内：
  - `crates/pixel_world_bridge/src/lib.rs`
  - 与 `pixel_world_bridge` 直接相关的 repo-owned build / runtime regression
  - 对应专题文档、模块主项目追踪与 task execution log
- 范围外：
  - `pixel_world_host.jsx` 宿主布局或 fallback 文案改版
  - `pixel_world_runtime_module_wasm.js` 事件协议调整
  - 新增 world DTO 字段、3D 渲染、shader/material 重写

## 范围
- 范围内：
  - `pixel_world_bridge` 的 Bevy wasm 内核结构优化。
  - 与该优化直接相关的 repo-owned wasm/unit/UI regression。
  - 对应专题文档、模块项目页 trace 与 task execution log 回写。
- 范围外：
  - 宿主页面结构或文案改版。
  - 运行时协议、world DTO 字段或 JS 事件契约调整。
  - 3D 渲染恢复、shader/material 重写或更激进的 camera 语义改造。

## 3. User Stories
- As a `viewer_engineer`, I want the embedded pixel-world wasm runtime to stop rebuilding the whole Bevy scene every frame, so that the main Web world stage can scale to denser snapshots without wasting CPU on ECS churn.
- As a `qa_engineer`, I want the optimization to preserve current runtime-visible contracts and explicit fallback behavior, so that existing wasm/runtime regressions stay valid while performance improves.

## 4. Technical Specifications

### 4.1 Current Bottlenecks
- `render_scene` 当前每帧都会：
  - 遍历并 `despawn` 所有 `PixelWorldVisual`
  - 重建 grid 线条 sprite
  - 重建全部 location / agent sprite
  - 重新分配 `hit_regions`
- `render_state` 只在 `render_version` 变化时替换，但平移/缩放动画仍会触发整场景重建。

### 4.2 P0 Optimization Slice
- 场景实体分层：
  - `PixelWorldGridVisual`: 常驻 grid 实体
  - `PixelWorldLocationVisual`: location 实体池
  - `PixelWorldAgentVisual`: agent 实体池
- 引入 runtime cache：
  - 记录上次 grid 布局 key（窗口宽高、zoom、pan）
  - 维护 `location_id -> Entity` 与 `agent_id -> Entity` 映射
  - 记录当前活跃实体集合，用于增量回收多余实体
- 更新策略：
  - grid 仅在窗口尺寸或 camera 参数变化时更新
  - location / agent 仅更新已有实体的 `Sprite` / `Transform`
  - 数据集缩小时只回收多余实体，不再先清空全部

### 4.3 Behavioral Constraints
- 以下外部行为必须保持不变：
  - `mount` / `update` / `tick` / `unmount` 返回状态
  - `camera_state_changed`
  - `hover_entity`
  - `select_entity`
  - wasm runtime 仍为唯一渲染内核，不恢复 JS fallback
- `hit_regions` 允许继续每帧重建，但必须从“场景重建副产物”切换为“基于已更新实体的轻量缓存”。

### 4.4 Out-of-Scope Follow-ups
- 不在本轮处理：
  - spatial hash / quadtree hit test
  - 将 pan/zoom 下沉到真实 `Camera2d` transform
  - procedural/shader grid
  - 渲染对象 viewport culling
- 若本轮后仍有性能压力，再拆下一 task。

## 接口 / 数据
- 保持不变的宿主接口：
  - `createPixelWorldBridge()`
  - `mount` / `update` / `tick` / `unmount`
  - `camera_state_changed`
  - `hover_entity`
  - `select_entity`
- 本轮涉及的数据与内部缓存：
  - `render_state` / `render_version`
  - grid layout key（窗口宽高、zoom、pan）
  - `location_id -> Entity` / `agent_id -> Entity`
  - `hit_regions`
- 本轮不新增任何 world DTO、宿主事件字段或 JS fallback 协议。

## 5. Risks & Roadmap
- M1：完成专题 PRD / Project 建模，冻结 P0 优化边界。
- M2：完成 `pixel_world_bridge` 的实体池与常驻 grid 改造。
- M3：补齐定向验证、task execution log 与模块主项目追踪。

## 里程碑
- M1：专题 PRD / Project 建模完成。
- M2：`pixel_world_bridge` 的 grid/entity cache 与增量 reconcile 完成。
- M3：host tests、wasm unit tests、wasm check、viewer UI regression 与文档回写完成。

### Technical Risks
- 增量更新若回收逻辑不严，容易留下 stale entity 或错位 hit region。
  - 对策：把 grid/location/agent cache 分开，并补最小单测覆盖布局 key 与实体复用语义。
- 如果把优化 scope 扩到相机语义或 shader，会显著增加回归面。
  - 对策：本轮强制限制为 P0 结构优化，不改宿主合同。

## 风险
- 增量 reconcile 若漏掉 stale entity 回收，可能出现残留 sprite 或错误命中区域。
- 测试如果只停留在 host target，会再次掩盖 wasm runner / schema 漂移问题。
- 若后续把 scope 扩大到 camera/shader 层，本专题的低风险收口边界会被破坏。

## 6. Acceptance Criteria
- AC-1: `pixel_world_bridge` 不再在每一帧先 `despawn` 再 `spawn` 全部 grid/location/agent 实体。
- AC-2: grid 实体在相机与窗口参数不变时保持常驻；location / agent 使用实体池或等价增量更新结构复用已有 entity。
- AC-3: 现有 wasm-only 宿主合同、explicit fallback contract 与 repo-owned runtime loader/host 测试语义保持不变。
- AC-4: 本轮完成 `cargo check -p pixel_world_bridge --target wasm32-unknown-unknown` 与相关前端 repo-owned 测试回归。

## 7. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-WORLD_SIMULATOR-046 | `task_40310c312e9f4681805b5b74b30cac9a` | `test_tier_required` | `env -u RUSTC_WRAPPER cargo check -p pixel_world_bridge --target wasm32-unknown-unknown` + `npm --prefix crates/oasis7_viewer run test:ui -- pixel_world` + `git diff --check` | `pixel_world_bridge` Bevy 内核增量更新、wasm runtime 宿主合同、repo-owned runtime/host 回归 |
