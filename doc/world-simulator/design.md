# world-simulator 模块设计总览

审计轮次: 6

- 对应需求文档: `doc/world-simulator/prd.md`
- 对应项目管理文档: `doc/world-simulator/project.md`
- 对应文件级索引: `doc/world-simulator/prd.index.md`

## 1. 设计定位
`world-simulator` 模块的 `design.md` 负责描述世界模拟、Viewer、Launcher、LLM 与场景系统的总体设计入口。

## 2. 阅读顺序
1. `doc/world-simulator/prd.md`
2. `doc/world-simulator/design.md`
3. `doc/world-simulator/project.md`
4. `doc/world-simulator/prd.index.md`
5. 下钻 `viewer/`、`launcher/`、`llm/`、`kernel/`、`scenario/`、`m4/` 等专题目录

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
- 执行入口：`doc/world-simulator/project.md`
- 索引入口：`doc/world-simulator/prd.index.md`

## 设计演进计划
- M1 (2026-03-09): 在 ROUND-006 中补齐模块级 `design.md` 标准入口。
- M2: 按专题继续补齐高复杂度主题的 `*.design.md`。

## 设计风险
- 若专题级设计未及时补齐，模块级 `design.md` 可能承载过多导航职责。
- 若 legacy redirect 未明确标注为兼容跳转，读者可能误判历史入口为当前执行入口。
