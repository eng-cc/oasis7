# 全局治理、组织连续性与宪制护栏

## 文档身份

- 所属产品模块：大世界基础设施
- 上位产品 PRD：[`prd.md`](prd.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- 专业域权威：[`doc/game/prd.md`](../../game/prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/p2p/prd.md`](../../p2p/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)

本文是长期产品分册，定义普通全局治理的有限产品范围、组织连续性和不可由普通治理改写的宪制保护。它不定义阈值、锁定/解锁时长、身份技术、控制人计算、链上或智能合约结构、runtime 状态机、签名/custody 实现、当前候选 verdict 或发行就绪。

## 1. 产品目标

玩家和组织可以在一个持续、可审计的世界中处理有限的共同运行事项，而不能把持有、付费、匿名账户、组织章程或普通投票扩展为无界的世界主权。普通治理服务于政策、财库和 charter 的运行；它与宪制保护、玩家基本权利、安全、validator/finality、signer 和 custody 分层。组织可以按自身目的协作、重组或退出，但个人资产、合同、退出、历史和 Agent 身份保有不可穿透的最低保护。

本目标不声称当前已具备全局治理、锁定、委托、组织清算、estate 或宪制修订的实现与 readiness。任何当前可用性和公开 claim 仍以根 `README.md`、同一候选的专业 authority 和 QA 证据为准。

## 2. 范围与玩家边界

### 2.1 普通 OC 治理的范围与资格

- 普通 OC 治理只可决定明确列举的政策、公共财库和已成立 charter 的日常运行事项，例如已授权公共项目的方向、预算用途、运行规则和受该 charter 约束的角色/服务安排。它不得自行扩展为对任意玩家、区域或世界系统的无界命令权。
- 宪制、玩家基本权利、安全策略、validator/finality 准入、signer、custody、密钥材料、网络最终性和其他系统级安全真值不属于普通 OC 治理事项；这些事项不能由普通提案、财库投票或组织章程间接取得。
- 一个经验证的游戏账户是普通治理主体。该主体只聚合其已批准的玩家身份、managed signer 与 external wallet bindings，用于判定同一治理主体的资格、委托和计票；这些绑定不把 signer、custody、密钥、钱包控制或安全权限转化为普通治理事项或公开控制面。
- 普通资格以线性锁定 OC 为基础，可撤销委托只在明确范围内转交代表权；同一实际控制人聚合后的可计权重受上限约束。委托、拆分账户、关联组织或短期循环转移不得制造额外的普通治理影响。
- 普通游玩可保持假名。仅在高影响资格需要防止重复控制或规避上限时，系统才消费私下最小化的控制关系断言；它与经验证账户的聚合保持一致。公开面应可审计治理主体的委托关系、最终计票与结果，却不得泄露非必要的邮箱、设备、密钥或其他个人数据。
- 对重复治理主体的 challenge 必须有 review，以及适用的 merge 或 revocation、计票 correction 与 appeal 路径，以防止双计。纠正不会抹去原有授权、投票或处理的历史 receipt，而是保留可审计的原记录、纠错理由和最终有效结果。
- 区域 charter 的本地双合法性仍以锁定 OC/可撤销委托/实际控制人上限与连续本地贡献共同成立；普通全局 OC 治理既不替代区域贡献 chamber，也不覆写区域、地点或受影响主体的有界权利。

### 2.2 外部 OC 与游戏内权利的分离

- OC 作为外部链上资产保持可自由转让；外部持有、转入或转出本身不自动取得、出售或保留游戏内普通治理、高影响资格、区域权利、世界资产或行动结果。
- 游戏内治理权只来自对特定事项可审计的锁定、snapshot 与明确退出/解除绑定过程。投票或委托一旦进入相应事项的有效窗口，后续外部转让、反向转移或解除绑定不能追溯改票、重复投票或同时在两个控制关系下行使同一权利。
- 本分离不改变现有 operator-managed 的单向 `OC -> LetAI Run quota` 服务额度桥：该桥不是治理权、OC 兑回、AMM 或自动提现路径，也不能通过额度、`token_key` 或服务消费影响任何治理资格。

### 2.3 组织 charter、解散与长期不活跃

- 组织可配置宗旨、角色、内部工作流、成员资格、授权与收益/剩余分配规则，但任何 charter 都不能排除个人对其独立资产、已有合同、可理解退出、可审计历史和 Agent 稳定身份/来源的最低保护。组织权力不得把成员或 Agent 变为可任意抹除、秘密没收或失去因果记录的对象。
- 解散或资不抵债的处理按公开、可审计的连续性顺序进行：先冻结进一步风险与越权处分；再履行、终止或结清适用合同；返还可识别的托管资产；处理债权、成本和责任；随后对 Agent、设施和持续业务进行可读的转让、拍卖、重组或退休；最后才按 charter 分配剩余。全过程保留历史、receipt、来源和责任链，不以解散删除已确认因果。
- 长期不活跃不能立即把个人/组织世界价值变为任意夺取的公共财产。可在通知、保护期和可恢复主张的前提下进入 estate 或可撤销 delegation，再按分阶段规则处理维护、风险隔离、有限重启或 reclaim；每一步均要说明触发、范围、保留价值、异议和申诉。不可识别或已履行的剩余事项才可按 charter 与宪制边界进入后续处置。

### 2.4 宪制双轨与不可穿透保护

- 普通治理不能修改基本玩家保护、身份/来源与历史连续性、独立资产与退出底线、审计/申诉、系统安全权威或本节普通治理的权限边界。
- 其余宪制性改变走独立于普通 OC 运行事项的双轨流程：公开审议与可理解影响说明、充分延迟与复核、超多数、跨区域或受影响主体确认，以及独立审计和程序性申诉。只有相应轨道全部满足，才可在预先说明的边界内生效。
- 宪制流程不能以紧急、财库、组织 charter、技术升级、局部多数或历史声望为名绕过受保护底线；紧急保护只可限制扩散并保留审计/申诉，不能替代宪制修订。

## 3. 权威与冲突处理

| 产品层拥有 | 专业与执行权威 |
| --- | --- |
| 普通治理范围、OC 与游戏权利分离、控制权公平、组织 universal floors、连续性 waterfall、宪制双轨和玩家可解释边界 | `doc/game/prd.md` 拥有玩家规则和经济/平衡；`doc/world-runtime/prd.md` 拥有权威状态、资格执行、receipt 与 replay；`doc/p2p/prd.md` 拥有 OC、签名、validator/finality、网络安全与 custody 技术边界；`doc/testing/prd.md` 拥有证据与当前 verdict |

产品层不得定义锁定比例/期限、权重公式、控制人身份技术、隐私实现、投票/委托状态字段、链上交易、智能合约、validator/signer/custody 操作、清算价格或具体解散执行。与区域 charter、世界连续性或现有单向 OC→quota bridge 冲突时，采用更窄的权利与安全边界，并由相应专业 owner 形成显式裁决。

## 4. 路线图

1. 有界普通治理：只让锁定 OC 与可撤销委托服务于政策、财库和 charter 运行事项，并按实际控制人防止权力放大。
2. 可持续组织：使可配置 charter、个人底线、退出、解散和长期不活跃在同一历史中可预期处理。
3. 宪制护栏：将普通运行决策与不可穿透保护和双轨宪制修订分离。
4. 诚实证据：仅在同一候选的专业实现和 QA 证据成立时，声明其中任一能力的当前可用性。

## 5. Done：成功标准与验收

- GG-1：普通治理样例证明其可决定的政策、财库和 charter 运行事项有清晰范围；宪制、玩家权利、安全、validator/finality、signer 与 custody 事项被拒绝或路由到相应非普通轨道。
- GG-2：资格样例证明经验证游戏账户作为治理主体聚合其已批准的玩家身份、managed signer 和 external wallet bindings；线性锁定 OC、可撤销委托和按实际控制人封顶共同防止账户/组织拆分扩大影响。公开面可审计委托关系与最终计票/结果而不泄露非必要个人数据；重复主体 challenge 具有 review、merge 或 revocation、correction 与 appeal，防止双计并保留历史 receipt。
- GG-3：外部 OC 转让与游戏内普通治理权的样例证明锁定、snapshot 和退出/解除绑定防止追溯改票、双投或额度桥旁路；现有 OC→quota 桥仍严格单向且非治理。
- GG-4：组织样例证明可配置 charter 不越过个人资产、合同、退出、历史和 Agent 身份的 universal floors；解散与长期不活跃按风险冻结、合同、托管返还、债权/成本/责任、Agent/设施处置、剩余分配和历史保留的连续性顺序处理。
- GG-5：宪制样例证明普通治理无法改写受保护底线，其余宪制修改须完成公开审议、延迟、超多数、跨区域或受影响主体确认、审计和程序申诉；任何局部或历史证据不得代签当前 readiness。

### 5.1 验收追踪

| 成功标准 | 专业 owner | 专业域 PRD-ID | 权威文档 | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- | --- |
| GG-1 | producer_system_designer / gameplay_designer / blockchain_ops_engineer / runtime_engineer / qa_engineer | PRD-GAME-002 / PRD-WORLD_RUNTIME-001 / PRD-P2P-003 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/p2p/prd.md`; `doc/testing/prd.md` | 普通事项白名单、排除事项拒绝与非普通轨道路由的组合证据 | test_tier_full |
| GG-2 | producer_system_designer / blockchain_ops_engineer / runtime_engineer / qa_engineer | PRD-WORLD_RUNTIME-001 / PRD-P2P-003 / PRD-TESTING-003 | `doc/world-runtime/prd.md`; `doc/p2p/prd.md`; `doc/testing/prd.md` | 验证账户主体聚合、锁定/委托/实际控制人封顶、公开委托/最终计票审计、最小数据披露、重复主体 review/merge/revocation/correction/appeal 与历史 receipt 证据 | test_tier_full |
| GG-3 | producer_system_designer / blockchain_ops_engineer / runtime_engineer / qa_engineer | PRD-WORLD_RUNTIME-001 / PRD-P2P-003 / PRD-TESTING-003 | `doc/world-runtime/prd.md`; `doc/p2p/prd.md`; `doc/testing/prd.md` | 外部 OC 转让、游戏权利快照/解除、非追溯投票与单向 quota bridge 负例 | test_tier_full |
| GG-4 | producer_system_designer / gameplay_designer / agent_engineer / runtime_engineer / blockchain_ops_engineer / qa_engineer | PRD-GAME-002 / PRD-WORLD_RUNTIME-001 / PRD-P2P-003 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/p2p/prd.md`; `doc/testing/prd.md` | charter floors、解散 waterfall、estate/delegation/reclaim、Agent/设施处置与历史连续性证据 | test_tier_full |
| GG-5 | producer_system_designer / blockchain_ops_engineer / runtime_engineer / qa_engineer | PRD-WORLD_RUNTIME-001 / PRD-P2P-003 / PRD-TESTING-003 | `doc/product/world-infrastructure/world-continuity-governance-and-recovery.prd.md`; `doc/world-runtime/prd.md`; `doc/p2p/prd.md`; `doc/testing/prd.md` | 普通/宪制双轨、保护底线、跨区域/受影响主体确认、延迟、审计与程序申诉证据 | test_tier_full |

## 6. Non-Goals

- 不设置阈值、权重、锁定/解除期限、上限、清算价格、资格评分、身份技术/schema 或任何具体经济参数。
- 不实现账户/身份绑定、投票、委托、控制人聚合、snapshot、unbonding、estate、清算、拍卖、reclaim、宪制流程、智能合约或 runtime/P2P/custody 操作。
- 不把普通 OC 治理扩展为 validator/finality、安全、signer、custody、玩家基本权利或全局主权。
- 不改变既有单向 `OC -> LetAI Run quota` 桥，也不把它写成资产兑换、治理权、AMM 或自动提现能力。
- 不把本长期专题或历史/局部证据写成当前功能、preview readiness、主网、发行或公开 claim。
