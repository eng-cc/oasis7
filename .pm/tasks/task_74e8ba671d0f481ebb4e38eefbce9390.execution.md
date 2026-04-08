# task_74e8ba671d0f481ebb4e38eefbce9390 Execution Log

- task_uid: task_74e8ba671d0f481ebb4e38eefbce9390
- title: Roll out sequencer stale-height fix and rerun real-env triad
- owner_role: runtime_engineer
- worktree_hint: oasis7-p2p-p2parch-6-sequencer-stale-height-rollout

<!-- Append entries using:
Example:
  ## YYYY-MM-DD HH:MM:SS CST / role_name
  - 完成内容: ...
  - 遗留事项: ...
-->

## 2026-04-08 13:23:03 CST / runtime_engineer
- 完成内容: 在当前 worktree `HEAD=f8b1baf97316` 上重新执行 `env -u RUSTC_WRAPPER cargo build --release -p oasis7 --bin oasis7_chain_runtime`，得到 fresh build `sha256=72a6008f24b85e3b8e223db2e141688c2d10cd58cff578c1550e2028796d7aa7`。第一次误用了主 worktree 中的 stale release artifact `d41ad948f60288ad9b619fdfcae308959feb2aab7043b23d8faf8c662126a515`，远端 service 因缺少 `--replication-network-*` CLI 起不来；已在同轮把两台 ECS 回滚到旧 release 后，再用 fresh build 重新发布到 `/opt/oasis7/p2p-triad/releases/f8b1baf97316-stale-height-rollout-20260408`。
- 完成内容: 复核 fresh rollout 后的 ECS live status。`triad-sequencer-a` 与 `triad-storage-b` 当前 `current/bin/oasis7_chain_runtime` 均为 `sha256=72a6008f24b85e3b8e223db2e141688c2d10cd58cff578c1550e2028796d7aa7`；sequencer 不再复现 `execution driver received stale height: context=57536 state=57560`，而是转成 `node consensus error: storage challenge gate network threshold unmet ... network blob not found ...`。
- 完成内容: 执行 `P2PARCH6_SEQ_SSH_PASSWORD='***' P2PARCH6_STORAGE_SSH_PASSWORD='***' ./scripts/p2p-real-env-triad-snapshot.sh --samples 3 --interval-secs 4 --out-dir .tmp/p2p_real_env_triad`，产物位于 `.tmp/p2p_real_env_triad/20260408-132008/`。same-window triad 结果显示：observer `known_peer_heads=1` 但 `committed_height=9383 -> 9383`、残留 `gap sync ... blob not found`；storage `committed_height=62291 -> 62293`；sequencer `committed_height=0 -> 0` 且新主签名为 `storage challenge gate network threshold unmet`。
- 完成内容: 已新增 `doc/testing/evidence/p2p-real-env-triad-stale-height-rollout-2026-04-08.md`，并把 `P2PARCH-6` project 真值更新为“stale-height blocker 已清掉，但 real-env triad 仍被 storage challenge / observer gap-sync residual 阻断”。
- 遗留事项: 本机 `triad-observer-local` 本轮未同步升级到 fresh build，仍停在旧二进制 `sha256=004aaf7529a4c1e26be5150aaf87ac4b648e241f29295b2dc23824d516ea4785`；因此本轮 evidence 只能证明 cloud-side stale-height rollout 已改变 blocker，不能冒充 triad 再次版本完全一致。
- 遗留事项: 下一阶段若继续在真实三节点环境推进，应优先追 `storage challenge gate network threshold unmet` 与 observer `gap sync ... blob not found` 的共同根因，而不是再回到 stale-height 恢复路径重复排查。
