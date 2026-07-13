# README production 文档入口

本目录收纳已经完成的 README 生产收口证据、仍可作为历史基线检索的专题三件套，以及与 README 项目台账直接关联的发布素材。它不是当前实现或运营动作的首读入口。

## 先读哪里

- 想确认当前 README 对外口径、活跃工作与下一步：先读 `doc/readme/prd.md` 和 `doc/readme/project.md`。
- 想按专题精确检索、区分活跃与历史压缩项：读 `doc/readme/prd.index.md`。
- 只在需要追溯生产收口的设计决策、验收证据或关联素材时，再进入本目录。

## 本目录分层

- **当前可检索专题**：`readme-prod-gap1245-wasm-repl-topology-player.*` 是仍保留在模块活跃专题清单中的 production closure 记录；它的项目文档已标记完成，因此不应被当作新的执行队列。
- **历史压缩专题**：`readme-llm-p1p2-production-closure.*`、`readme-p0-p1-closure.*` 与 `readme-prod-closure-llm-distfs-consensus.*` 均已完成且无下一步。它们保留原址，供审计和实现历史追溯；当前入口由模块根 PRD、项目台账与后续 gap/production 专题承接。
- **受台账约束的素材**：`xiaohongshu-ai-economy-report-draft-2026-06-18.md` 及 `assets/xhs-ai-economy-2026-06-18/` 仍由 `doc/readme/project.md` 的 PRD-README-052 完成项指向，不能仅因是 dated draft 删除。

## 收敛规则

- 不在此页平铺每份三件套或素材文件；文件级精确清单只维护在 `doc/readme/prd.index.md`。
- 新 production 专题先更新模块 `prd.md` / `project.md`，再在索引中标记其活跃或历史压缩状态；本页只维护分流与边界。
- 删除 dated 内容前，必须同时证明有现行承接入口且不存在模块台账、索引或其他活跃调用；审计/复核引用本身不等于可删除依据。
