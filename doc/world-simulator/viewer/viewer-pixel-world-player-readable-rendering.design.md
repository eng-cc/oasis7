# Viewer Pixel World Player-Readable Rendering 详细设计

- 对应需求文档: `doc/world-simulator/viewer/viewer-pixel-world-player-readable-rendering.prd.md`
- 历史任务追溯: `task_b399bf37eff94c44a300c55f5db739d3` / GitHub issue #1294；执行证据见 GitHub task issue evidence comments 与 `.pm/github-project-sync/task-archive.jsonl`。

## Current State
- `pixel_world_host.jsx` 已从 snapshot 派生 world bounds、locations、fragment terrain、agents、links、hotspots 和 selection，并挂载 World Feed/Director 终态面板。
- `pixel_world_bridge` 已按 grid -> fragments -> links -> locations -> agents -> hotspots 渲染主要层级。
- PixelWorldHost 仍保留 focus/right-panel 兼容 hooks；`#entity-search` 是当前
  Targets 过滤入口。Recent Events/Feedback 是现有反馈投影，独立 `world_feed/v1`
  由 runtime journal 提供并通过 `#viewer-world-feed` 呈现。

## Design Decision
- 把 pixel-world 主舞台定位成低保真商业游戏棋盘，而不是 renderer status panel。
- 新增 `commercial_surface` 作为 host-only DTO，由现有 gameplay summary 和 render state 派生。
- WASM bridge/Rust render-state 读取并发布该字段；它服务于 Solid host HUD 和 rendered DOM。
- renderer diagnostics、raw DTO 与模拟 fatal 控制默认折叠到诊断区。

## DTO Shape
`commercial_surface`:
- `objective.title`
- `objective.detail`
- `next_action.label`
- `next_action.detail`
- `active_agent_id`
- `player_leverage.state`
- `player_leverage.summary`
- `player_leverage.detail`
- `blocker.label`
- `blocker.detail`
- `world_read.agents`
- `world_read.routes`
- `world_read.fragments`
- `world_read.hotspots`

### Action Receipt mapping
`commercial_surface.action_receipt` follows the paired PRD's mapping from
`player_gameplay.recent_feedback`, `execution_state`, `causality_kind/detail`,
`last_world_change`, and local gameplay feedback while runtime acknowledgement is
pending. The stable fields are `action/stage/state`, `effect`, `reason`,
`hint/next`, `target_agent_id`, `effect_kind`, `delta_logical_time`, and
`delta_event_seq`. `accepted/submitted/queued/ack` are pending; only
`completed_advanced` expresses progress, while `completed_no_progress` remains a
no-progress result. Missing authoritative feedback renders `No action receipt yet`.

## Host Algorithm
1. Build existing render state fields exactly as before.
2. Resolve active agent from:
   - recommended action target agent.
   - accepted intent target if it matches an agent.
   - selected agent.
   - first visible agent.
3. Resolve next action from recommended action label, then gameplay next step, then objective.
4. Resolve player leverage from accepted intent summary and last world change/execution cause detail.
5. Count world read metrics from existing arrays.
6. Attach `commercial_surface` to render state.

## Fallback Route Rendering
- Use the same world-to-percent projection as fragment/location/agent DOM placement.
- Draw route lines after Fragment terrain and before entity buttons.
- Route lines are non-interactive; location/agent hit targets remain the only selection points.

## UI Layout
- `pixel-world-host__summary`: product title and current objective detail.
- `pixel-world-command-strip`: three compact status cells for objective, next move and player leverage.
- `pixel-world-canvas`: terrain/routes/entities/callouts.
- `pixel-world-render-diagnostics`: collapsed details with counts, renderer status, camera state, control buttons and raw DTO.
- In the default fullscreen Player HUD, desktop keeps the command strip as three compact
  cards in one row: `Objective`, `Next Move`, and `Player Leverage`. On mobile the
  `Objective` and `Player Leverage` cards share the compact first row, while `Next Move`
  occupies the second row. `More`/`Diagnostics` remains reachable as the secondary route.

## Terminal shell relationship
Pixel World is the world-board implementation surface for the target shell defined
by [`viewer-gameplay-release-experience-overhaul.prd.md`](viewer-gameplay-release-experience-overhaul.prd.md).
That PRD owns Player/Director, dock, console, Search/Quote, World Feed v1
status, and responsive/accessibility authority. This design owns only board/HUD
mapping and its receipt rendering; it does not define a second mode or a release
claim.

## World Feed visual handoff (implemented)
Render the `world_feed/v1` projection as a compact, collapsed top-right edge
overlay/ambient chip in the fullscreen Player shell. It opens on demand into a
bounded panel; responsive layouts may reposition it only to stay within the safe
viewport. Never style it as a success receipt or make it a persistent first-screen
column. The renderer consumes source-ordered events keyed by
`(world_id,reorg_epoch,event_seq)`, deduplicates by that identity, and exposes the
cursor/recovery state without sorting by local time. On the wire,
`reorg_epoch` and `event_seq` are exact non-negative decimal strings (Rust stores
`u64`; legacy numeric input remains accepted); Viewer state uses an exact-decimal
comparator and canonical ascending event order, never coercing them through
JavaScript `Number`.

| Feed state | Visual treatment | Recovery rule |
| --- | --- | --- |
| `loading` | reserved muted feed frame | retain board and receipt; show loading copy |
| `empty` | explicit no-events copy | do not hide the surface |
| `ready` | quiet ambient event list | nullable `receipt_ref` only from explicit runtime identity |
| `replay` | replay badge and bounded history copy | cursor-aware replay, not live-success styling |
| `gap`/reorg | warning state | reload cursor/snapshot; never splice unknown events |
| `unavailable` | honest unavailable state | explain next recovery action |

Current Recent Events/Feedback retain their names; they are not silently renamed by
the new projection. `#viewer-world-feed` is a source and generated-output anchor.
Runtime currently emits `receipt_ref=null`; a receipt link is rendered only for an
explicit runtime causal identity. Gap/reorg still requires snapshot reload.

## Verification
- Host unit test checks `commercial_surface` shape and active agent resolution.
- Host render test checks HUD visible text and renderer diagnostics collapsed by default.
- Existing Fragment terrain tests continue proving background/non-interactive terrain order.
- Build regenerates `viewer.js` and compat `software_safe.js`.

Focused visual evidence after implementation must include: desktop/mobile board
hierarchy; receipt versus ambient feed separation; unavailable/blocked/no-receipt
states; CJK/long labels; keyboard focus and Escape return. This design slice has no
browser or screenshot evidence in this design file; task evidence records GPU-enabled
WebGL2 ready and GPU-disabled explicit unavailable fallback separately.

## Follow-up Brainstorm
- `viewer-pixel-world-player-leverage-production-readability-2026-05-28.brainstorm.md` defines the next design direction after this first commercial HUD slice.
- The recommended sequence is command lens first, production pulse second, pixel moment theater after causality and production contracts are stable.
- Any implementation of production readability should create a new `.pm` task because it likely changes runtime snapshot contract and viewer interaction behavior.
