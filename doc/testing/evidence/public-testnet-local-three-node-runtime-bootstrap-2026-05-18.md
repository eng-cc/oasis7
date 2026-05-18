# Public Testnet Local Three-Node Runtime Bootstrap (2026-05-18)

审计轮次: 1

## Meta
- 责任角色:
  - `runtime_engineer`
- 对应 tier:
  - `public_testnet`
- 当前结论:
  - `runtime_bootstrap=pass`
  - `overall_readiness!=pass`
- world:
  - `oasis7-public-testnet-local-triad`
- manifest:
  - `output/public-testnet-local-triad/network-tier-public-testnet-local-triad.json`
- candidate bundle:
  - `output/public-testnet-local-triad/public-testnet-local-triad.json`
- raw status evidence:
  - `output/public-testnet-local-triad/evidence/pt-sequencer.status.json`
  - `output/public-testnet-local-triad/evidence/pt-storage.status.json`
  - `output/public-testnet-local-triad/evidence/pt-observer.status.json`

## 本轮目标
1. 用 repo-owned `network_tier_manifest` 起一个本地 `public_testnet` rehearsal manifest。
2. 实际部署 `3` 个链节点，并让每个节点都带 `--network-tier-manifest` 启动。
3. 确认 `/v1/chain/status` 是否暴露 formal `network_tier` 字段，以及 `bootstrap_peer_ref` 是否被 runtime 正确加载。

## 执行命令
1. 构建 runtime：
  - `env -u RUSTC_WRAPPER cargo build -p oasis7 --bin oasis7_chain_runtime`
2. 生成本地 rehearsal candidate bundle / manifest：
  - `./scripts/release-candidate-bundle.sh create --bundle output/public-testnet-local-triad/public-testnet-local-triad.json --candidate-id public-testnet-local-triad-20260518 --track public_testnet --runtime-build-ref target/debug/oasis7_chain_runtime --world-snapshot-ref output/public-testnet-local-triad/world-snapshot-placeholder --governance-manifest-ref output/public-testnet-local-triad/governance-manifest-placeholder.json --note 'local-only three-node public_testnet rehearsal candidate' --allow-dirty-worktree`
  - `./scripts/network-tier-manifest.sh create --manifest output/public-testnet-local-triad/network-tier-public-testnet-local-triad.json ...`
3. 部署三节点：
  - `target/debug/oasis7_chain_runtime --node-id pt-sequencer ... --status-bind 127.0.0.1:17631 --node-gossip-bind 127.0.0.1:17611 --replication-network-listen /ip4/127.0.0.1/tcp/17651 --network-tier-manifest output/public-testnet-local-triad/network-tier-public-testnet-local-triad.json`
  - `target/debug/oasis7_chain_runtime --node-id pt-storage ... --status-bind 127.0.0.1:17632 --node-gossip-bind 127.0.0.1:17612 --replication-network-listen /ip4/127.0.0.1/tcp/17652 --network-tier-manifest output/public-testnet-local-triad/network-tier-public-testnet-local-triad.json`
  - `target/debug/oasis7_chain_runtime --node-id pt-observer ... --status-bind 127.0.0.1:17633 --node-gossip-bind 127.0.0.1:17613 --replication-network-listen /ip4/127.0.0.1/tcp/17653 --network-tier-manifest output/public-testnet-local-triad/network-tier-public-testnet-local-triad.json`
4. 采样 live status / healthz：
  - `curl -fsS http://127.0.0.1:17631/v1/chain/status`
  - `curl -fsS http://127.0.0.1:17632/v1/chain/status`
  - `curl -fsS http://127.0.0.1:17633/v1/chain/status`
  - `curl -fsS http://127.0.0.1:17631/healthz`
  - `curl -fsS http://127.0.0.1:17632/healthz`
  - `curl -fsS http://127.0.0.1:17633/healthz`

## Live 结果
### `pt-sequencer`
- `running=true`
- `last_error=null`
- `network_tier.tier=public_testnet`
- `network_tier.status=rehearsal`
- `network_tier.bootstrap_peer_count=3`
- `network_tier.chain_id=oasis7-public-testnet-local-triad`

### `pt-storage`
- `running=true`
- `last_error=null`
- `network_tier.tier=public_testnet`
- `network_tier.status=rehearsal`
- `network_tier.bootstrap_peer_count=3`
- `network_tier.chain_id=oasis7-public-testnet-local-triad`

### `pt-observer`
- `running=true`
- `last_error=null`
- `network_tier.tier=public_testnet`
- `network_tier.status=rehearsal`
- `network_tier.bootstrap_peer_count=3`
- `network_tier.chain_id=oasis7-public-testnet-local-triad`

## 可以确认的事项
1. `oasis7_chain_runtime --network-tier-manifest <path>` 已能实际加载 formal `public_testnet` manifest，而不是只停留在单元测试或 smoke scaffold。
2. `bootstrap_peer_ref` 已被 runtime 读取，三端 `/v1/chain/status` 都暴露 `bootstrap_peer_count=3`。
3. `network_tier.{tier,status,network_id,chain_id,token_symbol,faucet_mode,reset_policy,value_semantics}` 已进入 live status contract。
4. 因此对这次本地 rehearsal 而言，`runtime_bootstrap` 可记为 `pass`。

## 不能外推成 pass 的事项
1. `shared_devnet_pass` 仍然不是这次部署能替代的前置条件。
2. `public_rpc_ready` 不能记 `pass`，因为 manifest `rpc_ref` 仍是 `127.0.0.1`，不是 public endpoint。
3. `explorer_public_ready` 不能记 `pass`，因为本轮没有真实 explorer，只是 placeholder URL。
4. `faucet_guard_ready` 不能记 `pass`，因为本轮没有 guarded faucet 与 abuse control 证据。
5. `reset_policy_announced` 不能记 `pass`，因为本轮只是 local rehearsal，没有正式对外 reset policy 公告。
6. `claims_boundary_review` 不能记 `pass`，因为本轮没有 producer/QA/liveops 的正式 claims review 记录。

## 结论
1. 这次本地三节点部署已经足够把 `runtime_bootstrap` 从模板级假设推进到 live evidence。
2. 这次部署不能把 `public_testnet` overall readiness 升到 `pass`，因为 public-facing lanes 仍缺真实 public / governance / liveops 证据。
3. 因而本轮最准确的收口是：`runtime_bootstrap=pass`，其余六条 readiness lane 仍然不能确认 `pass`。
