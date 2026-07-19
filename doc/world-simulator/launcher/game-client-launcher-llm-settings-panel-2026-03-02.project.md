# 客户端启动器 LLM 设置入口（2026-03-02）项目管理

- 对应设计文档: `doc/world-simulator/launcher/game-client-launcher-llm-settings-panel-2026-03-02.design.md`
- 对应需求文档: `doc/world-simulator/launcher/game-client-launcher-llm-settings-panel-2026-03-02.prd.md`

审计轮次: 6
## 任务拆解（含 PRD-ID 映射）
- [x] T0 (PRD-WORLD_SIMULATOR-001)：建档（设计文档 + 项目管理文档）。
- [x] T1 (PRD-WORLD_SIMULATOR-002)：功能实现（设置按钮/窗口 + `config.toml [llm]` 三字段读写 + 单测）。
- [x] T2 (PRD-WORLD_SIMULATOR-003)：回归与收口（测试、文档完成态、devlog、提交结项）。
- [x] T3 (PRD-WORLD_SIMULATOR-007)：设置窗口升级为完整设置中心（游戏/区块链/LLM 一体化配置入口）。

## 依赖
- `doc/world-simulator/launcher/game-client-launcher-llm-settings-panel-2026-03-02.design.md`
- doc/world-simulator/launcher/game-client-launcher-llm-settings-panel-2026-03-02.prd.md
- `crates/oasis7_client_launcher/src/main.rs`
- `crates/oasis7_client_launcher/src/main_tests.rs`
- `config.toml` 小写 TOML 结构约定（`[llm]`）
- `doc/world-simulator/llm/llm-agent-behavior.prd.md`
- `config.example.toml`

## 状态
- 最近更新：2026-03-06（ROUND-005 I5-001 字段补齐）
- 当前阶段：已完成（T0~T3 已完成）。
- 当前任务：无。
