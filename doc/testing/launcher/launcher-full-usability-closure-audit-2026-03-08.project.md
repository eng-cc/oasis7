# oasis7: 启动器全功能可用性审查与闭环验收（2026-03-08）（项目管理）

- 对应设计文档: `doc/testing/launcher/launcher-full-usability-closure-audit-2026-03-08.design.md`
- 对应需求文档: `doc/testing/launcher/launcher-full-usability-closure-audit-2026-03-08.prd.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] LAUNCHREV-1 (PRD-TESTING-LAUNCHER-REVIEW-001): 完成专题 PRD 与项目管理文档建档，明确审查范围/分级标准/追溯口径。
- [x] LAUNCHREV-2 (PRD-TESTING-LAUNCHER-REVIEW-001/002/003): 执行启动器定向回归与脚本行为审查（迁移入口 + 阻断入口 + 参数兼容）。
- [x] LAUNCHREV-3 (PRD-TESTING-LAUNCHER-REVIEW-002): 执行真实 Web 闭环（`oasis7_game_launcher + agent-browser`）并归档证据。
- [x] LAUNCHREV-4 (PRD-TESTING-LAUNCHER-REVIEW-001/003): 输出可用性分级结论、风险项与后续动作，完成文档/devlog 收口。

## 依赖
- doc/testing/launcher/launcher-full-usability-closure-audit-2026-03-08.prd.md
- `testing-manual.md`
- `doc/testing/manual/web-ui-agent-browser-closure-manual.prd.md`
- `crates/oasis7/src/bin/oasis7_game_launcher.rs`
- `crates/oasis7/src/bin/oasis7_game_launcher/oasis7_game_launcher_tests.rs`
- `crates/oasis7_client_launcher/src/main.rs`
- `scripts/run-game-test.sh`
- 历史已删除：`scripts/viewer-release-qa-loop.sh`
- `scripts/s10-five-node-game-soak.sh`
- `scripts/p2p-longrun-soak.sh`
- `doc/testing/prd.md`
- `doc/testing/project.md`

## 状态
- 更新日期：2026-03-08
- 当前阶段：已完成（LAUNCHREV-1~4）
- 阻塞项：无
- 审查结论：条件通过（`oasis7_game_launcher`/脚本链路可用；Web 闭环在 `--viewer-static-dir web` 存在协议错配风险，切换到 `output/release/game-launcher-local/web` 后通过）。
- 下一步：发布与手工验收统一使用显式静态目录（参数优先）并保留软件渲染 fail-fast 门禁。
