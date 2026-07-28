# oasis7 Runtime：线上模块发布合法性闭环补齐（项目管理文档）

- 对应设计文档: `doc/world-runtime/module/online-module-release-legality-closure-2026-03-08.design.md`
- 对应需求文档: `doc/world-runtime/module/online-module-release-legality-closure-2026-03-08.prd.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] TASK-WORLD_RUNTIME-016 (PRD-WORLD_RUNTIME-016/017/018) [test_tier_required]: 建立专题 PRD/项目管理文档，冻结目标态边界与验收口径。
- [x] TASK-WORLD_RUNTIME-017 (PRD-WORLD_RUNTIME-016) [test_tier_required]: 引入线上发布清单主真源与节点加载策略切换（生产禁用本地未授权 fallback）。
- [x] TASK-WORLD_RUNTIME-018 (PRD-WORLD_RUNTIME-016) [test_tier_required]: 将 `m1/m4/m5` builtin artifact 加载从仓库内置清单迁移到可治理清单入口，保留受控 bootstrap 兜底。
- [x] TASK-WORLD_RUNTIME-019 (PRD-WORLD_RUNTIME-016) [test_tier_full]: 补齐线上 manifest 不可达/回滚/版本漂移场景回归与故障签名。
- [x] TASK-WORLD_RUNTIME-020 (PRD-WORLD_RUNTIME-017) [test_tier_required]: 生产策略下禁用 `identity_hash_v1` 回退，强制 artifact ed25519 签名校验。
- [x] TASK-WORLD_RUNTIME-021 (PRD-WORLD_RUNTIME-017) [test_tier_required + test_tier_full]: `apply_proposal` 去本地自签路径，改为外部 finality 证书必需并补齐“epoch 快照验证者签名集阈值最终性”与轮换回归。
- [x] TASK-WORLD_RUNTIME-022 (PRD-WORLD_RUNTIME-018) [test_tier_required]: 新增去中心化发布提案与复构建证明收集流程（`proposal -> attestation`）并形成可审计证据结构。
- [x] TASK-WORLD_RUNTIME-023 (PRD-WORLD_RUNTIME-018) [test_tier_required]: 落地“epoch 快照验证者签名集”阈值签名聚合与 release manifest 激活路径（不依赖 CI 服务）并补齐拒绝路径测试。
- [x] TASK-WORLD_RUNTIME-024 (PRD-WORLD_RUNTIME-018) [test_tier_required]: 更新发布运行手册与告警策略（证明冲突、阈值不足、manifest 不可达），并明确 CI 仅用于开发回归且不承担生产发布写入。
- [x] TASK-WORLD_RUNTIME-025 (PRD-WORLD_RUNTIME-017) [test_tier_required + test_tier_full]: 扩展 finality 证书/信任根数据模型，落地 `epoch_id + validator_set_hash + stake_root + threshold_bps + min_unique_signers` 校验与回归。
- [x] TASK-WORLD_RUNTIME-026 (PRD-WORLD_RUNTIME-017) [test_tier_required]: 梳理安装/升级/回滚/发布应用调用点，生产路径禁止本地自签 `apply_proposal()`，统一切换外部证书 apply。
- [x] TASK-WORLD_RUNTIME-027 (PRD-WORLD_RUNTIME-016) [test_tier_required]: 为现有 `ModuleRelease*` 状态机补齐 `release manifest` 映射关系与回放一致性测试，保障迁移期兼容。
- [x] TASK-WORLD_RUNTIME-028 (PRD-WORLD_RUNTIME-018) [test_tier_required]: 从主 CI 移除生产发布写入/激活职责，仅保留 `--check` 类回归；补齐节点侧发布运行手册与验收脚本。
- [x] TASK-WORLD_RUNTIME-029 (PRD-WORLD_RUNTIME-018) [test_tier_required + test_tier_full]: 增加 `stake/epoch` 验签耗时与“2 epoch 收敛”固定基准入口，产出可归档性能与收敛报告。
- [x] TASK-WORLD_RUNTIME-030 (PRD-WORLD_RUNTIME-018) [test_tier_required]: 为 `ModuleReleaseSubmitAttestation` 增加 node-side submit API、proof payload 打包脚本与节点侧固定入口，使 `proof_cid` 对应可归档正式证据，而不是停留在 CI artifact。

## 依赖
- `doc/world-runtime/module/online-module-release-legality-closure-2026-03-08.prd.md`
- `doc/world-runtime/module/player-published-entities.prd.md`
- `doc/p2p/consensus/builtin-wasm-identity-consensus.prd.md`
- `doc/p2p/prd.md`（PoS 与 committed execution 当前合同）
- `testing-manual.md`

## 状态
- 更新日期: 2026-03-18
- 当前状态: complete（`TASK-WORLD_RUNTIME-030` 已完成；后续真实 `darwin-arm64` 跨宿主证据归档转由 `WDBP-3.2` 承接）
- 下一任务: `none`（后续 live 证据闭环见 `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.project.md`）
- 实施备注:
  - `TASK-WORLD_RUNTIME-019` 已完成：新增故障签名 `builtin_release_manifest_unreachable`、`builtin_release_manifest_missing_or_rolled_back`、`builtin_release_manifest_identity_drift`，并补齐 `test_tier_full` 回归。
  - `TASK-WORLD_RUNTIME-021` 已完成：finality 校验新增按 epoch 快照阈值与 signer 集校验，拒绝快照外 signer，并补齐轮换回归（旧 signer 拒绝、新 signer 通过）。
  - `TASK-WORLD_RUNTIME-022` 已完成：新增 `ModuleReleaseSubmitAttestation`/`ModuleReleaseAttested` 事件链路，落地 `signer_node_id + platform` 去重的证明状态存储与 `proof_cid` 审计字段，并补齐冲突拒绝/证据落盘回归。
  - `TASK-WORLD_RUNTIME-023` 已完成：`ModuleReleaseApply` 新增 epoch 快照 signer 阈值聚合门禁，仅统计快照内 signer 的 attestation；阈值不足/快照外 signer 仅提交均会拒绝激活，并补齐对应拒绝回归。
  - `TASK-WORLD_RUNTIME-024` 已完成：`testing-manual.md` 新增 `S11` 去中心化发布运行与告警章节，固化“证明冲突/阈值不足/manifest 不可达（含回滚与 identity 漂移）”分诊口径，并明确主 CI 仅保留 `--check` 回归与对账职责。
  - `TASK-WORLD_RUNTIME-025` 已完成：finality 证书与 epoch 快照新增 `epoch_id + validator_set_hash + stake_root + threshold_bps + min_unique_signers` 模型；`apply_proposal_with_finality` 强校验上述字段并引入 stake-bps 阈值判定，补齐 required/full 回归（含 `validator_set_hash`、`stake_root` 错配拒绝）。
  - `TASK-WORLD_RUNTIME-026` 已完成：新增 `Install/Upgrade/Rollback/ModuleReleaseApply` 的 `*WithFinality` 动作变体并统一接入外部证书 apply helper；生产策略下旧动作路径继续保留兼容但会被“local finality path is disabled”拒绝，补齐 required 回归覆盖“无证书拒绝/带证书通过”。
  - `TASK-WORLD_RUNTIME-028` 已完成：新增节点侧固定验收脚本 `scripts/module-release-node-acceptance.sh`（required + 可选 full + triage 信号检索）并在 `testing-manual.md` 的 S11 固化入口与证据路径；`scripts/sync-m1-builtin-wasm-artifacts.sh` 非 `--check` 写入改为“CI 禁止 + 本地显式授权（`OASIS7_WASM_SYNC_WRITE_ALLOW=local-dev`）”，确保主 CI 不参与生产发布写入/激活。
  - `TASK-WORLD_RUNTIME-029` 已完成：新增 finality 固定基准脚本 `scripts/oasis7-runtime-finality-baseline.sh`，固定执行 `stake_root/epoch_snapshot` 验签与 `2 epoch` 收敛回归，输出 `summary.md + summary.json`；`testing-manual.md` 的 S11 新增该脚本入口与归档口径。
  - `TASK-WORLD_RUNTIME-030` 已完成：`oasis7_chain_runtime` 已补 node-side `ModuleReleaseSubmitAttestation` submit API，新增 `scripts/package-module-release-attestation-proof.sh` / `scripts/submit-module-release-attestation.sh` 与 `scripts/module-release-node-attestation-flow.sh` 固定入口；proof 流水现会先把 release evidence canonicalize 成稳定 proof inputs，再生成 `proof_payload.json + submit_request.json` 并可由发布节点直接提交，不再要求 CI/workflow 代为承载正式证明。
