# task_9dab2aa9246b465c805f5e59ffb08cba Execution Log

- task_uid: task_9dab2aa9246b465c805f5e59ffb08cba
- title: Close triad fetch-commit startup ordering gap
- owner_role: runtime_engineer
- worktree_hint: oasis7-p2p-p2parch-6-fetch-commit-startup-ordering

<!-- Append entries using:
Example:
  ## YYYY-MM-DD HH:MM:SS CST / role_name
  - 完成内容: ...
  - 遗留事项: ...
-->

## 2026-04-08 22:12:30 CST / runtime_engineer
- 完成内容: 复盘 startup-ordering residual 后，确认当前主要风险不再是 `fetch-commit` handler 缺失，而是 `libp2p_replication_network` 会把启动窗口里命中的 `unsupported` 判定永久记入 `unsupported_protocol_peers`，导致 storage 即便后续已连上真正可用的 sequencer，也可能长期继续报 `libp2p-replication no connected peers for protocol /aw/node/replication/fetch-commit/1.0.0`。
- 完成内容: 已在 `crates/oasis7_node/src/libp2p_replication_network.rs` 把 unsupported peer 追踪从永久集合改为带重试窗口的临时 quarantine：每次命中 unsupported 只会在限定窗口内跳过该 peer，窗口过后允许自动重探，避免启动瞬时误判把单个 fetch 源永久毒化。
- 完成内容: 已新增定向回归 `filtered_request_peers_retries_unsupported_peer_after_retry_window` 与 `libp2p_replication_network_retries_previously_unsupported_single_peer_after_retry_window`，并复跑 `filtered_request_peers_excludes_known_unsupported_peers_without_fallback`，确认“先临时跳过、后自动恢复重探”的语义成立，同时不回退到立即兜底所有 unsupported peer 的旧行为。
- 完成内容: 当前本地已完成 `env -u RUSTC_WRAPPER cargo test -p oasis7_node filtered_request_peers_retries_unsupported_peer_after_retry_window`、`env -u RUSTC_WRAPPER cargo test -p oasis7_node libp2p_replication_network_retries_previously_unsupported_single_peer_after_retry_window`、`env -u RUSTC_WRAPPER cargo test -p oasis7_node filtered_request_peers_excludes_known_unsupported_peers_without_fallback`。
- 遗留事项: 还未 rollout 到现有 `1` 本机 + `2` ECS 三节点真实环境；下一步需要 build / deploy / same-window 复采，确认 storage 是否能在早期 `no connected peers` 窗口后自动恢复 `fetch-commit`，以及 retry window 是否需要继续按实机时序调参。

## 2026-04-08 22:43:30 CST / runtime_engineer
- 完成内容: 复采三节点后确认这次 slice 已经把“永久 unsupported 污染”缩掉，但真实环境仍存在两个残留面：`storage` 端 `connected_peers` 在 observer / sequencer / empty 之间抖动，且 `network_committed_height=0`、`known_peer_heads=0`；`sequencer` 端日志则出现 `storage challenge gate network blob not found` 后紧跟 `libp2p-replication no connected peers for protocol /aw/node/replication/fetch-blob/1.0.0`，说明请求层还有误隔离风险。
- 完成内容: 在 `crates/oasis7_node/src/libp2p_replication_network.rs` 收紧 unsupported 判定：`ErrNotFound` 不再视为“协议不支持”，只保留 `ErrUnsupported` 与显式 `handler missing` / 远端 `NetworkProtocolUnavailable` 这类真正的协议级失败进入 quarantine，避免把“内容不存在”的业务态缺失误升级成 peer 隔离。
- 完成内容: 新增回归 `libp2p_replication_network_does_not_quarantine_not_found_response_as_unsupported`，验证单个 peer 连续返回 `ErrNotFound` 时，后续请求仍然继续命中该 peer 并返回 `ErrNotFound`，不会在 retry window 内被错误转成 `no connected peers`。
- 完成内容: 已完成定向测试 `env -u RUSTC_WRAPPER cargo test -p oasis7_node libp2p_replication_network_does_not_quarantine_not_found_response_as_unsupported`、`env -u RUSTC_WRAPPER cargo test -p oasis7_node libp2p_replication_network_retries_previously_unsupported_single_peer_after_retry_window`、`env -u RUSTC_WRAPPER cargo test -p oasis7_node libp2p_replication_network_request_retries_next_peer_when_remote_handler_fails`。
- 遗留事项: 仍需重新 build / deploy 到三节点，并对照 `storage` / `sequencer` 的 `fetch-commit`、`fetch-blob` 日志和 `/v1/chain/status` 复采，确认误隔离减少后 residual 是否继续收敛到纯连接抖动或 gossip/head 种子缺失。

## 2026-04-08 23:04:20 CST / runtime_engineer
- 完成内容: 已基于当前 worktree 重编译 `release` 二进制并 rollout 到三节点：
  - release 目录：`7401ef56-notfound-no-quarantine-20260408`
  - sha256：`ff5a2b441330941e46ac545a1218df300ce0e1c5cd5805792cfb2676aa750d2f`
  - 本机 observer、ECS sequencer、ECS storage 的 `current/bin/oasis7_chain_runtime` 均已切换并核对到相同 hash。
- 完成内容: rollout 后 same-window 复采显示本次修复没有引入新回归，但仍未消除主 blocker：
  - observer：`committed_height=22811`、`network_committed_height=63496`、`known_peer_heads=1`，`unsupported_fetch_commit/blob=[]`，继续健康推进；
  - sequencer：`committed_height=57690`、`network_committed_height=57690`、`known_peer_heads=0`，当前 active 仅 observer，storage 仍常年停在 candidate；`unsupported_fetch_blob` 仍主要落在 observer；
  - storage：`committed_height=0`、`network_committed_height=0`、`known_peer_heads=0`，尽管 same-window 样本里 observer / sequencer 都可见为 active/direct，仍没有学到 head，也没有进入稳定 gap sync。
- 完成内容: rollout 后日志签名已更新并进一步确认 residual 不再主要是 `ErrNotFound` 误隔离，而是连接 / active-path 抖动与 provider-store 溢出叠加：
  - storage 新日志仍持续出现 `gap sync height ... no connected peers for protocol /aw/node/replication/fetch-commit/1.0.0`；
  - sequencer 新日志仍持续出现 `storage challenge gate network request failed ... no connected peers for protocol /aw/node/replication/fetch-blob/1.0.0`；
  - 两端都仍可见 `kad start_providing failed: the store cannot contain any more provider records`；
  - storage 新日志还出现 `outbound request failed: ... peer ... is not connected for protocol /aw/node/replication/fetch-commit/1.0.0`，说明请求发起瞬间与 status 采样看到的 active/direct 并不稳定一致。
- 遗留事项: 下一步更值得继续追的是 `active peer` / `connected_peers` 抖动窗口与 `kad provider record` 容量上限，而不是继续扩展 `ErrNotFound` 相关 quarantine；若要继续此 slice，建议优先处理：
  - `libp2p-replication` 请求等待窗口 / 连接稳定性；
  - `kad start_providing failed: the store cannot contain any more provider records`；
  - `storage` 启动后 head 种子为空时，是否需要从 DHT world head 或稳定 gossip 源补种。

## 2026-04-09 09:39:00 CST / runtime_engineer
- 完成内容: 已继续沿上一轮 residual 直接落两处更高概率修复：
  - `crates/oasis7_net/src/libp2p_net/swarm_behaviour.rs` 改为显式 `MemoryStore::with_config`，把 `kad` provider record 容量上限从默认 `1024` 提升到 `65536`，避免真实环境里反复出现 `kad start_providing failed: the store cannot contain any more provider records`。
  - `crates/oasis7_net/src/libp2p_net/runtime_loop.rs` 的 `RequestToPeer` 不再只信任陈旧的 `peers` 快照，补上 `swarm.is_connected(&peer)` 实时判断，减少“状态看起来 active/direct，但请求瞬间被本地旧列表拒绝”的窗口。
  - `crates/oasis7_node/src/libp2p_replication_network.rs` 为 `request/request_with_providers` 增加短窗 refreshed-peer 重试：若命中 `peer ... is not connected for protocol ...` 这类连接缝隙错误，会重新拉取当前 connected peers/providers 再试，而不是立即把这轮 fetch 判死。
- 完成内容: 已补充并通过本地回归：
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_net build_swarm_supports_more_than_default_max_provider_records`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_node retryable_connection_gap_detection_matches_request_to_peer_disconnects`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_node libp2p_replication_network_request_retries_next_peer_when_remote_handler_fails`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_node libp2p_replication_network_does_not_quarantine_not_found_response_as_unsupported`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_node libp2p_replication_network_retries_previously_unsupported_single_peer_after_retry_window`
  - `env -u RUSTC_WRAPPER cargo check -p oasis7_node -p oasis7_net`
- 完成内容: 已开始重编 `env -u RUSTC_WRAPPER cargo build --release -p oasis7 --bin oasis7_chain_runtime`，准备继续 rollout 到 triad。
- 遗留事项: 真实环境 rollout 暂时卡在 ECS 凭据：当前 shell 中已无可用 `P2PARCH6_SEQ_SSH_PASSWORD` / `P2PARCH6_STORAGE_SSH_PASSWORD`，对 `root@39.104.204.172` 与 `root@39.104.205.67` 的 SSH 均返回 `Permission denied`。收到新的服务器密码或可用 SSH key 后，下一步直接继续：
  - 拷贝新 binary 到两台 ECS 与本机 observer 对应 `releases/<release-id>/bin/oasis7_chain_runtime`
  - 切 `current` symlink 并重启 `oasis7-triad-{observer,sequencer,storage}.service`
  - 复跑 `./scripts/p2p-real-env-triad-snapshot.sh` 与三端 `chain-runtime.log` same-window 采样，判断 storage 是否脱离 `committed_height=0`

## 2026-04-09 09:59:00 CST / runtime_engineer
- 完成内容: 已从 `/home/scc/Documents/keys/aliyun_ecs.txt` 恢复两台 ECS 登录凭据，并完成新 release rollout：
  - release 目录：`7401ef56-provider-capacity-refresh-20260409`
  - sha256：`fe8b2e21c5c6f7405c236211a9d6cd1f2e1e94c33783a625b60db5bf9f682cf7`
  - 本机 observer、ECS sequencer、ECS storage 的 `current/bin/oasis7_chain_runtime` 均已切到同一 hash；本机 `oasis7-triad-observer.service` 与 ECS `oasis7-triad-storage.service` 均确认 `active`。
- 完成内容: 已复跑 `P2PARCH6_SEQ_SSH_PASSWORD='***' P2PARCH6_STORAGE_SSH_PASSWORD='***' ./scripts/p2p-real-env-triad-snapshot.sh --samples 3 --interval-secs 4 --out-dir .tmp/p2p_real_env_triad`，产物位于 `.tmp/p2p_real_env_triad/20260409-095444/`。这轮 snapshot 仍为 `claim_status=blocked`，签名为：
  - `cloud_pair_service_unhealthy`
  - `cloud_pair_chain_not_visible`
  - `sequencer_committed_height_zero`
  - `storage_committed_height_zero`
  - `cloud_pair_no_recent_progress_signal`
  - `observer_known_peer_heads_zero`
  - `observer_network_committed_height_zero`
  - `observer_committed_height_not_advancing`
- 完成内容: rollout 后即时状态显示 observer/storage 当前 runtime 可运行，但都停在 cold-start 基线：
  - observer：`tick_count=1`、`slot/latest_height/committed_height/network_committed_height=0`、`known_peer_heads=0`；replication 侧能看到 storage active，但对 sequencer `39.104.204.172:5611` 持续报 `ConnectionRefused/ConnectionReset`。
  - storage：`tick_count=523`、`slot/latest_height/committed_height/network_committed_height=0`、`known_peer_heads=0`；last_error 仍是 `libp2p-replication no connected peers for protocol /aw/node/replication/fetch-commit/1.0.0`。
- 完成内容: 新发现一个更上游的真实环境 blocker：ECS sequencer 在这轮 snapshot 期间持续被 `SIGKILL`，systemd restart counter 一度已在 `1625+`。`journalctl -u oasis7-triad-sequencer.service` 表明该服务在本轮 rollout 之前就已经反复 `Main process exited, code=killed, status=9/KILL`，因此本次 triad blocked 不应直接归咎于本轮 provider-capacity / refreshed-peer patch。
- 完成内容: 从 storage 当前最新状态面看，`last_error` 已收敛为 `fetch-commit no connected peers`；`kad start_providing failed: the store cannot contain any more provider records` 仍可在重启前后的历史 log 尾部看到，但不再是当前 status 的主错误签名。该现象说明“扩大 provider store 容量”至少没有恶化当前 live residual，但还不足以在 sequencer 不健康时单独拉起 triad。
- 遗留事项: 下一步优先级已变化：
  - 先定位 ECS sequencer 被 `SIGKILL` 的外部原因（OOM / 云侧 agent / host policy / 进程级 kill），否则 storage 与 observer 只会继续停在 `0`；
  - 在 sequencer 恢复稳定后，再复跑 same-window triad snapshot，重新判断 `fetch-commit no connected peers` 是否仍是主 residual；
  - 若 sequencer 稳定后 storage 仍继续报 `no connected peers`，再继续顺着 request-time connectivity / head seeding 追。

## 2026-04-09 13:22:00 CST / runtime_engineer
- 完成内容: 复查 rollout 后即时状态，确认 triad 已不再处于“三端全零”静止态：
  - 本机 observer 当前为 `committed_height=37236`、`network_committed_height=63496`、`known_peer_heads=0`，说明本机链路可运行但尚未恢复 peer head 观测；
  - ECS storage 当前为 `committed_height=63496`、`network_committed_height=63496`、`known_peer_heads=1`，说明其已至少追到一侧 head，但 `last_error` 仍反复落在 `libp2p-replication no connected peers for protocol /aw/node/replication/fetch-commit/1.0.0`；
  - ECS sequencer 的 `127.0.0.1:5631` 仍常态不可连，systemd 处于 `ActiveState=activating` / `SubState=auto-restart`，`ExecMainStatus=9`。
- 完成内容: 进一步隔离 `SIGKILL` 触发面后确认：
  - 普通 transient unit（`systemd-run ... /bin/sleep 20`）可正常跑满并 `Result=success`，说明不是“只要是名字像 sequencer 的 unit 就会被杀”；
  - 但用 `systemd-run` 起同一 `oasis7_chain_runtime`、改临时端口 `6631/6611`、改 `/tmp` 数据目录的 debug unit，仍会在约 `3s` 内 `Main process exited, code=killed, status=9/KILL`；
  - 与此前“同一二进制手工前台运行 + 临时端口/目录可存活 `timeout 15s`”结合，当前证据已把问题收窄到“systemd 管理态的 runtime 进程会被宿主机侧外力清理”，而不是正式 unit 文件、正式端口或正式数据目录本身。
- 完成内容: 复查 sequencer host 宿主进程后，确认其上仍常驻阿里云宿主机代理栈：`AliYunDun`、`AliYunDunMonitor`、`cloudmonitor`、`aliyun-assist`；同时 `aegis.service` 自身在历史上也曾出现 `Control process exited, code=killed, status=9/KILL`。目前全局 journal 尚未直接吐出“谁发送了 kill”的显式归因，但这条线已成为当前最可疑的外部宿主因子。
- 遗留事项:
  - 当前应用侧 `fetch-commit` / provider-capacity patch 已完成本地验证并成功 rollout，但真实环境的第一阻断已明确转为“sequencer host 上 systemd 管理态 runtime 被外部 `SIGKILL`”；
  - 下一步若继续 triad 闭环，应优先处理宿主机策略/云代理干扰（例如临时豁免、停用/旁路相关 agent、或改用不受其影响的启动方式）后，再重跑 triad snapshot 评估应用层 residual。

## 2026-04-09 14:24:00 CST / runtime_engineer
- 完成内容: 继续对 sequencer host 的 `SIGKILL` 做启动路径矩阵隔离，新增三条高价值证据：
  - 将 SSH session 内启动的进程手工移出 `session-*.scope` 到 cgroup 根 `/` 后，普通 `/bin/sleep 120` 可在 session 结束后继续存活，说明“session close 自清理”本身不是所有后台进程的统一结局；
  - 同样移到 cgroup 根 `/` 的 `./bin/start-node.sh` 若继续使用正式 `--execution-world-dir /opt/oasis7/p2p-triad/data/execution-world` 与 `--execution-records-dir /opt/oasis7/p2p-triad/data/execution-records`，仍会被直接 `Killed`，说明这次 kill 不再依赖 systemd unit，也不再依赖 session cgroup；
  - 非 systemd、cgroup 根 `/`、正式 `5611/5631` 端口、真实 `storage-root=/opt/oasis7/p2p-triad/data/storage`，但改用临时 execution 路径时，sequencer 可稳定存活超过 `1m40s`；而同样配置只要回到正式 execution 路径，就会重新被 kill。
- 完成内容: 通过对照进一步把触发面从“正式端口或正式数据目录整体”缩到更窄范围：
  - `--status-bind 127.0.0.1:5631` + `--node-gossip-bind 0.0.0.0:5611` 本身不是触发条件，因为正式端口 + 临时 execution 路径 + 真实 storage 可以存活；
  - `storage-root=/opt/oasis7/p2p-triad/data/storage` 也不是单独触发条件，因为真实 storage + 临时 execution 路径同样可存活；
  - 当前最可疑命中面已收敛为正式 `execution-world` / `execution-records` 路径（或其内容 / 文件模式）被宿主机侧策略命中。
- 完成内容: 试做 workaround：把现有 `execution-world`（约 `80M`）与 `execution-records`（约 `214M`）复制到 `/tmp/oasis7-seq-relocated-20260409-142013/` 后，配合真实 storage 重新起 sequencer；该实例未再触发外部 kill，但由于 execution 状态与当前链上持久状态不一致，只跑到 `committed_height=4` 并报 `gap sync ... persisted commit hash mismatch`，因此不能直接当作现网修复方案保留。
- 完成内容: 已在验证后手工停止所有测试 sequencer 进程，避免继续占用 `5611/5631` 或污染当前 triad。
- 遗留事项:
  - 当前最有价值的云侧 follow-up 不是“继续猜 systemd 参数”，而是请求宿主机/安全代理侧对 `/opt/oasis7/p2p-triad/data/execution-world` 与 `/opt/oasis7/p2p-triad/data/execution-records` 相关命中做白名单或审计解释；
  - 若需要临时恢复现网 sequencer，可继续尝试“复制 execution 路径到新持久目录 + 冷拷贝一致性”方案，但这已经超出应用层 `fetch-commit startup ordering` 的主修复范围，属于宿主机策略旁路。

## 2026-04-09 15:25:00 CST / runtime_engineer
- 完成内容: 已把 sequencer 当前本地 execution 持久态的真实陈旧程度量化出来：
  - `/opt/oasis7/p2p-triad/data/execution-records/latest.json` 只到 `height=58099`
  - `checkpoints/latest.json` 只到 `height=58048`
  - `/opt/oasis7/p2p-triad/data/execution-world/snapshot.manifest.json` 只到 `epoch=54014`
  - 但同时间 storage 侧已推进到 `committed_height=6359x`，说明 sequencer 本地 execution 数据早在宿主机 kill 之前就已经长期落后。
- 完成内容: 已验证“仅靠 relocation 并不能把 sequencer 拉回当前链高”：
  - 只 relocation execution 路径、保留旧 `output/node-distfs` 时，实例虽然可存活，但会在 `gap sync height 5` 触发 `persisted commit hash mismatch`，说明旧 replication root 里已有冲突持久化 commit；
  - 进一步把工作目录也 relocation，使 `replication_root` 变成全新空目录，并显式 relocation `execution_bridge_state` 后，实例不再报 persisted hash mismatch，但会在启动后稳定报 `node execution error: execution record at height 1 missing latest_state_ref`。
- 完成内容: 上述错误已定位到本地 execution record 数据质量本身：
  - `reward-runtime-execution-bridge-state.json` 与 `latest.json` 都表明 bridge 记忆停在 `58099`
  - `00000000000000000001.json` 虽是 `schema_version=2`，但只含 `external_effect_ref`，缺少 `latest_state_ref` / `snapshot_ref` / `journal_ref`
  - 当前恢复逻辑只会在本地 exact record 至少提供 `latest_state_ref`（或兜底 `snapshot_ref`）时才能回放恢复，因此这批老 record 本身不足以支撑从本地重建 sequencer 执行态。
- 完成内容: 已在验证后关闭所有 relocation 实例，确认远端不再残留额外 `oasis7_chain_runtime` 测试进程。
- 遗留事项:
  - 当前 triad 的阻断已经从“startup ordering”扩展为“双重环境债务”：一是宿主机会杀正式 execution 路径；二是 sequencer 本地 execution 持久态已经陈旧且早期 record 缺少恢复所需字段；
  - 在不引入新的恢复工具/迁移脚本前，继续靠现有本机文件做冷拷贝 relocation 已无法把 sequencer 恢复到当前链高；
  - 后续若要真正恢复，需要二选一：
    1. 提供可用的较新 execution state / records 作为恢复源；
    2. 实现专门的 legacy execution-record migration / rebuild 工具，使缺 `latest_state_ref` 的早期 record 也能被当前 runtime 接受。

## 2026-04-09 16:58:32 CST / runtime_engineer
- 完成内容: 已在 `crates/oasis7/src/bin/oasis7_chain_runtime/execution_bridge/driver.rs` 为 stale-height restore 增加更窄范围的 malformed V2 兼容：
  - 当 record 缺少 `latest_state_ref` / `snapshot_ref`，但仍保留 `execution_state_root` 时，恢复路径现在允许把 `execution_state_root` 当作 snapshot CAS key 读取；
  - 当 record 缺少 `journal_ref` 时，恢复路径会尝试从当前已加载的 `execution_world` 持久态中截取对应 `snapshot.journal_len` 的 journal 前缀，校验 `last_event_id` 后回灌 CAS，并把修复后的 ref 只写回该高度 record 文件。
- 完成内容: 这次实现刻意没有改全局 record 反序列化/归一化语义，避免把 retention 已有意裁掉 `snapshot_ref/journal_ref` 的 archive-only 旧高度重新膨胀；修复只在“确实命中 stale restore 且本地能自证恢复”的那一个 record 上生效。
- 完成内容: 已新增并通过定向回归：
  - `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime execution_bridge_record_recovery_snapshot_ref_falls_back_to_execution_state_root -- --nocapture`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime node_runtime_execution_driver_recovers_malformed_v2_record_from_state_root_and_local_journal -- --nocapture`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime node_runtime_execution_driver_reconciles_stale_state_from_exact_record -- --nocapture`
  - `env -u RUSTC_WRAPPER cargo check -p oasis7 --bin oasis7_chain_runtime`
- 遗留事项:
  - 该兼容只覆盖“snapshot 仍可由 `execution_state_root` 命中 CAS，且本地 `execution_world` 仍保有足够长 journal 前缀”的坏 record；如果 sequencer host 上更深历史高度也缺 ref、且当前 world journal 已不足以裁出所需前缀，仍需要更重的 rebuild / 外部恢复源。
  - 下一步应优先把这版 binary 先用于 relocated execution 路径复验，再判断 sequencer 是否已能越过 `height=1 missing latest_state_ref` 继续启动；若能，再回到宿主机 kill 旁路和真实 triad same-window 复采。

## 2026-04-09 17:26:53 CST / runtime_engineer
- 完成内容: 已把新逻辑编进 debug binary（`sha256=fdecaf7fe37eef755a8ac9bb154d19a01f7a5dc193daa6072727d8cd7de0e743`）并上传到 sequencer host，随后在不走正式 systemd unit 的前提下复跑 relocation 路径：
  - binary: `/tmp/oasis7_chain_runtime-p2parch6-debug`
  - execution dirs: `/tmp/oasis7-seq-relocated-20260409-142013/{execution-world,execution-records}`
  - copied bridge state: `/tmp/oasis7-seq-legacy-recovery-20260409-172229/reward-runtime-execution-bridge-state.json`
  - temp status/gossip: `127.0.0.1:6631` / `0.0.0.0:6611`
- 完成内容: 这轮实测确认仓内兼容修复已经把失败点从“record 缺 `latest_state_ref`”继续推进到更底层的数据缺失：
  - 进程已能打印 `oasis7_chain_runtime ready.`，说明不再卡死在之前的 malformed-record 结构检查；
  - 但随后日志变为 `execution driver restore snapshot ref 6674... failed at height 1: BlobNotFound`，即 `execution_state_root` 对应的 snapshot CAS blob 在当前 storage 根下并不存在。
- 完成内容: 已远端直接核对对应 blob 缺失，确认这不是恢复路径误判：
  - `MISSING:/opt/oasis7/p2p-triad/data/storage/blobs/6674...fb4f7b2a.blob`
  - `MISSING:/tmp/oasis7-seq-relocated-20260409-142013/execution-world/.distfs-state/blobs/6674...fb4f7b2a.blob`
- 完成内容: 测试后已清理临时 debug 进程，并把 `oasis7-triad-sequencer.service` 恢复到原来的 `active` 状态，避免长期改变云端机器当前基线。
- 遗留事项:
  - 当前 sequencer 的真实新 blocker 已进一步收敛为“早期 execution record 引到的 snapshot blob 本体缺失”，而不再是单纯字段兼容问题；仅靠 runtime 兼容无法凭空恢复缺失 blob。
  - 后续若继续恢复 sequencer，需要补一份可用的早期 snapshot blob 来源，或提供一条不依赖该缺失 blob 的 execution rebuild / bootstrap 路径；在此之前，relocation 实例最多只能证明代码已越过结构检查，不能证明数据可完整恢复。

## 2026-04-09 18:49:50 CST / runtime_engineer
- 完成内容: 在用户明确授权“删运行态数据，保留身份配置”后，已把 triad 三节点统一切回干净运行态基线，且每台都先只备份配置、不动身份文件：
  - ECS sequencer：保留 `/opt/oasis7/p2p-triad/config/node.env`、`/opt/oasis7/p2p-triad/config/node-keypair.toml`，删除 `data/{execution-world,execution-records,storage}` 与 `output/{chain-runtime/node-distfs}/triad-sequencer-a`，配置备份为 `/opt/oasis7/p2p-triad/backups/config-20260409-183901.tgz`。
  - 本机 observer：保留 `/opt/oasis7/p2p-triad-local/config/node.env`、`/opt/oasis7/p2p-triad-local/config/node-keypair.toml`，删除 `data/{execution-world,execution-records,storage}` 与 `output/{chain-runtime/node-distfs}/triad-observer-local`，配置备份为 `/opt/oasis7/p2p-triad-local/backups/config-20260409-184631.tgz`。
  - ECS storage：保留 `/opt/oasis7/p2p-triad/config/node.env`、`/opt/oasis7/p2p-triad/config/node-keypair.toml`，删除 `data/{execution-world,execution-records,storage}` 与 `output/{chain-runtime/node-distfs}/triad-storage-b`，配置备份为 `/opt/oasis7/p2p-triad/backups/config-20260409-184711-storage.tgz`。
- 完成内容: 清理后的 same-window 复采证明先前的 real-env 阻断主要是“三节点运行态代际不一致”而不是新代码回归：
  - 仅清 sequencer 后，`20260409-184116` 快照显示 sequencer 已能从 `8 -> 9` 推进，但 observer 仍背着旧缺 blob 运行态，停在 `38271`。
  - 进一步清 observer 后，observer 立刻从旧高度回到干净基线并重新推进到 `32+`，但由于 storage 仍留在旧链 `6397x`，triad 出现新旧链并存。
  - 在确认 `observer/sequencer` 与 `storage` 的 `last_block_hash` 已分叉后，再清 storage，把三台统一拉回同一起点。
- 完成内容: 最终快照 `./.tmp/p2p_real_env_triad/20260409-184908/summary.md` 已给出 `claim_status=pass_candidate`、`failure_signatures=(none)`：
  - observer：`committed_height 39 -> 42`
  - sequencer：`committed_height 40 -> 43`
  - storage：`committed_height 40 -> 43`
  - 三端最新同窗状态已对齐到同一链，且 `last_error=null`。
- 完成内容: 现场快速核对还确认三端在统一清态后已经重新汇合到同一块哈希：
  - observer `height=39 last_block_hash=f2fa1b0f7e2ded6e922a1a461cc35656df2c8054166adad8c9fc873f1d618e18`
  - sequencer `height=39 last_block_hash=f2fa1b0f7e2ded6e922a1a461cc35656df2c8054166adad8c9fc873f1d618e18`
  - storage `height=39 last_block_hash=f2fa1b0f7e2ded6e922a1a461cc35656df2c8054166adad8c9fc873f1d618e18`
- 遗留事项:
  - 当前 `pass_candidate` 证明“在三节点真实环境里，保留身份配置但清空运行态后，triad 能重新建出一致链并稳定前进”；它不等于“旧链历史数据已被迁移恢复”。
  - 若下一步要继续验证更高链高下的稳定性，建议基于当前干净三节点继续做更长时间窗复采，而不是再混入旧 execution/storage 持久态。

## 2026-04-09 19:52:00 CST / runtime_engineer
- 完成内容: 已继续沿“active/direct 但请求层仍报 no connected peers”的残留落一版更窄修复：
  - `crates/oasis7_node/src/libp2p_replication_network.rs` 增加 `connected_or_active_transport_peers` 回退，把 `request()` / `request_with_providers()` 的候选 peer 源从纯 `connected_peers()` 扩到“若 connected snapshot 为空，则退到 `peer_healths.active_path_kind.is_some()` 的 peer”。
  - 新增并通过定向回归：`connected_or_active_transport_peers_prefers_connected_snapshot`、`connected_or_active_transport_peers_falls_back_to_active_health_peers`、`libp2p_replication_network_request_retries_next_peer_when_remote_handler_fails`、`retryable_connection_gap_detection_matches_request_to_peer_disconnects`、`cargo check -p oasis7_node`。
- 完成内容: 已将该版编成 release 并 rollout 到 triad：
  - release 目录：`7401ef56-peer-health-fallback-20260409`
  - sha256：`84cac925ab73e1f8b82d72a05e459ae6b3d856db8b0a6757b5466cbba7f4c170`
  - 三端统一清态后短窗 `./.tmp/p2p_real_env_triad/20260409-192223/summary.md` 显示重新汇合并共同推进：observer/sequencer/storage 均为 `1 -> 4`。
- 完成内容: 已对这版旧阈值 binary 做更长时间窗留证，确认新的 real-env 主阻断不再是“sequencer 稳定卡在 85/86”而是更早的 storage challenge gate：
  - 长窗 `./.tmp/p2p_real_env_triad/20260409-192343/summary.md` 中，observer `8 -> 72`、storage `8 -> 73`，但 sequencer 只到 `7 -> 11`。
  - sequencer 同窗错误已收敛到 `storage challenge gate network threshold unmet: required_matches=2 successful_matches=1`，并伴随 `gap sync height 12 ... no connected peers for protocol /aw/node/replication/fetch-commit/1.0.0`；说明 peer-selection fallback 把 stall 从原先更后的 `85/86` 窗口前推到了更具体的“启动热身期 gate 过硬”。
- 完成内容: 已据此在 `crates/oasis7_node/src/lib.rs` / `crates/oasis7_node/src/lib_impl_part1.rs` 增加 storage challenge gate warmup：
  - 新增 `STORAGE_GATE_NETWORK_WARMUP_HEIGHT=32`，在早期高度把 `required_matches` 以 `min(1)` 收紧到单匹配，避免 clean-start/catch-up 前段因“只有 1 个远端样本可验证”就把 sequencer 直接卡死。
  - 新增并通过 `runtime_replication_storage_challenge_gate_allows_single_match_during_warmup`；同时补齐 `runtime_replication_storage_challenge_gate_falls_back_to_older_samples_during_catchup`、`runtime_replication_storage_challenge_gate_falls_back_after_provider_route_unavailable` 与 `cargo check -p oasis7_node`，确认原 catch-up / provider fallback 语义仍成立。
- 完成内容: 已将 warmup 版重新编成 release 并 rollout + 清态复验：
  - release 目录：`7401ef56-storage-gate-warmup-20260409`
  - sha256：`64f3571757da20fc9f2d97a2ae33082fa05da35654f9016f8a688faa41c09806`
  - 配置备份：
    - observer：`/opt/oasis7/p2p-triad-local/backups/config-20260409-194714-storage-gate-warmup-reset.tgz`
    - sequencer：`/opt/oasis7/p2p-triad/backups/config-20260409-194715-storage-gate-warmup-reset-sequencer.tgz`
    - storage：`/opt/oasis7/p2p-triad/backups/config-20260409-194714-storage-gate-warmup-reset-storage.tgz`
  - 短窗 `./.tmp/p2p_real_env_triad/20260409-194737/summary.md`：observer `2 -> 4`、sequencer `2 -> 5`、storage `2 -> 5`，sequencer 不再在 `11/12` 之前报错。
  - 中窗 `./.tmp/p2p_real_env_triad/20260409-194857/summary.md`：observer `8 -> 19`、storage `9 -> 19`、sequencer `8 -> 13`，sequencer 已明确跨过旧版稳定卡住的 `11/12` 窗口。
- 遗留事项:
  - warmup 版没有彻底消除 stall；sequencer 现在的新硬阻断已进一步收敛为 `required_matches=1 successful_matches=0`，即启动期后段对某批 sampled blob 已经连“1 个远端有效 match”都拿不到。
  - 当前 sequencer 最新日志同时保留两类签名：
    - `storage challenge gate network threshold unmet: required_matches=1 successful_matches=0`
    - `gap sync height 14 failed ... no connected peers for protocol /aw/node/replication/fetch-commit/1.0.0`
  - 下一步不应再继续单纯放宽阈值；更值得追的是：
    1. sampled blob provider 的发布 / 可见性时序为什么在 `height≈13/14` 后掉成全 `not found`
    2. sequencer `known_peer_heads=0` / `fetch-blob no connected peers` 为什么仍长期偏低于 observer/storage
    3. storage challenge gate 是否需要基于“远端样本完全 unavailable”与“真实内容校验失败”继续拆分策略，而不是统一算作同一类 `Unavailable`

## 2026-04-09 20:16:40 CST / runtime_engineer
- 完成内容: 已把新一层 warmup 补丁落到代码并完成本地验证：
  - `crates/oasis7_node/src/lib_impl_part1.rs` 在 `enforce_storage_challenge_gate()` 中新增“warmup 且 `peer_heads` 仍为空时直接跳过网络采样 gate”的短路，避免把“尚未学到任何 peer head”误当成内容完整性失败。
  - `crates/oasis7_node/src/tests_split_part3.rs` 新增 `runtime_replication_storage_challenge_gate_skips_network_probe_during_warmup_without_peer_heads`，并调整 `runtime_replication_storage_challenge_gate_allows_single_match_during_warmup` 以继续覆盖 warmup 非短路分支。
  - 已通过 `env -u RUSTC_WRAPPER cargo test -p oasis7_node runtime_replication_storage_challenge_gate_skips_network_probe_during_warmup_without_peer_heads -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7_node runtime_replication_storage_challenge_gate_allows_single_match_during_warmup -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7_node runtime_replication_storage_challenge_gate_falls_back_to_older_samples_during_catchup -- --nocapture`、`env -u RUSTC_WRAPPER cargo check -p oasis7_node`。
- 完成内容: 已将该版编成 release 并 rollout 到 triad：
  - release 目录：`7401ef56-storage-gate-peer-head-warmup-skip-20260409`
  - sha256：`423ab45ad2a49606ff89f2551f950b7d3f4de817228c76eb7f57e69702e7e92a`
  - 三端统一保留身份配置、清空运行态后重新起服务；配置备份：
    - observer：`/opt/oasis7/p2p-triad-local/backups/config-20260409-201039-storage-gate-peer-head-warmup-skip-reset.tgz`
    - sequencer：`/opt/oasis7/p2p-triad/backups/config-20260409-201039-storage-gate-peer-head-warmup-skip-reset-sequencer.tgz`
    - storage：`/opt/oasis7/p2p-triad/backups/config-20260409-201039-storage-gate-peer-head-warmup-skip-reset-storage.tgz`
- 完成内容: 真实三节点留证表明这次补丁已经把 sequencer 明确推过旧版 `13/14` stall：
  - 短窗 `./.tmp/p2p_real_env_triad/20260409-201221/summary.md`：observer `2 -> 5`、sequencer `2 -> 5`、storage `2 -> 5`，三端 `last_error=(none)`。
  - 中窗 `./.tmp/p2p_real_env_triad/20260409-201319/summary.md`：observer `7 -> 17`、sequencer `7 -> 17`、storage `7 -> 17`，sequencer 已不再复现上一版的 `8 -> 13` 停滞。
  - 窗口结束后现场核对 `/v1/chain/status` 仍继续推进：observer `committed_height=19`、sequencer `committed_height=19`、storage `committed_height=20`，三端 `last_error=null`。
- 完成内容: 当前 residual 也更清楚了：虽然 sequencer 已越过旧停点并持续前进，但 `known_peer_heads` 仍保持 `0`；因此这次 slice 证明“warmup 无 peer-head 短路”修正了启动窗口的误杀，不等于“peer head 学习链路已经完全恢复”。
- 完成内容: 已继续对 `known_peer_heads=0` 做代码与现场核对，确认它在当前 triad 拓扑下大概率不是新的 blocker：
  - `crates/oasis7/src/bin/oasis7_chain_runtime.rs` 明确对 `NodeRole::Sequencer` 启用 `with_require_peer_execution_hashes(true)`。
  - `crates/oasis7_node/src/lib_impl_part1.rs` 的 `validate_peer_commit_execution_binding()` 会在本地已持有 execution binding 时，拒绝缺少 execution hashes 的 peer commit。
  - 当前现场角色分布里，sequencer 有 execution side，observer/storage 没有；因此 sequencer 对这两类 peer commit 都可能不记 `peer_heads`，而 observer/storage 仍可继续记录来自 sequencer 的 peer head。该现象与实机状态一致：observer `known_peer_heads=2`、storage `=1`、sequencer `=0`，但三端高度继续共同推进到 `35+` 且 `last_error=null`。
- 遗留事项:
  - 下一步若继续追 residual，优先级不再是 `known_peer_heads=0` 本身，而是更长时间窗下是否会在更高高度出现新的 stall。
  - 建议继续基于当前干净三节点做更长时间窗复采；只有在出现新停点时，再回到 peer-head / gossip 传播与 execution-binding 校验交界处细查。

## 2026-04-09 21:02:10 CST / runtime_engineer
- 完成内容: 已基于上一轮长窗暴露的新 stall（sequencer 在 `height≈75` 处再次被 `storage challenge gate network threshold unmet` 卡住）继续收一层更窄修复：
  - `crates/oasis7_node/src/lib_impl_part1.rs` 里 network sample `required_matches` 的放宽条件不再只看 warmup 高度，也覆盖“`require_peer_execution_hashes=true` 且 `peer_heads` 仍为空”的 sequencer 拓扑，把门槛收为 `min(1)`，但仍保留“至少一个远端样本必须命中”的约束，不直接跳过 gate。
  - 该条件刻意限定在 `require_peer_execution_hashes` 路径，避免把 observer / storage 的普通无 peer-head 场景一起放宽。
- 完成内容: 已补充并通过本地回归：
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_node runtime_replication_storage_challenge_gate_allows_single_match_without_peer_heads_after_warmup -- --nocapture`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_node runtime_replication_storage_challenge_gate_falls_back_to_older_samples_during_catchup -- --nocapture`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_node runtime_replication_storage_challenge_gate_skips_network_probe_during_warmup_without_peer_heads -- --nocapture`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_node runtime_replication_storage_challenge_gate_allows_single_match_during_warmup -- --nocapture`
  - `env -u RUSTC_WRAPPER cargo check -p oasis7_node`
- 完成内容: 已将该版编成 release 并 rollout 到 triad：
  - release 目录：`7401ef56-storage-gate-peer-headless-single-match-20260409`
  - sha256：`6acd6fad174ea07be42bc19dbe24bc0f567faf28b36253100ba0811e303d29d0`
  - 三端统一清态并保留身份配置后的配置备份：
    - observer：`/opt/oasis7/p2p-triad-local/backups/config-20260409-205259-storage-gate-peer-headless-single-match-reset.tgz`
    - sequencer：`/opt/oasis7/p2p-triad/backups/config-20260409-205259-storage-gate-peer-headless-single-match-reset-sequencer.tgz`
    - storage：`/opt/oasis7/p2p-triad/backups/config-20260409-205259-storage-gate-peer-headless-single-match-reset-storage.tgz`
- 完成内容: 真实三节点复验结果明显优于上一版：
  - 短窗 `./.tmp/p2p_real_env_triad/20260409-205400/summary.md`：observer `2 -> 4`、sequencer `2 -> 5`、storage `2 -> 5`，`last_error=(none)`。
  - 长窗 `./.tmp/p2p_real_env_triad/20260409-205457/summary.md`：observer `6 -> 36`、sequencer `6 -> 36`、storage `7 -> 37`，整窗 `last_error=(none)`，上一版在 `height≈75` 暴露的 gate / fetch-blob / fetch-commit 复合 stall 未再出现。
  - 窗口结束后现场再核对 `/v1/chain/status`，三端仍共同推进：observer `h=38`、sequencer `h=39`、storage `h=39`，三端 `last_error=null`。
- 遗留事项:
  - 当前这版已经打掉已知的 `13/14` 与 `≈75` 两层 stall，但还没有做更长的 soak 去确认是否会在更高高度出现下一层残留。
  - sequencer `known_peer_heads=0` 仍保持不变，不过当前 evidence 继续支持它不是本轮 blocker；后续只有在更高高度再次出现 stall 时，才值得把它重新升级为主调查对象。

## 2026-04-09 21:13:20 CST / runtime_engineer
- 完成内容: 已完成 commit 前独立 subagent review；review 未发现阻断当前 landing 的高优先级缺陷，主要保留两条后续风险提示：
  - `require_peer_execution_hashes && peer_heads.is_empty()` 触发的 gate 放宽是当前 triad 拓扑特化语义，后续若推广到更多 provider 拓扑，需要重新审视完整性门槛。
  - libp2p replication 当前对 retryable connection gap / unsupported protocol 的错误分类仍依赖字符串签名，后续值得往结构化字段收敛。
- 完成内容: 已根据 review 顺手修正文档安全细节，把 evidence 中的 `sshpass -p` 示例改成 `sshpass -e`，避免把密码直接暴露到进程列表。
- 完成内容: 已补做更高层 smoke：
  - `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime execution_bridge_record_recovery_snapshot_ref_falls_back_to_execution_state_root -- --nocapture`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime node_runtime_execution_driver_recovers_malformed_v2_record_from_state_root_and_local_journal -- --nocapture`
  - `env -u RUSTC_WRAPPER cargo check -p oasis7`
- 完成内容: 已执行 `./scripts/pm/workflow-report.sh --phase close --role runtime_engineer --task-uid task_9dab2aa9246b465c805f5e59ffb08cba`、`./scripts/pm/move-task.sh --task-uid task_9dab2aa9246b465c805f5e59ffb08cba --to-status done` 与 `./scripts/pm/lint.sh`，确认 PM 收口与结构门禁正常。
- 遗留事项:
  - `./scripts/pm/codex-working-memory.sh --task-uid task_9dab2aa9246b465c805f5e59ffb08cba --role runtime_engineer` 因当前会话命名未匹配 worktree pattern 而未生成条目；本次先不把它作为 landing 阻断。
