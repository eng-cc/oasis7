# oasis7 Viewer 使用说明书

审计轮次: 10

## 文档定位
- 本文件是 Viewer 使用说明的 canonical `*.manual.md` 入口。
- 历史兼容路径已删除；请直接使用本 canonical `*.manual.md` 手册。
- 系统级测试分层与 suite 选择仍以 `testing-manual.md` 为权威总入口。

## 目标
- 提供 `viewer` Viewer Web 主入口的统一操作手册，并说明 `software_safe` 兼容 alias。
- 统一 live server、Web 静态入口、agent-browser 闭环与常见排查步骤。
- 明确当前仓库已不再提供退役的第二 Viewer 工具链。

## 适用范围
- live server：`crates/oasis7 --bin oasis7_viewer_live`
- Web 静态入口：源码与发布 canonical 页面使用 `crates/oasis7_viewer/viewer.html`；`software_safe.html` 作为 compat HTML 副本继续同步产出
- Web 启动脚本：`scripts/run-viewer-web.sh`
- Web 回归脚本：
  - `scripts/viewer-primary-web-entry-regression.sh`
  - `scripts/viewer-software-safe-step-regression.sh`
  - `scripts/viewer-software-safe-chat-regression.sh`
- 边界说明：本手册只适用于 `viewer` Viewer Web 页面（兼容 `software_safe` alias），不适用于 `oasis7_web_launcher` / launcher Web 控制面；后者默认先走 GUI Agent。
- 本地真实 LetAI provider-backed 游戏试玩不从裸 `run-viewer-web.sh` 开始；统一使用 `./scripts/run-local-letai-game-test.sh` 启动完全本地的 bridge + runtime/game 栈。该入口不连接 formal/public testnet。

## 本地真实 LLM 游戏测试
```bash
./scripts/run-local-letai-game-test.sh --local-world-playtest
```

- 该入口负责 LetAI token config 规范化、默认 Rust direct `127.0.0.1:5841` provider bridge、Rust bridge chat probe/provider contract smoke 与 launcher/runtime/viewer 启动。
- 日常手工试玩只用 `--local-world-playtest`。该 preset 固化以下易错项：`--startup-profile playtest`、`--provider-smoke-mode skip`、`--reuse-existing-build`、`--detach`、默认 `--auto-play`、viewer/web/live 端口 `48420/48421/48422`、`--json-ready`，以及 wrapper 默认的本地 standalone chain。若需要复现暂停态或手动 Play 调试，可显式传 `--no-auto-play`。
- 启动后等 `<output-dir>/launcher/session.meta` 出现 `STACK_READY=1`，再打开其中 `GAME_URL`。默认日常 URL 形如 `http://127.0.0.1:48420/?ws=ws://127.0.0.1:48421&test_api=1&locale=zh`。
- 需要冷构建、严格 provider smoke 或换端口时，再显式展开参数；例如第一次构建可去掉 `--local-world-playtest` 或去掉 `--reuse-existing-build`，临时换 HTTP 端口可在末尾透传 `-- --viewer-port 4174 --json-ready`。
- 默认 chain-enabled 路径会启动 launcher-managed chain runtime，并通过 `--chain-local-standalone-test` 保持本地 submit -> commit -> snapshot 闭环可在单节点试玩栈内完成。该路径只有在 `output/chain-runtime/<node-id>/reward-runtime-execution-world/snapshot.json` 与 `journal.json` 都出现后，才算 chain-enabled 本地世界就绪。
- launcher 管理 `oasis7_chain_runtime`、进程编排与持久 execution-world readiness；Viewer 只观察 runtime/world 输出。snapshot/event 是观测面，恢复与 replay 仍以权威 state + journal/event chain 为准。
- 如果页面停在“认领已提交，正在等待链上 committed 快照同步”，先检查启动命令是否漏掉 local standalone chain 配置；漏掉时 gameplay action 可能已进入 pending consensus queue，但本地单节点不会完成 commit/snapshot。
- 如果页面短暂打开后出现 `viewer.ws` / WebSocket 错误，先检查脚本输出目录下的 `launcher/oasis7_viewer_live.log` 是否报 execution-world persistence ready gate，而不要先把它归因成 Viewer 前端问题。
- 如只为人工查看页面或排查 WebSocket，可在 wrapper 参数后透传 `--chain-disable` 作为临时 page-play mitigation；该模式不代表本地 standalone chain-enabled 世界启动通过，更不能作为 public_testnet 证据。
- 下方 `oasis7_viewer_live` + `run-viewer-web.sh` 流程只用于 Viewer/debug 或定向 Web 回归，不代表本地真实 provider-backed gameplay 栈。

## 底层 Viewer Debug 快速开始

### 1）启动 live server
```bash
env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_viewer_live -- llm_bootstrap --allow-debug-scenario --llm --bind 127.0.0.1:5023 --web-bind 127.0.0.1:5011
```

- `oasis7_viewer_live` 当前默认走 runtime/world 链路。
- `oasis7_viewer_live` 负责 Viewer live server 与可选 Web bridge，不内嵌 node、consensus、reward runtime、topology 或 execution-world persistence，也不提供 `--tick-ms` 作为外部推进时钟。它仍可通过 `--chain-status-bind` 观察 committed world，并在配置 status bind 时通过 `--chain-submit-bind` 提交 chain-linked action；这些是 client endpoints，不授予 node ownership。Viewer event delivery 不能等同于 node/consensus tick。
- 正式 gameplay 要求已配置且可连通的 LLM provider。
- 若显式改用 `--no-llm`，则该链路只可用于 observer/debug，不计入正式 gameplay 证据。

### 2）启动 Web Viewer
```bash
env -u NO_COLOR ./scripts/run-viewer-web.sh --address 127.0.0.1 --port 4173
```

- 默认访问地址：`http://127.0.0.1:4173/?ws=ws://127.0.0.1:5011`
- 当前仓库只提供 `viewer` 单一 Web / UI 入口；`software_safe` 只保留为兼容 alias，不再作为正式模式名。

### 3）前置依赖
- Node.js / npm
- `python3`
- 若要跑 agent-browser 闭环，还需安装 `agent-browser`

## 页面能力
- 当前页面聚焦 `viewer` 实时观察与正式玩法摘要。
- 支持 `locale=zh|en` 初始化和页面内中英文切换。
- 支持最小 prompt/chat 控制面；仅在 auth/bootstrap 可用时开放。
- 在 `hosted_public_join` 路径下，页面支持获取/释放 hosted `player_session`、`reconnect_sync` 恢复，以及 `prompt_control` 的 preview-grade `strong_auth`（需 `Backend Approval Code`）。
- 面向普通用户的启动入口现在默认且只暴露 `hosted_public_join`。旧的 `trusted_local_only` 本地可信预览不再作为可选用户流程；本地旧配置应迁移到 hosted join，并走邮箱 hosted account / player session 登录链路。
- `main_token_transfer` 仍保持阻断，页面只显示 lane verdict，不提供资产转账表单。
- 页面不再提供第二 Viewer 跳转，也不再承担退役视觉专项工具职责。

### Terminal shell target versus current operation
- 终端 shell 的 Player/Director、`World / Targets / Command`、Search/Quote、
  Action Receipt 与 World Feed v1 产品契约，以
  `viewer-gameplay-release-experience-overhaul.prd.md` 为准。
- 当前任务分支的 Player 操作路线已经是 stage-first：Targets 继续使用
  `#entity-search` 过滤/导航，Targets 与 Command/Chat 通过按需 drawer 进入，
  Diagnostics 保持次级，Recent Events/Feedback 仍按现名显示。Focus hooks 只服务
  Player 沉浸呈现，不是第二产品模式。World Feed v1 已有 runtime transport、状态机
  与 `#viewer-world-feed` panel，但仍是只读环境上下文，不是 Action Receipt 替代。
- 当前 Player 已是 fullscreen world-first 单舞台；不要按旧“左侧/中间/右侧”三栏 IA
  操作或编写说明。三张 `Objective / Next Move / Player Leverage` 卡片是 HUD，不是
  旧三栏；密集三栏仅可能在显式、能力门控的 Director 工具视图出现。
- Director 的 `#viewer-director-entry` / `#viewer-director-exit`、Viewer 状态机与
  `/api/public/director/capability` fail-closed endpoint 已实现。当前 endpoint 在没有
  可信 issuer 绑定 live `session_epoch` 时返回 `director_capability_unavailable`，
  因此生产环境成功进入仍是 `capability_blocked`；queued/accepted 也不等于完成。

### Expanded Player visual contract (target handoff; not current release evidence)
- 实现目标的阅读顺序是 attention（仅在需要时）→ `Objective` → 一个主导性的
  `Next Move` 动作 → `Player Leverage`/selected context → `Action Receipt`；World Feed
  继续作为收起的环境 chip。`world_read` 的 `tick/agents/routes/fragments/hotspots` 只
  在既有权威 projection 提供时显示，不能补造 Power、Data、Consensus 或其他指标。
- `Next Move` 的目标交互是单一 CTA 和内联 blocker；当前三卡 HUD 是实现基线，完成该
  目标前不要把目标层级写成已经交付。
- Agent Console 默认显示当前选择、已授权的 command/chat、blocker 与 receipt。无选择或
  不可控时只提供 Targets/认领/等待 binding 的恢复说明；raw snapshot/JSON、auth/
  capability、Prompt override、world-scale 和 Director 工具均保持在收起的
  `Diagnostics / More` 中。
- 用户-facing Focus 名称为 `Cinematic View / Minimal HUD`，内部 `focusMode` hooks
  保留。显式进入后保留世界、选择标记、当前 objective/blocker、存在中的 receipt 和
  退出入口；Escape 先关局部 drawer，再退出呈现层，IME composition 不被打断。
- 这只是待实现的视觉 handoff，不改变 runtime/protocol、仿真/replay、renderer、权限、
  Director capability 或 receipt 因果。后续 QA 必须提供 headed `1440x1000` 与 `390x844`
  截图，并覆盖键盘、IME、CJK、长文本和无横向溢出。

### World-native target handoff（目标契约，不是当前实现声明）

本轮升级借鉴的是稳定的阅读框架，不是 Agentbox 的永久三栏布局。稳定的是世界舞台、
Objective、selected-agent context、单一 `Next Move`、Receipt 和边缘导航的阅读顺序；
Inspector、Command、World Feed 与 Diagnostics 仍按需出现，并在关闭后回到触发它的入口。
Player 不采用永久左/中/右列，也不采用可直接执行世界动作的底部 hotbar。

- **世界解释语义**：World Stage 直接表达已发布的实体身份/状态、活动/路线、关系类型/状态
  和重大事件。周边 UI 只负责解释、定位和操作；不能由距离、同屏、靠近、数量或 event text
  猜 owner、resource、trust、relation 或因果。
- **实体优先**：从选中的 Agent、Facility、Territory 或 Organization 进入 contextual
  Inspector/Console；Market、War、Governance、Production、Contracts、Relations 是实体能力，
  不是新的一级 Player 路由。缺少权威 projection 或权限时显示 unavailable/只读恢复路径。
- **间接控制**：玩家发送 `Ask`、`Suggest`、`Prioritize`、`Negotiate`、`Investigate`、
  `Propose`、`Warn` 或 `Delegate` 等 guidance/template。必须能区分
  `Guidance → Intent → accepted/rejected/blocked → Activity → World Action → Receipt`；
  accepted 不等于 executing，executing 不等于 completed。
- **因果与环境**：当前 presentation 只保留一个紧凑、可持续复查的 Action Receipt；新 toast
  不能覆盖或伪造 Receipt，只有新的权威 receipt 才能替换它。World Feed 默认是收起的 ambient
  chip，loading/empty/replay/gap/unavailable 各自有状态，绝不代替 Receipt。
- **玩家语言与诊断**：普通界面使用 `Program`、`Protocol`、`Module`、`Contract`。WASM、ABI、
  hash、deployment tx、runtime id、provider 和 auth material 只在显式 Diagnostics/Director
  disclosure 中展示；raw object 不能创造一个玩家可用的 capability。

可见值必须带有可追溯 source、logical position、freshness 和 permission 语义：current、Last
known/stale、reconnecting、unknown、conflict、unavailable、control lost 要有可区分的玩家
反馈；缺失或无权限时隐藏/脱敏/说明恢复，不用默认值或浏览器时间填空。以上是分阶段目标，
不表示当前 Web 已完成这些改造或已获得发行放行。

### World-native rollout（目标顺序）

1. **P0 Shell Lite**：收敛现有 fullscreen shell 的阅读顺序、单一 Next Move、轻量 Objective/
   selected-agent context、Receipt anchor、collapsed Feed 和 edge navigation。
2. **P1 Context + semantics**：在已有 projection 上组合按需 Inspector/Console，并补充实体/Intent/
   Activity/关系/事件的 headed、键盘、CJK、空态和 fallback 证据。
3. **Deferred / contract-gated**：全局资产框、完整 capability hotbar、新资源/ownership/
   relationship/经济字段和新的取消/批准/重排动作，待产品、玩法、runtime、agent 的规则、
   source、freshness、permission、idempotency 与 receipt 合同明确后再实现。

### Terminal Player shell playbook
This section describes the implemented Player-shell, World Feed, and Director
security-boundary routes in the current task branch. It is not a release claim:
World Feed has no inferred receipt links, and production Director success remains
blocked until a trusted issuer is available.

1. Fresh load/reload/new tab/session starts in Player. Read the stage, compact
   Objective/Next Move/Player Leverage HUD, then use the quiet `World / Targets /
   Command` route. In fullscreen desktop the three HUD cards share one compact row;
   on mobile Objective and Player Leverage share the first row and Next Move occupies
   the second row. `More / Diagnostics` remains reachable as a secondary route.
2. Search remains inside Targets and only filters the authoritative visible list.
   Selecting a target recenters/highlights and opens context; it never executes.
   Quote is reached from contextual Command, not a peer rail item.
3. Action Receipt is the only player-causality surface. `accepted`/`queued` is
   waiting, not completion; no receipt is shown explicitly as `No action receipt yet`. The target
   compact presentation keeps one anchor available for review until a newer authoritative receipt
   replaces it; transient feedback cannot create a second causal surface. This target is not current
   release evidence.
4. World Feed is the implemented `world_feed/v1` ambient projection. Its identity/dedup
   key is `(world_id,reorg_epoch,event_seq)`, source order is ascending, and the current
   runtime emits nullable `receipt_ref=null`; a link is rendered only when runtime
   supplies explicit causal identity. In fullscreen Player it is a compact collapsed
   top-right edge overlay/ambient chip that opens on demand, never a persistent column.
   Wire `reorg_epoch` and `event_seq` are exact decimal strings (legacy numeric input
   remains accepted); the Viewer compares them exactly and presents canonical ascending
   event order.
5. Director is a capability-gated secondary Diagnostics/operator action, ephemeral to
   the tab. The server validates a short-lived `director_open` grant; invalid/stale/
   revoked/unauthorized/unavailable entry returns to Player, sanitizes Director
   surfaces, preserves world/selection, and explains recovery. No trusted issuer is
   currently wired, so the successful production path remains `capability_blocked`.
6. Escape closes the local surface/drawer first, then Focus; while IME composition
   is active it must not close the Viewer surface. Closing returns focus to the invoker.
7. Target Director controls use `#viewer-director-entry` and `#viewer-director-exit`;
   these source anchors are implemented but do not persist capability. Fresh load,
   reload, and new tab remain Player; denied/stale/revoked/unavailable capability fails
   closed to Player.

8. The canonical pixel renderer is the WebGL2 runtime. GPU-enabled environments have a
   `ready` path; when WebGL2 is unavailable (including `canvas.getContext("webgl2")`
   returning null), the page exposes `Renderer Unavailable` with a diagnostic reason and
   does not present a false ready canvas.

### Resolved World Feed and resource-readout defects (#3315)

The following focused implementation defects are resolved on this branch; use the
named tests as the current evidence, not the historical browser artifacts:

- **P1 cursor continuation:** `world_feed_transport.js` now continues an initial page
  from the returned cursor and refreshes once after world activity. See
  `world_feed_transport.test.js` tests
  `continues an initial page asynchronously from the returned cursor` and
  `refreshes the latest cursor once after world activity and does not overlap in-flight requests`.
- **P2 gap/reorg retention:** `consumeWorldFeed()` clears stale `events` on a gap before
  snapshot reload. See `world_feed_state.test.js` test
  `stops appending on gap/reorg and requires authoritative snapshot reload`.
- **P2 multi-resource fallback:** the selected-resource formatter now parses every
  ` · `-joined host entry. See Rust test
  `render_selected_resource_readout.rs::resource_readout_formats_each_structured_entry_without_raw_json`.

The #3248 browser evidence referenced by the surrounding Viewer docs is historical. For
#3315, external Chrome freshly verified only the GPU-unavailable fallback at headed
`1440x1000` and `390x844`; the `ready=true` WebGL2/Cinematic path, post-probe async GPU
failure, and tablet widths `641–1240` remain unverified. This is browser evidence for the
fallback surface, not a release-readiness claim.

### Target QA evidence checklist
The implementation owner must supply headed evidence for desktop `1440x1000`, tablet orientations,
and mobile `390x844` shell hierarchy, Director allowed/denied/stale/revoked entry (the allowed path is
currently blocked by the missing trusted issuer), duplicate-ID/anchor
checks, keyboard/IME/Escape focus return, receipt states, world semantic entity/relationship/event
rendering, entity-first contextual capabilities, Programs/Protocols disclosure, empty-world and
missing/stale/permission-loss states, feed loading/empty/replay/gap/unavailable/reorg recovery, and
English/Chinese long-text overflow. Source/package
tests now cover the World Feed and fail-closed Director boundary; this manual does not
turn those tests into issuer-backed production access or a release claim.

### 当前 Agent Chat 与高级 Prompt 设置

- Chat 和 Prompt 控制只对当前账号已绑定/权威认领且当前可控制的 Agent 开放；选中共享世界中的其他 Agent 不会授予控制权。无可控制 Agent 时，页面保持 blocked，并引导先认领 Agent 或等待 binding sync。
- `Agent Chat` 是面向当前 Agent 的消息入口。发送成功只表示对应请求结果，不会绕过 runtime 权威裁决，也不证明产生了世界效果。
- 当前 canonical Web 使用普通多行输入框和显式 `Send Chat` 动作；未定义 Enter 发送快捷键，Enter/Shift+Enter 按浏览器多行文本编辑处理。当前文档不声明中文 IME 组合态或自动聚焦已经获得跨浏览器专项验证。
- `Advanced Prompt Settings` 属于 operator-level 控制，默认收起，不与玩家主路径竞争。显式展开后，当前页面提供 system/short-term/long-term override、preview、apply、rollback 与最近 Prompt feedback。
- preview 不等于 apply；apply acceptance 不等于 runtime 已应用。以页面返回的实际 feedback 为准，失败或未授权必须保持 blocker/error，不能显示假成功。
- `hosted_public_join` 下的 Prompt apply 需要有效 `player_session` 与后端重新授权；当前入口使用 `Backend Approval Code` 满足 preview-grade `strong_auth`。缺失、过期或拒绝时应重新注册/认证或按页面提示恢复。
- 产品层的对话、草稿、默认/override 与反馈承诺见 [`Agent 对话与 Prompt 控制`](../../product/agents-world-simulation/agent-conversation-and-prompt-control.prd.md)；自动化 grammar 与状态字段见 [`viewer-web-semantic-test-api.prd.md`](viewer-web-semantic-test-api.prd.md)。

### 已退役的 legacy 操作

历史 Standard-3D / egui 表面曾提供 auto select、右侧模块显隐及本地缓存、选中详情、Agent 快速定位、2D 全览图分层缩放和可复制文本面板；该表面及这些操作均已退役，不能作为当前 canonical `viewer` Web 页面的使用说明或能力声明。当前支持边界仍以实时观察、当前选中对象、prompt/chat、`window.__AW_TEST__` 以及本手册列出的三条回归脚本为准。

## 证据边界
- 当前 formal gameplay PASS 证据以 `doc/testing/evidence/software-safe-primary-web-entry-evidence-2026-04-07.md` 为准。
  - 该证据包含 2026-04-08 的 LLM-enabled follow-up PASS，结论是当前 Web 主入口已具备 release-grade formal gameplay 样本。
- `doc/world-runtime/evidence/formal-release-fixed-genesis-default-viewer-2026-05-16.md` 只证明 formal-release 技术启动状态在 `--no-llm` 观察链路下可加载、可连接、可选中实体。
  - 这份截图证据属于 observer/debug，不替代 formal gameplay 证明。

## Web 闭环

### 进程与 Launcher 边界

- `oasis7_viewer_live` 不启动或拥有 chain node、consensus gate、reward runtime、topology 或 execution-world directory。
- chain-linked formal/local evidence 使用 Launcher 管理的 `oasis7_chain_runtime`；Viewer 的 `--chain-status-bind` / `--chain-submit-bind` 只承担 client linkage。
- 退役的 embedded-node flags 必须失败并引导到 `oasis7_chain_runtime` / `oasis7_game_launcher`；编排、stale-world 与恢复合同见 `../launcher/game-client-launcher-runtime-session-continuity.prd.md`。
- `--no-llm` 仍只可作为 observer/debug，不是 formal gameplay evidence。

### 底层 Viewer Debug 人工闭环
终端 A：
```bash
env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_viewer_live -- llm_bootstrap --allow-debug-scenario --llm --bind 127.0.0.1:5023 --web-bind 127.0.0.1:5011
```

终端 B：
```bash
env -u NO_COLOR ./scripts/run-viewer-web.sh --address 127.0.0.1 --port 4173
```

终端 C：
```bash
command -v agent-browser >/dev/null || { echo "missing agent-browser" >&2; exit 1; }
mkdir -p output/playwright/viewer
agent-browser close-all || true
agent-browser --headed open "http://127.0.0.1:4173/?ws=ws://127.0.0.1:5011&render_mode=viewer&test_api=1"
agent-browser wait --load networkidle
agent-browser snapshot -i
agent-browser eval "JSON.stringify(window.__AW_TEST__?.getState?.() ?? null)"
agent-browser console | tee output/playwright/viewer/console.log
agent-browser screenshot output/playwright/viewer/viewer-web.png
agent-browser close
```

### 推荐自动化脚本
- 主入口 contract：
```bash
./scripts/viewer-primary-web-entry-regression.sh --headed
```
- 实时玩法推进 / blocker 观测：
```bash
./scripts/viewer-software-safe-step-regression.sh --headed
```
- prompt/chat 回归：
```bash
./scripts/viewer-software-safe-chat-regression.sh --headed
```

## 最小通过标准
- 页面可加载，且 `window.__AW_TEST__` 可用。
- `getState().renderMode=viewer`（兼容 alias 场景可回出 `software_safe` 但不再是 canonical 期望）。
- `connectionStatus=connected`，或页面显式给出可追溯 blocker。
- 至少产出 1 张截图与 1 份 console/state 证据。

## 常用调试点
- `window.__AW_TEST__.getState()`
- `window.__AW_TEST__.sendControl("step")`
- `window.__AW_TEST__.sendPromptControl("preview", { agentId: "agent-0", shortTermGoal: "test" })`
- `window.__AW_TEST__.sendPromptControl("apply", { agentId: "agent-0", shortTermGoal: "test" })`
- `window.__AW_TEST__.sendAgentChat("agent-0", "hello from viewer")`

## 常见问题排查
- 页面空白：确认 `run-viewer-web.sh` 已完成构建并监听目标端口。
- 连接失败：确认 `oasis7_viewer_live` 已启动，且 `ws=` 参数与 `--web-bind` 一致。
- 无法进入正式玩法：检查 LLM provider 配置；若显式 `--no-llm`，只允许 observer/debug。
- `agent-browser` 失败：先检查 `agent-browser --version` 与浏览器依赖。
- 有状态但不推进：优先跑 `viewer-software-safe-step-regression.sh`，确认是正常推进还是显式 blocker。
- `test_api=1` 下停在 `connecting` 且 `logicalTime=0`：立即检查 `getState().lastError` / `errorCount` 并归档 state、console、screenshot。已知 WebGL/SwiftShader fatal 最多自动 reload 一次；再次失败是 S6 环境/图形 blocker，不得当作 gameplay 证据。只有确认 Web 图形阻塞后，native 才可作为诊断 fallback。
- 如果只看到 `--no-llm` 截图证据：不要把它当成 formal gameplay PASS；回到 `doc/testing/evidence/software-safe-primary-web-entry-evidence-2026-04-07.md` 看 LLM-enabled follow-up 结论。

## 已移除能力
- 退役的第二 Viewer 启动路径
- 退役的辅助可视化检视 surface
- 退役的视觉资产、抓帧与专项检视工具链

## 参考文档
- `testing-manual.md`
- `doc/testing/manual/web-ui-agent-browser-closure-manual.manual.md`
- `doc/world-simulator/viewer/viewer-web-entry-compatibility.prd.md`
- `doc/testing/evidence/software-safe-primary-web-entry-evidence-2026-04-07.md`
- `doc/world-runtime/evidence/formal-release-fixed-genesis-default-viewer-2026-05-16.md`
