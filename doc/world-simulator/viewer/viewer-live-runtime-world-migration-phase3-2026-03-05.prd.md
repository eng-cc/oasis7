# Viewer Live runtime/world 接管 Phase 3（action 映射覆盖 + 旧分支移除）（2026-03-05）

- 对应设计文档: `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase3-2026-03-05.design.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase3-2026-03-05.project.md`

审计轮次: 5

## 1. Executive Summary
- Problem Statement: Phase 2 后 runtime live 已打通 `LLM/chat/prompt`，但动作映射覆盖仍有缺口，且 `oasis7_viewer_live` 保留 simulator 启动分支，持续造成双轨体验与回归成本。
- Proposed Solution: 在 runtime live 补齐高频可映射 action 覆盖并增加等价回归；同时删除 `oasis7_viewer_live` simulator 启动分支，统一为 runtime-only 链路。
- Success Criteria:
  - SC-1: `simulator_action_to_runtime` 覆盖扩展到旧链路高频动作集合（含模块工件相关动作）。
  - SC-2: 映射缺失动作保持结构化 `ActionRejected::RuleDenied`，错误语义稳定可回归。
  - SC-3: 新增等价回归测试，验证关键 action 的 runtime 映射输出与预期一致。
  - SC-4: `oasis7_viewer_live` 删除 simulator 启动分支，运行入口统一 runtime-only。
  - SC-5: required 回归命令通过并可追溯到 `PRD-WORLD_SIMULATOR-018`。

## 2. User Experience & Functionality
- User Personas:
  - 玩法架构开发者：希望 runtime/live 行为收敛到单一路径，减少分支差异排查成本。
  - 回归测试人员：希望 action 映射成功/拒绝边界稳定，避免跨版本漂移。
- User Scenarios & Frequency:
  - 日常联调：在 runtime-only 链路验证 llm/chat/prompt 与动作执行一致性。
  - 发布前回归：执行映射等价测试，确认关键动作不回退到不确定行为。
- User Stories:
  - As a 玩法架构开发者, I want runtime live action mapping coverage and runtime-only launch path, so that oasis7_viewer_live no longer has split simulator/runtime behavior.
- Critical User Flows:
  1. Flow-VIEWER-RUNTIME-LLM-005（action 映射覆盖）:
     `LLM 决策 action -> simulator_action_to_runtime -> runtime action 执行 / 结构化拒绝`
  2. Flow-VIEWER-RUNTIME-LLM-006（分支收敛）:
     `启动 oasis7_viewer_live -> 固定 runtime live server -> script/llm 按开关执行`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| runtime action 映射扩展 | `SimulatorAction` -> `RuntimeAction` | 可映射动作执行 runtime；不可映射返回结构化拒绝 | `decision -> mapped/rejected` | 先执行可映射动作，失败保留诊断上下文 | 不新增远程写接口 |
| oasis7_viewer_live runtime-only | `--llm/--no-llm` | 启动后仅走 runtime live server | `runtime_script/runtime_llm` | 不再分叉到 simulator server | 本地受控启动链路 |
- Acceptance Criteria:
  - AC-1: runtime live 新增模块工件相关动作映射覆盖，并通过单测验证输出。
  - AC-2: 关键旧链路动作（move/transfer/gameplay/economic）等价回归通过。
  - AC-3: 不可映射动作仍返回结构化拒绝，不发生 panic 或服务中断。
  - AC-4: `oasis7_viewer_live` 不再包含 simulator live 启动分支。
  - AC-5: 命令通过：`env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_viewer_live` 与 `env -u RUSTC_WRAPPER cargo check -p oasis7 --bin oasis7_viewer_live`。
- Non-Goals:
  - 不在 Phase 3 完成 simulator action 到 runtime action 的全量 1:1 映射。
  - 不在 Phase 3 改动 viewer 协议或前端 UI。

## 3. AI System Requirements (If Applicable)
- Tool Requirements:
  - runtime action bridge（覆盖扩展）
  - oasis7_viewer_live runtime-only 启动链路
- Evaluation Strategy:
  - 行为一致性：映射等价回归通过。
  - 可诊断性：不可映射动作拒绝语义稳定。

## 4. Technical Specifications
- Architecture Overview:
  - runtime live 继续作为唯一 live server 驱动，控制面保持 `WorldSnapshot/WorldEvent` 协议兼容。
  - action bridge 扩展采用“可安全 1:1 转换优先，无法对齐保持结构化拒绝”策略。
- Integration Points:
  - `crates/oasis7/src/viewer/runtime_live/control_plane.rs`
  - `crates/oasis7/src/viewer/runtime_live.rs`
  - `crates/oasis7/src/bin/oasis7_viewer_live.rs`
  - `doc/world-simulator/viewer/viewer-manual.manual.md`
- Edge Cases & Error Handling:
  - 模块动作字段不兼容：拒绝并返回 `ActionRejected::RuleDenied`。
  - legacy CLI 分支参数：不再触发 simulator server 启动。
  - 映射回归失败：保留单测断言定位具体 action 变更。
- Non-Functional Requirements:
  - NFR-1: runtime-only 启动路径不降低现有 required 回归通过率（目标 100%）。
  - NFR-2: 映射拒绝请求在本地环境返回延迟 `p95 <= 100ms`。
  - NFR-3: 新增/改造 Rust 文件行数 <= 1200。
- Security & Privacy:
  - 保持现有 prompt/chat 鉴权与 nonce anti-replay 约束不退化。
  - 错误输出不泄露私钥或签名私密材料。

## 5. Risks & Roadmap
- Phased Rollout:
  - M1: 文档建模与任务拆解（Phase 3 PRD/project）。
  - M2: action 映射扩展与等价回归落地。
  - M3: runtime-only 分支收敛、手册更新与 required 回归收口。
- Technical Risks:
  - 风险-1: 误映射导致行为偏差，需要等价回归兜底。
  - 风险-2: 旧参数/旧脚本依赖 simulator 分支时出现兼容性噪声。

## 6. Validation & Decision Record
- Test Plan & Traceability:
  - PRD-WORLD_SIMULATOR-018 -> TASK-WORLD_SIMULATOR-040/041 -> `test_tier_required`。
- Decision Log:
  - DEC-WS-014: 采用“动作映射覆盖扩展 + runtime-only 分支收敛”方案；否决“继续保留 simulator fallback”。依据：单链路可显著降低行为漂移与回归复杂度。
