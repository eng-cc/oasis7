# Builtin Wasm Nightly build-std 可复现构建（项目管理文档）

- 对应设计文档: `doc/scripts/wasm/builtin-wasm-nightly-build-std.design.md`
- 对应需求文档: `doc/scripts/wasm/builtin-wasm-nightly-build-std.prd.md`

审计轮次: 4

## 当前状态（2026-06-16）
- 本项目文档已完成并保留为历史执行证据（historical only; not a release-chain entry）。
- 当前 WASM 发布级 canonical build / release evidence 的项目状态入口为 `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.project.md`。
- 本项目文档不得覆盖 Docker-only publishable build、build receipt、single canonical token 或 cross-host evidence requirements；跨宿主 full closure 仍需真实 Docker-capable `darwin-arm64` live summary / proof。
- 若后续需要继续治理 build-std、Docker builder、receipt 或 cross-host evidence，请在 world-runtime WASM canonical pipeline 下新建/绑定任务，不要把本文恢复为活跃项目入口。

## 任务拆解
- [x] NBS-1 设计文档：`doc/scripts/wasm/builtin-wasm-nightly-build-std.prd.md`
- [x] NBS-2 项目管理文档：本文件
- [x] NBS-3 `scripts/build-wasm-module.sh` 固化 nightly toolchain + rust-src/wasm target 准备
- [x] NBS-4 `tools/wasm_build_suite` 接入 `-Z build-std` / `-Z build-std-features`
- [x] NBS-5 CI workflow 固化 nightly build-std 环境变量与组件安装
- [x] NBS-6 重新同步 m1/m4 hash 清单并通过 `sync --check`
- [x] NBS-7 required tier 回归通过（`CI_VERBOSE=1 ./scripts/ci-tests.sh required`）
- [x] NBS-8 更新 devlog、收口文档并提交

## 依赖
- pinned nightly：`nightly-2025-12-11`
- target：`wasm32-unknown-unknown`
- rustup component：`rust-src`

## 状态
- 当前阶段：已完成
- 最近更新：NBS-1~NBS-8 完成（2026-02-17）

## 迁移记录（2026-03-03）
- 已按 `TASK-ENGINEERING-014-D1 (PRD-ENGINEERING-006)` 从 legacy 命名迁移为 `.prd.md/.project.md`。
- 保留原任务拆解、依赖与状态语义，不改变既有结论。
