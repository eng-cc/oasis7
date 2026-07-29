# oasis7 Runtime：WASM Docker 确定性构建与工件治理管线（项目管理）

- 对应设计文档: `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.design.md`
- 对应需求文档: `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md`

审计轮次: 3

## 任务拆解（含 PRD-ID 映射）
- [x] WDBP-0 (PRD-WORLD_RUNTIME-020/021/022) [test_tier_required]: 将专题目标从“host deterministic guard + keyed 平台 hash 对账”修正为“Docker-first canonical builder”，并回写 root PRD / project / README / devlog。
- [x] WDBP-1 (PRD-WORLD_RUNTIME-020/021) [test_tier_required]: 新增 pinned WASM builder image（`docker/wasm-builder/Dockerfile`）与 host wrapper，固定 `linux-x86_64` container platform 作为 canonical publish build 平台。
- [x] WDBP-2 (PRD-WORLD_RUNTIME-020/021) [test_tier_required]: 将现有 `tools/wasm_build_suite` 收敛到容器内执行，输出 build receipt，并把 manifest 从多宿主 keyed token 迁移为单 canonical token `linux-x86_64=<sha256>`。
- [x] WDBP-2.1 (PRD-WORLD_RUNTIME-020/021) [test_tier_required]: 将 host wrapper、builder image、sync/check、CI summary 与 build suite 的 operator env key 收口到 `OASIS7_WASM_*` 当前入口，并移除旧品牌 wasm 运行入口。
- [x] wasm-source-hash-dependency-closure (PRD-WORLD_RUNTIME-021) [test_tier_required]: 将 `source_hash` 从“模块目录白名单”收紧为“模块源码 + 本地 `path` 依赖闭包”，并让 `tools/wasm_build_suite` 与 `sync_builtin_wasm_identity` 共用同一计算逻辑。 Trace: .pm/tasks/task_075b812172914487a06a93bda125bc9f.yaml
- [x] WDBP-3 (PRD-WORLD_RUNTIME-021/022) [test_tier_required + test_tier_full]: 将 identity / release evidence / CI summary / release gate 全面切换为 Docker canonical hash，对 macOS/Linux 只比较容器输出，不再比较 host-native 输出。
  - [x] WDBP-3.1 (PRD-WORLD_RUNTIME-021) [test_tier_required]: 固化 stable gate / full-tier cross-host evidence 的双层结论模型，并让 `wasm-release-evidence-report` 输出 `expected_runners/received_runners/cross_host_evidence_pending`。
  - [x] WDBP-3.2 (PRD-WORLD_RUNTIME-021/022) [test_tier_full]: 补齐真实 Docker-capable `darwin-arm64` summary 导入链路，使 release evidence 至少包含 `linux-x86_64 + darwin-arm64` 两类 runner 输入。Trace: .pm/tasks/task_0a6477b5b6b34b869c8b85c81c554dc0.yaml
    - [x] WDBP-3.2a (PRD-WORLD_RUNTIME-021/022) [test_tier_required]: 加固 external summary bundle 导入验真，拒绝 `host_platform` 或 `canonical_platform` 与 `darwin-arm64 + linux-x86_64 canonical builder` 目标态不一致的伪装证据。
  - [x] WDBP-3.3 (PRD-WORLD_RUNTIME-022) [test_tier_required]: 在 production runtime / node 主入口绑定 hardened `ReleaseSecurityPolicy`，并把 effective policy 写入 status / acceptance evidence。
- [x] WDBP-4 (PRD-WORLD_RUNTIME-022) [test_tier_required]: 把 `compile_module_artifact_from_source` 的生产路径外移到 external Docker builder 或 production 默认禁用，runtime 只消费 binary + build receipt。

## 已吸收的 historical nightly build-std 记录

早期 nightly build-std 专题已完成，并已将其唯一仍有价值的历史语义收口到本节；它不是当前发布链路，也不恢复 host-native build 为有效入口。

- 历史问题与目标：为 builtin WASM 固定输入闭环，消除宿主预编译 `std` 差异造成的 hash 漂移；当时通过路径归一化（`--remap-path-prefix`）和 WASM custom-section canonicalize，使 hash 只反映可执行语义。
- 历史构建约束：pinned `nightly-2025-12-11`（`OASIS7_WASM_TOOLCHAIN=nightly-2025-12-11`）、`rust-src`、`wasm32-unknown-unknown`，并以 `OASIS7_WASM_BUILD_STD=1`、`OASIS7_WASM_BUILD_STD_COMPONENTS=std,panic_abort` 和空的 `OASIS7_WASM_BUILD_STD_FEATURES` 驱动 `-Z build-std`（不追加 `-Z build-std-features`）。
- 历史范围边界：该轮不改 runtime ABI、DistFS 协议、hash 算法或 manifest 文件格式，也不定义发布级 Docker canonical builder；后者现由本专题的 active contract 定义。
- 历史实施与验证：`scripts/build-wasm-module.sh` 与 aggregate 入口 `scripts/build-builtin-wasm-modules.sh` 负责 toolchain/组件准备，`tools/wasm_build_suite` 注入受环境变量控制的 `-Z build-std*` 参数；CI 固化对应环境与组件；m1/m4 hash 清单完成同步，并通过 `scripts/sync-m1-builtin-wasm-artifacts.sh --check`、`scripts/sync-m4-builtin-wasm-artifacts.sh --check` 和 `CI_VERBOSE=1 ./scripts/ci-tests.sh required`。
- 已完成的 NBS-1 至 NBS-8：需求/设计/项目文档、nightly 构建入口、build-suite 参数、CI 环境、m1/m4 清单同步及 required-tier 回归均于 2026-02-17 收口；2026-03-03 仅完成命名迁移，不改变该历史结论。
- 历史风险：`-Z build-std` 首次构建成本较高；nightly 升级或不可用会改变 hash 或阻断 CI，故必须显式 pin 并按升级流程重新验证。

当前 authoritative policy 取代上述历史实现：所有 publishable WASM 走 digest-pinned Docker canonical builder，发布清单只写 `linux-x86_64` canonical token，并以 build receipt、identity/release evidence、cross-host Docker evidence 和 binary-only runtime consumption 作为有效发布边界。历史 build-std 记录只用于参数、旧 CI 约束和 hash 漂移背景追溯。

## 依赖
- `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md`
- `scripts/build-wasm-module.sh`
- `tools/wasm_build_suite/src/lib.rs`
- `crates/oasis7/src/runtime/module_source_compiler.rs`
- `scripts/sync-m1-builtin-wasm-artifacts.sh`
- `scripts/sync-m4-builtin-wasm-artifacts.sh`
- `scripts/sync-m5-builtin-wasm-artifacts.sh`
- `crates/oasis7_distfs/src/bin/sync_builtin_wasm_identity.rs`
- `scripts/ci-m1-wasm-summary.sh`
- `scripts/ci-verify-m1-wasm-summaries.py`
- `crates/oasis7/src/runtime/builtin_wasm_materializer.rs`
- `crates/oasis7/src/runtime/world/release_manifest.rs`

## 状态
- 更新日期: 2026-06-28
- 当前阶段: WDBP-3 跨宿主 evidence 已收口（WDBP-3.1 / WDBP-3.2 / WDBP-3.3 已完成，WDBP-4 已完成）
- 专业权威合并：已吸收早期 builtin DistFS storage/API closure 的有效合同，按当前 `m1/m4/m5`、SHA-256 hydrate、loader re-verification、identity/receipt 与 production hardened policy 重述；旧的 m1/m4-only 和 local-DistFS-only 描述不再作为当前真值。
- WDBP-3 收口证据:
  - `WDBP-3.2`: 真实 Docker-capable `darwin-arm64` summary/evidence 已由本机 self-hosted runner `oasis7-Mac-darwin-arm64-docker` 产出，并在 `Wasm Darwin Docker Evidence` main run `28297899706` 中与 GitHub-hosted `linux-x86_64` summaries 完成 cross-host report；Darwin job `83840926310` 与 Linux verify job `83843884654` 均通过。Trace: .pm/tasks/task_0a6477b5b6b34b869c8b85c81c554dc0.yaml
- owner role: `wasm_platform_engineer`
- 联审角色: `producer_system_designer`、`runtime_engineer`
- 验证角色: `qa_engineer`
- 阻塞项:
  - 无 WDBP-3.2 P0 阻塞；真实 Linux + Docker-capable macOS full-tier 证据已由 GitHub Actions run `28297899706` 产出并上传为 workflow artifacts。该证据不作为 repo-tracked binary 长期归档，长期复核索引用 run URL、job id 与 `.pm` execution log 保留。
  - GitHub-hosted `macos-14` runner 仍不能被当作 Docker-capable `darwin-arm64` producer；该能力边界由 self-hosted runner workflow 承接。
- 实施备注:
  - `docker/wasm-builder/Dockerfile` 与 `scripts/build-wasm-module.sh` 已落地，当前 canonical build 已收敛为 Docker-only path，不再提供 host-native fallback。
  - `scripts/build-wasm-module.sh`、`scripts/sync-m1-builtin-wasm-artifacts.sh`、`scripts/ci-m1-wasm-summary.sh`、`tools/wasm_build_suite` 与 `docker/wasm-builder/Dockerfile` 现已只读取/写入 `OASIS7_WASM_*` 当前入口，避免 operator 脚本与容器镜像继续扩散旧前缀。
  - runtime `builtin_wasm_materializer`、`m1/m4/m5_builtin_wasm_artifact` 与 `runtime/world/release_manifest` 现已只读取 `OASIS7_BUILTIN_WASM_*` 当前入口，避免构建链路已迁移后 runtime materialize/fetch/fallback 仍停留在旧前缀。
  - `tools/wasm_build_suite` 已新增 `build receipt`、`source_hash`、`build_manifest_hash`、`builder_image_digest` 与 `container_platform` 输出；builtin `m1/m4/m5` hash manifest 已全部改写为单 canonical token `linux-x86_64=<sha256>`。
  - `2026-05-22` 已补 `wasm-source-hash-dependency-closure`：`source_hash` 现通过共享 `oasis7_wasm_build` helper 基于 `cargo metadata --filter-platform wasm32-unknown-unknown` 计算模块与本地 `path` 依赖闭包；`tools/wasm_build_suite` 与 `sync_builtin_wasm_identity` 不再各自维护目录白名单实现。
  - `crates/oasis7_distfs/src/bin/sync_builtin_wasm_identity.rs` 已切换为 receipt 驱动 identity 生成；写路径只输出 canonical token，读路径仍兼容 legacy multi-token manifest。
  - `scripts/ci-m1-wasm-summary.sh` 与 `scripts/ci-verify-m1-wasm-summaries.py` 已区分 `host_platform` 与 `canonical_platform`，并新增 `receipt_evidence + identity_build_recipe` 对账；当前 CI 对账口径改为“不同宿主只比较 Docker canonical 输出与一致的 receipt/build recipe 证据”。
  - runtime `ModuleReleaseSubmitAttestation -> apply` 现已显式绑定 `builder_image_digest + container_platform + canonicalizer_version`；release gate 会拒绝阈值 attestation 间的 receipt evidence 不一致，且要求 attestation 的 `source_hash/build_manifest_hash/wasm_hash` 与 manifest identity 对齐。
  - `ModuleReleaseManifestMappingState` 与节点验收脚本现已补齐 release evidence 摘要：映射状态会落盘 `release_{wasm,source,build_manifest}_hash + builder_image_digest + container_platform + canonicalizer_version + attestation_platforms + proof_cids + receipt_evidence_conflict`，`scripts/module-release-node-acceptance.sh` 也已纳入 receipt mismatch 阻断用例。
  - 新增 `scripts/wasm-release-evidence-report.sh` 作为多 runner fixed entry，可统一收集/校验 `m1/m4/m5` summary 并输出 `summary.md/json`；当前 `.github/workflows/wasm-determinism-gate.yml` 已切换到 `--summary-import-dir` 模式，会把下载下来的 runner summaries 统一收口为可归档 evidence artifact。
  - `scripts/ci-verify-m1-wasm-summaries.py` 与 `scripts/wasm-release-evidence-report.sh` 现已把 `required_runners`（stable gate）与 `expected_runners`（full-tier cross-host evidence）拆开；GitHub-hosted workflow 当前以 `linux-x86_64` 作为 required runner，但 summary/report 会显式输出 `received_runners + missing_runners + cross_host_evidence_pending + gate_result=conditional-go`。
  - `WDBP-3.2` 的导入链路现已落地：`scripts/package-wasm-summary-bundle.sh` 可把外部 Docker-capable runner 的 `m1/m4/m5` summary 打成标准 bundle，`scripts/stage-wasm-summary-imports.sh` 可在 verify 前把 GitHub-hosted Linux summary 与外部 bundle 合并到同一 import dir；`workflow_dispatch` 也新增了 `external_summary_bundle_url` / `external_summary_runner_label` 入口。
  - `2026-06-27` 新增 `.github/workflows/wasm-darwin-docker-evidence.yml` 作为手动 Docker evidence workflow：第一段在 Docker-capable `darwin-arm64` self-hosted runner 上直接产出 summary bundle artifact，第二段可在 `ubuntu-24.04` 下载该 artifact、收集 `linux-x86_64` summaries，并通过既有 `stage-wasm-summary-imports.sh` + `wasm-release-evidence-report.sh` 生成 cross-host report。
  - 口径约束：`workflow_dispatch + external_summary_bundle_url` 代表“CI verify ready / external evidence import ready”，不代表 GitHub-hosted CI 已获得 `darwin-arm64` 产出能力；`WDBP-3.2` / cross-host closure 只有在真实 Docker-capable `darwin-arm64` runner 提供 live summary/proof 后才可宣称完成。当前完成证据为 `Wasm Darwin Docker Evidence` main run `28297899706`。
  - `WDBP-3.2a` 已继续加固 external bundle 验真：`package/stage/verify/report` 链路现在会强校验 summary/bundle 的 `host_platform` 与 `canonical_platform=linux-x86_64`，并通过 `scripts/wasm-summary-bundle-smoke.sh` 固定覆盖“真实 darwin bundle 可导入、伪装 darwin 的 linux bundle 必须失败”。
  - 仓库内已补 `scripts/dispatch-wasm-determinism-gate.sh` 作为 operator 入口，用于以 `gh workflow run` 触发带外部 bundle URL 的 full-tier evidence run；当前优先闭环路径为 `.github/workflows/wasm-darwin-docker-evidence.yml` 在 self-hosted Darwin runner 上直接产出 bundle，再由 Linux verify job 上传 cross-host report artifact 并记录 run/job 索引。
  - 节点侧 proof 收口已落地：`scripts/module-release-node-attestation-flow.sh` 现可在发布节点本地执行 `summary collect/import -> evidence verify -> canonical proof inputs -> proof payload -> attestation submit`，并刻意剥离 summary/report 中的时间戳与本地路径，避免把非语义字段写入 `proof_cid`。
  - `scripts/module-release-node-acceptance.sh` 现已新增 `required_attestation_flow` smoke，基于合成 `linux-x86_64 + darwin-arm64` summary 验证节点侧固定入口可以稳定生成 `proof_payload.json + submit_request.json`。
  - GitHub-hosted `macos-14` runner 当前不提供 Docker daemon，而 canonical build 已变为 Docker-only path；因此 workflow 已临时收敛为 Linux-only gate，跨宿主对账继续通过导入外部 Docker-capable macOS summary 的方式完成。
  - builtin wasm fallback materializer 现已把临时输出目录收敛到仓库内 `.tmp/`，避免与 Docker-only wrapper 的 workspace-root 约束冲突；同时 canonical builder receipt 默认复用受控 `builder_image_digest`，避免 CI 本地 image id 漂移直接打穿 identity manifest 对账。
  - `compile_module_artifact_from_source` 现已完成 production gate：`ReleaseSecurityPolicy` 新增 `allow_runtime_source_compile`，production 默认关闭该路径并要求改走 external Docker builder + deploy binary；dev/test 保留该 action 以支撑现有回归。
  - `oasis7_chain_runtime` 现已把 `release_default` storage profile 绑定到 hardened `ReleaseSecurityPolicy`，并通过 `/v1/chain/status` 输出 effective policy；`NodeRuntimeExecutionDriver::new_with_storage_profile` 会在装载 execution world 时同步应用该 policy。
  - `scripts/module-release-node-acceptance.sh` 现已新增 `required_release_policy` 步骤，并在 `.tmp/module_release_node_acceptance/20260318-134705/summary.json` 留下 production policy binding/status 证据。
  - `2026-03-31` 已补完 `WDBP-3.3` 的 runtime 侧余量审计：`viewer runtime_live` bootstrap、`governance_registry_import` 新建/加载 world、`reward_runtime_worker` 以及 `execution_bridge::load_execution_world` 的缺档案/旧样本装载路径现也统一切到 hardened `ReleaseSecurityPolicy`，避免 binary-only 语义只停留在 chain runtime 主入口。
  - `2026-06-28` 复核确认，`WDBP-3.2` 原阻塞已由 self-hosted Darwin Docker runner 与 main run `28297899706` 收口；真实 Docker-capable `darwin-arm64` live summary bundle 与 `linux-x86_64 + darwin-arm64` cross-host evidence report 由该 GitHub Actions run 产出并上传为 artifacts，repo 内保留 run/job 索引和 `.pm` evidence log。
