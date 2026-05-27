# oasis7：启动器人工测试清单（2026-03-10）（项目管理）

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] LMTC-1 (PRD-TESTING-LAUNCHER-MANUAL-001/003): 完成专题 PRD / project 建档，明确人工测试目标、优先级与结论口径。
- [x] LMTC-2 (PRD-TESTING-LAUNCHER-MANUAL-001/002/003): 编写启动器人工测试清单，覆盖 `P0/P1/P2`、证据要求与 verdict 建议。
- [x] LMTC-3 (PRD-TESTING-LAUNCHER-MANUAL-002/003): 对齐 `testing-manual.md`、模块项目管理文档与 PRD 索引，建立互链与追溯。
- [x] LMTC-4 (PRD-TESTING-LAUNCHER-MANUAL-003): 完成文档校验与 devlog 回写，收口本轮任务。
- [x] LMTC-5 (PRD-TESTING-LAUNCHER-MANUAL-001/002/003): 基于实际 Web/GUI Agent 闭环结果，将 `Explorer / Transfer` 升级为细粒度子能力矩阵，补充数据前置、结果分级与归因入口。

## 依赖
- `doc/testing/launcher/launcher-manual-test-checklist-2026-03-10.prd.md`
- `doc/testing/project.md`
- `doc/testing/prd.index.md`
- `testing-manual.md`
- `doc/testing/launcher/launcher-full-usability-closure-audit-2026-03-08.prd.md`
- `doc/testing/manual/web-ui-agent-browser-closure-manual.prd.md`
- `doc/devlog/README.md`

## 状态
- 更新日期：2026-03-10
- 当前阶段：已完成
- 阻塞项：无
- 下一步：后续若出现启动器逃逸缺陷，优先补充对应子能力矩阵与失败签名，不再只追加粗粒度 `P1/P2` 项。
