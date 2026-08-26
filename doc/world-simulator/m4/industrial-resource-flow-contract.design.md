# M4 工业资源流转合同设计

- 对应需求文档: `doc/world-simulator/m4/industrial-resource-flow-contract.prd.md`
- 当前任务状态与历史变更：GitHub task issue evidence 与 Git history。

## 1. 设计定位

将工业生产、材料账本与物流、电力 owner 归属收敛为一个可下钻的 M4 专业入口；它只整理现行合同，不改变 runtime、WASM、Viewer 或玩法行为。

## 2. 设计结构

- 工业规则层：Recipe/Product/Factory 模块与工厂状态。
- 资源流转层：多账本、调运约束、在途结算与兼容。
- 方案经济层：按同一 root/revision 对齐 planned/admissible/committed/actual 成本、损耗、产出价值与终端结算，不替代价格或数值平衡权威。
- 电力归属层：owner-held electricity、辐射电厂、Location pool 与储能路径下线。
- 可读性债务层：保留 GitHub #2166 quote 缺口，避免把历史设计文本误读为已实现 UI/ABI。
- 追溯层：completed source triplets 由 Git history 与 GitHub task evidence 追溯。

## 3. 关键接口 / 入口

- `BuildFactory`、`ScheduleRecipe`、`TransferMaterial`、`ProductValidated`。
- `factory.power.radiation.mk1`、`PowerPlant` 与 owner electricity 入账。
- `MaterialLedgerId`、在途材料事件、模块请求兼容字段。
- `doc/game/gameplay/gameplay-top-level-design.prd.md` 的玩家可读性合同和 GitHub #2166 债务路由。

## 4. 约束与边界

- 不重引 `PowerStorage`、`DrawPower`、`StorePower` 或 Location electricity pool。
- 不从 completed documentation 推断当前 quote、Viewer、LLM/provider、ABI 或 release readiness。
- 不重平衡规则；具体顺序、字段、hash/manifest 与回放语义仍由 runtime/ABI canonical authority 拥有。
- 不把预览、生产 receipt 或客户端估算当作终端价值；成本/价值归因只沿 M4、Recipe/Factory 与 runtime 的既有 receipt/lineage 组合表达。

## 5. 设计演进计划

先由专业实现分别收口 quote/readability、兼容与验证，再由对应 owner 更新当前结论；本专题只提供稳定的文档入口与边界。
