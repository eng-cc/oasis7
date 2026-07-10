# engineering PRD 文件级索引

审计轮次: 7

更新时间：2026-04-18

## 入口
- 模块 PRD：`doc/engineering/prd.md`
- 模块设计总览：`doc/engineering/design.md`
- 模块标准执行入口：`doc/engineering/project.md`
- 文档治理专题入口：`doc/engineering/doc-governance/README.md`

## 索引边界
- 本页只负责工程专题的文件级精确检索与三件套可达性；按治理问题分流统一从 `doc/engineering/doc-governance/README.md` 开始。

| 专题 PRD | 专题设计文档 | 专题项目文档 |
| --- | --- | --- |
| `doc/engineering/doc-governance/doc-surface-area-governance-2026-04-10.prd.md` | `doc/engineering/doc-governance/doc-surface-area-governance-2026-04-10.design.md` | `doc/engineering/doc-governance/doc-surface-area-governance-2026-04-10.project.md` |
| `doc/engineering/doc-governance/doc-corpus-maintenance-governance-2026-04-17.prd.md` | `doc/engineering/doc-governance/doc-corpus-maintenance-governance-2026-04-17.design.md` | `doc/engineering/doc-governance/doc-corpus-maintenance-governance-2026-04-17.project.md` |
| `doc/engineering/doc-governance/world-simulator-viewer-path-governance-2026-04-17.prd.md` | `doc/engineering/doc-governance/world-simulator-viewer-path-governance-2026-04-17.design.md` | `doc/engineering/doc-governance/world-simulator-viewer-path-governance-2026-04-17.project.md` |
| `doc/engineering/doc-governance/p2p-node-path-governance-2026-04-17.prd.md` | `doc/engineering/doc-governance/p2p-node-path-governance-2026-04-17.design.md` | `doc/engineering/doc-governance/p2p-node-path-governance-2026-04-17.project.md` |
| `doc/engineering/doc-governance/testing-evidence-path-governance-2026-04-17.prd.md` | `doc/engineering/doc-governance/testing-evidence-path-governance-2026-04-17.design.md` | `doc/engineering/doc-governance/testing-evidence-path-governance-2026-04-17.project.md` |
| `doc/engineering/doc-governance/readme-governance-path-governance-2026-04-18.prd.md` | `doc/engineering/doc-governance/readme-governance-path-governance-2026-04-18.design.md` | `doc/engineering/doc-governance/readme-governance-path-governance-2026-04-18.project.md` |
| `doc/engineering/doc-governance/doc-structure-standard.prd.md` | `doc/engineering/doc-governance/doc-structure-standard.design.md` | `doc/engineering/doc-governance/doc-structure-standard.project.md` |
| `doc/engineering/rust-governance/rust-1200-line-root-cause-governance-2026-03-29.prd.md` | `doc/engineering/rust-governance/rust-1200-line-root-cause-governance-2026-03-29.design.md` | `doc/engineering/rust-governance/rust-1200-line-root-cause-governance-2026-03-29.project.md` |
| `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.prd.md` | `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.design.md` | `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.project.md` |
| `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.prd.md` | `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.design.md` | `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.project.md` |
| `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.prd.md` | `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.design.md` | `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.project.md` |
| `doc/engineering/self-evolution/skill-surface-replacement-governance-2026-05-19.prd.md` | `doc/engineering/self-evolution/skill-surface-replacement-governance-2026-05-19.design.md` | `doc/engineering/self-evolution/skill-surface-replacement-governance-2026-05-19.project.md` |

## 说明
- 本索引用于保证模块专题文档在根入口文档树中可达。
- 文档配对规则：`*.prd.md`、`*.design.md` 与同名 `*.project.md`。
- `doc/engineering/doc-governance/README.md` 是 doc-governance 簇级分流入口；本索引保留完整三件套可达性，不再要求读者从长表里判断治理问题归属。
- `engineering` 根目录默认只保留 `README.md / prd.md / design.md / project.md / prd.index.md` 五个模块入口；治理专题已分别下沉到 `doc-governance/`、`rust-governance/`、`governance/` 与 `self-evolution/`。其中 `doc-surface-area-governance` 负责默认阅读面，`doc-corpus-maintenance-governance` 负责入口减重后的存量维护成本，`world-simulator-viewer-path-governance`、`p2p-node-path-governance`、`testing-evidence-path-governance` 与 `readme-governance-path-governance` 分别负责当前四个热点子域的路径级治理；`agent-workflow-borrowing-governance` 负责将外部 agent workflow 方法论映射为 repo-owned adopted / rejected / deferred 治理结论，`skill-surface-replacement-governance` 负责冻结本地 skill inventory 的 keep / replace / retire / defer 边界。`doc/devlog` 的当前入口是 `doc/devlog/README.md` compact archive summary，不再通过 active 专题三件套暴露。

## 历史审计留痕
- 2026-03 legacy 文档迁移 closure / handoff 记录已退役删除；历史迁移证据保留在 `doc/core/reviews/round-*` logs 与 git history，当前迁移规则入口为 `doc/engineering/doc-governance/doc-structure-standard.design.md`、`doc/engineering/workflow/source-of-truth.md` 与 GitHub task issue evidence comments。
- 2026-02 documentation-governance-engineering closure 三件套已退役删除；历史审读证据保留在 `doc/core/reviews/round-*` logs 与 git history，当前工程文档治理入口为 `doc/engineering/doc-governance/doc-structure-standard.design.md`、`doc/engineering/doc-governance/doc-corpus-maintenance-governance-2026-04-17.project.md`、`doc/engineering/workflow/source-of-truth.md` 与 GitHub task issue evidence comments。
- 2026-03 engineering governance producer->QA 一次性 handoff 记录已退役删除；当前 governance trend / quarterly review 证据入口为 GitHub task issue evidence comments、pre-PR local role review packet 与 workflow source-of-truth。
- 2026-03 engineering governance trend tracking 与 quarterly governance cycle 一次性建模三件套已退役删除；当前趋势基线保留在 `doc/engineering/evidence/engineering-governance-trend-baseline-2026-03-11.md`，季度复核从 `doc/engineering/governance/engineering-quarterly-review-template-2026-03-11.md` 开始，并以 `doc/engineering/doc-governance/doc-corpus-maintenance-governance-2026-04-17.project.md`、`doc/engineering/workflow/source-of-truth.md` 与 GitHub task issue evidence comments 作为当前治理真值。
- 2026-03 全量 PRD 审读机制三件套已退役删除；历史审读证据保留在 `doc/core/reviews/round-*` logs，当前新增/变更文档追踪以模块入口、`prd.index.md`、`project.md`、`prd.md` 与现行 doc governance 规则为准。
- 2026-03 core release-candidate / next-round producer/QA/LiveOps 一次性 handoff 面与 release-candidate readiness / version / go-no-go 根目录三件套已退役删除；历史发布候选证据保留在 `doc/core/reviews/*` 与 git history，当前角色证据 sink 为 GitHub task issue evidence comments 与 pre-PR local role review packet；历史 `.pm` execution log 只作为迁移前追溯层。
- 2026-03 self-evolution file-based PM project 面与历史 pointer PRD 已退役删除；`PRD-ENGINEERING-021` 历史锚点由 `file-based-self-evolution-management-2026-03-30.design.md` 的 object-model 背景承接，当前 task truth / evidence / reflection intake / PR-readiness 规则以 `doc/engineering/workflow/source-of-truth.md` 为准。
- 2026-04 `devlog-history-compaction` 一次性专题三件套已退役删除；`doc/devlog/README.md` 保留 compact archive summary，当前任务执行证据以 GitHub task issue evidence comments、GitHub Project fields 与 workflow source-of-truth 为准。
- 2026-02 oversized Rust file splitting round3 三件套已退役删除；当前 Rust 1200 行治理入口为 `doc/engineering/rust-governance/rust-1200-line-root-cause-governance-2026-03-29.prd.md`，历史 round3 审读证据保留在 `doc/core/reviews/round-*` logs 与 git history。
