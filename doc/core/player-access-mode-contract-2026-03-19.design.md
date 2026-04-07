# oasis7：玩家访问模式总契约设计（2026-03-19）

- 对应需求文档: `doc/core/player-access-mode-contract-2026-03-19.prd.md`
- 对应项目管理文档: `doc/core/player-access-mode-contract-2026-03-19.project.md`

审计轮次: 7

## 1. 设计定位
将 `standard_3d`、`software_safe`、`pure_api` 三种玩家访问模式提升为 `core` 级 taxonomy，并把 `agent_direct_connect/openclaw_local_http` 一起降为兼容 alias、把当前 operator-facing provider 模型收口为 `agent_decision_source + agent_provider_backend/contract/transport/url/auth/connect_timeout_ms/profile`、再把 `player_parity / headless_agent / debug_viewer` 降维到 execution lane，避免项目继续把“玩家入口”“兼容接入名”“正式配置模型”“观战旁路”混成一层概念。

## 2. 核心设计决策
- 保留三种玩家访问模式：
  - `standard_3d`：高保真主画面、视觉验收、截图语义与交互体验主口径。
  - `software_safe`：弱图形/无 GPU 下的 Web 最小玩法闭环与观测兜底。
  - `pure_api`：无 UI、无浏览器、formal gameplay 默认要求 active LLM access 的正式玩家入口。
- 将 `non-3D` / `2D 优先` 统一视为交付优先级或交互范围描述：
  - 它描述的是当前阶段先把资源投向哪些链路；
  - 它可以覆盖 `software_safe`、launcher/runtime interaction，必要时也可涵盖 `pure_api` 相关闭环；
  - 它不能回答“玩家现在走的是哪种产品入口”。
- 将 `agent_direct_connect/openclaw_local_http` 统一视为兼容迁移 alias：
  - 它们描述的是旧 UI/CLI/operator 曾如何称呼当前 OpenClaw 直连路径；
  - 当前正式 operator-facing provider 模型必须写成 `agent_decision_source + agent_provider_backend/contract/transport/url/auth/connect_timeout_ms/profile`；
  - 它们不能回答“玩家现在走的是哪种产品入口”，也不能继续充当唯一配置模型。
- 将 `player_parity / headless_agent / debug_viewer` 统一视为 execution lane：
  - 它们描述的是 Agent 如何执行、如何观察、是否只读；
  - 它们不能回答“玩家现在走的是哪种产品入口”。
- 采用 claim-first 设计：
  - 每个模式先冻结“允许宣称项 / 禁止宣称项”；
  - 所有 evidence、testing、playability 结论只能在 claim envelope 内输出。

## 3. 设计结构

### 3.1 Mode Registry Layer
- 在 `core` 维护唯一注册表：
  - `mode_id`
  - `surface_type`
  - `default_use_case`
  - `fallback_target`
  - `allowed_claims`
  - `forbidden_claims`
- 下游专题只负责实现和定向验收，不再各自发明同层 mode 名称。

### 3.2 Routing Layer
- 先按用户目标路由：
  - 视觉与主产品体验 -> `standard_3d`
  - 弱图形 Web 可玩性 -> `software_safe`
  - 无 UI / 自动化 / CLI 长玩 -> `pure_api`
- 若文档只是在说明阶段优先级：
  - 可以写 `non-3D` / `2D 优先`
  - 但必须补一句“这不是 mode_id，只是当前 delivery priority / interaction scope”
- 再按环境做 fallback：
  - `standard_3d` 在 `auto` 下可显式降到 `software_safe`
  - `software_safe` 不负责替代 `pure_api`
  - `pure_api` 不受浏览器/GPU 问题阻断

### 3.3 Evidence Layer
- 所有证据包必须挂一个主 `mode_id`。
- `compat_access_alias` 与结构化 `agent_provider_*` 维度作为附加记录，不允许提升为主模式。
- `execution_lane` 作为附加维度记录，不允许提升为主模式。
- 同一结论若同时涉及视觉与 no-UI 持续游玩，必须拆成两个 claim。

### 3.4 Terminology Compatibility Layer
- 兼容迁移表：
  - 旧“OpenClaw 模式” -> 新“兼容 alias `agent_direct_connect/openclaw_local_http` + 正式 provider 维度 `agent_decision_source + agent_provider_*` + execution lane”
  - 旧“Agent Provider Mode=openclaw_local_http” -> 新“配置/CLI/env 以 `agent_decision_source + agent_provider_backend/contract/transport/url/auth/connect_timeout_ms/profile` 为主；`agent_provider_mode` 仅保留兼容解析”
  - 旧“OpenClaw player mode” -> 新“玩家访问模式仍是 `standard_3d / software_safe / pure_api`，OpenClaw 相关字段只能作为附加维度”
  - 旧“non-3D 模式 / 2D 入口” -> 新“当前 delivery priority 或 interaction scope；若要表达真实玩家入口，必须回到 `standard_3d / software_safe / pure_api`”

### 3.5 Downstream Ownership
- `world-simulator/viewer/*`：
  - 负责 `standard_3d` / `software_safe` 实现与定向验收。
- `game/*`：
  - 负责 `pure_api` 的 canonical 玩家语义、动作面与 parity。
- `world-simulator/llm/*`：
  - 负责 OpenClaw provider-backed 路径、execution lane 与 provider contract；兼容 alias 只保留迁移说明。
- `testing-manual.md`：
  - 负责把脚本、证据与放行结论绑定到正确模式。

## 4. 关键约束
- `software_safe` 不能代签 `standard_3d` 的视觉放行。
- `pure_api` 不能代签截图、画面、headed Web 语义。
- `headless_agent` 不能代签 `pure_api` 玩家入口等价。
- `debug_viewer` 只回答观战/解释，不回答主执行或玩家入口。

## 5. 失败与降级语义
- `standard_3d` 命中 software renderer：
  - 若显式 `render_mode=standard` -> `blocked_by=graphics_env`
  - 若 `render_mode=auto` -> `degraded_to=software_safe`
- `software_safe` 命中 OpenClaw observer-only：
  - 结论是“弱图形观战链路可用”，不是“Viewer 是主执行依赖”
- `pure_api` 缺 canonical 玩家语义：
  - 降级为 `observer_only`
- 证据跨模式借用：
  - 自动收缩到更窄 claim；若无法拆清，直接阻断发布口径

## 6. 演进计划
- Phase 1：冻结 mode/provider/lane 三层 taxonomy，并把旧单字段 provider mode 降为兼容 alias。
- Phase 2：同步 core 主入口、README、索引与今日 devlog。
- Phase 3：后续新专题必须按本设计引用模式，不得新增同层别名。
