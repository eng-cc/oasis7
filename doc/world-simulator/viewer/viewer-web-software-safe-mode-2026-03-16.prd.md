# Viewer Web Software-Safe Mode（无 GPU 硬件依赖模式，2026-03-16）

- 对应设计文档: `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.design.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.project.md`

审计轮次: 2

## 目标
- 将 `software_safe` 从“弱图形兜底模式”升格为低保真但正式可玩的主要 Web 入口，优先承接浏览器 formal gameplay 主链路。
- 保留其“不依赖 GPU 硬件能力”的技术约束，使主要 Web 入口尽可能覆盖 software renderer / 受限浏览器环境。
- 将 issue `#39` 的原始诉求从“弱环境不黑屏”扩展为“默认 Web 主路径不再以 3D 图形能力作为正式可玩前置门槛”。

## 范围
- 覆盖 Web 端 `standard/auto/software_safe` 三态渲染模式设计，但重新定义默认用途：
  - `software_safe` = 主要正式 Web 入口
  - `standard` = opt-in visual QA / screenshot / spatial review 入口
  - `auto` = 兼容过渡态，不得再把“硬件好就默认切 3D”当产品默认
- 覆盖 bootstrap shell、轻量 software-safe frontend、共享 control/test API、正式 Web gameplay action envelope 与 `oasis7`/testing 手册口径。
- 不覆盖 native Viewer 图形重构，不覆盖高保真视觉等价实现，不默认把资产/治理/转账动作并入本专题。

## 接口 / 数据
- render mode 入口：URL query、launcher CLI、环境变量。
- 前端状态输出：`__AW_TEST__.getState().renderMode/rendererClass/softwareSafeReason`。
- 共享数据面：现有 viewer/runtime 协议，必要时新增安全模式友好的聚合 view model。
- 相关文档：`testing-manual.md`、`doc/world-simulator/viewer/viewer-manual.md`、`oasis7` real-play 工作流。

## 1. Executive Summary
- 旧问题说明：当前 Web taxonomy 虽然已经拥有 `software_safe`，但它仍被定位为“环境差时才用的 fallback”，这会让浏览器正式可玩性继续被 `standard_3d` 的硬件/图形门槛绑架。
- 单纯继续在现有 Bevy/WGPU Web Viewer 中降特效，不能把浏览器主入口从图形依赖里解耦；即使硬件良好，默认把 formal gameplay 绑定到 3D 也会放大环境波动、增加 QA 和 release claim 噪声。
- 本专题将 **Software-Safe Mode** 重写为主要正式 Web 入口：在 Web 端提供一个不依赖 WGPU / 硬件 WebGL 的低保真 frontend，用于保障“连接、观测、选择目标、推进世界、查看反馈、执行浏览器主玩法动作”的正式闭环，而 `standard_3d` 改为显式 opt-in 的视觉审查入口。

## 2. User Experience & Functionality

### In Scope
- 文件范围（目标态）：
  - `crates/oasis7/src/bin/oasis7_game_launcher.rs`
  - `crates/oasis7_viewer/**` 或新增轻量 Web viewer frontend 目录
  - `scripts/run-game-test-ab.sh`
  - `site/skills/oasis7.md`
  - `testing-manual.md`
  - `doc/world-simulator/viewer/viewer-manual.md`
- Web Viewer 新增三种 render mode：
  - `software_safe`：不依赖 GPU 硬件能力的主要正式 Web 模式
  - `standard`：现有高保真模式，但收口为显式 visual review / screenshot / spatial QA 模式
  - `auto`：兼容过渡模式；在正式产品入口上不得再默认把 3D 作为首选 claim
- `software_safe` 模式下必须保留的能力：
  - 连接状态、`tick/logicalTime/eventSeq/error` 可见
  - 基础世界观察能力：目标列表、地点/Agent 语义概览、最近事件/反馈
  - 基础交互能力：选中 1 个 Agent/地点，并通过 canonical gameplay actions 查看/推进世界语义反馈；若 runtime 已发布 `request_snapshot`、`live_control.play|step`、`gameplay_action.submit` 等动作，页面必须显式给出执行入口、禁用原因或 handoff
  - 浏览器正式玩法动作：带 auth/bootstrap 时可执行选中 Agent 的 `prompt/chat/rollback`，并显式展示 session/auth/recovery/blocking semantics
  - formal gameplay 叙事信息：至少能看到当前 `stage/goal/progress/blocker/next_step` 或等价的 canonical 玩家语义摘要
  - `__AW_TEST__` /脚本采证能力：agent-browser 可以在无硬件 GPU 的浏览器环境下完成正式 Web 主链路采证
- `oasis7`、`run-game-test-ab.sh`、制作人/QA 手册必须能显式声明 `software_safe` 是浏览器正式主路径，并避免再把环境图形故障误判为“浏览器主入口不可玩”。
- source-tree 入口（如 `oasis7-run.sh play`、开发态 `oasis7_game_launcher` Web 闭环）不得悄悄消费过期 Viewer Web dist；若 `index.html` / `software_safe.*` / wasm 输入已比 `dist/` 新，必须重建或阻断并给出明确指引。

### Out of Scope
- 不要求 `software_safe` 模式保留 3D 视觉效果、PBR、后处理、粒子或完整美术表现。
- 不要求 `software_safe` 与标准 Viewer 在视觉质量上等价。
- 不要求本轮修改 native Viewer 图形栈。
- 不要求在 `software_safe` 中立即暴露资产/治理/转账等专门动作面；这些动作若未暴露，必须明确 handoff 到其他 surface，而不是隐式缺失。
- 不修改 `third_party` 或 Bevy 上游实现。

## 3. User Stories
- As a 玩家 / 制作人, I want the default browser route to land on `software_safe`, so that I can formally play the game on the Web without first satisfying high-fidelity graphics prerequisites.
- As a QA / 自动化执行者, I want agent-browser to validate the main Web gameplay path through `software_safe`, so that formal Web PASS no longer depends on `standard_3d` graphics health.
- As a viewer_engineer, I want a clearly bounded `software_safe` action envelope, so that I can make it the primary Web surface without accidentally promising every advanced operation in the same release.

## 4. Technical Specifications

### 4.1 Mode Selection Contract
- 新增 `render_mode` 选择入口：
  - URL query: `?render_mode=standard|auto|software_safe`
  - launcher / product path: `--viewer-render-mode standard|auto|software_safe`
  - 环境变量（开发/脚本可选）: `OASIS7_VIEWER_RENDER_MODE`
- 新默认规则：
  1. 显式 query / CLI / env 优先级最高。
  2. 若未显式指定，正式浏览器产品入口默认落到 `software_safe`。
  3. `standard` 只在显式 visual intent 下进入；命中 `graphics_env` 时按 visual claim 阻断，不得偷转成视觉 PASS。
  4. `auto` 仅作兼容过渡态；在正式产品入口上，`auto` 不得再以“硬件可用则优先 3D”作为默认行为。

### 4.2 Architecture Decision
- **不采用**“继续在同一 Bevy/WGPU Web Viewer 中只关特效”的方案作为根治手段。
- **采用**“主 Web = software_safe、视觉 QA = standard_3d”的双前端方案：
  - `software_safe`：不依赖 WGPU/WebGL 的轻量 Web frontend（DOM/SVG/Canvas2D 优先），承担正式 Web 主玩法面。
  - `standard`：沿用现有 Bevy/WGPU Web Viewer，承担视觉质量验收、截图语义与空间 QA。
- 原因：只有把 formal Web 主入口从图形后端层面与 WGPU 解耦，才能真正满足“浏览器默认可玩”这一产品目标。

### 4.3 Software-Safe Mode Capability Envelope
- 作为主要正式 Web 入口，`software_safe` 必须保留：
  - 世界连接状态条
  - 顶部世界摘要：`tick/logicalTime/eventSeq/connectionStatus/provider info`
  - 目标列表 / 语义地图（2D 简化视图即可）
  - 最近事件流 / 控制反馈
  - 选中对象详情（Agent / Location）
  - hosted/public-join session acquire/release/recovery 与 auth tier 可视化
  - canonical 玩家语义摘要：`stage/goal/progress/blocker/next_step` 或等价字段
  - 当页面带有 viewer auth bootstrap 时，选中 Agent 的最小 `prompt/chat` 控制面（至少覆盖 Agent Chat 发送、消息流展示，以及 prompt override 的 preview/apply/rollback）；其中 prompt override 编辑表单允许收纳为显式 settings toggle，默认不必直接展开，但 automation API 与 ack/error 可观测性必须保持可用
  - 明确的 blocked / not_exposed / handoff 文案，告诉玩家哪些正式动作仍需转到其他 surface
- `software_safe` 不再单独维护一组脱离 runtime 的旧式回放控制面板；但当 canonical `available_actions` 已发布 `request_snapshot`、`live_control.play|step` 或 `gameplay_action.submit` 时，主入口必须显式暴露这些动作，而不能把它们降级成只读状态卡。
- 若 runtime 已发布 canonical gameplay summary，但当前快照缺少继续游玩所需的 `model.agents` 或 `model.locations`，主入口必须显式展示 `runtime_snapshot_empty_entities` blocker，并将除刷新快照之外的动作视为 blocked。
- 可延后/不保留：
  - 3D 摄像机、2D/3D 切换
  - 粒子、氛围、光照、景深、环境图生成
  - 高级 selection halo / 复杂 label LOD
  - 依赖 GPU shader 的视觉增强
  - `main_token_transfer` 等资产/治理专门动作 form

### 4.4 Shared Data Plane
- `software_safe` 模式必须复用现有 runtime / viewer 协议，而不是发明新后端协议。
- 允许新增一个“安全模式友好”的聚合快照接口，但必须由现有 viewer/runtime 状态推导，并维持 `__AW_TEST__`/自动化消费一致性。
- `__AW_TEST__.getState()` 至少新增：
  - `renderMode`
  - `rendererClass` (`hardware` / `software` / `none`)
  - `softwareSafeReason`

### 4.5 Bootstrap / Product Contract
- `oasis7_game_launcher` 的 Web 静态入口先加载一个轻量 bootstrap shell：
  - 解析主入口意图与 `render_mode`
  - 默认加载 `software_safe` bundle
  - 仅在显式要求 `standard` 或 visual review 时加载标准 Viewer
  - 在页面中显式显示当前模式、为什么在这里、以及如何切到其他 surface
- `oasis7` / `run-game-test-ab.sh`：
  - formal Web gameplay 默认使用 `software_safe`
  - 视觉 QA / 截图语义默认显式指定 `standard`
  - 仅当 `software_safe` 也不可用时，才视为浏览器主入口级阻断

## 5. Acceptance Criteria
- AC-1: 默认浏览器产品入口必须落到 `software_safe`，且页面必须显式显示自身正在承担“formal Web gameplay”角色，而不是“fallback only”。
- AC-2: `software_safe` 模式下，agent-browser 可以完成正式 Web 主链路：加载页面 -> 连接世界 -> 看到 canonical 玩家语义摘要 -> 选择目标 -> `step/play/pause` 推进 -> 观察新反馈。
- AC-3: 在带 viewer auth bootstrap 的 `software_safe` 页面中，玩家可对选中 Agent 发起最小 `prompt/chat/rollback` 闭环；prompt override 编辑表单可作为显式 settings toggle 默认收起，但 `__AW_TEST__` 与对应 ack/error 反馈必须保持可用。
- AC-4: `__AW_TEST__.getState()` 能明确区分 `standard` / `software_safe`，并给出主入口路由原因或显式 visual-mode 原因。
- AC-5: `oasis7` 与 testing/manual 口径必须把 `software_safe` 写成浏览器正式主路径，把 `standard` 写成 visual QA/screenshot 路径，而不是相反。
- AC-6: `standard` 在硬件可用时仍可独立验证高保真画面，但其 PASS 不得替代 `software_safe` 的 formal Web gameplay PASS。
- AC-7: `software_safe` 若未暴露 `main_token_transfer` 等专门动作，页面必须显式说明该动作未在此 surface 暴露，并给出 handoff 指引；不得让用户误以为这是 bug 或隐式权限失败。
- AC-8: 当 runtime live 使用 `Local Provider(Local HTTP)` 驱动 Agent 时，software-safe 页面必须显式标识自身处于 `debug_viewer` 旁路订阅层，并把 execution lane 期望 metadata 与 provider 实际 readiness check 分开展示：前者至少包含选中 Agent 的 `mode/schema/environment/fallback`，后者至少包含 `provider_check_status/source/fallback_reason/capabilities/supported_action_sets/error`；此时 prompt/chat 控制面需要明确提示 observer-only 边界。
- AC-9: canonical `available_actions` 不得只作为 ready/handoff 状态卡存在；对 `request_snapshot`、`live_control.play|step`、`gameplay_action.submit` 这类已支持动作，页面必须提供直接执行入口，并保持反馈可观察。
- AC-10: 当 gameplay summary 与空实体快照并存时，runtime 或 viewer 至少一侧必须显式把该状态标记为 `runtime_snapshot_empty_entities` blocker，且除刷新快照外不得把主玩法动作继续显示成可执行。

## 6. Non-Functional Requirements
- NFR-1: `software_safe` 模式不得依赖硬件 GPU；在 software renderer / 无 WebGL / 受限 WebGL 环境下仍可启动。
- NFR-2: `software_safe` 模式首页可见状态（连接状态或错误）应在 2 秒内可观测。
- NFR-3: `software_safe` 模式必须保持 `console fatal = 0` 的目标；若失败，必须给出结构化错误而不是黑屏。
- NFR-4: `software_safe` 作为主要正式 Web 入口，必须与 `pure_api` 共享同一套 canonical 世界 authority / 控制语义；若动作未暴露，必须显式 handoff，而不是行为漂移。
- NFR-5: Viewer Web freshness gate 必须覆盖 `crates/oasis7_viewer/` 根入口文件与 software-safe 构建输入（至少 `index.html`、`software_safe.html`、`software_safe.js`、`package.json`、`package-lock.json`、`vite.software-safe.config.mjs`、`software_safe_src/`）以及静态资源，避免 stale dist 重新放出 issue `#39` 的黑屏表象。

## 7. Risks & Roadmap
- 风险 1：双前端模式增加维护成本。
  - 缓解：严格限定 `software_safe` action envelope，把“主入口必须有”和“专门 surface 承担”明确拆开；复用 shared schema / control contract。
- 风险 2：如果继续复用 Bevy/WGPU，仍可能被图形后端阻断。
  - 缓解：明确要求 `software_safe` 前端与 WGPU/WebGL 解耦。
- 风险 3：如果只改默认入口，不补正式主链路语义，`software_safe` 会变成“名义主入口、实际还是调试页”。
  - 缓解：把 canonical 玩家语义、auth/recovery、blocked/handoff 与 formal Web action envelope 纳入本专题验收。

### Milestones
- M0: 完成 PRD / Design / Project 重写，把 `software_safe` 升格为主要正式 Web 入口。
- M1: 落地默认 Web 路由切换与 `standard` 的 visual QA 定位收口。
- M2: 补齐 `software_safe` 的 canonical 玩家语义与主玩法 action envelope。
- M3: 补齐未暴露正式动作的 handoff surface、manual/testing 入口与 release claim。
- M4: 以 `software_safe` 为主路径完成正式 Web 闭环验证与 issue `#39` 的产品级收口判断。

## 里程碑
- M0: 完成 PRD / Design / Project 重写，把 `software_safe` 升格为主要正式 Web 入口。
- M1: 落地默认 Web 路由切换与 `standard` 的 visual QA 定位收口。
- M2: 补齐 `software_safe` 的 canonical 玩家语义与主玩法 action envelope。
- M3: 补齐未暴露正式动作的 handoff surface、manual/testing 入口与 release claim。
- M4: 以 `software_safe` 为主路径完成正式 Web 闭环验证与 issue `#39` 的产品级收口判断。

## 风险
- 风险 1：双前端模式增加维护成本。
  - 缓解：严格限定 `software_safe` action envelope，把“主入口必须有”和“专门 surface 承担”明确拆开；复用 shared schema / control contract。
- 风险 2：若继续复用 Bevy/WGPU，则仍可能被图形后端阻断。
  - 缓解：明确要求 `software_safe` 前端与 WGPU/WebGL 解耦。
- 风险 3：如果只改默认入口，不补正式主链路语义，`software_safe` 会变成“名义主入口、实际还是调试页”。
  - 缓解：把 canonical 玩家语义、auth/recovery、blocked/handoff 与 formal Web action envelope 纳入本专题验收。

## 增量需求（2026-03-18）
- PRD-ID: `PRD-WORLD_SIMULATOR-039`
- Problem Statement:
  - 标准 Web Viewer 的 bootstrap shell 当前会在 `body` 中保留 `Loading standard viewer...` overlay；当标准模式 wasm 已启动并可交互时，该 overlay 仍可能继续可见并占据左侧布局，造成“世界已在运行但页面仍显示 loading”的假阻塞体验。
- Proposed Solution:
  - 为标准模式 bootstrap 增加显式 loading overlay 生命周期管理：
    - overlay 仅在标准模式 wasm 启动前短暂显示；
    - 一旦标准 Viewer 完成 bootstrap 并进入可绘制状态，overlay 必须淡出并从布局中移除；
    - overlay 呈现不得再与 canvas 并排占位，不得压缩真实 Viewer 视口。
- Functional Constraints:
  - 不改变 `render_mode=standard|auto|software_safe` 这组三态 mode id。
  - 只收口标准模式 loading overlay 生命周期；不在本子专题里重写 `software_safe` 已经确立的主入口语义。
  - 不以“仅隐藏文案”替代真实收口；必须保证 overlay 不再持续占布局。
- Acceptance Criteria:
  - AC-9: 标准模式页面在 wasm bootstrap 完成后，`Loading standard viewer...` overlay 必须自动隐藏或移除，且不再作为持续可见状态保留在页面正文中。
  - AC-10: loading overlay 可见时不得与主 canvas 并排占位；标准模式进入运行态后，Viewer 视口宽度不得再被左侧 loading 栏压缩。
  - AC-11: `test_tier_required` 至少覆盖一条 bootstrap 生命周期回归，验证标准模式启动后会执行 overlay cleanup，且不影响 `software_safe` 作为浏览器主路径时的入口语义。

## 8. Validation & Decision Record
- Traceability:
  - `PRD-WORLD_SIMULATOR-039 -> T13 / TASK-WORLD_SIMULATOR-162 -> test_tier_required`
  - `PRD-WORLD_SIMULATOR-039/040 -> T22 / TASK-WORLD_SIMULATOR-306 -> test_tier_required`

## 增量实现说明（2026-04-02）
- PRD-ID: `PRD-WORLD_SIMULATOR-039`
- Problem Statement:
  - `software_safe.js` 已从单文件 imperative UI 演进到较大体量的多面板实现；继续在单个脚本内叠加 observer/debug/auth/chat/prompt 逻辑，会持续放大维护与回归成本。
- Proposed Solution:
  - 保持 `software_safe.html`、`software_safe.js`、`render_mode`、`__AW_TEST__`、viewer auth/bootstrap 与 select 等主入口契约不变，同时把页面收口为纯实时模式，不再对外暴露回放/推进按钮与对应 gameplay summary action；
  - 将 UI 渲染层迁到 SolidJS 组件树，并把原有协议/状态/命令逻辑保留在可复用的 `legacy_core` 中；
  - freshness gate 必须把 Solid 构建输入与 finalize 脚本纳入 source scope，避免 source-tree Web 闭环错误消费旧 bundle。
- Functional Constraints:
  - 不新增新的后端协议，不改变 `software_safe` 的 capability floor。
  - 不把当前页面收口成依赖框架运行时特性的“新产品”；只允许做组件化拆分与维护性改造。
  - 产物路径继续保持 `crates/oasis7_viewer/software_safe.js`，避免 launcher / script / freshness contract 额外漂移。
- Acceptance Criteria:
  - AC-12: `software_safe` UI 组件化后，真实 Web smoke 仍能完成“加载 -> 连接 -> 选择目标 -> 观察实时事件/语义反馈”最小闭环，且页面不再暴露 `step/play/tick jump` 控件。
  - AC-13: `__AW_TEST__.getState()`、auth/bootstrap surface、observer/debug 标识与现有 `software_safe` 页面按钮/字段 contract 不得回退。
  - AC-14: Viewer Web freshness gate 必须把 `package.json`、`package-lock.json`、`vite.software-safe.config.mjs`、`scripts/` 与 `software_safe_src/` 作为 software-safe bundle 的正式输入。

## 增量需求（2026-04-07）
- PRD-ID: `PRD-WORLD_SIMULATOR-039`
- Problem Statement:
  - QA 实走 `software_safe` prompt/chat/rollback 流程后，当前页面虽然功能链路可用，但反馈语义仍不够清晰：rollback 后“当前版本 / 恢复来源 / 下一次 target”容易混淆，prompt/chat/control 主反馈被大段 raw JSON 淹没，而 `llm_init_failed` 之类配置错误会直接把底层 env 缺失暴露成首要用户文案。
- Proposed Solution:
  - 在 `software_safe` SolidJS 前端里为 prompt/chat/rollback/control 反馈补一层结构化摘要：
    - rollback 反馈明确区分当前生效版本、恢复来源版本与下一次 rollback target；
    - prompt/chat/control 默认展示可扫描的 summary/detail，raw payload 仍保留在可展开 diagnostics；

## 增量需求（2026-04-24）
- PRD-ID: `PRD-WORLD_SIMULATOR-039`
- Problem Statement:
  - `software_safe` 当前右侧 `Details` 面板在“选中对象明细 + 交互面”之后仍默认铺一整块 `Snapshot Summary` 原始 JSON；这块信息与中间 `世界摘要` 面板已有状态高度重复，却长期占据首屏右栏空间，削弱了选中对象与当前动作面的可读性。
- Proposed Solution:
  - 保持 `Details` 面板继续优先承载“当前选中对象 + 当前交互面”；
  - 把默认展开的 `Snapshot Summary` 大块 JSON 收口为紧凑世界规模信息（例如 agents/locations/promptProfiles/debugContexts）；
  - 原始快照 / hosted access / metrics 仍保留为按需展开的 diagnostics，而不是默认常驻主栏。
- Functional Constraints:
  - 不移除 `software_safe` 对 raw snapshot diagnostics 的可访问性。
  - 不把中间 `世界摘要` 的正式玩法 / lane / recent events 语义搬到右栏重复展示。
  - 不新增新的协议字段；仅调整现有前端信息层级与默认展开方式。
- Acceptance Criteria:
  - AC-27: `software_safe` 右侧 `Details` 面板默认不再展示大块 `Snapshot Summary` JSON。
  - AC-28: 当前快照规模仍需以紧凑形式可见，且不遮挡选中对象与交互面主体。
  - AC-29: 原始 snapshot/metrics/hosted access 仍可通过折叠 diagnostics 查看，供排障与 QA 采证使用。

## 增量需求（2026-04-24, 设置入口收口）
- PRD-ID: `PRD-WORLD_SIMULATOR-039`
- Problem Statement:
  - `software_safe` 当前把 `Language and Viewer Entry` 作为一整块主内容卡片放在首屏中间栏顶部，但它本质上只是设置/跳转入口，不属于当前正式玩法信息本体；继续占据主屏区域会压缩 `Formal Gameplay Summary` 的可视空间。
- Proposed Solution:
  - 将 `Language and Viewer Entry` 从主内容卡片收口为 `World Summary` 面板右上角的紧凑菜单；
  - 菜单内仅保留语言切换与 `Open standard Viewer` 跳转，不再为当前页重复提供“重新打开当前 software_safe”这类冗余主动作；
  - 保持 bilingual Viewer 入口的可发现性，但不再让它抢占 formal gameplay 首屏。
- Functional Constraints:
  - 不移除语言切换与标准 Viewer 跳转能力。
  - 不把这组设置项重新挪到新的首屏大卡片或长段说明里。
  - 不影响当前 `software_safe` 作为正式 Web 主入口的玩法与回归 contract。
- Acceptance Criteria:
  - AC-30: `software_safe` 主屏不再存在独立 `Language and Viewer Entry` 内容卡片。
  - AC-31: 语言切换与标准 Viewer 跳转必须在顶部右侧的紧凑入口中保持可达。
  - AC-32: `Formal Gameplay Summary` 成为首屏主要内容，不再被设置型入口顶在正文最前面。

## 增量需求（2026-04-28, 主入口诊断收口）
- PRD-ID: `PRD-WORLD_SIMULATOR-039`
- Problem Statement:
  - `software_safe` 虽然已经恢复 canonical 可执行动作与审批状态卡，但主入口首屏仍混排 execution lane、auth/session、hosted matrix、最近事件等诊断层信息；空实体快照时，右栏还会继续提示“先选 Agent”，导致真正的 blocker 与恢复路径没有成为首要视觉状态。
- Proposed Solution:
  - 将 execution lane / auth / session / hosted matrix / recent events 收进一个口径明确的折叠 diagnostics surface；
  - 在 `Formal Gameplay Summary` 中把 blocker 与 handoff 拆成独立 surface，避免“runtime 阻塞”与“资产治理另走 lane”混成一张卡；
  - 当 `runtime_snapshot_empty_entities` 命中时，右栏 `Details` 与交互面优先展示恢复指引，而不是继续催促选择不存在的 Agent。
- Functional Constraints:
  - 不移除 execution lane、auth/session、hosted matrix、recent events 等诊断真值，只调整其默认可见层级。
  - 不把领取/释放玩家会话这类必要 CTA 藏进深层诊断，避免玩家入口再次失去可操作性。
  - 不新增新的 runtime/viewer 协议字段；仅重排当前页面信息架构与默认表达顺序。
- Acceptance Criteria:
  - AC-33: `software_safe` 主入口默认视口不再常驻展示 execution lane、auth/session ladder、hosted action matrix 与最近事件详情；这些信息必须在折叠 diagnostics surface 内保持可达。
  - AC-34: `runtime_snapshot_empty_entities` 命中时，`Formal Gameplay Summary` 的 blocker card 必须独立于 asset/governance handoff 表达，并成为首屏主状态之一。
  - AC-35: 空实体快照且无可选 Agent 时，右栏 `Details` / `Interaction` 需要显示恢复指引与当前实体计数，而不是继续显示“先选 Agent”。

## 增量需求（2026-04-16）
- PRD-ID: `PRD-WORLD_SIMULATOR-039`
- Problem Statement:
  - 当前默认本地试玩会落到 `software_safe` 主 Web 入口，但该入口仍然缺少正式的中英语言切换；同时，仓库虽然已有带 `UiI18n` 的标准 Viewer 中英切换能力，本地试玩链路却没有把它作为一个显式可访问的 bilingual Viewer 入口暴露出来。
- Proposed Solution:
  - 为 `software_safe` 页面补齐 `zh/en` locale state、界面内语言切换与 locale query 支持；
  - 为标准 Viewer 补 `locale=zh|en` / `language=zh|en` URL 初始化；
  - 在本地试玩链路和 `software_safe` 页面内，都显式暴露“打开标准 Viewer（中文/英文）”入口，避免用户只能靠猜 query 参数进入 bilingual Viewer。
- Functional Constraints:
  - 不改变“默认 Web 主入口仍是 `software_safe`”这一产品约束；
  - 不把 `software_safe` 和标准 Viewer 混成同一条 UI 实现，仍保持主入口 vs visual/bilingual Viewer 的边界；
  - 标准 Viewer 的 locale 初始化只能是显式 query，不能偷改成按系统语言自动切换。
- Acceptance Criteria:
  - AC-15: `software_safe` 页面支持 `locale=zh|en`（或等价 `language=...`）初始化，并提供界面内切换入口。
  - AC-16: 标准 Viewer Web 入口支持 `locale=zh|en`（或等价 `language=...`）初始化，且保持原有 in-app toggle 不回退。
  - AC-17: 本地 `run-game-test.sh` / producer playtest 输出必须同时给出当前主入口 URL 与显式标准 Viewer 中文/英文 URL。
  - AC-18: `software_safe` 页面内必须提供可脚本化、可点击的“打开标准 Viewer”入口，不要求替代主入口，但必须让 bilingual Viewer 成为可发现的一等入口。
    - 对 `llm_init_failed`、rollback target 缺失、rollback noop 等已知错误给出面向 QA/operator 的产品级解释。
  - 将上述 feedback 语义 contract 以 repo-owned、无 npm 前置的 deterministic Node regression 纳入正式 required automation，避免只剩专题文档里的手动命令。
- Functional Constraints:
  - 不改变 runtime `PromptControlAck/PromptControlError/AgentChatAck/AgentChatError` 协议字段，不改 `__AW_TEST__` 现有反馈对象的核心键名。
  - 不移除 raw diagnostics；QA/开发仍需能直接读取 ack/error payload。
  - 不把 rollback 后当前版本误写成历史 target；页面必须如实表达 rollback 会生成新的保存版本。
- Acceptance Criteria:
  - AC-15: rollback ack 后，页面必须同时可读地表达当前生效 prompt 版本与被恢复的历史版本，且 rollback 输入框文案不再让用户误以为它显示的是刚恢复的版本。
  - AC-16: prompt/chat/control 主反馈区默认展示 concise summary/detail，raw JSON 仅通过折叠 diagnostics 查看。
  - AC-17: 当 `llm_init_failed` 或同类配置失败出现时，页面首要错误摘要必须说明“当前栈缺少可用 LLM 配置/能力”，而不是直接把 `missing env variable ...` 作为唯一主要文案。
  - AC-18: `software_safe` feedback summary/detail/rollback 语义 contract 必须由 repo-owned deterministic regression 覆盖，且该回归纳入 `./scripts/ci-tests.sh required`，不依赖额外 npm install。

## 增量需求（2026-04-07 / 主入口改向）
- PRD-ID: `PRD-WORLD_SIMULATOR-039`
- Problem Statement:
  - `software_safe` 当前虽然已经具备连接、反馈、prompt/chat 与 session/recovery 等基础能力，但产品定位仍停留在“弱图形 fallback”；这会导致浏览器正式可玩性仍被 `standard_3d` 的图形能力与视觉质量门槛劫持。
- Proposed Solution:
  - 将 `software_safe` 明确升格为低保真但正式可玩的主要 Web 入口；
  - 将 `standard` 收口为显式 visual review / screenshot / spatial QA surface；
  - 保留 `pure_api` 为一等公民 no-UI mode，并将 `main_token_transfer` 等专门动作继续排除在 `software_safe` 主 surface 之外，改为显式 handoff。
- Functional Constraints:
  - 不把 `software_safe` 重写成 3D viewer；低保真仍是产品定位的一部分。
  - 不把资产/治理/转账 form 默认并入 `software_safe`。
  - 不把 `pure_api` 降级为 debug-only 或内部-only。
- Acceptance Criteria:
  - AC-19: 浏览器产品默认入口必须以 `software_safe` 为主，而不是依赖 `standard` 成为 formal gameplay 前置。
  - AC-20: `software_safe` 必须能承接浏览器主玩法闭环，同时显式区分“已暴露动作”和“需 handoff 的专门动作”。
  - AC-21: `standard` 必须被文档与测试口径收口为 visual QA / screenshot mode；其 PASS 不得替代 `software_safe` formal Web PASS。
- AC-22: `pure_api` 在 mode taxonomy 与项目任务规划中继续保持一等公民定位，使用场景明确为无 UI、自动化、长稳与集成。
- AC-22A: `software_safe` 与 `pure_api` 这两个 formal 玩家 surface 必须能从同一份 `snapshot.player_gameplay` 事实源回答同一组核心问题：`stage_id`（当前阶段）、`goal_title/objective`（当前目标）、`progress_detail/progress_percent`（进度）、`blocker_kind/blocker_detail`（当前阻塞）、`next_step_hint`（下一步建议），以及 `recent_feedback`/`branch_hint`（最近关键世界变化与分支抉择提示）。`pure_api` 的 `parity_verified` 只在 active LLM access 下有效；若以 no-LLM 运行，只能作为 blocked/observer-debug 诊断样本。

## 增量需求（2026-04-08 / provider readiness truth）
- PRD-ID: `PRD-WORLD_SIMULATOR-039`
- Problem Statement:
  - `software_safe` 虽然已经展示了 lane 相关的 `mode/schema/environment/fallback` 信息，但这组字段表达的是当前 execution lane 期望 contract，不是 runtime 对实际 provider `/v1/provider/info` + health 的 readiness 真值；若不分开呈现，QA / producer 容易把“lane metadata 已 ready”误读成“provider 实际已 ready”。
- Proposed Solution:
  - 在 `software_safe` 的 Local Provider observer/debug surface 上新增独立的 actual provider check 区块，显式显示 `provider_check_status/source/fallback_reason/capabilities/supported_action_sets/error`；
  - 保留原有 lane metadata 作为“期望执行 contract”摘要，但必须以单独文案说明两者语义不同。
- Functional Constraints:
  - 不移除现有 `mode/schema/environment/fallback` 摘要；它仍是解释 execution lane 的必要信息。
  - 不把 actual provider check 伪装成新的控制权限；页面仍是 `debug_viewer` / observer-only surface。
  - 不要求 runtime live 在每一帧都同步重新握手；允许短 TTL probe cache，但 UI 必须展示数据来源。
- Acceptance Criteria:
  - AC-23: `software_safe` 页面在 Local Provider observer/debug 场景下，必须同时可读地展示“lane 期望 metadata”和“provider 实际 readiness check”，且二者标题/文案不能混淆。
  - AC-24: actual provider check 至少展示 `status` 与 `source`，在可用时展示 `fallback_reason`、`capabilities`、`supported_action_sets`，在失败时展示结构化 `error`。
  - AC-25: repo-owned contract regression 至少覆盖一条 Local Provider readiness truth 断言，验证 `compatibility_status=degraded` 时仍可单独看到 `provider_check_status=ready` 之类的语义分层，而不是把二者合并成单一字段。

## 增量需求（2026-04-28 / 主入口可玩性解阻）
- PRD-ID: `PRD-WORLD_SIMULATOR-039`
- Problem Statement:
  - 当前 `software_safe` 已是浏览器正式主入口，但页面把 canonical `available_actions` 渲染成只读状态卡，导致 runtime 明明已发布 `request_snapshot`、`live_control.play|step`、`gameplay_action.submit` 等动作，玩家仍然无法直接继续推进。
  - 同时，runtime 可能发布 `player_gameplay` 进度摘要，却返回没有 `agents/locations` 的空模型；这会让主入口表现成“有目标但无法继续”的静默死胡同。
- Proposed Solution:
  - `software_safe` 前端必须把 canonical `available_actions` 收口为“状态 + 可执行入口”的统一动作卡。
  - runtime compat snapshot 在发现 gameplay summary 与空实体模型并存时，必须主动回写显式 blocker，并把除刷新快照外的动作统一禁用；viewer 前端也要保留同类 defensive fallback，避免旧后端或中间态再次形成静默死局。
- Functional Constraints:
  - 不恢复独立的旧式播放控制面板；只允许围绕 runtime 发布的 canonical `available_actions` 暴露最小执行入口。
  - 不引入新的 gameplay 后端协议；继续复用现有 `request_snapshot` / `live_control.*` / `gameplay_action.submit` / `agent_chat` 契约。
  - 对于仍需走其他 surface 的动作，必须给出明确 handoff，而不是默默缺失。
- Acceptance Criteria:
  - AC-36: `software_safe` 页面在存在 canonical `available_actions` 时，必须为已支持动作显示按钮或等价执行控件，而不是只展示 ready/handoff 徽标。
  - AC-37: `software_safe` 页面在 runtime 发布 `live_control.play|step` 时，玩家可以直接从正式摘要区触发这些动作，并看到控制反馈或后续 gameplay feedback。
  - AC-38: `software_safe` 页面在 runtime 发布 `gameplay_action.submit` 时，玩家可以直接提交该玩法动作，并看到 ack/error 或后续快照反馈。
  - AC-39: 当 gameplay summary 与空实体快照并存时，页面必须显式显示 `runtime_snapshot_empty_entities` blocker，并阻止玩家误以为页面“只是还没点到地方”。
