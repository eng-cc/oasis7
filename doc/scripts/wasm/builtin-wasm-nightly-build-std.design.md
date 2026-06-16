# Builtin Wasm Nightly build-std 可复现构建方案设计

- 对应需求文档: `doc/scripts/wasm/builtin-wasm-nightly-build-std.prd.md`
- 对应项目管理文档: `doc/scripts/wasm/builtin-wasm-nightly-build-std.project.md`

## 当前状态（2026-06-16）
- 本设计保留为历史实现证据（historical only; not a release-chain entry），用于解释 nightly build-std 脚本切片的设计边界。
- 当前 WASM 发布级 canonical build / release evidence 设计入口为 `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.design.md`。
- 本设计不得覆盖 Docker-only publishable build、build receipt、single canonical token 或 cross-host evidence requirements。
- 新的构建链路设计评审应优先进入 world-runtime WASM canonical pipeline，再按需回查本文。

## 1. 设计定位
定义 WASM 构建脚本专题设计，统一 nightly build-std、可复现构建与环境约束。

## 2. 设计结构
- 构建入口层：定义 build-std 脚本、参数与目标环境。
- 可复现层：约束 nightly、依赖与产物一致性。
- 失败诊断层：为环境缺失、构建漂移与 target 问题提供诊断。
- 维护回写层：沉淀脚本使用边界与后续演进。

## 3. 关键接口 / 入口
- WASM 构建脚本入口
- nightly/build-std 环境约束
- 可复现产物检查
- 构建失败诊断说明

## 4. 约束与边界
- 脚本必须服务可复现构建目标。
- 环境约束需明确且可操作。
- 不在本专题扩展新的构建系统。

## 5. 设计演进计划
- 先固定构建入口与环境。
- 再补可复现与诊断逻辑。
- 最后固化使用说明。
