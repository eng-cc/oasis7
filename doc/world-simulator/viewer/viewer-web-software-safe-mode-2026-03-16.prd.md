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
  - `.agents/skills/oasis7/scripts/oasis7-run.sh`
  - `testing-manual.md`
  - `doc/world-simulator/viewer/viewer-manual.md`
- Web Viewer 新增三种 render mode：
  - `software_safe`：不依赖 GPU 硬件能力的主要正式 Web 模式
  - `standard`：现有高保真模式，但收口为显式 visual review / screenshot / spatial QA 模式
  - `auto`：兼容过渡模式；在正式产品入口上不得再默认把 3D 作为首选 claim
- `software_safe` 模式下必须保留的能力：
  - 连接状态、`tick/logicalTime/eventSeq/error` 可见
  - 基础世界观察能力：目标列表、地点/Agent 语义概览、最近事件/反馈
  - 基础交互能力：选中 1 个 Agent/地点、`play/pause/step`、查看控制反馈
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
  - `play/pause/step` 控制
  - 选中对象详情（Agent / Location）
  - hosted/public-join session acquire/release/recovery 与 auth tier 可视化
  - canonical 玩家语义摘要：`stage/goal/progress/blocker/next_step` 或等价字段
  - 当页面带有 viewer auth bootstrap 时，选中 Agent 的最小 `prompt/chat` 控制面（至少覆盖 Agent Chat 发送、消息流展示，以及 prompt override 的 preview/apply/rollback）
  - 明确的 blocked / not_exposed / handoff 文案，告诉玩家哪些正式动作仍需转到其他 surface
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
- AC-3: 在带 viewer auth bootstrap 的 `software_safe` 页面中，玩家可对选中 Agent 发起最小 `prompt/chat/rollback` 闭环，且 `__AW_TEST__` 能返回对应 ack/error 反馈。
- AC-4: `__AW_TEST__.getState()` 能明确区分 `standard` / `software_safe`，并给出主入口路由原因或显式 visual-mode 原因。
- AC-5: `oasis7` 与 testing/manual 口径必须把 `software_safe` 写成浏览器正式主路径，把 `standard` 写成 visual QA/screenshot 路径，而不是相反。
- AC-6: `standard` 在硬件可用时仍可独立验证高保真画面，但其 PASS 不得替代 `software_safe` 的 formal Web gameplay PASS。
- AC-7: `software_safe` 若未暴露 `main_token_transfer` 等专门动作，页面必须显式说明该动作未在此 surface 暴露，并给出 handoff 指引；不得让用户误以为这是 bug 或隐式权限失败。
- AC-8: 当 runtime live 使用 `OpenClaw(Local HTTP)` 驱动 Agent 时，software-safe 页面必须显式标识自身处于 `debug_viewer` 旁路订阅层，并展示选中 Agent 的 `mode/schema/environment/fallback` 摘要；此时 prompt/chat 控制面需要明确提示 observer-only 边界。

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

## 增量实现说明（2026-04-02）
- PRD-ID: `PRD-WORLD_SIMULATOR-039`
- Problem Statement:
  - `software_safe.js` 已从单文件 imperative UI 演进到较大体量的多面板实现；继续在单个脚本内叠加 observer/debug/auth/chat/prompt 逻辑，会持续放大维护与回归成本。
- Proposed Solution:
  - 保持 `software_safe.html`、`software_safe.js`、`render_mode`、`__AW_TEST__`、viewer auth/bootstrap 与 play/pause/step/select 等对外契约不变；
  - 将 UI 渲染层迁到 SolidJS 组件树，并把原有协议/状态/命令逻辑保留在可复用的 `legacy_core` 中；
  - freshness gate 必须把 Solid 构建输入与 finalize 脚本纳入 source scope，避免 source-tree Web 闭环错误消费旧 bundle。
- Functional Constraints:
  - 不新增新的后端协议，不改变 `software_safe` 的 capability floor。
  - 不把当前页面收口成依赖框架运行时特性的“新产品”；只允许做组件化拆分与维护性改造。
  - 产物路径继续保持 `crates/oasis7_viewer/software_safe.js`，避免 launcher / script / freshness contract 额外漂移。
- Acceptance Criteria:
  - AC-12: `software_safe` UI 组件化后，真实 Web smoke 仍能完成“加载 -> 连接 -> 选择目标 -> `step` -> 看到 control feedback”最小闭环。
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
