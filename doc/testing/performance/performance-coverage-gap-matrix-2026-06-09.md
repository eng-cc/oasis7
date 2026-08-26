# oasis7：性能测试补测矩阵（2026-06-09）

## 目的
- 基于当前仓库的现有功能面与已接入测试，整理“哪些性能测试已经有”“哪些地方更值得补”“适合进哪个测试层级”。
- 本表回答的是“性能测试该补到哪里最划算”，不是要求“每个功能都补一套独立性能测试”。

## 使用原则
- 性能测试优先覆盖高频路径、长时路径、发布敏感路径和已知易退化热点，不要求覆盖所有功能。
- 功能正确性仍主要由 `commit / required / full`、Web 闭环、契约测试和发布 gate 承担；性能测试负责补“慢了、卡了、抖了、长跑崩了”这类风险。
- 建议 tier 含义：
  - `required-scoped`: 仅在 changed-path 命中时进入默认 PR gate
  - `full-scoped`: 仅在较重改动或 full-tier 抽样时进入
  - `release`: 发布或高风险合流前必须覆盖
  - `on-demand`: 默认不进 PR gate，按改动面或故障排查补跑

## 补测矩阵
| Surface | 当前已有覆盖 | 当前缺口 | 建议补充 | 建议 tier | 优先级 |
| --- | --- | --- | --- | --- | --- |
| Viewer Web 帧率 / frame time | `./scripts/viewer-performance-probe.sh --profile smoke|release`；`test:perf-harness` 覆盖当前浏览器指标 contract；`testing-manual.md` / `scripts/ci-tests.sh` 已将 `smoke` profile 接入 `crates/oasis7_viewer/**` 命中时的 required-gate report-only scoped smoke。当前 Web probe 拥有浏览器 `summary.json/summary.md`、renderer ready/fallback、FPS/frame p95/long-task 等用户面观测，不替代已退役 native `RenderPerfSummary` / `PerfHotspot` / runtime-stage / old stress CSV 诊断。 | 当前缺口不再是“没有 scoped gate 入口”，而是阈值、环境噪音和 blocking rollout 尚未稳定；report-only failure 会被记录为 warning，不阻断 required-gate | 继续收集 `summary.json` / `summary.md`，稳定 `readyMs`、FPS、frame p95、long task count 的基线与噪音边界，再决定是否从 report-only 升级为 blocking；新增 runtime hotspot 语义需由 runtime authority 单独定义 | `required-scoped` report-only | P1 |
| Viewer headed Web 真环境渲染 | 有 `agent-browser` 闭环、software-safe contract、build/UI tests | 当前缺少“真实 headed + GPU 路径”的轻量性能回归，容易只测 DOM/contract 不测渲染体感 | 补一条 headed smoke profile，记录 `readyMs`、FPS、frame p95、long task count，并把 SwiftShader/软件渲染继续当环境阻断处理 | `full-scoped` | P2 |
| Viewer 移动端 / 窄屏性能 | 现有手册强调 desktop/mobile 截图与 visual review，但性能 probe 主要围绕当前默认 Viewer Web 入口 | 缺少移动布局下 DOM 规模、ready time、交互响应的独立预算 | 给 `viewer-performance-probe.mjs` 增加 mobile viewport profile，至少落 `readyMs + frame p95 + DOM size` | `on-demand`，稳定后可升 `required-scoped` | P2 |
| Runtime tick / step 热路径 | `RuntimePerfSnapshot` 保留 `tick/decision/action_execution/callback` 四个本地阶段及 512-sample window，输出 samples、budget、last/avg/min/max/p50/p95/p99 与累计 over-budget；默认预算分别为 33/20/20/10 ms。`test_tier_required/full` 覆盖正确性；`llm-longrun-stress.sh` 聚合 tick p95/over-budget；`runtime-module-routing-perf-harness.sh` 覆盖稳定内环。serde default 保持旧 snapshot/report 兼容。 | 现阶段轻量 harness 只覆盖 runtime module routing 内环，尚未扩到更完整 tick/step 预算；冷 `release` 编译耗时仍偏高 | 下一步把更多稳定热路径收进同一 summary/checklist，并评估哪些指标适合变成 scoped gate；schema/consumer 改动须由 runtime owner 做定向 S3 | `required-scoped`（仅限稳定 harness 命中路径）候选 | P1 |
| Runtime 长时稳定性 | `llm-longrun-stress.sh` 聚合 runtime perf health、bottleneck、tick p95/over-budget。health/bottleneck 只使用四个本地阶段：无本地 sample 为 unknown；p95 超预算或 over-budget >= 50,000 ppm 为 warn；p95 > 2×预算或 >= 200,000 ppm 为 critical；bottleneck 取 sampled local stage 中最大 p95。 | 仍偏长窗和按需使用，不适合普通 PR 快速暴露持续退化；这些是诊断/longrun evidence，不是默认 PR latency gate | 增加一个更短的 smoke profile，只盯关键 perf health 指标；在基线与噪音稳定前保持 `full-scoped` | `full-scoped` | P2 |
| Rust 编译效率 / 依赖闭包 / 包体积 | `compile-metrics.yml` + `ci-compile-metrics.sh` 已测 launcher 的 package closure、冷 `cargo check`、冷 `cargo build --release`、binary size；并可选择 `oasis7_node` 默认/无默认特性与 `oasis7` 默认特性的冷 `cargo check`，launcher gate `wasmtime` 不得漏入 | 三平台 current+baseline 冷编译成本高，且目前只覆盖 launcher、node 与 oasis7 三个 surface；其他关键 crate 的编译膨胀没有统一基线 | 将 compile metrics 保持为 `workflow_dispatch` 手动触发；按依赖/包体/编译性能风险或故障排查补跑，后续若要恢复默认 gate 需先做更窄 changed-path 触发 | launcher、node 与 oasis7 均先 `on-demand` | P1 |
| Launcher 运行时启动/健康探测延迟 | 现有测试更多关注 build、bundle、可用性、provider probe 正确性 | 缺少冷启动耗时、provider info/health latency 的回归预算 | 补一个 launcher startup perf smoke，记录 `info_latency_ms`、`health_latency_ms`、首屏 ready/可交互时间 | `on-demand`，稳定后可升 `required-scoped` | P2 |
| P2P / 共识 / 存储短窗性能回归 | `p2p-longrun-soak.sh` / `s10-five-node-game-soak.sh` / release soak gate 已存在，且 release 口径较强 | 日常 PR 默认不跑，很多退化只能到 release 或专项长跑才显现 | 为 `oasis7_node/**`、`oasis7_net/**`、存储/复制热点改动补一条短窗 scoped soak，减少“只在 release 才看到”的回归 | `on-demand` for dev, `release` for high-risk merge path | P1 |
| LLM / provider 回路性能 | `llm-longrun-stress.sh` 已采 `runtime_perf` 聚合；有 trace latency 时，本地 decision/tick 记录 `max(total - llm_api, 0)`，`llm_api` 以独立 1000-ms budget 只作诊断，不进入 local health/bottleneck。缺失 `AgentDecisionTrace.llm_diagnostics.latency_ms` 时 latency 记 0，该 sample 属于 observability-degraded，不能证明 local health 已与 API latency 解耦。 | 缺少“模型可用但变慢”的短窗预算，也缺少 missing-trace degradation 的专门回归；容易只知道能跑，不知道延迟在升高 | 补 provider-backed smoke，记录 request latency、extra wait、tick over-budget ratio，并与 builtin/基线做相对对比；若改变 missing-trace 行为，必须由 runtime task 补回归 | `on-demand`，关键 provider 改动时强制 | P2 |
| WASM 执行器 / module cache 热点 | 仓库里已有 ignored perf probes，如 `oasis7_wasm_executor` / `oasis7_wasm_abi` 的 local perf probe | 这些 probe 没有进入标准回归，热点退化更依赖手工意识 | 把 ignored probe 收敛成可复用的本地 perf checklist，先作为诊断手册，不急着进默认 CI | `on-demand` | P3 |
| 跨 surface 统一预算口径 | 局部有 compile closure/size、Viewer probe、release soak 阈值 | 缺少统一的 surface budget 表，导致哪些指标算退化依赖人记忆 | 建一份 repo-level perf budget 表，只维护少量核心指标：Viewer FPS/frame p95、runtime tick p95、launcher build/size、soak stall/lag | 先文档化，后逐步接 gate | P2 |

## 建议优先顺序
1. 继续收敛 Viewer changed-path scoped 性能 gate，因为 report-only 入口已经存在，下一步风险在阈值、噪音与 blocking rollout。
2. 再补 runtime tick/step 的轻量 deterministic perf harness，减少只能靠长跑发现性能退化的问题。
3. 然后补 P2P/存储/共识的短窗 scoped soak，把一部分 release 才暴露的问题前移。
4. 最后再考虑 launcher runtime startup perf、LLM/provider latency 预算和 WASM ignored probes 的标准化。

## 先做哪三项
| Rank | Item | Why now | Minimum deliverable | Gate recommendation | Rollout caution |
| --- | --- | --- | --- | --- | --- |
| 1 | Viewer changed-path scoped 性能 gate | Viewer 是最直接的用户面，掉帧/卡顿最容易被感知；现有 `viewer-performance-probe.sh` 已经进入 required-gate report-only scoped smoke | 已在 `crates/oasis7_viewer/**` 命中时运行 `./scripts/viewer-performance-probe.sh --profile smoke`，并稳定产出 `summary.json`/`summary.md`；下一步是积累基线与阈值噪音样本 | `required-scoped` report-only，blocking 仍待决策 | 不要在阈值和环境噪音稳定前转 blocking；report-only warning 不是 release/PR 阻断结论 |
| 2 | Runtime tick/step 轻量 deterministic perf harness | 当前 runtime 性能更多靠 longrun/stress 暴露，反馈太晚；需要一条能日常回归的热路径预算 | 第一阶段已落 `./scripts/runtime-module-routing-perf-harness.sh`，固定输入输出 `event/action avg ms`；首个 `release` baseline 为 `event_avg_ms=5.591`、`action_avg_ms=6.992`；后续再扩到更接近 tick/step 的稳定指标 | 先保持本地/reporting 入口，再评估 `required-scoped` 候选 | 不要一开始就拿整场景总耗时做 gate，优先选最稳定的内环指标，避免 flaky；冷 `release` 编译成本也要一起纳入 gate 设计 |
| 3 | P2P/存储/共识短窗 scoped soak | 这类退化现在太容易拖到 release soak 才暴露，修复成本高；需要前移一条便宜但有代表性的短窗检查 | 为 `oasis7_node/**`、`oasis7_net/**`、复制/存储高风险改动准备一个 reduced-duration soak 命令和统一 summary 判读 | 开发期先 `on-demand`，高风险合流或 release 前强制 | 必须按 changed paths 和风险等级触发，不能把 release 级长跑成本扩散到普通 PR |

## 实施顺序建议
1. 继续推进第 1 项，因为脚本、指标基础和 report-only scoped gate 已存在，最需要补的是基线样本与 blocking 决策。
2. 再做第 2 项，因为 runtime 缺的不是“有没有长跑”，而是“有没有日常可回归的小预算”。
3. 第 3 项随后做，因为它收益很高，但执行成本和 CI 时长影响也更大，需要在 scoped 触发条件上更谨慎。

## 每项完成定义
- Viewer changed-path scoped 性能 gate:
  - changed-path planner 能识别 `crates/oasis7_viewer/**`
  - smoke profile 能稳定落 summary 产物
  - 当前状态为 required-gate report-only scoped smoke；后续完成定义是积累足够基线，明确哪些阈值可升级为 blocking
- Runtime tick/step harness:
  - 至少 1 个稳定 hot path 场景
  - 指标输出可机读
  - 有明确“超预算”判定，但先不要求覆盖所有 runtime 功能
- P2P/短窗 soak:
  - 至少 1 条 reduced-duration 命令
  - summary 字段和 pass/fail 语义固定
  - changed-path/risk-class 触发条件清楚，不污染普通 PR

## 当前不建议做的事
- 不建议把所有性能测试都塞进默认 `required`，否则日常 PR 时延会明显上升，且噪音会很大。
- 不建议为每个功能单独建 perf case；优先围绕关键路径、共享热点和发布风险面抽样。
- 不建议在没有稳定基线前就给所有 surface 设硬阈值；先文档化预算，再逐步 gate 化。
