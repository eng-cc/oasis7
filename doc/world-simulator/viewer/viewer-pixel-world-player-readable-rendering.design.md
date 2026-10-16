# Viewer Pixel World Player-Readable Rendering 详细设计

- 对应需求文档: `doc/world-simulator/viewer/viewer-pixel-world-player-readable-rendering.prd.md`
- 历史任务追溯: `task_b399bf37eff94c44a300c55f5db739d3` / GitHub issue #1294；执行证据见 GitHub task issue evidence comments 与 `.pm/github-project-sync/task-archive.jsonl`。

## Current State
- `pixel_world_host.jsx` 将 bounded snapshot 输入交给 Rust host projection，并消费其 world bounds、locations、fragment terrain、agents、links、hotspots、selection 与 commercial_surface；World Feed/Director 面板各保留原 owner。
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

Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；局部承接[REQ-AGENT-STAGE-001](../../product/agents-world-simulation/player-readable-world-stage.prd.md#req-agent-stage-001)：以下仅为产业节点、路线和 blocker 的持续可读性目标；现有 HUD 不代签完成。

未来受支持产业视图以同一权威 projection 驱动文本与舞台，组织节点、流类型/方向、吞吐/损耗、层级/阶段/状态和定位目标。

<a id="industry-scale-target"></a>

Trace refs: O7-REQ-2ede7851-f1c0-4851-8d6b-5da4afcdefcf；局部承接[REQ-AGENT-STAGE-003](../../product/agents-world-simulation/player-readable-world-stage.prd.md#req-agent-stage-003)：World/Region/Node 可见集与目标定位仅是多尺度浏览的有限设计目标；不是完整探索或 semantic zoom 已实现。

World/Region/Node 控制可见集及负载：远景优先热区/主干流，近景展示配方/库存；未知数据和推断根因明确标示，保留可发现的目标定位路径。

本设计不恢复旧面板/图谱模块，不新增 DTO/runtime 协议；复用受支持渲染资产，数据扩展和交互深化须由专业 authority 确认。当前 routes 与热点只证明现行投影范围；完整产业图谱、吞吐流及 semantic zoom 仍须实现与浏览器验收证据。

### 2.1 需求承接与分配表

| 上游 requirement / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 未覆盖范围 |
| --- | --- | --- | --- | --- |
| Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；[专业条款](viewer-gameplay-release-experience-overhaul.prd.md#current-implementation-boundary) | 单一 Next Move 主区、Objective / Player Leverage 从属；窄屏第一行 | [设计条款](#ui-layout) | Viewer / visual design；shell authority | 不新增布局决策；源码或 mock 不证明 S6/完整可访问性 |
| Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；[专业条款](viewer-pixel-world-player-readable-rendering.prd.md#industry-detail-requirements) | 未来受支持产业节点、路线、吞吐/损耗、状态、blocker 与根因可读 | [设计条款](#industry-detail-target) | Runtime projection / Viewer / visual design / QA | 现有 routes/HUD 不证明完整产业图谱 |
| Trace refs: O7-REQ-2ede7851-f1c0-4851-8d6b-5da4afcdefcf；[专业条款](viewer-pixel-world-player-readable-rendering.prd.md#industry-detail-requirements) | 未来 World/Region/Node 可见集、目标定位与节点/边负载 | [设计条款](#industry-scale-target) | Runtime projection / Viewer / QA | 不证明 semantic zoom、完整探索或授权搜索/返回任务 |
| Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；[专业条款](viewer-pixel-world-player-readable-rendering.prd.md#41-commercial-surface-dto) | 现有 snapshot 派生 commercial_surface，保持 active-agent 优先级和 ambient 不造进度 | [设计条款](#dto-shape) | Rust host projection owns derivation；JS host consumes | mock loader 的 JS buildTestRustRenderState 不是执行 Rust/wasm；本次未运行 |
| Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；[专业条款](viewer-pixel-world-player-readable-rendering.prd.md#411-action-receipt-dtostate-mapping) | 仅权威因果 receipt；pending、advanced、no-progress、blocked、无回执分别呈现 | [设计条款](#action-receipt-mapping) | Runtime causality / Rust projection / Viewer | World Feed 不能补 receipt；不证明完整世界执行与恢复全链 |
| Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；[专业条款](viewer-pixel-world-player-readable-rendering.prd.md#412-world-feed-v1-implementation-contract) | Feed source order/三元去重/exact-decimal、cursor 与 gap/reorg reload；loading/empty/replay/unavailable、nullable receipt_ref | [设计条款](#world-feed-visual-handoff-implemented) | Runtime journal / transport / Viewer panel | 本次未运行 Feed/runtime 全链；环境事件与 LIVE 不代签玩家成功 |
| Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；[专业条款](viewer-pixel-world-player-readable-rendering.prd.md#412-world-feed-v1-implementation-contract) | 移动 Selected 安全区、bounded Feed、只读 hotspot、长 CJK/键盘/Escape、诚实 unavailable | [设计条款](#player-visual-feedback-invariants-2026-09) | Viewer / visual design / QA | mock、DOM 与 fallback 不代签真实 GPU/全可访问性或发行 |
| Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；[专业条款](viewer-pixel-world-player-readable-rendering.prd.md#42-ui-layering) | 现有 world-coordinate route 介于 Fragment 与实体之间且只读；不增加 terrain/route 命中权限 | [设计条款](#fallback-route-rendering) | Runtime relation truth / Rust projection / Viewer | 不从坐标补关系；不新增控制权或证明完整产业图谱 |

### 11.1 验证映射表

| 上游 requirement / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source、scenario/layer、candidate/environment | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；[专业条款](viewer-gameplay-release-experience-overhaul.prd.md#current-implementation-boundary) | [设计条款](#ui-layout) | 单一 Next Move 主区、Objective / Player Leverage 从属；窄屏第一行 | 同候选静态核对 host 与 shell CSS；`../../../crates/oasis7_viewer/software_safe_src/pixel_world_host.test.jsx` 的 HUD 断言；可见改动另按 [S6 手册](../../testing/manual/web-ui-agent-browser-closure-manual.manual.md) 取 desktop/narrow 主次与键盘证据 | source/diff、mock host 结果与真实 headed 截图分别记录 | 不新增布局决策；源码或 mock 不证明 S6/完整可访问性 |
| Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；[专业条款](viewer-pixel-world-player-readable-rendering.prd.md#industry-detail-requirements) | [设计条款](#industry-detail-target) | 未来受支持产业节点、路线、吞吐/损耗、状态、blocker 与根因可读 | [产业手册](viewer-manual.manual.md#industry-detail-acceptance)；同候选权威 projection 下 headed desktop/narrow 验证节点/流/未知/推断与目标定位 | 未来 snapshot 对账、截图、console、密度与根因证据；本次未执行 | 现有 routes/HUD 不证明完整产业图谱 |
| Trace refs: O7-REQ-2ede7851-f1c0-4851-8d6b-5da4afcdefcf；[专业条款](viewer-pixel-world-player-readable-rendering.prd.md#industry-detail-requirements) | [设计条款](#industry-scale-target) | 未来 World/Region/Node 可见集、目标定位与节点/边负载 | [产业手册](viewer-manual.manual.md#industry-detail-acceptance)；同候选比较远景热区/主干、区域关系与近景配方/库存、拥塞密度和目标可达 | 未来分层可见集、截图与负载证据；本次未执行 | 不证明 semantic zoom、完整探索或授权搜索/返回任务 |
| Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；[专业条款](viewer-pixel-world-player-readable-rendering.prd.md#41-commercial-surface-dto) | [设计条款](#dto-shape) | 现有 snapshot 派生 commercial_surface，保持 active-agent 优先级和 ambient 不造进度 | [测试手册](../../../testing-manual.md) Viewer host/bridge 分层入口； `../../../crates/pixel_world_bridge/src/host_state_tests.rs` 的 DTO/active-agent 断言与 `../../../crates/oasis7_viewer/software_safe_src/pixel_world_host.test.jsx` 分层运行；核对 `host_state.rs` 和 runtime.deriveRenderState 接线 | 固定 candidate 的 Rust 与 mock host 输出分别记录 | mock loader 的 JS buildTestRustRenderState 不是执行 Rust/wasm；本次未运行 |
| Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；[专业条款](viewer-pixel-world-player-readable-rendering.prd.md#411-action-receipt-dtostate-mapping) | [设计条款](#action-receipt-mapping) | 仅权威因果 receipt；pending、advanced、no-progress、blocked、无回执分别呈现 | `../../../crates/pixel_world_bridge/src/host_state_tests.rs` 与 `../../../crates/oasis7_viewer/software_safe_src/pixel_world_host.test.jsx` 的 receipt 断言；[S6 手册](../../testing/manual/web-ui-agent-browser-closure-manual.manual.md) 同候选验证无回执、queued、advanced、no-progress、blocked 与恢复提示 | 真实输入/DTO、分层测试结果、同候选 receipt 截图 | World Feed 不能补 receipt；不证明完整世界执行与恢复全链 |
| Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；[专业条款](viewer-pixel-world-player-readable-rendering.prd.md#412-world-feed-v1-implementation-contract) | [设计条款](#world-feed-visual-handoff-implemented) | Feed source order/三元去重/exact-decimal、cursor 与 gap/reorg reload；loading/empty/replay/unavailable、nullable receipt_ref | 核对 `world_feed_state.js` 与 transport/panel；`../../../crates/oasis7_viewer/software_safe_src/pixel_world_host.test.jsx` 的 Feed 状态断言；[S6 手册](../../testing/manual/web-ui-agent-browser-closure-manual.manual.md) 同候选验证有序/重放/恢复及与 receipt 隔离 | 输入身份/游标、分层状态测试输出、真实页面恢复证据 | 本次未运行 Feed/runtime 全链；环境事件与 LIVE 不代签玩家成功 |
| Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；[专业条款](viewer-pixel-world-player-readable-rendering.prd.md#412-world-feed-v1-implementation-contract) | [设计条款](#player-visual-feedback-invariants-2026-09) | 移动 Selected 安全区、bounded Feed、只读 hotspot、长 CJK/键盘/Escape、诚实 unavailable | `../../../crates/oasis7_viewer/software_safe_src/pixel_world_host.test.jsx` 的 unavailable、诊断折叠、CJK/keyboard 断言；[S6 手册](../../testing/manual/web-ui-agent-browser-closure-manual.manual.md) 同候选 desktop/mobile 检查 touch、focus、Escape、溢出及 GPU 可用/不可用分支 | mock host、真实 headed focus/截图/console 与 GPU 状态分别记录 | mock、DOM 与 fallback 不代签真实 GPU/全可访问性或发行 |
| Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；[专业条款](viewer-pixel-world-player-readable-rendering.prd.md#42-ui-layering) | [设计条款](#fallback-route-rendering) | 现有 world-coordinate route 介于 Fragment 与实体之间且只读；不增加 terrain/route 命中权限 | [测试手册](../../../testing-manual.md) Viewer host/bridge 与 S6：同候选核 `pixel_world_host.test.jsx` 的 route 非交互断言；Fragment/route 层级、坐标与选择目标另按实际源码及真实输入验收，不声称该测试文件已覆盖完整层级矩阵 | mock DOM、Rust projection 与真实 headed 输入证据分别记录；本次未执行 | 不从坐标补关系；不新增控制权或证明完整产业图谱 |

## DTO Shape

Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；局部承接[REQ-AGENT-STAGE-001](../../product/agents-world-simulation/player-readable-world-stage.prd.md#req-agent-stage-001)：现有目的、下一步、玩家杠杆与 blocker 的 HUD 表达；实际派生 owner 是 Rust，JS 只消费。

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

Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；局部承接[REQ-AGENT-STAGE-001](../../product/agents-world-simulation/player-readable-world-stage.prd.md#req-agent-stage-001)：因果反馈解释与下一步；pending 不等于完成，环境活动不补因果。

`commercial_surface.action_receipt` follows the paired PRD's mapping from
`player_gameplay.recent_feedback`, `execution_state`, `causality_kind/detail`,
`last_world_change`, and local gameplay feedback while runtime acknowledgement is
pending. The stable fields are `action/stage/state`, `effect`, `reason`,
`hint/next`, `target_agent_id`, `effect_kind`, `delta_logical_time`, and
`delta_event_seq`. `accepted/submitted/queued/ack` are pending; only
`completed_advanced` expresses progress, while `completed_no_progress` remains a
no-progress result. Missing authoritative feedback renders `No action receipt yet`.

## Host Algorithm

以下步骤由既有 Rust host projection 执行；`pixel_world_host.jsx` 通过 runtime.deriveRenderState 消费结果，不新建 JS 世界投影实现。

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

Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；局部承接[REQ-AGENT-STAGE-001](../../product/agents-world-simulation/player-readable-world-stage.prd.md#req-agent-stage-001)：仅保留受支持关系的空间可读层级，不由位置创造关系，不赋予新的选择或行动权限。

- Use the same world-to-percent projection as fragment/location/agent DOM placement.
- Draw route lines after Fragment terrain and before entity buttons.
- Route lines are non-interactive; location/agent hit targets remain the only selection points.

## UI Layout

Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；局部承接[REQ-AGENT-STAGE-001](../../product/agents-world-simulation/player-readable-world-stage.prd.md#req-agent-stage-001)：当前 shell 主次子集，不建立第二份布局 authority。

- `pixel-world-host__summary`: product title and current objective detail.
- `pixel-world-command-strip`: maps the objective, next move and player-leverage DTO fields into the [canonical Player shell layout](viewer-gameplay-release-experience-overhaul.prd.md#current-implementation-boundary), not three peer cards.
- `pixel-world-canvas`: terrain/routes/entities/callouts.
- `pixel-world-render-diagnostics`: collapsed details with counts, renderer status, camera state, control buttons and raw DTO.
- The shell authority owns placement: one `Next Move` primary region and subordinate
  `Objective / Player Leverage` supporting context; narrow screens put the primary
  region first and supporting context second. `More`/`Diagnostics` remains reachable.
  The retired three-card / Next-Move-second-row layout is not an active requirement.
  This source/document alignment does not provide new S6 or player-experience evidence.

## Terminal shell relationship
Pixel World is the world-board implementation surface for the target shell defined
by [`viewer-gameplay-release-experience-overhaul.prd.md`](viewer-gameplay-release-experience-overhaul.prd.md).
That PRD owns Player/Director, dock, console, Search/Quote, World Feed v1
status, and responsive/accessibility authority. This design owns only board/HUD
mapping and its receipt rendering; it does not define a second mode or a release
claim.

## World Feed visual handoff (implemented)

Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；局部承接[REQ-AGENT-STAGE-001](../../product/agents-world-simulation/player-readable-world-stage.prd.md#req-agent-stage-001)：环境活动与因果回执隔离，空/失效/恢复提示诚实；排序和游标保持既有专业合同。

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

Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；局部承接[REQ-AGENT-STAGE-001](../../product/agents-world-simulation/player-readable-world-stage.prd.md#req-agent-stage-001)：受支持移动与输入方式下保持目标、所选对象和下一步可读；不宣称完整产品组合验收。

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
- `viewer-pixel-world-player-leverage-production-readability-2026-05-28.brainstorm.md` retains the original design exploration; it is not a second current shell/layout authority.
- Evaluate its command-lens, production-pulse and pixel-moment ideas against the current PRD and the industry-detail target above. Keep unaccepted goals traceable; existing HUD/feed implementation does not prove all brainstorm slices delivered.
- A future runtime snapshot or interaction change updates the responsible contracts and applicable PR/test evidence under the [development workflow](../../engineering/workflow/source-of-truth.md); Issue/Project tracking is optional and no retired `.pm` task is required.

## 边界、决策与风险

世界舞台主导、单一 Next Move 与从属上下文继续由 shell authority 决定；本设计只承接 HUD/board 和 receipt，Feed 仍是环境投影。产业图谱、吞吐/损耗与分层可见集是条件性持续目标；缺权威数据不补事实，不借本次文档规范化扩大 DTO。2026-10-15 main `b57e30eaaa0f3eb48c273eb62363d2ac2ada8981` 源码审读确认 Rust 派生 owner 与 host 消费边界；`pixel_world_host_test_support.jsx` mock runtime loader，JS `buildTestRustRenderState` 断言不能当作 Rust/wasm/WebGL 执行。历史 UI/构建/浏览器证据仍按各候选保留，本次没有运行行为测试。
