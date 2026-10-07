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

<a id="industry-detail-target"></a>
## 产业关系与细节层级目标

未来受支持产业视图以同一权威 projection 驱动文本与舞台，组织节点、流类型/方向、吞吐/损耗、层级/阶段/状态和定位目标。World/Region/Node 控制可见集及负载：远景优先热区/主干流，近景展示配方/库存；未知数据和推断根因明确标示，保留可发现的目标定位路径。

本设计不恢复旧面板/图谱模块，不新增 DTO/runtime 协议；复用受支持渲染资产，数据扩展和交互深化须由专业 authority 确认。当前 routes 与热点只证明现行投影范围；完整产业图谱、吞吐流及 semantic zoom 仍须实现与浏览器验收证据。

### 2.1 需求承接与分配表

| 上游 requirement / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 未覆盖范围 |
| --- | --- | --- | --- | --- |
| [产业图谱目标](viewer-pixel-world-player-readable-rendering.prd.md#industry-detail-requirements) | 受支持产业 projection 的节点、流向/吞吐/损耗、层级/阶段/状态、根因定位及分层可见集 | [产业与细节目标](viewer-pixel-world-player-readable-rendering.design.md#industry-detail-target) | Runtime 数据 authority；game_visual_interaction_designer 可读性；QA 验收 | 不新增协议/资产导入；不声明完整产业图谱或 semantic zoom 已实现 |

### 11.1 验证映射表

| 上游 requirement / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source、scenario/layer、candidate/environment | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [产业图谱目标](viewer-pixel-world-player-readable-rendering.prd.md#industry-detail-requirements) | [产业与细节目标](viewer-pixel-world-player-readable-rendering.design.md#industry-detail-target) | 未来支持产业数据时验证分层节点/流/状态与目标定位、推断/未知及负载边界 | [Viewer 手册验收入口](viewer-manual.manual.md#industry-detail-acceptance)：同候选/权威数据下 external headed desktop+narrow 验证 World/Region/Node 主次信息、符号/根因/定位、未知和拥塞密度；未执行 | 未来 state/权威来源对账、console、桌面/窄屏截图、可见集和负载证据 | 当前 routes/HUD/热点不替代产业图谱或 semantic zoom 通过；本文仅映射验收方法 |

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

### Player visual feedback invariants (2026-09)

- The mobile board keeps a wrapped `Selected` chip in the upper safe area so a
  touch or keyboard selection remains tied to the visible entity.
- The compact world readout uses `LIVE` only for a connected `ready` feed;
  `REPLAY`, `NO EVENTS`, `GAP`, and `UNAVAILABLE` retain their own labels and
  status marks. None of these ambient states is a player action receipt.
- An expanded World Feed remains a bounded, scrollable context panel above the
  Next Move and Action Receipt safe area at tablet and mobile widths.
- Hotspots are read-only inspection controls. They expose their kind and label
  to keyboard and touch users, keep a short explanation open until Escape or
  an explicit close, and never submit or select a gameplay action.
- Long Next Move, blocker, and recovery copy wraps within the mobile command
  surface; secondary detail scrolls inside the reserved command area.

Current Recent Events/Feedback retain their names; they are not silently renamed by
the new projection. `#viewer-world-feed` is a source and generated-output anchor.
Runtime currently emits `receipt_ref=null`; a receipt link is rendered only for an
explicit runtime causal identity. Gap/reorg still requires snapshot reload.

## Verification
- Host unit test checks `commercial_surface` shape and active agent resolution.
- Host render test checks HUD visible text and renderer diagnostics collapsed by default.
- Existing Fragment terrain tests continue proving background/non-interactive terrain order.
- Build regenerates `viewer.js` and dist compat `software_safe.js`.

Focused visual evidence after implementation must include: desktop/mobile board
hierarchy; receipt versus ambient feed separation; unavailable/blocked/no-receipt
states; CJK/long labels; keyboard focus and Escape return. This design slice has no
browser or screenshot evidence in this design file; task evidence records GPU-enabled
WebGL2 ready and GPU-disabled explicit unavailable fallback separately.

## Follow-up Brainstorm
- `viewer-pixel-world-player-leverage-production-readability-2026-05-28.brainstorm.md` defines the next design direction after this first commercial HUD slice.
- The recommended sequence is command lens first, production pulse second, pixel moment theater after causality and production contracts are stable.
- Any implementation of production readability should create a new `.pm` task because it likely changes runtime snapshot contract and viewer interaction behavior.
