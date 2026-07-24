# 启动器 chain runtime stale execution world 恢复与默认 node_id 冲突规避（2026-03-12）

- 对应设计文档: `doc/world-simulator/launcher/game-client-launcher-chain-runtime-stale-execution-world-recovery-2026-03-12.design.md`
- 对应项目管理文档: `doc/world-simulator/launcher/game-client-launcher-chain-runtime-stale-execution-world-recovery-2026-03-12.project.md`

审计轮次: 6

## 1. Executive Summary
- Problem Statement: 当前启动器默认使用固定 `chain_node_id=viewer-live-node` 与稳定 `execution_world_dir`；当目录下残留旧执行世界且状态根不匹配时，`oasis7_chain_runtime` 会直接以 `DistributedValidationFailed` 退出，导致 launcher 只看到“链不可达”，用户缺少明确恢复路径。
- Proposed Solution: 在 launcher / Web 控制面增加 stale execution world 识别与恢复策略，针对默认 node id 冲突场景输出结构化错误、可操作恢复建议，并支持显式使用 fresh node id 或受控重置执行世界目录完成恢复。同时把 launcher 产品默认链配置从固定 `viewer-live-node` 收敛为自动生成 fresh `chain_node_id`；仅在用户显式填写时保留自定义值，从源头降低默认入口命中旧目录的概率。
- Success Criteria:
  - SC-1: 默认 launcher 链启动在遇到 stale execution world 冲突时，不再只暴露泛化 `unreachable`，而是能识别为可恢复的 `stale_execution_world` 类问题。
  - SC-2: Web 控制面、GUI Agent 与桌面/网页启动器界面对该类问题提供一致的恢复提示与结构化错误码。
  - SC-3: 至少一种受支持恢复路径可在不手工改 CLI 的前提下完成恢复（如 fresh node id 或受控清理后重试）。
  - SC-4: 恢复完成后，`start_chain -> query_explorer_overview -> start_game` 最小闭环可重新通过。
  - SC-5: `scripts/run-game-test.sh` 这类一键试玩包装脚本默认不再复用固定 `viewer-live-node`，而是使用 fresh `chain_node_id` 与明确的 `chain_status_bind`，避免把历史 execution world 脏状态带进新的试玩会话。
  - SC-6: `oasis7_web_launcher` / `oasis7_client_launcher` / `oasis7_game_launcher` 的默认链配置不再预填固定 `viewer-live-node`，而是自动生成 fresh `chain_node_id`；用户显式输入的 `chain_node_id` 不受影响。

## 2. User Experience & Functionality
- User Personas:
  - 启动器玩家 / 试玩者：希望链启用失败时得到明确、可执行的恢复指引，而不是只看到链不可达。
  - `viewer_engineer`: 需要保证 launcher / Web 控制面在状态污染场景下仍有可解释的错误与恢复流程。
  - `runtime_engineer`: 需要 launcher 对执行世界校验失败的表现层与恢复边界清晰，不破坏 runtime 数据完整性。
- User Scenarios & Frequency:
  - 重复试玩、重复启停、跨版本复用同一 bundle 时，默认 node id 极易复用旧执行世界目录；该问题在制作人长玩、QA 长稳回归与本地 demo 反复重启中均会高频出现。
- User Stories:
  - PRD-WORLD_SIMULATOR-034: As a 启动器用户, I want launcher to detect and help recover from stale chain execution-world conflicts, so that I can restart the chain without reading raw runtime logs or manually changing node IDs.
- Critical User Flows:
  1. Flow-LAUNCHER-034-001（默认链启动命中 stale 目录）:
     `点击启动区块链 -> oasis7_chain_runtime 返回 DistributedValidationFailed/latest state root mismatch -> launcher 识别为 stale_execution_world -> UI/GUI Agent 返回结构化恢复建议`
  2. Flow-LAUNCHER-034-002（fresh node id 恢复）:
     `用户/GUI Agent 选择使用 fresh node id -> launcher 重新拼装链配置 -> chain_status 进入 ready -> explorer overview 可查询`
  3. Flow-LAUNCHER-034-003（受控清理恢复）:
     `用户确认清理默认 execution world -> launcher 执行受控清理/重建 -> chain_status 进入 ready`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| stale 执行世界识别 | `error_code=stale_execution_world`、`reason`、`execution_world_dir`、`node_id` | 启动链失败后识别 `DistributedValidationFailed/latest state root mismatch` 并提升为结构化错误 | `starting -> stale_execution_world` | 优先于泛化 `unreachable` 展示 | 启动器控制面可读 |
| 恢复建议 | `recovery_mode`、`fresh_node_id`、`reset_required` | UI 展示恢复 CTA；GUI Agent 返回可执行恢复选项 | `stale_execution_world -> recovery_suggested` | 默认先推荐不破坏旧数据的 fresh node id | 玩家可见，自动化可调 |
| 默认 fresh node id | `chain_node_id` | 默认配置初始化时自动生成 fresh node id；仅显式自定义时保持原值 | `default_config -> ready/startable` | fresh id 应避免落回固定 `viewer-live-node` 旧目录 | 默认玩家路径可直接用 |
| fresh node id 恢复 | `chain_node_id`、`chain_status_bind` | 生成 fresh node id 并重试链启动 | `recovery_requested -> starting -> ready` | fresh id 应避免与现有活跃/历史默认目录冲突 | 启动器侧受控写配置 |
| 受控重置恢复 | `execution_world_dir`、确认标记 | 明确确认后清理/重建默认目录并重试 | `recovery_requested -> reset -> starting -> ready` | 仅在显式确认时允许破坏性恢复 | 必须显式确认 |
- Acceptance Criteria:
  - AC-1: `oasis7_web_launcher` / `oasis7_game_launcher` 能识别 stale execution world 失败签名，并返回/展示 `stale_execution_world` 结构化错误码。
  - AC-2: GUI Agent 对应动作返回中包含恢复所需字段（至少 `node_id`、恢复建议或恢复模式）。
  - AC-3: 默认 UI 至少提供 1 条非 CLI 的恢复路径（fresh node id 或受控清理）。
  - AC-4: 定向回归覆盖“旧默认目录失败 -> 恢复 -> explorer overview 查询成功”。
  - AC-5: fresh 启动的 `oasis7_web_launcher` / `oasis7_client_launcher` 默认状态中，`chain_node_id` 应为 `viewer-live-node-fresh-*` 形态而不是固定 `viewer-live-node`；按默认配置直接 `start_chain` 可进入 `ready` 并成功查询 `explorer overview`。
- Non-Goals:
  - 不在本任务内改变 runtime 的状态校验算法或分布式一致性规则。
  - 不在本任务内放宽 `DistributedValidationFailed` 的安全门槛。
  - 不要求自动删除任何历史目录而不经确认。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: GUI Agent HTTP JSON 接口用于自动化恢复动作验证；必要时使用 `agent-browser` 仅做页面状态与 CTA 校验。
- Evaluation Strategy: 以 stale 错误分类准确率、恢复成功率、恢复后 query explorer 成功率评估。

## 4. Technical Specifications
- Architecture Overview:
  - 识别层：launcher 捕获 `oasis7_chain_runtime` stderr / 退出原因并归类 stale execution world。
  - 恢复层：launcher 配置层支持 fresh node id 或受控目录重置恢复模式。
  - 表现层：GUI Agent / Web 控制面 / launcher UI 使用统一错误码和恢复建议。
- Integration Points:
  - `crates/oasis7/src/bin/oasis7_game_launcher.rs`
  - `crates/oasis7/src/bin/oasis7_web_launcher/control_plane.rs`
  - `crates/oasis7/src/bin/oasis7_web_launcher/gui_agent_api.rs`
  - `crates/oasis7/src/bin/oasis7_web_launcher/oasis7_web_launcher_tests.rs`
  - `crates/oasis7/src/bin/oasis7_game_launcher/oasis7_game_launcher_tests.rs`
  - `crates/oasis7_client_launcher/src/*`
  - `doc/world-simulator/prd.md`
  - `doc/world-simulator/project.md`
- Edge Cases & Error Handling:
  - bundle 混版导致的参数不兼容不应误归类为 stale execution world；应保持原始 `action_failed/proxy_error` 语义。
  - 用户自定义 `chain_node_id` 时不应强制改写；仅对默认/建议值提供恢复路径。
  - 默认 fresh node id 应在 launcher 生命周期内保持稳定，避免同一会话里每次 UI 重绘都改写配置。
  - 受控清理前必须明确提示目录与风险，避免误删活跃世界。
- Non-Functional Requirements:
  - NFR-1: stale 识别不增加额外网络依赖，最多依赖本地进程退出输出与现有状态探针。
  - NFR-2: 恢复提示必须在一次失败后的同一状态视图可见，避免用户反复翻日志。
  - NFR-3: fresh node id 恢复应在 1 次重试内使链达到 `ready`，前提是无其他真实 runtime 阻断。
- Security & Privacy:
  - 不允许未经确认清理用户历史世界目录。
  - 错误信息可包含相对目录与 node id，但不应额外泄露无关本地路径。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP: 识别 stale execution world，输出结构化错误与恢复建议。
  - v1.1: 支持 fresh node id 一键恢复并补齐 GUI Agent / UI 回归。
  - v2.0: 评估是否增加受控清理默认目录的恢复路径。
- Technical Risks:
  - 风险-1: 错误分类过宽，误把其他 runtime 启动失败误归类为 stale execution world。
  - 风险-2: fresh node id 恢复若未同步状态端口与展示字段，可能产生新的配置漂移。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-WORLD_SIMULATOR-034 | TASK-WORLD_SIMULATOR-103/104/107/108/109 | `test_tier_required` | `./scripts/doc-governance-check.sh` + `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_web_launcher -- --nocapture` + `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_game_launcher -- --nocapture` + `env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher -- --nocapture` + GUI Agent 默认链闭环（`start_chain -> query_explorer_overview`） | launcher 链启动恢复体验、GUI Agent 契约、默认试玩链路 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-LAUNCHER-STALE-001 | 将 stale execution world 作为 launcher 级可恢复错误处理，并优先提供 fresh node id 恢复 | 保持 runtime 原始日志外泄，由用户手工换 node id 或删目录 | 当前默认试玩路径会频繁复用默认 node id；若不做 launcher 级恢复，玩家/制作人/QA 都需要读底层日志，体验不可接受。 |
