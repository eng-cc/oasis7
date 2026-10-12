# world-simulator 模块设计总览

审计轮次: 6

- 对应需求文档: `doc/world-simulator/prd.md`
- 当前任务状态与历史变更：GitHub task issue evidence 与 Git history
- 对应文件级索引: `doc/world-simulator/prd.index.md`

## 1. 设计定位
`world-simulator` 模块的 `design.md` 负责描述世界模拟、Viewer、Launcher、LLM 与场景系统的总体设计入口。

## 2. 阅读顺序
1. `doc/world-simulator/prd.md`
2. `doc/world-simulator/design.md`
3. `doc/world-simulator/prd.index.md`
4. 对应 GitHub task issue（需要当前任务状态、阻断或验收过程时）
5. 下钻 `viewer/`、`launcher/`、`llm/`、`kernel/`、`scenario/`、`m4/` 等专题目录

### 2.1 需求承接与分配表

本表只承接本总览拥有的 kernel adapter 兼容边界；其余主题仍由下钻专题承担，不把模块导航扩写成各子系统的完整设计。

| 上游 requirement / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [Kernel 规则成本兼容边界](prd.md#kernel-rule-cost-contract) | 保留历史 adapter 的决策聚合行为；不得由 `cost` 字段推导自动结算或持久成本记录。 | [cost 边界](design.md#kernel-rule-cost-boundary) | `runtime_engineer`；simulator hook 与正式 runtime 规则执行链。 | 不启用新的扣账规则，不扩展模块生命周期、WASM ABI 或持久化能力。 |

## 3. 设计结构
- 交互层：Viewer / Launcher / Web Console / UI 流程。
- 模拟层：kernel、scenario、资源与世界状态演化。
- 智能层：LLM、agent 行为、间接控制与多场景评估。

## 4. 集成点
- `doc/world-runtime/prd.md`
- `doc/game/prd.md`
- `doc/site/prd.md`
- `doc/testing/prd.md`

## 5. 专题导航
- 交互体验进入 `viewer/`、`launcher/`
- 核心模拟进入 `kernel/`、`scenario/`
- LLM 与 agent 行为进入 `llm/`
- 发行阶段能力进入 `m4/`

## 6. Simulator kernel rule / Wasm adapter boundary

- `WorldKernel` 的 completed rule-hook foundation 保持默认 no-op 不改变动作行为；pre-action 决策按固定注册顺序合并，`deny` 优先于 `modify/allow`，缺失或冲突的 override 显式拒绝。pre-action hook 只能读取 `&WorldKernel`，post-action hook 只观察已产生的 event。

<a id="kernel-rule-cost-boundary"></a>

### 规则成本字段

`KernelRuleDecision.cost` 是历史 adapter 保留的决策数据。`crates/oasis7/src/simulator/kernel/types.rs::merge_kernel_rule_decisions` 聚合 cost；`kernel/step.rs::execute_action_envelope_at_time` 随后只按 verdict、override 和拒绝 notes 决定动作执行，不使用 cost 进行余额检查或资源扣账，也不把聚合 cost 作为独立事件写入 journal。动作自己的资源变化由其实现负责，不能据此推断所有动作都无成本。

正式 runtime 的 `RuleDecision` 与该 adapter 分属不同执行入口：`crates/oasis7/src/runtime/world/step.rs` 检查资源缺口并应用 `decision.cost`，成本应用失败形成结构化拒绝。保留 simulator 的兼容行为不改变正式 runtime 的结算合同。若以后让 simulator cost 生效，应明确付款主体、数值及原子性、事件/replay、旧决策与旧消费者兼容，并以针对该行为的回归验证替代对字段存在的推断。

### Wasm adapter 与其余模拟边界

- simulator 的可选 Wasm evaluator 使用 `KernelRuleModuleInput/Output` 作为 adapter 输入输出；缺失 `rule.decision` 明确表示 allow，而 evaluator、sandbox 调用、重复/不匹配/非法的 `rule.decision` payload 或 action id 校验失败都必须转为可解释的 structured deny，不能 silent fallback。`ModuleSandbox` bridge 只接单个 pre-action 模块，不定义多模块编排、发布或权限治理。
- 旧的 simulator 内存 artifact registry 是已完成接线阶段的局部实现细节；hook、evaluator 与 registry 都是 process-local，snapshot restore 时重置，不是当前模块生命周期或持久化 authority。replay 只消费已记录的 journal event，不重新求值 hook；`ActionRejected` 在 replay 中不改变 world state。当前 ABI / `rule.decision` wire contract、executor limits、artifact registry、release governance 和 Docker-canonical build 分别由 `doc/world-runtime/wasm/wasm-interface.md`、`wasm-executor.prd.md`、`doc/world-runtime/prd.md` 与 `wasm-deterministic-build-pipeline.prd.md` 拥有。
- `tools/wasm_build_suite` 仍是活跃实现入口，但其发布级构建语义只服从 Docker-canonical build authority；不得由历史 host-side build-suite 文档推导发布或跨宿主确定性结论。
- simulator `PowerStorage`、`StorePower`、`DrawPower` 与 Location electricity pool 已下线；该移除边界由 `m4/industrial-resource-flow-contract.prd.md` 维护。它不删除名称相近但独立的 runtime builtin `m1_power_storage`，也不取消对 legacy `require_power_storages` 输入的明确拒绝。
- simulator 内建 `ResourceKind` 只保留 `Electricity` 与 `Data`。`Compound` / `Hardware` 不接受旧别名或自动迁移，也不得作为内建 kind 重新进入 kernel、parser、Viewer 统计或 LLM prompt；工业材料语义由 M4/domain contract 表达，模块资产与 ABI/执行权限由 `doc/world-runtime/wasm/` authority 拥有。该边界不等于已经提供通用 WASM 资产标准。

## 设计目标
- 提供 `world-simulator` 模块的总体设计入口。

## 设计范围
- 覆盖模块级结构、主链路、分层与专题导航。
- 不替代专题 `*.design.md` 的细化设计。

## 关键接口 / 入口
- 需求入口：`doc/world-simulator/prd.md`
- 当前任务入口：对应 GitHub task issue / Project
- 索引入口：`doc/world-simulator/prd.index.md`

## 设计演进计划
- M1 (2026-03-09): 在 ROUND-006 中补齐模块级 `design.md` 标准入口。
- M2: 按专题继续补齐高复杂度主题的 `*.design.md`。

## 设计风险
- 若专题级设计未及时补齐，模块级 `design.md` 可能承载过多导航职责。
- 若 legacy redirect 未明确标注为兼容跳转，读者可能误判历史入口为当前执行入口。

## 11. 验证设计与可追溯性

### 11.1 验证映射表

| 上游 requirement / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source 或 ID、scenario/layer、candidate/environment | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [Kernel 规则成本兼容边界](prd.md#kernel-rule-cost-contract) | [cost 边界](design.md#kernel-rule-cost-boundary) | 当前 cost 聚合不自动结算；已有 deny/modify 与事件后 hook 行为保持原义。 | 固定候选，回读 `kernel/types.rs::merge_kernel_rule_decisions` 与 `kernel/step.rs::execute_action_envelope_at_time`；[kernel_rule_decisions.rs](../../crates/oasis7/src/simulator/tests/kernel_rule_decisions.rs) 的 `kernel_pre_action_rule_deny_rejects_action`、`kernel_pre_action_rule_modify_overrides_action`、`post_action_hook_receives_event_after_modify_decision` 是相关 L1 hook 基线。文档修补核对源码和既有断言，行为改动时运行适用回归。 | PR 的候选、源码回读及实际运行结果，明确区分未运行的测试计划。 | 现有用例没有非零 cost 不扣账的专门断言；不能据此声称该行为已有自动回归、聚合溢出已验证或成本已持久记录。 |
