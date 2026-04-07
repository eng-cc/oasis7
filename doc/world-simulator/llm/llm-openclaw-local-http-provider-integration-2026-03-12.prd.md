# Agent 直连接入的 OpenClaw 本地 HTTP Provider 首期方案（2026-03-12）

- 对应设计文档: `doc/world-simulator/llm/llm-openclaw-local-http-provider-integration-2026-03-12.design.md`
- 对应项目管理文档: `doc/world-simulator/llm/llm-openclaw-local-http-provider-integration-2026-03-12.project.md`

审计轮次: 1

## 1. Executive Summary
- Problem Statement: `Decision Provider` 标准层已经明确了“外部 provider 可参与 Agent 决策，但不得替代 runtime 权威”的边界；但若要回答“安装在用户机器上的 `OpenClaw` 怎么玩这个游戏”，还缺一份面向真实用户安装场景的接入方案，尤其是本地发现、握手、配置、玩家-agent 绑定、决策接口、失败恢复与最小可玩范围。
- Proposed Solution: 首期采用“`OpenClaw` 本地进程 + `localhost HTTP/JSON`”方案。`OpenClaw` 在用户机器上以本地服务形式运行，仅监听 `127.0.0.1`；world-simulator 侧通过 `OpenClawAdapter` 调用其本地 HTTP API，发送结构化 `DecisionRequest`，接收结构化 `DecisionResponse`。运行时仍由本地 runtime/kernel 权威执行动作、校验规则并产出 trace。Launcher / Viewer 仅负责配置、发现与可观测性展示。 在真实用户机缺少原生 world-simulator provider 时，允许通过 `oasis7_openclaw_local_bridge` 这一 loopback-only 兼容桥，把已安装的 `OpenClaw Gateway/CLI` 转译成 `/v1/provider/info`、`/v1/provider/health`、`/v1/world-simulator/decision`、`/v1/world-simulator/feedback` 四个端点，作为 `experimental` 的首期可跑实现。
- Success Criteria:
  - SC-1: 用户在本机安装并启动 `OpenClaw` 后，可在 launcher 中发现并选择 `OpenClaw(Local HTTP)` 作为 agent provider。
  - SC-2: 首期 `test_tier_required` 依赖 `localhost HTTP/JSON` 完成单一低频 NPC 的 `wait` / `wait_ticks` / `move_agent` / `speak_to_nearby` / `inspect_target` / `simple_interact` 决策闭环；其中后三者先以 lightweight event 语义落地，并继续受 parity 门禁约束。
  - SC-3: 若本机未安装、未启动、版本不兼容或握手失败，launcher 必须提供明确诊断与回退路径，不得阻断内置 provider 使用。
  - SC-4: 所有 `OpenClaw` 输出必须经 action schema 白名单和 runtime 校验后才能执行；非法输出一律映射为 `Wait` 或 `ActionRejected`。
  - SC-5: `OpenClaw` 决策过程可映射到 `AgentDecisionTrace`，在 viewer / QA 调试面中可见 provider 名称、延迟、错误与最近一次结构化决策。
  - SC-6: 首期 required 验证不依赖真实 `OpenClaw` 网络环境，必须可由 mock local HTTP server 覆盖。

## 2. User Experience & Functionality
- User Personas:
  - 玩家 / 制作人：希望在自己电脑上装好 `OpenClaw` 后，能通过 provider-backed OpenClaw 路径让游戏里的部分 agent 由它驱动，并知道当前是否正常连接。
  - `agent_engineer`：需要稳定的本地传输协议与 adapter，避免把 provider 细节泄露进模拟内核。
  - `viewer_engineer`：需要在 launcher / viewer 中展示 provider 发现、连接、版本、延迟与错误状态。
  - `qa_engineer`：需要使用 mock 本地 HTTP 服务验证协议与失败签名。
- User Scenarios & Frequency:
  - 首次配置：用户首次安装 `OpenClaw` 后，在 launcher 中选择本地 provider 并完成一次 agent 绑定。
  - 日常试玩：用户启动 launcher / game 后，默认自动探测本机 `OpenClaw` 是否在线。
  - 故障恢复：本机服务未启动、token 不匹配、版本过旧时，用户根据 launcher 提示修复后重试。
- User Stories:
  - PRD-WORLD_SIMULATOR-037: As a 玩家 / 制作人, I want a provider-backed OpenClaw path whose current contract tuple is `agent_decision_source=provider_backed + agent_provider_backend=openclaw + agent_provider_contract=worldsim_provider_v1 + agent_provider_transport=loopback_http`, so that I can try external agent-driven gameplay through localhost without deploying remote services or weakening runtime authority.
- Critical User Flows:
  1. Flow-OC-LOCAL-001（首次安装与发现）:
     `用户安装并启动 OpenClaw 本地服务 -> launcher 探测 localhost provider -> 显示版本/状态 -> 用户选择 provider-backed OpenClaw（compat alias 可显示为 agent_direct_connect/OpenClaw(Local HTTP)）`。
  2. Flow-OC-LOCAL-002（玩家绑定与启动）:
     `选择 provider -> 绑定 player_id / agent_id 或 NPC profile -> 启动游戏 -> runtime 为目标 agent 使用 OpenClawAdapter`。
  3. Flow-OC-LOCAL-003（决策闭环）:
     `ObservationEnvelope -> POST /v1/world-simulator/decision -> DecisionResponse -> runtime validate/execute -> feedback/trace`。
  4. Flow-OC-LOCAL-004（失败恢复）:
     `provider offline / version mismatch / timeout / invalid action -> launcher/viewer 告警 -> fallback 内置 provider 或禁用 OpenClaw provider`。
  5. Flow-OC-LOCAL-005（用户可观测）:
     `viewer 右侧调试面显示 provider=OpenClaw(Local HTTP)、连接状态、最近延迟、最后错误、最近动作摘要`。
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| Provider 发现 | `provider_id/version/capabilities/health` | launcher 自动探测或手动刷新本机 provider | `offline -> discovered -> ready` | 仅探测 allowlist 端口/路径 | 仅本机回环地址 |
| Provider 选择 | `agent_decision_source=provider_backed`、`agent_provider_backend=openclaw`、`agent_provider_contract=worldsim_provider_v1`、`agent_provider_transport=loopback_http`、`compat_aliases=[agent_direct_connect,openclaw_local_http]` | 用户在设置中心选择本地 OpenClaw | `builtin_llm -> provider_backed` | UI/文档使用结构化 provider 维度；旧接入方式/实现名只保留兼容说明 | 仅本地用户可配 |
| 决策请求 | `DecisionRequest` | runtime 对目标 agent 发起一次决策请求 | `observed -> requesting -> responded` | 每 tick 每 agent 至多一请求 | 仅本地 runtime 发起 |
| 结构化决策 | `decision/action_ref/args/diagnostics` | provider 返回 wait/act | `responded -> validated -> executed/rejected` | 动作必须先过 schema | runtime 权威裁定 |
| 状态反馈 | `FeedbackEnvelope` | runtime 把执行结果回写 provider | `executed/rejected -> feedback_sent` | 顺序跟随 action_id | 仅对应会话可写 |
| 故障回退 | `error_code/error/detail/retryable` | launcher/viewer 显示错误并允许回退 | `ready -> degraded -> fallback` | retryable 错误优先重试一次 | 不得自动切远端 |
- Acceptance Criteria:
  - AC-1: 文档定义 `OpenClaw(Local HTTP)` 的用户安装与接入路径，覆盖发现、选择、绑定、启动、调试与恢复。
  - AC-2: 文档冻结最小本地 HTTP 协议集合：`/v1/provider/info`、`/v1/provider/health`、`/v1/world-simulator/decision`、`/v1/world-simulator/feedback`。
  - AC-3: 文档明确首期仅监听 `127.0.0.1`，不开放局域网与公网访问。
  - AC-4: 文档明确首期可玩动作白名单与非目标范围，并要求非法输出统一降级处理。
  - AC-5: 文档明确 launcher / viewer 所需的状态字段与用户提示文案边界。
  - AC-6: 文档定义 required/full 验证矩阵，要求可用 mock local HTTP server 覆盖首期协议。
- Non-Goals:
  - 不在首期引入远程 `OpenClaw` provider、云托管 provider 或公网隧道。
  - 不在首期让 `OpenClaw` 直接执行高频战斗、经济关键路径或批量 agent 群控。
  - 不在首期引入反向 tool callback、双向流式 event feed 或复杂 OAuth 登录。
  - 不在首期把 `OpenClaw` 变成 launcher / viewer 的统一控制面；它只负责世界内 agent 决策。

## 3. AI System Requirements (If Applicable)
- Tool Requirements:
  - `OpenClaw` 首期只需提供本地 HTTP JSON 接口，不要求浏览器 DOM 自动化。
  - provider 输出必须是结构化动作，而不是自由文本。
  - provider 最好导出本轮消息摘要、tool 摘要、延迟与错误信息，以便映射到 trace。
- Evaluation Strategy:
  - `test_tier_required`: 文档建模 + mock local HTTP provider + adapter contract tests + error policy tests。
  - `test_tier_full`: 真实 `OpenClaw(Local HTTP)` 单 NPC 闭环试点，验证动作有效率、延迟、trace 完整度与用户可恢复性。
- Local Runtime Requirements:
  - launcher 与 game runtime 必须支持独立启动；本地 `OpenClaw` 不要求由游戏进程托管。
  - 若用户配置“启动游戏时自动探测本地 OpenClaw”，探测失败不得阻断游戏本身启动。

## 4. Technical Specifications
- Architecture Overview:
  - `OpenClaw`：用户机上独立运行的本地进程，只监听 `127.0.0.1:<port>`。
  - `OpenClawAdapter`：world-simulator 内的 provider adapter，实现 `DecisionProvider`/`AgentBehavior facade`。
  - `Launcher`：负责发现 provider、保存本地配置、展示状态与错误、允许用户启用/禁用本地 provider。
  - `Viewer`：负责展示 trace、provider 状态、最近错误与最近动作摘要。
  - `Runtime/Kernel`：继续负责动作校验、执行、事件与状态演化。
- Integration Points:
  - `crates/oasis7/src/simulator/agent.rs`
  - `crates/oasis7/src/simulator/memory.rs`
  - `crates/oasis7_proto/src/viewer.rs`
  - `crates/oasis7_client_launcher/src/*`
  - `crates/oasis7/src/bin/oasis7_web_launcher/gui_agent_api.rs`
- Local HTTP Endpoints:
  - `GET /v1/provider/info`
    - 返回 `provider_id/name/version/protocol_version/capabilities/supported_action_sets`。
  - `GET /v1/provider/health`
    - 返回 `ok/status/uptime_ms/last_error/queue_depth`。
  - `POST /v1/world-simulator/decision`
    - 请求体：`DecisionRequest`。
    - 响应体：`DecisionResponse`。
  - `POST /v1/world-simulator/feedback`
    - 请求体：`FeedbackEnvelope`。
    - 响应体：`ok/error_code/error`。
- Discovery & Configuration:
  - 默认探测地址：`127.0.0.1:5841`（可配置）。
  - launcher 设置项：
    - `agent_decision_source`：`builtin_llm` / `provider_backed`
    - `agent_provider_backend`：当前固定为 `openclaw`
    - `agent_provider_contract`：当前固定为 `worldsim_provider_v1`
    - `agent_provider_transport`：当前固定为 `loopback_http`
    - `agent_provider_mode`：仅兼容读取 `agent_direct_connect` / `openclaw_local_http`
    - `openclaw_base_url`
    - `openclaw_auth_token`（可选；若配置则仅本地保存）
    - `openclaw_auto_discover`
    - `openclaw_connect_timeout_ms`
    - `openclaw_agent_profile`
  - profile 约定：首期 `P0` / parity / experimental 试点默认使用 `oasis7_p0_low_freq_npc`；旧别名 `oasis7_p0_low_freq_npc` 已移除。若 provider 不识别当前默认 profile，必须返回结构化 `unsupported_agent_profile`，禁止静默改用通用玩法。
  - 发现逻辑：优先读取显式配置；若未配置且开启 auto-discover，则探测默认地址。
  - 产品主链路：`oasis7_client_launcher -> oasis7_game_launcher -> oasis7_viewer_live` 现已透传 `agent_decision_source + agent_provider_* + agent_execution_lane`，并通过子进程环境把 OpenClaw 设置送入 runtime live sidecar；`agent_provider_mode/openclaw_*` 仅保留兼容透传。
- DecisionRequest Shape:
  - 顶层字段：`request_id/agent_id/world_time/provider_session_id?/provider_config_ref?/agent_profile?/timeout_ms`
  - `observation`: 当前可见世界状态摘要、附近实体、最近事件、目标与资源摘要。
  - `memory`: 短期记忆摘要、长期记忆命中结果、最近失败动作。
  - `action_catalog`: 动作白名单、参数 schema、枚举值范围、cooldown / cost hint。
  - `player_context`: `player_id`、是否允许外部 provider 接管、绑定关系版本。
  - `trace_context`: 是否要求 provider 返回 transcript/tool summary/diagnostics。
  - `agent_profile`: provider-side 玩法 profile / skill 标识；首期 required 路径至少支持 `oasis7_p0_low_freq_npc`，旧别名 `oasis7_p0_low_freq_npc` 必须返回 `unsupported_agent_profile`。
- DecisionResponse Shape:
  - `ok`
  - `decision`: `wait` / `wait_ticks` / `act`
  - `action_ref`：仅当 `decision=act` 时出现
  - `args`
  - `diagnostics`: `provider/model/latency_ms/retry_count`
  - `trace_payload`: `messages/tool_calls/tool_results/summary/error`
  - `error_code/error/retryable`
- Phase-1 Action Whitelist:
  - `wait`
  - `wait_ticks`
  - `move_agent`
  - `speak_to_nearby`（lightweight speech event）
  - `inspect_target`（lightweight inspection event）
  - `simple_interact`（lightweight interaction event）
- Error Handling & Fallback:
  - `connection_refused` / `provider_unreachable`: launcher 显示“本地 OpenClaw 未启动”，允许一键切回内置 provider。
  - `version_mismatch`: 阻止启用该 provider，并显示期望协议版本。
  - `timeout`: 本轮决策降级为 `Wait`，若连续超时达到阈值则 provider 状态变 `degraded`。
  - `invalid_action_schema`: 直接 `ActionRejected` 并记录到 trace。
  - `unsupported_semantic_action`: 对于不在 phase-1 白名单内、或 target_kind / payload 不满足当前 lightweight 语义约束的 intent，required 路径必须降级为 `Wait` 并记录结构化错误，禁止伪装为已执行成功。
  - `unsupported_agent_profile`: provider 标记为 `misconfigured`，launcher / parity bench 必须提示用户切回 builtin 或修正 profile。
  - `agent_provider_chat_unsupported` / `agent_provider_prompt_control_unsupported`: 在当前主链路下，agent 直连 provider 尚不支持 runtime live 的 `agent_chat` 与 `prompt_control` 直接注入，必须显式报错而不是伪装成功。
  - `auth_failed`: provider 标记为 `unauthorized`，要求用户更新本地 token。
  - `openclaw_gateway_unreachable`: 本地兼容桥无法通过 `openclaw agent` / Gateway 拿到响应时，provider health 需暴露最近错误，launcher / parity bench 必须明确提示“OpenClaw Gateway 未就绪”。
  - `bundle_cache_path_unexpanded`: `oasis7` 的 bundle-first 下载辅助必须先展开当前用户 `~`，再落缓存与返回 `bundle_dir`；若解析后的 bundle 缺少 `run-game.sh`，`doctor` 必须输出解析后的绝对路径，避免把产物误写到 repo-local `~/...`。
  - `repo_bootstrap_unavailable`: `oasis7 doctor` 必须把 bundle-first no-`cargo` 可玩性（bundle + bridge 是否就绪）与 repo-backed bridge/bootstrap 能力（repo root + `cargo`）分开汇报；缺少 `cargo` 或 repo root 时，若 bundle-first reuse path 仍可用，不得把其伪装成通用阻断。`play` 若落到 repo-backed bridge/bootstrap 依赖，必须输出可执行指引：安装 `cargo` / 提供 `--repo-root`，或改走 `--reuse-bridge --skip-agent-setup`。
  - `play_wrapper_orphan_subtree`: `oasis7-run.sh play` 被中断或 wrapper 退出时，必须尽最大努力终止其启动的 launcher 子树；不能出现 wrapper 已退但 `oasis7_game_launcher` / `oasis7_chain_runtime` / `oasis7_viewer_live` 继续常驻并占端口的假停止状态。
  - `bundle_download_observability_gap`: `oasis7` 的 bundle-first 下载辅助必须输出可见阶段日志（至少覆盖 asset download / checksum / extract / bundle ready）；当 stderr 非 TTY 且下载耗时较长时，必须持续输出周期性 heartbeat，避免首轮下载被误判为卡死。
  - `bridge_model_output_invalid`: 兼容桥若拿到非 JSON、缺字段或超出 phase-1 白名单的输出，必须在 provider 侧记录结构化 diagnostics/trace；若当前 profile/fixture 已明确给出低风险可达动作（如 `P0-001` 巡游移动），允许通过 profile guardrail 把无效输出重路由到最近可达的合法动作，否则才降级为 `Wait`。
  - `session_cross_talk`: 兼容桥必须使用 `provider_config_ref + agent_profile + agent_id` 派生 OpenClaw session scope，防止不同 benchmark run / runtime live 进程复用同一 session 造成旧世界状态串线。
- Non-Functional Requirements:
  - NFR-1: 本地 HTTP 仅绑定 `127.0.0.1`，默认不使用 `0.0.0.0`。
  - NFR-2: `GET /info` 与 `GET /health` 本地探测 `p95 <= 200ms`。
  - NFR-3: 首期单次 `decision` 本地请求 `p95 <= 3s`。
  - NFR-4: 首期 provider 错误不得使 runtime tick 卡死；超时后必须回落为可继续推进的状态。
  - NFR-5: mock local HTTP provider 必须可用于 CI / required regression。
  - NFR-6: `DecisionRequest.agent_profile` 必须可经 `ProviderBackedAgentBehavior -> OpenClawAdapter -> local HTTP` 完整透传，并体现在 parity summary / trace 归档中。
  - NFR-7: `oasis7_openclaw_local_bridge` 只能绑定 `127.0.0.1`，且必须支持显式端口/agent/profile 配置，默认地址保持 `127.0.0.1:5841`。
  - NFR-8: 兼容桥必须把 `OpenClaw` 原始文本输出保存在 `trace_payload.transcript/output_summary`，并把 parse/repair 结果反映到 `schema_repair_count` 与最近错误。
  - NFR-9: 兼容桥派生的 OpenClaw session id 必须带上 `provider_config_ref` 作用域，至少要把 benchmark run / runtime live 进程彼此隔离，避免 `loc-2` 之类的历史上下文泄漏到新的世界样本。
  - NFR-10: 仓库必须提供可复用的轻量 OpenClaw runtime agent bootstrap（workspace 模板 + setup 脚本），用于把 world-simulator 决策路径与用户日常聊天/运营 workspace 隔离，降低 system prompt 体积与 session 污染风险。
  - NFR-11: world-simulator 兼容桥派生的 OpenClaw `sessionKey` 应优先使用 `subagent:` 作用域，显式触发 OpenClaw minimal prompt mode，避免把 NPC 决策误走成 full personal-assistant prompt。
  - NFR-12: `oasis7-run.sh download` 在 release bundle 检测失败时必须立即非零退出；禁止在 `detected_bundle` 为空、非绝对路径或缺少 `run-game.sh` 时创建/填充缓存 `bundle/`，避免误复制宿主 `/.`。
- Security & Privacy:
  - 仅接受 loopback 地址；launcher 对 base URL 做 host allowlist 校验。
  - 不向 provider 暴露私钥、完整 auth proof 或内部存储路径。
  - `openclaw_auth_token` 如启用，只存本地配置，不回显在 viewer / logs。
- 若 OpenClaw 玩法链路沿用产品默认 `oasis7_game_launcher` 启动栈并拉起 `oasis7_chain_runtime`，则所选 chain profile 下的 node private key 必须按高敏资产处理：禁止提交到仓库、回显到诊断日志/截图/issue，并要求操作者优先使用临时 profile 或明确的资产归属 profile。
  - 用户必须显式切到 provider-backed OpenClaw 组合（兼容 alias 可继续接受 `agent_direct_connect/openclaw_local_http`），默认仍使用内置 provider。

## 5. Risks & Roadmap
- Phased Rollout:
  - M1 (2026-03-12): 完成本地 HTTP 接入方案建模。
  - M2: 落地 provider config、discovery/health-check 与 mock local HTTP contract tests。
  - M3: 实现 `OpenClawAdapter` request/response/feedback 映射。
  - M4: 在 launcher / viewer 加入 provider 状态、错误与 trace 摘要面板。
  - M5: 补齐 `oasis7_openclaw_local_bridge`，先把已安装 `OpenClaw Gateway/CLI` 转成 world-simulator 兼容 provider，再执行单低频 NPC 实机试点并决定是否扩展动作集。
- Technical Risks:
  - 风险-1: 本地 `OpenClaw` 的实际接口与假定协议不完全一致，需要 adapter 额外归一化。
  - 风险-2: 用户机上本地端口冲突、杀毒软件、权限限制可能导致 provider 探测失败。
  - 风险-3: 若 observation 过大，请求体会膨胀；需要摘要策略和字段预算。
  - 风险-4: 若首期就支持反向 callback，会让本地安全边界复杂化；因此本方案明确首期只做单次 request/response。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-WORLD_SIMULATOR-037 | TASK-WORLD_SIMULATOR-113 | `test_tier_required` | `./scripts/doc-governance-check.sh` | 模块文档入口、专题索引、本地 HTTP 方案建模 |
| PRD-WORLD_SIMULATOR-037 | T1/T2/T3 | `test_tier_required` | mock local HTTP server + adapter contract tests + launcher config tests | provider 发现、握手、决策 contract、失败回退 |
| PRD-WORLD_SIMULATOR-037 | T4/T5 | `test_tier_full` | 真实 `OpenClaw(Local HTTP)` 单 NPC 闭环试点 | 玩家安装路径、体验可行性、trace 完整度 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-OC-LOCAL-001 | 首期采用 `localhost HTTP/JSON` 作为本地外脑传输层 | 首期直接使用 named pipe / UDS / stdio | HTTP 更易调试、跨平台、便于 launcher/viewer 共享健康检查与状态展示。 |
| DEC-OC-LOCAL-002 | `OpenClaw` 作为独立本地进程运行 | 由游戏进程托管/内嵌 OpenClaw 生命周期 | 独立进程更符合用户安装与升级路径，也避免把外部 provider 生命周期绑死到游戏进程。 |
| DEC-OC-LOCAL-003 | 首期只做 `request -> response -> feedback` 单次本地调用 | 首期就做反向 callback、流式工具调用、双向会话总线 | 先用最小协议跑通用户可玩闭环，降低首期安全与工程复杂度。 |
| DEC-OC-LOCAL-004 | 默认仅开放低频动作白名单 | 一开始就开放所有世界动作 | 先收敛风险、验证协议和用户路径，再决定是否扩面。 |
