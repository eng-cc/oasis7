# 智能体与世界模拟 PRD

## 文档身份

- 产品模块：智能体与世界模拟
- 产品模块 slug：`agents-world-simulation`
- 产品层唯一 PRD：`doc/product/agents-world-simulation/prd.md`
- 产品模块总入口：`doc/product/README.md`
- Product PRD-ID：`PRD-PRODUCT-003`
- 生命周期：`active`
- Owner role：`producer_system_designer`
- Last reviewed：`2026-07-18`
- 后继文档：`无`
- 下层专业域：[`doc/world-simulator/prd.md`](../../world-simulator/prd.md)

本文只承载场景、Agent/provider 决策、权威世界状态与可交互表达的产品组合。Viewer、Launcher、provider 与模拟合同由 `world-simulator` 专业域拥有。

## 1. 产品承诺

玩家进入的是场景、Agent 意图、世界事件和交互反馈相互对应的持久模拟，而不是一次性聊天或本地假世界。

## 2. 范围与玩家边界

覆盖可重现场景、Agent/LLM provider 权威、世界事件流、主 Web 界面、Launcher 转移和交互反馈。玩家可以观察、发起被允许的意图并看到权威结果；不能把 mock、本地演示或未授权 provider 输出当作世界状态。

## 3. 权威与冲突处理

| 产品层拥有 | 专业域权威 |
| --- | --- |
| 场景、Agent/provider、世界状态到交互模拟的端到端承诺 | `doc/world-simulator/prd.md` 拥有 Viewer、Launcher、Agent/provider、场景和验证合同 |

产品 PRD 不改写具体模式、API、UI 或 provider 选择；冲突时由对应 `agent_engineer`、`viewer_engineer` 或 runtime owner 与产品 owner 共同裁决。

## 4. 路线图

1. 可重现场景：初始状态、资源变化、关键事件与验收证据一致。
2. 权威 Agent 闭环：provider 和 Agent 意图不绕过世界校验。
3. 多入口等价：主 Web、Launcher 与 pure API 在权威状态与核心动作上一致。

## 5. Done：成功标准与验收

- SC-1：关键场景可从固定初始状态重现到相同权威事件和交互结果。
- SC-2：Agent/provider 的输入、决策、世界接受或拒绝及玩家反馈可追踪。
- SC-3：主 Web 入口呈现真实世界状态、核心动作和结构化失败。
- SC-4：Launcher、Viewer 与 pure API 不会将不同模式的证据或 claim 混为同一事实。

### 5.1 验收追踪

| 成功标准 | 专业 owner | 专业域 PRD-ID | 权威文档 | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- | --- |
| SC-1 | runtime_engineer | PRD-WORLD_SIMULATOR-001/002/003 | `doc/world-simulator/prd.md` | 场景初始化、事件与回放证据 | test_tier_required |
| SC-2 | agent_engineer | PRD-WORLD_SIMULATOR-016-019 | `doc/world-simulator/prd.md` | live provider 和权威行为闭环 | test_tier_required |
| SC-3 | viewer_engineer | PRD-WORLD_SIMULATOR-039/041/046 | `doc/world-simulator/prd.md` | 主 Web Viewer 真实世界交互证据 | test_tier_required |
| SC-4 | qa_engineer | PRD-WORLD_SIMULATOR-020-031 | `doc/world-simulator/prd.md` | 模式、转移与多入口 claim 回归 | test_tier_required |

## 6. Non-Goals

- 不在产品层决定 Viewer 组件、Launcher 协议、provider 或 runtime 实现。
- 不将 mock、software-safe、本地回退或单次截图包装为真实产品闭环。
- 不复制 `world-simulator` 专题 PRD、project 任务或测试步骤。
