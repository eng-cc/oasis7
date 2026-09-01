# Viewer 发行体验总改造设计

- 对应需求文档: `doc/world-simulator/viewer/viewer-gameplay-release-experience-overhaul.prd.md`
- 历史阶段实施与验收记录：GitHub task issue evidence。

## 1. 设计定位
本文是 Viewer-domain 的视觉/交互配对设计；产品语义仍以
[`doc/product/agents-world-simulation/player-readable-world-stage.prd.md`](../../product/agents-world-simulation/player-readable-world-stage.prd.md)
为上游权威，终端 shell 的 Viewer 边界以配对 PRD 的
`Terminal Viewer Authority` 为准。本文不把目标 shell 当作已发货或发行放行事实。

## 2. 设计结构
- **Player shell（当前默认）**：全舞台世界棋盘、紧凑权威 HUD、低存在感 edge dock、按需 contextual console。
- **Director（显式 opt-in，安全边界已实现）**：可保留密集工具，但不改变玩法语义、进度或回执因果；生产成功进入仍依赖可信 issuer。
- **信息层级**：`World / Targets / Command` 为主路径；`Search` 是 Targets 内的
  `#entity-search` 兼容过滤/导航能力，`Diagnostics` 次级；`Quote` 属于 Command/Console 上下文。
- **反馈**：`Action Receipt` 负责玩家因果，已实现的 `world_feed/v1` 仅是环境上下文投影。
- **Fullscreen HUD**：桌面默认以 `Objective / Next Move / Player Leverage` 三个紧凑卡片并列；
  移动端为 `Objective + Player Leverage` 紧凑首行、`Next Move` 第二行，`More / Diagnostics`
  仍从次级入口可达。

## 3. 当前实现与目标边界
- 当前 `main` 的 Player 是 fullscreen、world-first 单舞台：世界棋盘为首屏主体，
  `Objective / Next Move / Player Leverage` 是叠加 HUD，`World / Targets / Command`
  由按需 drawer 提供。旧“左侧/中间/右侧”三栏只适用于显式、能力门控的 Director
  工具密度，不能作为 Player 的当前 IA；三卡 HUD 也不等于旧三栏。
- 当前实现保留 focus/right-panel 兼容 hooks，且 `#entity-search` 仍是 Targets
  过滤入口；这些是实现基线/债务，不是第二产品模式或终端 authority。
- 当前页面仍以 Recent Events/Gameplay Feedback 等现有投影为准；它们不能被重命名为
  World Feed。独立 `world_feed/v1` 由 runtime journal 投影并通过 Viewer transport/panel
  呈现，`#viewer-world-feed` 是稳定实现 anchor，`receipt_ref` 无明确 runtime 身份时为 `null`。
- 目标契约暂不声称存在 `OASIS7_VIEWER_EXPERIENCE_MODE`、`ViewerExperienceMode`
  或 `RightPanel*` 运行时资源；需要这些字段时另开 runtime/viewer slice。
- Director 的 capability verifier、fail-closed endpoint 和 Player↔Director state machine
  已实现；launcher 无可信 issuer 时返回 `director_capability_unavailable`，所以这是
  已实现安全边界而非已放行的 operator 能力。目标实现仍须由 Viewer/QA 以真实桌面、移动、
  键盘和空/不可用状态验证。

### Expanded visual contract (target handoff)
- Player HUD 的层级固定为条件性 attention、`Objective`、一个主导性的 `Next Move`
  动作、`Player Leverage`/selection context、`Action Receipt`；World Feed 仍是收起的
  ambient chip。`world_read` 的 `tick/agents/routes/fragments/hotspots` 只有在既有
  projection 明确发布时才可作为次级摘要，不能假设 Power/Data/Consensus 等字段存在。
- `Next Move` 目标是单一 CTA + 内联 blocker；不要复制 Stage Hero 文案或让 World Feed
  代替 receipt。该层级是待实现的视觉目标，当前三卡 HUD 的实现边界仍以上文为准。
- Agent Console 默认先显示 selected context、授权 command/chat、blocker 与 receipt；
  raw snapshot/JSON、auth/capability、prompt override、world-scale 和 Director 工具
  默认收起到 `Diagnostics / More`，不改变控制权或 fail-closed 语义。
- Focus 的用户-facing 名称为 `Cinematic View / Minimal HUD`；保留 `focusMode` 兼容
  hooks。显式进入后只保留世界、选择标记、当前 objective/blocker、存在中的 receipt 和
  退出入口；Escape 按既有 local-surface → Focus 顺序处理，IME composition 优先。
- 这些约束不新增协议/runtime 字段，不改变仿真、replay、renderer、Director capability、
  ownership、经济或玩家权限；真实 headed 证据必须覆盖 `1440x1000` 与 `390x844`。

### Next optimization visual handoff (target; pending implementation)

The next bounded visual pass uses this reading order:

`connection/renderer/blocker attention → Objective → one dominant Next Move → selected Agent context / Player Leverage → Action Receipt → collapsed World Feed`.

- Objective and Next Move must not repeat the same instruction. Objective and Player Leverage
  may become typographic context; only Next Move needs the dominant actionable surface. This is a
  target, not evidence that the current three-cell responsive HUD has already been flattened.
- Action Receipt outranks ambient events and helper copy whenever present. Player and Cinematic
  must not show duplicate causal receipts; first verify the current normal/compact overlay geometry
  in a headed browser, then make presentation ownership explicit rather than masking overlap.
- Player-facing identity uses `name → label → id`. Raw IDs, `boundPlayer`, internal enums, and raw
  confidence values are secondary metadata or Diagnostics. Accessible names follow the same rule,
  selection has semantic state, and visible focus cannot depend on color alone.
- `LIVE` is a factual status, not decoration. The HUD must visually distinguish current, stale,
  reconnecting, offline, and renderer-unavailable states only from authoritative existing state;
  it must not synthesize liveness from an old snapshot or a running animation.
- Renderer Unavailable keeps the world shell usable and offers an honest visible recovery action.
  Retry does not promise that an incapable browser can acquire WebGL2 support.
- On narrow screens, Command keeps the selected readable Agent identity, truthful status,
  Objective, and close/back path visible while details scroll. Cinematic entry, readout, feed,
  fallback, diagnostics, and receipt each need a distinct safe-area slot.

#### Agent Context presentation boundary

`Agent Context Lite` may present explicit selected identity/location, Objective, published Next
Move, blocker, Player Leverage/execution state, and the latest explicit causal Action Receipt.
Non-causal feedback may remain visible as status only; it is never a Receipt. The formal causal
surface is the existing Viewer `data-viewer-overlay="receipt"` surface, with the existing
`#viewer-action-receipt` Player anchor where applicable, and there must be exactly one per active
Player/Cinematic presentation. Unknown, missing, local-pending, and stale values remain visibly
distinct. World Feed, recent event text, cursor movement, wall-clock time, coordinates, or raw debug
objects cannot create intent, progress, success, ETA, relationship, rationale, or a second Receipt.

`Agent Console V2` is a later contract-gated design surface. ETA, energy, cargo, module health,
relationships, plan/memory/rationale, provider or cost data, multi-Agent graphs, and new control
verbs require producer/runtime/agent definitions for source, freshness, permission, missing-state,
idempotency, and receipt semantics before visual design is treated as implementable.

#### P1-A Agent Context Lite（源码、自动化与 headed 验收已完成）

P1-A 先实现一个 Agent-only composition，作为后续实体 Inspector 的窄入口；Location、
Facility、Territory、Organization、Depot、Module 的 generalized Inspector 明确是 post-P1-A。
它不把当前 Player shell 扩成常驻三栏，也不声称已实现完整的 Agent Console V2。

- **Composition seam**：设计绑定现有
  `crates/oasis7_viewer/software_safe_src/main.jsx` `InteractionPanel`。该组件继续负责
  Command/chat 的 route 和 control boundary；Agent Context Lite 作为其中的 selected-Agent
  display region，而不是新的 dock 或独立 runtime surface。
- **Display-model seam**：canonical extraction 固定为
  `crates/oasis7_viewer/software_safe_src/viewer_agent_context_display_model.js`（纯
  source-level projection）与 `crates/oasis7_viewer/software_safe_src/agent_context_lite.jsx`
  （只接收 display inputs/callbacks 的组件）；二者已实现。输入只包括
  当前 selected Agent、`snapshot.model.agents[selectedAgentId]` 的显式 identity/activity、
  显式绑定同一 Agent 的 gameplay projection、connection/freshness 和既有
  semantic feedback/explicit receipt。反馈只能输出 non-causal status；它输出可渲染的
  sections 与 source/freshness/permission 状态，不发送动作、不写 core state、不读取 raw
  debug object 来补字段。Player-global `core.buildGameplaySummary(locale())` 保留在 Player HUD，
  不得注入 Agent Context；`InteractionPanel` 只负责组合，不继续 inline expansion。
- **Existing semantic surfaces**：`crates/oasis7_viewer/software_safe_src/agent_activity_surface.jsx`
  仍只表达 activity，`crates/oasis7_viewer/software_safe_src/agent_intent_surface.jsx` 仍只
  表达权威 Intent；它们可被组合但不能合并语义。世界舞台
  `crates/oasis7_viewer/software_safe_src/pixel_world_host.jsx` 的 `PixelWorldCommercialHud`
  和现有 `Action Receipt` 保持回执所有权，Context Lite 不创建副本。
- **Reading order**：Command/chat → Agent Context Lite → gameplay details → collapsed
  Diagnostics/raw state。Context 内部为 Identity → State/freshness → Objective/Next Move /
  blocker/Leverage → explicitly matched Intent；Next Move 仍只有一个主 CTA。
- **Non-Agent selection**：Location、Facility、Territory、Organization、Depot、Module 选择时显示选中实体 identity
  与 honest unavailable/待同步状态，不保留或借用上一 Agent 的 State、Intent、capability、
  owner、resource 或 relation。只有未来 projection contract 齐备后才可增加对应 section。
- **Causal discipline**：Context 只引用当前 Player/Cinematic 的唯一 Receipt anchor。Feed、
  recent events、chat echo、cursor、coordinates 和 wall-clock 不得产生 Intent、progress、
  success、ETA、relationship 或 rationale。

P1-A 的视觉验收只针对 selected Agent、缺失/陈旧/不可用态和非 Agent honest-unavailable 态；
必须覆盖 `1440x1000`、`1024x768`、`768x1024`、`390x844`、键盘/IME/CJK/长文本、Focus 返回
路径和单一正式 Receipt，并覆盖 current、last-known/stale、reconnecting、unknown、conflict、
replay、gap、reorg、unavailable 与 control/permission lost。源码、
focused 自动化和真实 WebGL headed 验收已完成；本节仍不单独构成发行放行证明。

P1-A 的 canonical acceptance crosswalk 为：`P1A-CTX-01 → ENT-01/CTX-01/EMP-01`、
`P1A-CTX-02 → STA-01/CTX-01/FED-01/FED-02`、`P1A-CTX-03 → INT-01/REC-01/REC-02`、
`P1A-CTX-04 → CAP-01/CON-01`、`P1A-CTX-05 → IA-01/HUD-01/TXT-01/FOC-01/FOC-02`。
这些是当前实现与 QA 的证据映射；发行放行仍服从仓库 terminal gates。

#### P1-B World semantic presentation（当前切片源码与 headed 验收已完成）

P1-B 将既有权威 world projection 组织成稳定的 stage visual language。可呈现的内容包括
Agent、Location、Facility、Territory、Organization、Depot、Module 的 identity/state/activity，
以及已发布的 route/assignment、power/resource、facility/module cues 和 relation kind/status；
没有 projection 时保持 unknown/unavailable，不由 Viewer 补造。

- **Attention order**：普通 Agent < selected Agent < active Intent < blocker < receipt target。
  Major event 只在 P1-C authority gate 通过后作为后置扩展；persistent selection 与 transient
  attention 必须有不同的视觉/语义状态；
  hit target、keyboard focus、selected state 和 readable identity 在高注意力状态下仍保持。
- **Authority boundary**：relation、Trust、ownership、resource、capability 和事件因果只能
  来自权威 projection；距离、同屏、proximity、文本、坐标或 renderer activity 不能推断
  这些语义。关系线只有在 projection 提供 relation kind/status 时才可绘制。
- **Renderer separation**：Renderer Unavailable、probe 后异步失败、fallback、retry、
  safe-area 和 recovery 属于跨切片安全验收（`REN-01`），不是 P1-B 的语义来源。P1-B 不新增或
  修改 upstream runtime/world/protocol/WASM ABI 字段；Viewer host/bridge 只可从既有 authoritative
snapshot 派生 additive、只读、fail-closed 的 presentation DTO，不得新增事实、权限、生命周期、
控制或因果语义。`active_intent_target` 只能来自 `snapshot.player_gameplay.primary_intent`。
  即使 Intent/Agent position 存在，只要 authoritative world bounds 缺失，也不得用 Agent ID 或
  fallback 坐标生成 stage cue。
  headed evidence 已覆盖 `1440x1000`、`1024x768`、`768x1024`、`390x844`、键盘/CJK/长文本，
  并以语义层级、焦点与无横向溢出为判据，而非把 canvas readiness 当作语义证明。

#### P1-C Major World Events（当前切片已实现 crisis-only 最小合同）

runtime 已用独立 additive projection 冻结 crisis-only authority：三种 canonical Crisis lifecycle、
severity `1..=5`、journal identity、显式 freshness/permission、非 Receipt lineage 与 gap/reorg
清理规则。World Feed wire 保持向后兼容，Viewer 对缺失、未知、冲突或未授权字段 fail closed。
当前 crisis 没有权威空间 anchor，因此只显示 ambient feed/status，不生成 marker/highlight，也不从
event text、距离、同屏、时间、cursor、resolver 或 renderer activity 推断空间与因果。War、
Governance 和其他类别仍明确排除；生产入口只读取独立的 runtime/operator audience policy，
默认 Unknown，不从登录、选择、控制或 Director diagnostics 推导 Public。

### World-native interaction composition (target handoff)

The visual reference contributes a stable frame, not a permanent Player layout. Keep stable slots for
the stage, Objective, selected-agent context, one Next Move, Receipt, and edge navigation; let
Inspector, Command, Feed, and Diagnostics appear on demand and return focus to their invoker. A
Player left/center/right column or an always-on bottom bar is explicitly rejected.

- **World explains semantics**: stage marks should make published identity, state, activity, route,
  relationship kind/status, and major events legible at the entity. UI copy may explain or act on the
  mark, but may not turn the stage into background decoration or infer a relation from distance,
  proximity, co-presence, or event text.
- **Entity-first context (post-P1-A)**: P1-A is selected-Agent-only. A later P1 slice may let
  selecting a Location, Facility, Territory, Organization, Depot, or Module open a consistent
  Inspector/Console composition, but it cannot override or widen the Agent-only P1-A boundary.
  Market, War, Governance, Production, Contracts, and Relations are contextual capabilities,
  filtered by published data and player permission, not peer navigation routes. A missing projection
  is an honest unavailable state.
- **Indirect-control rhythm**: the primary action is a high-level guidance/template (`Ask`, `Suggest`,
  `Prioritize`, `Negotiate`, `Investigate`, `Propose`, `Warn`, or `Delegate`). Render the sequence
  `guidance → Intent → accepted/rejected/blocked → Activity → world action → Receipt`; accepted is
  never executing, executing is never completed, and only a bound receipt can announce world change.
- **Persistent causal anchor**: keep one compact Receipt anchor visible for the active Player or
  Cinematic presentation until a newer authoritative receipt replaces it. Transient toasts may call
  attention but cannot replace or create a receipt. Keep World Feed as a collapsed ambient chip with
  loading/empty/replay/gap/unavailable states, never as causal feedback.
- **Player language**: use Program, Protocol, Module, and Contract in ordinary copy. Keep WASM, ABI,
  hashes, deployment transactions, runtime IDs, provider details, and auth material behind explicit
  Diagnostics/Director disclosure; raw fields do not create an apparent capability.

Visual states must carry source and freshness cues without exposing raw enums: current, Last known,
stale, reconnecting, unknown, conflict, unavailable, and control lost need distinct treatment. Missing
or unauthorized data is hidden/redacted or labeled unavailable; it is never replaced with a guessed
resource, owner, trust, relationship, liveness, or action result. These are target design rules, not
evidence that the current implementation already satisfies them.

#### Headed visual and interaction matrix

- `1440x1000`: GPU-ready Player and Cinematic; one Receipt; default hierarchy and focus return.
- `1440x1000`: Renderer Unavailable, visible recovery, and post-probe asynchronous failure.
- `1024x768` and `768x1024`: tablet/narrow-desktop HUD, feed, drawer, receipt, and safe areas.
- `390x844`: Player, Targets, Command, Cinematic, long entity names, long receipt copy, fallback.
- `390x844`: empty world and missing/stale/permission-lost entity data retain a usable stage,
  explicit recovery copy, and non-overlapping Receipt/Feed/fallback slots.
- Entity-selected desktop/mobile states (post-P1-A): Agent is covered by P1-A; Location, Facility,
  Territory, Organization, Depot, and Module require later slices and their own authority. Stage
  semantics and relation lines remain readable in sparse worlds.
- Program/Protocol states: ordinary copy contains no raw WASM/ABI/hash/transaction/runtime fields
  until the user explicitly opens Diagnostics.
- Every viewport: English/Chinese/CJK, long labels, no horizontal overflow, Tab/Shift+Tab,
  Escape priority, IME composition, visible focus, and semantic selected state.
- Stateful regressions: reconnect continuation, gap/reorg recovery, stale/offline truth, duplicate
  receipt, raw enum/ID leakage, and Advanced disclosure containment.

These rows are target evidence requirements. Historical GPU-ready or GPU-unavailable screenshots,
source tests, and CSS arithmetic do not prove exact-head headed readiness.

## 4. 约束与边界
- 本设计只重排 Viewer 的视觉/交互，不改网络协议、仿真核心逻辑和 `third_party`。
- Search/selection 只导航、居中和打开上下文；执行只能从已授权 Command/Console 路径发生。
- queued/accepted 不等于完成；无 receipt 时显示明确的 `No action receipt yet`。
- 所有实现必须保留键盘焦点、关闭/`Escape`、CJK/长文案和无横向溢出的可读性契约。

## 5. 设计演进计划
按以下阶段演进；每一阶段都必须回到配对 PRD 的 source/freshness/permission/receipt 合同，
不能把设计稿或静态截图当作实现完成：

1. **P0 Shell Lite**：冻结稳定 slot、edge navigation、Objective/selected-agent context、
   单一 Next Move、compact persistent Receipt 和 collapsed Feed；在不增加协议字段的前提下
   收敛现有 shell 的默认可见性与响应式布局。
2. **P1 Context + world semantics**：由 `viewer_engineer`/runtime 组合统一 Inspector/Console，
   让已发布的实体状态、Intent/Activity、路线、关系和重大事件可在 World Stage 或 contextual
   surface 中解释，并由 `qa_engineer` 用 headed desktop/mobile、键盘/CJK/空态验证。
3. **Deferred / contract-gated**：全局资源框、完整 capability hotbar、未经来源/权限/时效
   合同的经济/关系/健康字段，以及新的控制 verbs 延后；需 producer/gameplay/runtime/agent
   owner 先冻结语义与权限。GPU-enabled WebGL2 `ready` 与 GPU-disabled `Renderer Unavailable`
   fallback 的已有证据不代表任一阶段的发行结论。

## 6. 终端交互与实施边界
- Fresh load/reload/new tab/session 一律进入 Player。Director 只能由 Player 的
  capability-gated Diagnostics/operator action 显式打开，且仅改变可见性/密度、
  不持久化、不改变 command/auth/ownership/runtime 或 receipt 因果。
- Director entry 无效、过期、撤销或未授权时 fail closed 回 Player，清理 Director
  surfaces，保留 world/selection，并给出恢复文案。
- `World / Targets / Command` 是同一概念路径；Search 留在 Targets，Quote 留在
  contextual Command；选择只导航/高亮，不执行。
- Escape 先关闭局部 surface/drawer，再退出 Focus/presentation；IME composition
  期间由 IME 消费 Escape，关闭后焦点回到 invoker。
- 实施顺序固定为 anchors/tests → shell layout → console/receipt → focus/a11y →
  World Feed v1 → headed QA；当前 P1 切片已有源码/协议实现与聚焦自动化，headed 证据仅覆盖
  外部 Chrome 的 GPU-unavailable fallback（`1440x1000`、`390x844`）。`ready=true`
  WebGL2/Cinematic 正常路径、probe 后异步 GPU 失败和 tablet `641–1240` 宽度仍待验证，
  因此当前只维持 technical preview。issuer-backed Director allowed path 和仓库级 runtime/QA
  release verdict 仍是独立终端证据，不由这部分 headed 结果替代。

World Feed 现作为 `world_feed/v1` additive ambient projection：identity/dedup 为
`(world_id,reorg_epoch,event_seq)`，source order ascending；nullable receipt link
只能来自显式 runtime causal identity，当前 runtime 明确保持 `receipt_ref=null`。
loading、empty、replay、gap/reorg 和
unavailable 不可合并，当前 Recent Events/Feedback 名称保持不变。Fullscreen Player
默认将其作为右上紧凑折叠 edge overlay/ambient chip，按需展开，不形成常驻列。

### 已知 feed/readout 缺陷的修复状态（#3315）

- **P1 cursor continuation：已修复。** `world_feed_transport.js` 会按返回 cursor
  异步继续请求，并在世界活动后刷新最新 cursor；对应测试为
  `world_feed_transport.test.js` 中的
  `continues an initial page asynchronously from the returned cursor` 与
  `refreshes the latest cursor once after world activity and does not overlap in-flight requests`。
- **P2 gap/reorg 旧事件保留：已修复。** `consumeWorldFeed()` 在 gap 时清空旧
  `events`，再要求权威 snapshot reload；对应测试为
  `world_feed_state.test.js` 的
  `stops appending on gap/reorg and requires authoritative snapshot reload`。
- **P2 多资源 raw fallback：已修复。** 资源 formatter 逐项解析 Host 以 ` · ` 拼接的
  条目；对应 Rust 测试为
  `render_selected_resource_readout.rs` 的
  `resource_readout_formats_each_structured_entry_without_raw_json`。

这些是 #3315 的 focused implementation/test evidence；外部 Chrome 另在 headed
`1440x1000` 与 `390x844` 复核了 GPU 不可用 fallback。该浏览器证据不覆盖
`ready=true` WebGL2/Cinematic 正常路径、probe 后异步 GPU 失败或 tablet `641–1240`
宽度，也不是发行放行结论；#3248 浏览器证据仍是历史记录。
