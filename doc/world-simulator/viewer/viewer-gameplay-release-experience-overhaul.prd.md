# Viewer 发行体验改造（游戏化优先）

- 对应设计文档: `doc/world-simulator/viewer/viewer-gameplay-release-experience-overhaul.design.md`
- 历史阶段实施与验收记录：GitHub task issue evidence。

审计轮次: 5

## ROUND-002 物理合并
- 本文件为主文档（当前权威入口）。
- `immersion-phase8~10` 内容已物理合并入本文件，对应阶段文档已合并并从仓库移除（不再保留 archive 目录）。
- `immersion-phase2~7` 旧阶段三件套已在后续文档治理中退役删除；其完成态只作为历史阶段证据保留在 core review logs、git history 与 GitHub task issue evidence comments，当前 release / 体验收口入口统一读本文件与GitHub task issue evidence。

## 1. Executive Summary
- 将 `oasis7_viewer` 默认体验从“调试工具”切换为“可发行游戏前端”。
- 保持世界画面为第一视觉焦点，隐藏技术细节并改为按需显式唤出。
- 提升新玩家可上手性：进入后快速知道“当前目标”和“下一步动作”。
- 增强情绪反馈：关键事件必须有可感知的视觉/文本反馈。

## 1.1 Terminal Viewer Authority (2026-08-14)
本节是 Viewer-domain 的终端 shell authority；上游产品语义仍以
[`doc/product/agents-world-simulation/player-readable-world-stage.prd.md`](../../product/agents-world-simulation/player-readable-world-stage.prd.md)
为准。本节同时记录终态契约与 task #3248 epoch 2 的实现边界；它不是发行放行结论。
Player shell、World Feed v1 DTO/anchor，以及 Director 的服务端核验与 fail-closed
边界已经落地；真实受信 issuer 尚未接入，因此生产环境的 Director 成功进入仍为
`capability_blocked`。

- **Player shell（默认）**：全舞台世界棋盘、紧凑且只读的权威 HUD、低存在感
  的边缘 dock，以及按需打开的 contextual console。世界是首屏最大视觉主体。
- **Director（显式选择）**：仅供操作员/调试使用的模式，可保留密集的三列工具，
  但只改变可见性和工具密度，不改变玩法语义、进度或回执因果。入口先请求
  `/api/public/director/capability`，只有服务端返回可验证、短时 grant 才开放；端点
  当前在无可信 issuer 时明确返回不可用。
- **Dock 路由**：`World / Targets / Command` 是主路径；`Search` 是 Targets
  dock 内的 `#entity-search` 兼容过滤能力，`Diagnostics` 为次级路径。
  Search 只过滤当前权威列表中的可见目标，不执行动作，也不声称提供独立快照查询。
  选择对象只负责居中、高亮并打开上下文，不能直接执行动作。
- **Contextual console**：打开后的顺序固定为 primary command/chat、selected
  context、gameplay details、diagnostics/raw state。它是按需展开的行动/解释层，
  不是常驻的监控面板。
- **Fullscreen HUD**：默认 Player 桌面以三个紧凑卡片并列呈现 `Objective`、
  `Next Move`、`Player Leverage`；移动端将 `Objective` 与 `Player Leverage` 放在
  第一紧凑行，`Next Move` 放在第二行。`More / Diagnostics` 保持可达的次级入口。
- **因果与权威**：`Action Receipt` 是唯一的玩家因果反馈面；World Feed v1 是已实现
  的环境上下文投影，不能暗示玩家成功，也不能替代缺失的 receipt。HUD 只能格式化
  权威的 commercial/runtime projection，不得合成进度、指标、阻塞、receipt 或动作可行性。
- **跨屏与可访问性**：桌面、平板、移动端保持同一概念路径；键盘焦点、关闭/
  `Escape`、CJK 和长文本必须可读且无横向溢出。空、不可用和加载状态必须诚实地
  给出原因与下一步。

### Staged implementation boundary
1. 先统一 Player shell、dock、console、HUD、receipt/feed 的文档契约与稳定锚点。
2. 再由 `viewer_engineer` 在不新增协议字段的前提下收敛默认可见性和响应式布局。
3. 最后由 `qa_engineer` 以真实桌面/移动截图、键盘/长文本和空/不可用状态验证；本
   PRD 不把文档决定当作运行时或发行放行证明。

### Current implementation boundary
- 当前 Web 保留 focus/right-panel 兼容 hooks，Targets 过滤仍以 `#entity-search`
  为实现入口；它们是实现基线/债务，不是另一个产品模式。
- 当前 `main` 已交付的 Player shell 是 fullscreen、world-first 的单舞台：世界棋盘
  覆盖首屏，HUD 以 `Objective / Next Move / Player Leverage` 三张紧凑卡片叠加，
  `World / Targets / Command` 通过按需 drawer 进入。旧的“左侧/中间/右侧”三栏
  Player IA 不属于当前默认入口；密集三栏只保留给显式、能力门控的 Director 工具视图，
  不能与 Player shell 混写。
- 当前反馈投影仍包括 `player_gameplay.recent_feedback`、本地 gameplay feedback、
  `last_world_change` 与页面的 Recent Events/Feedback；Action Receipt 继续只读这些
  因果字段。独立的 `world_feed/v1` 已由 runtime journal → Viewer transport/panel
  实现，稳定 anchor 为 `#viewer-world-feed`；两类投影不得混用，`receipt_ref` 没有
  runtime 明确因果身份时保持 `null`，绝不从文本、时间或 delta 推断。
- Director 的 Viewer state machine、`#viewer-director-entry` / `#viewer-director-exit`
  anchors、服务端验证入口与 runtime fail-closed 校验已经实现。当前 launcher 端点
  会返回明确的 `director_capability_unavailable`，因为尚无可信 issuer 能绑定当前
  `session_epoch`；这属于能力阻断，不是安全边界缺失。
- WebGL2 runtime 已纳入 canonical build/dist。GPU 可用环境有 `ready` 证据；GPU
  不可用或 `canvas.getContext("webgl2")` 失败时，页面明确进入 `Renderer Unavailable`
  状态并保留可诊断原因，不把空白画布当作成功。
- Quote 是 Command/Console 的上下文只读/行动路线，不是与 World/Targets/Command
  并列的 dock route。任何 rail 暴露均属于实现债务，不能写成终端 IA。

### Expanded visual contract (target; pending implementation and headed QA)
- **World HUD hierarchy**：世界棋盘仍是最大视觉主体；Player HUD 的阅读顺序为条件性
  connection/renderer/blocker attention、`Objective`、一个主导性的 `Next Move` CTA、
  `Player Leverage`/selected-agent context、`Action Receipt`，World Feed 保持收起的
  ambient chip。HUD 只可格式化既有 `commercial_surface` 与 `buildGameplaySummary()`。
  `world_read` 的 `tick/agents/routes/fragments/hotspots` 只在运行时已发布且非空时作为
  次级摘要显示；不得补造 Power、Data、Consensus 或其他未发布指标。
- **Flattened Next Move**：`Next Move` 只保留一个清晰的主动作/路由，目标与杠杆作为
  支持上下文，阻塞原因内联；不得让 ambient World Feed 或重复引导文案冒充动作结果。
- **Agent Console disclosure**：默认打开后先呈现当前选择、授权的 command/chat、当前
  blocker 与 receipt；未选择或不可控时只给出导航/恢复说明。Prompt override、auth/
  capability、raw snapshot/JSON、world-scale 与 Director 工具保持在收起的
  `Diagnostics / More` 下，不改变既有权限或 fail-closed 语义。
- **Focus presentation**：用户-facing 名称为 `Cinematic View / Minimal HUD`，但保留
  `focusMode` 与既有 source/test hooks。显式进入时收起完整 HUD、dock、readout/feed，
  保留选择标记、当前 objective/blocker、存在中的 receipt 与明确退出入口；Escape 先
  关闭局部 drawer，再退出该呈现层，不能改变 world、command、auth 或 receipt 因果。
- **Non-goals**：本目标不新增 runtime/protocol 字段，不改仿真、replay、renderer、
  Director capability、ownership、经济或玩家权限，也不从 metrics、ambient event、
  wall clock、cursor 或文案推断玩家进展。以上是目标合同，不是当前已完成或发行结论。

### Director entry and failure contract (implemented boundary; issuer blocked)
- 每次 fresh load、reload、new tab 或新 session 都进入 Player；Director 不是可持久化
  的偏好，也不是另一个默认入口。
- Director 只能从 Player 的次级 `Diagnostics`/operator action 显式触发，并且必须通过
  capability gate。source anchors 固定为 `#viewer-director-entry` 与
  `#viewer-director-exit`；服务端必须返回带 `server_validated=true`、短 TTL、匹配
  `director_open/viewer_director/diagnostics_read` 的签名 grant。授权结果只改变可见性和工具密度，不改变 command、auth、ownership、
  runtime、world progress 或 receipt 因果。
- capability 缺失、entry 过期、被撤销、未授权或状态不再匹配时，入口 fail closed 回到
  Player；Director surfaces 必须被清理，world/selection 保持不变，并说明恢复动作。
- 当前 Viewer mode、endpoint 与 verifier 已实现；grant 仍不持久化。未授权、过期、撤销、
  会话失效或 endpoint unavailable 时，UI 必须停留 Player 并清理 Director surface。
  生产成功进入需要可信 operator issuer，现阶段仍为 `capability_blocked`；这不是
  本地 mock 可替代的 release evidence。

## 2. User Experience & Functionality
以下条目定义终态实现约束；已落地切片以 `Current implementation boundary` 和当前任务证据为准，
不把局部实现等同于完整发行验收：

- 修改 `crates/oasis7_viewer` 的默认 UI 行为与展示优先级。
- 将 world-first Player shell 作为唯一默认入口，并提供 capability-gated Director
  可见性边界。
- 优化默认模块可见性、默认面板状态、入口可发现性、基础提示层。
- 在不改协议字段的前提下，重排现有状态信息在 UI 中的展示方式。

## 非目标
- 本阶段不改动 `oasis7` 网络协议与仿真核心业务逻辑。
- 本阶段不引入重资产美术包（大型模型/贴图替换作为后续任务）。
- 本阶段不改动 third_party 代码。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications

### 实现映射（目标，不是现有协议声明）
- 不新增 runtime snapshot 或网络协议字段；Viewer 只格式化现有权威 projection。
- Player/Director 是目标可见性语义，不要求当前运行时存在同名 mode resource/env。
- 既有 focus/right-panel 状态和 generated Web hooks 可作为迁移实现，但不能被文档
  当作已收敛的终端产品边界。

### UI 结构约束
- Player 模式默认：
  - 世界舞台占据首屏，HUD 保持紧凑且只读。
  - 边缘 dock 保留单点入口，主路径为 `World / Targets / Command`，
    Targets 内的 Search 与次级 `Diagnostics` 按需打开。
  - contextual console 打开后按 primary command/chat、selected context、
    gameplay details、diagnostics/raw state 排列。
- Director 模式（显式选择）：
  - 仅在显式选择后进入，允许密集工具视图；不改变玩法语义或进度。

### 因果、数据与反馈约束
- HUD 的目标、下一步、阻塞、进度和指标必须来自权威 commercial/runtime
  projection；UI 只能格式化，不得从 ambient activity 推断玩家进展。
- `Action Receipt` 必须表达 action、result、reason、next；没有 receipt 时显示
  明确的 `No action receipt yet`，不得用 World Feed 冒充。
- World Feed v1 可以呈现环境变化和最近上下文，但不能宣称玩家动作已成功；
  当前 runtime 未提供显式因果身份时 `receipt_ref` 保持 `null`。
- Search、Targets 和选择态都是导航/定位能力；它们不会执行动作，执行只发生在
  contextual console 的已授权 command/chat 路径。
- Receipt 的最小映射为 `action`, `stage/state`, `effect`, `reason`, `hint/next`,
  `delta_logical_time`, `delta_event_seq`；`accepted/submitted/queued/ack` 只能
  呈现等待后续 world delta，`completed_advanced` 才能表达推进，`completed_no_progress`
  仍是无进展阻塞语义。

### World Feed v1 implementation contract
- **Owner**：`runtime_engineer` owns the journal projection and cursor/reorg truth;
  `viewer_engineer` owns Viewer transport/state/panel and `#viewer-world-feed`;
  `qa_engineer` owns cross-surface acceptance.
- **Source**：runtime `WorldEvent` journal is projected in ascending
  `(world_id,reorg_epoch,event_seq)` order and requested through
  `request_world_feed`. `state.recentEvents` remains non-contract ambient preview;
  `player_gameplay.recent_feedback` remains exclusive to Action Receipt.
- **Stable anchor**：`#viewer-world-feed` is implemented in source and generated
  output. The panel is read-only ambient context and never an Action Receipt substitute.
- **Placement**：in the fullscreen Player default, the panel is a compact collapsed
  top-right edge overlay/ambient chip and opens on demand. Responsive layouts may
  reposition it only to remain inside the safe viewport; it must not become a persistent
  first-screen column.
- **Acceptance**：environment context and Action Receipt stay separate; queued/accepted
  never display success; empty/loading/replay/gap/unavailable remain distinct; gap/reorg
  requires authoritative snapshot reload; desktop/mobile/keyboard/CJK/long text remain
  readable without overflow.

#### World Feed v1 additive envelope (implemented runtime/viewer contract)
The feed is an additive projection named `world_feed/v1`; it does not rename the
current Recent Events/Feedback fields, which remain separate compatibility projections.

| Field | Contract | Player-facing rule |
| --- | --- | --- |
| `schema_version` | literal `world_feed/v1` | unknown versions render unavailable, never guessed |
| `world_id` | canonical world identity | part of event identity and cursor scope |
| `reorg_epoch` | exact non-negative decimal string on the wire (Rust `u64`; legacy numeric input accepted by the deserializer) | epoch changes invalidate the prior cursor |
| `cursor` | opaque resume position carrying `world_id`, exact-decimal `reorg_epoch`, and last exact-decimal `event_seq` | request/replay recovery is cursor-aware; no wall-clock seek |
| `events[]` | source-ordered event projections | canonical ascending `event_seq` by exact-decimal comparison; duplicate identity is dropped |
| `event_seq` | exact non-negative decimal string on the wire (Rust `u64`; legacy numeric input accepted by the deserializer), monotonic within `(world_id, reorg_epoch)` | identity key is `(world_id,reorg_epoch,event_seq)` |
| `kind`, `summary`, `detail` | canonical event presentation fields | ambient context only; text cannot create causality |
| `receipt_ref` | nullable explicit runtime causal identity (the player-facing receipt link) | current runtime projection emits `null`; render a link only when an explicit identity is supplied; never infer from time/text/delta |
| `status` | `loading`, `ready`, `empty`, `replay`, `gap`, or `unavailable` | each state has distinct copy and recovery route |

The wire contract serializes `reorg_epoch` and `event_seq` as exact decimal strings
to prevent JavaScript precision loss. Deserializers accept legacy numeric input for
compatibility; Viewer state normalizes and compares the decimal strings exactly and
presents canonical ascending event order.

`gap` or `reorg` responses require cursor-aware recovery and snapshot reload; they
must not be rendered as a contiguous event stream. `loading`, `empty`, `replay`, and
`unavailable` remain distinct, and `accepted`/`queued` feed entries never imply a
completed action. Action Receipt remains the sole player-causality surface.

### Previously reported feed/readout defects — resolved by #3315

These implementation findings are now resolved on the current branch; the target
contract above remains the authority, and the tests below are the evidence for the
bounded fixes:

- **P1 initial-page cursor continuation — resolved:** `world_feed_transport.js` now
  schedules a deferred request with the returned cursor and refreshes once after world
  activity, without overlapping an in-flight request. This historical #3315 result does
  not cover the reconnect case recorded in the current follow-up backlog below. Coverage is in
  `crates/oasis7_viewer/software_safe_src/world_feed_transport.test.js`, tests
  `continues an initial page asynchronously from the returned cursor` and
  `refreshes the latest cursor once after world activity and does not overlap in-flight requests`.
- **P2 gap/reorg event retention — resolved:** `consumeWorldFeed()` clears the prior
  `events` array when `status === "gap"` before authoritative snapshot reload. Coverage is
  in `crates/oasis7_viewer/software_safe_src/world_feed_state.test.js`, test
  `stops appending on gap/reorg and requires authoritative snapshot reload`.
- **P2 multi-resource readout fallback — resolved:** the selected-resource formatter now
  parses each host entry joined by ` · ` and formats every structured resource instead of
  returning raw serialized text. Coverage is in
  `crates/pixel_world_bridge/src/render_selected_resource_readout.rs`, test
  `resource_readout_formats_each_structured_entry_without_raw_json`.

Task #3248 browser screenshots and headed runs remain historical evidence. For #3315,
external Chrome freshly verified only the GPU-unavailable fallback at headed `1440x1000`
and `390x844`; this does not verify the `ready=true` WebGL2/Cinematic path, post-probe
async GPU failure, or tablet widths `641–1240`, and is not a release-readiness claim.

### Current cleanup and bounded optimization backlog (2026-08-25)

This backlog is the durable Viewer-domain handoff derived from the current implementation
review. Priority labels describe player trust/readability risk; they are not release severity,
task status, or evidence that an implementation has shipped. The paired design document owns
the visual treatment. GitHub task evidence owns mutable execution status.

#### Current cleanup before further shell refinement

1. **World Feed reconnect continuation (P2 correctness).** The initial continuation fix above
   remains valid, but reconnect can retain the prior response cursor while issuing a fresh
   `cursor:null` request. Continuation must compare the returned cursor with the cursor used by
   that request, not with the prior response cursor. Reconnect coverage must reproduce the case
   where the fresh first-page cursor equals the retained cursor but later pages still exist.
2. **Truthful world freshness (P1 player trust).** `LIVE` may appear only when an existing
   authoritative connection/freshness projection supports it. Otherwise the Player surface must
   show a non-deceptive current/stale/reconnecting/offline/unavailable state and the last trusted
   logical position when available. Viewer must not invent freshness from wall-clock time,
   renderer readiness, a successful reconnect, or a retained snapshot.
3. **Advanced disclosure containment (P2 control safety).** Prompt Overrides, approval input,
   auth/capability state, and raw controls must remain inside the same collapsed Advanced or
   Diagnostics boundary. A persisted child preference must not expose editable controls while
   its parent disclosure is closed.
4. **Mobile overlay safe area (P2 readability).** Cinematic entry, compact world readout, feed,
   renderer fallback, and diagnostics must occupy non-overlapping slots at `390x844`, including
   CJK and long labels. Static CSS arithmetic is not sufficient proof; headed geometry or
   screenshot evidence is required.

#### Feasible with current authoritative Viewer data

- **Receipt uniqueness (verify first; P1 visual trust).** Player and Cinematic must present one
  causal receipt surface for the active presentation. Headed verification must first determine
  whether the normal and compact receipt overlays are simultaneously visible; if so, make them
  mutually exclusive rather than hiding the collision with z-index changes.
- **Readable entity identity (P1).** Targets, Canvas markers, Command context, search results, and
  accessible names use explicit `name`, then `label`, then stable `id` as fallback. Raw IDs and
  binding metadata remain secondary metadata or Diagnostics; Viewer must never infer a role or
  name from an ID.
- **Semantic selection state (P1/P2 accessibility).** Targets and Canvas hit targets expose the
  current selection with `aria-pressed`, `aria-current`, or an equivalent appropriate semantic,
  plus visible keyboard focus. Existing `data-selected` hooks remain available for automation.
- **Agent Context Lite (P1/P2).** The default Agent context may format only the selected Agent's
  explicit identity/location plus already-published Objective, recommended Next Move, blocker,
  Player Leverage/execution state, and most recent Action Receipt/feedback. It must not infer
  internal intent, plan, progress, rationale, or success from World Feed or event text.
- **Player-readable receipt state (P2).** Internal confidence/state values such as
  `world_delta` or `none` remain in stable data attributes and Diagnostics, while visible copy
  states whether a world change is confirmed, an action is pending, or no receipt exists.
- **Visible renderer recovery (P2; P1 when the stage is unavailable).** Renderer Unavailable
  presents an honest retry/reattach/reload action using existing Viewer capability. Copy must say
  that retry cannot add WebGL2 support to an incapable browser; fatal codes remain in Diagnostics.
- **Narrow Command continuity (P2).** While Command content scrolls, the selected readable Agent
  identity, current truthful status, Objective, and close/back route remain visible.

#### Agent Console V2: canonical runtime Intent contract

本节冻结 Agent Console V2 的最小产品语义，作为 `runtime_engineer`、`agent_engineer` 与
`viewer_engineer` 的实现边界；它不是当前发行放行结论。当前已存在的 Activity projection
继续表示“Agent 正在做什么”，V2 新增的 Intent projection 只表示“权威运行时接受、阻断、
结束或替换了什么”。Action Receipt 只表示“世界实际发生了什么”。三者不得互相推断：
Activity 不能代签 Intent，Intent 不能代签世界结果，环境 Feed、chat echo 或 render update
不能代签 Receipt。

ETA、energy、cargo、module health、relationships/trust、plan 或 memory 内容与 confidence、
decision rationale、provider/cost/debug context、multi-Agent graph，以及新的
cancel/approve/retry/reprioritize controls，不能因为存在相关 raw object 就成为 Viewer 字段。
Prompt profile、execution debug context、raw memory、metrics、cursor、wall-clock 或
`last_active` 不能被包装成玩家 Agent truth。

##### V2 projection fields and authority

以下是稳定的玩家语义字段；具体 Rust/DTO 名称、存储布局和事件结构仍由 runtime 专业权威
实现，但必须保留字段含义和兼容方向：

| Field | Player meaning / authority |
| --- | --- |
| `schema_version` | Versioned `agent_intent.v2` projection; legacy snapshots may omit it and must remain readable. |
| `agent_id` / `intent_id` | Stable Agent identity and immutable request identity. `intent_id` is the idempotency key; it is not a prompt hash displayed as player copy. |
| `kind` / `summary` | A bounded, sanitized, catalogued action class and player-readable summary. Summary is not raw prompt, hidden instruction, model output, or rationale. |
| `target_id` | Optional target from authoritative world state; absence is honest and does not imply the default target. |
| `status` | `proposed`, `submitted`, `accepted`, `blocked`, `completed`, `rejected`, `expired`, `cancelled`, or `superseded`, with the lifecycle rules below. |
| `source` | `player`, `agent`, or `provider_advisory`; provider remains advisory and never becomes the execution authority. |
| `position` | Authoritative `world_id`, `reorg_epoch` when applicable, `logical_time`, and `event_seq`; this is the ordering anchor, not wall-clock time. |
| `source_class` | `runtime_projection`, `receipt`, or explicitly `local_pending`; local pending content must never be rendered as runtime accepted. |
| `freshness` | `current`, `stale`, `reconnecting`, `unknown`, `conflict`, or `unavailable`, supplied by the authoritative snapshot/transport state. |
| `updated_at` | Runtime logical update position; it must not be synthesized from browser time, response arrival order, or render time. |
| `receipt_ref` | Optional explicit causal receipt/event reference. It remains `null` when runtime has not published a causal identity. |
| `reason_code` / `reason_summary` | Sanitized authority reason for block, rejection, replacement, expiry, cancellation, or alternative; no private policy or debug payload. |
| `replaced_by` | Optional new `intent_id` only for an explicit authoritative replacement; ordinary retry/reconnect does not populate it. |
| `control_state` | `controllable`, `read_only`, `control_lost`, or `unavailable`; this is a player-facing permission result, never a public-key or auth-material dump. |

`position`, `source_class`, and `freshness` are mandatory for any retained Agent Context. A
snapshot with an intent but without a trustworthy position is not `current`; it is stale/unknown or
cleared. A missing projection is not equivalent to `Idle`, `accepted`, or “no work”.

##### Intent lifecycle, Activity separation, and receipts

The product lifecycle is:

`local_pending -> submitted -> accepted -> blocked/completed` and, where applicable,
`accepted -> rejected/expired/cancelled/superseded`.

- `local_pending` is a local draft or request not yet admitted by runtime. It may be shown as
  **Draft / Not submitted**, but never as a current runtime Intent.
- `submitted` means the request reached an authenticated transport boundary; it does not mean the
  world accepted it or that the Agent applied it.
- `accepted` means runtime admitted the intent into its authoritative cadence/queue. It does not
  mean a world effect exists. An `accepted` intent may have an independent Activity of
  `executing`, `waiting`, or `blocked`.
- `blocked` means runtime retains the intent but cannot progress under the current authority or
  world constraint. It must expose a safe reason and a supported next step; it is not an inferred
  state from ambient events or a missing reply.
- `completed` is legal only when an explicit committed world receipt/event binds the Intent. A chat
  ack, queued echo, local optimistic update, Activity transition, or UI notification cannot produce
  `completed`.
- `rejected`, `expired`, `cancelled`, and `superseded` are terminal dispositions with no new world
  effect. A replacement must be explicit and leaves the old Intent auditable via `replaced_by`.

The Console presents the three timelines side by side but independently: Activity answers
“what is the Agent doing now?”, Intent answers “what did authority admit?”, and Receipt answers
“what changed in the world?”. No UI state may promote an Activity `executing` or `blocked` into a
completed Intent or Receipt.

The same `intent_id` submitted more than once must return the same authoritative acceptance or
terminal receipt and must not create a second world effect. Reconnect, provider switching,
browser retry, or copying a prior request cannot silently replay, merge, reprioritize, or transfer
an existing Intent. Any supported replacement/cancellation is a separate authenticated request
with its own identity and receipt.

##### Normative runtime-safety addendum

The complete Intent transition contract is:

| Current | Allowed next state | Normative meaning |
| --- | --- | --- |
| `proposed` | `submitted`, `rejected`, `expired` | Advisory/local candidate only; it has no authority or world effect. |
| `submitted` | `accepted`, `rejected`, `expired`, `cancelled` | Authenticated request received, but not yet admitted or effective. |
| `accepted` | `blocked`, `completed`, `rejected`, `expired`, `cancelled`, `superseded` | Runtime admitted the request; world effect still requires its own receipt. |
| `blocked` | `accepted`, `rejected`, `expired`, `cancelled`, `superseded` | A fresh authority re-check may resume it as `accepted`; no ambient event may do so. |
| `completed`, `rejected`, `expired`, `cancelled`, `superseded` | same state only | Terminal dispositions are immutable. Replay, reconnect, retry, or a later provider result may only return the recorded disposition; they cannot reopen or rewrite it. |

Every authoritative Intent persists its `intent_id`, a canonical request digest, and an idempotency
ledger entry in the runtime state/journal. The digest covers the normalized request and its target,
Agent, actor, world, authority scope, and applicable expiry/nonce—not merely a display string. The
ledger must survive snapshot/replay/reconnect for the authority's duplicate-protection window:

- the same `intent_id` with the same canonical digest returns exactly the original disposition and,
  when present, the same receipt reference, without a second reservation, event, or world effect;
- the same `intent_id` with a different digest returns deterministic `intent_conflict`, leaves the
  original disposition unchanged, and performs no side effect;
- replacement, cancellation, or a new retry uses a new `intent_id` and a new digest, with an
  explicit `replaced_by`/causal link where applicable. Client timestamps and response arrival order
  never decide identity or disposition.

Before enqueueing, reserving, debiting, mutating Agent/World state, or emitting an acceptance or
receipt, runtime must verify a signed Intent envelope with a versioned domain-separated purpose
(the exact cryptographic algorithm remains runtime authority). The envelope must bind the world,
Agent, `intent_id`, canonical digest, authorized actor/scope, nonce and applicable authority
position/expiry so that a signature cannot be replayed as another action, Agent, world, or protocol
message. Permission, ownership/binding, scope, nonce/replay, and world preconditions are checked
before any side effect; a failed check produces a deterministic rejection/blocked result and no
reservation, state mutation, acceptance, or receipt.

`receipt_ref` is valid only as the complete authority tuple
`{ intent_id, world_id, reorg_epoch, logical_time, event_seq, receipt_id }`. A partial, inferred,
text-derived, or mismatched tuple is `null`/invalid and cannot justify `completed`, player impact,
or a Viewer success message. Receipt identity and Intent identity remain immutable across replay;
reorg/recovery either supplies a replacement authoritative tuple or marks the retained projection
stale/unknown.

`provider_advisory` may create a `proposed` suggestion only. It cannot create an `accepted` Intent,
receipt, reservation, or world effect. An owner/player may convert a suggestion into a new,
independently authenticated request; that conversion rechecks permission and receives a new
`intent_id` and canonical digest.

Player-visible `summary`, `reason_code`, `reason_summary`, and next-step copy must come from a
versioned allowlist of templates plus bounded authority parameters. Raw prompt text, model output,
provider rationale, hidden policy, arbitrary runtime strings, credentials, or unrecognized reason
codes are never player copy; an unknown code uses a safe generic template or is redacted.

##### Freshness, control, privacy, and failure semantics

- `current` is allowed only when the snapshot/event chain is authoritative and current for the
  displayed `position`. A retained snapshot after reconnect, gap, reorg, replay, or reload is
  `stale` until a replacement arrives; it must not silently drive a high-consequence action.
- During refresh, an older value may remain as **Last known**, but never as **Current**. Conflicting
  observations are **Needs confirmation**; the client must not pick a truth by arrival order.
- `unknown`/`unavailable` clears or redacts unsupported details and provides wait/refresh/reselect/
  stop guidance. Missing Intent does not alter independently published Activity or Receipt values.
- Authorization is checked before value freshness. When control/ownership is lost, the surface
  enters `control_lost`, keeps any local draft explicitly unsubmitted, hides latest authority
  values and diffs, and offers no submit/replay/transfer path. When control remains valid but the
  authoritative value changed, the draft is `stale`; show the new authorized value and require
  refresh/re-edit, never auto-merge or overwrite.
- A viewer without control may receive only the visibility-approved identity, coarse Activity,
  and redacted status. Intent summary, target detail, private reason, prompt, memory, provider,
  organization policy, public key, and other sensitive fields remain hidden. Redaction is not
  `Idle`, `completed`, or proof that no Intent exists.
- Reason and next-step copy must be safe, bounded, localized where supported, and sourced from
  authority. Raw enum values, trace text, credentials, model/provider diagnostics, and hidden
  instructions stay in Diagnostics or are omitted.

##### V2 acceptance and explicit non-goals

V2 is accepted only when all of the following are proven by runtime/replay, Viewer contract, and
headed QA evidence:

1. Legacy snapshots without Intent remain readable; identical state + journal replay yields the
   same projection and position, and missing Intent remains distinct from Idle.
2. Accepted, blocked, completed, rejected, expired, cancelled, and superseded are visibly distinct;
   accepted/queued never present a world success, and only a bound receipt can present completion.
3. Duplicate `intent_id`, reconnect, retry, provider switch, and out-of-order response cases yield
   at most one authoritative Intent/Receipt/world effect.
4. Gap/reorg/reconnect/reload, stale, conflict, unavailable, control-lost, unauthorized, and
   redacted cases retain honest source/freshness/permission semantics and a valid next step.
5. Activity, Intent, Action Receipt, World Feed, and local pending draft remain separate in the
   rendered Console; no plan/rationale/ETA/relationship/success is inferred from another surface.
6. Headed desktop and narrow mobile checks cover current, blocked, missing, stale, permission-loss,
   duplicate, replacement, and terminal receipt scenarios with screenshot, state, and browser
   console evidence; release remains blocked when the authoritative runtime or WebGL2 headed lane
   cannot be verified.

Explicit non-goals are new world powers, new gameplay balance, new protocol-wide player controls,
automatic cancellation/retry/reprioritization, ETA/energy/cargo/health/relationship/plan/memory
disclosure, provider/model/cost/debug disclosure, private-policy disclosure, client-side
freshness inference, or a claim that local mock/fallback/software-safe screenshots constitute
release readiness.

### 可发现性增强
- 为关键入口增加显式提示文案（例如：快捷键提示、模式提示）。
- 不依赖用户阅读手册即可发现“如何打开面板”和“如何聚焦对象”。

## 5. Risks & Roadmap
- M1（目标）：冻结 shell/dock/console/HUD/receipt/feed 文档契约和稳定锚点。
- M2（目标）：Viewer 在既有 projection 上收敛 Player 默认可见性与响应式布局。
- M3（目标）：真实桌面/移动/键盘/空态截图与语义测试完成；未完成前不宣称发行放行。

### Ordered implementation slices (handoff)
The following order is frozen for implementation planning; each slice must preserve
the target/current boundary above and may not hand-author generated artifacts.

| Order | Slice | Owner handoff | Exit evidence |
| --- | --- | --- | --- |
| 1 | Stable anchors and duplicate-ID tests | `viewer_engineer` | source JSX anchors, compatibility assertions, no generated-file edits |
| 2 | Player shell layout and sticky rail | `viewer_engineer` + visual review | desktop/mobile IA screenshots and route smoke |
| 3 | Contextual console and Action Receipt | `viewer_engineer` + `runtime_engineer` | command/receipt states; queued != complete; no-receipt state |
| 4 | Focus, keyboard, IME, Escape and a11y | `viewer_engineer` + `qa_engineer` | focus-return, Escape priority, CJK/long-text/no-overflow evidence |
| 5 | Separate World Feed v1 projection | `runtime_engineer` + `viewer_engineer` | implemented schema/source/order/dedup/cursor/reorg tests; `receipt_ref` remains explicit/null-safe |
| 6 | Headed desktop/mobile QA | `qa_engineer` | fresh screenshots, DOM/keyboard smoke, explicit residual-risk verdict; release evidence remains outstanding |

### Terminal shell QA evidence matrix (target gate)
No row below is evidence that the target has shipped; it is the minimum evidence
required after the corresponding implementation slice.

| ID | Surface/state | Viewports/input | Required proof | Owner |
| --- | --- | --- | --- | --- |
| IA-01 | Player default | headed desktop 1440x1000 | stage/HUD/board/receipt dominate; quiet dock; no three-column default | viewer + visual |
| IA-02 | sticky rail | headed mobile 390x844 | `World / Targets / Command`; Search nested in Targets; More contains Diagnostics | viewer + visual |
| HUD-01 | World HUD hierarchy | headed 1440x1000 + 390x844 | attention → Objective → one Next Move CTA → Leverage/context → Receipt; optional world_read only when authoritative | viewer + visual |
| HUD-02 | Flattened Next Move | headed 1440x1000 + 390x844 | one dominant action/route, inline blocker, no duplicate or ambient success claim | viewer + visual + QA |
| CON-01 | Agent Console disclosure | desktop/mobile + keyboard | command/chat and selected context are default; raw/auth/operator details are collapsed Diagnostics | viewer + QA |
| CIN-01 | Cinematic / Minimal HUD | headed 1440x1000 + 390x844 + Escape/IME | explicit entry/exit, minimal objective/blocker/receipt and selected marker, focus return preserved | viewer + visual + QA |
| DIR-01 | Director allowed | fresh Player -> explicit Diagnostics action | capability-gated, ephemeral, visibility/density only, world/selection preserved | producer + viewer |
| DIR-02 | Director denied/stale/revoked | reload/new tab + denied entry | fail closed to Player, sanitized surfaces, recovery copy, no persistence | viewer + QA |
| FOC-01 | drawers/console/focus | keyboard + screen reader semantics | local surface closes before Focus; invoker receives focus | viewer + QA |
| FOC-02 | IME composition | CJK input | Escape does not cancel composition or close surface prematurely | viewer + QA |
| REC-01 | receipt states | accepted/queued, advanced, no-progress, blocked | Action Receipt alone expresses causality; no ambient success | runtime + viewer + QA |
| FED-01 | feed states | loading/empty/replay/gap/unavailable/reorg | schema/order/cursor/dedup and snapshot-reload recovery are distinct | runtime + viewer + QA |
| TXT-01 | copy and density | `locale=en`, `locale=zh`, long labels | matrix copy fits, wraps, and has no horizontal overflow | visual + QA |
| FED-02 | reconnect continuation | disconnect before deferred continuation, then same first-page cursor | later pages are requested from the request cursor; no omitted events or stale-generation request | viewer + QA |
| STA-01 | freshness truth | live/reconnecting/offline/stale/unavailable | visible state matches authoritative source; retained snapshot never remains green `LIVE` | runtime + viewer + QA |
| REC-02 | Player/Cinematic receipt | headed `1440x1000`, `1024x768`, `768x1024`, `390x844` | exactly one visible causal receipt; no overlay collision; raw confidence enum is not player copy | viewer + visual + QA |
| ENT-01 | Targets/Canvas/Command identity | keyboard + screen reader + long en/zh names | readable name is primary; stable ID remains metadata; selection is announced semantically | viewer + visual + QA |
| CTX-01 | Agent Context Lite | current/stale/missing/replay/gap/reorg | only explicit published fields render; source/freshness is honest; no inferred plan, ETA, relationship, or success | agent + runtime + viewer + QA |
| INT-01 | Agent Intent V2 | current/accepted, blocked, completed, missing, stale, conflict, control-lost, redacted, duplicate/replacement | Activity, Intent, and Receipt remain separate; accepted/queued never imply world success; duplicate identity yields one authority result; permission and freshness are explicit | agent + runtime + viewer + QA |
| REN-01 | Renderer unavailable recovery | headed desktop/mobile, sync and post-probe failure | recovery action is reachable and honest; retry failure preserves usable Player fallback and diagnostics | viewer + QA |

## 目标验收指标（实现后）
以下指标必须由后续 Viewer/runtime 实现与本页 terminal QA matrix 的真实证据证明；
本次文档 slice 不把它们报告为当前已通过：

- 默认启动时，世界画面可视占比显著提升（右侧大面板不再常驻）。
- 新用户无需文档可在首屏找到“打开面板”入口。
- Player 模式下首屏技术模块数量明显减少。
- Director 模式仍可访问原有调试能力。

### 目标实现风险
- 默认隐藏面板可能导致“信息不可见”感受上升。
  - 对策：提供显式入口按钮与清晰提示文案。
- Director 可见性边界可能引入状态分歧。
  - 对策：Director 仅改变当前 tab 的工具密度与可见性；Player 始终是 fresh-load
    默认，运行时状态保持统一权威数据结构。
- 旧测试可能绑定既有默认 UI 行为。
  - 对策：补充模式相关测试并更新默认行为断言。

## 当前结论
- **目标**：Player shell 默认、Director 显式 opt-in、Search 属于 Targets、Quote 属于
  Command/Console、Action Receipt 与 World Feed v1 分离。
- **当前源码候选**：task #3248 epoch 2 已将 Player Web shell 收敛为 stage-first
  单列、`World / Targets / Command` 主导航、按需 Targets/Command drawer、次级
  Diagnostics、默认折叠 Gameplay Details，以及 drawer/Focus 的 Escape、IME 与焦点返回
  合同；`#entity-search` 与现有 Recent Events/Feedback 语义保持不变。该分支同时包含
  `world_feed/v1` runtime/Viewer projection、`#viewer-world-feed` panel，以及 Director
  server-verifier/state-machine/fail-closed endpoint。
- 以上只描述当前任务分支的实现边界，不是正式发行结论。Director 真实可信 issuer
  尚未接入，成功生产入口仍为 `capability_blocked`；GPU-enabled WebGL2 ready 与
  GPU-disabled explicit unavailable 均已有证据，但完整 headed renderer/Focus QA、
  issuer-backed Director allowed path 和 release judgment 仍由后续 Viewer/runtime/QA
  evidence 提供。

## Phase 8~10 增量记录（ROUND-002 物理合并）
- 原阶段文档已合并并删除，以下段落仅为历史实施证据摘要，不是当前运行时、
  shell、mode/env、layout、receipt 或发行状态 authority。当前真值只读本文
  `Terminal Viewer Authority`、`Current implementation boundary` 与 GitHub task
  evidence；其中的旧状态名和截图/测试记录不得外推为已发货终端契约。

### Phase 8：信息分层减负与焦点收敛

#### 1. Executive Summary
- 在保持“可随时指挥 Agent”前提下，进一步降低 Player 首屏的信息密度，避免左侧引导卡片重复表达同一目标。
- 让玩家在隐藏态与展开态都能快速识别“当前应该做什么”，减少任务文案和引导文案的视觉竞争。
- 在面板展开时压缩任务 HUD 体量，确保视线焦点优先回到世界场景和指挥面板。

#### 2. User Experience & Functionality
- `crates/oasis7_viewer` Player 模式 UI 减负改造：
  - 调整“下一步目标提示卡”与“新手引导卡”的共存策略，消除重复提示。
  - 调整任务 HUD 在不同面板状态下的布局锚点与信息密度（展开态更紧凑）。
  - 保留并强化“任务目标 + 指挥入口 + 世界反馈”三要素的可达性。
- 不改动 Director 模式行为。

#### 非目标
- 不改动仿真协议、事件语义和后端链路。
- 不改动 `third_party` 代码。
- 不在本阶段引入新的 UI 框架或复杂动画系统。

#### 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

#### 4. Technical Specifications

##### 复用状态
- `PlayerOnboardingState`：用于判定当前步骤是否仍需展示新手引导。
- 历史实现状态 `RightPanelLayoutState`：曾用于判定面板隐藏/展开并驱动 HUD 密度；不属于当前 authority。
- `PlayerGuideProgressSnapshot` 与 `PlayerGuideStep`：用于任务目标与引导步骤一致化。

##### 新增/调整行为
- 目标提示卡显示策略：
  - 仅在“面板隐藏且当前步骤引导卡已收起”时显示，避免与引导卡重复。
- 任务 HUD 自适应策略：
  - 面板隐藏且引导卡显示时，下移任务 HUD 锚点避免叠压。
  - 面板展开时启用紧凑模式（保留核心目标与进度，降低次要文案占比）。

#### 5. Risks & Roadmap
- M1：第八阶段建档完成（设计 + 项管）。
- M2：完成目标提示卡与引导卡去重策略，并有单测覆盖。
- M3：完成任务 HUD 锚点与紧凑模式改造，并有单测覆盖。
- M4：完成回归测试、Web 闭环验证与文档收口。

##### Technical Risks
- 风险 1：提示层收敛后，部分玩家可能遗漏“下一步目标”。
  - 对策：保留顶部 compact HUD 的 objective 字段，并在引导卡收起后恢复底部目标提示卡。
- 风险 2：展开态任务 HUD 过度压缩导致可读性下降。
  - 对策：仅压缩冗余文案与奖励块，保留目标标题、进度条与关键动作按钮。
- 风险 3：新增状态分支导致行为不可预测。
  - 对策：将显示策略抽为纯函数并增加定向单测，保证规则可验证。

#### 验收结果（VRI8P3）
- 回归结果：
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_viewer` 通过（325 tests）。
  - `env -u RUSTC_WRAPPER cargo check -p oasis7_viewer --target wasm32-unknown-unknown` 通过。
- Web 闭环（agent-browser，按 `testing-manual.md` S6）：
  - 使用 `?test_api=1` 访问 viewer，`window.__AW_TEST__` 可用，状态采样为已连接且 tick 正常推进。
  - 完成隐藏态、聚焦态、面板展开态（紧凑任务 HUD）与再次收起态截图采样。
  - Console 汇总：`Total messages: 12 (Errors: 0, Warnings: 2)`，无新增功能错误。
- 验收产物：
  - `output/playwright/viewer/phase8/phase8-hidden-default.png`
  - `output/playwright/viewer/phase8/phase8-hidden-focused.png`
  - `output/playwright/viewer/phase8/phase8-panel-open-compact.png`
  - `output/playwright/viewer/phase8/phase8-panel-hidden-after-toggle.png`
  - `output/playwright/viewer/console-2026-02-23T12-40-54-234Z.log`
- 结论：
  - 目标提示与引导提示的重复展示已消除。
  - 任务 HUD 在展开态的体量显著收敛，隐藏态与引导卡不再发生叠压。
  - “世界优先 + 可随时指挥 Agent”的布局目标在 phase8 达成。

#### 6. Validation & Decision Record
- 追溯: 历史阶段实施与验收记录由 GitHub task issue evidence 维护，本文只保留当前体验合同。

### Phase 9：指挥链路前置与顶层减噪

#### 1. Executive Summary
- 继续降低 Player 首屏 UI 压力，消除顶部叠层竞争，让世界画面保持第一视觉焦点。
- 保证“玩家随时可指挥 Agent”是隐藏态下的一步操作，不需要先进入中间层再找 Chat。
- 在保留布局预设能力的同时，把其从“常驻干扰”改成“展开态高级能力”。

#### 2. User Experience & Functionality
- `crates/oasis7_viewer` Player 模式体验层调整：
  - 顶部布局预设条从隐藏态移除，仅在面板展开态展示，并下移锚点避免与顶部 HUD 叠压。
  - 在任务 HUD（隐藏态）增加“直接指挥 Agent”动作，点击后直接切到 Command 预设（展开面板+打开 Chat）。
  - 增加纯函数测试覆盖，确保新显示策略与一键指挥策略可回归验证。
- 不改动 Director 模式行为。

#### 非目标
- 不改动仿真协议、事件语义、后端链路。
- 不改动 `third_party`。
- 不在本阶段引入重资产特效系统与复杂动画框架。

#### 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

#### 4. Technical Specifications

##### 复用状态
- 历史实现状态 `RightPanelLayoutState`：曾决定面板隐藏/展开和布局预设条显隐；不属于当前 authority。
- 历史实现状态 `RightPanelModuleVisibilityState`：曾切换 Chat/Timeline/Details 可见性；不属于当前 authority。
- `PlayerGuideStep` 与 `PlayerGuideProgressSnapshot`：维持任务目标与动作按钮的语义一致性。

##### 新增/调整行为
- 顶部布局预设条策略：
  - 隐藏态不显示。
  - 展开态显示，并使用下移锚点避免与 compact HUD 同层竞争。
- 隐藏态任务 HUD 策略：
  - 新增“直接指挥 Agent”按钮。
  - 点击后直接应用 `PlayerLayoutPreset::Command`，保证 Chat 入口一步可达。

#### 5. Risks & Roadmap
- M1：第九阶段建档完成（设计 + 项管）。
- M2：顶部布局预设条减噪策略完成并有单测覆盖。
- M3：隐藏态任务 HUD 一键指挥入口完成并有单测覆盖。
- M4：回归测试、Web 闭环验收与文档收口完成。

##### Technical Risks
- 风险 1：隐藏态移除布局预设条后，玩家可能不易发现“布局切换”能力。
  - 对策：该能力保留在展开态，且隐藏态强调“直接指挥 Agent”主路径。
- 风险 2：新增任务 HUD 按钮过多导致操作犹豫。
  - 对策：仅在隐藏态显示“直接指挥 Agent”，展开态不重复显示。
- 风险 3：布局锚点调整引入新的 UI 叠压。
  - 对策：把显隐/锚点规则抽成纯函数并补充定向单测。

#### 验收结果（VRI9P3）
- 回归结果：
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_viewer` 通过（328 tests）。
  - `env -u RUSTC_WRAPPER cargo check -p oasis7_viewer --target wasm32-unknown-unknown` 通过。
- Web 闭环（agent-browser，按 `testing-manual.md` S6）：
  - 使用 `?test_api=1` 访问 viewer，`window.__AW_TEST__` 可用。
  - 执行 `runSteps(...)` 与 `Tab` 展开/收起链路，采样 `getState()` 为已连接。
  - Console 汇总：`Total messages: 11 (Errors: 0, Warnings: 1)`，无功能错误。
- 验收产物：
  - `output/playwright/viewer/phase9/phase9-hidden-default.png`
  - `output/playwright/viewer/phase9/phase9-hidden-focused.png`
  - `output/playwright/viewer/phase9/phase9-panel-open-command.png`
  - `output/playwright/viewer/phase9/phase9-panel-hidden-after-toggle.png`
  - `output/playwright/viewer/console-2026-02-23T13-04-48-150Z.log`
- 结论：
  - 隐藏态已移除布局预设条，首屏顶部竞争减弱。
  - 展开态布局预设条下移后不再与 compact HUD 同位叠压。
  - 隐藏态任务 HUD 已提供“直接指挥 Agent”入口，玩家可一键进入 Chat 指挥路径。

#### 6. Validation & Decision Record
- 追溯: 历史阶段实施与验收记录由 GitHub task issue evidence 维护，本文只保留当前体验合同。

### Phase 10：新手流程闭环与认知减负

#### 1. Executive Summary
- 修复新手流程中的“进度跳步”问题，确保教程按真实行为推进，不再出现 2/4 直接到 4/4。
- 把第 4 步操作提示从抽象描述改成可执行动作提示，让玩家明确“下一步点哪里”。
- 继续压缩隐藏态信息密度，减少首屏多卡片并行发声导致的认知负担。

#### 2. User Experience & Functionality
- `crates/oasis7_viewer` Player 模式 UI/引导逻辑调整：
  - 调整 `PlayerGuideProgressSnapshot` 的完成判定，新增“执行反馈”门槛后再允许 4/4 完成。
  - 调整 `ExploreAction` 阶段的任务动作文案，改为具体可执行的控制建议。
  - 调整隐藏态入口信息层级，避免与新手引导卡/任务 HUD 同时形成过多主卡片。
  - 补充纯函数与引导流程相关单测。
- 不改动 Director 模式调试语义。

#### 非目标
- 不改动仿真协议、事件语义与后端链路。
- 不改动 `third_party`。
- 不引入新的大型美术资源或复杂动效系统。

#### 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

#### 4. Technical Specifications

##### 复用状态
- `ViewerState.events`：用于判定是否出现“执行反馈”。
- 历史实现状态 `RightPanelLayoutState`：曾用于隐藏态/展开态引导层级；不属于当前 authority。
- `PlayerGuideStep` / `PlayerGuideProgressSnapshot`：维持教程步骤与任务 HUD 一致性。

##### 新增/调整行为
- 教程完成门槛：
  - `ExploreAction` 不再由“已选中目标”直接达成。
  - 需要出现“新反馈事件”后才计入第 4 步完成（忽略历史事件，避免首帧秒完成）。
- 文案策略：
  - 第 4 步动作提示改为“点击直接指挥并发送一条指令”等具体可执行动作。
- 隐藏态减噪：
  - 玩家模式隐藏态不再渲染右上大入口卡，保留边缘抽屉与任务 HUD 指挥入口。
  - Director 模式保持原有右上入口卡，避免调试能力回退。

#### 5. Risks & Roadmap
- M1：第十阶段建档完成（设计 + 项管）。
- M2：教程进度门槛与第 4 步动作文案改造完成并有单测覆盖。
- M3：隐藏态减噪策略完成并有定向验证。
- M4：回归测试、agent-browser 新手流程复测与文档收口完成。

##### Technical Risks
- 风险 1：第 4 步门槛提高后，部分场景下完成速度下降。
  - 对策：采用“历史事件不计入、仅新反馈事件计入”策略，避免首帧秒完成同时防止门槛过严。
- 风险 2：隐藏态入口卡减弱后可发现性下降。
  - 对策：保留边缘呼出入口与任务 HUD 指挥按钮双入口。
- 风险 3：文案更具体后中英文一致性回归。
  - 对策：同步更新中英文文案并覆盖关键断言测试。

#### 验收记录（VRI10P3）
- 回归命令：
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_viewer`
  - `env -u RUSTC_WRAPPER cargo check -p oasis7_viewer --target wasm32-unknown-unknown`
- Web 闭环（agent-browser）：
  - 使用 `oasis7_viewer_live + run-viewer-web.sh + agent-browser` 复测新手流程（隐藏态 -> 展开面板 -> 选择目标 -> 隐藏态复查）。
  - 控制台记录：`output/playwright/viewer/console-2026-02-23T13-41-21-606Z.log`（`Errors: 0`）。
  - 截图产物：
    - `output/playwright/viewer/phase10/step0-hidden-initial.png`
    - `output/playwright/viewer/phase10/step1-open-panel-tab.png`
    - `output/playwright/viewer/phase10/step2-selected-agent.png`
    - `output/playwright/viewer/phase10/step3-after-play-feedback.png`
    - `output/playwright/viewer/phase10/step4-hidden-no-top-card.png`

#### 6. Validation & Decision Record
- 追溯: 历史阶段实施与验收记录由 GitHub task issue evidence 维护，本文只保留当前体验合同。

## 6. Validation & Decision Record
- 追溯: 历史阶段实施与验收记录由 GitHub task issue evidence 维护，本文只保留当前体验合同。
