# viewer-web-playability-unblock-2026-02-26 项目管理

- 对应设计文档: `doc/world-simulator/viewer/viewer-web-playability-unblock-2026-02-26.design.md`
- 对应需求文档: `doc/world-simulator/viewer/viewer-web-playability-unblock-2026-02-26.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）
- [x] T0 建立设计文档与项目管理文档
- [x] T1 修复 `web_test_api` 的 `runSteps`/`sendControl` 入参契约，消除类型不匹配 panic。
- [x] T2 增加 wasm + Player 模式自动 `Play`，确保连接后默认可推进。
- [x] T3 修复 `scripts/run-game-test.sh` 的 WS 就绪探针，消除 `HandshakeIncomplete` 假故障。
- [x] T4 运行回归测试并回写文档/日志。

## 依赖
- `doc/world-simulator/viewer/viewer-web-playability-unblock-2026-02-26.prd.md`
- `crates/oasis7_viewer/src/web_test_api.rs`
- `crates/oasis7_viewer/src/headless.rs`
- `crates/oasis7_viewer/src/app_bootstrap.rs`
- `scripts/run-game-test.sh`

## 状态
- 当前阶段：已完成（T0~T4）
- 最近更新：2026-02-26
