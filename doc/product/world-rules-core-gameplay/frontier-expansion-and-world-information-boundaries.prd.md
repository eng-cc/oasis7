# Frontier 扩展与世界信息边界

## 文档身份

- 所属产品模块：世界规则与核心玩法
- 上位产品 PRD：[`prd.md`](prd.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- 专业域权威：[`doc/game/prd.md`](../../game/prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/p2p/prd.md`](../../p2p/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)

本文是长期产品分册，定义有限已知世界的 frontier 扩展、pioneer priority 和世界信息可见性的产品边界。它不定义地图生成、相邻判定、物流算法、charter 字段、priority 时长/价格、情报访问控制、数据格式、侦察 UI、runtime 状态机或当前 readiness。

## 1. 产品目标

世界的已知范围是有限但可持续扩展的：玩家、Agent 与组织只能借由受治理的相邻 frontier 探索、物流建立和 charter 扩展进入新区域。扩展不是生成新的独立服务器、经济或历史，而是将新的地点、资源、义务与结果接入同一条权威时间线、同一经济和同一可审计世界历史。

玩家需要既能审计公共规则和已结算结果，又不能免费获得所有实时战略情报。探索与投资可以产生有限的先行开发机会，但不产生领土主权或永久信息垄断。

## 2. 范围与玩家边界

### 2.1 受治理的相邻 frontier 扩展

- 只有已知世界相邻的 frontier 才能被探索、经由物流建立并进入 charter 扩展路径；任意跳跃、脱离相邻关系的宣称、私有副本或另起时间线不能形成世界扩展。
- 探索、物流和 charter 分别证明发现、可达/维持以及公共组织边界；任一环节不足都不能单独把未知地点变为定居区域、完整市场或自治政治体。
- 新区域的资源、合同、损失、公共义务和历史结果持续归属同一世界经济与因果链。frontier 扩展不重置旧区域、绕过既有权利/安全边界，或把发现者的局部行动写成全局治理权。

### 2.2 Pioneer priority 不是主权

- 对满足探索和开发里程碑的主体可以授予 pioneer priority：它是可转让的、会到期、受范围限制的先行开发或协作优先机会。
- priority 必须绑定明确的开发里程碑、地点/用途与失效条件；未推进、到期、转让或违反适用规则后，机会按规则释放、转交或重新竞争。
- priority 不等于土地所有权、区域主权、永久排他、军政控制、免除 charter、免除通行义务或阻止其他合格主体探索/提供服务。任何已形成的地点 tenure、公共通行或区域权利仍按各自的受治理边界处理。

### 2.3 探索数据与公共安全基线

- 探索获得的数据可在与地点、用途和时间相称的范围内提供私有商业使用或先行研究优势；这不构成永久秘密、对他人基本恢复能力的剥夺或安全风险的隐瞒许可。
- 至少与公共安全、可通行性、重大已知危险和受影响主体基本恢复有关的最低事实必须按适用授权公开；私有期、保护期或争议处理结束后，可公开的探索知识应形成可复核的公共 baseline。个人数据、敏感漏洞细节、实时防御部署或他人私有合同不因公共 baseline 而自动公开。
- 已披露或被消费的数据都必须说明来源、范围、取得/更新时间和不确定性；过期、冲突或局部观察不得被包装成整个 frontier 的实时完整真值。

### 2.4 可审计公共事实与需要取得的实时信息

- 公共规则、已结算世界结果、已成立 charter、适用公共安全信息和可公开 baseline 必须可审计，不能被拥有更强信息能力的主体秘密改写。
- 实时库存、路线容量/状态、部署、计划、未结算订单和其他 live operational information 默认需要侦察、已有关系、购买、合同或明确授权才能获得；玩家不能因访问产品、持有 OC、加入组织或历史名望自动读取。
- 任何向玩家或 Agent 提供的实时信息必须标明 freshness、范围和不确定性，并在授权失效、侦察不足、数据冲突或状态变化时给出刷新、替代、等待或停止的可读路径。

## 3. 权威与冲突处理

| 产品层拥有 | 专业与执行权威 |
| --- | --- |
| frontier 进入的产品结果、priority 非主权、探索数据的私有/安全/公共分层，以及公共事实与实时信息的可见性边界 | `doc/game/prd.md` 拥有探索、物流、工业和玩家规则；`doc/world-runtime/prd.md` 拥有相邻/资格、世界状态、授权、receipt 与确定性执行；`doc/p2p/prd.md` 拥有分布式状态与安全技术边界；`doc/testing/prd.md` 拥有证据与当前 verdict |

本分册不定义地理算法、范围/时长/价格、情报保密技术、侦察实现、物流数值、charter/tenure 状态或公开 API。它不扩大现有区域 charter、市场发现、情报或玩家 surface 的当前专业真值；缺少同一候选专业证据时，只能保持为长期目标或较窄未承诺边界。

## 4. 路线图

1. 连续扩展：建立相邻探索、物流和 charter 缺一不可的 frontier 进入结果。
2. 有界先行：让 pioneer priority 支持早期开发而不成为永久主权或圈地。
3. 信息公平：把公共规则/已结算事实与实时运营情报分层，并让探索数据最终可形成安全的公共 baseline。
4. 可读不确定性：所有影响行动的信息都可追溯范围与 freshness，不以缓存或局部观察伪装全知。

## 5. Done：成功标准与验收

- FI-1：frontier 样例证明未知区域只能由相邻探索、物流和 charter 的受治理组合扩展，并持续接入同一时间线、经济和历史；任一跳跃或旁路被拒绝或清楚说明。
- FI-2：pioneer priority 样例证明其可转让、会到期且绑定开发里程碑/范围；它不自动产生主权、永久排他、区域控制或豁免。
- FI-3：探索数据样例区分有界私有商业使用、最低公共安全/可通行性披露与最终可公开 baseline，并保留来源、范围、freshness 和不确定性。
- FI-4：信息样例证明公共规则、已结算结果、charter 和安全事实可审计，而 live inventory、routes、deployments、plans 等需要侦察、关系、购买、合同或授权；每项实时信息均有 freshness 与恢复路径。
- FI-5：专题说明不将本长期模型或局部探索/市场证据表述为当前地图、信息系统、frontier、preview 或发行 readiness。

### 5.1 验收追踪

| 成功标准 | 专业 owner | 专业域 PRD-ID | 权威文档 | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- | --- |
| FI-1 / FI-2 | producer_system_designer / gameplay_designer / runtime_engineer / blockchain_ops_engineer / qa_engineer | PRD-GAME-002 / PRD-WORLD_RUNTIME-001 / PRD-P2P-001 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/p2p/prd.md`; `doc/testing/prd.md` | 相邻 frontier、物流/charter、同世界连续性、priority 里程碑/期限/转让与非主权负例 | test_tier_full |
| FI-3 | producer_system_designer / gameplay_designer / runtime_engineer / agent_engineer / qa_engineer | PRD-GAME-002 / PRD-WORLD_RUNTIME-001 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/testing/prd.md` | 探索数据私有范围、最低安全/通行披露、公共 baseline、来源与 freshness 证据 | test_tier_required |
| FI-4 / FI-5 | producer_system_designer / gameplay_designer / runtime_engineer / agent_engineer / viewer_engineer / qa_engineer | PRD-GAME-002 / PRD-WORLD_RUNTIME-001 / PRD-TESTING-003 | `README.md`; `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/testing/prd.md` | 公共/实时信息分层、侦察/关系/购买/授权、freshness/恢复和当前 claim 分离审计 | test_tier_required |

## 6. Non-Goals

- 不定义地图、相邻、路线、侦察、库存、部署、计划、charter、priority、数据权限或 freshness 的算法、数值、字段、UI 或 runtime 实现。
- 不把 pioneer priority、探索数据或 frontier 开发扩写为主权、永久排他、全局治理权、免费实时情报或经济旁路。
- 不承诺当前存在任何可进入 frontier、地图扩展、信息商业权、公共 baseline 或公开 API。
