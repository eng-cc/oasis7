# P2P Real-Environment Triad Current-Version Full-Game Nodes (2026-05-16)

审计轮次: 3

## Meta
- 责任角色:
  - `runtime_engineer`
- 当前结论:
  - `pass_candidate`
- claim status:
  - `pass_candidate`
- claim mode:
  - `three_equal_validator`
- world:
  - `shared-devnet-ecs-v1`
- runtime commit:
  - `d104864026bb + local replication cursor fix`
- runtime release:
  - `d104864026bb-triad-full-game-nodes-20260517-patched-replication-cursor`
- runtime sha256:
  - `268c8d22cc8a0599239c3d9f84433daa99e78d6bab16166a9e6c2ad1befc72a4`
- snapshot run dir:
  - `.tmp/p2p_real_env_triad_sync_reset/20260516-214745`
- snapshot summary:
  - `.tmp/p2p_real_env_triad_sync_reset/20260516-214745/summary.json`
- follow-up reset summary:
  - `.tmp/p2p_real_env_triad_post_reset/20260517-145305/summary.md`

## 本轮目标
1. 用当前仓库版本升级本机 + 两台 ECS 三节点。
2. 让三节点都作为“完整游戏节点”运行，而不是只让本机跑 viewer/player-entry、云端只跑链。
3. 让三节点在 same-window 内恢复到 `three_equal_validator` 拓扑并留下正式快照。

## 本轮动作
1. 构建当前 release：
  - `env -u RUSTC_WRAPPER cargo build --release -p oasis7 --bin oasis7_chain_runtime`
  - `env -u RUSTC_WRAPPER cargo build --release -p oasis7 --bin oasis7_viewer_live`
2. 审计轮次 1 首轮统一 release：
  - 本机 `/opt/oasis7/p2p-triad-local/current`
  - ECS `/opt/oasis7/p2p-triad/current`
  - 当时三端 `oasis7_chain_runtime` 统一为 `sha256=a7f5f8d44aee4e8c0384ac3d63f3528bcadce2ae28c03976197d07ea281c3f8a`
3. 本机完整节点补件：
  - 由于本机 `PLAYER_ENTRY_ENABLE=1`，新 release 除 `oasis7_chain_runtime` 外还需补齐 `oasis7_viewer_live` 与 `web/` 静态目录；本轮已把旧 full-stack release 的 `web/` 复制到新 release，并放入新编译的 `oasis7_viewer_live`。
4. 冲突排查：
  - 本机原 `PLAYER_ENTRY_VIEWER_BIND=127.0.0.1:5023` 与 `PLAYER_ENTRY_WEB_BIND=127.0.0.1:5011` 被另一套本地 `oasis7_game_launcher/oasis7_viewer_live` 占用。
  - 为避免冲突，本机 triad 当前临时改到 `5123/5111/4273`。
5. 冷重置与同步重启：
  - 第一次单点恢复后，本地出现 `BlobNotFound` / `peer commit execution mismatch`，说明 cloud pair 已前进时再让 local 加入会复现 execution mismatch。
  - 因此对三节点统一执行第二轮同步冷重置，把 live `execution-world`、`execution-records`、`storage`、`output/node-distfs/<node>`、`output/chain-runtime/<node>` 迁出到：
    - 本机：`/opt/oasis7/p2p-triad-local/backups/20260516-214642-triad-synchronous-reset-local`
    - ECS sequencer：`/opt/oasis7/p2p-triad/backups/20260516-214642-triad-synchronous-reset-sequencer`
    - ECS storage：`/opt/oasis7/p2p-triad/backups/20260516-214642-triad-synchronous-reset-storage`
  - 随后同步启动三端 systemd 服务。

## 审计轮次 1：2026-05-16 same-window triad 结果
### 本机 `triad-observer-local`
- service / health:
  - `active`
  - `healthz_all_ok=true`
  - `status_fetch_all_ok=true`
- 4 个样本:
  - `committed_height=1 -> 2`
  - `network_committed_height=1 -> 2`
  - `known_peer_heads=1 -> 1`
  - `last_error=null`
- 额外说明:
  - 本机完整游戏节点网页入口当前可通过 `http://127.0.0.1:4273/` 访问。

### ECS `triad-sequencer-a`
- service / health:
  - `active`
  - `healthz_all_ok=true`
  - `status_fetch_all_ok=true`
- 4 个样本:
  - `committed_height=1 -> 2`
  - `network_committed_height=1 -> 2`
  - `known_peer_heads=1 -> 2`
  - `last_error=null`

### ECS `triad-storage-b`
- service / health:
  - `active`
  - `healthz_all_ok=true`
  - `status_fetch_all_ok=true`
- 4 个样本:
  - `committed_height=1 -> 3`
  - `network_committed_height=1 -> 3`
  - `known_peer_heads=1 -> 1`
  - `last_error=null`

## 审计轮次 2：2026-05-17 根因复核与同步冷重置
### 先前卡死根因
1. local 与 storage 旧故障窗口都曾停在 `committed_height=435`，sequencer 已独自推进到 `1807+`。
2. 两个坏节点的关键矛盾一致：
  - `execution-records/latest.json` 停在 `435`
  - `execution-records/436.json` 缺失
  - 当前 `execution-world/snapshot.manifest.json` 却已指向 `epoch=436`
3. 同一时刻 sequencer 真值 `436` 的 `state_root` 与坏节点 `execution-world` 中的 `436` 不同，说明坏节点并不是“正常落后一块”，而是 execution world 已写进错误快照、execution records 却没有对应前驱记录。
4. 运行时代码在处理非连续 committed height 时，要求从前驱 `execution-records/<height>.json` 恢复 bridge；当前驱记录缺失时会直接报：
  - `execution driver missing predecessor record for non-contiguous committed height`
5. 因而这次故障不属于普通同步延迟，而是 execution 持久态分叉后进入硬卡死。

### 2026-05-17 同步冷重置动作
1. 停掉三端服务：
  - 本机：`oasis7-triad-observer.service`
  - ECS sequencer：`oasis7-triad-sequencer.service`
  - ECS storage：`oasis7-triad-storage.service`
2. 备份并迁出 live 运行态到：
  - 本机：`/opt/oasis7/p2p-triad-local/backups/20260517-144932-triad-synchronous-reset-local`
  - ECS sequencer：`/opt/oasis7/p2p-triad/backups/20260517-144932-triad-synchronous-reset-sequencer`
  - ECS storage：`/opt/oasis7/p2p-triad/backups/20260517-144932-triad-synchronous-reset-storage`
3. 迁出的 live 内容包括：
  - `data/execution-world`
  - `data/execution-world-simulator-mirror`
  - `data/execution-records`
  - `data/storage`
  - `output/node-distfs/<node>`
  - `output/chain-runtime/<node>`
4. 三端重启后，本机 player-entry 已恢复默认端口 `4173/5011/5023/5633`。

### 2026-05-17 冷重置后短窗恢复
1. 直连 status 采样显示，冷重置后短窗曾恢复正常：
  - 第一轮：local / sequencer / storage 都到 `committed_height=10`，且 `last_error=null`
  - 第二轮：local=`13`、sequencer=`13`、storage=`12`，storage 仍在追高但无错误
  - 第三轮：local=`14`、storage=`14`，已重新追平且 `last_error=null`
2. 这一阶段说明同步冷重置确实能暂时把 triad 拉回同一条新链。

### 2026-05-17 18:06 CST 再复核结果
1. 继续半天后再次用直连 `curl /v1/chain/status` 复核，triad 已复发同类故障：
  - 本机 `triad-observer-local`
    - `committed_height=617`
    - `network_committed_height=617`
    - `last_execution_height=617`
    - `last_error=node execution error: execution driver missing predecessor record for non-contiguous committed height: last_applied=617 incoming=642 predecessor=641`
  - ECS `triad-sequencer-a`
    - `committed_height=753`
    - `network_committed_height=753`
    - `last_execution_height=753`
    - `last_error=null`
  - ECS `triad-storage-b`
    - `committed_height=617`
    - `network_committed_height=618`
    - `last_execution_height=617`
    - `last_error=node execution error: execution driver missing predecessor record for non-contiguous committed height: last_applied=617 incoming=753 predecessor=752`
2. 这证明 2026-05-17 的同步冷重置只带来了阶段性恢复，local + storage 在链继续前进后又回到了同类缺前驱记录卡死状态。
3. 因此当前 verdict 必须从审计轮次 1 的 `pass_candidate` 回退为 `blocked`。

### 观测边界
1. `.tmp/p2p_real_env_triad_post_reset/20260517-145305/summary.md` 所对应的 repo snapshot 脚本仍会误报云节点不可达；该脚本不应作为 2026-05-17 云节点健康判断的唯一依据。
2. 本轮云端状态以直连 SSH + `curl http://127.0.0.1:5631/5632/v1/chain/status` 为准。

## 审计轮次 3：2026-05-17 patched replication cursor rollout
### 根因修补
1. 进一步对照 live 故障与代码路径后，确认 `gap-sync` / `successor-probe` 存在同一类游标推进缺陷：
  - `crates/oasis7_node/src/node_engine_replication.rs`
  - `crates/oasis7_node/src/replication_probe_gate.rs`
2. 原问题是节点在 `apply_synced_replication_commit(...)` 成功前，就先把 `replication_persisted_height` 推到目标高度；一旦该高度的 execution hook 失败，下一轮会跳过该高度继续向上，最终把节点锁死在：
  - `execution driver missing predecessor record for non-contiguous committed height`
3. 本轮修补将游标推进改成“只有 execution commit 成功后才推进 persisted height”，避免 execution 失败后把缺口高度永久跳过。

### 回归验证
1. 已通过：
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_node successor_probe_does_not_advance_replication_cursor_when_execution_fails`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_node gap_sync_does_not_advance_replication_cursor_when_execution_fails`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_node runtime_network_replication_gap_sync_fetches_missing_commits`
2. 两条新增回归测试分别锁住 successor probe / gap sync 在 execution 失败时不会错误推进 replication cursor。

### patched rollout 与冷重置
1. patched binary：
  - `target/release/oasis7_chain_runtime`
  - `sha256=268c8d22cc8a0599239c3d9f84433daa99e78d6bab16166a9e6c2ad1befc72a4`
2. rollout 目标：
  - 本机 `/opt/oasis7/p2p-triad-local/current/bin/oasis7_chain_runtime`
  - ECS sequencer `/opt/oasis7/p2p-triad/current/bin/oasis7_chain_runtime`
  - ECS storage `/opt/oasis7/p2p-triad/current/bin/oasis7_chain_runtime`
3. rollout 后再次同步冷重置三端，新增备份目录：
  - 本机：`/opt/oasis7/p2p-triad-local/backups/20260517-183959-triad-patched-reset-local`
  - ECS sequencer：`/opt/oasis7/p2p-triad/backups/20260517-183959-triad-patched-reset-sequencer`
  - ECS storage：`/opt/oasis7/p2p-triad/backups/20260517-183959-triad-patched-reset-storage`

### 2026-05-17 18:39-18:53 CST patched rollout 复核
1. rollout 后首轮 4 个 same-window 样本都保持三端等高、无 execution 错误：
  - 第一轮：`2 / 2 / 2`
  - 第二轮：`4 / 4 / 4`
  - 第三轮：`7 / 7 / 7`
  - 第四轮：`12 / 12 / 12`
2. 2026-05-17 18:53 CST 再次用直连 `curl /v1/chain/status` 复核，三端继续推进到：
  - 本机 `triad-observer-local`: `committed_height=57`, `network_committed_height=57`, `last_execution_height=57`, `last_error=null`
  - ECS `triad-sequencer-a`: `committed_height=57`, `network_committed_height=57`, `last_execution_height=57`, `last_error=null`
  - ECS `triad-storage-b`: `committed_height=57`, `network_committed_height=57`, `last_execution_height=57`, `last_error=null`
3. 该时间点三端都保持：
  - `committed_height == network_committed_height == last_execution_height`
  - 没有再次出现 `missing predecessor record`
4. status 中仍可见 `replication_recent_errors` 观测告警，但当前没有形成高度落后或 execution 卡死；本轮 claim 以 committed/execution 等高和 `last_error=null` 为准。

## 更新结论
1. 当前版本已经完成三节点实网升级，且“三节点都带完整游戏节点构件”的部署目标已满足。
2. 先前导致 local + storage 长窗复发卡死的 replication cursor 根因已经在代码层确认并修补，且已补上针对 execution-failure 场景的回归测试。
3. 截至 2026-05-17 18:53 CST，patched rollout 后的真实环境 triad 已连续通过：
  - 首轮 same-window `2/4/7/12`
  - 后续继续推进到 `57/57/57`
  - 三端 `last_execution_height` 与 `committed_height` 保持一致，`last_error=null`
4. 因此本轮当前 claim status 可以恢复为 `pass_candidate`，但它仍只覆盖 patched rollout 之后的当前观察窗，不应外推为“长窗稳定性已完全证明”。
5. 后续若要把结论升级为长期稳定通过，仍需继续观察是否会再次复发此前 `617 -> 619` 跳高后的缺前驱记录故障。

## 边界
1. 审计轮次 1 中的 `pass_candidate` 只是冷重置后短窗证据，不能单独引用为“已长期稳定”的证明。
2. 2026-05-17 当前阻断面已经收敛到 execution 持久态一致性；若后续要保留旧 live 历史状态继续做“无冷重置升级”，需要单独追 execution/replication 历史迁移问题，不应混入本轮升级结论。
