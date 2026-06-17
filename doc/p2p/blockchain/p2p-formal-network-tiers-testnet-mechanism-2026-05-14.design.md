# oasis7 正式网络分层与 testnet 机制（设计文档）

- 对应需求文档: `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.prd.md`
- 对应项目管理文档: `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.project.md`

审计轮次: 1
## 设计目标
- 把本地、测试、正式三套目标网络层级收成清晰模型；历史 `shared_devnet` 只保留 legacy/rehearsal evidence 语义，不再兼任 testnet。
- 提供一个 repo-owned、机器可读的 `network_tier_manifest` skeleton，后续 runtime / liveops / QA 都可以围绕同一字段集接线。
- 明确这轮只做 spec + skeleton，不做 live `public_testnet` / `mainnet` 激活。

## 分层模型
Network tier 是统一持久大世界的运行/验证载体分层，不是玩家可见的多个世界模型。

| Tier | 目标 | 可见性 | 价值语义 | reset/faucet | 当前状态 |
| --- | --- | --- | --- | --- | --- |
| `local_devnet` | 本地开发 / 单人验证 | `local_only` | `preview` | 可重置 / 无正式 faucet | 现有开发态 |
| `public_testnet` | 对外公开 rehearsal / validator 候选演练 | `public` | `testnet` | 可重置 / guarded faucet | 已有 rehearsal / governed-bootstrap 证据，尚非 live candidate |
| `mainnet` | 正式价值网络 | `public` | `production` | 不可重置 / 无 faucet | 本轮只建 skeleton |

`shared_devnet` 不再作为目标 tier；旧文档和证据中出现时，只按 legacy 共享开发网络 / rehearsal evidence 追溯。

## Manifest Schema
- 顶层字段:
  - `schema_version`
  - `tier`
  - `status`
  - `network_id`
  - `chain_id`
- `runtime_refs`:
  - `release_candidate_bundle_ref`
  - `genesis_ref`
  - `bootstrap_peer_ref`
- `endpoint_policy`:
  - `rpc_ref`
  - `explorer_ref`
  - `faucet_ref`
- `validator_policy`:
  - `governance_mode`
  - `validator_admission`
  - `target_validator_count`
  - `allow_observer_nodes`
- `token_policy`:
  - `symbol`
  - `faucet_mode`
  - `reset_policy`
  - `value_semantics`
- `claims_policy`:
  - `allowed_claims`
  - `denied_claims`
- `promotion_policy`:
  - `promote_from`
  - `required_gates`
- `evidence_refs`

## 关键规则
- 规则-1: `shared_devnet` 是 legacy/rehearsal evidence，不是目标 test 环境，也不等于 `public_testnet`。
- 规则-2: `public_testnet` 必须显式具备 public RPC、explorer、guarded faucet 与 reset policy。
- 规则-3: `mainnet` 必须显式具备 `faucet_mode=none`、`reset_policy=frozen`、`value_semantics=production`，并把 `MAINNET-1~4` 固定到 `required_gates`。
- 规则-4: 这轮新增的 manifest validate 既检查字段存在性，也检查 tier 语义组合，避免“字段都齐了但还是 testnet/mainnet 混写”。
- 规则-5: `public_testnet` 且 `validator_policy.allow_observer_nodes=true` 时，observer/read-only fetch 准入采用开放签名策略：任意具备有效 requester 签名的节点可以读取 commit/blob 并完成 observer 同步，不需要手动刷新 validator 的 fetch requester allowlist。
- 规则-6: validator/remote writer 准入仍由 validator set、governance registry 或显式 writer allowlist 控制；observer 自动入网不得授予 gossip write、共识签名或 validator 身份。

## 与现有专题的关系
- `p2p-shared-network-release-train-minimum-2026-03-24`：
  - 负责 `shared_devnet/staging/canary` 内部 shared release-train。
  - 现在只作为 legacy/rehearsal evidence 追溯，不作为 `public_testnet` 的必需 promotion gate。
- `p2p-mainnet-grade-readiness-hardening-2026-03-23`：
  - 负责 `MAINNET-1~4` 的安全/治理/创世 readiness gates。
  - 现在被定义为 `mainnet manifest` 的 required gates 来源。
- `p2p-mainnet-public-claims-policy-2026-03-23`：
  - 继续负责 overall public claims deny/allowlist。
  - 本专题只把 tier 级 claims boundary 接到 manifest schema。

## Repo-Owned Skeleton
- `scripts/network-tier-manifest.sh`
  - `create`: 生成一份 `network_tier_manifest`。
  - `validate`: 校验字段完整性与 tier 语义组合。
- `scripts/network-tier-manifest-smoke.sh`
  - 验证 create/validate 主路径。
  - 验证目标 example manifests；public-testnet rehearsal 使用独立 canonical template。
- Example manifests:
  - `doc/testing/templates/network-tier-public-testnet-rehearsal.example.json`
  - `doc/testing/templates/network-tier-public-testnet.example.json`
  - `doc/testing/templates/network-tier-mainnet.example.json`

## 被否决方案
- 否决-1: 直接把现有 `shared_devnet` 改名为 `testnet`。
  - 原因: 这会把内部 shared access / partial rehearsal 与 public availability 混在一起。
- 否决-2: 先做 live public testnet，再考虑 manifest。
  - 原因: 没有统一 manifest，后续 runtime / QA / liveops 无法共享同一套 tier 真值。
- 否决-3: `mainnet` 继续沿用 testnet schema，不额外加语义校验。
  - 原因: 正式价值网络必须把 `no faucet / frozen reset / mainnet gates` 变成机器可判定约束。
