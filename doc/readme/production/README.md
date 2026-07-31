# README production 文档入口

本目录收纳仍有独立语义的 README 生产收口专题，以及与 README 项目台账直接关联的发布素材。它不是当前实现或运营动作的首读入口。

## 先读哪里

- 想确认当前 README 对外口径、活跃工作与下一步：先读 `doc/readme/prd.md` 和 `doc/readme/prd.md`。
- 想按专题精确检索、区分活跃与历史压缩项：读 `doc/readme/prd.index.md`。
- 只在需要追溯生产收口的设计决策、验收证据或关联素材时，再进入本目录。

## 本目录分层

- **当前可检索专题**：`readme-prod-gap1245-wasm-repl-topology-player.*` 是仍保留在模块活跃专题清单中的 production closure 记录；它的项目文档已标记完成，因此不应被当作新的执行队列。
- **已完成的专业权威合并**：旧 `readme-llm-p1p2-production-closure.*`、`readme-p0-p1-closure.*` 与 `readme-prod-closure-llm-distfs-consensus.*` 只重复已完成的 LLM、runtime、共识、DistFS 与 viewer 实现任务。当前产品承诺从 `doc/product/` 四模块树进入，专业规则和实现真值从 `doc/world-simulator/llm/`、`doc/world-runtime/`、`doc/p2p/` 与 `doc/world-simulator/viewer/` 进入；viewer 不拥有节点、拓扑或共识门控真值。旧三件套已删除，历史任务从 Git history 与 GitHub task evidence 追溯。
- **受台账约束的素材**：`xiaohongshu-ai-economy-report-draft-2026-06-18.md` 及 `assets/xhs-ai-economy-2026-06-18/` 仍由 `doc/readme/prd.md` 的 PRD-README-052 完成项指向，不能仅因是 dated draft 删除。

## 收敛规则

- 不在此页平铺每份三件套或素材文件；文件级精确清单只维护在 `doc/readme/prd.index.md`。
- 新 production 专题先更新模块 `prd.md` / GitHub task issue evidence comments，再在索引中标记其活跃或历史压缩状态；本页只维护分流与边界。
- 做迁移治理时，应逐项把仍有效语义合并到现行权威、修复引用并删除已被完整吸收的源文件；只有仍承担独立专业操作或未闭环状态时才保留，并写明 authority reason。
