# oasis7 Simulator：Location 电力池下线与 Agent 辐射电厂（项目管理文档）

- 对应设计文档: `doc/world-simulator/kernel/location-electricity-pool-removal-and-radiation-plant.design.md`
- 对应需求文档: `doc/world-simulator/kernel/location-electricity-pool-removal-and-radiation-plant.prd.md`

审计轮次: 5

## 任务拆解（含 PRD-ID 映射）

### R1 文档
- [x] 输出设计文档（`doc/world-simulator/kernel/location-electricity-pool-removal-and-radiation-plant.prd.md`）
- [x] 输出项目管理文档（本文件）

### R2 Location 电力池下线
- [x] 初始化清洗 Location `electricity` 库存（场景/初始化统一口径）
- [x] 下线 `DrawPower` / `StorePower` 动作路径
- [x] 限制 `BuyPower` / `SellPower`：Location owner 参与电力交易时拒绝
- [x] 发电入账路径不再写 Location 电力库存

### R3 Agent 辐射电厂建造
- [x] 新增 `factory.power.radiation.mk1` 可建造类型
- [x] `BuildFactory` 对该类型同步注册 `PowerPlant`
- [x] 发电入账到 owner（Agent）资源
- [x] 更新 LLM 提示（`factory_kind` 支持集）

### R4 测试与收口
- [x] 更新 simulator 单元测试（power/kernel/init/llm 相关）
- [x] 运行 required-tier 测试命令并通过
- [x] 更新本项目文档状态
- [x] 追加当日 `doc/devlog/README.md`
- [x] 提交 git commit

## 依赖
- `crates/oasis7/src/simulator/kernel/actions.rs`
- `crates/oasis7/src/simulator/kernel/power.rs`
- `crates/oasis7/src/simulator/kernel/replay.rs`
- `crates/oasis7/src/simulator/init.rs`
- `crates/oasis7/src/simulator/llm_agent/prompt_assembly.rs`
- `crates/oasis7/src/simulator/tests/*`

## 状态
- 最近更新：2026-03-06（ROUND-005 I5-001 字段补齐）
- 当前阶段：R1-R4 已完成
- 下一阶段：无（等待新需求）
