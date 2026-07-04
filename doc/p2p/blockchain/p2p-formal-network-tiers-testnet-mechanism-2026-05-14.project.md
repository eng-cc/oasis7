# oasis7 正式网络分层与 testnet 机制（项目管理文档）

- 对应设计文档: `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.design.md`
- 对应需求文档: `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.prd.md`

审计轮次: 2
## 任务拆解（含 PRD-ID 映射）
- [x] formal-network-tiers-testnet-mechanism (PRD-P2P-028) [test_tier_required]: 新建“正式网络分层与 testnet 机制”专题 PRD / design / project，并在同一专题内补齐 `network_tier_manifest` runtime/launcher 接线、repo-owned validate/smoke/exit-review、example manifests 与 public-testnet rehearsal/exit-review 模板。 Trace: .pm/tasks/task_7021c28970ef4f40b0367563df7f1e32.yaml
- [x] formal-public-testnet-readiness-gate (PRD-P2P-028) [test_tier_required]: 在 formal network tier 机制之上追加 `public_testnet` readiness review follow-up，补齐 repo-owned lane gate、placeholder-safe endpoint 判定、six-lane 模板与 skeleton evidence scaffold，确保只有真实全 pass lane/evidence 才能推进到 `ready_for_live_candidate`。 Trace: .pm/tasks/task_7a279b3f05a34def8d75f86ce2ede4e7.yaml
- [x] formal-public-testnet-live-candidate-checklist (PRD-P2P-028) [test_tier_required]: 为当前 rehearsal/skeleton 状态补一份 repo-owned companion runbook，冻结 `public_testnet` 进入 `ready_for_live_candidate` 前的 six-lane checklist、最小 evidence、canonical 命令、硬阻断条件与 claim boundary，避免“还差什么”只停留在聊天结论。 Trace: .pm/tasks/task_3f0ab6e26c034d42bedcecf38d066fb2.yaml
- [x] formal-public-testnet-claims-boundary-review (PRD-P2P-028) [test_tier_required]: 补齐 repo-owned `qa_engineer` claims boundary review evidence，把 live `public_testnet` 的 allowed/denied claims、guarded faucet 边界与 aggregate readiness 不可越界口径固定成正式 QA verdict，并新增非 template lanes TSV 供后续 readiness review 复用。 Trace: .pm/tasks/task_e74e62daf53a45d0bc24ac2d520bb1b3.yaml
- [x] formal-public-testnet-ecs-evidence-harvest (PRD-P2P-028) [test_tier_required]: 把 candidate manifest / bundle / bootstrap peers 镜像成 repo-owned readiness 输入，并执行一轮 same-window ECS + local freshness audit；若 public endpoint 仍可达但 runtime/config 已漂移，则必须把对应 public lanes 收紧回 `partial/block`，不延续旧 `pass`。 Trace: .pm/tasks/task_49d6af52e31d404eb80999993eb71b98.yaml
- [x] formal-public-testnet-local-observer-contract-sync (PRD-P2P-028) [test_tier_required]: 为 root-owned 本机 `oasis7-testnet-observer.service` 补 repo-owned local contract sync 脚本与 `reset-state` operator 入口，从两台 ECS 的 live two-validator env 推导出本机 observer 应使用的 validator/signer/bootstrap/manifest 合同，并把 live apply 后剩余的 execution drift 继续固化成 repo-owned blocker 证据。 Trace: .pm/tasks/task_dfb4d70a28884617b1506fa6570b34fc.yaml

### 后续切片
- `runtime_engineer` / TIER-2:
  - 已完成：把 `network_tier_manifest` 接到 runtime/network profile 选择、genesis/bootstrap/ref 校验与启动入口，并把 formal tier 暴露到 `/v1/chain/status` 与 launcher passthrough。
  - 已完成 local sync path：新增 `scripts/p2p-public-testnet-local-observer-sync.sh`，并已实际把本机 observer 收口到 two-validator contract、formal manifest 与 repo-owned `start-node.sh`。
  - 已完成 operator reset path：同一脚本已新增 `reset-state`，可 repo-owned 备份并清空 local observer 的 execution world / execution records / distfs replication root / reward execution bridge state。
  - 已完成 runtime drift guard：`oasis7_chain_runtime` 现会在加载 `--network-tier-manifest` 时同步读取 `release_candidate_bundle_ref`，强校验 `runtime_build.sha256` 与当前可执行文件一致；若 bundle/runtime 漂移，启动阶段直接 fail closed，不再把问题拖到后续 replay/gap-sync 才暴露。
  - 已完成 governed bootstrap artifact set：新增 repo-owned `public_testnet` cold-start candidate，明确当前 honest 四节点方案是 `2 validators + 2 observers`，并冻结 concrete genesis validator registry、bootstrap peers、bundle、manifest 与 topology evidence。
- `qa_engineer` + `liveops_community` / TIER-3:
  - 已完成 skeleton：建立 first `public_testnet` rehearsal / exit-review 模板，并补 `network-tier-exit-review.sh` 作为 formal gate 汇总入口。
  - 已完成 readiness gate：新增 `network-tier-public-testnet-readiness.sh`、lane scaffold 与 skeleton evidence placeholder，可把 `public_testnet` 从“只有 manifest skeleton”与“具备候选 lane evidence”区分开。
  - 已完成 claims review：新增 repo-owned `public-testnet-claims-boundary-review-2026-05-21.md` 与实际 lanes TSV，确认当前公开口径可放行到 `public_testnet/resettable/guarded faucet/non-mainnet`，但 aggregate readiness 仍不得越过 rehearsal / governed-bootstrap 证据本身。
  - 已完成 fresh audit：新增 repo-owned live manifest / bundle / bootstrap peers mirror，并用 `public-testnet-ecs-freshness-audit-2026-05-22.md` 同窗复核当前 ECS + local 现网；结果证明 public endpoint 仍在，但 local observer 已脱离 formal manifest 合同，sequencer 也出现 predecessor-gap runtime error，因此不能维持先前更乐观的 lane 判定。
  - 最新补充（2026-07-03 / live-candidate same-world hosted entry lane）：`network-tier-public-testnet-readiness.sh` 现在要求 `same_world_hosted_entry_ready` 作为 promotion required lane；pass evidence 必须是 `oasis7.same_world_hosted_entry.v1` JSON，证明 hosted-login / launcher / viewer / pure API 读取同一个 formal `public_testnet` world state，且未依赖手工 checkpoint/data copy。上方历史任务 row 中的 `six-lane` 保留为 2026-05 初始 scaffold scope 追溯，不代表当前 live-candidate contract；当前 contract 以后续“11 条 required-lane readiness”和 runbook 为准。
- `producer_system_designer` / TIER-3.5:
  - 已完成 checklist：新增 companion runbook，把 live-candidate required-lane owner、最小 evidence、执行顺序、canonical 命令与禁止 claims 冻结成单一入口。
  - 已完成 fresh bootstrap operator runbook：新增 `doc/p2p/blockchain/p2p-public-testnet-governed-bootstrap-2026-06-06.runbook.md`，把 governed bootstrap artifacts 如何真正起 validator pair、何时接两台 observer、哪些失败应视为 bootstrap bug 还是 observer follow-up，固定成单一 operator-facing 路径。
- `producer_system_designer` + `runtime_engineer` / TIER-4:
  - 剩余 live 工作：把 `public_testnet exit review -> mainnet gating` 接入 live `MAINNET-1~4` evidence、public claims policy 执行面与正式 no-reset commitment。

## 当前结论
- 当前阶段:
  - 游戏阶段口径: `limited playable technical preview`
  - 安全阶段口径: `crypto-hardened preview`
  - formal network-tier verdict: `block`
- 当前完成范围:
  - 已冻结 `local_devnet -> public_testnet -> mainnet` 三层 operator/runtime network-tier 模型；这些 tier 是统一持久大世界的运行/验证载体，不作为玩家世界名；`shared_devnet` 只保留 legacy/rehearsal evidence 语义。
  - 已落地 `network_tier_manifest` repo-owned create/validate、smoke、exit review 与 example manifests。
  - `oasis7_chain_runtime`、`oasis7_game_launcher`、`oasis7_web_launcher` 已支持 formal manifest 输入；runtime status 面已暴露 formal tier/status。
  - 已补 `public_testnet` rehearsal、`public_testnet`、`mainnet` 的 genesis/bootstrap example refs，以及 `public_testnet` rehearsal / exit-review 模板。
  - 已补 `public_testnet` readiness review 入口：repo-owned lane scaffold、skeleton evidence placeholder 与 `specified_skeleton_only|partial|block|ready_for_live_candidate` verdict 脚本；这些是判定枚举，不代表当前已 ready。
  - 已补 `public_testnet` live-candidate companion runbook，统一回答“当前还差哪些 lane / evidence / claims review 才能进入 live candidate”。
  - 已建立 `public_testnet` 候选输入侧的 public RPC / explorer / guarded faucet / reset-policy / claims-boundary evidence，其中 `claims_boundary_review` 已有独立 QA verdict；这不等于 live public testnet 已上线。
  - 已把候选 manifest / bundle / bootstrap peers 镜像回 repo-owned evidence，可作为 readiness review 的单一运行输入。
  - 已把 local observer remediation 收口成 repo-owned sync 脚本与 operator evidence，避免继续依赖 `/opt` 手改。
  - 已补 `public_testnet` fresh governed bootstrap artifact set，可作为“四节点 testnet 从 0 重建”的起始真值，而不再依赖旧 live-candidate 恢复链路。
  - 已补当前 formal `public_testnet` 11 条 required-lane packet：`doc/testing/evidence/public-testnet-current-required-lanes-2026-07-03.tsv` 与 `doc/testing/evidence/public-testnet-current-required-lanes-2026-07-03.md`。该 packet 是完整 blocker matrix，不是 live promotion evidence。
  - 已补 fresh public surface sample 与 governed reset-policy evidence：`public_rpc_ready`、`explorer_public_ready`、`reset_policy_announced` 现在可以按当前 evidence 记为 `pass`；`faucet_guard_ready` 因 fresh endpoint connection failed 收紧为 `block`。
  - 已补 `public_testnet` guarded faucet 的 repo-owned recovery package/runbook/evidence：`scripts/public-testnet-faucet/`、`doc/p2p/blockchain/p2p-public-testnet-faucet-operator-runbook-2026-07-04.md` 与 `doc/testing/evidence/public-testnet-faucet-recovery-blocker-2026-07-04.md`；这只补齐恢复路径，`faucet_guard_ready` 仍需 fresh live endpoint、guarded claim 与 cooldown evidence 才能转 `pass`。
  - 已明确 `shared_devnet` 只作为 legacy/rehearsal evidence，不等于目标 test 环境；aggregate readiness 不再要求 `shared_devnet_pass`。
- 当前缺口:
  - formal `public_testnet` 仍不能进入 `ready_for_live_candidate`，因为当前 governed bootstrap manifest 仍是 rehearsal，且 2026-07-03 11 条 required-lane readiness 汇总仍为 `block`；当前 blocker 是 `faucet_guard_ready`、`runtime_bootstrap`、`world_resource_provenance_ready`、`provider_resource_provenance_ready`、`resource_delta_replay_ready`、`api_viewer_projection_ready`、`same_world_hosted_entry_ready`。
  - legacy shared-devnet triad 的历史恢复证据只作 provenance，不能作为当前目标 test 环境或 `public_testnet` readiness gate。
  - mirrored candidate bundle/runtime drift guard、fresh public RPC/explorer evidence、reset-policy/claims evidence 仍然有价值，但在 governed bootstrap 仍为 rehearsal、faucet 当前不可达、runtime/provenance/replay/viewer/hosted-entry lanes 未 pass 前，只能证明“部分公开测试入口与边界控制”，不能把 formal `public_testnet` 提升到 `ready_for_live_candidate`。
  - `mainnet` 仍停留在 `MAINNET-1~4` readiness planning / partial execution 前阶段，仓库当前只有 formal manifest + gate skeleton。

## 依赖
- `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.prd.md`（legacy rehearsal provenance only）
- `doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.prd.md`
- `doc/p2p/blockchain/p2p-mainnet-public-claims-policy-2026-03-23.prd.md`
- `testing-manual.md`

## 本轮产物
- `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.prd.md`
- `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.design.md`
- `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.project.md`
- `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.runbook.md`
- `doc/p2p/blockchain/p2p-public-testnet-governed-bootstrap-2026-06-06.runbook.md`
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
- `doc/testing/templates/network-tier-public-testnet-rehearsal.example.json`
- `doc/testing/templates/network-tier-public-testnet.example.json`
- `doc/testing/templates/network-tier-mainnet.example.json`
- `doc/testing/templates/public-testnet-rehearsal-genesis.example.json`
- `doc/testing/templates/public-testnet-genesis.example.json`
- `doc/testing/templates/mainnet-genesis.example.json`
- `doc/testing/templates/public-testnet-rehearsal-bootstrap.example.txt`
- `doc/testing/templates/public-testnet-bootstrap.example.txt`
- `doc/testing/templates/mainnet-bootstrap.example.txt`
- `doc/testing/templates/public-testnet-rehearsal-template.md`
- `doc/testing/templates/public-testnet-exit-review-template.md`
- `doc/testing/templates/public-testnet-readiness-lanes.example.tsv`
- `doc/testing/templates/public-testnet-skeleton-evidence.example.md`
- `doc/testing/evidence/public-testnet-live-candidate-endpoint-deploy-2026-05-19.md`
- `doc/testing/evidence/p2p-public-testnet-faucet-service-2026-05-19.md`
- `doc/testing/evidence/public-testnet-claims-boundary-review-2026-05-21.md`
- `doc/testing/evidence/public-testnet-live-candidate-lanes-2026-05-21.tsv`
- `doc/testing/evidence/public-testnet-live-candidate-bundle-2026-05-22.json`
- `doc/testing/evidence/public-testnet-live-candidate-bootstrap-peers-2026-05-22.txt`
- `doc/testing/evidence/public-testnet-live-candidate-manifest-2026-05-22.json`
- `doc/testing/evidence/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json`
- `doc/testing/evidence/public-testnet-governed-bootstrap-genesis-2026-06-06.json`
- `doc/testing/evidence/public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.txt`
- `doc/testing/evidence/public-testnet-governed-bootstrap-world-2026-06-06/`
- `doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json`
- `doc/testing/evidence/public-testnet-governed-bootstrap-manifest-2026-06-06.json`
- `doc/testing/evidence/public-testnet-governed-bootstrap-topology-2026-06-06.md`
- `doc/testing/evidence/public-testnet-current-required-lanes-2026-07-03.tsv`
- `doc/testing/evidence/public-testnet-current-required-lanes-2026-07-03.md`
- `doc/testing/evidence/public-testnet-public-surface-freshness-2026-07-03.json`
- `doc/testing/evidence/public-testnet-public-surface-freshness-2026-07-03.md`
  - `doc/testing/evidence/public-testnet-governed-reset-policy-announcement-2026-07-03.md`
  - `doc/testing/evidence/public-testnet-faucet-recovery-blocker-2026-07-04.md`
- `doc/testing/evidence/public-testnet-ecs-freshness-audit-2026-05-22.md`
- `doc/testing/evidence/public-testnet-local-observer-contract-sync-2026-05-22.md`
- `doc/testing/evidence/public-testnet-live-candidate-lanes-2026-05-22.tsv`
  - `scripts/p2p-public-testnet-local-observer-sync.sh`
  - `scripts/public-testnet-faucet/start-public-testnet-faucet.sh`
  - `scripts/public-testnet-faucet/package-public-testnet-faucet.sh`
  - `scripts/public-testnet-faucet/oasis7-public-testnet-faucet.service`
  - `scripts/public-testnet-faucet/public-testnet-faucet.env.example`
  - `doc/p2p/blockchain/p2p-public-testnet-faucet-operator-runbook-2026-07-04.md`
- `doc/p2p/prd.md`
- `doc/p2p/project.md`
- `doc/p2p/prd.index.md`
- `testing-manual.md`
- `.pm/tasks/task_3f0ab6e26c034d42bedcecf38d066fb2.execution.md`
- `.pm/tasks/task_e74e62daf53a45d0bc24ac2d520bb1b3.execution.md`
- `.pm/tasks/task_49d6af52e31d404eb80999993eb71b98.execution.md`

## 验收命令（本轮）
- `./scripts/network-tier-manifest-smoke.sh`
- `./scripts/network-tier-manifest.sh validate --manifest doc/testing/templates/network-tier-public-testnet-rehearsal.example.json`
- `./scripts/network-tier-manifest.sh validate --manifest doc/testing/templates/network-tier-public-testnet.example.json`
- `./scripts/network-tier-manifest.sh validate --manifest doc/testing/templates/network-tier-mainnet.example.json`
- `./scripts/network-tier-exit-review.sh --manifest doc/testing/templates/network-tier-public-testnet.example.json`
- `./scripts/network-tier-exit-review.sh --manifest doc/testing/templates/network-tier-mainnet.example.json`
- `./scripts/network-tier-public-testnet-readiness.sh --manifest doc/testing/templates/network-tier-public-testnet.example.json`
- `./scripts/network-tier-manifest.sh validate --manifest doc/testing/evidence/public-testnet-live-candidate-manifest-2026-05-22.json`
- `./scripts/network-tier-public-testnet-readiness.sh --manifest doc/testing/evidence/public-testnet-live-candidate-manifest-2026-05-22.json --lanes-tsv doc/testing/evidence/public-testnet-live-candidate-lanes-2026-05-22.tsv`
- `./scripts/network-tier-public-testnet-readiness.sh --manifest doc/testing/evidence/public-testnet-governed-bootstrap-manifest-2026-06-06.json --lanes-tsv doc/testing/evidence/public-testnet-current-required-lanes-2026-07-03.tsv`
- `bash -n scripts/p2p-public-testnet-local-observer-sync.sh`
- `bash -n scripts/public-testnet-faucet/start-public-testnet-faucet.sh scripts/public-testnet-faucet/package-public-testnet-faucet.sh`
- `./scripts/public-testnet-faucet/package-public-testnet-faucet.sh --help`
- `./scripts/public-testnet-faucet/package-public-testnet-faucet.sh --profile dev --out-dir .tmp/public-testnet-faucet-package --archive .tmp/public-testnet-faucet-package.tar.gz`
- `./scripts/network-tier-public-testnet-readiness.sh --manifest doc/testing/evidence/public-testnet-governed-bootstrap-manifest-2026-06-06.json --lanes-tsv doc/testing/evidence/public-testnet-current-required-lanes-2026-07-03.tsv`
- `./scripts/p2p-public-testnet-local-observer-sync.sh render --local-env .tmp/p2p_testnet_reality/20260522-100229/nodes/local_node/node.env --sequencer-env .tmp/p2p_testnet_reality/20260522-100229/nodes/sequencer_ecs/node.env --storage-env .tmp/p2p_testnet_reality/20260522-100229/nodes/storage_ecs/node.env --manifest-path /opt/oasis7/p2p-testnet-local/config/network-tier-public-testnet-live-candidate.json`
- `tmpdir="$(mktemp -d)" && mkdir -p "$tmpdir/app/config" "$tmpdir/app/bin" && cp .tmp/p2p_testnet_reality/20260522-100229/nodes/local_node/node.env "$tmpdir/app/config/node.env" && ./scripts/p2p-public-testnet-local-observer-sync.sh apply --local-env "$tmpdir/app/config/node.env" --sequencer-env .tmp/p2p_testnet_reality/20260522-100229/nodes/sequencer_ecs/node.env --storage-env .tmp/p2p_testnet_reality/20260522-100229/nodes/storage_ecs/node.env --manifest-path /opt/oasis7/p2p-testnet-local/config/network-tier-public-testnet-live-candidate.json --manifest-source doc/testing/evidence/public-testnet-live-candidate-manifest-2026-05-22.json --manifest-dest "$tmpdir/app/config/network-tier-public-testnet-live-candidate.json" --start-script-dest "$tmpdir/app/bin/start-node.sh" --backup-dir "$tmpdir/backups"`
- `P2PARCH6_SEQ_SSH_PASSWORD='***' P2PARCH6_STORAGE_SSH_PASSWORD='***' ./scripts/p2p-real-env-triad-snapshot.sh --samples 2 --interval-secs 3 --out-dir .tmp/p2p_testnet_reality --world-id oasis7-public-testnet-parallel-20260518 --local-service oasis7-testnet-observer.service --local-status-url http://127.0.0.1:6633/v1/chain/status --local-health-url http://127.0.0.1:6633/healthz --local-env-file /opt/oasis7/p2p-testnet-local/config/node.env --sequencer-target root@39.104.204.172 --sequencer-service oasis7-testnet-sequencer.service --sequencer-status-url http://127.0.0.1:6631/v1/chain/status --sequencer-health-url http://127.0.0.1:6631/healthz --sequencer-env-file /opt/oasis7/p2p-testnet/config/node.env --storage-target root@39.104.205.67 --storage-service oasis7-testnet-storage.service --storage-status-url http://127.0.0.1:6632/v1/chain/status --storage-health-url http://127.0.0.1:6632/healthz --storage-env-file /opt/oasis7/p2p-testnet/config/node.env`
- `rg -n "ready_for_live_candidate|specified_skeleton_only|required-lane|same_world_hosted_entry_ready|claim boundary" doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.prd.md doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.project.md doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.runbook.md testing-manual.md doc/p2p/prd.md doc/p2p/project.md`
- `rg -n "claims_boundary_review|allowed_claims|denied_claims|ready_for_live_candidate" doc/testing/evidence/public-testnet-claims-boundary-review-2026-05-21.md doc/testing/evidence/public-testnet-live-candidate-endpoint-deploy-2026-05-19.md doc/testing/evidence/p2p-public-testnet-faucet-service-2026-05-19.md doc/testing/evidence/public-testnet-live-candidate-lanes-2026-05-21.tsv doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.runbook.md doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.prd.md`
- `env -u RUSTC_WRAPPER cargo check -p oasis7`
- `env -u RUSTC_WRAPPER cargo test -p oasis7 build_oasis7_chain_runtime_args_prefers_network_tier_manifest_when_present`
- `env -u RUSTC_WRAPPER cargo test -p oasis7 build_chain_runtime_args_uses_network_tier_manifest_when_present`
- `env -u RUSTC_WRAPPER cargo test -p oasis7 status_payload_exposes_loaded_network_tier_manifest`
- `rg -n "public_testnet|mainnet|shared_devnet|specified_skeleton_only|network_tier_manifest" doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.prd.md doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.design.md doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.project.md doc/p2p/prd.md doc/p2p/project.md doc/p2p/prd.index.md testing-manual.md scripts/network-tier-manifest.sh`
- `./scripts/doc-governance-check.sh`
- `git diff --check`

## 状态
- 当前阶段: completed
- 下一步: 当前 11 条 required-lane packet 保持 `4 pass / 0 partial / 7 block`；先用新增 faucet recovery package/runbook 恢复并重新验证 guarded faucet endpoint，再 realign live network-tier manifest surface 的 required gates 到当前 11-lane contract，然后继续补 runtime bootstrap、world/provider provenance、resource-delta replay、API/viewer projection 与 same-world hosted entry pass evidence。shared-devnet triad 运行态已在 2026-05-23 冷重建后恢复健康，但只作为 legacy/rehearsal evidence 追溯，不再作为目标 test 环境或必需 promotion gate。
- 最近更新: 2026-07-04
