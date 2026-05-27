# oasis7: 启动器 Viewer 鉴权自动继承 Node 配置（2026-03-02）（项目管理）

- 对应设计文档: `doc/testing/launcher/launcher-viewer-auth-node-config-autowire-2026-03-02.design.md`
- 对应需求文档: `doc/testing/launcher/launcher-viewer-auth-node-config-autowire-2026-03-02.prd.md`

审计轮次: 5

## 任务拆解（含 PRD-ID 映射）
- [x] AUTOWIRE-1 (PRD-TESTING-LAUNCHER-AUTH-001): 完成专题设计文档与项目管理文档建档。
- [x] AUTOWIRE-2 (PRD-TESTING-LAUNCHER-AUTH-001/002): `oasis7_game_launcher` 在 Web `index.html` 注入 Viewer 鉴权配置并补测试。
- [x] AUTOWIRE-3 (PRD-TESTING-LAUNCHER-AUTH-002/003): `oasis7_viewer` 增加 wasm 注入读取与 native `config.toml[node]` 回退并补测试。
- [x] AUTOWIRE-4 (PRD-TESTING-LAUNCHER-AUTH-003): 完成定向回归、文档状态与 devlog 收口。
- [x] AUTOWIRE-5 (PRD-TESTING-004): 专题文档按 strict schema 人工重写，并切换命名到 `.prd.md/.project.md`。
- [x] AUTOWIRE-6 (PRD-TESTING-LAUNCHER-AUTH-004): `oasis7_viewer` 修复 `AgentChatError` 误降级连接状态（发送聊天失败不再显示“连接异常”），补状态机回归测试并收口文档/devlog。

## 依赖
- doc/testing/launcher/launcher-viewer-auth-node-config-autowire-2026-03-02.prd.md
- `crates/oasis7/src/bin/oasis7_game_launcher.rs`
- `crates/oasis7/src/bin/oasis7_game_launcher/oasis7_game_launcher_tests.rs`
- `crates/oasis7_viewer/src/egui_right_panel_chat_auth.rs`
- `crates/oasis7_viewer/src/egui_right_panel_chat_tests.rs`
- `crates/oasis7_viewer/src/main_connection.rs`
- `crates/oasis7_viewer/src/tests.rs`
- `crates/oasis7_viewer/Cargo.toml`
- `config.toml` (`[node] private_key/public_key`)
- `doc/testing/prd.md`
- `doc/testing/project.md`
- `doc/devlog/README.md`
- `doc/devlog/README.md`

## 状态
- 更新日期：2026-03-08
- 当前阶段：已完成
- 阻塞项：无
- 下一步：无（当前专题已收口）。
