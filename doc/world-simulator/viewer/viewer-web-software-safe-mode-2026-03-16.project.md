# viewer-web-software-safe-mode-2026-03-16 项目管理

- 对应设计文档: `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.design.md`
- 对应需求文档: `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] software-safe-single-web-entry-contract (PRD-WORLD_SIMULATOR-039) [test_tier_required]: 完成 `software_safe` 源码入口到 `viewer` canonical Web 入口 contract 的收口，并保留 compat alias。 Trace: .pm/tasks/task_a2a5c83cb80f4a6f9deb3dfcb5ca8377.yaml
- [x] software-safe-canonical-gameplay-and-regression (PRD-WORLD_SIMULATOR-039) [test_tier_required]: 保留 `viewer` canonical gameplay summary / blocked-handoff / prompt-chat 契约，并补齐 freshness gate 与 repo-owned browser regression。 Trace: .pm/tasks/task_5eddd21920854c20a769915ac37df977.yaml
- [x] software-safe-single-entry-doc-surface-cleanup (PRD-WORLD_SIMULATOR-039) [test_tier_required]: 删除标准 Viewer 跳转及其当前文档口径，并同步 Viewer 手册、testing 手册、evidence 与站点镜像到单入口真值。 Trace: .pm/tasks/task_3c457e5583984f7da7c81620e4297009.yaml

## 依赖
- `testing-manual.md`
- `doc/world-simulator/viewer/viewer-manual.manual.md`
- `scripts/run-viewer-web.sh`
- `scripts/viewer-primary-web-entry-regression.sh`

## 状态
- 更新日期: 2026-05-18
- 当前状态: done
- 下一任务: `none`
- 最新完成: `viewer-formal-lane-truth-align`（已回写 formal gameplay PASS 与 `--no-llm` observer-only 证据边界，旧 `active/T6` 状态归零。）
