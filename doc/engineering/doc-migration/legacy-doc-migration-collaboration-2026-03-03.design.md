# 文档迁移并行协作方案设计（2026-03-03）

- 对应需求文档: `doc/engineering/doc-migration/legacy-doc-migration-collaboration-2026-03-03.prd.md`
- 对应项目管理文档: `doc/engineering/doc-migration/legacy-doc-migration-collaboration-2026-03-03.project.md`

## 1. 设计定位
定义 legacy 文档迁移的协作分层、分工边界、执行链路与复核约束，确保多人并行迁移时无范围冲突、无口径漂移。

## 2. 设计结构
- 协调层：由迁移协调人维护待迁移池、目录切片、燃尽统计与冲突仲裁。
- 执行层：Owner-A/B/C/D 只处理各自互斥目录范围，逐篇阅读并人工重写。
- 复核层：Reviewer 抽检内容保真、命名规范、引用完整性与门禁结果。

## 3. 关键接口 / 入口
- 待迁移池：2026-03-03 legacy migration backlog snapshot（后续已删除；完成态见 `doc/engineering/doc-migration/legacy-doc-migration-collaboration-2026-03-03.project.md`）
- 当前迁移规则：`doc/engineering/doc-governance/doc-structure-standard.design.md` 与 `doc/engineering/workflow/source-of-truth.md`
- 模块追踪：`doc/engineering/prd.md`、`doc/engineering/project.md`、`doc/engineering/prd.index.md`
- 校验门禁：`scripts/doc-governance-check.sh`
- 时序证据：`doc/devlog/YYYY-MM-DD.md`

## 4. 约束与边界
- 范围互斥：同一文档只能被单一 owner 领取。
- 内容保真：禁止脚本批量改写正文，必须逐篇人工重写。
- 闭环提交：每任务单独提交，并同步回写模块入口与日志。

## 5. 设计演进计划
- Phase-0：冻结待迁移池快照与目录切片。
- Phase-1：并行执行迁移并持续抽检。
- Phase-2：收口燃尽、替代链与引用修复。
