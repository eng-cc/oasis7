# 免费进入、世界内成长与有界认可

## 文档身份

- 所属产品模块：玩家入口与发行
- 上位产品 PRD：[`prd.md`](prd.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- 专业域权威：[`doc/game/prd.md`](../../game/prd.md)、[`doc/world-simulator/prd.md`](../../world-simulator/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)

本文是长期产品分册，定义免费基础进入、可选服务、世界内成长、认可和区域互赖之间的产品边界。它不定义支付渠道、定价、账户或 onboarding 实现、具体资格算法、奖励数值、OC 发放/兑换、治理字段或当前 preview 放行结论。

## 1. 产品目标

长期产品目标是让玩家无需购买客户端、账户或基础进入资格即可进入受支持世界路径；付费只能购买可选的 hosting、storage、support 等便利服务，不能购买、租用或跳过世界中的实质权力。早期体验应容易进入，并让愿意继续的玩家自愿展开真实的工业、训练、模块、合同、凭证、关系和区域协作深度，而不是用永久账号天赋树或全局战力分替代世界历史。

该目标描述的是产品方向与组合验收要求，不表示当前任何 preview 已实现免费广泛可用、某项付费服务已上线，或当前版本已经通过发行/可玩性门禁；当前入口、可用性和公开 claim 始终以根 `README.md`、对应专业域和同一候选证据为准。

## 2. 范围与玩家边界

### 2.1 免费基础进入与可选服务

- 免费客户端、账户和基础进入只解决进入世界路径的门槛；它们不承诺免费 Agent、免费世界资产、无成本建造、无限补贴、经济旁路或对当前技术预览的无条件可用性。
- 可选付费 hosting、storage 或 support 可以改善托管、保留、协助或时间便利，但不得直接授予/出售/租用世界资源、行动权限、设施控制、训练成果、模块能力、合同结果、凭证、区域优先权、OC、治理票权或冲突优势。
- 便利服务也不得绕过资源、时间、资格、授权、物流、治理、反支配或反滥用边界；若服务影响世界内动作，它只能在与免费路径相同的权威规则和可审计因果下执行。
- 首个 Agent 的取得、持续经营和任何受限资助仍由世界规则与专业 authority 决定。本分册不把免费账户或可选服务改写成免费 Agent claim，也不改变既有 OC 到 quota 的单向边界。

### 2.2 易进入而自愿展开的真实深度

- 早期路径先给出当前目标、主要阻塞、下一步和可理解的世界后果；玩家无需在进入时掌握完整经济、组织、外交或治理体系。
- 深度系统在玩家准备好时展开，且其成本、锁定、风险、权限、损失和恢复后果必须保持真实可读；渐进披露不等于删去这些后果、把它们交给不可审计的自动化，或把深度设为参与资格。
- 账户不是永久 power tree。能改变世界的持续能力必须来自世界内可失去、维护、训练、取得、协商或证明的资产、训练、模块、合同、credentials 与关系，并受同一物理、授权、治理和恢复规则约束。
- 玩家历史应以多维的角色、地点、区域、关系、贡献、合同和可审计结果表达；不得归约成跨一切情境自动支配机会的全局强度分、永久等级或账号权力。

### 2.3 有界认可、独立基线与区域互赖

- 可核验的贡献、信誉或区域历史可以在明确范围、期限、用途和复核条件下带来未来机会、候选资格、可见度、协作邀请或有限优先级；它们不自动产生世界资产、行动成功、OC、治理权、地区控制、战利品或不受约束的持续 power。
- 认可必须可追溯到相称的世界内事实或适用的审核/治理决定；访问、在线时长、付费服务、泛化互动和自我声明都不自动构成认可或资格。
- 玩家应始终存在可行的独立基线：在不立即加入强组织、接受赞助或交出自治的前提下，能够形成、维持或恢复一项有用能力。独立不等于孤立；区域专业化、贸易、物流、互助和协议应能带来更高效率、更多选择或更强韧性，但不得成为基础生存、基本成长或恢复的强制门票。
- 区域互赖产生的机会、资格与优先级必须受地点、贡献、容量、期限和治理边界约束；不能由一次付费、历史名望或全局分数永久锁定，也不能越界为全局治理或跨区域控制。

## 3. 权威与冲突处理

| 产品层拥有 | 专业与执行权威 |
| --- | --- |
| 免费进入与非权力型便利服务的产品边界、世界内成长/认可的结果语义、独立基线与区域互赖的组合要求、以及长期目标和当前 claim 的隔离 | 根 `README.md` 拥有当前公开状态与 claim envelope；`doc/world-simulator/prd.md` 拥有客户端、账户、入口、首个 Agent/onboarding 与发行实现；`doc/game/prd.md` 拥有成长、经济、玩法和数值；`doc/testing/prd.md` 拥有证据和当前 verdict |

产品层不得定义支付实现、价格、账户数据、资格评分、OC 额度或兑换、治理计票、世界资产发放、Agent claim 规则或具体成长/平衡数值。任何当前可用性、付费服务、资格或奖励说明缺少同一候选的专业证据与根 `README.md` 支持时，采用更窄的未承诺边界。

## 4. 路线图

1. 基础进入公平：把客户端、账户和基础进入与世界权力及付费便利服务明确分离。
2. 世界内成长：让早期路径可理解，随后自愿展开有真实成本与后果的深度，而非账号 power tree。
3. 有界协作：使独立基线可行，并让区域互赖和认可在可审计、有限且不自动授予权力的规则下提供额外机会。
4. 诚实发行：只在当前候选证据支持时，将其中已实现的部分写入公开 claim。

## 5. Done：成功标准与验收

- FE-1：产品说明能区分免费客户端/账户/基础进入、世界内需要取得和维护的实质能力，以及不授予世界权力的可选付费便利服务；不会把其中任何一项误写成当前 preview 已广泛可用。
- FE-2：代表性早期和后续路径证明玩家可逐步展开真实深度，且能读懂实质后果；持续 power 不来自永久账号树或全局强度分，而来自可审计的世界内资产、训练、模块、合同、credentials 和关系。
- FE-3：认可样例只授予有界的未来机会或资格，并能说明范围、期限、用途、事实/审核来源与复核条件；它不自动授予 power、OC、治理或区域控制。
- FE-4：成熟世界样例证明玩家具备不立即依附强组织的可行独立基线，同时区域专业化与互赖在不强制依附的前提下带来可读优势。
- FE-5：任何公开或入口 surface 将长期方向、当前候选证据、实际受支持入口与未承诺内容分开；历史或局部 evidence 不得代签免费可用性、支付服务、成长完整性或发行就绪。

### 5.1 验收追踪

| 成功标准 | 专业 owner | 专业域 PRD-ID | 权威文档 | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- | --- |
| FE-1 | producer_system_designer / viewer_engineer / qa_engineer / liveops_community | PRD-WORLD_SIMULATOR-042/043/045 / PRD-TESTING-003 | `README.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 免费基础入口、可选便利服务和当前 claim 的负例/组合审计 | test_tier_required |
| FE-2 | producer_system_designer / gameplay_designer / runtime_engineer / agent_engineer / viewer_engineer | PRD-GAME-015 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_SIMULATOR-001 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md` | 早期到长期的世界内能力、渐进深度和非账号 power tree 组合证据 | test_tier_required |
| FE-3 | producer_system_designer / gameplay_designer / blockchain_ops_engineer / qa_engineer | PRD-GAME-015 / PRD-P2P-001 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/p2p/prd.md`; `doc/testing/prd.md` | 有界资格/机会、非自动 OC/治理/power 与可审计来源的样例 | test_tier_required |
| FE-4 | producer_system_designer / gameplay_designer / agent_engineer / runtime_engineer / qa_engineer | PRD-GAME-015 / PRD-WORLD_RUNTIME-001 / PRD-TESTING-003 | `doc/product/world-rules-core-gameplay/mature-world-progression.prd.md`; `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/testing/prd.md` | 独立基线、区域互赖优势与非强制依附的 mature-world fresh sample | test_tier_full |
| FE-5 | producer_system_designer / viewer_engineer / qa_engineer / liveops_community | PRD-WORLD_SIMULATOR-042/043 / PRD-TESTING-003 | `README.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 长期方向、候选证据、支持入口和公开 claim 分离审计 | test_tier_required |

## 6. Non-Goals

- 不设计支付、订阅、托管、存储、支持、账户、登录或 onboarding 实现，也不声明任何服务已销售或可用。
- 不改变首个 Agent 的 claim/upkeep 合同、受限资助语义或 OC 到 quota 的既有单向桥接。
- 不定义永久等级、全局战力分、职业数值、资格算法、奖励分配、OC 经济、治理权或区域控制的执行规则。
- 不把长期免费进入、世界内成长或区域互赖目标当作当前 preview readiness、release gate、可玩性或公开承诺的替代证据。
