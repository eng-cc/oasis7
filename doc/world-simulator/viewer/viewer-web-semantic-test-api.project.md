# Viewer Web 语义化测试 API（项目管理）

- 对应设计文档: `doc/world-simulator/viewer/viewer-web-semantic-test-api.design.md`
- 对应需求文档: `doc/world-simulator/viewer/viewer-web-semantic-test-api.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）
- [x] WTA-0 文档建档：设计文档 + 项目管理文档
- [x] WTA-1 `viewer_automation` 支持运行时步骤入队
- [x] WTA-2 `window.__AW_TEST__` wasm 桥接与命令队列
- [x] WTA-3 `app_bootstrap` 接入命令消费/状态发布系统
- [x] WTA-4 回归测试与编译验证（`oasis7_viewer`）
- [x] WTA-5 状态回写与 devlog 收口
- [x] WTA-6 `testing-manual.md` S6 agent-browser 示例切换到 `window.__AW_TEST__`
- [x] WTA-7 `getState()` 增补相机语义字段（`cameraMode/cameraRadius/cameraOrthoScale`）
- [x] WTA-8 (PRD-WTA-R1-001) [test_tier_required]：round-1 补齐文档建模（人类高频操作缺口盘点 + 语义步骤设计 + 任务拆解）
- [x] WTA-9 (PRD-WTA-R1-002) [test_tier_required]：扩展 `viewer_automation` round-1 语义步骤（`panel/module/focus_selection/material_variant`）并补齐解析/映射测试
- [x] WTA-10 (PRD-WTA-R1-003) [test_tier_required]：执行 `oasis7_viewer` 定向回归、更新 PRD/project 状态与 devlog 收口
- [x] WTA-11 (PRD-WTA-R2-001) [test_tier_required]：round-2 补齐文档建模（`top_panel/locale/layout` 语义步骤设计 + 任务拆解）
- [x] WTA-12 (PRD-WTA-R2-002) [test_tier_required]：扩展 `viewer_automation` round-2 语义步骤（`top_panel/locale/layout`）并补齐解析/映射测试
- [x] WTA-13 (PRD-WTA-R2-003) [test_tier_required]：执行 round-2 定向回归、更新手册示例与文档状态收口
- [x] WTA-14 (PRD-WTA-R3-001) [test_tier_required]：round-3 补齐文档建模（`chat/prompt` 语义步骤设计 + 任务拆解）
- [x] WTA-15 (PRD-WTA-R3-002) [test_tier_required]：扩展 `viewer_automation` round-3 语义步骤（`chat/prompt`）并补齐解析/映射测试
- [x] WTA-16 (PRD-WTA-R3-003) [test_tier_required]：执行 round-3 定向回归、更新手册示例与文档状态收口
- [x] WTA-17 (PRD-WTA-R4-001) [test_tier_required]：round-4 补齐文档建模（`timeline_seek/filter/jump` 语义步骤设计 + 任务拆解）
- [x] WTA-18 (PRD-WTA-R4-002) [test_tier_required]：扩展 `viewer_automation + web_test_api` round-4 语义步骤（timeline + `sendControl.seek`）并补齐定向测试
- [x] WTA-19 (PRD-WTA-R4-003) [test_tier_required]：执行 round-4 定向回归、更新手册示例与文档状态收口

## 依赖
- `doc/world-simulator/viewer/viewer-web-semantic-test-api.design.md`
- doc/world-simulator/viewer/viewer-web-semantic-test-api.prd.md
- `crates/oasis7_viewer/src/viewer_automation.rs`
- `crates/oasis7_viewer/src/app_bootstrap.rs`
- `crates/oasis7_viewer/src/main.rs`
- `crates/oasis7_viewer/src/auto_focus.rs`
- `crates/oasis7_viewer/src/main_ui_runtime.rs`
- `crates/oasis7_viewer/src/i18n.rs`
- `crates/oasis7_viewer/src/timeline_controls.rs`
- `crates/oasis7_viewer/src/web_test_api.rs`
- `doc/world-simulator/viewer/viewer-manual.manual.md`
- `testing-manual.md`

## 状态
- 当前阶段：WTA 全部完成
- 下一步：可选继续 round-5（timeline 调整按钮与恢复动作语义补齐）
- 最近更新：2026-03-08（WTA-19 完成，round-4 回归与手册收口完成）
