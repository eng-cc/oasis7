# Viewer Pixel World Fragment LOD Terrain Rendering 详细设计（2026-05-27）

- 对应需求文档: `doc/world-simulator/viewer/viewer-pixel-world-fragment-lod-2026-05-27.prd.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-pixel-world-fragment-lod-2026-05-27.project.md`

## Current State
- `pixel_world_host.jsx` already derives world bounds, locations, agents, links, and hotspots from the snapshot.
- `pixel_world_bridge` currently renders grid, location sprites, agent sprites, relationship links, and hotspots.
- `Location` is still visually prominent even though the real visible world should be the fragment terrain under it.

## Design Decision
- Keep `Location` as the logical anchor and selection/link context.
- Derive Fragment terrain in the host from existing snapshot data; do not change runtime snapshot protocol.
- Render Fragment terrain as a background layer with screen-space LOD so Agent-readable zoom stays clean.

## DTO Shape
`fragment_terrain[]` entries contain:
- `id`
- `location_id`
- `pos`
- `footprint_cm`
- `dominant_compound`
- `color`
- `emphasis`

`locations[]` entries gain:
- `marker_role`: `logic_anchor` when fragment terrain exists, otherwise `primary_marker`.
- `marker_alpha`: renderer hint for de-emphasized visual weight.

## Host Algorithm
1. Build locations with normalized position and fragment block access.
2. For each block:
   - find largest `block.compounds.ppm` entry.
   - map the compound to a deterministic terrain color.
   - compute local center from `origin_cm + size_cm / 2`.
   - place it around the location center using `radius_cm` as the local half-extent when available.
   - clamp final position inside world bounds.
3. Attach terrain count and marker role to the owning location.
4. Return `fragment_terrain` with existing render state fields.

## Renderer Algorithm
1. Deserialize optional `fragment_terrain`.
2. Compute screen-space footprint:
   - use the smaller world-to-canvas scale of width/depth.
   - multiply by current camera zoom.
3. Classify:
   - `< 1.5px`: hidden.
   - `< 10px`: background.
   - `>= 10px`: detail.
4. Render fragments below links/agents/hotspots.
5. Do not add fragment hit regions in this slice; selection remains location/agent.

## Fallback DOM
- Host fallback draws non-interactive terrain cells behind location and agent buttons.
- Terrain cells are absolutely positioned using world coordinate percentages.
- Location marker remains clickable but visually subdued when `marker_role=logic_anchor`.

## Verification
- RED/GREEN host test:
  - sample snapshot with fragment blocks produces `fragment_terrain`.
  - fragment terrain carries stable compound color.
  - location marker is `logic_anchor`.
  - fallback DOM displays terrain cells behind entities.
- Rust check:
  - wasm bridge accepts new DTO fields.
  - LOD helper thresholds are unit-covered.
