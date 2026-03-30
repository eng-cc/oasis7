# world-runtime 文档索引

审计轮次: 10

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

## 模块职责
- 维护运行时主链路、存储治理、WASM 执行与模块发布口径。
- 维护 WASM Docker 确定性构建、工件 hash/identity/DistFS 分发与 runtime binary-first 消费口径。
- 汇总 runtime / wasm / module / governance / integration / testing 六类专题。
- 承接候选级证据、发布门禁指标与跨模块 runtime 收口事项。

## 主题文档
- `runtime/`：运行时主链路、数值正确性、存储治理与长稳专题。
- `wasm/`：WASM 接口、执行器、SDK 与沙箱治理。
- `module/`：模块生命周期、发布合法性、存储与订阅过滤专题。
- `governance/`：治理、审计与收据安全专题。
- `integration/`：跨模块桥接专题。
- `testing/`：运行时专用测试分册。

## 近期专题
- `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md`
- `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.prd.md`
- `doc/world-runtime/module/online-module-release-legality-closure-2026-03-08.prd.md`
- `doc/world-runtime/module/player-published-entities-2026-03-05.prd.md`
- `doc/world-runtime/governance/zero-trust-governance-receipt-hardening-2026-02-26.prd.md`
- `doc/world-runtime/wasm/wasm-agent-os-alignment-hardening.prd.md`

## 根目录收口
- 模块根目录主入口保留：`README.md`、`prd.md`、`design.md`、`project.md`、`prd.index.md`。
- 其余专题文档按主题下沉到 `runtime/`、`wasm/`、`module/`、`governance/`、`integration/`、`testing/`。

## 根目录 legacy
- `doc/world-runtime.prd.md`
- `doc/world-runtime.project.md`

上述两个根目录文件仅保留为兼容跳转入口；当前主入口以本目录 `prd.md` / `project.md` 为准。

## 维护约定
- runtime 行为、发布门禁或候选级证据口径变化时，优先回写 `doc/world-runtime/prd.md`。
- WASM Docker builder image、canonicalizer、hash/identity manifest 或 source compile 边界变化时，优先回写 `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md`。
- 新增专题后，需同步回写 `doc/world-runtime/prd.index.md` 与本目录索引。
- 若高频专题切换，需同步更新本目录“从这里开始”，避免 README 退化为只剩主题目录的纯列表页。
