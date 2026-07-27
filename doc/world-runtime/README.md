# world-runtime 文档索引

产品层归属：`world-runtime` 是“大世界基础设施”的世界执行专业域权威，不形成并列产品入口。产品总导航见 `doc/product/README.md`，组合承诺见 `doc/product/world-infrastructure/prd.md`。

审计轮次: 11

## 从这里开始
- 想先理解 runtime 的可信边界、目标态与验收范围：`doc/world-runtime/prd.md`
- 想看当前活跃任务、阻断、测试层级与最新完成项：`doc/world-runtime/project.md`
- 想直接定位某个 runtime / wasm / module / governance 专题文件：`doc/world-runtime/prd.index.md`
- 想先看当前最关键的发布/构建专题：`doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md`
- 想看 WASM 全局 timing/status/window，或给单个模块补标准化 contract/perf 观测：`doc/world-runtime/wasm/wasm-observability-timing-metrics.prd.md`
- 想看模块 SDK 的 no_std、共享 wire 与 codec 兼容契约：`doc/world-runtime/wasm/wasm-sdk.prd.md`
- 想先看运行态体积、恢复与 retention 治理：`doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.prd.md`
- 想先看线上模块发布合法性与 binary-only 边界：`doc/world-runtime/module/online-module-release-legality-closure-2026-03-08.prd.md`
- 想进入治理事件、审计导出或收据安全专题：`doc/world-runtime/governance/README.md`

## 入口
- PRD: `doc/world-runtime/prd.md`
- 设计总览: `doc/world-runtime/design.md`
- 标准执行入口: `doc/world-runtime/project.md`
- 文件级索引: `doc/world-runtime/prd.index.md`

## 入口分工
- `README.md` 只承担 landing page 职责：帮助读者决定先去 PRD、Project、长表索引还是具体高频专题。
- `prd.md` 是模块权威规格入口，适合先理解 runtime 的确定性、WASM、治理、存储与发布边界。
- `project.md` 是执行台账，适合确认当前仍在推进的 runtime / wasm 发布 / binary-only / traceability 任务。
- `prd.index.md` 是精确检索索引，适合已经知道专题名后按文件名直达，不适合作为第一次进入模块时的首读入口。
- 高频专题文档承担专题真值：`wasm-deterministic-build-pipeline` 负责 Docker canonical build / receipt / release evidence；`wasm-observability-timing-metrics` 同时负责全局 timing/status/window 与 module-local spec/runner/template；`wasm-sdk` 负责 no_std、共享 wire 与 codec 兼容；`runtime-storage-footprint-governance` 负责 retention / GC / replay contract；`online-module-release-legality-closure` 负责线上模块发布合法性与默认安全边界。

## 活跃阅读面边界
- 当前页只保留 `what / where / next / risk` 所需入口，不再直接平铺 runtime 高频专题长名单。
- 高频 active 入口保留在 `prd.md`、`project.md`、`prd.index.md` 与少量仍承担当前跨阶段判断职责的正式专题。
- `evidence/`、`templates/` 与 `checklists/` 继续保留可检索性，但默认从 `prd.index.md` 或具体专题路径按需进入；旧 2026-03 runtime handoff root 文档已退役删除，当前追溯改从正式 evidence 与专题 project 进入。

## 模块职责
- 维护运行时主链路、存储治理、WASM 执行与模块发布口径。
- 维护 WASM Docker 确定性构建、工件 hash/identity/DistFS 分发与 runtime binary-first 消费口径。
- 汇总 runtime / wasm / module / governance / integration / testing 六类专题。
- 承接候选级证据、发布门禁指标与跨模块 runtime 收口事项。

## 按子域进入
- 运行时主链路、数值正确性、存储治理、retention 与 replay contract：`runtime/`。
- Docker canonical build、执行器、观测、SDK、sandbox 与 ABI 治理：`wasm/`。
- 模块生命周期、线上发布合法性、存储与订阅过滤：`module/`。
- 治理事件、审计导出与收据安全：`governance/README.md`。
- 候选级指标、soak、storage gate 与 profile consistency 采证：`evidence/`。

本页不维护容易漂移的文件数量快照或子域长表。需要完整活跃专题清单时，进入 `doc/world-runtime/prd.index.md`；需要当前模块库存与热点二级目录概览时，运行 `./scripts/doc-inventory-report.sh`。

## 历史根入口
- root world-runtime PRD/project legacy redirect shells 已删除。
- 旧 2026-03 runtime P0 candidate / T7.2 / T7.3 / T7.4 role handoff root 文档已退役删除；当前 runtime candidate、storage cadence、GC fail-safe 与 profile consistency 证据以 `evidence/`、runtime storage topic project 与 GitHub task issue evidence comments 为准。
- 当前主入口以本目录 `prd.md` / `project.md` 为准。

## 共享约定
- 模块根入口、专题落位与 README/legacy redirect 的共享规则统一以 `doc/engineering/doc-governance/doc-structure-standard.design.md` 为准。
- runtime 行为、发布门禁或高频专题入口变化时，优先更新 `doc/world-runtime/prd.md` / `doc/world-runtime/project.md`；新增专题后，再按需回写 `doc/world-runtime/prd.index.md` 与本目录“从这里开始”。
