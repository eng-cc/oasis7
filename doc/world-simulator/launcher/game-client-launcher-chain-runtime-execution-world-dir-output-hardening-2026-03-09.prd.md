# 启动器 chain runtime execution world 输出路径收敛（2026-03-09）

- 对应设计文档: `doc/world-simulator/launcher/game-client-launcher-chain-runtime-execution-world-dir-output-hardening-2026-03-09.design.md`
- 对应项目管理文档: `doc/world-simulator/launcher/game-client-launcher-chain-runtime-execution-world-dir-output-hardening-2026-03-09.project.md`

审计轮次: 6

## 1. Executive Summary
- Problem Statement: 当前 `oasis7_chain_runtime` 的 `explorer-index.json` 持久化路径受 `execution_world_dir` 影响；当启动链路依赖默认工作目录且 cwd 偏离时，运行时文件可能落到源码目录。
- Proposed Solution: 在 `oasis7_game_launcher` 与 `oasis7_web_launcher` 构造 runtime 启动参数时，显式传递 `--execution-world-dir`，统一收敛到 `output/chain-runtime/<node_id>/reward-runtime-execution-world`。
- Success Criteria:
  - SC-1: 启动器托管 runtime 时，参数中稳定包含 `--execution-world-dir`。
  - SC-2: `execution_world_dir` 路径规则固定为 `output/chain-runtime/<node_id>/reward-runtime-execution-world`。
  - SC-3: `explorer-index.json` 等 execution world 持久化文件不再直接落到源码目录。
  - SC-4: `oasis7_game_launcher` 与 `oasis7_web_launcher` 的参数构建单测均锁住该行为，防止回归。

## 2. User Experience & Functionality
- User Personas:
  - 启动器维护者：需要运行时产物路径稳定、可预期，避免污染源码目录。
  - 发布/测试工程师：需要跨入口（CLI/Web 控制面）行为一致，降低环境依赖故障。
- User Scenarios & Frequency:
  - 每次通过 `oasis7_game_launcher` 或 `oasis7_web_launcher` 拉起链运行时都会触发；日常开发和回归高频发生。
- User Stories:
  - PRD-WORLD_SIMULATOR-033: As a 启动器维护者, I want launcher to pass an explicit execution world output path to chain runtime, so that runtime-generated explorer index files always stay under `output/`.
- Critical User Flows:
  1. Flow-LAUNCHER-033-001（CLI 启动链路）:
     `oasis7_game_launcher 解析链参数 -> 构建 oasis7_chain_runtime args -> 附带 --execution-world-dir -> runtime 写 explorer-index.json 到 output/...`
  2. Flow-LAUNCHER-033-002（Web 控制面链路）:
     `oasis7_web_launcher 接收 /api/chain/start -> 构建 oasis7_chain_runtime args -> 附带 --execution-world-dir -> runtime 写 explorer-index.json 到 output/...`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| runtime execution world 路径显式透传 | `chain_node_id`、`--execution-world-dir`、`output/chain-runtime/<node_id>/reward-runtime-execution-world` | 启动器拉起 runtime 时统一附带 `--execution-world-dir` | `launcher_start -> args_built -> runtime_running` | 路径拼接规则固定，不依赖 runtime 默认 cwd | 启动器维护路径可写；用户查询能力不变 |
- Acceptance Criteria:
  - AC-1: `oasis7_game_launcher::build_oasis7_chain_runtime_args` 返回参数必须包含 `--execution-world-dir` 及 `output/chain-runtime/<node_id>/reward-runtime-execution-world`。
  - AC-2: `oasis7_web_launcher::build_chain_runtime_args` 返回参数必须包含 `--execution-world-dir` 及同规则路径。
  - AC-3: 既有链参数（`--node-id`、`--status-bind`、`--storage-profile`、PoS 参数）保持兼容，不得回归。
  - AC-4: 定向测试覆盖两个参数构建函数，且通过 `test_tier_required` 回归。
- Non-Goals:
  - 不在本任务内改造 `oasis7_chain_runtime` 的默认路径解析逻辑。
  - 不在本任务内新增用户可配置的 execution world 路径字段。
  - 不改动 explorer 业务语义与持久化内容结构。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（本任务不涉及 AI 模型能力）。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview:
  - 改动点仅在启动器参数构建层：`oasis7_game_launcher` 与 `oasis7_web_launcher`。
  - runtime 继续消费 CLI 参数，不变更其持久化实现。
- Integration Points:
  - `crates/oasis7/src/bin/oasis7_game_launcher.rs`
  - `crates/oasis7/src/bin/oasis7_game_launcher/oasis7_game_launcher_tests.rs`
  - `crates/oasis7/src/bin/oasis7_web_launcher/control_plane.rs`
  - `crates/oasis7/src/bin/oasis7_web_launcher/oasis7_web_launcher_tests.rs`
  - `doc/world-simulator/prd.md`
  - `doc/world-simulator/project.md`
- Edge Cases & Error Handling:
  - `chain_node_id` 为空：沿用现有校验逻辑阻断启动。
  - 相对路径运行：通过显式 `--execution-world-dir` 固定到 `output/...`，不再落到裸根目录文件。
  - 参数回归：若遗漏 `--execution-world-dir`，由单测直接失败。
- Non-Functional Requirements:
  - NFR-1: 启动器参数构建开销保持 O(1)，不引入额外 I/O。
  - NFR-2: 变更仅影响参数拼接，保持现有 API/协议兼容。
  - NFR-3: 双入口（CLI/Web）路径规则必须一致。
- Security & Privacy:
  - 不新增网络暴露面，不引入额外敏感数据处理。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP: 双启动器显式传 `--execution-world-dir` 并补齐单测。
  - v1.1: 若后续需要，再扩展为可配置路径并保持默认安全值。
  - v2.0: 将运行态目录策略统一沉淀为启动器/脚本共享配置约束。
- Technical Risks:
  - 风险-1: 后续参数重构时遗漏该 flag，导致路径行为回退。
  - 风险-2: 仅修复启动器托管链路，直接手工运行 runtime 仍可能依赖调用方 cwd。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-WORLD_SIMULATOR-033 | TASK-WORLD_SIMULATOR-095/096 | `test_tier_required` | `./scripts/doc-governance-check.sh` + `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_game_launcher oasis7_game_launcher_tests::build_oasis7_chain_runtime_args_includes_storage_profile -- --nocapture` + `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_web_launcher oasis7_web_launcher_tests::build_chain_runtime_args_includes_chain_overrides_when_on -- --nocapture` + `env -u RUSTC_WRAPPER cargo check -p oasis7 --bin oasis7_game_launcher --bin oasis7_web_launcher` | 运行时产物目录可控性、启动器参数透传稳定性、双入口一致性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-LAUNCHER-033-001 | 在启动器层显式传 `--execution-world-dir` | 依赖 runtime 默认路径（随 cwd 变化） | 默认路径对调用环境敏感，容易污染源码目录。 |
| DEC-LAUNCHER-033-002 | 双入口（`oasis7_game_launcher` + `oasis7_web_launcher`）同时收敛 | 仅修复单入口 | 两条入口都可托管 runtime，必须保持一致避免隐式分叉。 |
| DEC-LAUNCHER-033-003 | 路径规则固定为 `output/chain-runtime/<node_id>/reward-runtime-execution-world` | 引入新配置项由用户填写 | 当前目标是消除歧义并快速止损，固定规则更稳妥。 |
