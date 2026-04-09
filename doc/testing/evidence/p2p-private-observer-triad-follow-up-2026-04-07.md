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
SSHPASS="$P2PARCH6_SEQ_SSH_PASSWORD" sshpass -e ssh -o StrictHostKeyChecking=no root@39.104.204.172 \
  "sha256sum /opt/oasis7/p2p-triad/current/bin/oasis7_chain_runtime"
SSHPASS="$P2PARCH6_STORAGE_SSH_PASSWORD" sshpass -e ssh -o StrictHostKeyChecking=no root@39.104.205.67 \
  "sha256sum /opt/oasis7/p2p-triad/current/bin/oasis7_chain_runtime"

curl -fsS http://127.0.0.1:5633/v1/chain/status | jq '{observed_at_unix_ms,node_id,consensus:.consensus|{latest_height,committed_height,network_committed_height,known_peer_heads,last_status},last_error}'
SSHPASS="$P2PARCH6_SEQ_SSH_PASSWORD" sshpass -e ssh -o StrictHostKeyChecking=no root@39.104.204.172 \
  "curl -fsS http://127.0.0.1:5631/v1/chain/status | jq '{observed_at_unix_ms,node_id,consensus:.consensus|{latest_height,committed_height,network_committed_height,known_peer_heads,last_status},last_error}'"
SSHPASS="$P2PARCH6_STORAGE_SSH_PASSWORD" sshpass -e ssh -o StrictHostKeyChecking=no root@39.104.205.67 \
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

## 2026-04-09 宿主机追加取证

### 当前 triad 真值

- 本机 observer 已继续推进到 `committed_height=38271`、`network_committed_height=63589`、`known_peer_heads=2`。
- ECS storage 已继续推进到 `committed_height=63589`、`network_committed_height=63589`、`known_peer_heads=1`，`last_error=null`。
- 真实环境的第一阻断已不再是 observer 反向建链，而是 ECS sequencer 被宿主机侧 kill 后无法稳定暴露。

### sequencer kill 触发矩阵

- `systemd-run ... /bin/sleep 20` 可正常跑满，因此不是“名字像 sequencer 的 unit 一律被杀”。
- `systemd-run` 起 `oasis7_chain_runtime`，哪怕改临时端口 `6631/6611` 且改 `/tmp` 数据目录，仍会在数秒内 `status=9/KILL`。
- 进一步将 SSH session 内启动的进程手工迁出 `session-*.scope` 到 cgroup 根 `/` 后：
  - 普通 `sleep 120` 可在 session 结束后继续存活；
  - 使用正式 `execution-world` / `execution-records` 路径的 `oasis7_chain_runtime` 仍会被直接 `Killed`；
  - 使用临时 execution 路径但保留真实 `storage-root=/opt/oasis7/p2p-triad/data/storage` 的 `oasis7_chain_runtime` 可在正式 `5611/5631` 端口上稳定存活超过 `1m40s`。

### 新结论

- 当前 kill 不再依赖 systemd unit，也不再依赖 SSH session cgroup；宿主机侧外力可以直接命中非 systemd、root cgroup 下的 runtime。
- 正式端口 `5611/5631` 本身不是触发条件，因为“正式端口 + 临时 execution 路径 + 真实 storage”可以持续存活。
- `storage-root=/opt/oasis7/p2p-triad/data/storage` 也不是单独触发条件，因为保留真实 storage 时仍可存活。
- 当前最可疑命中面已收敛为正式 `execution-world` / `execution-records` 路径或其内容/文件模式。

### 旁路实验

- 已将 `execution-world`（约 `80M`）与 `execution-records`（约 `214M`）复制到 `/tmp/oasis7-seq-relocated-20260409-142013/`，并配合真实 storage 启动 sequencer。
- 该实例未再触发宿主机 kill，但只跑到 `committed_height=4`，随后报 `gap sync ... persisted commit hash mismatch`，说明“execution 路径换位”可以绕过 kill，却不能直接保证持久状态一致性。
- 因此，这组实验证明宿主机命中面与 execution 路径强相关，但还不能把“复制 execution 目录到新路径”直接视作现网修复方案。

### 2026-04-09 冷拷贝恢复追加结论

- sequencer 本机 execution 持久态本身已经显著落后于当前链高：
  - `execution-records/latest.json.height=58099`
  - `execution-records/checkpoints/latest.json.height=58048`
  - `execution-world/snapshot.manifest.json.epoch=54014`
  - 而 storage 同时已推进到 `committed_height=6359x`
- 继续把本地 execution 状态 relocation 到新持久目录、并将 `replication_root` 切到全新空目录后，宿主机 kill 问题仍可绕开，但新的主错误变成：
  - `node execution error: execution record at height 1 missing latest_state_ref`
- 直接检查 `execution-records/00000000000000000001.json` 后可确认：
  - 该文件虽然标记 `schema_version=2`
  - 但缺少 `latest_state_ref`、`snapshot_ref`、`journal_ref`
  - 只保留 `external_effect_ref`
- 这意味着 sequencer 当前不仅受宿主机策略影响，本地 execution records 也已经不满足当前 runtime 的恢复前提；单靠冷拷贝 relocation 无法把 sequencer 拉回当前高度。

### 2026-04-09 仓内兼容修复补充

- 已在 `execution_bridge` 恢复路径补上针对这类 malformed V2 record 的窄兼容：
  - 若 record 缺 `latest_state_ref` / `snapshot_ref`，但 `execution_state_root` 仍存在，则允许将 `execution_state_root` 直接作为 snapshot CAS key 读取；
  - 若 record 同时缺 `journal_ref`，则允许从当前已加载 `execution_world` 的本地持久 journal 中截取 `snapshot.journal_len` 前缀，并校验 `snapshot.last_event_id` 后再回灌为新的 `journal_ref`。
- 该修复刻意不改全局 deserialization 语义，也不把所有“无 ref 的历史 record”统一自动回填，避免把 retention 已裁掉的 archive-only 高度重新膨胀；只有在 stale-height restore 真正命中且本地数据足以自证恢复时，才会对那个具体 record 做定点修复写回。
- 仓内新增回归已经通过：
  - `execution_bridge_record_recovery_snapshot_ref_falls_back_to_execution_state_root`
  - `node_runtime_execution_driver_recovers_malformed_v2_record_from_state_root_and_local_journal`
  - `node_runtime_execution_driver_reconciles_stale_state_from_exact_record`
- 因此，当前“height 1 missing latest_state_ref”已不再是仓库内代码的硬阻断；下一步应把这版 binary 先用于 relocated execution 路径复验，确认 sequencer 是否能越过 height 1 继续推进。

### 2026-04-09 relocated binary 复验结果

- 已把新逻辑编进 debug binary（`sha256=fdecaf7fe37eef755a8ac9bb154d19a01f7a5dc193daa6072727d8cd7de0e743`）并上传到 sequencer host 的 `/tmp/oasis7_chain_runtime-p2parch6-debug`。
- 在临时停掉 flapping 的 `oasis7-triad-sequencer.service` 后，直接以 relocation execution dirs + copied bridge state 起临时实例：
  - `execution-world-dir=/tmp/oasis7-seq-relocated-20260409-142013/execution-world`
  - `execution-records-dir=/tmp/oasis7-seq-relocated-20260409-142013/execution-records`
  - `execution-bridge-state=/tmp/oasis7-seq-legacy-recovery-20260409-172229/reward-runtime-execution-bridge-state.json`
  - `status-bind=127.0.0.1:6631`
  - `node-gossip-bind=0.0.0.0:6611`
- 这轮复验表明仓内修复已经生效，但又揭出更底层的数据缺失：
  - 临时实例已能打印 `oasis7_chain_runtime ready.`，说明它确实越过了先前 `height 1 missing latest_state_ref` 的结构性阻断；
  - 随后新的主错误变成：`execution driver restore snapshot ref 6674... failed at height 1: BlobNotFound`
- 已直接核对远端 blob 实体缺失：
  - `MISSING:/opt/oasis7/p2p-triad/data/storage/blobs/6674...fb4f7b2a.blob`
  - `MISSING:/tmp/oasis7-seq-relocated-20260409-142013/execution-world/.distfs-state/blobs/6674...fb4f7b2a.blob`
- 因而当前结论需要再次更新：
  - `missing latest_state_ref` 已不是第一阻断；
  - sequencer 现阶段真正缺的是 height-1 对应 snapshot CAS blob 本体；
  - 在补到该 blob、或提供不依赖该 blob 的 execution rebuild / bootstrap 路径前，relocation 实例仍无法把 sequencer 拉回可持续运行态。

### 对当前 triad 的意义

- private observer 的 reachability blocker 依然已经解除，observer/storage 当前都能继续看到云端 head。
- triad 当前新的硬阻断不是 observer path，而是 sequencer 同时背负两层环境债务：
  - 正式 execution 路径会触发宿主机外部 kill
  - 本机现存 execution state / records 已经陈旧，且早期 records 缺字段，无法直接用于本地恢复
- 因此，在拿到新的 execution 恢复源或补出专门的 legacy record 迁移工具前，不能把“旁路 relocation 已足够恢复 sequencer”当成有效结论。

## 2026-04-09 运行态清零复位

### 触发原因

- 同一轮继续实采后，sequencer 清空运行态后虽然已能重新从 `height=0` 起步，但 observer 仍停在旧高度 `38271`，并持续报 `gap sync height ... blob not found for hash ...`。
- 进一步对齐三端状态后，可确认问题已经不再是单点代码回归，而是 triad 三节点运行态代际不一致：
  - observer/sequencer 清态后已经开始生成新链；
  - storage 仍留在旧链 `6397x`；
  - 三端 `last_block_hash` 一度明确分叉。

### 执行动作

- 在用户明确授权“删运行态数据，保留身份配置”后，按同一策略对三节点逐个复位：
  - ECS sequencer：保留 `/opt/oasis7/p2p-triad/config/node.env` 与 `/opt/oasis7/p2p-triad/config/node-keypair.toml`，清空 `data/{execution-world,execution-records,storage}` 与 `output/{chain-runtime,node-distfs}/triad-sequencer-a`。
  - 本机 observer：保留 `/opt/oasis7/p2p-triad-local/config/node.env` 与 `/opt/oasis7/p2p-triad-local/config/node-keypair.toml`，清空 `data/{execution-world,execution-records,storage}` 与 `output/{chain-runtime,node-distfs}/triad-observer-local`。
  - ECS storage：保留 `/opt/oasis7/p2p-triad/config/node.env` 与 `/opt/oasis7/p2p-triad/config/node-keypair.toml`，清空 `data/{execution-world,execution-records,storage}` 与 `output/{chain-runtime,node-distfs}/triad-storage-b`。
- 配置备份产物：
  - sequencer：`/opt/oasis7/p2p-triad/backups/config-20260409-183901.tgz`
  - observer：`/opt/oasis7/p2p-triad-local/backups/config-20260409-184631.tgz`
  - storage：`/opt/oasis7/p2p-triad/backups/config-20260409-184711-storage.tgz`

### 关键观测

- 仅清 sequencer 后的快照 `./.tmp/p2p_real_env_triad/20260409-184116/summary.md`：
  - sequencer `committed_height 8 -> 9`
  - observer 仍停在 `38271`
  - storage 仍在 `63948 -> 63949`
- 再清 observer 后，本机 status 立即回到干净运行态并重新起步：
  - `committed_height=32`
  - `network_committed_height=63970`
  - `known_peer_heads=2`
- 在清 storage 之前，三端短时状态已明确显示新旧链混跑：
  - observer `height=39 last_block_hash=f2fa1b0f...`
  - sequencer `height=39 last_block_hash=f2fa1b0f...`
  - storage `height=63975 last_block_hash=15dbfde7...`

### 最终结果

- 在三节点全部按“保留身份、清空运行态”统一复位后，same-window 快照 `./.tmp/p2p_real_env_triad/20260409-184908/summary.md` 给出：
  - `claim_status=pass_candidate`
  - `failure_signatures=(none)`
- 同窗推进结果：
  - observer：`committed_height 39 -> 42`
  - sequencer：`committed_height 40 -> 43`
  - storage：`committed_height 40 -> 43`
- 三端在复位后重新汇合到同一链：
  - observer/sequencer/storage 在 `height≈39` 时的 `last_block_hash` 已统一为 `f2fa1b0f7e2ded6e922a1a461cc35656df2c8054166adad8c9fc873f1d618e18`

### 更新结论

- 当前三节点真实环境已经不再受“旧 execution/storage 持久态互相污染”的阻断，可在保留身份配置的前提下重新建立一致链并持续推进。
- 这次通过的是“干净运行态 triad 自愈与推进”验证，不等于“旧链历史 execution 数据已被迁移恢复”；旧持久态仍属于另一个恢复议题。

## 2026-04-09 peer fallback / warmup 追加验证

### 第一轮：peer-health fallback

- 已把 `libp2p_replication_network` 的请求候选从纯 `connected_peers()` 扩到：
  - 若 `connected_peers()` 非空，则沿用连接快照
  - 若连接快照暂时为空，则回退到 `peer_healths.active_path_kind.is_some()` 的 peer
- 对应 release：
  - `7401ef56-peer-health-fallback-20260409`
  - `sha256=84cac925ab73e1f8b82d72a05e459ae6b3d856db8b0a6757b5466cbba7f4c170`
- rollout 后若直接在脏运行态上重启，三端会重新分叉：
  - `observer` 落在 `committed_height=187`
  - `sequencer` 落在 `committed_height=85` 且报 `stale height`
  - `storage` 落在 `committed_height=202`
- 因而这轮不能直接拿脏态结果判补丁优劣，必须回到统一清态基线。

### 第二轮：清态后旧阈值 baseline

- 在三端保留身份配置、统一清空运行态后，短窗 `./.tmp/p2p_real_env_triad/20260409-192223/summary.md` 重新恢复：
  - observer：`1 -> 4`
  - sequencer：`1 -> 4`
  - storage：`1 -> 4`
- 但旧阈值 binary 的长窗 `./.tmp/p2p_real_env_triad/20260409-192343/summary.md` 证明问题仍未根治：
  - observer：`8 -> 72`
  - storage：`8 -> 73`
  - sequencer：`7 -> 11`
- 这轮最关键的变化是：sequencer 不再像更早版本那样稳定卡在 `85/86`，而是更早暴露出真正残留：
  - `storage challenge gate network threshold unmet: required_matches=2 successful_matches=1`
  - `gap sync height 12 ... no connected peers for protocol /aw/node/replication/fetch-commit/1.0.0`
- 因而第一轮补丁的价值已经明确：
  - 它把 stall 从“更后面的模糊连接残留”前推成“启动热身期 gate 过硬”的具体问题
  - 但它本身不足以让 sequencer 持续追上 observer/storage

### 第三轮：storage challenge gate warmup

- 已在 storage challenge gate 加入窄范围 warmup：
  - `STORAGE_GATE_NETWORK_WARMUP_HEIGHT=32`
  - 启动早期高度把 `required_matches` 允许降到 `1`
  - 只作用于 clean-start / catch-up 前段，不改变 warmup 之后的完整门槛
- 对应 release：
  - `7401ef56-storage-gate-warmup-20260409`
  - `sha256=64f3571757da20fc9f2d97a2ae33082fa05da35654f9016f8a688faa41c09806`
- rollout 时再次统一清态，且保留身份配置；新增配置备份：
  - observer：`/opt/oasis7/p2p-triad-local/backups/config-20260409-194714-storage-gate-warmup-reset.tgz`
  - sequencer：`/opt/oasis7/p2p-triad/backups/config-20260409-194715-storage-gate-warmup-reset-sequencer.tgz`
  - storage：`/opt/oasis7/p2p-triad/backups/config-20260409-194714-storage-gate-warmup-reset-storage.tgz`

### warmup 版结果

- 短窗 `./.tmp/p2p_real_env_triad/20260409-194737/summary.md`：
  - observer：`2 -> 4`
  - sequencer：`2 -> 5`
  - storage：`2 -> 5`
  - 三端重新对齐，sequencer 早期无错误
- 中窗 `./.tmp/p2p_real_env_triad/20260409-194857/summary.md`：
  - observer：`8 -> 19`
  - sequencer：`8 -> 13`
  - storage：`9 -> 19`
- 这说明 warmup 版确实打穿了旧版稳定卡死的 `11/12` 窗口，sequencer 已明确越过：
  - 旧版基线：`7 -> 11`
  - warmup 版：`8 -> 13`

### 新残留结论

- warmup 版没有彻底修平 triad；它只是把 blocker 再往下收敛了一层。
- sequencer 当前新的主错误已经从“单匹配不够”变成“连 1 个远端有效样本都拿不到”：
  - `storage challenge gate network threshold unmet: required_matches=1 successful_matches=0`
  - 原因组合主要是：
    - `storage challenge gate network blob not found for hash ...`
    - `fetch-blob ... no connected peers`
    - `gap sync height 14 ... no connected peers for protocol /aw/node/replication/fetch-commit/1.0.0`
- 因而下一步的优先级应该转到：
  - sampled blob provider 的发布 / 可见性 / 传播时序
  - sequencer 为什么仍长期维持 `known_peer_heads=0`
  - 将“远端样本暂时 unavailable”与“真实 blob 校验失败”继续拆成不同处理策略

## 2026-04-09 peer-head warmup skip 追加验证

### 补丁内容

- 在 `enforce_storage_challenge_gate()` 中新增一条更窄的 warmup 短路：
  - 当 `committed_height < STORAGE_GATE_NETWORK_WARMUP_HEIGHT`
  - 且 `peer_heads` 仍为空
  - 直接跳过 network blob sampling gate
- 设计意图不是继续放宽完整性门槛，而是避免把“当前还没学到任何 peer head，因而网络采样天然全 unavailable”的状态误算成 content mismatch。
- 对应 release：
  - `7401ef56-storage-gate-peer-head-warmup-skip-20260409`
  - `sha256=423ab45ad2a49606ff89f2551f950b7d3f4de817228c76eb7f57e69702e7e92a`

### rollout 与清态

- rollout 后再次按相同策略统一清态，只保留身份配置：
  - observer：`/opt/oasis7/p2p-triad-local/backups/config-20260409-201039-storage-gate-peer-head-warmup-skip-reset.tgz`
  - sequencer：`/opt/oasis7/p2p-triad/backups/config-20260409-201039-storage-gate-peer-head-warmup-skip-reset-sequencer.tgz`
  - storage：`/opt/oasis7/p2p-triad/backups/config-20260409-201039-storage-gate-peer-head-warmup-skip-reset-storage.tgz`
- 三端 `current/bin/oasis7_chain_runtime` 均已切到同一 hash：`423ab45ad2a49606ff89f2551f950b7d3f4de817228c76eb7f57e69702e7e92a`

### 结果

- 短窗 `./.tmp/p2p_real_env_triad/20260409-201221/summary.md`：
  - observer：`2 -> 5`
  - sequencer：`2 -> 5`
  - storage：`2 -> 5`
  - `claim_status=pass_candidate`
  - `failure_signatures=(none)`
- 中窗 `./.tmp/p2p_real_env_triad/20260409-201319/summary.md`：
  - observer：`7 -> 17`
  - sequencer：`7 -> 17`
  - storage：`7 -> 17`
  - 三端 `last_error` 仍为 `(none)`
- 这次最关键的变化是：sequencer 已明确越过上一版 warmup binary 的 `8 -> 13` 停点，说明“warmup 且无 peer head 时直接跳过 network probe”这条修正命中了当前 startup-ordering 残留。

### 现场核对

- 中窗结束后即刻读取链状态，三端仍继续推进：
  - observer：`committed_height=19`、`network_committed_height=19`、`known_peer_heads=2`
  - sequencer：`committed_height=19`、`network_committed_height=19`、`known_peer_heads=0`
  - storage：`committed_height=20`、`network_committed_height=20`、`known_peer_heads=1`
- 继续拉高到 `height=35` 后，三端仍共同推进；现场核对表明这个 `known_peer_heads=0` 更像当前角色设计的结果，而不是新的故障：
  - `oasis7_chain_runtime` 会对 `NodeRole::Sequencer` 启用 `require_peer_execution_hashes`
  - `validate_peer_commit_execution_binding()` 会拒绝缺少 execution hashes 的 peer commit
  - 当前 triad 中只有 sequencer 具备 execution side，observer/storage 不具备，因此 sequencer 不记它们的 `peer_heads` 是可解释现象；observer/storage 反而仍能继续记录来自 sequencer 的 peer head

### 更新结论

- 这次补丁已经把当前三节点真实环境里的主 stall 从“高度 `13/14` 左右的启动 gate 误杀”移除掉。
- 现阶段更值得继续追的不是再放宽 warmup，也不是把 `sequencer known_peer_heads=0` 直接当成 bug，而是继续看更长时间窗下是否还会出现新的后续停点。

## 2026-04-09 peer-headless single-match 追加验证

### 上一版长窗暴露的新停点

- 在 `7401ef56-storage-gate-peer-head-warmup-skip-20260409` 上继续做长窗后，`./.tmp/p2p_real_env_triad/20260409-202451/summary.md` 显示：
  - observer：`64 -> 68`
  - sequencer：`64 -> 75`
  - storage：`65 -> 95`
- 新 stall 已收敛到更具体的一层：
  - sequencer `last_error` 变成 `storage challenge gate network threshold unmet: samples=6 required_matches=2 successful_matches=1`
  - 同窗还伴随 `fetch-blob no connected peers` 与 `gap sync height 76 ... fetch-commit no connected peers`
- 这说明上一版虽然打掉了 `13/14` 停点，但在更高高度上仍被“只拿到 1 个远端样本、却要求 2 个 match”的 gate 卡住。

### 补丁内容

- 已把 network gate 再收窄一层：
  - 当节点启用了 `require_peer_execution_hashes`
  - 且 `peer_heads` 仍为空
  - 则 network sample 的 `required_matches` 收为 `1`
- 这个条件只作用在当前这类 sequencer 拓扑，不影响 observer / storage 的普通无 peer-head 场景。
- 对应 release：
  - `7401ef56-storage-gate-peer-headless-single-match-20260409`
  - `sha256=6acd6fad174ea07be42bc19dbe24bc0f567faf28b36253100ba0811e303d29d0`

### rollout 与清态

- rollout 后再次统一清态，只保留身份配置：
  - observer：`/opt/oasis7/p2p-triad-local/backups/config-20260409-205259-storage-gate-peer-headless-single-match-reset.tgz`
  - sequencer：`/opt/oasis7/p2p-triad/backups/config-20260409-205259-storage-gate-peer-headless-single-match-reset-sequencer.tgz`
  - storage：`/opt/oasis7/p2p-triad/backups/config-20260409-205259-storage-gate-peer-headless-single-match-reset-storage.tgz`

### 结果

- 短窗 `./.tmp/p2p_real_env_triad/20260409-205400/summary.md`：
  - observer：`2 -> 4`
  - sequencer：`2 -> 5`
  - storage：`2 -> 5`
  - `claim_status=pass_candidate`
  - `failure_signatures=(none)`
- 长窗 `./.tmp/p2p_real_env_triad/20260409-205457/summary.md`：
  - observer：`6 -> 36`
  - sequencer：`6 -> 36`
  - storage：`7 -> 37`
  - 整窗三端 `last_error=(none)`
- 最关键的变化是：上一版在 `height≈75` 暴露的 sequencer stall，这一版在等长窗口内没有再出现。

### 现场核对

- 长窗结束后立即读取链状态，三端仍继续一起前进：
  - observer：`h=38`、`nh=38`、`last_error=null`
  - sequencer：`h=39`、`nh=39`、`last_error=null`
  - storage：`h=39`、`nh=39`、`last_error=null`
- `sequencer known_peer_heads=0` 仍保持不变，但在这版 evidence 中继续没有演化成新阻断。

### 更新结论

- 当前这版已经把已知的两层真实环境 stall 都压掉了：
  - 启动早期 `13/14` 左右的 warmup gate 误杀
  - 更高高度 `≈75` 的 peer-headless `required_matches=2` 卡死
- 下一步最合理的动作不再是继续拍脑袋放宽 gate，而是做更长的 soak，确认 triad 是否能在更高高度继续稳定推进。
