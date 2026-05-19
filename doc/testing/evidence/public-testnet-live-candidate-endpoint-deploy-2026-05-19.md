# Public Testnet Live-Candidate Endpoint Deploy (2026-05-19)

审计轮次: 2

## Meta
- 责任角色:
  - `runtime_engineer`
- 当前结论:
  - `runtime_bootstrap=pass`
  - `public_rpc_ready=partial`
  - `explorer_public_ready=partial`
  - `overall_readiness=partial`
- worktree:
  - `task/p2p-public-testnet-live-candidate-endpoint-deploy`
- 对应 world:
  - `oasis7-public-testnet-parallel-20260518`
- 远端节点:
  - `oasis7-testnet-sequencer.service` on `39.104.204.172`
  - `oasis7-testnet-storage.service` on `39.104.205.67`
  - `oasis7-testnet-observer.service` on local host

## 本轮目标
1. 基于已有两台阿里云机器，确认现有 `oasis7-testnet-*` 三节点是否已经具备继续推进 `public_testnet` lane 的基础。
2. 把远端 testnet 节点补到 formal `public_testnet` manifest 语义，而不是只跑裸 runtime。
3. 明确哪些 lane 能因为这次部署推进，哪些仍然不能。

## Repo 侧变更
1. `scripts/p2p-triad-node-start.sh`
  - 新增 `NETWORK_TIER_MANIFEST_PATH` 环境变量支持。
  - dry-run 已确认会透传 `--network-tier-manifest <path>` 到 `oasis7_chain_runtime`。
2. 未保留任何 `t2t.oasis7.tech` 的 testnet 路由改动。
  - 该域名不是 testnet 专用域名，本轮已把误用收回，不再把它当作 testnet endpoint 结论的一部分。

## 远端部署动作
1. 下发 public-testnet live-candidate bundle / manifest / bootstrap / genesis 到两台 ECS：
  - `/opt/oasis7/p2p-testnet/config/public-testnet-live-candidate.bundle.json`
  - `/opt/oasis7/p2p-testnet/config/network-tier-public-testnet-live-candidate.json`
  - `/opt/oasis7/p2p-testnet/config/bootstrap-peers.txt`
  - `/opt/oasis7/p2p-testnet/config/public-testnet-genesis.example.json`
2. 用 repo-owned `scripts/p2p-triad-node-start.sh` 覆盖两台 ECS 的 `/opt/oasis7/p2p-testnet/bin/start-node.sh`。
3. 在两台 ECS 的 `config/node.env` 中写入：
  - `NETWORK_TIER_MANIFEST_PATH=/opt/oasis7/p2p-testnet/config/network-tier-public-testnet-live-candidate.json`
4. 纠正端点口径：
  - 初始尝试曾错误把非 testnet 专用域名写进 manifest endpoint refs。
  - 在收到“`t2t` 不能用于 testnet”的更正后，本轮已把 manifest endpoint refs 收回为未分配 testnet 公网入口的 placeholder，不再把 `t2t` 作为 testnet 证据。
5. 重启：
  - `systemctl restart oasis7-testnet-sequencer.service`
  - `systemctl restart oasis7-testnet-storage.service`

## 远端节点结果
### ECS sequencer `39.104.204.172`
- `systemctl is-active oasis7-testnet-sequencer.service`:
  - `active`
- `/v1/chain/status` 关键字段:
  - `world_id=oasis7-public-testnet-parallel-20260518`
  - `role=sequencer`
  - `last_error=null`
  - `network_tier.tier=public_testnet`
  - `network_tier.status=rehearsal`
  - `network_tier.bootstrap_peer_count=3`
  - `network_tier.rpc_ref=https://public-testnet.example.invalid/rpc`
  - `network_tier.explorer_ref=https://public-testnet.example.invalid/explorer`
  - `committed_height` 与 `network_committed_height` 继续推进

### ECS storage `39.104.205.67`
- `systemctl is-active oasis7-testnet-storage.service`:
  - `active`
- `/v1/chain/status` 关键字段:
  - `world_id=oasis7-public-testnet-parallel-20260518`
  - `role=sequencer`
  - `last_error=null`
  - `network_tier.tier=public_testnet`
  - `network_tier.status=rehearsal`
  - `network_tier.bootstrap_peer_count=3`
  - `network_tier.rpc_ref=https://public-testnet.example.invalid/rpc`
  - `network_tier.explorer_ref=https://public-testnet.example.invalid/explorer`
  - `committed_height` 与 `network_committed_height` 继续推进

## 边界更正
1. `t2t.oasis7.tech` 不是 testnet 专用域名。
2. 因此本轮不能把任何基于 `t2t` 的 nginx 代理或 TLS 观测当作 `public_rpc_ready` / `explorer_public_ready` 的有效证据。
3. 这次部署真正新增的，只是“远端 live runtime 已实际加载 formal `public_testnet` manifest”，而不是“testnet 公网入口已经就绪”。

## Readiness 收口
1. `runtime_bootstrap`
  - `pass`
  - 依据:
    - 2026-05-18 的本地三节点 formal manifest rehearsal 已确认该 lane 可 `pass`
    - 本轮又补充确认两台 ECS 的 live runtime 已实际加载 formal `public_testnet` manifest
2. `public_rpc_ready`
  - `partial`
  - 依据:
    - 这次部署证明远端 runtime 已经有能力挂 formal endpoint policy
    - 但当前没有正确的 testnet 专用公网入口，因此不能记 `pass`
3. `explorer_public_ready`
  - `partial`
  - 依据:
    - runtime explorer API 本身存在，且可继续作为后续 testnet 专用入口的上游
    - 但当前没有正确的 testnet 专用公网入口，因此不能记 `pass`
4. `faucet_guard_ready`
  - `partial`
  - 依据:
    - 本轮没有真实 guarded faucet 与 abuse control 验证
5. `reset_policy_announced`
  - `partial`
  - 依据:
    - manifest 已声明 `reset_policy=resettable`
    - 但本轮没有正式对外 reset policy 公告
6. `claims_boundary_review`
  - `partial`
  - 依据:
    - 本轮只有 runtime deploy 证据，没有 producer / QA / liveops 正式 claims review
7. `shared_devnet_pass`
  - `partial`
  - 依据:
    - 仍沿用既有 repo 证据状态，没有被这次部署替代

## 结论
1. 这次“进一步做部署”已经把两台阿里云上的 testnet 节点推进成了实际加载 formal `public_testnet` manifest 的 live runtime。
2. 但由于没有 testnet 专用公网域名，本轮不能把 `public_rpc_ready` 或 `explorer_public_ready` 推进成 `pass`。
3. 因而截至本轮，能确认新增的是“formal manifest live load”这层事实；不能确认新增的是“公网入口已经合规可用”。
