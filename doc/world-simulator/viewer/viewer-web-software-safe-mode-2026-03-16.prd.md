# Viewer Web Software-Safe Mode（无 GPU 硬件依赖模式，2026-03-16）

- 对应设计文档: `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.design.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.project.md`

审计轮次: 2

## 目标
- 为 Web Viewer 新增一个不依赖 GPU 硬件能力的 `software_safe` 模式，保障 software renderer / 受限浏览器环境下的最小玩法闭环。
- 将 issue `#39` 从“环境一旦弱就整体阻断”改为“标准模式失败时可显式降级到安全模式”。
- 为后续研发冻结 mode contract、能力地板、测试口径与任务拆解。

## 范围
- 覆盖 Web 端 `standard/auto/software_safe` 三态渲染模式设计。
- 覆盖 bootstrap shell、轻量 software-safe frontend、共享 control/test API 与 `oasis7`/testing 手册口径。
- 不覆盖 native Viewer 图形重构，不覆盖高保真视觉等价实现。

## 接口 / 数据
- render mode 入口：URL query、launcher CLI、环境变量。
- 前端状态输出：`__AW_TEST__.getState().renderMode/rendererClass/softwareSafeReason`。
- 共享数据面：现有 viewer/runtime 协议，必要时新增安全模式友好的聚合 view model。
- 相关文档：`testing-manual.md`、`doc/world-simulator/viewer/viewer-manual.md`、`oasis7` real-play 工作流。

## 1. Executive Summary
- `#39` 暴露的问题说明：当前 Web Viewer 即使已经补了 fatal 透出与 SwiftShader 快失败口径，仍然把“环境不支持硬件 WebGL/WGPU”视为阻断条件，无法完成真实 `oasis7` 玩家闭环。
- 单纯继续在现有 Bevy/WGPU Web Viewer 中关闭特效、降 shader 复杂度，并不能从根上消除对图形硬件/驱动路径的依赖；在 SwiftShader / software renderer / 受限 WebGL 环境下，初始化阶段仍可能直接失败。
- 本专题新增一个 **Software-Safe Mode**：在 Web 端提供一个不依赖 WGPU / 硬件 WebGL 的安全模式前端，用于保障“连接、观测、选择目标、推进世界、查看反馈、执行基础控制”最小玩法闭环。

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
  - `standard`：现有高保真模式（默认产品态）
  - `auto`：根据环境自动在 `standard` / `software_safe` 间选路
  - `software_safe`：不依赖 GPU 硬件能力的安全模式
- `software_safe` 模式下必须保留的能力：
  - 连接状态、`tick/logicalTime/eventSeq/error` 可见
  - 基础世界观察能力：目标列表、地点/Agent 语义概览、最近事件/反馈
  - 基础交互能力：选中 1 个 Agent/地点、`play/pause/step`、查看控制反馈
  - `__AW_TEST__` /脚本采证能力：agent-browser 可以在无硬件 GPU 的浏览器环境下完成最小闭环
- `oasis7`、`run-game-test-ab.sh`、制作人/QA 手册必须能显式声明或自动落到 `software_safe`，避免再次把环境图形故障误判为玩法故障。
- source-tree 入口（如 `oasis7-run.sh play`、开发态 `oasis7_game_launcher` Web 闭环）不得悄悄消费过期 Viewer Web dist；若 `index.html` / `software_safe.*` / wasm 输入已比 `dist/` 新，必须重建或阻断并给出明确指引。

### Out of Scope
- 不要求 `software_safe` 模式保留 3D 视觉效果、PBR、后处理、粒子或完整美术表现。
- 不要求 `software_safe` 与标准 Viewer 在视觉质量上等价。
- 不要求本轮修改 native Viewer 图形栈。
- 不修改 `third_party` 或 Bevy 上游实现。

## 3. User Stories
- As a 制作人/玩家, I want `oasis7` real-play to remain usable on machines/browsers without hardware-backed WebGL, so that I can验证 Agent 与玩法闭环而不是被图形环境阻断。
- As a QA/自动化执行者, I want agent-browser 在 software renderer 环境下仍能完成最小 Web 闭环, so that 环境门禁与玩法回归可以明确分流，而不是“全黑屏=全部失败”。
- As a viewer_engineer, I want a clearly bounded software-safe surface, so that 我能控制维护成本并避免把高保真图形复杂度带进低能力环境。

## 4. Technical Specifications

### 4.1 Mode Selection Contract
- 新增 `render_mode` 选择入口：
  - URL query: `?render_mode=standard|auto|software_safe`
  - launcher / product path: `--viewer-render-mode standard|auto|software_safe`
  - 环境变量（开发/脚本可选）: `OASIS7_VIEWER_RENDER_MODE`
- `auto` 为推荐默认：
  1. 显式 query / CLI / env 优先级最高。
  2. 若未显式指定，bootstrap shell 先探测浏览器环境。
  3. 若探测到 `SwiftShader` / `llvmpipe` / software renderer / WebGL 不可用 / 已知 fatal 签名，则自动转入 `software_safe`。

### 4.2 Architecture Decision
- **不采用**“继续在同一 Bevy/WGPU Web Viewer 中只关特效”的方案作为根治手段。
- **采用**“双前端模式”方案：
  - 标准模式：沿用现有 Bevy/WGPU Web Viewer。
  - 安全模式：新增一个 **不依赖 WGPU/WebGL** 的轻量 Web frontend（DOM/SVG/Canvas2D 优先）；当前组件化实现允许使用 SolidJS，但不得改变既有 product contract。
- 原因：只有把 `software_safe` 模式从图形后端层面与 WGPU 解耦，才能真正满足“无 GPU 硬件依赖”。

### 4.3 Software-Safe Mode Capability Floor
- 必须保留：
  - 世界连接状态条
  - 顶部世界摘要：`tick/logicalTime/eventSeq/connectionStatus/provider info`
  - 目标列表 / 语义地图（2D 简化视图即可）
  - 最近事件流 / 控制反馈
  - `play/pause/step` 控制
  - 选中对象详情（Agent / Location）
  - 当页面带有 viewer auth bootstrap 时，选中 Agent 的最小 `prompt/chat` 控制面（至少覆盖 Agent Chat 发送、消息流展示，以及 prompt override 的 preview/apply/rollback）
- 可延后/不保留：
  - 3D 摄像机、2D/3D 切换
  - 粒子、氛围、光照、景深、环境图生成
  - 高级 selection halo / 复杂 label LOD
  - 依赖 GPU shader 的视觉增强

### 4.4 Shared Data Plane
- `software_safe` 模式必须复用现有 runtime / viewer 协议，而不是发明新后端协议。
- 允许新增一个“安全模式友好”的聚合快照接口，但必须由现有 viewer/runtime 状态推导，并维持 `__AW_TEST__`/自动化消费一致性。
- `__AW_TEST__.getState()` 至少新增：
  - `renderMode`
  - `rendererClass` (`hardware` / `software` / `none`)
  - `softwareSafeReason`

### 4.5 Bootstrap / Product Contract
- `oasis7_game_launcher` 的 Web 静态入口先加载一个轻量 bootstrap shell：
  - 探测浏览器 WebGL / renderer 环境
  - 决定加载标准 Viewer 还是 `software_safe` bundle
  - 在页面中显式显示当前模式与切换原因
- `oasis7` / `run-game-test-ab.sh`：
  - 在 `auto` 模式下允许落到 `software_safe`
  - 仅当 `standard` 与 `software_safe` 都不可用时，才视为产品级阻断

## 5. Acceptance Criteria
- AC-1: 在 `SwiftShader` / software renderer 环境中，Web 路径不再只剩黑屏或直接 fatal；应能进入 `software_safe` 并显示明确模式标识。
- AC-2: `software_safe` 模式下，agent-browser 可以完成最小闭环：加载页面 -> 连接世界 -> 选择目标 -> `step` 推进 -> 观察新反馈。
- AC-3: `__AW_TEST__.getState()` 能明确区分 `standard` / `software_safe`，并给出 fallback 原因。
- AC-4: `oasis7` 与 testing 手册明确说明 `software_safe` 是“玩法/验证兜底模式”，不是视觉质量签收模式。
- AC-5: 当硬件 WebGL 可用时，`auto` 不得错误降级到 `software_safe`，以免影响正常画面验收。
- AC-6: 在带 viewer auth bootstrap 的 software-safe 页面中，玩家可对选中 Agent 发起最小 `prompt/chat` 闭环：发送一条 chat，并完成一次 prompt preview/apply，且 `__AW_TEST__` 能返回对应 ack/error 反馈。
- AC-7: software-safe 的选中 Agent 控制面支持一次 prompt rollback，并能把玩家出站 chat ack 与 `agent_spoke` 事件汇成可见消息流；rollback 成功后需要刷新 prompt 版本/状态。
- AC-8: 当 runtime live 使用 `OpenClaw(Local HTTP)` 驱动 Agent 时，software-safe 页面必须显式标识自身处于 `debug_viewer` 旁路订阅层，并展示选中 Agent 的 `mode/schema/environment/fallback` 摘要；此时 prompt/chat 控制面需要明确提示 observer-only 边界。

## 6. Non-Functional Requirements
- NFR-1: `software_safe` 模式不得依赖硬件 GPU；在 software renderer / 无 WebGL / 受限 WebGL 环境下仍可启动。
- NFR-2: `software_safe` 模式首页可见状态（连接状态或错误）应在 2 秒内可观测。
- NFR-3: `software_safe` 模式必须保持 `console fatal = 0` 的目标；若失败，必须给出结构化错误而不是黑屏。
- NFR-4: `software_safe` 模式与标准模式共享同一套世界 authority / 控制语义，禁止出现“安全模式能做的控制与标准模式行为不一致”。
- NFR-5: Viewer Web freshness gate 必须覆盖 `crates/oasis7_viewer/` 根入口文件与 software-safe 构建输入（至少 `index.html`、`software_safe.html`、`software_safe.js`、`package.json`、`package-lock.json`、`vite.software-safe.config.mjs`、`software_safe_src/`）以及静态资源，避免 stale dist 重新放出 issue `#39` 的黑屏表象。

## 7. Risks & Roadmap
- 风险 1：双前端模式增加维护成本。
  - 缓解：严格限定 `software_safe` 能力地板，只做闭环必需功能；复用 shared schema / control contract。
- 风险 2：如果继续复用 Bevy/WGPU，仍可能被图形后端阻断。
  - 缓解：明确要求 `software_safe` 前端与 WGPU/WebGL 解耦。
- 风险 3：自动 fallback 可能掩盖标准模式问题。
  - 缓解：页面与 `__AW_TEST__` 必须显式标识当前模式与 fallback 原因；视觉验收仍要求 `standard`。

### Milestones
- M0: 完成 PRD / Design / Project 建模与索引回写。
- M1: 落地 bootstrap shell 与 render mode 选路。
- M2: 落地 `software_safe` MVP（连接/观察/选择/step/反馈）。
- M3: 打通 `oasis7` / `run-game-test-ab.sh` / 手册口径。
- M4: 完成 Web 闭环验证与 issue `#39` 收口判断。

## 里程碑
- M0: 完成 PRD / Design / Project 建模与索引回写。
- M1: 落地 bootstrap shell 与 render mode 选路。
- M2: 落地 `software_safe` MVP（连接/观察/选择/step/反馈）。
- M3: 打通 `oasis7` / `run-game-test-ab.sh` / 手册口径。
- M4: 完成 Web 闭环验证与 issue `#39` 收口判断。

## 风险
- 风险 1：双前端模式增加维护成本。
  - 缓解：严格限定 `software_safe` 能力地板并复用 shared schema / control contract。
- 风险 2：若继续复用 Bevy/WGPU，则仍可能被图形后端阻断。
  - 缓解：明确要求 `software_safe` 前端与 WGPU/WebGL 解耦。
- 风险 3：自动 fallback 可能掩盖标准模式问题。
  - 缓解：页面与 `__AW_TEST__` 必须显式标识当前模式与 fallback 原因。

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
  - 不改变 `render_mode=standard|auto|software_safe` 的选路语义。
  - 不改变 software-safe 的 fallback 判定与 URL 重定向逻辑。
  - 不以“仅隐藏文案”替代真实收口；必须保证 overlay 不再持续占布局。
- Acceptance Criteria:
  - AC-9: 标准模式页面在 wasm bootstrap 完成后，`Loading standard viewer...` overlay 必须自动隐藏或移除，且不再作为持续可见状态保留在页面正文中。
  - AC-10: loading overlay 可见时不得与主 canvas 并排占位；标准模式进入运行态后，Viewer 视口宽度不得再被左侧 loading 栏压缩。
  - AC-11: `test_tier_required` 至少覆盖一条 bootstrap 生命周期回归，验证标准模式启动后会执行 overlay cleanup，且不影响 software-safe fallback。

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
  - freshness gate 必须把 Solid 构建输入纳入 source scope，避免 source-tree Web 闭环错误消费旧 bundle。
- Functional Constraints:
  - 不新增新的后端协议，不改变 `software_safe` 的 capability floor。
  - 不把当前页面收口成依赖框架运行时特性的“新产品”；只允许做组件化拆分与维护性改造。
  - 产物路径继续保持 `crates/oasis7_viewer/software_safe.js`，避免 launcher / script / freshness contract 额外漂移。
- Acceptance Criteria:
  - AC-12: `software_safe` UI 组件化后，真实 Web smoke 仍能完成“加载 -> 连接 -> 选择目标 -> `step` -> 看到 control feedback”最小闭环。
  - AC-13: `__AW_TEST__.getState()`、auth/bootstrap surface、observer/debug 标识与现有 `software_safe` 页面按钮/字段 contract 不得回退。
  - AC-14: Viewer Web freshness gate 必须把 `package.json`、`package-lock.json`、`vite.software-safe.config.mjs` 与 `software_safe_src/` 作为 software-safe bundle 的正式输入。
