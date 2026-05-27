# oasis7 Simulator：Viewer 快速定位 Agent 按钮（项目管理文档）

- 对应设计文档: `doc/world-simulator/viewer/viewer-agent-quick-locate.design.md`
- 对应需求文档: `doc/world-simulator/viewer/viewer-agent-quick-locate.prd.md`

审计轮次: 5

## 任务拆解（含 PRD-ID 映射）
- [x] QAG1：输出设计文档（`doc/world-simulator/viewer/viewer-agent-quick-locate.prd.md`）与项目管理文档（本文件）
- [x] QAG2：新增快速定位 Agent 动作（优先当前 Agent，否则首个 Agent）
- [x] QAG3：接入按钮与多语言文案（Egui Event Link + 兼容旧 UI）
- [x] QAG4：补充测试并完成回归验证（`test_tier_required`）
- [x] QAG5：更新总项目文档与开发日志，完成任务收口

## 依赖
- `crates/oasis7_viewer/src/selection_linking.rs`
- `crates/oasis7_viewer/src/selection_linking/tests.rs`
- `crates/oasis7_viewer/src/egui_right_panel.rs`
- `crates/oasis7_viewer/src/ui_locale_text.rs`
- `doc/world-simulator.project.md`
- `doc/devlog/README.md`

## 状态
- 当前阶段：已完成
- 最近更新：补齐旧 UI 路径系统调度（`handle_quick_locate_agent_button`）并复跑回归（2026-02-15）
