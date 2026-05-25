# Public Testnet Live-Candidate Endpoint Deploy (2026-05-19)

审计轮次: 3

## Meta
- 责任角色:
  - `runtime_engineer`
- 当前结论:
  - `runtime_bootstrap=pass`
  - `public_rpc_ready=pass`
  - `explorer_public_ready=pass`
  - `overall_readiness=block`
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
5. 改用 `IP:port` 公网入口：
  - ECS sequencer `STATUS_BIND` 从 `127.0.0.1:6631` 改为 `0.0.0.0:6631`
  - ECS storage `STATUS_BIND` 从 `127.0.0.1:6632` 改为 `0.0.0.0:6632`
  - formal manifest endpoint refs 改为：
    - `rpc_ref=http://39.104.204.172:6631/v1/chain/status`
    - `explorer_ref=http://39.104.205.67:6632/v1/chain/explorer/overview`
6. 重启：
  - `systemctl restart oasis7-testnet-sequencer.service`
  - `systemctl restart oasis7-testnet-storage.service`

## 远端节点结果
### ECS sequencer `39.104.204.172`
- `systemctl is-active oasis7-testnet-sequencer.service`:
  - `active`
- `/v1/chain/status` 关键字段:
  - `world_id=oasis7-public-testnet-parallel-20260518`
  - `role=storage`
  - `last_error=null`
  - `network_tier.tier=public_testnet`
  - `network_tier.status=rehearsal`
  - `network_tier.bootstrap_peer_count=3`
  - `network_tier.rpc_ref=http://39.104.204.172:6631/v1/chain/status`
  - `network_tier.explorer_ref=http://39.104.205.67:6632/v1/chain/explorer/overview`
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
  - `network_tier.rpc_ref=http://39.104.204.172:6631/v1/chain/status`
  - `network_tier.explorer_ref=http://39.104.205.67:6632/v1/chain/explorer/overview`
  - `committed_height` 与 `network_committed_height` 继续推进

## 公网验证
1. 从当前控制机直接访问 RPC：
  - `curl http://39.104.204.172:6631/v1/chain/status`
  - 返回 `ok=true`
  - 返回 `node_id=triad-testnet-sequencer`
  - 返回 `last_error=null`
  - 返回 `network_tier.tier=public_testnet`
2. 从当前控制机直接访问第二个节点状态：
  - `curl http://39.104.205.67:6632/v1/chain/status`
  - 返回 `ok=true`
  - 返回 `node_id=triad-testnet-storage`
  - 返回 `last_error=null`
  - 返回 `network_tier.tier=public_testnet`
3. 从当前控制机直接访问 explorer 概览：
  - `curl http://39.104.205.67:6632/v1/chain/explorer/overview`
  - 返回 `ok=true`
  - 返回 `latest_height=4826`
  - 返回 `world_id=oasis7-public-testnet-parallel-20260518`

## 边界更正
1. `t2t.oasis7.tech` 不是 testnet 专用域名，因此本轮不再使用它作为任何 testnet readiness 证据。
2. 这次改为用公网 `IP:port` 直接验证 testnet RPC / explorer，而不是借用非 testnet 专用域名。

## Readiness 收口
1. `runtime_bootstrap`
  - `pass`
  - 依据:
    - 2026-05-18 的本地三节点 formal manifest rehearsal 已确认该 lane 可 `pass`
    - 本轮又补充确认两台 ECS 的 live runtime 已实际加载 formal `public_testnet` manifest
2. `public_rpc_ready`
  - `pass`
  - 依据:
    - `http://39.104.204.172:6631/v1/chain/status` 已从当前控制机直接返回 `ok=true`
    - live status 中 `network_tier.rpc_ref` 已与该公网入口一致
3. `explorer_public_ready`
  - `pass`
  - 依据:
    - `http://39.104.205.67:6632/v1/chain/explorer/overview` 已从当前控制机直接返回 `ok=true`
    - live status 中 `network_tier.explorer_ref` 已与该公网入口一致
4. `faucet_guard_ready`
  - `partial`
  - 依据:
    - 本轮没有真实 guarded faucet 与 abuse control 验证
5. `reset_policy_announced`
  - `pass`
  - 依据:
    - manifest 已声明 `reset_policy=resettable`
    - 已发布公开 GitHub issue `#249` `Public testnet reset policy announcement for oasis7-public-testnet-parallel-20260518`
    - issue 已明确当前 `public_testnet` 的 reset 语义、claim boundary 与当前公网入口
6. `claims_boundary_review`
  - `partial`
  - 依据:
    - `#249` 只提供 reset policy public announcement
    - 本轮仍没有 producer / QA / liveops 的正式 claims review 记录
7. `shared_devnet_pass`
  - `partial`
  - 依据:
    - 仍沿用既有 repo 证据状态，没有被这次部署替代

## 2026-05-21 QA addendum
1. `doc/testing/evidence/public-testnet-claims-boundary-review-2026-05-21.md` 已补 repo-owned `qa_engineer` claims verdict。
2. 因此 `claims_boundary_review` 这条 lane 不再依赖 `#249` 的 reset announcement 单独代偿。
3. 该补充不会改变本文件主结论：aggregate readiness 仍受 `shared_devnet_pass` 未满足约束。

## 2026-05-22 operator correction
1. 本文件的拓扑描述包含了本机 `oasis7-testnet-observer.service`，但 2026-05-19 当天真正执行的 repo-owned deploy 动作只覆盖了两台 ECS。
2. 后续 fresh audit 证明本机 `/opt/oasis7/p2p-testnet-local` 仍保留旧的三 validator / 三 signer 合同，也没有 `NETWORK_TIER_MANIFEST_PATH`。
3. 因此不能再把“2026-05-19 endpoint deploy”理解为三节点全部已收口到同一份 formal `public_testnet` contract。
4. 当前正确的 local remediation 入口已经收口到：
  - `scripts/p2p-public-testnet-local-observer-sync.sh`
  - `doc/testing/evidence/public-testnet-local-observer-contract-sync-2026-05-22.md`
5. 截至 2026-05-22 13:25 CST，本机 observer 已经通过上述脚本完成 live apply，并成功加载 `public_testnet` manifest。
6. 截至 2026-05-22 16:31 CST，`fetch-commit authorization failed` 与 writer-switch stale-state 已不再是主阻断；repo-owned `reset-state` 也已加入同一脚本并执行过多轮 live reset。
7. 但当前本机仍在 `height 15` 卡住 execution hash mismatch，且 live current runtime binary hash 已与 ECS 对齐、却仍与 mirrored candidate bundle 的 `runtime_build.sha256` 分叉。因此这份 2026-05-19 endpoint deploy 证据只能继续证明公网入口存在，不能证明 triad runtime 已健康收敛。
8. 后续镜像的 live-candidate manifest 已把 `faucet_ref` 固定为 `http://39.104.204.172:6681/`，不再是 placeholder；当前保守结论应聚焦 runtime / shared-devnet gate 仍未同时满足，而不是继续把 manifest faucet 字段称为 placeholder。

## 结论
1. 这次“进一步做部署”已经把两台阿里云上的 testnet 节点推进成了实际加载 formal `public_testnet` manifest 的 live runtime。
2. 这次通过公网 `IP:port` 直接验证，已经把 `public_rpc_ready` 与 `explorer_public_ready` 推进成 `pass`，不依赖 testnet 专用域名。
3. 当前仍然不能放行为 `ready_for_live_candidate`，因为这份 2026-05-19 endpoint evidence 只证明公网 RPC / explorer 入口存在；后续 repo-owned evidence 已补齐 `faucet_guard_ready`、`claims_boundary_review`，且 mirrored live-candidate manifest 的 `faucet_ref` 已是真实 guarded faucet URL，但 readiness 仍必须由最新 seven-lane gate 重新聚合，不能由本文件单独升级。
