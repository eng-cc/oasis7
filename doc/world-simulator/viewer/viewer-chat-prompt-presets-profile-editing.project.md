# Chat Panel 预设区扩展 Agent Prompt 字段编辑（项目管理文档）

- 对应设计文档: `doc/world-simulator/viewer/viewer-chat-prompt-presets-profile-editing.design.md`
- 对应需求文档: `doc/world-simulator/viewer/viewer-chat-prompt-presets-profile-editing.prd.md`

审计轮次: 5

## 任务拆解（含 PRD-ID 映射）
- [x] VCPE1 输出设计文档（`doc/world-simulator/viewer/viewer-chat-prompt-presets-profile-editing.prd.md`）
- [x] VCPE2 输出项目管理文档（本文件）
- [x] VCPE3 在 Chat Panel 预设区新增 `system/short/long` 三字段编辑 UI
- [x] VCPE4 接入 `prompt_control.apply` 提交链路（按选中 Agent）
- [x] VCPE5 更新测试与手册，并执行 `test_tier_required` 回归
- [x] VCPE6 回写状态与 devlog，完成收口提交

## 依赖
- `crates/oasis7_viewer/src/egui_right_panel_chat.rs`
- `crates/oasis7_viewer/src/main.rs`
- `doc/world-simulator/viewer/viewer-manual.manual.md`
- `doc/world-simulator/viewer/viewer-chat-prompt-presets-profile-editing.prd.md`

## 状态
- 当前阶段：VCPE1-VCPE6 已全部完成。
- 下一步：如需进一步防误操作，可在同一折叠区补充 `preview/rollback` 交互与回执态展示。
- 最近更新：2026-02-16（收口）。
