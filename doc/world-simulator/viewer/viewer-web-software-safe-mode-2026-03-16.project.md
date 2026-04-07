# viewer-web-software-safe-mode-2026-03-16 项目管理

- 对应设计文档: `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.design.md`
- 对应需求文档: `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md`

审计轮次: 2
## 任务拆解（含 PRD-ID 映射）
- [x] T0 (PRD-WORLD_SIMULATOR-039) [test_tier_required]: 完成“Viewer Web Software-Safe Mode”PRD / Design / Project 建模，并回写模块主文档、索引与 devlog。
- [x] T1 (PRD-WORLD_SIMULATOR-039) [test_tier_required]: 为 `oasis7_game_launcher` / Web 静态入口增加 bootstrap shell 与 `render_mode=standard|auto|software_safe` 选路契约。
- [x] T2 (PRD-WORLD_SIMULATOR-039) [test_tier_required]: 落地 `software_safe` MVP 前端，覆盖连接状态、目标列表、对象详情、`play/pause/step` 与最近事件/反馈。
- [x] T3 (PRD-WORLD_SIMULATOR-039) [test_tier_required]: 为 `__AW_TEST__` / 自动化脚本补齐 `renderMode`、`rendererClass`、`softwareSafeReason` 等模式可观测字段。
- [x] T4 (PRD-WORLD_SIMULATOR-039) [test_tier_required]: 打通 `oasis7`、`run-game-test-ab.sh`、`testing-manual.md`、`viewer-manual.md` 的 software-safe 执行口径。
- [x] T5 (PRD-WORLD_SIMULATOR-039) [test_tier_required]: 在 software renderer / SwiftShader 环境复验“加载 -> 选择目标 -> step -> 新反馈”最小闭环，并据此判断 `#39` 是否收口。
- [x] T6 (PRD-WORLD_SIMULATOR-039) [test_tier_required]: 为 `software_safe` 补选中 Agent 的 `prompt/chat` MVP（含 auth bootstrap 签名、ack/error 可观测性与自动化接口），并复验一次真实交互。
- [x] T7 (PRD-WORLD_SIMULATOR-039) [test_tier_required]: 为 `software_safe` 补 prompt rollback 与 chat history/message flow，确保 rollback 后能刷新 prompt 状态，且玩家出站消息与事件侧消息都能汇入统一消息流并被脚本读取。
- [x] T8 (PRD-WORLD_SIMULATOR-039) [test_tier_required]: 由 `qa_engineer` 为 `software_safe` 补 prompt/chat/rollback/message-flow 回归方案与专用 agent-browser 脚本，沉淀 `agent_spoke` 缺失的失败签名。
- [x] T9 (PRD-WORLD_SIMULATOR-039) [test_tier_required]: 由 `runtime_engineer` / `viewer_engineer` 补齐 `agent_chat -> AgentSpoke` 的测试态稳定触发链路，并修正 software-safe 对 runtime 事件形状的兼容解析。
- [x] T10 (PRD-WORLD_SIMULATOR-039) [test_tier_required]: 为 source-tree Viewer Web 入口补齐 dist freshness gate，确保 `oasis7-run.sh play` / Web 闭环在 `index.html`、`software_safe.*` 或静态资源漂移时优先重建 fresh dist，而不是继续消费 stale `crates/oasis7_viewer/dist`。
- [x] T11 (PRD-WORLD_SIMULATOR-039/040) [test_tier_required]: 重构 `oasis7` operator 口径，明确 `headless_agent` 是 Local Provider 主执行/回归 lane，Viewer 仅承担 `player_parity` / `debug_viewer` / `software_safe` 的体验、观战与弱图形观察职责，并写清当前 Local Provider real-play 下 `prompt/chat` 的 observer-only 边界。
- [x] T12 (PRD-WORLD_SIMULATOR-039/040) [test_tier_required]: 将 `oasis7` 主入口中的 UI/observer 细节拆到独立 reference，保持主 skill 聚焦执行闭环，仅保留最小 UI 结论与跳转关系。
- [x] T13 (PRD-WORLD_SIMULATOR-039) [test_tier_required]: 修复标准模式 bootstrap loading overlay 在 wasm 已启动后仍残留并压缩左侧视口的问题，补齐 cleanup 生命周期与最小回归验证。
- [x] T14 (PRD-WORLD_SIMULATOR-039) [test_tier_required]: 将 `software_safe` UI 渲染层迁到 SolidJS 组件架构，保留既有 `software_safe.js` 产物/协议契约，并把 freshness gate 扩到 Solid 构建输入。
- [x] T15 (PRD-WORLD_SIMULATOR-039) [test_tier_required]: 将 `software_safe` SolidJS 构建改为“临时 bundle 输出 + finalize 回写 `software_safe.js`”正式流程，消除当前 Vite `outDir` 警告，并把 finalize 脚本纳入 freshness source scope。
- [x] T16 (PRD-WORLD_SIMULATOR-039) [test_tier_required]: 新增 repo-owned `software_safe` 最小浏览器回归脚本，稳定验证“加载 -> 连接 -> 选择目标 -> `step` -> DOM/`lastControlFeedback` 反馈”闭环，并把入口写回 testing/manual。
- [x] T17 (PRD-WORLD_SIMULATOR-039) [test_tier_required]: 收口 `software_safe` prompt/chat/rollback/control 反馈可读性，明确 rollback 版本语义、将 raw diagnostics 折叠展示，并为 `llm_init_failed` 等配置失败补产品级摘要与 contract test；随后把该 contract test 纳入 repo-owned required automation，固定 canonical entry 为 `node crates/oasis7_viewer/scripts/software-safe-feedback-contract.test.mjs`。
- [x] T18 (PRD-WORLD_SIMULATOR-039) [test_tier_required]: 按最新产品设定重写本专题 PRD / design / project，把 `software_safe` 从弱图形 fallback 重定位为低保真但正式可玩的主要 Web 入口，并把 `standard` 收口为 visual QA surface。
- [x] T19 (PRD-WORLD_SIMULATOR-039) [test_tier_required]: 由 `viewer_engineer` / `runtime_engineer` 将默认 Web 产品入口切到 `software_safe`，并把 `render_mode` / launcher / bootstrap 的默认语义改成“formal Web 主入口优先 software_safe，standard 仅在显式 visual intent 下进入”。
- [x] T20 (PRD-WORLD_SIMULATOR-039) [test_tier_required]: 由 `viewer_engineer` / `runtime_engineer` 补齐 `software_safe` 作为主要正式 Web 入口所需的 canonical 玩家语义摘要、blocked/handoff surface，以及缺失动作的显式交接口径。
- [ ] T21 (PRD-WORLD_SIMULATOR-039) [test_tier_required]: 由 `qa_engineer` / `producer_system_designer` 按新的主入口 contract 重跑 formal Web vs visual QA 证据，拆分 `software_safe` formal gameplay PASS 与 `standard` visual PASS 两类结论。

## 依赖
- `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md`
- `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.design.md`
- `doc/world-simulator/viewer/viewer-web-runtime-fatal-surfacing-2026-03-12.prd.md`
- `testing-manual.md`
- `oasis7` / `run-game-test-ab` 现有脚本与 Web 闭环证据路径

## 状态
- 当前阶段：T0~T10 已完成，software-safe 已具备 prompt/chat、rollback、消息流、QA 回归、稳定 `agent_spoke` 触发闭环，以及 source-tree Viewer dist freshness gate。
- 当前阶段：T0~T11 已完成，software-safe 已具备 prompt/chat、rollback、消息流、QA 回归、稳定 `agent_spoke` 触发闭环，以及面向 `oasis7` 的 observer/debug/operator 口径收口。
- 当前阶段：T0~T12 已完成，`oasis7` 主入口已回到执行主链路，UI/observer 细节改由独立 reference 承接。
- 当前阶段：T0~T13 已完成，标准模式 bootstrap loading overlay 已改为一次性覆盖层并在 wasm Viewer 启动后自动 cleanup，不再持续压缩左侧视口。
- 当前阶段：T0~T14 已完成，`software_safe` 已迁到 SolidJS 组件化渲染层，同时保留既有 product contract，并把 source-tree freshness gate 扩到框架构建输入。
- 当前阶段：T0~T15 已完成，`software_safe` 构建链已收口为无 warning 的临时 bundle + finalize 流程，最终产物路径仍保持 `software_safe.js`，且 freshness gate 覆盖到了构建 finalize 脚本。
- 当前阶段：T0~T16 已完成，repo 已具备 `software_safe` 最小 step browser regression，能重复验证连接、选中目标、`step` 推进与 DOM/`lastControlFeedback` 反馈。
- 当前阶段：T0~T17 已完成，`software_safe` feedback UX 已收口为 summary/detail + diagnostics，且对应 deterministic contract regression 已进入 repo-owned required automation。
- 当前阶段：T18~T20 已完成，默认 Web 入口、launcher URL 与 `software_safe` canonical gameplay summary / blocked-handoff surface 已落地；剩余 `T21` 负责按新 contract 重跑 formal gameplay vs visual QA 证据。
- 联动状态：已承接 `PRD-WORLD_SIMULATOR-040 T3`，在 software-safe 页面补齐 `debug_viewer` 旁路订阅标识、选中 Agent 的 headless lane 元数据展示，以及 provider runtime live 下的 observer-only 提示。
- 最近更新：2026-04-07（`viewer_engineer` 已将默认 Web / launcher / bootstrap 改为以 `software_safe` 为正式主入口，并在页面顶层补齐 canonical gameplay summary 与显式 handoff surface；下一步只剩 `qa_engineer` / `producer_system_designer` 重跑新 contract 证据。）
- 阻塞项：`T21` formal gameplay vs visual QA 证据尚未重跑，README / current-entry / release claim 仍不得先于证据把该改向表述成“已完成上线行为”。

## 备注
- 本专题的目标不是让 `software_safe` 与标准模式“视觉等价”，而是让它成为低保真但正式可玩的主要 Web 入口。
- `standard` 仍然是视觉与交互质量签收口径；`software_safe` 现在承接正式 Web 主玩法口径。
- `pure_api` 继续是一等公民，但主要承担无 UI、自动化、长稳与集成场景，不被本专题替代。
