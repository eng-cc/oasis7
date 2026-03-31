# oasis7 Runtime：WASM Docker 确定性构建与工件治理管线（项目管理）

- 对应设计文档: `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.design.md`
- 对应需求文档: `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md`

审计轮次: 3

## 任务拆解（含 PRD-ID 映射）
- [x] WDBP-0 (PRD-WORLD_RUNTIME-020/021/022) [test_tier_required]: 将专题目标从“host deterministic guard + keyed 平台 hash 对账”修正为“Docker-first canonical builder”，并回写 root PRD / project / README / devlog。
- [x] WDBP-1 (PRD-WORLD_RUNTIME-020/021) [test_tier_required]: 新增 pinned WASM builder image（`docker/wasm-builder/Dockerfile`）与 host wrapper，固定 `linux-x86_64` container platform 作为 canonical publish build 平台。
- [x] WDBP-2 (PRD-WORLD_RUNTIME-020/021) [test_tier_required]: 将现有 `tools/wasm_build_suite` 收敛到容器内执行，输出 build receipt，并把 manifest 从多宿主 keyed token 迁移为单 canonical token `linux-x86_64=<sha256>`。
- [x] WDBP-2.1 (PRD-WORLD_RUNTIME-020/021) [test_tier_required]: 将 host wrapper、builder image、sync/check、CI summary 与 build suite 的 operator env key 收口到 `OASIS7_WASM_*` 当前入口，并移除旧品牌 wasm 运行入口。
- [ ] WDBP-3 (PRD-WORLD_RUNTIME-021/022) [test_tier_required + test_tier_full]: 将 identity / release evidence / CI summary / release gate 全面切换为 Docker canonical hash，对 macOS/Linux 只比较容器输出，不再比较 host-native 输出。
  - [x] WDBP-3.1 (PRD-WORLD_RUNTIME-021) [test_tier_required]: 固化 stable gate / full-tier cross-host evidence 的双层结论模型，并让 `wasm-release-evidence-report` 输出 `expected_runners/received_runners/cross_host_evidence_pending`。
  - [ ] WDBP-3.2 (PRD-WORLD_RUNTIME-021/022) [test_tier_full]: 补齐真实 Docker-capable `darwin-arm64` summary 导入链路，使 release evidence 至少包含 `linux-x86_64 + darwin-arm64` 两类 runner 输入。
    - [x] WDBP-3.2a (PRD-WORLD_RUNTIME-021/022) [test_tier_required]: 加固 external summary bundle 导入验真，拒绝 `host_platform` 或 `canonical_platform` 与 `darwin-arm64 + linux-x86_64 canonical builder` 目标态不一致的伪装证据。
  - [x] WDBP-3.3 (PRD-WORLD_RUNTIME-022) [test_tier_required]: 在 production runtime / node 主入口绑定 hardened `ReleaseSecurityPolicy`，并把 effective policy 写入 status / acceptance evidence。
- [x] WDBP-4 (PRD-WORLD_RUNTIME-022) [test_tier_required]: 把 `compile_module_artifact_from_source` 的生产路径外移到 external Docker builder 或 production 默认禁用，runtime 只消费 binary + build receipt。

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
- 更新日期: 2026-03-31
- 当前阶段: WDBP-3 跨宿主 evidence 收口中（WDBP-3.1 / WDBP-3.3 已完成，WDBP-4 已完成）
- WDBP-3 剩余设计切片:
  - `WDBP-3.2`: 需要一条真实 Docker-capable `darwin-arm64` summary/evidence 输入，并通过节点侧固定入口生成正式 proof payload / attestation；当前剩余的是 live 证据本身，不是导入、打包或提交工具。
- owner role: `wasm_platform_engineer`
- 联审角色: `producer_system_designer`、`runtime_engineer`
- 验证角色: `qa_engineer`
- 阻塞项:
  - GitHub-hosted `macos-14` runner 当前无 Docker daemon，release evidence / release gate 仍需补齐真实 Linux + Docker-capable macOS full-tier 证据归档。
- 实施备注:
  - `docker/wasm-builder/Dockerfile` 与 `scripts/build-wasm-module.sh` 已落地，当前 canonical build 已收敛为 Docker-only path，不再提供 host-native fallback。
  - `scripts/build-wasm-module.sh`、`scripts/sync-m1-builtin-wasm-artifacts.sh`、`scripts/ci-m1-wasm-summary.sh`、`tools/wasm_build_suite` 与 `docker/wasm-builder/Dockerfile` 现已只读取/写入 `OASIS7_WASM_*` 当前入口，避免 operator 脚本与容器镜像继续扩散旧前缀。
  - runtime `builtin_wasm_materializer`、`m1/m4/m5_builtin_wasm_artifact` 与 `runtime/world/release_manifest` 现已只读取 `OASIS7_BUILTIN_WASM_*` 当前入口，避免构建链路已迁移后 runtime materialize/fetch/fallback 仍停留在旧前缀。
  - `tools/wasm_build_suite` 已新增 `build receipt`、`source_hash`、`build_manifest_hash`、`builder_image_digest` 与 `container_platform` 输出；builtin `m1/m4/m5` hash manifest 已全部改写为单 canonical token `linux-x86_64=<sha256>`。
  - `crates/oasis7_distfs/src/bin/sync_builtin_wasm_identity.rs` 已切换为 receipt 驱动 identity 生成；写路径只输出 canonical token，读路径仍兼容 legacy multi-token manifest。
  - `scripts/ci-m1-wasm-summary.sh` 与 `scripts/ci-verify-m1-wasm-summaries.py` 已区分 `host_platform` 与 `canonical_platform`，并新增 `receipt_evidence + identity_build_recipe` 对账；当前 CI 对账口径改为“不同宿主只比较 Docker canonical 输出与一致的 receipt/build recipe 证据”。
  - runtime `ModuleReleaseSubmitAttestation -> apply` 现已显式绑定 `builder_image_digest + container_platform + canonicalizer_version`；release gate 会拒绝阈值 attestation 间的 receipt evidence 不一致，且要求 attestation 的 `source_hash/build_manifest_hash/wasm_hash` 与 manifest identity 对齐。
  - `ModuleReleaseManifestMappingState` 与节点验收脚本现已补齐 release evidence 摘要：映射状态会落盘 `release_{wasm,source,build_manifest}_hash + builder_image_digest + container_platform + canonicalizer_version + attestation_platforms + proof_cids + receipt_evidence_conflict`，`scripts/module-release-node-acceptance.sh` 也已纳入 receipt mismatch 阻断用例。
  - 新增 `scripts/wasm-release-evidence-report.sh` 作为多 runner fixed entry，可统一收集/校验 `m1/m4/m5` summary 并输出 `summary.md/json`；当前 `.github/workflows/wasm-determinism-gate.yml` 已切换到 `--summary-import-dir` 模式，会把下载下来的 runner summaries 统一收口为可归档 evidence artifact。
  - `scripts/ci-verify-m1-wasm-summaries.py` 与 `scripts/wasm-release-evidence-report.sh` 现已把 `required_runners`（stable gate）与 `expected_runners`（full-tier cross-host evidence）拆开；GitHub-hosted workflow 当前以 `linux-x86_64` 作为 required runner，但 summary/report 会显式输出 `received_runners + missing_runners + cross_host_evidence_pending + gate_result=conditional-go`。
  - `WDBP-3.2` 的导入链路现已落地：`scripts/package-wasm-summary-bundle.sh` 可把外部 Docker-capable runner 的 `m1/m4/m5` summary 打成标准 bundle，`scripts/stage-wasm-summary-imports.sh` 可在 verify 前把 GitHub-hosted Linux summary 与外部 bundle 合并到同一 import dir；`workflow_dispatch` 也新增了 `external_summary_bundle_url` / `external_summary_runner_label` 入口。
  - `WDBP-3.2a` 已继续加固 external bundle 验真：`package/stage/verify/report` 链路现在会强校验 summary/bundle 的 `host_platform` 与 `canonical_platform=linux-x86_64`，并通过 `scripts/wasm-summary-bundle-smoke.sh` 固定覆盖“真实 darwin bundle 可导入、伪装 darwin 的 linux bundle 必须失败”。
  - 仓库内已补 `scripts/dispatch-wasm-determinism-gate.sh` 作为 operator 入口，用于以 `gh workflow run` 触发带外部 bundle URL 的 full-tier evidence run；待真实 `darwin-arm64` bundle 到位后即可直接归档正式 closure artifact。
  - 节点侧 proof 收口已落地：`scripts/module-release-node-attestation-flow.sh` 现可在发布节点本地执行 `summary collect/import -> evidence verify -> canonical proof inputs -> proof payload -> attestation submit`，并刻意剥离 summary/report 中的时间戳与本地路径，避免把非语义字段写入 `proof_cid`。
  - `scripts/module-release-node-acceptance.sh` 现已新增 `required_attestation_flow` smoke，基于合成 `linux-x86_64 + darwin-arm64` summary 验证节点侧固定入口可以稳定生成 `proof_payload.json + submit_request.json`。
  - GitHub-hosted `macos-14` runner 当前不提供 Docker daemon，而 canonical build 已变为 Docker-only path；因此 workflow 已临时收敛为 Linux-only gate，跨宿主对账继续通过导入外部 Docker-capable macOS summary 的方式完成。
  - builtin wasm fallback materializer 现已把临时输出目录收敛到仓库内 `.tmp/`，避免与 Docker-only wrapper 的 workspace-root 约束冲突；同时 canonical builder receipt 默认复用受控 `builder_image_digest`，避免 CI 本地 image id 漂移直接打穿 identity manifest 对账。
  - `compile_module_artifact_from_source` 现已完成 production gate：`ReleaseSecurityPolicy` 新增 `allow_runtime_source_compile`，production 默认关闭该路径并要求改走 external Docker builder + deploy binary；dev/test 保留该 action 以支撑现有回归。
  - `oasis7_chain_runtime` 现已把 `release_default` storage profile 绑定到 hardened `ReleaseSecurityPolicy`，并通过 `/v1/chain/status` 输出 effective policy；`NodeRuntimeExecutionDriver::new_with_storage_profile` 会在装载 execution world 时同步应用该 policy。
  - `scripts/module-release-node-acceptance.sh` 现已新增 `required_release_policy` 步骤，并在 `.tmp/module_release_node_acceptance/20260318-134705/summary.json` 留下 production policy binding/status 证据。
  - `2026-03-31` 已补完 `WDBP-3.3` 的 runtime 侧余量审计：`viewer runtime_live` bootstrap、`governance_registry_import` 新建/加载 world、`reward_runtime_worker` 以及 `execution_bridge::load_execution_world` 的缺档案/旧样本装载路径现也统一切到 hardened `ReleaseSecurityPolicy`，避免 binary-only 语义只停留在 chain runtime 主入口。
  - `2026-03-31` 在当前 `Linux x86_64 + Docker(linux/x86_64)` 工位复核后确认，仓库内已不存在缺失的 `darwin-arm64` 导入/打包/attestation tooling；`WDBP-3.2` 剩余阻塞仅是真实 Docker-capable `darwin-arm64` live summary / proof 输入本身。
