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

### 2.1 需求承接与分配表

| 上游 requirement / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 未覆盖范围 |
| --- | --- | --- | --- | --- |
| [工业报价账本语义](industrial-resource-flow-contract.prd.md#industrial-quote-ledger-semantics) | 精炼净变化、逐种缺口、实际消费账本、转移失败无部分效果与物流解释分别成立。 | [报价计算与消费者](industrial-resource-flow-contract.design.md#industrial-quote-calculation-design) | Simulator/Runtime 领域计算、Viewer/Agent 消费者与 QA 独立验收。 | 不改资源种类、规则数值、ABI、供给政策或宣称实现通过。 |
| [跨资源承诺阶段](../../product/world-rules-core-gameplay/prd.md#跨资源承诺、失效与重复提交边界) | 未提交预览可解释，pending 不冒充领域承诺；只有当前支持且 fresh revalidation 通过的动作可提交。 | [报价计算与消费者](industrial-resource-flow-contract.design.md#industrial-quote-calculation-design) | Runtime 颁发权威 identity/outcome，产品拥有玩家阶段语义。 | 不自行生成 root/revision 或扩展完整工业执行系统。 |

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

<a id="industrial-quote-calculation-design"></a>
## 6. 现有报价的领域计算与消费者合同

本节承接 [PRD 的账本与净缺口条款](industrial-resource-flow-contract.prd.md#industrial-quote-ledger-semantics)，只定义待修正或验证的领域映射，不把文档修订当作实现通过。Simulator WorldKernel 与 Runtime World 各从自己的当前权威状态计算；消费者不另算一套“可执行性”。

### 6.1 Simulator 精炼净变化

对同一个 owner、snapshot 与 BuildFactory 目标，令 `D` 为动作前可用于建厂的 Data、`M` 为精炼投入、`H` 为按现有公式得到的产出、`B` 为该目标的 Data 成本。先验证精炼原有 amount、owner、电力、最小产出和溢出条件，再只读计算：

```text
data_before = D
data_after = D - M + H
shortfall_before = max(B - data_before, 0)
shortfall_after = max(B - data_after, 0)
```

before 不能使用 `D-M`，after 不能从已经截零的 before 只减 gross H。`partial_progress` 要求 `0 < shortfall_after < shortfall_before`；`enough_to_advance` 要求原缺口为正且本次消除该缺口；无改善或恶化走现有 `poor_power_tradeoff` 并解释净损失。`recommended_refine_amount` 用同一净效果评估，目标已满足或没有有益的合法精炼量时推荐暂缓，不生成 amount=0 的非法精炼动作。保留现有公式和字段名，不重建独立 compound/hardware ResourceKind。

回归从同一合法 snapshot 分叉，以真实 BuildFactory 可负担性核对 before/after；2500 Data、50 Electricity 的默认反例应显示 `0 -> 3`，不再把 5→3 作为预期。低电力、最小产出、overflow、已有目标资源与只读不改变世界继续独立验证。

### 6.2 Runtime 材料缺口与消费账本

`market_quote_decision_preview` 消费规范化、按当前材料同一性聚合的需求；每个 kind 先得到 `max(requested - selected_available, 0)`，再进行有溢出保护的总和。选用 local/world 的规则仍取现行 `build_material_market_quotes`/实际 admission，不因修复负缺口而改变采购政策。signed `saturating_sub` 仅防溢出，不能替代逐项截零。

报价若用于 ScheduleRecipe，preferred consume ledger 必须与该 factory/recipe 实际 input ledger 一致。Agent 只表示发起主体时，不能用其余额代替 factory 库存；独立市场查询也须明示账本与条件边界。DTO 从同一结果生成 contributions、total shortfall、allowed 与 recommendation，Viewer/Agent 在报价到达时读取这份结果，不等额外 snapshot 才展示，也不重算不同账本的短缺。

Focused fixtures 分别覆盖本地富余、世界富余、一盈一缺、精确覆盖、普通短缺、重复材料聚合；所有分项/总量非负，盈余不抵销短缺，allowed 与推荐一致。另以 Agent/factory 余额不同的反例证明实际消费 ledger，quote 调用不得产生 sink、hold 或世界更新。UI 到达即刷新需 consumer 自己的独立测试，内核通过不能代签。

### 6.3 转移与物流解释

Simulator `TransferResource` 在源扣减前完成 from/to 支持的 ResourceKind、owner、共址、余额与目标入账检查；Location Electricity 提前结构化拒绝。源/目标变化作为同一转移结果发布，后续失败不能留下半转移；自转移不使用两个独立快照相互覆盖。通过公开正数提交验证 Agent→Location 拒绝、原源余额不变、没有 ResourceTransferred，再检查合法 Agent→Agent Electricity、Data、自转移和余额不足。保留既有事件/历史兼容，不改变 ActionRejected 的资源无效果语义。

Runtime 物流 quote 的 path binding 与可执行性分开投影：空 binding 的合法 direct/unbound 路径显示中性“无需已绑定路径”等同义说明，真正 route unavailable 才显示阻塞。priority reason 按 producer 的 route/material/explicit/keyword 分类独立解释；无法提交时不展示无条件 ETA。分别验证合法 direct、真实 unavailable、各 reason 和旧零距离 lane，消费者不得自己新增 route、blocker projection 或默认执行动作。

### 6.4 预览到承诺的关联

首次预览用 candidate/config/snapshot 标识比较对象；新 root/revision 仅在 Runtime 权威 accepted outcome 颁发后进入承诺和 actual 对账。重读已有 operation 时可引用其已颁发 root，不能因第一次预览没有 root 而拒绝解释，也不能为通过预览自行构造 root。请求入队/pending、领域承诺和 receipt-backed settlement 服从[产品阶段合同](../../product/world-rules-core-gameplay/prd.md#跨资源承诺、失效与重复提交边界)。blocked/unknown 候选保留无副作用预览及缺失事实；只有当前支持且 fresh revalidation 通过的动作才可提交。

文档验收检查上述公式、账本与阶段在 PRD/design/消费者说明一致，并运行 `./scripts/doc-governance-check.sh`。未来实现验收复用上述有区分力的离线 fixtures，按实际消费者补 UI/Agent 证据；本节不要求把后续完整工业系统实现作为这次文档交付条件。

## 11. 验证设计与可追溯性

### 11.1 验证映射表

| 上游 requirement / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source、scenario/layer、candidate/environment | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [工业报价账本语义](industrial-resource-flow-contract.prd.md#industrial-quote-ledger-semantics) | [报价计算与消费者](industrial-resource-flow-contract.design.md#industrial-quote-calculation-design) | 同一快照的真实净余额、逐项缺口、消费账本和失败无部分效果。 | 待执行：扩展 [Simulator 精炼 fixtures](../../../crates/oasis7/src/simulator/tests/kernel_regressions.rs)、`simulator/tests/kernel_actions.rs`、`runtime/tests/market_quote_decision_preview.rs` 与 `runtime/tests/logistics_transfer_quote.rs`，按 §6.1–6.3 的正负例绑定实际 candidate；消费者到达态另用 `viewer/runtime_live/tests/transfer_material_quote.rs` 等相应 surface 验证。 | 实施 PR 的 snapshot 分叉、请求/quote/receipt、账本前后值与实际 consumer captures。 | 这些文件是扩展位置，文档未运行业务测试，不代签完整 UI/Agent parity 或发布。 |
| [跨资源承诺阶段](../../product/world-rules-core-gameplay/prd.md#跨资源承诺、失效与重复提交边界) | [报价计算与消费者](industrial-resource-flow-contract.design.md#industrial-quote-calculation-design) | 首次预览不依赖未颁发 root，blocked/unknown 可解释，pending 与 accepted 领域承诺分开。 | 待执行：按 [测试手册](../../../testing-manual.md) 的当前工业/消费者场景绑定实际 candidate 与受支持动作，分别捕获未提交候选、pending、权威 accepted 和 receipt-backed settlement；缺失阶段明确记录 blocked/unverified。 | 各阶段的实际 request、snapshot、权威 identity/outcome 与 Viewer/Agent 投影。 | 方法不使 target 工业协议、root 或全量动作变成现行能力。 |
