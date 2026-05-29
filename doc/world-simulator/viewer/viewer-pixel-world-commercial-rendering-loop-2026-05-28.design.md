# Viewer Pixel World Commercial Rendering Loop 详细设计（2026-05-28）

- 对应需求文档: `doc/world-simulator/viewer/viewer-pixel-world-commercial-rendering-loop-2026-05-28.prd.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-pixel-world-commercial-rendering-loop-2026-05-28.project.md`

## Current State
- `pixel_world_host.jsx` 已从 snapshot 派生 world bounds、locations、fragment terrain、agents、links、hotspots 和 selection。
- `pixel_world_bridge` 已按 grid -> fragments -> links -> locations -> agents -> hotspots 渲染主要层级。
- PixelWorldHost 首屏仍展示 renderer/runtime/camera/debug control badges，商业玩法语义只分散在 goal/blocker callouts 与外层面板里。

## Design Decision
- 把 pixel-world 主舞台定位成低保真商业游戏棋盘，而不是 renderer status panel。
- 新增 `commercial_surface` 作为 host-only DTO，由现有 gameplay summary 和 render state 派生。
- WASM bridge 不读取该字段；它服务于 Solid host HUD 和 fallback DOM。
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
- Route lines are non-interactive; location/agent hit targets remain the only fallback selection points.

## UI Layout
- `pixel-world-host__summary`: product title and current objective detail.
- `pixel-world-command-strip`: three compact status cells for objective, next move and player leverage.
- `pixel-world-canvas`: terrain/routes/entities/callouts.
- `pixel-world-render-diagnostics`: collapsed details with counts, renderer status, camera state, control buttons and raw DTO.

## Verification
- Host unit test checks `commercial_surface` shape and active agent resolution.
- Host render test checks HUD visible text and renderer diagnostics collapsed by default.
- Existing Fragment terrain tests continue proving background/non-interactive terrain order.
- Build regenerates `viewer.js` and compat `software_safe.js`.

## Follow-up Brainstorm
- `viewer-pixel-world-player-leverage-production-readability-2026-05-28.brainstorm.md` defines the next design direction after this first commercial HUD slice.
- The recommended sequence is command lens first, production pulse second, pixel moment theater after causality and production contracts are stable.
- Any implementation of production readability should create a new `.pm` task because it likely changes runtime snapshot contract and viewer interaction behavior.
