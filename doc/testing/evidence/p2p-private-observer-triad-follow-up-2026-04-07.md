# P2P 私有 observer triad follow-up 验证（2026-04-07）

- 对应专题: `doc/p2p/network/p2p-mainnet-private-reachability-architecture-2026-04-01.project.md`
- 对应任务: `P2PARCH-6`
- PM task: `task_c0fa78756f6e4105abdd0d7f5f96de2d`
- owner_role: `runtime_engineer`

## 环境

- 本机 observer:
  - node_id: `triad-observer-local`
  - gossip bind: `0.0.0.0:5613`
  - replication listen: `/ip4/0.0.0.0/tcp/5613`
- 阿里云 sequencer:
  - node_id: `triad-sequencer-a`
  - gossip bind: `0.0.0.0:5611`
  - replication listen: `/ip4/0.0.0.0/tcp/5611`
  - public IP: `39.104.204.172`
- 阿里云 storage:
  - node_id: `triad-storage-b`
  - gossip bind: `0.0.0.0:5612`
  - replication listen: `/ip4/0.0.0.0/tcp/5612`
  - public IP: `39.104.205.67`

## 本次落地

- 统一部署二进制:
  - `sha256=004aaf7529a4c1e26be5150aaf87ac4b648e241f29295b2dc23824d516ea4785`
- 本机 observer 启动参数新增:
  - `--node-validator-signer-public-key triad-sequencer-a:4e15e631d5d76029c502b4a36ec119670be72cb4fe342dd607ec9dce1ecc3cea`
  - `--node-validator-signer-public-key triad-storage-b:cab725ca1a58dbe1b7e4fdd6920ff18b41cc3020e5195058b085a73744e7a6ab`
  - `--replication-network-peer /ip4/39.104.204.172/tcp/5611`
  - `--replication-network-peer /ip4/39.104.205.67/tcp/5612`
- 两台云节点新增:
  - `--replication-remote-writer-public-key f1ac8ce49716bfb59d972cb17ad8b2afa050424e797d23a008b956ad8e654a06`
- follow-up 修平的真实阻断顺序:
  - reverse UDP path 未建立
  - mixed-root validator signer mismatch
  - replication topology 未显式配置
  - replication fetch requester allowlist 未放行 observer signer

## 验证命令

```bash
sha256sum /opt/oasis7/p2p-triad-local/current/bin/oasis7_chain_runtime
sshpass -p '&UJM8ik,' ssh -o StrictHostKeyChecking=no root@39.104.204.172 \
  "sha256sum /opt/oasis7/p2p-triad/current/bin/oasis7_chain_runtime"
sshpass -p '%TGB6yhn' ssh -o StrictHostKeyChecking=no root@39.104.205.67 \
  "sha256sum /opt/oasis7/p2p-triad/current/bin/oasis7_chain_runtime"

curl -fsS http://127.0.0.1:5633/v1/chain/status | jq '{observed_at_unix_ms,node_id,consensus:.consensus|{latest_height,committed_height,network_committed_height,known_peer_heads,last_status},last_error}'
sshpass -p '&UJM8ik,' ssh -o StrictHostKeyChecking=no root@39.104.204.172 \
  "curl -fsS http://127.0.0.1:5631/v1/chain/status | jq '{observed_at_unix_ms,node_id,consensus:.consensus|{latest_height,committed_height,network_committed_height,known_peer_heads,last_status},last_error}'"
sshpass -p '%TGB6yhn' ssh -o StrictHostKeyChecking=no root@39.104.205.67 \
  "curl -fsS http://127.0.0.1:5632/v1/chain/status | jq '{observed_at_unix_ms,node_id,consensus:.consensus|{latest_height,committed_height,network_committed_height,known_peer_heads,last_status},last_error}'"
```

## 结果快照

### observer-local

- `observed_at_unix_ms=1775575100504`
- `latest_height=88`
- `committed_height=88`
- `network_committed_height=58086`
- `known_peer_heads=1`
- `last_status=pending`
- `last_error=null`

### storage-b

- `observed_at_unix_ms=1775575101543`
- `latest_height=58086`
- `committed_height=58086`
- `network_committed_height=58086`
- `known_peer_heads=0`
- `last_status=pending`
- `last_error=null`

### sequencer-a

- `observed_at_unix_ms=1775575102907`
- `latest_height=0`
- `committed_height=0`
- `network_committed_height=0`
- `known_peer_heads=0`
- `last_status=null`
- `last_error=node execution error: execution driver received stale height: context=57536 state=57560`

## 结论

- `P2PARCH-6` 在这组真实三节点上已修平 private observer 的核心 reachability blocker。
- 真实 observer 已不再停在 `known_peer_heads=0 / network_committed_height=0 / committed_height=0`，而是能看到云端 head，并持续拉取 commit 到本地。
- 当前窗口内残留问题已经切换为 `triad-sequencer-a` 的本地执行态陈旧；它不再构成“private observer 无法反向建链”的同类 blocker。
