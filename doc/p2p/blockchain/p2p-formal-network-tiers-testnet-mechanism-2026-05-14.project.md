# oasis7 正式网络分层与 testnet 机制（项目管理文档）

- 对应设计文档: `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.design.md`
- 对应需求文档: `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.prd.md`

审计轮次: 1
## 任务拆解（含 PRD-ID 映射）
- [x] formal-network-tiers-testnet-mechanism (PRD-P2P-028) [test_tier_required]: 新建“正式网络分层与 testnet 机制”专题 PRD / design / project，并落 `network_tier_manifest` repo-owned skeleton、smoke 与 `shared_devnet/public_testnet/mainnet` example manifests。 Trace: .pm/tasks/task_7021c28970ef4f40b0367563df7f1e32.yaml

### 后续切片
- `runtime_engineer` / TIER-2:
  - 把 `network_tier_manifest` 接到 runtime/network profile 选择、genesis/bootstrap/ref 校验与启动入口。
- `qa_engineer` + `liveops_community` / TIER-3:
  - 建立 first `public_testnet` rehearsal runbook、public RPC/explorer/faucet/reset evidence 与 exit review gate。
- `producer_system_designer` + `runtime_engineer` / TIER-4:
  - 把 `public_testnet exit review -> mainnet gating` 接入 `MAINNET-1~4`、public claims policy 与正式 no-reset commitment。

## 当前结论
- 当前阶段:
  - 游戏阶段口径: `limited playable technical preview`
  - 安全阶段口径: `crypto-hardened preview`
  - formal network-tier verdict: `specified_skeleton_only`
- 当前完成范围:
  - 已冻结 `local_devnet -> shared_devnet -> public_testnet -> mainnet` 四层模型。
  - 已落地 `network_tier_manifest` 脚本骨架、smoke 与 example manifests。
  - 已明确 `shared_devnet` 仍是 shared release-train，不等于 live public testnet。
- 当前缺口:
  - runtime 还没有正式按 tier manifest 选择网络 profile。
  - 仓库里还没有 live `public_testnet` 的 public RPC/explorer/faucet/reset evidence。
  - `mainnet` 仍停留在 `MAINNET-1~4` readiness planning / partial execution 前阶段。

## 依赖
- `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.prd.md`
- `doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.prd.md`
- `doc/p2p/blockchain/p2p-mainnet-public-claims-policy-2026-03-23.prd.md`
- `testing-manual.md`

## 本轮产物
- `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.prd.md`
- `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.design.md`
- `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.project.md`
- `scripts/network-tier-manifest.sh`
- `scripts/network-tier-manifest-smoke.sh`
- `doc/testing/templates/network-tier-shared-devnet.example.json`
- `doc/testing/templates/network-tier-public-testnet.example.json`
- `doc/testing/templates/network-tier-mainnet.example.json`
- `doc/p2p/prd.md`
- `doc/p2p/project.md`
- `doc/p2p/prd.index.md`
- `testing-manual.md`

## 验收命令（本轮）
- `./scripts/network-tier-manifest-smoke.sh`
- `./scripts/network-tier-manifest.sh validate --manifest doc/testing/templates/network-tier-shared-devnet.example.json`
- `./scripts/network-tier-manifest.sh validate --manifest doc/testing/templates/network-tier-public-testnet.example.json`
- `./scripts/network-tier-manifest.sh validate --manifest doc/testing/templates/network-tier-mainnet.example.json`
- `rg -n "public_testnet|mainnet|shared_devnet|specified_skeleton_only|network_tier_manifest" doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.prd.md doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.design.md doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.project.md doc/p2p/prd.md doc/p2p/project.md doc/p2p/prd.index.md testing-manual.md scripts/network-tier-manifest.sh`
- `./scripts/doc-governance-check.sh`
- `git diff --check`

## 状态
- 当前阶段: completed
- 下一步: 先由 `runtime_engineer` 把 tier manifest 接到 runtime/network profile 入口，再由 `qa_engineer`/`liveops_community` 建 first public testnet rehearsal；在此之前继续明确当前没有 live `public_testnet` / `mainnet`。
- 最近更新: 2026-05-14
