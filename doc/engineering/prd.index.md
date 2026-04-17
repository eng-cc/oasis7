# engineering PRD 文件级索引

审计轮次: 6

更新时间：2026-04-14

## 入口
- 模块 PRD：`doc/engineering/prd.md`
- 模块设计总览：`doc/engineering/design.md`
- 模块标准执行入口：`doc/engineering/project.md`

| 专题 PRD | 专题设计文档 | 专题项目文档 |
| --- | --- | --- |
| `doc/engineering/doc-governance/doc-surface-area-governance-2026-04-10.prd.md` | `doc/engineering/doc-governance/doc-surface-area-governance-2026-04-10.design.md` | `doc/engineering/doc-governance/doc-surface-area-governance-2026-04-10.project.md` |
| `doc/engineering/doc-governance/doc-corpus-maintenance-governance-2026-04-17.prd.md` | `doc/engineering/doc-governance/doc-corpus-maintenance-governance-2026-04-17.design.md` | `doc/engineering/doc-governance/doc-corpus-maintenance-governance-2026-04-17.project.md` |
| `doc/engineering/doc-governance/doc-structure-standard.prd.md` | `doc/engineering/doc-governance/doc-structure-standard.design.md` | `doc/engineering/doc-governance/doc-structure-standard.project.md` |
| `doc/engineering/doc-migration/legacy-doc-migration-collaboration-2026-03-03.prd.md` | `doc/engineering/doc-migration/legacy-doc-migration-collaboration-2026-03-03.design.md` | `doc/engineering/doc-migration/legacy-doc-migration-collaboration-2026-03-03.project.md` |
| `doc/engineering/doc-governance/documentation-governance-engineering-closure-2026-02-27.prd.md` | `doc/engineering/doc-governance/documentation-governance-engineering-closure-2026-02-27.design.md` | `doc/engineering/doc-governance/documentation-governance-engineering-closure-2026-02-27.project.md` |
| `doc/engineering/rust-governance/oversized-rust-file-splitting-2026-02-23.prd.md` | `doc/engineering/rust-governance/oversized-rust-file-splitting-2026-02-23.design.md` | `doc/engineering/rust-governance/oversized-rust-file-splitting-2026-02-23.project.md` |
| `doc/engineering/rust-governance/rust-1200-line-root-cause-governance-2026-03-29.prd.md` | `doc/engineering/rust-governance/rust-1200-line-root-cause-governance-2026-03-29.design.md` | `doc/engineering/rust-governance/rust-1200-line-root-cause-governance-2026-03-29.project.md` |
| `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.prd.md` | `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.design.md` | `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.project.md` |
| `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.prd.md` | `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.design.md` | `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.project.md` |
| `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.prd.md` | `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.design.md` | `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.project.md` |
| `doc/engineering/governance/engineering-governance-trend-tracking-2026-03-11.prd.md` | `doc/engineering/governance/engineering-governance-trend-tracking-2026-03-11.design.md` | `doc/engineering/governance/engineering-governance-trend-tracking-2026-03-11.project.md` |
| `doc/engineering/governance/engineering-quarterly-governance-review-cycle-2026-03-11.prd.md` | `doc/engineering/governance/engineering-quarterly-governance-review-cycle-2026-03-11.design.md` | `doc/engineering/governance/engineering-quarterly-governance-review-cycle-2026-03-11.project.md` |
| `doc/engineering/prd-review/prd-full-system-audit-2026-03-03.prd.md` | `doc/engineering/prd-review/prd-full-system-audit-2026-03-03.design.md` | `doc/engineering/prd-review/prd-full-system-audit-2026-03-03.project.md` |

## 说明
- 本索引用于保证模块专题文档在根入口文档树中可达。
- 文档配对规则：`*.prd.md`、`*.design.md` 与同名 `*.project.md`。
- `engineering` 根目录默认只保留 `README.md / prd.md / design.md / project.md / prd.index.md` 五个模块入口；治理专题已分别下沉到 `doc-governance/`、`rust-governance/`、`governance/`、`doc-migration/`、`prd-review/` 与 `self-evolution/`。其中 `doc-surface-area-governance` 负责默认阅读面，`doc-corpus-maintenance-governance` 负责入口减重后的存量维护成本。
