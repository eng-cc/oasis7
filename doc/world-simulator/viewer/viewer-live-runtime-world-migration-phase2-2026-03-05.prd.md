# Viewer Live runtime/world 接管 Phase 2（LLM/chat/prompt）（2026-03-05）

- 对应设计文档: `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase2-2026-03-05.design.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase2-2026-03-05.project.md`

审计轮次: 5

## 1. Executive Summary
- Problem Statement: Phase 1 下 `oasis7_viewer_live --runtime-world` 仅支持脚本驱动，`PromptControl` 与 `AgentChat` 直接返回 `unsupported_in_runtime_live_phase1`，导致 runtime 与 simulator 在 LLM 体验上断裂。
- Proposed Solution: 在保持 `WorldSnapshot/WorldEvent` 协议兼容前提下，runtime live 增加 llm 模式分支，打通 `--runtime-world --llm`、prompt/chat 鉴权与配置生效链路，并对 runtime 可映射动作先行桥接。
- Success Criteria:
  - SC-1: `oasis7_viewer_live --runtime-world --llm` 可正常启动，不再被 CLI 预检拒绝。
  - SC-2: runtime live 在 llm 模式下支持 `PromptControl`（preview/apply/rollback）与 `AgentChat` 请求闭环，且错误码与 simulator live 语义对齐。
  - SC-3: runtime live 在 script 模式下对 prompt/chat 明确返回 `llm_mode_required`，避免静默失败。
  - SC-4: prompt/chat 鉴权签名校验、nonce anti-replay、agent-player 绑定校验可用。
  - SC-5: llm 决策链路可驱动 runtime 动作（先覆盖 core + gameplay/economic 可映射子集），不可映射动作需可诊断降级。
  - SC-6: `test_tier_required` 命令通过并可追溯到 `PRD-WORLD_SIMULATOR-017`。

## 2. User Experience & Functionality
- User Personas:
  - 玩法架构开发者：希望 runtime 模式也能做 prompt 调优与 chat 交互，不再切回 simulator 模式。
  - LLM 行为调试者：希望鉴权失败、版本冲突、回滚失败等错误有统一结构，便于快速定位。
  - 回归测试人员：希望 runtime/script 与 runtime/llm 的行为边界可验证且稳定。
- User Scenarios & Frequency:
  - 日常联调：开发者使用 `--runtime-world --llm` 进行提示词迭代与聊天驱动测试。
  - 发布前回归：测试人员验证 runtime 模式下 prompt/chat 的成功与失败路径。
  - 故障排查：出现鉴权或映射失败时，开发者通过结构化错误快速定位问题。
- User Stories:
  - As a 玩法架构开发者, I want runtime live to support llm/chat/prompt controls, so that I can debug runtime behavior without switching back to simulator live.
  - As a 安全与测试人员, I want runtime prompt/chat to enforce auth + nonce replay protections, so that control requests remain auditable and safe.
  - As a Viewer 开发者, I want runtime llm mode to keep protocol compatibility, so that front-end interaction remains unchanged.
- Critical User Flows:
  1. Flow-VIEWER-RUNTIME-LLM-001（runtime llm 启动）:
     `oasis7_viewer_live --runtime-world --llm -> 启动 runtime live llm server -> Hello/Subscribe/RequestSnapshot`
  2. Flow-VIEWER-RUNTIME-LLM-002（prompt apply）:
     `PromptControl::Apply(含签名) -> 校验签名/nonce/绑定/版本 -> 更新 profile -> 回写 snapshot model -> 返回 PromptControlAck`
  3. Flow-VIEWER-RUNTIME-LLM-003（agent chat）:
     `AgentChat(含签名) -> 校验签名/nonce/绑定 -> 注入 LLM 行为消息队列 -> 下个 step 消费决策`
  4. Flow-VIEWER-RUNTIME-LLM-004（动作桥接降级）:
     `LLM 决策 action -> runtime 可映射则执行 runtime action -> 不可映射则输出结构化拒绝并保持服务不崩溃`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| runtime llm 模式开关 | `--runtime-world` + `--llm` | 启用 runtime live + llm 决策分支 | `runtime_script <-> runtime_llm`（启动参数决定） | 启动时一次性判定 | 本地启动参数控制 |
| PromptControl（runtime） | `agent_id/player_id/public_key/auth/expected_version/overrides` | Preview 仅回传候选版本与 digest；Apply/Rollback 生效并返回 ack | `profile[vN] -> profile[vN+1]` | 版本单调递增；digest 基于 profile 内容计算 | 必须通过 auth 签名、nonce anti-replay 与 agent-player 绑定 |
| AgentChat（runtime） | `agent_id/player_id/public_key/auth/message` | 消息入队并返回 accepted ack | `chat_pending -> llm_decision_pending -> consumed` | 消息长度按字符计数返回 | 必须通过 auth 签名、nonce anti-replay 与 agent-player 绑定 |
| LLM 决策动作桥接 | simulator action -> runtime action（子集） | 可映射动作提交 runtime；不可映射动作输出拒绝事件 | `decision -> mapped_action/rejected` | 优先执行可映射动作；失败保留错误上下文 | 不新增远程写接口，仅本地受控链路 |
- Acceptance Criteria:
  - AC-1: `oasis7_viewer_live` 不再拒绝 `--runtime-world --llm` 组合参数。
  - AC-2: `ViewerRuntimeLiveServerConfig` 支持 llm/script 决策模式，并在 runtime live 中生效。
  - AC-3: runtime live 在 llm 模式下 `PromptControl` 与 `AgentChat` 不再返回 `unsupported_in_runtime_live_phase1`。
  - AC-4: runtime live 在 script 模式下 `PromptControl` 与 `AgentChat` 返回 `llm_mode_required`。
  - AC-5: runtime live prompt/chat 使用签名校验与 nonce anti-replay；校验失败返回结构化错误码。
  - AC-6: Apply/Rollback 后 snapshot 中可观察到 prompt profile / player 绑定 / nonce 状态更新。
  - AC-7: llm 决策推进 runtime 时，可映射动作成功执行；不可映射动作返回可诊断拒绝。
  - AC-8: 命令通过：`env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_viewer_live` 与 `env -u RUSTC_WRAPPER cargo check -p oasis7 --bin oasis7_viewer_live`。
- Non-Goals:
  - 不在 Phase 2 完成 simulator action 到 runtime action 的全量 1:1 映射。
  - 不在 Phase 2 改造 viewer 前端协议或 UI 结构。
  - 不在 Phase 2 移除 runtime script 模式与 simulator live 模式。

## 3. AI System Requirements (If Applicable)
- Tool Requirements:
  - runtime live llm runner（`AgentRunner<LlmAgentBehavior<_>>`）
  - prompt/chat auth 签名校验（viewer auth proof）
  - runtime action bridge（core + gameplay/economic 可映射子集）
- Evaluation Strategy:
  - 功能性：prompt/chat 成功与失败路径回归通过。
  - 安全性：nonce replay、签名篡改、绑定冲突均可稳定拦截。
  - 一致性：runtime llm 模式下返回结构与 simulator live 对齐，不再出现 phase1 专属错误码。

## 4. Technical Specifications
- Architecture Overview:
  - runtime live 新增 llm sidecar 状态：维护 `WorldKernel + AgentRunner + auth/profile/binding`。
  - 控制面仍由 runtime::World 对外输出兼容快照/事件；llm sidecar 负责 prompt/chat 与决策产出。
  - 决策执行采用“可映射动作直接落 runtime，不可映射动作结构化拒绝”的渐进桥接策略。
- Integration Points:
  - `crates/oasis7/src/bin/oasis7_viewer_live.rs`
  - `crates/oasis7/src/viewer/runtime_live.rs`
  - `crates/oasis7/src/viewer/auth.rs`
  - `crates/oasis7/src/viewer/protocol.rs`
  - `crates/oasis7/src/bin/oasis7_llm_agent_demo/runtime_bridge.rs`
  - `doc/world-simulator/viewer/viewer-manual.manual.md`
- Edge Cases & Error Handling:
  - 缺失 auth proof：返回 `auth_proof_required`。
  - 签名校验失败：返回 `auth_signature_invalid` / `auth_claim_mismatch`。
  - nonce replay：返回 `auth_nonce_replay`。
  - agent 与 player/public_key 绑定冲突：返回 `agent_control_forbidden`。
  - prompt version 冲突：返回 `version_conflict`。
  - runtime action 映射缺失：返回结构化 `ActionRejected::RuleDenied`（带映射失败原因），服务不 panic。
  - llm 模式未启用时调用 prompt/chat：返回 `llm_mode_required`。
- Non-Functional Requirements:
  - NFR-1: runtime llm 模式启动监听端口时间 `p95 <= 2s`（本地环境，不含首轮 LLM API 调用）。
  - NFR-2: prompt/chat 鉴权失败请求返回 `p95 <= 100ms`（本地环境）。
  - NFR-3: runtime live 新增/改造 Rust 文件行数 <= 1200。
- Security & Privacy:
  - prompt/chat 必须走签名校验与 nonce anti-replay。
  - 日志不得输出私钥、完整签名私密材料。
  - runtime live 默认本地绑定，不扩展远程写入暴露面。

## 5. Risks & Roadmap
- Phased Rollout:
  - M1: 文档建模与任务拆解（Phase 2 PRD / project）。
  - M2: CLI 接线与 runtime live llm 控制面打通。
  - M3: prompt/chat/auth/bridge 回归与文档收口。
- Technical Risks:
  - 风险-1: simulator->runtime 动作映射覆盖不足导致 llm 决策有效动作比例下降。
  - 风险-2: runtime 与 llm sidecar 状态双轨存在时间同步偏差，需要在回归中重点关注。
  - 风险-3: 外部 LLM 依赖波动影响体验稳定性，需保留可诊断错误上下文。

## 6. Validation & Decision Record
- Test Plan & Traceability:
  - PRD-WORLD_SIMULATOR-017 -> TASK-WORLD_SIMULATOR-038/039 -> `test_tier_required`。
- Decision Log:
  - DEC-WS-013: 采用“runtime 控制面 + llm sidecar + 动作桥接子集”的 Phase 2 渐进方案；否决“等待全量动作 1:1 映射后再开放 prompt/chat”。依据：先打通控制体验可快速消除前后端断裂，并把映射风险约束在可诊断范围内。
