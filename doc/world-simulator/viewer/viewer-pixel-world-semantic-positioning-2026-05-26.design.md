# Viewer Pixel World Semantic Positioning 详细设计（2026-05-26）

## Current State
- `pixel_world_host.jsx` already converts snapshot state into `world_bounds`, `locations`, `agents`, `links`, and `visual_hotspots`.
- `pixel_world_bridge` consumes those DTO fields and renders Bevy sprites, links, hotspots, selection, pan and zoom.
- The weak point is sparse runtime snapshots: many agents have `location_id` but no `pos`, so relationships disappear before the renderer sees the data.

## Design Decision
- Keep the semantic repair in Viewer host DTO construction.
- Do not add runtime fields or change wasm bridge API.
- Treat derived coordinates as presentation data with explicit provenance.

## Data Flow
1. Build locations from snapshot positions.
2. Build `locationById`.
3. Resolve each agent position:
   - `snapshot` if `agent.pos` exists.
   - `snapshot` if selected-object fallback position exists.
   - `location_derived` if `location_id` resolves to a positioned location.
   - `missing` otherwise.
4. Build links from the resolved agent positions.
5. Render via wasm or explicit host fallback.

## Derivation Algorithm
- Input: `agent.id`, `agent.location_id`, `location.pos`, `location.radius_cm`, `world_bounds`.
- Hash key: `${agent.id}:${agent.location_id}`.
- Angle: `hash % 360`.
- Radius: clamped between `10_000cm`, `1.5%` of the larger world dimension, and `location.radius_cm` when available.
- Clamp final point inside `world_bounds`.

## UI Surface
- Toolbar badge: `derived_positions=<count>`.
- Agent DTO: `position_source`.
- Agent status badges: `position=location_derived` only for derived positions.
- Host fallback DOM: positions entities using world-coordinate percentages when possible.

## Verification
- Unit/UI test proves a sparse snapshot produces:
  - `agent.position_source === "location_derived"`
  - stable repeated derived coordinates
  - one relationship link from derived agent position to assigned location
- Existing fallback test continues proving wasm runtime failure produces explicit fallback surface.
