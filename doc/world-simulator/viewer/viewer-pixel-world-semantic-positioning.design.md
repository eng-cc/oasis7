# Viewer Pixel World Semantic Positioning 详细设计

### 2.1 需求承接与分配表

| 上游 requirement / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [Semantic positioning acceptance](viewer-pixel-world-semantic-positioning.prd.md#semantic-positioning-acceptance) | Sparse position remains stable with provenance; links require separate explicit current authority | [Position and relation flow](#position-and-relation-projection) | Rust host projection / Viewer / QA | No inferred ownership, runtime DTO change or new browser-pass claim |

## Current State
- `pixel_world_host.jsx` passes bounded snapshot inputs to the Rust host projection and consumes `world_bounds`, `locations`, `agents`, `links`, and `visual_hotspots`.
- `pixel_world_bridge` consumes those DTO fields and renders Bevy sprites, links, hotspots, selection, pan and zoom.
- Sparse snapshots may have `location_id` without `pos`. Deterministic presentation placement addresses visibility; relationship truth remains a separate input.

## Design Decision
- Keep the semantic repair in Viewer host DTO construction.
- Do not add runtime fields or change wasm bridge API.
- Treat derived coordinates as presentation data with explicit provenance.
- Consume the existing [P1-B relation boundary](viewer-gameplay-release-experience-overhaul.design.md#p1-b-world-semantic-presentation当前切片源码与-headed-验收已完成): never derive assignment, ownership, route or trust from `location_id` or geometry alone.

<a id="position-and-relation-projection"></a>
## Data Flow
1. Build locations from snapshot positions.
2. Build `locationById`.
3. Resolve each agent position:
   - `snapshot` if `agent.pos` exists.
   - `snapshot` if selected-object alternate position exists.
   - `location_derived` if `location_id` resolves to a positioned location.
   - `missing` otherwise.
4. Independently validate the existing explicit `relation` / `assignment` envelope. Resolve a supported kind, valid status, `source_class=runtime_projection`, `freshness=current` and both positioned endpoints before building a link. Missing or invalid authority yields no link even when both positions exist.
5. Render via wasm/Rust bridge; report unavailable if the bridge cannot derive render state.

## Derivation Algorithm
- Input: `agent.id`, `agent.location_id`, `location.pos`, `location.radius_cm`, `world_bounds`.
- Hash key: `${agent.id}:${agent.location_id}`.
- Angle: `hash % 360`.
- Radius: `max(10_000cm, min(1.5% of the larger world dimension, radius_hint))`, where `radius_hint` is a positive location radius or the existing `35_000cm` fallback. This is display spacing, not a claim that the point lies within a runtime assignment radius.
- Clamp final point inside `world_bounds`.

## UI Surface
- Toolbar badge: `derived_positions=<count>`.
- Agent DTO: `position_source`.
- Agent status badges: `position=location_derived` only for derived positions.
- Rendered DOM: positions entities using world-coordinate percentages when possible.
- Relationship labels and lines retain their own source/status/freshness; a derived-position badge is never relationship evidence. Reconcile removes an invalidated old relation independently of the Agent's still-valid display point.

## Verification
- Position tests prove a sparse snapshot produces:
  - `agent.position_source === "location_derived"`
  - stable repeated derived coordinates
  - no relationship link from `location_id` alone
- Relation tests add a supported explicit current envelope and assert a stable link, then remove or invalidate kind/status/source/freshness and assert no link. Explicit position priority, missing location, radius fallback/clamp and world-coordinate DOM placement remain separate assertions.
- Existing unavailable-state test continues proving wasm runtime failure produces an explicit diagnostic state.

## Implementation and validation boundary

The existing Rust host projection is owned by `crates/pixel_world_bridge/src/host_state.rs` and `host_relation_projection.rs`; the JS host passes its bounded inputs and consumes the DTO. At source baseline `5e930face77dd284bbba6186fd997e7e63fa9164`, `host_state_tests.rs` already asserts derived position with zero links and the explicit-current relation positive/negative cases. This document correction retains those semantics; it does not add an alternate JS relation renderer or claim a new browser run.

For an implementation change, run the affected `pixel_world_bridge` host-state tests and `pixel_world_host` UI tests, then the canonical Viewer build and applicable S6 scene. Record the actual candidate and results separately; unavailable-path or position tests do not prove the complete P1-B visual matrix.

### 11.1 验证映射表

| 上游 requirement / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 验证方法、test/manual source 与环境 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [Semantic positioning acceptance](viewer-pixel-world-semantic-positioning.prd.md#semantic-positioning-acceptance) | [Position and relation flow](#position-and-relation-projection) | Position priority/derivation/missing, zero inferred links, explicit-current positive and invalid authority negative cases | [测试手册](../../../testing-manual.md) Viewer host/bridge tests and applicable S6; use `host_state_tests.rs` and `pixel_world_host` UI tests at the actual implementation candidate as described above | Implementation PR test results and scoped scene evidence | Source reviewed only in this correction; no new runtime/browser execution or complete P1-B pass |
