# Frontier 扩展与世界信息边界

## 文档身份

- 所属产品模块：世界规则与核心玩法
- 上位产品 PRD：[`prd.md`](prd.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- 专业域权威：[`doc/game/prd.md`](../../game/prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/p2p/prd.md`](../../p2p/prd.md)、[`doc/world-simulator/prd.md`](../../world-simulator/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)

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

### 2.3 Priority 与 tenure 的独立切割

- pioneer priority 只是对下一个合格开发步骤的限时、限范围优先机会；它本身不创建 tenure、所有权、charter 权力或排他访问。tenure 只能在其独立资格、用途和确认程序完成后成立。
- tenure 成立前转让 priority，只转让剩余的地点/用途范围、里程碑义务、原到期时点和相关 receipt 链；受让人不继承发现者身份、声誉、私有数据权限或预先批准的 tenure，也不得借转让延长原期限。
- priority 到期、撤销或里程碑失败时，只终止尚未消费的优先机会并释放对应开发步骤；不得静默取消已独立确认的 tenure、已完成交付、已结算合同或其 receipt。已确认 tenure 仅按自身的续期、使用、违约、公共必要性、通知、申诉与恢复规则处理；过往 priority 也不为 tenure 保留新的排他授予或续期优势。
- priority 资格、里程碑或到期事实处于争议且证据不足时，拒绝新的 priority 排他授予与转让，但不冻结普通安全披露、非排他探索、已确认 tenure 或已结算义务。争议结果的 receipt 必须说明范围、原因、下一路径以及 tenure 是否曾被独立确认；已撤销 priority 不得通过重放历史 receipt 重新取得。

#### 一次性消费、并发转让与可恢复失败

pioneer priority 在其未到期、未撤销且里程碑仍适用时，只有两类会改变机会归属的结果：**已接受的转让**，或**已接受的下一个合格开发步骤**。前者将剩余机会及其原有期限转给一个受让人；后者只消费该步骤所需的优先机会，不能顺带创建 tenure、扩展到其他地点/用途，或把后续步骤预先排他。尚未得到权威结果的转让或开发请求只是待决，不能被玩家、Agent、界面缓存或链下约定当作已取得/已消费 priority。

- 对同一 priority 的并发转让、受让后立即提交、原持有人与受让人竞争提交，或重连后的重复请求，世界只能接受其中一个符合当时条件的归属/消费结果；其余请求必须原子拒绝、过期或按专业合同保持待决，且不得产生第二次 priority 消费、额外开发资格、隐性排队优势或资源 sink。玩家能读到当前持有人或已消费范围、冲突原因，以及重新评估、等待、以非 priority 路径探索，或在条件允许时重新竞争的下一步。
- 网络中断、审计尚未完成或争议开始时，未能证明已被接受的请求不得延长到期时点、保留私下排他，或阻止无关的安全披露、普通探索和既有已结算义务。恢复后只能查询 receipt 并按当时仍有效的资格重新评估；历史 transfer/priority receipt 不能重放为新的转让或开发提交。
- 为防止把有限先行机会变成可套利的永久排他，拆分、循环倒手、向关联主体回转或用未结算请求反复刷新，均不得重置期限、里程碑、地点/用途范围或已消费范围；任何允许的转让仍保留完整 provenance，并受同一争议、撤销和反滥用边界约束。

#### Tenure 与 charter 状态转换的待决边界

tenure 的续期、转让、收回、迁移、补偿或申诉请求必须绑定提交时的 charter 身份、适用范围和 tenure 状态。登记、排队、链下约定或界面提示不等于控制、处置、补偿或其他世界结果已经成立。

- charter 暂停、解散、合并、边界收缩或由新授权替代后，尚未得到权威 receipt 的旧请求不得自动转入新 charter、继承优先级或产生控制、资产、补偿或其他处置效果。它们只能在旧状态仍允许时按专业合同确认，或被明确拒绝/保持待决；新 charter 确实覆盖相同需要时，主体必须显式重提，并保留与旧请求的可读关联。
- 同一 tenure 范围内并发的续期、转让、收回、迁移或补偿请求，至多一个权威 receipt 可以改变控制或处置归属。其余请求必须原子拒绝、过期或保持待决；重连、自动重试或历史 receipt 重放不得制造第二次世界效果、隐性排队优势或静默没收。
- 受影响主体必须能读到原请求绑定的 charter、当前状态、范围变化或拒绝原因，以及重提、申诉、迁移、独立替代或等待等适用的下一步。已确认历史保持可审计，不能因后续 charter 状态变化被追溯改写。

这些规则保持 **world-first**：归属与消费只以同一权威时间线中的接受结果为准；保持 **emergence-first**：探索和协作可竞争、可转让但不产生主权或免费旁路；保持 **persistent/auditable**：已结算结果与未结算失败可由 receipt 和范围追溯；保持 **extensible**：未来可增加开发步骤或转让机制，但必须保留一次性消费、原期限和无重复权益的不变量。

### 2.4 探索数据与公共安全基线

- 探索获得的数据可在与地点、用途和时间相称的范围内提供私有商业使用或先行研究优势；这不构成永久秘密、对他人基本恢复能力的剥夺或安全风险的隐瞒许可。
- 至少与公共安全、可通行性、重大已知危险和受影响主体基本恢复有关的最低事实必须按适用授权公开；私有期、保护期或争议处理结束后，可公开的探索知识应形成可复核的公共 baseline。个人数据、敏感漏洞细节、实时防御部署或他人私有合同不因公共 baseline 而自动公开。
- 已披露或被消费的数据都必须说明来源、范围、取得/更新时间和不确定性；过期、冲突或局部观察不得被包装成整个 frontier 的实时完整真值。

### 2.5 可审计公共事实与需要取得的实时信息

- 公共规则、已结算世界结果、已成立 charter、适用公共安全信息和可公开 baseline 必须可审计，不能被拥有更强信息能力的主体秘密改写。
- 实时库存、路线容量/状态、部署、计划、未结算订单和其他 live operational information 默认需要侦察、已有关系、购买、合同或明确授权才能获得；玩家不能因访问产品、持有 OC、加入组织或历史名望自动读取。
- 任何向玩家或 Agent 提供的实时信息必须标明 freshness、范围和不确定性，并在授权失效、侦察不足、数据冲突或状态变化时给出刷新、替代、等待或停止的可读路径。

## 3. 权威与冲突处理

| 产品层拥有 | 专业与执行权威 |
| --- | --- |
| frontier 进入的产品结果、priority 非主权、探索数据的私有/安全/公共分层，以及公共事实与实时信息的可见性边界 | `doc/game/prd.md` 拥有探索、物流、工业和玩家规则；`doc/world-runtime/prd.md` 拥有相邻/资格、世界状态、授权、receipt 与确定性执行；`doc/p2p/prd.md` 拥有分布式状态与安全技术边界；`doc/world-simulator/prd.md` 仅拥有玩家可见状态、receipt 与下一动作的表面表达；`doc/testing/prd.md` 拥有证据与当前 verdict |

本分册不定义地理算法、范围/时长/价格、情报保密技术、侦察实现、物流数值、charter/tenure 状态或公开 API。它不扩大现有区域 charter、市场发现、情报或玩家 surface 的当前专业真值；缺少同一候选专业证据时，只能保持为长期目标或较窄未承诺边界。

## 4. 路线图

1. 连续扩展：建立相邻探索、物流和 charter 缺一不可的 frontier 进入结果。
2. 有界先行：让 pioneer priority 支持早期开发而不成为永久主权或圈地。
3. 信息公平：把公共规则/已结算事实与实时运营情报分层，并让探索数据最终可形成安全的公共 baseline。
4. 可读不确定性：所有影响行动的信息都可追溯范围与 freshness，不以缓存或局部观察伪装全知。

## 5. Done：成功标准与验收

- FI-1：frontier 样例证明未知区域只能由相邻探索、物流和 charter 的受治理组合扩展，并持续接入同一时间线、经济和历史；任一跳跃或旁路被拒绝或清楚说明。
- FI-2：pioneer priority 样例证明其可转让、会到期且绑定开发里程碑/范围；它不自动产生 tenure、所有权、主权、永久排他、区域控制或豁免。验收至少覆盖：到期于 tenure 成立前时释放/重新竞争；成立前转让不转移身份/声誉/数据权限且不延期；已独立成立 tenure 后 priority 到期不追溯没收；争议期不重复授予、不冻结无关安全/通行/已结算义务；已撤销 priority 不能凭历史 receipt 重放恢复；以及同一 priority 的转让、开发提交、重连重试或关联主体倒手竞争时，至多一个结果改变归属或消费范围，其他请求不产生第二次权益/sink，也不能重置期限或里程碑。charter 暂停、边界变更、解散、合并或授权替代时，待决 tenure 续期、转让、收回、迁移、补偿或申诉请求不得跨越旧授权自动确认、迁移或继承优先级；同范围竞态至多一个 receipt 改变处置归属，受影响主体可读到原绑定、当前结果与适用下一步。每个样例都必须区分待定/争议中、有效、已转让、已消费、已独立成立 tenure 和到期/撤销状态，并提供 receipt 支持的下一动作。
- FI-3：探索数据样例区分有界私有商业使用、最低公共安全/可通行性披露与最终可公开 baseline，并保留来源、范围、freshness 和不确定性。
- FI-4：信息样例证明公共规则、已结算结果、charter 和安全事实可审计，而 live inventory、routes、deployments、plans 等需要侦察、关系、购买、合同或授权；每项实时信息均有 freshness 与恢复路径。
- FI-5：专题说明不将本长期模型或局部探索/市场证据表述为当前地图、信息系统、frontier、preview 或发行 readiness。

### 5.1 验收追踪

| 成功标准 | 专业 owner | 专业域 PRD-ID | 权威文档 | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- | --- |
| FI-1 / FI-2 | producer_system_designer / gameplay_designer / runtime_engineer / blockchain_ops_engineer / viewer_engineer / qa_engineer | PRD-GAME-002 / PRD-WORLD_RUNTIME-001 / PRD-P2P-001 / PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/p2p/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 相邻 frontier、物流/charter、同世界连续性；priority 里程碑/期限/转让；priority/tenure 独立状态、转让与开发提交并发、重复转让/重试、关联倒手、receipt 重放和静默 tenure 撤销负例；charter 状态转换下 tenure 待决请求的非跨越、单一处置 receipt 与玩家可读下一步 | test_tier_full |
| FI-3 | producer_system_designer / gameplay_designer / runtime_engineer / agent_engineer / qa_engineer | PRD-GAME-002 / PRD-WORLD_RUNTIME-001 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/testing/prd.md` | 探索数据私有范围、最低安全/通行披露、公共 baseline、来源与 freshness 证据 | test_tier_required |
| FI-4 / FI-5 | producer_system_designer / gameplay_designer / runtime_engineer / agent_engineer / viewer_engineer / qa_engineer | PRD-GAME-002 / PRD-WORLD_RUNTIME-001 / PRD-TESTING-003 | `README.md`; `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/testing/prd.md` | 公共/实时信息分层、侦察/关系/购买/授权、freshness/恢复和当前 claim 分离审计 | test_tier_required |

## 6. Non-Goals

- 不定义地图、相邻、路线、侦察、库存、部署、计划、charter、priority、数据权限或 freshness 的算法、数值、字段、UI 或 runtime 实现。
- 不把 pioneer priority、探索数据或 frontier 开发扩写为主权、永久排他、全局治理权、免费实时情报或经济旁路。
- 不承诺当前存在任何可进入 frontier、地图扩展、信息商业权、公共 baseline 或公开 API。
