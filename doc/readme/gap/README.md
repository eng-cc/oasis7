# README gap 文档入口

## 从这里开始

- 想先理解 README 与实现/流程缺口的总收口边界：读 `readme-gap-distributed-prod-hardening-gap12345.prd.md`；它是本子域主文档。
- 想确认该主收口的完成动作、验证与后续状态：读 `readme-gap-distributed-prod-hardening-gap12345.project.md`。
- 想追溯已完成的模块安装目标语义增量：当前合同已由 `readme-gap-distributed-prod-hardening-gap12345.{prd,design,project}.md` 吸收；历史实施只从 Git history 与 GitHub task evidence 追溯。

## 权威边界

- 主文档三件套定义跨 Gap 1–5 的总收口和当前主从关系。
- 已完成的基础设施执行/编译 sandbox、WASM live 持久化与实例升级、共识/市场/生命周期、世界内编译与计费、LLM-WASM 生命周期及生命周期/订单簿增量已完成专业权威合并并删除源三件套；当前合同分别进入 `doc/world-runtime/`、`doc/p2p/`、`doc/world-simulator/` 与 `doc/game/`。
- 每组 `*.project.md` 保存执行与完成证据；不要从完成态项目记录推导当前产品、运行时或发布状态。
- 当前 README 对外口径与模块活跃队列分别由 `doc/readme/prd.md` 和 `doc/readme/prd.md` 维护；本目录只组织历史 gap 收口资料。

## 保留与清理边界

- 退役源文件只从 Git history 与 GitHub task evidence 追溯；新增或退役专题时，先完成专业权威吸收与引用修复，再同步本页与 `doc/readme/prd.index.md`。
