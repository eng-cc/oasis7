# P2P Real-Environment Triad Reconfirm (2026-04-08)

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
  - `task_21d194cd65564fd0967f64f847cefd3d`
- snapshot run dir:
  - `.tmp/p2p_real_env_triad/20260408-120134`
- snapshot summary:
  - `.tmp/p2p_real_env_triad/20260408-120134/summary.json`

## 本轮目标
1. 用带远端认证的 same-window triad snapshot，重新确认当前 `1` 本机 observer + `2` 阿里云 ECS 的真实状态。
2. 判断 `P2PARCH-6` 当前真实环境 blocker 是否仍是 observer 接入问题，还是已经进一步收敛到云端某一侧的本地执行态。
3. 若 blocker 已足够明确且能在仓库内补验证，则顺手把对应 runtime/脚本收敛路径补齐，避免 summary 或本地回归继续失真。

## 执行命令
```bash
P2PARCH6_SEQ_SSH_PASSWORD='***' \
P2PARCH6_STORAGE_SSH_PASSWORD='***' \
./scripts/p2p-real-env-triad-snapshot.sh \
  --samples 3 \
  --interval-secs 4 \
  --out-dir .tmp/p2p_real_env_triad

env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime \
  node_runtime_execution_driver_reconciles_stale_state_from_exact_record \
  -- --nocapture
```

## same-window triad 样本
### 本机 `triad-observer-local`
- service / health:
  - `active`
  - `healthz_all_ok=true`
  - `status_fetch_all_ok=true`
- 3 个样本:
  - `committed_height=7004 -> 7019`
  - `network_committed_height=61902 -> 61904`
  - `known_peer_heads=1 -> 1`
  - `last_error=node replication error: gap sync height 7005 failed after 3 attempts: attempt 3/3 failed: node replication error: gap sync height 7005 blob not found for hash 6bb1c1d04872ae00b214c0d00701488de60911c269951adeb5239f1787849143`
- 结论:
  - observer 当前已经稳定越过 `known_peer_heads=0 / network_committed_height=0` 阶段，并继续向云端高位推进 commit；本轮虽然又观测到间歇性的 gap-sync `blob not found`，但它没有重新成为 same-window triad 的主 blocker，因为 observer 在同窗内仍保持 `known_peer_heads=1` 且 committed height 持续推进。

### ECS `triad-storage-b`
- service / health:
  - `active`
  - `healthz_all_ok=true`
  - `status_fetch_all_ok=true`
- 3 个样本:
  - `committed_height=61903 -> 61904`
  - `network_committed_height=61903 -> 61904`
  - `known_peer_heads=0 -> 0`
  - `last_error=null`
- 结论:
  - storage 仍保持真实云端高位，并在本轮窗口内继续推进；云端至少一侧链路与状态可见性正常。

### ECS `triad-sequencer-a`
- service / health:
  - `active`
  - `healthz_all_ok=true`
  - `status_fetch_all_ok=true`
- 3 个样本:
  - `committed_height=0 -> 0`
  - `network_committed_height=0 -> 0`
  - `known_peer_heads=0 -> 0`
  - `last_error=node execution error: execution driver received stale height: context=57536 state=57560`
- 结论:
  - 当前 same-window triad 的阻断点已经明确收敛到 sequencer 本地执行态，而不是 observer 反向建链失败，也不是“云端整体不可见”。

## 脚本与 runtime 收敛
1. `scripts/p2p-real-env-triad-snapshot.sh` 现在会把 real-env blocker 精确写成：
   - `sequencer_committed_height_zero`
   - `sequencer_execution_stale_height`
   而不再把“storage 正常、sequencer 零高”误写成 `cloud_pair_chain_not_visible`。
2. `oasis7_chain_runtime` 的 execution bridge 现在在收到比本地 state 更旧的 committed height 时，会优先尝试从现有 execution record 恢复到目标高度，而不是直接卡死在 stale-height 报错。
3. 新增定向回归 `node_runtime_execution_driver_reconciles_stale_state_from_exact_record`，固定“state file 比 exact record 更新、但 consensus 回到旧高度”时的恢复语义。

## 验证结果
1. same-window triad snapshot 已成功采到本机 observer、ECS storage、ECS sequencer 三端 live sample，产物位于 `.tmp/p2p_real_env_triad/20260408-114909/`。
2. `summary.json` 当前为 `claim_status=blocked`，failure signature 已精确到 `sequencer_committed_height_zero + sequencer_execution_stale_height`，不再误导成云端双节点整体不可见；observer 的间歇性 gap-sync `blob not found` 继续作为次级 residual 留在样本明细里，而不覆盖本轮主 blocker。
3. `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime node_runtime_execution_driver_reconciles_stale_state_from_exact_record -- --nocapture` 通过。

## 对 P2PARCH-6 的意义
1. 当前 `1` 本机 + `2` ECS 真实环境已经满足“可作为 same-window mixed-topology 真样本继续追 blocker”的最低条件，但仍不能被写成 shared-network `pass`。
2. observer 侧 blocker 已经从 `peer_heads_zero` 进一步收敛；现在真实环境里最值得追的 residual 是 sequencer 的 execution bridge stale-height 收口与远端部署复核，而不是再回到 observer peering 基础面重复排查。
3. 本轮仓库内修复只证明 execution bridge 已具备从 exact execution record 回收 stale state 的能力；是否能彻底清掉 ECS sequencer 当前 residual，还需要后续把新二进制部署到 `triad-sequencer-a` 后再做同窗复采。
