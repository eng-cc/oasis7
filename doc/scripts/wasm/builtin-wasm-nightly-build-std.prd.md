# Builtin Wasm Nightly build-std 可复现构建方案

- 对应设计文档: `doc/scripts/wasm/builtin-wasm-nightly-build-std.design.md`
- 对应项目管理文档: `doc/scripts/wasm/builtin-wasm-nightly-build-std.project.md`

审计轮次: 4

## 当前状态（2026-06-16）
- 本文保留为历史实现证据（historical only; not a release-chain entry），记录 nightly + `-Z build-std` 方案如何收口当时的 builtin wasm 构建输入。
- 当前发布级 canonical build / release evidence 入口已收敛到 `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md`；涉及 Docker builder、build receipt、identity 与 release evidence 的新判断应从该入口开始。
- 本文不得覆盖 Docker-only publishable build、build receipt、single canonical token 或 cross-host evidence requirements；`WDBP-3.2` 的跨宿主 closure 仍以真实 Docker-capable `darwin-arm64` live summary / proof 输入为准。
- 本文仍可用于追溯脚本参数、旧 CI 环境约束和 hash 漂移治理背景，但不再作为 WASM 发布链路的首读入口。

## 目标
- 采用 nightly + `-Z build-std` 重建 wasm 目标 std，实现 builtin wasm 构建输入闭环可控。
- 在保留现有 hash/manifest 校验机制前提下，消除宿主预编译 std 差异导致的 hash 漂移。
- 延续现有路径归一化策略（`--remap-path-prefix`）与 wasm custom section canonicalize，确保 hash 仅反映可执行语义。

## 范围
- In Scope：
  - 固定 builtin wasm 构建工具链到 pinned nightly（含 `rust-src` 与 `wasm32-unknown-unknown`）。
  - 在 wasm 构建调用链启用 `-Z build-std`、`-Z build-std-features`。
  - 更新 CI required/full gate 的 wasm 构建环境变量与组件安装步骤。
  - 重新同步 `m1/m4` hash 清单并回归 `sync --check` 与 required tier。
- Out of Scope：
  - runtime ABI、DistFS 协议、hash 算法与 manifest 文件格式改动。
  - 定义发布级 Docker canonical builder 目标态；当前发布口径由 `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md` 管理。

## 接口 / 数据
- 构建入口：
  - `scripts/build-wasm-module.sh`
  - `scripts/build-builtin-wasm-modules.sh`
- 构建参数（新增/固化）：
  - `OASIS7_WASM_TOOLCHAIN`（默认 `nightly-2025-12-11`）
  - `OASIS7_WASM_BUILD_STD`（默认 `1`）
  - `OASIS7_WASM_BUILD_STD_COMPONENTS`（默认 `std,panic_abort`）
  - `OASIS7_WASM_BUILD_STD_FEATURES`（默认空，不追加 `-Z build-std-features`）
- wasm build suite：
  - `tools/wasm_build_suite/src/lib.rs` 在 cargo build 参数注入 `-Z build-std*`（受环境变量控制）。
- 清单与校验：
  - `crates/oasis7/src/runtime/world/artifacts/m1_builtin_modules.sha256`
  - `crates/oasis7/src/runtime/world/artifacts/m4_builtin_modules.sha256`
  - `scripts/sync-m1-builtin-wasm-artifacts.sh --check`
  - `scripts/sync-m4-builtin-wasm-artifacts.sh --check`

## 里程碑
- M1：文档与任务拆解完成。
- M2：nightly + build-std 在 wasm 构建链路落地并通过本地构建。
- M3：CI required/full 固化 nightly build-std 环境。
- M4：m1/m4 清单同步，`sync --check` 与 required tier 回归通过。

## 风险
- `-Z build-std` 会显著增加首次构建成本（时间与网络下载）。
- nightly 版本升级会影响 hash，需要固定日期并建立升级流程。
- 若 nightly 源发生不可用/回滚，CI 会受影响；需允许显式切换到新 pinned nightly。

## 原文约束点映射（内容保真）
- 约束-1（目标与问题定义）：沿用原“目标”章节约束，不改变问题定义与解决方向。
- 约束-2（范围边界）：沿用原“范围”章节的 In Scope/Out of Scope 语义，不扩散到新增范围。
- 约束-3（接口/里程碑/风险）：沿用原接口字段、阶段节奏与风险口径，并保持可追溯。
