# viewer-web-runtime-fatal-surfacing-2026-03-12 项目管理

- 对应设计文档: `doc/world-simulator/viewer/viewer-web-runtime-fatal-surfacing-2026-03-12.design.md`
- 对应需求文档: `doc/world-simulator/viewer/viewer-web-runtime-fatal-surfacing-2026-03-12.prd.md`

审计轮次: 7
## 任务拆解（含 PRD-ID 映射）
- [x] T0 建立 PRD / Design / Project 文档，并回写模块主文档与索引。
- [x] T1 在 `web_test_api` 补齐浏览器 runtime fatal 透出。
- [x] T2 在 `run-game-test-ab.sh` 增加 `lastError` 快失败分支。
- [x] T3 同步 `viewer-manual.md` 与 `testing-manual.md` 的执行与排障口径。
- [x] T4 运行定向验证并回写 devlog / commit 收口。
- [x] T5 在 `web_test_api` runtime diagnostic hook 增加“已知图形 fatal 仅自动 reload 一次”恢复逻辑，收敛手动 reopen。
- [x] T6 追加 `web_test_api` 快照即时落盘修复，并复验“首开 fatal 不再伪装成 connecting”。

## 依赖
- `doc/world-simulator/viewer/viewer-web-runtime-fatal-surfacing-2026-03-12.prd.md`
- `crates/oasis7_viewer/src/web_test_api.rs`
- `scripts/run-game-test-ab.sh`
- `doc/world-simulator/viewer/viewer-manual.manual.md`
- `testing-manual.md`

## 状态
- 当前阶段：已完成（T0~T6）
- 最近更新：2026-03-12
- 阻塞项：SwiftShader/WebGL2 仍可能存在其他图形差异；本轮已完成“已知 fatal 一次自动 reload”、错误透出与脚本快失败。
