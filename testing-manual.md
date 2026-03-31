# oasis7: 系统性应用测试手册（Human/AI 通用）

## 目标
- 基于仓库当前实现，提供一套可直接执行的分层测试手册，让人类开发者与 AI Agent 都能对“整应用”做足够充分的测试。
- 解决“只跑一条命令看总绿灯”但无法定位风险层的问题，把测试明确拆成基础门禁、核心逻辑、协议集成、分布式子系统、UI 闭环、压力回归。
- 把 `test_tier_required` 与 `test_tier_full` 放回整体测试体系中：它们是核心层基线，不等于“整应用全覆盖”。
- 统一证据标准（命令、日志、截图、结论），保证测试可复盘、可审计。

## 对标入口
- 若当前任务涉及“主流公链测试体系一般怎么做”“oasis7 还缺哪几层才接近主流链测试成熟度”，先看：
  - `doc/p2p/blockchain/p2p-mainstream-public-chain-testing-benchmark-2026-03-24.prd.md`
  - `doc/p2p/blockchain/p2p-mainstream-public-chain-testing-benchmark-2026-03-24.design.md`
- 若当前任务涉及“shared network / release train 最小要做到什么、现在是否已具备正式共享轨道”，再看：
  - `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.prd.md`
  - `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.design.md`
- 本手册负责 oasis7 自己的执行分层与命令入口；benchmark 专题负责把这些层对位到主流公链常见 testing stack。
- shared network / release train 专题负责冻结 `L5` 的目标态与 claims gate；它当前表示“已建档、未执行”，不代表仓库已经具备正式 shared devnet/staging/canary。

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

## 当前实现分布（2026-02-18 基线）

### 应用主链（world + runtime + simulator + viewer 协议）
- 核心 crate：`crates/oasis7`
- 主要测试分布：
  - 运行时：`crates/oasis7/src/runtime/tests/*.rs`
  - 模拟器：`crates/oasis7/src/simulator/tests/*.rs`
  - LLM 行为：`crates/oasis7/src/simulator/llm_agent/tests_part2.rs`
  - Viewer live 服务：`crates/oasis7/src/bin/oasis7_viewer_live.rs`（内置 `#[cfg(test)]`）
  - 端到端集成：`crates/oasis7/tests/*.rs`

### Viewer 客户端（Bevy/egui + wasm）
- crate：`crates/oasis7_viewer`
- 覆盖：
  - UI/相机/事件联动等单测散布在 `src/*.rs` 与 `src/tests_*.rs`
  - 快照基线：`crates/oasis7_viewer/tests/snapshots/*.png`
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
- `required`：
  - `./scripts/doc-governance-check.sh`
  - `./scripts/check-rust-file-size.sh`
  - `cargo fmt --check`
  - `cargo test -p oasis7 --tests --features test_tier_required`
  - `cargo test -p oasis7_consensus --lib`
  - `cargo test -p oasis7_distfs --lib`
  - `cargo test -p oasis7_viewer`
  - `cargo check -p oasis7_viewer --target wasm32-unknown-unknown`
- `full`：
  - `required` 全部
  - `cargo test -p oasis7 --tests --features test_tier_full,wasmtime,viewer_live_integration`
  - `cargo test -p oasis7_node --lib`
  - `cargo test -p oasis7_net --lib`
  - `cargo test -p oasis7_net --features libp2p --lib`
  - `./scripts/llm-baseline-fixture-smoke.sh`
  - `cargo test -p oasis7 --features wasmtime --lib --bins`
- 入口 B：`.github/workflows/rust.yml`（required-gate）
  - `CI_VERBOSE=1 ./scripts/ci-tests.sh required`
  - `./scripts/viewer-visual-baseline.sh`
- 入口 C：`.github/workflows/wasm-determinism-gate.yml`（构建 hash / receipt evidence 独立 gate）
  - GitHub-hosted runner 矩阵：`(m1|m4|m5) x (ubuntu-24.04/linux-x86_64)`
  - 每个 runner 执行：`./scripts/ci-m1-wasm-summary.sh --module-set <m1|m4|m5> --runner-label ... --out ...`
  - verify job 会按 `module_set` 下载 summaries，并执行：`./scripts/wasm-release-evidence-report.sh --module-sets <m1|m4|m5> --skip-collect --summary-import-dir <downloaded-summary-dir> --expected-runners linux-x86_64`
  - verify job 同时上传 `summary.md/json + logs + module_sets.tsv` 的 release evidence report artifact
  - 若要补跨宿主 full-tier 证据，可把外部 Docker-capable macOS runner 产出的 summary 作为额外 import 输入，再以 `--expected-runners linux-x86_64,darwin-arm64` 做离线对账

### 当前 CI 未直接覆盖（需手册补齐）
- Web UI agent-browser 闭环（现为手动/agent 流程，不在 CI 默认路径中）。
- `m4/m5` builtin wasm hash 校验（`scripts/ci-tests.sh` 已移除 `sync-m4/m5 --check`）。

结论：
- `required/full` 是“核心链路测试层”的主入口（required 含 `oasis7 + consensus + distfs + viewer`，full 追加 `node + net/libp2p`）；
- `required-gate` 已补充 viewer 视觉基线脚本（snapshot 基线 + 定向测试）；
- `wasm-determinism-gate` 负责 `m1/m4/m5` hash / receipt evidence 独立 gate；
- 若目标是“整应用充分测试”，仍需在此基础上叠加 UI 闭环层（S6）与压力层（S8）。

## 分层模型（针对当前仓库）

### L0 静态与工件一致性层
- 目标：尽早拦截格式漂移、内置 wasm 工件漂移、构建目标缺失。
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

### L4 UI 闭环层（Web 为默认）
- 目标：验证真实用户路径可用性（加载、交互、状态可见、无 console error）。
- 默认：agent / QA 在当前 git worktree 内做开发回归时，优先使用 `./scripts/worktree-harness.sh up` 起一套 worktree 隔离 Web 栈；它会为当前 worktree 派生独立端口组、bundle / runtime / artifact 根目录与浏览器 session，并把状态写到 `output/harness/<worktree_id>/state.json`。制作人试玩 / 发布前人工验收仍优先使用 `./scripts/run-producer-playtest.sh`（需要自动打开浏览器时加 `--open-headed`）；其默认 bundle 根目录也会落到当前 worktree 自己的 `output/harness/<worktree_id>/bundle/` 下。`scripts/run-game-test.sh` 保留为底层 bootstrap，并支持 `--bundle-dir <bundle>` 复用产物入口；当 bundle 缺少 freshness manifest 或已落后于当前工作区源码时，脚本会默认阻断，制作人入口则会自动重建。launcher stack 已不再接受 no-LLM 启动；`--no-llm` 只保留给直接 `oasis7_viewer_live` 观战/调试排障。
- source-tree `oasis7-run.sh play` 与 `run-game-test.sh` 的 Viewer Web 开发态入口都必须走 freshness gate；当 `crates/oasis7_viewer/index.html`、`software_safe.html`、`software_safe.js` 或相关静态资源比 `dist/` 更新时，默认应优先重建 fresh dist，而不是继续拿 stale `dist` 给 Web 闭环下结论。
- native 抓图：仅 fallback（Web 无法复现或 native 图形链路问题）。

### L5 长稳与压力层
- 目标：验证在长时运行/高事件量下系统退化策略和稳定性。
- 入口：`viewer-owr4-stress.sh`、`llm-longrun-stress.sh`。

## 测试套件目录（S0~S10）

### S0：基础门禁套件（L0）
```bash
./scripts/doc-governance-check.sh
./scripts/check-rust-file-size.sh
env -u RUSTC_WRAPPER cargo fmt --all -- --check
env -u RUSTC_WRAPPER cargo check -p oasis7_viewer --target wasm32-unknown-unknown
```
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

### S1：核心 required 套件（L1）
```bash
./scripts/ci-tests.sh required
```
- 覆盖重点：
  - runtime/simulator 大量单元与集成测试
  - `oasis7_viewer_live` 二进制测试
  - viewer offline integration
  - 分布式基础子系统（轻量）：`oasis7_consensus`、`oasis7_distfs`
  - `oasis7_viewer` 全量单测 + wasm 编译检查

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
- 运行与审计口径补充：
  - 设计与运行要点：`doc/p2p/token/mainchain-token-allocation-mechanism.prd.md`
  - 发布说明：`doc/p2p/token/mainchain-token-allocation-mechanism.release.md`
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
- default/live execution world 正式证据入口：
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
  - clone-world 样本只证明 runbook/tooling 正确，不替代 default/live execution world 的最终 QA 证据
  - `governance-registry-live-drill.sh` 会在真实默认 world 上自动执行 `baseline -> pass -> block -> restore`
  - 当 block case 仍可导入时，`governance-registry-drill.sh` / `governance-registry-live-drill.sh` 会额外执行 `rejoin`
  - controller slot 可保持原 `signer_id` 仅替换公钥；`governance.finality.v1` 不行，必须显式传入新的 `--replacement-signer-id`
  - `--block-remove-signer-id` 可重复使用；当 block manifest 让 `finality signer_count < threshold` 时，默认预期是 `import_policy_reject`
  - 若对 finality slot 复用原 `signer_id`，真实导入会命中 `GovernancePolicyInvalid`，因为 finality signer 绑定到现有 node identity

### S5：Viewer crate 单测与 wasm 编译套件（L4 前置）
```bash
env -u RUSTC_WRAPPER cargo test -p oasis7_viewer
env -u RUSTC_WRAPPER cargo check -p oasis7_viewer --target wasm32-unknown-unknown
```
- 说明：
  - `oasis7_viewer` 内已有大量 UI/相机/交互逻辑测试；
  - 这是 UI 闭环前的稳定性筛网；
  - 该套件已并入 `S1/S2` 的默认 gate。

### S6：Web UI 闭环 smoke 套件（L4）
- S6 详细执行步骤、agent-browser 命令、发布门禁与补充约定已拆分到：
  - `doc/testing/manual/web-ui-agent-browser-closure-manual.manual.md`
  - `doc/testing/manual/web-ui-agent-browser-closure-manual.prd.md`（需求边界/成功标准）
  - `doc/testing/launcher/launcher-manual-test-checklist-2026-03-10.prd.md`（发布前人工体验与异常恢复检查清单）
- 本手册仅保留分层与触发矩阵，执行时按上述文档操作。
- 模式总口径（`PRD-CORE-009`）：
  - `standard_3d` / `software_safe` / `pure_api` 是玩家访问模式，分别对应标准 3D 视觉入口、弱图形安全入口和纯接口正式入口。
  - `pure_api` 的正式游玩与 headed Web/UI 一样，默认要求 active LLM access；禁用 LLM 后只能做 blocked/observer-debug 诊断，不再计入正式可玩性证据。
  - `player_parity` / `headless_agent` / `debug_viewer` 是 execution lane，只描述 OpenClaw / agent 的执行或观战方式，不构成额外玩家访问模式。
  - 任何 QA / release / playability 结论都应先标明玩家访问模式，再补充 execution lane；不得把 `headless_agent` 或 `debug_viewer` 直接当成“第四种入口”。
- `oasis7_viewer_live` / Viewer 页面：默认使用 `agent-browser` 驱动页面与采集证据；当 `renderMode=software_safe` 且带 viewer auth bootstrap 时，允许继续验证选中 Agent 的最小 `prompt/chat` 闭环。
- 若需要把 `software_safe` 的 prompt/chat/rollback/message-flow 做成独立 QA smoke，优先执行 `./scripts/viewer-software-safe-chat-regression.sh`；该脚本默认把 `agent_spoke` 缺失记为可追溯 warning，显式加 `--require-agent-spoke` 时再升级为阻断失败。
- 若需要稳定触发一条标准 `AgentSpoke` 供消息流验收，在 source runtime 启动前显式设置 `OASIS7_RUNTIME_AGENT_CHAT_ECHO=1`；该开关仅用于 Viewer / QA 测试态，默认产品路径必须保持关闭。
- 若 Viewer 页面长期停在 `connecting` 且 `logicalTime=0`，必须查看 `window.__AW_TEST__.getState().lastError`；命中 `copy_deferred_lighting_id_pipeline` / `CONTEXT_LOST_WEBGL` 等 fatal 时，按图形环境门禁失败处理，不进入玩法结论。
- `headed` 不是充分条件：若 `browser_env.json` / WebGL renderer 显示 `SwiftShader` 或其他 software renderer，先查看 `window.__AW_TEST__.getState().renderMode`。
  - `renderMode=software_safe`：允许继续做最小闭环验证（连接、选择目标、`step`、新反馈）。
  - `renderMode!=software_safe`：仍按图形环境阻断处理；默认先使用 `--use-angle=gl,--ignore-gpu-blocklist` 固定硬件路径。
- `oasis7_web_launcher` / launcher Web 控制面：默认优先使用 GUI Agent 驱动产品动作，再用 Web 页面做状态与字段校验；Canvas 直点仅作补充。制作人试玩与发布前人工验收若要进入真实产品路径，优先直接执行 `./scripts/run-producer-playtest.sh`（需要自动打开浏览器时加 `--open-headed`，脚本退出时会自动关闭该浏览器会话）；如需手动控制 bundle，再使用 `<bundle>/run-game.sh` 或 `./scripts/run-game-test.sh --bundle-dir <bundle>` 启动。
- agent / QA 若只是想在当前 worktree 内起一套隔离回归栈，优先执行 `./scripts/worktree-harness.sh up`，然后通过 `./scripts/worktree-harness.sh url` / `status --json` / `logs` 获取 URL 与状态；`run-game-test.sh` 继续作为该 harness 的底层启动器，不应再被当作并行 worktree 回归的顶层主入口。
- 不要把 Viewer 页面专用的 `agent-browser` 操作步骤直接套用到 launcher 控制面动作执行上。
- 涉及 `Explorer / Transfer` 的闭环时，先准备可观测数据，再执行查询与字段断言；不得只以“页面打开了/接口返回 200”判定通过。
- 防误用约束：
  - `scripts/run-game-test-ab.sh` 仅用于自动化回归哨兵（TTFC/命中率/无进展窗口）；推荐与 `--bundle-dir <bundle>` 搭配做产物态 smoke，但仍不等价于“真实玩家长玩评测”。
- `run-game-test-ab.sh --headless` 若命中 `SwiftShader` / software renderer，应先确认页面是否已自动切到 `software_safe`；只有未切入 safe-mode 时才按环境阻断处理，不得把 `connectionStatus=connecting` 误判为 fresh Web 构建或玩法回归；Viewer Web 默认继续使用 headed 模式。
  - 发布前结论仍需补充手动长玩与卡片填写（按 `doc/playability_test_result/game-test.prd.md` 执行）。
- 若改动影响前期工业引导（`首个制成品 / 停机恢复 / 首座工厂单元`），必须补跑 `doc/playability_test_result/topics/industrial-onboarding-required-tier-cards-2026-03-15.md` 中对应卡片，并把结论回写正式 playability 卡。
  - 对外样张链路需使用 strict 语义门禁，不得以 `off` / `soft` 结果作为发布判定证据。
- 若需要为 `#46 PostOnboarding` 补无 UI / 非浏览器验证，执行 `./scripts/viewer-post-onboarding-headless-smoke.sh`。
  - 该脚本只验证 live TCP 协议、快照推进、控制完成 ack 与 runtime event feed；不替代 headed Web/UI 截图复核。
- 若需要直接以纯 API 客户端操作 live 会话，可使用 `cargo run -q -p oasis7 --bin oasis7_pure_api_client -- ...`。
  - 该链路属于 `pure_api` 玩家访问模式；若同时牵涉 OpenClaw agent，应额外标注实际 execution lane。
  - 推荐最小链路：
```bash
cargo run -q -p oasis7 --bin oasis7_pure_api_client -- --addr 127.0.0.1:5023 snapshot --player-gameplay-only
cargo run -q -p oasis7 --bin oasis7_pure_api_client -- --addr 127.0.0.1:5023 step --count 8 --events
cargo run -q -p oasis7 --bin oasis7_pure_api_client -- keygen
cargo run -q -p oasis7 --bin oasis7_pure_api_client -- --addr 127.0.0.1:5023 reconnect-sync --player-id player-1 --with-snapshot
```
  - 若要覆盖 `agent_chat` / `prompt_control`，需先 `keygen`，再携带 `--player-id` 与 `--private-key-hex` 走签名请求；当前产品设定下，只要 LLM 不可用，`step / play / gameplay_action / agent_chat / prompt_control` 都会被阻断为 `llm_mode_required` 或 `llm_init_failed`。
- 若需要执行 pure API required/full 回归，优先运行 `./scripts/oasis7-pure-api-parity-smoke.sh`。
  - 该回归验证的是 `pure_api` 玩家访问模式在 active LLM access 下的正式可玩性，不等同于 OpenClaw `headless_agent` 回归。
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
    当前脚本已覆盖 `player_gameplay`、正式 `gameplay_action` 推进、`reconnect-sync --with-snapshot` 恢复，以及 `FirstSessionLoop -> PostOnboarding -> choose_midloop_path` 的 required/full 收口路径。
    `parity_verified` 正式判定继续以 `doc/testing/evidence/pure-api-parity-validation-2026-03-19.md` 为准；当前产品设定已把该结论收口为“仅适用于 active LLM access 路径”，若重跑 no-LLM 仅能记为 blocked/observer-debug。
- 快速入口：
```bash
./scripts/run-producer-playtest.sh
./scripts/run-producer-playtest.sh --open-headed
./scripts/worktree-harness.sh up
./scripts/worktree-harness.sh status --json
./scripts/worktree-harness.sh down
./scripts/build-game-launcher-bundle.sh --out-dir output/release/game-launcher-local
./scripts/run-game-test.sh --bundle-dir output/release/game-launcher-local --with-llm
./scripts/run-game-test-ab.sh --bundle-dir output/release/game-launcher-local --with-llm
./scripts/viewer-post-onboarding-qa.sh --bundle-dir output/release/game-launcher-local --with-llm
./scripts/viewer-post-onboarding-headless-smoke.sh --bundle-dir output/release/game-launcher-local --with-llm
./scripts/viewer-software-safe-chat-regression.sh --bundle-dir output/release/game-launcher-local
cargo run -q -p oasis7 --bin oasis7_pure_api_client -- --addr 127.0.0.1:5023 snapshot --player-gameplay-only
./scripts/oasis7-pure-api-parity-smoke.sh --tier required --bundle-dir output/release/game-launcher-local --with-llm
./scripts/viewer-release-qa-loop.sh
./scripts/viewer-release-full-coverage.sh --quick
./scripts/viewer-release-art-baseline.sh
```

### S7：场景矩阵回归套件（L1 + L4）
```bash
env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_required scenario_specs_match_ids -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_required scenarios_are_stable -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_full oasis7_init_demo_runs_ -- --nocapture
```
- 配套文档：`doc/world-simulator/scenario/scenario-files.prd.md` 的“场景测试覆盖矩阵”。

### S6.5：Chain Runtime Storage Profile / Gate 核验（L4/L5 补充）
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

### S8：长稳与压力套件（L5）
- Viewer 压测：
```bash
./scripts/viewer-owr4-stress.sh --duration-secs 45 --scenarios triad_region_bootstrap,llm_bootstrap
```
- LLM 长稳：
```bash
./scripts/llm-longrun-stress.sh --scenario llm_bootstrap --ticks 240
```
- LLM 覆盖门禁（发行口径）：
```bash
./scripts/llm-longrun-stress.sh --scenario llm_bootstrap --ticks 240 --release-gate --release-gate-profile hybrid
```
- LLM gameplay 对照（bridge 开/关）：
```bash
./scripts/llm-longrun-stress.sh --scenario llm_bootstrap --ticks 240 --prompt-pack story_balanced --runtime-gameplay-bridge
./scripts/llm-longrun-stress.sh --scenario llm_bootstrap --ticks 240 --prompt-pack story_balanced --no-runtime-gameplay-bridge
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
  - `viewer-owr4-stress` 在无 `OPENAI_API_KEY` 时，`llm_bootstrap` 会退化为 script_fallback；
  - `scripts/ci-tests.sh full` 已接入 `./scripts/llm-baseline-fixture-smoke.sh`；
  - 压测结果需保留 CSV/summary/log 产物。

### S9：P2P/存储/共识在线长跑套件（L5）
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
- 漂移定位/回滚演练门禁（TASK-GAME-014）：
```bash
env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_required runtime::tests::persistence::rollback_with_reconciliation_recovers_from_detected_tick_consensus_drift -- --nocapture
```
- 演练通过标准：
  - 能定位 `mismatch_tick`；
  - `rollback_to_snapshot_with_reconciliation` 后 `first_tick_consensus_drift() == None`；
  - `verify_tick_consensus_chain()` 通过。
- 参考文档：`doc/testing/longrun/chain-runtime-soak-script-reactivation-2026-02-28.prd.md`、`doc/testing/longrun/p2p-storage-consensus-longrun-online-stability-2026-02-24.prd.md`。
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

### S10：五节点真实游戏数据在线长跑套件（L5）
- 当前状态（2026-02-28）：`scripts/s10-five-node-game-soak.sh` 已恢复为可执行脚本，底座为五进程 `oasis7_chain_runtime`。
- 当前状态补充（2026-03-01）：reward worker 在空存储时会自动写入 distfs probe seed blob，发布基线下 `distfs_total_checks` 应为正数。
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
- 参考文档：`doc/testing/longrun/chain-runtime-soak-script-reactivation-2026-02-28.prd.md`、`doc/testing/longrun/s10-five-node-real-game-soak.prd.md`。

### 发布门禁一键收口（S0 + S1 + S6 + S9 + S10）
```bash
./scripts/release-gate.sh
./scripts/release-gate.sh --quick
./scripts/release-gate.sh --dry-run
```
- 默认串行执行：`ci-tests full`、`sync-m1/m4/m5 --check`、Web strict、S9/S10。
- `--quick` 用于缩短 S9/S10 时长并关闭 Web visual baseline。

### Shared Network / Release Train Minimum（Benchmark L5，首轮 dry run 已落地）
- 参考专题：
  - `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.prd.md`
  - `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.project.md`
- 当前状态：
  - `partial`
  - 这表示 first `shared_devnet` dry run 已完成并留有正式 evidence，但 shared access / multi-entry / longrun / rollback target 仍未达到 promotion-ready `pass`，`staging/canary` 也尚未执行。
- 通过 shared-network gate 之前，以下表述都不允许：
  - `production release train is established`
  - `shared network validated`
  - `mainnet-grade testing maturity`
- 当前最小执行顺序：
  1. 生成统一 `release_candidate_bundle`
  2. 执行 first `shared_devnet` dry run（当前已完成，结论 `partial`）
  3. 把 `shared_devnet` 从 `partial` 提升到 `pass`
  4. 执行 `staging` rehearsal
  5. 执行 `canary` rehearsal
- 当前 evidence 入口：
  - `doc/testing/evidence/shared-network-shared-devnet-dry-run-2026-03-24.md`
  - `doc/testing/evidence/shared-network-shared-devnet-promotion-record-2026-03-24.md`
  - `doc/testing/evidence/shared-network-shared-devnet-incident-2026-03-24.md`
- 当前 runtime 入口：
```bash
./scripts/release-candidate-bundle.sh create \
  --bundle output/release-candidates/shared-devnet-01.json \
  --candidate-id shared-devnet-01 \
  --track shared_devnet \
  --runtime-build-ref <runtime-build-path> \
  --world-snapshot-ref <world-snapshot-path> \
  --governance-manifest-ref <governance-manifest-path> \
  --evidence-ref <evidence-path>

./scripts/release-candidate-bundle.sh validate \
  --bundle output/release-candidates/shared-devnet-01.json \
  --check-git-head

./scripts/release-gate.sh --candidate-bundle output/release-candidates/shared-devnet-01.json --dry-run
./scripts/shared-devnet-rehearsal.sh \
  --window-id shared-devnet-20260324-02 \
  --candidate-bundle output/release-candidates/shared-devnet-01.json \
  --bundle-dir output/release/game-launcher-local \
  --viewer-port 4174 \
  --live-bind 127.0.0.1:5123 \
  --web-bind 127.0.0.1:5111 \
  --release-gate-mode dry-run \
  --web-mode execute \
  --headless-mode execute \
  --pure-api-mode execute \
  --longrun-mode dry-run
./scripts/shared-devnet-rehearsal-smoke.sh
./scripts/shared-devnet-blocker-packet.sh \
  --window-id shared-devnet-20260324-06 \
  --candidate-bundle output/release-candidates/shared-devnet-20260324-05.json \
  --candidate-gate-summary output/shared-network/shared-devnet-20260324-06/gate/shared_devnet-20260324-175501/summary.md \
  --access-out doc/testing/evidence/shared-network-shared-devnet-shared-access-draft-2026-03-24.md \
  --rollback-out doc/testing/evidence/shared-network-shared-devnet-rollback-target-draft-2026-03-24.md
./scripts/shared-devnet-blocker-packet-smoke.sh
./scripts/release-candidate-bundle-smoke.sh
./scripts/shared-network-track-gate.sh \
  --track shared_devnet \
  --candidate-bundle output/release-candidates/shared-devnet-01.json \
  --lanes-tsv doc/testing/templates/shared-network-track-gate-lanes.shared_devnet.template.tsv \
  --out-dir output/shared-network-gates
./scripts/shared-network-track-gate-smoke.sh
```
- 当前 `release_candidate_bundle` 最小职责：
  - 固定 `candidate_id`
  - 固定 `git_commit`
  - 固定 `runtime_build/world_snapshot/governance_manifest` 的路径与 hash
  - 固定 `evidence_refs`
- 当前 `shared-devnet-rehearsal` 最小职责：
  - 复用同一 `candidate_bundle` 作为 shared-devnet 编排真值
  - 用 execute/evidence/skip 模式统一收口 same-candidate `headed Web + no-ui + pure_api`
  - 统一生成 `multi-entry-summary`、lane scaffold、`lanes.shared_devnet.tsv` 与 gate 输出
  - 未提供 shared access / rollback / governance / short-window 新证据时，默认保持保守 `partial`，避免误判为已 `pass`
- 当前 `shared-devnet-blocker-packet` 最小职责：
  - 基于已通过的 candidate bundle 和当前 gate 输出，生成 `shared_access` / `rollback_target_ready` 的实例草稿
  - 固定最后两条 blocker 的留证字段，避免后续 shared operator / fallback candidate 输入到位后还要手工重写结构
- 当前 QA gate 最小职责：
  - 按 `shared_devnet / staging / canary` 校验 required lanes 是否齐全
  - 统一输出 `pass / partial / block`
  - 统一生成 `summary.json` 与 `summary.md`
  - 缺 required lane 时直接 `block`
- QA 模板入口：
  - `doc/testing/templates/shared-network-track-gate-template.md`
  - `doc/testing/templates/shared-network-track-gate-lanes.shared_devnet.template.tsv`
  - `doc/testing/templates/shared-network-track-gate-lanes.staging.template.tsv`
  - `doc/testing/templates/shared-network-track-gate-lanes.canary.template.tsv`
- LiveOps runbook 入口：
  - `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.runbook.md`
- LiveOps 模板入口：
  - `doc/testing/templates/shared-network-promotion-record-template.md`
  - `doc/testing/templates/shared-network-incident-template.md`
  - `doc/testing/templates/shared-network-incident-review-template.md`
  - `doc/testing/templates/shared-network-exit-decision-template.md`
  - `doc/testing/templates/shared-network-shared-access-check-template.md`
  - `doc/testing/templates/shared-network-rollback-target-template.md`
- 当前 liveops 最小职责：
  - 每个窗口固定 `window_id/candidate_id/fallback_candidate_id/owners_on_duty/claim_envelope`
  - 发现真值漂移、QA `block`、共享访问失效或 preview 口径越界时立即 `freeze`
  - `rollback` 只能回到最近一次 `pass` 的 candidate bundle
  - 没有 `promotion_record`、`incident_review`、`exit_decision` 的 track 不得记完成
- 当前 `shared_devnet` dry-run 结论：
  - `gate_result=partial`
  - `promotion_recommendation=hold_promotion`
  - 这不是 shared-network `pass`，也不允许升级 public claims
- 当前 follow-up window `shared-devnet-20260324-05` 结论：
  - `candidate_bundle_integrity=pass`
  - `multi_entry_closure=pass`
  - `governance_live_drill=pass`
  - `shared_access=partial`
  - `short_window_longrun=partial`
  - `rollback_target_ready=partial`
  - 因此整体仍是 `gate_result=partial`
- 当前 short-window follow-up `shared-devnet-20260324-06` 结论：
  - `candidate_bundle_integrity=pass`
  - `multi_entry_closure=pass`
  - `governance_live_drill=pass`
  - `short_window_longrun=pass`
  - `shared_access=partial`
  - `rollback_target_ready=partial`
  - 因此 shared-devnet 剩余 blocker 只收敛到 `shared_access / rollback_target_ready`
- `--dry-run` 用于门禁编排冒烟，不执行真实命令。

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
| S0 | 基础门禁 / 文档 / shell / 格式 / 快速健康检查 | 任何代码、脚本、文档、工作流改动 | 命令日志 + 通过/失败结论 |
| S1 | 核心 required | `oasis7` 主链路代码改动 | required 测试日志 |
| S2 | 核心 full | 发布前、协议/规则高风险改动、required 无法充分覆盖时 | full 测试日志 |
| S3 | 应用主链定向 | runtime / simulator / viewer live / web bridge 定向改动 | 定向 cargo test 日志 |
| S4 | 分布式子系统 | node / net / consensus / distfs / P2P 链路改动 | 子系统测试日志 |
| S5 | viewer crate / wasm 编译 | `crates/oasis7_viewer/**` 或 viewer wasm 构建链路改动 | viewer 单测 + wasm 编译日志 |
| S6 | Web UI 闭环 smoke | Viewer / launcher / Web 控制台 / 交互链路改动 | 截图、console、语义结果 |
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
| `crates/oasis7_viewer/**` | S0 + S5 + S6 | S2 + S8 | 若改动只影响静态资源 / 样式，可抽样 S1；若影响 bridge，追加 S3 |
| `crates/oasis7_node/**` | S0 + S4（node） + S9/S10（按改动面至少一条） | S2 + S3 + S8 + 另一条在线长跑（S9 或 S10） | 共识推进 / 节点编排改动优先加 S10；网络 / 复制改动优先加 S9 |
| `crates/oasis7_net/**` | S0 + S4（net） + S9/S10（按改动面至少一条） | S2 + runtime_bridge 变体 + S8 + 另一条在线长跑（S9 或 S10） | 若仅桥接层改动，可用 S3 + S9 smoke；若影响真实联机，补 S10 |
| `crates/oasis7_consensus/**` | S0 + S4（consensus） + S9/S10（按改动面至少一条） | S2 + S8 + 另一条在线长跑（S9 或 S10） | epoch / attest / finality 逻辑改动优先补 S10 |
| `crates/oasis7_distfs/**` | S0 + S4（distfs） + S9/S10（按改动面至少一条） | S2 + S8 + 另一条在线长跑（S9 或 S10） | 存储复制 / challenge / 修复逻辑改动优先补 S9 |
| `doc/**`（非 `doc/devlog/**`） | S0（含 `./scripts/doc-governance-check.sh`） | 命中模块的抽样 required 证据核验 | 若文档改变发布 / 测试口径，追加对应模块的最小必跑集 |
| `scripts/ci-tests.sh` / `.github/workflows/rust.yml` | S0（含 `./scripts/doc-governance-check.sh`） + S1 + `./scripts/viewer-visual-baseline.sh` + （full）`./scripts/llm-baseline-fixture-smoke.sh` | S2 + S4 + S6（抽样） | 若更改默认 gate 组合，需抽样至少一条 S9 或 S10 |
| `scripts/release-gate.sh` / `.github/workflows/release-packages.yml` | `./scripts/ci-tests.sh full` + `sync-m1/m4/m5 --check` + Web strict + S9 + S10 | `./scripts/release-gate.sh --quick` / `--dry-run` | 任何发布 gate 逻辑变更均不允许跳过 S9/S10 |
| `scripts/ci-m1-wasm-summary.sh` / `scripts/ci-verify-m1-wasm-summaries.py` / `scripts/wasm-release-evidence-report.sh` / `.github/workflows/wasm-determinism-gate.yml` | `S0` + `./scripts/ci-m1-wasm-summary.sh --module-set m4 --runner-label linux-x86_64 --out output/ci/m4-wasm-summary/linux-x86_64.json` + `./scripts/wasm-release-evidence-report.sh --module-sets m4 --skip-collect --summary-import-dir output/ci/m4-wasm-summary --expected-runners linux-x86_64` | `workflow_dispatch` 触发 GitHub-hosted Linux runner gate；若补入外部 macOS summary，可再用 `--expected-runners linux-x86_64,darwin-arm64` 做双宿主对账 | 若改动 hash/summary/evidence report 格式，Linux gate 必跑；跨宿主 full-tier 在有 Docker-capable macOS summary 时追加 |
| `scripts/run-viewer-web.sh` / `scripts/capture-viewer-frame.sh` | S0 + S6 | S5 + S8 | 若涉及 native 图形链路 fallback，补 native 截图证据 |
| `scripts/p2p-longrun-soak.sh` / `doc/testing/p2p-storage-consensus-longrun-online-stability-2026-02-24*` | S0 + S9 smoke（含 summary/timeline 校验） | S9 endurance（含 chaos） | 任何阈值/summary 字段变更必须补 endurance |
| `scripts/s10-five-node-game-soak.sh` / `doc/testing/s10-five-node-real-game-soak*` | S0 + S10 smoke（含 summary/timeline 校验） | S10 默认长窗（30min+） | 任何门禁字段 / 结算 / mint 改动都需补长窗 |

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
6. L5 失败：判定是否性能退化、资源泄漏、长时状态累计问题。

## TODO（待收口）
- [x] TODO-1：修正 S7 场景矩阵回归命令的覆盖口径。
  - 处理结果（2026-03-05）：S7 的 `oasis7_init_demo_runs_` 已切换到 `test_tier_full` 执行档位。
  - 验收记录：`env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_full oasis7_init_demo_runs_ -- --nocapture` 命中多场景用例（非 1 条）。

- [x] TODO-2：修复 S5 `oasis7_viewer` 测试编译阻塞。
  - 处理结果（2026-02-21）：`oasis7_viewer` 测试集已恢复可编译可执行，并已纳入 `scripts/ci-tests.sh` 的 `required/full` 默认 gate。
  - 验收记录：`env -u RUSTC_WRAPPER cargo test -p oasis7_viewer` 通过，且 `required-gate` 增加 `./scripts/viewer-visual-baseline.sh`。

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
