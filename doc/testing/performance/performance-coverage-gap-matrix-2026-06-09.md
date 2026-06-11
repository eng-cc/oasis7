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
| Viewer Web 帧率 / frame time | `./scripts/viewer-performance-probe.sh --profile smoke|release`；`test:perf-harness` 覆盖部分前端性能指标 contract | PR 默认 gate 不盯 FPS / frame p95 / long task，Viewer 改动可能到人工验收才暴露卡顿 | 把 `viewer-performance-probe.sh --profile smoke` 做成 `crates/oasis7_viewer/**` 命中时的 changed-path scoped gate，保留 `release` profile 给更高风险改动 | `required-scoped` | P1 |
| Viewer headed Web 真环境渲染 | 有 `agent-browser` 闭环、software-safe contract、build/UI tests | 当前缺少“真实 headed + GPU 路径”的轻量性能回归，容易只测 DOM/contract 不测渲染体感 | 补一条 headed smoke profile，记录 `readyMs`、FPS、frame p95、long task count，并把 SwiftShader/软件渲染继续当环境阻断处理 | `full-scoped` | P2 |
| Viewer 移动端 / 窄屏性能 | 现有手册强调 desktop/mobile 截图与 visual review，但性能 probe 主要围绕当前默认 Viewer Web 入口 | 缺少移动布局下 DOM 规模、ready time、交互响应的独立预算 | 给 `viewer-performance-probe.mjs` 增加 mobile viewport profile，至少落 `readyMs + frame p95 + DOM size` | `on-demand`，稳定后可升 `required-scoped` | P2 |
| Runtime tick / step 热路径 | `test_tier_required/full` 覆盖正确性；`llm-longrun-stress.sh` 会产出 `runtime_perf.tick.p95_ms` 等指标；`./scripts/runtime-module-routing-perf-harness.sh` 已补上稳定内环 module routing micro harness，并已有首个 `release` baseline（`event_avg_ms=5.591`、`action_avg_ms=6.992`、`modules=192`、`iterations=80`） | 现阶段轻量 harness 只覆盖 runtime module routing 内环，尚未扩到更完整 tick/step 预算；冷 `release` 编译耗时仍偏高 | 下一步把更多稳定热路径收进同一 summary/checklist，并评估哪些指标适合变成 scoped gate | `required-scoped`（仅限稳定 harness 命中路径）候选 | P1 |
| Runtime 长时稳定性 | `./scripts/llm-longrun-stress.sh` 已有 runtime perf health、bottleneck、tick 指标聚合 | 仍偏长窗和按需使用，不适合普通 PR 快速暴露持续退化 | 增加一个更短的 smoke profile，缩短到适合 pre-PR 的窗口，只盯最关键 perf health 指标 | `full-scoped` | P2 |
| Launcher 编译效率 / 依赖闭包 / 包体积 | `compile-metrics.yml` + `ci-compile-metrics.sh` 已测 package closure、冷 `cargo check`、冷 `cargo build --release`、binary size，并 gate `wasmtime` 不得漏入 | 三平台 current+baseline 冷编译成本高，且目前只盯 launcher；其他关键 crate 的编译膨胀没有统一基线 | 将 compile metrics 保持为 `workflow_dispatch` 手动触发；按依赖/包体/编译性能风险或故障排查补跑，后续若要恢复默认 gate 需先做更窄 changed-path 触发 | launcher 与其他 crate 均先 `on-demand` | P1 |
| Launcher 运行时启动/健康探测延迟 | 现有测试更多关注 build、bundle、可用性、provider probe 正确性 | 缺少冷启动耗时、provider info/health latency 的回归预算 | 补一个 launcher startup perf smoke，记录 `info_latency_ms`、`health_latency_ms`、首屏 ready/可交互时间 | `on-demand`，稳定后可升 `required-scoped` | P2 |
| P2P / 共识 / 存储短窗性能回归 | `p2p-longrun-soak.sh` / `s10-five-node-game-soak.sh` / release soak gate 已存在，且 release 口径较强 | 日常 PR 默认不跑，很多退化只能到 release 或专项长跑才显现 | 为 `oasis7_node/**`、`oasis7_net/**`、存储/复制热点改动补一条短窗 scoped soak，减少“只在 release 才看到”的回归 | `on-demand` for dev, `release` for high-risk merge path | P1 |
| LLM / provider 回路性能 | `llm-longrun-stress.sh` 已采 `runtime_perf` 聚合；provider contract smoke 偏正确性 | 缺少“模型可用但变慢”的短窗预算，容易只知道能跑，不知道延迟在升高 | 补 provider-backed smoke，记录 request latency、extra wait、tick over-budget ratio，并与 builtin/基线做相对对比 | `on-demand`，关键 provider 改动时强制 | P2 |
| WASM 执行器 / module cache 热点 | 仓库里已有 ignored perf probes，如 `oasis7_wasm_executor` / `oasis7_wasm_abi` 的 local perf probe | 这些 probe 没有进入标准回归，热点退化更依赖手工意识 | 把 ignored probe 收敛成可复用的本地 perf checklist，先作为诊断手册，不急着进默认 CI | `on-demand` | P3 |
| 跨 surface 统一预算口径 | 局部有 compile closure/size、Viewer probe、release soak 阈值 | 缺少统一的 surface budget 表，导致哪些指标算退化依赖人记忆 | 建一份 repo-level perf budget 表，只维护少量核心指标：Viewer FPS/frame p95、runtime tick p95、launcher build/size、soak stall/lag | 先文档化，后逐步接 gate | P2 |

## 建议优先顺序
1. 先补 Viewer changed-path scoped 性能 gate，因为这是最容易感知、也最容易退化的用户面。
2. 再补 runtime tick/step 的轻量 deterministic perf harness，减少只能靠长跑发现性能退化的问题。
3. 然后补 P2P/存储/共识的短窗 scoped soak，把一部分 release 才暴露的问题前移。
4. 最后再考虑 launcher runtime startup perf、LLM/provider latency 预算和 WASM ignored probes 的标准化。

## 先做哪三项
| Rank | Item | Why now | Minimum deliverable | Gate recommendation | Rollout caution |
| --- | --- | --- | --- | --- | --- |
| 1 | Viewer changed-path scoped 性能 gate | Viewer 是最直接的用户面，掉帧/卡顿最容易被感知；现有 `viewer-performance-probe.sh` 已经存在，补 gate 的改造成本最低 | 在 `crates/oasis7_viewer/**` 命中时运行 `./scripts/viewer-performance-probe.sh --profile smoke`，先稳定产出 `summary.json`/`summary.md` 与基础阈值结果 | 先做 `required-scoped` 候选 | 前几轮建议 `report-only`，先稳定阈值和环境噪音，再转 blocking |
| 2 | Runtime tick/step 轻量 deterministic perf harness | 当前 runtime 性能更多靠 longrun/stress 暴露，反馈太晚；需要一条能日常回归的热路径预算 | 第一阶段已落 `./scripts/runtime-module-routing-perf-harness.sh`，固定输入输出 `event/action avg ms`；首个 `release` baseline 为 `event_avg_ms=5.591`、`action_avg_ms=6.992`；后续再扩到更接近 tick/step 的稳定指标 | 先保持本地/reporting 入口，再评估 `required-scoped` 候选 | 不要一开始就拿整场景总耗时做 gate，优先选最稳定的内环指标，避免 flaky；冷 `release` 编译成本也要一起纳入 gate 设计 |
| 3 | P2P/存储/共识短窗 scoped soak | 这类退化现在太容易拖到 release soak 才暴露，修复成本高；需要前移一条便宜但有代表性的短窗检查 | 为 `oasis7_node/**`、`oasis7_net/**`、复制/存储高风险改动准备一个 reduced-duration soak 命令和统一 summary 判读 | 开发期先 `on-demand`，高风险合流或 release 前强制 | 必须按 changed paths 和风险等级触发，不能把 release 级长跑成本扩散到普通 PR |

## 实施顺序建议
1. 先做第 1 项，因为脚本和指标基础已经存在，最容易快速落成一条可见收益高的 gate。
2. 再做第 2 项，因为 runtime 缺的不是“有没有长跑”，而是“有没有日常可回归的小预算”。
3. 第 3 项随后做，因为它收益很高，但执行成本和 CI 时长影响也更大，需要在 scoped 触发条件上更谨慎。

## 每项完成定义
- Viewer changed-path scoped 性能 gate:
  - changed-path planner 能识别 `crates/oasis7_viewer/**`
  - smoke profile 能稳定落 summary 产物
  - 至少完成一轮 report-only 基线收集
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
