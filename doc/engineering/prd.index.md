# engineering PRD 文件级索引

## 入口
- 模块 PRD：`doc/engineering/prd.md`
- 模块设计总览：`doc/engineering/design.md`
- 模块标准执行入口：`doc/engineering/prd.md`
- 文档治理专题入口：`doc/engineering/doc-governance/README.md`
- Rust 体量治理专题入口：`doc/engineering/rust-governance/README.md`

## 索引边界
- 本页只负责工程专题的文件级精确检索与三件套可达性；文档树问题从 `doc/engineering/doc-governance/README.md` 开始，Rust 体量与结构切片问题从 `doc/engineering/rust-governance/README.md` 开始。

| 专题 PRD | 专题设计文档 | 专题项目文档 |
| --- | --- | --- |
| `doc/engineering/doc-governance/doc-structure-standard.prd.md` | `doc/engineering/doc-governance/doc-structure-standard.design.md` | `doc/engineering/doc-governance/doc-structure-standard.prd.md` |
| `doc/engineering/rust-governance/rust-1200-line-root-cause-governance-2026-03-29.prd.md` | n/a（当前契约已收敛到 PRD） | n/a（执行证据归 GitHub task 与 git history） |
| Historical document-governance triplets | Current organization and consumption rules: `doc/engineering/doc-governance/doc-structure-standard.design.md`; inventory and maintenance-cost routing: `doc/engineering/governance/README.md` | Historical decision/rollout evidence: Git history and GitHub task issue evidence comments |
| Historical self-evolution / memory / borrowing / skill-surface triplets | Current task/evidence rules: `doc/engineering/workflow/source-of-truth.md`; retained pending scope: `doc/engineering/prd.md`; default-vs-library skill reachability: `.agents/skills/README.md` | Historical decision and rollout evidence: Git history and GitHub task issue evidence comments |

## 说明
- 本索引用于保证模块专题文档在根入口文档树中可达。
- 文档配对规则：`*.prd.md`、`*.design.md` 与同名 GitHub task issue evidence comments。
- `doc/engineering/doc-governance/README.md` 是 doc-governance 簇级分流入口；本索引保留完整三件套可达性，不再要求读者从长表里判断治理问题归属。
- `doc/engineering/rust-governance/README.md` 是 Rust 文件结构治理簇级分流入口；本索引保留单一当前契约的精确检索入口，不再复制已完成专题的 design / project。
- `engineering` 根目录默认只保留 `README.md / prd.md / design.md / GitHub task issue evidence comments / prd.index.md` 五个模块入口；治理专题已分别下沉到 `doc-governance/`、`rust-governance/` 与 `governance/`。`doc-structure-standard` 负责组织、职责和消费层边界，`governance/README.md` 负责 inventory 和维护成本 follow-up 路由；workflow source of truth 与 `.agents/skills/README.md` 分别承接 self-evolution task/evidence boundary 与 skill reachability。已完成且规则已被这些 current authorities 承接的一次性专题不再作为 live 三件套暴露。`doc/devlog` 的当前入口是 `doc/devlog/README.md` compact archive summary，不再通过 active 专题三件套暴露。

## 历史审计留痕
- 2026-04 `p2p-node-path-governance` 一次性落位三件套已退役删除；当前 `p2p/node` 的首读分流、主题簇与维护触发器由 `doc/p2p/node/README.md` 承接，历史实施证据保留在 git history 与 GitHub task evidence 中。
- 2026-04 `testing-evidence-path-governance` 一次性落位三件套已退役删除；当前 `testing/evidence` 的分流与维护边界由 `doc/testing/evidence/README.md` 承接，历史实施证据保留在 git history 与 GitHub task evidence 中。
- 2026-04 `world-simulator-viewer-path-governance` 一次性落位三件套已退役删除；当前 `world-simulator/viewer` 的首读分流、主题簇与维护触发器由 `doc/world-simulator/viewer/README.md` 承接，历史实施证据保留在 git history 与 GitHub task evidence 中。
- 2026-04 `readme-governance-path-governance` 一次性落位三件套已退役删除；当前 `readme/governance` 的首读分流、主题簇与维护触发器由 `doc/readme/governance/README.md` 承接，历史实施证据保留在 git history 与 GitHub task evidence 中。
- 2026-03 legacy 文档迁移 closure / handoff 记录已退役删除；历史迁移证据保留在 Git history logs 与 git history，当前迁移规则入口为 `doc/engineering/doc-governance/doc-structure-standard.design.md`、`doc/engineering/workflow/source-of-truth.md` 与 GitHub task issue evidence comments。
- 2026-02 documentation-governance-engineering closure 三件套已退役删除；历史审读证据保留在 Git history logs 与 git history，当前工程文档治理入口为 `doc/engineering/doc-governance/doc-structure-standard.design.md`、`doc/engineering/governance/README.md`、`doc/engineering/workflow/source-of-truth.md` 与 GitHub task issue evidence comments。
- 2026-03 engineering governance producer->QA 一次性 handoff 记录已退役删除；当前 governance trend / quarterly review 证据入口为 GitHub task issue evidence comments、pre-PR local role review packet 与 workflow source-of-truth。
- 2026-03 engineering governance trend tracking 与 quarterly governance cycle 一次性建模三件套已退役删除；当前趋势、季度复核、环境与仓库健康巡检统一从 `doc/engineering/governance/README.md` 分流，专题正文与 GitHub task issue evidence comments 保留各自事实和执行证据；文档树规则仍以 `doc/engineering/doc-governance/README.md`、task/PR 规则仍以 `doc/engineering/workflow/source-of-truth.md` 为准。
- 2026-03 全量 PRD 审读机制三件套已退役删除；历史审读证据保留在 Git history logs，当前新增/变更文档追踪以模块入口、`prd.index.md`、GitHub task issue evidence comments、`prd.md` 与现行 doc governance 规则为准。
- 2026-03 core release-candidate / next-round producer/QA/LiveOps 一次性 handoff 面与 release-candidate readiness / version / go-no-go 根目录三件套已退役删除；历史发布候选证据保留在 Git history 与 git history，当前角色证据 sink 为 GitHub task issue evidence comments 与 pre-PR local role review packet；历史 `.pm` execution log 只作为迁移前追溯层。
- 2026-03 self-evolution file-based PM project 面与历史 pointer PRD 已退役删除；`PRD-ENGINEERING-021` 历史锚点由 `file-based-self-evolution-management-2026-03-30.design.md` 的 object-model 背景承接，当前 task truth / evidence / reflection intake / PR-readiness 规则以 `doc/engineering/workflow/source-of-truth.md` 为准。
- 2026-03/05 dated self-evolution memory, external-workflow borrowing and skill-surface triplets，以及 2026-04 dated document-reading-surface and corpus-maintenance triplets 已退役删除；当前规则以 workflow source of truth、doc structure standard、engineering governance README、engineering project pending ledger 与 `.agents/skills/README.md` 为准。历史决策和 rollout 证据保留在 Git history 与 GitHub task issue evidence comments。
- 2026-04 `devlog-history-compaction` 一次性专题三件套已退役删除；`doc/devlog/README.md` 保留 compact archive summary，当前任务执行证据以 GitHub task issue evidence comments、GitHub Project fields 与 workflow source-of-truth 为准。
- 2026-02 oversized Rust file splitting round3 三件套已退役删除；当前 Rust 1200 行治理统一从 `doc/engineering/rust-governance/README.md` 分流，历史 round3 审读证据保留在 Git history logs 与 git history。
