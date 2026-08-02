# Workflow Reference (Single Source)
- 本手册仅定义测试分层、套件入口和操作细节。
- 流程阶段、责任边界、必需/可选 gate、失败回退路径统一以 `doc/engineering/workflow/source-of-truth.md` 为准。
- 如测试流程规则与 source-of-truth 冲突，以 source-of-truth 为准，并先更新 source-of-truth。

# oasis7: 系统性应用测试手册（Human/AI 通用）

## 目标
- 基于仓库当前实现，提供一套可直接执行的分层测试手册，让人类开发者与 AI Agent 都能对“整应用”做足够充分的测试。
- 解决“只跑一条命令看总绿灯”但无法定位风险层的问题，把测试明确拆成基础门禁、核心逻辑、协议集成、分布式子系统、UI 闭环、压力回归。
- 把 `test_tier_required` 与 `test_tier_full` 放回整体测试体系中：它们是核心层基线，不等于“整应用全覆盖”。
- 统一证据标准（命令、日志、截图、结论），保证测试可复盘、可审计。

## 对标入口
- 本手册负责 oasis7 自己的执行分层与命令入口；当前 `public_testnet` readiness 以本手册的 Network Tiers 入口和 formal `public_testnet` runbook 为准。
- 主流公链成熟度 benchmark 与 legacy network-rehearsal / release-train 专题只作对标和历史 evidence 追溯，不作为当前 `public_testnet`、`mainnet` 或统一持久大世界上线结论。

## 范围

### In Scope
- 结合当前仓库真实实现给出分层模型与命令清单。
- 给出“改动路径 -> 应跑测试层级”的触发矩阵。
- 给出 Human/AI 共用执行剧本、通过标准、失败分诊与证据规范。
- 明确现有 CI 覆盖能力与手册补充覆盖能力的边界。

### Out of Scope
- 不在本任务修改 CI workflow 或测试脚本行为。
- 不引入新的测试框架或新的业务代码。
- 不做覆盖率百分比硬门槛治理（如行覆盖率 >= N%）。

## 开发态缓存约定
- 若当前是在同一 repo family 的多个 git worktree 之间做本地迭代，开发态 `cargo check/test/run/build` 默认优先使用 `./scripts/cargo-dev.sh <cargo-args...>`，让多个 task worktree 复用 shared target 目录，减少重复编译。通过 `./scripts/new-task-worktree.sh` 创建的新 task worktree 还会默认把 git-ignored `target` 链接到同一个 shared target 目录，使直接 cargo 与 wrapper 混用时也优先落到同一开发态缓存。
- 本地 smoke / playtest / prewarm / regression / drill / longrun 脚本若只是为了开发反馈，应优先 source `scripts/cargo-dev-lib.sh` 并调用 `oasis7_cargo_dev ...` / `oasis7_cargo_dev_debug_bin_dir`，从而与手工 `cargo-dev.sh` 使用同一个 shared target；`CI=1`、`OASIS7_CARGO_DEV_SHARED=0` 或 `OASIS7_FORCE_RAW_CARGO=1` 会回退到原始 cargo target 解析。
- 该入口只服务开发态缓存复用，不替代本手册中的正式验收命令；手册里的 canonical 验收命令仍显式写原始 `env -u RUSTC_WRAPPER cargo ...`。
- deterministic wasm / release 链路继续保持 `CARGO_TARGET_DIR` 为空；涉及 `scripts/build-wasm-module.sh`、release evidence 或 hash/receipt 对账时，不要改用 `scripts/cargo-dev.sh`。

## 当前实现分布（2026-02-18 基线）

### 应用主链（world + runtime + simulator + viewer 协议）
- 核心 crate：`crates/oasis7`
- 主要测试分布：
  - 运行时：`crates/oasis7/src/runtime/tests/*.rs`
  - 模拟器：`crates/oasis7/src/simulator/tests/*.rs`
  - LLM 行为：`crates/oasis7/src/simulator/llm_agent/tests_part2.rs`
  - Viewer live 服务：`crates/oasis7/src/bin/oasis7_viewer_live.rs`（内置 `#[cfg(test)]`）
  - 端到端集成：`crates/oasis7/tests/*.rs`

### Pixel World Bridge（Bevy + wasm）
- crate：`crates/pixel_world_bridge`
- 覆盖：
  - Bevy world projection、相机/选中态与渲染逻辑测试散布在 `src/*_tests.rs`
  - 确定性 raster 基线由 `scripts/viewer-pixel-world-bevy-pixel-regression.sh` 采集
  - `wasm32-unknown-unknown` 编译检查覆盖 WebGL2 bridge 的目标特定构建入口

### Viewer Web 前端（JS/HTML + wasm bundle）
- package：`crates/oasis7_viewer`
- 覆盖：
  - source 结构、feedback contract 与 SolidJS 组件测试由 package scripts 提供
  - Web 启动入口：`oasis7_game_launcher`（内置静态服务，`run-viewer-web.sh` 仅保留为兼容/排障工具）
  - Web 闭环采样：agent-browser CLI（详见 `doc/testing/manual/web-ui-agent-browser-closure-manual.manual.md`）

### 分布式与共识子系统
- Node：`crates/oasis7_node`
- Net：`crates/oasis7_net`
- Consensus：`crates/oasis7_consensus`
- DistFS：`crates/oasis7_distfs`
- 这些子系统有独立测试集，但当前 `scripts/ci-tests.sh` 只覆盖了其中一部分（见下文“CI 现状与缺口”）。

### 场景系统
- 场景定义：`crates/oasis7/src/simulator/scenario.rs`
- 场景矩阵设计：`doc/world-simulator/scenario/scenario-files.prd.md`
- 场景是 UI 闭环、协议闭环、压力回归的统一输入源。

## CI 现状与缺口（事实口径）

### 当前 CI/脚本已覆盖
- 入口 A：`scripts/ci-tests.sh`（主流程）
- `commit`：
  - `./scripts/doc-governance-check.sh`
  - `./scripts/check-script-executable-bits.sh`
  - `./scripts/check-rust-file-size.sh`
  - `cargo fmt --check`
  - `cargo test -p oasis7_consensus --lib`
  - `cargo test -p oasis7_distfs --lib`
  - `npm --prefix crates/oasis7_viewer run test:feedback-contract`
  - `npm --prefix crates/oasis7_viewer run test:ui`
  - 用途：显式本地诊断 baseline，不由普通 `pre-commit` 自动执行；不包含 `cargo test -p oasis7 --tests --features test_tier_required`，但包含 repo-owned Viewer Web contract + Solid 组件锚点回归。
- `required`：
  - `./scripts/doc-governance-check.sh`
  - `./scripts/check-script-executable-bits.sh`
  - `./scripts/check-rust-file-size.sh`
  - `cargo fmt --check`
  - `cargo test -p oasis7 --tests --features test_tier_required`
    - 该 shard 现在只覆盖轻量核心基线；需要注册或执行 builtin wasm artifact 的 runtime 闭环用例已下放到 `test_tier_full`。
  - `cargo test -p oasis7_consensus --lib`
  - `cargo test -p oasis7_distfs --lib`
  - `npm --prefix crates/oasis7_viewer run test:feedback-contract`
  - `npm --prefix crates/oasis7_viewer run test:ui`
  - `npm --prefix crates/oasis7_viewer run build:software-safe`
- `full`：
  - `required` 全部
  - `cargo test -p oasis7 --tests --features test_tier_full,wasmtime,viewer_live_integration`
  - `cargo test -p oasis7_node --lib`
  - `cargo test -p oasis7_net --lib`
  - `cargo test -p oasis7_net --features libp2p --lib`
  - `./scripts/llm-baseline-fixture-smoke.sh`
  - `cargo test -p oasis7 --features wasmtime --lib --bins`

`./scripts/ci-tests.sh` 不设默认 tier；省略参数会打印 usage 并失败，避免误把升级专用的 `full` 当成本地默认。

- 入口 B：`.github/workflows/rust.yml`（required-gate）
  - planner 先执行：`./scripts/plan-rust-required-scope.sh --event-name <push|pull_request> --base-ref <base> --head-ref <head>`
  - `CI_VERBOSE=1 ./scripts/ci-tests.sh required`
  - 本地显式 `./scripts/ci-tests.sh required` 仍保持基础 required 语义；只有 CI `required-gate` 与 `prepare-task-pr.sh` 推荐命令会根据 planner 输出注入选择性组件环境变量，并在命中 `crates/oasis7_node/**` / `crates/oasis7_net/**` 或 shared gate/full scope 时额外拉起 support-crate shard
  - 当 planner 选择 `crates/oasis7_viewer/**` 性能 surface 时，`required-gate` 必须通过 `viewer-performance-report-only.sh` 采集环境、web-dist 原始复现、样本 summary/markdown 与截图并上传 artifact；采集缺失、损坏或 summary/probe 状态矛盾必须阻断，只有完整有效样本的阈值 miss 在稳定可复现的环境特定采样阈值和有时限 waiver 生命周期建立前保持 report/watch
- 入口 B2：`.github/workflows/rust.yml`（人工 full escalation）
  - 先在绑定 task issue 写明升级理由与 frozen PR head，再以 `workflow_dispatch` 选择 `run_mode=full_escalation`，填写 `task_uid`、`pr_number`、`expected_head`、`escalation_reason`（`release|high_risk|history_defect|signal`）和同仓库 `evidence_url`；评论/标签本身不自动授权 CI
  - preflight 必须确认当前 checkout 与 `expected_head` 完全一致；成功后执行显式 `./scripts/ci-tests.sh full`，并始终上传 `oasis7-full-escalation-receipt-v1`；失败 receipt 只证明运行与失败，不得作为通过证据
  - schedule `full-regression` 是非 PR 定时回归，不能替代 PR exact-head escalation receipt
- 入口 C：`.github/workflows/wasm-determinism-gate.yml`（构建 hash / receipt evidence 独立 gate）
  - GitHub-hosted runner 矩阵：`(m1|m4|m5) x (ubuntu-24.04/linux-x86_64)`
  - planner 先执行：`./scripts/plan-wasm-determinism-scope.sh --event-name <push|pull_request|workflow_dispatch> --base-ref <base> --head-ref <head>`
  - 仅命中的 module set 会实际执行：`./scripts/ci-m1-wasm-summary.sh --module-set <m1|m4|m5> --runner-label ... --out ...`
  - 未命中的 `m1|m4|m5` 仍保留 stable required context，但 collect/verify job 只输出 scope note 并 no-op success
  - verify job 会按命中的 `module_set` 下载 summaries，并执行：`./scripts/wasm-release-evidence-report.sh --module-sets <m1|m4|m5> --skip-collect --summary-import-dir <downloaded-summary-dir> --expected-runners linux-x86_64`
  - verify job 同时上传 `summary.md/json + logs + module_sets.tsv` 的 release evidence report artifact
  - 若要补跨宿主 full-tier 证据，可把外部 Docker-capable macOS runner 产出的 summary 作为额外 import 输入，再以 `--expected-runners linux-x86_64,darwin-arm64` 做离线对账

### 当前 CI 未直接覆盖（需手册补齐）
- Web UI agent-browser 闭环（现为手动/agent 流程，不在 CI 默认路径中）。
- `m4/m5` builtin wasm hash 校验（`scripts/ci-tests.sh` 已移除 `sync-m4/m5 --check`）。
- runtime builtin wasm bootstrap / default-module / body-action 闭环（已从 required 下放到 `test_tier_full`）。
- Viewer headed / GPU 真环境性能在被选择时必须采集，但在稳定可复现的环境特定采样阈值、原始复现与有时限 waiver 生命周期建立前仍是 report/watch，不是 blocking required gate。

结论：
- `commit` 是默认本地提交基线，目标是尽快暴露格式/治理/viewer-support 回归，但不承担 `oasis7 --tests` required shard；
- ordinary PR 的 `required-gate` 是 impact-scoped premerge 最小 blocking set：先拦截改动影响面的缺陷，再在不降低充分度的前提下优化速度；基础 required 含 `oasis7 + consensus + distfs + viewer`，GitHub 可按 planner 追加 `node + net/libp2p` support shard；
- `full` 不是 ordinary PR 默认，只用于 release、高风险、历史缺陷升级、信号触发或 schedule 回归；planner 的 `scope=full` 仅表示 required tier 内 fail-closed 覆盖扩张，不等于选择 full tier；
- `required-gate` 已补充 changed-path scope planner；
- `wasm-determinism-gate` 负责 `m1/m4/m5` hash / receipt evidence 独立 gate；
- 若目标是“整应用充分测试”，仍需在此基础上叠加 UI 闭环层（S6）与压力层（S8）。

## 默认测试选择树（影响面覆盖 + 最小充分测试）

先按改动影响面选择最小测试集，再按风险升级；不要把 `required`、`full` 或 release gate 当作“无脑全跑”的替代品。每次结论至少写清：本次影响面、已跑最小测试集、证据路径、未覆盖/残余风险。

CI 分层口径：ordinary PR 以 impact-scoped `required-gate` 作为 premerge 最小 blocking set，缺陷拦截优先、速度优化其次；`full` 只因 release、高风险、历史缺陷升级、信号触发或 schedule 运行。任何被选择的性能 surface 都必须采集环境、原始复现与样本；稳定可复现的环境特定采样阈值及有时限 waiver 生命周期成熟前，结论仅为 report/watch。

1. 文档、治理、脚本元数据：默认 S0；docs-only 同时执行命中的 contract / planner 样例。若改动测试/发布口径，再追加对应脚本的 syntax/dry-run 或 planner 样例，不因文档改动直接升级到 full。
2. runtime / simulator / world-model：先跑命中的定向 S3；落地前补 S1；只有跨模块、持久化、规则/历史回归风险无法由定向测试覆盖时才升 S2。
   - runtime timing、`RuntimePerfSnapshot`、LLM latency split、health/bottleneck 或 report consumer 变更，先读 `doc/testing/performance/performance-coverage-gap-matrix-2026-06-09.md` 的 Runtime/LLM 行；`llm-longrun-stress.sh` 输出属于 S8 诊断/长跑证据，不是默认 PR latency gate。
3. Viewer / Web / 可见 UI：先跑 deterministic contract 或 component test；触达 `crates/oasis7_viewer/**`、viewer wasm/build 链路或目标特定编译时，把 S5 作为 scoped required。触达可见表面时追加 S6 截图与模型视觉评审。
4. node / net / consensus / distfs：先跑命中的 S4 子系统测试；涉及在线拓扑、恢复、公开网络或存储/共识 claim 时追加 S9/S10。
5. builtin wasm / module release / hash：先跑对应 scope planner 与 module-set evidence；发布或跨 runner claim 才进入 release evidence 对账。
6. playability / player continue claim：自动化只能证明“没坏/可回归”；`L4A`、`L4B`、`L5` 按 claim 强度升级，不得互相替代。
7. release candidate：默认完整 release gate；任何 `--skip-*` 都必须写明原因，并在 summary 里保留 claim boundary。带 skip 的结果只能支撑剩余已执行步骤，不能支撑被跳过层级的 release claim。

## 分层模型（针对当前仓库）

### L0 静态与工件一致性层
- 目标：尽早拦截格式漂移、内置 wasm 工件漂移与基础脚本/文档契约问题；不承担目标特定编译。
- 性质：最快、最确定。

### L1 核心逻辑确定性层（oasis7 主体）
- 目标：覆盖 runtime/simulator/world-model/LLM 行为/viewer 协议主逻辑。
- 入口：`test_tier_required` 与 `test_tier_full`（主要在 `oasis7` crate）。
- 性质：主覆盖层，应承接绝大多数回归风险。

### L2 协议与联机集成层
- 目标：验证 viewer live、web bridge、离线回放链路、wasmtime 路径等跨模块协作。
- 性质：比 L1 慢，但比 UI 端到端稳定。

### L3 分布式子系统层（node/net/consensus/distfs）
- 目标：验证共识、网络、复制、存储一致性与恢复链路。
- 性质：不应缺席；否则“整应用测试”会有明显盲区。

### L4A synthetic 内部闭环层（Web 为默认）
- 目标：验证 formal player surface 的真实可用性（加载、交互、状态可见、无 console error），并在此基础上完成 synthetic internal playability review。
- 默认：agent / QA 在当前 git worktree 内做开发回归时，优先使用 `./scripts/worktree-harness.sh up` 起一套 worktree 隔离 Web 栈；它会为当前 worktree 派生独立端口组、bundle / runtime / artifact 根目录与浏览器 session，并把状态写到 `output/harness/<worktree_id>/state.json`。这一层属于 `L4A synthetic`，可以产出 UI 闭环、subagent review、persona panel 等内部模拟证据。`scripts/run-launcher-stack.sh` 保留为底层 bootstrap，并支持 `--bundle-dir <bundle>` 复用产物入口；当 bundle 缺少 freshness manifest 或已落后于当前工作区源码时，脚本会默认阻断。launcher stack 与 `--with-harness` 预热都走 formal gameplay 的 active LLM path；`--no-llm` 只保留给直接 `oasis7_viewer_live` 观战/调试排障。
- 完整 `L4` scaffold：先运行 `./scripts/prepare-playability-l4-review.sh --with-l4a-stack`。它会在当前 worktree 自己的 `output/harness/<worktree_id>/artifacts/playability-l4-<timestamp>/` 下生成 `l4-review-packet.md`、`role-review-cards/*.md`、`persona-cards/*.md`、`l4-summary.md`、`commands.sh`、`manifest.json`，并在 `evidence/` 下冻结当前 `L4A` harness state / URL。这个入口继承 formal gameplay 的 active LLM provider preflight；若当前环境缺少 `OASIS7_LLM_MODEL` / 等价 `config.toml`，会在 harness 启动前 fail-fast。
- 最低完成定义：至少回填 review packet、`producer_system_designer` / `qa_engineer` 角色卡，以及命中的其余角色卡与 persona cards；只跑 harness / S6，不回填这些卡片，不算 `L4A` 完成。
- 结论边界：这一层可以回答“synthetic 看起来会不会继续玩”，不能直接回答“真人是否真的想继续玩”。
- native 抓图：仅 fallback（Web 无法复现或 native 图形链路问题）。

### L4B 具身 agent 试玩层
- 目标：验证 agent 实际进入 formal player surface、执行真实操作链路后，是否表现出可持续继续游玩的行为与可解释的玩家杠杆判断；这一层的设计目标是尽可能逼近真人评审“还想不想继续玩”的判断效果。
- 默认：agent 试玩优先使用 `./scripts/run-playability-l4b-agent.sh --l4-manifest <artifact>/manifest.json`。它会通过 `./scripts/run-producer-playtest.sh --open-headed` 拉起真实游玩入口、在同一浏览器 session 内执行最小真实操作链路（至少包含一步推进与一次玩法动作提交）、并把状态快照、截图、启动日志路径与 `L4B` summary 落到当前 artifact；只有启动脚本、没有实际 agent play session 与 summary/card 产物，不算 `L4B` 完成。
- 与 `L4A` 的衔接：`./scripts/prepare-playability-l4-review.sh` 会在同一 artifact 目录里复制 `l4b-agent-playtest-card.md`、生成 `optional-internal-human-corroboration.md`、`manifest.json` 和可直接执行的 `commands.sh`；`commands.sh` 默认调用 `run-playability-l4b-agent.sh` 收口 `L4B` evidence。默认完整 `L4` 至少收口 `L4A` packet / cards、`L4B` agent 卡和最终 `l4-summary.md`；若还需要内部人类校准，只能作为 `L4B` 的可选佐证附录，不新增正式层。
- 最低完成定义：脚本实际跑过、agent 实际游玩过、`evidence/l4b-agent-*/l4b-agent-summary.json` 与 `l4b-agent-playtest-card.md`（或等价正式卡片）已落盘，并在 `l4-summary.md` 明确写出 `L4B` verdict；只有启动脚本，没有 agent 主动操作或 summary，不算完整 `L4B`。
- 结论边界：这一层可以回答“agent 在真实操作链路里是否表现出继续玩的倾向”，并应尽量逼近真人评审效果，但仍不能自动等价于 `L5` 真实人类或外部市场验证。
- 可选内部真人佐证：制作人试玩 / QA headed rerun 仍可沿用 `./scripts/run-producer-playtest.sh`；如执行，必须把结果写入 `optional-internal-human-corroboration.md` 或等价正式卡片，并在 `l4-summary.md` 里明确它是 `L4B` corroboration / contradiction，而不是新层级。只有人类试玩而没有对 `L4B` 的对照说明，不算合格佐证。
- source-tree `oasis7-run.sh play` 与 `run-launcher-stack.sh` 的 Viewer Web 开发态入口都必须走 freshness gate；当 `crates/oasis7_viewer/viewer.html`、`software_safe.html`、`viewer.js`、`software_safe.js`、`package.json`、`package-lock.json`、`vite.software-safe.config.mjs`、`scripts/`、`software_safe_src/` 或相关静态资源比 `dist/` 更新时，默认应优先重建 fresh dist，而不是继续拿 stale `dist` 给 Web 闭环下结论。

### L5 真实人类 / 受控线上验证层
- 目标：验证真实人类或受控外部玩家在真实时间、注意力和机会成本约束下，是否仍愿意继续玩；这是 `L4B` 之上的正式验证层。
- 入口：limited preview、liveops 反馈回流、真实玩家 session、受控人工试玩样本。
- 结论边界：只有到这一层，才能正式回答“真实人类/真实环境里是否仍想继续玩”；内部真人 spot-check 仍只算 `L4B` 的可选校准，不单列新层。
- 说明：长稳、压力和 soak 仍属于测试套件层（如 S8/S10），不是这里的 playability `L5`。

### Playability review、persona 与 claim 收口
- L4A packet 记录 change scope、target claim、formal surfaces、evidence、blockers、roles、personas、questions 与 target lane；card 必须区分直接证据、推断、未证明项和 follow-up。输入不足写 `insufficient_input`，只转述他人结论写 `secondary_review_only`。
- 默认先由 `producer_system_designer` 与 `qa_engineer` 审核，按 surface 追加 viewer/agent/runtime/WASM 角色；涉及 limited-preview 或外部 claim 时追加 `liveops_community`。角色 review 完整只构成 L4A。
- persona 仅按主观风险选择 `new_player_confused`、`impatient_action_player`、`systems_optimizer`、`narrative_curiosity_player`、`chaos_tester`，并回流标准角色。多 persona 同节点负面或发现 exploit/dominant strategy/softlock 时升级工程角色与 L4B；persona 不是正式角色，不能单独给 L4B、L5 或 stage verdict。
- `block`: formal surface 不稳定或没有稳定玩家杠杆；`hold`: L4A 未支持 continue 或 L4B 有高价值反证；`watch`: L4B 缺失/样本薄/与校准冲突；`go`: 仅当目标 claim envelope 内 L1-L3、需要的 L4A/L4B 无高价值反证且 L5 无新反证。producer 定义 claim envelope，QA 守证据边界。

## 测试套件目录（S0~S10）

### S0：基础门禁套件（L0）
```bash
./scripts/doc-governance-check.sh
./scripts/check-script-executable-bits.sh
./scripts/cargo-dev-lib.test.sh
./scripts/check-rust-file-size.sh
env -u RUSTC_WRAPPER cargo fmt --all -- --check
```
- S0 是适用于任何改动的快速静态基线：不运行 Viewer/Bevy、wasm 或任何其他 target-specific 编译；这类验证必须按改动面进入 scoped required（例如 S5）。
- 本地日常迭代若只是为了更快得到开发反馈，默认把同类命令替换为 `./scripts/cargo-dev.sh check/test/run/build ...`；新 task worktree 的 ignored `target` symlink 只是降低误用直接 cargo 时的重复存储，不改变正式验收入口。但进入正式 required/full 验收、尤其是 deterministic wasm/release 相关链路时，仍以本手册列出的原始 cargo 命令为准。
- 本地脚本内的开发态 build/test/run 应通过 `scripts/cargo-dev-lib.sh` 复用 shared target；正式验收脚本、release workflow、deterministic wasm、module release acceptance、hash/receipt evidence 仍不得被该 helper 隐式改写。
- `./scripts/check-rust-file-size.sh` 现同时校验超限基线、`touch-and-shrink` 和 `split_part/include!` 结构切片基线，不再只是“有没有新 >1200 文件”。
- 可选（按需执行 builtin wasm hash 校验）：
```bash
./scripts/sync-m1-builtin-wasm-artifacts.sh --check
./scripts/sync-m4-builtin-wasm-artifacts.sh --check
./scripts/sync-m5-builtin-wasm-artifacts.sh --check
```
- 本地策略（2026-03-08 起）：
  - 主 CI 仅允许 `--check`；生产发布清单写入与激活由发布节点链上流水完成。
  - 本地非 `--check` 仅允许显式维护清单（需设置 `OASIS7_WASM_SYNC_WRITE_ALLOW=local-dev`），不属于生产发布路径。
  - `CI=true` 不再作为生产发布写入/激活授权条件；CI 产物仅用于开发回归和可审计对账证据。

### S0.5：本地提交 commit baseline（L0 + 轻量 support/viewer）
```bash
./scripts/ci-tests.sh commit
```
- 覆盖重点：
  - 文档治理、release 关键脚本执行位、Rust 文件体量、`cargo fmt --check`
  - `oasis7_consensus` / `oasis7_distfs` 轻量 support crate
  - `software_safe` feedback contract regression
- 边界：
  - 普通 `pre-commit` 是静默 no-op；这一层仅由操作者显式运行，不是自动提交门禁。
  - 这一层不包含 `cargo test -p oasis7 --tests --features test_tier_required`。
  - 这一层也不包含 `cargo test -p pixel_world_bridge --lib` 与 `cargo check -p pixel_world_bridge --target wasm32-unknown-unknown`；若改动命中 Viewer/Bevy 或 pixel-world wasm/build 链路，显式执行 S5。

### S1：核心 required 套件（L1）
```bash
./scripts/ci-tests.sh required
```

PR-ready lifecycle does not rerun S1 locally. The frozen draft candidate's
GitHub `required-gate` supplies the trusted exact-head CI receipt; S1 remains an
explicit diagnostic command.
- 覆盖重点：
  - runtime/simulator 大量单元与集成测试
  - `oasis7_viewer_live` 二进制测试
  - viewer offline integration
  - 分布式基础子系统（轻量）：`oasis7_consensus`、`oasis7_distfs`
  - release 关键脚本执行位前置校验，避免只在 `release-gate-web` / `package-native` runner 上才暴露 `Permission denied`
  - full support-crate shard 覆盖 `pixel_world_bridge` lib tests；目标特定 wasm/build 检查仍按 S5 scoped required 触发
  - 适用场景：PR/CI required gate，以及本地 landing 前需要显式补跑 `oasis7 --tests` required shard 的改动

### S2：核心 full 套件（L1 + L2）
```bash
./scripts/ci-tests.sh full
```
- 相对 S1 增量：
  - `test_tier_full`
  - `wasmtime` 路径
  - `viewer_live_integration`
  - `oasis7_node --lib`、`oasis7_net --lib`
  - `oasis7_net` 的 `libp2p` 路径
  - `llm-baseline-fixture-smoke`（基线加载与离线治理续跑断言）

### S3：应用主链定向套件（L1 + L2）
```bash
env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_required runtime::tests:: -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_required simulator::tests:: -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_required viewer::live::tests:: -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_required viewer::web_bridge::tests:: -- --nocapture
```
- 电价/市场机制定向回归（required/full）：
```bash
env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_required simulator::tests::power::power_buy_zero_price_ -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_required simulator::tests::power::power_order_ -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_full simulator::tests::power:: -- --nocapture
```
- 主链 Token / NodePoints 桥接定向回归（required/full）：
```bash
./scripts/main-token-regression.sh required
./scripts/main-token-regression.sh full
```
- 当前运行与审计权威：`doc/p2p/token/mainchain-token-allocation-mechanism.prd.md`。
- 用途：
  - 快速定位 `oasis7` 内部模块回归，不必每次跑全套 full。

### S4：分布式子系统套件（L3）
```bash
env -u RUSTC_WRAPPER cargo test -p oasis7_node
env -u RUSTC_WRAPPER cargo test -p oasis7_distfs
env -u RUSTC_WRAPPER cargo test -p oasis7_consensus
env -u RUSTC_WRAPPER cargo test -p oasis7_net --lib
env -u RUSTC_WRAPPER cargo test -p oasis7_net --features libp2p --lib
```
- 可选增强（涉及 runtime_bridge 改动时）：
```bash
env -u RUSTC_WRAPPER cargo test -p oasis7_net --features runtime_bridge --lib
```

### S4B：Governance Registry Drill（L4 前置 / L4）
- 适用场景：
  - 治理 signer rotation / revocation / failover runbook 首轮验证
  - `MAINNET-2` / `BENCH-G1` 需要留下 `pass + block` 审计证据时
- 推荐入口：
```bash
./scripts/governance-registry-drill.sh \
  --source-world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world \
  --baseline-manifest /path/to/public_manifest.json \
  --slot-id msig.foundation_ops.v1 \
  --replace-signer-id signer03 \
  --replacement-public-key <replacement_public_key_hex> \
  --out-dir output/governance-drills/<run_id>
```
- finality slot 示例：
```bash
./scripts/governance-registry-drill.sh \
  --source-world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world \
  --baseline-manifest /path/to/public_manifest.json \
  --slot-id governance.finality.v1 \
  --replace-signer-id signer03 \
  --replacement-signer-id signer04 \
  --replacement-public-key <replacement_public_key_hex> \
  --out-dir output/governance-drills/<run_id>
```
- 统一持久大世界的 committed execution-state 正式证据入口：
```bash
./scripts/governance-registry-live-drill.sh \
  --source-world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world \
  --baseline-manifest /path/to/public_manifest.json \
  --slot-id governance.finality.v1 \
  --replace-signer-id signer02 \
  --replacement-signer-id signer05 \
  --replacement-public-key <replacement_public_key_hex> \
  --out-dir output/governance-drills/<run_id>
```
- finality multi-signer loss / rejoin 示例：
```bash
./scripts/governance-registry-live-drill.sh \
  --source-world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world \
  --baseline-manifest /path/to/public_manifest.json \
  --slot-id governance.finality.v1 \
  --replace-signer-id signer02 \
  --replacement-signer-id signer05 \
  --block-remove-signer-id signer01 \
  --block-remove-signer-id signer02 \
  --replacement-public-key <replacement_public_key_hex> \
  --out-dir output/governance-drills/<run_id>
```
- finality non-baseline rejoin 示例：
```bash
./scripts/governance-registry-live-drill.sh \
  --source-world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world \
  --baseline-manifest /path/to/public_manifest.json \
  --slot-id governance.finality.v1 \
  --replace-signer-id signer02 \
  --replacement-signer-id signer05 \
  --replacement-public-key <replacement_public_key_hex> \
  --out-dir output/governance-drills/<run_id>
```
- finality baseline rejoin 示例：
```bash
./scripts/governance-registry-live-drill.sh \
  --source-world-dir output/chain-runtime/viewer-live-node/reward-runtime-execution-world \
  --baseline-manifest /path/to/public_manifest.json \
  --slot-id governance.finality.v1 \
  --pass-manifest-mode baseline \
  --replace-signer-id signer02 \
  --out-dir output/governance-drills/<run_id>
```
- 产物约定：
  - `run_config.json`
  - `summary.json`
  - `summary.md`
  - `manifests/{rotated_pass_manifest.json,degraded_block_manifest.json}`
  - `logs/*`
  - live-world 额外包含 `world-backup-pre-drill/*`
- 判定口径：
  - baseline / pass case 应返回 `overall_status=ready_for_ops_drill`
  - negative block case 可能有两种合法阻断结果：
    - `audit_failover_gate`: `block_import_rc=0` 且 `overall_status=failover_blocked`
    - `import_policy_reject`: `block_import_rc!=0` 且后续对 block manifest 的审计表现为 `manifest_mismatch`
  - 若 `block_enforcement_stage=audit_failover_gate`，脚本还会继续产出 `rejoin_case`；其期望结果是 `overall_status=ready_for_ops_drill`
  - `pass_manifest_mode=baseline` 适用于 temporary offline / same-signer rejoin；`pass_manifest_mode=rotate` 适用于 replacement / revocation 恢复
  - clone-world 样本只证明 runbook/tooling 正确，不替代统一持久大世界 committed execution-state 的最终 QA 证据
  - `governance-registry-live-drill.sh` 会在真实默认 execution-state 上自动执行 `baseline -> pass -> block -> restore`
  - 当 block case 仍可导入时，`governance-registry-drill.sh` / `governance-registry-live-drill.sh` 会额外执行 `rejoin`
  - controller slot 可保持原 `signer_id` 仅替换公钥；`governance.finality.v1` 不行，必须显式传入新的 `--replacement-signer-id`
  - `--block-remove-signer-id` 可重复使用；当 block manifest 让 `finality signer_count < threshold` 时，默认预期是 `import_policy_reject`
  - 若对 finality slot 复用原 `signer_id`，真实导入会命中 `GovernancePolicyInvalid`，因为 finality signer 绑定到现有 node identity

### S5：Pixel World Bridge（Bevy）单测与 wasm 编译套件（L4A 前置）
```bash
env -u RUSTC_WRAPPER cargo test -p pixel_world_bridge --lib
env -u RUSTC_WRAPPER cargo check -p pixel_world_bridge --target wasm32-unknown-unknown
```
- 说明：
  - `pixel_world_bridge` 是当前 Bevy/WebGL2 pixel-world 目标；`oasis7_viewer` 是其 JS/HTML bundle 消费方，不是 Rust Viewer/Bevy crate；
  - 这是 Viewer/Bevy 或 pixel-world wasm build 改动的 scoped required 稳定性筛网，并且是 UI 闭环前的稳定性前置；
  - raster、真实浏览器与性能证据不由本套件替代，按 S6 的 JS-browser / JS-full 升级。

### S6：Web UI 闭环 smoke 套件（L4A）
- JS-required（结构/反馈/Vitest/freshness/build）：触达 `crates/oasis7_viewer/**` 的 source、生成物边界或 bundle 时，执行 `npm --prefix crates/oasis7_viewer run test:frontend-structure`、`npm --prefix crates/oasis7_viewer run test:feedback-contract`、`npm --prefix crates/oasis7_viewer run test:ui`、`./scripts/agent-browser-viewer-dist-freshness-test.sh` 与 `./scripts/build-viewer-software-safe.sh`。
- JS-browser（真实浏览器）：WASM bundle ready 后，以真实 browser 验证关键交互、console、desktop 与 narrow viewport；任何 player-visible 改动必须通过此层，不能只以 JS-required 替代。
- JS-full（release/risk-triggered）：跨模式、恢复路径、长运行与性能验证；发布或风险触发时执行，包含适用的 raster/browser/performance 证据。
- S6 详细执行步骤、agent-browser 命令、发布门禁与补充约定已拆分到：
  - `doc/testing/manual/web-ui-agent-browser-closure-manual.manual.md`
  - `doc/testing/manual/web-ui-playwright-closure-manual.manual.md`（Playwright 实跑系列入口，管理真实本地栈 + 真实 UI 操作流程矩阵）
  - `doc/testing/manual/web-ui-agent-browser-closure-manual.prd.md`（需求边界/成功标准）
  - `doc/testing/manual/model-visual-review-sop-2026-05-29.manual.md`（截图加模型视觉评审，用于替代 routine 人工视觉 review）
  - `doc/testing/launcher/launcher-manual-test-checklist-2026-03-10.prd.md`（发布前人工体验与异常恢复检查清单）
- 本手册仅保留分层与触发矩阵，执行时按上述文档操作。
- S6 选择器：
  - UI 结构 / 文案 / DOM anchor：优先 `npm --prefix crates/oasis7_viewer run test:ui` 或对应 contract test。
  - 可见视觉 / 布局 / 遮挡 / 响应式：先采 desktop/mobile 截图，再执行模型视觉评审卡。
  - formal player surface 可用性：执行 headed/browser S6，证据必须包含截图、console、关键状态与操作结果。
  - playability claim：S6 只能作为 `L4A` 前置；继续游玩倾向需升级到 `L4A/L4B/L5` 对应证据。
- 模式总口径（`PRD-CORE-009`）：
  - `viewer` / `pure_api` 是当前正式玩家访问模式，分别对应默认 Web 入口和纯接口正式入口；`software_safe` 仅保留为 `viewer` 的兼容 alias。
  - `pure_api` 的正式游玩与 headed Web/UI 一样，默认要求 active LLM access；禁用 LLM 后只能做 blocked/observer-debug 诊断，不再计入正式可玩性证据。
  - `player_parity` / `headless_agent` 是 agent provider 的 execution lane；当前 Local Provider provider-backed 主口径必须写成 `agent_decision_source=provider_backed + agent_provider_backend=provider_local_bridge + agent_provider_contract=worldsim_provider_v1 + agent_provider_transport=loopback_http`，远程托管 bridge 则必须显式写成 `agent_provider_transport=remote_https`，`agent_direct_connect/provider_loopback_http` 只保留为兼容 alias，这些字段都不构成额外玩家访问模式。仅做本地 plumbing / UI 闭环时可使用 `provider_local_mock` deterministic loopback bridge；它不调用 LLM、不计费、不限额，但不能替代真实 LLM / parity / release playability evidence。
  - repo-owned `remote_https` 参考装配当前采用 `oasis7_provider_local_bridge` + `scripts/provider-remote-https/letai_provider_cli.py` + `nginx` HTTPS 反代；操作步骤见 `doc/world-runtime/runtime/provider-remote-https-bridge-operator-runbook.md`。
  - 任何 QA / release / playability 结论都应先标明玩家访问模式，再补充 execution lane；不得把 `headless_agent` 直接当成“第三种入口”。
- `oasis7_viewer_live` / Viewer 页面：默认使用 `agent-browser` 驱动页面与采集证据；当 `renderMode=viewer`（或兼容 alias `software_safe`）且带 viewer auth bootstrap 时，允许继续验证选中 Agent 的最小 `prompt/chat` 闭环。
- 需要验证真实玩家输入流程、真实 provider、真实本地栈的一整条 Playwright 闭环时，进入 `doc/testing/manual/web-ui-playwright-closure-manual.manual.md`。当前首个用例是 `PWT-001 Real Agent Chat`：`./scripts/viewer-real-agent-chat-regression.sh` 会启动本地 LetAI 栈、打开 Viewer、通过可见 UI 聊天输入框和发送按钮发消息、等待真实 Agent 回复，并断言不含 mock 标记；后续所有 Playwright 实跑用例按该手册的 `PWT-###` 矩阵扩展。
- 若只需要回归 `software_safe` 纯实时最小闭环（加载 -> 连接 -> 选择目标 -> 实时事件/语义摘要可见，且页面不再暴露回放控件），优先执行 `./scripts/viewer-software-safe-step-regression.sh`；该脚本不再主动调用 `__AW_TEST__.sendControl('step')`，而是等待 `logicalTime/eventSeq` 自然增长；若当前 runtime 被 `llm_required` 等 gameplay blocker 卡住，则要求页面显式暴露 blocker，而不是再用手动步进补推进。
- 若只想先确认 Web/UI automation tooling 本身没有漂移，而不想起完整 runtime/build，先执行 `./scripts/viewer-software-safe-step-regression-smoke.sh`；它会用临时 fixture 页面复用真 `agent-browser` 与 `viewer-software-safe-step-regression.sh` 验证最小浏览器链路和 summary/state 产物契约，但不替代正式 S6 证据。
- 若需要把 `software_safe` 的 prompt/chat/rollback/message-flow 做成独立 QA smoke，优先执行 `./scripts/viewer-software-safe-chat-regression.sh`；当脚本自举 source stack 并自动启用 `OASIS7_RUNTIME_AGENT_CHAT_ECHO=1` 时，若 QA echo 没有在 `chat ack` 后、无额外 `step/play` 的同一轮交互里进入消息流，会直接判为阻断失败；外部 URL 场景仍默认把 `agent_spoke` 缺失记为可追溯 warning，显式加 `--require-agent-spoke` 时再升级为阻断失败。
- 若用户反馈“Viewer 发卡 / 掉帧”，优先执行 `./scripts/viewer-performance-probe.sh --profile smoke --min-fps 55 --max-frame-p95-ms 20 --max-long-task-count 0`。该链路使用 `crates/oasis7_viewer/scripts/viewer-performance-probe.mjs` + `agent-browser` 直接采集 `requestAnimationFrame` frame timings / FPS、`PerformanceObserver` long tasks（浏览器支持时）、navigation DOM readiness、DOM 规模与截图，并输出 `output/playwright/viewer-performance/<run-id>/summary.json` 与 `summary.md`。
- 若改动只触达 `software_safe` feedback 语义映射而不需要浏览器自举，优先执行 `npm --prefix crates/oasis7_viewer run test:feedback-contract`；该 deterministic contract regression 已纳入 `./scripts/ci-tests.sh required`。
- 若改动触达 `crates/oasis7_viewer/software_safe_src/**` 的结构、Prompt/Chat surface、主入口锚点或移动端分区导航，优先执行 `npm --prefix crates/oasis7_viewer run test:ui`；这套 Vitest + `@solidjs/testing-library` 回归用于验证 repo-owned `World / Targets / Command` 锚点、`Runtime Diagnostics` 降级面、`Agent Chat` 与 `Prompt Overrides` 的 DOM 可达性，不替代 S6 headed browser 证据。
- 只要改动触达可视化相关代码、样式、资源或可见输出，S6 截图采证后默认执行模型视觉评审；典型触发面包括 UI component / DOM / CSS / layout / responsive / canvas / WebGL / pixel-world renderer / visual DTO 映射 / screenshot fixture / 图片资产 / 可见状态文案。`verdict=pass` 且 `confidence=high` 可替代 routine 人工视觉 review；`verdict=watch/block/human_escalation`、`confidence=low` 或对外 claim 影响必须升级人类 owner。输出格式使用 `doc/testing/templates/model-visual-review-card-template.md`；只有明确不影响任何可见 surface 的改动才能豁免，并需在任务日志或 PR evidence 写明理由。
- 若需要稳定触发一条标准 `AgentSpoke` 供消息流验收，在 source runtime 启动前显式设置 `OASIS7_RUNTIME_AGENT_CHAT_ECHO=1`；该开关仅用于 Viewer / QA 测试态，默认产品路径必须保持关闭。
- 若 Viewer 页面长期停在 `connecting` 且 `logicalTime=0`，必须查看 `window.__AW_TEST__.getState().lastError`；命中 `copy_deferred_lighting_id_pipeline` / `CONTEXT_LOST_WEBGL` 等 fatal 时，按图形环境门禁失败处理，不进入玩法结论。
- `headed` 不是充分条件：若 `browser_env.json` / WebGL renderer 显示 `SwiftShader` 或其他 software renderer，先查看 `window.__AW_TEST__.getState().renderMode`。
  - `renderMode=viewer`（或兼容 alias `software_safe`）：允许继续做最小闭环验证（连接、选择目标、自然实时推进；若运行态被 blocker 卡住，则要求 blocker 文案显式可见）。
  - `renderMode` 既不是 `viewer` 也不是兼容 alias `software_safe`：仍按图形环境阻断处理；默认先使用 `--use-angle=gl,--ignore-gpu-blocklist` 固定硬件路径。
- `oasis7_web_launcher` / launcher Web 控制面：默认优先使用 GUI Agent 驱动产品动作，再用 Web 页面做状态与字段校验；Canvas 直点仅作补充。若目标是 `L4A synthetic`，优先走 harness / Web UI 闭环；若目标是 `L4B embodied-agent`，优先执行 `./scripts/run-playability-l4b-agent.sh --l4-manifest <artifact>/manifest.json`，由它内部调用 `./scripts/run-producer-playtest.sh --open-headed` 并自动采集证据，再审阅/补齐 `l4b-agent-playtest-card.md`。若需要内部人类 spot-check，只能沿用同一入口并把结果写进 `optional-internal-human-corroboration.md` 或等价正式卡片，作为 `L4B` 校准附录。仅执行启动脚本、不产出 `L4B` summary/card，不算 `L4B` 完成；如需手动控制 bundle，再使用 `<bundle>/run-game.sh` 或 `./scripts/run-launcher-stack.sh --bundle-dir <bundle>` 启动。
- agent / QA 若只是想在当前 worktree 内起一套隔离回归栈，优先执行 `./scripts/worktree-harness.sh up`，然后通过 `./scripts/worktree-harness.sh url` / `status --json` / `logs` 获取 URL 与状态；`run-launcher-stack.sh` 继续作为该 harness 的底层启动器，不应再被当作并行 worktree 回归的顶层主入口。
- 不要把 Viewer 页面专用的 `agent-browser` 操作步骤直接套用到 launcher 控制面动作执行上。
- 涉及 `Explorer / Transfer` 的闭环时，先准备可观测数据，再执行查询与字段断言；不得只以“页面打开了/接口返回 200”判定通过。
- 防误用约束：
  - `scripts/run-game-test-ab.sh` 仅用于自动化回归哨兵（TTFC/命中率/无进展窗口）；推荐与 `--bundle-dir <bundle>` 搭配做产物态 smoke，但仍不等价于“真实玩家长玩评测”。
- `run-game-test-ab.sh --headless` 若命中 `SwiftShader` / software renderer，应先确认页面是否已自动切到 `software_safe`；只有未切入 safe-mode 时才按环境阻断处理，不得把 `connectionStatus=connecting` 误判为 fresh Web 构建或玩法回归；Viewer Web 默认继续使用 headed 模式。
  - 发布前若只要求 agent 可执行闭环，至少补齐 `L4B` agent 试玩与卡片填写；若要对“真实人类是否想继续玩”下结论，必须进入 `L5` 的真人 / 受控线上样本，内部真人试玩只能作为 `L4B` 的可选校准。
- 若改动影响前期工业引导（`首个制成品 / 停机恢复 / 首座工厂单元`），必须补跑 `doc/playability_test_result/topics/industrial-onboarding-required-tier-cards-2026-03-15.md` 中对应卡片，并把结论回写正式 playability 卡。
  - 对外样张链路需使用 strict 语义门禁，不得以 `off` / `soft` 结果作为发布判定证据。
- 若需要为 `#46 PostOnboarding` 补无 UI / 非浏览器验证，执行 `./scripts/viewer-post-onboarding-headless-smoke.sh`。
  - 该脚本只验证 live TCP 协议、快照推进、控制完成 ack 与 runtime event feed；不替代 headed Web/UI 截图复核。
- 若需要直接以纯 API 客户端操作 live 会话，可使用 `cargo run -q -p oasis7 --bin oasis7_pure_api_client -- ...`。
  - 该链路属于 `pure_api` 玩家访问模式；若同时牵涉 Local Provider provider-backed 路径，应额外标注 `agent_decision_source + agent_provider_backend/contract/transport` 与实际 execution lane；`agent_direct_connect/provider_loopback_http` 只在兼容迁移说明里保留。
  - 推荐最小链路：
```bash
cargo run -q -p oasis7 --bin oasis7_pure_api_client -- --addr 127.0.0.1:5023 snapshot --player-gameplay-only
cargo run -q -p oasis7 --bin oasis7_pure_api_client -- --addr 127.0.0.1:5023 step --count 8 --events
cargo run -q -p oasis7 --bin oasis7_pure_api_client -- keygen
cargo run -q -p oasis7 --bin oasis7_pure_api_client -- --addr 127.0.0.1:5023 reconnect-sync --player-id player-1 --with-snapshot
```
  - 若要覆盖 `agent_chat` / `prompt_control`，需先 `keygen`，再携带 `--player-id` 与 `--private-key-hex` 走签名请求；当前产品设定下，只要 LLM 不可用，`gameplay_action / agent_chat / prompt_control` 会直接返回 `llm_mode_required` 或 `llm_init_failed`，而 `step / play` 会返回 `ControlCompletionAck { status: Blocked, error_code, error_message }`。
- 若需要执行 pure API required/full 回归，优先运行 `./scripts/oasis7-pure-api-parity-smoke.sh`。
  - 该回归验证的是 `pure_api` 玩家访问模式在 active LLM access 下的正式可玩性，不等同于 Local Provider `headless_agent` 回归。
  - required-tier 推荐 bundle 口径：
```bash
./scripts/build-game-launcher-bundle.sh --out-dir output/release/game-launcher-local
./scripts/oasis7-pure-api-parity-smoke.sh --tier required --bundle-dir output/release/game-launcher-local --with-llm
```
  - full-tier 抽样：
```bash
./scripts/oasis7-pure-api-parity-smoke.sh --tier full --bundle-dir output/release/game-launcher-local --with-llm
```
  - 结果说明：
    当前脚本已覆盖 `player_gameplay`、正式 `gameplay_action` 推进、`reconnect-sync --with-snapshot` 恢复，以及 `FirstSessionLoop -> PostOnboarding -> establish_first_capability|stabilize_first_line_after_output|choose_midloop_path` 的 required/full 收口路径。
    `parity_verified` 当前以 `doc/testing/evidence/pure-api-shared-player-gameplay-parity-2026-04-28.md` 为准；旧 `doc/testing/evidence/pure-api-parity-validation-2026-03-19.md` 只保留历史输入，不再作为 no-LLM 正式可玩性的现行依据。
- 本地试玩入口瘦身口径：普通操作者只需要按意图记住 `2 + 1` 个入口；底层脚本仍保留给 wrapper、排障和专项回归使用。
  - 入口语义先分流：

| 意图 | 入口 | 连接的 world state | 不能声明 |
| --- | --- | --- | --- |
| 纯本地试玩 / local-only gameplay | `./scripts/run-local-letai-game-test.sh --local-world-playtest` | local-standalone-chain | formal/public testnet connected |
| 本地接入 public_testnet 测试环境 | `./scripts/run-local-public-testnet-letai-test-environment.sh` | formal `public_testnet` node + submit-capable endpoint | 仅因页面打开就声明 public_testnet ready |

  - 本地真实 LetAI bridge + runtime/game：
```bash
./scripts/run-local-letai-game-test.sh --local-world-playtest
```
    常规本地真实 LLM 游戏测试统一从 `scripts/run-local-letai-game-test.sh --local-world-playtest` 启动；这个入口只代表完全本地的 provider/runtime/viewer/local-standalone-chain 试玩，不连接 formal/public testnet。若目标是“本地入口读取/提交 formal `public_testnet` world state”，不要使用这个 preset，改走 `scripts/run-local-public-testnet-letai-test-environment.sh` 和对应 runbook。每次纯本地试玩都应优先使用这个 preset，而不是手工分别启动 provider bridge / launcher / viewer binary，或展开端口、detach、reuse、provider smoke、chain profile 等低层参数。该 preset 固化 playtest startup、provider smoke skip、reuse existing source build、后台启动、手动 Play、viewer/web/live 标准端口 `48420/48421/48422`、`--json-ready` 与 wrapper 默认的 local standalone chain。脚本仍默认优先读取 `OASIS7_LETAI_CONFIG_PATH`，再使用 `/Users/scc/Documents/keys/letai.txt`，最后回退到 `OASIS7_LETAI_TOKEN_CONFIG_PATH` 或 `/Users/scc/Documents/keys/letai-token-local.txt`，设置本地代理默认值，规范化临时 project token config，转发 `--auto-topup-usd`、`OASIS7_REMOTE_LLM_PLATFORM_KEY`、`OASIS7_REMOTE_LLM_PLATFORM_USER_ID` 等真实 provider 必需环境，后台启动默认 `rust-direct-letai` 的 `127.0.0.1:5841` Rust provider bridge，再启动 `run-launcher-stack.sh` 指向该 bridge；额外 launcher 参数仅在高级排障时放在 `--` 之后。该 wrapper 默认保持 `OASIS7_RUNTIME_AGENT_CHAT_ECHO` 关闭，使本地 playtest 遵循真实 provider-backed 路径；只有做低层 receipt/debug 验证时才显式传 `--chat-echo`。若直接跑已编译二进制或手工拼装脚本，操作者必须自行保证这些 env 与 wrapper 完全等价，否则可能出现不自动充值、token/project 绑定不一致、或 mock/receipt 路径误入本地试玩证据。需要查完整调试参数时，用 `./scripts/run-local-letai-game-test.sh --help-all`。
  - 制作人 / 发布前人工验收：
```bash
./scripts/run-producer-playtest.sh --open-headed
```
    这是 bundle-first 人工试玩入口。脚本会自动准备或复用 fresh bundle，再打开 headed browser；若只想起栈并手动复制 URL，可省略 `--open-headed`。
  - QA / subagent evidence：
```bash
./scripts/worktree-harness.sh up
./scripts/worktree-harness.sh status --json
GAME_URL="$(./scripts/worktree-harness.sh url)"
./scripts/run-game-test-ab.sh --url "$GAME_URL"
```
    `worktree-harness.sh` 是当前 worktree 的隔离 L4A synthetic 栈入口；`run-game-test-ab.sh` 是 TTFC / 控制命中率 / 无进展窗口哨兵，不替代真人试玩或制作人验收。
  - 高级 / 底层入口（仅用于 wrapper、排障或专项回归，不作为普通试玩菜单）：
```bash
./scripts/build-game-launcher-bundle.sh --out-dir output/release/game-launcher-local
./scripts/check-active-llm-provider.sh --pretty
./scripts/run-launcher-stack.sh --bundle-dir output/release/game-launcher-local --with-llm
./scripts/viewer-post-onboarding-qa.sh --bundle-dir output/release/game-launcher-local --with-llm
./scripts/viewer-post-onboarding-headless-smoke.sh --bundle-dir output/release/game-launcher-local --with-llm
./scripts/viewer-software-safe-chat-regression.sh --bundle-dir output/release/game-launcher-local
cargo run -q -p oasis7 --bin oasis7_pure_api_client -- --addr 127.0.0.1:5023 snapshot --player-gameplay-only
./scripts/oasis7-pure-api-parity-smoke.sh --tier required --bundle-dir output/release/game-launcher-local --with-llm
```
  - active-LLM / provider 预检：
    `run-launcher-stack.sh` 仍是底层 launcher/runtime bootstrap。它会在启动 launcher 前先跑一次 active LLM provider probe，复用同一套 `config.toml` / `OASIS7_LLM_*` 配置，并同时验证 Responses hello 文本响应与 required tool-call 合约；若 provider/model/auth/base URL 当前不可用，或模型能回文本但不能稳定返回 tool call，会直接 fail-fast，不再等到首个 `step` 才暴露。需要故意保留“stack 可启动但 formal lane 在首步 blocked”的负向验证时，显式加 `--skip-llm-provider-preflight`。
    若本地只验证 provider-backed plumbing，可先启动 `oasis7_provider_local_bridge --mode mock`，再用 `run-launcher-stack.sh --agent-provider-lane local-mock` 指向 deterministic `provider_local_mock`。
    若本地需要 builtin LLM 直连验证，`scripts/with-letai-llm-config.sh` 只适用于已经包含 chat-completions/Responses-compatible `token_key` / `api_key` 的配置；不要把只有 `Doc` 和平台管理 `Key` 的文件直接当 inference token 使用。platform-only 配置应先通过 `scripts/ensure-letai-local-token-config.sh` 或 `scripts/run-local-letai-game-test.sh` 规范化成临时 project token config；不得把 raw key、生成的 `letai-local-token.env`、或其它 token config 写入日志、仓库文件或 public evidence。
    LetAI provider bridge 参考装配走 `/chat/completions`，与 builtin LLM 的 Responses preflight 不是同一协议；`scripts/run-local-letai-provider-bridge.sh` 是 `run-local-letai-game-test.sh` 使用的底层 bridge 入口，也可用于专项排障，随后用 `provider-bridge-contract-smoke.sh` 验证真实 provider decision。普通本地试玩不要从这个底层入口开始；应回到 `run-local-letai-game-test.sh`，让 wrapper 统一处理 token config、auto-topup 和 launcher 参数。`scripts/check-letai-chat-completions.sh` 和 `--chat-probe-backend legacy-cli` 只保留给兼容排障或对比 Python legacy adapter，不作为普通本地试玩默认 gate。
    本地真实 Rust provider bridge 默认在上游返回 `insufficient_user_quota` 且同时存在 `OASIS7_REMOTE_LLM_PLATFORM_KEY` / `OASIS7_REMOTE_LLM_PLATFORM_USER_ID` 时按 `$0.1`（`quota=50000`）自动充值；这是 paid real-provider 行为。充值触发按 chat request 发生，随后使用有限次数延迟重试来等待 LetAI 余额更新可见，而不是全局每次运行只充值一次；没有这些管理面字段时不会自动充值，仍应把 quota error 作为失败证据。

### S7：场景矩阵回归套件（L1 + L4）
```bash
env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_required scenario_specs_match_ids -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_required scenarios_are_stable -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_full oasis7_init_demo_runs_ -- --nocapture
```
- 配套文档：`doc/world-simulator/scenario/scenario-files.prd.md` 的“场景测试覆盖矩阵”。

### S6.5：Chain Runtime Storage Profile / Gate 技术核验
```bash
env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime node_runtime_execution_driver_uses_storage_profile_checkpoint_interval -- --nocapture
./scripts/oasis7-runtime-storage-gate.sh --status-json <status.json> --expected-profile release_default --min-checkpoint-count 1 --max-orphan-blob-count 0 --require-no-degraded
OASIS7_CHAIN_STORAGE_PROFILE=release_default bash -x <bundle>/run-game.sh --help
OASIS7_CHAIN_STORAGE_PROFILE=soak_forensics bash -x <bundle>/run-web-launcher.sh --help
OASIS7_CHAIN_STORAGE_PROFILE=dev_local bash -x <bundle>/run-chain-runtime.sh --help
```
- 说明：
  - 用于 `TASK-WORLD_RUNTIME-033` 的 storage profile / storage gate / bundle wrapper 一致性核验；
  - 若验证真实 `release_default` cadence，优先对比 `<64` 与 `>=64` 两个采样点，确认 `full_log_only -> checkpoint_plus_log` 切换；
  - 若验证 bundle 入口，只需保留 `bash -x` trace 作为“wrapper 实际注入了正确 profile 参数”的证据；
  - 参考证据：
    - `doc/world-runtime/evidence/runtime-storage-gate-sample-2026-03-10.md`
    - `doc/world-runtime/evidence/runtime-sidecar-orphan-gc-failsafe-2026-03-11.md`
    - `doc/world-runtime/evidence/runtime-launcher-profile-consistency-2026-03-11.md`

### S8：长稳与压力技术套件
- Viewer 当前 Web 性能 probe（当前活跃入口）：
```bash
./scripts/viewer-performance-probe.sh --profile smoke
./scripts/viewer-performance-probe.sh --profile release --duration-ms 8000
```
- Runtime 轻量 deterministic perf harness（当前先覆盖稳定内环 module routing）：
```bash
./scripts/runtime-module-routing-perf-harness.sh
```
- LLM 长稳：
```bash
./scripts/llm-longrun-stress.sh --with-local-letai-provider-bridge --scenario llm_bootstrap --ticks 240
```
- LLM 覆盖门禁（发行口径）：
```bash
./scripts/llm-longrun-stress.sh --with-local-letai-provider-bridge --scenario llm_bootstrap --ticks 240 --release-gate --release-gate-profile hybrid
```
- LLM gameplay 对照（bridge 开/关）：
```bash
./scripts/llm-longrun-stress.sh --with-local-letai-provider-bridge --scenario llm_bootstrap --ticks 240 --prompt-pack story_balanced --runtime-gameplay-bridge
./scripts/llm-longrun-stress.sh --with-local-letai-provider-bridge --scenario llm_bootstrap --ticks 240 --prompt-pack story_balanced --no-runtime-gameplay-bridge
```
- git 跟踪基线 fixture smoke（`test_tier_full`）：
```bash
./scripts/llm-baseline-fixture-smoke.sh
```
- Prompt 切换覆盖对比（定向排障）：
```bash
./scripts/llm-switch-coverage-diff.sh --log <run.log> --switch-tick 24
```
- 说明：
  - 详细参数与 profile 组合请以 `./scripts/llm-longrun-stress.sh --help` 为准；
  - `llm-longrun-stress.sh` 是 provider-backed LLM 长跑入口；`--no-llm-io` 只关闭 raw LLM I/O 日志，不会禁用 provider。日常本机 LetAI 长跑优先使用 `--with-local-letai-provider-bridge`，让脚本生成临时 token config、继承本地代理并启动本地 Rust provider bridge，保留 auto-topup；若确实要复用已启动 bridge，必须显式传 `--reuse-local-provider-bridge` 接受既有 bridge 的 config/token 状态。`--with-letai-config` 只保留给直连 Responses API 的低层排障；
  - 多场景评测固定 scenario/fixture、profile、provider/adapter 与协议版本、timeout、tick budget 和 `--jobs`，并保留每场景及聚合工件；单次样本、总量或并行运行本身都不证明 provider parity、成本稳定性或 release readiness。历史多场景/工业调试样本仅供追溯，当前语义以 `doc/world-simulator/llm/decision-provider-contract.prd.md` 和相应 gameplay/runtime authority 为准；
  - `./scripts/runtime-module-routing-perf-harness.sh` 现在走专用 bin `cargo run -p oasis7 --bin oasis7_runtime_module_routing_perf`，避免继续依赖 ignored test 的重编译路径；它会把稳定内环 module routing 的 event/action 平均耗时写到 `.tmp/runtime_module_routing_perf/<run>/summary.{json,md}`；当前目的是提供日常可回归的轻量 runtime perf 入口，不替代长窗 `runtime_perf.tick.*` 指标；
  - 当前已记录首个 `release` baseline 数值：`modules=192`、`iterations=80`、`event_avg_ms=5.591`、`action_avg_ms=6.992`；对应一次本地验证 run 的冷 `release` 编译耗时约 `23m 10s`。原始 `summary.json` 属于本地临时产物，应以 task execution log 中登记的验证记录为证据来源；因此当前更适合先作为本地/report-only 基线，而不是立刻升成默认 blocking gate；
  - Viewer 性能 probe 使用当前 `crates/oasis7_viewer` canonical `viewer` Web 入口，通过 `agent-browser` 采集 rAF frame timing、Long Task、ready time、DOM 规模与 gate 结果，并输出 `output/playwright/viewer-performance/<run>/summary.{json,md}`；
  - 旧 `viewer-owr4-stress` 属于历史已删除入口，不再作为当前 Viewer 性能门禁真值；
  - `scripts/ci-tests.sh full` 已接入 `./scripts/llm-baseline-fixture-smoke.sh`；
  - 压测结果需保留 CSV/summary/log 产物。

### S9：P2P/存储/共识在线长跑技术套件
- 当前状态（2026-02-28）：`scripts/p2p-longrun-soak.sh` 已恢复为可执行脚本，底座为多进程 `oasis7_chain_runtime`。
- 时间语义说明：PoS 出块/提案节拍由 `--pos-slot-duration-ms` 与 `--pos-ticks-per-slot` 锚定；`--node-tick-ms` 仅表示 worker 轮询/回退间隔。
- 建议命令（smoke）：
```bash
./scripts/p2p-longrun-soak.sh --profile soak_smoke --topologies triad --duration-secs 600 --no-prewarm
```
- 建议命令（endurance + chaos）：
```bash
./scripts/p2p-longrun-soak.sh --profile soak_endurance --topologies triad_distributed --chaos-continuous-enable --chaos-continuous-interval-secs 30 --chaos-continuous-max-events 60
```
- 建议命令（endurance + chaos + feedback）：
```bash
./scripts/p2p-longrun-soak.sh --profile soak_endurance --topologies triad_distributed --duration-secs 900 --chaos-continuous-enable --chaos-continuous-interval-secs 30 --chaos-continuous-max-events 30 --feedback-events-enable --feedback-events-start-sec 30 --feedback-events-interval-secs 60 --feedback-events-max-events 12
```
- 发布门禁基线命令（2026-02-28，300s）：
```bash
./scripts/p2p-longrun-soak.sh --profile soak_release --topologies triad_distributed --duration-secs 300 --no-prewarm --max-stall-secs 240 --max-lag-p95 50 --max-distfs-failure-ratio 0.1 --chaos-continuous-enable --chaos-continuous-interval-secs 30 --chaos-continuous-start-sec 30 --chaos-continuous-max-events 8 --chaos-continuous-actions restart,pause --chaos-continuous-seed 1772284566 --chaos-continuous-restart-down-secs 1 --chaos-continuous-pause-duration-secs 2 --out-dir .tmp/release_gate_p2p
```
- 通过标准：
  - 命令返回 `rc=0`；
  - `summary.json` 中 `overall_status == "ok"` 且 `totals.topology_failed_count == 0`；
  - `soak_release` 档位下 `topologies[].metric_gate.status` 必须为 `pass`（`insufficient_data` 会转失败）；
  - `topologies[].metrics.consensus_hash_consistent` 必须为 `true`，且 `consensus_hash_mismatch_count == 0`（若失败需检查 `topology/.consensus_hash_mismatch.tsv`）；
  - 如启用 chaos，`chaos_events.log` 与 `summary.json.totals.chaos_events_total` 一致。
  - 如启用 feedback events，`summary.json.totals.feedback_events_total == summary.json.totals.feedback_events_success_total + summary.json.totals.feedback_events_failed_total`，且 `feedback_events.log` 中 `phase=completed/failed` 事件数量与 `feedback_events_total` 一致。
- reward-runtime 指标口径：S9/S10 只消费 `oasis7_chain_runtime` 的
  `/v1/chain/status.reward_runtime` 原生快照，禁止以兼容字段回推。每个可用节点
  先取累计最大值再聚合；缺失节点必须留告警，status 不可达记为 HTTP failure 并
  终止门禁判定。`distfs_total_checks=0` 为 `insufficient_data`，不得与比例超阈值
  混写；仅 `reward_runtime.invariant_ok=false` 可触发
  `reward_asset_invariant_violation`，`running_false/http_failure` 必须保留为独立
  失败分类，即使发生于 chaos 恢复窗口。
- 漂移定位/回滚演练门禁（TASK-GAME-014）：
```bash
env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_required runtime::tests::persistence::rollback_with_reconciliation_recovers_from_detected_tick_consensus_drift -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_required runtime_authoritative_recovery_rollback -- --nocapture
```
- 演练通过标准：
  - 能定位 `mismatch_tick`；
  - `rollback_to_snapshot_with_reconciliation` 后 `first_tick_consensus_drift() == None`；
  - `verify_tick_consensus_chain()` 通过。
  - Viewer 缺少签名信封返回 `rollback_approval_required`；篡改、过期、目标不匹配或重放信封返回 `rollback_authorization_invalid`，且两类失败均不得改变 world、journal、batch 或 reorg epoch。
- 参考文档：`doc/testing/longrun/p2p-longrun-soak-and-chaos.prd.md`、`doc/testing/longrun/game-world-state-sync-commit-closure-2026-06-26.prd.md`。

### S9A：链上大世界状态底座自闭环
- 目标语义：本节的闭环对象不是单独的 libp2p/P2P transport，而是 `链上大世界状态底座`：P2P transport、分布式存储/blob closure、replication/gap sync/state sync、consensus/finality、execution record/receipt、observer/validator/storage ops，以及 API/viewer 对同一 world state 的投影。
- Claim boundary / 状态提交闭环首读入口：`doc/testing/longrun/game-world-state-sync-commit-closure-2026-06-26.prd.md`。涉及 world state sync、commit closure、state-sync bundle、observer catch-up 或 API/viewer projection 的结论，先按 GWSC 口径定档，再下钻 S9/S10 执行套件。
- 可声明结论：
  - `module_required` 通过：底座本地合同可集成。
  - `module_full` 通过：底座在当前可执行 proxy/triad 拓扑下能持续推进和恢复。
  - `integration_required` 通过：真实大世界状态接到底座后，`action -> consensus -> execution -> receipt -> world state -> API/viewer` 没有明显漂移。
  - `release_full` 通过：真实环境具备公开测试候选所需的同窗口节点、manifest、readiness lane 与 claims-boundary 证据。
- 禁止声明：
  - S4/S9B required 绿，不得声明真实公网可达、physical NAT/CGNAT 已覆盖、`public_testnet ready` 或游戏整机体验成立。
  - S9/S9B proxy 绿，不得冒充 dedicated sentry/NAT lab、真实公网或 public testnet 证据。
  - 手工 copy validator `data/`、checkpoint 或 seed 只能作为 break-glass/recovery 证据，不得作为 live-candidate readiness。
- 开发/验证阶梯：
```text
Phase 0 contract inventory:
  world_id / chain_id / genesis / manifest
  action payload / consensus payload
  execution record / receipt / state hash
  blob/store closure
  peer head / gap sync / checkpoint
  validator / storage / observer role
  /v1/chain/status observability

Phase 1 single-node execution:
  action -> execution record -> receipt -> state hash
  replay same input -> same result
  rollback/checkpoint recovery

Phase 2 deterministic substrate contracts:
  oasis7_node / oasis7_net / oasis7_consensus / oasis7_distfs
  S9B required exact matrix

Phase 3 proxy multi-node substrate:
  S9 triad / triad_distributed soak
  chaos restart/pause/disconnect
  consensus hash, peer heads, gap sync, blob closure

Phase 4 state-sync / observer:
  high-head checkpoint or seed closure
  observer automatic catch-up
  storage blob availability and historical backfill

Phase 5 real-env ops:
  live local_peer_id, runtime sha256, service health
  connected peers, height/head progress, resource/traffic/wasm observability

Phase 6 world-state integration / release:
  S10 five-node real game soak
  public_testnet readiness lanes + same-window real-env evidence
```
- `module_required` 推荐命令：
```bash
./scripts/game-world-state-sync-commit-module-required.sh
```
- 等价展开命令：
```bash
env -u RUSTC_WRAPPER cargo test -p oasis7 --tests --features test_tier_required
env -u RUSTC_WRAPPER cargo test -p oasis7_node
env -u RUSTC_WRAPPER cargo test -p oasis7_net --lib
env -u RUSTC_WRAPPER cargo test -p oasis7_net --features libp2p --lib
env -u RUSTC_WRAPPER cargo test -p oasis7_consensus --lib
env -u RUSTC_WRAPPER cargo test -p oasis7_distfs --lib
./scripts/p2p-mixed-topology-matrix.sh --tier required
```
- `module_full` 推荐命令：
```bash
./scripts/p2p-longrun-soak.sh --profile soak_smoke --topologies triad --duration-secs 600 --no-prewarm
./scripts/p2p-mixed-topology-matrix.sh --tier full
```
- state-sync / blob closure 推荐命令：
```bash
./scripts/p2p-verify-state-sync-closure.sh \
  --world-dir <seed-world-dir> \
  --execution-records-dir <seed-execution-records-dir> \
  --store-dir <seed-store-dir>
```
- `module_full` state-sync closure 证据包模板：`doc/testing/templates/state-sync-closure-evidence-packet-template.md`。复制模板到实际 evidence path 后回填；模板文件本身不能作为 pass evidence。
- real-env / release 入口：
```bash
./scripts/p2p-real-env-triad-snapshot.sh ...
./scripts/p2p-real-env-observability-monitor.sh ...
./scripts/s10-five-node-game-soak.sh --duration-secs 300 --no-prewarm --max-stall-secs 240 --max-lag-p95 50 --out-dir .tmp/release_gate_s10
./scripts/network-tier-public-testnet-readiness.sh --manifest <manifest> --lanes-tsv <lanes.tsv>
```
- S10 `summary.json` 固定输出 `api_viewer_projection`，S10 `summary.md` 固定输出 `API / Viewer Projection Contract`。默认状态是 `not_collected`，只有补齐同窗口 API/viewer evidence refs 后才能声明 projection 已验证。
- public_testnet readiness 必须包含 `api_viewer_projection_ready` lane；若缺失该 lane，或 pass evidence 仍指向 template/placeholder，或缺少 `api_viewer_projection.status=pass`、同窗口 refs 与 `world_state_projection_match=true` 的 JSON 证据，则不得声明 `ready_for_live_candidate`。
- 最小验收指标：
  - `committed_height` 单调推进；
  - `consensus_hash_consistent == true` 且 `consensus_hash_mismatch_count == 0`；
  - execution/state hash 与 receipt/event sequence 可追溯；
  - peer heads 非空且新鲜；
  - gap sync 成功且 replication error 不持续；
  - blob/store closure 完整；
  - observer 能自动追高；
  - checkpoint / rollback / state-sync 可恢复；
  - `/v1/chain/status` 能解释当前 readiness、degraded 或 blocker。
- 阻断签名：
  - `consensus_hash_divergence`；
  - `committed_height_not_monotonic`；
  - `known_peer_heads_zero_samples`；
  - `http_failure_samples`；
  - real-env 中 `sequencer_committed_height_zero`、`sequencer_execution_stale_height` 或同类 stale execution / stale peer-head；
  - readiness lane 为 `partial` / `block`；
  - manifest 仍是 example/template/placeholder/private-only endpoint；
  - 非同窗口 real-env 证据；
  - 通过手工复制数据目录获得的同步假象。

### S9B：P2P Mixed-Topology Matrix（P2PARCH-6）
- 当前状态（2026-04-07）：`scripts/p2p-mixed-topology-matrix.sh` 已把 `P2PARCH-6` 收口成一个可执行矩阵，区分 `exact` 与 `proxy` 两类覆盖，并把 shared-window / dedicated-lab / pass-uplift 外部证据与 blocker 语义写进 `summary.json`。
- 目标语义：
  - `exact`：直接运行当前仓库已经存在的 deterministic cargo tests，验证 private/validator_hidden/relay_only 边界、bootstrap poisoning、relay exhaustion 和 path failover。
  - `proxy`：运行当前可用的 triad/triad_distributed longrun 命令，给 mixed-topology recovery 留下可执行 full-tier drill；它不等价于 dedicated sentry/NAT lab。
- Path behavior taxonomy（2026-06-15 更新）：
  - `evidence_class`: `exact / proxy / manual_lab / real_env / unsupported`。`exact` 是 deterministic repo test；`proxy` 是当前可执行近似 longrun；`manual_lab` 需要专门 NAT/sentry lab；`real_env` 必须是同窗口真实网络证据；`unsupported` 表示该 topology 当前明确不承诺。
  - `path_expectation`: `must_direct / may_direct_must_recover / must_relay / must_not_publish_public_direct / manual_lab_required / unsupported`。该字段描述粗粒度 claim，不要求所有 NAT pair 都 direct-connect；case-specific route sequence 留在 `expected_route`。
  - `reachability_pair`: `public_direct / home_nat / cgnat / relay_only / validator_hidden / cloud_public / mixed_validator_observer`。
  - `degradation_class`: `none / sentry_loss / relay_exhaustion / degraded_latency_loss / restart_pause_disconnect / bootstrap_poisoning`。
  - `claim_boundary`: `required_exact / full_proxy / physical_nat_pending / shared_window_partial / public_testnet_blocked_or_ready`。
  - 这些字段属于 QA evidence taxonomy；canonical reachability/path label 必须来自 runtime peer reachability contract 或现有 `TransportPath` projection，matrix 脚本不得反向定义 runtime truth。
- 当前 full-tier 基线（2026-04-07 latest）：
  - latest full summary: `.tmp/p2p_mixed_topology_validation/20260407-120951-full/summary.json`
  - `required_exact_ready=true`
  - `full_proxy_ready=false`
  - latest failure signatures:
    - `sentry_loss_proxy_longrun`: `consensus_hash_divergence`, `committed_height_not_monotonic nodes=sequencer`, `known_peer_heads_zero_samples`, `http_failure_samples`
    - `mixed_topology_release_proxy`: `consensus_hash_divergence`, `committed_height_not_monotonic nodes=sequencer`, `known_peer_heads_zero_samples`, `http_failure_samples`
- 历史 real-env 基线（2026-04-08 incident chain terminal state；不是 current readiness authority）：
  - credentialed reconfirmation snapshot: `.tmp/p2p_real_env_triad/20260408-120134/summary.json`
  - historical provenance: `doc/testing/evidence/p2p-real-env-triad-incident-provenance-2026-07-31.md`
  - `claim_status=blocked`
  - latest real-env failure signatures:
    - `sequencer_committed_height_zero` / `sequencer_execution_stale_height` were the reconfirmation signatures; the absorbed chain's terminal residual was `fetch-commit` source readiness and peer/head reconvergence.
- 建议命令（required）：
```bash
./scripts/p2p-mixed-topology-matrix.sh --tier required
```
- 建议命令（full plan / 预览）：
```bash
./scripts/p2p-mixed-topology-matrix.sh \
  --tier full \
  --shared-window-evidence-ref doc/testing/evidence/shared-network-shared-devnet-mixed-topology-2026-05-23.md \
  --shared-window-evidence-ref doc/testing/evidence/shared-network-shared-devnet-short-window-pass-2026-05-23.md \
  --dry-run
```
- 建议命令（full 执行）：
```bash
./scripts/p2p-mixed-topology-matrix.sh \
  --tier full \
  --shared-window-evidence-ref doc/testing/evidence/shared-network-shared-devnet-mixed-topology-2026-05-23.md \
  --shared-window-evidence-ref doc/testing/evidence/shared-network-shared-devnet-short-window-pass-2026-05-23.md
```
- 建议命令（real env triad snapshot）：
```bash
P2PARCH6_SEQ_SSH_PASSWORD='***' \
P2PARCH6_STORAGE_SSH_PASSWORD='***' \
./scripts/p2p-real-env-triad-snapshot.sh \
  --samples 4 \
  --interval-secs 5 \
  --out-dir .tmp/p2p_real_env_triad
```
- 建议命令（real env triad complete observability）：
```bash
P2PARCH6_SEQ_SSH_PASSWORD='***' \
P2PARCH6_STORAGE_SSH_PASSWORD='***' \
./scripts/p2p-real-env-observability-monitor.sh \
  --samples 4 \
  --interval-secs 5 \
  --traffic-samples 3 \
  --traffic-interval-secs 20 \
  --window-minutes 10 \
  --out-dir .tmp/p2p_real_env_observability
```
- 脚本 smoke：
```bash
./scripts/p2p-mixed-topology-matrix-smoke.sh
```
- 通过标准：
  - live 执行时 `summary.json.overall_status == "ok"` 且 `totals.failed_count == 0`；
  - `required` 档位下所有 case 必须为 `coverage=exact`；
  - `full` 档位必须额外包含 `coverage=proxy` 的 longrun case，并在 evidence 中明确它们是 sentry-loss / mixed-topology live recovery 的当前代理，而不是 dedicated sentry/NAT lab 真值；
  - `summary.json.evidence_contract.executable_boundary.required_exact_ready` 必须为 `true`；
  - 若要宣称 full-tier proxy drill 已真实执行，则 `summary.json.evidence_contract.executable_boundary.full_proxy_ready` 必须为 `true`；
  - 若本轮 triad 仍是历史 mixed topology（本机 `observer` + 两台云端 validator），`summary.json.analysis.claim_mode` 应为 `observer_mixed_topology`，且若要把当前 `1` 本机 + `2` ECS real env 计入可审计 baseline，`summary.json.claim_status` 至少要达到 `partial_with_observer_blocker`，并明确写出真实 blocker；
  - 若本轮 triad 已切到三节点等权 validator，`summary.json.analysis.claim_mode` 应为 `three_equal_validator`，且若要计入该 baseline，`summary.json.claim_status` 应达到 `pass_candidate`；
  - 若要宣称本机 mixed-topology observer 接入已打通，则 real-env summary 里不得再出现 `observer_known_peer_heads_zero`、`observer_network_committed_height_zero`、`observer_committed_height_not_advancing`；
  - 若要宣称三节点等权 validator 拓扑已打通，则 real-env summary 里不得再出现 `local_committed_height_zero`、`local_known_peer_heads_zero`、`local_network_committed_height_zero`、`local_no_recent_progress_signal`、`triad_not_all_validator_roles`；
  - 若要继续给 network-rehearsal lane 提供 uplift 输入，`summary.json.external_evidence.shared_window_evidence_refs` 必须明确列出 same-window refs，且 `summary.json.evidence_contract.claim_readiness.shared_network_pass_blockers` 只能保留经审计接受的剩余 blocker；
  - 产物目录下必须同时有 `summary.json`、`summary.md`、`cases/<case_id>/command.txt`，live 执行还必须留下 `stdout.log/stderr.log`。
- 当前 exact case 入口：
  - `nat_private_role_policy`
  - `validator_hidden_boundary`
  - `relay_only_lane_budget`
  - `cgnat_relay_path_ranking`
  - `bootstrap_poisoning_dedupe`
  - `relay_budget_detection`
  - `path_failover_selection`
- 当前 proxy case 入口：
  - `sentry_loss_proxy_longrun`
  - `mixed_topology_release_proxy`
- 产物路径：
  - `.tmp/p2p_mixed_topology/<timestamp>-<tier>/summary.json`
  - `.tmp/p2p_mixed_topology/<timestamp>-<tier>/summary.md`
  - `.tmp/p2p_mixed_topology/<timestamp>-<tier>/cases/<case_id>/`
  - `.tmp/p2p_real_env_triad/<timestamp>/summary.json`
  - `.tmp/p2p_real_env_triad/<timestamp>/summary.md`
  - `.tmp/p2p_real_env_triad/<timestamp>/nodes/<label>/`
  - `.tmp/p2p_real_env_observability/<timestamp>/host/<timestamp>/summary.json`
  - `.tmp/p2p_real_env_observability/<timestamp>/traffic/latest_summary.json`
  - `.tmp/p2p_real_env_observability/<timestamp>/wasm/<label>/latest_summary.json`
  - `.tmp/p2p_real_env_observability/<timestamp>/report/latest_summary.json`
  - `.tmp/p2p_real_env_observability/latest_summary.json`
- summary 关键字段：
  - `external_evidence.shared_window_evidence_refs`
  - `external_evidence.dedicated_lab_evidence_refs`
  - `external_evidence.pass_uplift_decision_ref`
  - `evidence_contract.executable_boundary.required_exact_ready`
  - `evidence_contract.executable_boundary.full_proxy_ready`
  - `evidence_contract.executable_boundary.stronger_full_tier_truth_ready`
  - `evidence_contract.claim_readiness.mixed_topology_full_tier_status`
  - `evidence_contract.claim_readiness.stronger_full_tier_truth_blockers`
  - `evidence_contract.claim_readiness.shared_network_pass_blockers`
  - `claim_status`
  - `failure_signatures`
  - `cases[*].evidence_class`
  - `cases[*].path_expectation`
  - `cases[*].reachability_pair`
  - `cases[*].degradation_class`
  - `cases[*].claim_boundary`
  - `analysis.cloud_pair_service_healthy`
  - `analysis.cloud_pair_chain_visible`
  - `analysis.cloud_pair_progress_signal_present`
  - `analysis.claim_mode`
  - `analysis.local_peer_visibility_ok`
  - `analysis.local_network_commit_visible`
- complete observability summary 关键字段：
  - `overall.status`
  - `overall.alerts`
  - `overall.optimization_candidate_count`
  - `optimization_candidates[*].node_label/module/key/severity`
  - `nodes.<label>.host.runtime_cpu_percent`
  - `nodes.<label>.host.runtime_cpu_core_ratio`
  - `nodes.<label>.host.mem_available_percent`
  - `nodes.<label>.host.storage_used_percent`
  - `nodes.<label>.traffic.payload_total_bytes`
  - `nodes.<label>.traffic.control_plane_total_events`
  - `nodes.<label>.wasm.top_hotspot`
  - `nodes.<label>.modules.consensus.height_lag`
  - `nodes.<label>.modules.replication.recent_error_count`
  - `nodes.<label>.modules.p2p_reachability.selected_path_kind`
  - `nodes.<label>.modules.p2p_reachability.selected_path_age_ms`
  - `nodes.<label>.modules.p2p_reachability.path_transition_counters`
  - `nodes.<label>.modules.p2p_reachability.active_path_mix`
  - `nodes.<label>.modules.p2p_reachability.recent_fallback_reason`
  - `nodes.<label>.modules.p2p_reachability.reachability_confidence`
  - `nodes.<label>.modules.transactions.pending_count`
  - `nodes.<label>.modules.traffic_control_plane.control_plane_wire_ratio`
  - `nodes.<label>.optimization_candidates[*].key`
- 边界说明：
  - 当前仓库还没有 dedicated sentry role live harness，也没有物理 NAT/CGNAT 实验编排；因此 `proxy` 只代表“现在可执行的近似恢复 drill”，不能拿来冒充完整 mixed-topology 实证。
  - 2026-04-07 latest full run 已证明 matrix 能真实执行到 proxy soak，但当前 proxy drill 仍会因为 `consensus_hash_divergence / committed_height_not_monotonic / known_peer_heads_zero_samples / http_failure_samples` 失败；在这些签名被修平前，`P2PARCH-6` 仍不能宣称 `full_proxy_ready=true`。
  - 2026-04-08 historical real-env triad incident chain 已证明当时可重复采集 same-window 三节点样本，而且本机 observer 不再是主 blocker；`execution driver received stale height` 是链中间阶段签名，后续 rollout 后的终态 residual 是 `fetch-commit` source readiness、peer/head reconvergence 与 storage restart retry ordering。这组历史证据不能升级当前 lane 为 `pass`；任何当前判断都必须重新采集 same-window evidence。
  - complete observability monitor 只解决当前 triad 的 repo-owned 资源/状态/流量/WASM 统一监控，不等价于已经补齐 Prometheus/OTel/长期告警平台。
  - reachability/path observability 只能消费 `/v1/chain/status.observability` 的 bounded projection；字段缺失时应输出 `not_reported` 或等价状态，不允许 report helper 自行重建 peer path truth。
  - `relay_reserved`、`proxy` case pass、或 control-plane byte split 都不能单独支撑 `public reachable` / physical NAT / public_testnet readiness claim；这类 claim 必须回到 matrix taxonomy、same-window evidence 与 producer/QA pass-uplift decision。

### S9C：P2P 用户模式自动选择验证（P2PARCH-8/P2PARCH-9）
- 当前状态（2026-04-07）：
  - `P2PARCH-8` 已完成文档冻结：用户层默认只暴露 `自动加入 / 私有安全 / 公网入口` 三档简单模式，底层继续保留 `deployment_mode/node_role` 正式语义。
  - `P2PARCH-9` 已接通 viewer/launcher UX：`oasis7_web_launcher` 会把 chain runtime 的 P2P recommendation payload 一并透传给 `oasis7_client_launcher`，客户端会展示 requested/recommended/applied user mode、底层 role mapping 和 detection rationale，并为 `public_entry` 提供显式接受/拒绝路径。
- required 验证（本轮 docs-only baseline）：
```bash
rg -n "自动加入|私有安全|公网入口|deployment_mode|node_role|AutoNAT|高风险职责|显式确认" \
  doc/p2p/network/mainnet-private-reachability-architecture.prd.md \
  doc/p2p/network/mainnet-private-reachability-architecture.prd.md \
  testing-manual.md
./scripts/doc-governance-check.sh
git diff --check
```
- `P2PARCH-8` 通过标准：
  - PRD / project / testing manual 对用户可见模式、内部正式角色语义、默认全自动路径三者口径一致。
  - 文档明确要求普通用户默认路径不必理解 `deployment_mode/node_role`。
  - 文档明确要求涉及 `公网入口`、`relay`、`sentry` 等外部暴露职责时必须显式确认，不能静默自动升级。
- `P2PARCH-9` required 覆盖要求（实现落地时必须补 executable required evidence）：
  - 启动后 `60s` 内至少完成一次 reachability 检测，并在默认路径给出一个用户模式建议。
  - 当 AutoNAT、公网端口探测与打洞结果互相矛盾时，默认回退到保守模式，不能直接提升为 `公网入口`。
  - 当检测结果建议承担 `公网入口` 或等价暴露职责时，UI/CLI 必须要求显式确认，并记录检测依据与风险提示。
  - 用户拒绝 `公网入口` 后，系统必须回退到非入口模式，同时不破坏底层 `deployment_mode/node_role` 权限边界。
- `P2PARCH-9` full 覆盖要求（实现落地时必须补 executable full evidence）：
  - 至少覆盖 `公网可达`、`受限 NAT 可打洞`、`对称 NAT/CGNAT 无法打洞`、`检测结果抖动或冲突` 四类 reachability 场景。
  - 覆盖用户从默认推荐切到高级设置覆盖，再回退到自动模式的往返路径，并验证审计证据持续可读。
  - 覆盖多节点混合场景中，非公网节点、公网入口节点与 relay/sentry 语义的映射一致性，避免 UI 用户模式与底层 role policy 脱节。
  - 若复用 network-rehearsal / mixed-topology lane 作为 full-tier 证据，summary 中必须单独标注哪些 case 验证的是用户模式推荐，哪些 case 验证的是底层 reachability 真值。
- `P2PARCH-9` executable full-tier 基线（2026-04-07）：
```bash
env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_web_launcher -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher -- --nocapture
env -u RUSTC_WRAPPER cargo check -p oasis7_client_launcher --target wasm32-unknown-unknown
```
- 当前 full-tier 对账重点：
  - `oasis7_web_launcher` 必须把 `/v1/chain/status` 内的 P2P recommendation/evidence 正确代理到 `/api/state`
  - `oasis7_client_launcher` 必须正确渲染 requested/recommended/applied user mode、底层 `deployment_mode/node_role_claim` 与 rationale
  - `public_entry` 在 launcher 配置层必须要求显式确认；未确认时 `public_entry` 启动路径必须被拒绝
  - `auto_join -> recommended public_entry -> reject -> keep non-entry mode` 与 `auto_join -> accept public_entry -> restart apply` 这两条 UX 路径必须至少有单测或自动化闭环覆盖
- 证据要求：
  - docs-only 阶段至少留下 `rg` 命中结果与文档门禁通过记录。
  - 实现阶段至少留下模式推荐结果、触发确认的风险提示文本、用户接受/拒绝后的最终模式，以及对应的检测依据摘要。
  - 若当轮仍无法提供 dedicated NAT/public-entry lab，则必须在 evidence 中明确哪些结果来自 proxy/network-rehearsal drill，不能冒充真实公网探测实验。
- 反作弊/反女巫证据链门禁（TASK-GAME-015）：
```bash
env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_required runtime::tests::governance::governance_identity_penalty_ -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_required governance_identity_penalty_and_appeal_drive_vote_rights -- --nocapture
```
- 通过标准：
  - 同目标主体 + 同证据哈希的惩罚重放被拒绝（incident 指纹不重复通过）。
  - 惩罚 -> 申诉 -> 复核后 `evidence_chain_hash` 逐阶段变化且 `appeal_evidence_hash/resolution_evidence_hash` 非空。
  - `governance_identity_penalty_monitor_stats` 输出误伤率与高风险未闭环数量。
- 经济源汇审计与阈值门禁（TASK-GAME-016）：
```bash
env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_required main_token_economy_ -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_required main_token_treasury_distribution_applies_closed_loop_and_records_audit -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_required main_token_fee_settlement_burns_supply_and_tracks_treasury_buckets -- --nocapture
```
- 通过标准：
  - 审计报表输出 `mint_total/burn_total/net_flow` 与当期 `issued/distributed` 指标。
  - `enforce_main_token_economy_gate` 在 `inflation:*` 或 `arbitrage:*` 告警时返回阻断错误。
  - 报表中 `exploit_signature` 可用于治理升级与 runbook 分诊。
- 可运维发布阻断门禁（TASK-GAME-017）：
```bash
env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_required longrun_operability_release_gate_ -- --nocapture
```
- 通过标准：
  - `evaluate_longrun_operability_release_gate` 产出统一报告，覆盖 `SLO + 告警 + 灾备演练 + 灰度阶段 + 经济告警`。
  - `enforce_longrun_operability_release_gate` 对首个违规项返回阻断错误（包含 `gate + reason`）。
  - 报告中的 `economy_report.alerts` 会同步升级为发布阻断违规项。

### S10：五节点真实游戏数据在线长跑技术套件
- 当前状态（2026-02-28）：`scripts/s10-five-node-game-soak.sh` 已恢复为可执行脚本，底座为五进程 `oasis7_chain_runtime`。
- DistFS probe 边界：reward worker 在空 blob set 时可幂等写入非敏感 seed，为 `distfs_total_checks` 提供采样前提；seed 成功不是 pass。当前窗口仍无样本时必须保留 `insufficient_data`，最终结论由完整 S10 metric gate 决定。
- 时间语义说明：S10 与 S9 口径一致，`slot_duration_ms/ticks_per_slot` 决定 PoS 逻辑时间，`node_tick_ms` 仅作轮询/回退间隔。
- 建议命令（smoke）：
```bash
./scripts/s10-five-node-game-soak.sh --duration-secs 600 --no-prewarm
```
- 建议命令（默认长窗）：
```bash
./scripts/s10-five-node-game-soak.sh
```
- 发布门禁基线命令（2026-02-28，300s）：
```bash
./scripts/s10-five-node-game-soak.sh --duration-secs 300 --no-prewarm --max-stall-secs 240 --max-lag-p95 50 --out-dir .tmp/release_gate_s10
```
- 通过标准：
  - 命令返回 `rc=0`；
  - `summary.json` 中 `run.status == "ok"`，并产出 `timeline.csv`；
  - `summary.json` 中 `run.metric_gate.status == "pass"`（一般告警通过 `run.metric_gate.notes` 留痕，不应降级为 `insufficient_data`）；
  - 若失败，必须保留 `failures.md` 作为分诊依据。
- metric-source boundary：S10 的 mint、DistFS、settlement 与 invariant 只以
  `reward_runtime` 原生快照为准；每节点累计最大值聚合、缺失节点告警、
  `distfs_total_checks=0 -> insufficient_data`、以及 invariant/HTTP/运行状态的
  独立分类均与 S9 一致。历史 run 或空集 probe seed 不能替代本次完整 metric gate。
- 参考文档：`doc/testing/longrun/s10-five-node-real-game-soak.prd.md`、`doc/testing/longrun/p2p-longrun-soak-and-chaos.prd.md`、`doc/testing/longrun/game-world-state-sync-commit-closure-2026-06-26.prd.md`。

### 发布门禁一键收口（S0 + S1 + S6 + S9 + S10）
```bash
./scripts/release-gate.sh
./scripts/release-gate.sh --quick
./scripts/release-gate.sh --dry-run
```
- 默认串行执行：`ci-tests full`、`sync-m1/m4/m5 --check`、Web strict、S9/S10。
- `--quick` 用于缩短 S9/S10 时长并关闭 Web visual baseline。
- `--skip-*` 只用于已知外部约束或分片排障；必须在 summary 中保留 skip reason 与 claim boundary。跳过 `ci_full`、sync、Web strict、S9 或 S10 后，不得把本次结果写成完整 release coverage。

### Network Tiers / Shared-Network Evidence
- 当前网络层真值统一以 `doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.prd.md` 与对应 project/runbook 为准：
  - operator/runtime network-tier 是 `local_devnet -> public_testnet -> mainnet`，不作为玩家世界模型。
  - `public_testnet_rehearsal` 只作 legacy/rehearsal evidence，不能替代 formal `public_testnet` 的 six-lane readiness，也不代表 live `public_testnet`、`mainnet`、public launch、赛季上线或公开大世界已建立。
- Canonical docs:
  - Current network-tier source of truth: `doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.prd.md`
  - `public_testnet` live-candidate checklist: `doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.runbook.md`
  - Legacy network-rehearsal evidence: `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.runbook.md`
  - Benchmark background: `doc/testing/benchmarks/mainstream-public-chain-testing-benchmark.prd.md`
- Canonical commands:
```bash
./scripts/network-tier-manifest.sh validate \
  --manifest doc/testing/templates/network-tier-public-testnet.example.json
./scripts/network-tier-public-testnet-readiness.sh \
  --manifest doc/testing/templates/network-tier-public-testnet.example.json
./scripts/network-tier-exit-review.sh \
  --manifest doc/testing/templates/network-tier-public-testnet.example.json
./scripts/network-tier-manifest-smoke.sh
```
- Testnet-connected hosted entry flow:
  - 本机暴露 `hosted_public_join` / hosted-login 形态入口时，先证明本机节点已按 formal `public_testnet` manifest / `world_id` / `chain_id` / genesis / bootstrap peers 接入并同步到 testnet，再证明 hosted-login / launcher / viewer / pure API 的 runtime/status/API endpoint 指向该节点 world state。
  - 最小证据包括：manifest validation、local observer sync/preflight 输出、节点 health/status、connected peers、height/head 推进、hosted-login `login/start -> login/complete` smoke、以及 viewer / pure API 读取同一 testnet world state 的截图或 JSON 摘要。
  - 若缺少节点同步证据或 hosted-login 接入面仍指向 local execution world，只能记为 local hosted-public-join / hosted-login smoke，不得支撑 `public_testnet` unified-world 测试 claim。
- Boundary:
  - `testing-manual.md` is an execution index only; do not duplicate current network-tier verdicts here.
  - `public_testnet_rehearsal` history may explain historical technical benchmark context, but is not L5 evidence; current public readiness belongs to formal `public_testnet` readiness docs.

### S11：去中心化模块发布运行与告警（world-runtime）
- 适用范围：线上模块发布（`proposal -> attestation -> apply`）与 builtin 在线清单加载故障分诊。
- 生产执行边界（强制）：
  - 生产发布写入/激活只能由发布节点提交链上动作（`ModuleReleaseSubmit*` / `ModuleReleaseApply`）完成。
  - 主 CI 仅允许执行 `--check` 类回归与对账，不参与生产发布写入、阈值签名或激活判定。
- 节点侧固定验收入口（默认 required，按需追加 full）：
```bash
./scripts/module-release-node-acceptance.sh
./scripts/module-release-node-acceptance.sh --include-full
./scripts/module-release-node-attestation-flow.sh --help
./scripts/package-module-release-attestation-proof.sh --help
./scripts/submit-module-release-attestation.sh --help
./scripts/wasm-release-evidence-report.sh --expected-runners linux-x86_64
./scripts/wasm-release-evidence-report.sh \
  --skip-collect \
  --summary-import-dir output/ci/m1-wasm-summary \
  --module-sets m1 \
  --expected-runners linux-x86_64
./scripts/wasm-release-evidence-report.sh \
  --skip-collect \
  --summary-import-dir output/ci/m1-wasm-summary \
  --module-sets m1 \
  --expected-runners linux-x86_64,darwin-arm64
./scripts/module-release-node-attestation-flow.sh \
  --module-sets m1 \
  --summary-import-dir output/ci/m1-wasm-summary \
  --skip-local-collect \
  --required-runners linux-x86_64 \
  --expected-runners linux-x86_64,darwin-arm64 \
  --request-id 17 \
  --operator-agent-id operator-1 \
  --signer-node-id attestor-node-1 \
  --build-manifest-hash <hex> \
  --source-hash <hex> \
  --wasm-hash <hex> \
  --builder-image-digest <sha256:digest> \
  --container-platform linux-x86_64 \
  --canonicalizer-version strip-custom-sections-v1
```
- 产物与证据：
  - 默认输出目录：`.tmp/module_release_node_acceptance/<timestamp>/`
  - 最小归档：`summary.md`、`summary.json`、各 step log（含 triage 信号检索）
  - node-side attestation flow 默认输出目录：`.tmp/module_release_node_attestation_flow/<timestamp>/`
  - node-side attestation flow 最小归档：`flow_summary.md`、`flow_summary.json`、`staged_summaries/`、`proof_inputs/`、`proof/proof_payload.json`、`proof/submit_request.json`
  - attestation proof payload 默认输出目录：`.tmp/module_release_attestation_proof/<timestamp>/`
  - attestation proof 最小归档：`proof_payload.json`、`submit_request.json`、`evidence/` 附件目录或对应 archive、稳定 `proof_cid`
  - WASM release evidence 默认输出目录：`.tmp/wasm_release_evidence_report/<timestamp>/`
  - WASM release evidence 最小归档：`summary.md`、`summary.json`、`module_sets.tsv`、各 module set verify log 与 per-runner summary json
- 等价拆分命令（便于定向排障）：
```bash
env -u RUSTC_WRAPPER cargo test -p oasis7 module_release_submit_attestation_ --features test_tier_required -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 module_release_apply_rejects_when_attestation_threshold_not_met --features test_tier_required -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 module_release_apply_rejects_when_attestation_receipt_evidence_mismatches --features test_tier_required -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 power_bootstrap_release_manifest_full --features test_tier_full -- --nocapture
./scripts/module-release-node-attestation-flow.sh --help
./scripts/package-module-release-attestation-proof.sh --help
./scripts/submit-module-release-attestation.sh --help
./scripts/ci-m1-wasm-summary.sh --module-set m1 --runner-label linux-x86_64 --out <summary-dir>/m1/linux-x86_64.json
python3 ./scripts/ci-verify-m1-wasm-summaries.py --module-set m1 --summary-dir <summary-dir>/m1 --expected-runners linux-x86_64
```
- finality 基准固定入口（`stake/epoch` 验签耗时 + 2 epoch 收敛）：
```bash
./scripts/oasis7-runtime-finality-baseline.sh
./scripts/oasis7-runtime-finality-baseline.sh --required-samples 1 --full-samples 1 --warmup-samples 0
```
- 基准产物：
  - 默认输出目录：`.tmp/world_runtime_finality_baseline/<timestamp>/`
  - 归档文件：`summary.md`（人读）与 `summary.json`（机器读）
- 运行时分诊检索（日志/审计）：
```bash
rg -n "conflicting attestation already exists|attestation threshold not met|attestation receipt evidence mismatch|fault_signature=builtin_release_manifest_" output .tmp
```
- 告警策略（发布阻断）：
| 场景 | 识别信号（日志/事件） | 阻断策略 | 首轮处置 |
| --- | --- | --- | --- |
| 证明冲突 | `module release attestation rejected: conflicting attestation already exists for signer=<id> platform=<platform>` | 阻断对应 `request_id` 的继续激活 | 冻结该 `request_id`，核对 `build_manifest_hash/source_hash/wasm_hash/proof_cid`，保留首条证据并重新发起发布单。 |
| 阈值不足 | `module release apply rejected: attestation threshold not met epoch_id=<id> threshold=<n> aggregated_signers=<m>` | 阻断 `ModuleReleaseApply`，保持旧 `active_manifest_hash` | 对齐当前 `epoch` 快照 signer 集，补齐缺失 signer 证明后重试 apply。 |
| 证明未入链 | 只有 CI / workflow artifact，缺少 node-side `proof_payload.json` 或未执行 attestation submit | 不得进入 `ModuleReleaseApply` | 先用 proof 脚本打包正式证据，生成稳定 `proof_cid`，再由发布节点提交 `ModuleReleaseSubmitAttestation`。 |
| manifest 不可达/回滚/漂移 | `fault_signature=builtin_release_manifest_unreachable` / `fault_signature=builtin_release_manifest_missing_or_rolled_back` / `fault_signature=builtin_release_manifest_identity_drift` | 阻断 builtin 新版本加载，维持旧版本 | 检查 distfs artifact 可达性、release manifest 条目与 identity 是否一致，修复后再触发加载。 |

## 改动路径 -> 必跑套件矩阵（针对性执行）

### 套件触发总表（S0~S10）

| 套件 | 主要覆盖面 | 默认触发条件 | 最小证据 |
|---|---|---|---|
| S0 | 通用静态基线 / 文档 / shell / 格式 / 快速健康检查（不含 target-specific 编译） | 任何代码、脚本、文档、工作流改动 | 命令日志 + 通过/失败结论 |
| S1 | 核心 required | ordinary PR 的 `oasis7` 主链路 impact-scoped premerge 最小 blocking set | required 测试日志 |
| S2 | 核心 full | release、高风险、历史缺陷升级、信号触发或 schedule；不是 ordinary PR 默认 | full 测试日志 |
| S3 | 应用主链定向 | runtime / simulator / viewer live / web bridge 定向改动 | 定向 cargo test 日志 |
| S4 | 分布式子系统 | node / net / consensus / distfs / P2P 链路改动 | 子系统测试日志 |
| S5 | Pixel World Bridge（Bevy）lib / wasm 编译 | Viewer/Bevy、`crates/pixel_world_bridge/**` 或 pixel-world wasm/build 链路改动 | `pixel_world_bridge` lib 测试 + wasm 编译日志 |
| S6 | Web UI 闭环 smoke | Viewer / launcher / Web 控制台 / 交互链路改动；真实玩家输入流程或真实 provider 回归优先补 Playwright 实跑用例；任何可视化相关代码、样式、资源或可见输出改动还必须叠加截图模型视觉评审 | 截图、console、语义结果；Playwright summary/state；visual review card |
| S7 | 场景矩阵回归 | scenario / gameplay 初始化 / 场景 ID 与稳定性改动 | 场景测试日志 |
| S8 | 长稳与压力 | 性能、内存、恢复、资源压力或 soak 相关改动 | stress/soak 目录与 summary |
| S9 | P2P/存储/共识在线长跑 | 分布式一致性、存储、共识、在线网络改动 | S9 summary / timeline / failures |
| S10 | 五节点真实游戏在线长跑 | 真实游戏链路、结算、mint、验证器编排改动 | S10 summary / timeline / failures |

### 改动路径矩阵

| 改动路径 | 必跑 | 推荐追加 | 升级规则 |
|---|---|---|---|
| `crates/oasis7/src/runtime/**` | S0 + S1 | S2 + S3 + S7 | 若涉及确定性 / 治理 / 持久化，追加 S8；若触达在线状态复制，追加 S9 |
| `crates/oasis7/src/simulator/**` | S0 + S1 | S2 + S3 + S7 + S8 | 若触达 UI 表达或交互入口，追加 S6 |
| `crates/oasis7/src/viewer/**` 或 `src/bin/oasis7_viewer_live.rs` | S0 + S1 + S6 | S2 + S3 + S5 | 若改动 viewer 协议或 wasm 构建链路，S5 变为必跑 |
| `crates/pixel_world_bridge/**` | S0 + S5 + S6 | S2 + S8 | raster/browser/performance 证据按 S6 JS-browser / JS-full 风险升级 |
| `crates/oasis7_viewer/**` | S0 + S5 + S6（JS-required；可见输出还需 JS-browser） | S2 + S8（JS-full） | 若改动只影响静态资源 / 样式，可抽样 S1；若影响 bridge，追加 S3 |
| `crates/oasis7_node/**` | S0 + S4（node） + S9/S10（按改动面至少一条） | S2 + S3 + S8 + 另一条在线长跑（S9 或 S10） | 共识推进 / 节点编排改动优先加 S10；网络 / 复制改动优先加 S9 |
| `crates/oasis7_net/**` | S0 + S4（net） + S9/S10（按改动面至少一条） | S2 + runtime_bridge 变体 + S8 + 另一条在线长跑（S9 或 S10） | 若仅桥接层改动，可用 S3 + S9 smoke；若影响真实联机，补 S10 |
| `crates/oasis7_consensus/**` | S0 + S4（consensus） + S9/S10（按改动面至少一条） | S2 + S8 + 另一条在线长跑（S9 或 S10） | epoch / attest / finality 逻辑改动优先补 S10 |
| `crates/oasis7_distfs/**` | S0 + S4（distfs） + S9/S10（按改动面至少一条） | S2 + S8 + 另一条在线长跑（S9 或 S10） | 存储复制 / challenge / 修复逻辑改动优先补 S9 |
| `doc/**`（非 `doc/devlog/**`） | S0（含 `./scripts/doc-governance-check.sh`） | 命中模块的抽样 required 证据核验 | 若文档改变发布 / 测试口径，追加对应模块的最小必跑集 |
| `scripts/ci-tests.sh` / `.github/workflows/rust.yml` | S0（含 `./scripts/doc-governance-check.sh`） + `bash -n scripts/plan-rust-required-scope.sh` + planner 样例 + S1 + （full）`./scripts/llm-baseline-fixture-smoke.sh` | S2 + S4 + S6（抽样） | 若更改默认 gate 组合，需抽样至少一条 S9 或 S10；docs-only / `.pm` / 无关元数据 PR 必须验证 planner 可输出 `scope=minimal` 且保留 stable `required-gate` 上下文 |
| `scripts/plan-rust-required-scope.sh` | `bash -n scripts/plan-rust-required-scope.sh` + `./scripts/plan-rust-required-scope.sh --event-name pull_request --changed-path crates/oasis7_viewer/src/lib.rs` + `./scripts/plan-rust-required-scope.sh --event-name pull_request --changed-path crates/pixel_world_bridge/src/render.rs` + `./scripts/plan-rust-required-scope.sh --event-name pull_request --changed-path crates/oasis7/src/runtime/mod.rs` + `./scripts/plan-rust-required-scope.sh --event-name pull_request --changed-path crates/oasis7_node/src/network_bridge.rs` + `./scripts/plan-rust-required-scope.sh --event-name pull_request --changed-path crates/oasis7_net/src/lib.rs` + `./scripts/plan-rust-required-scope.sh --event-name pull_request --changed-path doc/testing/prd.md` + `./scripts/plan-rust-required-scope.sh --event-name pull_request --changed-path scripts/ci-tests.sh` | 与 `required-gate` 同步执行；PR/push 上由 `scripts/ci-required-scope.v2.json` 把 changed paths 映射为可审计的 `selected_capabilities`，再执行对应 required 组件；一般 Viewer 改动选 JS-required，性能输入才追加 report/watch probe，Pixel World/Bevy 改动独立选择 bridge lib + wasm32 检查 | 命中共享 CI / gate 输入或未分类代码路径时必须回退 `scope=full`；docs-only / `.pm` / 无关元数据应输出 `scope=minimal` 且不跳过治理/fmt |
| `scripts/release-gate.sh` / `.github/workflows/release-packages.yml` | `./scripts/ci-tests.sh full` + `sync-m1/m4/m5 --check` + Web strict + S9 + S10 | `./scripts/release-gate.sh --quick` / `--dry-run` | 任何发布 gate 逻辑变更均不允许跳过 S9/S10 |
| `scripts/ci-m1-wasm-summary.sh` / `scripts/ci-verify-m1-wasm-summaries.py` / `scripts/wasm-release-evidence-report.sh` / `.github/workflows/wasm-determinism-gate.yml` | `S0` + `./scripts/ci-m1-wasm-summary.sh --module-set m4 --runner-label linux-x86_64 --out output/ci/m4-wasm-summary/linux-x86_64.json` + `./scripts/wasm-release-evidence-report.sh --module-sets m4 --skip-collect --summary-import-dir output/ci/m4-wasm-summary --expected-runners linux-x86_64` | `workflow_dispatch` 触发 GitHub-hosted Linux runner gate；若补入外部 macOS summary，可再用 `--expected-runners linux-x86_64,darwin-arm64` 做双宿主对账 | 若改动 hash/summary/evidence report 格式，Linux gate 必跑；跨宿主 full-tier 在有 Docker-capable macOS summary 时追加 |
| `scripts/plan-wasm-determinism-scope.sh` | `bash -n scripts/plan-wasm-determinism-scope.sh` + `./scripts/plan-wasm-determinism-scope.sh --event-name pull_request --changed-path crates/oasis7_builtin_wasm_modules/m4_factory_miner_mk1/Cargo.toml` + `./scripts/plan-wasm-determinism-scope.sh --event-name pull_request --changed-path doc/testing/prd.md` | 与 `wasm-determinism-gate` 同步执行；PR/push 上先规划命中的 module set，再决定 collect/verify 是否实际执行 | 若共享 wasm pipeline 输入命中，则必须扩成 `m1,m4,m5`；无关改动应输出 `scope=skip` 并保留 stable required contexts |
| `scripts/run-viewer-web.sh` | S0 + S5 + S6 | S8 | 若涉及 software_safe 静态入口、构建 freshness 或浏览器自动化契约，追加对应 smoke 与 bundle 验证 |
| `scripts/p2p-longrun-soak.sh` / `doc/testing/longrun/p2p-longrun-soak-and-chaos*` | S0 + S9 smoke（含 summary/timeline 校验） | S9 endurance（含 chaos） | 任何阈值/summary 字段变更必须补 endurance |
| `scripts/s10-five-node-game-soak.sh` / `doc/testing/longrun/s10-five-node-real-game-soak*` | S0 + S10 smoke（含 summary/timeline 校验） | S10 默认长窗（30min+） | 任何门禁字段 / 结算 / mint 改动都需补长窗 |

### 选择规则
1. 先按“改动路径”命中一行矩阵，执行“必跑”。
2. 若同一变更命中多行，取并集，不取其一。
3. 若改动同时触达协议 / UI / 分布式链路，必须把 S6 与 S9/S10 同时纳入。
4. 若发布 / 文档口径改变了测试边界，至少补一条对应模块的抽样 required 证据，避免只改文档不改验证。
5. `S11` 属于 world-runtime 去中心化模块发布专题，不纳入本 `S0~S10` 触发矩阵，但若改动触及该链路，需叠加执行 `S11` 专题手册。

## Human/AI 共用执行剧本

### 阶段 A：确定测试范围
1. 识别改动路径命中哪一行“矩阵”。
2. 生成本次要跑的套件列表（至少含“必跑”列）。
3. 在日志中写清“为什么跑这些、不跑哪些”。

### 阶段 B：先跑低层，后跑高层
1. 先执行 S0。
2. 再执行对应的 L1/L2/L3 套件（S1/S2/S3/S4/S5）。
3. 最后执行 UI 闭环与压力（S6/S8；分布式改动需补 S9 或 S10）。
4. 任意层失败立即停止上层，先定位并修复。

### 阶段 C：记录结论
1. 对每个套件记录：命令、结果、失败点、是否复跑。
2. 记录证据路径（截图、console、CSV、关键日志）。
3. 给出“是否达到本次任务充分度标准”的结论。

## 充分度标准（按任务风险分级）

### 日常改动（低风险）
- 必须通过：S0 + S1
- 若触达 Viewer/UI：追加 S6

### 功能改动（中风险）
- 必须通过：S0 + S1 + 对应路径必跑矩阵
- 至少 1 条 S6 Web 闭环 smoke

### 高风险改动（协议/共识/分布式/发布前）
- 必须通过：S0 + S2 + S4 + S6
- 建议通过：S8 至少一条压力脚本；并执行至少一条 S9 或 S10 在线长跑。

## 证据规范

### 必备证据
- 命令执行记录（终端或 CI 日志）。
- 失败堆栈或关键断言信息。
- UI 闭环截图与 console 结果（若执行 S6）。

### 推荐证据目录
- `output/playwright/viewer/*.png`
- `output/playwright/viewer/console.log`（或等价重定向日志）
- `.tmp/viewer_owr4_stress/<timestamp>/`
- `.tmp/llm_stress/`
- `.tmp/p2p_longrun/<timestamp>/`
- `.tmp/s10_game_longrun/<timestamp>/`

### 结果记录模板
```md
- 目标变更：
- 触发路径：
- 执行者（Human/AI）：
- 套件清单（S0~S10）：
  - Sx: 命令 / 结果 / 证据路径
- 失败分诊：
  - 层级（L0~L5）：
  - 原因分类（确定性/环境/flaky）：
  - 处理结论：
- 最终结论：
- 遗留事项：
```

## 失败分诊（按层）
1. L0 失败：优先修复格式、工件、目标安装问题。
2. L1 失败：优先定位业务逻辑回归或断言漂移。
3. L2 失败：优先检查协议兼容、连接时序、桥接参数。
4. L3 失败：优先检查分布式状态恢复、签名校验、网络行为。
5. L4 失败：先判定是否环境问题（端口、launcher 进程、wasm 初始化），再判定 UI 回归。
6. L5 失败：只针对真实人类或受控外部玩家样本，先核验样本、环境、继续游玩意愿与反馈采集；性能、资源泄漏和长时状态累计问题回到对应技术套件（S8/S9/S10）分诊。

## TODO（待收口）
- [x] TODO-1：修正 S7 场景矩阵回归命令的覆盖口径。
  - 处理结果（2026-03-05）：S7 的 `oasis7_init_demo_runs_` 已切换到 `test_tier_full` 执行档位。
  - 验收记录：`env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_full oasis7_init_demo_runs_ -- --nocapture` 命中多场景用例（非 1 条）。

- [x] TODO-2：修复 S5 Pixel World Bridge（Bevy）测试编译阻塞。
  - 处理结果（2026-02-21）：当前 `pixel_world_bridge` lib 测试已恢复可编译可执行，并纳入 `scripts/ci-tests.sh` 的 full support-crate shard。
  - 验收记录：`env -u RUSTC_WRAPPER cargo test -p pixel_world_bridge --lib` 通过。

## 风险
- 风险 1：把 `required/full` 当作整应用全覆盖。
  - 缓解：按本手册补齐 S4/S5/S6/S8。
- 风险 2：UI 闭环只看截图，不看状态与 console。
  - 缓解：S6 强制 `console error = 0` + 可见状态判断。
- 风险 3：分布式子系统改动未触发对应 crate 测试。
  - 缓解：必须使用“改动路径矩阵”决策套件。
- 风险 4：压力回归长期缺失，问题只在长跑暴露。
  - 缓解：高风险改动或发布前至少执行一条 S8，并执行一条 S9 或 S10 在线长跑。

## 里程碑
- T1：完成基于仓库现状的分层模型与套件目录。
- T2：完成改动路径触发矩阵与 Human/AI 共用剧本。
- T3：完成充分度标准、证据规范、失败分诊规则。
- T4：后续按真实缺陷复盘持续调整各层用例配额与命令清单。
