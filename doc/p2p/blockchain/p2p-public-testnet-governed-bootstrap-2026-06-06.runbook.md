# Public Testnet Governed Bootstrap Runbook (2026-06-06)

- 对应项目文档:
  - `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.project.md`
- 关联证据:
  - `doc/testing/evidence/public-testnet-governed-bootstrap-manifest-2026-06-06.json`
  - `doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json`
  - `doc/testing/evidence/public-testnet-governed-bootstrap-genesis-2026-06-06.json`
  - `doc/testing/evidence/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json`
  - `doc/testing/evidence/public-testnet-governed-bootstrap-world-2026-06-06/world`
  - `doc/testing/evidence/public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.txt`
  - `doc/testing/evidence/public-testnet-governed-bootstrap-topology-2026-06-06.md`

审计轮次: 2

## 1. Purpose
这份 runbook 的目标不是描述理念，而是定义一条可以重复执行的四节点 `public_testnet` 重建流程：

1. 从冻结的 governed bootstrap truth 出发。
2. 先拉起 2 个 validator。
3. 再以 observer 身份接入 2 个本地节点。
4. 将每一步的失败边界压缩为可判定的 gate，而不是运行中再猜。

这份 runbook 明确吸收了本轮 live rebuild 的几个教训：

1. `node-keypair.toml` 是受保护真值，删除它会连带改变派生 signer truth。
2. libp2p bootstrap peer id 是部署真值，不是可以长期硬编码的常量。
3. observer 超过低高度阈值后不能再依赖空 world 自举，必须走 verified seed/state-sync bundle。
4. state-sync artifact 必须是闭包完整的，不能只拷 `world/` 和 `execution-records/`，遗漏 `store/blobs/`。

## 2. Scope and Topology
固定目标拓扑:

| node_id | role | location | in validator set |
| --- | --- | --- | --- |
| `triad-testnet-sequencer` | validator / sequencer | ECS `39.104.204.172` | yes |
| `triad-testnet-storage` | validator / storage | ECS `39.104.205.67` | yes |
| `triad-testnet-local` | observer | local | no |
| `triad-testnet-fourth-local` | observer | local | no |

固定 stack root:

- ECS validator stack root: `/opt/oasis7/p2p-testnet`
- local observer stack roots:
  - `.tmp/testnet-local-node-bootstrap`
  - `.tmp/testnet-fourth-node-bootstrap`

## 3. Truth Model
必须先区分两类真值：

### 3.1 Frozen repo truth
仓库里冻结的治理/启动真值：

- manifest
- genesis
- validator registry
- canonical bootstrap world
- bootstrap peers
- release bundle schema

这些文件定义“网络应当是什么”。

### 3.2 Deployment truth
真正部署时动态生成或动态确认的真值：

- validator host 上实际存在的 `config/node-keypair.toml`
- 从 root node key 派生出的 consensus signer public key
- validator 实际 libp2p `local_peer_id`
- 当前 runtime package sha256
- 当前可用于 observer attach 的 state-sync / seed bundle

这些值定义“当前这一轮 live rebuild 实际用了什么”。

结论:

1. repo truth 冻结后，不代表 deployment truth 永远不变。
2. 只要 validator host key 变化，deployment truth 就必须重新生成。
3. observer 接入必须消费 deployment truth，不能继续使用旧的 bootstrap peer id / signer / seed artifact。

## 4. Hard Rules
下面这些规则必须作为强约束执行：

1. 禁止对 `config/` 目录使用会删除未列出文件的同步方式。
   - 特别是禁止裸用 `rsync --delete` 覆盖整目录。
2. 禁止删除 validator 的 `config/node-keypair.toml`，除非明确要做 key rotation。
3. 一旦发生 key rotation，必须把以下真值一并重建：
   - derived validator signer truth
   - validator registry
   - governed bootstrap world
   - observer `REPLICATION_REMOTE_WRITERS_CSV`
4. libp2p bootstrap peer id 必须在 validator 启动后实时读取，不能沿用旧证据文件中的值。
5. 当 validator 高度超过低高度阈值后，observer 不得使用空 world 从 genesis replay 追链，必须改走 verified seed/state-sync。
6. seed/state-sync 产物必须包含 execution restore 所需 blob 闭包，否则 observer 会在 restore snapshot/journal 时落入 `BlobNotFound`。

## 5. Required Inputs
开始前，操作者必须准备好以下输入：

### 5.1 Repo artifacts

- `doc/testing/evidence/public-testnet-governed-bootstrap-manifest-2026-06-06.json`
- `doc/testing/evidence/public-testnet-governed-bootstrap-genesis-2026-06-06.json`
- `doc/testing/evidence/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json`
- `doc/testing/evidence/public-testnet-governed-bootstrap-world-2026-06-06/world`
- `doc/testing/evidence/public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.txt`

### 5.2 Runtime/package artifact

必须是 Linux package truth，不允许用本机 macOS binary 直接替代 ECS runtime。

最少校验项：

```bash
./scripts/release-candidate-bundle.sh validate \
  --bundle doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json
```

### 5.3 Host access

- validator ECS SSH 可写权限
- local observer stack 可写权限

## 6. Phase Overview
执行顺序固定为：

1. Phase A: Preflight and truth capture
2. Phase B: Build deployment truth
3. Phase C: Stage and rebuild validators
4. Phase D: Verify validator pair
5. Phase E: Build verified observer seed/state-sync bundle
6. Phase F: Attach observers
7. Phase G: Final health verification
8. Phase H: Failure handling and rollback

任何 phase 未通过，不进入下一 phase。

## 7. Phase A - Preflight and Truth Capture
### Goal
在任何 destructive 操作前，把本轮 live rebuild 真值先读出来并记下。

### Required checks
1. 优先使用标准 truth capture 脚本：

```bash
./scripts/p2p-public-testnet-capture-truth.sh \
  --bundle doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json \
  --sequencer-status-url http://39.104.204.172:6631/v1/chain/status \
  --storage-status-url http://39.104.205.67:6632/v1/chain/status \
  --sequencer-ssh-host root@39.104.204.172 \
  --storage-ssh-host root@39.104.205.67 \
  --out .tmp/public-testnet-deployment-truth.json
```

2. 若只做最小手工校验，至少要覆盖：

```bash
./scripts/release-candidate-bundle.sh validate \
  --bundle doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json
```

3. 记录当前 ECS host key material 是否已存在：

```bash
ssh root@39.104.204.172 'ls -l /opt/oasis7/p2p-testnet/config/node-keypair.toml'
ssh root@39.104.205.67 'ls -l /opt/oasis7/p2p-testnet/config/node-keypair.toml'
```

4. 记录当前远端 runtime hash：

```bash
ssh root@39.104.204.172 'sha256sum /opt/oasis7/p2p-testnet/current/bin/oasis7_chain_runtime'
ssh root@39.104.205.67 'sha256sum /opt/oasis7/p2p-testnet/current/bin/oasis7_chain_runtime'
```

5. 若 validator 已在运行，读取当前实际 libp2p peer id：

```bash
ssh root@39.104.204.172 'curl -s http://127.0.0.1:6631/v1/chain/status | jq -r .replication.local_peer_id'
ssh root@39.104.205.67 'curl -s http://127.0.0.1:6632/v1/chain/status | jq -r .replication.local_peer_id'
```

### Pass criteria
1. bundle validate 通过
2. 已知每台 validator 的当前 runtime hash
3. 已知每台 validator 的 `node-keypair.toml` 是否会被保留
4. 若节点当前可读，已记录 live `local_peer_id`

## 8. Phase B - Build Deployment Truth
### Goal
把本轮 live deploy 需要的动态真值显式生成出来。

### When to rebuild deployment truth
满足任一条件就必须重建 deployment truth：

1. `node-keypair.toml` 不存在
2. `node-keypair.toml` 被替换
3. validator 实际 `local_peer_id` 与旧 bootstrap peers 不一致
4. validator 派生 signer 与旧 registry 不一致

### Required outputs
至少生成或确认以下内容：

1. current validator signer truth
2. current bootstrap peer ids
3. current deployment-only validator registry
4. current deployment-only bootstrap world
5. current deployment-only manifest/bundle wiring

### Required checks
1. 若 key rotation 发生，先从 host 当前 `node-keypair.toml` 派生 signer truth。
2. 用新的 signer truth 生成 deployment-only validator registry。
3. 用新的 registry 重建 deployment-only governed bootstrap world。
4. 读取 validator 实际 `local_peer_id`，更新 observer 的 `REPLICATION_NETWORK_BOOTSTRAP_PEERS_CSV`。
5. 更新 observer 的 `REPLICATION_REMOTE_WRITERS_CSV`，对齐 validator 当前 allowlist。

### Pass criteria
1. deployment-only registry 与 host 当前 signer truth 一致
2. deployment-only bootstrap world 已重新生成并验证
3. observer env 里的 bootstrap peer ids 与 validator live `local_peer_id` 一致

observer env 刷新建议直接使用：

```bash
./scripts/p2p-public-testnet-refresh-bootstrap-peers.sh \
  --sequencer-status-url http://39.104.204.172:6631/v1/chain/status \
  --sequencer-ip 39.104.204.172 \
  --sequencer-port 6831 \
  --storage-status-url http://39.104.205.67:6632/v1/chain/status \
  --storage-ip 39.104.205.67 \
  --storage-port 6832 \
  --env-file .tmp/testnet-local-node-bootstrap/node.env \
  --env-file .tmp/testnet-fourth-node-bootstrap/node.env
```

## 9. Phase C - Stage and Rebuild Validators
### Goal
从零重建 validator pair。

### Stage
把以下内容放到两台 validator：

- current release package/runtime
- manifest
- genesis
- validator registry
- bootstrap peers
- deployment-only bootstrap world

建议直接使用标准重建脚本，而不是临时拼接远程命令。该脚本已经处理了一个真实踩过的流程坑：不能把同一条 tar 流连续喂给两个远端 `tar -xf -`，否则第二次解包只会读到 EOF 并报 `This does not look like a tar archive`。脚本会先把 world 解到 `staged-world/`，再在远端复制到 `data/execution-world/`：

```bash
./scripts/p2p-public-testnet-rebuild-validators.sh \
  --config-dir .tmp/public-testnet-ci-rebuild-stage/config \
  --world-dir .tmp/public-testnet-ci-rebuild-stage/generated-world-from-rotated-signers/world \
  --sequencer-ssh-host root@39.104.204.172 \
  --sequencer-sshpass-env PUBLIC_TESTNET_SEQUENCER_SSHPASS \
  --sequencer-service oasis7-testnet-sequencer.service \
  --sequencer-status-url http://39.104.204.172:6631/v1/chain/status \
  --storage-ssh-host root@39.104.205.67 \
  --storage-sshpass-env PUBLIC_TESTNET_STORAGE_SSHPASS \
  --storage-service oasis7-testnet-storage.service \
  --storage-status-url http://39.104.205.67:6632/v1/chain/status \
  --out-dir .tmp/public-testnet-validator-rebuild
```

### Reset
停止服务并 destructive reset 旧链状态：

```bash
systemctl stop oasis7-testnet-sequencer.service
systemctl stop oasis7-testnet-storage.service
```

必须清理旧链数据目录，但保留受保护的 `config/node-keypair.toml`，除非本轮明确要轮换 key。

### Start order
固定顺序：

1. start `triad-testnet-sequencer`
2. confirm sequencer starts cleanly
3. start `triad-testnet-storage`
4. confirm storage joins sequencer

### Validator launch invariants
1. `NETWORK_TIER_MANIFEST_PATH` 必须指向 governed bootstrap manifest
2. `GENESIS_VALIDATOR_REGISTRY_PATH` 必须指向当前 deployment truth registry
3. `EXECUTION_WORLD_DIR` 必须预先放好 deployment truth world
4. runtime binary hash 必须与当前 staged package 一致
5. world staging 必须是“单次传输 + 远端复制”，不能复用同一 stdin tar 流做双重解包

## 10. Phase D - Verify Validator Pair
### Goal
确认两台 validator 形成可持续推进的 governed chain。

### Required checks
对两台 validator 分别检查：

```bash
curl -s http://127.0.0.1:6631/v1/chain/status | jq '{running,last_error,committed_height:.consensus.committed_height,last_execution_height:.consensus.last_execution_height,connected_peers:.replication.connected_peers,local_peer_id:.replication.local_peer_id}'
curl -s http://127.0.0.1:6632/v1/chain/status | jq '{running,last_error,committed_height:.consensus.committed_height,last_execution_height:.consensus.last_execution_height,connected_peers:.replication.connected_peers,local_peer_id:.replication.local_peer_id}'
```

### Pass criteria
1. `running=true`
2. `last_error=null`
3. sequencer:
   - `committed_height > 0`
   - `last_execution_height > 0`
4. storage:
   - `committed_height > 0`
   - `network_head.height >= committed_height`
   - `connected_peers` 至少包含 sequencer
5. sequencer 和 storage 互相出现在 `connected_peers`
6. 记录本轮 live `local_peer_id`
7. 若 storage `last_execution_height == 0`，本轮不得把 storage 当作 observer seed source；但只要 storage 已验证并传播 sequencer 的 execution binding，就不应把这件事本身视作 validator rebuild 失败

### Hard stop conditions
出现以下任一项立即停止并回到 Phase B：

1. signer binding mismatch
2. libp2p `WrongPeerId`
3. empty-world `height 1` execution mismatch
4. validator 自身 `BlobNotFound`

## 11. Phase E - Build Verified Observer Seed/State-Sync Bundle
### Goal
不要再让 observer 从空 world 硬追到高链高，而是从 validator 导出闭包完整的 seed artifact。

### Required contents
observer seed/state-sync bundle 至少要覆盖：

1. `world/`
2. `execution-records/`
3. `store/`
4. restore snapshot/journal 所需全部 `store/blobs/`
5. 若运行时依赖存在，还要包含:
   - simulator mirror
   - execution bridge state
   - replication head metadata

### Important rule
如果只复制 `world/` 与 `execution-records/`，但没有复制 restore 需要的 blob 闭包，observer 会在如下阶段失败：

- `restore snapshot ref ... BlobNotFound`
- `restore journal ref ... BlobNotFound`

所以导出脚本必须保证 storage closure 完整，不能做“最小猜测拷贝”。

### Pass criteria
1. seed bundle 对单个 observer 恢复后，runtime 不报 `BlobNotFound`
2. seed bundle 可被两个 observer 重复消费

在 observer 启动前，必须对 seed bundle 执行闭包校验：

```bash
./scripts/p2p-verify-state-sync-closure.sh \
  --world-dir <seed-world-dir> \
  --execution-records-dir <seed-execution-records-dir> \
  --store-dir <seed-store-dir>
```

## 12. Phase F - Attach Observers
### Goal
把两个 observer 接进 validator 网络。

### Required prep
1. observer env 使用当前 deployment truth bootstrap peer ids
2. observer env 使用当前 validator writer allowlist
3. observer manifest 指向当前 deployment truth genesis/manifest
4. observer state 先 reset，再导入 verified seed/state-sync bundle

### Start order
1. `triad-testnet-local`
2. verify
3. `triad-testnet-fourth-local`
4. verify

### Required checks
```bash
curl -s http://127.0.0.1:19082/v1/chain/status | jq '{running,last_error,committed_height:.consensus.committed_height,network_committed_height:.consensus.network_committed_height,last_execution_height:.consensus.last_execution_height,connected_peers:.replication.connected_peers}'
curl -s http://127.0.0.1:19083/v1/chain/status | jq '{running,last_error,committed_height:.consensus.committed_height,network_committed_height:.consensus.network_committed_height,last_execution_height:.consensus.last_execution_height,connected_peers:.replication.connected_peers}'
```

### Pass criteria
1. `running=true`
2. 两个 observer 都能看到 validator peer
3. `last_error=null`
4. `committed_height` 和 `last_execution_height` 向 validator 高度收敛

### Hard stop conditions
出现以下任一项，observer 接入失败，回到 Phase E：

1. `WrongPeerId`
2. `fetch requester is not authorized`
3. `BlobNotFound`
4. `height 1 peer commit execution mismatch`

## 13. Phase G - Final Health Verification
### Goal
确认“四节点已跑起来”和“四节点真正 ready”不是一回事，并分别打结论。

### Required final snapshot
需要同时保留 validator 和 observer 四个状态快照，至少包括：

1. `running`
2. `last_error`
3. `committed_height`
4. `network_committed_height`
5. `last_execution_height`
6. `connected_peers`
7. `readiness.ready`
8. `readiness.failed_gates`

### Verdict rules
1. **Four-node live**
   - 四个节点都在跑
   - validator 与 observer 都已接入
   - 高度在收敛或已收敛

2. **Four-node healthy**
   - 在满足 four-node live 的基础上
   - `last_error=null`
   - 不存在阻断性 gate
   - observer 不再依赖缺失 blob / 缺失 state-sync artifact

如果只满足第一条，不得对外宣称“完全健康”。

## 14. Phase H - Failure Handling and Rollback
### Deployment rollback
如果 validator 新包启动失败：

1. 切回上一版 `current` symlink
2. 恢复上一版 manifest/config
3. 保留失败现场日志和当前 deployment truth 快照

### Observer rollback
如果 observer 接入失败：

1. 不改 validator 链真值
2. 保留 observer 当前 seed/state-sync 失败现场
3. 回退到上一个 verified seed bundle 或重新导出完整 bundle

### What not to do
1. 不要为 observer attach 问题去修改 validator registry 真值
2. 不要在 peer id 已变的情况下继续使用旧 bootstrap peer list
3. 不要在缺少完整 blob 闭包时宣布 state-sync 已可用

## 15. Standard Command Checklist
### Preflight
```bash
./scripts/p2p-public-testnet-preflight.sh \
  --bundle doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json \
  --sequencer-status-url http://39.104.204.172:6631/v1/chain/status \
  --sequencer-ip 39.104.204.172 \
  --sequencer-port 6831 \
  --storage-status-url http://39.104.205.67:6632/v1/chain/status \
  --storage-ip 39.104.205.67 \
  --storage-port 6832 \
  --sequencer-ssh-host root@39.104.204.172 \
  --sequencer-sshpass-env PUBLIC_TESTNET_SEQUENCER_SSHPASS \
  --storage-ssh-host root@39.104.205.67 \
  --storage-sshpass-env PUBLIC_TESTNET_STORAGE_SSHPASS \
  --observer-env .tmp/testnet-local-node-bootstrap/node.env \
  --observer-env .tmp/testnet-fourth-node-bootstrap/node.env \
  --out-dir .tmp/public-testnet-preflight
```

### Validator status
```bash
ssh root@39.104.204.172 'curl -s http://127.0.0.1:6631/v1/chain/status'
ssh root@39.104.205.67 'curl -s http://127.0.0.1:6632/v1/chain/status'
```

### Observer status
```bash
curl -s http://127.0.0.1:19082/v1/chain/status
curl -s http://127.0.0.1:19083/v1/chain/status
```

### Peer id truth
```bash
./scripts/p2p-public-testnet-refresh-bootstrap-peers.sh \
  --sequencer-status-url http://39.104.204.172:6631/v1/chain/status \
  --sequencer-ip 39.104.204.172 \
  --sequencer-port 6831 \
  --storage-status-url http://39.104.205.67:6632/v1/chain/status \
  --storage-ip 39.104.205.67 \
  --storage-port 6832 \
  --env-file .tmp/testnet-local-node-bootstrap/node.env \
  --env-file .tmp/testnet-fourth-node-bootstrap/node.env
```

### Seed closure
```bash
./scripts/p2p-verify-state-sync-closure.sh \
  --world-dir <seed-world-dir> \
  --execution-records-dir <seed-execution-records-dir> \
  --store-dir <seed-store-dir>
```

## 16. Open Design Follow-Ups
这份 runbook 可以让部署更稳，但它也明确暴露出两项仍需代码修复的设计缺口：

1. empty-world observer bootstrap 与 validator live execution truth 在 height 1 仍不严格同构
2. 当前 observer seed/state-sync 导出链路还没有被产品化成“闭包完整的标准 artifact”

所以后续必须补两类自动化：

1. 空 world observer 对 validator truth 的回归测试
2. seed/state-sync bundle 完整性回归测试，至少覆盖 restore snapshot/journal 所需 blob 闭包

## 17. Completion Boundary
这份 runbook 的交付物是“标准流程”，不是单次 live 执行记录。

每次真实执行仍然必须额外记录：

1. 当前 runtime/package hash
2. 当前 validator signer truth
3. 当前 validator live peer ids
4. 当前 observer seed bundle 来源与 hash
5. 最终四节点状态快照
