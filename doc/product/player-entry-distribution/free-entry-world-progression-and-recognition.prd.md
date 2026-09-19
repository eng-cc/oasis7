# 免费进入、世界内成长与有界认可

## 文档身份

- 所属产品模块：玩家接入与发行
- 上位产品 PRD：[`prd.md`](prd.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- 专业域权威：[`doc/game/prd.md`](../../game/prd.md)、[`doc/world-simulator/prd.md`](../../world-simulator/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)
- Last reviewed：2026-09-13

本文是长期产品分册，定义免费基础进入、可选服务、世界内成长、认可和区域互赖之间的产品边界。它不定义支付渠道、定价、账户或 onboarding 实现、具体资格算法、奖励数值、OC 发放/兑换、治理字段或当前 preview 放行结论。

## 设计适用性与生命周期闭合

- 设计判定：`simple-topic-exemption`（`PRD-only-sufficient`）。
- 设计判定 task issue：#3680。
- 设计适用性理由：本 PRD 只规定便利服务、独立基线和认可资格的产品边界；独立 design 不会增加不同的玩家操作语义。
- 当前 GitHub task evidence：本次分类见 [Issue #3680 C4 设计判定](https://github.com/eng-cc/oasis7/issues/3680#issuecomment-5652452993)，本次闭合要求见 [Issue #3680 accepted repair](https://github.com/eng-cc/oasis7/issues/3680#issuecomment-5652870280)。
## 1. 产品目标

长期产品目标是让玩家无需购买客户端、账户或基础进入资格即可进入受支持世界路径；付费只能购买可选的 hosting、storage、support 等便利服务，不能购买、租用或跳过世界中的实质权力。早期体验应容易进入，并把愿意继续的玩家引向世界规则与玩法系统中真实、有代价、可审计的成长、认可和区域协作路径；这些世界内语义分别由[`成熟世界成长与区域参与`](../world-rules-core-gameplay/mature-world-progression.prd.md#req-wr-mw-001)与[`沟通、合同、声誉与 R&D 连续性`](../world-rules-core-gameplay/communication-contracts-reputation-and-rd-continuity.prd.md#req-wr-cr-003)主责，本分册不复制其规范性条件。

该目标描述的是产品方向与组合验收要求，不表示当前任何 preview 已实现免费广泛可用、某项付费服务已上线，或当前版本已经通过发行/可玩性门禁；当前入口、可用性和公开 claim 始终以根 `README.md`、对应专业域和同一候选证据为准。

## 2. 范围与玩家边界

### 2.1 免费基础进入与可选服务

- 免费客户端、账户和基础进入只解决进入世界路径的门槛；它们不承诺免费 Agent、免费世界资产、无成本建造、无限补贴、经济旁路或对当前技术预览的无条件可用性。
- 可选付费 hosting、storage 或 support 可以改善托管、保留、协助或时间便利，但不得直接授予/出售/租用世界资源、行动权限、设施控制、训练成果、模块能力、合同结果、凭证、区域优先权、OC、治理票权或冲突优势。
- 便利服务也不得绕过资源、时间、资格、授权、物流、治理、反支配或反滥用边界；若服务影响世界内动作，它只能在与免费路径相同的权威规则和可审计因果下执行。
- 首个 Agent 的取得、持续经营和任何受限资助仍由世界规则与专业 authority 决定。本分册不把免费账户或可选服务改写成免费 Agent claim，也不改变既有 OC 到 quota 的单向边界。

#### 可选服务中断、取消与独立基线

- hosting、storage 或 support 的不可用、降级、到期、取消或续费失败，只能改变其便利交付状态；它们不得自动撤销玩家身份、Agent 控制权、已确认世界结果、仍有效的合同义务或历史 receipt，也不得把服务状态改写成世界权威状态。
- 玩家 surface 必须区分“便利服务不可用”“世界权威不可用”和“动作尚未结算”。恢复、重购或更换服务商本身不确认、回滚、迁移或补发世界内结果；未结算动作由对应专业合同明确保持待决、拒绝、到期或要求重新规划。
- 服务中断时，玩家必须获得与真实状态相符的下一步，例如保留或导出适用数据、重新进入、等待、采用不依赖该服务的独立路径，或在无法安全继续时停止。可选服务不得成为基本成长、独立恢复或读取已确认历史的唯一正常路径；备份、缓存或支持工单也不得代签世界迁移、恢复或结算。
- 重购、续费成功或更换服务商只建立新的便利服务承诺；它们不自动续期世界内资格、恢复已经失效的机会或扩大玩家的世界权力。具体支付、保留期、导出格式、迁移协议与恢复实现仍由对应专业 authority 定义。

### 2.2 易进入而自愿展开的真实深度

- 早期路径应让玩家先读到当前目标、主要阻塞、下一步、下一决策和可理解的世界后果；首局至首次持续能力的目标清晰度、渐进披露与恢复闭环由[`首局与持续游玩`](../world-rules-core-gameplay/first-session-and-continuation.prd.md#1-产品目标)主责。
- 首次持续能力之后，真实成本、锁定、风险、权限、损失、恢复和非账号成长由[`成熟世界成长与区域参与`](../world-rules-core-gameplay/mature-world-progression.prd.md#req-wr-mw-001)主责；本分册仅保留“容易进入、可自愿展开、不能把账号当永久 power tree”的入口组合说明。
- 本专题的 `FE-2` 是跨模块组合验收：它消费首局/成熟世界的世界内能力、可审计历史和后续路径证据，不在玩家接入模块另立成长循环或账号权力语义。

### 2.3 有界认可、独立基线与区域互赖

- 世界内成长、独立基线、区域互赖和通用机会的规范性主责已经迁入[`成熟世界成长与区域参与`](../world-rules-core-gameplay/mature-world-progression.prd.md#req-wr-mw-001)；其 [`REQ-WR-MW-004`](../world-rules-core-gameplay/mature-world-progression.prd.md#req-wr-mw-004) / [`AC-WR-MW-004`](../world-rules-core-gameplay/mature-world-progression.prd.md#ac-wr-mw-004) 承接来源/范围/期限/复核、待决/hold/receipt、并发单次效果和失效恢复边界。
- 情境声誉的主体、地点/服务/合同/时间范围、更新、到期和申诉由[`沟通、合同、声誉与 R&D 连续性`](../world-rules-core-gameplay/communication-contracts-reputation-and-rd-continuity.prd.md#req-wr-cr-003)的 [`REQ-WR-CR-003`](../world-rules-core-gameplay/communication-contracts-reputation-and-rd-continuity.prd.md#req-wr-cr-003) / [`AC-WR-CR-003`](../world-rules-core-gameplay/communication-contracts-reputation-and-rd-continuity.prd.md#ac-wr-cr-003) 主责；本专题只保留免费/付费服务、访问和外部参与不能自动兑换世界内认可或资格的入口边界。
- `FE-3` 与 `FE-4` 是跨模块组合验收：玩家可读的世界机会、独立基线和区域专业化优势必须分别满足成熟世界及情境声誉的主责条款，不在玩家接入模块重述成长或资格规则。pioneer priority 与区域设施容量继续使用其既有窄范围专业专题。

### 2.4 认可的生命周期、失效与反滥用

- 认可的来源、范围、用途、期限、复核/申诉、失效、反滥用和恢复规则由[`成熟世界成长与区域参与`](../world-rules-core-gameplay/mature-world-progression.prd.md#req-wr-mw-004)主责；情境声誉记录与更新由[`沟通、合同、声誉与 R&D 连续性`](../world-rules-core-gameplay/communication-contracts-reputation-and-rd-continuity.prd.md#req-wr-cr-003)主责。本专题保留 `REQ-ENTRY-FREE-002` / `AC-ENTRY-FREE-002` 作为入口的负边界与组合验收，不再另立一套认可生命周期。
- 入口必须继续说明：付费便利、访问、在线时长、泛化互动或自我声明不会自动兑换世界内认可、资格、区域优先级或权力；世界内事实、专业审核和治理决定的具体资格、评分、分配与执行仍由 gameplay/runtime/blockchain/QA authority 决定。

#### 容量竞争下的机会申领生命周期

- 容量竞争的预览、待决、hold/排队、receipt、并发去重和失败恢复统一消费[`成熟世界成长与区域参与`](../world-rules-core-gameplay/mature-world-progression.prd.md#ac-wr-mw-004)；pioneer priority 和区域设施提交分别回链[`Frontier 扩展与世界信息边界`](../world-rules-core-gameplay/frontier-expansion-and-world-information-boundaries.prd.md#req-wr-fi-002)与[`受治理的区域能力与扩展`](../world-rules-core-gameplay/governed-regional-capabilities-and-extensions.prd.md#req-wr-gr-001)，不能在入口专题泛化。
- `FE-6` / `FE-7` 保留为跨模块组合验收：失效/拒绝/撤销不删除历史、不封锁独立基线；资格/预览不产生容量效果，至多一个当前有效 receipt 产生世界效果，其余请求明确拒绝、释放或待决并提供下一步。上述规则的唯一规范性主责是目标 gameplay 专题。

## 3. 权威与冲突处理

| 产品层拥有 | 专业与执行权威 |
| --- | --- |
| 免费进入与非权力型便利服务的产品边界、世界内成长/认可/区域机会的跨模块组合要求、以及长期目标和当前 claim 的隔离 | 根 `README.md` 拥有当前公开状态与 claim envelope；`doc/world-simulator/prd.md` 拥有客户端、账户、入口、首个 Agent/onboarding 与发行实现；`doc/game/prd.md` 拥有成长、经济、玩法和数值；`doc/testing/prd.md` 拥有证据和当前 verdict；规范性世界成长、情境声誉与机会申领分别由[`成熟世界成长与区域参与`](../world-rules-core-gameplay/mature-world-progression.prd.md#req-wr-mw-004)与[`沟通、合同、声誉与 R&D 连续性`](../world-rules-core-gameplay/communication-contracts-reputation-and-rd-continuity.prd.md#req-wr-cr-003)主责 |

产品层不得定义支付实现、价格、账户数据、资格评分、OC 额度或兑换、治理计票、世界资产发放、Agent claim 规则或具体成长/平衡数值。任何当前可用性、付费服务、资格或奖励说明缺少同一候选的专业证据与根 `README.md` 支持时，采用更窄的未承诺边界。

## 4. 路线图

1. 基础进入公平：把客户端、账户和基础进入与世界权力及付费便利服务明确分离。
2. 世界内成长：让早期路径可理解，随后由[`成熟世界成长与区域参与`](../world-rules-core-gameplay/mature-world-progression.prd.md#req-wr-mw-001)承接有真实成本与后果的深度，而非账号 power tree。
3. 有界协作：由成熟世界与[`沟通、合同、声誉与 R&D 连续性`](../world-rules-core-gameplay/communication-contracts-reputation-and-rd-continuity.prd.md#req-wr-cr-003)承接独立基线、区域互赖和情境认可；本专题只保留入口与 claim 的组合边界。
4. 诚实发行：只在当前候选证据支持时，将其中已实现的部分写入公开 claim。

## 4.1 叶级产品要求与验收

<a id="req-entry-free-001"></a>
### REQ-ENTRY-FREE-001：便利服务不得购买世界权力

- 要求：免费基础进入必须与世界权力分离；hosting、storage 或 support 等可选服务只能改善便利交付，不能购买、租用、跳过或自动恢复资源、权限、治理、合同、Agent 能力或区域控制。
- 验收：AC-ENTRY-FREE-001

<a id="ac-entry-free-001"></a>
### AC-ENTRY-FREE-001：服务中断保留独立基线与历史

- 覆盖要求：REQ-ENTRY-FREE-001
- 场景与结果：便利服务不可用、降级、到期、取消或续费失败时，玩家身份、Agent 控制权、已确认世界结果和仍有效义务保持可追溯；玩家仍有不依赖该服务的读取、等待、重新进入、独立行动或安全停止路径，重购/换商不自动扩权。
- 证据边界：支付、托管、存储、迁移和世界执行由对应专业 authority 验证；产品层不声称服务当前上线或可用。

<a id="req-entry-free-002"></a>
### REQ-ENTRY-FREE-002：认可与机会必须有界且可失效

- 要求：入口必须消费并显式链接[`成熟世界成长与区域参与`](../world-rules-core-gameplay/mature-world-progression.prd.md#req-wr-mw-004)和[`沟通、合同、声誉与 R&D 连续性`](../world-rules-core-gameplay/communication-contracts-reputation-and-rd-continuity.prd.md#req-wr-cr-003)的世界内认可/机会主责；免费基础进入、付费便利、访问或外部参与不得自动变成世界资产、资格、区域优先级、OC、治理权、区域控制或永久全局权力。目标专题定义的失效仍不得抹除历史或封锁独立基线。
- 验收：AC-ENTRY-FREE-002

<a id="ac-entry-free-002"></a>
### AC-ENTRY-FREE-002：认可到期不会重放为新资格

- 覆盖要求：REQ-ENTRY-FREE-002
- 场景与结果：入口组合样例能回链成熟世界 [`REQ-WR-MW-004`](../world-rules-core-gameplay/mature-world-progression.prd.md#req-wr-mw-004) / [`AC-WR-MW-004`](../world-rules-core-gameplay/mature-world-progression.prd.md#ac-wr-mw-004) 与情境声誉 [`REQ-WR-CR-003`](../world-rules-core-gameplay/communication-contracts-reputation-and-rd-continuity.prd.md#req-wr-cr-003) / [`AC-WR-CR-003`](../world-rules-core-gameplay/communication-contracts-reputation-and-rd-continuity.prd.md#ac-wr-cr-003)；代表性授予、使用、到期、拒绝、暂停或撤销仍能读到来源/范围/时间边界，历史认可、旧 receipt、重复申领或重连重试不会产生第二次分配、隐性优先权或永久权力，失败方仍可走独立路径、补证、替代路线、恢复或复核。
- 证据边界：资格、评分、分配、OC、治理和反滥用实现由 gameplay/runtime/blockchain/QA authority 定义；产品层不冻结数值或算法。

## 5. Done：成功标准与验收

- FE-1：产品说明能区分免费客户端/账户/基础进入、世界内需要取得和维护的实质能力，以及不授予世界权力的可选付费便利服务；不会把其中任何一项误写成当前 preview 已广泛可用。
- FE-2：组合[`首局与持续游玩`](../world-rules-core-gameplay/first-session-and-continuation.prd.md#1-产品目标)与[`成熟世界成长与区域参与`](../world-rules-core-gameplay/mature-world-progression.prd.md#req-wr-mw-001)的证据，证明玩家可逐步展开真实深度并读懂实质后果；持续 power 不来自永久账号树或全局强度分，而来自可审计的世界内资产、训练、模块、合同、credentials 和关系。
- FE-3：组合成熟世界 [`REQ-WR-MW-004`](../world-rules-core-gameplay/mature-world-progression.prd.md#req-wr-mw-004) / [`AC-WR-MW-004`](../world-rules-core-gameplay/mature-world-progression.prd.md#ac-wr-mw-004) 与情境声誉 [`REQ-WR-CR-003`](../world-rules-core-gameplay/communication-contracts-reputation-and-rd-continuity.prd.md#req-wr-cr-003) / [`AC-WR-CR-003`](../world-rules-core-gameplay/communication-contracts-reputation-and-rd-continuity.prd.md#ac-wr-cr-003) 的认可样例，只授予有界未来机会或资格，并说明范围、期限、用途、事实/审核来源与复核条件；它不自动授予 power、OC、治理或区域控制。
- FE-4：[`成熟世界成长与区域参与`](../world-rules-core-gameplay/mature-world-progression.prd.md#req-wr-mw-001)样例证明玩家具备不立即依附强组织的可行独立基线，同时区域专业化与互赖在不强制依附的前提下带来可读优势。
- FE-5：任何公开或入口 surface 将长期方向、当前候选证据、实际受支持入口与未承诺内容分开；历史或局部 evidence 不得代签免费可用性、支付服务、成长完整性或发行就绪。
- FE-6：组合成熟世界 [`REQ-WR-MW-004`](../world-rules-core-gameplay/mature-world-progression.prd.md#req-wr-mw-004) / [`AC-WR-MW-004`](../world-rules-core-gameplay/mature-world-progression.prd.md#ac-wr-mw-004) 与情境声誉 [`REQ-WR-CR-003`](../world-rules-core-gameplay/communication-contracts-reputation-and-rd-continuity.prd.md#req-wr-cr-003) / [`AC-WR-CR-003`](../world-rules-core-gameplay/communication-contracts-reputation-and-rd-continuity.prd.md#ac-wr-cr-003) 的证据，证明认可的授予、使用、到期、拒绝、暂停或撤销都能说明来源、范围、用途和时间边界；无效或被滥用的认可不会被交易、重放或洗成永久权力，且其处置不静默删除历史、改写已确认因果或封锁独立基线。
- FE-7：组合成熟世界 [`REQ-WR-MW-004`](../world-rules-core-gameplay/mature-world-progression.prd.md#req-wr-mw-004) / [`AC-WR-MW-004`](../world-rules-core-gameplay/mature-world-progression.prd.md#ac-wr-mw-004) 与适用的 frontier/区域设施窄范围合同，证明两名合格玩家竞争同一有限机会时，资格/邀请和申领预览不产生容量效果；最多一项符合当前条件的申领以 receipt 结算；其余请求明确拒绝/释放或保持待决，不产生分配、欠费或隐性优先权；重连重试不产生第二次效果，且失败方仍有可读恢复或独立路径。
- FE-8：可选服务不可用、降级、到期、取消或续费失败的样例证明：便利服务状态与世界权威/结算状态保持分离；玩家身份、Agent 控制权、已确认历史和仍有效义务连续可追溯；未结算动作按专业合同真实处置；玩家具有不依赖付费服务的恢复或安全停止路径；重购、续费或换商不自动续期资格、恢复机会或扩大世界权力。

### 5.1 验收追踪

| 成功标准 | 专业 owner | 专业域 PRD-ID | 权威文档 | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- | --- |
| FE-1 | producer_system_designer / viewer_engineer / qa_engineer / liveops_community | PRD-WORLD_SIMULATOR-042/043/045 / PRD-TESTING-003 | `README.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 免费基础入口、可选便利服务和当前 claim 的负例/组合审计 | test_tier_required |
| FE-2 | producer_system_designer / gameplay_designer / runtime_engineer / agent_engineer / viewer_engineer | PRD-GAME-015 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_SIMULATOR-001 | `doc/product/world-rules-core-gameplay/first-session-and-continuation.prd.md`; `doc/product/world-rules-core-gameplay/mature-world-progression.prd.md`; `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md` | 首局至成熟世界的世界内能力、渐进深度和非账号 power tree 组合证据；本专题仅消费该结果 | test_tier_required |
| FE-3 | producer_system_designer / gameplay_designer / blockchain_ops_engineer / qa_engineer | PRD-GAME-015 / PRD-P2P-001 / PRD-TESTING-003 | `doc/product/world-rules-core-gameplay/mature-world-progression.prd.md`; `doc/product/world-rules-core-gameplay/communication-contracts-reputation-and-rd-continuity.prd.md`; `doc/game/prd.md`; `doc/p2p/prd.md`; `doc/testing/prd.md` | 有界资格/机会、情境来源与复核、非自动 OC/治理/power 与可审计来源的组合样例；本专题作为入口消费者 | test_tier_required |
| FE-4 | producer_system_designer / gameplay_designer / agent_engineer / runtime_engineer / qa_engineer | PRD-GAME-015 / PRD-WORLD_RUNTIME-001 / PRD-TESTING-003 | `doc/product/world-rules-core-gameplay/mature-world-progression.prd.md`; `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/testing/prd.md` | 独立基线、区域互赖优势与非强制依附的 mature-world fresh sample | test_tier_full |
| FE-5 | producer_system_designer / viewer_engineer / qa_engineer / liveops_community | PRD-WORLD_SIMULATOR-042/043 / PRD-TESTING-003 | `README.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 长期方向、候选证据、支持入口和公开 claim 分离审计 | test_tier_required |
| FE-6 | producer_system_designer / gameplay_designer / runtime_engineer / blockchain_ops_engineer / qa_engineer | PRD-GAME-015 / PRD-WORLD_RUNTIME-001 / PRD-P2P-001 / PRD-TESTING-003 | `doc/product/world-rules-core-gameplay/mature-world-progression.prd.md`; `doc/product/world-rules-core-gameplay/communication-contracts-reputation-and-rd-continuity.prd.md`; `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/p2p/prd.md`; `doc/testing/prd.md` | 认可来源/范围/期限、拒绝/暂停/撤销、不可交易/重放、历史与因果连续性、独立基线与复核路径的组合证据；本专题保留入口组合验收 | test_tier_full |
| FE-7 | producer_system_designer / gameplay_designer / runtime_engineer / blockchain_ops_engineer / viewer_engineer / qa_engineer | PRD-GAME-015 / PRD-WORLD_RUNTIME-001 / PRD-P2P-001 / PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | `doc/product/world-rules-core-gameplay/mature-world-progression.prd.md`; `doc/product/world-rules-core-gameplay/frontier-expansion-and-world-information-boundaries.prd.md`; `doc/product/world-rules-core-gameplay/governed-regional-capabilities-and-extensions.prd.md`; `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/p2p/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 双申领人容量竞争样例的预览/待决/hold 或排队/receipt 语义、原子单次效果、重试幂等、失败恢复与独立基线证据；本专题保留跨模块组合验收 | test_tier_full |
| FE-8 | producer_system_designer / runtime_engineer / viewer_engineer / qa_engineer | PRD-WORLD_RUNTIME-001 / PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 服务中断/降级/到期/取消下的状态分离、已确认历史连续性、未结算动作处置、独立恢复或安全停止，以及非自动续期/恢复/扩权的组合证据 | test_tier_required |

## 6. Non-Goals

- 不设计支付、订阅、托管、存储、支持、账户、登录或 onboarding 实现，也不声明任何服务已销售或可用。
- 不改变首个 Agent 的 claim/upkeep 合同、受限资助语义或 OC 到 quota 的既有单向桥接。
- 不定义永久等级、全局战力分、职业数值、资格算法、奖励分配、OC 经济、治理权或区域控制的执行规则。
- 不把长期免费进入、世界内成长或区域互赖目标当作当前 preview readiness、release gate、可玩性或公开承诺的替代证据。

## 全量语义追踪

| REQ / AC | 专业 owner | 专业权威 | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- |
| [REQ-ENTRY-FREE-001](#req-entry-free-001) / [AC-ENTRY-FREE-001](#ac-entry-free-001) | `producer_system_designer` | [`doc/game/prd.md`](../../game/prd.md#3-player-facing-authority-boundary)、[`doc/world-simulator/prd.md`](../../world-simulator/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md) | 本专题对应要求、验收与专业 authority 的可导航追踪证据 | `test_tier_required` |
| [REQ-ENTRY-FREE-002](#req-entry-free-002) / [AC-ENTRY-FREE-002](#ac-entry-free-002) | `producer_system_designer` | [`doc/game/prd.md`](../../game/prd.md#3-player-facing-authority-boundary)、[`成熟世界成长与区域参与`](../world-rules-core-gameplay/mature-world-progression.prd.md#req-wr-mw-004)、[`沟通、合同、声誉与 R&D 连续性`](../world-rules-core-gameplay/communication-contracts-reputation-and-rd-continuity.prd.md#req-wr-cr-003)、[`doc/world-simulator/prd.md`](../../world-simulator/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md) | 入口负边界与成熟世界/情境声誉主责的可导航组合追踪；认可失效、容量竞争、独立恢复和 specialized authority 不重复立法 | `test_tier_full` |
