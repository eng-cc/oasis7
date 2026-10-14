# Runtime Launcher Profile Consistency Evidence（2026-03-11）

以下是 2026-03-11 的 T7.4 wrapper/profile 历史证据与当时 T7.5 交接。原 Task 和旧 `.tmp` 原始输出未唯一核实，原内容保留；不声称当前 wrapper 已复跑、当前发行放行或恢复旧收尾义务。

审计轮次: 1

## Meta
- 日期: `2026-03-11`
- 执行角色: `viewer_engineer`
- 关联任务: `TASK-WORLD_RUNTIME-033 / T7.4`
- 关联 PRD-ID: `PRD-WORLD_RUNTIME-014/015`
- 结论: `pass`

## 输入
- bundle 构建命令: `./scripts/build-game-launcher-bundle.sh --profile dev --out-dir .tmp/t74_bundle_20260311-005939 --web-dist crates/oasis7_viewer/dist`
- bundle 产物目录: `.tmp/t74_bundle_20260311-005939`
- wrapper 实物:
  - `.tmp/t74_bundle_20260311-005939/run-game.sh`
  - `.tmp/t74_bundle_20260311-005939/run-web-launcher.sh`
  - `.tmp/t74_bundle_20260311-005939/run-chain-runtime.sh`
- 动态 trace:
  - `.tmp/t74_bundle_20260311-005939/run-game.trace`
  - `.tmp/t74_bundle_20260311-005939/run-web.trace`
  - `.tmp/t74_bundle_20260311-005939/run-chain.trace`

## 静态核对
- `run-game.sh` 从 `OASIS7_CHAIN_STORAGE_PROFILE` 组装 `--chain-storage-profile <value>`。
- `run-web-launcher.sh` 从 `OASIS7_CHAIN_STORAGE_PROFILE` 组装 `--chain-storage-profile <value>`。
- `run-chain-runtime.sh` 从 `OASIS7_CHAIN_STORAGE_PROFILE` 组装 `--storage-profile <value>`。

## 动态核对
- `OASIS7_CHAIN_STORAGE_PROFILE=soak_forensics bash -x ./run-game.sh --help`：trace 显示最终执行 `oasis7_game_launcher ... --chain-storage-profile soak_forensics --help`。
- `OASIS7_CHAIN_STORAGE_PROFILE=release_default bash -x ./run-web-launcher.sh --help`：trace 显示最终执行 `oasis7_web_launcher --chain-storage-profile release_default --help`。
- `OASIS7_CHAIN_STORAGE_PROFILE=dev_local bash -x ./run-chain-runtime.sh --help`：trace 显示最终执行 `oasis7_chain_runtime --storage-profile dev_local --help`。

## 结论
- bundle wrapper、launcher CLI 与 `oasis7_chain_runtime` 的 storage profile 枚举口径一致。
- `OASIS7_CHAIN_STORAGE_PROFILE` 在 bundle 入口层不会硬编码默认值，只会在显式设置时注入参数，符合 PRD/T6.4 约束。
- T7.4 当前可视为已补齐 launcher / bundle / runtime 的 profile 一致性证据；下一步转入 T7.5 的最终文档收口。
