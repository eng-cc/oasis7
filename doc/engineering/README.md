# engineering 文档索引

审计轮次: 6

## 入口
- PRD: `doc/engineering/prd.md`
- 设计总览: `doc/engineering/design.md`
- 标准执行入口: `doc/engineering/project.md`
- 文件级索引: `doc/engineering/prd.index.md`

## 从这里开始
- 想看工程治理边界、验收条件与长期规则：`doc/engineering/prd.md`
- 想看当前工程治理任务、当前收口策略与下一步：`doc/engineering/project.md`
- 想精确定位某个治理专题：`doc/engineering/prd.index.md`

## 模块职责
- 维护工程治理规则、文档组织标准与执行门禁。
- 跟踪文档迁移、文件级索引可达性与角色协作规范。
- 承接 engineering 治理趋势、季度审查与模板化流程沉淀。

## 主题文档
- `doc-governance/`：文档组织规范、入口减重与早期文档治理收口专题。
- `rust-governance/`：Rust 体量治理、结构切片与超限 burn-down 专题。
- `governance/`：工程治理趋势、季度审查、修复模板与模块收口专题。
- `doc-migration/`：历史文档迁移协作方案、清单与批次记录。
- `prd-review/`：PRD 审读机制、清单与检查模板。
- `self-evolution/`：仓库内文件化项目管理、自我进化 memory/backlog、signal inbox 与 stage/gate 专题。

## 高频专题
- 文档治理分两阶段：`doc/engineering/doc-governance/doc-surface-area-governance-2026-04-10.prd.md` 负责默认阅读面减重，`doc/engineering/doc-governance/doc-corpus-maintenance-governance-2026-04-17.prd.md` 负责入口减重后的存量维护成本治理；结构落位规则仍由 `doc/engineering/doc-governance/doc-structure-standard.prd.md` 统一维护。
- Rust 结构治理：`doc/engineering/rust-governance/rust-1200-line-root-cause-governance-2026-03-29.prd.md`
- `.pm` / self-evolution：`doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.prd.md`、`doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.prd.md`、`doc/engineering/self-evolution/role-long-term-memory-2026-03-30.prd.md`
- 趋势 / 迁移 / 审读：`doc/engineering/governance/engineering-governance-trend-tracking-2026-03-11.prd.md`、`doc/engineering/governance/engineering-quarterly-governance-review-cycle-2026-03-11.prd.md`、`doc/engineering/doc-migration/legacy-doc-migration-collaboration-2026-03-03.prd.md`、`doc/engineering/prd-review/prd-full-system-audit-2026-03-03.prd.md`

## 共享约定
- 模块根入口、专题落位、README 职责与 legacy redirect 约定统一以 `doc/engineering/doc-governance/doc-structure-standard.design.md` 为准。
- `engineering` 根目录默认只保留模块入口文件；治理专题按对象下沉到 `doc-governance/`、`rust-governance/`、`governance/`、`doc-migration/`、`prd-review/` 与 `self-evolution/`。
- 工程治理规则、目录标准或角色协作口径变化时，优先更新 `doc/engineering/prd.md`；新增专题时，再同步回写 `doc/engineering/prd.index.md` 与本目录索引。若问题已从“首读面噪音”转向“文档存量维护成本”，优先进入 `doc-corpus-maintenance-governance` 与 `scripts/doc-inventory-report.sh`，而不是继续只改 README 首屏。
- 最近完成的细粒度治理任务默认回写对应 topic `*.project.md` 与 `.pm/tasks/*.yaml`；根 `doc/engineering/project.md` 不再手工维护按时间排序的 `最新完成` 长列表。
