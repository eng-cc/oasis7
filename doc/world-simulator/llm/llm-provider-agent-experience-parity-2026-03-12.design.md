# Local Provider 与内置 Agent 体验等价（parity）验收方案（2026-03-12）设计

- 对应需求文档: `doc/world-simulator/llm/llm-provider-agent-experience-parity-2026-03-12.prd.md`
- 对应项目管理文档: `doc/world-simulator/llm/llm-provider-agent-experience-parity-2026-03-12.project.md`

## 1. 设计定位
定义 `Local Provider` provider 与内置 agent 的体验等价验收框架，使“是否可默认启用/是否可扩大覆盖范围”由标准化 parity 结果驱动，而不是由单次 PoC 或主观印象驱动。

## 2. 设计结构
- 场景层：按 `P0 低频单 NPC`、`P1 多轮记忆`、`P2 多 agent 并发` 分层。
- 指标层：采集完成率、无效动作率、超时率、额外等待时间、trace 完整度、恢复率。
- 评分层：组合自动 benchmark 与 QA/producer 主观评分卡。
- 准入层：基于通过线/阻断线判定 `experimental`、`gated`、`default_ready`。
- 追溯层：记录 provider 版本、adapter 版本、协议版本与场景结果。

## 3. 关键接口 / 入口
- parity 场景列表与评分卡
- builtin vs Local Provider 对照结果汇总
- `Decision Provider` trace 与 diagnostics
- launcher / viewer provider 状态展示

## 4. 约束与边界
- parity 目标针对“用户体验”，不要求内部实现完全一致。
- 未通过 parity 的 provider 不得标记为默认体验。
- 首期 parity 仅覆盖低频和中低复杂度场景，不扩展到高频战斗/经济关键路径。
- 自动指标与主观评分必须同时存在，缺一不可。

## 5. 设计演进计划
- 先冻结 P0/P1/P2 场景和指标阈值。
- 再补自动 benchmark 与评分卡。
- 再执行真实 `Local Provider(Local HTTP)` 对标试玩。
- 最后基于 parity 结论决定是否进入默认体验。
