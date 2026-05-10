# PowerStorage 全量下线（项目管理，2026-03-06）

- 对应设计文档: `doc/world-simulator/kernel/power-storage-complete-removal-2026-03-06.design.md`
- 对应需求文档: `doc/world-simulator/kernel/power-storage-complete-removal-2026-03-06.prd.md`

审计轮次: 3

## 任务拆解（含 PRD-ID 映射）
- [x] T1 (PSR-001) [test_tier_required]: 新建 PRD 与项目文档，冻结删除范围、验收口径、风险与决策记录。
- [x] T2 (PSR-002) [test_tier_required]: 删除 simulator 侧 `PowerStorage` 模型/动作/事件/初始化/回放与相关测试。
- [x] T3 (PSR-003) [test_tier_required]: 删除 viewer 侧 `power_storage` 渲染、selection、自动化、配置与相关测试。
- [x] T4 (PSR-004) [test_tier_required]: 更新 scripts/视觉评审模板与 UI 评审卡，去除 `power_storage` 检查项。
- [x] T5 (PSR-002/003/004) [test_tier_required]: 执行回归、补 `doc/devlog/2026-03-06.md`、提交收口。

## 依赖
- `doc/world-simulator/kernel/power-storage-complete-removal-2026-03-06.prd.md`
- `crates/oasis7/src/simulator/*`
- `crates/oasis7_viewer/src/*`
- 历史上的 `scripts/validate-viewer-theme-pack.py`
- 历史上的 `scripts/viewer-texture-inspector.sh`
- 历史上的 `scripts/viewer-texture-inspector-lib.sh`
- 历史上的 `historical removed standard_3d viewer doc set: visual-review-score-card`
- `doc/ui_review_result/card_2026_03_06_11_50_29.md`

## 状态
- 更新日期: 2026-03-06
- 当前阶段: completed
- 当前任务: none
- 风险备注: `test_tier_required` 存在已知 unrelated builtin identity manifest 校验失败，执行时需单独记录。
