# Viewer 3D 可视化暂停与用户交互分支暂存设计

- 对应需求文档: `doc/world-simulator/viewer/viewer-3d-pause-user-interaction-hold-2026-04-01.prd.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-3d-pause-user-interaction-hold-2026-04-01.project.md`

审计轮次: 1

## 目标
- 将 Viewer 当前工作流从“3D 与正式交互并行推进”改为“3D 暂停、非 3D 主链路优先”。
- 定义用户交互分支的 `hold` 语义，避免后续被误当成可继续开发的 active branch。

## 范围
- 覆盖 3D workstream 的暂停边界、允许修改范围、恢复门禁。
- 覆盖模块主 PRD / project / index 的回写要求。
- 不覆盖具体 3D 代码重构或删除。

## 设计
- 工作流状态机：
  - `active`: 只有非 3D 主链路处于当前 active delivery。
  - `paused`: 新的 3D 可视化需求统一打入暂停池。
  - `hold`: 当前用户交互分支只保留为暂存参考，不再堆叠新实现。
  - `resume_review`: 只有制作人显式触发恢复 review，才允许准备恢复。
- 暂停范围：
  - 3D scene、camera、rendering、material、lighting、post-process、3D-only visual polish。
  - 与这些目标直接耦合的“仅为 3D 体验而做”的 UI / automation / capture 扩展。
- 允许继续的修改：
  - 文档治理、引用修复、编译兼容、避免主链路腐烂的最小维护。
  - 为 `software_safe`、launcher、runtime interaction、formal gameplay 直接服务的修改。
- 恢复门禁：
  - 当前主链路稳定。
  - QA 资源已准备。
  - 恢复范围明确到具体专题，而不是“全部 3D”。
  - `producer_system_designer` 在正式文档中显式解除暂停。

## 风险
- 若允许“顺手继续改一点 3D”，暂停策略会被逐步掏空。
- 若不保留 hold 参考，后续恢复会重复踩回原来的设计和边界问题。

## 验证
- `./scripts/doc-governance-check.sh`
- `git diff --check`
- 文档检索确认 `paused / hold / resume gate` 口径存在且一致
