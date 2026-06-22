# oasis7: 启动器 Viewer 鉴权自动继承 Node 配置（2026-03-02）

- 对应设计文档: `doc/testing/launcher/launcher-viewer-auth-node-config-autowire-2026-03-02.design.md`
- 对应项目管理文档: `doc/testing/launcher/launcher-viewer-auth-node-config-autowire-2026-03-02.project.md`

审计轮次: 5


## 1. Executive Summary
- Problem Statement: 通过启动器打开 Web Viewer 时，聊天与 Prompt 控制鉴权依赖手工设置 `OASIS7_VIEWER_AUTH_PUBLIC_KEY/PRIVATE_KEY`，导致易错且与节点配置割裂。且 `agent_chat` 业务拒绝会被 Viewer 误判为连接失败，造成“发送消息后连接异常”的错误反馈。
- Proposed Solution: 将 Viewer 鉴权默认口径收敛到 `config.toml [node]` keypair，并在启动器注入/Viewer 回退链路中统一读取，同时保留环境变量覆盖能力。补充连接状态分类规则：`AgentChatError` 仅作为业务层错误，不降级全局连接状态。
- Success Criteria:
  - SC-1: `oasis7_game_launcher` 在 Web 注入 `window.__OASIS7_VIEWER_AUTH_ENV`，默认带入 node keypair。
  - SC-2: Viewer wasm 端优先读取注入配置，native 端支持 `config.toml [node]` 回退。
  - SC-3: `OASIS7_VIEWER_*` 环境变量覆盖能力保持不变。
  - SC-4: 用户无需手工设置鉴权 env 即可在启动器链路使用聊天与 Prompt 控制。
  - SC-5: `AgentChatError` 发生时 UI 不再显示“连接异常”，连接状态保持 transport 真实状态。

## 2. User Experience & Functionality
- User Personas:
  - 本地调试开发者：希望“一键启动即可用”而非手工配鉴权。
  - 测试维护者：希望 CLI/GUI 与 Web/native 鉴权口径一致。
  - 自动化脚本维护者：希望历史环境变量覆盖逻辑不被破坏。
  - 启动器体验验证者：希望业务失败与链路断连在 UI 上可区分，降低误判成本。
- User Scenarios & Frequency:
  - 启动器打开 Web Viewer：每次本地调试或演示均会触发。
  - 本地 Viewer 回归：测试链路验证回退解析。
  - CI/脚本场景：通过环境变量覆盖 player/keypair。
- User Stories:
  - PRD-TESTING-LAUNCHER-AUTH-001: As a 开发者, I want viewer auth keys auto-inherited from node config, so that chat/prompt controls work without manual env setup.
  - PRD-TESTING-LAUNCHER-AUTH-002: As a 测试维护者, I want wasm/native auth source precedence defined and testable, so that behavior is deterministic.
  - PRD-TESTING-LAUNCHER-AUTH-003: As a 自动化维护者, I want env overrides preserved, so that existing scripts remain compatible.
  - PRD-TESTING-LAUNCHER-AUTH-004: As a 启动器体验验证者, I want agent_chat business errors to not be treated as transport disconnects, so that connection diagnosis is accurate.
- Critical User Flows:
  1. Flow-AUTH-001: `oasis7_game_launcher 读取 config.toml[node] -> 注入 __OASIS7_VIEWER_AUTH_ENV -> Web Viewer 使用鉴权`
  2. Flow-AUTH-002: `wasm Viewer 优先读注入 -> 注入缺失时回退进程环境变量`
  3. Flow-AUTH-003: `本地 Viewer 读取环境变量 -> 无环境变量时回退 config.toml[node]`
  4. Flow-AUTH-004: `玩家发送 agent_chat -> runtime 返回 AgentChatError -> Viewer 显示聊天失败原因但保持连接状态`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 启动器 Web 注入 | `window.__OASIS7_VIEWER_AUTH_ENV`、`PLAYER_ID/PUBLIC_KEY/PRIVATE_KEY` | 返回 `index.html` 时注入脚本变量 | `config-loaded -> injected -> served` | 注入键名与环境变量键名保持一致 | 启动器维护者可调整注入逻辑 |
| wasm 鉴权解析 | 注入对象、进程环境变量 | 读取鉴权来源并构建 signer | `resolved -> auth-ready/failed` | 优先级：注入 > 环境变量 | Viewer 客户端自动执行 |
| native 鉴权回退 | 环境变量、`config.toml[node]` | 环境变量缺失时读配置文件回退 | `resolved -> auth-ready/failed` | 优先级：环境变量 > config 回退 | 本地运行用户可通过 env 覆盖 |
| player_id 规则 | `viewer-player` 默认值、`OASIS7_VIEWER_PLAYER_ID` 覆盖 | 构建请求时附带 player_id | `defaulted -> overridden` | 存在 env 时优先使用 env | 运行者可显式覆盖 |
| 聊天错误状态归类 | `ViewerResponse::AgentChatError`、`ConnectionStatus` | 收到业务错误时只更新聊天错误反馈，不覆盖 transport 状态 | `connected -> connected`（业务错误）/ `connected -> error`（仅 transport 错误） | transport 错误优先影响连接态；业务错误只影响功能反馈 | Viewer 客户端自动执行 |
- Acceptance Criteria:
  - AC-1: 启动器在 Web 响应中注入标准化 `OASIS7_VIEWER_AUTH_*` 键。
  - AC-2: wasm 端支持注入读取并保持环境变量兼容路径。
  - AC-3: native 端支持 `config.toml[node]` 回退。
  - AC-4: `player_id` 默认/覆盖逻辑稳定并可测试。
  - AC-5: 文档、项目状态与测试结果完成收口。
  - AC-6: 发送 `agent_chat` 后若收到 `AgentChatError`，连接状态仍保持 `Connected`（或原 transport 状态），不显示“连接异常”。
- Non-Goals:
  - 不移除环境变量覆盖机制。
  - 不改造 Viewer 鉴权协议本身。
  - 不在本任务中引入非本地部署的密钥分发机制。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（本专题为启动器/Viewer 鉴权配置继承，不涉及 AI 推理模型改造）。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 通过启动器注入 + Viewer 双端回退策略，将鉴权默认源统一到 node 配置，兼容历史 env 覆盖并减少手工配置成本。
- Integration Points:
  - `crates/oasis7/src/bin/oasis7_game_launcher.rs`
  - `crates/oasis7/src/bin/oasis7_game_launcher/oasis7_game_launcher_tests.rs`
  - `crates/oasis7_viewer/src/egui_right_panel_chat_auth.rs`
  - `crates/oasis7_viewer/src/egui_right_panel_chat_tests.rs`
  - `crates/oasis7_viewer/src/main_connection.rs`
  - `crates/oasis7_viewer/src/tests.rs`
  - `crates/oasis7_viewer/Cargo.toml`
  - `config.toml` (`[node] private_key/public_key`)
- Edge Cases & Error Handling:
  - `config.toml` 缺失/损坏：回退路径失败时输出明确错误，建议使用 env 覆盖。
  - 仅有单边 key（缺私钥或公钥）：视为无效鉴权源，不静默成功。
  - wasm 注入缺失：自动回退到进程环境变量路径。
  - 非本地网络暴露：注入密钥会被页面访问者可见，需限制部署口径为本机调试/单机发布。
  - player_id 缺失：使用默认 `viewer-player`，保障功能可用。
  - `agent_chat` 被服务端拒绝（如 `--llm` 关闭、签名不合法）：返回业务错误，不触发连接态降级或重连风暴。
- Non-Functional Requirements:
  - NFR-AUTH-1: 启动器链路默认可用，无需额外手工配置 keypair。
  - NFR-AUTH-2: wasm/native 来源优先级行为一致且可测试。
  - NFR-AUTH-3: 环境变量覆盖兼容现有自动化脚本。
  - NFR-AUTH-4: 鉴权失败日志可直接定位来源缺失或格式异常。
  - NFR-AUTH-5: 连接状态语义与 transport 真实状态一致，业务错误不得造成误导性断连展示。
- Security & Privacy: 启动器注入链路应默认用于本地受信环境；对外暴露需额外防护或禁用私钥注入。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (AUTOWIRE-1): 建立设计与项目文档。
  - v1.1 (AUTOWIRE-2): 启动器完成 Web 鉴权注入并补测试。
  - v2.0 (AUTOWIRE-3): Viewer 完成 wasm 注入读取与 native 配置回退。
  - v2.1 (AUTOWIRE-4): 回归验证与文档/devlog 收口。
  - v2.2 (AUTOWIRE-6): Viewer 聊天业务错误与连接状态解耦，修复误报“连接异常”。
- Technical Risks:
  - 风险-1: Web 注入包含私钥，在非本机场景存在可见性风险。
  - 风险-2: 配置文件异常导致回退失败，需清晰报错与替代路径。
  - 风险-3: 多来源优先级逻辑变更可能引起兼容回归。
  - 风险-4: 业务错误不再占用连接状态后，若缺少聊天反馈承载，用户可能忽略具体失败原因。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-TESTING-LAUNCHER-AUTH-001 | AUTOWIRE-1/2 | `test_tier_required` | 启动器注入字段与默认值测试 | Web 启动鉴权可用性 |
| PRD-TESTING-LAUNCHER-AUTH-002 | AUTOWIRE-2/3 | `test_tier_required` | wasm/native 优先级与回退单测 | Viewer 鉴权解析稳定性 |
| PRD-TESTING-LAUNCHER-AUTH-003 | AUTOWIRE-3/4 | `test_tier_required` | 环境变量覆盖兼容与文档收口检查 | 自动化脚本兼容性 |
| PRD-TESTING-LAUNCHER-AUTH-004 | AUTOWIRE-6 | `test_tier_required` | `AgentChatError` 状态机分支单测（连接态保持） | Web 启动器聊天体验与连接诊断准确性 |
- 执行记录（2026-03-08）: AUTOWIRE-6 已完成，`oasis7_viewer` 连接状态机不再将 `AgentChatError` 视为 transport 断连。
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-AUTH-001 | 默认继承 `config.toml[node]` keypair | 强制用户手工设置 env | 降低使用门槛并统一配置入口。 |
| DEC-AUTH-002 | 保留环境变量覆盖优先能力 | 迁移后移除 env 路径 | 保证历史脚本与调试流程不回归。 |
| DEC-AUTH-003 | wasm 注入优先、native 配置回退 | 两端单一来源统一处理 | 兼顾 Web 启动器注入与 native 运行习惯。 |
| DEC-AUTH-004 | `AgentChatError` 归类为业务错误，不降级 `ConnectionStatus` | 将业务错误与 transport 错误统一映射到连接异常 | 连接状态应反映链路健康而非业务成功率，避免误导排障。 |

## 原文约束点映射（内容保真）
- 原“目标（自动继承 node 配置、保留 env 覆盖）” -> 第 1 章 Problem/Solution/SC。
- 原“范围（launcher/viewer/tests/Cargo）” -> 第 2 章 AC + 第 4 章 Integration。
- 原“接口/数据（注入对象、优先级、player_id）” -> 第 2 章规格矩阵 + 第 4 章技术细节。
- 原“里程碑 M1~M4” -> 第 5 章 Phased Rollout（AUTOWIRE-1~4）。
- 原“风险（Web 可见性、配置一致性）” -> 第 4 章 Edge Cases + 第 5 章 Risks。
- 本轮补充“`AgentChatError` 误判连接异常” -> 第 1 章 Problem、第 2 章 Flow/AC、第 4 章 Edge Cases、第 6 章 Validation/Decision。
