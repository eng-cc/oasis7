# Viewer 控制面拆分：回放/Live 分离（2026-02-27）项目管理

- 对应设计文档: `doc/world-simulator/viewer/viewer-control-plane-split-live-playback-2026-02-27.design.md`
- 对应需求文档: `doc/world-simulator/viewer/viewer-control-plane-split-live-playback-2026-02-27.prd.md`

审计轮次: 6

## 任务拆解（含 PRD-ID 映射）
- [x] T0 建立设计文档与项目管理文档
- [x] T1 协议层与 server/live 控制处理拆分（含兼容桥接）
- [x] T2 viewer 控制发送按 profile 路由并收敛 seek 发送
- [x] T3 测试、文档收口与 devlog 收口
- [x] T4 live seek 语义显式化：dispatch 结果区分 unsupported / send-failed，`__AW_TEST__` 公开 `controlProfile` 并动态裁剪 seek，timeline / egui / automation 与 profile 行为对齐

## 依赖
- `doc/world-simulator/viewer/viewer-control-plane-split-live-playback-2026-02-27.prd.md`
- `crates/oasis7_proto/src/viewer.rs`
- `crates/oasis7/src/viewer/{protocol.rs,server.rs,live_split_part2.rs,mod.rs}`
- `crates/oasis7_viewer/src/{main.rs,app_bootstrap.rs,main_connection.rs,timeline_controls.rs,egui_right_panel.rs,egui_right_panel_controls.rs,egui_right_panel_player_experience.rs,egui_right_panel_player_guide.rs,viewer_automation.rs,web_test_api.rs,headless.rs}`
- `doc/devlog/README.md`

## 状态
- 最近更新：2026-03-18（ROUND-006 I44-001 live seek 语义显式化）
- 当前阶段：已完成（T0~T4）
- 备注：与“live 禁用 seek（P2P 不可回退）”策略保持一致；Viewer UI、automation 与 `__AW_TEST__` 现在都按 `controlProfile` 对齐 seek 暴露边界与错误签名。
