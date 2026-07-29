# oasis7 Simulator：场景文件化（设计文档）设计

- 对应需求文档: `doc/world-simulator/scenario/scenario-files.prd.md`
- 对应项目管理文档: `doc/world-simulator/scenario/scenario-files.project.md`

## 1. 设计定位
定义场景文件化设计，把内置 `WorldScenario` 迁移为 JSON 场景文件并作为单一权威来源。

## 2. 设计结构
- 文件模型层：用场景 JSON 描述 seed、地点生成与可选碎片配置。
- 加载接线层：通过 `include_str!` 与调试入口统一场景加载。
- 生成表达层：用 `location_generator`、origin 等字段驱动初始化。
- 有效配置层：初始化时复制 `WorldConfig.asteroid_fragment` 后仅应用 JSON 显式 override；不修改全局默认值，也不提供运行期 hot configuration。
- 测试矩阵层：维护场景到测试目标的映射与稳定性校验。

## 3. 关键接口 / 入口
- `crates/oasis7/scenarios/*.json`
- `WorldInitConfig::from_scenario`
- `WorldScenario::parse`
- `oasis7_init_demo --scenario-file`

## 4. 约束与边界
- 内置场景文件需保持单一来源。
- 同场景配置在相同 seed 下必须可复现。
- `asteroid_fragment` 是唯一规范字段名；`min_fragment_spacing_cm` 缺省继承 `50_000` cm，`<= 0` 关闭额外 spacing。effective config 必须进入固定 seed/触发序列的确定性上下文，replay 只应用 committed delta。
- 只有显式 `power_bootstrap` 数据可注入 `power_plants`；其他内置和 `asteroid_fragment*` 场景无隐式设施。该注入不改变 runtime 电力规则、storage/动作 ABI、链上经济或治理边界。
- 不在本专题扩展复杂 DSL 与版本迁移工具。

## 5. 设计演进计划
- 先迁移内置场景为 JSON。
- 再接入加载逻辑与调试入口。
- 最后维护场景测试覆盖矩阵。
