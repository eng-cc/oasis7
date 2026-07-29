# LLM Agent Decision Provider 标准层 + Local Provider 外部适配可行性（2026-03-12）项目管理文档

- 对应设计文档: `doc/world-simulator/llm/decision-provider-contract.design.md`
- 对应需求文档: `doc/world-simulator/llm/decision-provider-contract.prd.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] T0 (PRD-WORLD_SIMULATOR-036) [test_tier_required]: 完成 `Decision Provider` 标准层 + `Local Provider` 外部适配可行性 PRD / Design / Project 建模，并回写模块主文档、索引与 devlog。
- [x] T1 (PRD-WORLD_SIMULATOR-036) [test_tier_required]: 在 simulator 侧冻结 provider contract 类型与 golden observation fixture，形成 provider-agnostic 契约测试样本。
- [x] T2 (PRD-WORLD_SIMULATOR-036) [test_tier_required]: 实现 `MockProvider`，验证 `AgentBehavior facade -> DecisionProvider -> runtime -> trace` 最小闭环可离线运行。
- [ ] T3 (PRD-WORLD_SIMULATOR-036) [test_tier_full]: 实现 `Local ProviderAdapter` PoC，限定在低频、低破坏性动作集上试点；完成定义改挂到 `PRD-WORLD_SIMULATOR-038` 的 parity 通过线，禁止以“已接通”代替“已完成”。
- [ ] T4 (PRD-WORLD_SIMULATOR-036) [test_tier_required]: 完成 provider trace / memory write intent / error policy 映射，保持与 viewer/QA 诊断契约一致。
- [ ] T5 (PRD-WORLD_SIMULATOR-036) [test_tier_full]: 选取单一低频 NPC 场景做闭环评估，对比本地 provider 与 `Local Provider` provider 的动作有效率、超时率、成本与 trace 完整度。

## 依赖
- `doc/world-simulator/prd.md`
- `doc/world-simulator/project.md`
- `doc/world-simulator/prd.index.md`
- `crates/oasis7/src/simulator/agent.rs`
- `crates/oasis7/src/simulator/memory.rs`
- `crates/oasis7_proto/src/viewer.rs`
- `doc/world-simulator/viewer/viewer-control-plane-split-live-playback.prd.md`
- `doc/world-simulator/prd.md`
- `doc/world-simulator/launcher/game-client-launcher-control-plane-and-machine-interface.prd.md`（仅 operator HTTP-JSON interface，不提供 `DecisionProvider` 语义）

## 状态
- 最近更新：2026-03-12
- 当前阶段: T3 pending
- 当前任务: `实现 Local ProviderAdapter PoC，并以 parity 通过线作为完成门禁`
- owner: `agent_engineer`
- 联审: `runtime_engineer`、`viewer_engineer`
- 发起建模: `producer_system_designer`
- 备注: `T1/T2` 已完成并形成离线 required 测试基座；后续 `T3/T5` 仍必须同时满足 `PRD-WORLD_SIMULATOR-038` 的 parity 门禁，禁止把 provider 接通视作功能完成。
- 专业权威合并不改变 `T3/T4/T5` pending；历史 runtime-live bridge 文档退役不代表 adapter parity、trace/error mapping 或成本稳定性已完成。
- 已吸收的多场景评测与工业调试长跑只提供历史 harness / 风险演进证据：后续评测须固定 scenario/fixture/profile、provider/adapter/协议版本、timeout、tick budget 与并行度，保留分场景和聚合工件，并对非确定性 provider 重复采样。它们不关闭 T3/T4/T5，不证明当前 provider parity、成本或默认启用资格；debug resource injection 运行也不得纳入普通 parity 样本。

## 已吸收的 builtin Agent 与 prompt-receipt provenance（2026-07-29）

- builtin `LlmAgentBehavior` 已完成配置/profile 解析、Responses transport、结构化最终决策、失败收敛、demo/viewer lane 与定向单测；其完成记录解释当前基线，不改变本专题 T3/T4/T5 pending，也不证明外部 provider parity、Viewer 等价或默认启用。
- 已完成 simulator-side module-call intent/receipt、trace event 与 journal diagnostic wiring，并保持旧 trace 的兼容默认值；当时也完成 oasis7_viewer 对新增 trace/event 字段的兼容与全仓测试。该完成仅是 Viewer 对诊断字段的兼容 provenance，不证明跨 surface 体验等价、玩家可见因果、完整 receipt/replay closure 或 release readiness。未完成债务仍是 shared effect schema 以及 T4/T5 的 action-result 因果、replay-no-provider-call 与恢复幂等证明。
- 本次来源合并只把可持续的行为/诊断语义放入本专题；详细实施历史继续由 Git history 与 GitHub task evidence 追溯。后续评测仍须使用固定 scenario/fixture/profile/provider/adapter/protocol/timeout epoch 和逐场景工件，历史样本、总量或并行度不能证明成本、稳定性或 release readiness。
