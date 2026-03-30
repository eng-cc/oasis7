# engineering 文档索引

审计轮次: 6

## 入口
- PRD: `doc/engineering/prd.md`
- 设计总览: `doc/engineering/design.md`
- 标准执行入口: `doc/engineering/project.md`
- 文件级索引: `doc/engineering/prd.index.md`

## 模块职责
- 维护工程治理规则、文档组织标准与执行门禁。
- 跟踪文档迁移、文件级索引可达性与角色协作规范。
- 承接 engineering 治理趋势、季度审查与模板化流程沉淀。

## 主题文档
- `governance/`：工程治理趋势、季度审查、修复模板与模块收口专题。
- `doc-migration/`：历史文档迁移协作方案、清单与批次记录。
- `prd-review/`：PRD 审读机制、清单与检查模板。
- `self-evolution/`：仓库内文件化项目管理、自我进化 memory/backlog、signal inbox 与 stage/gate 专题。

## 近期专题
- `doc/engineering/doc-structure-standard.prd.md`
- `doc/engineering/rust-1200-line-root-cause-governance-2026-03-29.prd.md`
- `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.prd.md`
- `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.prd.md`
- `doc/engineering/governance/engineering-governance-trend-tracking-2026-03-11.prd.md`
- `doc/engineering/governance/engineering-quarterly-governance-review-cycle-2026-03-11.prd.md`
- `doc/engineering/doc-migration/legacy-doc-migration-collaboration-2026-03-03.prd.md`
- `doc/engineering/prd-review/prd-full-system-audit-2026-03-03.prd.md`

## 根目录收口
- 模块根目录主入口保留：`README.md`、`prd.md`、`design.md`、`project.md`、`prd.index.md`。
- 其余专题文档按主题下沉到 `governance/`、`doc-migration/`、`prd-review/`、`self-evolution/`。

## 维护约定
- 工程治理规则、目录标准或角色协作口径变化时，优先更新 engineering PRD。
- 新增专题后，需同步回写 `doc/engineering/prd.index.md` 与本目录索引。
- 不再保留 `doc/engineering/archive/` 归档目录。
