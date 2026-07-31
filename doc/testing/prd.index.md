# testing PRD 文件级索引

审计轮次: 10

更新时间：2026-06-26

## 入口
- 模块 PRD：`doc/testing/prd.md`
- 模块设计总览：`doc/testing/design.md`
- 可变任务状态与历史：GitHub task issue evidence comments
- 当前 QA 测试合同与覆盖边界：`doc/testing/prd.md`

## 首读分流
- 想先回答 testing 模块覆盖哪些测试层级、证据与门禁边界：先读 `doc/testing/prd.md`
- 想判断好玩性证据、`L4A/L4B/L5` 边界、角色 subagent review 或 simulated player persona 的 canonical topic：先读 `doc/testing/governance/README.md`
- 想直接在一个 worktree 里准备完整 `L4A + L4B` 执行包：先经 `doc/testing/governance/README.md` 确认所需证据层，再读 `testing-manual.md` 的 `L4A/L4B/L5` 段落并执行 `./scripts/prepare-playability-l4-review.sh`；正式 `L4B` embodied-agent run 再由 `./scripts/run-playability-l4b-agent.sh --l4-manifest <artifact>/manifest.json` 收口。
- 想执行 Web UI、Playwright、public-testnet attach 或模型视觉评审手册：先读 `doc/testing/manual/README.md`，再按问题进入对应 manual。
- 想快速判断“现有性能测试覆盖到哪、哪些功能面最值得补性能测试、哪些更适合进 scoped gate”：先读 `doc/testing/performance/performance-coverage-gap-matrix-2026-06-09.md`
- 想先回答 world state sync、commit closure、state-sync bundle、API/viewer projection，或 `module_required/module_full/integration_required/release_full` 如何定档：先读 `doc/testing/longrun/game-world-state-sync-commit-closure-2026-06-26.prd.md`，再下钻 S9/S10 执行套件。
- 想确认当前任务、QA 阻断或收口状态：读取 GitHub task issue evidence comments；文档只提供稳定测试合同与证据边界。
- 想直接决定要跑哪套测试或按步骤执行：先读 `testing-manual.md`；涉及专项操作再进入 `doc/testing/manual/README.md`。
- 想先进入 `evidence` 热点子域，并按 release gate / hosted access / legacy p2p network rehearsal / governance drill / claim-audit 问题分流：先读 `doc/testing/evidence/README.md`
- 想继续按子域或文件名下钻：使用下方热点子域导航，再跳到对应清单区域

## 密度说明
- 本索引不维护静态文件数量；数量快照容易漂移，只能作为历史审计材料，不作为默认阅读顺序或覆盖完整性的依据。
- 需要重新统计时，以 `./scripts/doc-inventory-report.sh` 的 module density / hotspot 状态为准，并把正式报告结果写入具体审计任务；`find doc/testing -type f` / `rg --files doc/testing` 只可作为本地探索辅助，不能替代正式 inventory 状态。

## 热点子域导航
| 子域 | 适合回答的问题 |
| --- | --- |
| `benchmarks/` | 测试成熟度对标、单实现栈等价要求、历史缺口矩阵与 claims denylist；不维护当前网络状态 |
| `evidence/` | 发布证据、趋势基线与审计留痕；当前已补 `evidence/README.md` 作为热点子域入口 |
| `ci/` | CI、wasm determinism、tiering、required check 保护 |
| `longrun/` | 长稳、chaos、soak、world state sync / commit closure 与在线稳定性 |
| `launcher/` | 启动器链路测试、playtest 与配置自动接线 |
| `governance/` | 质量趋势、release-gate 指标、审计检查与 playability 证据治理；先读 `doc/testing/governance/README.md` 再按问题下钻 |
| `templates/` | 证据包、报告、模型视觉评审卡与检查清单模板；默认按需进入 |
| `performance/` | runtime / viewer 性能观测与方法学 |
| `manual/` | 系统测试手册分册、Web UI / Playwright / public-testnet attach 与模型视觉评审；先读 `doc/testing/manual/README.md` |
| `chaos-plans/` | 专项 chaos plan 入口 |

## 活跃补充文档
- `doc/testing/governance/README.md`：`governance/` 热点子域 landing page，按质量门禁、playability 证据、L4 分层、role review 与 persona 问题分流读者。
- `doc/testing/evidence/README.md`：`evidence/` 热点子域 landing page，按 release gate、hosted access、legacy p2p network rehearsal、governance drill 与 claim/audit 分流读者。
- `testing-manual.md`：仓库级系统测试手册，不并入下方模块 PRD 三件套长表。
- `doc/testing/manual/README.md`：`manual/` 子树 landing page，按 Web UI 页面闭环、真实 Playwright 流程、formal public-testnet attach 与模型视觉评审分流；具体 manual 保留 operator 真值，不并入下方模块 PRD 三件套长表。
- `doc/testing/performance/performance-coverage-gap-matrix-2026-06-09.md`：按 surface 汇总现有性能覆盖、当前缺口、建议补测和建议 tier 的速查表。
- Provider dual-mode 历史 blocker 已收敛到 `doc/world-simulator/llm/provider-agent-dual-mode.prd.md`、`provider-agent-dual-mode-contract.md` 与复签 evidence；当前 task blockers 只从 GitHub task issue evidence comments 进入。

Manual convergence note: Web UI manual 当前按 driver / evidence mode 分流，不再作为单一 PRD/design/project 三件套维护。`web-ui-agent-browser-closure-manual.*` 保留 Viewer 页面通用闭环的 PRD / project / manual 真值；`web-ui-playwright-closure-manual.manual.md` 保留真实本地栈 + 玩家 UI 操作流程的 `PWT-###` 系列入口；`web-ui-playwright-closure-manual.design.md` 仅作为历史/共享设计 companion 追溯，不再把两条 manual surface 合并成一个 normal triplet。

## 默认阅读面边界
- 本页首屏只负责分流，不再要求读者从第一行开始顺扫完整长表。
- README 不再平铺“近期专题”；专题清单继续保留在下方，用于精确文件名检索和互链可达性。
- `evidence/README.md` 负责最高密度热点子域的首读分流；完整长表继续由本页与目标 evidence 文件承担。
- 手册、blocker、evidence 与 template 等 supporting / 审计材料继续保留可检索性，但不并入模块 PRD 三件套长表。

## 覆盖规则
- 纳入规则：一般专题纳入 `doc/testing/**` 下所有 `*.prd.md` 及同名 `*.design.md` / `*.project.md`，继续遵守专题三件套治理；仅下方明确列出的、已经完成且当前设计与验收口径已并入 PRD 的 launcher 两个专题采用退休例外，其 design/project 拆解由 GitHub task evidence 与 Git 历史承接。
- 活跃补充：`testing-manual.md`、`*.manual.md` 与仍被当前模块 PRD / 项目态直接引用的 blocker/supporting spec，可在“活跃补充文档”区定向列出，但不并入下方三件套长表。
- 排除规则：不纳入 `doc/devlog/**`、`doc/testing/evidence/**`、`doc/testing/templates/**` 与其他非 PRD 配对文档。
- 按需进入：evidence、template、blocker、closure 说明与历史归档继续保留可检索性；除非它们重新成为当前 operator 或 owner 的直接入口，否则不进入默认首屏。

## 专题清单（含历史完成项，按文件名精确检索）
本表用于互链可达性和 traceability，不代表每一行都是默认首读或当前活跃执行面；一般已完成规划三件套应先读其 redirect / historical status，再进入当前 canonical 手册或 topic 文档。launcher 两个已完成专题的退休例外仅用于本次明确收敛，不改变新建或其他现存专题的三件套规则。

2026-07-15 governance note: 早期 launcher 专题三件套与已完成的 design/project 拆解已退役删除；当前 launcher / playtest / Web UI 闭环入口收敛到 `testing-manual.md`、`doc/testing/launcher/README.md`、两份 launcher 现行 PRD、Web UI manual、GitHub task issue evidence comments 与 Git 历史。

Performance observability historical/provenance note: 2026-02-25 的 runtime foundation、LLM/API decoupling、Viewer native bottleneck 与 Viewer methodology 四组三件套已完成专业权威合并并物理删除；当前 authority 为 `doc/testing/performance/performance-coverage-gap-matrix-2026-06-09.md`、`doc/testing/performance/viewer-current-web-performance-harness-2026-06-02.prd.md` 与 `testing-manual.md`。旧 Viewer native `RenderPerfSummary` / `PerfHotspot` / runtime-stage / CSV contract 随 old `perf_probe` / `viewer-owr4-stress` 退役；当前浏览器 probe 不承诺 hotspot 分类或 runtime-stage diagnosis。历史里程碑、阈值来源与完成证据仅从 Git/GitHub task evidence 追溯。

Builtin WASM CI 历史收口：早期 m1 independent multi-runner、required-check protection、base-gate hash、wasm32 target install 与 build-determinism guard 五组三件套已退役删除。当前测试入口为 `doc/testing/ci/ci-builtin-wasm-docker-canonical-gate.prd.md`、`doc/testing/prd.md`、`testing-manual.md` 与 `.github/workflows/wasm-determinism-gate.yml`；发布级 build/hash/identity/release evidence authority 为 `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md`。历史实施与完成证据只从 Git history 和 GitHub task issue evidence comments 追溯。

| 专题 PRD | 专题设计文档 |
| --- | --- |
| `doc/testing/benchmarks/mainstream-public-chain-testing-benchmark.prd.md` | `doc/testing/benchmarks/mainstream-public-chain-testing-benchmark.design.md` |
| `doc/testing/ci/ci-builtin-wasm-docker-canonical-gate.prd.md` | `doc/testing/ci/ci-builtin-wasm-docker-canonical-gate.design.md` |
| `doc/testing/ci/ci-test-coverage.prd.md` | `doc/testing/ci/ci-test-coverage.design.md` |
| `doc/testing/ci/ci-testcase-tiering.prd.md` | `doc/testing/ci/ci-testcase-tiering.design.md` |
| `doc/testing/ci/ci-tiered-execution.prd.md` | `doc/testing/ci/ci-tiered-execution.design.md` |
| `doc/testing/governance/llm-skip-tick-ratio-metric.prd.md` | `doc/testing/governance/llm-skip-tick-ratio-metric.design.md` |
| `doc/testing/prd.md`（Durable Playability Evidence Governance） | `testing-manual.md` 的 L4A/L4B/L5 operator steps | 已吸收的 2026-05 playability governance batch：Git history 与 GitHub task evidence |
| `doc/testing/governance/token-genesis-allocation-audit-checklist.prd.md` | `doc/testing/governance/token-genesis-allocation-audit-checklist.design.md` |
| `doc/testing/governance/testing-quality-trend-tracking-2026-03-11.prd.md` | `doc/testing/governance/testing-quality-trend-tracking-2026-03-11.design.md` |
| `doc/testing/launcher/launcher-bundle-first-playtest-entry-2026-03-12.prd.md` | 当前设计决策与验收记录已并入 PRD | 已完成项目拆解见 Git 历史与 GitHub task evidence |
| `doc/testing/launcher/launcher-manual-test-checklist-2026-03-10.prd.md` | 当前执行设计与分层清单已并入 PRD | 已完成项目拆解见 Git 历史与 GitHub task evidence |
| `doc/testing/longrun/game-world-state-sync-commit-closure-2026-06-26.prd.md` | `doc/testing/longrun/game-world-state-sync-commit-closure-2026-06-26.design.md` |
| `doc/testing/longrun/p2p-longrun-soak-and-chaos.prd.md` | `doc/testing/longrun/p2p-longrun-soak-and-chaos.design.md` |
| `doc/testing/longrun/s10-five-node-real-game-soak.prd.md` | `doc/testing/longrun/s10-five-node-real-game-soak.design.md` |

## 说明
- 本索引用于保证模块专题文档在根入口文档树中可达。
- 文档配对规则：新建及一般现存专题保持 `*.prd.md`、`*.design.md` 与同名 `*.project.md`；本页明确标记的 launcher 完成态退休例外不作为后续建档模板。
