# P2P Real-Environment Triad Three Equal Validator Rollout (2026-04-29)

审计轮次: 1

## Meta
- 责任角色:
  - `runtime_engineer`
- 协作角色:
  - `qa_engineer`
- 当前结论:
  - `pass_candidate`
- claim status:
  - `pass_candidate`
- claim mode:
  - `three_equal_validator`
- world:
  - `shared-devnet-ecs-v1`
- related task:
  - `task_a9536ec4810d411da78d30a1522b5a5e`
- runtime commit:
  - `3f43ef911`
- runtime sha256:
  - `9722c25ab2a58af17a439d4baf7872713dc5ddbc3fb08300ff84bbf96c78c26f`
- deployed release dir:
  - `3f43ef911-clock-restore-clamp-20260505-232617`
- snapshot run dir:
  - `.tmp/p2p_real_env_triad/20260506-100041`
- snapshot summary:
  - `.tmp/p2p_real_env_triad/20260506-100041/summary.json`
- 最终快照时间:
  - `2026-05-06 10:01:11 CST`

## 本轮目标
1. 把本机 `triad-observer-local`、ECS `triad-sequencer-a`、ECS `triad-storage-b` 真正收口为 `3` 个等权 `sequencer` validator，而不是继续停留在 mixed-topology 历史残留。
2. 修复此前阻断 same-window triad 的恢复/复制问题，包括 future-slot 持久态污染、非 proposer 本地 replication commit 分叉，以及近同时启动时的 writer epoch 冲突。
3. 留下正式 same-window triad evidence，证明三节点在同一窗口内都以 `three_equal_validator` 模式推进且没有 `last_error`。

## 本轮修复与运维动作
1. 代码修复:
  - `crates/oasis7_node/src/pos_state_store.rs`: 固定 genesis 恢复时按当前墙钟裁剪持久化 `next_slot/last_observed_slot/last_observed_tick`，避免节点重启后继续带着超前 POS 状态运行。
  - `crates/oasis7_node/src/lib.rs`: 启动恢复 `node_pos_state.json` 时传入当前时间，让上述裁剪逻辑在真实 runtime 生效。
  - `crates/oasis7_node/src/tests_pos_engine_guardrails.rs`: 新增 `restore_state_snapshot_clamps_future_clock_state_when_fixed_genesis_is_configured`，并保留重启/时钟单调回归。
2. 已验证回归:
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_node restore_state_snapshot_clamps_future_clock_state_when_fixed_genesis_is_configured`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_node runtime_restart_reconciles_stale_pos_state_from_persisted_replication_height`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_node runtime_pos_state_persists_across_restart`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_node pos_engine_observed_slot_does_not_backtrack_on_clock_rewind`
  - `env -u RUSTC_WRAPPER cargo check -p oasis7_node`
3. 发布与切换:
  - 本机 `current` 切到 `/opt/oasis7/p2p-triad-local/releases/3f43ef911-clock-restore-clamp-20260505-232617`
  - 两台 ECS `current` 切到 `/opt/oasis7/p2p-triad/releases/3f43ef911-clock-restore-clamp-20260505-232617`
  - 三端 `current/bin/oasis7_chain_runtime` 统一为 `sha256=9722c25ab2a58af17a439d4baf7872713dc5ddbc3fb08300ff84bbf96c78c26f`
4. 协调冷重置:
  - 本机旧状态转存到 `/opt/oasis7/p2p-triad-local/backups/20260505-234000-cold-reset`
  - ECS sequencer 旧状态转存到 `/opt/oasis7/p2p-triad/backups/20260505-234000-cold-reset-sequencer`
  - ECS storage 旧状态转存到 `/opt/oasis7/p2p-triad/backups/20260505-234000-cold-reset-storage`
  - 仅转移 `execution-world`、`execution-records`、`storage`、对应节点的 `output/node-distfs` 与 `output/chain-runtime`；config / keypair / env / release 未改写。

## same-window triad 样本
### 本机 `triad-observer-local`
- service / health:
  - `active`
  - `healthz_all_ok=true`
  - `status_fetch_all_ok=true`
- 4 个样本:
  - `committed_height=6 -> 8`
  - `network_committed_height=6 -> 8`
  - `known_peer_heads=1 -> 1`
  - `last_error=null`
- 结论:
  - 本机节点已经不再是历史 `observer` 阻断点，而是以 `sequencer` 身份和 cloud pair 一起推进。

### ECS `triad-sequencer-a`
- service / health:
  - `active`
  - `healthz_all_ok=true`
  - `status_fetch_all_ok=true`
- 4 个样本:
  - `committed_height=7 -> 9`
  - `network_committed_height=7 -> 9`
  - `known_peer_heads=2 -> 2`
  - `last_error=null`
- 结论:
  - `seq-a` 在窗口内同时看见另外两台 validator，并维持稳定推进。

### ECS `triad-storage-b`
- service / health:
  - `active`
  - `healthz_all_ok=true`
  - `status_fetch_all_ok=true`
- 4 个样本:
  - `committed_height=7 -> 9`
  - `network_committed_height=7 -> 9`
  - `known_peer_heads=1 -> 1`
  - `last_error=null`
- 结论:
  - `storage-b` 从此前的旧链头残留恢复为和另外两台同窗推进，不再保留 `1022` 高度的单机孤岛状态。

## 结论
1. 真实环境 triad 当前已经达到 `claim_status=pass_candidate`，且 claim mode 明确为 `three_equal_validator`。
2. 三节点在同一窗口内都报告 `role=sequencer`、`service=active`、`last_error=null`，并从 `6/7` 稳定推进到 `8/9`。
3. 本轮成功依赖于两层收口同时成立:
  - 代码层保证 fixed-genesis 恢复不会继续复活超前 POS 状态。
  - 运维层把此前已经分叉的 execution / replication 持久态整体转移出 live 根目录，让三节点从同一 genesis 重新建链。

## 边界
1. 本轮 evidence 证明的是“保留配置与身份绑定前提下，清空运行态后，三节点等权 validator 可以重新建出一致 live 链并同窗推进”，不等于“旧历史链状态已经无损迁移恢复”。
2. 当前结论仍是 `pass_candidate`，不是更高等级的 shared-network / multi-operator / ASN-diverse 正式放行。
3. 后续若继续在这组节点上推进新版本，应优先沿用本轮 release + cold-reset + same-window snapshot 的闭环，而不是在旧 execution/replication 状态上重复叠加试验。
