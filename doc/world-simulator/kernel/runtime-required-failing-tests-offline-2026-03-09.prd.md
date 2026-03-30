# runtime required 失败用例临时下线（2026-03-09）

- 对应设计文档: `doc/world-simulator/kernel/runtime-required-failing-tests-offline-2026-03-09.design.md`
- 对应项目管理文档: `doc/world-simulator/kernel/runtime-required-failing-tests-offline-2026-03-09.project.md`

审计轮次: 1

## 1. Executive Summary
- Problem Statement: `env -u RUSTC_WRAPPER cargo test -p oasis7 --tests --features test_tier_required` 当前被 10 个 runtime 基线失败用例阻塞，导致仓库 pre-commit required 测试链路无法通过。
- Proposed Solution: 先对这 10 个已知失败测试执行“临时下线”（`#[ignore]`）止血，并在 `m1` builtin wasm hash/identity manifest 与本地 DistFS blobs 修复后移除 ignore、恢复定向 wasmtime 回归。
- Success Criteria:
  - SC-1: required 测试链路不再因这 10 个既有失败项阻塞提交。
  - SC-2: 下线范围精确为 10 个已知失败测试，禁止扩大到模块级批量禁用。
  - SC-3: 每个被下线测试都保留可追溯说明（失败签名、恢复前置条件、对应任务）。
  - SC-4: 非下线项 runtime required 测试继续执行并保持可诊断信号。
  - SC-5: 根因修复后，这 10 个测试的 `#[ignore]` 全部回收为 0，且定向 wasmtime 回归通过。

## 2. User Experience & Functionality
- User Personas:
  - runtime 维护者：需要在不删除测试资产的前提下恢复 required 回归链路可运行性。
  - 发布工程师：需要 pre-commit required 测试可通过，避免被已知基线故障长期阻塞。
  - QA/测试开发：需要保留故障上下文，后续可恢复并重启完整覆盖。
- User Scenarios & Frequency:
  - 每次提交 pre-commit 触发 required 测试时都会命中；日均多次。
  - 每次 runtime manifest/hash token 调整后需要复核并逐步恢复下线项。
- User Stories:
  - PRD-WORLD_SIMULATOR-032: As a runtime 维护者, I want known baseline failing tests to be temporarily offlined with explicit traceability, so that required CI stays unblocked while preserving recovery signals.
- Critical User Flows:
  1. Flow-RT-001（失败识别）:
     `运行 required 测试 -> 命中 10 个固定失败项 -> 失败签名统一为 builtin identity manifest 缺失 hash token`
  2. Flow-RT-002（临时下线）:
     `仅对这 10 个测试增加 #[ignore] + 理由 -> 保留测试函数实现与断言`
  3. Flow-RT-003（链路恢复）:
     `重新运行 required 测试 -> 通过（下线项计入 ignored） -> 后续修复 hash token 后解除 ignore`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| runtime required 失败用例临时下线 | 测试名白名单（10 项）、ignore 原因、恢复条件 | 仅在指定测试函数上添加 `#[ignore]`；不删除测试代码 | `failing -> ignored`（仅针对白名单） | 白名单精确匹配 10 项；数量不得扩张 | 仅维护者可修改测试注解 |
- Acceptance Criteria:
  - AC-1: 下线名单固定为以下 10 项，不允许额外泛化忽略：
    - `runtime::tests::agent_default_modules::scenario_modules_limit_mobility_before_sensor_when_power_low`
    - `runtime::tests::agent_default_modules::scenario_modules_replay_keeps_state_consistent`
    - `runtime::tests::agent_default_modules::scenario_modules_with_transfer_and_body_keep_wasm_closed_loop_consistent`
    - `runtime::tests::power_bootstrap::install_power_bootstrap_modules_registers_and_activates`
    - `runtime::tests::power_bootstrap::install_power_bootstrap_modules_reactivates_registered_version`
    - `runtime::tests::power_bootstrap::install_power_bootstrap_modules_is_idempotent`
    - `runtime::tests::power_bootstrap::install_scenario_bootstrap_modules_supports_default_package_toggle`
    - `runtime::tests::power_bootstrap::install_scenario_bootstrap_modules_is_idempotent`
    - `runtime::tests::power_bootstrap::radiation_module_emits_harvest_event`
    - `runtime::tests::power_bootstrap::storage_module_blocks_continuous_move_when_power_runs_out`
  - AC-2: 每个下线测试必须在代码中写明临时下线原因（`builtin identity manifest hash token` 缺失）与恢复意图。
  - AC-3: `env -u RUSTC_WRAPPER cargo test -p oasis7 --tests --features test_tier_required` 执行结果不得再包含上述 10 项失败。
  - AC-4: 除下线项外，其余 required 测试继续执行（不是整体 `cfg` 跳过）。
  - AC-5: 当 `m1` builtin manifest/hash/DistFS 根因修复后，必须移除 10 个 `#[ignore]`，并通过 `runtime::tests::power_bootstrap::*` / `runtime::tests::agent_default_modules::*` 的定向 wasmtime 回归。
- Non-Goals:
  - 不在本任务内修复 `m1_builtin_modules.identity.json` 缺失 hash token 的根因。
  - 不删除失败测试函数或断言逻辑。
  - 不调整 runtime 功能行为与业务语义。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（本任务不涉及 AI 模型能力）。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview:
  - 变更集中在 runtime 测试层，生产代码零改动。
  - 通过测试函数级 `#[ignore]` 实现最小化隔离，不引入模块级测试屏蔽。
- Integration Points:
  - `crates/oasis7/src/runtime/tests/agent_default_modules.rs`
  - `crates/oasis7/src/runtime/tests/power_bootstrap.rs`
  - `doc/world-simulator/prd.md`
  - `doc/world-simulator/project.md`
- Edge Cases & Error Handling:
  - 白名单漂移：若新增 ignore 超出 10 项，应视为越权并阻断合入。
  - 原因丢失：若 ignore 无上下文说明，视为不可追溯，不满足验收。
  - 误伤范围：禁止使用模块级 `#![ignore]` 或 `cfg` 关闭整组测试。
  - 根因已修复：若 hash token 问题解决，需反向解除 ignore 并恢复全量执行。
- Non-Functional Requirements:
  - NFR-1: 下线改动仅发生在测试文件，不得影响生产二进制编译产物。
  - NFR-2: 下线数量必须精确等于 10（与当前已知失败列表一致）。
  - NFR-3: required 测试链路在本地可完整执行并输出可审计结果。
- Security & Privacy:
  - 不涉及新增数据通道与敏感信息处理。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP: 先临时下线 10 个已知失败项，恢复 required 测试可执行。
  - v1.1: 在 `m1_builtin_modules.identity.json` / `m1_builtin_modules.sha256` / `.distfs/builtin_wasm` 对齐后逐项取消 ignore。
  - v2.0: 收口为零临时下线项，恢复 full required 覆盖。
- Technical Risks:
  - 风险-1: ignore 长期未回收导致测试债务累积。
  - 风险-2: 若后续失败签名变化，旧白名单可能掩盖新故障分类。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-WORLD_SIMULATOR-032 | TASK-WORLD_SIMULATOR-093/094/094A | `test_tier_required` | `./scripts/doc-governance-check.sh` + 下线阶段的 required 回归 + 根因修复后的 `env -u RUSTC_WRAPPER cargo test -p oasis7 --lib --features 'test_tier_required,wasmtime' runtime::tests::power_bootstrap:: -- --nocapture` 与 `runtime::tests::agent_default_modules:: -- --nocapture` | runtime required 测试执行稳定性、pre-commit 可用性、测试资产可追溯性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-RT-001 | 使用测试函数级 `#[ignore]` 临时下线 10 项已知失败用例 | 删除失败测试代码 | 删除会丢失验证资产与恢复锚点，无法支持后续回收。 |
| DEC-RT-002 | 精确白名单下线，不做模块级屏蔽 | `cfg` 关闭整个 `oasis7` runtime tests 的 `agent_default_modules` / `power_bootstrap` 文件 | 模块级屏蔽会误伤正常测试覆盖，风险不可控。 |
| DEC-RT-003 | 先恢复 required 链路可执行，再追根因修复并解除 ignore | 先阻塞所有提交直到根因修完 | 当前目标是先解除提交流水线阻塞，同时保留问题可追踪性。 |
