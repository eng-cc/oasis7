# Viewer Pixel World Fragment LOD Terrain Rendering（2026-05-27）

- 对应设计文档: `doc/world-simulator/viewer/viewer-pixel-world-fragment-lod-2026-05-27.design.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-pixel-world-fragment-lod-2026-05-27.project.md`
- 关联专题:
  - `doc/world-simulator/viewer/viewer-pixel-world-semantic-positioning-2026-05-26.prd.md`
  - `doc/world-simulator/viewer/viewer-fragment-element-rendering.prd.md`

审计轮次: 1

## 1. Executive Summary
- Pixel-world 主画面应以 `location.fragment_profile.blocks.blocks` 生成的 Fragment terrain 为世界本体，而不是把 `Location` marker 当作主视觉。
- Agent 可读视角下，Fragment block 只提供地表/材料背景；只有 block 投影到足够大的屏幕尺寸时才进入细节层。
- 本轮不改 runtime snapshot 协议，不恢复 3D viewer；只扩展 Pixel-world host DTO 与 wasm bridge 渲染层。

## 2. User Experience & Functionality

## 目标
- 让 pixel-world 默认画面以 Fragment terrain 作为世界本体背景。
- 让 Agent 可读视角下的 Fragment block 自动降级为背景信息。
- 保留 Location 的逻辑锚点职责，不再把 Location marker 当作主视觉。

## 范围
- 范围内：host DTO 派生、fallback DOM terrain、wasm bridge terrain sprite、screen-space LOD helper、测试与文档。
- 范围外：snapshot 协议扩展、正式 fragment interaction、体素资产接入、dense terrain spatial index。

### In Scope
- 在 `buildPixelWorldRenderState` 中从现有 snapshot 的 `location.fragment_profile.blocks.blocks` 派生 `fragment_terrain` DTO。
- `fragment_terrain` 记录 block 的世界坐标、footprint、dominant compound/material color 与所属 location。
- `pixel_world_bridge` 在 grid 与 links/agents 之间渲染 Fragment terrain 背景层。
- LOD 基于 screen-space size，而不是硬编码某个 zoom 值：
  - 小于细节阈值时按背景色块/地表肌理渲染，不参与 hover/selection。
  - 大于细节阈值时增强 alpha/尺寸表现，为后续 block-level inspection 留接口。
- Location marker 退为逻辑锚点表达：保留 selection/link context，但视觉优先级低于 Agent 与任务热点。

### Out of Scope
- 不新增 world server 字段。
- 不实现真实体素切割、LOD chunking、quadtree picking 或 runtime binary asset format。
- 不把 Fragment block 变成正式可交互采矿/建造对象；本轮只做视觉层和 DTO 合同。
- 不接入六视图体素资产管线；那是后续 asset pipeline 任务。

## 3. User Stories
- As a player, I want the pixel-world stage to show the actual fragment terrain beneath agents, so that I can read what kind of world the agents are standing on.
- As a viewer engineer, I want Fragment block detail to fade into background at agent-readable zoom, so that terrain context does not overpower agents, facilities, routes, and goals.
- As a QA engineer, I want DTO, renderer, and pixel regression tests for fragment LOD thresholds, so that future visual work does not regress into always-on block clutter.

## 4. Technical Specifications

## 接口 / 数据
- 输入：现有 `WorldSnapshot.model.locations[*].fragment_profile.blocks.blocks`。
- 输出：Pixel-world host render state 新增 `fragment_terrain[]`，wasm bridge 使用 serde default 接收。
- 兼容：没有 `fragment_profile` 的 snapshot 继续只渲染 locations/agents/links/hotspots。

### 4.1 Fragment Terrain DTO
- Source: `WorldSnapshot.model.locations[*].fragment_profile.blocks.blocks`.
- Host derives:
  - `id`: stable `fragment:${location.id}:${index}`.
  - `location_id`.
  - `pos`: top-down world position from location position plus block local center.
  - `footprint_cm`: max of block `size_cm.x_cm` and `size_cm.z_cm`.
  - `dominant_compound`: largest `block.compounds.ppm` entry.
  - `color`: stable RGB palette keyed by dominant compound.
- Location with fragment blocks sets marker role to `logic_anchor` and uses a smaller marker hint.

### 4.2 Screen-Space LOD
- Renderer computes approximate fragment footprint in pixels from:
  - `footprint_cm`
  - `world_bounds`
  - canvas dimensions
  - current camera zoom
- LOD buckets:
  - `hidden`: below minimum terrain visibility.
  - `background`: visible terrain context, no block border or picking.
  - `detail`: stronger block visual, still no selection in this slice.
- The threshold is screen-space, so auto-fit, user zoom, and viewport size all affect the result naturally.

### 4.3 Layering
- Grid: lowest reference layer.
- Fragment terrain: world background layer.
- Location marker: logic anchor, subdued.
- Links/hotspots/agents: foreground gameplay layers.

## 5. Risks & Roadmap

## 里程碑
- M1：专题 PRD/design/project 与 RED 测试完成。
- M2：host DTO、fallback DOM 与 wasm terrain renderer 完成。
- M3：targeted tests、wasm check、build 与治理检查完成。

## 风险
- Dense fragment snapshots 可能增加 sprite 数量；本轮先保持非交互背景层，后续再做 culling/aggregation。
- Palette 只是工程色板，不代表最终美术；后续可替换颜色但保持 DTO 形状。
- 直接 wasm-target unit runner 在本机可能不稳定；以 native helper test + wasm check + Web runtime build 作为本轮验证组合。

- Risk: adding every block as a sprite may add wasm ECS churn on dense snapshots.
  - Mitigation: first slice keeps fragment terrain non-interactive and simple; future dense snapshots can add aggregation/culling.
- Risk: compound palette may not match final art direction.
  - Mitigation: palette is deterministic and centralized, so art can replace it without changing DTO shape.
- Risk: sparse snapshots may not include `fragment_profile`.
  - Mitigation: DTO is optional and existing marker/agent rendering remains valid.

## 6. Acceptance Criteria
- AC-1: Pixel-world render state includes `fragment_terrain` derived from existing location fragment blocks.
- AC-2: A location with fragment terrain is marked as a subdued `logic_anchor` marker instead of the main visual entity.
- AC-3: Wasm bridge accepts and renders fragment terrain without breaking existing `mount|update|tick|unmount`, hover/select, and camera contracts.
- AC-4: LOD classification is based on computed screen-space footprint and covers hidden/background/detail thresholds.
- AC-5: Host UI tests, native `pixel_world_bridge` Rust tests, Bevy render-probe evidence, Bevy pixel regression evidence, browser visual smoke, wasm `cargo check`, build, and doc/diff hygiene checks pass.

## 7. Validation & Decision Record
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-WORLD_SIMULATOR-046 | `task_428db5366f654c5e892ac300807cb9cc` | `test_tier_required` | `npm --prefix crates/oasis7_viewer run test:ui -- pixel_world_host` + `env -u RUSTC_WRAPPER cargo test -p pixel_world_bridge --lib` + `./scripts/viewer-pixel-world-bevy-render-probe.sh` + `./scripts/viewer-pixel-world-bevy-pixel-regression.sh` + `./scripts/viewer-pixel-world-fragment-visual-smoke.sh` + `env -u RUSTC_WRAPPER cargo check -p pixel_world_bridge --target wasm32-unknown-unknown` + `./scripts/doc-governance-check.sh` + `git diff --check` | Pixel-world host DTO、fragment terrain LOD、Bevy ECS visual reconciliation、Bevy pixel regression、wasm bridge render state schema、fallback DOM |
