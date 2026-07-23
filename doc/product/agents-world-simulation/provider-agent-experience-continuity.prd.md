# Agent/provider 体验连续性

## 文档身份

- 所属产品模块：智能体与世界模拟
- 上位产品 PRD：[prd.md](prd.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- 专业域权威：[Local Provider 与内置 Agent 体验等价（parity）验收方案](../../world-simulator/llm/provider-agent-experience-parity.prd.md)

本文是长期产品分册，定义 Agent/provider 切换时玩家体验连续性的产品承诺。它不声明任何 provider 当前受支持、可用、默认或已就绪；具体 provider 组合的场景范围、评估、技术合同和结论仅由专业域权威文档维护。

## 1. 产品目标

当玩家在专业域已明确场景范围的 provider 组合间切换时，切换不能带来无法解释的、实质性的体验退化。连续性以玩家可感知的目标结果、等待体验、多轮记忆与意图延续、问题可诊断性及恢复路径为准，而不要求内部实现相同。

体验连续性不改变世界规则或执行权威。provider 只影响 Agent 如何形成意图；世界结果仍须经过同一权威裁决，不能因切换而把未发生、未接受或被拒绝的结果呈现为成功。

`player_parity` 与 `headless_agent` 是 Agent 的执行 lane，不是新的玩家访问模式。无论 Agent 经由哪一 lane 提出意图，玩家所理解、纠正或继续行动的只能是同一条权威世界结果链；lane 差异不得制造另一份世界事实、绕开裁决，或把 Agent 执行能力误述为玩家获得新的进入权限。两条 lane 的具体合同、适用范围与 `experimental` / 非默认边界由[专业域 parity 权威](../../world-simulator/llm/provider-agent-experience-parity.prd.md)维护，产品层不将其提升为当前支持、默认或发布就绪结论。

## 2. 玩家边界

- 玩家可以期待在已声明场景范围内继续理解当前目标、Agent 意图、世界结果与下一步，而不需要因 provider 切换重新猜测世界事实。
- 当组合不在适用范围内，或体验处于受限、退化或阻塞状态时，产品必须如实表达该状态，保留权威世界真相，并给出适用于当前状态的下一步；不得把回退、静默替代或局部技术通路包装成等价体验。
- 连续性承诺不等于所有 provider、所有场景或所有体验维度天然等价，也不以一次成功、单一入口或历史证据代签当前产品结论。

## 3. 产品验收

- PC-1：在专业域明确的场景范围内，provider 切换后的玩家可感知目标结果、等待体验、多轮记忆与意图延续、可诊断性和恢复路径不存在无法解释的实质性退化。
- PC-2：受限、退化、阻塞或不适用的情形不会伪造成功或改写权威世界状态；玩家能够理解当前限制及适用的下一步。
- PC-3：产品验收只引用专业域对具体组合和场景的证据，不把历史评估、局部样本或技术接通提升为当前支持、默认或发布就绪结论。

### 3.1 验收权威与证据边界

| 产品承诺 | 专业 owner | 专业域权威 | 证据边界 | 测试层级 |
| --- | --- | --- | --- | --- |
| 场景内的体验连续性 | agent_engineer / viewer_engineer / qa_engineer | `doc/world-simulator/llm/provider-agent-experience-parity.prd.md` | 专业域维护适用场景、评估方法与具体结论；产品层只消费其可追溯结论 | test_tier_required |
| 受限或退化时的真实表达与下一步 | agent_engineer / viewer_engineer / qa_engineer | `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 以权威世界真相、诊断和恢复证据验证；不以技术连通或截图代签 | test_tier_required |

## 4. 非目标与权威边界

- 不定义 provider profile、支持矩阵、评分、阈值、等待等级、基准协议、命令、实现接线或 rollout verdict。
- 不决定 provider 的当前支持状态、默认选择、可用性、发布准入或扩面顺序。
- 不复制或替代专业域 source 的技术和验证合同；需要具体组合判断时，回到上述专业域权威文档并由对应 owner 裁决。
