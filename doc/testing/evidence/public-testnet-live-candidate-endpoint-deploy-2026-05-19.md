# Public Testnet Live-Candidate Endpoint Deploy (2026-05-19)

审计轮次: 1

## Meta
- 责任角色:
  - `runtime_engineer`
- 当前结论:
  - `runtime_bootstrap=pass`
  - `public_rpc_ready!=pass`
  - `explorer_public_ready!=pass`
  - `overall_readiness=block`
- worktree:
  - `task/p2p-public-testnet-live-candidate-endpoint-deploy`
- 对应 world:
  - `oasis7-public-testnet-parallel-20260518`
- 远端节点:
  - `oasis7-testnet-sequencer.service` on `39.104.204.172`
  - `oasis7-testnet-storage.service` on `39.104.205.67`
  - `oasis7-testnet-observer.service` on local host
- 公网域名:
  - `https://t2t.oasis7.tech`
- 本轮生成文件:
  - `output/public-testnet-live-candidate/public-testnet-live-candidate.bundle.json`
  - `output/public-testnet-live-candidate/network-tier-public-testnet-live-candidate.local.json`
  - `output/public-testnet-live-candidate/network-tier-public-testnet-live-candidate.remote.json`
  - `output/public-testnet-live-candidate/bootstrap-peers.txt`

## 本轮目标
1. 基于已有两台阿里云机器，把现有 `oasis7-testnet-*` 真实节点推进到 formal `public_testnet` manifest 语义。
2. 给 `t2t.oasis7.tech` 补真实链状态 / explorer 代理入口，验证哪些 readiness lane 能从“只在本地 rehearsal”往前推进。
3. 明确区分“远端 runtime 已部署”与“公网入口已可从当前控制机确认 pass”。

## Repo 侧变更
1. `scripts/p2p-triad-node-start.sh`
  - 新增 `NETWORK_TIER_MANIFEST_PATH` 环境变量支持。
  - dry-run 已确认会透传 `--network-tier-manifest <path>` 到 `oasis7_chain_runtime`。
2. `scripts/provider-remote-https/t2t.oasis7.tech.nginx.conf`
  - 新增 `/healthz`、`/v1/chain/status`、`/v1/chain/explorer/` 代理到 `127.0.0.1:6632`。

## 远端部署动作
1. 下发 public-testnet live-candidate bundle / manifest / bootstrap / genesis 到两台 ECS：
  - `/opt/oasis7/p2p-testnet/config/public-testnet-live-candidate.bundle.json`
  - `/opt/oasis7/p2p-testnet/config/network-tier-public-testnet-live-candidate.json`
  - `/opt/oasis7/p2p-testnet/config/bootstrap-peers.txt`
  - `/opt/oasis7/p2p-testnet/config/public-testnet-genesis.example.json`
2. 用 repo-owned `scripts/p2p-triad-node-start.sh` 覆盖两台 ECS 的 `/opt/oasis7/p2p-testnet/bin/start-node.sh`。
3. 在两台 ECS 的 `config/node.env` 中写入：
  - `NETWORK_TIER_MANIFEST_PATH=/opt/oasis7/p2p-testnet/config/network-tier-public-testnet-live-candidate.json`
4. 重启：
  - `systemctl restart oasis7-testnet-sequencer.service`
  - `systemctl restart oasis7-testnet-storage.service`
5. 在 `39.104.205.67` 上更新并 reload nginx：
  - `/etc/nginx/sites-available/t2t.oasis7.tech.conf`
  - `nginx -t`
  - `systemctl reload nginx`

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
  - `network_tier.rpc_ref=https://t2t.oasis7.tech/v1/chain/status`
  - `network_tier.explorer_ref=https://t2t.oasis7.tech/v1/chain/explorer/overview`
  - `committed_height=4704`
  - `network_committed_height=4704`

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
  - `network_tier.rpc_ref=https://t2t.oasis7.tech/v1/chain/status`
  - `network_tier.explorer_ref=https://t2t.oasis7.tech/v1/chain/explorer/overview`
  - `committed_height=4701`
  - `network_committed_height=4701`

## 域名入口验证
### 在 storage 主机本机验证
1. `curl -k https://127.0.0.1/v1/chain/status -H 'Host: t2t.oasis7.tech'`
  - 返回 `ok=true`
  - 返回 `node_id=triad-testnet-storage`
  - 返回 `network_tier.tier=public_testnet`
2. `curl -kv https://t2t.oasis7.tech/v1/chain/status`
  - 已完成 TLS 握手
  - 返回 `HTTP/2 200`

### 在当前控制机验证
1. `curl -k https://t2t.oasis7.tech/v1/chain/status`
2. `curl -k https://t2t.oasis7.tech/v1/chain/explorer/overview`
3. `openssl s_client -connect t2t.oasis7.tech:443 -servername t2t.oasis7.tech -tls1_2`

结果:
- 三条命令都失败在 TLS 建连阶段或直接 `connection reset by peer`。
- 因此当前只能确认“域名入口已在远端主机本机打通”，不能确认“公网入口已从当前控制机稳定可达”。

## Readiness 收口
1. `runtime_bootstrap`
  - `pass`
  - 依据:
    - 2026-05-18 的本地三节点 formal manifest rehearsal 已确认该 lane 可 `pass`
    - 本轮又补充确认两台 ECS 的 live runtime 已实际加载 formal `public_testnet` manifest
2. `public_rpc_ready`
  - `partial`
  - 依据:
    - 域名与 nginx 路由已部署
    - storage 主机本机可 `HTTP 200`
    - 但当前控制机仍遇到 TLS reset，不能记 `pass`
3. `explorer_public_ready`
  - `partial`
  - 依据:
    - 域名路由已部署到 `/v1/chain/explorer/`
    - 但当前控制机仍遇到 TLS reset，不能记 `pass`
4. `faucet_guard_ready`
  - `partial`
  - 依据:
    - manifest 已声明 `faucet_ref=https://t2t.oasis7.tech/faucet`
    - 但本轮没有真实 guarded faucet 与 abuse control 验证
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
2. 这次部署把 `t2t.oasis7.tech` 的链状态 / explorer 入口推进到“远端主机本机可用”，但还没有把它推进到“从当前控制机可稳定确认 pass”。
3. 因此截至本轮，新增的不是“更多 lane 已正式 pass”，而是把 `public_rpc_ready` / `explorer_public_ready` 从纯理论推进成了有真实公网入口部署、但仍待外部可达性收口的 `partial`。
