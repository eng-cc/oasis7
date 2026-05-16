# task_dfb9d8eedfe14f218c2f6e77151dad25 Execution Log

- task_uid: task_dfb9d8eedfe14f218c2f6e77151dad25
- title: optimize hot snapshot storage budget
- owner_role: runtime_engineer
- worktree_hint: /home/scc/worktrees/oasis7-world-runtime-storage-hot-snapshot-budget-optimization

<!-- Append entries using:
Example:
  ## YYYY-MM-DD HH:MM:SS CST / role_name
  - 完成内容: ...
  - 遗留事项: ...
-->

## 2026-05-15 19:02:31 CST / runtime_engineer
- 完成内容: 复盘本地三节点真实样本后，确认默认磁盘增量主要来自 `release_default` 档位的 execution hot snapshots；代码上将 `StorageProfile::ReleaseDefault` 的 `execution_hot_head_heights` 从 `128` 收紧到 `64`，与 `execution_checkpoint_interval=64` 对齐，并补了定向 driver/profile 回归，确保 `height=65` 时仅裁剪窗口外 refs、不影响 `height=64` 的 checkpoint cadence。
- 完成内容: 回写 `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.prd.md`、`doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.project.md` 与 `doc/world-runtime/project.md`，记录本轮为何先收紧默认热窗口预算，而不是直接删除全部非最新热高度 snapshot。
- 遗留事项: 若要继续显著压缩 `execution_store_root`，下一轮需要先补“checkpoint + commit log 恢复具体高度”的能力；当前 `stale/non-contiguous` 恢复仍依赖热窗口内按高度直接读取 snapshot，因此暂不能安全清空所有非最新热高度 snapshot。

## 2026-05-16 18:25:29 CST / runtime_engineer
- 完成内容: 在 `LocalCasStore` 增加大 blob 的磁盘透明压缩：`content_hash` 仍然基于原始 payload，`get/get_verified` 仍返回原始 bytes，但对压缩后更小的 `.blob` 自动以压缩 envelope 落盘，直接覆盖 execution bridge snapshots/journals 与 sidecar blobs 的存储路径。
- 完成内容: 补齐 `oasis7_distfs` 定向回归，覆盖 CAS roundtrip、透明压缩、以及 challenge hash mismatch 路径，确认压缩落盘不会破坏 `get_verified` / sampling / hash mismatch 检测语义；同时复跑 `oasis7_chain_runtime` 的 storage-profile 定向回归，确认 execution bridge 行为不变。
- 遗留事项: 透明压缩先解决“同样保留范围下单 blob 体积过大”的问题，但不改变 retained-height exact restore 的结构性依赖；若要再继续大幅下降 `execution_store_root`，仍需补齐 checkpoint + commit-log 级的高度恢复能力。

## 2026-05-16 18:58:44 CST / runtime_engineer
- 完成内容: 根据 commit 前 review 结果，修正透明压缩实现的两个关键边界：不再使用第二份 metadata 文件，而是把 `content_hash + raw_size` 一并写入同一个 compressed blob header，避免双写状态漂移；同时新增“raw blob 看起来像压缩格式也不能误解码”的回归。
- 完成内容: 根据 review 结果修正 replay planner 的热窗口判定，不再写死 `32`，而是按当前 record 中真实保留的 snapshot/journal 起点推导 `no checkpoint` 场景的 retained hot window，并新增 `execution_replay_plan_uses_actual_retained_hot_window_without_checkpoint` 覆盖 `64` 窗口配置。
- 完成内容: 复跑 `cargo test -p oasis7_distfs cas_ -- --nocapture`、`cargo test -p oasis7 --bin oasis7_chain_runtime execution_replay_plan_uses_actual_retained_hot_window_without_checkpoint -- --nocapture`、`cargo test -p oasis7 --bin oasis7_chain_runtime uses_storage_profile -- --nocapture`、`./scripts/doc-governance-check.sh` 与 `git diff --check`，结果均通过。
- 遗留事项: 若后续要继续显著压缩 `execution_store_root`，下一阶段仍需把 retained-height exact restore 从“热窗口整份 snapshot”迁到“checkpoint + canonical commit log replay”，这不在本轮 PR 范围内。
