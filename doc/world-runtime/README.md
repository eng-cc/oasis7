# world-runtime 文档索引

审计轮次: 11

## 从这里开始
- 想先理解 runtime 的可信边界、目标态与验收范围：`doc/world-runtime/prd.md`
- 想看当前活跃任务、阻断、测试层级与最新完成项：`doc/world-runtime/project.md`
- 想直接定位某个 runtime / wasm / module / governance 专题文件：`doc/world-runtime/prd.index.md`
- 想先看当前最关键的发布/构建专题：`doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md`
- 想先看运行态体积、恢复与 retention 治理：`doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.prd.md`
- 想先看线上模块发布合法性与 binary-only 边界：`doc/world-runtime/module/online-module-release-legality-closure-2026-03-08.prd.md`

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
- 高频专题文档承担专题真值：`wasm-deterministic-build-pipeline` 负责 Docker canonical build / receipt / release evidence；`runtime-storage-footprint-governance` 负责 retention / GC / replay contract；`online-module-release-legality-closure` 负责线上模块发布合法性与默认安全边界。

## 活跃阅读面边界
- 当前页只保留 `what / where / next / risk` 所需入口，不再直接平铺 runtime 高频专题长名单。
- 高频 active 入口保留在 `prd.md`、`project.md`、`prd.index.md` 与少量仍承担当前跨阶段判断职责的正式专题。
- `evidence/`、`templates/`、`checklists/` 与 handoff 文档继续保留可检索性，但默认从 `prd.index.md` 或具体专题路径按需进入。

## 模块职责
- 维护运行时主链路、存储治理、WASM 执行与模块发布口径。
- 维护 WASM Docker 确定性构建、工件 hash/identity/DistFS 分发与 runtime binary-first 消费口径。
- 汇总 runtime / wasm / module / governance / integration / testing 六类专题。
- 承接候选级证据、发布门禁指标与跨模块 runtime 收口事项。

## 热点子域导航（2026-04-10 快照）
- `runtime/`（55）：运行时主链路、数值正确性、存储治理、retention 与 replay contract。
- `wasm/`（19）：Docker canonical build、执行器、SDK、sandbox 与 ABI 治理。
- `module/`（16）：模块生命周期、线上发布合法性、模块存储与订阅过滤专题。
- 根目录入口与 handoff（9）：模块主入口与 runtime 候选/验证交接留痕。
- `evidence/`（6）：候选级指标、soak、storage gate 与 profile consistency 采证。
- `governance/`（5）：治理事件与收据安全专题。

## 高密度提示
- `doc/world-runtime/` 当前共有 115 份文件，其中 `runtime/` 占 55 份；默认入口不再尝试把 runtime/wasm/module 长表直接摊平展示。
- 需要完整活跃专题清单时，进入 `doc/world-runtime/prd.index.md`；需要 evidence / template / checklist / handoff 时，再按子域定向进入。

## 兼容跳转
- `doc/world-runtime.prd.md`
- `doc/world-runtime.project.md`

上述两个根目录文件仅保留为最小兼容跳转；当前主入口以本目录 `prd.md` / `project.md` 为准。

## 共享约定
- 模块根入口、专题落位与 README/legacy redirect 的共享规则统一以 `doc/engineering/doc-governance/doc-structure-standard.design.md` 为准。
- runtime 行为、发布门禁或高频专题入口变化时，优先更新 `doc/world-runtime/prd.md` / `doc/world-runtime/project.md`；新增专题后，再按需回写 `doc/world-runtime/prd.index.md` 与本目录“从这里开始”。
