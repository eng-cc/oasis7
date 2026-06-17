# oasis7: Builtin Wasm 构建确定性护栏

- 对应设计文档: `doc/testing/governance/wasm-build-determinism-guard.design.md`
- 对应项目管理文档: `doc/testing/governance/wasm-build-determinism-guard.project.md`

审计轮次: 4

## 当前状态（2026-06-16）
- 本文保留为历史 QA gate substrate，记录 builtin wasm 构建确定性护栏、污染环境变量拦截和 workspace 编译期风险前置。
- 当前 WASM 发布级 canonical build / release evidence 入口为 `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md`；本文不替代 Docker canonical build、release evidence 或 cross-host closure 的当前口径。
- 需要判断 required/full 验证归属时，继续以 `doc/testing/prd.md`、`doc/testing/project.md`、`testing-manual.md` 和当前 `.pm` task truth 为准；本文仅作为该护栏切片的历史证据。
- 不得宣称 cross-host/full closure，除非 Linux 加真实 Docker-capable `darwin-arm64` evidence 以及 node-side proof/attestation path 均已归档。

## 1. Executive Summary
- Problem Statement: builtin wasm 在不同执行环境下可能因构建输入不一致产生 hash 漂移，导致本地与 CI 结果不一致且难以定位。
- Proposed Solution: 在 wasm 构建入口与构建工具层引入确定性护栏，统一关键输入、限制污染环境变量，并在 workspace 级阻断高风险编译期目标。
- Success Criteria:
  - SC-1: `scripts/build-wasm-module.sh` 默认开启确定性护栏，并支持显式关闭用于本地实验。
  - SC-2: 构建链路默认使用 `--locked`，lockfile 不一致时直接失败并可定位。
  - SC-3: workspace 本地包出现 `build.rs` 或 `proc-macro` 目标时可被编译期拦截并输出包级错误信息。
  - SC-4: m1/m4 hash manifest 与 DistFS 加载机制保持兼容，不引入协议级变更。

## 2. User Experience & Functionality
- User Personas:
  - CI 维护者：需要稳定可复现的 wasm 构建输入。
  - 平台工程师：需要在编译前就发现不可确定性风险。
  - 发布负责人：需要 hash 产物可追溯且可审计。
- User Scenarios & Frequency:
  - PR 门禁执行：每次涉及 wasm 构建链路变更时触发。
  - 日常本地构建：开发调试时执行并快速发现环境污染。
  - 发布前核验：每个发布候选至少执行一轮确定性检查。
- User Stories:
  - PRD-TESTING-GOV-WASMDET-001: As a CI 维护者, I want canonical wasm build inputs enforced by default, so that runner differences do not alter output hashes.
  - PRD-TESTING-GOV-WASMDET-002: As a 平台工程师, I want compile-time workspace guards for `build.rs` and `proc-macro`, so that non-deterministic risks are blocked early.
  - PRD-TESTING-GOV-WASMDET-003: As a 发布负责人, I want deterministic guard failures to be diagnosable, so that release decisions remain auditable.
- Critical User Flows:
  1. Flow-WASMDET-001: `执行 build-wasm-module.sh -> 读取护栏开关 -> 规范化输入与环境 -> 调用构建工具`
  2. Flow-WASMDET-002: `wasm_build_suite 读取 metadata -> 校验 workspace 编译期目标 -> 通过后执行 --locked 构建`
  3. Flow-WASMDET-003: `命中护栏失败 -> 输出 package/变量级错误信息 -> 维护者修复后重试`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 入口脚本护栏 | 开关、toolchain、target、build-std、环境变量名单 | 执行脚本时默认开启护栏，可显式关闭 | `pending -> guarded -> building -> passed/failed` | 默认启用确定性规则，实验场景允许 opt-out | CI/发布流程默认不可绕过，本地允许显式关闭 |
| 环境变量拦截 | `RUSTFLAGS`、`CARGO_ENCODED_RUSTFLAGS`、`CARGO_TARGET_DIR` 等 | 命中污染变量时中断构建并报错 | `scanning -> blocked/clean` | 按阻断优先级先检查高风险变量 | 构建维护者可调整拦截列表 |
| workspace 编译期拦截 | package 名称、target kind、来源类型 | 遇到本地 `build.rs` 或 `proc-macro` 即失败 | `metadata-loaded -> validated/blocked` | 仅对 workspace 本地包生效，third-party 不一刀切 | 平台维护者拥有放行策略定义权限 |
| 可复现环境固定 | `CARGO_INCREMENTAL=0`、`SOURCE_DATE_EPOCH`、`TZ/LANG/LC_ALL` | 构建前统一注入/覆盖复现参数 | `unset -> normalized` | 先标准化环境再进入构建 | 脚本维护者可更新固定策略 |
- Acceptance Criteria:
  - AC-1: 脚本层默认启用 `OASIS7_WASM_DETERMINISTIC_GUARD=1` 并支持显式关闭。
  - AC-2: 构建工具层 `cargo metadata` 与 `cargo build` 默认带 `--locked`。
  - AC-3: workspace 本地包若含 `custom-build(build.rs)` 或 `proc-macro` 目标，构建被阻断且错误可定位。
  - AC-4: 现有 m1/m4 hash manifest 与 DistFS 加载链路行为不变。
  - AC-5: 变更附带单元测试与最小回归验证记录。
- Non-Goals:
  - 不替换 wasm canonicalize 算法。
  - 不对 third-party 依赖做全面 `build.rs/proc-macro` 封禁。
  - 不调整 runtime builtin wasm loader 协议与 hash 清单格式。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（本专题聚焦构建确定性治理，不涉及 AI 推理链路）。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 通过“入口脚本护栏 + wasm_build_suite 编译期校验”双层机制，将确定性约束前置到构建启动前与 metadata 解析阶段。
- Integration Points:
  - `scripts/build-wasm-module.sh`
  - `tools/wasm_build_suite/src/lib.rs`
  - `scripts/ci-tests.sh`
  - `scripts/sync-m1-builtin-wasm-artifacts.sh --check`
  - `scripts/sync-m4-builtin-wasm-artifacts.sh --check`
- Edge Cases & Error Handling:
  - 本地实验需要自定义 `RUSTFLAGS`：允许显式关闭护栏，默认路径保持阻断。
  - lockfile 未同步：`--locked` 失败并直接提示同步 lockfile。
  - workspace 新增 `build.rs/proc-macro`：编译期立即拦截并输出 package 名称。
  - CI 与本地环境差异：以脚本标准化后的环境变量与日志为准。
  - 构建参数遗漏：缺失 canonical 参数时脚本层直接失败，避免进入后续流程。
- Non-Functional Requirements:
  - NFR-WASMDET-1: 同一 commit 在标准化环境下的 wasm 构建结果可复现。
  - NFR-WASMDET-2: 护栏失败日志需在一次构建内定位到变量或 package。
  - NFR-WASMDET-3: 护栏机制不改变 DistFS 与 hash manifest 消费协议。
  - NFR-WASMDET-4: required 门禁可在不放宽策略的前提下稳定执行。
- Security & Privacy: 仅处理构建参数与包元数据，不引入额外敏感信息采集。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (WASMDET-1): 完成设计/项目文档，明确护栏边界。
  - v1.1 (WASMDET-2): 落地入口脚本 canonical 输入和环境变量拦截。
  - v2.0 (WASMDET-3): 在 `wasm_build_suite` 增加 `--locked` 与 workspace 编译期拦截，补齐测试回归。
- Technical Risks:
  - 风险-1: 更严格护栏会阻断历史本地流程，需要团队适配显式开关策略。
  - 风险-2: workspace 编译期限制可能影响未来模块设计灵活性，需在评审阶段提前约束。
  - 风险-3: `--locked` 提升稳定性的同时增加 lockfile 维护频率。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-TESTING-GOV-WASMDET-001 | WASMDET-1/2 | `test_tier_required` | 脚本护栏开关与 canonical 输入检查 | wasm 构建入口稳定性 |
| PRD-TESTING-GOV-WASMDET-002 | WASMDET-2/3 | `test_tier_required` | workspace `build.rs/proc-macro` 拦截单测与构建失败签名验证 | 构建确定性与风险前置 |
| PRD-TESTING-GOV-WASMDET-003 | WASMDET-3 | `test_tier_required` | required 门禁与工件检查脚本回归 | 发布门禁可信度 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-WASMDET-001 | 默认开启脚本护栏，允许显式关闭用于实验 | 默认关闭、仅 CI 启用 | 默认开启可减少遗漏，显式关闭保留开发弹性。 |
| DEC-WASMDET-002 | metadata/build 均加 `--locked` | 仅构建阶段加锁 | metadata 与 build 双阶段一致能更早失败并提升可追溯性。 |
| DEC-WASMDET-003 | 仅拦截 workspace 本地包的 `build.rs/proc-macro` | 全量包含 third-party 一刀切拦截 | 控制影响范围，避免对外部依赖生态造成不可控阻断。 |

## 原文约束点映射（内容保真）
- 原“目标（输入一致/编译期拦截/保持 loader 与 manifest 不变）” -> 第 1 章 Problem/Solution/SC。
- 原“In Scope/Out of Scope” -> 第 2 章 AC 与 Non-Goals。
- 原“接口/数据（脚本开关、工具开关、失败信息）” -> 第 2 章规格矩阵 + 第 4 章 Integration。
- 原“里程碑 M1~M4” -> 第 5 章 Phased Rollout（WASMDET-1/2/3）。
- 原“风险（流程阻断、设计自由度、lockfile 门槛）” -> 第 5 章 Technical Risks。
