# README gap 文档入口

## 从这里开始

- 想先理解 README 与实现/流程缺口的总收口边界：读 `readme-gap-distributed-prod-hardening-gap12345.prd.md`；它是本子域主文档。
- 想确认该主收口的完成动作、验证与后续状态：读 `readme-gap-distributed-prod-hardening-gap12345.project.md`。
- 想按具体增量下钻：从 `doc/readme/prd.index.md` 的当前活跃专题清单按文件名进入；该索引保留精确检索，不是本子域的首读入口。
- 想追溯已完成的模块安装目标语义增量：按需进入 `readme-gap3-install-target-infrastructure.prd.md`；它只保留历史证据，不承担当前动作。

## 权威边界

- 主文档三件套定义跨 Gap 1–5 的总收口和当前主从关系。
- 其余八组 triplet 是具体能力的增量：基础设施执行/编译 sandbox、WASM live 持久化与实例升级、共识/市场/生命周期、世界内编译与计费、LLM-WASM 生命周期、安装目标语义、以及生命周期/订单簿；它们不替代主文档。
- 每组 `*.project.md` 保存执行与完成证据；不要从完成态项目记录推导当前产品、运行时或发布状态。
- 当前 README 对外口径与模块活跃队列分别由 `doc/readme/prd.md` 和 `doc/readme/project.md` 维护；本目录只组织历史 gap 收口资料。

## 保留与清理边界

- `readme-gap3-install-target-infrastructure.*` 已由主文档承接当前入口，但仍被文件索引和 core review 审计记录引用，故保留为可定位历史，不删除。
- 其他增量 triplet 仍处于索引的当前活跃专题清单；新增或退役专题时，先更新主文档/模块台账，再同步本页与 `doc/readme/prd.index.md`。
