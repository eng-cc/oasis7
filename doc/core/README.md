# core 文档索引

审计轮次: 7

## 从这里开始
- 想先理解项目级总览、跨模块边界与当前唯一全局口径：`doc/core/prd.md`
- 想看当前 core 模块在推进什么、最新完成项和下一步：`doc/core/project.md`
- 想按主题或文件名继续下钻，而不是从活跃专题列表逐条找：`doc/core/prd.index.md`
- 想先看下一轮跨模块优先级主入口：`doc/core/next-round-priority-slate-2026-03-11.prd.md`
- 想先看三种玩家访问模式与 execution lane 的正式契约：`doc/core/player-access-mode-contract-2026-03-19.prd.md`
- 想先看“统一持久大世界”默认产品模型与术语迁移计划：`doc/core/unified-persistent-world-terminology-upgrade-plan-2026-06-16.md`
- 想追溯 2026-03 版本候选 readiness / go-no-go：按需进入 `doc/core/reviews/release-candidate-readiness-board-version-2026-03-11.md` 与 `doc/core/reviews/release-candidate-go-no-go-version-2026-03-11.md`

## 入口
- PRD: `doc/core/prd.md`
- 设计总览: `doc/core/design.md`
- 标准执行入口: `doc/core/project.md`
- 文件级索引: `doc/core/prd.index.md`

## 入口分工
- `README.md` 只承担 landing page 职责：告诉读者先去哪个权威入口，不重复索引长表和审计台账。
- `prd.md` 是 core 模块权威规格入口，适合先理解项目级模块地图、链路、阶段口径与跨模块规则。
- `project.md` 是执行台账，适合确认当前 core 收口动作、最近完成项与下一步。
- `prd.index.md` 是定向检索索引，适合已经知道主题后按文件名继续下钻，不是新读者的首读入口。
- `reviews/`、`templates/`、`checklists/` 属于审计与配套材料层，默认按需进入，不再和活跃主题入口混成同一层。

## 活跃阅读面边界
- 当前页只保留 `what / where / next / risk` 所需入口，不再直接平铺活跃专题长名单或 review 台账。
- 高频 active 入口保留在 `prd.md`、`project.md`、`prd.index.md` 与少量仍在承担当前跨模块判断职责的正式专题。
- ROUND 审查记录、采证、模板与 checklist 继续保留可检索性，但默认从 `prd.index.md` 或具体专题路径进入。

## 模块职责
- 提供项目级设计总览、模块地图、关键链路与术语口径。
- 维护跨模块治理基线、候选级 readiness / go-no-go 审计留痕与文档总入口同步。
- 维护项目级设计阅读顺序、下一轮优先级与 ROUND 台账入口。

## 热点子域导航（2026-06-23 快照）
- `reviews/`（45）：ROUND 审查、go/no-go、readiness board 与 audit-progress 留痕；默认按需进入。
- 根目录保留专题：下一轮优先级、player access mode contract、统一持久大世界术语迁移等正式 cross-module 入口；2026-03 release-candidate 根目录三件套、一次性 docs hub 同步专题与 core handoff 文件已退役删除，候选证据追溯改从 `reviews/` 审计记录进入，当前 docs hub 语义从 `doc/README.md`、`doc/core/project.md` 与 doc structure standard 进入。
- `templates/`（2）：阶段收口和 PRD-ID 追踪模板。
- `checklists/`（1）：跨模块影响检查清单。

## 高密度提示
- `doc/core/` 当前共有 68 份文件，其中 `reviews/` 占 45 份；默认入口不再尝试把 review / audit 材料直接摊平展示。
- 需要完整活跃专题清单时，进入 `doc/core/prd.index.md`；需要 round 审查、采证或模板时，再按子域进入。

## 共享约定
- 模块根入口、专题落位与 README/legacy redirect 的共享规则统一以 `doc/engineering/doc-governance/doc-structure-standard.design.md` 为准。
- 跨模块边界、候选级入口、下一轮优先级或主链路变化时，优先更新 `doc/core/prd.md` 与 `doc/core/project.md`；新增专题后，再同步回写 `doc/core/prd.index.md` 与本目录索引。
