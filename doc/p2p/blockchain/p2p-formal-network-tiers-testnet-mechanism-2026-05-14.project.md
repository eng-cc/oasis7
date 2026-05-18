# oasis7 正式网络分层与 testnet 机制（项目管理文档）

- 对应设计文档: `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.design.md`
- 对应需求文档: `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.prd.md`

审计轮次: 1
## 任务拆解（含 PRD-ID 映射）
- [x] formal-network-tiers-testnet-mechanism (PRD-P2P-028) [test_tier_required]: 新建“正式网络分层与 testnet 机制”专题 PRD / design / project，并在同一专题内补齐 `network_tier_manifest` runtime/launcher 接线、repo-owned validate/smoke/exit-review、example manifests 与 public-testnet rehearsal/exit-review 模板。 Trace: .pm/tasks/task_7021c28970ef4f40b0367563df7f1e32.yaml
- [x] formal-public-testnet-readiness-gate (PRD-P2P-028) [test_tier_required]: 在 formal network tier 机制之上追加 `public_testnet` readiness review follow-up，补齐 repo-owned lane gate、placeholder-safe endpoint 判定、seven-lane rehearsal 模板与 skeleton evidence scaffold，确保当前仓库只能把真实 lane/evidence 推进到 `ready_for_live_candidate`。 Trace: .pm/tasks/task_7a279b3f05a34def8d75f86ce2ede4e7.yaml

### 后续切片
- `runtime_engineer` / TIER-2:
  - 已完成：把 `network_tier_manifest` 接到 runtime/network profile 选择、genesis/bootstrap/ref 校验与启动入口，并把 formal tier 暴露到 `/v1/chain/status` 与 launcher passthrough。
- `qa_engineer` + `liveops_community` / TIER-3:
  - 已完成 skeleton：建立 first `public_testnet` rehearsal / exit-review 模板，并补 `network-tier-exit-review.sh` 作为 formal gate 汇总入口。
  - 已完成 readiness gate：新增 `network-tier-public-testnet-readiness.sh`、lane scaffold 与 skeleton evidence placeholder，可把 `public_testnet` 从“只有 manifest skeleton”与“具备 live candidate lane evidence”区分开。
- `producer_system_designer` + `runtime_engineer` / TIER-4:
  - 剩余 live 工作：把 `public_testnet exit review -> mainnet gating` 接入 live `MAINNET-1~4` evidence、public claims policy 执行面与正式 no-reset commitment。

## 当前结论
- 当前阶段:
  - 游戏阶段口径: `limited playable technical preview`
  - 安全阶段口径: `crypto-hardened preview`
  - formal network-tier verdict: `specified_skeleton_only`
- 当前完成范围:
  - 已冻结 `local_devnet -> shared_devnet -> public_testnet -> mainnet` 四层模型。
  - 已落地 `network_tier_manifest` repo-owned create/validate、smoke、exit review 与 example manifests。
  - `oasis7_chain_runtime`、`oasis7_game_launcher`、`oasis7_web_launcher` 已支持 formal manifest 输入；runtime status 面已暴露 formal tier/status。
  - 已补 `shared_devnet/public_testnet/mainnet` 的 genesis/bootstrap example refs，以及 `public_testnet` rehearsal / exit-review 模板。
  - 已补 `public_testnet` readiness review 入口：repo-owned lane scaffold、skeleton evidence placeholder 与 `specified_skeleton_only|partial|block|ready_for_live_candidate` verdict 脚本。
  - 已明确 `shared_devnet` 仍是 shared release-train，不等于 live public testnet。
- 当前缺口:
  - 仓库里还没有 live `public_testnet` 的 public RPC/explorer/faucet/reset evidence。
  - `public_testnet` readiness review 目前仍只能输出 skeleton / placeholder 结论，尚未接入真实 live candidate lane evidence。
  - `mainnet` 仍停留在 `MAINNET-1~4` readiness planning / partial execution 前阶段，仓库当前只有 formal manifest + gate skeleton。

## 依赖
- `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.prd.md`
- `doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.prd.md`
- `doc/p2p/blockchain/p2p-mainnet-public-claims-policy-2026-03-23.prd.md`
- `testing-manual.md`

## 本轮产物
- `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.prd.md`
- `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.design.md`
- `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.project.md`
- `crates/oasis7/src/network_tier_manifest.rs`
- `crates/oasis7/src/bin/oasis7_chain_runtime.rs`
- `crates/oasis7/src/bin/oasis7_chain_runtime/cli.rs`
- `crates/oasis7/src/bin/oasis7_chain_runtime/status_payload.rs`
- `crates/oasis7/src/bin/oasis7_chain_runtime/status_server_support.rs`
- `crates/oasis7/src/bin/oasis7_game_launcher.rs`
- `crates/oasis7/src/bin/oasis7_game_launcher/cli.rs`
- `crates/oasis7/src/bin/oasis7_web_launcher.rs`
- `crates/oasis7/src/bin/oasis7_web_launcher/control_plane.rs`
- `scripts/network-tier-manifest.sh`
- `scripts/network-tier-manifest-smoke.sh`
- `scripts/network-tier-exit-review.sh`
- `scripts/network-tier-public-testnet-readiness.sh`
- `.pm/tasks/task_7a279b3f05a34def8d75f86ce2ede4e7.execution.md`
- `doc/testing/templates/network-tier-shared-devnet.example.json`
- `doc/testing/templates/network-tier-public-testnet.example.json`
- `doc/testing/templates/network-tier-mainnet.example.json`
- `doc/testing/templates/shared-devnet-genesis.example.json`
- `doc/testing/templates/public-testnet-genesis.example.json`
- `doc/testing/templates/mainnet-genesis.example.json`
- `doc/testing/templates/shared-devnet-bootstrap.example.txt`
- `doc/testing/templates/public-testnet-bootstrap.example.txt`
- `doc/testing/templates/mainnet-bootstrap.example.txt`
- `doc/testing/templates/public-testnet-rehearsal-template.md`
- `doc/testing/templates/public-testnet-exit-review-template.md`
- `doc/testing/templates/public-testnet-readiness-lanes.example.tsv`
- `doc/testing/evidence/public-testnet-skeleton-example.md`
- `doc/p2p/prd.md`
- `doc/p2p/project.md`
- `doc/p2p/prd.index.md`
- `testing-manual.md`

## 验收命令（本轮）
- `./scripts/network-tier-manifest-smoke.sh`
- `./scripts/network-tier-manifest.sh validate --manifest doc/testing/templates/network-tier-shared-devnet.example.json`
- `./scripts/network-tier-manifest.sh validate --manifest doc/testing/templates/network-tier-public-testnet.example.json`
- `./scripts/network-tier-manifest.sh validate --manifest doc/testing/templates/network-tier-mainnet.example.json`
- `./scripts/network-tier-exit-review.sh --manifest doc/testing/templates/network-tier-public-testnet.example.json`
- `./scripts/network-tier-exit-review.sh --manifest doc/testing/templates/network-tier-mainnet.example.json`
- `./scripts/network-tier-public-testnet-readiness.sh --manifest doc/testing/templates/network-tier-public-testnet.example.json`
- `env -u RUSTC_WRAPPER cargo check -p oasis7`
- `env -u RUSTC_WRAPPER cargo test -p oasis7 build_oasis7_chain_runtime_args_prefers_network_tier_manifest_when_present`
- `env -u RUSTC_WRAPPER cargo test -p oasis7 build_chain_runtime_args_uses_network_tier_manifest_when_present`
- `env -u RUSTC_WRAPPER cargo test -p oasis7 status_payload_exposes_loaded_network_tier_manifest`
- `rg -n "public_testnet|mainnet|shared_devnet|specified_skeleton_only|network_tier_manifest" doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.prd.md doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.design.md doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.project.md doc/p2p/prd.md doc/p2p/project.md doc/p2p/prd.index.md testing-manual.md scripts/network-tier-manifest.sh`
- `./scripts/doc-governance-check.sh`
- `git diff --check`

## 状态
- 当前阶段: completed
- 下一步: 基础 formal mechanism 与 `public_testnet` readiness gate 已补齐；后续只在建立 live `public_testnet` lane evidence 与 `MAINNET-1~4` 实证时再推进，不得把当前 skeleton/runtime 接线误报为 live `public_testnet` / `mainnet`。
- 最近更新: 2026-05-14
