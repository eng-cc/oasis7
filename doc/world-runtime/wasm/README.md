# WASM 平台文档入口

本目录定义 oasis7 的 WASM 平台稳定契约：ABI 与 executor、安全边界、工件
identity/hash、发布构建、SDK wire 兼容和本地观测。当前任务、状态、执行
历史、CI run 与验收过程只从对应 GitHub task issue / Project 和 Git history
追溯。

## 按问题阅读

| 问题 | 权威文档 |
| --- | --- |
| ABI、模块上下文、Canonical CBOR、生命周期与限制 | [wasm-interface.md](wasm-interface.md)、[wasm-executor.prd.md](wasm-executor.prd.md) |
| sandbox、fuel/memory、结构化失败与 compiled cache 兼容 | [wasm-executor.design.md](wasm-executor.design.md) |
| Docker canonical build、receipt、identity 和 release proof | [wasm-deterministic-build-pipeline.prd.md](wasm-deterministic-build-pipeline.prd.md) |
| build/executor/router timing 与 bounded status/summary | [wasm-observability-timing-metrics.prd.md](wasm-observability-timing-metrics.prd.md) |
| SDK `no_std`、共享 wire 与 codec 兼容 | [wasm-sdk.prd.md](wasm-sdk.prd.md) |
| 可重复验证命令、实现锚点和证据边界 | [evidence.md](evidence.md) |

## 文档边界

- PRD 说明稳定需求、兼容性与验收契约；design 说明实现结构与演进约束。
- `evidence.md` 只保留可重复验证的实现锚点和证据边界，不记录可变任务台账。
- 模块安装、升级、禁用和实例持久化由 `doc/world-runtime/module/` 的生命周期
  权威定义；世界规则、Agent 策略、Viewer 交互和节点部署不由本目录定义。
- 破坏性 ABI、运行时权限或共识格式变更必须经 TPM 组织的跨角色评审，并提供
  显式迁移方案。
