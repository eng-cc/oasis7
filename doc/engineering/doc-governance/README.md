# engineering/doc-governance 专题入口

本目录是 `doc/` 文档树治理的唯一上游分流入口，收拢文档组织、路径落位、入口减重与存量维护成本治理。读者应先从这里判断问题属于哪一类，再进入对应 PRD / design / project 三件套；不要把同一治理规则继续复制到模块 README 或零散专题里。

## 首读路径
- 文档组织规则、后缀职责、模块 README 边界：`doc/engineering/doc-governance/doc-structure-standard.design.md`
- 默认阅读面减重与根/模块入口职责：`doc/engineering/doc-governance/doc-surface-area-governance-2026-04-10.prd.md`
- 文档存量、热点目录、近限文件与维护成本判断：`doc/engineering/doc-governance/doc-corpus-maintenance-governance-2026-04-17.prd.md`
- 全量专题三件套索引：`doc/engineering/prd.index.md`

## 按治理问题分流
| 问题 | Canonical 入口 | 说明 |
| --- | --- | --- |
| 新文档应该放在哪里、承担什么职责 | `doc-structure-standard.design.md` | 顶层组织规范；定义模块、专题、分册、README、PRD/design/project/manual/runbook 边界 |
| 根入口或模块 README 过长、重复维护共享规则 | `doc-surface-area-governance-2026-04-10.prd.md` | 处理默认阅读面噪音，避免 landing page 变成第二份规范正文 |
| 文档总量、热点子目录、devlog backlog 或近限长文件抬高维护成本 | `doc-corpus-maintenance-governance-2026-04-17.prd.md` | 处理入口减重之后的存量维护成本，配合 `scripts/doc-inventory-report.sh` 复算 |
| `world-simulator` / Viewer 文档路径混叠 | `world-simulator-viewer-path-governance-2026-04-17.prd.md` | 针对 Viewer、launcher、world-simulator 热点路径的专题治理 |
| `p2p` 节点、链与网络层文档路径混叠 | `p2p-node-path-governance-2026-04-17.prd.md` | 针对 p2p/node/blockchain 子域的路径级治理 |
| testing evidence 与手册/门禁文档混叠 | `../../testing/evidence/README.md` | 现行 evidence 子域入口；与 testing 模块入口、文件级索引和 operator 手册分层 |
| readme 外部口径、渠道 runbook 与入口职责混叠 | `readme-governance-path-governance-2026-04-18.prd.md` | 针对 `doc/readme/` 及对外说明树的路径级治理 |

## 维护规则
- 本页只做专题导航和抽象分流，不承载完整治理规则正文。
- 就文档树共享治理规则而言，上游 `doc/README.md` 与 `doc/engineering/README.md` 只链接本页；其他模块或专题导航按各自职责保留。具体规则、专题正文与执行状态必须从本页继续下钻，避免上游入口各自固定某个专题文件。
- 新增 doc-governance 专题时，同批更新 `doc/engineering/prd.index.md`，并在本页补一行“按治理问题分流”。
- 历史证据、旧审读轮次和任务过程仍保留在 GitHub task issue evidence comments、`doc/core/reviews/round-*` 与 git history；不要为减重而批量改写历史文件。
