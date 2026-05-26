# Viewer Pixel World Semantic Positioning（2026-05-26）

- 对应设计文档: `doc/world-simulator/viewer/viewer-pixel-world-semantic-positioning-2026-05-26.design.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-pixel-world-semantic-positioning-2026-05-26.project.md`
- 关联主专题:
  - `doc/world-simulator/viewer/viewer-web-entry-visual-redesign-2026-05-12.prd.md`
  - `doc/world-simulator/viewer/viewer-pixel-world-bridge-render-optimization-2026-05-17.prd.md`

审计轮次: 1

## 1. Executive Summary
- 当前 pixel-world 已具备 wasm-only renderer、关系线、热点层和增量渲染，但实际 runtime 快照经常只提供 `agent.location_id`，不提供 agent 精确坐标。
- 当 agent 缺少坐标时，现有舞台只能把 agent 落到 renderer fallback 点，导致首屏关系线缺失，玩家难以读出 Agent 属于哪个地点。
- 本轮不改 runtime 协议，通过 Viewer host DTO 增加确定性语义定位：有精确坐标时使用 snapshot truth；缺坐标但有 `location_id` 时派生到地点附近，并标记 `position_source=location_derived`。

## 目标
- 让稀疏快照下的 pixel-world 仍能稳定显示 agent、地点和关系线。
- 把派生位置作为 Viewer 表达层语义，不伪装成 runtime 真坐标。
- 保持 `pixel_world_bridge` wasm-only 宿主合同、event contract、build/finalize 产物边界不变。

## 范围
- 范围内：
  - `crates/oasis7_viewer/software_safe_src/pixel_world_host.jsx`
  - `crates/oasis7_viewer/software_safe_src/pixel_world_host.test.jsx`
  - pixel-world semantic positioning 的 PRD / design / project / index writeback
- 范围外：
  - 新增 runtime snapshot 字段
  - 修改 `pixel_world_bridge` wasm bindgen API
  - 恢复 JS renderer fallback 或改 3D Viewer

## 接口 / 数据
- Viewer DTO:
  - `agents[].pos`
  - `agents[].position_source`
  - `agents[].status_badges`
  - `links[]`
- Runtime snapshot inputs:
  - `agent.pos`
  - `agent.location_id`
  - `location.pos`
  - `location.profile.radius_cm`
  - `snapshot.config.space`
- Renderer inputs保持不变:
  - `world_bounds`
  - `locations`
  - `agents`
  - `links`
  - `visual_hotspots`
  - `selection`

## 3. User Stories
- As a player, I want agents without exact coordinates to still appear near their assigned location, so that the world stage communicates relationships instead of sparse points.
- As a `viewer_engineer`, I want derived positions to be deterministic and labelled, so that tests can lock the rendering DTO without claiming runtime has exact agent coordinates.

## 4. Technical Specifications

### 4.1 Position Source Priority
- `snapshot`: use `agent.pos` or selected-object position when present.
- `location_derived`: if `agent.location_id` resolves to a location with position, derive a deterministic offset near that location.
- `missing`: keep `pos=null`; renderer may still use its existing fallback point.

### 4.2 Deterministic Derivation
- The derivation key is `agent.id:agent.location_id`.
- The offset angle comes from a stable string hash.
- The offset radius is bounded by world size and location radius, then clamped into `world_bounds`.
- The DTO exposes `position_source` and adds `position=location_derived` to `status_badges`.

### 4.3 Rendering Contract
- `links` are built after derived positions, so an assigned agent with no precise snapshot coordinate still produces an agent-location relationship line.
- Host fallback DOM positions use world coordinates when available, instead of index-only placement.
- The wasm renderer continues consuming the existing `agents[].pos`, `links[]`, `visual_hotspots[]`, and `selection` DTO fields.

## 5. Risks & Roadmap
- Risk: derived positions may be mistaken for authoritative runtime coordinates.
  - Mitigation: expose `position_source` and badge text, and keep runtime protocol unchanged.
- Risk: multiple agents at one location can overlap.
  - Mitigation: deterministic hash distributes nearby agents around the location; future slices can add collision relaxation if needed.

## 里程碑
- M1: PRD / design / project 建模完成，冻结 sparse snapshot semantic positioning 合同。
- M2: `pixel_world_host.jsx` 完成 `location_derived` 派生坐标、source badge 与 fallback DOM world-coordinate placement。
- M3: Targeted UI tests、software-safe build、doc governance 与 diff hygiene 通过。

## 风险
- 派生坐标被误读为 runtime 权威位置。
- 同一地点多 agent 仍可能在视觉上靠得过近。
- 若后续上游补精确坐标，需要保持 `snapshot` source 优先级，避免派生逻辑覆盖真坐标。

## 6. Acceptance Criteria
- AC-1: An agent with `location_id` but no `pos` gets a deterministic `location_derived` position in the Viewer DTO.
- AC-2: The same sparse snapshot produces a stable agent-location `links[]` entry.
- AC-3: Host fallback rendering uses world-coordinate placement when DTO positions are available.
- AC-4: Tests cover sparse snapshot semantic positioning and explicit renderer fallback behavior.

## 7. Validation & Decision Record
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-WORLD_SIMULATOR-046 | `task_4ade083740bc4d9f9f9bb742a7ce153f` | `test_tier_required` | `npm --prefix crates/oasis7_viewer run test:ui -- pixel_world_host` + `npm --prefix crates/oasis7_viewer run build:software-safe` + `./scripts/doc-governance-check.sh` + `git diff --check` | Pixel-world host DTO, sparse snapshot placement, fallback DOM positioning, generated viewer bundle |
