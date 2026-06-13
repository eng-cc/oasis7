# oasis7: 启动链路脚本迁移（2026-02-28）（项目管理）

- 对应设计文档: `doc/testing/launcher/launcher-chain-script-migration-2026-02-28.design.md`
- 对应需求文档: `doc/testing/launcher/launcher-chain-script-migration-2026-02-28.prd.md`

审计轮次: 5

## 任务拆解（含 PRD-ID 映射）
- [x] LAUNCHMIG-1 (PRD-TESTING-LAUNCHER-SCRIPT-001/003): 完成专题设计文档与项目管理文档建档。
- [x] LAUNCHMIG-2 (PRD-TESTING-LAUNCHER-SCRIPT-001/002): 迁移 `run-game-test.sh`、`viewer-release-qa-loop.sh` 到 `oasis7_game_launcher`。
- [x] LAUNCHMIG-3 (PRD-TESTING-LAUNCHER-SCRIPT-002/003): 为 `s10-five-node-game-soak.sh`、`p2p-longrun-soak.sh` 添加显式阻断与迁移提示。
- [x] LAUNCHMIG-4 (PRD-TESTING-LAUNCHER-SCRIPT-003): 完成手册口径同步、项目状态更新与任务日志收口。
- [x] LAUNCHMIG-5 (PRD-TESTING-004): 专题文档按 strict schema 人工重写，并切换命名到 `.prd.md/.project.md`。
- [x] LAUNCHMIG-6 (PRD-TESTING-LAUNCHER-SCRIPT-004): 修复 Viewer 静态资源目录兼容（`viewer-release-qa-loop` 透传 `--viewer-static-dir` + `oasis7_game_launcher` 支持 `OASIS7_GAME_STATIC_DIR` 覆盖默认目录）。

## 依赖
- doc/testing/launcher/launcher-chain-script-migration-2026-02-28.prd.md
- `scripts/run-game-test.sh`
- `scripts/viewer-release-qa-loop.sh`
- `scripts/s10-five-node-game-soak.sh`
- `scripts/p2p-longrun-soak.sh`
- `crates/oasis7/src/bin/oasis7_game_launcher.rs`
- `crates/oasis7/src/bin/oasis7_chain_runtime.rs`
- `testing-manual.md`
- `doc/testing/prd.md`
- `doc/testing/project.md`

## 状态
- 更新日期：2026-03-07
- 当前阶段：已完成
- 阻塞项：无
- 下一步：无（当前专题已收口）
