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
Move, blocker, Player Leverage/execution state, and the latest authoritative receipt/feedback.
Unknown, missing, local-pending, and stale values remain visibly distinct. World Feed, recent event
text, cursor movement, wall-clock time, coordinates, or raw debug objects cannot create intent,
progress, success, ETA, relationship, or rationale.

`Agent Console V2` is a later contract-gated design surface. ETA, energy, cargo, module health,
relationships, plan/memory/rationale, provider or cost data, multi-Agent graphs, and new control
verbs require producer/runtime/agent definitions for source, freshness, permission, missing-state,
idempotency, and receipt semantics before visual design is treated as implementable.

#### Headed visual and interaction matrix

- `1440x1000`: GPU-ready Player and Cinematic; one Receipt; default hierarchy and focus return.
- `1440x1000`: Renderer Unavailable, visible recovery, and post-probe asynchronous failure.
- `1024x768` and `768x1024`: tablet/narrow-desktop HUD, feed, drawer, receipt, and safe areas.
- `390x844`: Player, Targets, Command, Cinematic, long entity names, long receipt copy, fallback.
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
1. 先冻结配对 PRD 的 shell、dock、console、HUD、receipt/feed 语义和稳定锚点。
2. 再由 `viewer_engineer` 在不新增协议字段前提下收敛当前实现的可见性/响应式布局。
3. 最后由 `qa_engineer` 以真实 headed 浏览器 `1440x1000` 与 `390x844` 截图、键盘、长文案和空/不可用状态验证；GPU-enabled
   WebGL2 `ready` 与 GPU-disabled `Renderer Unavailable` fallback 已有证据，但本设计不构成
   发行结论。

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
  World Feed v1 → headed QA；前五项已有源码/协议实现与测试，真实截图、浏览器 smoke、
  issuer-backed Director allowed path 和 runtime/QA release verdict 仍是后续证据。

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
