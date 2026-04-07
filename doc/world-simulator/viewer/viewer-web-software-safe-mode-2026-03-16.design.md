# Viewer Web Software-Safe Mode 设计

- 对应需求文档: `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.project.md`

## 1. 设计定位
为 Web Viewer 提供一个真正不依赖 GPU 硬件能力的主要正式入口，在 `SwiftShader` / software renderer / 无 WebGL 环境下仍能保持“可连接、可观测、可控制、可采证、可正式游玩”的浏览器主链路；`standard_3d` 则转为显式视觉/截图/QA 入口。

## 2. 核心设计决策
- **保留**现有 Bevy/WGPU Viewer 作为 `standard` 高保真路径。
- **将** `software_safe` 轻量 Web frontend 升格为主要正式 Web 路径，技术上与 WGPU/WebGL 解耦。
- **新增/重写** bootstrap shell 负责主入口意图、模式探测与选路；不要再让重型 wasm viewer 冒充默认正式入口。

## 3. 设计结构

### 3.1 Bootstrap Shell
职责：
- 解析 `render_mode`（query / CLI / env）
- 探测浏览器环境：WebGL 可用性、renderer 信息、已知 software renderer 标记
- 区分“formal Web gameplay 默认入口”和“显式 visual review”两类意图
- 决定加载：
  - 默认 `software_safe` 资源入口
  - 显式 visual intent 下的 `standard` 资源入口
- 在页面级显式展示：当前模式、为什么在这里、如何切到 visual review 或其他 surface

### 3.2 Standard Viewer
- 沿用现有 `oasis7_viewer` wasm 路径。
- 继续承担视觉质量验收、3D 视角、既有艺术表现与高保真交互。
- 不再默认承担 formal Web gameplay 主路径；主要用于 visual QA、空间语义和截图采证。

### 3.3 Software-Safe Frontend
技术方向：
- 优先采用 DOM/SVG/Canvas2D 组合，而不是继续依赖 WGPU/WebGL。
- UI 目标是“世界主玩法控制台/语义地图”，而不是“美术展示”。

建议模块：
- `status_bar`：连接状态、tick、eventSeq、provider info、render mode
- `semantic_map`：简化 2D 语义图（点/标签/区域）
- `entity_list`：Agent / Location 列表与过滤
- `detail_panel`：选中对象详情
- `control_panel`：play/pause/step
- `gameplay_summary`：`stage/goal/progress/blocker/next_step` 或等价 canonical 玩家语义
- `auth_session_panel`：session ladder、hosted recovery、auth/binding/rebind 状态
- `handoff_surface`：未暴露正式动作的显式指引
- `event_feed`：最近事件/控制反馈

### 3.4 Shared Data / Control Adapter
- 复用当前 viewer/runtime 协议。
- 如有必要，在前端增加一层 `safe_mode_view_model` 聚合层，把现有状态整理成安全模式 UI 更容易消费的数据结构。
- `__AW_TEST__` 保持统一入口，确保脚本不因模式切换而完全重写。
- formal gameplay 语义与 `pure_api` 的 canonical 字段必须可追溯到同一 authority source；若 `software_safe` 未暴露某动作，必须回到 handoff/hint，而不是自造一套弱化协议。

## 4. 为什么不是“继续在 Bevy/WGPU 里降特效”
- 降 deferred / 后处理可以降低 shader 复杂度，但不能保证避开 WGPU/WebGL 初始化失败。
- `#39` 证明问题发生在更底层的 renderer / pipeline 建立阶段；若底层仍绑在 software WebGL 不稳定路径，就无法满足“无 GPU 硬件依赖”。
- 因此，`software_safe` 必须在技术栈层面与标准 Viewer 解耦，而不是仅靠运行时配置降级。

## 5. 模式切换策略
- 默认产品 Web 入口：优先进入 `software_safe`。
- `render_mode=standard`：显式尝试标准模式；失败则报 `graphics_env` 级错误，不自动隐藏成 visual PASS。
- `render_mode=software_safe`：始终走主 Web 入口模式，用于正式浏览器玩法、CI、agent-browser 和弱机。
- `render_mode=auto`：
  1. 启动 bootstrap
  2. 解析产品入口意图
  3. formal Web gameplay 意图默认仍走 `software_safe`
  4. 仅在显式 visual intent 下才优先尝试 `standard`

## 6. 与现有专题关系
- 与 `viewer-web-runtime-fatal-surfacing-2026-03-12` 的关系：
  - 该专题负责“错误透明度与快失败”
  - 本专题负责“把不依赖 GPU 的 `software_safe` 收口为正式 Web 主入口，而不是在弱环境下直接失败”
- 与 `viewer-webgl-deferred-compat-2026-02-24` 的关系：
  - 该专题负责标准模式下减少部分 WebGL 兼容问题
  - 本专题不再试图把浏览器主路径绑定到标准模式，而是引入独立的主 Web 前端

## 7. 演进计划
- Phase 1：主入口重写，默认 Web 路由切到 `software_safe`
- Phase 2：补齐 formal Web gameplay 必需的 summary/auth/recovery/handoff 能力
- Phase 3：`oasis7` / testing / manual 对齐主入口与 visual QA 术语
- Phase 4：根据使用数据决定是否补更多 `software_safe` 正式动作，而不是追求 3D 视觉等价

## 8. 标准模式 Loading Overlay 生命周期（2026-03-18）
- overlay 继续由静态 `index.html` 提供，但职责仅限于标准模式 wasm 尚未启动前的短暂引导。
- overlay 层必须改为独立覆盖层：
  - 不能再依赖 `body` flex 排版与 canvas 并排；
  - 默认覆盖在页面中心，可淡出，但不可继续吃掉标准 Viewer 的宽度。
- bootstrap shell 负责注册一次性 cleanup：
  - 优先监听 `TrunkApplicationStarted`；
  - 若事件先于 canvas 插入，则用轻量轮询或 `requestAnimationFrame` 等待标准 canvas 出现；
  - cleanup 触发后，将 overlay 标记为 hidden，并在过渡结束后从 DOM 移除。
- software-safe 路径不复用该 cleanup：
  - `render_mode=software_safe` 或仍处兼容过渡态的 `auto` 路径，继续由 `software_safe` 页面负责自身初始态；
  - 本次只收口标准模式 overlay 残留，不改变 software-safe 的引导页面。
- 回归重点：
  - 标准模式启动后 overlay 会被 cleanup；
  - cleanup 不依赖连接态 `connected`，避免“Viewer 已可交互但 runtime 尚未连上时仍一直显示 loading”；
  - cleanup 后 `body` 不再保留 loading 文案的持续可见节点。
