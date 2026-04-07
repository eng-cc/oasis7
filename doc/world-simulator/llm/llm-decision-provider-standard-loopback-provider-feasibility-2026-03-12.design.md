# LLM Agent Decision Provider 标准层 + Local Provider 外部适配可行性（2026-03-12）设计

- 对应需求文档: `doc/world-simulator/llm/llm-decision-provider-standard-loopback-provider-feasibility-2026-03-12.prd.md`
- 对应项目管理文档: `doc/world-simulator/llm/llm-decision-provider-standard-loopback-provider-feasibility-2026-03-12.project.md`

## 1. 设计定位
定义 world-simulator 中“世界内 Agent 契约”与“具体外部 agent provider”之间的标准层，使 `Local Provider` 之类的外部框架可通过 adapter 参与模拟，但不侵入 runtime 权威、规则执行与回放边界。

## 2. 设计结构
- 世界权威层：`WorldKernel / runtime` 继续负责校验、状态变更、事件与 receipt。
- Agent 外观层：保留 `AgentBehavior` 作为 simulator 现有调用入口，对上保持 `decide/on_event/on_action_result` 语义。
- Decision Provider 标准层：在本地定义稳定的 request / response / feedback / trace 契约，作为唯一 provider 集成点。
- Provider Adapter 层：为 `Local Provider`、本地 mock provider、未来其他 agent framework 分别实现适配器。
- Memory / Trace 桥接层：把 provider transcript、tool trace、diagnostics、memory write intent 收敛到本地 `AgentMemory` 与 `AgentDecisionTrace`。
- 验证与评估层：通过 golden fixtures、错误签名与 QoS 指标，对不同 provider 进行横向评估。

## 3. 关键接口 / 入口
- `AgentBehavior`：`crates/oasis7/src/simulator/agent.rs`
- `AgentDecisionTrace`：`crates/oasis7/src/simulator/agent.rs`
- `AgentMemory`：`crates/oasis7/src/simulator/memory.rs`
- viewer 诊断协议：`crates/oasis7_proto/src/viewer.rs`
- runtime live 真 LLM 接管专题：`doc/world-simulator/viewer/viewer-live-runtime-world-llm-full-bridge-2026-03-05.prd.md`
- launcher GUI Agent 统一机器接口专题：`doc/world-simulator/launcher/game-client-launcher-web-console-gui-agent-interface-2026-03-08.prd.md`

## 4. 约束与边界
- 外部 provider 只提供决策建议，不得直接改 world state、存储或 runtime 内核。
- 所有 world action 必须先经过本地 action schema 白名单，再进入 runtime 校验。
- provider trace 需要可脱敏、可裁剪、可映射；不得把外部协议泄露为 viewer 唯一调试口径。
- memory 的权威副本保留在本地 world-simulator；外部 memory 仅可作为 provider 内部缓存或检索辅助。
- `Local Provider` PoC 只允许在低频、低破坏性 agent 类型上试点；高频强一致 actor 不在首轮范围。
- required 测试必须可离线执行，因此标准层必须先支持 `MockProvider`，不能以外部联网能力作为主验证前提。

## 5. 设计演进计划
- 先冻结 `Decision Provider` 最小契约与 fixture 评估口径。
- 再实现 `MockProvider`，验证标准层与本地 trace / memory / action mapping 正常闭环。
- 再实现 `Local ProviderAdapter` PoC，并限制到单一低频 NPC 场景。
- 最后依据动作有效率、超时率、成本与诊断完整度，决定是否扩展到更多 agent 类型。
