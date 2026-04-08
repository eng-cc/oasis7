# P2P Real-Environment Triad Stale-Height Rollout (2026-04-08)

审计轮次: 1

## Meta
- 责任角色:
  - `runtime_engineer`
- 协作角色:
  - `qa_engineer`
- 当前结论:
  - `blocked`
- claim status:
  - `blocked`
- world:
  - `shared-devnet-ecs-v1`
- related task:
  - `task_74e8ba671d0f481ebb4e38eefbce9390`
- rollout commit:
  - `f8b1baf97316`
- fresh build sha256:
  - `72a6008f24b85e3b8e223db2e141688c2d10cd58cff578c1550e2028796d7aa7`
- snapshot run dir:
  - `.tmp/p2p_real_env_triad/20260408-132008`
- snapshot summary:
  - `.tmp/p2p_real_env_triad/20260408-132008/summary.json`

## 本轮目标
1. 将 `oasis7_chain_runtime` 里已落地的 execution bridge stale-height 恢复修复滚到真实 ECS sequencer/storage。
2. 用 same-window triad snapshot 复核真实三节点 blocker 是否还停留在 `execution driver received stale height`。
3. 若 stale-height 已被清掉，则把新的 residual 签名与当前 triad 真值固定到正式 evidence / project / task log。

## 执行命令
```bash
env -u RUSTC_WRAPPER cargo build --release -p oasis7 --bin oasis7_chain_runtime

P2PARCH6_SEQ_SSH_PASSWORD='***' \
P2PARCH6_STORAGE_SSH_PASSWORD='***' \
./scripts/p2p-real-env-triad-snapshot.sh \
  --samples 3 \
  --interval-secs 4 \
  --out-dir .tmp/p2p_real_env_triad
```

## 部署结果
1. 当前 worktree `HEAD=f8b1baf97316` 上 fresh build 产出的 release binary `sha256=72a6008f24b85e3b8e223db2e141688c2d10cd58cff578c1550e2028796d7aa7`，并显式支持 `--replication-network-listen`、`--replication-network-peer` 与 `--replication-remote-writer-public-key` CLI。
2. ECS `triad-sequencer-a` 与 `triad-storage-b` 已统一切到 `/opt/oasis7/p2p-triad/releases/f8b1baf97316-stale-height-rollout-20260408`，二者当前 `current/bin/oasis7_chain_runtime` 的 `sha256` 都是 `72a6008f24b85e3b8e223db2e141688c2d10cd58cff578c1550e2028796d7aa7`。
3. 本机 `triad-observer-local` 本轮未同步滚到同版二进制，仍停在 `/opt/oasis7/p2p-triad-local/releases/89860f6eb6d5-observer-seed-signer-20260407-223911`，当前 `sha256=004aaf7529a4c1e26be5150aaf87ac4b648e241f29295b2dc23824d516ea4785`；因此本轮 evidence 的作用是“验证 cloud-side stale-height rollout 是否改变 blocker”，而不是宣称 triad 已重新实现版本完全一致。

## same-window triad 样本
### 本机 `triad-observer-local`
- service / health:
  - `active`
  - `healthz_all_ok=true`
  - `status_fetch_all_ok=true`
- 3 个样本:
  - `committed_height=9383 -> 9383`
  - `network_committed_height=62289 -> 62289`
  - `known_peer_heads=1 -> 1`
  - `last_error=node replication error: gap sync height 9384 failed after 3 attempts: attempt 3/3 failed: node replication error: gap sync height 9384 blob not found for hash 9d63bf80dfef6ab0a3d5002e534f436f26a00b60e3d719ffa0836286ccea9f22`
- 结论:
  - observer 仍能稳定看到 `known_peer_heads=1`，说明这轮 real-env 主 blocker 不是重新退回 observer peering 失联；但 observer 本地 committed height 在当前窗口内没有继续推进，gap-sync `blob not found` 继续作为次级 residual 存在。

### ECS `triad-sequencer-a`
- service / health:
  - `active`
  - `healthz_all_ok=true`
  - `status_fetch_all_ok=true`
- 3 个样本:
  - `committed_height=0 -> 0`
  - `network_committed_height=0 -> 0`
  - `known_peer_heads=0 -> 0`
  - `last_error=node consensus error: storage challenge gate network threshold unmet: samples=3 required_matches=2 successful_matches=0 reasons=["storage challenge gate network blob not found for hash 2e78104fa88c24d4eca5c49c49f0591a473e62d2c4822d8a667ed68d5d726dba", "storage challenge gate network blob not found for hash d739c1c9d74067ee21a92404371d61deeaf387843018899393df5d8d20cfbe8c", "storage challenge gate network blob not found for hash 0a5f27facd9e9ea28a2db2240b7315e4a553d95b59b75dba88e7c13c682b7be9"]`
- 结论:
  - `execution driver received stale height: context=57536 state=57560` 本轮已经不再出现，说明 stale-height 修复至少在真实 ECS sequencer 的重启恢复路径上成功接管。
  - 当前新 blocker 已切换成 `storage challenge gate network threshold unmet`；sequencer 仍维持 `committed_height=0 / network_committed_height=0`，所以 `P2PARCH-6` real-env triad 仍然不能 uplift 成 `pass`。

### ECS `triad-storage-b`
- service / health:
  - `active`
  - `healthz_all_ok=true`
  - `status_fetch_all_ok=true`
- 3 个样本:
  - `committed_height=62291 -> 62293`
  - `network_committed_height=62291 -> 62293`
  - `known_peer_heads=0 -> 1`
  - `last_error=null`
- 结论:
  - storage 继续保持高位推进，并在窗口内重新看到 peer head；因此 cloud 侧并不是整体不可见，而是 sequencer 单点卡在新的 consensus/storage challenge gate 上。

## 验证结果
1. same-window triad snapshot 已成功采到三端 live sample，产物位于 `.tmp/p2p_real_env_triad/20260408-132008/`。
2. `summary.json` 当前仍为 `claim_status=blocked`，但主 failure signatures 已从上一轮的 `sequencer_execution_stale_height` 变成 `sequencer_committed_height_zero + observer_committed_height_not_advancing`。
3. sequencer 当前最新 `last_error` 已精确收敛为 `storage challenge gate network threshold unmet`，说明 stale-height rollout 已把真实环境 blocker 往前推进了一步，而不是原地复现旧签名。

## 对 P2PARCH-6 的意义
1. 当前 `1` 本机 + `2` ECS 环境仍然只足够作为 real-env same-window partial truth，不能替代 full mixed-topology gate。
2. execution bridge stale-height 修复已经在真实云端 sequencer 上完成一次有效 rollout 验证；后续不必再把 `execution driver received stale height` 当作当前首要 residual。
3. 下一步更值得追的真实环境问题，已经从 sequencer 本地 execution state 恢复转移到 `storage challenge gate network threshold unmet`，以及 observer 侧仍未清掉的 gap-sync `blob not found`。
